use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use godot::prelude::*;

use crate::terrain::GenerationRules;
use crate::terrain::{BiomeManager, ChunkManager, TerrainWorldIntegration};
use crate::core::event_bus::EventBus;
use crate::core::config_manager::{GameConfiguration, GameModeConfig, WorldSize};


// Unique identifier for world entities
type EntityId = Uuid;

// Base trait for all world entities
trait WorldEntity: Send + Sync {
    fn get_id(&self) -> EntityId;
    fn serialize(&self) -> Vec<u8>;
    fn deserialize(data: &[u8]) -> Self where Self: Sized;
}

// World state configuration
#[derive(Clone, Serialize, Deserialize)]
pub struct WorldStateConfig {
    pub seed: u64,
    pub world_size: (u32, u32),
    pub generation_parameters: GenerationRules,
}

// Comprehensive world state management
pub struct WorldStateManager {
    // Atomic, thread-safe world state
    entities: Arc<RwLock<HashMap<EntityId, Arc<dyn WorldEntity>>>>,
    
    // World configuration
    config: WorldStateConfig,
    
    // State versioning for synchronization
    current_version: u64,
    
    // Terrain generation system - old implementation
    terrain_generator: TerrainGenerator,

    // Pending initialization data
    pending_init: bool,
    pending_seed: u64,
    pending_size: (u32, u32),
    
    // Terrain system integration
    terrain_integration: Option<TerrainWorldIntegration>,
    
    // Event bus reference
    event_bus: Option<Arc<EventBus>>,
    
    // Initialization status
    is_terrain_initialized: bool,
    initialized: bool,
}

impl WorldStateManager {
    // Create a new world state
    pub fn new(config: WorldStateConfig) -> Self {
        Self {
            entities: Arc::new(RwLock::new(HashMap::new())),
            config: config.clone(),
            current_version: 0,
            terrain_generator: TerrainGenerator::new(config.clone()),
            pending_init: false,
            pending_seed: 0,
            pending_size: (0, 0),
            terrain_integration: None,
            event_bus: None,
            is_terrain_initialized: false,
            initialized: false,

        }
    }

    // Create a world state with dependencies
    pub fn new_with_dependencies(
        config: WorldStateConfig,
        event_bus: Option<Arc<EventBus>>,
        terrain_integration: Option<TerrainWorldIntegration>,
    ) -> Self {
        // Determine initialization status before potentially moving the value
        let is_terrain_initialized = terrain_integration.is_some();
    
        Self {
            entities: Arc::new(RwLock::new(HashMap::new())),
            config: config.clone(),
            current_version: 0,
            terrain_generator: TerrainGenerator::new(config.clone()),
            pending_init: false,
            pending_seed: 0,
            pending_size: (0, 0),
            terrain_integration,
            event_bus,
            is_terrain_initialized, // Use the pre-computed value
            initialized: false,
        }
    }

    
    pub fn initialize(&mut self) -> Result<(), String> {
        println!("WorldStateManager: Initializing world state");
        
        // Initialize terrain integration if not already done
        if self.terrain_integration.is_none() {
            // Create a clone of self to avoid ownership issues
            let self_arc = Arc::new(Mutex::new(self.clone()));
            let terrain_integration = TerrainWorldIntegration::new(self_arc);
            self.terrain_integration = Some(terrain_integration);
        }
        
        // If we're not in a Godot context, just initialize world structures
        // without visual representation
        self.pending_init = true;
        self.pending_seed = self.config.seed;
        self.pending_size = self.config.world_size;
        
        // Create and initialize BiomeManager and ChunkManager directly from Rust
        if !self.is_terrain_initialized {
            println!("WorldStateManager: Auto-creating BiomeManager and ChunkManager for terrain");
            
            // Create the Godot objects needed for terrain
            let biome_manager = BiomeManager::new_alloc();
            let chunk_manager = ChunkManager::new_alloc();
            
            // Configure BiomeManager with world parameters
            {
                let mut bm = biome_manager.clone();
                bm.bind_mut().set_seed(self.config.seed as u32);
                bm.bind_mut().set_world_dimensions(
                    self.config.world_size.0 as f32, 
                    self.config.world_size.1 as f32
                );
            }
            
            // Set up the ChunkManager
            {
                let mut cm = chunk_manager.clone();
                cm.bind_mut().set_biome_manager(biome_manager.clone());
                cm.bind_mut().update_thread_safe_biome_data();
            }
            
            // Initialize the terrain system with these managers
            // We need to handle this differently to avoid the move issue
            let terrain_init_result = match &mut self.terrain_integration {
                Some(terrain) => {
                    terrain.initialize_terrain(biome_manager.clone(), chunk_manager.clone())
                },
                None => Err("Terrain integration missing".to_string())
            };
            
            // Now handle the result
            if let Err(e) = terrain_init_result {
                println!("WorldStateManager: Warning: Auto-terrain initialization failed: {}", e);
                // Continue anyway, as we've marked the pending initialization
            } else {
                self.is_terrain_initialized = true;
                println!("WorldStateManager: Auto-terrain initialization successful");
            }
        }
        
        println!("WorldStateManager: Initialization complete");
        self.initialized = true;
        Ok(())
    }
    
    // Check if initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
       
    // Initialize terrain with BiomeManager and ChunkManager
    pub fn initialize_terrain(&mut self, 
        biome_manager: Gd<BiomeManager>, 
        chunk_manager: Gd<ChunkManager>) -> Result<(), String> {
        // If terrain integration is not initialized, try to initialize the whole world state
        println!("WorldStateManager: Initializing terrain with provided components");
        if self.terrain_integration.is_none() {
            // Use ? to propagate any initialization errors
            self.initialize()?;
            let self_arc = Arc::new(Mutex::new(self.clone()));
            let terrain_integration = TerrainWorldIntegration::new(self_arc);
            self.terrain_integration = Some(terrain_integration);

        }
    
        // Clone the terrain integration to avoid moving
        if let Some(ref mut terrain) = self.terrain_integration {
            // Create a GameConfiguration from our WorldStateConfig
            let game_config = GameConfiguration {
                world_seed: self.config.seed,
                world_size: WorldSize {
                    width: self.config.world_size.0,
                    height: self.config.world_size.1,
                },
                game_mode: GameModeConfig::Standalone,
                network: Default::default(),
                generation_rules: self.config.generation_parameters.clone(),
                custom_settings: Default::default(),
            };
    
            // Initialize terrain system
            let result = terrain.initialize_terrain(
                biome_manager.clone(), 
                chunk_manager.clone()
            );
    
            if result.is_ok() {
                self.is_terrain_initialized = true;
                println!("WorldStateManager: Terrain system initialized successfully");
            }
    
            result
        } else {
            Err("Terrain integration not available".to_string())
        }
    }
    
    // Set event bus reference
    pub fn set_event_bus(&mut self, event_bus: Arc<EventBus>) {
        self.event_bus = Some(event_bus.clone());
        
        // Connect terrain system if already created
        if let Some(terrain) = &self.terrain_integration.as_mut() {
            terrain.connect_to_event_bus(event_bus);
        }
    }
    
    // Process pending events - should be called regularly from main thread
    pub fn process_pending_events(&mut self) {
        if let Some(terrain) = &mut self.terrain_integration {
            terrain.process_pending_events();
        }
    }
    
    // Check if terrain is fully initialized
    pub fn is_terrain_initialized(&self) -> bool {
        self.is_terrain_initialized
    }
    
    // Get terrain integration
    pub fn get_terrain_integration(&self) -> Option<&TerrainWorldIntegration> {
        self.terrain_integration.as_ref()
    }
    
    // Existing methods
    pub fn set_pending_world_init(&mut self, seed: u64, size: (u32, u32)) {
        self.pending_init = true;
        self.pending_seed = seed;
        self.pending_size = size;
    }
    
    pub fn get_pending_world_init(&self) -> (bool, u64, (u32, u32)) {
        (self.pending_init, self.pending_seed, self.pending_size)
    }
    
    pub fn clear_pending_world_init(&mut self) {
        self.pending_init = false;
    }

    // Generate initial world state
    pub fn generate_initial_world(&mut self) {
        println!("WorldStateManager: Generating initial world");
        
        // If not initialized, make sure initialization happens
        if !self.is_terrain_initialized {
            println!("WorldStateManager: Terrain not initialized, attempting auto-initialization");
            // Try initializing first
            if let Err(e) = self.initialize() {
                println!("WorldStateManager: Failed to auto-initialize: {}", e);
            }
        }
        
        // Check again after attempted initialization
        if self.is_terrain_initialized {
            if let Some(mut terrain) = self.terrain_integration.as_mut() {
                // Update terrain based on world state
                println!("WorldStateManager: Updating terrain integration with the latest configuration");
                terrain.update();
                println!("WorldStateManager: Generated world using modern terrain system");
            }
        } else {
            // Legacy approach - use the old terrain generator
            println!("WorldStateManager: Falling back to legacy terrain generator");
            let terrain = self.terrain_generator.generate_world();
            self.populate_initial_entities(terrain);
            println!("WorldStateManager: Generated world using legacy terrain generator");
        }
        
        // Increment world version
        self.current_version += 1;
        println!("WorldStateManager: World generation complete, version incremented to {}", self.current_version);
    }

    fn populate_initial_entities(&mut self, terrain: WorldTerrain) {
        // This method creates the initial set of entities in the world
        // based on the generated terrain
        println!("Populating world with initial entities for seed: {}", terrain.seed);
        
        // In a full implementation, you would:
        // - Add resource nodes
        // - Create spawn points
        // - Add environmental objects
        // - Set up initial NPCs if any
    }

    // Add an entity to the world
    fn add_entity(&mut self, entity: Arc<dyn WorldEntity>) {
        let mut entities = self.entities.write().unwrap();
        entities.insert(entity.get_id(), entity);
        
        // Increment world version to track changes
        self.current_version += 1;
    }

    // Remove an entity from the world
    fn remove_entity(&mut self, entity_id: EntityId) {
        let mut entities = self.entities.write().unwrap();
        entities.remove(&entity_id);
        
        // Increment world version to track changes
        self.current_version += 1;
    }

    // Get an entity by ID
    fn get_entity(&self, entity_id: &EntityId) -> Option<Arc<dyn WorldEntity>> {
        let entities = self.entities.read().unwrap();
        entities.get(entity_id).cloned()
    }

    // Serialize world state for network transmission
    pub fn serialize_world_state(&self) -> Vec<u8> {
        let entities = self.entities.read().unwrap();
        
        // Also get terrain data if available
        let terrain_data = if let Some(terrain) = &self.terrain_integration {
            terrain.get_terrain_data()
        } else {
            Vec::new()
        };
        
        // Serialize entities and world state
        let serialized_entities: Vec<_> = entities
            .values()
            .map(|entity| entity.serialize())
            .collect();
        
        // Use bincode for efficient serialization
        bincode::serialize(&(self.current_version, serialized_entities, terrain_data))
            .expect("Failed to serialize world state")
    }

    // Deserialize and apply world state
    fn deserialize_world_state(&mut self, data: &[u8]) {
        // Deserialize world state
        let (version, serialized_entities, terrain_data): (u64, Vec<Vec<u8>>, Vec<u8>) = 
            bincode::deserialize(data)
            .expect("Failed to deserialize world state");
        
        // Only update if newer version
        if version > self.current_version {
            let mut entities = self.entities.write().unwrap();
            
            // Clear existing entities
            entities.clear();
            
            // Recreate entities from serialized data
            for entity_data in serialized_entities {
                // This would require a registry of entity types
                // and a way to deserialize each type
                // Placeholder implementation
                // let entity = SomeEntityType::deserialize(&entity_data);
                // entities.insert(entity.get_id(), Arc::new(entity));
            }
            
            // Apply terrain data if available
            if !terrain_data.is_empty() && self.terrain_integration.is_some() {
                if let Some(terrain) = &mut self.terrain_integration {
                    terrain.apply_terrain_data(&terrain_data);
                }
            }
            
            // Update version
            self.current_version = version;
        }
    }

    // Reconcile state differences
    fn reconcile_state(&mut self, other_state: &WorldStateManager) {
        // Compare and merge states
        if other_state.current_version > self.current_version {
            // Deep copy state from other manager
            *self = other_state.clone();
        }
    }
    
    // Get configuration
    pub fn get_config(&self) -> &WorldStateConfig {
        &self.config
    }
    
    // Update configuration
    pub fn update_config(&mut self, config: WorldStateConfig) {
        self.config = config;
        
        // Update terrain generator
        self.terrain_generator = TerrainGenerator::new(self.config.clone());
        
        // Notify terrain system if initialized
        if let Some(terrain) = &mut self.terrain_integration {
            if let Some(biome_mgr) = terrain.get_biome_manager() {
                let mut bm = biome_mgr.clone();
                bm.bind_mut().set_seed(self.config.seed as u32);
                bm.bind_mut().set_world_dimensions(
                    self.config.world_size.0 as f32, 
                    self.config.world_size.1 as f32
                );
            }
        }
    }
}

// Helper struct for GameConfiguration compatibility
struct GameSize {
    width: u32,
    height: u32,
}

// Implement Clone for WorldStateManager
impl Clone for WorldStateManager {
    fn clone(&self) -> Self {
        // Create a new instance with the same configuration
        let cloned = Self {
            entities: Arc::new(RwLock::new(HashMap::new())),
            config: self.config.clone(),
            current_version: self.current_version,
            terrain_generator: self.terrain_generator.clone(),
            pending_init: self.pending_init,
            pending_seed: self.pending_seed,
            pending_size: self.pending_size,
            terrain_integration: None, // Can't clone easily, will be recreated when needed
            event_bus: self.event_bus.clone(),
            is_terrain_initialized: self.is_terrain_initialized,
            initialized: self.initialized,
        };
        
        // Copy entities if needed
        if let Ok(entities) = self.entities.read() {
            let mut cloned_entities = cloned.entities.write().unwrap();
            for (id, entity) in entities.iter() {
                cloned_entities.insert(id.clone(), entity.clone());
            }
        }
        
        cloned
    }
}

// Terrain generation system (legacy approach)
#[derive(Clone)]
struct TerrainGenerator {
    config: WorldStateConfig,
}

impl TerrainGenerator {
    fn new(config: WorldStateConfig) -> Self {
        Self { config }
    }

    // Generate world terrain
    fn generate_world(&self) -> WorldTerrain {
        // Use configuration to generate deterministic terrain
        WorldTerrain {
            seed: self.config.seed,
            size: self.config.world_size,
            // Additional terrain generation logic
        }
    }
}

// Basic terrain representation (legacy approach)
struct WorldTerrain {
    seed: u64,
    size: (u32, u32),
    // Additional terrain data
}

// NEW: Helper extension for TerrainWorldIntegration
trait TerrainWorldIntegrationExt {
    fn get_biome_manager(&self) -> Option<Gd<BiomeManager>>;
    fn get_chunk_manager(&self) -> Option<Gd<ChunkManager>>;
}

impl TerrainWorldIntegrationExt for TerrainWorldIntegration {
    fn get_biome_manager(&self) -> Option<Gd<BiomeManager>> {
        // This would need to be implemented in TerrainWorldIntegration
        // For now, return None as a placeholder
        None
    }
    
    fn get_chunk_manager(&self) -> Option<Gd<ChunkManager>> {
        // This would need to be implemented in TerrainWorldIntegration
        // For now, return None as a placeholder
        None
    }
}

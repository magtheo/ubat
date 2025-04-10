use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use godot::prelude::*;

use crate::terrain::GenerationRules;
use crate::terrain::{BiomeManager, ChunkManager};
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

    // Pending initialization data
    pending_init: bool,
    pending_seed: u64,
    pending_size: (u32, u32),

    // terrain managers
    biome_manager: Option<Gd<BiomeManager>>,
    chunk_manager: Option<Gd<ChunkManager>>,

    
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
            pending_init: false,
            pending_seed: 0,
            pending_size: (0, 0),
            event_bus: None,
            chunk_manager: None,
            biome_manager: None,
            is_terrain_initialized: false,
            initialized: false,

        }
    }

    // Create a world state with dependencies
    pub fn new_with_dependencies(
        config: WorldStateConfig,
        event_bus: Option<Arc<EventBus>>,
    ) -> Self {
        Self {
            entities: Arc::new(RwLock::new(HashMap::new())),
            config: config.clone(),
            current_version: 0,
            biome_manager: None,
            chunk_manager: None,
            pending_init: false,
            pending_seed: 0,
            pending_size: (0, 0),
            event_bus,
            is_terrain_initialized: false, // Use the pre-computed value
            initialized: false,
        }
    }

    
    pub fn initialize(&mut self) -> Result<(), String> {
        println!("WorldStateManager: Initializing world state");
        
        // If we're not in a Godot context, just initialize world structures
        // without visual representation
        self.pending_init = true;
        self.pending_seed = self.config.seed;
        self.pending_size = self.config.world_size;
        
        // Don't auto-create terrain components - rely on them being passed in
        // via initialize_terrain()
        
        println!("WorldStateManager: Basic initialization complete");
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
        
        println!("WorldStateManager: Initializing terrain with provided components");
        
        // Store the managers directly instead of using TerrainWorldIntegration
        self.biome_manager = Some(biome_manager.clone());
        self.chunk_manager = Some(chunk_manager.clone());
        
        // Configure BiomeManager with our world settings
        {
            let mut bm = biome_manager.clone();
            bm.bind_mut().set_seed(self.config.seed as u32);
            bm.bind_mut().set_world_dimensions(
                self.config.world_size.0 as f32, 
                self.config.world_size.1 as f32
            );
        }
        
        // Configure ChunkManager 
        {
            let mut cm = chunk_manager.clone();
            cm.bind_mut().set_biome_manager(biome_manager.clone());
            cm.bind_mut().update_thread_safe_biome_data();
        }
        
        // Connect to event bus if available
        if let Some(event_bus) = &self.event_bus {
            // TODO: Publish initialization events through the event bus
            // ...
        }
        
        self.is_terrain_initialized = true;
        println!("WorldStateManager: Terrain system initialized successfully");
        
        Ok(())
    }
    
    // Set event bus reference
    pub fn set_event_bus(&mut self, event_bus: Arc<EventBus>) {
        self.event_bus = Some(event_bus.clone());
        // The event bus will be passed to all components via the system initializer
        // No additional connections needed here
    }
    
    // Process pending events - should be called regularly from main thread
    pub fn process_pending_events(&mut self) {
        // With the central system initializer, event processing is handled elsewhere
        // This method can be simplified or removed
        
        // If we still need to do any world-specific event processing:
        if let Some(event_bus) = &self.event_bus {
            // Process world-specific events
        }
    }
    
    // Check if terrain is fully initialized
    pub fn is_terrain_initialized(&self) -> bool {
        self.is_terrain_initialized
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
            // Use direct references to biome_manager and chunk_manager
            if let (Some(biome_mgr), Some(chunk_mgr)) = (&self.biome_manager, &self.chunk_manager) {
                // Update terrain based on world state
                println!("WorldStateManager: Generating world using terrain managers");
                
                // First make sure biome data is updated correctly
                {
                    let mut bm = biome_mgr.clone();
                    bm.bind_mut().set_seed(self.config.seed as u32);
                    // Other biome configuration...
                }
                
                // Then update the chunk manager
                {
                    let mut cm = chunk_mgr.clone();
                    // Generate chunks around origin point
                    cm.bind_mut().get_chunk(0, 0);
                    cm.bind_mut().get_chunk(-1, 0);
                    cm.bind_mut().get_chunk(0, -1);
                    cm.bind_mut().get_chunk(1, 0);
                    cm.bind_mut().get_chunk(0, 1);
                }
                
                println!("WorldStateManager: Generated world using terrain systems");
            } else {
                println!("WorldStateManager: Cannot generate world - terrain managers not available");
            }
        } else {
            println!("WorldStateManager: Cannot generate world - terrain not initialized");
        }
        
        // Increment world version
        self.current_version += 1;
        println!("WorldStateManager: World generation complete, version incremented to {}", self.current_version);
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
        
        // Get terrain data from direct managers if available
        let terrain_data: Vec<u8> = if let (Some(biome_mgr), Some(chunk_mgr)) = (&self.biome_manager, &self.chunk_manager) {
            // Serialize terrain data - implementation depends on your needs
            Vec::new() // Placeholder
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
            if !terrain_data.is_empty() {
                if let (Some(biome_mgr), Some(chunk_mgr)) = (&mut self.biome_manager, &mut self.chunk_manager) {
                    // Apply terrain data to the managers directly
                    // This would need to be implemented based on your serialization format
                    // For example:
                    // biome_mgr.bind_mut().deserialize_from(&terrain_data[0..biome_size]);
                    // chunk_mgr.bind_mut().deserialize_from(&terrain_data[biome_size..]);
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
        
        // Notify terrain system if initialized
        if let Some(biome_mgr) = &self.biome_manager {
            let mut bm = biome_mgr.clone();
            bm.bind_mut().set_seed(self.config.seed as u32);
            bm.bind_mut().set_world_dimensions(
                self.config.world_size.0 as f32, 
                self.config.world_size.1 as f32
            );
        }
    }

    pub fn get_biome_manager(&self) -> Option<Gd<BiomeManager>> {
        self.biome_manager.clone()
    }
    
    pub fn get_chunk_manager(&self) -> Option<Gd<ChunkManager>> {
        self.chunk_manager.clone()
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
            pending_init: self.pending_init,
            pending_seed: self.pending_seed,
            pending_size: self.pending_size,
            biome_manager: self.biome_manager.clone(),  // Clone the Gd pointers
            chunk_manager: self.chunk_manager.clone(),  // Clone the Gd pointers
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

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use serde::{Serialize, Deserialize};
use uuid::Uuid;

use crate::terrain::GenerationRules;

// World State Manager (Catalog/Blueprint)

// Keeps track of everything in the world
// Can take a "snapshot" of the entire world
// Can recreate that world exactly on another machine
// Manages complex interactions between entities


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
    pub generation_parameters: GenerationRules, // TODO: Change the Generation rules so that they make sense in terms of the desired terrain results.
}

// Comprehensive world state management
pub struct WorldStateManager {
    // Atomic, thread-safe world state
    entities: Arc<RwLock<HashMap<EntityId, Arc<dyn WorldEntity>>>>,
    
    // World configuration
    config: WorldStateConfig,
    
    // State versioning for synchronization
    current_version: u64,
    
    // Terrain generation system
    terrain_generator: TerrainGenerator,
}


impl WorldStateManager {
    // Create a new world state
    pub fn new(config: WorldStateConfig) -> Self {
        Self {
            entities: Arc::new(RwLock::new(HashMap::new())),
            config: config.clone(),
            current_version: 0,
            terrain_generator: TerrainGenerator::new(config),
        }
    }

    // Generate initial world state
    pub fn generate_initial_world(&mut self) {
        // Generate terrain
        let terrain = self.terrain_generator.generate_world();
        
        // Create initial world entities
        self.populate_initial_entities(terrain);
        
        // Increment world version
        self.current_version += 1;
    }

    fn populate_initial_entities(&mut self, terrain: WorldTerrain) {
        // This method creates the initial set of entities in the world
        // based on the generated terrain
        // TODO: implemetn this function

        // Example implementation:
        // 1. Create spawn points based on terrain features
        // 2. Add resource nodes based on biome distribution
        // 3. Place initial structures or landmarks

        // For now, we'll just log that entities are being populated
        println!("Populating world with initial entities for seed: {}", terrain.seed);
        
        // In a full implementation, you would:
        // - Add resource nodes
        // - Create spawn points
        // - Add environmental objects
        // - Set up initial NPCs if any
        
        // This would use functionality like:
        // let entity_id = Uuid::new_v4();
        // let entity = YourEntityType::new(entity_id, position, properties);
        // self.add_entity(Arc::new(entity));
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
        
        // Serialize entities and world state
        let serialized_entities: Vec<_> = entities
            .values()
            .map(|entity| entity.serialize())
            .collect();
        
        // Use bincode for efficient serialization
        bincode::serialize(&(self.current_version, serialized_entities))
            .expect("Failed to serialize world state")
    }

    // Deserialize and apply world state
    fn deserialize_world_state(&mut self, data: &[u8]) {
        // Deserialize world state
        let (version, serialized_entities): (u64, Vec<Vec<u8>>) = 
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
}

// Implement Clone for WorldStateManager
impl Clone for WorldStateManager {
    fn clone(&self) -> Self {
        // Create a new instance with the same configuration
        let mut cloned = Self {
            entities: Arc::new(RwLock::new(HashMap::new())),
            config: self.config.clone(),
            current_version: self.current_version,
            terrain_generator: self.terrain_generator.clone(),
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

// Terrain generation system
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

// Basic terrain representation
struct WorldTerrain {
    seed: u64,
    size: (u32, u32),
    // Additional terrain data
}

// Demonstration of usage
fn demonstrate_world_state_management() {
    // Create initial world configuration
    let world_config = WorldStateConfig {
        seed: 12345,
        world_size: (10000, 10000),
        generation_parameters: GenerationRules::default(),
    };

    // Create world state manager
    let mut world_state = WorldStateManager::new(world_config.clone());

    // Generate initial world
    world_state.generate_initial_world();

    // Serialize world state for network transmission
    let serialized_state = world_state.serialize_world_state();

    // Simulate receiving state on another machine
    let mut received_world_state = WorldStateManager::new(world_config);
    received_world_state.deserialize_world_state(&serialized_state);
}
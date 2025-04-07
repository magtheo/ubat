// File: src/core/initialization/world_initializer.rs

use std::sync::{Arc, Mutex};
use std::error::Error;
use std::fmt;

use crate::core::event_bus::EventBus;
use crate::core::config_manager::{ConfigurationManager, GameConfiguration};
use crate::core::world_manager::{WorldStateManager, WorldStateConfig};
use crate::initialization::world::TerrainInitializer;
use crate::networking::network_manager::{NetworkHandler, NetworkMode};

// Custom error type for world initialization
#[derive(Debug)]
pub enum WorldInitError {
    ConfigError(String),
    TerrainError(String),
    EntityError(String),
    WorldStateError(String),
    OtherError(String),
}

impl fmt::Display for WorldInitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            WorldInitError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            WorldInitError::TerrainError(msg) => write!(f, "Terrain error: {}", msg),
            WorldInitError::EntityError(msg) => write!(f, "Entity error: {}", msg),
            WorldInitError::WorldStateError(msg) => write!(f, "World state error: {}", msg),
            WorldInitError::OtherError(msg) => write!(f, "Other error: {}", msg),
        }
    }
}

impl Error for WorldInitError {}

// Convert string errors to WorldInitError
impl From<String> for WorldInitError {
    fn from(error: String) -> Self {
        WorldInitError::OtherError(error)
    }
}

pub struct WorldInitializer {
    // Core dependencies
    config_manager: Arc<Mutex<ConfigurationManager>>,
    event_bus: Arc<EventBus>,
    
    // Initialized systems
    world_manager: Option<Arc<Mutex<WorldStateManager>>>,
    terrain_initializer: Option<TerrainInitializer>,
    
    // Initialization state
    initialized: bool,
    terrain_initialized: bool,
    entity_initialized: bool,
}

impl WorldInitializer {
    pub fn new(
        config_manager: Arc<Mutex<ConfigurationManager>>, 
        event_bus: Arc<EventBus>
    ) -> Self {
        Self {
            config_manager,
            event_bus,
            world_manager: None,
            terrain_initializer: None,
            initialized: false,
            terrain_initialized: false,
            entity_initialized: false,
        }
    }
    
    // Main initialization method
    pub fn initialize(&mut self) -> Result<(), WorldInitError> {
        println!("WorldInitializer: Starting world initialization");
        
        // Phase 1: Initialize WorldStateManager
        self.initialize_world_manager()?;
        
        // Phase 2: Initialize terrain systems
        self.initialize_terrain_systems()?;
        
        // Phase 3: Initialize entity systems (placeholder for now)
        self.initialize_entity_systems()?;
        
        self.initialized = true;
        println!("WorldInitializer: World initialization complete");
        
        Ok(())
    }
    
    // Phase 1: Initialize the world state manager
    fn initialize_world_manager(&mut self) -> Result<(), WorldInitError> {
        println!("WorldInitializer: Initializing world state manager");
        
        // Get configuration
        let world_config = {
            let config_manager = self.config_manager.lock()
                .map_err(|_| WorldInitError::ConfigError("Failed to lock config manager".to_string()))?;
            
            let game_config = config_manager.get_config();
            
            // Convert from GameConfiguration to WorldStateConfig
            WorldStateConfig {
                seed: game_config.world_seed,
                world_size: (game_config.world_size.width, game_config.world_size.height),
                generation_parameters: game_config.generation_rules.clone(),
            }
        };
        
        // Create the world manager
        let world_manager = Arc::new(Mutex::new(
            WorldStateManager::new_with_dependencies(
                world_config,
                Some(self.event_bus.clone()),
                None // TerrainWorldIntegration will be created by WorldStateManager
            )
        ));
        
        // Basic initialization
        {
            let mut world_mgr = world_manager.lock()
                .map_err(|_| WorldInitError::WorldStateError("Failed to lock world manager".to_string()))?;
            
            world_mgr.initialize()
                .map_err(|e| WorldInitError::WorldStateError(e))?;
        }
        
        // Store reference
        self.world_manager = Some(world_manager);
        
        println!("WorldInitializer: World state manager initialized");
        Ok(())
    }
    
    // Phase 2: Initialize terrain systems
    fn initialize_terrain_systems(&mut self) -> Result<(), WorldInitError> {
        println!("WorldInitializer: Initializing terrain systems");
        
        // Create TerrainInitializer
        let mut terrain_init = TerrainInitializer::new();
        
        // Configure it based on world configuration
        let (seed, world_size) = {
            if let Some(world_manager) = &self.world_manager {
                let world_mgr = world_manager.lock()
                    .map_err(|_| WorldInitError::WorldStateError("Failed to lock world manager".to_string()))?;
                
                let config = world_mgr.get_config();
                (config.seed, config.world_size)
            } else {
                return Err(WorldInitError::TerrainError("World manager not initialized".to_string()));
            }
        };
        
        // Set up terrain initializer
        terrain_init.set_seed(seed as u32);
        terrain_init.set_world_dimensions(world_size.0 as f32, world_size.1 as f32);
        
        // Initialize terrain systems
        terrain_init.initialize_terrain_system()
            .map_err(|e| WorldInitError::TerrainError(e))?;
        
        // Connect terrain systems with world manager
        if let Some(world_manager) = &self.world_manager {
            let mut world_mgr = world_manager.lock()
                .map_err(|_| WorldInitError::WorldStateError("Failed to lock world manager".to_string()))?;
            
            // Get the initialized components
            let terrain_context = terrain_init.get_terrain_context();
            
            // Connect to world manager
            if let (Some(biome_mgr), Some(chunk_mgr)) = (terrain_context.biome_manager, terrain_context.chunk_manager) {
                // Initialize the world manager's terrain integration with these components
                world_mgr.initialize_terrain(biome_mgr, chunk_mgr)
                    .map_err(|e| WorldInitError::TerrainError(e))?;
            } else {
                return Err(WorldInitError::TerrainError("Failed to get terrain components".to_string()));
            }
        }
        
        self.terrain_initialized = true;
        self.terrain_initializer = Some(terrain_init);
        
        println!("WorldInitializer: Terrain systems initialized");
        Ok(())
    }
    
    // Phase 3: Initialize entity systems (placeholder)
    fn initialize_entity_systems(&mut self) -> Result<(), WorldInitError> {
        println!("WorldInitializer: Initializing entity systems");
        
        // Placeholder for entity system initialization
        // In a real implementation, you would:
        // 1. Initialize entity factory
        // 2. Initialize entity manager
        // 3. Create base entities if needed
        
        self.entity_initialized = true;
        println!("WorldInitializer: Entity systems initialized");
        Ok(())
    }
    
    // Getters for initialized components
    pub fn get_world_manager(&self) -> Option<Arc<Mutex<WorldStateManager>>> {
        self.world_manager.clone()
    }
    
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
    
    pub fn is_terrain_initialized(&self) -> bool {
        self.terrain_initialized
    }
    
    pub fn is_entity_initialized(&self) -> bool {
        self.entity_initialized
    }
}

// Ensure WorldInitializer is cleaned up after use
impl Drop for WorldInitializer {
    fn drop(&mut self) {
        println!("WorldInitializer: Dropping initializer");
        
        // Clean up terrain initializer if it was created
        if let Some(mut terrain_init) = self.terrain_initializer.take() {
            // Any cleanup needed for terrain initializer
            // Most Rust resources will be cleaned up automatically
        }
        
        // Note: We don't clean up the world_manager as it's an Arc that will be owned
        // by the SystemInitializer after initialization
    }
}
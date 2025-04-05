// File: src/terrain/world_integration.rs

use godot::prelude::*;
use std::sync::{Arc, Mutex};
use std::marker::PhantomPinned;

use crate::core::event_bus::EventBus;
use crate::core::world_manager::WorldStateManager;
use crate::core::config_manager::GameConfiguration;
use crate::terrain::chunk_manager::ChunkManager;
use crate::terrain::biome_manager::BiomeManager;


// Define a state enum for tracking initialization
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TerrainInitializationState {
    Uninitialized,
    ConfigLoaded,
    BiomeInitialized,
    ChunkManagerInitialized,
    Ready,
    Error,
}

// Thread-safe struct that doesn't store Godot objects directly
pub struct TerrainWorldIntegration {
    // Reference to the world manager
    world_manager: Arc<Mutex<WorldStateManager>>,
    
    // Current seed and dimensions - store these instead of Godot objects
    current_seed: u32,
    current_dimensions: (f32, f32),
    
    // Initialization state
    initialization_state: TerrainInitializationState,
    
    // Using PhantomData to maintain type association without storing objects
    _marker: PhantomPinned,
}

impl TerrainWorldIntegration {
    pub fn new(world_manager: Arc<Mutex<WorldStateManager>>) -> Self {
        Self {
            world_manager,
            current_seed: 0,
            current_dimensions: (0.0, 0.0),
            initialization_state: TerrainInitializationState::Uninitialized,
            _marker: PhantomPinned,
        }
    }
    
    // Initialize the terrain system - store configuration values, not Godot objects
    pub fn initialize_terrain(&mut self, biome_manager: Gd<BiomeManager>, 
            chunk_manager: Gd<ChunkManager>) -> Result<(), String> {
        println!("TerrainWorldIntegration: Initializing terrain system");

        // Set up initial state
        if let Ok(world_manager) = self.world_manager.lock() {
            let config = world_manager.get_config();
            self.current_seed = config.seed as u32;
            self.current_dimensions = (config.world_size.0 as f32, config.world_size.1 as f32);
            }

        // Configure the biome manager
        {
            let mut bm = biome_manager.clone();
            bm.bind_mut().set_seed(self.current_seed);
            bm.bind_mut().set_world_dimensions(
            self.current_dimensions.0,
            self.current_dimensions.1
            );
        }

        // Set up the chunk manager
        {
            let mut cm = chunk_manager.clone();
            cm.bind_mut().set_biome_manager(biome_manager.clone());
            cm.bind_mut().update_thread_safe_biome_data();
        }

        self.initialization_state = TerrainInitializationState::Ready;
        println!("TerrainWorldIntegration: Terrain system initialized successfully");
        Ok(())
    }

    
    // Update the system with new configuration values
    pub fn update(&mut self) {
        println!("TerrainWorldIntegration: Update called");
        
        // Update any system state here
        // This method doesn't use any Godot objects directly
    }
    
    // Connect to event bus for event-based updates
    pub fn connect_to_event_bus(&self, event_bus: Arc<EventBus>) {
        // Create a thread-safe handler that doesn't capture Godot objects
        let world_manager_clone = self.world_manager.clone();
        
        // This handler only deals with the WorldStateManager, not Godot objects
        let world_gen_handler = Arc::new(move |event: &crate::core::event_bus::WorldGeneratedEvent| {
            if let Ok(mut manager) = world_manager_clone.lock() {
                // Store the event parameters for later processing
                let seed = event.seed;
                let size = event.world_size;
                
                manager.set_pending_world_init(seed, size);
                println!("TerrainWorldIntegration: Received world generation event (seed: {})", seed);
            }
        });
        
        // Subscribe to the event bus
        event_bus.subscribe(world_gen_handler);
        println!("TerrainWorldIntegration: Connected to event bus");
    }
    
    // Process pending events by looking at the WorldStateManager's pending data
    pub fn process_pending_events(&mut self) {
        // Check if there are pending parameters in the world manager
        let (has_pending, seed, size) = {
            if let Ok(world_manager) = self.world_manager.lock() {
                world_manager.get_pending_world_init()
            } else {
                (false, 0, (0, 0))
            }
        };
        
        if has_pending {
            println!("TerrainWorldIntegration: Processing pending world initialization (seed: {})", seed);
            
            // Update our internal state
            self.current_seed = seed as u32;
            self.current_dimensions = (size.0 as f32, size.1 as f32);
            
            // Note: This only updates internal state
            // BiomeManager and ChunkManager would need to be updated elsewhere
            // (typically in a Godot _process method)
            
            // Clear the pending flag
            if let Ok(mut world_manager) = self.world_manager.lock() {
                world_manager.clear_pending_world_init();
            }
            
            println!("TerrainWorldIntegration: Processed pending initialization");
        }
    }
    
    // Get serializable terrain data for network transmission
    pub fn get_terrain_data(&self) -> Vec<u8> {
        // Serialize our current state
        bincode::serialize(&(self.current_seed, self.current_dimensions))
            .unwrap_or_else(|_| Vec::new())
    }
    
    // Apply terrain data from network
    pub fn apply_terrain_data(&mut self, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        
        // Try to deserialize the terrain data
        if let Ok((seed, dimensions)) = bincode::deserialize::<(u32, (f32, f32))>(data) {
            self.current_seed = seed;
            self.current_dimensions = dimensions;
            println!("TerrainWorldIntegration: Applied terrain data with seed {}", seed);
            
            // Note: BiomeManager and ChunkManager would need to be updated elsewhere
        }
    }
    
    // Get the current initialization state
    pub fn get_initialization_state(&self) -> TerrainInitializationState {
        self.initialization_state
    }
    
    // Get current seed (for display purposes)
    pub fn get_current_seed(&self) -> u32 {
        self.current_seed
    }
    
    // Get current dimensions (for display purposes)
    pub fn get_current_dimensions(&self) -> (f32, f32) {
        self.current_dimensions
    }
}
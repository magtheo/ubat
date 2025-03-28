// File: addons/ubat/src/terrain/world_integration.rs

use godot::prelude::*;
use std::sync::{Arc, Mutex};
use crate::core::event_bus::WorldGeneratedEvent;
use crate::core::EventBus;
use crate::terrain::{BiomeManager, ChunkManager, ChunkController};
use crate::core::world_manager::WorldStateManager;
use crate::core::config_manager::GameConfiguration;

pub struct TerrainWorldIntegration {
    biome_manager: Option<Gd<BiomeManager>>,
    chunk_manager: Option<Gd<ChunkManager>>,
    world_manager: Arc<Mutex<WorldStateManager>>,
}

impl TerrainWorldIntegration {
    pub fn new(world_manager: Arc<Mutex<WorldStateManager>>) -> Self {
        Self {
            biome_manager: None,
            chunk_manager: None,
            world_manager,
        }
    }
    
    // Initialize terrain from configuration
    pub fn initialize_terrain(&mut self, biome_manager: Gd<BiomeManager>, chunk_manager: Gd<ChunkManager>, config: &GameConfiguration) {
        // Store managers
        self.biome_manager = Some(biome_manager.clone());
        self.chunk_manager = Some(chunk_manager.clone());
        
        // Set world seed from configuration
        {
            let mut biome_mgr = biome_manager.clone();
            biome_mgr.bind_mut().set_seed(config.world_seed as u32);
            
            // Set world dimensions
            biome_mgr.bind_mut().set_world_dimensions(
                config.world_size.width as f32,
                config.world_size.height as f32
            );
        }
        
        // Configure chunk manager with the same seed
        if let Some(chunk_mgr) = &self.chunk_manager {
            let mut cm = chunk_mgr.clone();
            
            // Set render distance based on configuration
            // This could be a custom setting in your GameConfiguration
            cm.bind_mut().set_render_distance(8); // Default value
        }
        
        godot_print!("Terrain initialized with seed: {}", config.world_seed);
    }

    pub fn connect_to_event_bus(&self, event_bus: Arc<EventBus>) {
        // We can't safely pass Godot objects between threads
        // Instead, store the event parameters and process them in the main thread
        
        // First, create a signal handler that will be called on the main thread
        let world_manager_clone = self.world_manager.clone();
        
        // Define the handler
        let world_gen_handler = Arc::new(move |event: &WorldGeneratedEvent| {
            // Store the event data in a thread-safe way
            if let Ok(mut manager) = world_manager_clone.lock() {
                // Store the event parameters for later processing
                let seed = event.seed;
                let size = event.world_size;
                
                // For example: store in a special field that the main thread checks
                manager.set_pending_world_init(seed, size);
                
                godot_print!("WorldStateManager: Received world generation event with seed {}", seed);
            }
        });
        
        // Subscribe to world generation events
        event_bus.subscribe(world_gen_handler);
        godot_print!("TerrainWorldIntegration: Connected to event bus");
    }
    
    // Then, add a method to process these events in the main thread (called from _process)
    pub fn process_pending_events(&mut self) {
        // Process any events that were handled by the event bus
        
        if let Some(ref biome_mgr) = self.biome_manager {
            // Check if there's pending initialization parameters
            let (has_pending, seed, size) = {
                if let Ok(world_manager) = self.world_manager.lock() {
                    world_manager.get_pending_world_init()
                } else {
                    (false, 0, (0, 0))
                }
            };
            
            if has_pending {
                godot_print!("TerrainWorldIntegration: Processing pending world initialization");
                
                // Set biome manager seed
                let mut bm = biome_mgr.clone();
                bm.bind_mut().set_seed(seed as u32);
                bm.bind_mut().set_world_dimensions(
                    size.0 as f32,
                    size.1 as f32
                );
                
                // Notify ChunkManager
                if let Some(ref chunk_mgr) = self.chunk_manager {
                    let mut cm = chunk_mgr.clone();
                    cm.bind_mut().update_thread_safe_biome_data();
                }
                
                // Clear the pending flag
                if let Ok(mut world_manager) = self.world_manager.lock() {
                    world_manager.clear_pending_world_init();
                }
                
                godot_print!("TerrainWorldIntegration: World initialization complete");
            }
        }
    }
    
    // Update terrain based on world state
    pub fn update(&self) {
        // Perform any needed updates based on world state
    }
    
    // Get serializable terrain data for network synchronization
    pub fn get_terrain_data(&self) -> Vec<u8> {
        // Serialize terrain state
        // For now we just need the seed and dimensions
        if let Some(biome_mgr) = &self.biome_manager {
            let seed = biome_mgr.bind().get_seed();
            // In a real implementation, serialize properly
            vec![seed as u8]
        } else {
            vec![]
        }
    }
    
    // Update terrain from serialized data
    pub fn apply_terrain_data(&mut self, data: &[u8]) {
        // Apply serialized terrain state
        if data.is_empty() {
            return;
        }
        
        if let Some(biome_mgr) = &self.biome_manager {
            let mut bm = biome_mgr.clone();
            // In a real implementation, deserialize properly
            let seed = data[0] as u32;
            bm.bind_mut().set_seed(seed);
        }
    }
}
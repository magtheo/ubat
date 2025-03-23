// File: addons/ubat/src/terrain/world_integration.rs

use godot::prelude::*;
use std::sync::{Arc, Mutex};
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
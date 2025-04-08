// File: terrain_initializer.rs
use godot::prelude::*;
use godot::classes::Node;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::terrain::biome_manager::ThreadSafeBiomeData;
use crate::initialization::world::terrainInitState::{TerrainInitializationTiming, TerrainInitializationState};
use crate::terrain::BiomeManager;
use crate::terrain::ChunkManager;
use crate::terrain::ChunkController;
use crate::utils::error_logger::{ErrorLogger, ErrorSeverity};
use crate::core::event_bus::EventBus;


// TerrainSystemContext stores references to initialized terrain components
pub struct TerrainSystemContext {
    pub biome_manager: Option<Gd<BiomeManager>>,
    pub chunk_manager: Option<Gd<ChunkManager>>,
    pub thread_safe_biome_data: Option<Arc<ThreadSafeBiomeData>>,
}

pub struct TerrainInitializer {

    biome_manager: Option<Gd<BiomeManager>>,
    chunk_manager: Option<Gd<ChunkManager>>,
    chunk_controller: Option<Gd<ChunkController>>,

    timing: TerrainInitializationTiming,
    error_logger: Option<Arc<ErrorLogger>>,
    event_bus: Option<Arc<EventBus>>,

    world_width: f32,

    world_height: f32,

    seed: u32,

    render_distance: i32,
    
    initialized: bool,
}



impl TerrainInitializer {
    pub fn new() -> Self {
        Self {
            biome_manager: None,
            chunk_manager: None,
            chunk_controller: None,
            event_bus: None,
            timing: TerrainInitializationTiming::new(),
            error_logger: Some(Arc::new(ErrorLogger::new(100))),
            world_width: 10000.0,
            world_height: 10000.0,
            seed: 12345,
            render_distance: 8,
            initialized: true,
        }
    }

    // This is the main method to initialize the terrain system
    pub fn initialize_terrain_system(&mut self) -> Result<(), String> {
        godot_print!("TerrainInitializer: TerrainInitializer starting initialization...");
        
        // Create BiomeManager
        let mut biome_manager = BiomeManager::new_alloc();

        // Configure BiomeManager before initialization
        {
            let mut biome_mgr_mut = biome_manager.bind_mut();
            biome_mgr_mut.set_world_dimensions(self.world_width, self.world_height);
            biome_mgr_mut.set_seed(self.seed);
            
            // Now explicitly initialize the BiomeManager
            if !biome_mgr_mut.initialize_explicitly() {
                return Err("Failed to initialize BiomeManager".to_string());
            }
        }

        // We're not adding to scene tree in this pure Rust approach

        // Store reference
        self.biome_manager = Some(biome_manager);

        // Update initialization state
        self.timing.update_state(TerrainInitializationState::BiomeInitialized);

        // Create ChunkManager
        let mut chunk_manager = ChunkManager::new_alloc();

        // Connect the ChunkManager to BiomeManager
        if let Some(biome_mgr) = &self.biome_manager {
            let mut chunk_mgr_mut = chunk_manager.bind_mut();
            chunk_mgr_mut.set_biome_manager(biome_mgr.clone());
            chunk_mgr_mut.set_render_distance(self.render_distance);
        }

        // Store reference
        self.chunk_manager = Some(chunk_manager);

        // Update initialization state
        self.timing.update_state(TerrainInitializationState::ChunkManagerInitialized);

        // Create ChunkController
        let chunk_controller = ChunkController::new_alloc();
        self.chunk_controller = Some(chunk_controller);

        // Update initialization state
        self.timing.update_state(TerrainInitializationState::Ready);

        godot_print!("TerrainInitializer: Terrain system fully initialized in Rust");
        Ok(())
    }

    // Get the terrain context (components needed by the world manager)
    pub fn get_terrain_context(&self) -> TerrainSystemContext {
        TerrainSystemContext {
            biome_manager: self.biome_manager.clone(),
            chunk_manager: self.chunk_manager.clone(),
            thread_safe_biome_data: if let Some(biome_mgr) = &self.biome_manager {
                Some(Arc::new(ThreadSafeBiomeData::from_biome_manager(&biome_mgr.bind())))
            } else {
                None
            },
        }
    }

    pub fn get_initialization_status(&self) -> Dictionary {
        let mut result = Dictionary::new();

        // Get status of each component
        let biome_initialized = self.biome_manager.is_some() && 
            self.biome_manager.as_ref().unwrap().bind().is_fully_initialized();

        let chunk_manager_initialized = self.chunk_manager.is_some() && 
            self.chunk_manager.as_ref().unwrap().bind().is_initialized();

        let controller_initialized = self.chunk_controller.is_some();

        result.insert("biome_initialized", biome_initialized);
        result.insert("chunk_manager_initialized", chunk_manager_initialized);
        result.insert("controller_initialized", controller_initialized);
        result.insert("fully_initialized", biome_initialized && chunk_manager_initialized && controller_initialized);

        result
    }
    
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
    
    // Configuration setters
    pub fn set_world_dimensions(&mut self, width: f32, height: f32) {
        self.world_width = width;
        self.world_height = height;
    }
    
    pub fn set_seed(&mut self, seed: u32) {
        self.seed = seed;
    }
    
    pub fn set_render_distance(&mut self, distance: i32) {
        self.render_distance = distance;
    }
}
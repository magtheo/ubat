// src/terrain/terrain_config.rs

use godot::prelude::*; // Use godot::prelude
use std::sync::{Arc, RwLock};
use num_cpus;
use crate::config::global_config; // Import the global config access module
use crate::config::config_manager::TerrainInitialConfigData; // Import the struct holding initial data
use once_cell::sync::OnceCell;

// --- TerrainConfig Struct (Holds RUNTIME values) ---
#[derive(Clone, Debug)] // Added Clone, Debug
pub struct TerrainConfig {
    // Thread management
    pub max_threads: usize,
    pub chunk_size: u32,

    // Generation settings
    pub blend_distance: f32,
    pub use_parallel_processing: bool,

    // Memory management
    pub chunk_cache_size: usize,

    // Performance tuning
    pub chunks_per_frame: usize,

    // Render distance (might be used by chunk controller/manager at runtime)
    pub render_distance: i32,
    pub amplification: f64,
    pub mesh_updates_per_frame: usize, 
}

// Default implementation for TerrainConfig (RUNTIME defaults, used if init fails)
// This might not be strictly needed if init always succeeds or panics
impl Default for TerrainConfig {
    fn default() -> Self {
        let cpu_count = num_cpus::get();
        godot_warn!("Creating default RUNTIME TerrainConfig. Should have been initialized from global config.");
        TerrainConfig {
            max_threads: std::cmp::max(1, cpu_count.saturating_sub(1)),
            chunk_size: 32,
            blend_distance: 800.0,
            use_parallel_processing: true,
            chunk_cache_size: 400,
            chunks_per_frame: 4,
            render_distance: 4,
            amplification: 1.0,
            mesh_updates_per_frame: 4, 
        }
    }
}


// --- TerrainConfigManager ---
// Manages the RUNTIME TerrainConfig singleton

// --- Use OnceCell for the singleton ---
static RUNTIME_TERRAIN_CONFIG: OnceCell<Arc<RwLock<TerrainConfig>>> = OnceCell::new();

// --- Internal init function for TerrainConfigManager ---
fn internal_init_terrain_config() -> Arc<RwLock<TerrainConfig>> {
    godot_print!("Attempting to initialize runtime TerrainConfig lazily...");

    // Get initial data from the (also lazily initialized) global config
    let initial_data: TerrainInitialConfigData = global_config::get_terrain_config_data();
    godot_print!("Obtained initial terrain data from global config: {:?}", initial_data);

    // Create the runtime TerrainConfig struct using the initial data
    let runtime_config = TerrainConfig {
        max_threads: initial_data.max_threads,
        chunk_size: initial_data.chunk_size,
        blend_distance: initial_data.blend_distance,
        use_parallel_processing: initial_data.use_parallel_processing,
        chunk_cache_size: initial_data.chunk_cache_size,
        chunks_per_frame: initial_data.chunks_per_frame,
        render_distance: initial_data.render_distance,
        amplification: initial_data.amplification,
        mesh_updates_per_frame: initial_data.mesh_updates_per_frame,
    };
    godot_print!("Created runtime TerrainConfig: {:?}", runtime_config);

    Arc::new(RwLock::new(runtime_config))
}

pub struct TerrainConfigManager; // Make it a ZST as it only has static methods

impl TerrainConfigManager {

    /// Gets a static reference to the runtime terrain configuration.
    /// Initializes using global config values on the first call if necessary.
    pub fn get_config() -> &'static Arc<RwLock<TerrainConfig>> {
        RUNTIME_TERRAIN_CONFIG.get_or_init(internal_init_terrain_config)
    }

    // If you need to change terrain config AFTER init, you'd need to
    // re-introduce an update mechanism here, potentially triggered by
    // game events or specific commands, but not a Godot node.
}

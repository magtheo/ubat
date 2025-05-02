// Export all components from the terrain module

pub mod chunk_manager;
pub mod chunk_controller;
pub mod generation_utils;

pub mod terrain_config;

pub mod section;
pub mod noise;


// Re-export main types for easier access
pub use chunk_manager::ChunkManager;
pub use chunk_controller::ChunkController;

pub use terrain_config::TerrainConfig;
// pub use generation_rules::GenerationRules;

// Component descriptions:

// ChunkManager

// Primary Role: Handle the lifecycle of chunks and coordinate generation
// Functionality:
// Coordinates chunk loading, generation, and unloading
// Uses BiomeManager to determine biomes for generated chunks
// Manages thread-safe access to chunk data
// Provides the public API for the terrain system



// ChunkController

// Primary Role: Interface between Godot and the terrain system
// Functionality:
// Handles player movement and triggers updates
// Manages visualization of chunks in the game world
// Provides debug tools and statistics
// Connects the components together in the Godot scene tree
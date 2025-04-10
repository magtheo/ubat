// Export all components from the terrain module

pub mod chunk_manager;
pub mod chunk_controller;
pub mod chunk_storage;
pub mod biome_manager;
pub mod thread_pool;

pub mod generation_rules;


// Re-export main types for easier access
pub use chunk_manager::ChunkManager;
pub use chunk_controller::ChunkController;
pub use chunk_storage::ChunkStorage;
pub use biome_manager::BiomeManager;

pub use thread_pool::ThreadPool;

pub use generation_rules::GenerationRules;

// Component descriptions:

// BiomeManager

// Primary Role: Determine which biome exists at any world coordinate
// Functionality:
// Reads section information from a color-based biome mask image
// Maps colors to section IDs
// Maintains Voronoi points for each section with their associated biomes
// Handles biome blending at boundaries using noise and distance fields
// Provides a fast cache system for performance optimization


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
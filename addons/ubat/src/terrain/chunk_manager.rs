use godot::prelude::*;
use godot::classes::Node;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Serialize, Deserialize};

use crate::terrain::BiomeManager;
use crate::terrain::biome_manager::ThreadSafeBiomeData;

use crate::terrain::ChunkStorage;
use crate::terrain::ThreadPool;

// Constants for chunk management
const CHUNK_SIZE: u32 = 32; // Size of a chunk in world units
const UNLOAD_TIMEOUT: Duration = Duration::from_secs(30); // Time before unloading inactive chunks

// Unique identifier for a chunk based on its position
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkPosition {
    pub x: i32,
    pub z: i32,
}

// Enum to represent the current state of a chunk
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkState {
    NotLoaded,
    Generating,
    Loading,
    Active,
    Inactive,
    Unloading,
}

// Data structure for a chunk
pub struct Chunk {
    pub position: ChunkPosition,
    pub state: ChunkState,
    pub last_accessed: Instant,
    pub heightmap: Vec<f32>,
    pub biome_ids: Vec<u8>,
    // Add other chunk data as needed (entities, structures, etc.)
}

impl Chunk {
    pub fn new(position: ChunkPosition) -> Self {
        Chunk {
            position,
            state: ChunkState::NotLoaded,
            last_accessed: Instant::now(),
            heightmap: vec![0.0; (CHUNK_SIZE * CHUNK_SIZE) as usize],
            biome_ids: vec![0; (CHUNK_SIZE * CHUNK_SIZE) as usize],
        }
    }
}

// ChunkManager class with Godot integration
#[derive(GodotClass)]
#[class(base=Node)]
pub struct ChunkManager {
    #[base]
    base: Base<Node>,
    
    // Core chunk management data
    chunks: Arc<Mutex<HashMap<ChunkPosition, Arc<Mutex<Chunk>>>>>,
    thread_pool: Arc<ThreadPool>,
    biome_manager: Option<Gd<BiomeManager>>,
    thread_safe_biome_data: Option<Arc<ThreadSafeBiomeData>>,
    storage: Arc<ChunkStorage>,
    render_distance: i32,
}

#[godot_api]
impl INode for ChunkManager {
    fn init(base: Base<Node>) -> Self {
        // Initialize storage and thread pool
        let storage = Arc::new(ChunkStorage::new("user://terrain_data"));
        let thread_pool = Arc::new(ThreadPool::new(4)); // 4 worker threads
        
        ChunkManager {
            base,
            chunks: Arc::new(Mutex::new(HashMap::new())),
            thread_pool,
            biome_manager: None,
            thread_safe_biome_data: None,
            storage,
            render_distance: 8, // Default render distance
        }
    }
    
    fn ready(&mut self) {
        let start_time = std::time::Instant::now();
        godot_print!("TERRAIN: ChunkManager initializing...");
        
        // Try to find BiomeManager in the scene tree
        let parent = self.base().get_parent();
        if let Some(parent) = parent {
            // Try to find BiomeManager as a sibling node
            let biome_manager = parent.get_node_as::<BiomeManager>("BiomeManager");
            
            // Time the thread-safe biome data creation
            let biome_data_start = std::time::Instant::now();
            godot_print!("TERRAIN: Creating thread-safe biome data...");
            
            // Create thread-safe biome data first
            self.thread_safe_biome_data = Some(Arc::new(
                ThreadSafeBiomeData::from_biome_manager(&biome_manager.bind())
            ));
            
            godot_print!("TERRAIN: Thread-safe biome data created in {}ms", 
                         biome_data_start.elapsed().as_millis());
            
            // Then set the biome_manager field (clone to avoid move)
            self.biome_manager = Some(biome_manager.clone());
            
            godot_print!("TERRAIN: ChunkManager successfully connected to BiomeManager");
        } else {
            godot_error!("TERRAIN: ChunkManager could not find parent node");
        }

        // Initialize thread pool configuration
        let thread_pool_start = std::time::Instant::now();
        godot_print!("TERRAIN: Configuring chunk generation thread pool...");
        
        // The thread pool is already initialized in init() with 4 threads
        
        godot_print!("TERRAIN: Thread pool configured in {}ms", 
                     thread_pool_start.elapsed().as_millis());
        
        // Log total initialization time
        godot_print!("TERRAIN: ChunkManager initialization completed in {}ms", 
                     start_time.elapsed().as_millis());
    }
}

#[godot_api]
impl ChunkManager {
    #[func]
    pub fn is_initialized(&self) -> bool {
        self.thread_safe_biome_data.is_some()
    }

    // Get a chunk by position, load if necessary
    #[func]
    pub fn get_chunk(&self, position_x: i32, position_z: i32) -> bool {
        let timer = std::time::Instant::now();
        let position = ChunkPosition { x: position_x, z: position_z };
        
        // Log for debugging on first access
        let is_debug = position_x == 0 && position_z == 0;
        if is_debug {
            godot_print!("TERRAIN: Getting initial chunk at position ({}, {})", position_x, position_z);
        }
        
        let chunks_result = self.chunks.lock();
        
        let mut chunks = match chunks_result {
            Ok(chunks) => chunks,
            Err(e) => {
                godot_error!("TERRAIN: Failed to lock chunks: {}", e);
                return false;
            }
        };
        
        // Timer for lock acquisition
        if is_debug {
            godot_print!("TERRAIN: Acquired chunks lock in {}μs", timer.elapsed().as_micros());
        }
        
        if let Some(chunk) = chunks.get(&position) {
            let update_timer = std::time::Instant::now();
            
            // Update last accessed time
            if let Ok(mut chunk_guard) = chunk.lock() {
                chunk_guard.last_accessed = Instant::now();
                
                // If inactive, mark as active
                if chunk_guard.state == ChunkState::Inactive {
                    chunk_guard.state = ChunkState::Active;
                }
                
                if is_debug {
                    godot_print!("TERRAIN: Chunk found, updated state in {}μs", 
                              update_timer.elapsed().as_micros());
                }
            }
            
            return true;
        }
        
        if is_debug {
            godot_print!("TERRAIN: Chunk not found, starting generation/loading");
        }
        
        // Chunk not in memory, start loading process
        let new_chunk = Arc::new(Mutex::new(Chunk::new(position)));
        chunks.insert(position, Arc::clone(&new_chunk));
        
        // Check if this chunk exists in storage
        let chunks_clone = Arc::clone(&self.chunks);
        let storage_clone = Arc::clone(&self.storage);
        let new_chunk_clone = Arc::clone(&new_chunk);
        let biome_manager = self.biome_manager.clone();
        
        // Pass thread-safe biome data to thread
        let thread_safe_biome = self.thread_safe_biome_data.as_ref().map(Arc::clone);
        
        // Track if this is a special position for debugging
        let debug_chunk = is_debug;
    
        self.thread_pool.execute(move || {
            let thread_timer = std::time::Instant::now();
            
            if debug_chunk {
                godot_print!("TERRAIN: Thread started for chunk generation at ({}, {})", 
                          position.x, position.z);
            }
            
            if storage_clone.chunk_exists(position) {
                // Chunk exists in storage, load it
                if let Ok(mut chunk_guard) = new_chunk_clone.lock() {
                    chunk_guard.state = ChunkState::Loading;
                }
                
                let load_timer = std::time::Instant::now();
                if let Some(loaded_data) = storage_clone.load_chunk(position) {
                    if let Ok(mut chunk_guard) = new_chunk_clone.lock() {
                        // Update chunk with loaded data
                        chunk_guard.heightmap = loaded_data.heightmap;
                        chunk_guard.biome_ids = loaded_data.biome_ids;
                        chunk_guard.state = ChunkState::Active;
                        
                        if debug_chunk {
                            godot_print!("TERRAIN: Loaded chunk from storage in {}ms", 
                                      load_timer.elapsed().as_millis());
                        }
                    }
                }
            } else {
                // Chunk doesn't exist, generate it
                let gen_timer = std::time::Instant::now();
                Self::generate_chunk(new_chunk_clone, position, storage_clone, thread_safe_biome);
                
                if debug_chunk {
                    godot_print!("TERRAIN: Generated new chunk in {}ms", 
                              gen_timer.elapsed().as_millis());
                }
            }
            
            if debug_chunk {
                godot_print!("TERRAIN: Total thread execution time for chunk: {}ms", 
                          thread_timer.elapsed().as_millis());
            }
        });
        
        if is_debug {
            godot_print!("TERRAIN: Total get_chunk operation took {}ms", 
                      timer.elapsed().as_millis());
        }
        
        true
    }
    
    // Generate a new chunk
    fn generate_chunk(
        chunk: Arc<Mutex<Chunk>>, 
        position: ChunkPosition,
        storage: Arc<ChunkStorage>,
        thread_safe_biome: Option<Arc<ThreadSafeBiomeData>>
    ) {
        let is_debug = position.x == 0 && position.z == 0;
        let gen_timer = std::time::Instant::now();
        
        if is_debug {
            godot_print!("TERRAIN: Starting detailed chunk generation for ({}, {})", position.x, position.z);
        }

        // Generate heightmap
        let mut heightmap = vec![0.0; (CHUNK_SIZE * CHUNK_SIZE) as usize];
        let mut biome_ids = vec![0; (CHUNK_SIZE * CHUNK_SIZE) as usize];
        
        let heightmap_timer = std::time::Instant::now();
        
        // Process chunk in rows for better cache locality
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let world_x = position.x as f32 * CHUNK_SIZE as f32 + x as f32;
                let world_z = position.z as f32 * CHUNK_SIZE as f32 + z as f32;
                
                // Use thread-safe biome data instead of biome_manager
                let biome_id = if let Some(ref biome_data) = thread_safe_biome {
                    biome_data.get_biome_id(world_x, world_z)
                } else {
                    // Default generation if no BiomeManager
                    ((world_x.cos() * 0.5 + world_z.sin() * 0.5) * 2.0) as u8
                };
            
                
                 // Simple example noise function (should be enhanced for specific biomes)
                let height = (world_x.cos() * 0.5 + world_z.sin() * 0.5) * 10.0;
                let idx = (z * CHUNK_SIZE + x) as usize;

                heightmap[idx] = height;
                biome_ids[idx] = biome_id;
            }
        }
        
        // Apply height blending at biome boundaries
        if let Some(ref biome_data) = thread_safe_biome {
            Self::blend_heights(&mut heightmap, &biome_ids, CHUNK_SIZE, biome_data.blend_distance);
        }
        
        if is_debug {
            godot_print!("TERRAIN: Heightmap and biome generation took {}ms", 
                      heightmap_timer.elapsed().as_millis());
        }
        
        // Clone data before potential move
        let saved_heightmap = heightmap.clone();
        let saved_biome_ids = biome_ids.clone();
        
        // Update chunk data
        let update_timer = std::time::Instant::now();
        if let Ok(mut chunk_guard) = chunk.lock() {
            chunk_guard.heightmap = heightmap;
            chunk_guard.biome_ids = biome_ids;
            chunk_guard.state = ChunkState::Active;
            
            if is_debug {
                godot_print!("TERRAIN: Chunk data update took {}μs", 
                          update_timer.elapsed().as_micros());
            }
        }
        
        // Save to storage
        let storage_timer = std::time::Instant::now();
        storage.save_chunk(position, &saved_heightmap, &saved_biome_ids);
        
        if is_debug {
            godot_print!("TERRAIN: Chunk storage save took {}ms", 
                      storage_timer.elapsed().as_millis());
            godot_print!("TERRAIN: Complete chunk generation process took {}ms", 
                      gen_timer.elapsed().as_millis());
        }
    }

    fn blend_heights(heightmap: &mut [f32], biome_ids: &[u8], chunk_size: u32, blend_radius: i32) {
            // Create a copy of the original heightmap to read from
            let original_heights = heightmap.to_vec();
      
            // Process each vertex
            for z in 0..chunk_size {
                for x in 0..chunk_size {
                    let idx = (z * chunk_size + x) as usize;
                    let current_biome = biome_ids[idx];
      
                    // Check if this is a boundary vertex (has neighbors with different biome IDs)
                    let mut is_boundary = false;
                    for dz in -1..=1 {
                        for dx in -1..=1 {
                            if dx == 0 && dz == 0 {
                                continue; // Skip self
                            }
      
                            let nx = x as i32 + dx;
                            let nz = z as i32 + dz;
      
                            // Check if neighbor is within chunk bounds
                            if nx >= 0 && nx < chunk_size as i32 && nz >= 0 && nz < chunk_size as i32 {
                                let nidx = (nz as u32 * chunk_size + nx as u32) as usize;
                                if biome_ids[nidx] != current_biome {
                                    is_boundary = true;
                                    break;
                                }
                            }
                        }
                        if is_boundary {
                            break;
                        }
                    }
      
                    // If this is a boundary vertex, blend the heights
                    if is_boundary {
                        let mut total_weight = 1.0; // Weight for the current vertex
                        let mut weighted_height = original_heights[idx];
      
                        // Check all neighbors within blend_radius
                        for dz in -blend_radius..=blend_radius {
                            for dx in -blend_radius..=blend_radius {
                                if dx == 0 && dz == 0 {
                                    continue; // Skip self
                                }
      
                                let nx = x as i32 + dx;
                                let nz = z as i32 + dz;
      
                                // Check if neighbor is within chunk bounds
                                if nx >= 0 && nx < chunk_size as i32 && nz >= 0 && nz < chunk_size as i32 {
                                    let nidx = (nz as u32 * chunk_size + nx as u32) as usize;
      
                                    // Calculate distance-based weight (further = less influence)
                                    let distance = ((dx * dx + dz * dz) as f32).sqrt();
                                    let weight = 1.0 / (1.0 + distance);
      
                                    total_weight += weight;
                                    weighted_height += original_heights[nidx] * weight;
                                }
                            }
                        }
      
                        // Apply weighted average
                        if total_weight > 0.0 {
                            heightmap[idx] = weighted_height / total_weight;
                        }
                    }
                }
            }
        }
      
    
    // Update chunks based on player position
    #[func]
    pub fn update(&self, player_x: f32, _player_y: f32, player_z: f32) {
        let player_chunk_x = (player_x / CHUNK_SIZE as f32).floor() as i32;
        let player_chunk_z = (player_z / CHUNK_SIZE as f32).floor() as i32;
        
        // Determine chunks to load
        let mut chunks_to_load = Vec::new();
        for x in (player_chunk_x - self.render_distance)..=(player_chunk_x + self.render_distance) {
            for z in (player_chunk_z - self.render_distance)..=(player_chunk_z + self.render_distance) {
                chunks_to_load.push(ChunkPosition { x, z });
            }
        }
        
        // Load nearby chunks
        for position in chunks_to_load {
            self.get_chunk(position.x, position.z);
        }
        
        // Find chunks to unload
        let mut chunks_to_unload = Vec::new();
        let chunks = self.chunks.lock().unwrap();
        
        for (&position, chunk) in chunks.iter() {
            let dx = position.x - player_chunk_x;
            let dz = position.z - player_chunk_z;
            let distance_squared = dx * dx + dz * dz;
            
            if distance_squared > (self.render_distance + 2) * (self.render_distance + 2) {
                // Outside render distance plus buffer
                chunks_to_unload.push(position);
            } else if let Ok(chunk_guard) = chunk.lock() {
                // Check if chunk has been inactive for too long
                if chunk_guard.state == ChunkState::Inactive && 
                   chunk_guard.last_accessed.elapsed() > UNLOAD_TIMEOUT {
                    chunks_to_unload.push(position);
                }
            }
        }
        
        // Unload distant chunks
        drop(chunks); // Release the lock before unloading
        for position in chunks_to_unload {
            self.unload_chunk(position.x, position.z);
        }
    }
    
    // Mark a chunk as inactive
    #[func]
    pub fn deactivate_chunk(&self, position_x: i32, position_z: i32) {
        let position = ChunkPosition { x: position_x, z: position_z };
        if let Some(chunk) = self.chunks.lock().unwrap().get(&position) {
            if let Ok(mut chunk_guard) = chunk.lock() {
                if chunk_guard.state == ChunkState::Active {
                    chunk_guard.state = ChunkState::Inactive;
                }
            }
        }
    }
    
    // Unload a chunk from memory
    #[func]
    pub fn unload_chunk(&self, position_x: i32, position_z: i32) {
        let position = ChunkPosition { x: position_x, z: position_z };
        let mut chunks = self.chunks.lock().unwrap();
        
        if let Some(chunk) = chunks.get(&position) {
            let can_unload = if let Ok(chunk_guard) = chunk.lock() {
                chunk_guard.state != ChunkState::Generating && 
                chunk_guard.state != ChunkState::Loading
            } else {
                false
            };
            
            if can_unload {
                // Set state to unloading
                if let Ok(mut chunk_guard) = chunk.lock() {
                    chunk_guard.state = ChunkState::Unloading;
                }
                
                // Clone necessary references for the thread
                let chunks_clone = Arc::clone(&self.chunks);
                let storage_clone = Arc::clone(&self.storage);
                let chunk_clone = Arc::clone(chunk);
                
                self.thread_pool.execute(move || {
                    // Ensure data is saved
                    if let Ok(chunk_guard) = chunk_clone.lock() {
                        storage_clone.save_chunk(
                            position,
                            &chunk_guard.heightmap,
                            &chunk_guard.biome_ids
                        );
                    }
                    
                    // Remove from memory
                    chunks_clone.lock().unwrap().remove(&position);
                });
            }
        }
    }
    
    // Check if a specific chunk is loaded
    #[func]
    pub fn is_chunk_loaded(&self, position_x: i32, position_z: i32) -> bool {
        let position = ChunkPosition { x: position_x, z: position_z };
        let chunks = self.chunks.lock().unwrap();
        chunks.contains_key(&position)
    }
    
    // Check if a specific chunk is ready (fully generated/loaded)
    #[func]
    pub fn is_chunk_ready(&self, position_x: i32, position_z: i32) -> bool {
        let position = ChunkPosition { x: position_x, z: position_z };
        
        if let Some(chunk) = self.chunks.lock().unwrap().get(&position) {
            if let Ok(chunk_guard) = chunk.lock() {
                return chunk_guard.state == ChunkState::Active;
            }
        }
        
        false
    }

    #[func]
    pub fn update_thread_safe_biome_data(&mut self) {
        if let Some(ref biome_mgr) = self.biome_manager {
            self.thread_safe_biome_data = Some(Arc::new(
                ThreadSafeBiomeData::from_biome_manager(&biome_mgr.bind())
            ));
        }
    }
    
    // Get the heightmap data for a chunk
    #[func]
    pub fn get_chunk_heightmap(&self, position_x: i32, position_z: i32) -> PackedFloat32Array {
        let position = ChunkPosition { x: position_x, z: position_z };
        
        if let Some(chunk) = self.chunks.lock().unwrap().get(&position) {
            if let Ok(chunk_guard) = chunk.lock() {
                if chunk_guard.state == ChunkState::Active {
                    return PackedFloat32Array::from(&chunk_guard.heightmap[..]);
                }
            }
        }
        
        // Return empty array if chunk not available
        PackedFloat32Array::new()
    }
    
    // Get the biome data for a chunk
    #[func]
    pub fn get_chunk_biomes(&self, position_x: i32, position_z: i32) -> PackedInt32Array {
        let position = ChunkPosition { x: position_x, z: position_z };
        
        if let Some(chunk) = self.chunks.lock().unwrap().get(&position) {
            if let Ok(chunk_guard) = chunk.lock() {
                if chunk_guard.state == ChunkState::Active {
                    // Convert u8 to i32 for Godot compatibility
                    let biomes_i32: Vec<i32> = chunk_guard.biome_ids
                        .iter()
                        .map(|&id| id as i32)
                        .collect();
                    
                    return PackedInt32Array::from(&biomes_i32[..]);
                }
            }
        }
        
        // Return empty array if chunk not available
        PackedInt32Array::new()
    }
    
    // Get count of all chunks managed
    #[func]
    pub fn get_chunk_count(&self) -> i32 {
        self.chunks.lock().unwrap().len() as i32
    }
    
    // Set render distance
    #[func]
    pub fn set_render_distance(&mut self, distance: i32) {
        self.render_distance = distance.max(1).min(32); // Clamp between 1 and 32
    }
    
    // Get render distance
    #[func]
    pub fn get_render_distance(&self) -> i32 {
        self.render_distance
    }
    
    // Set BiomeManager reference
    #[func]
    pub fn set_biome_manager(&mut self, biome_manager: Gd<BiomeManager>) {
        self.biome_manager = Some(biome_manager);
    }
}
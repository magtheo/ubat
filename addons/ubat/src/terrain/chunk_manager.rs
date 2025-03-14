use godot::prelude::*;
use godot::classes::Node;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::terrain::BiomeManager;
use crate::terrain::ChunkStorage;
use crate::terrain::ThreadPool;

// Constants for chunk management
const CHUNK_SIZE: u32 = 32; // Size of a chunk in world units
const UNLOAD_TIMEOUT: Duration = Duration::from_secs(30); // Time before unloading inactive chunks

// Unique identifier for a chunk based on its position
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
            storage,
            render_distance: 8, // Default render distance
        }
    }
    
    fn ready(&mut self) {
        // Try to find BiomeManager in the scene tree
        let parent = self.base().get_parent();
        if let Some(parent) = parent {
            // Try to find BiomeManager as a sibling node
            let biome_manager = parent.try_get_node_as::<BiomeManager>("BiomeManager");
            if let Ok(manager) = biome_manager {
                self.biome_manager = Some(manager);
                godot_print!("ChunkManager found BiomeManager node");
            } else {
                godot_error!("ChunkManager couldn't find BiomeManager node");
            }
        }
    }
}

#[godot_api]
impl ChunkManager {
    // Get a chunk by position, load if necessary
    #[func]
    pub fn get_chunk(&self, position_x: i32, position_z: i32) -> bool {
        let position = ChunkPosition { x: position_x, z: position_z };
        let mut chunks = self.chunks.lock().unwrap();
        
        if let Some(chunk) = chunks.get(&position) {
            // Update last accessed time
            if let Ok(mut chunk_guard) = chunk.lock() {
                chunk_guard.last_accessed = Instant::now();
                
                // If inactive, mark as active
                if chunk_guard.state == ChunkState::Inactive {
                    chunk_guard.state = ChunkState::Active;
                }
            }
            
            return true;
        }
        
        // Chunk not in memory, start loading process
        let new_chunk = Arc::new(Mutex::new(Chunk::new(position)));
        chunks.insert(position, Arc::clone(&new_chunk));
        
        // Check if this chunk exists in storage
        let chunks_clone = Arc::clone(&self.chunks);
        let storage_clone = Arc::clone(&self.storage);
        let new_chunk_clone = Arc::clone(&new_chunk);
        let biome_manager = self.biome_manager.clone();
        
        self.thread_pool.execute(move || {
            if storage_clone.chunk_exists(position) {
                // Chunk exists in storage, load it
                if let Ok(mut chunk_guard) = new_chunk_clone.lock() {
                    chunk_guard.state = ChunkState::Loading;
                }
                
                if let Some(loaded_data) = storage_clone.load_chunk(position) {
                    if let Ok(mut chunk_guard) = new_chunk_clone.lock() {
                        // Update chunk with loaded data
                        chunk_guard.heightmap = loaded_data.heightmap;
                        chunk_guard.biome_ids = loaded_data.biome_ids;
                        chunk_guard.state = ChunkState::Active;
                    }
                }
            } else {
                // Chunk doesn't exist, generate it
                Self::generate_chunk(new_chunk_clone, position, storage_clone, biome_manager);
            }
        });
        
        true
    }
    
    // Generate a new chunk
    fn generate_chunk(
        chunk: Arc<Mutex<Chunk>>, 
        position: ChunkPosition,
        storage: Arc<ChunkStorage>,
        biome_manager: Option<Gd<BiomeManager>>
    ) {
        if let Ok(mut chunk_guard) = chunk.lock() {
            chunk_guard.state = ChunkState::Generating;
        }
        
        // Generate heightmap
        let mut heightmap = vec![0.0; (CHUNK_SIZE * CHUNK_SIZE) as usize];
        let mut biome_ids = vec![0; (CHUNK_SIZE * CHUNK_SIZE) as usize];
        
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let world_x = position.x as f32 * CHUNK_SIZE as f32 + x as f32;
                let world_z = position.z as f32 * CHUNK_SIZE as f32 + z as f32;
                
                // Get biome from BiomeManager if available
                let biome_id = if let Some(ref biome_mgr) = biome_manager {
                    let color = biome_mgr.bind().get_biome_color(world_x, world_z);
                    // Simple mapping from color to biome ID
                    ((color.r * 5.0) as u8) % 5
                } else {
                    // Default heightmap generation if no BiomeManager
                    ((world_x.cos() * 0.5 + world_z.sin() * 0.5) * 2.0) as u8
                };
                
                // Simple example noise function - would be more sophisticated in real implementation
                let height = (world_x.cos() * 0.5 + world_z.sin() * 0.5) * 10.0;
                let idx = (z * CHUNK_SIZE + x) as usize;
                
                heightmap[idx] = height;
                biome_ids[idx] = biome_id;
            }
        }
        
        if let Ok(mut chunk_guard) = chunk.lock() {
            chunk_guard.heightmap = heightmap;
            chunk_guard.biome_ids = biome_ids;
            chunk_guard.state = ChunkState::Active;
        }
        
        // Save to storage
        storage.save_chunk(position, &heightmap, &biome_ids);
    }
    
    // Update chunks based on player position
    #[func]
    pub fn update(&self, player_x: f32, player_y: f32, player_z: f32) {
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
    
    // Get the heightmap data for a chunk
    #[func]
    pub fn get_chunk_heightmap(&self, position_x: i32, position_z: i32) -> PackedFloat32Array {
        let position = ChunkPosition { x: position_x, z: position_z };
        
        if let Some(chunk) = self.chunks.lock().unwrap().get(&position) {
            if let Ok(chunk_guard) = chunk.lock() {
                if chunk_guard.state == ChunkState::Active {
                    return PackedFloat32Array::from(&chunk_guard.heightmap);
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
                    
                    return PackedInt32Array::from(&biomes_i32);
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
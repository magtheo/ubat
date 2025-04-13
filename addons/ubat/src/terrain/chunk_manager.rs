use godot::prelude::*;
use godot::classes::{FastNoiseLite, Node3D}; // Need FastNoiseLite for generation
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize}; // Needed for ChunkPosition if defined here
use godot::classes::fast_noise_lite::{NoiseType, FractalType};

// Use ChunkData from ChunkStorage
use crate::threading::chunk_storage::{ChunkData, ChunkStorage};
// Use ThreadPool (specifically for compute tasks, using the global pool)
use crate::threading::thread_pool::{ThreadPool, global_thread_pool, get_or_init_global_pool};
// Use BiomeManager and its thread-safe data
use crate::terrain::biome_manager::{BiomeManager, ThreadSafeBiomeData};
use crate::terrain::terrain_config::{TerrainConfigManager, TerrainConfig};

// ChunkPosition (Defined here or in a shared location like terrain/mod.rs)
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkPosition {
    pub x: i32,
    pub z: i32,
}

// State for tracking generation/loading status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChunkGenState {
    Unknown,
    Loading,    // Queued for loading from storage
    Generating, // Queued for generation
    Ready(Instant), // Data is available (either loaded or generated)
}

// Constants
const UNLOAD_CHECK_INTERVAL: Duration = Duration::from_secs(5); // How often to check for unloading

// ChunkManager class
#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct ChunkManager {
    #[base]
    base: Base<Node3D>,

    storage: Arc<ChunkStorage>,
    compute_pool: Arc<RwLock<ThreadPool>>,
    chunk_states: Arc<RwLock<HashMap<ChunkPosition, ChunkGenState>>>,
    biome_manager: Option<Gd<BiomeManager>>,
    thread_safe_biome_data: Arc<RwLock<Option<Arc<ThreadSafeBiomeData>>>>,

    // Configurable values
    render_distance: i32,
    chunk_size: u32,

    // Internal state
    last_unload_check: Instant,
}

#[godot_api]
impl INode3D for ChunkManager {
    fn init(base: Base<Node3D>) -> Self {
        godot_print!("ChunkManager: Initializing...");
        let storage = Arc::new(ChunkStorage::new("user://terrain_data"));
        let compute_pool = get_or_init_global_pool(); // Use global pool

        let chunk_size = if let Some(config_arc) = TerrainConfigManager::get_config() {
            config_arc.read().map_or(32, |guard| guard.chunk_size())
        } else {
            32 // Default if config not ready
        };

        ChunkManager {
            base,
            storage,
            compute_pool,
            chunk_states: Arc::new(RwLock::new(HashMap::new())),
            biome_manager: None,
            thread_safe_biome_data: Arc::new(RwLock::new(None)),
            render_distance: 8, // Default
            chunk_size,
            last_unload_check: Instant::now(),
        }
    }

    fn ready(&mut self) {
        let start_time = std::time::Instant::now();
        godot_print!("ChunkManager: Ready. Linking BiomeManager...");
        self.biome_manager = None; // Ensure starts as None

        if let Some(parent) = self.base().get_parent() {
            // --- Use string literal for get_node_as ---
            let biome_manager_node = parent.get_node_as::<BiomeManager>("BiomeManager");
            if biome_manager_node.is_instance_valid() {
                if biome_manager_node.bind().is_fully_initialized() {
                    godot_print!("ChunkManager: BiomeManager is initialized.");
                    // Assign if ready (original simple assignment should work now)
                    self.biome_manager = Some(biome_manager_node.clone());
                    self.update_thread_safe_biome_data();
                } else {
                    godot_error!("ChunkManager: Found 'BiomeManager', but it's not initialized yet.");
                    // biome_manager remains None
                }
            } else {
                godot_error!("ChunkManager: Could not find node 'BiomeManager' under parent.");
                // biome_manager remains None
            }
        } else {
            godot_error!("ChunkManager: Could not find parent node!");
            // biome_manager remains None
        }

        // Ensure chunk size matches latest config
        self.apply_config_updates();

        godot_print!("ChunkManager: Ready completed in {}ms", start_time.elapsed().as_millis());
    }

    fn process(&mut self, _delta: f64) {
        // Periodic checks can happen here if needed,
        // but main update driven by ChunkController `update` call.
        // Example: check for timed unload independent of player movement
        if self.last_unload_check.elapsed() > UNLOAD_CHECK_INTERVAL {
             // Note: This unload check requires knowing the required chunks.
             // It's better handled within the `update` method called by ChunkController.
             // self.unload_distant_chunks(&HashSet::new()); // Example, needs refinement
             self.last_unload_check = Instant::now();
         }
    }
}

#[godot_api]
impl ChunkManager {
    #[func]
    pub fn is_initialized(&self) -> bool {
        // Consider initialized if biome data is available
        self.thread_safe_biome_data.read().unwrap().is_some()
    }

    // Ensure chunk data is loaded or generation is triggered.
    fn ensure_chunk_is_ready(&self, pos: ChunkPosition) {
        // godot_print!("ChunkManager: ensure_chunk_position: {:?}", pos);
        // Fast path: Check read lock first
        let current_state = self.chunk_states.read().unwrap().get(&pos).cloned();

        match current_state {
            Some(ChunkGenState::Ready(_)) | Some(ChunkGenState::Loading) | Some(ChunkGenState::Generating) => {
                return; // Already handled or in progress
            }
            _ => {} // Unknown or None, proceed below
        }

        // Acquire write lock to modify state
        let mut states = self.chunk_states.write().unwrap();
        // Double-check state after acquiring write lock
        match states.get(&pos) {
             Some(ChunkGenState::Ready(_)) | Some(ChunkGenState::Loading) | Some(ChunkGenState::Generating) => {
                 return; // Another thread handled it
             }
            _ => {
                // Set state to Loading and queue load operation
                states.insert(pos, ChunkGenState::Loading);
                // Drop write lock before calling queue_load_chunk if it might re-enter
                drop(states);

                let storage_clone = Arc::clone(&self.storage);
                let states_clone = Arc::clone(&self.chunk_states);
                let compute_pool_clone = Arc::clone(&self.compute_pool);
                let biome_data_clone = Arc::clone(&self.thread_safe_biome_data);
                let chunk_size = self.chunk_size;

                let storage_for_inner_closure = Arc::clone(&storage_clone);

                godot_print!("ChunkManager::ensure_chunk_is_ready: Queuing load for {:?}", pos); // ADD
                storage_clone.queue_load_chunk(pos, move |load_result| {
                    godot_print!("ChunkManager: Load callback executed for {:?}. Found data: {}", pos, load_result.is_some());

                match load_result {
                        Some(_chunk_data) => {
                            // Loaded from storage
                            godot_print!("ChunkManager: Load callback: Found data for {:?}, setting Ready.", pos); // ADD
                            let mut states_w = states_clone.write().unwrap();
                            states_w.insert(pos, ChunkGenState::Ready(Instant::now()));
                            // godot_print!("ChunkManager: Chunk {:?} loaded from storage.", pos);
                        }
                        None => {
                            // Not in storage, trigger generation
                            {
                                godot_print!("ChunkManager: Load callback: No data for {:?}, proceeding to generate.", pos); // ADD
                                let mut states_w = states_clone.write().unwrap();
                                // Ensure state is still Loading before switching to Generating
                                if states_w.get(&pos) == Some(&ChunkGenState::Loading) {
                                    states_w.insert(pos, ChunkGenState::Generating);
                                } else {
                                    godot_warn!("ChunkManager: State changed unexpectedly for {:?} while queuing generation.", pos);
                                    return; // Avoid queuing generation if state changed
                                }
                            } // Write lock released

                            godot_print!("ChunkManager: Chunk {:?} not found in storage. Triggering generation.", pos);
                            compute_pool_clone.read().unwrap().execute(move || {
                                Self::generate_and_save_chunk(
                                    pos,
                                    storage_for_inner_closure, // Use the already cloned storage Arc
                                    states_clone,
                                    biome_data_clone,
                                    chunk_size,
                                );
                            });
                        }
                    }
                });
            }
        }
    }

    // Generation logic (runs on compute pool)
    fn generate_and_save_chunk(
        pos: ChunkPosition,
        storage: Arc<ChunkStorage>,
        states: Arc<RwLock<HashMap<ChunkPosition, ChunkGenState>>>,
        biome_data_arc_rwlock: Arc<RwLock<Option<Arc<ThreadSafeBiomeData>>>>,
        chunk_size: u32,
    ) {
        // Acquire read lock on Option<Arc<ThreadSafeBiomeData>>
        let biome_data_opt_arc = biome_data_arc_rwlock.read().unwrap();
        let biome_data = match &*biome_data_opt_arc {
            Some(arc) => Some(Arc::clone(arc)), // Clone the inner Arc<ThreadSafeBiomeData>
            None => None,
        };
        drop(biome_data_opt_arc); // Release read lock on the Option

        // Check if biome data is available
        if biome_data.is_none() {
            godot_error!("ChunkManager: Cannot generate chunk {:?}, BiomeData is missing.", pos);
            let mut states_w = states.write().unwrap();
            states_w.insert(pos, ChunkGenState::Unknown); // Reset state
            return;
        }
        let biome_data = biome_data.unwrap(); // Now we have Arc<ThreadSafeBiomeData>

        // --- Generation ---
        let chunk_area = (chunk_size * chunk_size) as usize;
        let mut heightmap = vec![0.0f32; chunk_area];
        let mut biome_ids = vec![0u8; chunk_area];

        // TODO: modify chunkManager to take premade noises
        // Example noise setup (consider passing noise config via TerrainConfig/BiomeData)
        let mut noise = FastNoiseLite::new_gd();
        noise.set_seed(biome_data.seed() as i32);
        noise.set_frequency(0.05); // Example frequency
        noise.set_noise_type(NoiseType::SIMPLEX_SMOOTH);
        noise.set_fractal_type(FractalType::FBM);
        noise.set_fractal_octaves(4);
        noise.set_fractal_lacunarity(2.0);
        noise.set_fractal_gain(0.5);

        for z in 0..chunk_size {
            for x in 0..chunk_size {
                let idx = (z * chunk_size + x) as usize;
                let world_x = pos.x as f32 * chunk_size as f32 + x as f32;
                let world_z = pos.z as f32 * chunk_size as f32 + z as f32;

                let biome_id = biome_data.get_biome_id(world_x, world_z);
                biome_ids[idx] = biome_id;

                // Example height calculation
                let base_height = noise.get_noise_2d(world_x * 0.1, world_z * 0.1) * 15.0;
                 let biome_height_mod = match biome_id {
                    1 => -2.0 + noise.get_noise_2d(world_x * 0.5, world_z * 0.5) * 1.0, // Coral lower with ripples
                    2 => -3.0 + noise.get_noise_2d(world_x * 0.8, world_z * 0.8) * 0.5, // Sand lower and flatter
                    3 => 5.0 + noise.get_noise_2d(world_x * 0.2, world_z * 0.2) * 5.0, // Rock higher and rougher
                    4 => 0.0 + noise.get_noise_2d(world_x * 0.3, world_z * 0.3) * 2.0, // Kelp baseline with medium noise
                    5 => 8.0 + noise.get_noise_2d(world_x * 0.15, world_z * 0.15) * 8.0, // Lavarock very high and rough
                    _ => 0.0, // Unknown
                };
                heightmap[idx] = base_height + biome_height_mod;
            }
        }

        // Blend heights at biome boundaries
        Self::blend_heights(&mut heightmap, &biome_ids, chunk_size, biome_data.blend_distance());

        // --- Save and Update State ---
        // Storage Arc was already cloned and passed in
        storage.queue_save_chunk(pos, &heightmap, &biome_ids);

        // Update the state map
        let mut states_w = states.write().unwrap();
        states_w.insert(pos, ChunkGenState::Ready(Instant::now()));
        // godot_print!("ChunkManager: Generation finished for {:?}", pos);
    }

    #[func]
    pub fn get_chunk(&mut self, x: i32, z: i32) -> bool {
        let pos = ChunkPosition { x, z };
        
        // Check if the chunk is already ready
        if self.is_chunk_ready(x, z) {
            return true;
        }
        
        // Request the chunk to be loaded/generated
        self.ensure_chunk_is_ready(pos);
        
        // Return true to indicate the request was initiated
        // Note: This doesn't mean the chunk is immediately available
        true
    }

    // Height Blending Logic (Static)
    fn blend_heights(heightmap: &mut [f32], biome_ids: &[u8], chunk_size: u32, blend_distance: i32) {
        if blend_distance <= 0 { return; } // Skip if no blend distance

        let original_heights = heightmap.to_vec();
        let blend_radius = blend_distance.max(1); // Ensure radius is at least 1

        for z in 0..chunk_size {
            for x in 0..chunk_size {
                let idx = (z * chunk_size + x) as usize;
                let current_biome = biome_ids[idx];
                let mut is_boundary = false;
                let mut blend_needed = false;
                let mut total_weight = 0.0;
                let mut weighted_height_sum = 0.0;

                // Check immediate neighbors first to determine if it's a boundary
                for dz in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dz == 0 { continue; }
                        let nx = x as i32 + dx;
                        let nz = z as i32 + dz;
                        if nx >= 0 && nx < chunk_size as i32 && nz >= 0 && nz < chunk_size as i32 {
                            let nidx = (nz as u32 * chunk_size + nx as u32) as usize;
                            if biome_ids[nidx] != current_biome {
                                is_boundary = true;
                                break;
                            }
                        }
                    }
                    if is_boundary { break; }
                }

                // If it's a boundary, perform blending using the blend radius
                if is_boundary {
                    for dz in -blend_radius..=blend_radius {
                        for dx in -blend_radius..=blend_radius {
                            let nx = x as i32 + dx;
                            let nz = z as i32 + dz;

                            // Check if neighbor is within chunk bounds
                            if nx >= 0 && nx < chunk_size as i32 && nz >= 0 && nz < chunk_size as i32 {
                                let nidx = (nz as u32 * chunk_size + nx as u32) as usize;
                                let distance = ((dx * dx + dz * dz) as f32).sqrt();

                                // Simple linear falloff weight
                                let weight = (blend_radius as f32 - distance).max(0.0) / blend_radius as f32;

                                if weight > 0.0 {
                                    total_weight += weight;
                                    weighted_height_sum += original_heights[nidx] * weight;
                                    blend_needed = true;
                                }
                            }
                        }
                    }

                    // Apply weighted average if blending occurred
                    if blend_needed && total_weight > 0.0 {
                        heightmap[idx] = weighted_height_sum / total_weight;
                    }
                }
                // If not a boundary, heightmap[idx] remains unchanged (equal to original_heights[idx])
            }
        }
    }

    // Called by ChunkController to update based on player position
    #[func]
    pub fn update(&self, player_x: f32, _player_y: f32, player_z: f32) {
        
        let player_chunk_x = (player_x / self.chunk_size as f32).floor() as i32;
        let player_chunk_z = (player_z / self.chunk_size as f32).floor() as i32;
        godot_print!("ChunkManager: update at: {:?}, {:?}", player_chunk_x, player_chunk_z);
        
        let mut required_chunks = HashSet::new();
        for x in (player_chunk_x - self.render_distance)..=(player_chunk_x + self.render_distance) {
            for z in (player_chunk_z - self.render_distance)..=(player_chunk_z + self.render_distance) {
                let pos = ChunkPosition { x, z };
                required_chunks.insert(pos);
                self.ensure_chunk_is_ready(pos); // Request load/generation if needed
            }
        }
        // After queueing all potential loads for this update cycle,
        // explicitly trigger the IO queue processing.
        // process_io_queue takes Arc<Self>, so we clone the storage Arc.
        // godot_print!("ChunkManager::update: Finished queuing loads. Triggering IO processing.");
        // Arc::clone(&self.storage).process_io_queue();
        // adding this above coused a crash witout any error ar warnings

        // Perform unload check now that we know required chunks
        self.unload_distant_chunks(&required_chunks);
    }

     // Unload chunks no longer needed
     fn unload_distant_chunks(&self, required_chunks: &HashSet<ChunkPosition>) {
         let mut chunks_to_remove = Vec::new();
         let unload_dist_sq = (self.render_distance + 2) * (self.render_distance + 2); // Use buffer

         // Scope for read lock
         {
             let states_read = self.chunk_states.read().unwrap();
             for (&pos, &state) in states_read.iter() {
                 // Check if outside required set
                 if !required_chunks.contains(&pos) {
                    // Check if ready and inactive for a while, or just unknown/not busy
                     if let ChunkGenState::Ready(ready_time) = state {
                        if ready_time.elapsed() > UNLOAD_CHECK_INTERVAL * 2 { // Example longer timeout
                             chunks_to_remove.push(pos);
                         }
                     } else if state == ChunkGenState::Unknown {
                          chunks_to_remove.push(pos); // Remove unknown states outside view
                     }
                 }
             }
         } // Read lock dropped

         if !chunks_to_remove.is_empty() {
            //  godot_print!("ChunkManager: Unloading {} chunk states.", chunks_to_remove.len());
             let mut states_write = self.chunk_states.write().unwrap();
             for pos in chunks_to_remove {
                 states_write.remove(&pos);
                 // Optional: Hint to storage cache to remove, but LRU should handle it.
                 // self.storage.evict_from_cache(pos); // Needs implementation in ChunkStorage
             }
         }
     }

    // Public API Methods
    #[func]
    pub fn is_chunk_ready(&self, position_x: i32, position_z: i32) -> bool {
        let pos = ChunkPosition { x: position_x, z: position_z };
        matches!(
            self.chunk_states.read().unwrap().get(&pos),
            Some(ChunkGenState::Ready(_))
        )
    }

    #[func]
    pub fn get_chunk_heightmap(&self, position_x: i32, position_z: i32) -> PackedFloat32Array {
        let pos = ChunkPosition { x: position_x, z: position_z };

        // Check state first for efficiency
        if !self.is_chunk_ready(position_x, position_z) {
            // godot_warn!("Requested heightmap for non-ready chunk {:?}", pos);
            return PackedFloat32Array::new();
        }

        // Attempt to load (checks cache first)
        match self.storage.load_chunk(pos) {
            Some(chunk_data) => PackedFloat32Array::from(&chunk_data.heightmap[..]),
            None => {
                // Should not happen if state is Ready
                godot_error!("CRITICAL: Chunk state for {:?} is Ready, but load_chunk failed!", pos);
                // Reset state?
                 self.chunk_states.write().unwrap().insert(pos, ChunkGenState::Unknown);
                PackedFloat32Array::new()
            }
        }
    }

    #[func]
    pub fn get_chunk_biomes(&self, position_x: i32, position_z: i32) -> PackedInt32Array {
        let pos = ChunkPosition { x: position_x, z: position_z };

        if !self.is_chunk_ready(position_x, position_z) {
            // godot_warn!("Requested biomes for non-ready chunk {:?}", pos);
            return PackedInt32Array::new();
        }

        match self.storage.load_chunk(pos) {
            Some(chunk_data) => {
                let biomes_i32: Vec<i32> = chunk_data.biome_ids.iter().map(|&id| id as i32).collect();
                PackedInt32Array::from(&biomes_i32[..])
            },
            None => {
                godot_error!("CRITICAL: Chunk state for {:?} is Ready, but load_chunk failed for biomes!", pos);
                 self.chunk_states.write().unwrap().insert(pos, ChunkGenState::Unknown);
                PackedInt32Array::new()
            }
        }
    }

    #[func]
    pub fn get_chunk_count(&self) -> i32 {
        self.chunk_states.read().unwrap().len() as i32
    }

    #[func]
    pub fn set_render_distance(&mut self, distance: i32) {
        let new_distance = distance.max(1).min(32); // Clamp
         if new_distance != self.render_distance{
             self.render_distance = new_distance;
             godot_print!("ChunkManager: Render distance set to {}", self.render_distance);
             // Trigger an unload check immediately? Optional.
         }
    }

    #[func]
    pub fn get_render_distance(&self) -> i32 {
        self.render_distance
    }

    #[func]
    pub fn set_biome_manager(&mut self, biome_manager: Gd<BiomeManager>) {
        godot_print!("ChunkManager: BiomeManager reference set.");
        self.biome_manager = Some(biome_manager);
        self.update_thread_safe_biome_data(); // Update data immediately
    }

    // Update thread-safe biome data cache
    #[func]
    pub fn update_thread_safe_biome_data(&mut self) {
        if let Some(ref biome_mgr_gd) = self.biome_manager {
            let biome_mgr_bind = biome_mgr_gd.bind();
            if biome_mgr_bind.is_fully_initialized() {
                // godot_print!("ChunkManager: Updating thread-safe biome data cache.");
                let new_data = Arc::new(ThreadSafeBiomeData::from_biome_manager(&biome_mgr_bind));
                // Acquire write lock on the Option<Arc<...>>
                let mut biome_data_guard = self.thread_safe_biome_data.write().unwrap();
                *biome_data_guard = Some(new_data); // Set the new data
            } else {
                godot_warn!("ChunkManager: Attempted to update biome data, but BiomeManager is not ready.");
            }
        } else {
            godot_warn!("ChunkManager: Cannot update biome data, BiomeManager reference missing.");
        }
    }

     // Apply config changes dynamically
     #[func]
     pub fn apply_config_updates(&mut self) {
         if let Some(config_arc) = TerrainConfigManager::get_config() {
             if let Ok(guard) = config_arc.read() {
                 let old_chunk_size = self.chunk_size;
                 self.chunk_size = guard.chunk_size();
                 // Tell storage to update its cache limit
                 self.storage.update_cache_limit();
                 godot_print!("ChunkManager: Applied config updates (chunk_size: {}, cache_limit updated)", self.chunk_size);
                 // If chunk size changes, existing data becomes invalid!
                 if old_chunk_size != self.chunk_size {
                     godot_warn!("ChunkManager: Chunk size changed! Clearing all chunk states and storage cache. Chunks will regenerate.");
                     self.chunk_states.write().unwrap().clear();
                     self.storage.clear_cache();
                     // Ideally, also delete stored files, but that's more complex.
                 }
             }
         }
     }
}
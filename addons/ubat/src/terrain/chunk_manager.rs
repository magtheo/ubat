use godot::prelude::*;
use godot::classes::{FastNoiseLite, Node3D}; // Need FastNoiseLite for generation
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize}; // Needed for ChunkPosition if defined here
use godot::classes::fast_noise_lite::{NoiseType, FractalType};
use std::sync::mpsc::{channel, Sender, Receiver, TryRecvError};

use crate::terrain::noise::noise_manager::NoiseManager;
use crate::terrain::noise::noise_parameters::{NoiseParameters, RustNoiseType, RustFractalType}; // Import enums too
use noise::NoiseFn; // Keep NoiseFn trait import
use std::cmp::Ordering;

// Use ChunkData from ChunkStorage
use crate::threading::chunk_storage::{ChunkData, MeshGeometry, ChunkStorage};
use crate::terrain::generation_utils::{generate_mesh_geometry, get_clamped_height};
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

#[derive(Debug, Clone)] // Make sure ChunkData also derives Clone
pub enum ChunkResult {
    Loaded(ChunkPosition, ChunkData),
    LoadFailed(ChunkPosition),
    Generated(ChunkPosition, ChunkData),
    GenerationFailed(ChunkPosition, String),
    LogMessage(String), // Added LogMessage variant


    // Saved(ChunkPosition), // Optional for now
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

    // Channel for results from background tasks
    result_sender: Sender<ChunkResult>,
    result_receiver: Receiver<ChunkResult>,

    compute_pool: Arc<RwLock<ThreadPool>>,
    chunk_states: Arc<RwLock<HashMap<ChunkPosition, ChunkGenState>>>,
    biome_manager: Option<Gd<BiomeManager>>,
    noise_manager: Option<Gd<NoiseManager>>, // Add this
    thread_safe_biome_data: Arc<RwLock<Option<Arc<ThreadSafeBiomeData>>>>,
    is_thread_safe_data_ready: bool,

    // handle to the noise parameter cache
    noise_functions_cache: Option<Arc<RwLock<HashMap<String, Arc<dyn NoiseFn<f64, 2> + Send + Sync>>>>>,

    // Configurable values
    render_distance: i32,
    chunk_size: u32,

    // Internal state
    last_unload_check: Instant,
}

#[godot_api]
impl INode3D for ChunkManager {
    fn init(base: Base<Node3D>) -> Self {
        println!("ChunkManager: Initializing...");
        let (tx, rx) = channel(); // Create the channel
        let storage = Arc::new(ChunkStorage::new("user://terrain_data", tx.clone()));
        let compute_pool = get_or_init_global_pool(); // Use global pool

        let config_arc:&'static Arc<RwLock<TerrainConfig>> = TerrainConfigManager::get_config(); // Get static ref
        let chunk_size = match config_arc.read() { // Lock it
            Ok(guard) => guard.chunk_size, // Access field
            Err(_) => {
                eprintln!("ChunkManager::init: Failed to read terrain config lock for chunk size. Using default 32.");
                32 // Default if lock fails
            }
        };

        ChunkManager {
            base,
            storage,
            compute_pool,
            result_sender: tx, // Store sender
            result_receiver: rx, // Store receiver

            chunk_states: Arc::new(RwLock::new(HashMap::new())),
            biome_manager: None,
            noise_manager: None,
            thread_safe_biome_data: Arc::new(RwLock::new(None)),
            is_thread_safe_data_ready: false,
            noise_functions_cache: None, // Initialize as None
            render_distance: 4, // TODO This overides terrain initalizer, and it shuold not
            chunk_size,
            last_unload_check: Instant::now(),
        }
    }

    fn ready(&mut self) {
        let start_time = std::time::Instant::now();
        println!("ChunkManager: Ready. Linking BiomeManager...");
        self.biome_manager = None; // Ensure starts as None

        if let Some(parent) = self.base().get_parent() {
            // --- Use string literal for get_node_as ---
            let biome_manager_node = parent.get_node_as::<BiomeManager>("BiomeManager");
            if biome_manager_node.is_instance_valid() {
                if biome_manager_node.bind().is_fully_initialized() {
                    println!("ChunkManager: BiomeManager is initialized.");
                    // Assign if ready (original simple assignment should work now)
                    self.biome_manager = Some(biome_manager_node.clone());
                } else {
                    eprintln!("ChunkManager: Found 'BiomeManager', but it's not initialized yet.");
                    // biome_manager remains None
                }
            } else {
                eprintln!("ChunkManager: Could not find node 'BiomeManager' under parent.");
                // biome_manager remains None
            }

            // --- Link NoiseManager ---
            let noise_manager_node = parent.get_node_as::<NoiseManager>("NoiseManager");
            if noise_manager_node.is_instance_valid() {
                println!("ChunkManager: Linking NoiseManager"); // Use println!
                let nm_gd = noise_manager_node;
                // Get the FUNCTION cache handle using the new NoiseManager getter
                self.noise_functions_cache = Some(nm_gd.bind().get_function_cache_handle()); // <- Use new getter
                self.noise_manager = Some(nm_gd); // Keep Gd if needed
            } else {
                 eprintln!("ChunkManager: Could not find node 'NoiseManager'. Noise functions will be unavailable."); // Use eprintln!
                 self.noise_functions_cache = None;
                 self.noise_manager = None; // Ensure this is None too
            }

        } else {
            eprintln!("ChunkManager: Could not find parent node!");
            // biome_manager remains None
        }

        // Ensure chunk size matches latest config
        self.apply_config_updates();

        println!("ChunkManager: Ready completed in {}ms", start_time.elapsed().as_millis());
    }

    fn process(&mut self, _delta: f64) {
        
        // --- Initialization Check (at the beginning of process) ---
        if !self.is_thread_safe_data_ready {
            // Attempt to initialize/update if managers seem ready
            if self.biome_manager.is_some() && self.noise_manager.is_some() {
                // Call the update function (it has internal checks for None)
                self.update_thread_safe_biome_data();

                // Check if the update was successful by reading the value
                if self.thread_safe_biome_data.read().unwrap().is_some() {
                    println!("ChunkManager: ThreadSafeBiomeData successfully initialized/updated in process."); // Use println!
                    self.is_thread_safe_data_ready = true; // Set flag only on success
                } else {
                    // Optional: Log that managers are present but data update failed
                    // eprintln!("ChunkManager process: Managers linked, but ThreadSafeBiomeData update still results in None.");
                }
            } // Else: Managers not linked yet, will try again next frame
        }
        
        // Process results received from background tasks
        let mut result_count = 0;
        loop {
            match self.result_receiver.try_recv() {
                Ok(result) => {
                    result_count += 1;
                    match &result {
                        ChunkResult::LoadFailed(pos) => {
                            godot_print!("ChunkManager: Received LoadFailed for {:?}, will queue generation", pos);
                        },
                        ChunkResult::Loaded(pos, _) => {
                            // godot_print!("ChunkManager: Received Loaded result for {:?}", pos);
                        },
                        ChunkResult::Generated(pos, _) => {
                            // godot_print!("ChunkManager: Received Generated result for {:?}", pos);
                        },
                        _ => {}
                    }
                    self.handle_chunk_result(result);
                }
                Err(TryRecvError::Empty) => {
                    if result_count > 0 {
                        godot_print!("ChunkManager process: Processed {} results this frame", result_count);
                    }
                    break;
                }
                Err(TryRecvError::Disconnected) => {
                    godot_error!("ChunkManager: Result channel disconnected! This is a critical error.");
                    break;
                }
            }
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
        // Fast path check (read lock) - unchanged
        let current_state = self.chunk_states.read().unwrap().get(&pos).cloned();
        match current_state {
            Some(ChunkGenState::Ready(_)) | Some(ChunkGenState::Loading) | Some(ChunkGenState::Generating) => return,
            _ => {}
        }
   
        // Acquire write lock - unchanged
        let mut states = self.chunk_states.write().unwrap();
        // Double-check state - unchanged
        match states.get(&pos) {
            Some(ChunkGenState::Ready(_)) | Some(ChunkGenState::Loading) | Some(ChunkGenState::Generating) => return,
            _ => {
                // Set state to Loading
                // godot_print!("ChunkManager::ensure_chunk_is_ready: Setting state Loading for {:?}", pos);
                states.insert(pos, ChunkGenState::Loading);
                // Drop write lock *before* calling storage
                drop(states);
   
                // Queue load task - NO CALLBACK CLOSURE NEEDED
                // `queue_load_chunk` now needs the sender, passed via storage Arc
                // godot_print!("ChunkManager::ensure_chunk_is_ready: Queuing load for {:?}", pos);
                self.storage.queue_load_chunk(pos); // queue_load_chunk internally uses sender passed during its init
   
                // Generation is no longer triggered directly here.
                // It's triggered by handle_chunk_result when LoadFailed is received.
            }
        }
    }

    fn queue_generation(&self, pos: ChunkPosition) {
        println!("ChunkManager: Queuing generation task for {:?}", pos);
        let storage_clone = Arc::clone(&self.storage);
        // --- Clone the Arc containing the Option<Arc<ThreadSafeBiomeData>> ---
        let biome_data_rwlock_arc = Arc::clone(&self.thread_safe_biome_data);
        // --- Do NOT read() here, read inside the worker thread ---

        let config_arc:&'static Arc<RwLock<TerrainConfig>> = TerrainConfigManager::get_config();
        let amplification = match config_arc.read() {
            Ok(guard) => guard.amplification,
            Err(_) => {
                eprintln!("ERROR: Failed to read terrain config lock in queue_generation. Using default amplification 1.0");
                1.0 // Default on error
            }
        };
        println!("ChunkManager: Amplification = {}", amplification);

    
        let chunk_size = self.chunk_size;
        let sender_clone = self.result_sender.clone();
    
        // Fetch the noise parameter cache for height generation (still needed)
        let noise_funcs_cache_handle = match &self.noise_functions_cache { // Use the correct field
            Some(cache_arc) => Arc::clone(cache_arc), // Clone the Arc handle
            None => {
                eprintln!("ChunkManager: Cannot queue generation for {:?}, Noise function cache is not available.", pos);
                let _ = sender_clone.send(ChunkResult::GenerationFailed(pos, "Noise function cache unavailable".to_string()));
                return;
            }
        };
  
        self.compute_pool.read().unwrap().execute(move || {
            // --- Read the BiomeData Arc INSIDE the worker ---
            let biome_data_guard = biome_data_rwlock_arc.read().unwrap();
            // Clone the inner Arc<ThreadSafeBiomeData> if it exists
            let biome_data_clone = match *biome_data_guard {
                Some(ref arc) => Some(Arc::clone(arc)),
                None => None,
            };
            // Drop the read guard quickly
            drop(biome_data_guard);
    
            // --- Check if biome data is available ---
            if biome_data_clone.is_none() {
                let err_msg = format!("BiomeData unavailable for generation task at {:?} (ChunkManager's shared data is None)", pos);                // Send the original String
                let _ = sender_clone.send(ChunkResult::GenerationFailed(pos, err_msg.clone())); // Clone err_msg here
                // Log using the original still-owned string
                eprintln!("{}", err_msg);
                return;
            }
            
            // We know it's Some now
            let biome_data = biome_data_clone.unwrap();
    
    
            Self::generate_and_save_chunk(
                pos,
                storage_clone,
                biome_data, // Pass the Arc<ThreadSafeBiomeData>
                noise_funcs_cache_handle, // Pass the noise cache handle for heights
                chunk_size,
                sender_clone,
                amplification,
            );
        });
    }

    fn handle_chunk_result(&mut self, result: ChunkResult) {
        // Lock states only when modification is needed
        match result {
            ChunkResult::Loaded(pos, data) => { // data is owned here
                // --- Update storage cache immediately ---
                match self.storage.cache.write() { // Access the cache field directly
                    Ok(mut cache_w) => {
                        // godot_print!("ChunkManager handle_chunk_result: Caching Loaded data for {:?}", pos); // Optional debug log
                        cache_w.push(pos, data); // Push loaded data into LRU cache
                    },
                    Err(_) => {
                        eprintln!("ChunkManager handle_chunk_result: Cache write lock poisoned updating cache for loaded {:?}", pos);
                    }
                }
    
                // Update state AFTER caching attempt
                let mut states = self.chunk_states.write().unwrap();
                // godot_print!("ChunkManager: Setting state Ready for loaded chunk {:?}", pos);
                states.insert(pos, ChunkGenState::Ready(Instant::now()));
            }    
            ChunkResult::LoadFailed(pos) => {
                let mut states = self.chunk_states.write().unwrap();
                match states.get(&pos) {
                    Some(ChunkGenState::Loading) => {
                        println!("ChunkManager: LoadFailed for {:?} - state is correctly Loading, changing to Generating", pos);
                        states.insert(pos, ChunkGenState::Generating);
                        drop(states); // Drop lock BEFORE queueing
                        self.queue_generation(pos);
                    },
                    other_state => {
                        eprintln!("ChunkManager: Received LoadFailed for {:?} but state was not Loading: {:?}",
                                   pos, other_state);
                        states.insert(pos, ChunkGenState::Unknown); // Reset state
                    }
                }
            }
            ChunkResult::Generated(pos, data) => { // data is owned here
                // --- Update storage cache immediately ---
                match self.storage.cache.write() { // Access the cache field directly
                   Ok(mut cache_w) => {
                       // godot_print!("ChunkManager handle_chunk_result: Caching Generated data for {:?}", pos); // Optional debug log
                       cache_w.push(pos, data); // Push generated data into LRU cache
                   },
                   Err(_) => {
                       eprintln!("ChunkManager handle_chunk_result: Cache write lock poisoned updating cache for generated {:?}", pos);
                   }
                }
   
                // Update state AFTER caching attempt
                let mut states = self.chunk_states.write().unwrap();
                // godot_print!("ChunkManager: Received Generated for {:?}, setting Ready.", pos);
                states.insert(pos, ChunkGenState::Ready(Instant::now()));
            }
            ChunkResult::GenerationFailed(pos, err) => {
                eprintln!("ChunkManager: Received GenerationFailed for {:?}: {}", pos, err);
                let mut states = self.chunk_states.write().unwrap();
                states.insert(pos, ChunkGenState::Unknown); // Reset state
            }
            ChunkResult::LogMessage(msg) => {
                // Log messages received from worker threads
                eprintln!("Log from Worker: {}", msg); // Or godot_print!
                // No state change needed for log messages
            }
        }
    }

    // Generation logic (runs on compute pool)
    fn generate_and_save_chunk(
        pos: ChunkPosition,
        storage: Arc<ChunkStorage>,
        biome_data: Arc<ThreadSafeBiomeData>,
        // Accept function cache handle
        noise_functions_cache_handle: Arc<RwLock<HashMap<String, Arc<dyn NoiseFn<f64, 2> + Send + Sync>>>>,
        chunk_size: u32,
        sender: Sender<ChunkResult>,
        amplification: f64,
    ) {
        let chunk_area = (chunk_size * chunk_size) as usize;
        let mut heightmap = vec![0.0f32; chunk_area];
        let mut biome_ids_primary = vec![0u8; chunk_area]; // Store the *primary* biome for potential later use/debugging

        let noise_funcs_reader = noise_functions_cache_handle.read().unwrap();
        // Pre-fetch blend noise function Arc
        let blend_noise_fn_arc = noise_funcs_reader.get("biome_blend").cloned();

        for z in 0..chunk_size {
            for x in 0..chunk_size {
                let idx = (z * chunk_size + x) as usize;
                let world_x = pos.x as f32 * chunk_size as f32 + x as f32;
                let world_z = pos.z as f32 * chunk_size as f32 + z as f32;

                // Get biome influences
                let influences = biome_data.get_biome_id_and_weights(world_x, world_z);

                if influences.is_empty() {
                    heightmap[idx] = 0.0; // Should not happen if get_biome_id_and_weights handles defaults
                    biome_ids_primary[idx] = 0;
                    continue;
                }
    
                // Store primary biome (highest weight)
                biome_ids_primary[idx] = influences.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal)).map_or(0, |(id, _)| *id);
    
                let mut final_height = 0.0;
                let mut total_weight = 0.0;
    
                for &(biome_id, weight) in &influences {
                    if weight < 1e-3 { continue; } // Skip negligible influence
   
                    let biome_key = format!("{}", biome_id);
                    if let Some(noise_fn_arc) = noise_funcs_reader.get(&biome_key) {
                        let height_val = noise_fn_arc.get([world_x as f64, world_z as f64]);
                        // Apply amplification *per biome noise source* before interpolation
                        final_height += (height_val * amplification) as f32 * weight;
                        total_weight += weight;
                    } else {
                        println!("Warning: Noise function for biome key '{}' not found during generation.", biome_key);
                        // How to handle missing noise? Add 0 height? Skip weight?
                        // Adding 0 height might be okay if weights don't sum to 1.
                        // If weights *should* sum to 1, this needs careful handling.
                        // Let's assume for now it contributes 0 height but keep its weight.
                        total_weight += weight;
                    }
               }
   
   
                // Normalize height if weights summed correctly, otherwise use as is
                if total_weight > 1e-3 {
                    // If weights were designed to sum to 1, normalization might not be needed,
                    // but it adds robustness if they don't perfectly sum due to float issues or missing noise.
                    heightmap[idx] = final_height / total_weight;
                } else if influences.len() == 1 && influences[0].0 == 0 {
                    // If the only influence is biome 0 (Unknown/Fallback)
                    heightmap[idx] = 0.0; // Explicitly set to 0
                }
                else {
                    heightmap[idx] = 0.0; // Default fallback if no valid weights/noise found
                }
           }
       }
       drop(noise_funcs_reader); // Drop read lock


        // Create ChunkData (unchanged)
        let chunk_data = ChunkData { 
            position: pos, 
            heightmap, 
            biome_ids: biome_ids_primary, 
        };

        // Save and Send Result (unchanged)
        storage.queue_save_chunk(chunk_data.clone());
        if let Err(e) = sender.send(ChunkResult::Generated(pos, chunk_data)) {
            eprintln!("ChunkManager Worker: Failed to send Generated result for {:?}: {}", pos, e);
        }
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

    /// Attempts to retrieve ChunkData (including height, biomes, and potentially mesh)
    /// directly from the underlying storage cache.
    /// Returns None if the chunk is not ready or not found in the cache.
    pub fn get_cached_chunk_data(&self, position_x: i32, position_z: i32) -> Option<ChunkData> {
        let pos = ChunkPosition { x: position_x, z: position_z };
        // Check readiness state first (optional, but good practice)
        // Note: This read lock is brief
        let is_ready = matches!(
            self.chunk_states.read().unwrap().get(&pos),
            Some(ChunkGenState::Ready(_))
        );

        if is_ready {
            // Access storage cache directly
             self.storage.get_data_from_cache(pos)
        } else {
            None // Not ready, so definitely not in cache in a usable state
        }
    }

    // Called by ChunkController to update based on player position
    #[func]
    pub fn update(&self, player_x: f32, _player_y: f32, player_z: f32) {
        
        let player_chunk_x = (player_x / self.chunk_size as f32).floor() as i32;
        let player_chunk_z = (player_z / self.chunk_size as f32).floor() as i32;
        println!("ChunkManager: update at: {:?}, {:?}", player_chunk_x, player_chunk_z);
        
        let mut required_chunks = HashSet::new();
        for x in (player_chunk_x - self.render_distance)..=(player_chunk_x + self.render_distance) {
            for z in (player_chunk_z - self.render_distance)..=(player_chunk_z + self.render_distance) {
                let pos = ChunkPosition { x, z };
                required_chunks.insert(pos);
                self.ensure_chunk_is_ready(pos); // Request load/generation if needed
            }
        }
        
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

        if !self.is_chunk_ready(position_x, position_z) {
            return PackedFloat32Array::new();
        }

        // Use the new direct cache access method in ChunkStorage
        match self.storage.get_data_from_cache(pos) {
            Some(chunk_data) => PackedFloat32Array::from(&chunk_data.heightmap[..]),
            None => {
                eprintln!("CRITICAL: Chunk {:?} state is Ready, but data not found in storage cache!", pos);
                PackedFloat32Array::new()
            }
        }
    }

    #[func]
    pub fn get_chunk_biomes(&self, position_x: i32, position_z: i32) -> PackedInt32Array {
        let pos = ChunkPosition { x: position_x, z: position_z };

        if !self.is_chunk_ready(position_x, position_z) {
            return PackedInt32Array::new();
        }

        // Use the new direct cache access method in ChunkStorage
        match self.storage.get_data_from_cache(pos) {
            Some(chunk_data) => {
                let biomes_i32: Vec<i32> = chunk_data.biome_ids.iter().map(|&id| id as i32).collect();
                PackedInt32Array::from(&biomes_i32[..])
            },
            None => {
                eprintln!("CRITICAL: Chunk {:?} state is Ready, but biome data not found in storage cache!", pos);
                PackedInt32Array::new()
            }
        }
    }

    // Function used foor debugging together with godot terrainDebugger node
    #[func]
    pub fn get_chunk_state_at(&self, chunk_x: i32, chunk_z: i32) -> i32 {
        let pos = ChunkPosition { x: chunk_x, z: chunk_z };
        match self.chunk_states.read().unwrap().get(&pos) {
            Some(ChunkGenState::Unknown) => 0,
            Some(ChunkGenState::Loading) => 1,
            Some(ChunkGenState::Generating) => 2,
            Some(ChunkGenState::Ready(_)) => 3,
            None => -1, // Not tracked
        }
    }

    // Function to get data specifically at world coords
    // This might need refinement based on how you want to sample data precisely
    #[func]
    pub fn get_terrain_data_at(&self, world_x: f32, world_z: f32) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.insert("world_x", world_x.to_variant());
        dict.insert("world_z", world_z.to_variant());

        // Find chunk coords
        let chunk_x = (world_x / self.chunk_size as f32).floor() as i32;
        let chunk_z = (world_z / self.chunk_size as f32).floor() as i32;
        dict.insert("chunk_x", chunk_x.to_variant());
        dict.insert("chunk_z", chunk_z.to_variant());

        let pos = ChunkPosition { x: chunk_x, z: chunk_z };

        // Get chunk state
        dict.insert("chunk_state", self.get_chunk_state_at(chunk_x, chunk_z).to_variant());

        // Try to get height and biome from cache if ready
        if let Some(data) = self.get_cached_chunk_data(chunk_x, chunk_z) {
            // Calculate exact index within the chunk's heightmap/biomemap
            let local_x = (world_x - (chunk_x as f32 * self.chunk_size as f32)).floor() as u32;
            let local_z = (world_z - (chunk_z as f32 * self.chunk_size as f32)).floor() as u32;
            let idx = (local_z.clamp(0, self.chunk_size -1) * self.chunk_size
                   + local_x.clamp(0, self.chunk_size -1)) as usize;

            if idx < data.heightmap.len() {
                dict.insert("height", data.heightmap[idx].to_variant());
            } else {
                 dict.insert("height", Variant::nil()); // Index out of bounds
            }
             if idx < data.biome_ids.len() {
                 dict.insert("primary_biome_id", (data.biome_ids[idx] as i32).to_variant());
                 // TODO: Get biome name from BiomeManager if needed
             } else {
                 dict.insert("primary_biome_id", Variant::nil());
             }
             // TODO: Potentially add biome weights here if ChunkData stores them
        } else {
            dict.insert("height", Variant::nil());
            dict.insert("primary_biome_id", Variant::nil());
        }

        // TODO: Get Section ID / Weights from BiomeManager if needed
        // You might need a direct reference or call into BiomeManager here
        // if let Some(bm) = &self.biome_manager {
        //     let mut bm_bind = bm.bind_mut(); // May need mut if it uses cache
        //     dict.insert("section_id", bm_bind.get_section_id(world_x, world_z).to_variant());
        //     // Add weights etc.
        // }

        dict
    }

    #[func]
    pub fn get_chunk_count(&self) -> i32 {
        self.chunk_states.read().unwrap().len() as i32
    }

    #[func]
    pub fn shutdown(&mut self) {
        eprintln!("ChunkManager: Initiating explicit shutdown sequence...");
        // If we have unique ownership of the storage Arc, we can call shutdown
        if let Some(storage_mut) = Arc::get_mut(&mut self.storage) {
            storage_mut.shutdown();
        } else {
            eprintln!("ChunkManager: Cannot get exclusive access to ChunkStorage for explicit shutdown");
        }
    }

    #[func]
    pub fn set_render_distance(&mut self, distance: i32) {
        let new_distance = distance.max(1).min(32); // Clamp
        if new_distance != self.render_distance{
            self.render_distance = new_distance;
            println!("ChunkManager: Render distance set to {}", self.render_distance);
            // Trigger an unload check immediately? Optional.
        }
    }

    #[func]
    pub fn get_render_distance(&self) -> i32 {
        self.render_distance
    }

    #[func]
    pub fn set_biome_manager(&mut self, biome_manager: Gd<BiomeManager>) {
        println!("ChunkManager: BiomeManager reference set.");
        self.biome_manager = Some(biome_manager);
        self.update_thread_safe_biome_data(); // Update data immediately
    }

    // Update thread-safe biome data cache
    #[func]
    pub fn update_thread_safe_biome_data(&mut self) {
        if let (Some(biome_mgr_gd), Some(noise_mgr_gd)) = (&self.biome_manager, &self.noise_manager) {
            let biome_mgr_bind = biome_mgr_gd.bind();
            let noise_mgr_bind = noise_mgr_gd.bind(); // Bind noise manager
    
            if biome_mgr_bind.is_fully_initialized() {
                println!("ChunkManager: Updating thread-safe biome data cache using BiomeManager and NoiseManager.");
    
                let mut current_data_guard = self.thread_safe_biome_data.write().unwrap();
    
                if let Some(ref mut existing_data_arc) = *current_data_guard {
                    // Try to get mutable access to update existing data efficiently
                    if let Some(existing_data_mut) = Arc::get_mut(existing_data_arc) {
                        existing_data_mut.update_from_biome_manager(&biome_mgr_bind, &noise_mgr_bind);
                    } else {
                        // If shared elsewhere, clone and update (less efficient)
                        let mut cloned_data = (**existing_data_arc).clone(); // Requires ThreadSafeBiomeData to derive Clone
                        cloned_data.update_from_biome_manager(&biome_mgr_bind, &noise_mgr_bind);
                        *existing_data_arc = Arc::new(cloned_data);
                    }
                } else {
                     // Create new data if none exists
                    let new_data = Arc::new(ThreadSafeBiomeData::from_biome_manager(&biome_mgr_bind, &noise_mgr_bind));
                    *current_data_guard = Some(new_data);
                }
    
    
            } else {
                eprintln!("ChunkManager: Attempted to update biome data, but BiomeManager is not ready.");
            }
        } else {
            eprintln!("ChunkManager: Cannot update biome data, BiomeManager or NoiseManager reference missing.");
        }
    }

    // Apply config changes dynamically
    #[func]
    pub fn apply_config_updates(&mut self) {
    let config_arc:&'static Arc<RwLock<TerrainConfig>> = TerrainConfigManager::get_config(); // Get static ref
    if let Ok(guard) = config_arc.read() { // Lock it
        let old_chunk_size = self.chunk_size;
        self.chunk_size = guard.chunk_size; // Access field
        // REMOVED: self.storage.update_cache_limit();
        println!("ChunkManager: Applied config updates (chunk_size: {})", self.chunk_size);
        if old_chunk_size != self.chunk_size {
            eprintln!("ChunkManager: Chunk size changed! Clearing all chunk states and storage cache. Chunks will regenerate.");
            self.chunk_states.write().unwrap().clear();
            self.storage.clear_cache(); // Make sure clear_cache exists or remove if LRU handles it
        }
        } else {
            eprintln!("ChunkManager::apply_config_updates: Failed to read terrain config lock.");
        }
    }
}

impl Drop for ChunkManager {
    fn drop(&mut self) {
        println!("ChunkManager: Dropping. Shutting down ChunkStorage IO thread...");
        // Since storage is an Arc, we can only call shutdown if we have unique ownership
        if let Some(storage_mut) = Arc::get_mut(&mut self.storage) {
            storage_mut.shutdown();
        } else {
            eprintln!("ChunkManager::drop: Cannot get mutable access to ChunkStorage for shutdown (still shared). IO thread may not stop cleanly.");
        }
    }
}
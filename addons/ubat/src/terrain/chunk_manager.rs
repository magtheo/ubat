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
    noise_params_cache: Option<Arc<RwLock<HashMap<String, NoiseParameters>>>>,

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
        let (tx, rx) = channel(); // Create the channel
        let storage = Arc::new(ChunkStorage::new("user://terrain_data", tx.clone()));
        let compute_pool = get_or_init_global_pool(); // Use global pool

        let config_arc:&'static Arc<RwLock<TerrainConfig>> = TerrainConfigManager::get_config(); // Get static ref
        let chunk_size = match config_arc.read() { // Lock it
            Ok(guard) => guard.chunk_size, // Access field
            Err(_) => {
                godot_error!("ChunkManager::init: Failed to read terrain config lock for chunk size. Using default 32.");
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
            noise_params_cache: None, // Initialize as None
            render_distance: 2, // TODO This overides terrain initalizer, and it shuold not
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
                } else {
                    godot_error!("ChunkManager: Found 'BiomeManager', but it's not initialized yet.");
                    // biome_manager remains None
                }
            } else {
                godot_error!("ChunkManager: Could not find node 'BiomeManager' under parent.");
                // biome_manager remains None
            }

            // --- Link NoiseManager ---
            let noise_manager_node = parent.get_node_as::<NoiseManager>("NoiseManager"); // Adjust path if needed
            if noise_manager_node.is_instance_valid() {
                godot_print!("ChunkManager: Linking NoiseManager");
                self.noise_manager = Some(noise_manager_node);
                // Get the Arc handle from NoiseManager
                // self.noise_params_cache = Some(noise_manager_node.bind().get_noise_cache_handle());
            } else {
                 godot_error!("ChunkManager: Could not find node 'NoiseManager'. Noise parameters will be unavailable.");
                 self.noise_params_cache = None;
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
                            godot_print!("ChunkManager: Received Loaded result for {:?}", pos);
                        },
                        ChunkResult::Generated(pos, _) => {
                            godot_print!("ChunkManager: Received Generated result for {:?}", pos);
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
    
        let chunk_size = self.chunk_size;
        let sender_clone = self.result_sender.clone();
    
        // Fetch the noise parameter cache for height generation (still needed)
        let noise_cache_handle = match &self.noise_manager {
            Some(nm_gd) => Some(nm_gd.bind().get_noise_cache_handle()),
            None => {
                eprintln!("ChunkManager: Cannot queue generation for {:?}, NoiseManager is not available.", pos);
                let _ = sender_clone.send(ChunkResult::GenerationFailed(pos, "NoiseManager unavailable".to_string()));
                return;
            }
        };
        // We already checked noise_manager is Some, so noise_cache_handle should also be Some
         let noise_cache_handle = noise_cache_handle.unwrap();
    
    
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
                noise_cache_handle, // Pass the noise cache handle for heights
                chunk_size,
                sender_clone,
            );
        });
    }

    fn handle_chunk_result(&mut self, result: ChunkResult) {
        // Lock states only when modification is needed
        match result {
            ChunkResult::Loaded(pos, _data) => {
                let mut states = self.chunk_states.write().unwrap();
                godot_print!("ChunkManager: Setting state Ready for loaded chunk {:?}", pos);
                states.insert(pos, ChunkGenState::Ready(Instant::now()));
            }
            ChunkResult::LoadFailed(pos) => {
                let mut states = self.chunk_states.write().unwrap();
                match states.get(&pos) {
                    Some(ChunkGenState::Loading) => {
                        godot_print!("ChunkManager: LoadFailed for {:?} - state is correctly Loading, changing to Generating", pos);
                        states.insert(pos, ChunkGenState::Generating);
                        drop(states); // Drop lock BEFORE queueing
                        self.queue_generation(pos);
                    },
                    other_state => {
                        godot_warn!("ChunkManager: Received LoadFailed for {:?} but state was not Loading: {:?}",
                                   pos, other_state);
                        states.insert(pos, ChunkGenState::Unknown); // Reset state
                    }
                }
            }
            ChunkResult::Generated(pos, _data) => {
                 let mut states = self.chunk_states.write().unwrap();
                 godot_print!("ChunkManager: Received Generated for {:?}, setting Ready.", pos);
                 states.insert(pos, ChunkGenState::Ready(Instant::now()));
            }
            ChunkResult::GenerationFailed(pos, err) => {
                 godot_error!("ChunkManager: Received GenerationFailed for {:?}: {}", pos, err);
                 let mut states = self.chunk_states.write().unwrap();
                 states.insert(pos, ChunkGenState::Unknown); // Reset state
            }
            // **FIXED:** Handle LogMessage here
            ChunkResult::LogMessage(msg) => {
                // Log messages received from worker threads
                godot_warn!("Log from Worker: {}", msg); // Or godot_print!
                // No state change needed for log messages
            }
        }
    }

    // Generation logic (runs on compute pool)
    fn generate_and_save_chunk(
        pos: ChunkPosition,
        storage: Arc<ChunkStorage>,
        biome_data: Arc<ThreadSafeBiomeData>, // Changed type
        noise_params_cache_handle: Arc<RwLock<HashMap<String, NoiseParameters>>>, // Renamed for clarity
        chunk_size: u32,
        sender: Sender<ChunkResult>,
    ) {
        // No need to get biome_data again, it's passed directly
    
        // --- Generation ---
        let chunk_area = (chunk_size * chunk_size) as usize;
        let mut heightmap = vec![0.0f32; chunk_area];
        let mut biome_ids = vec![0u8; chunk_area];
    
        // Lock noise cache for height params
        let noise_cache_reader = noise_params_cache_handle.read().unwrap();
    
        for z in 0..chunk_size {
            for x in 0..chunk_size {
                let idx = (z * chunk_size + x) as usize;
                let world_x = pos.x as f32 * chunk_size as f32 + x as f32;
                let world_z = pos.z as f32 * chunk_size as f32 + z as f32;
    
                // *** Use the enhanced get_biome_id from the passed biome_data Arc ***
                let biome_id = biome_data.get_biome_id(world_x, world_z);
                biome_ids[idx] = biome_id;
    
                // --- Height generation using noise cache (remains the same) ---
                let biome_key = format!("{}", biome_id);
                if let Some(params) = noise_cache_reader.get(&biome_key) {
                    // Use Self::create_noise_function (the one for heights)
                    let noise_fn = Self::create_noise_function(params);
                    let height_val = noise_fn.get([world_x as f64, world_z as f64]);
                    // TODO: Use biome-specific scaling from biome_data or parameters
                    let height_scale = 15.0; // Example scale
                    heightmap[idx] = (height_val * height_scale) as f32;
                } else {
                    // ... handle missing height noise params ...
                    heightmap[idx] = 0.0;
                }
            }
        }
        drop(noise_cache_reader); // Drop lock
    
        // --- Blend heights (remains the same, uses generated biome_ids) ---
        let blend_noise_params_for_heights = noise_params_cache_handle.read().unwrap().get("blend").cloned(); // Use blend params for height smoothing too?
        Self::blend_heights(&mut heightmap, &biome_ids, chunk_size, biome_data.blend_distance(), blend_noise_params_for_heights, pos);
    
        // --- Save and Send Result ---
        let heightmap_vec = heightmap;
        let biome_ids_vec = biome_ids;
        storage.queue_save_chunk(pos, &heightmap_vec, &biome_ids_vec);
        let chunk_data = ChunkData { position: pos, heightmap: heightmap_vec, biome_ids: biome_ids_vec };
        if let Err(e) = sender.send(ChunkResult::Generated(pos, chunk_data)) {
             eprintln!("Error sending Generated result for {:?}: {}", pos, e);
        }
    }


    // --- Helper function to create noise-rs NoiseFn ---
    // (Needs careful implementation based on NoiseParameters struct and noise-rs library)
    fn create_noise_function(params: &NoiseParameters) -> Box<dyn noise::NoiseFn<f64, 2> + Send + Sync> {
        // Import necessary items INSIDE the function
        use noise::{NoiseFn, Fbm, Perlin, Billow, RidgedMulti, ScalePoint, MultiFractal}; // Added Perlin here
        use crate::terrain::noise::noise_parameters::{RustNoiseType, RustFractalType};

        // Determine the base noise type explicitly
        // We default to Perlin here based on previous logic, adjust if needed
        // Note: noise-rs doesn't have built-in Simplex, consider external crates or fallback
        let base_noise_generator = Perlin::new(params.seed as u32);

        // Create the final noise function, potentially wrapping the base with fractals
        let final_noise: Box<dyn NoiseFn<f64, 2> + Send + Sync> = match params.fractal_type {
            RustFractalType::Fbm => {
                // **FIXED:** Specify <Perlin> for Fbm
                Box::new(Fbm::<Perlin>::new(params.seed as u32)
                    .set_frequency(params.frequency as f64)
                    .set_octaves(params.fractal_octaves as usize)
                    .set_lacunarity(params.fractal_lacunarity as f64)
                    .set_persistence(params.fractal_gain as f64))
            }
            RustFractalType::Ridged => {
                 // **FIXED:** Specify <Perlin> for RidgedMulti
                Box::new(RidgedMulti::<Perlin>::new(params.seed as u32)
                    .set_frequency(params.frequency as f64)
                    .set_octaves(params.fractal_octaves as usize)
                    .set_lacunarity(params.fractal_lacunarity as f64))
                    // Add .set_attenuation() if needed based on Godot's Ridged settings
            }
            RustFractalType::PingPong => {
                 // **FIXED:** Specify <Perlin> for Billow
                Box::new(Billow::<Perlin>::new(params.seed as u32)
                    .set_frequency(params.frequency as f64)
                    .set_octaves(params.fractal_octaves as usize)
                    .set_lacunarity(params.fractal_lacunarity as f64)
                    .set_persistence(params.fractal_gain as f64))
            }
            RustFractalType::None => {
                 // No fractal, just use the base noise scaled by frequency
                 Box::new(ScalePoint::new(base_noise_generator).set_scale(params.frequency as f64))
            }
        };

        // TODO: Apply Offset - Example: Add offset AFTER getting noise value in generate_and_save_chunk
        // TODO: Apply Domain Warp - Wrap `final_noise` with noise-rs domain warp modules if needed

        final_noise
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
    fn blend_heights(
        heightmap: &mut [f32],
        biome_ids: &[u8],
        chunk_size: u32,
        blend_distance: i32,
        blend_noise_params: Option<NoiseParameters>,
        chunk_pos: ChunkPosition, // **ADDED chunk_pos** to calculate world coords
    ) {
        if blend_distance <= 0 { return; }

        let original_heights = heightmap.to_vec();
        let blend_radius = blend_distance.max(1);

        let blend_noise_fn = blend_noise_params.as_ref().map(Self::create_noise_function);

        for z in 0..chunk_size {
            for x in 0..chunk_size {
                let idx = (z * chunk_size + x) as usize;
                let current_biome = biome_ids[idx];
                let mut is_boundary = false;

                // Check neighbors
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

                if is_boundary {
                    let mut total_weight = 0.0;
                    let mut weighted_height_sum = 0.0;
                    let mut blend_needed = false;

                    for dz in -blend_radius..=blend_radius {
                        for dx in -blend_radius..=blend_radius {
                            let nx = x as i32 + dx;
                            let nz = z as i32 + dz;

                            if nx >= 0 && nx < chunk_size as i32 && nz >= 0 && nz < chunk_size as i32 {
                                let nidx = (nz as u32 * chunk_size + nx as u32) as usize;
                                let distance = ((dx * dx + dz * dz) as f32).sqrt();
                                let mut weight = (blend_radius as f32 - distance).max(0.0) / blend_radius as f32;

                                if let Some(ref noise_fn) = blend_noise_fn {
                                    // **FIXED:** Calculate world coords for consistent blend noise
                                    let world_x = chunk_pos.x as f32 * chunk_size as f32 + nx as f32;
                                    let world_z = chunk_pos.z as f32 * chunk_size as f32 + nz as f32;
                                    // Use a different scale for blend noise if desired
                                    let noise_val = noise_fn.get([world_x as f64 * 0.01, world_z as f64 * 0.01]);
                                    let noise_influence = (noise_val * 0.4) as f32; // Example influence range
                                    weight = (weight + noise_influence).clamp(0.0, 1.0);
                                }

                                if weight > 0.001 {
                                    total_weight += weight;
                                    weighted_height_sum += original_heights[nidx] * weight;
                                    blend_needed = true;
                                }
                            }
                        }
                    }

                    if blend_needed && total_weight > 0.001 {
                        heightmap[idx] = weighted_height_sum / total_weight;
                    }
                }
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
                godot_error!("CRITICAL: Chunk {:?} state is Ready, but data not found in storage cache!", pos);
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
                godot_error!("CRITICAL: Chunk {:?} state is Ready, but biome data not found in storage cache!", pos);
                PackedInt32Array::new()
            }
        }
    }

    #[func]
    pub fn get_chunk_count(&self) -> i32 {
        self.chunk_states.read().unwrap().len() as i32
    }

    #[func]
    pub fn shutdown(&mut self) {
        godot_print!("ChunkManager: Initiating explicit shutdown sequence...");
        // If we have unique ownership of the storage Arc, we can call shutdown
        if let Some(storage_mut) = Arc::get_mut(&mut self.storage) {
            storage_mut.shutdown();
        } else {
            godot_warn!("ChunkManager: Cannot get exclusive access to ChunkStorage for explicit shutdown");
        }
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
        if let (Some(biome_mgr_gd), Some(noise_mgr_gd)) = (&self.biome_manager, &self.noise_manager) {
            let biome_mgr_bind = biome_mgr_gd.bind();
            let noise_mgr_bind = noise_mgr_gd.bind(); // Bind noise manager
    
            if biome_mgr_bind.is_fully_initialized() {
                godot_print!("ChunkManager: Updating thread-safe biome data cache using BiomeManager and NoiseManager.");
    
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
                godot_warn!("ChunkManager: Attempted to update biome data, but BiomeManager is not ready.");
            }
        } else {
            godot_warn!("ChunkManager: Cannot update biome data, BiomeManager or NoiseManager reference missing.");
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
        godot_print!("ChunkManager: Applied config updates (chunk_size: {})", self.chunk_size);
        if old_chunk_size != self.chunk_size {
            godot_warn!("ChunkManager: Chunk size changed! Clearing all chunk states and storage cache. Chunks will regenerate.");
            self.chunk_states.write().unwrap().clear();
            self.storage.clear_cache(); // Make sure clear_cache exists or remove if LRU handles it
        }
        } else {
            godot_error!("ChunkManager::apply_config_updates: Failed to read terrain config lock.");
        }
    }
}

impl Drop for ChunkManager {
    fn drop(&mut self) {
        godot_print!("ChunkManager: Dropping. Shutting down ChunkStorage IO thread...");
        // Since storage is an Arc, we can only call shutdown if we have unique ownership
        if let Some(storage_mut) = Arc::get_mut(&mut self.storage) {
            storage_mut.shutdown();
        } else {
            godot_warn!("ChunkManager::drop: Cannot get mutable access to ChunkStorage for shutdown (still shared). IO thread may not stop cleanly.");
        }
    }
}
use godot::prelude::*;
use godot::classes::{Node3D}; // Need FastNoiseLite for generation
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
use crate::terrain::terrain_config::{TerrainConfigManager, TerrainConfig};
use crate::terrain::section::{SectionManager, ThreadSafeSectionData};

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
    section_manager: Option<Gd<SectionManager>>,
    noise_manager: Option<Gd<NoiseManager>>, // Add this
    thread_safe_section_data: Arc<RwLock<Option<Arc<ThreadSafeSectionData>>>>,
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
            section_manager: None,
            noise_manager: None,
            thread_safe_section_data: Arc::new(RwLock::new(None)),
            is_thread_safe_data_ready: false,
            noise_functions_cache: None, // Initialize as None
            render_distance: 4, // TODO This overides terrain initalizer, and it shuold not
            chunk_size,
            last_unload_check: Instant::now(),
        }
    }

    fn ready(&mut self) {
        let start_time = std::time::Instant::now();
        println!("ChunkManager: Ready. Linking SectionManager...");
        self.section_manager = None; // Ensure starts as None

        if let Some(parent) = self.base().get_parent() {
            // --- Use string literal for get_node_as ---
            let section_manager_node = parent.get_node_as::<SectionManager>("SectionManager");
            if section_manager_node.is_instance_valid() {
                if section_manager_node.bind().is_fully_initialized() {
                    println!("ChunkManager: SectionManager is initialized.");
                    // Assign if ready (original simple assignment should work now)
                    self.section_manager = Some(section_manager_node.clone());
                } else {
                    eprintln!("ChunkManager: Found 'SectionManager', but it's not initialized yet.");
                    // section_manager remains None
                }
            } else {
                eprintln!("ChunkManager: Could not find node 'SectionManager' under parent.");
                // section_manager remains None
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
            // section_manager remains None
        }

        // Ensure chunk size matches latest config
        self.apply_config_updates();

        println!("ChunkManager: Ready completed in {}ms", start_time.elapsed().as_millis());
    }

    fn process(&mut self, _delta: f64) {
        
        // --- Initialization Check (at the beginning of process) ---
        if !self.is_thread_safe_data_ready {
            // Attempt to initialize/update if managers seem ready
            if self.section_manager.is_some() && self.noise_manager.is_some() {
                // Call the update function (it has internal checks for None)
                self.update_thread_safe_section_data();

                // Check if the update was successful by reading the value
                if self.thread_safe_section_data.read().unwrap().is_some() {
                    println!("ChunkManager: ThreadSafeSectionData successfully initialized/updated in process."); // Use println!
                    self.is_thread_safe_data_ready = true; // Set flag only on success
                } else {
                    // Optional: Log that managers are present but data update failed
                    // eprintln!("ChunkManager process: Managers linked, but ThreadSafeSectionData update still results in None.");
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
                        // godot_print!("ChunkManager process: Processed {} results this frame", result_count);
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
        // Consider initialized if section data is available
        self.thread_safe_section_data.read().unwrap().is_some()
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
        // --- Clone the Arc containing the Option<Arc<ThreadSafeSectionData>> ---
        let section_data_rwlock_arc = Arc::clone(&self.thread_safe_section_data);
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
            // --- Read the SectionData Arc INSIDE the worker ---
            let section_data_guard = section_data_rwlock_arc.read().unwrap();
            // Clone the inner Arc<ThreadSafeSectionData> if it exists
            let section_data_clone = match *section_data_guard {
                Some(ref arc) => Some(Arc::clone(arc)),
                None => None,
            };
            // Drop the read guard quickly
            drop(section_data_guard);
    
            // --- Check if section data is available ---
            if section_data_clone.is_none() {
                let err_msg = format!("SectionData unavailable for generation task at {:?} (ChunkManager's shared data is None)", pos);                // Send the original String
                let _ = sender_clone.send(ChunkResult::GenerationFailed(pos, err_msg.clone())); // Clone err_msg here
                // Log using the original still-owned string
                eprintln!("{}", err_msg);
                return;
            }
            
            // We know it's Some now
            let section_data = section_data_clone.unwrap();
    
    
            Self::generate_and_save_chunk(
                pos,
                storage_clone,
                section_data, // Pass the Arc<ThreadSafeSectionData>
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
                // godot_print!("Log from Worker: {}", msg); // Or godot_print!
                // No state change needed for log messages
            }
        }
    }

    // Generation logic (runs on compute pool)
    fn generate_and_save_chunk(
        pos: ChunkPosition,
        storage: Arc<ChunkStorage>,
        section_data: Arc<ThreadSafeSectionData>, // Receives Arc<ThreadSafeSectionData>
        noise_functions_cache_handle: Arc<RwLock<HashMap<String, Arc<dyn NoiseFn<f64, 2> + Send + Sync>>>>,
        chunk_size: u32,
        sender: Sender<ChunkResult>,
        amplification: f64, // Passed from caller (queue_generation)
    ) {
        let grid_width = chunk_size + 1;
        let vertex_count = (grid_width * grid_width) as usize;

        let mut heightmap = vec![0.0f32; vertex_count];
        let mut biome_indices_data = vec![[0u8; 3]; vertex_count]; // Top 3 biome IDs for texturing
        let mut biome_weights_data = vec![[0.0f32; 3]; vertex_count]; // Blend weights for top 3

        let noise_funcs_reader = noise_functions_cache_handle.read().unwrap();
        
        // Get blend_noise_strength from section_data (it now holds it)
        let blend_noise_strength = section_data.blend_noise_strength;

        for iz in 0..grid_width {
            for ix in 0..grid_width {
                let current_vertex_idx = (iz * grid_width + ix) as usize;
                let world_x = pos.x as f32 * chunk_size as f32 + ix as f32;
                let world_z = pos.z as f32 * chunk_size as f32 + iz as f32;

                // 1. Get initial biome influences (already considers sections and Voronoi falloff)
                // These influences are effectively [(biome_id, pre_noise_modulated_weight)]
                let initial_influences = section_data.get_biome_id_and_weights(world_x, world_z, &sender);

                if initial_influences.is_empty() {
                    let _ = sender.send(ChunkResult::LogMessage(format!("generate_and_save_chunk: No initial influences at ({:.1}, {:.1}), chunk {:?}. Defaulting.", world_x, world_z, pos)));
                    heightmap[current_vertex_idx] = 0.0;
                    biome_indices_data[current_vertex_idx] = [0, 0, 0]; // Default biome 0
                    biome_weights_data[current_vertex_idx] = [1.0, 0.0, 0.0];
                    continue;
                }

                // 2. Create `effective_influences` by modulating with biome_blend_noise
                let mut effective_influences = Vec::with_capacity(initial_influences.len());
                let mut total_effective_weight_for_modulation = 0.0;

                for (biome_id, original_weight) in &initial_influences {
                    if *original_weight < 1e-5 { continue; } // Skip truly negligible original weights

                    let mut current_effective_weight = *original_weight;

                    if let Some(blend_noise_fn) = &section_data.biome_blend_noise_fn {
                        if blend_noise_strength > 1e-5 { // Only apply if strength is meaningful
                            let noise_val = blend_noise_fn.get([world_x as f64, world_z as f64]) as f32; // -1 to 1
                            // Modulate: increase or decrease weight based on noise
                            // The (1.0 + noise_val * strength) factor can push weight > original_weight or < original_weight
                            // It's important original_weights sum to 1 (or are normalized later)
                            current_effective_weight *= (1.0 + noise_val * blend_noise_strength);
                            current_effective_weight = current_effective_weight.max(0.0); // Ensure non-negative
                        }
                    }
                    if current_effective_weight > 1e-5 { // Check after modulation
                        effective_influences.push((*biome_id, current_effective_weight));
                        total_effective_weight_for_modulation += current_effective_weight;
                    }
                }
                
                // Normalize these `effective_influences` so their weights sum to 1.0
                if total_effective_weight_for_modulation > 1e-5 {
                    for (_, weight) in effective_influences.iter_mut() {
                        *weight /= total_effective_weight_for_modulation;
                    }
                } else if !initial_influences.is_empty() {
                    // Fallback: if noise made all effective weights zero, use the strongest initial influence
                    effective_influences.clear();
                    // Sort initial_influences to get the strongest if not already sorted
                    // Assuming get_biome_id_and_weights already returns sorted or we sort it.
                    // For safety, let's just pick the first one if initial_influences was already sorted.
                    // If not, this might not be the "strongest". A proper sort would be needed here.
                    // Let's assume initial_influences[0] is the strongest for now.
                    if let Some(strongest_initial) = initial_influences.get(0) {
                        effective_influences.push(*strongest_initial);
                         // if total_effective_weight_for_modulation is 0, and we have a strongest_initial,
                         // its effective_weight must be 1.0 for that single entry.
                        if let Some(entry) = effective_influences.get_mut(0) {
                            entry.1 = 1.0;
                        }
                    } else {
                        // Ultra fallback if initial_influences became empty after filter
                        effective_influences.push((0, 1.0)); // Default to biome 0
                    }
                } else { // initial_influences was empty, so effective_influences is also empty
                    effective_influences.push((0, 1.0)); // Default to biome 0
                }


                // 3. Height Calculation using `effective_influences`
                let mut final_height_accumulator = 0.0;
                // let mut summed_weights_for_height = 0.0; // Not strictly needed if effective_influences sum to 1

                for (biome_id, effective_weight) in &effective_influences {
                    if *effective_weight < 1e-5 { continue; }
                    
                    // Fetch biome-specific noise for height
                    if let Some(noise_fn_arc) = noise_funcs_reader.get(&format!("{}", biome_id)) {
                        let height_val = noise_fn_arc.get([world_x as f64, world_z as f64]);
                        final_height_accumulator += (height_val * amplification) as f32 * effective_weight;
                        // summed_weights_for_height += effective_weight; // if not pre-normalized
                    } else {
                         // Log if noise not found but biome has weight
                         let _ = sender.send(ChunkResult::LogMessage(format!("generate_and_save_chunk: Height noise for biome {} not found at ({:.1}, {:.1}). Skipping its height contribution.", biome_id, world_x, world_z)));
                    }
                }
                heightmap[current_vertex_idx] = final_height_accumulator; // Directly assign as effective_weights sum to 1


                // 4. Texture Data Preparation using (sorted) `effective_influences`
                // Sort `effective_influences` by weight, descending, to pick top N
                effective_influences.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

                let top_n = 3;
                let mut temp_ids_for_shader = [0u8; 3];
                let mut temp_weights_for_shader = [0.0f32; 3];
                let mut sum_top_n_weights_for_shader: f32 = 0.0;

                for i in 0..top_n {
                    if i < effective_influences.len() && effective_influences[i].1 > 1e-5 {
                        temp_ids_for_shader[i] = effective_influences[i].0;
                        temp_weights_for_shader[i] = effective_influences[i].1;
                        sum_top_n_weights_for_shader += effective_influences[i].1;
                    } else {
                        temp_ids_for_shader[i] = 0; // Pad with default biome ID
                        temp_weights_for_shader[i] = 0.0;
                    }
                }

                // Normalize the top N weights specifically for the shader (so these 3 sum to 1)
                if sum_top_n_weights_for_shader > 1e-5 {
                    for i in 0..top_n {
                        biome_weights_data[current_vertex_idx][i] = temp_weights_for_shader[i] / sum_top_n_weights_for_shader;
                    }
                } else if !effective_influences.is_empty() && effective_influences[0].1 > 1e-5 {
                    // Fallback: if sum is tiny but there was a dominant biome, give it full weight
                    biome_weights_data[current_vertex_idx][0] = 1.0;
                    for i in 1..top_n { biome_weights_data[current_vertex_idx][i] = 0.0; }
                } else { // No significant influences at all
                    biome_weights_data[current_vertex_idx][0] = 1.0; // Default to biome 0
                    for i in 1..top_n { biome_weights_data[current_vertex_idx][i] = 0.0; }
                }
                biome_indices_data[current_vertex_idx] = temp_ids_for_shader;
            }
        }

        drop(noise_funcs_reader);

        let chunk_data = ChunkData {
            position: pos,
            heightmap,
            biome_indices: biome_indices_data,
            biome_blend_weights: biome_weights_data,
        };

        storage.queue_save_chunk(chunk_data.clone());
        if let Err(e) = sender.send(ChunkResult::Generated(pos, chunk_data)) {
           let _ = sender.send(ChunkResult::LogMessage(format!("ChunkManager Worker: Failed to send Generated result for {:?}: {}", pos, e)));
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
                let primary_biomes_i32: Vec<i32> = chunk_data.biome_indices.iter()
                    .map(|ids| ids[0] as i32) // Take the first ID (index 0)
                    .collect();
                PackedInt32Array::from(&primary_biomes_i32[..])
            },
            None => {
                eprintln!("CRITICAL: Chunk {:?} state is Ready, but section data not found in storage cache!", pos);
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

        // Try to get height and section from cache if ready
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
            if idx < data.biome_indices.len() {
                // Report primary biome ID
                dict.insert("primary_biome_id", (data.biome_indices[idx][0] as i32).to_variant());

                // Optionally add all top IDs and weights
                let ids_arr = PackedInt32Array::from(&data.biome_indices[idx].map(|id| id as i32)[..]);
                let weights_arr = PackedFloat32Array::from(&data.biome_blend_weights[idx][..]);
                dict.insert("top_biome_ids", ids_arr.to_variant());
                dict.insert("top_biome_weights", weights_arr.to_variant());

            } else {
                dict.insert("primary_biome_id", Variant::nil());
                dict.insert("top_biome_ids", Variant::nil());
                dict.insert("top_biome_weights", Variant::nil());
            }

             // TODO: Potentially add section weights here if ChunkData stores them
        } else {
            dict.insert("height", Variant::nil());
            dict.insert("primary_biome_id", Variant::nil());
        }

        // TODO: Get Section ID / Weights from SectionManager if needed
        // You might need a direct reference or call into SectionManager here
        // if let Some(bm) = &self.section_manager {
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
    pub fn set_section_manager(&mut self, section_manager: Gd<SectionManager>) {
        println!("ChunkManager: SectionManager reference set.");
        self.section_manager = Some(section_manager);
        self.update_thread_safe_section_data(); // Update data immediately
    }

    // Update thread-safe section data cache
    #[func]
    pub fn update_thread_safe_section_data(&mut self) {
        if let (Some(section_mgr_gd), Some(noise_mgr_gd)) = (&self.section_manager, &self.noise_manager) {
            let section_mgr_bind = section_mgr_gd.bind();
            
            if section_mgr_bind.is_fully_initialized() {
                println!("ChunkManager: Updating thread-safe section data cache using SectionManager and NoiseManager.");
                
                let mut current_data_guard = self.thread_safe_section_data.write().unwrap();
                
                // Create new data
                let new_data = ThreadSafeSectionData::from_section_manager(
                    &section_mgr_bind,
                    &noise_mgr_gd.bind()
                );
                
                *current_data_guard = Some(Arc::new(new_data));
            } else {
                eprintln!("ChunkManager: Attempted to update section data, but SectionManager is not ready.");
            }
        } else {
            eprintln!("ChunkManager: Cannot update section data, SectionManager or NoiseManager reference missing.");
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
use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use rayon::prelude::*;
use std::sync::mpsc::{Sender, Receiver, channel}; // Add import
use std::thread;
use std::panic::{catch_unwind, AssertUnwindSafe}; // Added for panic catching
use std::io::{Read, Write};
const FILE_READ: i32 = 1;  // This is typically the value for READ
const FILE_WRITE: i32 = 2; // This is typically the value for WRITE




use crate::threading::thread_pool::{ThreadPool, global_thread_pool};
use crate::terrain::chunk_manager::{ChunkPosition, ChunkResult};
use crate::terrain::terrain_config::{TerrainConfigManager, TerrainConfig};


// Enum to differentiate request types
#[derive(Debug)]
enum IORequestType {
    Load,
    Save(ChunkData), // Include the data to save
    Shutdown, // For graceful exit
}

// The actual request message structure
#[derive(Debug)]
struct IORequest {
    position: ChunkPosition,
    request_type: IORequestType,
}

// Data structure for serializing chunk data
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChunkData {
    pub position: ChunkPosition,
    pub heightmap: Vec<f32>,
    pub biome_ids: Vec<u8>,
    // Add other data as needed
}

struct LoadRequest {
    position: ChunkPosition,
    sender: Sender<ChunkResult>,
}


// ChunkStorage handles saving and loading chunks from disk
pub struct ChunkStorage {
    save_dir: String,
    cache: Arc<RwLock<HashMap<ChunkPosition, ChunkData>>>,
    cache_size_limit: Arc<RwLock<usize>>,
   
    result_sender: Sender<ChunkResult>, // Store a clone of the sender from ChunkManager
    io_request_sender: Option<Sender<IORequest>>, // To send requests TO IO thread
    io_thread_handle: Option<thread::JoinHandle<()>>, // Handle to the IO thread


}

// Helper function for cache eviction (independent function)
fn enforce_cache_limit(cache: &mut HashMap<ChunkPosition, ChunkData>, limit: usize) {
    while cache.len() > limit {
        if let Some(key_to_remove) = cache.keys().next().cloned() {
            cache.remove(&key_to_remove);
        } else {
            break; // Should not happen if len > 0
        }
    }
}

impl ChunkStorage {
    /// Creates a new ChunkStorage instance.
    /// - Initializes the cache.
    /// - Ensures the save directory exists.
    /// - Spawns a dedicated IO thread for loading and saving chunks.
    ///
    /// # Arguments
    /// * `save_dir` - The path to the directory where chunk files will be stored (e.g., "user://terrain_data").
    /// * `result_sender` - An `mpsc::Sender` to send loaded or failed chunk results back to the main thread (typically held by ChunkManager).
    pub fn new(save_dir: &str, result_sender: Sender<ChunkResult>) -> Self {
        println!("ChunkStorage: Initializing new storage with save_dir: {}", save_dir);
    
        // Ensure directory exists using standard Rust fs
        match fs::create_dir_all(save_dir) {
            Ok(_) => {
                println!("ChunkStorage: Save directory verified/created: {}", save_dir);
            }
            Err(e) => {
                eprintln!("ChunkStorage: ERROR - Failed to create save directory '{}': {}. Subsequent saves WILL likely fail.", save_dir, e);
            }
        }
    
        // Get cache limit from config
        let default_cache_limit = 128; // Sensible default if config fails
        let cache_limit = TerrainConfigManager::get_config()
            .and_then(|config_arc| {
                match config_arc.read() {
                    Ok(guard) => {
                        let limit = guard.chunk_cache_size();
                        Some(limit)
                    },
                    Err(e) => {
                        eprintln!("ChunkStorage: Failed to read TerrainConfig lock: {}. Using default cache limit.", e);
                        None // Fallback to default
                    }
                }
            })
            .unwrap_or_else(|| {
                eprintln!("ChunkStorage: TerrainConfig not available. Using default cache limit: {}", default_cache_limit);
                default_cache_limit
            });
    
        println!("ChunkStorage: Cache limit set to: {}", cache_limit);
    
        // Create the channel for sending requests TO the IO thread
        let (io_tx, io_rx): (Sender<IORequest>, Receiver<IORequest>) = channel();
    
        // Prepare shared data for the IO thread
        let cache_arc = Arc::new(RwLock::new(HashMap::<ChunkPosition, ChunkData>::new()));
        let limit_arc = Arc::new(RwLock::new(cache_limit)); // Store the resolved limit
    
        // Clone data needed *specifically* for the IO thread's continuous operation
        let save_dir_clone = save_dir.to_string();
        let result_sender_clone = result_sender.clone(); // Clone sender for results back to main
        let cache_arc_thread = Arc::clone(&cache_arc); // Clone Arc for thread access
        let limit_arc_thread = Arc::clone(&limit_arc); // Clone Arc for thread access
    
        println!("ChunkStorage: Spawning IO thread...");
    
        // Spawn the dedicated IO thread
        let handle = thread::spawn(move || {
            println!("IO Thread: <<< STARTED (stdout) >>>");
            println!("IO Thread: <<< STARTED (println) >>>");
    
            // Optional: Catch panics to prevent silent thread death and log the event.
            let result = catch_unwind(AssertUnwindSafe(|| {
                println!("IO Thread: Starting receiver loop...");
    
                // This loop runs until the sender (io_tx) is dropped (channel closed)
                // or until a Shutdown request is received.
                for request in io_rx { // io_rx moved into this closure, owned by the loop
                    println!("IO Thread: Processing request for {:?}: {:?}", request.position, request.request_type);
    
                    match request.request_type {
                        IORequestType::Load => {
                            let pos = request.position;
                            // --- Load Logic ---
    
                            // 1. Check cache FIRST (read lock scope)
                            let mut found_in_cache = false;
                            if let Ok(cache_guard) = cache_arc_thread.read() {
                                if let Some(data) = cache_guard.get(&pos) {
                                    println!("IO Thread: Cache hit for {:?}. Sending Loaded.", pos);
                                    // Send result back - ignore error if receiver disconnected
                                    let _ = result_sender_clone.send(ChunkResult::Loaded(pos, data.clone()));
                                    found_in_cache = true;
                                }
                            } else {
                                eprintln!("IO Thread: Cache read lock poisoned for {:?} during load check!", pos);
                            }
    
                            // If found and sent from cache, skip the rest
                            if found_in_cache {
                                println!("IO Thread: Finished processing Load (Cache Hit) for {:?}", pos);
                                continue;
                            }
    
                            // 2. Cache miss - try loading from disk using std fs
                            println!("IO Thread: Cache miss for {:?}. Attempting disk load.", pos);
                            let path_str = format!("{}/chunk_{}_{}.json", save_dir_clone, pos.x, pos.z);
                            let path = Path::new(&path_str);
    
                            // Using standard Rust file operations - not Godot's
                            let load_outcome = if path.exists() {
                                match std::fs::File::open(path) {
                                    Ok(mut file) => {
                                        let mut contents = String::new();
                                        match file.read_to_string(&mut contents) {
                                            Ok(_) => {
                                                // Try to deserialize
                                                match serde_json::from_str::<ChunkData>(&contents) {
                                                    Ok(data) => {
                                                        println!("IO Thread: Deserialized chunk {:?} successfully.", pos);
                                                        Ok(data)
                                                    },
                                                    Err(e) => {
                                                        eprintln!("IO Thread: Failed to deserialize chunk {:?} from {}: {}", pos, path_str, e);
                                                        Err(format!("Deserialize error: {}", e))
                                                    }
                                                }
                                            },
                                            Err(e) => {
                                                eprintln!("IO Thread: Failed to read file {}: {}", path_str, e);
                                                Err(format!("File read error: {}", e))
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        eprintln!("IO Thread: Failed to open chunk file {} for reading: {}", path_str, e);
                                        Err(format!("File open error: {}", e))
                                    }
                                }
                            } else {
                                // Handle file not found specifically for clearer logging
                                println!("IO Thread: Chunk file not found for {:?}: {}", pos, path_str);
                                Err(format!("File not found: {}", path_str))
                            };
    
                            // 3. Process outcome & send result
                            match load_outcome {
                                Ok(loaded_data) => {
                                    println!("IO Thread: Successfully loaded {:?} from disk. Updating cache.", pos);
                                    // Update cache (write lock scope)
                                    if let Ok(mut cache_w) = cache_arc_thread.write() {
                                        cache_w.insert(pos, loaded_data.clone());
                                        // Check and enforce cache limit AFTER inserting
                                        if let Ok(limit) = limit_arc_thread.read() {
                                            enforce_cache_limit(&mut cache_w, *limit);
                                        } else {
                                            eprintln!("IO Thread: Cache limit read lock poisoned while enforcing limit for loaded {:?}", pos);
                                        }
                                    } else {
                                        eprintln!("IO Thread: Cache write lock poisoned when updating for loaded {:?}", pos);
                                    }
                                    // Send loaded result back
                                    let _ = result_sender_clone.send(ChunkResult::Loaded(pos, loaded_data));
                                }
                                Err(error_msg) => {
                                    // Log specific reason for load failure
                                    println!("IO Thread: Load failed for {:?}: {}. Sending LoadFailed.", pos, error_msg);
                                    let _ = result_sender_clone.send(ChunkResult::LoadFailed(pos));
                                }
                            }
                            println!("IO Thread: Finished processing Load (Disk Attempt) for {:?}", pos);
                        } // End IORequestType::Load case
    
                        IORequestType::Save(chunk_data) => {
                            let pos = request.position;
                            println!("IO Thread: Processing Save for {:?}", pos);
                            
                            // --- Save Logic ---
                            let path_str = format!("{}/chunk_{}_{}.json", save_dir_clone, pos.x, pos.z);
                            let path = Path::new(&path_str);
    
                            // Ensure parent directory exists
                            if let Some(parent) = path.parent() {
                                if !parent.exists() {
                                    if let Err(e) = std::fs::create_dir_all(parent) {
                                        eprintln!("IO Thread: Failed to create parent directories for {}: {}", path_str, e);
                                        continue;
                                    }
                                }
                            }
    
                            match serde_json::to_string(&chunk_data) {
                                Ok(json) => {
                                    // Use standard Rust file operations for writing
                                    match std::fs::File::create(path) {
                                        Ok(mut file) => {
                                            match file.write_all(json.as_bytes()) {
                                                Ok(_) => {
                                                    println!("IO Thread: Successfully wrote chunk {:?} to {}.", pos, path_str);
    
                                                    // Update cache AFTER successful save (write lock scope)
                                                    if let Ok(mut cache_w) = cache_arc_thread.write() {
                                                        cache_w.insert(pos, chunk_data.clone());
                                                        if let Ok(limit) = limit_arc_thread.read() {
                                                            enforce_cache_limit(&mut cache_w, *limit);
                                                        } else {
                                                            eprintln!("IO Thread: Cache limit read lock poisoned while enforcing limit for saved {:?}", pos);
                                                        }
                                                    } else {
                                                        eprintln!("IO Thread: Cache write lock poisoned when updating for saved {:?}", pos);
                                                    }
                                                },
                                                Err(e) => {
                                                    eprintln!("IO Thread: Failed to write to chunk file {}: {}", path_str, e);
                                                }
                                            }
                                        },
                                        Err(e) => {
                                            eprintln!("IO Thread: Failed to create chunk file {} for writing: {}", path_str, e);
                                        }
                                    }
                                },
                                Err(e) => {
                                    eprintln!("IO Thread: Failed to serialize chunk {:?}: {}", pos, e);
                                }
                            }
                            println!("IO Thread: Finished processing Save for {:?}", pos);
                        } // End IORequestType::Save case
    
                        IORequestType::Shutdown => {
                            println!("IO Thread: Processing Shutdown request. Breaking loop.");
                            break; // Exit the `for request in io_rx` loop
                        } // End IORequestType::Shutdown case
                    } // End match request.request_type
                } // End `for request in io_rx` loop
    
                // This point is reached if the loop terminates either by
                // receiving Shutdown or by the channel closing (sender dropped).
                println!("IO Thread: Receiver loop finished.");
            })); // End of catch_unwind closure
    
            // Check if the thread panicked after the catch_unwind call
            if result.is_err() {
                eprintln!("!!!!!!!!!!!!!!!! IO Thread: *** PANICKED *** !!!!!!!!!!!!!!!!");
            }
    
            println!("IO Thread: <<< TERMINATED >>>");
        }); // End of thread::spawn
    
        println!("ChunkStorage: Construction complete. IO thread spawned.");
    
        // Return the ChunkStorage instance for the main thread
        ChunkStorage {
            save_dir: save_dir.to_string(),
            cache: cache_arc, // Original Arc for main thread access
            cache_size_limit: limit_arc, // Original Arc for main thread access
            result_sender, // Original sender (passed in) for sending results back
            io_request_sender: Some(io_tx), // Sender *TO* the IO thread
            io_thread_handle: Some(handle), // Handle to join the IO thread later during shutdown
        }
    }
        
    // Make this method public
    pub fn get_chunk_path(&self, position: ChunkPosition) -> String {
        format!("{}/chunk_{}_{}.json", self.save_dir, position.x, position.z)
    }
    
    // Check if a chunk exists in storage
    pub fn chunk_exists(&self, position: ChunkPosition) -> bool {
        // Check cache first
        if let Ok(cache) = self.cache.read() {
            if cache.contains_key(&position) {
                return true;
            }
        }
        
        // Check file system
        let path = self.get_chunk_path(position);
        Path::new(&path).exists()
    }
    
    // Queue a chunk to be saved asynchronously
    pub fn queue_save_chunk(&self, position: ChunkPosition, heightmap: &[f32], biome_ids: &[u8]) {
        let chunk_data = ChunkData {
            position,
            heightmap: heightmap.to_vec(),
            biome_ids: biome_ids.to_vec(),
        };
        // Cache update is done by IO thread AFTER successful save. Send request.
        let request = IORequest { position, request_type: IORequestType::Save(chunk_data) };
        if let Some(sender) = &self.io_request_sender {
            if let Err(e) = sender.send(request) {
                eprintln!("Failed to send Save request for {:?}: {}", position, e);
            }
        }
    }
   
    pub fn update_cache_limit(&self) {
        if let Some(config_arc) = TerrainConfigManager::get_config() {
           if let Ok(guard) = config_arc.read() {
                let new_limit = guard.chunk_cache_size();
                self.set_cache_size_limit(new_limit); // Call existing method
                println!("ChunkStorage: Updated cache limit to {}", new_limit);
           }
       }
   }
    
    // Queue a chunk to be loaded asynchronously
    pub fn queue_load_chunk(&self, position: ChunkPosition) {
        // Cache check is now done by the IO thread. Just send the request.
        let request = IORequest { position, request_type: IORequestType::Load };
        if let Some(sender) = &self.io_request_sender {
            if let Err(e) = sender.send(request) {
                eprintln!("Failed to send Load request for {:?}: {}", position, e);
            }
        }
    }
    
    pub fn get_data_from_cache(&self, position: ChunkPosition) -> Option<ChunkData> {
        match self.cache.read() {
            Ok(guard) => guard.get(&position).cloned(),
            Err(_) => {
                eprintln!("Cache lock poisoned while reading for {:?}", position);
                None
            }
        }
    }

    pub fn shutdown(&mut self) {
        println!("ChunkStorage: Sending shutdown request to IO thread...");
        if let Some(sender) = self.io_request_sender.take() {
            let shutdown_request = IORequest {
                position: ChunkPosition { x: 0, z: 0 },
                request_type: IORequestType::Shutdown
            };
            if sender.send(shutdown_request).is_err() {
                eprintln!("IO thread receiver already dropped before shutdown message.");
            }
        }
    
        if let Some(handle) = self.io_thread_handle.take() {
            println!("ChunkStorage: Waiting for IO thread to join...");
            if handle.join().is_err() {
                eprintln!("IO thread panicked during shutdown!");
            } else {
                println!("ChunkStorage: IO thread joined successfully.");
            }
        }
    }
        
    // Clear the cache
    pub fn clear_cache(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.clear();
    }
    
    // Set cache size limit
    pub fn set_cache_size_limit(&self, limit: usize) {
        // Update the limit
        *self.cache_size_limit.write().unwrap() = limit;
        
        // If current cache exceeds new limit, trim it
        let mut cache = self.cache.write().unwrap();
        let keys: Vec<ChunkPosition> = cache.keys().cloned().collect();
        
        if keys.len() > limit {
            let to_remove = keys.len() - limit;
            for key in keys.iter().take(to_remove) {
                cache.remove(key);
            }
        }
    }
    
    // Get the current size of the cache
    pub fn get_cache_size(&self) -> usize {
        let cache = self.cache.read().unwrap();
        cache.len()
    }
    
    // Get all chunk positions currently in the cache
    pub fn get_cached_chunks(&self) -> Vec<ChunkPosition> {
        let cache = self.cache.read().unwrap();
        cache.keys().cloned().collect()
    }
    
    // Preload chunks in a region to cache
    pub fn preload_chunks_in_region(&self, center: ChunkPosition, radius: i32) {
        let mut positions = Vec::new();
        
        // Generate positions in the region
        for x in (center.x - radius)..=(center.x + radius) {
            for z in (center.z - radius)..=(center.z + radius) {
                positions.push(ChunkPosition { x, z });
            }
        }
        
        // Sort by distance to center
        positions.sort_by(|a, b| {
            let a_dist = (a.x - center.x).pow(2) + (a.z - center.z).pow(2);
            let b_dist = (b.x - center.x).pow(2) + (b.z - center.z).pow(2);
            a_dist.cmp(&b_dist)
        });
        
        // Queue them for loading
        for position in positions {
            self.queue_load_chunk(position
                // No callback action needed, just load into cache
            );
        }
    }
}
use std::fs;
use std::path::Path;
use godot::prelude::*;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::{Mutex, Arc, RwLock};
use rayon::prelude::*;
use std::sync::mpsc::{Sender, Receiver, channel}; // Add import
use std::thread;

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
    pub fn new(save_dir: &str, result_sender: Sender<ChunkResult>) -> Self {
        godot_print!("ChunkStorage: Initializing new storage with save_dir: {}", save_dir);
        
        // Ensure directory exists
        fs::create_dir_all(save_dir).unwrap_or_else(|e| {
            godot_error!("Failed to create save directory: {}", e);
        });
    
        // Get cache limit from config
        let cache_limit = TerrainConfigManager::get_config()
            .and_then(|config| match config.read() {
                Ok(guard) => Some(guard.chunk_cache_size()),
                Err(_) => None,
            })
            .unwrap_or(100);
        
        godot_print!("ChunkStorage: Cache limit set to: {}", cache_limit);
    
        // Create the channel for sending requests TO the IO thread
        let (io_tx, io_rx): (Sender<IORequest>, Receiver<IORequest>) = channel();
        
        // Prepare shared data for the IO thread
        let cache_arc = Arc::new(RwLock::new(HashMap::<ChunkPosition, ChunkData>::new()));
        let limit_arc = Arc::new(RwLock::new(cache_limit));
        let save_dir_clone = save_dir.to_string();
        let result_sender_clone = result_sender.clone();
        
        // Clone the Arcs for the thread - this is the key fix!
        let cache_arc_thread = Arc::clone(&cache_arc);
        let limit_arc_thread = Arc::clone(&limit_arc);
        
        godot_print!("ChunkStorage: Starting IO thread...");
        
        // Spawn the dedicated IO thread
        let handle = thread::spawn(move || {
            godot_print!("IO Thread: Started.");
            
            // This loop runs until the sender (io_tx) is dropped
            for request in io_rx {
                match &request.request_type {
                    IORequestType::Load => {
                        let pos = request.position;
                        godot_print!("IO Thread: Processing load request for {:?}", pos);
                    },
                    IORequestType::Save(_) => {
                        let pos = request.position;
                        godot_print!("IO Thread: Processing save request for {:?}", pos);
                    },
                    IORequestType::Shutdown => {
                        godot_print!("IO Thread: Received Shutdown request. Exiting.");
                        break;
                    }
                }
                
                // Process request
                match request.request_type {
                    IORequestType::Load => {
                        let pos = request.position;
                        // 1. Check cache FIRST
                        if let Ok(cache_guard) = cache_arc_thread.read() {
                            if let Some(data) = cache_guard.get(&pos) {
                                godot_print!("IO Thread: Cache hit for {:?}, sending Loaded result", pos);
                                let _ = result_sender_clone.send(ChunkResult::Loaded(pos, data.clone()));
                                continue; // Skip disk access
                            }
                        } // Drop read lock here
    
                        // 2. Cache miss - try loading from disk
                        let path = format!("{}/chunk_{}_{}.json", save_dir_clone, pos.x, pos.z);
                        let load_outcome = match fs::read_to_string(&path) {
                            Ok(json) => match serde_json::from_str::<ChunkData>(&json) {
                                Ok(data) => Ok(data),
                                Err(e) => Err(format!("Deserialize error: {}", e)),
                            },
                            Err(e) => Err(format!("File read error: {}", e)), // Includes NotFound
                        };
    
                        // 3. Process outcome & send result
                        match load_outcome {
                            Ok(data) => {
                                godot_print!("IO Thread: Successfully loaded chunk {:?} from disk", pos);
                                // Update cache (write lock)
                                if let Ok(mut cache_w) = cache_arc_thread.write() {
                                    cache_w.insert(pos, data.clone());
                                    // Check and enforce cache limit AFTER inserting
                                    if let Ok(limit) = limit_arc_thread.read() {
                                        enforce_cache_limit(&mut cache_w, *limit);
                                    }
                                } // Write lock dropped
                                let _ = result_sender_clone.send(ChunkResult::Loaded(pos, data));
                            }
                            Err(e) => {
                                godot_print!("IO Thread: Failed to load chunk {:?}: {}. Sending LoadFailed result.", pos, e);
                                let _ = result_sender_clone.send(ChunkResult::LoadFailed(pos));
                            }
                        }
                    }
    
                    IORequestType::Save(chunk_data) => {
                        let pos = request.position;
                        let path = format!("{}/chunk_{}_{}.json", save_dir_clone, pos.x, pos.z);
                        match serde_json::to_string(&chunk_data) {
                            Ok(json) => {
                                if let Err(e) = fs::write(&path, json) {
                                    godot_error!("IO Thread: Failed to write chunk {:?} to {}: {}", pos, path, e);
                                } else {
                                    // Update cache (write lock) - AFTER successful save
                                    if let Ok(mut cache_w) = cache_arc_thread.write() {
                                        cache_w.insert(pos, chunk_data.clone()); // Clone the moved chunk_data
                                        if let Ok(limit) = limit_arc_thread.read() {
                                            enforce_cache_limit(&mut cache_w, *limit);
                                        }
                                    } // Write lock dropped
                                    // Optionally send Saved result if needed
                                }
                            }
                            Err(e) => {
                                godot_error!("IO Thread: Failed to serialize chunk {:?}: {}", pos, e);
                            }
                        }
                    }
    
                    IORequestType::Shutdown => {
                        // Already handled above
                    }
                }
            }
            godot_print!("IO Thread: Terminated.");
        }); // End of thread::spawn
    
        godot_print!("ChunkStorage: Construction complete with live IO thread");
        
        ChunkStorage {
            save_dir: save_dir.to_string(),
            cache: cache_arc,
            cache_size_limit: limit_arc,
            result_sender,
            io_request_sender: Some(io_tx),
            io_thread_handle: Some(handle),
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
                godot_error!("Failed to send Save request for {:?}: {}", position, e);
            }
        }
    }
   
    pub fn update_cache_limit(&self) {
        if let Some(config_arc) = TerrainConfigManager::get_config() {
           if let Ok(guard) = config_arc.read() {
                let new_limit = guard.chunk_cache_size();
                self.set_cache_size_limit(new_limit); // Call existing method
                godot_print!("ChunkStorage: Updated cache limit to {}", new_limit);
           }
       }
   }
    
    // Queue a chunk to be loaded asynchronously
    pub fn queue_load_chunk(&self, position: ChunkPosition) {
        // Cache check is now done by the IO thread. Just send the request.
        let request = IORequest { position, request_type: IORequestType::Load };
        if let Some(sender) = &self.io_request_sender {
            if let Err(e) = sender.send(request) {
                godot_error!("Failed to send Load request for {:?}: {}", position, e);
            }
        }
    }
    
    pub fn get_data_from_cache(&self, position: ChunkPosition) -> Option<ChunkData> {
        match self.cache.read() {
            Ok(guard) => guard.get(&position).cloned(),
            Err(_) => {
                godot_error!("Cache lock poisoned while reading for {:?}", position);
                None
            }
        }
    }

    pub fn shutdown(&mut self) {
        godot_print!("ChunkStorage: Sending shutdown request to IO thread...");
        if let Some(sender) = self.io_request_sender.take() {
            let shutdown_request = IORequest {
                position: ChunkPosition { x: 0, z: 0 },
                request_type: IORequestType::Shutdown
            };
            if sender.send(shutdown_request).is_err() {
                godot_warn!("IO thread receiver already dropped before shutdown message.");
            }
        }
    
        if let Some(handle) = self.io_thread_handle.take() {
            godot_print!("ChunkStorage: Waiting for IO thread to join...");
            if handle.join().is_err() {
                godot_error!("IO thread panicked during shutdown!");
            } else {
                godot_print!("ChunkStorage: IO thread joined successfully.");
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
use std::fs;
use std::path::Path;
use godot::prelude::*;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::{Mutex, Arc, RwLock};
use rayon::prelude::*;

use crate::threading::thread_pool::{ThreadPool, global_thread_pool};
use crate::terrain::chunk_manager::ChunkPosition;
use crate::terrain::terrain_config::{TerrainConfigManager, TerrainConfig};

// Data structure for serializing chunk data
#[derive(Serialize, Deserialize, Clone)]
pub struct ChunkData {
    pub position: ChunkPosition,
    pub heightmap: Vec<f32>,
    pub biome_ids: Vec<u8>,
    // Add other data as needed
}

// Thread-safe queue for pending IO operations
struct IOQueue {
    save_queue: Mutex<Vec<(ChunkPosition, ChunkData)>>,
    load_queue: Mutex<Vec<(ChunkPosition, Box<dyn FnOnce(Option<ChunkData>) + Send>)>>,
}

impl IOQueue {
    fn new() -> Self {
        IOQueue {
            save_queue: Mutex::new(Vec::new()),
            load_queue: Mutex::new(Vec::new()),
        }
    }
}

// ChunkStorage handles saving and loading chunks from disk
pub struct ChunkStorage {
    save_dir: String,
    cache: RwLock<HashMap<ChunkPosition, ChunkData>>,
    cache_size_limit: RwLock<usize>,
    thread_pool: Option<ThreadPool>,
    io_queue: Arc<IOQueue>,
    is_processing_queue: Mutex<bool>,
}

impl ChunkStorage {
    pub fn new(save_dir: &str) -> Self {
        // Ensure directory exists
        fs::create_dir_all(save_dir).unwrap_or_else(|e| {
            eprintln!("Failed to create save directory: {}", e);
        });
        
        // Get thread pool configuration
        let (num_threads, cache_limit) = if let Some(config_arc) = TerrainConfigManager::get_config() {
            if let Ok(guard) = config_arc.read() {
                (guard.max_threads(), guard.chunk_cache_size()) // Get both values
            } else {
                eprintln!("ChunkStorage: Failed to read TerrainConfig, using defaults.");
                (2, 100) // Default if we can't read config
            }
        } else {
            eprintln!("ChunkStorage: No TerrainConfig found, using defaults.");
            (2, 100) // Default if no config available
        };
        
        // Decide on thread pool (local vs global)
        let thread_pool = if global_thread_pool().is_some() {
            godot_print!("ChunkStorage: Using global thread pool for IO.");
            None // Use global pool
        } else {
            godot_print!("ChunkStorage: Creating local thread pool with {} threads for IO.", num_threads);
            Some(ThreadPool::new(num_threads))
        };
        
        ChunkStorage {
            save_dir: save_dir.to_string(),
            cache: RwLock::new(HashMap::new()),
            cache_size_limit: RwLock::new(cache_limit), // Store up to 100 chunks in memory
            thread_pool,
            io_queue: Arc::new(IOQueue::new()),
            is_processing_queue: Mutex::new(false),
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
    pub fn queue_save_chunk(self: Arc<Self>, position: ChunkPosition, heightmap: &[f32], biome_ids: &[u8]) {
        let chunk_data = ChunkData {
            position,
            heightmap: heightmap.to_vec(),
            biome_ids: biome_ids.to_vec(),
        };
    
        // Add to the save queue (self is Arc, but derefs to &self for accessing fields)
        {
            let mut queue = self.io_queue.save_queue.lock().unwrap();
            queue.push((position, chunk_data.clone()));
        }
    
        // Also update the cache immediately (update_cache takes &self, Arc derefs)
        self.update_cache(position, chunk_data);
    
        // Process the queue if not already processing
        // Now self is Arc<Self>, so this call is valid.
        // Note: This consumes the Arc passed into queue_save_chunk.
        // The caller will need to clone the Arc before calling queue_save_chunk
        // if they want to use the storage object afterwards.
        self.process_io_queue();
    }
    
    // Save a chunk to storage (synchronous version)
    pub fn save_chunk(&self, position: ChunkPosition, heightmap: &[f32], biome_ids: &[u8]) {
        let chunk_data = ChunkData {
            position,
            heightmap: heightmap.to_vec(),
            biome_ids: biome_ids.to_vec(),
        };
        
        // Save to file
        let path = self.get_chunk_path(position);
        let json = serde_json::to_string(&chunk_data).unwrap_or_else(|e| {
            eprintln!("Failed to serialize chunk data: {}", e);
            String::new()
        });
        
        if !json.is_empty() {
            fs::write(&path, json).unwrap_or_else(|e| {
                eprintln!("Failed to write chunk data to {}: {}", path, e);
            });
        }
        
        // Update cache
        self.update_cache(position, chunk_data);
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
    pub fn queue_load_chunk<F>(&self, position: ChunkPosition, callback: F)
        where
            F: FnOnce(Option<ChunkData>) + Send + 'static,
    {
        // Check cache first
        if let Ok(cache) = self.cache.read() { // Read lock
            if let Some(data) = cache.get(&position) {
                callback(Some(data.clone()));
                return;
            }
        }
        // Drop read lock here

        // Add to the load queue
        {
            let mut queue = self.io_queue.load_queue.lock().unwrap();
            queue.push((position, Box::new(callback)));
        } // Drop lock on load_queue

        // Process the queue if not already processing
        // This requires an Arc<Self> to call process_io_queue.
        // This indicates queue_load_chunk should also potentially take Arc<Self>
        // OR the caller needs to hold the Arc and call process_io_queue manually after queueing.
        // Let's assume the caller manages the Arc and calls process_io_queue.
        // If you want queue_load_chunk to trigger processing, it would need access
        // to an Arc<ChunkStorage>.
        // For now, let's leave this commented out, assuming manual trigger or trigger from queue_save_chunk
        // self.clone().process_io_queue(); // Needs self to be Arc<Self>
         println!("Chunk {:?} queued for load. Manual trigger of process_io_queue needed if not saving.", position);

    }
    
    // Load a chunk from storage (synchronous version)
    pub fn load_chunk(&self, position: ChunkPosition) -> Option<ChunkData> {
        // Check cache first
        if let Ok(cache) = self.cache.read() {
            if let Some(data) = cache.get(&position) {
                return Some(data.clone());
            }
        }
        
        // Load from file
        let path = self.get_chunk_path(position);
        match fs::read_to_string(&path) {
            Ok(json) => {
                match serde_json::from_str::<ChunkData>(&json) {
                    Ok(data) => {
                        // Update cache
                        self.update_cache(position, data.clone());
                        Some(data)
                    },
                    Err(e) => {
                        eprintln!("Failed to deserialize chunk data from {}: {}", path, e);
                        None
                    }
                }
            },
            Err(_) => None,
        }
    }
    
    // Process IO queue using thread pool
    fn process_io_queue(self: Arc<Self>) { // <-- Takes ownership of the Arc
        // Ensure we only process the queue once at a time
        // Use the is_processing_queue from the Arc<Self>
        let mut is_processing = self.is_processing_queue.lock().unwrap();
        if *is_processing {
            // If already processing, another thread will handle it.
            // We drop the Arc here, decrementing the count.
            return;
        }

        // Set the flag to true
        *is_processing = true;
        // Drop the lock quickly
        drop(is_processing);

        // Clone necessary data for the closure
        // Clone the Arc itself to move into the closure
        let self_clone = Arc::clone(&self);

        // Function to process the queue in a thread
        let process_queue = move || { // Closure now captures self_clone (Arc<ChunkStorage>)
            // Clone fields needed from self_clone inside the closure
            let io_queue = Arc::clone(&self_clone.io_queue);
            let save_dir = self_clone.save_dir.clone(); // String implements Clone

            // Process save queue
            let save_tasks = {
                let mut queue = io_queue.save_queue.lock().unwrap();
                std::mem::take(&mut *queue)
            };

            if !save_tasks.is_empty() {
                 // Process in parallel if we have multiple items
                 // Use rayon's scope or pass necessary data directly if needed
                save_tasks.par_iter().for_each(|(position, chunk_data)| {
                    let path = format!("{}/chunk_{}_{}.json", save_dir, position.x, position.z);
                    match serde_json::to_string(&chunk_data) {
                         Ok(json) => {
                            if let Err(e) = fs::write(&path, json) {
                                eprintln!("Failed to write chunk data to {}: {}", path, e);
                            }
                        },
                        Err(e) => {
                             eprintln!("Failed to serialize chunk data for {}: {}", path, e);
                        }
                    }
                });
            }

            // Process load queue
            let load_tasks = {
                let mut queue = io_queue.load_queue.lock().unwrap();
                std::mem::take(&mut *queue)
            };

            if !load_tasks.is_empty() {
                // Process each load task sequentially
                for (position, callback) in load_tasks {
                    let path = format!("{}/chunk_{}_{}.json", save_dir, position.x, position.z);
                    let result = match fs::read_to_string(&path) {
                        Ok(json) => {
                            match serde_json::from_str::<ChunkData>(&json) {
                                Ok(data) => {
                                    // Use self_clone to call update_cache
                                    // update_cache takes &self, which Arc<T> can deref to
                                    self_clone.update_cache(position, data.clone());
                                    Some(data)
                                },
                                Err(e) => {
                                    eprintln!("Failed to deserialize chunk data from {}: {}", path, e);
                                    None
                                }
                            }
                        },
                        Err(_) => None, // Consider logging file read errors too
                    };

                    // Call the callback with the result
                    // The callback is Box<dyn FnOnce>, so it's called here
                    callback(result);
                }
            }

            // Reset processing flag using self_clone
            // Need to acquire the lock again
            let mut is_processing_guard = self_clone.is_processing_queue.lock().unwrap();
            *is_processing_guard = false;
            // Lock is released when is_processing_guard goes out of scope
        }; // End of closure definition

        // Use thread pool if available
        // We need to handle the case where self.thread_pool is Some or None inside the Arc
        // Use thread pool if available
        let maybe_local_pool = self.thread_pool.as_ref(); // Borrow Option<ThreadPool>

        if let Some(ref local_pool) = maybe_local_pool { // Use the local pool if it exists
            local_pool.execute(process_queue);
        } else if let Some(global_pool_arc) = crate::threading::thread_pool::global_thread_pool() { // Check for the global pool
             // Attempt to read the global pool
             match global_pool_arc.read() { // Use match for better error handling on lock failure
                 Ok(guard) => {
                     // Successfully locked the global pool for reading.
                     // guard is a RwLockReadGuard<ThreadPool>.
                     // It Derefs to &ThreadPool, so we can call execute directly.
                     guard.execute(process_queue);
                 },
                 Err(e) => {
                     // Failed to lock global pool (poisoned). Run synchronously.
                     eprintln!("Failed to lock global thread pool ({}), running IO synchronously.", e);
                     process_queue();
                 }
             }
        } else {
            // No local or global thread pool configured/initialized. Run synchronously.
            eprintln!("No thread pool configured, running IO synchronously.");
            process_queue();
        }

        // The original Arc<Self> passed to process_io_queue is dropped here if it wasn't cloned elsewhere.
    }
    
    // Update the cache with new chunk data
    fn update_cache(&self, position: ChunkPosition, data: ChunkData) {
        // Lock the RwLock directly
        let mut cache = self.cache.write().unwrap();
        let cache_size_limit = *self.cache_size_limit.read().unwrap();

        // Add new data first (simpler than complex LRU without tracking)
        cache.insert(position, data);

        // If cache exceeds limit, remove entries (simple HashMap iteration order, not LRU)
        if cache.len() > cache_size_limit {
            let keys_to_remove: Vec<ChunkPosition> = cache.keys()
                .take(cache.len() - cache_size_limit) // Calculate how many to remove
                .cloned()
                .collect();

            println!("Cache limit {} reached (size {}). Evicting {} chunks.", cache_size_limit, cache.len(), keys_to_remove.len());
            for key in keys_to_remove {
                cache.remove(&key);
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
            self.queue_load_chunk(position, |_| {
                // No callback action needed, just load into cache
            });
        }
    }
}
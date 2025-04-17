use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::sync::mpsc::{Sender, Receiver, channel}; // Add import
use std::thread;
use std::panic::{catch_unwind, AssertUnwindSafe}; // Added for panic catching
use std::io::{Read, Write};


use crate::terrain::chunk_manager::{ChunkPosition, ChunkResult};
use crate::terrain::terrain_config::{TerrainConfigManager};
use lru::LruCache;
use std::num::NonZeroUsize; 


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
    cache: Arc<RwLock<LruCache<ChunkPosition, ChunkData>>>,
   
    result_sender: Sender<ChunkResult>, // Store a clone of the sender from ChunkManager
    io_request_sender: Option<Sender<IORequest>>, // To send requests TO IO thread
    io_thread_handle: Option<thread::JoinHandle<()>>, // Handle to the IO thread
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

        // Convert Godot path (like user://) to an absolute path if necessary for std::fs
        // This assumes save_dir is already a path std::fs can handle.
        // If save_dir uses Godot's pseudo-protocols, you might need:
        // let absolute_save_dir = ProjectSettings::singleton().globalize_path(save_dir.into()).to_string();
        // For simplicity, we'll use save_dir directly assuming it's valid for std::fs.
        let fs_save_dir = save_dir; // Use this variable below

        // Ensure directory exists using standard Rust fs
        match fs::create_dir_all(fs_save_dir) {
            Ok(_) => {
                println!("ChunkStorage: Save directory verified/created: {}", fs_save_dir);
            }
            Err(e) => {
                // Use eprintln! for critical errors
                eprintln!("ChunkStorage: ERROR - Failed to create save directory '{}': {}. Subsequent saves WILL likely fail.", fs_save_dir, e);
                // Depending on requirements, you might want to panic or return Result here.
            }
        }

        // Get cache limit from config (using lazy init for TerrainConfigManager)
        let default_cache_limit = 400; // Sensible default matching TerrainInitialConfigData default
        let cache_limit = {
            let terrain_config_arc = TerrainConfigManager::get_config(); // Ensures TerrainConfigManager is initialized
            match terrain_config_arc.read() {
                Ok(guard) => guard.chunk_cache_size, // Direct field access
                Err(e) => {
                    eprintln!("ChunkStorage: Failed to read TerrainConfig lock: {}. Using default cache limit.", e);
                    default_cache_limit // Fallback to default
                }
            }
        };

        println!("ChunkStorage: Cache limit set to: {}", cache_limit);

        // Validate and create NonZeroUsize for LruCache capacity
        let lru_capacity = NonZeroUsize::new(cache_limit).unwrap_or_else(|| {
             eprintln!("Chunk cache size config is zero or invalid, defaulting cache capacity to 1.");
             NonZeroUsize::new(1).expect("Default LRU capacity of 1 failed unexpectedly")
        });

        // Create the channel for sending requests TO the IO thread
        let (io_tx, io_rx): (Sender<IORequest>, Receiver<IORequest>) = channel();

        // Prepare shared data for the IO thread
        let cache_arc = Arc::new(RwLock::new(LruCache::<ChunkPosition, ChunkData>::new(lru_capacity)));
        let save_dir_clone = fs_save_dir.to_string(); // Clone the potentially globalized path
        let result_sender_clone = result_sender.clone(); // Clone sender for results back to main
        let cache_arc_thread = Arc::clone(&cache_arc); // Clone Arc for thread access

        println!("ChunkStorage: Spawning IO thread...");

        // Spawn the dedicated IO thread
        let handle = thread::spawn(move || {
            println!("IO Thread: <<< STARTED >>>");

            // Optional: Catch panics to prevent silent thread death and log the event.
            let result = catch_unwind(AssertUnwindSafe(|| {
                println!("IO Thread: Starting receiver loop...");

                // Loop processes requests until channel closes or Shutdown received
                for request in io_rx {
                    // Uncomment for detailed logging:
                    // println!("IO Thread: Processing request for {:?}: {:?}", request.position, request.request_type);

                    match request.request_type {
                        IORequestType::Load => {
                            let pos = request.position;
                            let mut found_in_cache = false;

                            // --- Check cache FIRST (write lock for get_mut) ---
                            if let Ok(mut cache_guard) = cache_arc_thread.write() {
                                if let Some(data) = cache_guard.get_mut(&pos) { // get_mut updates LRU order
                                    // println!("IO Thread: Cache hit for {:?}. Sending Loaded.", pos);
                                    let _ = result_sender_clone.send(ChunkResult::Loaded(pos, data.clone()));
                                    found_in_cache = true;
                                }
                            } else {
                                eprintln!("IO Thread: Cache write lock poisoned for {:?} during load check!", pos);
                            }

                            if found_in_cache { continue; } // Skip disk if found

                            // --- Cache miss - Load from disk ---
                            // println!("IO Thread: Cache miss for {:?}. Attempting disk load.", pos);
                            let path_str = format!("{}/chunk_{}_{}.json", save_dir_clone, pos.x, pos.z);
                            let path = Path::new(&path_str);

                            // Standard Rust file IO
                            let load_outcome = match fs::File::open(path) {
                                Ok(mut file) => {
                                    let mut contents = String::new();
                                    match file.read_to_string(&mut contents) {
                                        Ok(_) => match serde_json::from_str::<ChunkData>(&contents) {
                                            Ok(data) => Ok(data),
                                            Err(e) => Err(format!("Deserialize error: {}", e)),
                                        },
                                        Err(e) => Err(format!("File read error: {}", e)),
                                    }
                                }
                                Err(e) => {
                                    // Distinguish file not found from other errors
                                    if e.kind() == std::io::ErrorKind::NotFound {
                                        Err(format!("File not found: {}", path_str)) // Normal case if chunk never saved/generated
                                    } else {
                                        Err(format!("File open error: {}", e)) // Other OS-level error
                                    }
                                }
                            };

                            // --- Process outcome ---
                            match load_outcome {
                                Ok(loaded_data) => {
                                    // println!("IO Thread: Loaded {:?} from disk. Updating cache.", pos);
                                    if let Ok(mut cache_w) = cache_arc_thread.write() {
                                        cache_w.push(pos, loaded_data.clone()); // Add to LRU cache
                                    } else {
                                        eprintln!("IO Thread: Cache write lock poisoned updating cache for loaded {:?}", pos);
                                    }
                                    let _ = result_sender_clone.send(ChunkResult::Loaded(pos, loaded_data));
                                }
                                Err(error_msg) => {
                                    // Don't spam errors if it's just file not found
                                    if !error_msg.starts_with("File not found") {
                                        eprintln!("IO Thread: Load failed for {:?}: {}", pos, error_msg);
                                    }
                                    let _ = result_sender_clone.send(ChunkResult::LoadFailed(pos));
                                }
                            }
                        } // End Load case

                        IORequestType::Save(chunk_data) => {
                            let pos = request.position;
                            // println!("IO Thread: Processing Save for {:?}", pos);
                            let path_str = format!("{}/chunk_{}_{}.json", save_dir_clone, pos.x, pos.z);
                            let path = Path::new(&path_str);

                            // Ensure parent directory exists (optional, create_dir_all did this)
                            // if let Some(parent) = path.parent() { fs::create_dir_all(parent).ok(); }

                            match serde_json::to_string_pretty(&chunk_data) { // Use pretty print for readability
                                Ok(json) => match fs::File::create(path) {
                                    Ok(mut file) => {
                                        if let Err(e) = file.write_all(json.as_bytes()) {
                                            eprintln!("IO Thread: Failed to write to chunk file {}: {}", path_str, e);
                                        } else {
                                            // println!("IO Thread: Successfully wrote chunk {:?} to {}.", pos, path_str);
                                            // Update cache AFTER successful save
                                            if let Ok(mut cache_w) = cache_arc_thread.write() {
                                                cache_w.push(pos, chunk_data.clone()); // Add/Update in LRU
                                            } else {
                                                eprintln!("IO Thread: Cache write lock poisoned updating cache for saved {:?}", pos);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("IO Thread: Failed to create chunk file {} for writing: {}", path_str, e);
                                    }
                                },
                                Err(e) => {
                                    eprintln!("IO Thread: Failed to serialize chunk {:?}: {}", pos, e);
                                }
                            }
                        } // End Save case

                        IORequestType::Shutdown => {
                            println!("IO Thread: Processing Shutdown request. Breaking loop.");
                            break; // Exit the loop
                        }
                    } // End match request_type
                } // End loop

                println!("IO Thread: Receiver loop finished.");
            })); // End catch_unwind

            if result.is_err() {
                eprintln!("!!!!!!!!!!!!!!!! IO Thread: *** PANICKED *** !!!!!!!!!!!!!!!!");
            }
            println!("IO Thread: <<< TERMINATED >>>");
        }); // End thread::spawn

        println!("ChunkStorage: Construction complete. IO thread spawned.");

        // Return the ChunkStorage instance for the main thread
        ChunkStorage {
            save_dir: fs_save_dir.to_string(), // Store the potentially globalized path
            cache: cache_arc, // Original Arc for main thread access
            result_sender, // Original sender passed in
            io_request_sender: Some(io_tx), // Sender *TO* the IO thread
            io_thread_handle: Some(handle), // Handle to join the IO thread later
        }
    } // End new()
        
    // Make this method public
    pub fn get_chunk_path(&self, position: ChunkPosition) -> String {
        format!("{}/chunk_{}_{}.json", self.save_dir, position.x, position.z)
    }
    
    // Check if a chunk exists in storage
    pub fn chunk_exists(&self, position: ChunkPosition) -> bool {
        // Check cache first
        if let Ok(cache) = self.cache.read() {
            if cache.contains(&position) {
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
        match self.cache.write() { // *** Use write lock for get_mut to update LRU order ***
            Ok(mut guard) => guard.get_mut(&position).cloned(), // Use get_mut
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
    
    // Get the current size of the cache
    pub fn get_cache_size(&self) -> usize {
        match self.cache.read() { // Use read lock for len()
             Ok(guard) => guard.len(),
             Err(_) => {
                  eprintln!("Cache lock poisoned while getting size.");
                  0
             }
        }
   }

    // Get all chunk positions currently in the cache
    pub fn get_cached_chunks(&self) -> Vec<ChunkPosition> {
        match self.cache.read() { // Use read lock
             Ok(guard) => guard.iter().map(|(pos, _data)| *pos).collect(), // Iterate over LRU
             Err(_) => {
                  eprintln!("Cache lock poisoned while getting cached chunks.");
                  vec![]
             }
        }
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
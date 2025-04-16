use godot::prelude::*;
use godot::classes::Node;
use std::sync::{Arc, RwLock};
use num_cpus;

// Static configuration singleton for terrain generation
pub struct TerrainConfig {
    // Thread management
    max_threads: usize,
    chunk_size: u32,
    
    // Generation settings
    blend_distance: f32,
    use_parallel_processing: bool,
    
    // Memory management
    chunk_cache_size: usize,
    
    // Performance tuning
    chunks_per_frame: usize,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        let cpu_count = num_cpus::get();
        
        // Default configuration - can be overridden
        TerrainConfig {
            // By default, use all available CPUs minus 1 (leave one for the main thread)
            // but ensure we have at least 1 thread
            max_threads: std::cmp::max(1, cpu_count.saturating_sub(1)),
            chunk_size: 32,
            blend_distance: 200.0,
            use_parallel_processing: true,
            chunk_cache_size: 400,
            chunks_per_frame: 4,
        }
    }
}

impl TerrainConfig {
    // Get the number of threads to use for terrain generation
    pub fn max_threads(&self) -> usize {
        self.max_threads
    }
    
    // Get the chunk size
    pub fn chunk_size(&self) -> u32 {
        self.chunk_size
    }
    
    // Get the blend distance
    pub fn blend_distance(&self) -> f32 {
        self.blend_distance
    }
    
    // Check if parallel processing is enabled
    pub fn use_parallel_processing(&self) -> bool {
        self.use_parallel_processing
    }
    
    // Get the chunk cache size
    pub fn chunk_cache_size(&self) -> usize {
        self.chunk_cache_size
    }
    
    // Get the number of chunks to process per frame
    pub fn chunks_per_frame(&self) -> usize {
        self.chunks_per_frame
    }
}

// Global access to terrain configuration
pub struct TerrainConfigManager {
    config: Arc<RwLock<TerrainConfig>>,
}

// Singleton implementation
impl TerrainConfigManager {
    // Initialize the global terrain configuration
    pub fn init() -> Arc<RwLock<TerrainConfig>> {
        let config = Arc::new(RwLock::new(TerrainConfig::default()));
        TERRAIN_CONFIG.with(|cell| {
            *cell.borrow_mut() = Some(config.clone());
        });
        config
    }
    
    // Get the global terrain configuration
    pub fn get_config() -> Option<Arc<RwLock<TerrainConfig>>> {
        TERRAIN_CONFIG.with(|cell| {
            cell.borrow().clone()
        })
    }
    
    // Update the terrain configuration
    pub fn update_config<F>(updater: F)
    where
        F: FnOnce(&mut TerrainConfig),
    {
        if let Some(config) = Self::get_config() {
            if let Ok(mut config_guard) = config.write() {
                updater(&mut *config_guard);
            }
        }
    }
}

// Thread-local storage for the terrain configuration
thread_local! {
    static TERRAIN_CONFIG: std::cell::RefCell<Option<Arc<RwLock<TerrainConfig>>>> = std::cell::RefCell::new(None);
}

// Godot node for configuring terrain generation
#[derive(GodotClass)]
#[class(base=Node)]
pub struct TerrainConfigNode {
    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for TerrainConfigNode {
    fn init(base: Base<Node>) -> Self {
        // Initialize the terrain configuration
        TerrainConfigManager::init();
        
        Self { base }
    }
}

#[godot_api]
impl TerrainConfigNode {
    // Set the number of threads to use
    #[func]
    pub fn set_max_threads(&self, max_threads: i64) { // Changed usize to i64
        TerrainConfigManager::update_config(|config| {
            // Convert i64 to usize, ensuring it's not negative and using max(1)
            config.max_threads = std::cmp::max(1, max_threads.try_into().unwrap_or(1));
        });
    }

    // Set the chunk size (u32 is usually fine, often mapped to i64 by Godot)
    #[func]
    pub fn set_chunk_size(&self, chunk_size: i64) { // Changed u32 to i64 for consistency
        TerrainConfigManager::update_config(|config| {
             // Convert i64 to u32, ensuring it's not negative and using max(1)
            config.chunk_size = std::cmp::max(1, chunk_size.try_into().unwrap_or(1));
        });
    }

    // Set the blend distance (f32 is fine)
    #[func]
    pub fn set_blend_distance(&self, blend_distance: f32) {
        TerrainConfigManager::update_config(|config| {
            config.blend_distance = blend_distance;
        });
    }
    
    // Enable or disable parallel processing
    #[func]
    pub fn set_parallel_processing(&self, enabled: bool) {
        TerrainConfigManager::update_config(|config| {
            config.use_parallel_processing = enabled;
        });
    }

    // Set the chunk cache size
    #[func]
    pub fn set_chunk_cache_size(&self, cache_size: i64) { // Changed usize to i64
        TerrainConfigManager::update_config(|config| {
            // Convert i64 to usize, handling potential negative values
            config.chunk_cache_size = cache_size.try_into().unwrap_or(0);
        });
    }

    // Set the number of chunks to process per frame
    #[func]
    pub fn set_chunks_per_frame(&self, chunks_per_frame: i64) { // Changed usize to i64
        TerrainConfigManager::update_config(|config| {
            // Convert i64 to usize, handling potential negative values
            config.chunks_per_frame = chunks_per_frame.try_into().unwrap_or(0);
        });
    }

    // Get the current configuration as a Dictionary
    #[func]
    pub fn get_config_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();

        if let Some(config) = TerrainConfigManager::get_config() {
            if let Ok(config_guard) = config.read() {
                // Cast usize/u32 fields to i64 for Godot
                dict.insert("max_threads", config_guard.max_threads as i64);
                dict.insert("chunk_size", config_guard.chunk_size as i64); // Cast u32
                dict.insert("blend_distance", config_guard.blend_distance);
                dict.insert("use_parallel_processing", config_guard.use_parallel_processing);
                dict.insert("chunk_cache_size", config_guard.chunk_cache_size as i64);
                dict.insert("chunks_per_frame", config_guard.chunks_per_frame as i64);
            }
        }

        dict
    }
    
    #[func]
    pub fn apply_config_dict(&self, dict: Dictionary) -> bool {
        TerrainConfigManager::update_config(|config| {
            // Update each field if present in the dictionary
            if let Some(val) = dict.get("max_threads") {
                // Expect i64 from Godot
                if let Ok(val) = val.try_to::<i64>() {
                    // Convert i64 to usize safely
                    config.max_threads = std::cmp::max(1, val.try_into().unwrap_or(1));
                }
            }

            if let Some(val) = dict.get("chunk_size") {
                 // Expect i64 from Godot
                if let Ok(val) = val.try_to::<i64>() {
                    // Convert i64 to u32 safely
                    config.chunk_size = std::cmp::max(1, val.try_into().unwrap_or(1));
                }
            }

            if let Some(val) = dict.get("blend_distance") {
                if let Ok(val) = val.try_to::<f64>() { // Godot uses f64 (float)
                    config.blend_distance = val as f32;
                }
            }

            if let Some(val) = dict.get("use_parallel_processing") {
                if let Ok(val) = val.try_to::<bool>() {
                    config.use_parallel_processing = val;
                }
            }

            if let Some(val) = dict.get("chunk_cache_size") {
                 // Expect i64 from Godot
                if let Ok(val) = val.try_to::<i64>() {
                     // Convert i64 to usize safely
                    config.chunk_cache_size = val.try_into().unwrap_or(0);
                }
            }

            if let Some(val) = dict.get("chunks_per_frame") {
                 // Expect i64 from Godot
                if let Ok(val) = val.try_to::<i64>() {
                    // Convert i64 to usize safely
                    config.chunks_per_frame = val.try_into().unwrap_or(0);
                }
            }
        });

        true
    }
    
    // Detect and set optimal configuration based on the system
    #[func]
    pub fn detect_optimal_settings(&self) -> Dictionary {
        let cpu_count = num_cpus::get();
        let ram_gb = self.estimate_system_memory() / (1024 * 1024 * 1024);
        
        TerrainConfigManager::update_config(|config| {
            // Use N-1 threads on systems with many cores, or half on systems with few cores
            if cpu_count > 4 {
                config.max_threads = cpu_count - 1;
            } else {
                config.max_threads = std::cmp::max(1, cpu_count / 2);
            }
            
            // Scale cache size based on available RAM
            if ram_gb > 16 {
                config.chunk_cache_size = 200;
            } else if ram_gb > 8 {
                config.chunk_cache_size = 100;
            } else {
                config.chunk_cache_size = 50;
            }
            
            // Adjust chunks per frame based on CPU count
            if cpu_count > 8 {
                config.chunks_per_frame = 8;
            } else if cpu_count > 4 {
                config.chunks_per_frame = 4;
            } else {
                config.chunks_per_frame = 2;
            }
        });
        
        // Return the new configuration
        self.get_config_dict()
    }
    
    // Estimate available system memory (in bytes)
    fn estimate_system_memory(&self) -> usize {
        // Default to 8GB if we can't determine
        let default_memory = 8 * 1024 * 1024 * 1024;
        
        #[cfg(target_os = "linux")]
        {
            match std::fs::read_to_string("/proc/meminfo") {
                Ok(meminfo) => {
                    if let Some(line) = meminfo.lines().find(|line| line.starts_with("MemTotal:")) {
                        if let Some(kb_str) = line.split_whitespace().nth(1) {
                            if let Ok(kb) = kb_str.parse::<usize>() {
                                return kb * 1024;
                            }
                        }
                    }
                },
                Err(_) => {}
            }
        }
        
        // For other platforms, or if the above failed, return the default
        default_memory
    }
}
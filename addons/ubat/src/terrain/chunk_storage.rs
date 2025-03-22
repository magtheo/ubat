use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

use crate::terrain::chunk_manager::ChunkPosition;

// Data structure for serializing chunk data
#[derive(Serialize, Deserialize, Clone)]
pub struct ChunkData {
    pub position: ChunkPosition,
    pub heightmap: Vec<f32>,
    pub biome_ids: Vec<u8>,
    // Add other data as needed
}

// ChunkStorage handles saving and loading chunks from disk
pub struct ChunkStorage {
    save_dir: String,
    cache: HashMap<ChunkPosition, ChunkData>,
    cache_size_limit: usize,
}

impl ChunkStorage {
    pub fn new(save_dir: &str) -> Self {
        // Ensure directory exists
        fs::create_dir_all(save_dir).unwrap_or_else(|e| {
            eprintln!("Failed to create save directory: {}", e);
        });
        
        ChunkStorage {
            save_dir: save_dir.to_string(),
            cache: HashMap::new(),
            cache_size_limit: 100, // Store up to 100 chunks in memory
        }
    }
    
    // Check if a chunk exists in storage
    pub fn chunk_exists(&self, position: ChunkPosition) -> bool {
        // Check cache first
        if self.cache.contains_key(&position) {
            return true;
        }
        
        // Check file system
        let path = self.get_chunk_path(position);
        Path::new(&path).exists()
    }
    
    // Save a chunk to storage
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
    }
    
    // Load a chunk from storage
    pub fn load_chunk(&mut self, position: ChunkPosition) -> Option<ChunkData> {
        // Check cache first
        if let Some(data) = self.cache.get(&position) {
            return Some(data.clone());
        }
        
        // Load from file
        let path = self.get_chunk_path(position);
        match fs::read_to_string(&path) {
            Ok(json) => {
                match serde_json::from_str::<ChunkData>(&json) {
                    Ok(data) => {
                        // Add to cache if not full, otherwise manage cache
                        if self.cache.len() >= self.cache_size_limit {
                            // Simple strategy: remove a random entry
                            if let Some(key_to_remove) = self.cache.keys().next().cloned() {
                                self.cache.remove(&key_to_remove);
                            }
                        }
                        
                        // Add to cache
                        self.cache.insert(position, data.clone());
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
    
    // Get the file path for a chunk
    fn get_chunk_path(&self, position: ChunkPosition) -> String {
        format!("{}/chunk_{}_{}.json", self.save_dir, position.x, position.z)
    }
    
    // Clear the cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
    
    // Set cache size limit
    pub fn set_cache_size_limit(&mut self, limit: usize) {
        self.cache_size_limit = limit;
        
        // If current cache exceeds new limit, trim it
        while self.cache.len() > self.cache_size_limit {
            if let Some(key_to_remove) = self.cache.keys().next().cloned() {
                self.cache.remove(&key_to_remove);
            } else {
                break;
            }
        }
    }
}
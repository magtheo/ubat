use godot::prelude::*;
use godot::classes::Node;
use std::collections::HashMap;

/// There is a hight chance all of this needs to be remade

/// Structure to hold chunk data
struct ChunkData {
    // Position data
    chunk_x: i32,
    chunk_y: i32,
    
    // Biome information
    biome_color: Color,
    
    // Chunk state
    is_fully_loaded: bool,
    
    // Reference to the actual chunk node (if instantiated)
    chunk_instance: Option<Gd<Node>>,
}

/// ChunkHandler manages loading, generation and unloading of world chunks
#[derive(GodotClass)]
#[class(base=Node)]
pub struct ChunkHandler {
    #[base]
    base: Base<Node>,
    
    // Map of loaded chunks by coordinates
    chunks: HashMap<(i32, i32), ChunkData>,
    
    // Configuration
    chunk_size: f32,
    chunk_scene: Gd<PackedScene>,
    max_concurrent_loads: i32,
    pending_loads: Vec<(i32, i32)>,
}

#[godot_api]
impl INode for ChunkHandler {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            chunks: HashMap::new(),
            chunk_size: 256.0,
            // This will be set in _ready from an exported property
            chunk_scene: PackedScene::new_gd(),
            max_concurrent_loads: 3,
            pending_loads: Vec::new(),
        }
    }

    fn ready(&mut self) {
        // Load chunk scene from export property
        // For now, we just create a new one
        godot_print!("ChunkHandler initialized");
    }
    
    fn process(&mut self, _delta: f64) {
        // Process any pending chunk loads
        self.process_pending_loads();
    }
}

#[godot_api]
impl ChunkHandler {
    /// Load a new chunk at the specified coordinates with the given biome color
    #[func]
    pub fn load_chunk(&mut self, chunk_x: i32, chunk_y: i32, biome_color: Color) {
        // Skip if already loaded or pending
        let chunk_key = (chunk_x, chunk_y);
        if self.chunks.contains_key(&chunk_key) {
            return;
        }
        
        // Add to pending loads if not already there
        if !self.pending_loads.contains(&chunk_key) {
            self.pending_loads.push(chunk_key);
            godot_print!("Queued chunk ({}, {}) for loading", chunk_x, chunk_y);
        }
        
        // Create initial chunk data entry
        let chunk_data = ChunkData {
            chunk_x,
            chunk_y,
            biome_color,
            is_fully_loaded: false,
            chunk_instance: None,
        };
        
        // Add to chunks map
        self.chunks.insert(chunk_key, chunk_data);
    }
    
    /// Unload a chunk at the specified coordinates
    #[func]
    pub fn unload_chunk(&mut self, chunk_x: i32, chunk_y: i32) {
        let chunk_key = (chunk_x, chunk_y);
        
        // Remove from pending loads if present
        if let Some(index) = self.pending_loads.iter().position(|&x| x == chunk_key) {
            self.pending_loads.remove(index);
        }
        
        // Remove from loaded chunks and free the instance
        if let Some(chunk_data) = self.chunks.remove(&chunk_key) {
            if let Some(chunk_instance) = chunk_data.chunk_instance {
                chunk_instance.queue_free();
            }
            godot_print!("Unloaded chunk ({}, {})", chunk_x, chunk_y);
        }
    }
    
    /// Process any pending chunk loads up to the concurrent load limit
    #[func]
    fn process_pending_loads(&mut self) {
        // Count how many chunks are currently being loaded
        let active_loads = self.chunks
            .values()
            .filter(|chunk| !chunk.is_fully_loaded && chunk.chunk_instance.is_some())
            .count() as i32;
        
        // Calculate how many new loads we can start
        let available_slots = (self.max_concurrent_loads - active_loads).max(0);
        
        // Start new loads up to the limit
        for _ in 0..available_slots {
            if let Some(&chunk_key) = self.pending_loads.first() {
                // Start loading this chunk
                self.start_chunk_load(chunk_key.0, chunk_key.1);
                self.pending_loads.remove(0);
            } else {
                break;
            }
        }
    }
    
    /// Start loading a specific chunk
    #[func]
    fn start_chunk_load(&mut self, chunk_x: i32, chunk_y: i32) {
        let chunk_key = (chunk_x, chunk_y);
        
        if let Some(chunk_data) = self.chunks.get_mut(&chunk_key) {
            // Create instance from scene
            let instance = self.chunk_scene.instantiate();
            if let Some(instance) = instance {
                // Add to scene tree
                self.base.add_child(instance.clone().upcast::<Node>());
                
                // Set position
                let world_x = chunk_x as f32 * self.chunk_size;
                let world_y = chunk_y as f32 * self.chunk_size;
                instance.set("position", Vector2::new(world_x, world_y).to_variant());
                
                // Set biome color
                instance.set("biome_color", chunk_data.biome_color.to_variant());
                
                // Update chunk data
                chunk_data.chunk_instance = Some(instance.clone());
                
                godot_print!("Started loading chunk ({}, {})", chunk_x, chunk_y);
                
                // In a real implementation, you might connect signals from the chunk for load completion
                // For now we'll just mark it as loaded immediately
                chunk_data.is_fully_loaded = true;
            } else {
                godot_error!("Failed to instantiate chunk scene");
            }
        }
    }
    
    #[func]
    pub fn set_chunk_size(&mut self, size: f32) {
        self.chunk_size = size;
    }
    
    #[func]
    pub fn get_chunk_size(&self) -> f32 {
        self.chunk_size
    }
    
    #[func]
    pub fn get_chunk_at_position(&self, world_x: f32, world_y: f32) -> Option<Gd<Node>> {
        let chunk_x = (world_x / self.chunk_size).floor() as i32;
        let chunk_y = (world_y / self.chunk_size).floor() as i32;
        
        if let Some(chunk_data) = self.chunks.get(&(chunk_x, chunk_y)) {
            chunk_data.chunk_instance.clone()
        } else {
            None
        }
    }
    
    #[func]
    pub fn get_loaded_chunk_count(&self) -> i32 {
        self.chunks.len() as i32
    }
    
    #[func]
    pub fn get_pending_load_count(&self) -> i32 {
        self.pending_loads.len() as i32
    }
}
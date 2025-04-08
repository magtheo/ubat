// File: src/bridge/terrain_bridge.rs

use godot::prelude::*;
use godot::classes::Node;
use std::sync::{Arc, Mutex};

use crate::terrain::ChunkController;

#[derive(GodotClass)]
#[class(base=Node)]
pub struct TerrainBridge {
    #[base]
    base: Base<Node>,
    
    // Reference to the ChunkController
    chunk_controller: Option<Gd<ChunkController>>,
    
    // Cache for player position
    current_chunk_x: i32,
    current_chunk_z: i32,
    chunk_size: f32,
}

#[godot_api]
impl INode for TerrainBridge {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            chunk_controller: None,
            current_chunk_x: 0,
            current_chunk_z: 0,
            chunk_size: 32.0,
        }
    }
    
    fn ready(&mut self) {
        godot_print!("TerrainBridge: Ready, waiting for initialization");
    }
}

#[godot_api]
impl TerrainBridge {
    #[func]
    pub fn initialize(&mut self, chunk_controller: Gd<ChunkController>) -> bool {
        self.chunk_controller = Some(chunk_controller);
        godot_print!("TerrainBridge: Successfully connected to ChunkController");
        true
    }
    
    #[func]
    pub fn update_player_position(&mut self, position: Vector3) -> bool {
        if let Some(controller) = &self.chunk_controller {
            let new_chunk_x = (position.x / self.chunk_size).floor() as i32;
            let new_chunk_z = (position.z / self.chunk_size).floor() as i32;
            
            // Only update if chunk changed
            if new_chunk_x != self.current_chunk_x || new_chunk_z != self.current_chunk_z {
                self.current_chunk_x = new_chunk_x;
                self.current_chunk_z = new_chunk_z;
                
                // Update the chunk controller
                let mut cc = controller.clone();
                cc.bind_mut().update_player_position(position);
                
                // Emit signal for other systems
                self.base_mut().emit_signal("terrain_updated".into(), &[
                    Vector2::new(new_chunk_x as f32, new_chunk_z as f32).to_variant()
                ]);
                
                godot_print!("Player moved to chunk: {}, {}", new_chunk_x, new_chunk_z);
                return true;
            }
        }
        false
    }
    
    #[func]
    pub fn force_update(&self) -> bool {
        if let Some(controller) = &self.chunk_controller {
            let mut cc = controller.clone();
            cc.bind_mut().force_update();
            return true;
        }
        false
    }
    
    #[func]
    pub fn get_terrain_stats(&self) -> Dictionary {
        if let Some(controller) = &self.chunk_controller {
            return controller.bind().get_stats();
        }
        Dictionary::new()
    }
    
    #[signal]
    fn terrain_updated(position: Vector2) {}
}
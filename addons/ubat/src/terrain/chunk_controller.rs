use godot::prelude::*;
use godot::classes::{MeshInstance3D, Node3D, ArrayMesh};
use std::sync::Arc;
use std::collections::{HashMap, HashSet};

use crate::terrain::BiomeManager;
use crate::terrain::ChunkManager;

// ChunkController - Main interface between Godot and the terrain generation system
#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct ChunkController {
    #[base]
    base: Base<Node3D>,
    
    // References to other components
    chunk_manager: Option<Gd<ChunkManager>>,
    biome_manager: Option<Gd<BiomeManager>>,
    
    // Configuration
    render_distance: i32,
    player_position: Vector3,
    needs_update: bool,
    
    // Visualization - if true, will create mesh instances for chunks
    visualization_enabled: bool,
    chunk_meshes: HashMap<(i32, i32), Gd<MeshInstance3D>>,
}

#[godot_api]
impl INode3D for ChunkController {
    fn init(base: Base<Node3D>) -> Self {
        ChunkController {
            base,
            chunk_manager: None,
            biome_manager: None,
            render_distance: 8,
            player_position: Vector3::ZERO,
            needs_update: true,
            visualization_enabled: true,
            chunk_meshes: HashMap::new(),
        }
    }
    
    fn ready(&mut self) {
        // Find the ChunkManager and BiomeManager in the scene tree
        let chunk_manager = self.base().get_node_as::<ChunkManager>("../ChunkManager");
        let biome_manager = self.base().get_node_as::<BiomeManager>("../BiomeManager");
        
        self.chunk_manager = Some(chunk_manager);
        self.biome_manager = Some(biome_manager);
        
        // Set the reference in ChunkManager to BiomeManager (if needed)
        if let (Some(chunk_mgr), Some(biome_mgr)) = (&self.chunk_manager, &self.biome_manager) {
            let mut chunk_mgr_mut = chunk_mgr.clone();
            chunk_mgr_mut.bind_mut().set_biome_manager(biome_mgr.clone());
        }
        
        // Initialize the render distance in the ChunkManager
        if let Some(chunk_mgr) = &self.chunk_manager {
            let mut chunk_mgr_mut = chunk_mgr.clone();
            chunk_mgr_mut.bind_mut().set_render_distance(self.render_distance);
        }

        godot_print!("ChunkController initialized");
    }
    
    fn process(&mut self, _delta: f64) {
        // Only process if we have necessary components
        if self.chunk_manager.is_none() {
            return;
        }
        
        // Update chunks based on player position if needed
        if self.needs_update {
            if let Some(ref chunk_mgr) = self.chunk_manager {
                chunk_mgr.bind().update(
                    self.player_position.x,
                    self.player_position.y,
                    self.player_position.z
                );
            }
            self.needs_update = false;
            
            // Update visualization if enabled
            if self.visualization_enabled {
                self.update_visualization();
            }
        }
    }
}

#[godot_api]
impl ChunkController {
    // Update player position (call this from Godot)
    #[func]
    pub fn update_player_position(&mut self, position: Vector3) {
        let old_chunk_x = (self.player_position.x / 32.0).floor() as i32;
        let old_chunk_z = (self.player_position.z / 32.0).floor() as i32;
        
        self.player_position = position;
        
        let new_chunk_x = (position.x / 32.0).floor() as i32;
        let new_chunk_z = (position.z / 32.0).floor() as i32;
        
        // Only flag for update if the player moved to a different chunk
        if old_chunk_x != new_chunk_x || old_chunk_z != new_chunk_z {
            self.needs_update = true;
        }
    }
    
    // Set render distance
    #[func]
    pub fn set_render_distance(&mut self, distance: i32) {
        self.render_distance = distance.max(1).min(32);
        
        if let Some(chunk_mgr) = &self.chunk_manager {
            let mut cm = chunk_mgr.clone();
            cm.bind_mut().set_render_distance(self.render_distance);
        }
        
        // Force an update next frame
        self.needs_update = true;
    }
    
    // Enable/disable visualization
    #[func]
    pub fn set_visualization_enabled(&mut self, enabled: bool) {
        self.visualization_enabled = enabled;
        
        // If disabling, clear all meshes
        if !enabled {
            for (_, mut mesh) in self.chunk_meshes.drain() {
                mesh.queue_free();
            }
        } else {
            // If enabling, force an update next frame
            self.needs_update = true;
        }
    }
    
    // Get stats as a dictionary
    #[func]
    pub fn get_stats(&self) -> Dictionary {
        let mut dict = Dictionary::new();
        
        if let Some(ref chunk_mgr) = self.chunk_manager {
            dict.insert("chunk_count", chunk_mgr.bind().get_chunk_count());
            dict.insert("render_distance", chunk_mgr.bind().get_render_distance());
        }
        
        dict.insert("visualization_enabled", self.visualization_enabled);
        
        dict
    }
    
    // Force an update on next frame
    #[func]
    pub fn force_update(&mut self) {
        self.needs_update = true;
    }
    
    // Update the mesh visualization for chunks
    fn update_visualization(&mut self) {
        // Collect all the chunks to process first
        let mut chunks_to_process: Vec<(i32, i32, bool)> = Vec::new();
        
        if let Some(ref chunk_mgr) = self.chunk_manager {
            let chunk_mgr_bind = chunk_mgr.bind();
            let player_chunk_x = (self.player_position.x / 32.0).floor() as i32;
            let player_chunk_z = (self.player_position.z / 32.0).floor() as i32;
            
            // Collect chunks in render distance
            for x in (player_chunk_x - self.render_distance)..=(player_chunk_x + self.render_distance) {
                for z in (player_chunk_z - self.render_distance)..=(player_chunk_z + self.render_distance) {
                    let is_ready = chunk_mgr_bind.is_chunk_ready(x, z);
                    chunks_to_process.push((x, z, is_ready));
                }
            }
        }
        
        // Now process chunks without holding the immutable borrow
        let mut updated_chunks = HashSet::new();
        
        for (x, z, is_ready) in chunks_to_process {
            updated_chunks.insert((x, z));
            
            // Only create mesh if chunk is ready
            if is_ready {
                self.create_or_update_chunk_mesh(x, z);
            }
        }
        
        // Remove meshes that are no longer in render distance
        let keys_to_remove: Vec<(i32, i32)> = self.chunk_meshes.keys()
            .filter(|&&key| !updated_chunks.contains(&key))
            .cloned()
            .collect();
                
        for key in keys_to_remove {
            if let Some(mut mesh) = self.chunk_meshes.remove(&key) {
                mesh.queue_free();
            }
        }
    }
    
    // Create or update a mesh for a specific chunk
    fn create_or_update_chunk_mesh(&mut self, chunk_x: i32, chunk_z: i32) {
        // First get all the data we need from the immutable borrow
        let heightmap = if let Some(ref chunk_mgr) = self.chunk_manager {
            let heightmap = chunk_mgr.bind().get_chunk_heightmap(chunk_x, chunk_z);
            if heightmap.is_empty() {
                return;
            }
            heightmap
        } else {
            return;
        };
        
        let chunk_key = (chunk_x, chunk_z);
        
        // Now no immutable borrows are active, so we can do mutable operations
        if !self.chunk_meshes.contains_key(&chunk_key) {
            let mut mesh_instance = MeshInstance3D::new_alloc();
            mesh_instance.set_position(Vector3::new(
                chunk_x as f32 * 32.0, 
                0.0, 
                chunk_z as f32 * 32.0
            ));
            
            // Now we can safely call base_mut()
            let node = mesh_instance.clone().upcast::<Node>();
            self.base_mut().add_child(&node);
            
            self.chunk_meshes.insert(chunk_key, mesh_instance);
        }
        
        // Update existing mesh
        if let Some(mesh_ref) = self.chunk_meshes.get(&chunk_key) {
            let mut mesh_mut = mesh_ref.clone();
            
            mesh_mut.set_position(Vector3::new(
                chunk_x as f32 * 32.0, 
                0.0, 
                chunk_z as f32 * 32.0
            ));
        }
    }
}
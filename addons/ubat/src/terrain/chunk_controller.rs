use godot::prelude::*;
use godot::classes::{MeshInstance3D, Node3D, ArrayMesh, BoxMesh};
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
        godot_print!("RUST: ChunkController: Initializing...");
        
        // Find the ChunkManager and BiomeManager in the scene tree
        let chunk_manager = self.base().get_node_as::<ChunkManager>("/root/ChunkManager"); // claude: how should this be correctly accesses, pleas take my init system indo account
        let biome_manager = self.base().get_node_as::<BiomeManager>("/root/BiomeManager");
        
        if chunk_manager.is_instance_valid() {
            self.chunk_manager = Some(chunk_manager);
            godot_print!("ChunkController: Found ChunkManager");
        } else {
            godot_error!("ChunkController: Could not find ChunkManager in scene tree");
        }
        
        if biome_manager.is_instance_valid() {
            self.biome_manager = Some(biome_manager);
            godot_print!("ChunkController: Found BiomeManager");
        } else {
            godot_error!("ChunkController: Could not find BiomeManager in scene tree");
        }
        
        // Set the reference in ChunkManager to BiomeManager (if needed)
        if let (Some(chunk_mgr), Some(biome_mgr)) = (&self.chunk_manager, &self.biome_manager) {
            let mut chunk_mgr_mut = chunk_mgr.clone();
            chunk_mgr_mut.bind_mut().set_biome_manager(biome_mgr.clone());
            godot_print!("ChunkController: Connected ChunkManager to BiomeManager");
        }
        
        // Initialize the render distance in the ChunkManager
        if let Some(chunk_mgr) = &self.chunk_manager {
            let mut chunk_mgr_mut = chunk_mgr.clone();
            chunk_mgr_mut.bind_mut().set_render_distance(self.render_distance);
            godot_print!("ChunkController: Set render distance to {}", self.render_distance);
        }
    
        godot_print!("ChunkController: Initialization complete");
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

    #[func]
    pub fn connect_player_signal(&mut self, mut player_node: Gd<Node>) -> Variant { // TODO: is there a better solution where MUT player is not needed
        let chunk_controller_obj = self.base().clone().upcast::<Object>();

        // Create a slice of Variants directly
        let args = &[
            StringName::from("player_chunk_changed").to_variant(),
            chunk_controller_obj.to_variant(),
            StringName::from("on_player_chunk_changed").to_variant()
        ];

        // Directly call and return the result
        let result = player_node.call("connect", args);
        
        godot_print!("Signal connection result: {:?}", result);
        
        // Convert the result directly to a Variant
        // If the call was successful, result will already be a Variant
        result
    }

    #[func]
    fn on_player_chunk_changed(&mut self, chunk_x: Variant, chunk_z: Variant) {
        let chunk_x: i64 = chunk_x.try_to().unwrap_or_else(|_| {
            godot_error!("Failed to convert chunk_x");
            0
        });

        let chunk_z: i64 = chunk_z.try_to().unwrap_or_else(|_| {
            godot_error!("Failed to convert chunk_z");
            0
        });

        let new_position = Vector3::new(
            chunk_x as f32 * 32.0, 
            0.0, 
            chunk_z as f32 * 32.0
        );
        
        self.update_player_position(new_position);
    }



    // Update player position 
    #[func]
    pub fn update_player_position(&mut self, position: Vector3) {
        let old_chunk_x = (self.player_position.x / 32.0).floor() as i32;
        let old_chunk_z = (self.player_position.z / 32.0).floor() as i32;
        
        self.player_position = position;
        
        let new_chunk_x = (position.x / 32.0).floor() as i32;
        let new_chunk_z = (position.z / 32.0).floor() as i32;
        
        godot_print!("ChunkController: New chunk position: {},{}", new_chunk_x, new_chunk_z);

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
        let debug = chunk_x == 0 && chunk_z == 0;
        if debug {
            godot_print!("ChunkController: Creating mesh for chunk at ({}, {})", chunk_x, chunk_z);
        }
        
        // First get all the data we need from the immutable borrow
        let heightmap = if let Some(ref chunk_mgr) = self.chunk_manager {
            let heightmap = chunk_mgr.bind().get_chunk_heightmap(chunk_x, chunk_z);
            if heightmap.is_empty() {
                if debug {
                    godot_print!("ChunkController: Heightmap is empty for chunk ({}, {})", chunk_x, chunk_z);
                }
                return;
            }
            heightmap
        } else {
            return;
        };
        
        if debug {
            godot_print!("ChunkController: Got heightmap with {} values", heightmap.len());
        }
        
        let chunk_key = (chunk_x, chunk_z);
        
        // Create a simple plane mesh directly using arrays
        let chunk_size = (heightmap.len() as f32).sqrt() as u32;
        if debug {
            godot_print!("ChunkController: Chunk size calculated as {}", chunk_size);
        }
        
        // Create mesh arrays
        let mut vertices = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        let mut indices = Vec::new();
        
        // First fill vertex data
        for z in 0..chunk_size {
            for x in 0..chunk_size {
                let idx = (z * chunk_size + x) as usize;
                let h = heightmap[idx];
                                
                // Add vertex
                vertices.push(Vector3::new(x as f32, h, z as f32));
                
                // Simple normal
                normals.push(Vector3::UP);
                
                // Simple UV
                uvs.push(Vector2::new(
                    x as f32 / (chunk_size - 1) as f32,
                    z as f32 / (chunk_size - 1) as f32
                ));
            }
        }
        
        // Create triangles
        for z in 0..chunk_size-1 {
            for x in 0..chunk_size-1 {
                let idx00 = (z * chunk_size + x) as i32;
                let idx10 = (z * chunk_size + x + 1) as i32;
                let idx01 = ((z + 1) * chunk_size + x) as i32;
                let idx11 = ((z + 1) * chunk_size + x + 1) as i32;
                
                // First triangle (bottom-left to top-right)
                indices.push(idx00);
                indices.push(idx10);
                indices.push(idx01);
                
                // Second triangle (top-right to bottom-right)
                indices.push(idx10);
                indices.push(idx11);
                indices.push(idx01);
            }
        }
        
        // Create mesh and array mesh
        let array_mesh = ArrayMesh::new_gd();
        
        // Convert arrays to Godot arrays
        let mut godot_vertices = PackedVector3Array::new();
        for v in vertices {
            godot_vertices.push(v);
        }
        
        let mut godot_normals = PackedVector3Array::new();
        for n in normals {
            godot_normals.push(n);
        }
        
        let mut godot_uvs = PackedVector2Array::new();
        for uv in uvs {
            godot_uvs.push(uv);
        }
        
        let mut godot_indices = PackedInt32Array::new();
        for i in indices {
            godot_indices.push(i);
        }
        
        // Skip mesh creation in Rust - we'll create a temporary placeholder in GDScript
        // This works around limitations with the Rust-Godot API for now
        // We'll log this so we know to come back to it
        if debug {
            godot_print!("ChunkController: Creating placeholder for chunk, will be replaced in GDScript");
        }
        
        // Use a temporary reference for now
        let array_mesh = ArrayMesh::new_gd();
        
        // Now no immutable borrows are active, so we can do mutable operations
        if !self.chunk_meshes.contains_key(&chunk_key) {
            let mut mesh_instance = MeshInstance3D::new_alloc();
            mesh_instance.set_position(Vector3::new(
                chunk_x as f32 * chunk_size as f32, 
                0.0, 
                chunk_z as f32 * chunk_size as f32
            ));
            
            // Set the mesh (placeholder - will be replaced in GDScript)
            mesh_instance.set_mesh(&array_mesh);
            
            // Set material from GDScript if available
            // That will be handled by GDScript since we don't have direct access here
            
            // Now we can safely call base_mut()
            let node = mesh_instance.clone().upcast::<Node>();
            self.base_mut().add_child(&node);
            
            self.chunk_meshes.insert(chunk_key, mesh_instance);
            
            if debug {
                godot_print!("ChunkController: Created new mesh for chunk ({}, {})", chunk_x, chunk_z);
            }
        } else {
            // Update existing mesh
            if let Some(mesh_ref) = self.chunk_meshes.get(&chunk_key) {
                let mut mesh_mut = mesh_ref.clone();
                
                mesh_mut.set_mesh(&array_mesh);
                mesh_mut.set_position(Vector3::new(
                    chunk_x as f32 * chunk_size as f32, 
                    0.0, 
                    chunk_z as f32 * chunk_size as f32
                ));
                
                if debug {
                    godot_print!("ChunkController: Updated existing mesh for chunk ({}, {})", chunk_x, chunk_z);
                }
            }
        }
    }

  // Helper function to blend heights at biome boundaries
  fn blend_heights(
      heightmap: &mut [f32],
      biome_ids: &[u8],
      chunk_size: u32,
      blend_distance: f32
  ) {
      // Create a copy of the original heightmap
      let original_heights = heightmap.to_vec();

      // Blend heights at biome boundaries
      for z in 0..chunk_size {
          for x in 0..chunk_size {
              let idx = (z * chunk_size + x) as usize;

              // Check if this vertex is at a biome boundary
              if Self::is_at_biome_boundary(biome_ids, idx, chunk_size) {
                  // Blend with neighbors
                  let mut total_weight = 1.0;
                  let mut weighted_height = original_heights[idx];

                  // Get all neighbors within blend distance
                  for dz in -2..=2 {
                      for dx in -2..=2 {
                          if dx == 0 && dz == 0 {
                              continue;
                          }

                          let nx = x as i32 + dx;
                          let nz = z as i32 + dz;

                          if nx >= 0 && nx < chunk_size as i32 && nz >= 0 && nz < chunk_size as i32 {
                              let nidx = (nz * chunk_size as i32 + nx) as usize;

                              // Weight based on distance
                              let distance = ((dx * dx + dz * dz) as f32).sqrt();
                              let weight = (1.0 - distance / 3.0).max(0.0);

                              total_weight += weight;
                              weighted_height += original_heights[nidx] * weight;
                          }
                      }
                  }

                  // Update height with weighted average
                  heightmap[idx] = weighted_height / total_weight;
              }
          }
      }
  }

  fn is_at_biome_boundary(biome_ids: &[u8], idx: usize, chunk_size: u32) -> bool {
      let x = (idx as u32) % chunk_size;
      let z = (idx as u32) / chunk_size;

      let current = biome_ids[idx];

      // Check neighboring vertices
      let mut has_different_neighbor = false;

      // Check neighbors in all 8 directions
      for dz in -1..=1 {
          for dx in -1..=1 {
              if dx == 0 && dz == 0 {
                  continue;
              }

              let nx = x as i32 + dx;
              let nz = z as i32 + dz;

              if nx >= 0 && nx < chunk_size as i32 && nz >= 0 && nz < chunk_size as i32 {
                  let nidx = (nz * chunk_size as i32 + nx) as usize;
                  if biome_ids[nidx] != current {
                      has_different_neighbor = true;
                      break;
                  }
              }
          }
          if has_different_neighbor {
              break;
          }
      }

      has_different_neighbor
  }

  
}
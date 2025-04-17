use godot::prelude::*;
use godot::global::Error;
use godot::classes::{MeshInstance3D, Node3D, ArrayMesh, Mesh, Material, ResourceLoader};
use std::collections::{HashMap, HashSet};
use godot::classes::mesh::{PrimitiveType, ArrayType};

// Use ChunkManager and its types
use crate::terrain::chunk_manager::{ChunkManager, ChunkPosition};
// Use BiomeManager if needed for materials etc.
use crate::terrain::biome_manager::BiomeManager;
// Use TerrainConfig to get chunk size if needed
use crate::terrain::terrain_config::TerrainConfigManager;

// --- Helper function to safely get height, clamping at edges ---
// Returns the height at (x, z) within the heightmap, clamping coordinates to chunk bounds.
fn get_clamped_height(x: i32, z: i32, heightmap: &[f32], chunk_size: u32) -> f32 {
    // Clamp coordinates to be within the valid range [0, chunk_size - 1]
    let clamped_x = x.clamp(0, chunk_size as i32 - 1) as u32;
    let clamped_z = z.clamp(0, chunk_size as i32 - 1) as u32;
    let idx = (clamped_z * chunk_size + clamped_x) as usize;
    // Safety check for index bounds (shouldn't be necessary with clamp, but good practice)
    heightmap.get(idx).copied().unwrap_or(0.0)
}

#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct ChunkController {
    #[base]
    base: Base<Node3D>,
    chunk_manager: Option<Gd<ChunkManager>>,
    // Keep BiomeManager ref if needed (e.g., for getting materials)
    // biome_manager: Option<Gd<BiomeManager>>,

    // Config/State
    render_distance: i32,
    player_position: Vector3,
    needs_update: bool,
    chunk_size: u32, // Store chunk size locally for convenience

    // Visualization
    visualization_enabled: bool,
    chunk_meshes: HashMap<ChunkPosition, Gd<MeshInstance3D>>, // Use ChunkPosition as key
    // Optional: Preload materials
    // default_material: Option<Gd<Material>>,
}

#[godot_api]
impl INode3D for ChunkController {
    fn init(base: Base<Node3D>) -> Self {
        ChunkController {
            base,
            chunk_manager: None,
            // biome_manager: None,
            render_distance: 4, // TODO This overides terrain initalizer, and it shuold not
            player_position: Vector3::ZERO,
            needs_update: true,
            chunk_size: 32, // Default, will be updated in ready
            visualization_enabled: true,
            chunk_meshes: HashMap::new(),
            // default_material: None,
        }
    }

    fn ready(&mut self) {
        godot_print!("ChunkController: Initializing...");

        // Find ChunkManager sibling
        if let Some(parent) = self.base().get_parent() {
            self.chunk_manager = Some(parent.get_node_as::<ChunkManager>("ChunkManager"));
            // self.biome_manager = parent.get_node_as::<BiomeManager>("BiomeManager"); // Find if needed

            if self.chunk_manager.is_none() {
                godot_error!("ChunkController: Could not find ChunkManager sibling!");
            }
        } else {
            godot_error!("ChunkController: No parent node found!");
        }

        // Get initial render distance and chunk size from ChunkManager/Config
        if let Some(cm) = &self.chunk_manager {
            let cm_bind = cm.bind();
            self.render_distance = cm_bind.get_render_distance();

            // Get chunk size directly from config manager for consistency
            if let Some(config_arc) = TerrainConfigManager::get_config() {
                if let Ok(guard) = config_arc.read() {
                    self.chunk_size = guard.chunk_size();
                } else {
                     godot_error!("ChunkController: Failed to read config for chunk size.");
                 }
            } else {
                 godot_warn!("ChunkController: Config manager not available for chunk size.");
             }

            godot_print!("ChunkController: Initial render distance: {}, chunk size: {}", self.render_distance, self.chunk_size);
        } else {
            godot_error!("ChunkController: Cannot get initial settings, ChunkManager not found!");
        }

        // Preload default material (example)
        // let loader = ResourceLoader::singleton();
        // self.default_material = loader.load("res://materials/default_terrain_mat.tres".into()); // Adjust path

        godot_print!("ChunkController: Initialization complete.");
    }

    fn process(&mut self, _delta: f64) {
        if self.chunk_manager.is_none() { return; } // Need ChunkManager

        if self.needs_update {
            if let Some(ref chunk_mgr) = self.chunk_manager {
                // Call update on ChunkManager
                chunk_mgr.bind().update(
                    self.player_position.x,
                    self.player_position.y, // Pass Y if needed
                    self.player_position.z
                );
            }
            self.needs_update = false; // Reset flag

            // Update visualization only if needed and enabled
            if self.visualization_enabled {
                self.update_visualization();
            }
        }
    }
}

#[godot_api]
impl ChunkController {
    // Connect player signal (using Godot's built-in connect)
    #[func]
    pub fn connect_player_signal(&mut self, player_node: Gd<Node>) -> bool {
        let mut player = player_node; // Make mutable for connect
        let target_object = self.base().clone().cast::<ChunkController>(); // Get self as Gd
        let method_name = StringName::from("on_player_chunk_changed");
        let target_callable = Callable::from_object_method(&target_object, &method_name);

        // Connect signal directly
        let result = player.connect(
            "player_chunk_changed", // Signal name
            &target_callable,
        );

        // Check result code (OK = 0)
        if result == Error::OK {
            godot_print!("ChunkController: Successfully connected 'player_chunk_changed' signal.");
            true
        } else {
            godot_error!("ChunkController: Failed to connect player signal, error code: {:?}", result);
            false
        }
    }

    // Signal handler when player moves to a new chunk
    #[func]
    fn on_player_chunk_changed(&mut self, chunk_x_var: Variant, chunk_z_var: Variant) {
        // First try to convert directly to i32
        godot_print!("ChunkController Rust: on_player_chunk_changed CALLED with variants: {:?}, {:?}", chunk_x_var, chunk_z_var);
        let chunk_x = match chunk_x_var.try_to::<i32>() {
            Ok(x) => x,
            Err(_) => {
                // If that fails, try converting from float to i32
                match chunk_x_var.try_to::<f32>() {
                    Ok(x_float) => x_float.floor() as i32, // Convert float to int
                    Err(e) => {
                        godot_error!("Failed to convert chunk_x Variant to i32 or f32: {:?}", e);
                        return;
                    }
                }
            }
        };
        
        // Same for z coordinate
        let chunk_z = match chunk_z_var.try_to::<i32>() {
            Ok(z) => z,
            Err(_) => {
                match chunk_z_var.try_to::<f32>() {
                    Ok(z_float) => z_float.floor() as i32, // Convert float to int
                    Err(e) => {
                        godot_error!("Failed to convert chunk_z Variant to i32 or f32: {:?}", e);
                        return;
                    }
                }
            }
        };
    
        // Calculate the *center* world position of the player's new chunk for update trigger
        let new_position = Vector3::new(
            (chunk_x as f32 + 0.5) * self.chunk_size as f32, // Center X
            self.player_position.y, // Keep current Y? Or query ground height?
            (chunk_z as f32 + 0.5) * self.chunk_size as f32  // Center Z
        );
    
        self.update_player_position(new_position); // Trigger update logic
    }

    // Update internal player position and flag for chunk updates
    #[func]
    pub fn update_player_position(&mut self, position: Vector3) {
        // Calculate old and new chunk coords based on stored chunk size
        let old_chunk_x = (self.player_position.x / self.chunk_size as f32).floor() as i32;
        let old_chunk_z = (self.player_position.z / self.chunk_size as f32).floor() as i32;

        self.player_position = position; // Update stored position

        let new_chunk_x = (position.x / self.chunk_size as f32).floor() as i32;
        let new_chunk_z = (position.z / self.chunk_size as f32).floor() as i32;

        // If the player moved to a different chunk, set the update flag
        if old_chunk_x != new_chunk_x || old_chunk_z != new_chunk_z {
            // godot_print!("ChunkController: Player entered chunk ({}, {})", new_chunk_x, new_chunk_z);
            self.needs_update = true;
        }
    }

    // Set render distance and update ChunkManager
    #[func]
    pub fn set_render_distance(&mut self, distance: i32) {
        let new_distance = distance.max(1).min(32); // Clamp value
        if new_distance != self.render_distance {
            self.render_distance = new_distance;
            // Update ChunkManager's render distance
            if let Some(chunk_mgr) = &mut self.chunk_manager {
                chunk_mgr.bind_mut().set_render_distance(self.render_distance);
            } else {
                 godot_warn!("ChunkController: ChunkManager not found when setting render distance.");
            }
            self.needs_update = true; // Force update to load/unload based on new distance
            godot_print!("ChunkController: Render distance set to {}", self.render_distance);
        }
    }

    // Enable/disable visualization and manage existing meshes
    #[func]
    pub fn set_visualization_enabled(&mut self, enabled: bool) {
        if enabled != self.visualization_enabled {
            self.visualization_enabled = enabled;
            if !enabled {
                // Clear all stored meshes if disabling
                for (_, mut mesh_instance) in self.chunk_meshes.drain() {
                    if mesh_instance.is_instance_valid() {
                        mesh_instance.queue_free();
                    }
                }
                godot_print!("ChunkController: Visualization disabled, meshes cleared.");
            } else {
                self.needs_update = true; // Force update to create meshes if enabling
                godot_print!("ChunkController: Visualization enabled.");
            }
        }
    }

    // Get stats dictionary
    #[func]
    pub fn get_stats(&self) -> Dictionary {
        let mut dict = Dictionary::new();
        if let Some(ref chunk_mgr) = self.chunk_manager {
            let cm_bind = chunk_mgr.bind();
            dict.insert("managed_chunk_states", cm_bind.get_chunk_count());
            dict.insert("render_distance", cm_bind.get_render_distance());
        } else {
            // Provide defaults if manager is missing
            dict.insert("managed_chunk_states", 0);
            dict.insert("render_distance", self.render_distance);
        }
        dict.insert("visualization_enabled", self.visualization_enabled);
        dict.insert("visualized_mesh_count", self.chunk_meshes.len() as i64);
        dict
    }

    // Force an update in the next process frame
    #[func]
    pub fn force_update(&mut self) {
        self.needs_update = true;
    }

    // Update the visual representation of chunks
    fn update_visualization(&mut self) {
        if self.chunk_manager.is_none() { return; }
        
        // Fix: Create a local data copy to avoid borrow conflicts
        let player_chunk_x = (self.player_position.x / self.chunk_size as f32).floor() as i32;
        let player_chunk_z = (self.player_position.z / self.chunk_size as f32).floor() as i32;
        let render_distance = self.render_distance;
        let chunk_size = self.chunk_size;
        
        let mut active_chunks_in_view = HashSet::new();
        let chunk_mgr = self.chunk_manager.as_ref().unwrap().clone();  // Clone to avoid borrow issues

        // Iterate within render distance
        for x in (player_chunk_x - render_distance)..=(player_chunk_x + render_distance) {
            for z in (player_chunk_z - render_distance)..=(player_chunk_z + render_distance) {
                let pos = ChunkPosition { x, z };
                active_chunks_in_view.insert(pos); // Mark this chunk as required visually

                // Check if the chunk data is ready in ChunkManager
                if chunk_mgr.bind().is_chunk_ready(x, z) {
                    // If ready, ensure its mesh exists
                    if !self.chunk_meshes.contains_key(&pos) {
                        // Mesh doesn't exist, need to create it. Get data.
                        let heightmap = chunk_mgr.bind().get_chunk_heightmap(x, z);
                        // let biomes = chunk_mgr.bind().get_chunk_biomes(x, z); // Get if needed

                        if !heightmap.is_empty() {
                             // Got data, create the mesh instance
                            self.create_or_update_chunk_mesh(pos, &heightmap.to_vec());
                        } else {
                            // Chunk ready but data fetch failed (should be rare)
                            godot_warn!("ChunkController: Chunk {:?} is Ready but heightmap is empty for visualization.", pos);
                        }
                    }
                    // If mesh already exists, assume it's up-to-date for now
                } else {
                    // Chunk is not ready, remove its mesh if it exists
                    if let Some(mut mesh_instance) = self.chunk_meshes.remove(&pos) {
                        if mesh_instance.is_instance_valid() {
                            // godot_print!("ChunkController: Removing mesh for non-ready chunk {:?}", pos);
                            mesh_instance.queue_free();
                        }
                    }
                }
            }
        }

        // Remove meshes that are no longer in the active view area
        let keys_to_remove: Vec<ChunkPosition> = self.chunk_meshes.keys()
            .filter(|&key| !active_chunks_in_view.contains(key))
            .cloned()
            .collect();

        for key in keys_to_remove {
            if let Some(mut mesh_instance) = self.chunk_meshes.remove(&key) {
                if mesh_instance.is_instance_valid() {
                    // godot_print!("ChunkController: Removing mesh for out-of-view chunk {:?}", key);
                    mesh_instance.queue_free();
                }
            }
        }
    }

    // Create or update a MeshInstance3D for a chunk
    fn create_or_update_chunk_mesh(&mut self, pos: ChunkPosition, heightmap: &[f32]) {
        let chunk_size = self.chunk_size;
        if chunk_size == 0 { godot_error!("Chunk size is 0!"); return; }
        let expected_len = (chunk_size * chunk_size) as usize;
        if heightmap.len() != expected_len {
            godot_error!("Heightmap size mismatch for chunk {:?}! Expected {}, got {}", pos, expected_len, heightmap.len());
            return;
        }
    
        // --- Generate Vertices and UVs ---
        let mut vertices_vec = Vec::with_capacity(expected_len);
        let mut uvs_vec = Vec::with_capacity(expected_len);
        for z in 0..chunk_size {
            for x in 0..chunk_size {
                let idx = (z * chunk_size + x) as usize;
                let h = heightmap[idx];
                vertices_vec.push(Vector3::new(x as f32, h, z as f32)); // Local chunk coords
                uvs_vec.push(Vector2::new(
                    x as f32 / (chunk_size - 1).max(1) as f32,
                    z as f32 / (chunk_size - 1).max(1) as f32
                ));
            }
        }
    
        // --- Generate Normals (using finite difference & edge clamping) ---
        let mut normals_vec = Vec::with_capacity(expected_len);
        for z in 0..chunk_size {
            for x in 0..chunk_size {
                // Get heights of neighbours using the helper function for safety/clamping
                let h_l = get_clamped_height(x as i32 - 1, z as i32, heightmap, chunk_size); // Left
                let h_r = get_clamped_height(x as i32 + 1, z as i32, heightmap, chunk_size); // Right
                let h_d = get_clamped_height(x as i32, z as i32 - 1, heightmap, chunk_size); // Down (Top in UV)
                let h_u = get_clamped_height(x as i32, z as i32 + 1, heightmap, chunk_size); // Up (Bottom in UV)
    
                // Calculate differences (gradients)
                let dx = h_l - h_r; // Change in height over 2 units in x
                let dz = h_d - h_u; // Change in height over 2 units in z
    
                // Calculate the normal vector (perpendicular to the surface slope)
                // The Y component '2.0' scales relative to the distance between samples (which is 2 units here: x-1 to x+1)
                let normal = Vector3::new(dx, 2.0, dz).normalized();
    
                normals_vec.push(normal);
            }
        }
    
        // --- Generate Indices (with corrected winding order) ---
        let index_count = (chunk_size as usize - 1) * (chunk_size as usize - 1) * 6;
        let mut indices_vec = Vec::with_capacity(index_count);
        for z in 0..chunk_size - 1 {
            for x in 0..chunk_size - 1 {
                let idx00 = (z * chunk_size + x) as i32;        // Top-left
                let idx10 = idx00 + 1;                          // Top-right
                let idx01 = idx00 + chunk_size as i32;          // Bottom-left
                let idx11 = idx01 + 1;                          // Bottom-right
    
                // Triangle 1 (TL -> TR -> BL : CCW)
                indices_vec.push(idx00);
                indices_vec.push(idx10);
                indices_vec.push(idx01);
    
                // Triangle 2 (TR -> BR -> BL : CCW)
                indices_vec.push(idx10);
                indices_vec.push(idx11);
                indices_vec.push(idx01);
            }
        }
    
        // --- Convert to Godot Packed Arrays ---
        let vertices = PackedVector3Array::from(&vertices_vec[..]);
        let normals = PackedVector3Array::from(&normals_vec[..]);
        let uvs = PackedVector2Array::from(&uvs_vec[..]);
        let indices = PackedInt32Array::from(&indices_vec[..]);
    
        // --- Create/Update Godot Mesh ---
        let mut array_mesh = ArrayMesh::new_gd();
        let mut arrays = VariantArray::new();
        arrays.resize(13_usize, &Variant::nil()); // Resize for index 12
    
        // Set arrays at correct indices, casting enum .ord() to usize
        arrays.set(ArrayType::VERTEX.ord() as usize, &vertices.to_variant());
        arrays.set(ArrayType::NORMAL.ord() as usize, &normals.to_variant());
        arrays.set(ArrayType::TEX_UV.ord() as usize, &uvs.to_variant());
        arrays.set(ArrayType::INDEX.ord() as usize, &indices.to_variant());
    
        // Add surface using the 2-argument method
        array_mesh.add_surface_from_arrays(
            PrimitiveType::TRIANGLES,
            &arrays,
        );

        // --- **UPCAST ONCE HERE** ---
        // array_mesh (Gd<ArrayMesh>) is moved by upcast,
        // but we get mesh_resource (Gd<Mesh>) which holds a reference to the underlying mesh data.
        // This Gd<Mesh> can be shared and used multiple times via references.
        let mesh_resource: Gd<Mesh> = array_mesh.upcast();

        // --- Create/Update MeshInstance3D ---
        let chunk_world_pos = Vector3::new(
            pos.x as f32 * chunk_size as f32,
            0.0,
            pos.z as f32 * chunk_size as f32
        );

        // Flag to track if we need to create a new instance
        let mut create_new_instance = true;

        if let Some(mesh_instance) = self.chunk_meshes.get_mut(&pos) {
            // Check if the existing instance is valid
            if mesh_instance.is_instance_valid() {
                // Update existing instance using a reference to the Gd<Mesh>
                mesh_instance.set_mesh(&mesh_resource); // Pass reference to Gd<Mesh>
                mesh_instance.set_position(chunk_world_pos);
                create_new_instance = false; // Found and updated, no need to create
            } else {
                // Instance was in the map but invalid (e.g., freed elsewhere)
                godot_error!("MeshInstance for chunk {:?} was invalid. Removing.", pos);
                self.chunk_meshes.remove(&pos);
                // Keep create_new_instance = true to make a new one
            }
        }

        // Create a new instance ONLY if it wasn't found or the existing one was invalid
        if create_new_instance {
            let mut mesh_instance = MeshInstance3D::new_alloc();
            // Set mesh using a reference to the same Gd<Mesh>
            mesh_instance.set_mesh(&mesh_resource); // Pass reference to Gd<Mesh>
            mesh_instance.set_position(chunk_world_pos);
            let mesh_name: GString = format!("ChunkMesh_{}_{}", pos.x, pos.z).into();
            mesh_instance.set_name(&mesh_name);
            self.base_mut().add_child(&mesh_instance.clone().upcast::<Node>());
            self.chunk_meshes.insert(pos, mesh_instance);
        }

    }
    
}

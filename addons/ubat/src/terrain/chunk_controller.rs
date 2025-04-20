use godot::prelude::*;
use godot::global::Error;
use godot::classes::{MeshInstance3D, Node3D, ArrayMesh, Mesh, Material, ResourceLoader};
use std::collections::{HashMap, HashSet};
use godot::classes::mesh::{PrimitiveType, ArrayType};

use std::sync::{Arc, RwLock};

// Use ChunkManager and its types
use crate::terrain::chunk_manager::{ChunkManager, ChunkPosition};
// Use BiomeManager if needed for materials etc.
use crate::terrain::biome_manager::BiomeManager;
// Use TerrainConfig to get chunk size if needed
use crate::terrain::terrain_config::{TerrainConfigManager, TerrainConfig};

use crate::threading::chunk_storage::MeshGeometry;

// --- Helper function to safely get height, clamping at edges ---
// Returns the height at (x, z) within the heightmap, clamping coordinates to chunk bounds.
pub fn get_clamped_height(x: i32, z: i32, heightmap: &[f32], chunk_size: u32) -> f32 {
    // Clamp coordinates to be within the valid range [0, chunk_size - 1]
    let clamped_x = x.clamp(0, chunk_size as i32 - 1) as u32;
    let clamped_z = z.clamp(0, chunk_size as i32 - 1) as u32;
    let idx = (clamped_z * chunk_size + clamped_x) as usize;
    // Safety check for index bounds (shouldn't be necessary with clamp, but good practice)
    heightmap.get(idx).copied().unwrap_or(0.0)
}

#[derive(Clone)] // Need Clone if we store MeshGeometry directly
enum ChunkAction {
    CreateMesh(ChunkPosition, MeshGeometry),
    RemoveMesh(ChunkPosition),
    Keep,
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
            let config_arc:&'static Arc<RwLock<TerrainConfig>> = TerrainConfigManager::get_config(); // Get static ref
            if let Ok(guard) = config_arc.read() { // Lock it
                self.chunk_size = guard.chunk_size; // Access field
            } else {
                godot_error!("ChunkController: Failed to read terrain config lock for chunk size.");
                // Keep default self.chunk_size
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

        let player_chunk_x = (self.player_position.x / self.chunk_size as f32).floor() as i32;
        let player_chunk_z = (self.player_position.z / self.chunk_size as f32).floor() as i32;
        let render_distance = self.render_distance;

        let mut actions_to_take = HashMap::<ChunkPosition, ChunkAction>::new();
        let mut current_visible_keys = HashSet::new(); // Track keys currently managed visually

        // --- Phase 1: Determine Action for Each Chunk (Minimize Borrow Conflicts) ---
        { // Scope for immutable borrows of self needed for reads
            let chunk_manager_bind = self.chunk_manager.as_ref().unwrap().bind(); // Immutable borrow of self.chunk_manager

            // Determine actions for chunks within render distance
            for x in (player_chunk_x - render_distance)..=(player_chunk_x + render_distance) {
                for z in (player_chunk_z - render_distance)..=(player_chunk_z + render_distance) {
                    let pos = ChunkPosition { x, z };
                    current_visible_keys.insert(pos); // Mark as potentially visible

                    let is_ready = chunk_manager_bind.is_chunk_ready(x, z);
                    // Immutable borrow of self.chunk_meshes
                    let mesh_exists = self.chunk_meshes.contains_key(&pos);

                    let action = if is_ready {
                        if !mesh_exists {
                            // Ready but no mesh -> try to get data to create
                            if let Some(chunk_data) = chunk_manager_bind.get_cached_chunk_data(x, z) {
                                if let Some(geometry) = chunk_data.mesh_geometry {
                                    // Clone geometry here to own it for the action
                                    ChunkAction::CreateMesh(pos, geometry.clone())
                                } else {
                                    godot_warn!("ChunkController: Chunk {:?} Ready, data found, but mesh_geometry is None.", pos);
                                    ChunkAction::Keep // Cannot create mesh
                                }
                            } else {
                                godot_error!("ChunkController: Chunk {:?} Ready, but failed to retrieve cached data!", pos);
                                ChunkAction::Keep // Cannot create mesh
                            }
                        } else {
                            ChunkAction::Keep // Ready and mesh exists
                        }
                    } else { // Not ready
                        if mesh_exists {
                            ChunkAction::RemoveMesh(pos) // Not ready but mesh exists
                        } else {
                            ChunkAction::Keep // Not ready and no mesh exists
                        }
                    };
                    actions_to_take.insert(pos, action);
                }
            }

             // Determine actions for chunks currently visualized but now out of view
             // Need to clone keys to avoid borrowing self.chunk_meshes while iterating and modifying actions_to_take
             let existing_mesh_keys: Vec<ChunkPosition> = self.chunk_meshes.keys().cloned().collect();
             for pos in existing_mesh_keys {
                 if !current_visible_keys.contains(&pos) {
                     // If it's tracked visually but not in the current view range, mark for removal
                      actions_to_take.insert(pos, ChunkAction::RemoveMesh(pos));
                 }
             }

        } // Immutable borrows end here

        // --- Phase 2: Execute Actions (Mutable Borrow Allowed) ---
        for (_pos, action) in actions_to_take {
            match action {
                ChunkAction::CreateMesh(pos, geometry) => {
                    // Check again if mesh was somehow created between phases (unlikely but safe)
                    if !self.chunk_meshes.contains_key(&pos) {
                         self.apply_mesh_data_to_instance(pos, &geometry);
                    }
                }
                ChunkAction::RemoveMesh(pos) => {
                    if let Some(mut mesh_instance) = self.chunk_meshes.remove(&pos) {
                        if mesh_instance.is_instance_valid() {
                            mesh_instance.queue_free();
                        }
                    }
                }
                ChunkAction::Keep => {
                    // Do nothing
                }
            }
        }
    }

    // Create or update a MeshInstance3D for a chunk
    /// Creates or updates the visual MeshInstance3D using pre-calculated geometry data.
    /// This function performs only Godot API calls and MUST run on the main thread.
    fn apply_mesh_data_to_instance(&mut self, pos: ChunkPosition, geometry: &MeshGeometry) {
        // The `geometry` argument now contains chunk mesh data.

        // --- Convert Vecs to Godot Packed Arrays ---
        // Check if geometry is empty (e.g., from failed generation)
        if geometry.vertices.is_empty() || geometry.indices.is_empty() {
            godot_warn!("Attempted to apply empty mesh geometry for chunk {:?}", pos);
            // Optionally remove existing mesh instance if it exists
            if let Some(mut mesh_instance) = self.chunk_meshes.remove(&pos) {
                if mesh_instance.is_instance_valid() {
                    mesh_instance.queue_free();
                }
            }
            return;
        }

        let vertices_gd: Vec<Vector3> = geometry.vertices.iter().map(|v| Vector3::new(v[0], v[1], v[2])).collect();
        let normals_gd: Vec<Vector3> = geometry.normals.iter().map(|n| Vector3::new(n[0], n[1], n[2])).collect();
        let uvs_gd: Vec<Vector2> = geometry.uvs.iter().map(|u| Vector2::new(u[0], u[1])).collect();

        // --- Convert Vecs to Godot Packed Arrays ---
        let vertices = PackedVector3Array::from(&vertices_gd[..]); // Use from_slice
        let normals = PackedVector3Array::from(&normals_gd[..]);   // Use from_slice
        let uvs = PackedVector2Array::from(&uvs_gd[..]);       // Use from_slice
        let indices = PackedInt32Array::from(&geometry.indices[..]); // Use from_slice

        // --- Create/Update Godot Mesh ---
        let mut array_mesh = ArrayMesh::new_gd();
        let mut arrays = VariantArray::new();
        arrays.resize(13_usize, &Variant::nil()); // TODO: Find a way to access godot max array size dynamicaly
    
        // Set arrays at correct indices, casting enum .ord() to usize
        arrays.set(ArrayType::VERTEX.ord() as usize, &vertices.to_variant());
        arrays.set(ArrayType::NORMAL.ord() as usize, &normals.to_variant());
        arrays.set(ArrayType::TEX_UV.ord() as usize, &uvs.to_variant());
        arrays.set(ArrayType::INDEX.ord() as usize, &indices.to_variant());

        array_mesh.add_surface_from_arrays(
            PrimitiveType::TRIANGLES,
            &arrays,
        );

        let mesh_resource: Gd<Mesh> = array_mesh.upcast();

        // --- Create/Update MeshInstance3D ---
        let chunk_world_pos = Vector3::new(
            pos.x as f32 * self.chunk_size as f32,
            0.0, // Base position, actual height is in vertices
            pos.z as f32 * self.chunk_size as f32,
        );

        if let Some(mesh_instance) = self.chunk_meshes.get_mut(&pos) {
            if mesh_instance.is_instance_valid() {
                // Update existing instance
                mesh_instance.set_mesh(&mesh_resource);
                mesh_instance.set_position(chunk_world_pos); // Ensure position is correct
                // No need to re-add child or change name
            } else {
                 // Instance in map but invalid, remove and recreate below
                 godot_error!("MeshInstance for chunk {:?} was invalid. Will recreate.", pos);
                 self.chunk_meshes.remove(&pos); // Remove invalid entry
                 // Fall through to create new instance
            }
        }

        // If it wasn't in the map, or the existing one was invalid, create a new one
        if !self.chunk_meshes.contains_key(&pos) {
             let mut mesh_instance = MeshInstance3D::new_alloc();
             mesh_instance.set_mesh(&mesh_resource);
             mesh_instance.set_position(chunk_world_pos);
             let mesh_name: GString = format!("ChunkMesh_{}_{}", pos.x, pos.z).into();
             mesh_instance.set_name(&mesh_name);

             // Add to scene and store
             self.base_mut().add_child(&mesh_instance.clone().upcast::<Node>());
             self.chunk_meshes.insert(pos, mesh_instance);
             // godot_print!("Created mesh instance for chunk {:?}", pos);
        }
    }    
}

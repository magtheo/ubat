use godot::prelude::*;
use godot::global::Error;
use godot::classes::{MeshInstance3D, Node3D, ArrayMesh, Mesh, Material, ResourceLoader, ShaderMaterial};
use std::collections::{HashMap, HashSet};
use godot::classes::mesh::{PrimitiveType, ArrayType};

// Use ChunkManager and its types
use crate::terrain::chunk_manager::{ChunkManager, ChunkPosition};
// Use BiomeManager if needed for materials etc.
// Use TerrainConfig to get chunk size if needed
use crate::terrain::terrain_config::{TerrainConfigManager, TerrainConfig};
use crate::terrain::generation_utils::{generate_mesh_geometry, get_clamped_height};
use std::collections::VecDeque;
use crate::threading::chunk_storage::MeshGeometry;


// Define constants for debug modes for clarity
const DEBUG_MODE_NORMAL: i32 = 0;
const DEBUG_MODE_HEIGHT: i32 = 1;
const DEBUG_MODE_BIOME_ID: i32 = 2;
// Add more constants if you add more modes


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

    // Config/State
    render_distance: i32,
    player_position: Vector3,
    needs_update: bool,
    chunk_size: u32, // Store chunk size locally for convenience

    // Visualization
    visualization_enabled: bool,
    chunk_meshes: HashMap<ChunkPosition, Gd<MeshInstance3D>>, // Use ChunkPosition as key
    biome_material: Option<Gd<Material>>,
    // Optional: Preload materials
    // default_material: Option<Gd<Material>>,

    // Debugging
    debug_mode: i32, // 0: Normal, 1: Height Vis, 2: Biome ID Vis, etc.
    needs_visual_update: bool, // Flag to force mesh recreation

    mesh_creation_queue: VecDeque<ChunkPosition>, // Queue positions needing meshes
    mesh_removal_queue: VecDeque<ChunkPosition>, // Queue positions for mesh removal
    mesh_updates_per_frame: usize, // Store the configured limit

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
            biome_material: None,

            mesh_creation_queue: VecDeque::new(),
            mesh_removal_queue: VecDeque::new(),
            mesh_updates_per_frame: 4, // Initial default, overridden in ready
            
            debug_mode: 0,
            needs_visual_update: false,

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
            let config_arc = TerrainConfigManager::get_config(); // Get static ref
            if let Ok(guard) = config_arc.read() { // Lock it
                self.chunk_size = guard.chunk_size; // Access field
                self.mesh_updates_per_frame = guard.mesh_updates_per_frame;
            } else {
                godot_error!("ChunkController: Failed to read terrain config lock for chunk size or mesh_updates_per_frame");
                // Keep default self.chunk_size
            }

            godot_print!("ChunkController: Initial render distance: {}, chunk size: {}", self.render_distance, self.chunk_size);
        } else {
            godot_error!("ChunkController: Cannot get initial settings, ChunkManager not found!");
        }

        let mut loader = ResourceLoader::singleton();
        let path = "res://project/terrain/shader/terrain_material.tres";
        match loader.load(path) {
            Some(res) => {
                match res.try_cast::<Material>() {
                    Ok(mat) => {
                        self.biome_material = Some(mat);
                        godot_print!("ChunkController: Loaded biome material from {}", path);
                    }
                    Err(_) => {
                         godot_error!("ChunkController: Failed to cast resource at {} to Material.", path);
                         self.biome_material = None; // Ensure it's None on error
                    }
                }
            }
            None => {
                godot_error!("ChunkController: Failed to load biome material resource at {}. Check path.", path);
                self.biome_material = None; // Ensure it's None on error
            }
        }
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

        // Add check for visual update flag
        if self.needs_visual_update {
            self.force_regenerate_visuals();
            self.needs_visual_update = false;
        }

        self.process_mesh_queues();
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
        let mut current_visible_keys = HashSet::new();

        // --- Phase 1: Determine Action ---
        { // Scope for chunk_manager_bind read lock
            let chunk_manager_bind = self.chunk_manager.as_ref().unwrap().bind();

            // Identify chunks needing creation
            for x in (player_chunk_x - render_distance)..=(player_chunk_x + render_distance) {
                for z in (player_chunk_z - render_distance)..=(player_chunk_z + render_distance) {
                    let pos = ChunkPosition { x, z };
                    current_visible_keys.insert(pos);

                    let is_ready = chunk_manager_bind.is_chunk_ready(x, z);
                    let mesh_exists = self.chunk_meshes.contains_key(&pos);

                    if is_ready && !mesh_exists {
                        // Need to create mesh, enqueue position if not already queued
                        // Simple check: avoids adding duplicates in the same frame
                        if !self.mesh_creation_queue.contains(&pos) {
                             // godot_print!("ChunkController: Enqueuing {:?} for mesh creation.", pos); // Debug log
                             self.mesh_creation_queue.push_back(pos);
                        }
                    }
                    // Note: We don't need to handle the case where it's ready and mesh exists here,
                    // nor the case where it's not ready and no mesh exists.
                }
            }

            // Identify meshes needing removal
            let existing_mesh_keys: Vec<ChunkPosition> = self.chunk_meshes.keys().cloned().collect();
            for pos in existing_mesh_keys {
                if !current_visible_keys.contains(&pos) {
                    // Mesh exists but is out of range, enqueue for removal if not already queued
                     if !self.mesh_removal_queue.contains(&pos) {
                          // godot_print!("ChunkController: Enqueuing {:?} for mesh removal.", pos); // Debug log
                          self.mesh_removal_queue.push_back(pos);
                     }
                }
            }
        } // chunk_manager_bind lock released

        // --- Phase 2: Execute Actions (Will be modified in Phase 2 of plan) ---
        // For now, keep immediate execution to test Phase 1 works
        for (_pos, action) in actions_to_take {
            match action {
                ChunkAction::CreateMesh(pos, geometry) => {
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
                ChunkAction::Keep => { /* Do nothing */ }
            }
        }
    }


    #[func]
    pub fn set_debug_visualization_mode(&mut self, mode: i32) {
        if mode != self.debug_mode {
            godot_print!("ChunkController: Setting debug viz mode to {}", mode);
            self.debug_mode = mode.max(0); // Ensure non-negative
            // Force existing meshes to be updated/recreated
            self.needs_visual_update = true;
         }
    }

    // Helper to force regeneration (can be called internally or exposed)
    // This is a simple approach: remove all, let update recreate
    fn force_regenerate_visuals(&mut self) {
         godot_print!("ChunkController: Forcing visual regeneration...");
         // Clear queues to avoid processing outdated requests
         self.mesh_creation_queue.clear();
         self.mesh_removal_queue.clear();

         // Remove existing meshes immediately
         for (_, mut mesh_instance) in self.chunk_meshes.drain() {
             if mesh_instance.is_instance_valid() {
                 mesh_instance.queue_free();
             }
         }
         // Mark for update so update_visualization runs next frame
         self.needs_update = true;
    }

    #[func]
    pub fn get_player_chunk_coords(&self) -> Vector2i {
        Vector2i::new(
            (self.player_position.x / self.chunk_size as f32).floor() as i32,
            (self.player_position.z / self.chunk_size as f32).floor() as i32,
        )
    }

    /// Creates or updates the visual MeshInstance3D using pre-calculated geometry data.
    /// Also applies debug visualization coloring based on self.debug_mode.
    /// This function performs only Godot API calls and MUST run on the main thread.
    fn apply_mesh_data_to_instance(&mut self, pos: ChunkPosition, geometry: &MeshGeometry) {
        // --- Basic Geometry Validation ---
        if geometry.vertices.is_empty() {
            godot_warn!("Attempted to apply mesh geometry with empty vertices for chunk {:?}", pos);
            if let Some(mut mesh_instance) = self.chunk_meshes.remove(&pos) {
                if mesh_instance.is_instance_valid() { mesh_instance.queue_free(); }
            }
            return;
        }
        // Indices might be empty for non-triangle meshes, allow but check other arrays
        if geometry.vertices.len() != geometry.normals.len() ||
           geometry.vertices.len() != geometry.uvs.len() ||
           geometry.vertices.len() != geometry.colors.len() { // Check against colors now
             godot_error!("Mismatched array lengths in MeshGeometry for chunk {:?}! V: {}, N: {}, UV: {}, C: {}. Aborting mesh creation.",
                          pos, geometry.vertices.len(), geometry.normals.len(), geometry.uvs.len(), geometry.colors.len());
            if let Some(mut mesh_instance) = self.chunk_meshes.remove(&pos) {
                if mesh_instance.is_instance_valid() { mesh_instance.queue_free(); }
            }
            return;
        }

        // --- Convert Rust Vecs to Godot Packed Arrays ---
        // Note: geometry.colors is assumed to be Vec<[f32; 4]> from generate_mesh_geometry
        let vertices_gd: Vec<Vector3> = geometry.vertices.iter().map(|v| Vector3::new(v[0], v[1], v[2])).collect();
        let normals_gd: Vec<Vector3> = geometry.normals.iter().map(|n| Vector3::new(n[0], n[1], n[2])).collect();
        let uvs_gd: Vec<Vector2> = geometry.uvs.iter().map(|u| Vector2::new(u[0], u[1])).collect();
        let indices_gd: Vec<i32> = geometry.indices.iter().map(|&i| i).collect();

        let vertices = PackedVector3Array::from(&vertices_gd[..]);
        let normals = PackedVector3Array::from(&normals_gd[..]);
        let uvs = PackedVector2Array::from(&uvs_gd[..]);
        let indices = PackedInt32Array::from(&indices_gd[..]);

        // --- Calculate Vertex Colors Based on Debug Mode ---
        // We'll create a new Vec<Color> based on the mode, using data from geometry.vertices and geometry.colors
        let mut final_colors_gd: Vec<Color> = Vec::with_capacity(geometry.vertices.len());
        for i in 0..geometry.vertices.len() {
            // Safely get required data for this vertex
            let height = geometry.vertices.get(i).map_or(0.0, |v| v[1]);
            // Original color data (contains biome_id in alpha if generated correctly)
            let original_color_data = geometry.colors.get(i).copied().unwrap_or([1.0, 1.0, 1.0, 0.0]);
            // Decode biome_id from original alpha for convenience in BIOME_ID mode
            let biome_id = (original_color_data[3] * 255.0).round() as u8;

            let debug_or_final_color = match self.debug_mode {
                // Mode 1: Heightmap Visualization
                DEBUG_MODE_HEIGHT => {
                    let min_h = -50.0_f32; // Specify f32
                    let max_h = 150.0_f32; // Specify f32
                    let range = (max_h - min_h).max(1.0_f32); // Use f32 literal and max
                    let t = ((height - min_h) / range).clamp(0.0, 1.0);

                    // Gradient using lerp - Fix arguments
                    let blue = Color::from_hsv(0.66, 1.0, 1.0);
                    let green = Color::from_hsv(0.33, 1.0, 1.0);
                    let red = Color::from_hsv(0.0, 1.0, 1.0);

                    if t < 0.5 {
                        // Lerp requires owned Color for 'to' and f64 for weight
                        blue.lerp(green, (t * 2.0) as f64)
                    } else {
                        green.lerp(red, ((t - 0.5) * 2.0) as f64)
                    }
                }
                // Mode 2: Biome ID Visualization
                DEBUG_MODE_BIOME_ID => {
                     // Use the decoded biome_id
                     match biome_id {
                         1 => Color::from_rgb(1.0, 0.2, 0.2), // Reddish (Coral?)
                         2 => Color::from_rgb(0.9, 0.9, 0.2), // Yellowish (Sand?)
                         3 => Color::from_rgb(0.5, 0.5, 0.5), // Gray (Rock?)
                         4 => Color::from_rgb(0.2, 0.8, 0.2), // Greenish (Kelp?)
                         5 => Color::from_rgb(0.8, 0.4, 0.1), // Orange (Lavarock?)
                         _ => Color::from_rgb(0.2, 0.2, 0.2), // Dark Gray for others/unknown (0)
                     }
                }
                // Mode 0 or default: Normal Rendering Preparation
                DEBUG_MODE_NORMAL | _ => {
                    // Use the color data directly from MeshGeometry, assuming it was
                    // generated correctly (e.g., white RGB, biome_id in alpha)
                    // by generate_mesh_geometry
                    Color::from_rgba(
                        original_color_data[0],
                        original_color_data[1],
                        original_color_data[2],
                        original_color_data[3]
                    )
                }
            };
            final_colors_gd.push(debug_or_final_color);
        }
        let colors = PackedColorArray::from(&final_colors_gd[..]); // Final colors to use
        // --- End Color Calculation ---


        // --- Create Godot ArrayMesh Resource ---
        let mut array_mesh = ArrayMesh::new_gd();
        let mut arrays = VariantArray::new();
        arrays.resize(ArrayType::MAX.ord() as usize, &Variant::nil());

        arrays.set(ArrayType::VERTEX.ord() as usize, &vertices.to_variant());
        arrays.set(ArrayType::NORMAL.ord() as usize, &normals.to_variant());
        arrays.set(ArrayType::TEX_UV.ord() as usize, &uvs.to_variant());
        arrays.set(ArrayType::COLOR.ord() as usize, &colors.to_variant()); // Use the FINAL calculated colors

        if !geometry.indices.is_empty() {
            arrays.set(ArrayType::INDEX.ord() as usize, &indices.to_variant());
        }

        array_mesh.add_surface_from_arrays(
            PrimitiveType::TRIANGLES,
            &arrays,
        );

        let mesh_resource: Gd<Mesh> = array_mesh.upcast();

        // --- Create or Update MeshInstance3D Node ---
        let chunk_world_pos = Vector3::new(
            pos.x as f32 * self.chunk_size as f32,
            0.0,
            pos.z as f32 * self.chunk_size as f32,
        );

        let is_debug_render = self.debug_mode > DEBUG_MODE_NORMAL;

        // Closure to handle setting shader parameter and material
        let apply_material_and_shader_param = |mesh_instance: &mut Gd<MeshInstance3D>, base_material: &Option<Gd<Material>>, is_debug: bool| {
            if let Some(base_mat_gd) = base_material {
                let mut material_to_set: Option<Gd<Material>> = None; // Use Option for clarity

                // Try to cast the base material to ShaderMaterial
                if let Ok(base_shader_mat) = base_mat_gd.clone().try_cast::<ShaderMaterial>() {
                    // Duplicate the material to avoid modifying the shared resource
                    // This ensures shader parameters are unique per instance
                    if let Some(duplicated_res) = base_shader_mat.duplicate() {
                        if let Ok(mut unique_shader_mat) = duplicated_res.try_cast::<ShaderMaterial>() {
                            unique_shader_mat.set_shader_parameter("u_debug_mode", &is_debug.to_variant());
                            material_to_set = Some(unique_shader_mat.upcast::<Material>());
                        } else {
                            godot_warn!("Failed to cast duplicated material to ShaderMaterial. Applying base material without unique debug param.");
                            // Fallback: Apply base material, debug param might be wrong if base is shared
                            material_to_set = Some(base_mat_gd.clone());
                        }
                    } else {
                        godot_warn!("Failed to duplicate ShaderMaterial. Applying base material without unique debug param.");
                        // Fallback: Apply base material
                        material_to_set = Some(base_mat_gd.clone());
                    }
                } else {
                    // The base material is not a ShaderMaterial, just apply it directly
                    godot_warn!("Assigned biome_material is not a ShaderMaterial. Cannot set debug parameter. Applying base material.");
                    material_to_set = Some(base_mat_gd.clone());
                }

                // Apply the determined material (unique or base)
                if let Some(mat) = material_to_set {
                    mesh_instance.set_surface_override_material(0, &mat);
                } else {
                    // Should not happen if base_material was Some, but clear just in case
                    mesh_instance.set_surface_override_material(0, Gd::<Material>::null_arg()); // Use null Gd
                }

            } else {
                 // No base material assigned, clear any existing override
                 mesh_instance.set_surface_override_material(0, Gd::<Material>::null_arg()); // Use null Gd
            }
        };


        // --- Apply to existing or create new instance ---
        let instance_exists = self.chunk_meshes.contains_key(&pos);

        if instance_exists {
             if let Some(mut mesh_instance) = self.chunk_meshes.get_mut(&pos) {
                 if mesh_instance.is_instance_valid() {
                     mesh_instance.set_mesh(&mesh_resource);
                     mesh_instance.set_position(chunk_world_pos);
                     apply_material_and_shader_param(&mut mesh_instance, &self.biome_material, is_debug_render);
                 } else {
                     godot_error!("MeshInstance for chunk {:?} was invalid. Removing entry.", pos);
                     self.chunk_meshes.remove(&pos);
                     // Let creation logic handle it below if needed (or alternatively, recreate here)
                 }
             }
        }

        // If it wasn't in the map OR the entry was removed because it was invalid
        if !self.chunk_meshes.contains_key(&pos) {
             let mut mesh_instance = MeshInstance3D::new_alloc();
             mesh_instance.set_mesh(&mesh_resource);
             mesh_instance.set_position(chunk_world_pos);
             let mesh_name: GString = format!("ChunkMesh_{}_{}", pos.x, pos.z).into();
             mesh_instance.set_name(&mesh_name);

             apply_material_and_shader_param(&mut mesh_instance, &self.biome_material, is_debug_render);

             // Add to scene tree and internal tracking map
             // Use '&' for add_child as it expects a reference-like argument
             self.base_mut().add_child(&mesh_instance.clone().upcast::<Node>());
             self.chunk_meshes.insert(pos, mesh_instance);
        }
    }

    fn process_mesh_queues(&mut self) {
        // Process removals first (generally less costly)
        for _ in 0..self.mesh_updates_per_frame {
            if let Some(pos) = self.mesh_removal_queue.pop_front() {
                // Ensure it wasn't added back to visible set or creation queue since enqueued
                // (More robust check might be needed if rapid back-and-forth is possible)
                if !self.mesh_creation_queue.contains(&pos) { // Basic check
                if let Some(mut mesh_instance) = self.chunk_meshes.remove(&pos) {
                    if mesh_instance.is_instance_valid() {
                        // godot_print!("ChunkController ProcessQueue: Removing mesh for {:?}", pos); // Debug log
                        mesh_instance.queue_free();
                    }
                }
                } else {
                    // It was re-queued for creation, so don't remove
                    // godot_print!("ChunkController ProcessQueue: Skipping removal for {:?}, re-queued for creation.", pos); // Debug log
                }

            } else {
                break; // Queue empty
            }
        }

        // Process creations
        let mut processed_creations = 0; // Keep track of how many we actually process
        while processed_creations < self.mesh_updates_per_frame {
            // Get the position first, we need it even if we skip
            if let Some(pos) = self.mesh_creation_queue.front().cloned() { // Clone position to check
                // --- Get ChunkManager Gd and Bind *inside* loop iteration ---
                let needs_processing: bool = if let Some(manager_gd) = &self.chunk_manager {
                    let chunk_manager_bind = manager_gd.bind(); // Borrow manager shortly
                    chunk_manager_bind.is_chunk_ready(pos.x, pos.z)
                        && !self.chunk_meshes.contains_key(&pos) // Check self immutably
                        && !self.mesh_removal_queue.contains(&pos) // Check self immutably
                    // chunk_manager_bind borrow ends here
                } else {
                    false // No manager, cannot process
                };

                if needs_processing {
                    // Remove from queue *before* potential mutable borrow
                    self.mesh_creation_queue.pop_front();

                    // --- Get data (requires binding again) ---
                    let chunk_data_option = if let Some(manager_gd) = &self.chunk_manager {
                        manager_gd.bind().get_cached_chunk_data(pos.x, pos.z)
                    } else {
                        None
                    };

                    if let Some(chunk_data) = chunk_data_option {
                        let geometry = generate_mesh_geometry(
                            &chunk_data.heightmap, 
                            self.chunk_size,
                            &chunk_data.biome_ids, 
                        );
                        if !geometry.vertices.is_empty() {
                            // Now we can call the function requiring &mut self
                            self.apply_mesh_data_to_instance(pos, &geometry);
                            processed_creations += 1;
                        } else {
                            godot_warn!("ChunkController ProcessQueue: Generated empty mesh for {:?}, skipping.", pos);
                            processed_creations += 1; // Still count as processed
                        }
                    } else {
                        godot_warn!("ChunkController ProcessQueue: Failed to get cached data for Ready chunk {:?}. Discarding.", pos);
                        processed_creations += 1; // Still count as processed
                    }
                } else {
                     // Condition not met (not ready, mesh exists, removing, no manager)
                     // Remove from queue to avoid infinite loop if condition persists
                     self.mesh_creation_queue.pop_front();
                     // godot_print!("ChunkController ProcessQueue: Skipping creation for {:?}, condition no longer met.", pos);
                     // Don't increment processed_creations, allow loop to try next if budget allows
                     continue; // Check next item without decrementing budget implicitly
                }
            } else {
                break; // Queue empty
            }
        } // end while
    } // end process_mesh_queues    
}


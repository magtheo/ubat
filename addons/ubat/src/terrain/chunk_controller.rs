use chrono::format;
use godot::prelude::*;
use godot::global::Error;

use godot::classes::{
    // REMOVE this or keep commented out: mesh::PrimitiveType,
    // rendering_seMesrver::PrimitiveType, // TRY THIS PATH for the enum
    // REMOVE mesh::ArrayFormat, // Don't import the enum itself
    ArrayMesh, MeshInstance3D, SurfaceTool, Material, ShaderMaterial, RenderingServer, ResourceLoader, Mesh, World3D, Node
};
use godot::classes::mesh::{PrimitiveType, ArrayFormat, ArrayCustomFormat, ArrayType};

use godot::builtin::PackedColorArray;
use std::convert::TryInto;

use std::collections::{HashMap, HashSet};

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
                if mesh_instance.is_instance_valid() {
                    mesh_instance.queue_free();
                }
            }
            return;
        }

        // --- 1. Get or Create MeshInstance3D ---
        let mut is_new_instance = false;
        let mesh_instance_entry = self.chunk_meshes.entry(pos);
        let mut mesh_instance = mesh_instance_entry.or_insert_with(|| {
            godot_print!("ChunkController: Creating new MeshInstance3D for {:?}", pos);
            is_new_instance = true;
            let mut inst = MeshInstance3D::new_alloc();
            let name = GString::from(format!("Chunk_{}_{}", pos.x, pos.z));
            inst.set_name(&name);
            inst
        }).clone();

        // --- 2. Create the ArrayMesh ---
        let mut mesh = ArrayMesh::new_gd();

        // --- 3. Prepare PackedArrays from geometry data ---
        // Standard vertex attributes
        let mut vertices = PackedVector3Array::new();
        let mut normals = PackedVector3Array::new();
        let mut uvs = PackedVector2Array::new();
        let mut indices = PackedInt32Array::new();
        let mut colors = PackedColorArray::new(); // For debug visualization

        // Custom vertex attributes as PackedByteArrays
        let mut custom0_bytes_vec: Vec<u8> = Vec::new(); // For biome IDs
        let mut custom1_bytes_vec: Vec<u8> = Vec::new(); // For biome weights

        let vertex_count = geometry.vertices.len();

        // --- Reserve Capacity (Now on Vec<u8>, which works) ---
        custom0_bytes_vec.reserve(vertex_count * 4);
        custom1_bytes_vec.reserve(vertex_count * 16);

        // --- Populate PackedArrays (Main Loop) ---
        for i in 0..vertex_count {
            // Populate standard attributes
            vertices.push(Vector3::new(geometry.vertices[i][0], geometry.vertices[i][1], geometry.vertices[i][2]));
            normals.push(Vector3::new(geometry.normals[i][0], geometry.normals[i][1], geometry.normals[i][2]));
            uvs.push(Vector2::new(geometry.uvs[i][0], geometry.uvs[i][1]));

            // --- Custom Attributes (Packed into Bytes) ---
            let biome_ids = geometry.custom0_biome_ids[i]; // [u8; 3]
            let biome_weights = geometry.custom1_biome_weights[i]; // [f32; 3]

            // Pack CUSTOM0 (IDs) as bytes
            custom0_bytes_vec.push(biome_ids[0]);
            custom0_bytes_vec.push(biome_ids[1]);
            custom0_bytes_vec.push(biome_ids[2]);
            custom0_bytes_vec.push(0);
            // Note: If your shader or Godot expects a 4-byte alignment (like RGBA8),
            // you might need to add padding: custom0_bytes_vec.push(0);

            // Pack CUSTOM1 (Weights) as bytes (using little-endian byte order)
            custom1_bytes_vec.extend_from_slice(&biome_weights[0].to_le_bytes());
            custom1_bytes_vec.extend_from_slice(&biome_weights[1].to_le_bytes());
            custom1_bytes_vec.extend_from_slice(&biome_weights[2].to_le_bytes());
            custom1_bytes_vec.extend_from_slice(&0.0f32.to_le_bytes()); // <-- PADDING to make 4 floats

            // --- Calculate Vertex Colors Based on Debug Mode ---
            let height = geometry.vertices[i][1]; // Use the Y component of the vertex
            let vertex_color = match self.debug_mode {
                DEBUG_MODE_HEIGHT => {
                    // Logic for height visualization (Blue -> Green -> Red gradient)
                    let min_h = -50.0f32; // Example range, adjust as needed
                    let max_h = 150.0f32; // Example range, adjust as needed
                    let range = (max_h - min_h).max(1.0f32); // Avoid division by zero
                    let t = ((height - min_h) / range).clamp(0.0f32, 1.0f32);
                    let blue = Color::BLUE;
                    let green = Color::GREEN;
                    let red = Color::RED;
                    if (t as f64) < 0.5 {
                        blue.lerp(green, t as f64 * 2.0)
                    } else {
                        green.lerp(red, (t as f64 - 0.5) * 2.0)
                    }
                }
                DEBUG_MODE_BIOME_ID => {
                    // Visualize primary biome ID using the original geometry data
                    let primary_id = geometry.custom0_biome_ids[i][0];
                    match primary_id as u8 % 6 { // Simple color cycle based on ID
                        0 => Color::RED,
                        1 => Color::GREEN,
                        2 => Color::BLUE,
                        3 => Color::YELLOW,
                        4 => Color::CYAN,
                        _ => Color::MAGENTA,
                    }
                }
                _ => { // DEBUG_MODE_NORMAL (0) and any other values
                    Color::WHITE // Default color
                }
            }; // End match
            colors.push(vertex_color);
        } // End vertex loop

        // Populate indices array
        for &idx in &geometry.indices {
            indices.push(idx);
        }

        // --- 4. Create the main Godot Array to hold all vertex data ---
        let mut arrays = VariantArray::new();
        // Resize the VariantArray to the exact size Godot expects (Mesh::ARRAY_MAX)
        arrays.resize(ArrayType::MAX.ord() as usize, &Variant::nil());

        
        // --- 5. Place the packed arrays into the main VariantArray at correct indices ---
        arrays.set(ArrayType::VERTEX.ord() as usize, &vertices.to_variant());
        arrays.set(ArrayType::NORMAL.ord() as usize, &normals.to_variant());
        arrays.set(ArrayType::TEX_UV.ord() as usize, &uvs.to_variant());
        arrays.set(ArrayType::COLOR.ord() as usize, &colors.to_variant());
        if !geometry.indices.is_empty() {
            arrays.set(ArrayType::INDEX.ord() as usize, &indices.to_variant());
        }

        // Convert Vec<u8> to PackedByteArray before setting in VariantArray
        // Use `from_iter` or `from_slice` - check gdext docs for PackedByteArray constructors
        // Assuming `from_slice` exists and is efficient:
        // Convert Vec<u8> to PackedByteArray using FromIterator
        let packed_custom0_bytes: PackedByteArray = custom0_bytes_vec.into_iter().collect();
        let packed_custom1_bytes: PackedByteArray = custom1_bytes_vec.into_iter().collect();
        // If `from_slice` doesn't exist, `from_iter(custom0_bytes_vec)` might work.

        godot_print!(
            "DEBUG: vertices: {}, custom0_bytes: {}, custom1_bytes: {}",
            vertices.len(),
            packed_custom0_bytes.len(),
            packed_custom1_bytes.len()
        );
        

        arrays.set(ArrayType::CUSTOM0.ord() as usize, &packed_custom0_bytes.to_variant());
        arrays.set(ArrayType::CUSTOM1.ord() as usize, &packed_custom1_bytes.to_variant());
        
        // --- 6. Add the surface to the mesh ---
        // Use the 2-argument version, passing the correctly sized VariantArray
        mesh.add_surface_from_arrays(
            PrimitiveType::TRIANGLES,
            &arrays
            // Format flags, blend shapes, LODs are not passed here in Godot 4+ bindings typically
        );

        // --- 7. Set Mesh on Instance ---
        let mesh_resource: Gd<Mesh> = mesh.upcast();
        mesh_instance.set_mesh(&mesh_resource);

        // --- 8. Apply Material ---
        // This helper function likely needs no changes, but ensure it handles surface 0 correctly.
        let is_debug_render = self.debug_mode > DEBUG_MODE_NORMAL;
        Self::apply_material_and_shader_param(
            &mut mesh_instance,
            &self.biome_material,
            is_debug_render,
        );

        // --- 9. Add to Scene Tree ---
        if is_new_instance {
            // Ensure `self` has access to `base_mut()` or equivalent to add child
            self.base_mut()
                .add_child(&mesh_instance.clone().upcast::<Node>());
        }

        // --- 10. Set Position ---
        let world_pos = Vector3::new(
            pos.x as f32 * self.chunk_size as f32,
            0.0, // Set Y position as needed (e.g., based on min height, or keep at 0)
            pos.z as f32 * self.chunk_size as f32,
        );
        mesh_instance.set_global_position(world_pos);

        // --- 11. Ensure Visibility ---
        if !mesh_instance.is_visible() {
            mesh_instance.show();
        }
    }

    /// Helper function to apply material and potentially set shader parameters.
    /// Adapted from the older provided code for robustness. Moved inside impl block.
    fn apply_material_and_shader_param(
        mesh_instance: &mut Gd<MeshInstance3D>,
        base_material: &Option<Gd<Material>>,
        is_debug: bool,
    ) {
        let material_to_set: Option<Gd<Material>>;
        if let Some(base_mat_gd) = base_material {
            if let Ok(base_shader_mat) = base_mat_gd.clone().try_cast::<ShaderMaterial>() {
                if let Some(duplicated_res) = base_shader_mat.duplicate() {
                    if let Ok(mut unique_shader_mat) =
                        duplicated_res.try_cast::<ShaderMaterial>()
                    {
                        unique_shader_mat
                            .set_shader_parameter("u_debug_mode", &is_debug.to_variant());
                        material_to_set = Some(unique_shader_mat.upcast::<Material>());
                    } else {
                        godot_warn!(
                            "Failed to cast duplicated material to ShaderMaterial. Using base."
                        );
                        material_to_set = Some(base_mat_gd.clone());
                    }
                } else {
                    godot_warn!("Failed to duplicate ShaderMaterial. Using base.");
                    material_to_set = Some(base_mat_gd.clone());
                }
            } else {
                material_to_set = Some(base_mat_gd.clone());
            }
        } else {
            material_to_set = None;
        }
        mesh_instance.set_surface_override_material(0, material_to_set.as_ref());
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
                    self.mesh_creation_queue.pop_front();
    
                    let chunk_data_option = if let Some(manager_gd) = &self.chunk_manager {
                        // *** NOTE: This line has the ERROR in your log ***
                        // Original erroneous line might be here or where get_cached_chunk_data is called
                        // We fix the generate_mesh_geometry call below
                        manager_gd.bind().get_cached_chunk_data(pos.x, pos.z)
                    } else {
                        None
                    };
    
                    if let Some(chunk_data) = chunk_data_option {
                        // --- FIX: Pass new fields to generate_mesh_geometry ---
                        // Ensure generate_mesh_geometry function signature is updated too!
                        let geometry = generate_mesh_geometry(
                            &chunk_data.heightmap,
                            self.chunk_size, // Assuming chunk_size is available here
                            &chunk_data.biome_indices,     // Pass indices
                            &chunk_data.biome_blend_weights // Pass weights
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


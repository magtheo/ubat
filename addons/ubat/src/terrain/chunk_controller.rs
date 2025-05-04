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

use godot::classes::rendering_server::ArrayFormat as RSArrayFormat;

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

const ARRAY_CUSTOM_FORMAT_RGBA8_UNORM: i64 = 1; // WAS: ArrayCustomFormat::RGBA8_UNORM.ord() as i64;
const ARRAY_CUSTOM_FORMAT_RGBA32F: i64 = 6;   // WAS: ArrayCustomFormat::RGBA_FLOAT.ord() as i64;

const ARRAY_FORMAT_CUSTOM_BASE_SHIFT: i64 = 16;
const ARRAY_FORMAT_CUSTOM_BITS: i64 = 3;

const ARRAY_FORMAT_CUSTOM0_SHIFT: i64 = ARRAY_FORMAT_CUSTOM_BASE_SHIFT + ARRAY_FORMAT_CUSTOM_BITS * 0; // 16
const ARRAY_FORMAT_CUSTOM1_SHIFT: i64 = ARRAY_FORMAT_CUSTOM_BASE_SHIFT + ARRAY_FORMAT_CUSTOM_BITS * 1; // 19

// const ARRAY_CUSTOM_FORMAT_RGBA8_UNORM: i64 = 0; // Placeholder - Find actual value
// const ARRAY_CUSTOM_FORMAT_RGBA32F: i64 = 7; // Placeholder - Find actual value
// const ARRAY_FORMAT_CUSTOM0_SHIFT: i64 = 13;   // Placeholder - Find actual value (RS::ARRAY_FORMAT_CUSTOM_BASE_SHIFT + 0)
// const ARRAY_FORMAT_CUSTOM1_SHIFT: i64 = 16;   // Placeholder - Find actual value (RS::ARRAY_FORMAT_CUSTOM_BASE_SHIFT + 1)



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

    /// Creates or updates the visual MeshInstance3D by directly creating PackedArrays
    /// and using `mesh.add_surface_from_arrays`.
    /// NOTE: This bypasses SurfaceTool and requires the shader to manually unpack
    /// byte data sent via CUSTOM0 and CUSTOM1 vertex attributes.
    /// This function performs only Godot API calls and MUST run on the main thread.
    fn apply_mesh_data_to_instance(&mut self, pos: ChunkPosition, geometry: &MeshGeometry) {
        // --- Basic Geometry Validation ---
        if geometry.vertices.is_empty() || geometry.indices.is_empty() {
            godot_warn!("Apply Mesh: Empty vertices or indices for chunk {:?}, skipping.", pos);
            if let Some(mut mesh_instance) = self.chunk_meshes.remove(&pos) {
                if mesh_instance.is_instance_valid() { mesh_instance.queue_free(); }
            }
            return;
        }

        let vertex_count = geometry.vertices.len();
        let index_count = geometry.indices.len();

        // --- Extended Validation ---
        if geometry.normals.len() != vertex_count ||
            geometry.uvs.len() != vertex_count ||
            geometry.custom0_biome_ids.len() != vertex_count || // Expects [u8; 4] per vertex
            geometry.custom1_biome_weights.len() != vertex_count // Expects [f32; 3] or similar per vertex
        {
            godot_error!(
                "Apply Mesh: Attribute length mismatch for chunk {:?}. Vertices: {}, Indices: {}, Normals: {}, UVs: {}, Custom0: {}, Custom1: {}",
                pos, vertex_count, index_count, geometry.normals.len(), geometry.uvs.len(), geometry.custom0_biome_ids.len(), geometry.custom1_biome_weights.len()
            );
            if let Some(mut mesh_instance) = self.chunk_meshes.remove(&pos) {
                if mesh_instance.is_instance_valid() { mesh_instance.queue_free(); }
            }
            return;
        }
        // --- End Validation ---

        // --- 1. Get or Create MeshInstance3D ---
        // (Keep your existing logic for getting/creating MeshInstance3D)
        let mut is_new_instance = false;
        let mesh_instance_entry = self.chunk_meshes.entry(pos);
        let mut mesh_instance = mesh_instance_entry.or_insert_with(|| {
            is_new_instance = true;
            let mut inst = MeshInstance3D::new_alloc();
            inst.set_name(&GString::from(format!("ChunkMesh_{},{}", pos.x, pos.z)));
            inst
        }).clone();

        // --- 2. Prepare Interleaved Vertex Data Buffer ---
        let mut vertex_byte_vec: Vec<u8> = Vec::new(); // Capacity calculation is complex, let it grow
        let mut min_point = Vector3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
        let mut max_point = Vector3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);

        for i in 0..vertex_count {
            let start_len = vertex_byte_vec.len();

            // Position (Vector3 = 12 bytes)
            let v = Vector3::new(geometry.vertices[i][0], geometry.vertices[i][1], geometry.vertices[i][2]);
            vertex_byte_vec.extend_from_slice(&v.x.to_le_bytes());
            vertex_byte_vec.extend_from_slice(&v.y.to_le_bytes());
            vertex_byte_vec.extend_from_slice(&v.z.to_le_bytes());

            // Update AABB
            min_point.x = min_point.x.min(v.x);
            min_point.y = min_point.y.min(v.y);
            min_point.z = min_point.z.min(v.z);
            max_point.x = max_point.x.max(v.x);
            max_point.y = max_point.y.max(v.y);
            max_point.z = max_point.z.max(v.z);

            // Normal (Vector3 = 12 bytes)
            let n = Vector3::new(geometry.normals[i][0], geometry.normals[i][1], geometry.normals[i][2]);
            vertex_byte_vec.extend_from_slice(&n.x.to_le_bytes());
            vertex_byte_vec.extend_from_slice(&n.y.to_le_bytes());
            vertex_byte_vec.extend_from_slice(&n.z.to_le_bytes());

            // Color (Color = 4x f32 = 16 bytes)
            let height = geometry.vertices[i][1];
            let vertex_color = match self.debug_mode {
                DEBUG_MODE_HEIGHT => {
                    let normalized_height = (height / 100.0).clamp(0.0, 1.0); // Use f32
                    Color::from_hsv(
                        (0.6 - (normalized_height * 0.6)) as f64,
                        0.8,
                        (0.5 + (normalized_height * 0.5)) as f64,
                    )
                },
                DEBUG_MODE_BIOME_ID => {
                    let biome_id = geometry.custom0_biome_ids[i][0] as f32; // Assuming first byte is main ID
                    let hue = (biome_id / 20.0) % 1.0;
                    Color::from_hsv(hue as f64, 0.8, 0.8)
                },
                _ => Color::WHITE
            };
            vertex_byte_vec.extend_from_slice(&vertex_color.r.to_le_bytes());
            vertex_byte_vec.extend_from_slice(&vertex_color.g.to_le_bytes());
            vertex_byte_vec.extend_from_slice(&vertex_color.b.to_le_bytes());
            vertex_byte_vec.extend_from_slice(&vertex_color.a.to_le_bytes());

            // UV (Vector2 = 8 bytes)
            let uv = Vector2::new(geometry.uvs[i][0], geometry.uvs[i][1]);
            vertex_byte_vec.extend_from_slice(&uv.x.to_le_bytes());
            vertex_byte_vec.extend_from_slice(&uv.y.to_le_bytes());

            // Custom0 (Assuming [u8; 4] matching ARRAY_CUSTOM_FORMAT_RGBA8_UNORM = 4 bytes)
            vertex_byte_vec.extend_from_slice(&geometry.custom0_biome_ids[i]);

            // Custom1 (Assuming [f32; 3] + padding, matching ARRAY_CUSTOM_FORMAT_RGBA32F = 16 bytes)
            let biome_weights = geometry.custom1_biome_weights[i];
            vertex_byte_vec.extend_from_slice(&biome_weights[0].to_le_bytes()); // Weight 0
            vertex_byte_vec.extend_from_slice(&biome_weights[1].to_le_bytes()); // Weight 1
            vertex_byte_vec.extend_from_slice(&biome_weights[2].to_le_bytes()); // Weight 2
            vertex_byte_vec.extend_from_slice(&0.0f32.to_le_bytes());          // Padding (4th float)
            
            let end_len = vertex_byte_vec.len();
            if end_len - start_len != 68 {
                godot_error!("Stride mismatch at index {}: Added {} bytes, expected 68", i, end_len - start_len);
                // Optional: Print sizes of individual components added in this iteration
                // godot_print!("  custom0 size: {}", geometry.custom0_biome_ids[i].len());
            }
        }
        
        // Handle case with no vertices for AABB
        if vertex_count == 0 {
            min_point = Vector3::ZERO;
            max_point = Vector3::ZERO;
        }
        let surface_aabb = Aabb::from_corners(min_point, max_point);
        
        let vertex_byte_array: PackedByteArray = vertex_byte_vec.into();

        if vertex_byte_array.len() != vertex_count as usize * 68 {
            godot_error!("Final buffer size incorrect! Size: {}, Expected: {}", vertex_byte_array.len(), vertex_count as usize * 68);
        }       

        // --- 3. Prepare Index Buffer ---
        let mut index_byte_vec: Vec<u8> = Vec::with_capacity(index_count * 4); // Indices are usually u32
        for &idx in &geometry.indices {
            // Godot typically uses 32-bit indices
            index_byte_vec.extend_from_slice(&(idx as u32).to_le_bytes());
        }
        let index_byte_array: PackedByteArray = index_byte_vec.into();

        // --- 4. Define Correct Vertex Format Bitmask ---
        godot_print!("Vertex Ord: {}", RSArrayFormat::VERTEX.ord());
        godot_print!("Normal Ord: {}", RSArrayFormat::NORMAL.ord());
        godot_print!("Color Ord: {}", RSArrayFormat::COLOR.ord());
        godot_print!("TexUV Ord: {}", RSArrayFormat::TEX_UV.ord());
        godot_print!("Custom0 Ord: {}", RSArrayFormat::CUSTOM0.ord());
        godot_print!("Custom1 Ord: {}", RSArrayFormat::CUSTOM1.ord());
        // Print any other ordinals you use

        godot_print!("Custom Format RGBA8_UNORM: {}", ARRAY_CUSTOM_FORMAT_RGBA8_UNORM);
        godot_print!("Custom Shift 0: {}", ARRAY_FORMAT_CUSTOM0_SHIFT);
        godot_print!("Custom Format RGBA32F: {}", ARRAY_CUSTOM_FORMAT_RGBA32F);
        godot_print!("Custom Shift 1: {}", ARRAY_FORMAT_CUSTOM1_SHIFT);
        godot_print!("--- End Debugging Shift Values ---");
        
        let mut format: i64 = 0;
        // Use bit shifts (1 << enum_value) - Make sure RSArrayFormat enum values are correct (0, 1, 2, ...)
        // format |= RSArrayFormat::VERTEX.ord() as i64;
        // format |= RSArrayFormat::NORMAL.ord() as i64;
        // // format |= RSArrayFormat::TANGENT.ord() as i64; // If you add tangents
        // format |= RSArrayFormat::COLOR.ord() as i64;
        // format |= RSArrayFormat::TEX_UV.ord() as i64;
        // // format |= RSArrayFormat::TEX_UV2.ord() as i64; // If you add second UV
        // format |= RSArrayFormat::CUSTOM0.ord() as i64; // Use the value (64) directly
        // format |= RSArrayFormat::CUSTOM1.ord() as i64; // Use the value (128) directly
        // format |= 1 << RSArrayFormat::BONES.ord();   // Bit 10 (If using skeletal anim)
        // format |= 1 << RSArrayFormat::WEIGHTS.ord(); // Bit 11 (If using skeletal anim)
        // --- DO NOT ADD INDEX FLAG HERE --- format |= 1 << RSArrayFormat::INDEX.ord(); // Bit 12

        // --- "CORRECT" new ---
        format |=  RSArrayFormat::VERTEX.ord() as i64;    // Bit 0 -> Flag 1
        format |= RSArrayFormat::NORMAL.ord() as i64;    // Bit 1 -> Flag 2
        format |= RSArrayFormat::COLOR.ord() as i64;     // Bit 3 -> Flag 8
        format |= RSArrayFormat::TEX_UV.ord() as i64;    // Bit 4 -> Flag 16
        format |= RSArrayFormat::CUSTOM0.ord() as i64;   // Bit 6 -> Flag 64
        format |= RSArrayFormat::CUSTOM1.ord() as i64;   // Bit 7 -> Flag 128
        // Add other flags like TANGENT, UV2 if needed using the same pattern

        // --- Custom format specifiers (These shifts look correct) ---
        format |= ARRAY_CUSTOM_FORMAT_RGBA8_UNORM << ARRAY_FORMAT_CUSTOM0_SHIFT;
        format |= ARRAY_CUSTOM_FORMAT_RGBA32F << ARRAY_FORMAT_CUSTOM1_SHIFT;

        godot_print!("Final format mask: {}", format); // Debug print

        // ---> GET THE SIZE **BEFORE** MOVING THE ARRAY <--- used for debugging
        let actual_byte_array_size = vertex_byte_array.len();

        // --- 5. Create Parameter Dictionary ---
        let mut surface_params = Dictionary::new();
        surface_params.insert("primitive", PrimitiveType::TRIANGLES.ord() as i32); // Use i32
        surface_params.insert("format", format as i32); // The calculated bitmask
        surface_params.insert("vertex_data", vertex_byte_array); // Single interleaved buffer
        surface_params.insert("vertex_count", vertex_count as i64);
        surface_params.insert("index_data", index_byte_array);   // Separate index buffer
        surface_params.insert("index_count", index_count as i64);
        surface_params.insert("aabb", surface_aabb);
        // Optional: Add default empty arrays/dictionaries if strictly needed by API
        // surface_params.insert("blend_shapes", VariantArray::new());
        // surface_params.insert("lods", Dictionary::new());
        // surface_params.insert("flags", 0u32); // Use 0 for flags unless specific ones are needed

        // --- 6. Use RenderingServer to Add Surface ---
        let mut rs = RenderingServer::singleton();
        // Ensure mesh_resource is created or retrieved correctly
        // If updating, get existing RID. If new, create ArrayMesh and get RID.
        let mut mesh_resource: Gd<ArrayMesh>;
        if let Some(existing_mesh) = mesh_instance.get_mesh() {
             // Attempt to cast to ArrayMesh, clone if successful
             if let Ok(am) = existing_mesh.try_cast::<ArrayMesh>() {
                mesh_resource = am; // Use existing
                // Clear previous surfaces before adding new one
                rs.mesh_clear(mesh_resource.get_rid());
             } else {
                // If it's not an ArrayMesh or cast fails, create a new one
                godot_warn!("Existing mesh was not an ArrayMesh, creating new one.");
                mesh_resource = ArrayMesh::new_gd();
                mesh_instance.set_mesh(&mesh_resource.clone().upcast::<Mesh>()); // Assign new mesh
                // No need to clear, it's new
             }
        } else {
            // No mesh existed, create a new one
            mesh_resource = ArrayMesh::new_gd();
            mesh_instance.set_mesh(&mesh_resource.clone().upcast::<Mesh>()); // Assign new mesh
            // No need to clear, it's new
        }

        let mesh_rid = mesh_resource.get_rid();

        let actual_vertex_count = geometry.vertices.len();
        let vertex_count_passed = vertex_count as i64; // Assuming vertex_count IS geometry.vertices.len()

        godot_print!("Passing to mesh_add_surface:");
        godot_print!("  vertex_count key: {}", vertex_count_passed);
        godot_print!("  actual geometry.vertices.len(): {}", actual_vertex_count);
        // ---> USE THE PRE-CALCULATED SIZE HERE <---
        godot_print!("  vertex_byte_array size: {}", actual_byte_array_size);
        godot_print!("  Expected byte array size (if stride=68): {}", actual_vertex_count as i64 * 68);
        godot_print!("  Format mask value: {}", surface_params.get("format").unwrap_or_default().try_to::<i64>().unwrap_or(-1));


        // Call mesh_add_surface with RID and Dictionary
        rs.mesh_add_surface(
            mesh_rid,
            &surface_params // Pass the dictionary
        );
        // Check RenderingServer errors in the Godot output/debugger after this call

        // --- 7. Set Material, Position, etc. ---
        // This should now work as surface 0 exists (assuming rs.mesh_add_surface succeeded)
        let is_debug_render = self.debug_mode > DEBUG_MODE_NORMAL;
        Self::apply_material_and_shader_param(&mut mesh_instance, &self.biome_material, is_debug_render);

        if is_new_instance {
            self.base_mut().add_child(&mesh_instance.clone().upcast::<Node>());
        }

        let world_pos = Vector3::new(
            pos.x as f32 * self.chunk_size as f32,
            0.0,
            pos.z as f32 * self.chunk_size as f32,
        );
        mesh_instance.set_position(world_pos);

        if !mesh_instance.is_visible() {
            mesh_instance.show();
        }
        // Mesh is now stored in self.chunk_meshes via the entry API earlier
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
                        let expected_size = ((self.chunk_size + 1) * (self.chunk_size + 1)) as usize;
                        godot_print!(
                            "DEBUG ChunkData Check for {:?}: Expected Size: {}, Heightmap: {}, Biomes: {}, Weights: {}",
                            pos,
                            expected_size,
                            chunk_data.heightmap.len(),
                            chunk_data.biome_indices.len(),
                            chunk_data.biome_blend_weights.len()
                        );

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


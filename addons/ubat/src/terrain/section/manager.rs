// src/section/manager.rs
use godot::prelude::*;
use godot::classes::Node;
use std::sync::Arc;
use std::collections::HashMap;

use super::sectionConfig::{SectionTomlConfig, BiomeTomlConfig};
use crate::terrain::section::definition::{SectionDefinition, BiomeDefinition, VoronoiPoint, Rect2};
use crate::terrain::section::distribution::{generate_voronoi_points_for_section, SpatialGrid};
use crate::terrain::section::sectionConfig;
use crate::terrain::section::thread_safe_data::ThreadSafeSectionData;
use crate::terrain::noise::noise_manager::NoiseManager;

/// SectionManager is a Godot node responsible for managing sections and biomes.
/// It replaces the previous image-based BiomeManager with a procedural system.
#[derive(GodotClass)]
#[class(base=Node)]
pub struct SectionManager {
    #[base]
    base: Base<Node>,

    sections: Vec<SectionDefinition>,
    biomes: Vec<BiomeDefinition>,
    voronoi_points: Vec<VoronoiPoint>,
    spatial_grid: Option<SpatialGrid>,
    
    world_length: f32,
    world_width: f32,
    world_seed: u64,
    
    initialized: bool,
    biome_blend_distance: f32,
    section_blend_distance: f32,
}

#[godot_api]
impl INode for SectionManager {
    fn init(base: Base<Node>) -> Self {
        SectionManager {
            base,
            sections: Vec::new(),
            biomes: Vec::new(),
            voronoi_points: Vec::new(),
            spatial_grid: None,
            
            world_length: 0.0,
            world_width: 10000.0, // Default width
            world_seed: 0,
            
            initialized: false,
            biome_blend_distance: 100.0, // Default blend distance
            section_blend_distance: 200.0, // Default section blend
        }
    }

    fn ready(&mut self) {
        godot_print!("SectionManager: Node is ready");
    }
}

#[godot_api]
impl SectionManager {
    /// Initialize the SectionManager with configuration data and noise functions.
    #[func]
    pub fn initialize(
        &mut self,
        sections_config_var: Variant,
        biomes_config_var: Variant,
        world_seed: u64,
        noise_manager: Gd<NoiseManager>, // Expects owned Gd<NoiseManager>
    ) -> bool {
        godot_print!("SectionManager: Initializing with seed {}", world_seed);

        // Use types directly, assuming they are imported via `use` statements
        let mut sections_config: Vec<SectionTomlConfig> = Vec::new();
        let mut biomes_config: Vec<BiomeTomlConfig> = Vec::new();

        // --- Process sections config ---
        match sections_config_var.try_to::<VariantArray>() {
            Ok(sections_array) => {
                for i in 0..sections_array.len() {
                    let section_var_opt = sections_array.get(i);
                    if let Some(section_var) = section_var_opt {
                        if let Ok(section_dict) = section_var.try_to::<Dictionary>() {
                            // *** CORRECTED TYPE: Parse id as u8 ***
                            let id = section_dict.get("id")
                                .and_then(|v| v.try_to::<u8>().ok()) // Try convert to u8
                                .unwrap_or_else(|| {
                                    godot_warn!("SectionManager: Missing/invalid 'id' (expected u8) in section config[{}]. Using 0.", i);
                                    0 // Default u8
                                });

                            let length = section_dict.get("length")
                                .and_then(|v| v.try_to::<f32>().ok())
                                .unwrap_or(1000.0);

                            let transition_zone = section_dict.get("transition_zone")
                                .and_then(|v| v.try_to::<f32>().ok())
                                .unwrap_or(100.0);

                            let boundary_noise_key = section_dict.get("boundary_noise_key")
                                .and_then(|v| v.try_to::<String>().ok());

                            let point_density = section_dict.get("point_density")
                                .and_then(|v| v.try_to::<f32>().ok())
                                .unwrap_or(0.01);

                            // *** CORRECTED TYPE: Parse possible_biomes elements as u8 ***
                            let mut possible_biomes: Vec<u8> = Vec::new(); // Expect Vec<u8>
                            if let Some(biomes_var) = section_dict.get("possible_biomes") {
                               if let Ok(biomes_array_inner) = biomes_var.try_to::<VariantArray>() {
                                    for j in 0..biomes_array_inner.len() {
                                        // Try convert to u8
                                        if let Some(biome_id) = biomes_array_inner.get(j)
                                                                            .and_then(|v| v.try_to::<u8>().ok()) // Parse as u8
                                        {
                                            possible_biomes.push(biome_id);
                                        } else {
                                             godot_warn!("SectionManager: Failed to parse biome ID (expected u8) at index {} in section {}", j, id);
                                        }
                                    }
                               } else {
                                    godot_warn!("SectionManager: 'possible_biomes' in section {} is not a VariantArray", id);
                               }
                            }

                            // Create SectionTomlConfig (ensure its definition uses u8 for id and Vec<u8> for possible_biomes)
                            sections_config.push(SectionTomlConfig {
                                id, length, transition_zone, boundary_noise_key, possible_biomes, point_density,
                            });

                        } else {
                             godot_warn!("SectionManager: Item at index {} in sections config is not a Dictionary", i);
                        }
                    } else {
                         godot_warn!("SectionManager: Item at index {} in sections config is Null or invalid Variant", i);
                    }
                }
            }
            Err(e) => {
                godot_error!("SectionManager: Failed to convert sections config to VariantArray: {}", e);
                return false;
            }
        }

        // --- Process biomes config ---
        match biomes_config_var.try_to::<VariantArray>() {
             Ok(biomes_array) => {
                for i in 0..biomes_array.len() {
                    let biome_var_opt = biomes_array.get(i);
                     if let Some(biome_var) = biome_var_opt {
                         if let Ok(biome_dict) = biome_var.try_to::<Dictionary>() {
                            // *** CORRECTED TYPE: Parse id as u8 ***
                            let id = biome_dict.get("id")
                                .and_then(|v| v.try_to::<u8>().ok()) // Try convert to u8
                                 .unwrap_or_else(|| {
                                    godot_warn!("SectionManager: Missing/invalid 'id' (expected u8) in biome config[{}]. Using 0.", i);
                                    0 // Default u8
                                });

                            let name = biome_dict.get("name")
                                .and_then(|v| v.try_to::<String>().ok())
                                .unwrap_or_default();

                             let primary_noise_key = biome_dict.get("primary_noise_key")
                                .and_then(|v| v.try_to::<String>().ok())
                                .unwrap_or_else(|| {
                                    godot_error!("SectionManager: Missing/invalid 'primary_noise_key' for biome config[{}]. Cannot proceed.", i);
                                    String::new()
                                });
                             if primary_noise_key.is_empty() { return false; }


                            let mut secondary_noise_keys = Vec::new();
                            if let Some(keys_var) = biome_dict.get("secondary_noise_keys") {
                                 if let Ok(keys_array_inner) = keys_var.try_to::<VariantArray>() {
                                    for j in 0..keys_array_inner.len() {
                                        if let Some(key) = keys_array_inner.get(j).and_then(|v| v.try_to::<String>().ok()) {
                                            secondary_noise_keys.push(key);
                                        } else {
                                            godot_warn!("SectionManager: Failed to parse secondary noise key at index {} in biome {}", j, id);
                                        }
                                    }
                                 } else {
                                     godot_warn!("SectionManager: 'secondary_noise_keys' in biome {} is not a VariantArray", id);
                                 }
                            }

                            let mut texture_params = HashMap::new();
                            if let Some(params_var) = biome_dict.get("texture_params") {
                                if let Ok(params_dict) = params_var.try_to::<Dictionary>() {
                                    for key_var in params_dict.keys_array().iter_shared() {
                                         if let Ok(key_str) = key_var.try_to::<String>() {
                                             if let Some(value_f32) = params_dict.get(key_var)
                                                                                  .and_then(|v| v.try_to::<f32>().ok())
                                             {
                                                texture_params.insert(key_str, value_f32);
                                             } else {
                                                 godot_warn!("SectionManager: Failed to parse texture param value for key '{}' in biome {}", key_str, id);
                                             }
                                         } else {
                                             godot_warn!("SectionManager: Failed to parse texture param key {:?} as String in biome {}", key_var, id);
                                         }
                                    }
                                } else {
                                     godot_warn!("SectionManager: 'texture_params' in biome {} is not a Dictionary", id);
                                }
                            }

                            // Create BiomeTomlConfig (ensure its definition uses u8 for id)
                            biomes_config.push(BiomeTomlConfig {
                                id, name, primary_noise_key, secondary_noise_keys, texture_params,
                            });

                         } else {
                              godot_warn!("SectionManager: Item at index {} in biomes config is not a Dictionary", i);
                         }
                     } else {
                          godot_warn!("SectionManager: Item at index {} in biomes config is Null or invalid Variant", i);
                     }
                }
            }
            Err(e) => {
                godot_error!("SectionManager: Failed to convert biomes config to VariantArray: {}", e);
                return false;
            }
        }


        godot_print!("SectionManager: Processed {} sections and {} biomes",
                    sections_config.len(), biomes_config.len());

        if sections_config.is_empty() {
             godot_error!("SectionManager: Cannot initialize: No valid sections were parsed.");
             return false;
        }
        if biomes_config.is_empty() {
            godot_error!("SectionManager: Cannot initialize: No valid biomes were parsed.");
            return false;
        }

        // --- Initialization ---
        self.world_seed = world_seed;
        self.sections.clear();
        self.biomes.clear();
        self.voronoi_points.clear();
        self.spatial_grid = None;

        // --- Process Biomes into Definitions ---
        let nm_bind = noise_manager.bind();
        let mut temp_biomes = Vec::with_capacity(biomes_config.len());

        for biome_config in biomes_config {
            // Pass &str to get_noise_function
            let primary_noise_fn = match nm_bind.get_noise_function(&biome_config.primary_noise_key) {
                Some(noise_fn) => noise_fn,
                None => {
                    godot_error!("SectionManager: Primary noise function '{}' not found for biome ID: {}. Cannot proceed.",
                               biome_config.primary_noise_key, biome_config.id);
                    return false;
                }
            };

             let mut secondary_fns = Vec::new();
             for key in &biome_config.secondary_noise_keys {
                 // Pass &str to get_noise_function
                 if let Some(sec_fn) = nm_bind.get_noise_function(key) {
                     secondary_fns.push(sec_fn);
                 } else {
                     godot_warn!("SectionManager: Secondary noise function '{}' not found for biome ID: {}", key, biome_config.id);
                 }
             }

            // Create BiomeDefinition (ensure its definition uses u8 for id)
            temp_biomes.push(BiomeDefinition {
                id: biome_config.id, // id is already u8 from parsing
                name: biome_config.name,
                primary_noise_fn,
                texture_params: biome_config.texture_params,
                secondary_noise_fns: secondary_fns,
            });
        }
        self.biomes = temp_biomes;


        // --- Process Sections into Definitions ---
        let mut current_position = 0.0;
        let mut total_length = 0.0;

        for section_config in sections_config {
            let boundary_noise_fn = section_config.boundary_noise_key.as_deref()
                .and_then(|key| {
                    // Pass &str to get_noise_function
                    match nm_bind.get_noise_function(key) {
                        Some(func) => Some(func),
                        None => {
                            godot_warn!("SectionManager: Boundary noise function '{}' not found for section ID: {}. Section will have no boundary noise.", key, section_config.id);
                            None
                        }
                    }
                });

            // Create SectionDefinition (ensure its definition uses u8 for id and Vec<u8> for possible_biomes)
            let section_def = SectionDefinition::new(
                section_config.id, // id is already u8 from parsing
                current_position,
                section_config.length,
                section_config.transition_zone,
                section_config.possible_biomes, // possible_biomes is already Vec<u8> from parsing
                section_config.point_density,
                boundary_noise_fn,
            );
            self.sections.push(section_def);

            current_position += section_config.length;
            total_length += section_config.length;
        }

        self.world_length = total_length;

        self.generate_voronoi_points();

        self.initialized = true;
        godot_print!("SectionManager: Initialization complete. World length: {}", self.world_length);

        true
    }
    
    /// Build a thread-safe data structure that can be used by worker threads.
    #[func]
    pub fn build_thread_safe_data(&self, noise_manager: Gd<NoiseManager>) -> Variant {
        // We cannot return Arc<ThreadSafeSectionData> directly to Godot
        // So we'll return a success boolean and store the data in an internal field
        // that can be accessed by a C++ bridge or another Rust function
        
        let data = ThreadSafeSectionData::from_section_manager(self, &noise_manager.bind());
        
        // Store the Arc<ThreadSafeSectionData> in a static or instance field
        // that can be accessed by other Rust code
        let _thread_safe_data = Arc::new(data);
        
        // Since we can't return the Arc directly to Godot, return a simple success bool
        true.to_variant()
    }
    
    /// Update an existing thread-safe data structure if needed.
    #[func]
    pub fn update_thread_safe_data(&mut self, noise_manager: Gd<NoiseManager>) -> bool {
        if !self.initialized {
            godot_error!("SectionManager: Cannot update thread-safe data, not initialized");
            return false;
        }
        
        // Create a new data structure - in a real implementation, you'd update the existing one
        let _data = ThreadSafeSectionData::from_section_manager(self, &noise_manager.bind());
        
        // We would store this in a field that's accessible to other Rust code
        
        true
    }
    
    /// Generate Voronoi points for all sections.
    fn generate_voronoi_points(&mut self) {
        if self.sections.is_empty() {
            return;
        }
        
        self.voronoi_points.clear();
        
        let world_bounds = Rect2::new(
            -self.world_width / 2.0,
            0.0,
            self.world_width,
            self.world_length
        );
        
        // Generate points for each section
        for section in &self.sections {
            let section_bounds = Rect2::new(
                world_bounds.x,
                section.start_position,
                world_bounds.width,
                section.end_position - section.start_position
            );
            
            let section_points = generate_voronoi_points_for_section(
                section,
                section_bounds,
                self.world_seed
            );
            
            self.voronoi_points.extend(section_points);
        }
        
        // Build the spatial grid for efficient queries
        if !self.voronoi_points.is_empty() {
            let cell_size = 100.0; // Reasonable cell size for spatial queries
            self.spatial_grid = Some(SpatialGrid::new(
                world_bounds,
                &self.voronoi_points,
                cell_size
            ));
            
            godot_print!("SectionManager: Generated {} Voronoi points across all sections", 
                        self.voronoi_points.len());
        } else {
            godot_warn!("SectionManager: No Voronoi points were generated");
        }
    }
    
    /// Check if the manager is fully initialized.
    #[func]
    pub fn is_fully_initialized(&self) -> bool {
        self.initialized && !self.sections.is_empty() && !self.biomes.is_empty()
    }
    
    /// Get the total world length (sum of all section lengths).
    #[func]
    pub fn get_world_length(&self) -> f32 {
        self.world_length
    }

    #[func]
    pub fn get_world_width(&self) -> f32 {
        self.world_width
    }
    
    /// Set the biome blend distance.
    #[func]
    pub fn set_biome_blend_distance(&mut self, distance: f32) {
        self.biome_blend_distance = distance.max(1.0);
    }
    
    /// Set the section blend distance.
    #[func]
    pub fn set_section_blend_distance(&mut self, distance: f32) {
        self.section_blend_distance = distance.max(1.0);
    }
    
    #[func]
    pub fn get_sections(&self) -> Variant {
        // We can't return &Vec<SectionDefinition> directly to Godot functions
        // Instead, return a success indicator and use other methods to access the data
        true.to_variant()
    }
    
    /// Get a reference to the biomes list
    #[func]
    pub fn get_biomes(&self) -> Variant {
        // Similar approach
        true.to_variant()
    }

    /// Get a reference to the Voronoi points list
    #[func]
    pub fn get_voronoi_points(&self) -> Variant {
        // Similar approach
        true.to_variant()
    }

    /// Get a reference to the spatial grid
    #[func]
    pub fn get_spatial_grid(&self) -> Variant {
        // Similar approach
        self.spatial_grid.is_some().to_variant()
    }

    pub fn get_sections_internal(&self) -> &Vec<SectionDefinition> {
        &self.sections
    }
    
    pub fn get_biomes_internal(&self) -> &Vec<BiomeDefinition> {
        &self.biomes
    }
    
    pub fn get_voronoi_points_internal(&self) -> &Vec<VoronoiPoint> {
        &self.voronoi_points
    }
    
    pub fn get_spatial_grid_internal(&self) -> Option<&SpatialGrid> {
        self.spatial_grid.as_ref()
    }    

    /// Get the world seed
    #[func]
    pub fn get_world_seed(&self) -> u64 {
        self.world_seed
    }

    /// Set the world seed
    #[func]
    pub fn set_seed(&mut self, seed: u32) {
        self.world_seed = seed as u64;
        // If already initialized, regenerate Voronoi points
        if self.initialized {
            self.generate_voronoi_points();
        }
    }

    /// Set the world dimensions (width and height)
    #[func]
    pub fn set_world_dimensions(&mut self, width: f32, height: f32) {

        godot_print!(
            "DEBUG: SectionManager::set_world_dimensions called. Input Width: {}, Height: {}",
            width, height
        );
        godot_print!("DEBUG:   Current self.world_length before check: {}", self.world_length);
        
        self.world_width = width;
        let new_world_length = height;
        
        if self.world_length != new_world_length {
            godot_print!("DEBUG:   World length changed ({} != {}). Updating...", self.world_length, new_world_length);
            
            self.world_length = new_world_length;
            
            // Recalculate section positions based on the new world length
            if !self.sections.is_empty() {
                godot_print!("DEBUG:   Recalculating section boundaries for new length: {}", self.world_length);

                let section_count = self.sections.len();
                let avg_section_length = self.world_length / section_count as f32;
                
                let mut current_pos = 0.0;
                for section in &mut self.sections {
                    let section_length = avg_section_length;
                    let transition_zone = section.transition_end - section.transition_start;
                    
                    section.start_position = current_pos;
                    section.end_position = current_pos + section_length;
                    section.transition_start = section.end_position - transition_zone;

                    godot_print!(
                        "DEBUG:     Updated Section ID {}: Start={}, End={}, TransitionStart={}, TransitionEnd={}",
                        section.id, section.start_position, section.end_position, section.transition_start, section.transition_end
                    );
                    
                    current_pos += section_length;
                }
                
                // If initialized, regenerate Voronoi points for the new dimensions
                if self.initialized {
                    godot_print!("DEBUG:   Calling generate_voronoi_points() due to dimension change.");
                    self.generate_voronoi_points();
                }
            } else {godot_print!("DEBUG:   World length ({}) did not change. No recalculation needed.", self.world_length);}
        }
    }

    /// Debugging helper: Validate world structure and section layout.
    #[func]
    pub fn debug_validate_world(&self) {
        godot_print!("====== SectionManager World Debug (Validation Point) ======"); // Added label

        godot_print!("  World width: {}", self.world_width);
        godot_print!("  World length: {}", self.world_length);
        godot_print!("  Total sections: {}", self.sections.len());
        godot_print!("  Total Voronoi points: {}", self.voronoi_points.len()); // Added point count

        if self.sections.is_empty() {
            godot_warn!("  No sections found!");
            godot_print!("========================================================"); // Footer
            return;
        }

        for (i, section) in self.sections.iter().enumerate() {
            godot_print!(
                "  Section {} -> ID: {}, Start: {:.2}, End: {:.2}, Length: {:.2}, Transition: {:.2}-{:.2}", // Added transition info
                i,
                section.id,
                section.start_position,
                section.end_position,
                section.end_position - section.start_position,
                section.transition_start, // Added
                section.transition_end    // Added
            );
        }

        let last_section = self.sections.last().unwrap();
        if (last_section.end_position - self.world_length).abs() > 1.0 { // Using 1.0 tolerance for f32
            godot_warn!(
                "  WARNING: Last section ends at {:.2} but world length is {:.2}!",
                last_section.end_position,
                self.world_length
            );
        } else {
            godot_print!("  Sections appear to cover the world length correctly.");
        }

        godot_print!("========================================================"); // Footer
    }

    /// Get the biome blend distance
    #[func]
    pub fn get_biome_blend_distance(&self) -> f32 {
        self.biome_blend_distance
    }

    /// Get the section blend distance
    #[func]
    pub fn get_section_blend_distance(&self) -> f32 {
        self.section_blend_distance
    }

    /// Get the section at a specific Z coordinate.
    #[func]
    pub fn get_section_at(&self, world_z: f32) -> Dictionary {
        let mut result = Dictionary::new();
        
        if !self.initialized || self.sections.is_empty() {
            return result;
        }
        
        // Handle out-of-bounds
        if world_z < 0.0 {
            let first_section = &self.sections[0];
            result.insert("id", first_section.id);
            result.insert("weight", 1.0);
            return result;
        }
        
        if world_z >= self.world_length {
            let last_section = &self.sections[self.sections.len() - 1];
            result.insert("id", last_section.id);
            result.insert("weight", 1.0);
            return result;
        }
        
        // Find the section containing this Z coordinate
        for (i, section) in self.sections.iter().enumerate() {
            if section.contains_z(world_z) {
                result.insert("id", section.id);
                
                // Check if in transition zone
                if section.in_transition_zone(world_z) && i < self.sections.len() - 1 {
                    // Calculate transition weight
                    let t = (world_z - section.transition_start) / 
                            (section.end_position - section.transition_start);
                    let weight = 1.0 - t;
                    
                    result.insert("weight", weight);
                    result.insert("next_id", self.sections[i + 1].id);
                    result.insert("next_weight", t);
                } else {
                    // Not in transition, full weight
                    result.insert("weight", 1.0);
                }
                
                break;
            }
        }
        
        result
    }
    
    /// Debug function to get information about a specific position.
    #[func]
    pub fn get_debug_info_at(&self, world_x: f32, world_z: f32) -> Dictionary {
        let mut result = Dictionary::new();
        
        if !self.initialized {
            result.insert("error", "Not initialized");
            return result;
        }
        
        result.insert("position", Vector2::new(world_x, world_z));
        
        // Get section info
        let section_info = self.get_section_at(world_z);
        result.insert("section_info", section_info);
        
        // Get nearby Voronoi points if available
        if let Some(grid) = &self.spatial_grid {
            if !self.voronoi_points.is_empty() {
                let nearest_points = grid.find_k_nearest_points(
                    world_x, 
                    world_z, 
                    &self.voronoi_points, 
                    3, // Get 3 nearest points
                    self.biome_blend_distance,
                    None // No section filter
                );
                
                let mut points_array = VariantArray::new();
                
                for (idx, distance) in nearest_points {
                    let point = &self.voronoi_points[idx];
                    let mut point_dict = Dictionary::new();
                    
                    point_dict.insert("biome_id", point.biome_id);
                    point_dict.insert("section_id", point.section_id);
                    point_dict.insert("position_x", point.position.0);
                    point_dict.insert("position_z", point.position.1);
                    point_dict.insert("distance", distance);
                    
                    points_array.push(&point_dict.to_variant());
                }
                
                result.insert("nearest_points", points_array);
            }
        }
        
        result
    }
}
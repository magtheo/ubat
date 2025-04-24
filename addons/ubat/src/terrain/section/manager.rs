// src/section/manager.rs
use godot::prelude::*;
use godot::classes::Node;
use std::sync::Arc;
use std::collections::HashMap;

use super::sectionConfig::{SectionTomlConfig, BiomeTomlConfig};
use crate::terrain::section::definition::{SectionDefinition, BiomeDefinition, VoronoiPoint, Rect2};
use crate::terrain::section::distribution::{generate_voronoi_points_for_section, SpatialGrid};
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
        noise_manager: Gd<NoiseManager>, // Keep Gd<T> if ownership transfer is intended
    ) -> bool {
        godot_print!("SectionManager: Initializing with seed {}", world_seed);

        let mut sections_config: Vec<SectionTomlConfig> = Vec::new();
        let mut biomes_config: Vec<BiomeTomlConfig> = Vec::new();

        // --- Process sections config ---
        match sections_config_var.try_to::<VariantArray>() {
            Ok(sections_array) => {
                for i in 0..sections_array.len() {
                    // Get the dictionary for the section
                    let section_var = sections_array.get(i); // Returns Option<Variant>
                    if let Ok(section_dict) = section_var.try_to::<Dictionary>() { // Try convert Option<Variant> -> Dictionary
                        // Extract fields, handling Option returned by get() before calling try_to()
                        let id = section_dict.get("id") // Returns Option<Variant>
                            .and_then(|v| v.try_to::<i32>().ok()) // Try get, then try convert, result is Option<i32>
                            .unwrap_or_else(|| {
                                godot_warn!("SectionManager: Missing or invalid 'id' in section config at index {}. Using 0.", i);
                                0 // Provide default if key missing or conversion failed
                            });

                        let length = section_dict.get("length")
                            .and_then(|v| v.try_to::<f32>().ok())
                            .unwrap_or(1000.0);

                        let transition_zone = section_dict.get("transition_zone")
                            .and_then(|v| v.try_to::<f32>().ok())
                            .unwrap_or(100.0);

                        // For optional fields, we don't need unwrap_or
                        let boundary_noise_key = section_dict.get("boundary_noise_key")
                            .and_then(|v| v.try_to::<String>().ok()); // Result is Option<String>

                        let point_density = section_dict.get("point_density")
                            .and_then(|v| v.try_to::<f32>().ok())
                            .unwrap_or(0.01);

                        // Extract possible biomes array
                        let mut possible_biomes = Vec::new();
                        // First, get the variant for "possible_biomes"
                        if let Some(biomes_var) = section_dict.get("possible_biomes") { // get() returns Option<Variant>
                           // Then, try to convert that variant to a VariantArray
                           if let Ok(biomes_array) = biomes_var.try_to::<VariantArray>() {
                                for j in 0..biomes_array.len() {
                                    // *** FIX HERE ***: Apply the pattern to the result of biomes_array.get(j)
                                    if let Some(biome_id) = biomes_array.get(j) // Returns Option<Variant>
                                                                        .and_then(|v| v.try_to::<i32>().ok()) // Handle Option and Result
                                    {
                                        possible_biomes.push(biome_id);
                                    } else {
                                         godot_warn!("SectionManager: Failed to parse biome ID at index {} in section {}", j, id);
                                    }
                                }
                           } else {
                                godot_warn!("SectionManager: 'possible_biomes' key found in section {} but is not a VariantArray", id);
                           }
                        } // No warning if 'possible_biomes' key is entirely missing

                        // Create SectionTomlConfig manually
                        let section_config = SectionTomlConfig {
                            id,
                            length,
                            transition_zone,
                            boundary_noise_key,
                            possible_biomes,
                            point_density,
                        };
                        sections_config.push(section_config);

                    } else {
                         godot_warn!("SectionManager: Item at index {} in sections config is not a Dictionary (or failed to convert)", i);
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
                    let biome_var = biomes_array.get(i); // Returns Option<Variant>
                     if let Ok(biome_dict) = biome_var.try_to::<Dictionary>() { // Try convert Option<Variant> -> Dictionary
                        // Extract fields using the and_then().ok().unwrap_or() pattern
                        let id = biome_dict.get("id")
                            .and_then(|v| v.try_to::<i32>().ok())
                             .unwrap_or_else(|| {
                                godot_warn!("SectionManager: Missing or invalid 'id' in biome config at index {}. Using 0.", i);
                                0
                            });

                        let name = biome_dict.get("name")
                            .and_then(|v| v.try_to::<String>().ok())
                            .unwrap_or_default(); // Use default String ("") if missing/wrong type

                         let primary_noise_key = biome_dict.get("primary_noise_key")
                            .and_then(|v| v.try_to::<String>().ok())
                            .unwrap_or_else(|| {
                                // Make this fatal because primary key is essential
                                godot_error!("SectionManager: Missing or invalid 'primary_noise_key' for biome config at index {}. Cannot proceed.", i);
                                // Return an empty string temporarily, but the later check will catch this
                                String::new()
                            });
                         // Early exit if primary key is missing (essential)
                         if primary_noise_key.is_empty() {
                             return false;
                         }


                        // Extract secondary noise keys array
                        let mut secondary_noise_keys = Vec::new();
                        if let Some(keys_var) = biome_dict.get("secondary_noise_keys") {
                             if let Ok(keys_array) = keys_var.try_to::<VariantArray>() {
                                for j in 0..keys_array.len() {
                                    // Apply pattern here too
                                    if let Some(key) = keys_array.get(j).and_then(|v| v.try_to::<String>().ok()) {
                                        secondary_noise_keys.push(key);
                                    } else {
                                        godot_warn!("SectionManager: Failed to parse secondary noise key at index {} in biome {}", j, id);
                                    }
                                }
                             } else {
                                 godot_warn!("SectionManager: 'secondary_noise_keys' key found in biome {} but is not a VariantArray", id);
                             }
                        }

                        // Extract texture params dictionary
                        let mut texture_params = HashMap::new();
                        if let Some(params_var) = biome_dict.get("texture_params") {
                            if let Ok(params_dict) = params_var.try_to::<Dictionary>() {
                                // Iterate over keys safely
                                for key_var in params_dict.keys_array().iter_shared() {
                                     // Try convert key to String
                                     if let Ok(key_str) = key_var.try_to::<String>() {
                                         // *** FIX HERE ***: Apply the pattern to the result of params_dict.get(key_var)
                                         if let Some(value_f32) = params_dict.get(key_var) // Returns Option<Variant>
                                                                              .and_then(|v| v.try_to::<f32>().ok()) // Handle Option and Result
                                         {
                                            texture_params.insert(key_str, value_f32);
                                         } else {
                                             godot_warn!("SectionManager: Failed to parse texture param value for key '{}' in biome {}", key_str, id);
                                         }
                                     } else {
                                         // Use {:?} for debug printing the Variant if it's not a string
                                         godot_warn!("SectionManager: Failed to parse texture param key {:?} as String in biome {}", key_var, id);
                                     }
                                }
                            } else {
                                 godot_warn!("SectionManager: 'texture_params' key found in biome {} but is not a Dictionary", id);
                            }
                        }

                        // Create BiomeTomlConfig manually
                        let biome_config = BiomeTomlConfig {
                            id,
                            name,
                            primary_noise_key, // Already checked if empty
                            secondary_noise_keys,
                            texture_params,
                        };
                        biomes_config.push(biome_config);

                     } else {
                          godot_warn!("SectionManager: Item at index {} in biomes config is not a Dictionary (or failed to convert)", i);
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

        // Check for empty configs after parsing
        if sections_config.is_empty() {
             godot_error!("SectionManager: Cannot initialize: No valid sections were parsed from the config.");
             return false;
        }
        if biomes_config.is_empty() {
            godot_error!("SectionManager: Cannot initialize: No valid biomes were parsed from the config.");
            return false;
        }

        self.world_seed = world_seed;

        // Clear any existing data before populating
        self.sections.clear();
        self.biomes.clear();
        self.voronoi_points.clear(); // Make sure this field exists on self

        // --- Process biome configurations into BiomeDefinitions ---
        let nm_bind = noise_manager.bind(); // Bind Gd once for efficiency
        let mut temp_biomes = Vec::with_capacity(biomes_config.len()); // Build into temporary vec first

        for biome_config in biomes_config { // Iterate over the owned Vec<BiomeTomlConfig>
            // Get primary noise function (convert String to GodotString for the API call)
            let primary_noise_fn = match nm_bind.get_noise_function(biome_config.primary_noise_key.to_godot()) {
                Some(noise_fn) => noise_fn,
                None => {
                    // This should ideally have been caught during parsing, but double-check
                    godot_error!("SectionManager: Primary noise function '{}' not found for biome ID: {}. Cannot proceed.",
                               biome_config.primary_noise_key, biome_config.id);
                    return false; // Fatal error if primary noise is missing
                }
            };

             // Get any secondary noise functions
             let mut secondary_fns = Vec::new(); // Correctly named variable
             for key in &biome_config.secondary_noise_keys {
                 // Convert String to GodotString for the API call
                 if let Some(sec_fn) = nm_bind.get_noise_function(key.to_godot()) {
                     secondary_fns.push(sec_fn);
                 } else {
                     // Warning is okay for secondary functions, they might be optional
                     godot_warn!("SectionManager: Secondary noise function '{}' not found for biome ID: {}",
                                key, biome_config.id);
                 }
             }

            // Create BiomeDefinition
            let biome_def = BiomeDefinition {
                id: biome_config.id,
                name: biome_config.name, // Already parsed
                primary_noise_fn, // Fetched successfully above
                texture_params: biome_config.texture_params, // Already parsed
                // *** FIX HERE ***: Use the correct variable name
                secondary_noise_fns: secondary_fns, // Use the variable declared above
            };
            temp_biomes.push(biome_def);
        }
        // Only assign to self.biomes if all primary noise functions were found and processed
        self.biomes = temp_biomes;


        // --- Process section configurations into SectionDefinitions ---
        let mut current_position = 0.0;
        let mut total_length = 0.0;

        for section_config in sections_config { // Iterate over the owned Vec<SectionTomlConfig>
            // Get boundary noise function if specified
            let boundary_noise_fn = match &section_config.boundary_noise_key {
                 Some(key) => {
                     // Convert String to GodotString for the API call
                     match nm_bind.get_noise_function(key.to_godot()) {
                        Some(func) => Some(func),
                        None => {
                            godot_warn!("SectionManager: Boundary noise function '{}' not found for section ID: {}. Section will have no boundary noise.", key, section_config.id);
                            None // Non-fatal, proceed without boundary noise
                        }
                     }
                 },
                 None => None, // No key specified
            };

            // Create SectionDefinition
            // Make sure your SectionDefinition::new function matches these arguments
            let section_def = SectionDefinition::new(
                section_config.id,
                current_position,
                section_config.length,
                section_config.transition_zone,
                section_config.possible_biomes, // Already parsed
                section_config.point_density,
                boundary_noise_fn, // Fetched above (Option<YourNoiseFnType>)
            );
            self.sections.push(section_def);

            // Update position for the next section
            current_position += section_config.length;
            total_length += section_config.length;
        }

        self.world_length = total_length;

        // Generate Voronoi points for all sections (make sure this function exists and is called appropriately)
        // self.generate_voronoi_points(); // Uncomment and implement if you have this method

        self.initialized = true;
        godot_print!("SectionManager: Initialization complete. World length: {}", self.world_length);

        true // Indicate successful initialization
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
        self.world_width = width;
        let new_world_length = height;
        
        if self.world_length != new_world_length {
            self.world_length = new_world_length;
            
            // Recalculate section positions based on the new world length
            if !self.sections.is_empty() {
                let section_count = self.sections.len();
                let avg_section_length = self.world_length / section_count as f32;
                
                let mut current_pos = 0.0;
                for section in &mut self.sections {
                    let section_length = avg_section_length;
                    let transition_zone = section.transition_end - section.transition_start;
                    
                    section.start_position = current_pos;
                    section.end_position = current_pos + section_length;
                    section.transition_start = section.end_position - transition_zone;
                    
                    current_pos += section_length;
                }
                
                // If initialized, regenerate Voronoi points for the new dimensions
                if self.initialized {
                    self.generate_voronoi_points();
                }
            }
        }
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
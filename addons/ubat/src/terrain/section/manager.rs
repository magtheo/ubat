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
use crate::terrain::terrain_config::TerrainConfigManager;

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
            
            world_length: 1000.0, // Placeholder/Default - Set by set_world_dimensions

            world_width: 10000.0, // Placeholder/Default - Set by set_world_dimensions
            world_seed: 0,
            
            initialized: false,
            biome_blend_distance: 300.0, // Default, overwritten later
            section_blend_distance: 300.0, // Default section blend
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

        godot_print!("SectionManager::initialize - About to generate Voronoi points.");
        godot_print!("  Effective self.world_width = {}", self.world_width);
        godot_print!("  Effective self.world_length = {}", self.world_length); // This should be sum of TOML section lengths
        godot_print!("  Effective self.world_seed = {}", self.world_seed);
        godot_print!("  Effective self.biome_blend_distance = {}", self.biome_blend_distance);

        // <<< ADDED >>> Reset state at the beginning
        self.sections.clear();
        self.biomes.clear();
        self.voronoi_points.clear();
        self.spatial_grid = None;
        self.initialized = false;
        self.world_seed = world_seed;

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
        let mut total_length_from_toml = 0.0;

        for section_config in &sections_config { // Iterate over ref first to get total length
            total_length_from_toml += section_config.length;
        }

        // <<< MODIFIED >>> Determine scaling factor based on pre-set self.world_length vs TOML sum
        let length_scale_factor = if total_length_from_toml > 1e-5 && (self.world_length - total_length_from_toml).abs() > 1.0 {
            godot_warn!(
                "SectionManager::initialize - World length from config ({}) differs from sum of section lengths ({}). Rescaling sections.",
                self.world_length, total_length_from_toml
            );
            self.world_length / total_length_from_toml // Rescale to fit global config height
        } else {
            // If lengths match or TOML sum is zero, don't scale. If lengths didn't match, world_length might be adjusted below.
            if total_length_from_toml > 1e-5 && self.world_length < 1.0 {
                 // If world_length wasn't set properly beforehand, use the TOML sum
                 godot_warn!("SectionManager::initialize - Pre-set world_length ({}) seems invalid. Using sum of TOML section lengths ({}) instead.", self.world_length, total_length_from_toml);
                 self.world_length = total_length_from_toml;
            } else if total_length_from_toml < 1e-5 {
                 godot_warn!("SectionManager::initialize - Sum of TOML section lengths is zero. Cannot determine scale factor.");
            }
            1.0 // Default to no scaling
        };
        godot_print!("SectionManager::initialize - Using length scale factor: {}", length_scale_factor);

        for section_config_item in &sections_config { // Iterate over ref
            let boundary_noise_fn = section_config_item.boundary_noise_key.as_deref()
                .and_then(|key| nm_bind.get_noise_function(key));

            let scaled_length = section_config_item.length * length_scale_factor;
            let scaled_transition = (section_config_item.transition_zone * length_scale_factor).min(scaled_length * 0.99).max(0.0);

            let section_def = SectionDefinition::new(
                section_config_item.id, current_position, scaled_length, scaled_transition,
                section_config_item.possible_biomes.clone(), section_config_item.point_density, boundary_noise_fn,
            );
            self.sections.push(section_def);
            current_position += scaled_length;
        }
        // If scaling occurred, current_position should now closely match self.world_length
        godot_print!("  Created {} SectionDefinitions. Final calculated end position of last section: {}", self.sections.len(), current_position);


        // --- Set Blend Distance and Generate Points ---
        if let Ok(tc_guard) = TerrainConfigManager::get_config().read() {
            self.biome_blend_distance = tc_guard.blend_distance;
            godot_print!("  Set Biome Blend Distance from config: {}", self.biome_blend_distance);
        } else {
            godot_error!("  Failed to read TerrainConfig for blend_distance! Using default: {}", self.biome_blend_distance);
        }

        godot_print!("SectionManager::initialize - FINAL CHECK Before Generating Voronoi Points:");
        godot_print!("  World Width = {}", self.world_width);
        godot_print!("  World Length = {}", self.world_length); // This is now the definitive length
        godot_print!("  World Seed = {}", self.world_seed);
        godot_print!("  Biome Blend Distance = {}", self.biome_blend_distance);

        self.generate_voronoi_points();

        self.initialized = true;
        godot_print!("SectionManager: Initialization complete.");
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
            godot_print!("SectionManager::generate_voronoi_points - No sections defined, cannot generate points.");
            self.voronoi_points.clear();
            self.spatial_grid = None;
            return;
        }
        
        self.voronoi_points.clear();
        
        // Define the overall bounds for Voronoi point generation and the spatial grid.
        // Points are generated per section, but the grid covers the whole world.
        let world_bounds = Rect2::new(
            -self.world_width / 2.0, // Centered around X=0
            0.0,                     // Starts at Z=0
            self.world_width,
            self.world_length
        );
        
        godot_print!(
            "SectionManager::generate_voronoi_points - World Bounds for grid: X: {:.1}, Z: {:.1}, W: {:.1}, H: {:.1}",
            world_bounds.x, world_bounds.z, world_bounds.width, world_bounds.height
        );

        // Generate points for each section
        for section in &self.sections {
            // Define the specific bounds for *this* section's point generation
            let section_bounds = Rect2::new(
                world_bounds.x,             // Use the same X start as world_bounds
                section.start_position,
                world_bounds.width,         // Use the full world width for points in this section
                section.end_position - section.start_position // Length of this section
            );
            
            // godot_print!( // Optional: Log individual section bounds
            //     "  Generating points for Section ID {}: Bounds X: {:.1}, Z: {:.1}, W: {:.1}, H: {:.1}",
            //     section.id, section_bounds.x, section_bounds.z, section_bounds.width, section_bounds.height
            // );

            let section_points = generate_voronoi_points_for_section(
                section,
                section_bounds,
                self.world_seed // Use the manager's world seed
            );
            
            self.voronoi_points.extend(section_points);
        }
        
        // Build the spatial grid for efficient queries
        if !self.voronoi_points.is_empty() {
            // --- START OF MODIFICATION: Make cell_size adaptive ---
            // Aim for the 3x3 grid cell search to cover roughly the blend_distance radius.
            // A 3x3 grid search (1 cell neighbor in each direction) covers a square region
            // of 3*cell_size width/height. The diagonal of this is sqrt(2) * 3 * cell_size.
            // We want this search area to be generous enough for biome_blend_distance.
            // A simpler heuristic: ensure one cell is not drastically larger than the blend distance.
            // Let's make cell_size roughly half to a third of the blend_distance,
            // clamped to reasonable min/max values.
            // Example: if blend_distance = 150, cell_size could be 75. A 3x3 search covers 225x225.
            let calculated_cell_size = (self.biome_blend_distance / 2.0).max(50.0).min(self.world_width / 10.0); // Ensure at least 10 cells across world width
            
            godot_print!(
                "SectionManager: Biome Blend Distance: {:.1}. Using adaptive cell_size for SpatialGrid: {:.1}",
                self.biome_blend_distance,
                calculated_cell_size
            );

            self.spatial_grid = Some(SpatialGrid::new(
                world_bounds, // Grid covers the entire world_bounds
                &self.voronoi_points,
                calculated_cell_size // Use the adaptive cell_size
            ));
            
            godot_print!("SectionManager: Generated {} Voronoi points across all sections and built SpatialGrid.", 
                        self.voronoi_points.len());
        } else {
            godot_warn!("SectionManager: No Voronoi points were generated. SpatialGrid will not be built.");
            self.spatial_grid = None;
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
// src/section/thread_safe_data.rs
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::Sender;

use crate::terrain::chunk_manager::ChunkResult;
use crate::terrain::noise::noise_manager::NoiseManager;
use crate::terrain::section::definition::{BiomeDefinition, SectionDefinition};
use crate::terrain::section::layout::calculate_section_weights;
use crate::terrain::section::manager::SectionManager;
use crate::terrain::terrain_config::TerrainConfigManager;
use noise::NoiseFn;

use crate::godot_print;
use godot::prelude::*;
use std::fmt;

/// A simple smoothstep function.
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Thread-safe data container for section and biome information.
/// This structure can be safely shared between threads for terrain generation.
#[derive(Clone)]
pub struct ThreadSafeSectionData {
    pub sections: Arc<Vec<SectionDefinition>>,
    pub biomes: Arc<Vec<BiomeDefinition>>,

    pub world_length: f32,
    pub seed: u64,

    pub section_blend_distance: f32,
    pub blend_noise_strength: f32,
}

impl ThreadSafeSectionData {
    /// Create a new ThreadSafeSectionData from a SectionManager.
    pub fn from_section_manager(manager: &SectionManager, _noise_manager: &NoiseManager) -> Self {
        let blend_noise_strength = if let Ok(guard) = TerrainConfigManager::get_config().read() {
            guard.blend_noise_strength
        } else {
            godot_print!(
                "Failed to read terrain config for blend_noise_strength. Using default 0.25"
            );
            0.25f32 // Default value if lock fails
        };

        Self {
            sections: Arc::new(manager.get_sections_internal().clone()),
            biomes: Arc::new(manager.get_biomes_internal().clone()),

            world_length: manager.get_world_length(),
            seed: manager.get_world_seed(),

            section_blend_distance: manager.get_section_blend_distance(),
            blend_noise_strength,
        }
    }

    /// Get the sections and biomes that influence a position, along with their weights.
    /// This is the core logic for blending biomes based on section Z-position and intra-section noise.
    pub fn get_section_and_biome_weights(
        &self,
        world_x: f32,
        world_z: f32,
        _sender: &Sender<ChunkResult>, // Keep sender for potential future logging
    ) -> Vec<(u8, f32)> {
        // Step 1: Calculate overall section weights based on Z-axis position.
        // This determines the influence of entire sections (e.g., 70% Section A, 30% Section B).
        let section_contributions = calculate_section_weights(world_z, world_x, &self.sections);
        if section_contributions.is_empty() {
            // Fallback: if no sections, return a default biome.
            return vec![(0, 1.0)];
        }

        // Step 2: Initialize a HashMap to accumulate the final weights for each biome.
        let mut final_biome_weights: HashMap<u8, f32> = HashMap::new();

        // Step 3: Iterate through each contributing section and distribute its weight among its possible biomes.
        for (section_id, overall_section_weight) in section_contributions {
            if overall_section_weight < 0.001 {
                continue;
            }

            // Find the definition for the current section.
            if let Some(section_def) = self.sections.iter().find(|s| s.id == section_id) {
                let possible_biomes = &section_def.possible_biomes;

                if possible_biomes.is_empty() {
                    continue; // Should not happen with valid config.
                }

                // If there's a blend noise function and more than one biome, use noise to blend them.
                if let (Some(blend_noise_fn), true) =
                    (&section_def.blend_noise_fn, possible_biomes.len() > 1)
                {
                    // Get the noise value for the current world position.
                    let noise_val = blend_noise_fn.get([world_x as f64, world_z as f64]); // Range [-1, 1]

                    // --- Interpretation Logic ---
                    // This part determines how the noise value maps to biome weights.
                    let num_possible = possible_biomes.len();
                    let mut intra_section_biome_weights = vec![0.0f32; num_possible];

                    // Example for 2 biomes: simple linear interpolation mapped from noise.
                    if num_possible == 2 {
                        let t = (noise_val * 0.5 + 0.5) as f32; // Maps [-1,1] to [0,1]

                        // FIXED: Use wider range for more blending
                        intra_section_biome_weights[0] = 1.0 - smoothstep(0.2, 0.8, t); // 60% blend zone
                        intra_section_biome_weights[1] = smoothstep(0.2, 0.8, t);

                        // OR for even more mixing:
                        // intra_section_biome_weights[0] = 1.0 - smoothstep(0.1, 0.9, t); // 80% blend zone
                        // intra_section_biome_weights[1] = smoothstep(0.1, 0.9, t);
                    }
                    // Example for 3+ biomes: divide the noise range into segments.
                    else {
                        // FIXED: Use overlapping influence zones
                        let segment_size = 2.0 / num_possible as f64;
                        let overlap_factor = 1.5; // Allow 50% overlap between biome zones

                        for i in 0..num_possible {
                            let biome_center_noise_val = -1.0 + segment_size * (i as f64 + 0.5);
                            let dist_to_center = (noise_val - biome_center_noise_val).abs();

                            // Use wider influence with overlap
                            let influence_radius = segment_size * overlap_factor;
                            let raw_weight =
                                (1.0 - dist_to_center / influence_radius).max(0.0) as f32;

                            // Optional: Use smoother falloff
                            // let normalized_dist = (dist_to_center / influence_radius).clamp(0.0, 1.0);
                            // let raw_weight = (1.0 - normalized_dist * normalized_dist) as f32; // Quadratic falloff

                            intra_section_biome_weights[i] = raw_weight;
                        }
                    }

                    // Normalize the intra-section weights to sum to 1.
                    let total_intra_weight: f32 = intra_section_biome_weights.iter().sum();
                    if total_intra_weight > 0.01 {
                        for i in 0..num_possible {
                            let biome_id = possible_biomes[i];
                            let normalized_intra_weight =
                                intra_section_biome_weights[i] / total_intra_weight;
                            // The final contribution is the section's overall weight multiplied by the biome's intra-section weight.
                            *final_biome_weights.entry(biome_id).or_insert(0.0) +=
                                normalized_intra_weight * overall_section_weight;
                        }
                    } else {
                        // Fallback if all weights are tiny, give full section weight to the first possible biome.
                        *final_biome_weights.entry(possible_biomes[0]).or_insert(0.0) +=
                            overall_section_weight;
                    }
                } else {
                    // If no blend noise or only one biome, give the section's entire weight to the first possible biome.
                    *final_biome_weights.entry(possible_biomes[0]).or_insert(0.0) +=
                        overall_section_weight;
                }
            }
        }

        // Step 4: Convert the HashMap to a Vec, sort by weight, and take the top N (e.g., 3).
        let mut sorted_weights: Vec<(u8, f32)> = final_biome_weights.into_iter().collect();
        sorted_weights.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take the top 3 biomes for the shader.
        sorted_weights.truncate(3);

        // Step 5: Re-normalize the weights of the top N biomes so they sum to 1.0.
        // This is crucial for the shader to correctly blend textures.
        let total_top_weight: f32 = sorted_weights.iter().map(|&(_, w)| w).sum();
        if total_top_weight > 0.01 {
            for (_, weight) in sorted_weights.iter_mut() {
                *weight /= total_top_weight;
            }
        } else if !sorted_weights.is_empty() {
            // If total weight is negligible but we have biomes, give the first one full weight.
            sorted_weights[0].1 = 1.0;
        } else {
            // Absolute fallback if no biomes were found at all.
            return vec![(0, 1.0)];
        }

        sorted_weights
    }

    /// Get a biome definition by ID.
    pub fn get_biome_definition(&self, biome_id: u8) -> Option<&BiomeDefinition> {
        self.biomes.iter().find(|b| b.id == biome_id)
    }
}

impl fmt::Debug for ThreadSafeSectionData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ThreadSafeSectionData")
            .field("sections_count", &self.sections.len())
            .field("biomes_count", &self.biomes.len())
            .field("world_length", &self.world_length)
            .field("seed", &self.seed)
            .field("section_blend_distance", &self.section_blend_distance)
            .field("blend_noise_strength", &self.blend_noise_strength)
            .finish()
    }
}

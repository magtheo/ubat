// src/section/thread_safe_data.rs
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::collections::HashMap;

use crate::terrain::section::definition::{SectionDefinition, BiomeDefinition, VoronoiPoint};
use crate::terrain::section::distribution::SpatialGrid;
use crate::terrain::section::manager::SectionManager;
use crate::terrain::section::layout::calculate_section_weights;
use crate::terrain::noise::noise_manager::NoiseManager;
use crate::terrain::chunk_manager::ChunkResult;
use noise::NoiseFn;
use crate::terrain::terrain_config::TerrainConfigManager; // To get runtimeconfig

use std::fmt;

/// Thread-safe data container for section and biome information.
/// This structure can be safely shared between threads for terrain generation.
#[derive(Clone)]
pub struct ThreadSafeSectionData {
    pub sections: Arc<Vec<SectionDefinition>>,
    pub biomes: Arc<Vec<BiomeDefinition>>,
    pub points: Arc<Vec<VoronoiPoint>>,
    pub grid: Option<Arc<SpatialGrid>>,
    
    pub world_length: f32,
    pub seed: u64,
    
    pub biome_blend_noise_fn: Option<Arc<dyn NoiseFn<f64, 2> + Send + Sync>>,
    pub biome_blend_distance: f32,
    pub section_blend_distance: f32,
    pub blend_noise_strength: f32,
}

impl ThreadSafeSectionData {
    /// Create a new ThreadSafeSectionData from a SectionManager.
    pub fn from_section_manager(manager: &SectionManager, noise_manager: &NoiseManager) -> Self {
        // Get blend noise if available
        let biome_blend_noise = noise_manager.get_noise_function("biome_blend");
        
        let grid_arc = if let Some(grid) = manager.get_spatial_grid_internal() {
            Some(Arc::new(grid.clone()))
        } else {
            None
        };
        let blend_noise_strength = if let Ok(guard) = TerrainConfigManager::get_config().read() {
            guard.blend_noise_strength
        } else {
            eprint!("Failed to read terrain config for blend_noise_strength. Using default 0.25");
            0.25f32 // Default value if lock fails
        };
        
        Self {
            sections: Arc::new(manager.get_sections_internal().clone()),
            biomes: Arc::new(manager.get_biomes_internal().clone()),
            points: Arc::new(manager.get_voronoi_points_internal().clone()),
            grid: grid_arc,
            
            world_length: manager.get_world_length(),
            seed: manager.get_world_seed(),
            
            biome_blend_noise_fn: biome_blend_noise,
            biome_blend_distance: manager.get_biome_blend_distance(),
            section_blend_distance: manager.get_section_blend_distance(),
            blend_noise_strength,
        }
    }    
    
    /// Get the sections and biomes that influence a position, along with their weights.
    /// Implements REQ-BD-07 for blending across section boundaries and between Voronoi points.
    pub fn get_section_and_biome_weights(
        &self,
        world_x: f32,
        world_z: f32,
        sender: &Sender<ChunkResult> // Keep sender for logging
    ) -> Vec<(u8, f32)> {

        // --- Basic Logging ---
        let log_coord = format!("DEBUG get_weights (Falloff) at (X:{:.2}, Z:{:.2})", world_x, world_z);
        let _ = sender.send(ChunkResult::LogMessage(log_coord));
        // ---

        // --- Boundary checks remain the same ---
        if world_z < 0.0 && !self.sections.is_empty() { /* ... return first biome ... */ }
        if world_z >= self.world_length && !self.sections.is_empty() { /* ... return last biome ... */ }
        // ---

        // Step 1: Calculate section weights (no change here)
        let section_weights = calculate_section_weights(world_z, world_x, &self.sections);
        let log_sec_weights = format!("  SectionWeights: {:?}", section_weights);
        let _ = sender.send(ChunkResult::LogMessage(log_sec_weights));
        if section_weights.is_empty() { return vec![(0, 1.0)]; }

        // Step 2: Initialize final biome weights (using HashMap)
        let mut final_biome_weights = HashMap::new();

        // Step 3: Check grid/points availability (no change here)
        if self.grid.is_none() || self.points.is_empty() {
            let log_no_grid = format!("  WARNING: No grid or points available, using section fallback.");
            let _ = sender.send(ChunkResult::LogMessage(log_no_grid));
            // ... (existing fallback logic using first biome of section) ...
            for (section_id, section_weight) in &section_weights {
                if let Some(section) = self.sections.iter().find(|s| s.id == *section_id) {
                    if let Some(&biome_id) = section.possible_biomes.first() {
                        *final_biome_weights.entry(biome_id).or_insert(0.0) += section_weight;
                    }
                }
            }
            let mut result: Vec<(u8, f32)> = final_biome_weights.into_iter().collect();
            result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            return result;
        }

        // Step 4: For each weighted section, find *all* points within radius and calculate falloff weights
        for (section_id, section_weight) in &section_weights {
            if *section_weight < 0.01 { continue; } // Skip negligible sections

            let log_proc_sec = format!("  Processing SectionID: {}, Weight: {:.3}", section_id, section_weight);
            let _ = sender.send(ChunkResult::LogMessage(log_proc_sec));

            // We know grid is Some from check above
            if let Some(grid) = &self.grid {
                // --- MODIFICATION: Call new query function ---
                let points_in_radius = grid.find_points_within_radius(
                    world_x,
                    world_z,
                    &self.points,
                    self.biome_blend_distance, // Use blend distance as radius
                    Some(*section_id)          // Keep filtering by section
                );
                // ---

                // Log points found
                let points_details: Vec<(usize, u8, f32)> = points_in_radius.iter().map(|&(idx, dist)| {
                    let biome_id = self.points.get(idx).map_or(255, |p| p.biome_id);
                    (idx, biome_id, dist)
                }).collect();
                let log_near_pts = format!("    Points within radius {:.1} (Section {} Filter): {} points found: {:?}",
                                           self.biome_blend_distance, section_id, points_details.len(), points_details);
                let _ = sender.send(ChunkResult::LogMessage(log_near_pts));

                // --- NEW BLENDING LOGIC ---
                if points_in_radius.is_empty() {
                    // Fallback if no points found even within radius
                    let log_fallback = format!("    FALLBACK: No points found within radius for section {}. Using default biome.", section_id);
                    let _ = sender.send(ChunkResult::LogMessage(log_fallback));
                    if let Some(section) = self.sections.iter().find(|s| s.id == *section_id) {
                        if let Some(&biome_id) = section.possible_biomes.first() {
                            *final_biome_weights.entry(biome_id).or_insert(0.0) += section_weight;
                        }
                    }
                    continue; // Next section
                }

                // Calculate falloff weights for all found points
                let mut falloff_contributions = Vec::new(); // Store (biome_id, falloff_weight)
                let mut total_falloff_weight: f32 = 0.0;
                let blend_dist_sq = self.biome_blend_distance * self.biome_blend_distance; // Avoid repeated calc

                for &(idx, dist) in &points_in_radius {
                    if idx >= self.points.len() { continue; } // Safety check

                    let biome_id = self.points[idx].biome_id;
                    let t = (dist / self.biome_blend_distance).clamp(0.0, 1.0); // Normalized distance

                    // Smoothstep falloff: weight = 1 at dist=0, 0 at dist=blend_distance
                    let falloff = 1.0 - (t * t * (3.0 - 2.0 * t));

                    // --- Optional: Log individual falloff weights ---
                    // let log_falloff = format!("      PointIdx:{}, Biome:{}, Dist:{:.2}, t:{:.2}, Falloff:{:.3}", idx, biome_id, dist, t, falloff);
                    // let _ = sender.send(ChunkResult::LogMessage(log_falloff));
                    // ---

                    if falloff > 1e-4 { // Only consider non-negligible weights
                        falloff_contributions.push((biome_id, falloff));
                        total_falloff_weight += falloff;
                    }
                }

                // Normalize falloff weights and apply section weight
                if total_falloff_weight > 1e-6 {
                    let log_total_falloff = format!("    Total falloff weight for section {}: {:.3}", section_id, total_falloff_weight);
                    let _ = sender.send(ChunkResult::LogMessage(log_total_falloff));

                    for (biome_id, falloff) in falloff_contributions {
                        let intra_weight = falloff / total_falloff_weight; // Normalize
                        let weighted_contribution = section_weight * intra_weight;

                        let log_contribution = format!(
                            "    Biome {} Contribution: {:.4} (SectionWeight {:.3} * NormFalloff {:.3} [Raw: {:.3}])",
                             biome_id, weighted_contribution, section_weight, intra_weight, falloff
                        );
                        let _ = sender.send(ChunkResult::LogMessage(log_contribution));

                       *final_biome_weights.entry(biome_id).or_insert(0.0) += weighted_contribution;
                    }
                } else {
                    // Handle case where total falloff is zero (e.g., all points exactly at blend distance)
                     let log_zero_falloff = format!("    WARNING: Total falloff weight is zero for section {}. Using closest point.", section_id);
                     let _ = sender.send(ChunkResult::LogMessage(log_zero_falloff));
                     // Fallback: use the single closest point found
                     if let Some(&(closest_idx, _)) = points_in_radius.first() {
                          if closest_idx < self.points.len() {
                              let biome_id = self.points[closest_idx].biome_id;
                              *final_biome_weights.entry(biome_id).or_insert(0.0) += section_weight; // Full section weight to closest
                          }
                     }
                }
                // --- END NEW BLENDING LOGIC ---
            }
            // Grid was None case handled before the loop
        } // End loop over section_weights


        // Step 5: Final Processing (Convert HashMap, Sort, Normalize if needed)
        let mut result: Vec<(u8, f32)> = final_biome_weights
            .into_iter()
            .filter(|&(_, w)| w > 1e-4) // Filter negligible weights
            .collect();

        if result.is_empty() {
             let log_empty_final = format!("  WARNING: All final biome weights were negligible. Defaulting to biome 0.");
             let _ = sender.send(ChunkResult::LogMessage(log_empty_final));
             return vec![(0, 1.0)];
        }

        result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Optional: Final normalization if sum is still off (less likely now)
        let sum: f32 = result.iter().map(|&(_, w)| w).sum();
         if sum < 1e-6 {
             let log_zero_sum = format!("  WARNING: Final weight sum is near zero ({:.4}). Defaulting to first biome.", sum);
             let _ = sender.send(ChunkResult::LogMessage(log_zero_sum));
             return vec![(result[0].0, 1.0)];
         } else if (sum - 1.0).abs() > 0.01 {
             let log_norm = format!("  Normalizing final weights (Sum: {:.3}). Original: {:?}", sum, result);
             let _ = sender.send(ChunkResult::LogMessage(log_norm));
             for entry in result.iter_mut() { entry.1 /= sum; }
             let log_norm_res = format!("    Normalized Result: {:?}", result);
             let _ = sender.send(ChunkResult::LogMessage(log_norm_res));
        }

        result
    } // --- End of get_section_and_biome_weights ---

    
    /// Get biome weights at a specific position (used by REQ-TG-01).
    /// This is a convenience wrapper around get_section_and_biome_weights.
    pub fn get_biome_id_and_weights(&self, world_x: f32, world_z: f32, sender: &Sender<ChunkResult>) -> Vec<(u8, f32)> {
        self.get_section_and_biome_weights(world_x, world_z, sender)
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
            .field("points_count", &self.points.len())
            .field("has_grid", &self.grid.is_some())
            .field("world_length", &self.world_length)
            .field("seed", &self.seed)
            .field("has_biome_blend_noise", &self.biome_blend_noise_fn.is_some())
            .field("biome_blend_distance", &self.biome_blend_distance)
            .field("section_blend_distance", &self.section_blend_distance)
            .field("blend_noise_strength", &self.blend_noise_strength)
            .finish()
    }
}

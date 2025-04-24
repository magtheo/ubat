// src/section/thread_safe_data.rs
use std::sync::Arc;

use crate::terrain::section::definition::{SectionDefinition, BiomeDefinition, VoronoiPoint};
use crate::terrain::section::distribution::SpatialGrid;
use crate::terrain::section::manager::SectionManager;
use crate::terrain::section::layout::calculate_section_weights;
use crate::terrain::noise::noise_manager::NoiseManager;
use noise::NoiseFn;

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
        }
    }    
    
    /// Get the sections and biomes that influence a position, along with their weights.
    /// Implements REQ-BD-07 for blending across section boundaries and between Voronoi points.
    pub fn get_section_and_biome_weights(&self, world_x: f32, world_z: f32) -> Vec<(u8, f32)> {
        // Boundary checks
        if world_z < 0.0 && !self.sections.is_empty() {
            // Before world start - use only first section's first biome
            if let Some(first_biome) = self.sections[0].possible_biomes.first() {
                return vec![(*first_biome, 1.0)];
            }
            return vec![(0, 1.0)]; // Fallback
        }
        
        if world_z >= self.world_length && !self.sections.is_empty() {
            // After world end - use only last section's first biome
            let last_section = &self.sections[self.sections.len() - 1];
            if let Some(last_biome) = last_section.possible_biomes.first() {
                return vec![(*last_biome, 1.0)];
            }
            return vec![(0, 1.0)]; // Fallback
        }
        
        // Step 1: Calculate section weights
        let section_weights = calculate_section_weights(world_z, world_x, &self.sections);
        
        // If no valid sections, return default biome
        if section_weights.is_empty() {
            return vec![(0, 1.0)];
        }
        
        // Step 2: Initialize final biome weights
        let mut final_biome_weights = std::collections::HashMap::new();
        
        // Early return for missing/empty grid
        if self.grid.is_none() || self.points.is_empty() {
            // Just use the first biome from each section
            for (section_id, section_weight) in &section_weights {
                // Find section definition
                if let Some(section) = self.sections.iter().find(|s| s.id == *section_id) {
                    if let Some(&biome_id) = section.possible_biomes.first() {
                        final_biome_weights.insert(biome_id, *section_weight);
                    }
                }
            }
            
            return final_biome_weights
                .iter()
                .map(|(&id, &weight)| (id, weight))
                .collect();
        }
        
        // Step 3: For each weighted section, find nearest Voronoi points and calculate biome weights
        for (section_id, section_weight) in &section_weights {
            // Skip if section weight is negligible
            if *section_weight < 0.01 {
                continue;
            }
            
            // Find the nearest 2 points belonging to this section
            if let Some(grid) = &self.grid {
                let nearest_points = grid.find_k_nearest_points(
                    world_x,
                    world_z,
                    &self.points,
                    2, // Find 2 nearest points
                    self.biome_blend_distance,
                    Some(*section_id) // Filter by section ID
                );
                
                if nearest_points.is_empty() {
                    // Fallback: No points found for this section
                    // Use the first biome in the section's possible_biomes list
                    if let Some(section) = self.sections.iter().find(|s| s.id == *section_id) {
                        if let Some(&biome_id) = section.possible_biomes.first() {
                            *final_biome_weights.entry(biome_id).or_insert(0.0) += section_weight;
                        }
                    }
                    continue;
                }
                
                // Calculate weights between the nearest points
                let mut intra_section_weights = Vec::new();
                
                if nearest_points.len() == 1 {
                    // Only one point, full weight
                    let (idx, _) = nearest_points[0];
                    let biome_id = self.points[idx].biome_id;
                    intra_section_weights.push((biome_id, 1.0));
                } else {
                    // Two or more points, calculate weights based on distance
                    let (idx1, dist1) = nearest_points[0];
                    let (idx2, dist2) = nearest_points[1];
                    
                    let biome_id1 = self.points[idx1].biome_id;
                    let biome_id2 = self.points[idx2].biome_id;
                    
                    // Apply optional noise perturbation to the blend
                    let mut blend_factor = dist1 / (dist1 + dist2);
                    
                    if let Some(noise_fn) = &self.biome_blend_noise_fn {
                        // Get noise value in range [-1, 1]
                        let noise_value = noise_fn.get([world_x as f64, world_z as f64]) as f32;
                        // Scale by a factor (e.g., 0.3) to control noise influence
                        let noise_scale = 0.3;
                        blend_factor = (blend_factor + noise_value * noise_scale).clamp(0.0, 1.0);
                    }
                    
                    // Smoothstep for nicer blending
                    let t = blend_factor;
                    let smoothed = t * t * (3.0 - 2.0 * t);
                    
                    intra_section_weights.push((biome_id2, smoothed));
                    intra_section_weights.push((biome_id1, 1.0 - smoothed));
                }
                
                // Apply section weight to intra-section biome weights
                for (biome_id, intra_weight) in intra_section_weights {
                    *final_biome_weights.entry(biome_id).or_insert(0.0) += section_weight * intra_weight;
                }
            }
        }
        
        // Step 4: Convert HashMap to Vec and normalize if needed
        let mut result: Vec<(u8, f32)> = final_biome_weights
            .iter()
            .map(|(&id, &weight)| (id, weight))
            .collect();
        
        // Sort by weight (highest first) for consistent results
        result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        // Normalize weights if they don't sum to ~1.0
        let sum: f32 = result.iter().map(|&(_, w)| w).sum();
        if sum > 0.0 && (sum < 0.99 || sum > 1.01) {
            result = result.iter().map(|&(id, w)| (id, w / sum)).collect();
        }
        
        result
    }
    
    /// Get biome weights at a specific position (used by REQ-TG-01).
    /// This is a convenience wrapper around get_section_and_biome_weights.
    pub fn get_biome_id_and_weights(&self, world_x: f32, world_z: f32) -> Vec<(u8, f32)> {
        self.get_section_and_biome_weights(world_x, world_z)
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
            .finish()
    }
}

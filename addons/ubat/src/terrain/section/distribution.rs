// src/section/distribution.rs
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use std::collections::HashMap;

use crate::terrain::section::definition::{SectionDefinition, VoronoiPoint, Rect2};

/// Generate Voronoi points for a section within the specified bounds.
///
/// # Arguments
///
/// * `section_def` - The section definition containing configuration
/// * `section_bounds` - The rectangular area to generate points in
/// * `rng_seed` - Seed for the random number generator
///
/// # Returns
///
/// A vector of VoronoiPoint structures with assigned biome IDs
pub fn generate_voronoi_points_for_section(
    section_def: &SectionDefinition,
    section_bounds: Rect2,
    rng_seed: u64
) -> Vec<VoronoiPoint> {
    let mut points = Vec::new();
    
    // Create a deterministic RNG based on section ID and provided seed
    let combined_seed = rng_seed.wrapping_add((section_def.id as u64) << 32);
    let mut rng = StdRng::seed_from_u64(combined_seed);
    
    // Calculate number of points based on area and density
    let area = section_bounds.width * section_bounds.height;
    let num_points = (area * section_def.point_density).ceil() as usize;
    
    // Bail early if no biomes available or point density is zero
    if section_def.possible_biomes.is_empty() || num_points == 0 {
        return points;
    }
    
    // Generate random points
    for _ in 0..num_points {
        // Random position within bounds
        let x = section_bounds.x + rng.r#gen::<f32>() * section_bounds.width;
        let z = section_bounds.z + rng.r#gen::<f32>() * section_bounds.height;
        
        // Pick a random biome from the possible ones
        let biome_idx = rng.gen_range(0..section_def.possible_biomes.len());
        let biome_id = section_def.possible_biomes[biome_idx];
        
        // Create and add the point
        points.push(VoronoiPoint {
            position: (x, z),
            biome_id,
            section_id: section_def.id,
        });
    }
    
    points
}

/// A spatial grid for optimizing proximity queries.
/// Divides the world into cells and stores which points are in each cell.
pub struct SpatialGrid {
    pub cell_size: f32,
    pub grid_width: usize,
    pub grid_height: usize,
    pub origin_x: f32,
    pub origin_z: f32,
    
    // Maps each cell to the indices of points contained within it
    // (grid_x, grid_z) -> [point_indices]
    grid_cells: HashMap<(usize, usize), Vec<usize>>,
}

impl SpatialGrid {
    /// Create a new SpatialGrid for the given bounds and points.
    pub fn new(bounds: Rect2, points: &[VoronoiPoint], cell_size: f32) -> Self {
        let grid_width = (bounds.width / cell_size).ceil() as usize;
        let grid_height = (bounds.height / cell_size).ceil() as usize;
        
        let mut grid = Self {
            cell_size,
            grid_width,
            grid_height,
            origin_x: bounds.x,
            origin_z: bounds.z,
            grid_cells: HashMap::new(),
        };
        
        // Add all points to the grid
        for (idx, point) in points.iter().enumerate() {
            grid.add_point(idx, point);
        }
        
        grid
    }
    
    /// Add a point to the grid.
    fn add_point(&mut self, point_idx: usize, point: &VoronoiPoint) {
        let (x, z) = point.position;
        let cell_x = ((x - self.origin_x) / self.cell_size).floor() as usize;
        let cell_z = ((z - self.origin_z) / self.cell_size).floor() as usize;
        
        // Ensure within bounds
        if cell_x < self.grid_width && cell_z < self.grid_height {
            self.grid_cells.entry((cell_x, cell_z))
                .or_insert_with(Vec::new)
                .push(point_idx);
        }
    }
    
    /// Get indices of points potentially near the specified coordinates.
    pub fn get_nearby_point_indices(&self, x: f32, z: f32) -> Vec<usize> {
        let mut result = Vec::new();
        
        // Determine which cell contains the query point
        let cell_x = ((x - self.origin_x) / self.cell_size).floor() as isize;
        let cell_z = ((z - self.origin_z) / self.cell_size).floor() as isize;
        
        // Check the containing cell and all 8 neighbors
        for dz in -1..=1 {
            for dx in -1..=1 {
                let nx = cell_x + dx;
                let nz = cell_z + dz;
                
                // Skip if out of bounds
                if nx < 0 || nz < 0 || nx as usize >= self.grid_width || nz as usize >= self.grid_height {
                    continue;
                }
                
                // Add points from this cell
                if let Some(indices) = self.grid_cells.get(&(nx as usize, nz as usize)) {
                    result.extend(indices);
                }
            }
        }
        
        result
    }
    
    /// Find the closest points to the specified coordinates, optionally filtered by section.
    /// Returns up to k closest points.
    pub fn find_k_nearest_points(
        &self,
        x: f32, 
        z: f32, 
        points: &[VoronoiPoint], 
        k: usize,
        max_distance: f32,
        section_filter: Option<u8>
    ) -> Vec<(usize, f32)> {
        let nearby_indices = self.get_nearby_point_indices(x, z);
        
        // Calculate distances to nearby points
        let mut distances = Vec::new();
        for &idx in &nearby_indices {
            let point = &points[idx];
            
            // Skip if not in the requested section
            if let Some(section_id) = section_filter {
                if point.section_id != section_id {
                    continue;
                }
            }
            
            let (px, pz) = point.position;
            let dx = px - x;
            let dz = pz - z;
            let dist_sq = dx * dx + dz * dz;
            
            // Filter by max distance (squared)
            if dist_sq <= max_distance * max_distance {
                distances.push((idx, dist_sq.sqrt()));
            }
        }
        
        // Sort by distance (closest first)
        distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        
        // Return at most k points
        distances.truncate(k);
        distances
    }
}

impl Clone for SpatialGrid {
    fn clone(&self) -> Self {
        Self {
            cell_size: self.cell_size,
            grid_width: self.grid_width,
            grid_height: self.grid_height,
            origin_x: self.origin_x,
            origin_z: self.origin_z,
            grid_cells: self.grid_cells.clone(),
        }
    }
}

impl std::fmt::Debug for SpatialGrid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpatialGrid")
            .field("cell_size", &self.cell_size)
            .field("grid_width", &self.grid_width)
            .field("grid_height", &self.grid_height)
            .field("origin_x", &self.origin_x)
            .field("origin_z", &self.origin_z)
            .field("cell_count", &self.grid_cells.len())
            .finish()
    }
}
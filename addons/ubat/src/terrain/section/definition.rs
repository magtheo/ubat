// src/section/definition.rs
use std::sync::Arc;
use noise::NoiseFn;

/// Runtime representation of a section.
/// Processed from SectionTomlConfig with calculated positions.
pub struct SectionDefinition {
    /// Unique identifier for the section
    pub id: u8,
    
    /// Start position of this section along the world Z-axis
    pub start_position: f32,
    
    /// End position of this section along the world Z-axis
    pub end_position: f32,
    
    /// Start of the transition zone to the next section
    pub transition_start: f32,
    
    /// End of the transition zone (usually equals end_position)
    pub transition_end: f32,
    
    /// List of biome IDs that can appear in this section
    pub possible_biomes: Vec<u8>,
    
    /// Density of Voronoi points (points per unit area)
    pub point_density: f32,
    
    /// Optional noise function for boundary perturbation
    pub boundary_noise_fn: Option<Arc<dyn NoiseFn<f64, 2> + Send + Sync>>,
}

/// Runtime representation of a biome.
/// Processed from BiomeTomlConfig with initialized noise functions.
pub struct BiomeDefinition {
    /// Unique identifier for the biome
    pub id: u8,
    
    /// Descriptive name of the biome
    pub name: String,
    
    /// Primary noise function used for heightmap generation
    pub primary_noise_fn: Arc<dyn NoiseFn<f64, 2> + Send + Sync>,
    
    /// Additional texture and visual parameters for this biome
    pub texture_params: std::collections::HashMap<String, f32>,
    
    /// Optional secondary noise functions
    pub secondary_noise_fns: Vec<Arc<dyn NoiseFn<f64, 2> + Send + Sync>>,
}

/// Represents a point in the Voronoi diagram with an assigned biome ID.
#[derive(Clone)]
pub struct VoronoiPoint {
    /// World position of the point (x, z)
    pub position: (f32, f32),
    
    /// The biome ID assigned to this point
    pub biome_id: u8,
    
    /// The section ID this point belongs to
    pub section_id: u8,
}

/// Simple 2D rectangle for defining section areas.
#[derive(Clone, Copy, Debug)]
pub struct Rect2 {
    pub x: f32,
    pub z: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect2 {
    pub fn new(x: f32, z: f32, width: f32, height: f32) -> Self {
        Self { x, z, width, height }
    }
    
    pub fn contains(&self, x: f32, z: f32) -> bool {
        x >= self.x && x < (self.x + self.width) && 
        z >= self.z && z < (self.z + self.height)
    }
}

impl SectionDefinition {
    pub fn new(
        id: u8,
        start_position: f32,
        length: f32,
        transition_zone: f32,
        possible_biomes: Vec<u8>,
        point_density: f32,
        boundary_noise_fn: Option<Arc<dyn NoiseFn<f64, 2> + Send + Sync>>,
    ) -> Self {
        let end_position = start_position + length;
        let transition_start = end_position - transition_zone;
        
        Self {
            id,
            start_position,
            end_position,
            transition_start,
            transition_end: end_position,
            possible_biomes,
            point_density,
            boundary_noise_fn,
        }
    }
    
    /// Check if a given Z coordinate falls within this section.
    pub fn contains_z(&self, z: f32) -> bool {
        z >= self.start_position && z < self.end_position
    }
    
    /// Check if a given Z coordinate falls within this section's transition zone.
    pub fn in_transition_zone(&self, z: f32) -> bool {
        z >= self.transition_start && z < self.transition_end
    }
}

impl Clone for SectionDefinition {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            start_position: self.start_position,
            end_position: self.end_position,
            transition_start: self.transition_start,
            transition_end: self.transition_end,
            possible_biomes: self.possible_biomes.clone(),
            point_density: self.point_density,
            boundary_noise_fn: self.boundary_noise_fn.clone(),
        }
    }
}

impl Clone for BiomeDefinition {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            name: self.name.clone(),
            primary_noise_fn: self.primary_noise_fn.clone(),
            texture_params: self.texture_params.clone(),
            secondary_noise_fns: self.secondary_noise_fns.clone(),
        }
    }
}


impl std::fmt::Debug for SectionDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SectionDefinition")
            .field("id", &self.id)
            .field("start_position", &self.start_position)
            .field("end_position", &self.end_position)
            .field("transition_start", &self.transition_start)
            .field("transition_end", &self.transition_end)
            .field("possible_biomes", &self.possible_biomes)
            .field("point_density", &self.point_density)
            .field("has_boundary_noise", &self.boundary_noise_fn.is_some())
            .finish()
    }
}

// For BiomeDefinition
impl std::fmt::Debug for BiomeDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BiomeDefinition")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("has_primary_noise", &true) // We can't easily debug the noise function
            .field("texture_params", &self.texture_params)
            .field("secondary_noise_count", &self.secondary_noise_fns.len())
            .finish()
    }
}

// For VoronoiPoint - already has Clone, add Debug derivation
impl std::fmt::Debug for VoronoiPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VoronoiPoint")
            .field("position", &self.position)
            .field("biome_id", &self.biome_id)
            .field("section_id", &self.section_id)
            .finish()
    }
}
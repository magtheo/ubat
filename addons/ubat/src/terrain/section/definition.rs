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
    
    /// Optional noise function for blending biomes within this section
    pub blend_noise_fn: Option<Arc<dyn NoiseFn<f64, 2> + Send + Sync>>,
    
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

impl SectionDefinition {
    pub fn new(
        id: u8,
        start_position: f32,
        length: f32,
        transition_zone: f32,
        possible_biomes: Vec<u8>,
        blend_noise_fn: Option<Arc<dyn NoiseFn<f64, 2> + Send + Sync>>,
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
            blend_noise_fn,
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
            blend_noise_fn: self.blend_noise_fn.clone(),
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
            .field("has_blend_noise", &self.blend_noise_fn.is_some())
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
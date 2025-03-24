// In src/terrain/mod.rs or src/terrain/generation_rules.rs
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRules {
    // Core terrain parameters
    pub terrain_octaves: u32,
    pub terrain_scale: f32,
    pub terrain_persistence: f32,
    pub terrain_lacunarity: f32,
    
    // Biome parameters
    pub biome_blend_distance: f32,
    pub biome_transition_noise: f32,
    
    // Feature parameters (rivers, mountains, etc.)
    pub feature_density: f32,
    pub mountain_threshold: f32,
    pub river_width: f32,
    
    // Vegetation and decoration
    pub tree_density: f32,
    pub grass_coverage: f32,
}

impl Default for GenerationRules {
    fn default() -> Self {
        Self {
            terrain_octaves: 6,
            terrain_scale: 100.0,
            terrain_persistence: 0.5,
            terrain_lacunarity: 2.0,
            biome_blend_distance: 100.0,
            biome_transition_noise: 0.2,
            feature_density: 0.1,
            mountain_threshold: 0.7,
            river_width: 5.0,
            tree_density: 0.2,
            grass_coverage: 0.5,
        }
    }
}

pub use self::generation_rules::GenerationRules;
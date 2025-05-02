// src/section/config.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for a section loaded from TOML
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SectionTomlConfig {
    /// Unique identifier for the section
    pub id: u8,
    
    /// Length of the section along the Z axis
    pub length: f32,
    
    /// Size of the transition zone with the next section
    pub transition_zone: f32,
    
    /// List of biome IDs that can appear in this section
    pub possible_biomes: Vec<u8>,
    
    /// Density of Voronoi points to generate (points per unit area)
    pub point_density: f32,
    
    /// Optional noise key for boundary perturbation
    #[serde(default)]
    pub boundary_noise_key: Option<String>,
}

/// Configuration for a biome loaded from TOML
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BiomeTomlConfig {
    /// Unique identifier for the biome
    pub id: u8,
    
    /// Descriptive name of the biome
    pub name: String,
    
    /// Key for the primary noise function used for heightmap generation
    pub primary_noise_key: String,
    
    /// Additional texture and visual parameters for this biome
    #[serde(default)]
    pub texture_params: HashMap<String, f32>,
    
    /// Optional secondary noise functions
    #[serde(default)]
    pub secondary_noise_keys: Vec<String>,
}

impl Default for SectionTomlConfig {
    fn default() -> Self {
        Self {
            id: 0,
            length: 1000.0,
            transition_zone: 100.0,
            possible_biomes: vec![0], // Default biome
            point_density: 0.0001,
            boundary_noise_key: None,
        }
    }
}

impl Default for BiomeTomlConfig {
    fn default() -> Self {
        Self {
            id: 0,
            name: "Default".to_string(),
            primary_noise_key: "default".to_string(),
            texture_params: HashMap::new(),
            secondary_noise_keys: Vec::new(),
        }
    }
}
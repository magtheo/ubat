use godot::prelude::{Dictionary, GString};
use serde::{Serialize, Deserialize};

/// Comprehensive terrain generation rules with detailed configuration options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRules {
    // Core terrain generation parameters
    /// Number of noise octaves for terrain generation
    /// Higher values create more detailed terrain
    /// Recommended range: 1-10
    pub terrain_octaves: f32,
    
    /// Scale of terrain features
    /// Larger values create smoother, more gradual terrain
    /// Recommended range: 10.0 - 1000.0
    pub terrain_scale: f32,
    
    /// Persistence controls the amplitude of each octave
    /// Lower values create smoother terrain, higher values create more rugged terrain
    /// Recommended range: 0.1 - 1.0
    pub terrain_persistence: f32,
    
    /// Lacunarity controls the frequency increase between octaves
    /// Higher values create more detailed terrain with more high-frequency details
    /// Recommended range: 1.5 - 3.0
    pub terrain_lacunarity: f32,
    
    // Biome generation parameters
    /// Distance over which biomes blend into each other
    /// Larger values create smoother biome transitions
    /// Recommended range: 50.0 - 500.0
    pub biome_blend_distance: f32,
    
    /// Noise factor for biome transitions
    /// Controls the randomness of biome border generation
    /// Recommended range: 0.0 - 1.0
    pub biome_transition_noise: f32,
    
    // World feature parameters
    /// Density of world features like mountains, rivers, etc.
    /// Higher values increase feature complexity
    /// Recommended range: 0.0 - 1.0
    pub feature_density: f32,
    
    /// Threshold for mountain generation
    /// Higher values create more mountainous terrain
    /// Recommended range: 0.5 - 0.9
    pub mountain_threshold: f32,
    
    /// Width of river features
    /// Recommended range: 1.0 - 20.0
    pub river_width: f32,
    
    // Vegetation parameters
    /// Density of trees and vegetation
    /// Recommended range: 0.0 - 1.0
    pub tree_density: f32,
    
    /// Coverage of ground vegetation like grass
    /// Recommended range: 0.0 - 1.0
    pub grass_coverage: f32,
}

impl Default for GenerationRules {
    /// Provide sensible default terrain generation rules
    /// Optimized for a balanced, naturally looking terrain
    fn default() -> Self {
        Self {
            terrain_octaves: 6.0,
            terrain_scale: 250.0,
            terrain_persistence: 0.5,
            terrain_lacunarity: 2.0,
            
            biome_blend_distance: 200.0,
            biome_transition_noise: 0.3,
            
            feature_density: 0.2,
            mountain_threshold: 0.7,
            river_width: 10.0,
            
            tree_density: 0.3,
            grass_coverage: 0.6,
        }
    }
}

impl GenerationRules {
    /// Validate and correct generation rules
    /// 
    /// Returns a vector of warning messages for any corrected parameters
    pub fn validate_and_fix(&mut self) -> Vec<GString> {
        let mut warnings = Vec::new();
        
        // Validate terrain generation parameters
        if self.terrain_octaves < 1.0 {
            warnings.push("Terrain octaves set to minimum value of 1".into());
            self.terrain_octaves = 1.0;
        } else if self.terrain_octaves > 10.0 {
            warnings.push("Terrain octaves capped at maximum value of 10".into());
            self.terrain_octaves = 10.0;
        }
        
        // Scale validation
        if self.terrain_scale <= 0.0 {
            warnings.push("Terrain scale must be positive. Set to default 250.0".into());
            self.terrain_scale = 250.0;
        }
        
        // Persistence and lacunarity
        self.terrain_persistence = self.terrain_persistence.clamp(0.1, 1.0);
        self.terrain_lacunarity = self.terrain_lacunarity.clamp(1.5, 3.0);
        
        // Biome blend distance
        if self.biome_blend_distance <= 0.0 {
            warnings.push("Biome blend distance must be positive. Set to default 200.0".into());
            self.biome_blend_distance = 200.0;
        }
        
        // Biome transition noise
        self.biome_transition_noise = self.biome_transition_noise.clamp(0.0, 1.0);
        
        // Feature parameters
        self.feature_density = self.feature_density.clamp(0.0, 1.0);
        self.mountain_threshold = self.mountain_threshold.clamp(0.5, 0.9);
        
        if self.river_width <= 0.0 {
            warnings.push("River width must be positive. Set to default 10.0".into());
            self.river_width = 10.0;
        }
        
        // Vegetation parameters
        self.tree_density = self.tree_density.clamp(0.0, 1.0);
        self.grass_coverage = self.grass_coverage.clamp(0.0, 1.0);
        
        warnings
    }
    
    /// Create terrain rules preset for mountainous terrain
    pub fn mountainous_preset() -> Self {
        let mut rules = Self::default();
        rules.terrain_octaves = 8.0;
        rules.terrain_scale = 500.0;
        rules.mountain_threshold = 0.85;
        rules.feature_density = 0.5;
        rules.validate_and_fix();
        rules
    }
    
    /// Create terrain rules preset for flat terrain
    pub fn flat_preset() -> Self {
        let mut rules = Self::default();
        rules.terrain_octaves = 3.0;
        rules.terrain_scale = 1000.0;
        rules.mountain_threshold = 0.5;
        rules.feature_density = 0.1;
        rules.validate_and_fix();
        rules
    }
    
    /// Convert from Godot Dictionary to GenerationRules
    pub fn from_dictionary(dict: &Dictionary) -> Self {
        let mut rules = GenerationRules::default();
        
        // Macro to safely extract and set values
        macro_rules! set_from_dict {
            ($field:ident, $dict:expr) => {
                if let Some(variant) = $dict.get(stringify!($field)) {
                    if let Ok(value) = variant.try_to::<f32>() {
                        rules.$field = value;
                    }
                }
            }
        }
        
        // Extract values from dictionary
        set_from_dict!(terrain_octaves, dict);
        set_from_dict!(terrain_scale, dict);
        set_from_dict!(terrain_persistence, dict);
        set_from_dict!(terrain_lacunarity, dict);
        set_from_dict!(biome_blend_distance, dict);
        set_from_dict!(biome_transition_noise, dict);
        set_from_dict!(feature_density, dict);
        set_from_dict!(mountain_threshold, dict);
        set_from_dict!(river_width, dict);
        set_from_dict!(tree_density, dict);
        set_from_dict!(grass_coverage, dict);
        
        // Validate and fix the rules
        rules.validate_and_fix();
        
        rules
    }
}
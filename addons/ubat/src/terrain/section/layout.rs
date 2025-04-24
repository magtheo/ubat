// src/section/layout.rs
use crate::terrain::section::definition::SectionDefinition;
use noise::NoiseFn;

/// Calculate the influence weight of each section at a given world Z coordinate.
/// 
/// The function determines which sections affect the given point, based on:
/// 1. Linear progression (primary axis for section changes is Z)
/// 2. Transition zones between adjacent sections
/// 3. Optional boundary noise for natural, non-linear transitions
///
/// # Arguments
///
/// * `world_z` - The Z-coordinate to evaluate
/// * `sections` - Slice of all available SectionDefinition objects
/// * `world_x` - Optional X-coordinate for noise-based boundary perturbation
///
/// # Returns
///
/// A vector of (section_id, weight) tuples. Weights sum to approximately 1.0.
pub fn calculate_section_weights(
    world_z: f32, 
    world_x: f32,
    sections: &[SectionDefinition]
) -> Vec<(u8, f32)> {
    if sections.is_empty() {
        return vec![];
    }
    
    // Handle out-of-bounds cases
    if world_z < sections[0].start_position {
        return vec![(sections[0].id, 1.0)];
    }
    
    let last_section = &sections[sections.len() - 1];
    if world_z >= last_section.end_position {
        return vec![(last_section.id, 1.0)];
    }
    
    // Find which section(s) the point is in
    for (idx, section) in sections.iter().enumerate() {
        // Check if in main part (non-transition) of section
        if world_z >= section.start_position && world_z < section.transition_start {
            return vec![(section.id, 1.0)];
        }
        
        // Check if in transition zone
        if world_z >= section.transition_start && world_z < section.end_position {
            // Apply boundary noise if available
            let mut effective_z = world_z;
            if let Some(noise_fn) = &section.boundary_noise_fn {
                // Use noise to perturb the effective boundary
                let noise_value = noise_fn.get([world_x as f64, world_z as f64]) as f32;
                // Map noise from [-1,1] to a fraction of transition zone size
                let perturbation = noise_value * (section.transition_end - section.transition_start) * 0.5;
                effective_z = world_z + perturbation;
                
                // Re-check if still in transition after perturbation
                if effective_z < section.transition_start {
                    return vec![(section.id, 1.0)];
                }
                if effective_z >= section.end_position {
                    // Check for next section
                    if idx + 1 < sections.len() {
                        return vec![(sections[idx + 1].id, 1.0)];
                    }
                    return vec![(section.id, 1.0)]; // Fallback to current section
                }
            }
            
            // We're in the transition zone - calculate weights using smoothstep
            let t = (effective_z - section.transition_start) / 
                    (section.end_position - section.transition_start);
            
            // Use smoothstep for nice easing: 3t² - 2t³
            let smoothed_t = t * t * (3.0 - 2.0 * t);
            
            // Weight of current section decreases, next section increases
            let current_weight = 1.0 - smoothed_t;
            
            // Ensure next section exists
            if idx + 1 < sections.len() {
                return vec![
                    (section.id, current_weight),
                    (sections[idx + 1].id, smoothed_t)
                ];
            } else {
                // Edge case: no next section, keep full weight on current
                return vec![(section.id, 1.0)];
            }
        }
    }
    
    // Fallback: If we somehow didn't find a section, return first section
    vec![(sections[0].id, 1.0)]
}

/// Helper function to smooth transitions using a smoothstep function.
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
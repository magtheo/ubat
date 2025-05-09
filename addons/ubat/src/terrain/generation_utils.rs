// Example: src/terrain/generation_utils.rs
use crate::threading::chunk_storage::MeshGeometry; // Assuming MeshGeometry is here or pub

pub fn get_clamped_height(x: i32, z: i32, heightmap: &[f32], chunk_size: u32) -> f32 {
    // Your existing implementation...
    // Make sure it correctly handles boundaries
    let x_clamp = x.clamp(0, chunk_size as i32 - 1) as u32;
    let z_clamp = z.clamp(0, chunk_size as i32 - 1) as u32;
    let idx = (z_clamp * chunk_size + x_clamp) as usize;
    if idx < heightmap.len() {
        heightmap[idx]
    } else {
        0.0 // Fallback, though clamp should prevent this
    }
}

/// Applies a smooth transition function 
fn smooth_value(t: f32) -> f32 { // Renamed for clarity, t is assumed to be 0-1
    let clamped_t = t.clamp(0.0, 1.0);
    clamped_t * clamped_t * (3.0 - 2.0 * clamped_t)
}

pub fn generate_mesh_geometry(
    heightmap: &Vec<f32>,
    chunk_size: u32, // Number of quads per side
    biome_indices_data: &Vec<[u8; 3]>,
    biome_weights_data: &Vec<[f32; 3]>,
) -> MeshGeometry {
    if chunk_size == 0 {
        return MeshGeometry::default(); // Cannot generate mesh for size 0
    }

    let grid_width = chunk_size + 1; // Number of vertices per side
    let vertex_count = (grid_width * grid_width) as usize;
    let quad_count = (chunk_size * chunk_size) as usize;
    let expected_map_size = vertex_count;

    // Basic validation of input data sizes
    if heightmap.len() != expected_map_size
        || biome_indices_data.len() != expected_map_size
        || biome_weights_data.len() != expected_map_size
    {
        eprintln!(
            "Error generating mesh: Input data size mismatch! Expected {}, Heightmap: {}, Biomes: {}, Weights: {}",
            expected_map_size,
            heightmap.len(),
            biome_indices_data.len(),
            biome_weights_data.len()
        );
        return MeshGeometry::default(); // Return empty geometry on error
    }

    let mut geometry = MeshGeometry {
        vertices: Vec::with_capacity(vertex_count),
        normals: Vec::with_capacity(vertex_count),
        uvs: Vec::with_capacity(vertex_count),
        indices: Vec::with_capacity(quad_count * 6),
        custom0_biome_ids: Vec::with_capacity(vertex_count),
        custom1_biome_weights: Vec::with_capacity(vertex_count),
    };

    let chunk_size_f = chunk_size as f32;

    // --- First Pass: Generate Vertex Data (Position, UV, Custom, Normals) ---
    for iz in 0..grid_width {
        for ix in 0..grid_width {
            let current_index = (iz * grid_width + ix) as usize;

            // 1. Vertex Position
            let x_pos = ix as f32; // Local X within the chunk
            let y_pos = heightmap[current_index];
            let z_pos = iz as f32; // Local Z within the chunk
            geometry.vertices.push([x_pos, y_pos, z_pos]);

            // 2. UV Coordinates - now with a slight variation for breaking patterns
            let u = ix as f32 / chunk_size_f;
            let v = iz as f32 / chunk_size_f;
            
            // Add a tiny offset based on vertex position to break tiling patterns
            let u_offset = ((x_pos * 0.53 + z_pos * 0.71).sin() * 0.01) as f32;
            let v_offset = ((x_pos * 0.73 + z_pos * 0.47).cos() * 0.01) as f32;
            
            geometry.uvs.push([u + u_offset, v + v_offset]);

            // 3. Custom Data (Biome IDs and Weights) - completely reworked
            
            // --- Get original biome IDs and weights ---
            let biome_ids_3 = biome_indices_data[current_index];
            let original_weights = biome_weights_data[current_index];
            
            // Create a completely new weighting scheme based on distance fields
            let mut new_weights = [0.0; 3];
            let mut has_valid_biomes = false;
            
            // First pass - identify valid biomes and calculate total
            let mut total_influence = 0.0;
            for i in 0..3 {
                if biome_ids_3[i] > 0 && original_weights[i] > 0.001 {
                    has_valid_biomes = true;
                    
                    // Create a non-linear curve for smoother transitions
                    // Apply smooth_falloff for a more organic transition feeling
                    let weight = smooth_value(original_weights[i]);
                    new_weights[i] = weight;
                    total_influence += weight;
                } else {
                    new_weights[i] = 0.0;
                }
            }
            
            // Normalize the new weights
            if has_valid_biomes && total_influence > 0.001 {
                for i in 0..3 {
                    new_weights[i] /= total_influence;
                }
            } else if has_valid_biomes {
                // If we have biomes but total influence is too small, 
                // give full weight to the first valid biome
                for i in 0..3 {
                    if biome_ids_3[i] > 0 && original_weights[i] > 0.0 {
                        new_weights[i] = 1.0;
                        break;
                    }
                }
            } else {
                // No valid biomes, default to first slot with full weight
                new_weights[0] = 1.0;
            }
            
            // --- Create the 4-byte array, padding the 4th component ---
            let biome_ids_4 = [biome_ids_3[0], biome_ids_3[1], biome_ids_3[2], 0u8];
            
            // --- Push the data to geometry ---
            geometry.custom0_biome_ids.push(biome_ids_4);
            geometry.custom1_biome_weights.push(new_weights);

            // 4. Calculate Normals (using central difference)
            // [Keep the existing normal calculation code]
            let get_height = |x: i32, z: i32| -> f32 {
                let clamped_x = x.clamp(0, chunk_size as i32) as u32;
                let clamped_z = z.clamp(0, chunk_size as i32) as u32;
                heightmap[(clamped_z * grid_width + clamped_x) as usize]
            };

            let h_l = get_height(ix as i32 - 1, iz as i32);
            let h_r = get_height(ix as i32 + 1, iz as i32);
            let h_d = get_height(ix as i32, iz as i32 - 1);
            let h_u = get_height(ix as i32, iz as i32 + 1);

            let normal_x = h_l - h_r;
            let normal_y = 2.0;
            let normal_z = h_d - h_u;

            let len = (normal_x * normal_x + normal_y * normal_y + normal_z * normal_z).sqrt();
            let norm = if len > 0.0 {
                [normal_x / len, normal_y / len, normal_z / len]
            } else {
                [0.0, 1.0, 0.0]
            };
            geometry.normals.push(norm);
        }
    }

    // --- Second Pass: Generate Indices for Triangles ---
    // [Keep the existing index generation code]
    for iz in 0..chunk_size {
        for ix in 0..chunk_size {
            let idx00 = iz * grid_width + ix;
            let idx10 = iz * grid_width + (ix + 1);
            let idx01 = (iz + 1) * grid_width + ix;
            let idx11 = (iz + 1) * grid_width + (ix + 1);

            let i00 = idx00 as i32;
            let i10 = idx10 as i32;
            let i01 = idx01 as i32;
            let i11 = idx11 as i32;

            geometry.indices.push(i00);
            geometry.indices.push(i10);
            geometry.indices.push(i01);

            geometry.indices.push(i10);
            geometry.indices.push(i11);
            geometry.indices.push(i01);
        }
    }

    geometry
}
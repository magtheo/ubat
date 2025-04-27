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

pub fn generate_mesh_geometry(
    heightmap: &[f32],
    chunk_size: u32,
    // UPDATED parameters: accept indices and weights
    biome_indices: &[[u8; 3]],
    biome_weights: &[[f32; 3]],
) -> MeshGeometry {

    if chunk_size == 0 || heightmap.is_empty() {
        eprintln!("generate_mesh_geometry: Called with zero chunk_size or empty heightmap.");
        return MeshGeometry::default(); // Return default empty geometry
    }

    let expected_len = (chunk_size * chunk_size) as usize;
    // --- FIX: Update length check ---
    if heightmap.len() != expected_len
        || biome_indices.len() != expected_len
        || biome_weights.len() != expected_len
    {
        eprintln!(
            "generate_mesh_geometry: Mismatched lengths! H: {}, BI: {}, BW: {}, Expected: {}. Returning empty.",
            heightmap.len(),
            biome_indices.len(), // Check new array length
            biome_weights.len(), // Check new array length
            expected_len
        );
        return MeshGeometry::default(); // Return default empty geometry
    }
    // --- END FIX ---


    // Initialize vectors
    let mut vertices_vec: Vec<[f32; 3]> = Vec::with_capacity(expected_len);
    let mut uvs_vec: Vec<[f32; 2]> = Vec::with_capacity(expected_len);
    let mut normals_vec: Vec<[f32; 3]> = vec![[0.0, 1.0, 0.0]; expected_len]; // Default normal up
    // Initialize NEW vectors for custom attributes
    let mut custom0_ids_vec: Vec<[u8; 3]> = Vec::with_capacity(expected_len);
    let mut custom1_weights_vec: Vec<[f32; 3]> = Vec::with_capacity(expected_len);


    // --- Generate Vertex Data ---
    for z in 0..chunk_size {
        for x in 0..chunk_size {
            let idx = (z * chunk_size + x) as usize;
            let h = heightmap[idx];
            vertices_vec.push([x as f32, h, z as f32]);

            // Calculate UVs (ensure no division by zero if chunk_size is 1)
            let u_coord = if chunk_size > 1 { x as f32 / (chunk_size - 1) as f32 } else { 0.0 };
            let v_coord = if chunk_size > 1 { z as f32 / (chunk_size - 1) as f32 } else { 0.0 };
            uvs_vec.push([u_coord, v_coord]);

            // --- FIX: Populate Custom Attributes ---
            let ids = biome_indices[idx];   // Get the [u8; 3] array of IDs
            let wghts = biome_weights[idx]; // Get the [f32; 3] array of weights

            // Convert IDs to f32 and store
            custom0_ids_vec.push([ids[0] as u8, ids[1] as u8, ids[2] as u8]);
            // Store weights directly
            custom1_weights_vec.push([wghts[0], wghts[1], wghts[2]]);
            // --- END FIX ---

            // NOTE: The old color calculation based on single biome_id is removed.
        }
    }


    // --- Calculate Normals (Your existing logic) ---
    // This happens *after* all vertex positions are calculated.
    for z in 0..chunk_size {
        for x in 0..chunk_size {
            let idx = (z * chunk_size + x) as usize;
            let h_l = get_clamped_height(x as i32 - 1, z as i32, heightmap, chunk_size);
            let h_r = get_clamped_height(x as i32 + 1, z as i32, heightmap, chunk_size);
            let h_d = get_clamped_height(x as i32, z as i32 - 1, heightmap, chunk_size);
            let h_u = get_clamped_height(x as i32, z as i32 + 1, heightmap, chunk_size);

            let dx = h_l - h_r;
            let dz = h_d - h_u;
            let dy = 2.0; // Adjust this based on horizontal scale if needed

            let mag_sq = dx*dx + dy*dy + dz*dz;
            let mag = if mag_sq > 1e-6 { mag_sq.sqrt() } else { 1.0 };
            normals_vec[idx] = [dx / mag, dy / mag, dz / mag];
        }
    }


    // --- Generate Indices (Triangle definitions) ---
    // Ensure sufficient capacity to avoid reallocations
    let index_count = (chunk_size as usize - 1) * (chunk_size as usize - 1) * 6;
    let mut indices_vec: Vec<i32> = Vec::with_capacity(index_count); // Use i32

    if chunk_size > 1 { // Only generate indices if there's more than one vertex along edges
        for z in 0..chunk_size - 1 {
            for x in 0..chunk_size - 1 {
                // Calculate indices based on current quad
                let idx00 = (z * chunk_size + x) as i32; // Top-left
                let idx10 = idx00 + 1;                   // Top-right
                let idx01 = idx00 + chunk_size as i32;   // Bottom-left
                let idx11 = idx01 + 1;                   // Bottom-right

                // Triangle 1 (Top-left -> Top-right -> Bottom-left)
                indices_vec.push(idx00);
                indices_vec.push(idx10);
                indices_vec.push(idx01);

                // Triangle 2 (Top-right -> Bottom-right -> Bottom-left)
                indices_vec.push(idx10);
                indices_vec.push(idx11);
                indices_vec.push(idx01);
            }
        }
    }


    // Return the populated MeshGeometry struct
    MeshGeometry {
        vertices: vertices_vec,
        normals: normals_vec,
        uvs: uvs_vec,
        indices: indices_vec,
        // colors field removed
        custom0_biome_ids: custom0_ids_vec,       // Add new field
        custom1_biome_weights: custom1_weights_vec, // Add new field
    }
}

// Example: src/terrain/generation_utils.rs
use crate::threading::chunk_storage::MeshGeometry; // Assuming MeshGeometry is here or pub

pub fn get_clamped_height(x: i32, z: i32, heightmap: &[f32], chunk_size: u32) -> f32 {
    // ... implementation from previous steps ...
    let clamped_x = x.clamp(0, chunk_size as i32 - 1) as u32;
    let clamped_z = z.clamp(0, chunk_size as i32 - 1) as u32;
    let idx = (clamped_z * chunk_size + clamped_x) as usize;
    if idx >= heightmap.len() {
        eprintln!("get_clamped_height: Index {} out of bounds for heightmap (len {})", idx, heightmap.len());
        return 0.0;
    }
    // heightmap[idx]; // Use direct index now that we checked bounds? Or stick to .get()? .get() is safer.
    heightmap.get(idx).copied().unwrap_or(0.0)
}

pub fn generate_mesh_geometry(
    heightmap: &[f32], 
    chunk_size: u32, 
    biome_ids: &[u8],
) -> MeshGeometry {
    // ... implementation using pure Rust math (no Godot types) ...
    // ... Ensure it uses the get_clamped_height from this module or passed in ...
    if chunk_size == 0 || heightmap.is_empty() {
        eprintln!("generate_mesh_geometry: Called with zero chunk_size or empty heightmap. Returning empty.");
        // FIX: Initialize all fields
        return MeshGeometry {
            vertices: vec![],
            normals: vec![],
            uvs: vec![],
            indices: vec![],
            colors: vec![],
        };
    }

    let expected_len = (chunk_size * chunk_size) as usize;
    if heightmap.len() != expected_len {
        eprintln!("generate_mesh_geometry: Mismatched lengths! H: {}, B: {}, Expected: {}. Returning empty.",
            heightmap.len(), biome_ids.len(), expected_len);

        // FIX: Initialize all fields
        return MeshGeometry {
            vertices: vec![],
            normals: vec![],
            uvs: vec![],
            indices: vec![],
            colors: vec![],
        };
    }

    println!("generate_mesh_geometry: Initial checks passed. Expected_len: {}", expected_len);

    let mut vertices_vec: Vec<[f32; 3]> = Vec::with_capacity(expected_len);
    let mut uvs_vec: Vec<[f32; 2]> = Vec::with_capacity(expected_len);
    let mut normals_vec: Vec<[f32; 3]> = vec![[0.0, 1.0, 0.0]; expected_len];
    let mut colors_vec: Vec<[f32; 4]> = Vec::with_capacity(expected_len); 

    // Vertices and UVs
    for z in 0..chunk_size {
        for x in 0..chunk_size {
            let idx = (z * chunk_size + x) as usize;
            let h = heightmap[idx];
            vertices_vec.push([x as f32, h, z as f32]);
            uvs_vec.push([
                x as f32 / (chunk_size - 1).max(1) as f32,
                z as f32 / (chunk_size - 1).max(1) as f32,
            ]);

            // --- Add Color Data ---
            let biome_id = biome_ids[idx];
            // Encode biome ID into alpha, normalized. Keep RGB white for now.
            let color = [1.0, 1.0, 1.0, biome_id as f32 / 255.0];
            colors_vec.push(color);
        }
    }

    // --- Normals (Recalculate after vertices are set) ---
    // Iterate through triangles to calculate face normals and average them for vertex normals
    // This is more complex than the simple neighbour check and often better handled by Godot's generate_normals,
    // but let's stick to the neighbour check for consistency with your previous code. Ensure it runs *after* vertices are populated.

    // --- (Your existing Normal Calculation - ensure it uses the final vertices_vec) ---
    // NOTE: This simple normal calculation might contribute to visual issues on steep slopes.
    // Consider recalculating normals *after* interpolation if needed, or using Godot's generation.
    for z in 0..chunk_size {
        for x in 0..chunk_size {
            let idx = (z * chunk_size + x) as usize;
            // Use get_clamped_height on the original heightmap, as vertices_vec uses local coords
            let h_l = get_clamped_height(x as i32 - 1, z as i32, heightmap, chunk_size);
            let h_r = get_clamped_height(x as i32 + 1, z as i32, heightmap, chunk_size);
            let h_d = get_clamped_height(x as i32, z as i32 - 1, heightmap, chunk_size);
            let h_u = get_clamped_height(x as i32, z as i32 + 1, heightmap, chunk_size);

            // Normal points slightly away from steeper slopes
            let dx = h_l - h_r; // Difference in X direction height
            let dz = h_d - h_u; // Difference in Z direction height
            let dy = 2.0; // Constant upward bias (adjust based on coordinate system/scale)

            // Normalize
            let mag_sq = dx*dx + dy*dy + dz*dz;
            let mag = if mag_sq > 1e-6 { mag_sq.sqrt() } else { 1.0 };
            normals_vec[idx] = [dx / mag, dy / mag, dz / mag];
        }
    }

    // Indices
    let index_count = (chunk_size as usize - 1) * (chunk_size as usize - 1) * 6;
    let mut indices_vec = Vec::with_capacity(index_count);
    for z in 0..chunk_size - 1 {
        for x in 0..chunk_size - 1 {
            let idx00 = (z * chunk_size + x) as i32;
            let idx10 = idx00 + 1;
            let idx01 = idx00 + chunk_size as i32;
            let idx11 = idx01 + 1;
            indices_vec.push(idx00); indices_vec.push(idx10); indices_vec.push(idx01);
            indices_vec.push(idx10); indices_vec.push(idx11); indices_vec.push(idx01);
        }
    }

    MeshGeometry { 
        vertices: vertices_vec, 
        normals: normals_vec, 
        uvs: uvs_vec, 
        indices: indices_vec,
        colors: colors_vec,
    }
}
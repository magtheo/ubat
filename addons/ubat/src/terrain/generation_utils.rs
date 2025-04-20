// Example: src/terrain/generation_utils.rs
use crate::threading::chunk_storage::MeshGeometry; // Assuming MeshGeometry is here or pub

pub fn get_clamped_height(x: i32, z: i32, heightmap: &[f32], chunk_size: u32) -> f32 {
    // ... implementation from previous steps ...
     let clamped_x = x.clamp(0, chunk_size as i32 - 1) as u32;
     let clamped_z = z.clamp(0, chunk_size as i32 - 1) as u32;
     let idx = (clamped_z * chunk_size + clamped_x) as usize;
     heightmap.get(idx).copied().unwrap_or(0.0)
}

pub fn generate_mesh_geometry(heightmap: &[f32], chunk_size: u32) -> MeshGeometry {
    // ... implementation using pure Rust math (no Godot types) ...
    // ... Ensure it uses the get_clamped_height from this module or passed in ...
    if chunk_size == 0 || heightmap.is_empty() {
        // FIX: Initialize all fields
        return MeshGeometry {
            vertices: vec![],
            normals: vec![],
            uvs: vec![],
            indices: vec![]
        };
    }

    let expected_len = (chunk_size * chunk_size) as usize;
    if heightmap.len() != expected_len {
        // FIX: Initialize all fields
        return MeshGeometry {
            vertices: vec![],
            normals: vec![],
            uvs: vec![],
            indices: vec![]
        };
    }

    let mut vertices_vec: Vec<[f32; 3]> = Vec::with_capacity(expected_len);
    let mut uvs_vec: Vec<[f32; 2]> = Vec::with_capacity(expected_len);
    let mut normals_vec: Vec<[f32; 3]> = Vec::with_capacity(expected_len);

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
        }
    }

    // Normals
    for z in 0..chunk_size {
        for x in 0..chunk_size {
            let h_l = get_clamped_height(x as i32 - 1, z as i32, heightmap, chunk_size);
            let h_r = get_clamped_height(x as i32 + 1, z as i32, heightmap, chunk_size);
            let h_d = get_clamped_height(x as i32, z as i32 - 1, heightmap, chunk_size);
            let h_u = get_clamped_height(x as i32, z as i32 + 1, heightmap, chunk_size);
            let dx = h_l - h_r;
            let dz = h_d - h_u;
            let dy = 2.0;
            let mag_sq = dx * dx + dy * dy + dz * dz;
            let mag = if mag_sq > 1e-6 { mag_sq.sqrt() } else { 1.0 };
            normals_vec.push([dx / mag, dy / mag, dz / mag]);
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

    MeshGeometry { vertices: vertices_vec, normals: normals_vec, uvs: uvs_vec, indices: indices_vec }
}
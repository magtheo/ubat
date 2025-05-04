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
        // Use godot_error! or eprintln! if this runs outside main thread context reliably
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
        // 6 indices per quad (2 triangles * 3 vertices)
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

            // 2. UV Coordinates (Normalized 0-1 across the chunk)
            let u = ix as f32 / chunk_size_f;
            let v = iz as f32 / chunk_size_f;
            geometry.uvs.push([u, v]);

            // 3. Custom Data (Biome IDs and Weights)
            
            // --- Get original 3 IDs ---
            let biome_ids_3 = biome_indices_data[current_index];

            // --- Create the 4-byte array, padding the 4th component (Alpha channel) ---
            // You can use 0 or 255 for padding; 0 is common if unused.
            let biome_ids_4 = [biome_ids_3[0], biome_ids_3[1], biome_ids_3[2], 0u8]; // Padding with 0

            // --- Push the new 4-byte array ---
            geometry.custom0_biome_ids.push(biome_ids_4); // <-- Pushes [u8; 4]

            // Weights remain the same
            geometry
                .custom1_biome_weights
                .push(biome_weights_data[current_index]);

            // 4. Calculate Normals (using central difference)
            // Helper function to get height safely, handling boundaries
            let get_height = |x: i32, z: i32| -> f32 {
                // Clamp coordinates to grid boundaries
                let clamped_x = x.clamp(0, chunk_size as i32) as u32;
                let clamped_z = z.clamp(0, chunk_size as i32) as u32;
                heightmap[(clamped_z * grid_width + clamped_x) as usize]
            };

            // Get heights of neighbours (central difference method)
            // Use integer coords (ix, iz) relative to the grid_width
            let h_l = get_height(ix as i32 - 1, iz as i32); // Height Left
            let h_r = get_height(ix as i32 + 1, iz as i32); // Height Right
            let h_d = get_height(ix as i32, iz as i32 - 1); // Height Down (towards -Z)
            let h_u = get_height(ix as i32, iz as i32 + 1); // Height Up (towards +Z)

            // Calculate the normal vector components
            // The '2.0' comes from the distance between neighbours (e.g., (x+1) - (x-1) = 2)
            // Adjust scale if your grid units aren't 1.0
            let normal_x = h_l - h_r;
            let normal_y = 2.0; // Adjust vertical scale if needed
            let normal_z = h_d - h_u;

            // Normalize the vector
            let len = (normal_x * normal_x + normal_y * normal_y + normal_z * normal_z).sqrt();
            let norm = if len > 0.0 {
                [normal_x / len, normal_y / len, normal_z / len]
            } else {
                [0.0, 1.0, 0.0] // Default to pointing straight up if length is zero
            };
            geometry.normals.push(norm);
        }
    }

    // --- Second Pass: Generate Indices for Triangles ---
    for iz in 0..chunk_size {
        for ix in 0..chunk_size {
            // Calculate indices of the 4 corners of the current quad
            // Using u32 for indices before casting to i32 needed by Godot
            let idx00 = iz * grid_width + ix;           // Bottom-left
            let idx10 = iz * grid_width + (ix + 1);     // Bottom-right
            let idx01 = (iz + 1) * grid_width + ix;     // Top-left
            let idx11 = (iz + 1) * grid_width + (ix + 1); // Top-right

            // Ensure indices are i32 for Godot
            let i00 = idx00 as i32;
            let i10 = idx10 as i32;
            let i01 = idx01 as i32;
            let i11 = idx11 as i32;

            // First triangle (bottom-left, bottom-right, top-left) - CCW order
            geometry.indices.push(i00);
            geometry.indices.push(i10);
            geometry.indices.push(i01);

            // Second triangle (bottom-right, top-right, top-left) - CCW order
            geometry.indices.push(i10);
            geometry.indices.push(i11);
            geometry.indices.push(i01);
        }
    }

    geometry // Return the populated struct
}

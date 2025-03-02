#include "chunk_generator.hpp"
#include "core/print_string.hpp"
#include "godot_cpp/classes/node3d.hpp"
#include "godot_cpp/classes/mesh_instance3d.hpp"
#include "godot_cpp/classes/array_mesh.hpp"
#include "godot_cpp/classes/shader_material.hpp"
#include <godot_cpp/classes/shader.hpp>
#include "godot_cpp/classes/image_texture.hpp"
#include "godot_cpp/classes/viewport_texture.hpp"
#include "godot_cpp/classes/texture2d.hpp"
#include "godot_cpp/classes/image.hpp"
#include <godot_cpp/classes/resource_loader.hpp>
#include "../utils/SingletonAccessor.hpp"
#include <cmath>
#include <godot_cpp/godot.hpp>
#include <godot_cpp/templates/hash_map.hpp>

namespace godot {

ChunkGenerator::ChunkGenerator() {}
ChunkGenerator::~ChunkGenerator() {}

void ChunkGenerator::_init() {}

// TODO: implement Memory Pooling: The C++ code frequently uses memnew() which could be replaced with memory pool allocations.

void ChunkGenerator::initialize(int chunk_size) {
    m_chunkSize = chunk_size;
    godot::print_line("ChunkGenerator initialized with chunk size: ", m_chunkSize);

    // ─────────────────────────────────────────────────────────────────────
    // 1. Load all noise resources once
    // ─────────────────────────────────────────────────────────────────────
    m_noiseCorral = ResourceLoader::get_singleton()->load("res://project/terrain/noise/coralNoise.tres");
    m_noiseSand   = ResourceLoader::get_singleton()->load("res://project/terrain/noise/sandNoise.tres");
    m_noiseRock   = ResourceLoader::get_singleton()->load("res://project/terrain/noise/rockNoise.tres");
    m_noiseKelp   = ResourceLoader::get_singleton()->load("res://project/terrain/noise/kelpNoise.tres");
    m_noiseLavarock = ResourceLoader::get_singleton()->load("res://project/terrain/noise/lavaRockNoise.tres");
    m_noiseSection = ResourceLoader::get_singleton()->load("res://project/terrain/noise/sectionNoise.tres");
    m_noiseBlend    = ResourceLoader::get_singleton()->load("res://project/terrain/noise/blendNoise.tres");

    m_biomeNoises["Corral"]   = m_noiseCorral;
    m_biomeNoises["Sand"]     = m_noiseSand;
    m_biomeNoises["Rock"]     = m_noiseRock;
    m_biomeNoises["Kelp"]     = m_noiseKelp;
    m_biomeNoises["Lavarock"] = m_noiseLavarock;

    if (m_noiseBlend.is_null()) {
        godot::print_line("❌ Failed to load one or more noise resources.");
    } else {
        godot::print_line("✅ Noise resources loaded successfully.");
    }

    // ─────────────────────────────────────────────────────────────────────
    // 2. Load all biome textures once
    // ─────────────────────────────────────────────────────────────────────
    corral_tex   = ResourceLoader::get_singleton()->load("res://project/terrain/textures/corral.png");
    sand_tex     = ResourceLoader::get_singleton()->load("res://project/terrain/textures/sand.png");
    rock_tex     = ResourceLoader::get_singleton()->load("res://project/terrain/textures/dark.png");
    kelp_tex     = ResourceLoader::get_singleton()->load("res://project/terrain/textures/green.png");
    lavarock_tex = ResourceLoader::get_singleton()->load("res://project/terrain/textures/orange.png");

    // ─────────────────────────────────────────────────────────────────────
    // 3. Load the terrain shader only once and store it
    // ─────────────────────────────────────────────────────────────────────
    m_terrainShader = ResourceLoader::get_singleton()->load("res://project/terrain/shader/chunkShader.gdshader");
    if (m_terrainShader.is_valid()) {
        godot::print_line("✅ Terrain shader loaded once at initialization.");
    } else {
        godot::print_line("❌ Failed to load terrain shader. Check your path.");
    }

    // You can also create and keep a single shared material if you want
    // but typically you'll need to set different parameters per-chunk.
    // For a shared base, do something like:
    // m_sharedMaterial = memnew(ShaderMaterial);
    // if (m_terrainShader.is_valid()) {
    //     m_sharedMaterial->set_shader(m_terrainShader);
    // }

    biome_manager_node = SingletonAccessor::get_singleton("BiomeManager");
    if (!biome_manager_node) {
        godot::print_line("❌ ChunkGenerator: BiomeManager not found at initialization!");
    }

    biome_mask_node = SingletonAccessor::get_singleton("BiomeMask");
    if (!biome_mask_node) {
        godot::print_line("❌ ChunkGenerator: BiomeMask not found at initialization!");
    }
}

void ChunkGenerator::_bind_methods() {
    // Register public functions so they can be called from GDScript
    ClassDB::bind_method(D_METHOD("initialize", "chunk_size"), &ChunkGenerator::initialize);
    ClassDB::bind_method(D_METHOD("generate_chunk_with_biome_data", "cx", "cy", "biome_data"), &ChunkGenerator::generate_chunk_with_biome_data);
    ClassDB::bind_method(D_METHOD("generate_biome_data", "cx", "cy", "chunk_size"), &ChunkGenerator::generate_biome_data);

    // If you want to expose more functions, add them here:
    ClassDB::bind_method(D_METHOD("is_boss_area", "color"), &ChunkGenerator::is_boss_area);
    ClassDB::bind_method(D_METHOD("load_shader", "shader_path"), &ChunkGenerator::load_shader);
}

MeshInstance3D *ChunkGenerator::generate_chunk_with_biome_data(int cx, int cy, const Dictionary &biome_data) {
    godot::print_line("C++ Chunk_generator: Generating chunk with biome data at: ", cx, ", ", cy);

    // ─────────────────────────────────────────────────────────────────────
    // 1. Create the new MeshInstance3D and Mesh
    // ─────────────────────────────────────────────────────────────────────
    MeshInstance3D *mesh_instance = memnew(MeshInstance3D);
    Ref<ArrayMesh> mesh = memnew(ArrayMesh);

    // Create arrays for your vertex data
    Array arrays;
    arrays.resize(Mesh::ARRAY_MAX);

    PackedVector3Array vertices;
    PackedVector2Array uvs;
    PackedInt32Array indices;

    // Basic LOD logic
    int resolution = 32;
    float distance = sqrt(float(cx*cx + cy*cy));
    if (distance > 3) resolution = 16;
    if (distance > 6) resolution = 8;

    float step = float(m_chunkSize) / float(resolution);

    // ─────────────────────────────────────────────────────────────────────
    // 2. Generate terrain geometry (vertices/indices)
    // ─────────────────────────────────────────────────────────────────────
    for (int z = 0; z <= resolution; z++) {
        for (int x = 0; x <= resolution; x++) {
            float xpos = x * step;
            float zpos = z * step;

            float worldX = cx * m_chunkSize + xpos;
            float worldZ = cy * m_chunkSize + zpos;

            // Sample biome color
            Color biomeColor;
            if (x < m_chunkSize && z < m_chunkSize) {
                biomeColor = get_biome_color_from_data(xpos, zpos, biome_data);
            } else {
                // Fallback for edges
                biomeColor = get_biome_color(worldX, worldZ);
            }

            float height = compute_height(worldX, worldZ, biomeColor, biome_data);

            // Push vertex
            vertices.push_back(Vector3(xpos, height * 10.0f, zpos));
            // Push UV
            uvs.push_back(Vector2(float(x) / float(resolution), float(z) / float(resolution)));
        }
    }

    // Indices
    for (int z = 0; z < resolution; z++) {
        for (int x = 0; x < resolution; x++) {
            int i = z * (resolution + 1) + x;
            indices.push_back(i);
            indices.push_back(i + 1);
            indices.push_back(i + (resolution + 1));

            indices.push_back(i + 1);
            indices.push_back(i + (resolution + 1) + 1);
            indices.push_back(i + (resolution + 1));
        }
    }

    arrays[Mesh::ARRAY_VERTEX] = vertices;
    arrays[Mesh::ARRAY_TEX_UV] = uvs;
    arrays[Mesh::ARRAY_INDEX]  = indices;

    mesh->add_surface_from_arrays(Mesh::PRIMITIVE_TRIANGLES, arrays);
    mesh_instance->set_mesh(mesh);
    mesh_instance->set_position(Vector3(cx * m_chunkSize, 0.0f, cy * m_chunkSize));

    // ─────────────────────────────────────────────────────────────────────
    // 3. Create a ShaderMaterial using the pre-loaded terrain shader
    // ─────────────────────────────────────────────────────────────────────
    Ref<ShaderMaterial> material = memnew(ShaderMaterial);
    if (m_terrainShader.is_valid()) {
        material->set_shader(m_terrainShader);
        godot::print_line("C++ Chunk_generator: Shader assigned from cached reference.");
    } else {
        godot::print_line("C++ Chunk_generator: m_terrainShader is null; check initialization.");
    }

    // ─────────────────────────────────────────────────────────────────────
    // 4. Generate / assign biome blend & height textures
    // ─────────────────────────────────────────────────────────────────────
    Ref<ImageTexture> biome_blend_texture = generate_biome_blend_texture_with_data(cx, cy, biome_data);
    Ref<ImageTexture> height_map_texture  = generate_heightmap_texture_with_data(cx, cy, biome_data);

    // Example shader parameters
    material->set_shader_parameter("height_scale",     10.0f);
    material->set_shader_parameter("texture_scale",    0.1f);
    material->set_shader_parameter("blend_sharpness",  5.0f);

    // Assign the textures you loaded once in initialize()
    material->set_shader_parameter("corral_texture",   corral_tex);
    material->set_shader_parameter("sand_texture",     sand_tex);
    material->set_shader_parameter("rock_texture",     rock_tex);
    material->set_shader_parameter("kelp_texture",     kelp_tex);
    material->set_shader_parameter("lavarock_texture", lavarock_tex);

    material->set_shader_parameter("debug_mode", false);

    if (biome_blend_texture.is_valid() && height_map_texture.is_valid()) {
        material->set_shader_parameter("biome_blend_map", biome_blend_texture);
        material->set_shader_parameter("height_map",       height_map_texture);
        mesh_instance->set_material_override(material);
    } else {
        godot::print_line("❌ Failed to create textures for chunk: ", cx, ", ", cy);
    }

    return mesh_instance;
}

Dictionary ChunkGenerator::generate_biome_data(int cx, int cy, int chunk_size) {
    Dictionary biome_data;

    // Set our own chunk size from the parameter (this will help with initialization issues)
    if (m_chunkSize <= 0 && chunk_size > 0) {
        m_chunkSize = chunk_size;
        godot::print_line("Setting chunk size from parameter: ", m_chunkSize);
    }

    // Make sure we have access to BiomeMask and BiomeManager
    if (!biome_mask_node) {
        biome_mask_node = SingletonAccessor::get_singleton("BiomeMask");
        if (!biome_mask_node) {
            godot::print_line("❌ ChunkGenerator: BiomeMask not found!");
            return biome_data; // Return empty dictionary
        }
    }
    
    if (!biome_manager_node) {
        biome_manager_node = SingletonAccessor::get_singleton("BiomeManager");
        if (!biome_manager_node) {
            godot::print_line("❌ ChunkGenerator: BiomeManager not found!");
            return biome_data; // Return empty dictionary
        }
    }
    
    // Pre-generate all biome colors and weights in one go
    for (int y = 0; y < chunk_size; y++) {
        for (int x = 0; x < chunk_size; x++) {
            // Convert to world coordinates
            float world_x = cx * chunk_size + x;
            float world_y = cy * chunk_size + y;
            
            // Create the key (local position in chunk)
            Vector2i key(x, y);
            
            // Get the biome color
            Color biome_color = get_biome_color(world_x, world_y);
            
            // Store color
            biome_data[key] = biome_color;
            
            // Pre-compute and store weights
            Dictionary weights = get_biome_weights(biome_color);
            String weights_key = String("weights_") + String::num_int64(x) + "_" + String::num_int64(y);
            biome_data[weights_key] = weights;
        }
    }
    
    return biome_data;
}

Ref<ImageTexture> ChunkGenerator::generate_biome_blend_texture_with_data(int cx, int cy, const Dictionary &biome_data) {
    godot::print_line("Creating biome blend texture with pre-generated data for chunk: ", cx, ", ", cy);
    
    if (m_chunkSize <= 0) {
        // Attempt to determine size from the biome data
        m_chunkSize = find_chunk_size_from_data(biome_data);
        godot::print_line("Using derived chunk size: ", m_chunkSize);
    }
    
    if (m_chunkSize <= 0) {
        godot::print_line("ERROR: Invalid chunk size: ", m_chunkSize);
        return Ref<ImageTexture>(); // Return empty reference
    }

    PackedByteArray data;
    data.resize(m_chunkSize * m_chunkSize * 3); // RGB format needs 3 bytes per pixel
    
    // Fill the data with default values (black)
    for (int i = 0; i < data.size(); i++) {
        data[i] = 0;
    }
    
    // Create a new image with explicit dimensions
    Ref<Image> image;
    image.instantiate();    
    // Create the image from raw data
    image->set_data(m_chunkSize, m_chunkSize, false, Image::FORMAT_RGB8, data);
    
    godot::print_line("Biome blend image created with dimensions: ", image->get_width(), "x", image->get_height());
    
    // Set pixel values using pre-generated biome data
    for (int y = 0; y < m_chunkSize; y++) {
        for (int x = 0; x < m_chunkSize; x++) {
            Color biomeColor = get_biome_color_from_data(x, y, biome_data);
            image->set_pixel(x, y, biomeColor);
        }
    }
    
    // Create texture from image
    Ref<ImageTexture> texture;
    texture.instantiate();
    texture->create_from_image(image);
    
    return texture;
}

Ref<ImageTexture> ChunkGenerator::generate_heightmap_texture_with_data(int cx, int cy, const Dictionary &biome_data) {
    godot::print_line("Creating heightmap texture with pre-generated data for chunk: ", cx, ", ", cy);
    
    // Create a new image with explicit dimensions
    Ref<Image> image;
    image.instantiate();
    
    // Try a different approach to create the image
    PackedByteArray data;
    data.resize(m_chunkSize * m_chunkSize * 3); // RGB format needs 3 bytes per pixel
    
    // Fill the data with default values (black)
    for (int i = 0; i < data.size(); i++) {
        data[i] = 0;
    }
    
    // Create the image from raw data
    image->set_data(m_chunkSize, m_chunkSize, false, Image::FORMAT_RGB8, data);
    
    godot::print_line("Heightmap image created with dimensions: ", image->get_width(), "x", image->get_height());
    
    // Set pixel values using pre-generated biome data
    for (int y = 0; y < m_chunkSize; y++) {
        for (int x = 0; x < m_chunkSize; x++) {
            Color biomeColor = get_biome_color_from_data(x, y, biome_data);
            float height = compute_height(cx * m_chunkSize + x, cy * m_chunkSize + y, biomeColor, biome_data);
            image->set_pixel(x, y, Color(height, height, height));
        }
    }
    
    // Create texture from image
    Ref<ImageTexture> texture;
    texture.instantiate();
    texture->create_from_image(image);
    
    return texture;
}

int ChunkGenerator::find_chunk_size_from_data(const Dictionary &biome_data) {
    int max_x = 0;
    int max_y = 0;
    
    Array keys = biome_data.keys();
    for (int i = 0; i < keys.size(); i++) {
        Vector2i key = keys[i];
        max_x = MAX(max_x, key.x + 1);
        max_y = MAX(max_y, key.y + 1);
    }
    
    return max_x > max_y ? max_x : max_y;
}

Color ChunkGenerator::get_biome_color_from_data(int x, int y, const Dictionary &biome_data) {
    Vector2i key(x, y);
    if (biome_data.has(key)) {
        Variant color_var = biome_data[key];
        if (color_var.get_type() == Variant::COLOR) {
            return (Color)color_var;
        }
    }
    // Fallback to white if the color is not found
    return Color(1.0f, 1.0f, 1.0f, 1.0f);
}


float ChunkGenerator::compute_height(float world_x, float world_y, const Color &biomeColor, const Dictionary &biome_data) {
    // Get local coordinates within the chunk
    int chunk_x = int(world_x) % m_chunkSize;
    int chunk_y = int(world_y) % m_chunkSize;
    
    // Get pre-computed weights
    String weights_key = String("weights_") + String::num_int64(chunk_x) + "_" + String::num_int64(chunk_y);
    Dictionary biome_weights_dict;
    
    if (biome_data.has(weights_key)) {
        biome_weights_dict = biome_data[weights_key];
    } else {
        // Fallback - should not happen in normal operation
        return 0.0f;
    }
    
    // Sample the blend noise directly
    float blendNoise = 0.0f;
    if (m_noiseBlend.is_valid()) {
        // Use call() to invoke the method on the resource
        Variant result = m_noiseBlend->call("get_noise_2d", world_x, world_y);
        if (result.get_type() == Variant::FLOAT) {
            blendNoise = (float)result;
        }
    }
    
    float blendedHeight = 0.0f;
    float totalWeight = 0.0f;
    Array keys = biome_weights_dict.keys();
    
    for (int i = 0; i < keys.size(); i++) {
        String biome_name = keys[i];
        float weight = biome_weights_dict[biome_name];
        
        // Get the appropriate noise resource from our map
        if (m_biomeNoises.has(biome_name)) {
            Ref<Resource> biome_noise = m_biomeNoises[biome_name];
            if (biome_noise.is_valid()) {
                Variant result = biome_noise->call("get_noise_2d", world_x, world_y);
                float biomeNoise = (result.get_type() == Variant::FLOAT) ? (float)result : 0.0f;
                blendedHeight += weight * biomeNoise * blendNoise;
                totalWeight += weight;
            }
        }
    }
    
    return (totalWeight > 1e-6f) ? blendedHeight / totalWeight : 0.0f;
}

Color ChunkGenerator::get_biome_color(float world_x, float world_y) {
    if (!biome_mask_node) biome_mask_node = SingletonAccessor::get_singleton("BiomeMask");
    Variant result = biome_mask_node->call("get_biome_color", world_x, world_y);
    return (result.get_type() == Variant::COLOR) ? (Color)result : Color(1.0f, 1.0f, 1.0f, 1.0f);
}

Dictionary ChunkGenerator::get_biome_weights(const Color &color) {
    if (!biome_manager_node) biome_manager_node = SingletonAccessor::get_singleton("BiomeManager");
    Variant biome_weights_var = biome_manager_node->call("get_biome_weights", color);
    return (biome_weights_var.get_type() == Variant::DICTIONARY) ? (Dictionary)biome_weights_var : Dictionary();
}

bool ChunkGenerator::is_boss_area(const Color &color) {
    return color == Color(1, 0, 0, 1);
}

Ref<Shader> ChunkGenerator::load_shader(const String &shader_path) {
    // godot::print_line("chunk_generator: loading shader");
    Ref<Shader> shader = ResourceLoader::get_singleton()->load(shader_path);
    if (shader.is_null()) {
        godot::print_line("❌ Failed to load shader: " + shader_path);
    } else {
        // godot::print_line("✅ Shader loaded successfully: " + shader_path);
    }
    return shader;
}

} // namespace godot

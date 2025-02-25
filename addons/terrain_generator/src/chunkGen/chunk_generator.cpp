#include "chunk_generator.hpp"
#include "core/print_string.hpp"
#include "godot_cpp/classes/node.hpp"
#include <cmath>
#include "../utils/SingletonAccessor.hpp"
#include "godot_cpp/variant/dictionary.hpp"
#include "godot_cpp/variant/string.hpp"

namespace godot {

ChunkGenerator::ChunkGenerator() {
    // Constructor (if needed)
}

ChunkGenerator::~ChunkGenerator() {
    // Destructor (if needed)
}

void ChunkGenerator::_init() {
    // Called by Godot when the object is created.
}

void ChunkGenerator::initialize(int chunk_size, int seed) {
    m_chunkSize = chunk_size;
    m_seed = seed;
    // Randomize seeds for all noise instances.
    m_noiseWrapper.randomize_seeds(seed);

    // Fetch singletons once at init
    
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
    ClassDB::bind_method(D_METHOD("initialize", "chunk_size", "seed"), &ChunkGenerator::initialize);
    ClassDB::bind_method(D_METHOD("generate_chunk", "cx", "cy"), &ChunkGenerator::generate_chunk);

    // If you want to expose more functions, add them here:
    ClassDB::bind_method(D_METHOD("get_biome_color", "world_x", "world_y"), &ChunkGenerator::get_biome_color);
    ClassDB::bind_method(D_METHOD("get_biome_weights", "color"), &ChunkGenerator::get_biome_weights);
    ClassDB::bind_method(D_METHOD("is_boss_area", "color"), &ChunkGenerator::is_boss_area);
}


//
// Helper function to compute vertex height using biome noise and blending.
//
float ChunkGenerator::compute_height(float world_x, float world_y, const Color &biomeColor) {
    if (is_boss_area(biomeColor)) {
        return m_noiseWrapper.get_boss_noise(world_x, world_y);
    }

    // Get biome weights from the biome color (now a Dictionary)
    Dictionary biome_weights_dict = get_biome_weights(biomeColor);

    // Get blending noise
    float blendNoise = m_noiseWrapper.get_blending_noise(world_x, world_y);

    float blendedHeight = 0.0f;
    float totalWeight = 0.0f;

    // Iterate over the Dictionary to blend biome heights
    Array keys = biome_weights_dict.keys();
    for (int i = 0; i < keys.size(); i++) {
        // Convert biome_name (Godot String) to std::string
        String biome_name = keys[i];
        std::string biome_name_std = biome_name.utf8().get_data();
        
        float weight = biome_weights_dict[biome_name];
        

        // Retrieve noise for each biome
        float biomeNoise = m_noiseWrapper.get_noise_2d(biome_name_std, world_x, world_y);
        float contribution = weight * biomeNoise * blendNoise;
        blendedHeight += contribution;
        totalWeight += weight;
    }

    // Normalize the height if needed
    if (totalWeight > 1e-6f) {
        blendedHeight /= totalWeight;
    } else {
        blendedHeight = 0.0f;  // Default value in case of no weight
    }

    return blendedHeight;
}


//
// The generate_chunk method builds a full mesh for a chunk and converts it
// into a Godot Dictionary containing two Arrays: "vertices" and "indices".
//
Dictionary ChunkGenerator::generate_chunk(int cx, int cy) {
    godot::print_line("-C++ generating chunk at: ", cx, ", ", cy);
    Mesh mesh;
    int numVerticesPerSide = m_chunkSize + 1;
    float gridSpacing = 1.0f; // Adjust scale if necessary.

    // Compute world offsets.
    float worldOffsetX = cx * m_chunkSize * gridSpacing;
    float worldOffsetY = cy * m_chunkSize * gridSpacing;

    // Reserve space for vertices.
    mesh.vertices.resize(numVerticesPerSide * numVerticesPerSide * 3);

    int vertexIndex = 0;
    for (int j = 0; j < numVerticesPerSide; ++j) {
        for (int i = 0; i < numVerticesPerSide; ++i) {
            float world_x = worldOffsetX + i * gridSpacing;
            float world_y = worldOffsetY + j * gridSpacing;
            
            // Call the (placeholder) function to get the biome color.
            Color biomeColor = get_biome_color(world_x, world_y);
            // maybe add std::unordered_map<std::string, float> biomeWeights = get_biome_weights(biomeColor);
            
            // Compute the height.
            float height = compute_height(world_x, world_y, biomeColor);
            
            // Store vertex (x, y, z).
            mesh.vertices[vertexIndex++] = world_x;
            mesh.vertices[vertexIndex++] = height;
            mesh.vertices[vertexIndex++] = world_y;
        }
    }

    // Create indices for quads (2 triangles per quad).
    for (int j = 0; j < m_chunkSize; ++j) {
        for (int i = 0; i < m_chunkSize; ++i) {
            int topLeft = j * numVerticesPerSide + i;
            int topRight = topLeft + 1;
            int bottomLeft = (j + 1) * numVerticesPerSide + i;
            int bottomRight = bottomLeft + 1;
            
            // First triangle.
            mesh.indices.push_back(topLeft);
            mesh.indices.push_back(bottomLeft);
            mesh.indices.push_back(topRight);
            
            // Second triangle.
            mesh.indices.push_back(topRight);
            mesh.indices.push_back(bottomLeft);
            mesh.indices.push_back(bottomRight);
        }
    }

    // Convert the mesh to a Dictionary so it can be used in GDScript.
    Dictionary mesh_dict;
    Array vertices_array;
    for (size_t i = 0; i < mesh.vertices.size(); ++i) {
        vertices_array.append(mesh.vertices[i]);
    }
    Array indices_array;
    for (size_t i = 0; i < mesh.indices.size(); ++i) {
        indices_array.append(mesh.indices[i]);
    }
    mesh_dict["vertices"] = vertices_array;
    mesh_dict["indices"] = indices_array;

    return mesh_dict;
}


Color ChunkGenerator::get_biome_color(float world_x, float world_y) {
    if (!biome_mask_node) {
        godot::print_line("Chunk_generator.cpp: BiomeMask is NULL, trying to re-fetch...");
        biome_mask_node = SingletonAccessor::get_singleton("BiomeMask");

        if (!biome_mask_node) {
            godot::print_line("ChunkGenerator.cpp: BiomeMask still not found!");
            return Color(1.0f, 1.0f, 1.0f, 1.0f); // Default color
        }
    }

    Variant result = biome_mask_node->call("get_biome_color", world_x, world_y);
    if (result.get_type() != Variant::COLOR) {
        return Color(1.0f, 1.0f, 1.0f, 1.0f);
    }

    return result;
}



Dictionary ChunkGenerator::get_biome_weights(const Color &color) {
    if (!biome_manager_node) {
        godot::print_line("Chunk_generator.cpp: BiomeManager is NULL, trying to re-fetch...");
        biome_manager_node = SingletonAccessor::get_singleton("BiomeManager");
        if (!biome_manager_node) {
            godot::print_line("Chunk_generator.cpp: BiomeManager still not found!");
            return Dictionary();
        }
    }

    Variant biome_weights_var = biome_manager_node->call("get_biome_weights", color);
    if (biome_weights_var.get_type() != Variant::DICTIONARY) {
        godot::print_line("Chunk_generator.cpp: Failed to get biome weights!");
        return Dictionary();
    }

    godot::print_line("Chunk_generator.cpp: Got biome weights: ", biome_weights_var);
    return biome_weights_var;
}



bool ChunkGenerator::is_boss_area(const Color &color) {
    // Dummy implementation. Replace with actual logic.
    if (color == Color(1,0,0,1)) {
        return true;
    }
    return false;
}

} // namespace godot

#include "chunk_generator.hpp"
#include <cmath>
#include <godot_cpp/godot.hpp>
#include <godot_cpp/core/class_db.hpp>
#include <godot_cpp/classes/node.hpp>
#include <godot_cpp/classes/scene_tree.hpp>
#include <godot_cpp/classes/engine.hpp>

#include "../utils/SingletonAccessor.hpp"

namespace godot {

ChunkGenerator::ChunkGenerator() {
    // Constructor (if needed)
}

ChunkGenerator::~ChunkGenerator() {
    // Destructor (if needed)
}

void ChunkGenerator::_init() {
    // Object *biome_manager_obj = Engine::get_singleton("BiomeManager");
    
    // Called by Godot when the object is created.
}

void ChunkGenerator::initialize(int chunk_size, int seed) {
    
    // TODO ad accessing of singleton 
    m_chunkSize = chunk_size;
    m_seed = seed;
    // Randomize seeds for all noise instances.
    m_noiseWrapper.randomize_seeds(seed);
}

//
// Helper function to compute vertex height using biome noise and blending.
//
float ChunkGenerator::compute_height(float world_x, float world_y, const Color &biomeColor) {
    // If the point is in the boss area, use boss noise.
    if (is_boss_area(biomeColor)) {
        return m_noiseWrapper.get_boss_noise(world_x, world_y);
    }

    // Get biome weights from the biome color.
    std::map<std::string, float> biomeWeights = get_biome_weights(biomeColor);

    // Get blending noise.
    float blendNoise = m_noiseWrapper.get_blending_noise(world_x, world_y);

    float blendedHeight = 0.0f;
    float totalWeight = 0.0f;

    // Blend contributions from each biome.
    for (const auto &pair : biomeWeights) {
        const std::string &biomeName = pair.first;
        float weight = pair.second;
        float biomeNoise = m_noiseWrapper.get_noise_2d(biomeName, world_x, world_y);
        float contribution = weight * biomeNoise * blendNoise;
        blendedHeight += contribution;
        totalWeight += weight;
    }

    if (totalWeight > 0.0f) {
        blendedHeight /= totalWeight;
    }
    return blendedHeight;
}

//
// The generate_chunk method builds a full mesh for a chunk and converts it
// into a Godot Dictionary containing two Arrays: "vertices" and "indices".
//
Dictionary ChunkGenerator::generate_chunk(int cx, int cy) {
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

//
// -- Placeholder functions --
// In your project these would be provided by your GDScript code or other logic.
//
Color ChunkGenerator::get_biome_color(float world_x, float world_y) {
    // Dummy implementation. Replace with actual logic.
    return {1.0f, 1.0f, 1.0f, 1.0f};
}

std::map<std::string, float> ChunkGenerator::get_biome_weights(const Color &color) {
    // Dummy implementation. Replace with actual logic.
    // For example, based on the color, decide how much each biome contributes.
    return { {"Corral", 0.5f}, {"Sand", 0.5f} };
}

bool ChunkGenerator::is_boss_area(const Color &color) {
    // Dummy implementation. Replace with actual logic.
    return false;
}

} // namespace godot

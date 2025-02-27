#ifndef CHUNK_GENERATOR_HPP
#define CHUNK_GENERATOR_HPP

#include <godot_cpp/classes/object.hpp>        // Using Object instead of Reference/RefCounted
#include <godot_cpp/core/class_db.hpp>
#include <godot_cpp/godot.hpp>
#include <godot_cpp/variant/color.hpp>
#include <vector>
#include <string>
#include "../noiseGen/FastNoiseLiteWrapper.hpp" // Make sure this relative path is valid
#include "godot_cpp/classes/node.hpp"
#include "godot_cpp/variant/dictionary.hpp"

namespace godot {

/// Internal mesh structure used for generating the mesh.
struct Mesh {
    std::vector<float> vertices;         // Flat array: x, y, z for each vertex.
    std::vector<unsigned int> indices;
};

class ChunkGenerator : public Node {
    GDCLASS(ChunkGenerator, Node)      // Using Object as the base

public:
    ChunkGenerator();
    ~ChunkGenerator();

    // Godot initialization method.
    void _init();

    // Register methods to Godot.
    static void _bind_methods();

    // Exposed methods.
    void initialize(int chunk_size, int seed);
    Dictionary generate_chunk(int cx, int cy);

private:
    int m_chunkSize = 0;   // Number of quads per side (grid resolution)
    int m_seed = 0;

    Node *biome_manager_node = nullptr;
    Node *biome_mask_node = nullptr;

    // Noise wrapper instance.
    FastNoiseLiteWrapper m_noiseWrapper;

    // Helper to compute height from biome noise.
    float compute_height(float world_x, float world_y, const Color &biomeColor);

    // --- Placeholders for integration with GDScript logic ---
    // In your project these functions are implemented in GDScript.
    Color get_biome_color(float world_x, float world_y);
    Dictionary get_biome_weights(const Color &color);
    bool is_boss_area(const Color &color);
};

} // namespace godot

#endif // CHUNK_GENERATOR_HPP

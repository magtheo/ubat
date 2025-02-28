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
#include "godot_cpp/classes/mesh_instance3d.hpp"
#include "godot_cpp/classes/shader_material.hpp"
#include "godot_cpp/classes/shader.hpp"
#include "godot_cpp/classes/image_texture.hpp"

namespace godot {

/// Internal mesh structure used for generating the mesh.


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
    void initialize(int chunk_size, Node *seedNode);
    MeshInstance3D *generate_chunk(int cx, int cy);

private:
    int m_chunkSize = 0;   // Number of quads per side (grid resolution)
    Node *m_seedNode;

    Node *biome_manager_node = nullptr;
    Node *biome_mask_node = nullptr;

    // Noise wrapper instance.
    FastNoiseLiteWrapper m_noiseWrapper;

    // Helper to compute height from biome noise.
    float compute_height(float world_x, float world_y, const Color &biomeColor);

    Ref<ShaderMaterial> create_shader_material();
    Ref<ImageTexture> generate_biome_blend_texture(int cx, int cy);
    Ref<ImageTexture> generate_heightmap_texture(int cx, int cy);

    Ref<Shader> load_shader(const String &shader_path);

    // --- Placeholders for integration with GDScript logic ---
    // In your project these functions are implemented in GDScript.
    Color get_biome_color(float world_x, float world_y);
    Dictionary get_biome_weights(const Color &color);
    bool is_boss_area(const Color &color);
};

} // namespace godot

#endif // CHUNK_GENERATOR_HPP

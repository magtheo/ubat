#ifndef CHUNK_GENERATOR_HPP
#define CHUNK_GENERATOR_HPP

#include <godot_cpp/classes/object.hpp>        // Using Object instead of Reference/RefCounted
#include <godot_cpp/core/class_db.hpp>
#include <godot_cpp/godot.hpp>
#include <godot_cpp/variant/color.hpp>  
#include <vector>
#include <string>
#include "godot_cpp/classes/node.hpp"
#include "godot_cpp/variant/dictionary.hpp"
#include "godot_cpp/classes/mesh_instance3d.hpp"
#include "godot_cpp/classes/shader_material.hpp"
#include "godot_cpp/classes/shader.hpp"
#include "godot_cpp/classes/image_texture.hpp"
#include "godot_cpp/variant/vector2i.hpp"
// #include "../utils/FastNoiseLite.h"
#include "godot_cpp/templates/hash_map.hpp"
#include <godot_cpp/classes/noise_texture2d.hpp>

namespace godot {

class ChunkGenerator : public Node {
    GDCLASS(ChunkGenerator, Node)

public:
    ChunkGenerator();
    ~ChunkGenerator();

    // Godot initialization method.
    void _init();

    // Register methods to Godot.
    static void _bind_methods();

    // Exposed methods.
    void initialize(int chunk_size);
    MeshInstance3D *generate_chunk(int cx, int cy);
    
    // New method that accepts pre-generated biome data
    MeshInstance3D *generate_chunk_with_biome_data(int cx, int cy, const Dictionary &biome_data);
    Dictionary generate_biome_data(int cx, int cy, int chunk_size);


private:
    // Noise resources
    Ref<NoiseTexture2D> m_noiseCorral;
    Ref<NoiseTexture2D> m_noiseSand;
    Ref<NoiseTexture2D> m_noiseRock;
    Ref<NoiseTexture2D> m_noiseKelp;
    Ref<NoiseTexture2D> m_noiseLavarock;
    Ref<NoiseTexture2D> m_noiseSection;
    Ref<NoiseTexture2D> m_noiseBlend;
    
    // Map biome names to noise resources
    Dictionary m_biomeNoises;

    Dictionary m_cachedBiomeNoiseImages; // key: biome name, value: Ref<Image>
    Ref<Image> m_blendNoiseImage;         // Cached blend noise image

    // texture resources
    Ref<Texture2D> corral_tex;
    Ref<Texture2D> sand_tex;
    Ref<Texture2D> rock_tex;
    Ref<Texture2D> kelp_tex;
    Ref<Texture2D> lavarock_tex;


    int m_chunkSize = 0;   // Number of quads per side (grid resolution)
    float m_heightMultiplier = 20.0f;  // default

    Node *biome_manager_node = nullptr;
    Node *biome_mask_node = nullptr;

    Ref<ShaderMaterial> m_sharedMaterial;
    Ref<Shader> m_terrainShader;  // Add this line

    // Helper to compute height from biome noise.
    float compute_height(float world_x, float world_y, const Color &biomeColor, const Dictionary &biome_data);

    Ref<ImageTexture> generate_biome_blend_texture(int cx, int cy);
    Ref<ImageTexture> generate_heightmap_texture(int cx, int cy);
    
    Ref<ImageTexture> generate_biome_blend_texture_with_data(int cx, int cy, const Dictionary &biome_data);
    Ref<ImageTexture> generate_heightmap_texture_with_data(int cx, int cy, const Dictionary &biome_data);
    
    int find_chunk_size_from_data(const Dictionary &biome_data);

    // Ref<Shader> load_shader(const String &shader_path);

    // --- Placeholders for integration with GDScript logic ---
    // In your project these functions are implemented in GDScript.
    Color get_biome_color(float world_x, float world_y);
    Dictionary get_biome_weights(const Color &color);
    bool is_boss_area(const Color &color);
    
    // Helper to get biome color from pre-generated data
    Color get_biome_color_from_data(int x, int y, const Dictionary &biome_data);
};

} // namespace godot

#endif // CHUNK_GENERATOR_HPP

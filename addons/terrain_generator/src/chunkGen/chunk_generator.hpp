#ifndef CHUNK_GENERATOR_HPP
#define CHUNK_GENERATOR_HPP
#include <godot_cpp/classes/object.hpp> // Using Object instead of Reference/RefCounted
#include <godot_cpp/core/class_db.hpp>
#include <godot_cpp/godot.hpp>
#include <godot_cpp/variant/color.hpp>
#include <godot_cpp/classes/timer.hpp>
#include "godot_cpp/classes/fast_noise_lite.hpp"
#include "godot_cpp/classes/node.hpp"
#include "godot_cpp/variant/dictionary.hpp"
#include "godot_cpp/classes/mesh_instance3d.hpp"
#include "godot_cpp/classes/shader_material.hpp"
#include "godot_cpp/classes/shader.hpp"
#include "godot_cpp/classes/image_texture.hpp"
#include "godot_cpp/variant/vector2i.hpp"
// #include "../utils/FastNoiseLite.h"ChunkGenerator
#include "godot_cpp/templates/hash_map.hpp"
#include <godot_cpp/classes/noise_texture2d.hpp>

namespace godot {

class ChunkGenerator : public Node3D {
    GDCLASS(ChunkGenerator, Node3D)

public:
    ChunkGenerator();
    ~ChunkGenerator();

    // Godot initialization method.
    void _init();

    // Register methods to Godot.
    static void _bind_methods();

    // Exposed methods.
    void initialize(int chunk_size);
    void _ready();
    MeshInstance3D *generate_chunk_with_biome_data(int cx, int cy, const Dictionary &biome_data);
    Dictionary generate_biome_data(int cx, int cy, int chunk_size);
    void cleanup_chunk_caches(Vector2i min_chunk, Vector2i max_chunk);
    bool is_boss_area(const Color &color);

    // Debugging
    bool debug_biome_distribution = true;
    void print_chunk_biome_distribution(int cx, int cy,  const Dictionary &biome_data, bool debug_biome_distribution);

private:
    //  FastNoiseLite resources
    Ref<FastNoiseLite> noiseCorral;
    Ref<FastNoiseLite> noiseSand;
    Ref<FastNoiseLite> noiseRock;
    Ref<FastNoiseLite> noiseKelp;
    Ref<FastNoiseLite> noiseLavarock;
    Ref<FastNoiseLite> noiseSection;
    Ref<FastNoiseLite> noiseBlend;


    // NoiseTexture resources
    Ref<NoiseTexture2D> m_noiseCorral;
    Ref<NoiseTexture2D> m_noiseSand;
    Ref<NoiseTexture2D> m_noiseRock;
    Ref<NoiseTexture2D> m_noiseKelp;
    Ref<NoiseTexture2D> m_noiseLavarock;
    Ref<NoiseTexture2D> m_noiseSection;
    Ref<NoiseTexture2D> m_noiseBlend;

    // Map biome names to noise resources
    HashMap<String, Ref<NoiseTexture2D>> m_biomeNoises;
    HashMap<String, Ref<Image>> m_cachedBiomeNoiseImages;

    Ref<Image> m_blendNoiseImage; // Cached blend noise image

    // texture resources
    Ref<Texture2D> corral_tex;
    Ref<Texture2D> sand_tex;
    Ref<Texture2D> rock_tex;
    Ref<Texture2D> kelp_tex;
    Ref<Texture2D> lavarock_tex;

    bool load_resources();
    bool cache_resources();
    void wait_for_texture_async(Ref<NoiseTexture2D> texture, String biome_key);
    void on_texture_ready(Ref<NoiseTexture2D> texture, String biome_key, Timer* timer);


    Ref<NoiseTexture2D> create_noise_texture(const Ref<FastNoiseLite>& noise, int width, int height, bool seamless);


    int m_chunkSize = 0; // Number of quads per side (grid resolution)
    float m_heightMultiplier = 20.0f; // default

    Node *biome_manager_node = nullptr;
    Node *biome_mask_node = nullptr;

    HashMap<Vector2i, Ref<ImageTexture>> m_biomeBlendTextureCache;
    HashMap<Vector2i, Ref<ImageTexture>> m_heightmapTextureCache;
    Ref<ShaderMaterial> m_sharedMaterial;
    Ref<Shader> m_terrainShader;

    // Helper to compute height from biome noise.
    float compute_height(float world_x, float world_y, const Dictionary &biome_data);
    
    Ref<ImageTexture> generate_biome_blend_texture_with_data(int cx, int cy, const Dictionary &biome_data);
    Ref<ImageTexture> generate_heightmap_texture_with_data(int cx, int cy, const Dictionary &biome_data);
    int find_chunk_size_from_data(const Dictionary &biome_data);

    // --- Methods called from C++ but implemented in GDScript ---
    Color get_biome_color(float world_x, float world_y);
    Dictionary get_biome_weights(const Color &color);
    
    // Helper to get biome color from pre-generated data
    Color get_biome_color_from_data(int x, int y, const Dictionary &biome_data);
};

} // namespace godot
#endif // CHUNK_GENERATOR_HPP
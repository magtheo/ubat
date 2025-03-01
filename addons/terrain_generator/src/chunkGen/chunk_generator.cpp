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

namespace godot {

ChunkGenerator::ChunkGenerator() {}
ChunkGenerator::~ChunkGenerator() {}

void ChunkGenerator::_init() {}

void ChunkGenerator::initialize(int chunk_size) {
    m_chunkSize = chunk_size;

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
    ClassDB::bind_method(D_METHOD("generate_chunk", "cx", "cy"), &ChunkGenerator::generate_chunk);

    // If you want to expose more functions, add them here:
    ClassDB::bind_method(D_METHOD("get_biome_color", "world_x", "world_y"), &ChunkGenerator::get_biome_color);
    ClassDB::bind_method(D_METHOD("get_biome_weights", "color"), &ChunkGenerator::get_biome_weights);
    ClassDB::bind_method(D_METHOD("is_boss_area", "color"), &ChunkGenerator::is_boss_area);
    ClassDB::bind_method(D_METHOD("load_shader", "shader_path"), &ChunkGenerator::load_shader);
}

MeshInstance3D *ChunkGenerator::generate_chunk(int cx, int cy) {
    godot::print_line("C++ Chunk_generator: Generating chunk at: ", cx, ", ", cy);
    
    MeshInstance3D *mesh_instance = memnew(MeshInstance3D);
    Ref<ArrayMesh> mesh = memnew(ArrayMesh);
    
    // Assign mesh
    mesh_instance->set_mesh(mesh);
    
    // Assign textures
    Ref<ShaderMaterial> material = create_shader_material();
    material->set_shader_parameter("biome_blend_map", generate_biome_blend_texture(cx, cy));
    material->set_shader_parameter("height_map", generate_heightmap_texture(cx, cy));
    
    mesh_instance->set_material_override(material);
    return mesh_instance;
}

Ref<ShaderMaterial> ChunkGenerator::create_shader_material() {
    Ref<ShaderMaterial> material = memnew(ShaderMaterial);
    material->set_shader(load_shader("res://project/terrain/shader/chunkShader.gdshader"));
    return material;
}

Ref<ImageTexture> ChunkGenerator::generate_biome_blend_texture(int cx, int cy) {
    Ref<Image> image = memnew(Image);
    image->create(m_chunkSize, m_chunkSize, false, Image::FORMAT_RGB8);
    for (int y = 0; y < m_chunkSize; y++) {
        for (int x = 0; x < m_chunkSize; x++) {
            Color biomeColor = get_biome_color(cx * m_chunkSize + x, cy * m_chunkSize + y);
            image->set_pixel(x, y, biomeColor);
        }
    }
    Ref<ImageTexture> texture = memnew(ImageTexture);
    texture->create_from_image(image);
    return texture;
}

Ref<ImageTexture> ChunkGenerator::generate_heightmap_texture(int cx, int cy) {
    Ref<Image> image = memnew(Image);
    image->create(m_chunkSize, m_chunkSize, false, Image::FORMAT_R8);
    for (int y = 0; y < m_chunkSize; y++) {
        for (int x = 0; x < m_chunkSize; x++) {
            float height = compute_height(cx * m_chunkSize + x, cy * m_chunkSize + y, get_biome_color(cx, cy));
            image->set_pixel(x, y, Color(height, height, height));
        }
    }
    Ref<ImageTexture> texture = memnew(ImageTexture);
    texture->create_from_image(image);
    return texture;
}

float ChunkGenerator::compute_height(float world_x, float world_y, const Color &biomeColor) {
    Dictionary biome_weights_dict = get_biome_weights(biomeColor);
    float blendNoise = m_noiseWrapper.get_blending_noise(world_x, world_y);
    float blendedHeight = 0.0f, totalWeight = 0.0f;
    Array keys = biome_weights_dict.keys();
    for (int i = 0; i < keys.size(); i++) {
        String biome_name = keys[i];
        float weight = biome_weights_dict[biome_name];
        float biomeNoise = m_noiseWrapper.get_noise_2d(biome_name.utf8().get_data(), world_x, world_y);
        blendedHeight += weight * biomeNoise * blendNoise;
        totalWeight += weight;
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

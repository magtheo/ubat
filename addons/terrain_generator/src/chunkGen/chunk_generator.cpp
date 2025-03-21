#include "chunk_generator.hpp"

#include "core/print_string.hpp"
#include "godot_cpp/classes/node3d.hpp"
#include "godot_cpp/classes/mesh_instance3d.hpp"
#include "godot_cpp/classes/array_mesh.hpp"
#include "godot_cpp/classes/shader_material.hpp"
#include <godot_cpp/classes/shader.hpp>
#include "godot_cpp/classes/image_texture.hpp"
#include <godot_cpp/classes/noise_texture2d.hpp>
#include <godot_cpp/classes/fast_noise_lite.hpp>
#include "godot_cpp/classes/viewport_texture.hpp"
#include "godot_cpp/classes/texture2d.hpp"
#include "godot_cpp/classes/image.hpp"
#include <godot_cpp/classes/resource_loader.hpp>
#include <godot_cpp/templates/hash_map.hpp>
#include <godot_cpp/godot.hpp>
#include <cmath>
#include <godot_cpp/classes/timer.hpp>

#include "../utils/SingletonAccessor.hpp"
#include "../utils/ResourceLoaderHelper.hpp"
#include "variant/variant.hpp"


using namespace godot;
namespace godot {

ChunkGenerator::ChunkGenerator() {}
ChunkGenerator::~ChunkGenerator() {}

void ChunkGenerator::_init() {}

// TODO: implement Memory Pooling: The C++ code frequently uses memnew() which could be replaced with memory pool allocations.
// TODO: implement LOD with propper vertex stitching

bool ChunkGenerator::load_resources() {
    godot::print_line("🔄 Loading and caching resources...");
    
    // First, try to load the FastNoiseLite resources
    Ref<FastNoiseLite> corralNoise = ResourceLoaderHelper::load_cached<FastNoiseLite>("res://project/terrain/noise/corralNoise.tres", "corral Noise");
    Ref<FastNoiseLite> sandNoise = ResourceLoaderHelper::load_cached<FastNoiseLite>("res://project/terrain/noise/sandNoise.tres", "sand Noise");
    Ref<FastNoiseLite> rockNoise = ResourceLoaderHelper::load_cached<FastNoiseLite>("res://project/terrain/noise/rockNoise.tres", "rock Noise");
    Ref<FastNoiseLite> kelpNoise = ResourceLoaderHelper::load_cached<FastNoiseLite>("res://project/terrain/noise/kelpNoise.tres", "kelp Noise");
    Ref<FastNoiseLite> lavaRockNoise = ResourceLoaderHelper::load_cached<FastNoiseLite>("res://project/terrain/noise/lavaRockNoise.tres", "lavarock Noise");
    Ref<FastNoiseLite> sectionNoise = ResourceLoaderHelper::load_cached<FastNoiseLite>("res://project/terrain/noise/sectionNoise.tres", "section Noise");
    Ref<FastNoiseLite> blendNoise = ResourceLoaderHelper::load_cached<FastNoiseLite>("res://project/terrain/noise/blendNoise.tres", "blend Noise");
    
    // Then create NoiseTexture2D objects and assign the loaded noise
    m_noiseCorral = create_noise_texture(corralNoise, 256, 256, true);
    m_noiseSand = create_noise_texture(sandNoise, 256, 256, true);
    m_noiseRock = create_noise_texture(rockNoise, 256, 256, true);
    m_noiseKelp = create_noise_texture(kelpNoise, 256, 256, true);
    m_noiseLavarock = create_noise_texture(lavaRockNoise, 256, 256, true);
    m_noiseSection = create_noise_texture(sectionNoise, 256, 256, true);
    m_noiseBlend = create_noise_texture(blendNoise, 256, 256, true);
    
    // Add to dictionary
    if (m_noiseCorral.is_valid())   m_biomeNoises.insert("corral", m_noiseCorral);
    if (m_noiseSand.is_valid())     m_biomeNoises.insert("sand", m_noiseSand);
    if (m_noiseRock.is_valid())     m_biomeNoises.insert("rock", m_noiseRock);
    if (m_noiseKelp.is_valid())     m_biomeNoises.insert("kelp", m_noiseKelp);
    if (m_noiseLavarock.is_valid()) m_biomeNoises.insert("lavarock", m_noiseLavarock);

    // Load biome textures
    corral_tex   = ResourceLoaderHelper::load_cached<Texture2D>("res://textures/corral.png", "corral Texture");
    sand_tex     = ResourceLoaderHelper::load_cached<Texture2D>("res://textures/sand.png", "sand Texture");
    rock_tex     = ResourceLoaderHelper::load_cached<Texture2D>("res://textures/dark.png", "rock Texture");
    kelp_tex     = ResourceLoaderHelper::load_cached<Texture2D>("res://textures/green.png", "kelp Texture");
    lavarock_tex = ResourceLoaderHelper::load_cached<Texture2D>("res://textures/orange.png", "lavarock Texture");

    if (corral_tex.is_null() || sand_tex.is_null() || rock_tex.is_null() ||
        kelp_tex.is_null() || lavarock_tex.is_null()) {
        godot::print_line("ERROR One or more biome textures failed to load.");
    } else {
        godot::print_line("SUCCESS All biome textures loaded successfully.");
    }

    // Load terrain shader
    m_terrainShader = ResourceLoaderHelper::load_cached<Shader>("res://project/terrain/shader/chunkShader.gdshader", "Terrain Shader");
    if (m_terrainShader.is_valid()) {
        godot::print_line("SUCCESS Terrain shader loaded once at initialization.");
    } else {
        godot::print_line("ERROR Failed to load terrain shader. Check your path.", m_terrainShader);
    }
    
    return true;
}

Ref<NoiseTexture2D> ChunkGenerator::create_noise_texture(const Ref<FastNoiseLite>& noise, int width, int height, bool seamless) {
    // Create a new noise texture
    Ref<NoiseTexture2D> texture;
    texture.instantiate();
    
    if (!texture.is_valid()) {
        godot::print_line("ERROR Failed to instantiate NoiseTexture2D");
        return Ref<NoiseTexture2D>();
    }
    
    // Check if the noise resource is valid
    if (!noise.is_valid()) {
        godot::print_line("ERROR Provided FastNoiseLite is invalid, creating default noise");
        
        // Create a default noise if the provided one is invalid
        Ref<FastNoiseLite> default_noise;
        default_noise.instantiate();
        default_noise->set_noise_type(FastNoiseLite::TYPE_PERLIN);
        default_noise->set_frequency(0.05);
        
        texture->set_noise(default_noise);
    } else {
        // Use the provided noise
        texture->set_noise(noise);
    }
    
    // Configure the texture
    texture->set_width(width);
    texture->set_height(height);
    texture->set_seamless(seamless);
    texture->set_invert(false);
    
    // Force the texture to generate immediately to catch any issues
    texture->set_generate_mipmaps(true);
    
    godot::print_line("SUCCESS Successfully created noise texture");
    return texture;
}

// initialize() should only handle data-independent setup—things that don’t require the node to be inside the scene tree.
void ChunkGenerator::initialize(int chunk_size) {
    m_chunkSize = chunk_size;
    godot::print_line("ChunkGenerator initialized with chunk size: ", m_chunkSize);

    bool resources_are_loaded = load_resources();
    if (!resources_are_loaded) {
        godot::print_line("resources failed to load");
    }

    // bool resources_cached = cache_resources(); // TODO: test and ask gpt: forbedringer/tanker
    // if (!resources_cached) {
    //     godot::print_line("resources failed to cache");
    // }
    
     
    // Get BiomeManager and BiomeMask singletons
    biome_manager_node = SingletonAccessor::get_singleton("BiomeManager");
    if (!biome_manager_node) {
        godot::print_line("ERROR ChunkGenerator: BiomeManager not found at initialization!");
    }

    biome_mask_node = SingletonAccessor::get_singleton("BiomeMask");
    if (!biome_mask_node) {
        godot::print_line("ERROR ChunkGenerator: BiomeMask not found at initialization!");
    } else {
        bool ready = biome_mask_node->is_node_ready();
        if (ready == false ) {
            godot::print_line("ERROR BiomeMask is not ready. Terrain generation should wait.");
        } else {
            godot::print_line("SUCCESS BiomeMask is ready for use.");
        }
    }
}

void ChunkGenerator::_ready() {
    godot::print_line("✅ ChunkGenerator is now inside the scene tree!");

    // Ensure biome mask is ready before caching resources
    if (!biome_mask_node || !biome_mask_node->is_node_ready()) {
        godot::print_line("⚠️ BiomeMask is not ready in _ready(), terrain generation should wait.");
        return;
    }

    // Now it's safe to cache resources (uses timers inside scene tree)
    bool resources_cached = cache_resources();
    if (!resources_cached) {
        godot::print_line("❌ Resources failed to cache in _ready()!");
    }
}


bool ChunkGenerator::cache_resources() {
    // Cache blend noise image
    if (m_noiseBlend.is_valid()) {
        wait_for_texture_async(m_noiseBlend, "blend");
    } else {
        godot::print_line("❌ m_noiseBlend is not valid");
    }

    // Cache each biome's noise image
    for (KeyValue<String, Ref<NoiseTexture2D>> &E : m_biomeNoises) {
        String key = E.key;
        Ref<NoiseTexture2D> noise_tex = E.value;

        if (noise_tex.is_valid()) {
            wait_for_texture_async(noise_tex, key);
        }
    }
    return true;
}


void ChunkGenerator::wait_for_texture_async(Ref<NoiseTexture2D> texture, String biome_key) {
    Timer* timer = memnew(Timer);
    timer->set_wait_time(0.1);  // Check every 100ms
    timer->set_one_shot(false); // Keep checking until texture is ready
    
    if (is_inside_tree()) {
        add_child(timer);
    } else {
        godot::print_line("⚠️ Timer not added to scene tree because ChunkGenerator is not inside the tree yet!");
    }

    timer->connect("timeout", callable_mp(this, &ChunkGenerator::on_texture_ready).bind(texture, biome_key));
    
    add_child(timer); // Add timer to scene tree
    timer->start();
}

void ChunkGenerator::on_texture_ready(Ref<NoiseTexture2D> texture, String biome_key, Timer* timer) {
    if (!texture.is_valid()) {
        godot::print_line("❌ Invalid noise texture: ", biome_key);
        return;
    }
    
    Ref<Image> noise_img = texture->get_image();
    if (noise_img.is_valid()) {
        // Store it in cache
        if (biome_key == "blend") {
            m_blendNoiseImage = noise_img;
        } else {
            m_cachedBiomeNoiseImages[biome_key] = noise_img;
        }

        godot::print_line("✅ Cached biome noise image for: ", biome_key,
            " size: ", noise_img->get_width(), "x", noise_img->get_height());

        // Stop the timer and remove it
        timer->stop();
        remove_child(timer);
        timer->queue_free();
    } else {
        godot::print_line("⏳ Waiting for noise texture to be generated for: ", biome_key);
    }
}


void ChunkGenerator::_bind_methods() {
    // Register public functions so they can be called from GDScript
    ClassDB::bind_method(D_METHOD("initialize", "chunk_size"), &ChunkGenerator::initialize);
    ClassDB::bind_method(D_METHOD("generate_chunk_with_biome_data", "cx", "cy", "biome_data"), &ChunkGenerator::generate_chunk_with_biome_data);
    ClassDB::bind_method(D_METHOD("generate_biome_data", "cx", "cy", "chunk_size"), &ChunkGenerator::generate_biome_data);
    ClassDB::bind_method(D_METHOD("cleanup_chunk_caches", "min_chunk", "max_chunk"), &ChunkGenerator::cleanup_chunk_caches);
    ClassDB::bind_method(D_METHOD("is_boss_area", "color"), &ChunkGenerator::is_boss_area);
    // ClassDB::bind_method(D_METHOD("load_shader", "shader_path"), &ChunkGenerator::load_shader);
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
    int resolution = m_chunkSize * 2; 
    float distance = sqrt(float(cx*cx + cy*cy));
    if (distance > 6) resolution = m_chunkSize;   // Reduce only at farther distances
    if (distance > 12) resolution = 16;
    if (distance > 20) resolution = 8;
    

    float step = float(m_chunkSize) / float(resolution);

    // ─────────────────────────────────────────────────────────────────────
    // 2. Generate terrain geometry (vertices/indices)
    // ─────────────────────────────────────────────────────────────────────
    for (int z = 0; z <= resolution; z++) {
        for (int x = 0; x <= resolution; x++) {
            float xpos = float(x) / resolution * m_chunkSize;
            float zpos = float(z) / resolution * m_chunkSize;

            float worldX = cx * m_chunkSize + xpos;
            float worldZ = cy * m_chunkSize + zpos;

            // Sample biome color
            Color biomeColor = get_biome_color_from_data(worldX, worldZ, biome_data);
            float height = compute_height(worldX, worldZ, biome_data);

            // Push vertex        Color biomeColor = get_biome_color_from_data(xpos, zpos, biome_data);

            vertices.push_back(Vector3(xpos, height * m_heightMultiplier, zpos));

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

    material->set_shader_parameter("blend_min", 0.4f);  // Below this value, use biome 1 only
    material->set_shader_parameter("blend_max", 0.6f);  // Above this value, use biome 2 only

    // Assign the textures you loaded once in initialize()
    material->set_shader_parameter("corral_texture",   corral_tex);
    material->set_shader_parameter("sand_texture",     sand_tex);
    material->set_shader_parameter("rock_texture",     rock_tex);
    material->set_shader_parameter("kelp_texture",     kelp_tex);
    material->set_shader_parameter("lavarock_texture", lavarock_tex);

    material->set_shader_parameter("debug_mode", 0);

    if (biome_blend_texture.is_valid() && height_map_texture.is_valid()) {
        material->set_shader_parameter("biome_blend_map", biome_blend_texture);
        material->set_shader_parameter("height_map",       height_map_texture);
        mesh_instance->set_material_override(material);
    } else {
        godot::print_line("ERROR Failed to create textures for chunk: ", cx, ", ", cy);
    }

    print_chunk_biome_distribution(cx, cy, biome_data, debug_biome_distribution);
    return mesh_instance;
}

Dictionary ChunkGenerator::generate_biome_data(int cx, int cy, int chunk_size) {
    Dictionary biome_data;
    Dictionary biome_colors;
    Dictionary biome_weights;

    if (!biome_mask_node->is_node_ready()) {
        godot::print_line("ERROR Attempted to generate biome data before BiomeMask is ready.");
        return Dictionary();  // Return empty to prevent bad data
    }

    for (int y = 0; y < chunk_size; y++) {
        for (int x = 0; x < chunk_size; x++) {
            // Convert to world coordinates
            float world_x = cx * chunk_size + x;
            float world_y = cy * chunk_size + y;

            // Create a key for the color (using Vector2i is fine)
            Vector2i color_key(x, y);
            Color biome_color = get_biome_color(world_x, world_y);
            biome_colors[color_key] = biome_color;

            // Pre-compute and store weights under a string key
            Dictionary weights = get_biome_weights(biome_color);
            // godot::print_line("Boime weights for: ", cy, ":", cy, ": ", weights);
            String weights_key = String("weights_") + String::num_int64(x) + "_" + String::num_int64(y);
            biome_weights[weights_key] = weights;
        }
    }

    // Combine the two dictionaries into one parent dictionary.
    biome_data["colors"] = biome_colors;
    biome_data["weights"] = biome_weights;

    return biome_data;
}

Ref<ImageTexture> ChunkGenerator::generate_biome_blend_texture_with_data(int cx, int cy, const Dictionary &biome_data) {
    godot::print_line("Creating biome blend texture for chunk: ", cx, ", ", cy);
    
    // Check for cached texture first
    Vector2i chunk_pos(cx, cy);
    if (m_biomeBlendTextureCache.has(chunk_pos)) {
        godot::print_line("Using cached biome blend texture for chunk: ", cx, ", ", cy);
        return m_biomeBlendTextureCache[chunk_pos];
    }
    
    if (m_chunkSize <= 0) {
        // Attempt to determine size from the biome data
        m_chunkSize = find_chunk_size_from_data(biome_data);
        godot::print_line("Using derived chunk size: ", m_chunkSize);
    }
    
    if (m_chunkSize <= 0) {
        godot::print_line("ERROR: Invalid chunk size: ", m_chunkSize);
        return Ref<ImageTexture>(); // Return empty reference
    }

    // Create a new image with RGBA8 format to store both section color and blend factor
    Ref<Image> image = image->create(m_chunkSize, m_chunkSize, false, Image::FORMAT_RGBA8);
    
    godot::print_line("Biome blend image created with dimensions: ", 
        image->get_width(), "x", image->get_height());
    
    // Extract biome colors and data
    Dictionary colors_data;
    if (biome_data.has("colors")) {
        colors_data = (Dictionary)biome_data["colors"];
    } else {
        godot::print_line("Error: 'colors' dictionary missing from biome data");
        return Ref<ImageTexture>();
    }
    
    // Ensure we have blend noise image
    if (!m_blendNoiseImage.is_valid() && m_noiseBlend.is_valid()) {
        m_blendNoiseImage = m_noiseBlend->get_image();
    }

    // Debug output to verify blend noise
    if (m_blendNoiseImage.is_valid()) {
        godot::print_line("SUCCESS: Blend noise image is valid, dimensions: ", 
            m_blendNoiseImage->get_width(), "x", m_blendNoiseImage->get_height());
    } else {
        godot::print_line("WARNING: Blend noise image is NOT valid");
    }
    
    // Set pixel values
    for (int y = 0; y < m_chunkSize; y++) {
        for (int x = 0; x < m_chunkSize; x++) {
            // Get the biome color for this pixel (identifies the section)
            Vector2i color_key(x, y);
            Color biome_color = Color(1.0, 1.0, 1.0); // Default white
            
            if (colors_data.has(color_key)) {
                biome_color = (Color)colors_data[color_key];
            }
            
            // Sample blend noise for this world position
            float world_x = cx * m_chunkSize + x;
            float world_y = cy * m_chunkSize + y;
            float blend_factor = 0.5f; // Default middle value
            
            // Sample from blend noise texture if available
            if (m_blendNoiseImage.is_valid()) {
                int img_width = m_blendNoiseImage->get_width();
                int img_height = m_blendNoiseImage->get_height();
                
                // Ensure we wrap properly and avoid index errors
                int sample_x = ((int)world_x % img_width + img_width) % img_width;
                int sample_y = ((int)world_y % img_height + img_height) % img_height;
                
                // Get noise value from the blend noise texture
                Color noise_pixel = m_blendNoiseImage->get_pixel(sample_x, sample_y);
                blend_factor = noise_pixel.r; // Use red channel for noise value
            }
            
            // Store section color in RGB channels, blend factor in alpha channel
            Color pixel_color(biome_color.r, biome_color.g, biome_color.b, blend_factor);
            image->set_pixel(x, y, pixel_color);
        }
    }
    
    // Create texture from image
    Ref<ImageTexture> texture;
    texture.instantiate();
    texture->create_from_image(image);
    
    // Cache the texture for future use
    m_biomeBlendTextureCache[chunk_pos] = texture;
    
    return texture;
}

void ChunkGenerator::print_chunk_biome_distribution(int cx, int cy, const Dictionary &biome_data, bool debug_biome_distribution) {
    // Only run this in debug builds or when explicitly requested
    if ( debug_biome_distribution) {
        
        // Get the weights dictionary
        if (!biome_data.has("weights")) {
            return;
        }
        
        Dictionary weights_data = (Dictionary)biome_data["weights"];
        
        // Track total weights for each biome
        float total_corral = 0.0f;
        float total_sand = 0.0f;
        float total_rock = 0.0f;
        float total_kelp = 0.0f;
        float total_lavarock = 0.0f;
        
        // Sample only a subset of pixels for performance
        // For a 64x64 chunk, sampling every 8th pixel gives us 64 samples (8x8 grid)
        int sample_step = m_chunkSize / 8;
        if (sample_step < 1) sample_step = 1;
        
        int samples_count = 0;
        
        // Process the samples
        for (int y = 0; y < m_chunkSize; y += sample_step) {
            for (int x = 0; x < m_chunkSize; x += sample_step) {
                String weights_key = String("weights_") + String::num_int64(x) + "_" + String::num_int64(y);
                
                if (weights_data.has(weights_key)) {
                    Dictionary specific_weights = (Dictionary)weights_data[weights_key];
                    
                    if (specific_weights.has("corral")) total_corral += (float)(real_t)specific_weights["corral"];
                    if (specific_weights.has("sand")) total_sand += (float)(real_t)specific_weights["sand"];
                    if (specific_weights.has("rock")) total_rock += (float)(real_t)specific_weights["rock"];
                    if (specific_weights.has("kelp")) total_kelp += (float)(real_t)specific_weights["kelp"];
                    if (specific_weights.has("lavarock")) total_lavarock += (float)(real_t)specific_weights["lavarock"];
                    
                    samples_count++;
                }
            }
        }
        
        // Calculate the total weight
        float total_weight = total_corral + total_sand + total_rock + total_kelp + total_lavarock;
        
        // Calculate percentages (avoid division by zero)
        if (total_weight > 0.001f) {
            float corral_pct = (total_corral / total_weight) * 100.0f;
            float sand_pct = (total_sand / total_weight) * 100.0f;
            float rock_pct = (total_rock / total_weight) * 100.0f;
            float kelp_pct = (total_kelp / total_weight) * 100.0f;
            float lavarock_pct = (total_lavarock / total_weight) * 100.0f;
            
            // Only print biomes with significant presence (> 1%)
            String biome_info = "📊 Chunk(" + String::num_int64(cx) + "," + String::num_int64(cy) + ") Biomes: ";
            if (corral_pct > 1.0f) biome_info += "Corral:" + String::num(corral_pct, 1) + "% ";
            if (sand_pct > 1.0f) biome_info += "Sand:" + String::num(sand_pct, 1) + "% ";
            if (rock_pct > 1.0f) biome_info += "Rock:" + String::num(rock_pct, 1) + "% ";
            if (kelp_pct > 1.0f) biome_info += "Kelp:" + String::num(kelp_pct, 1) + "% ";
            if (lavarock_pct > 1.0f) biome_info += "Lavarock:" + String::num(lavarock_pct, 1) + "% ";
            
            godot::print_line(biome_info);
        }
    }
}


Ref<ImageTexture> ChunkGenerator::generate_heightmap_texture_with_data(int cx, int cy, const Dictionary &biome_data) {
    godot::print_line("Creating heightmap texture for chunk: ", cx, ", ", cy);
    
    // Check for cached texture first
    Vector2i chunk_pos(cx, cy);
    if (m_heightmapTextureCache.has(chunk_pos)) {
        godot::print_line("SUCCESS Using cached heightmap texture for chunk: ", cx, ", ", cy);
        return m_heightmapTextureCache[chunk_pos];
    }
    
    // Create a new image with explicit dimensions
    // Ref<Image> image;
    // image.instantiate();
    
    // IMPORTANT: Create the image with proper dimensions first
    Ref<Image> image = image->create(m_chunkSize, m_chunkSize, false, Image::FORMAT_RGB8);
    
    godot::print_line("Heightmap image created with dimensions: ", 
        image->get_width(), "x", image->get_height());
    
    // Set pixel values using pre-generated biome data
    for (int y = 0; y < m_chunkSize; y++) {
        for (int x = 0; x < m_chunkSize; x++) {
            float height = compute_height(cx * m_chunkSize + x, cy * m_chunkSize + y, biome_data);
            image->set_pixel(x, y, Color(height, height, height));
        }
    }
    
    // Create texture from image
    Ref<ImageTexture> texture;
    texture.instantiate();
    texture->create_from_image(image);
    
    // Cache the texture for future use
    m_heightmapTextureCache[chunk_pos] = texture;
    
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
    if (biome_data.has("colors")) {
        Dictionary colors = biome_data["colors"];
        Vector2i key(x, y);
        if (colors.has(key)) {
            Variant color_var = colors[key];
            if (color_var.get_type() == Variant::COLOR) {
                return (Color)color_var;
            }
        }
    } else {
        // Try direct lookup if "colors" dictionary isn't present
        Vector2i key(x, y);
        if (biome_data.has(key)) {
            Variant color_var = biome_data[key];
            if (color_var.get_type() == Variant::COLOR) {
                return (Color)color_var;
            }
        }
    }
    
    // Fallback to white if the color is not found
    return Color(1.0f, 1.0f, 1.0f, 1.0f);
}

float ChunkGenerator::compute_height(float world_x, float world_y, const Dictionary &biome_data) {
    // Adjust for local chunk coordinates
    int local_x = ((int)world_x % m_chunkSize + m_chunkSize) % m_chunkSize;
    int local_y = ((int)world_y % m_chunkSize + m_chunkSize) % m_chunkSize;
    
    // Get the biome color - used for section identification
    Vector2i color_key(local_x, local_y);
    Color biome_color;
    
    // Get from colors dict if available
    if (biome_data.has("colors")) {
        Dictionary colors = (Dictionary)biome_data["colors"];
        if (colors.has(color_key)) {
            biome_color = (Color)colors[color_key];
        } else {
            // Fallback to white if color not found
            biome_color = Color(1.0f, 1.0f, 1.0f);
        }
    } else {
        // Another fallback if colors dict missing
        biome_color = Color(1.0f, 1.0f, 1.0f);
    }
    
    // Get blend factor from blend noise
    float blend_factor = 0.5f;  // Default mid value
    
    if (m_blendNoiseImage.is_valid()) {
        int img_width = m_blendNoiseImage->get_width();
        int img_height = m_blendNoiseImage->get_height();
        
        // Ensure we wrap properly and avoid index errors
        int sample_x = ((int)world_x % img_width + img_width) % img_width;
        int sample_y = ((int)world_y % img_height + img_height) % img_height;
        
        // Make absolutely sure we're not out of bounds
        sample_x = Math::clamp(sample_x, 0, img_width - 1);
        sample_y = Math::clamp(sample_y, 0, img_height - 1);
        
        Color noise_pixel = m_blendNoiseImage->get_pixel(sample_x, sample_y);
        blend_factor = noise_pixel.r;
    }
    
    // Apply stepped blending logic
    float blend_min = 0.4f;
    float blend_max = 0.6f;
    float weight;
    
    if (blend_factor < blend_min) {
        weight = 0.0f; // 100% biome 1
    } else if (blend_factor > blend_max) {
        weight = 1.0f; // 100% biome 2
    } else {
        weight = (blend_factor - blend_min) / (blend_max - blend_min);
    }
    
    // Determine which biomes to blend based on the section color
    String biome1, biome2;
    
    // Identify section based on color - this must match the shader and BiomeManager logic!
    float tolerance = 0.2f;
    if (std::sqrt(std::pow(biome_color.r - 0.8f, 2) + 
                 std::pow(biome_color.g - 0.8f, 2) + 
                 std::pow(biome_color.b - 0.8f, 2)) < tolerance) {
        // Section 1: Corral + Sand
        biome1 = "corral";
        biome2 = "sand";
    } else if (biome_color.r > 0.7f) {
        // Section 2: Rock + Kelp
        biome1 = "rock";
        biome2 = "kelp";
    } else {
        // Section 3: Rock + Lavarock
        biome1 = "rock";
        biome2 = "lavarock";
    }
    
    // Sample height for biome 1
    float height1 = 0.0f;
    if (m_biomeNoises.has(biome1)) {
        Ref<NoiseTexture2D> biome_tex = m_biomeNoises[biome1];
        if (biome_tex.is_valid()) {
            Ref<Image> noise_image;
            if (m_cachedBiomeNoiseImages.has(biome1)) {
                noise_image = m_cachedBiomeNoiseImages[biome1];
            } else if (biome_tex->get_image().is_valid()) {
                noise_image = biome_tex->get_image();
                m_cachedBiomeNoiseImages.insert(biome1, noise_image);
            }
            
            if (noise_image.is_valid()) {
                int img_width = noise_image->get_width();
                int img_height = noise_image->get_height();
                
                // Ensure proper wrapping and bounds checking
                int sample_x = ((int)world_x % img_width + img_width) % img_width;
                int sample_y = ((int)world_y % img_height + img_height) % img_height;
                
                // Extra bounds check
                sample_x = Math::clamp(sample_x, 0, img_width - 1);
                sample_y = Math::clamp(sample_y, 0, img_height - 1);
                
                Color pixel = noise_image->get_pixel(sample_x, sample_y);
                height1 = pixel.r;
            }
        }
    }
    
    // Sample height for biome 2
    float height2 = 0.0f;
    if (m_biomeNoises.has(biome2)) {
        Ref<NoiseTexture2D> biome_tex = m_biomeNoises[biome2];
        if (biome_tex.is_valid()) {
            Ref<Image> noise_image;
            if (m_cachedBiomeNoiseImages.has(biome2)) {
                noise_image = m_cachedBiomeNoiseImages[biome2];
            } else if (biome_tex->get_image().is_valid()) {
                noise_image = biome_tex->get_image();
                m_cachedBiomeNoiseImages.insert(biome2, noise_image);
            }
            
            if (noise_image.is_valid()) {
                int img_width = noise_image->get_width();
                int img_height = noise_image->get_height();
                
                // Ensure proper wrapping and bounds checking
                int sample_x = ((int)world_x % img_width + img_width) % img_width;
                int sample_y = ((int)world_y % img_height + img_height) % img_height;
                
                // Extra bounds check
                sample_x = Math::clamp(sample_x, 0, img_width - 1);
                sample_y = Math::clamp(sample_y, 0, img_height - 1);
                
                Color pixel = noise_image->get_pixel(sample_x, sample_y);
                height2 = pixel.r;
            }
        }
    }
    
    // Blend heights using the calculated weight
    float final_height = height1 * (1.0f - weight) + height2 * weight;
    return final_height;
}


void ChunkGenerator::cleanup_chunk_caches(Vector2i min_chunk, Vector2i max_chunk) {
    // Clean up textures for chunks outside the given range
    Array keys_to_remove_blend;
    for (KeyValue<Vector2i, Ref<ImageTexture>> &E : m_biomeBlendTextureCache) {
        Vector2i chunk_pos = E.key;
        if (chunk_pos.x < min_chunk.x || chunk_pos.x > max_chunk.x || 
            chunk_pos.y < min_chunk.y || chunk_pos.y > max_chunk.y) {
            keys_to_remove_blend.push_back(chunk_pos);
        }
    }
    
    for (int i = 0; i < keys_to_remove_blend.size(); i++) {
        Vector2i key = keys_to_remove_blend[i];
        m_biomeBlendTextureCache.erase(key);
    }
    
    // Do the same for heightmap textures
    Array keys_to_remove_height;
    for (KeyValue<Vector2i, Ref<ImageTexture>> &E : m_heightmapTextureCache) {
        Vector2i chunk_pos = E.key;
        if (chunk_pos.x < min_chunk.x || chunk_pos.x > max_chunk.x || 
            chunk_pos.y < min_chunk.y || chunk_pos.y > max_chunk.y) {
            keys_to_remove_height.push_back(chunk_pos);
        }
    }
    
    for (int i = 0; i < keys_to_remove_height.size(); i++) {
        Vector2i key = keys_to_remove_height[i];
        m_heightmapTextureCache.erase(key);
    }
    
    godot::print_line("ChunkGenerator: Cleaned up ", keys_to_remove_blend.size(), 
        " blend textures and ", keys_to_remove_height.size(), " heightmap textures");
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

// Ref<Shader> ChunkGenerator::load_shader(const String &shader_path) {
//     // godot::print_line("chunk_generator: loading shader");
//     Ref<Shader> shader = ResourceLoader::get_singleton()->load(shader_path);
//     if (shader.is_null()) {
//         godot::print_line("ERROR Failed to load shader: " + shader_path);
//     } else {
//         godot::print_line("SUCCESS Shader loaded successfully: " + shader_path);
//     }
//     return shader;
// }

} // namespace godot

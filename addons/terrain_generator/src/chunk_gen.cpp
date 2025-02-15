#include "chunk_gen.h"
#include <godot_cpp/classes/resource_loader.hpp>
#include <godot_cpp/classes/noise_texture2d.hpp>
#include <godot_cpp/classes/fast_noise_lite.hpp>
#include <godot_cpp/core/class_db.hpp>
#include <godot_cpp/classes/global_constants.hpp>
#include <cstdlib> // for srand, rand

using namespace godot;

void ChunkGenerator::_bind_methods() {
    ClassDB::bind_method(D_METHOD("initialize",
                                  "corral_path",
                                  "sand_path",
                                  "rock_path",
                                  "kelp_path",
                                  "lavarock_path",
                                  "section_path",
                                  "blend_path",
                                  "chunk_size",
                                  "seed"),
                         &ChunkGenerator::initialize);

    ClassDB::bind_method(D_METHOD("generate_chunk", "cx", "cy"),
                         &ChunkGenerator::generate_chunk);
}

ChunkGenerator::ChunkGenerator() {
}

ChunkGenerator::~ChunkGenerator() {
}

void ChunkGenerator::initialize(
    const String corral_path,
    const String sand_path,
    const String rock_path,
    const String kelp_path,
    const String lavarock_path,
    const String section_path,
    const String blend_path,
    int p_chunk_size,
    int p_seed
) {
    chunk_size = p_chunk_size;

    // 1) Load each .tres resource
    Ref<ResourceLoader> loader = ResourceLoader::get_singleton();

    corral_noise_tex = loader->load(corral_path);
    sand_noise_tex   = loader->load(sand_path);
    rock_noise_tex   = loader->load(rock_path);
    kelp_noise_tex   = loader->load(kelp_path);
    lavarock_noise_tex = loader->load(lavarock_path);

    section_noise_tex = loader->load(section_path);
    blend_noise_tex   = loader->load(blend_path);

    // 2) Randomize seeds (for each embedded FastNoiseLite) but keep other settings
    srand(p_seed);

    // Helper function to randomize the seed in one NoiseTexture2D
    auto randomize_seed = [&](Ref<NoiseTexture2D> &tex) {
        if (!tex.is_valid()) return;
        Ref<FastNoiseLite> fn = tex->get_fast_noise_lite();
        if (fn.is_valid()) {
            fn->set_seed(rand()); // override the seed
            // Force re-generation of the texture
            tex->notify_changed();
        }
    };

    randomize_seed(corral_noise_tex);
    randomize_seed(sand_noise_tex);
    randomize_seed(rock_noise_tex);
    randomize_seed(kelp_noise_tex);
    randomize_seed(lavarock_noise_tex);
    randomize_seed(section_noise_tex);
    randomize_seed(blend_noise_tex);
}

TypedArray<float> ChunkGenerator::generate_chunk(int cx, int cy) {
    TypedArray<float> result;
    result.resize(chunk_size * chunk_size);

    // Simple example: we treat (cx, cy) as chunk coords in a grid
    // We'll compute real world coords by offset
    float start_x = cx * float(chunk_size);
    float start_y = cy * float(chunk_size);

    for (int ly = 0; ly < chunk_size; ++ly) {
        for (int lx = 0; lx < chunk_size; ++lx) {
            float x = start_x + float(lx);
            float y = start_y + float(ly);

            // 1) Which section are we in?
            SectionID section = get_section_for_point(x, y);

            // 2) Sample the 2 biome noises and blend them
            float height_val = get_height_for_section(section, x, y);

            // store in array
            int idx = ly * chunk_size + lx;
            result[idx] = height_val;
        }
    }

    return result;
}

// Utility: sample from NoiseTexture2D's embedded FastNoiseLite.
float ChunkGenerator::get_noise_value(const Ref<NoiseTexture2D> &tex, float x, float y) const {
    if (!tex.is_valid()) {
        return 0.0f;
    }
    Ref<FastNoiseLite> fn = tex->get_fast_noise_lite();
    if (!fn.is_valid()) {
        return 0.0f;
    }
    // GetNoise expects float x, y. If your scale differs, adjust accordingly.
    return fn->get_noise_2d(x, y);
}

SectionID ChunkGenerator::get_section_for_point(float x, float y) const {
    // 1) sample section_noise_tex => [-1..1], map to [0..1]
    float val = get_noise_value(section_noise_tex, x, y);
    val = remap01(val); // now in [0..1]

    // 2) Decide which section
    if (val < 0.33f) {
        return SECTION_1; // Corral + Sand
    } else if (val < 0.66f) {
        return SECTION_2; // Rock + Kelp
    } else {
        return SECTION_3; // Rock + Lavarock
    }
}

float ChunkGenerator::get_height_for_section(SectionID section, float x, float y) const {
    // 1) sample the "blend_noise_tex"
    float blend_val = get_noise_value(blend_noise_tex, x, y); // [-1..1]
    blend_val = remap01(blend_val); // [0..1], 0 => full BiomeA, 1 => full BiomeB

    // 2) pick the two biomes
    float biomeA_val = 0.0f;
    float biomeB_val = 0.0f;

    switch (section) {
        case SECTION_1:
            // corral & sand
            biomeA_val = get_noise_value(corral_noise_tex, x, y);
            biomeB_val = get_noise_value(sand_noise_tex, x, y);
            break;
        case SECTION_2:
            // rock & kelp
            biomeA_val = get_noise_value(rock_noise_tex, x, y);
            biomeB_val = get_noise_value(kelp_noise_tex, x, y);
            break;
        case SECTION_3:
            // rock & lavarock
            biomeA_val = get_noise_value(rock_noise_tex, x, y);
            biomeB_val = get_noise_value(lavarock_noise_tex, x, y);
            break;
    }

    // optional: remap [-1..1] -> [0..1] if you want strictly positive heights
    biomeA_val = remap01(biomeA_val);
    biomeB_val = remap01(biomeB_val);

    // 3) Weighted blend
    float blended = biomeA_val * (1.0f - blend_val) + biomeB_val * blend_val;
    return blended;
}

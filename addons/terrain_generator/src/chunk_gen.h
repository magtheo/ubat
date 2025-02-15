#ifndef CHUNK_GENERATOR_H
#define CHUNK_GENERATOR_H

#include <godot_cpp/classes/ref_counted.hpp>
#include <godot_cpp/classes/noise_texture2d.hpp>
#include <godot_cpp/core/class_db.hpp>
#include <godot_cpp/templates/vector.hpp>
#include <godot_cpp/variant/typed_array.hpp>

namespace godot {

enum SectionID {
    SECTION_1 = 0, // (Corral, Sand)
    SECTION_2,     // (Rock, Kelp)
    SECTION_3      // (Rock, Lavarock)
};

class ChunkGenerator : public RefCounted {
    GDCLASS(ChunkGenerator, RefCounted);

protected:
    static void _bind_methods();

private:
    // We store references to NoiseTexture2D for each biome
    // and also for the "section noise" and "blend noise."
    Ref<NoiseTexture2D> corral_noise_tex;
    Ref<NoiseTexture2D> sand_noise_tex;
    Ref<NoiseTexture2D> rock_noise_tex;
    Ref<NoiseTexture2D> kelp_noise_tex;
    Ref<NoiseTexture2D> lavarock_noise_tex;

    Ref<NoiseTexture2D> section_noise_tex; // picks which section
    Ref<NoiseTexture2D> blend_noise_tex;   // blends the 2 biomes in a section

    int chunk_size = 64;

public:
    ChunkGenerator();
    ~ChunkGenerator();

    // Initialize with paths to your .tres resources and a master seed
    void initialize(
        const String corral_path,
        const String sand_path,
        const String rock_path,
        const String kelp_path,
        const String lavarock_path,
        const String section_path,
        const String blend_path,
        int p_chunk_size,
        int p_seed
    );

    // Generates a chunk's height data
    TypedArray<float> generate_chunk(int cx, int cy);

private:
    // Helper to get the "fast noise lite" from a NoiseTexture2D
    float get_noise_value(const Ref<NoiseTexture2D> &tex, float x, float y) const;

    // Convert noise output from [-1..1] to [0..1]
    inline float remap01(float val) const { return (val + 1.0f) * 0.5f; }

    // Decide which section for point (x, y)
    SectionID get_section_for_point(float x, float y) const;

    // Blend the 2 biomes for a given section
    float get_height_for_section(SectionID section, float x, float y) const;
};

} // namespace godot

#endif

#ifndef CHUNK_GENERATOR_H
#define CHUNK_GENERATOR_H

#include <godot_cpp/classes/ref_counted.hpp>
#include <godot_cpp/classes/node.hpp>
#include <godot_cpp/core/class_db.hpp>
#include <godot_cpp/templates/vector.hpp>
#include <godot_cpp/variant/typed_array.hpp>
#include <godot_cpp/variant/dictionary.hpp>

namespace godot {

class ChunkGenerator : public RefCounted {
    GDCLASS(ChunkGenerator, RefCounted);

protected:
    static void _bind_methods();

private:
    // 🎯 Reference to BiomeManager (GDScript)
    Object *biome_manager = nullptr;

    // Chunk settings
    int chunk_size = 64;

public:
    ChunkGenerator();
    ~ChunkGenerator();

    // Initialize with BiomeManager and chunk settings
    void initialize(Object *p_biome_manager, int p_chunk_size);

    // Generate chunk data
    TypedArray<float> generate_chunk(int cx, int cy);

private:
    // Helper to call BiomeManager's GDScript methods
    String get_section_name(float x, float y) const;
    Dictionary get_biome_blend(float x, float y) const;

    // Height calculation
    float get_height_for_position(float x, float y) const;

    // Convert noise output from [-1..1] to [0..1]
    inline float remap01(float val) const { return (val + 1.0f) * 0.5f; }
};

} // namespace godot

#endif

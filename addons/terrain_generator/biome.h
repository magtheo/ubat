#ifndef BIOME_H
#define BIOME_H

#include <godot_cpp/classes/fast_noise_lite.hpp>
#include <godot_cpp/godot.hpp>

using namespace godot;

const int BIOME_SCALE = 100;

enum BiomeType {
    CORAL_REEF,
    SANDY_BOTTOM,
    ROCKY_OUTCROP,
    KELP_FOREST,
    VOLCANIC_VENT
};

class BiomeData: public RefCounted {
    GDCLASS(BiomeData, RefCounted);

private:
    Ref<FastNoiseLite> noise;
    Ref<FastNoiseLite> weight_noise;
    float height_multiplier;
    float blend_start;
    float blend_end;

public:
    BiomeData(Ref<FastNoiseLite> terrain_noise, Ref<FastNoiseLite> blend_noise,
              float h_mult, float b_start, float b_end);

    Ref<FastNoiseLite> get_noise() const;
    Ref<FastNoiseLite> get_weight_noise() const;
    float get_height_multiplier() const;
    float get_blend_start() const;
    float get_blend_end() const;
};

#endif // BIOME_H
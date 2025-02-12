#include "biome.h"

BiomeData::BiomeData(Ref<FastNoiseLite> terrain_noise, Ref<FastNoiseLite> blend_noise,
                     float h_mult, float b_start, float b_end) {
    noise = terrain_noise;
    weight_noise = blend_noise;
    height_multiplier = h_mult;
    blend_start = b_start;
    blend_end = b_end;
}

Ref<FastNoiseLite> BiomeData::get_noise() const { 
    return noise; 
}

Ref<FastNoiseLite> BiomeData::get_weight_noise() const { 
    return weight_noise; 
}

float BiomeData::get_height_multiplier() const { 
    return height_multiplier; 
}

float BiomeData::get_blend_start() const { 
    return blend_start; 
}

float BiomeData::get_blend_end() const { 
    return blend_end; 
}
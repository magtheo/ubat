#include "FastNoiseLiteWrapper.hpp"
#include <cmath>
#include <cstdlib>

FastNoiseLiteWrapper::FastNoiseLiteWrapper() {
    initialize_biomes();
}

void FastNoiseLiteWrapper::initialize_biomes() {
    // Initialize noise generators for each biome.
    // Note: Adjust biome names as necessary.
    m_biomeNoises["Corral"] = FastNoiseLite();
    m_biomeNoises["Sand"] = FastNoiseLite();
    m_biomeNoises["Rock"] = FastNoiseLite();
    m_biomeNoises["Kelp"] = FastNoiseLite();
    m_biomeNoises["Lavarock"] = FastNoiseLite();
    // m_blendNoise and m_bossNoise are already member variables.
}

void FastNoiseLiteWrapper::set_seed(int seed) {
    // Set seed for blending and boss noise.
    m_blendNoise.setSeed(seed);
    m_bossNoise.setSeed(seed + 1000);
}

void FastNoiseLiteWrapper::randomize_seeds(int seed) {
    // Set seed for each biome noise by offsetting the given seed.
    for (auto &pair : m_biomeNoises) {
        pair.second.setSeed(seed + std::hash<std::string>{}(pair.first) % 1000);
    }
    // Also randomize the blending noise and boss noise.
    m_blendNoise.setSeed(seed + 500);
    m_bossNoise.setSeed(seed + 1000);
}

float FastNoiseLiteWrapper::get_noise_2d(const std::string &biome, float x, float y) {
    // Look up the noise generator for this biome.
    if (m_biomeNoises.find(biome) != m_biomeNoises.end()) {
        return m_biomeNoises[biome].getNoise(x, y);
    }
    // Fallback: if the biome is not found, return zero.
    return 0.0f;
}

float FastNoiseLiteWrapper::get_blending_noise(float x, float y) {
    return m_blendNoise.getNoise(x, y);
}

float FastNoiseLiteWrapper::get_boss_noise(float x, float y) {
    return m_bossNoise.getNoise(x, y);
}

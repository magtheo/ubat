#ifndef FAST_NOISE_LITE_WRAPPER_HPP
#define FAST_NOISE_LITE_WRAPPER_HPP

#include <cmath>
#include <string>
#include <map>

// A stub class representing a noise generator. In your actual project,
// this would wrap FastNoiseLite functionality.
class FastNoiseLite {
public:
    FastNoiseLite() : m_seed(0) {}
    void setSeed(int seed) { m_seed = seed; }
    // Dummy noise function – replace with FastNoiseLite noise generation.
    float getNoise(float x, float y) {
        // Simple pseudo-noise example. In practice, use FastNoiseLite's API.
        return static_cast<float>(sin(x * 0.1 + m_seed) * cos(y * 0.1 + m_seed));
    }
private:
    int m_seed;
};

//
// FastNoiseLiteWrapper manages multiple noise instances – one per biome,
// one for blending noise, and one for boss noise.
//
class FastNoiseLiteWrapper {
public:
    FastNoiseLiteWrapper();

    // Set a global seed (if needed)
    void set_seed(int seed);

    // Randomize seeds for all noise instances.
    void randomize_seeds(int seed);

    // Get noise value for a given biome.
    float get_noise_2d(const std::string &biome, float x, float y);

    // Get the blending noise value.
    float get_blending_noise(float x, float y);

    // Get the boss area noise value.
    float get_boss_noise(float x, float y);

private:
    // Store noise generators for each biome.
    std::map<std::string, FastNoiseLite> m_biomeNoises;
    // Separate noise generator for biome blending.
    FastNoiseLite m_blendNoise;
    // Separate noise generator for boss area.
    FastNoiseLite m_bossNoise;

    // Helper to initialize biome noise generators.
    void initialize_biomes();
};

#endif // FAST_NOISE_LITE_WRAPPER_HPP

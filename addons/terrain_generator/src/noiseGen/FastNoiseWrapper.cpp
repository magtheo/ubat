#include "FastNoiseLiteWrapper.hpp"

FastNoiseLiteWrapper::FastNoiseLiteWrapper() {
    // You can set default parameters here if desired.
    noise.SetSeed(1337);
    noise.SetNoiseType(FastNoiseLite::NoiseType_OpenSimplex2);
}

FastNoiseLiteWrapper::~FastNoiseLiteWrapper() {
    // No dynamic memory to free in this simple example.
}

void FastNoiseLiteWrapper::set_seed(int seed) {
    noise.SetSeed(seed);
}

float FastNoiseLiteWrapper::get_noise_2d(float x, float y) {
    // Return the noise value for the given coordinates.
    return noise.GetNoise(x, y);
}

void FastNoiseLiteWrapper::_bind_methods() {
    ClassDB::bind_method(D_METHOD("set_seed", "seed"), &FastNoiseLiteWrapper::set_seed);
    ClassDB::bind_method(D_METHOD("get_noise_2d", "x", "y"), &FastNoiseLiteWrapper::get_noise_2d);
}

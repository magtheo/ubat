#ifndef FAST_NOISE_LITE_WRAPPER_HPP
#define FAST_NOISE_LITE_WRAPPER_HPP

#include <godot_cpp/classes/object.hpp>
#include <godot_cpp/core/class_db.hpp>
#include "../thirdParty/FastNoiseLite.h"  // Adjust the path as needed

using namespace godot;

class FastNoiseLiteWrapper : public Object {
    GDCLASS(FastNoiseLiteWrapper, Object);

private:
    // Instance of FastNoiseLite
    FastNoiseLite noise;

protected:
    static void _bind_methods();

public:
    FastNoiseLiteWrapper();
    ~FastNoiseLiteWrapper();

    // Set the seed for the noise generator.
    void set_seed(int seed);

    // Example: Get 2D noise at coordinates (x, y).
    float get_noise_2d(float x, float y);

    // You can add more methods as needed to expose additional functionality.
};

#endif // FAST_NOISE_LITE_WRAPPER_HPP

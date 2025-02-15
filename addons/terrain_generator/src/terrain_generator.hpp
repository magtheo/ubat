#ifndef TERRAIN_GENERATOR_HPP
#define TERRAIN_GENERATOR_HPP

#include <godot_cpp/classes/array_mesh.hpp>
#include <godot_cpp/classes/surface_tool.hpp>
#include "noiseGen/FastNoiseLiteWrapper.hpp"
#include <godot_cpp/godot.hpp>
#include <vector>

namespace TerrainGenerator {

using namespace godot;

class TerrainGenerator : public Object {
    GDCLASS(TerrainGenerator, Object);

private:
    std::vector<Ref<FastNoiseLiteWrapper>> noise_generators;

protected:
    static void _bind_methods();

public:
    TerrainGenerator();
    ~TerrainGenerator();

    void set_noise_generator(Ref<FastNoiseLiteWrapper> noise, int index);
    Array generate_chunk_data(int x, int z, int biome_index);
    Ref<ArrayMesh> generate_chunk_mesh(Array height_data);
};

} // namespace TerrainGenerator

#endif // TERRAIN_GENERATOR_HPP

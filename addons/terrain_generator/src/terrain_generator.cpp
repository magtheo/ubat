#include <godot_cpp/classes/array_mesh.hpp>
#include <godot_cpp/classes/surface_tool.hpp>
#include "noiseGen/FastNoiseLiteWrapper.hpp"
#include <godot_cpp/godot.hpp>
using namespace godot;

namespace TerrainGenerator { 
class TerrainGenerator : public Object {
    GDCLASS(TerrainGenerator, Object);

private:
    OpenSimplexNoise *noise;

public:
    static void _bind_methods() {
        ClassDB::bind_method(D_METHOD("generate_chunk_data", "x", "z"), &TerrainGenerator::generate_chunk_data);
        ClassDB::bind_method(D_METHOD("generate_chunk_mesh", "height_data"), &TerrainGenerator::generate_chunk_mesh);
    }

    TerrainGenerator() {
        noise = memnew(OpenSimplexNoise);
        noise->set_seed(12345);
        noise->set_octaves(4);
        noise->set_period(20.0);
        noise->set_persistence(0.5);
    }

    ~TerrainGenerator() {
        memdelete(noise);
    }

    Array generate_chunk_data(int x, int z) {
        Array height_data;
        int chunk_size = 32;

        for (int i = 0; i < chunk_size; i++) {
            for (int j = 0; j < chunk_size; j++) {
                float height = noise->get_noise_2d(x + i, z + j) * 10.0;
                height_data.append(height);
            }
        }

        return height_data;
    }

    Ref<ArrayMesh> generate_chunk_mesh(Array height_data) {
        int chunk_size = 32;
        Ref<ArrayMesh> mesh;
        mesh.instantiate();
        Ref<SurfaceTool> st;
        st.instantiate();

        st->begin(Mesh::PRIMITIVE_TRIANGLES);

        for (int i = 0; i < chunk_size - 1; i++) {
            for (int j = 0; j < chunk_size - 1; j++) {
                Vector3 v1(i, height_data[i * chunk_size + j], j);
                Vector3 v2(i + 1, height_data[(i + 1) * chunk_size + j], j);
                Vector3 v3(i, height_data[i * chunk_size + (j + 1)], j + 1);
                Vector3 v4(i + 1, height_data[(i + 1) * chunk_size + (j + 1)], j + 1);

                st->add_vertex(v1);
                st->add_vertex(v2);
                st->add_vertex(v3);

                st->add_vertex(v2);
                st->add_vertex(v4);
                st->add_vertex(v3);
            }
        }

        st->commit(mesh);
        return mesh;
    }
};
}
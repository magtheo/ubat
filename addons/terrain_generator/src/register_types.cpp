#include <godot.hpp>
#include "chunkGen/chunk_generator.hpp"

using namespace godot;

extern "C" void GDN_EXPORT godot_nativescript_init(void *handle) {
    Godot::nativescript_init(handle);
    register_class<ChunkGenerator>();
}

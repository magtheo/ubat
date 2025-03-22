#include "register_types.h"
#include "./chunkGen/chunk_generator.hpp"  // Ensure this path is correct
#include <gdextension_interface.h>
#include <godot_cpp/core/defs.hpp>
#include <godot_cpp/godot.hpp>
#include <godot_cpp/classes/engine.hpp>

using namespace godot;

void initialize_chunk_generator_module(ModuleInitializationLevel p_level) {
    if (p_level != MODULE_INITIALIZATION_LEVEL_SCENE) {
        return;
    }

    // ClassDB::register_class<BiomeManager>();
    Engine::get_singleton();

    godot::print_line("Initializing: ChunkGenerator -> " + String(typeid(ChunkGenerator).name()));
    // GDREGISTER_CLASS(ChunkGenerator);
    ClassDB::register_class<ChunkGenerator>();
}

void uninitialize_chunk_generator_module(ModuleInitializationLevel p_level) {
    if (p_level != MODULE_INITIALIZATION_LEVEL_SCENE) {
        return;
    }
}

extern "C" {

// Initialization.
GDExtensionBool GDE_EXPORT chunk_generator_library_init(
    GDExtensionInterfaceGetProcAddress p_get_proc_address,
    const GDExtensionClassLibraryPtr p_library,
    GDExtensionInitialization *r_initialization) {
    
    godot::GDExtensionBinding::InitObject init_obj(p_get_proc_address, p_library, r_initialization);

    init_obj.register_initializer(initialize_chunk_generator_module);
    init_obj.register_terminator(uninitialize_chunk_generator_module);
    init_obj.set_minimum_library_initialization_level(MODULE_INITIALIZATION_LEVEL_SCENE);

    return init_obj.init();
}

}

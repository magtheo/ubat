use godot::prelude::*;

mod core;
mod networking;
mod resource;
mod terrain;

// Bring your main classes into scope.
use core::game_manager::game_manager;
use networking::network_manager::network_manager;
use networking::object_manager::object_manager;
use networking::physics_manager::physics_manager;
use networking::world_manager::world_manager;
use resource::resource_manager::resource_manager;
use terrain::terrain_manager::terrain_manager;

// The entry point of your extension library.
struct UbatExtension;

#[gdextension]
unsafe impl ExtensionLibrary for UbatExtension {
    fn on_level_init(level: InitLevel) {
        if level == InitLevel::Scene {
            // Register all your classes here
            godot::register_class::<game_manager>();
            godot::register_class::<network_manager>();
            godot::register_class::<object_manager>();
            godot::register_class::<physics_manager>();
            godot::register_class::<world_manager>();
            godot::register_class::<resource_manager>();
            godot::register_class::<terrain_manager>();
        }
    }
}

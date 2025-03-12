use godot::prelude::*;

mod core;
mod networking;
mod resource;
mod terrain;

// Bring your main classes into scope.
use core::game_manager::GameManager;
use Networking::network_manager::NetworkManager;
use Networking::object_manager::ObjectManager;
use Networking::physics_manager::PhysicsManager;
use Networking::world_manager::WorldManager;
use Resource::resource_manager::ResourceManager;
use Terrain::terrain_manager::TerrainManager;

// The entry point of your extension library.
struct UbatExtension;

#[gdextension]
unsafe impl ExtensionLibrary for UbatExtension {
    fn on_level_init(level: InitLevel) {
        if level == InitLevel::Scene {
            // Register all your classes here
            godot::register_class::<GameManager>();
            godot::register_class::<NetworkManager>();
            godot::register_class::<ObjectManager>();
            godot::register_class::<PhysicsManager>();
            godot::register_class::<WorldManager>();
            godot::register_class::<ResourceManager>();
            godot::register_class::<TerrainManager>();
        }
    }
}

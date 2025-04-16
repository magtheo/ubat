// Bridge module for Godot integration

// Re-export bridge components
// pub use self::config_bridge::ConfigBridge;
pub use self::event_bridge::{EventBridge, EventData};
pub use self::game_bridge::GameManagerBridge;
pub use self::network_bridge::NetworkManagerBridge;
pub use self::game_init_helper::GameInitHelper;
pub use self::terrain_bridge::TerrainBridge;
// pub use self::world_bridge::WorldManagerBridge;

// Internal modules (keep the same order as re-exports)
// mod config_bridge;
mod event_bridge;
mod game_bridge;
mod game_init_helper;
mod network_bridge;
mod terrain_bridge;

// Optional: Rename modules for clearer importing
// pub mod config {
//     pub use super::config_bridge::*;
// }

pub mod event {
    pub use super::event_bridge::*;
}

pub mod game {
    pub use super::game_bridge::*;
}

pub mod network {
    pub use super::network_bridge::*;
}

pub mod terrain {
    pub use super::terrain_bridge::*;
}
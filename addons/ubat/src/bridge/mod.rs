// Bridge module for Godot integration

// Re-export bridge components
pub use self::config_bridge::ConfigBridge;
pub use self::event_bridge::{EventBridge, EventData};
pub use self::game_bridge::GameManagerBridge;
// pub use self::network_bridge::NetworkManagerBridge;
// pub use self::world_bridge::WorldManagerBridge;

// Internal modules (keep the same order as re-exports)
mod config_bridge;
mod event_bridge;
mod game_bridge;
// mod network_bridge;
// mod world_bridge;

// Optional: Rename modules for clearer importing
pub mod config {
    pub use super::config_bridge::*;
}

pub mod event {
    pub use super::event_bridge::*;
}

pub mod game {
    pub use super::game_bridge::*;
}

// pub mod network {
//     pub use super::network_bridge::*;
// }

// pub mod world {
//     pub use super::world_bridge::*;
// }
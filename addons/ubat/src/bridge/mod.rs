// This module exports all bridge components for the Godot integration
// This file serves as a central place to organize the bridge components

// Re-export individual bridges
pub use self::event_bridge::EventBridge;
pub use self::game_bridge::GameManagerBridge;
pub use self::config_bridge::ConfigBridge;
pub use self::world_bridge::WorldManagerBridge;
pub use self::network_bridge::NetworkManagerBridge;

// Also export the EventData resource for structured event information
pub use self::event_bridge::EventData;

// Internal modules
mod event_bridge;
mod game_bridge;
mod config_bridge;
mod world_bridge;
mod network_bridge;

// Rename the modules for clarity when imported
pub mod event_bridge {
    pub use super::event_bridge::*;
}

pub mod game_bridge {
    pub use super::game_bridge::*;
}

pub mod config_bridge {
    pub use super::config_bridge::*;
}

pub mod world_bridge {
    pub use super::world_bridge::*;
}

pub mod network_bridge {
    pub use super::network_bridge::*;
}
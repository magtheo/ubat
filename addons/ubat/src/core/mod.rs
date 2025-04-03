pub mod game_manager;
pub mod event_bus;
pub mod config_manager;
pub mod world_manager;
pub mod initialization;

pub use initialization::core_system_initializer;
pub use event_bus::EventBus;
pub use game_manager::GameManager;

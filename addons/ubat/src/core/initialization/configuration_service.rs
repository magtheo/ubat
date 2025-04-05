use std::sync::{Arc, Mutex};

use godot::prelude::*;

use crate::core::config_manager::{ConfigurationManager, GameConfiguration, GameModeConfig};
use crate::core::game_manager::GameManager;
use crate::networking::network_manager::{NetworkHandler, NetworkConfig, NetworkMode};
use crate::core::world_manager::WorldStateManager;
use crate::core::event_bus::EventBus;

/// Configuration service to centralize game initialization logic
pub struct ConfigurationService {
    game_manager: Arc<Mutex<GameManager>>,
    config_manager: Arc<Mutex<ConfigurationManager>>,
    network_handler: Arc<Mutex<NetworkHandler>>,
    world_manager: Arc<Mutex<WorldStateManager>>,
    event_bus: Arc<EventBus>,
}

impl ConfigurationService {
    /// Create a new configuration service with all dependencies
    pub fn new(
        game_manager: Arc<Mutex<GameManager>>,
        config_manager: Arc<Mutex<ConfigurationManager>>,
        network_handler: Arc<Mutex<NetworkHandler>>,
        world_manager: Arc<Mutex<WorldStateManager>>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        Self {
            game_manager,
            config_manager,
            network_handler,
            world_manager,
            event_bus,
        }
    }

    /// Configure the game based on the network mode and options
    pub fn configure(&mut self, options: &Dictionary) -> Result<(), String> {
        // Extract network mode from options
        let network_mode = options.get("network_mode")
            .and_then(|v| v.try_to::<i64>().ok())
            .map(|mode| match mode {
                0 => NetworkMode::Standalone,
                1 => NetworkMode::Host,
                2 => NetworkMode::Client,
                _ => NetworkMode::Standalone, // Default fallback
            })
            .unwrap_or(NetworkMode::Standalone);

        // Update configuration manager
        self.update_configuration(network_mode, options)?;

        // Configure network handler
        self.configure_network(network_mode, options)?;

        // Initialize world
        self.initialize_world(network_mode, options)?;

        // Mark game manager as initialized
        self.finalize_initialization(network_mode)?;

        Ok(())
    }

    /// Update configuration based on mode and options
    fn update_configuration(&mut self, mode: NetworkMode, options: &Dictionary) -> Result<(), String> {
        let mut config_manager = self.config_manager.lock()
            .map_err(|_| "Failed to lock config manager".to_string())?;

        // Get current configuration
        let mut config = config_manager.get_config().clone();

        // Update configuration based on options
        config.game_mode = match mode {
            NetworkMode::Standalone => GameModeConfig::Standalone,
            NetworkMode::Host => GameModeConfig::Host(config_manager::HostConfig {
                world_generation_seed: options.get("world_seed")
                    .and_then(|v| v.try_to::<i64>().ok())
                    .unwrap_or_else(|| rand::random()) as u64,
                admin_password: options.get("admin_password")
                    .and_then(|v| v.try_to::<GString>().ok())
                    .map(|s| s.to_string()),
            }),
            NetworkMode::Client => GameModeConfig::Client(config_manager::ClientConfig {
                server_address: options.get("server_address")
                    .and_then(|v| v.try_to::<GString>().ok())
                    .unwrap_or_else(|| "127.0.0.1:7878".to_string()),
                username: options.get("player_name")
                    .and_then(|v| v.try_to::<GString>().ok())
                    .unwrap_or_else(|| "Player".to_string()),
            }),
        };

        // Save updated configuration
        config_manager.update_config(config);

        Ok(())
    }

    /// Configure network handler based on mode
    fn configure_network(&mut self, mode: NetworkMode, options: &Dictionary) -> Result<(), String> {
        let mut network_handler = self.network_handler.lock()
            .map_err(|_| "Failed to lock network handler".to_string())?;

        // Configure network based on mode
        let network_config = match mode {
            NetworkMode::Standalone => NetworkConfig::Standalone,
            NetworkMode::Host => NetworkConfig::Host {
                port: options.get("server_port")
                    .and_then(|v| v.try_to::<i64>().ok())
                    .unwrap_or(7878) as u16,
                max_players: options.get("max_players")
                    .and_then(|v| v.try_to::<i64>().ok())
                    .unwrap_or(64) as u32,
            },
            NetworkMode::Client => NetworkConfig::Client {
                server_address: options.get("server_address")
                    .and_then(|v| v.try_to::<GString>().ok())
                    .unwrap_or_else(|| "127.0.0.1:7878".to_string()),
            },
        };

        // Apply network configuration
        network_handler.configure(network_config)
            .map_err(|e| format!("Network configuration failed: {:?}", e))?;

        Ok(())
    }

    /// Initialize world based on mode and options
    fn initialize_world(&mut self, mode: NetworkMode, options: &Dictionary) -> Result<(), String> {
        let mut world_manager = self.world_manager.lock()
            .map_err(|_| "Failed to lock world manager".to_string())?;

        // World initialization parameters
        let seed = options.get("world_seed")
            .and_then(|v| v.try_to::<i64>().ok())
            .unwrap_or_else(|| rand::random());

        let width = options.get("world_width")
            .and_then(|v| v.try_to::<i64>().ok())
            .unwrap_or(10000);

        let height = options.get("world_height")
            .and_then(|v| v.try_to::<i64>().ok())
            .unwrap_or(10000);

        // Initialize world based on mode
        match mode {
            NetworkMode::Standalone | NetworkMode::Host => {
                world_manager.initialize_with_params(seed, width, height)
                    .map_err(|e| format!("World initialization failed: {}", e))?;
                
                // Generate initial world for standalone and host modes
                world_manager.generate_initial_world();
            },
            NetworkMode::Client => {
                // For client, just initialize (wait for world sync from host)
                world_manager.initialize()
                    .map_err(|e| format!("World initialization failed: {}", e))?;
            }
        }

        Ok(())
    }

    /// Finalize initialization by marking game manager
    fn finalize_initialization(&mut self, mode: NetworkMode) -> Result<(), String> {
        let mut game_manager = self.game_manager.lock()
            .map_err(|_| "Failed to lock game manager".to_string())?;

        // Mark as initialized and transition to appropriate state
        game_manager.mark_initialized();

        // Optional: Publish initialization event
        self.event_bus.publish(crate::core::game_manager::GameEvent::StateChanged(
            match mode {
                NetworkMode::Standalone => crate::core::game_manager::GameState::Running,
                NetworkMode::Host => crate::core::game_manager::GameState::Running,
                NetworkMode::Client => crate::core::game_manager::GameState::Loading,
            }
        ));

        Ok(())
    }
}
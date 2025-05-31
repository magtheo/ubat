// File: configuration_service.rs

use std::sync::{Arc, Mutex, RwLock};
use godot::prelude::*;

use crate::config::config_manager::{self, ConfigurationManager, GameConfiguration, GameModeConfig, ClientConfig};
use crate::core::game_manager::GameManager;
use crate::networking::network_manager::{NetworkHandler, NetworkConfig, NetworkMode};
use crate::core::world_manager::WorldStateManager;
use crate::core::event_bus::EventBus;
use godot::classes::RandomNumberGenerator;

/// Configuration service to centralize game initialization logic
pub struct ConfigurationService {
    game_manager: Arc<Mutex<GameManager>>,
    config_manager: Arc<RwLock<ConfigurationManager>>,
    network_handler: Arc<Mutex<NetworkHandler>>,
    world_manager: Arc<Mutex<WorldStateManager>>,
    event_bus: Arc<EventBus>,
    rng: Gd<RandomNumberGenerator>,
}

impl ConfigurationService {
    /// Create a new configuration service with all dependencies
    pub fn new(
        game_manager: Arc<Mutex<GameManager>>,
        config_manager: Arc<RwLock<ConfigurationManager>>,
        network_handler: Arc<Mutex<NetworkHandler>>,
        world_manager: Arc<Mutex<WorldStateManager>>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        // Create and initialize the random number generator
        let mut rng = RandomNumberGenerator::new_gd();
        rng.randomize(); // Initialize with a random seed
        
        Self {
            game_manager,
            config_manager,
            network_handler,
            world_manager,
            event_bus,
            rng,
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
        self.update_configuration(&network_mode, options)?;

        // Configure network handler
        self.configure_network(&network_mode, options)?;

        // Initialize world
        self.initialize_world(&network_mode, options)?;

        // Mark game manager as initialized
        self.finalize_initialization(&network_mode)?;

        Ok(())
    }

    /// Update configuration based on mode and options
    fn update_configuration(&mut self, mode: &NetworkMode, options: &Dictionary) -> Result<(), String> {
        // Lock the global config manager for writing
        let mut config_manager_guard = self.config_manager.write()
            .map_err(|_| "Failed to lock global config manager for writing".to_string())?;

        // Get current configuration
        let config: &mut GameConfiguration = config_manager_guard.get_config_mut();

        // Update configuration based on options
        config.game_mode = match mode {
            NetworkMode::Standalone => GameModeConfig::Standalone,
            NetworkMode::Host => GameModeConfig::Host(config_manager::HostConfig {
                world_generation_seed: options.get("world_seed")
                    .and_then(|v| v.try_to::<i64>().ok().map(|s| s as u64))
                    .unwrap_or(config.world_seed), // Fallback to existing seed
                admin_password: options.get("admin_password")
                    .and_then(|v| v.try_to::<GString>().ok())
                    .map(|s| s.to_string()),
            }),
            NetworkMode::Client => GameModeConfig::Client(ClientConfig {
                // Use address from options if present, otherwise keep loaded/default
                server_address: options.get("server_address")
                     .and_then(|v| v.try_to::<GString>().ok().map(|s| s.to_string()))
                     .unwrap_or_else(|| crate::config::config_manager::default_server_address()), // Use pub function
                // CORRECT THE FIELD NAME AND ASSIGNMENT HERE
                username: options.get("player_name") // Ensure this key is correct
                     .and_then(|v| v.try_to::<GString>().ok().map(|s| s.to_string()))
                     .unwrap_or_else(|| crate::config::config_manager::default_username()), // Use pub function
            }),
        };

        // Update other config fields directly if needed based on options
        // Example: Override world seed for this session if provided in options
        if let Some(seed_variant) = options.get("world_seed") {
            if let Ok(seed) = seed_variant.try_to::<i64>() {
                config.world_seed = seed as u64;
                godot_print!("ConfigurationService: Overriding world seed for session: {}", config.world_seed);
            }
        }
         // Example: Override world size for this session if provided
         if let Some(width_v) = options.get("world_width") {
              if let Ok(width) = width_v.try_to::<i64>() { config.world_size.width = width as u32; }
         }
         if let Some(height_v) = options.get("world_height") {
              if let Ok(height) = height_v.try_to::<i64>() { config.world_size.height = height as u32; }
         }

        Ok(())
    }

    /// Configure network handler based on mode
    fn configure_network(&mut self, mode: &NetworkMode, options: &Dictionary) -> Result<(), String> {
        let mut network_handler_guard = self.network_handler.lock()
            .map_err(|_| "Failed to lock network handler".to_string())?;

        // Get defaults from the loaded config (read lock)
        let (default_port, default_max_players, default_server_address) = {
            let config_manager_guard = self.config_manager.read()
                .map_err(|_| "Failed to lock global config manager for reading network defaults".to_string())?;
            let net_config = &config_manager_guard.get_config().network;
            (
                net_config.default_port,
                net_config.max_players as usize, // Cast u8 to usize
                // Determine default address - maybe ClientConfig default is better?
                 match &config_manager_guard.get_config().game_mode {
                      GameModeConfig::Client(c) => Some(c.server_address.clone()),
                      _ => None,
                 }
            )
        };

        // Configure network based on mode, using options OR loaded defaults
        let network_runtime_config = match mode {
            NetworkMode::Standalone => NetworkConfig { mode: NetworkMode::Standalone, port: 0, max_connections: 0, server_address: None },
            NetworkMode::Host => NetworkConfig {
                mode: NetworkMode::Host,
                port: options.get("server_port")
                    .and_then(|v| v.try_to::<i64>().ok().map(|p| p as u16))
                    .unwrap_or(default_port), // Use loaded default port
                max_connections: options.get("max_players")
                    .and_then(|v| v.try_to::<i64>().ok().map(|p| p as usize))
                    .unwrap_or(default_max_players), // Use loaded default players
                server_address: None,
            },
            NetworkMode::Client => NetworkConfig {
                mode: NetworkMode::Client,
                port: 0,
                max_connections: 1, // Client only connects to one server
                server_address: Some(
                    options.get("server_address")
                        .and_then(|v| v.try_to::<GString>().ok().map(|s| s.to_string()))
                        .or(default_server_address) // Use loaded default if option missing
                        .unwrap_or_else(|| { // Final fallback
                            godot_warn!("ConfigurationService: Client server address not found in options or config, using fallback.");
                            "127.0.0.1:7878".to_string()
                        })
                ),
            },
        };

        // Re-initialize the NetworkHandler with the determined runtime config
        // Note: This creates a *new* handler. Ensure this is the desired behavior.
        // If NetworkHandler has a reconfigure method, use that instead.
        godot_print!("ConfigurationService: Configuring NetworkHandler with: {:?}", network_runtime_config);
        *network_handler_guard = NetworkHandler::new(network_runtime_config)
            .map_err(|e| format!("Network configuration failed: {:?}", e))?;

        Ok(())
    }

    /// Initialize world based on mode and options
    fn initialize_world(&mut self, mode: &NetworkMode, _options: &Dictionary) -> Result<(), String> {
        // Options are already applied to the global config in update_configuration
        let mut world_manager_guard = self.world_manager.lock()
            .map_err(|_| "Failed to lock world manager".to_string())?;

        // Get final world parameters from the possibly-updated global config
        let (final_seed, final_width, final_height) = {
            let config_manager_guard = self.config_manager.read()
                 .map_err(|_| "Failed to lock global config manager for world init".to_string())?;
             let config = config_manager_guard.get_config();
             (config.world_seed, config.world_size.width, config.world_size.height)
        };

        // Update world manager's internal configuration before initialization
        // Assuming WorldStateManager has a method like update_config or similar
        let mut world_state_config = world_manager_guard.get_config().clone(); // Clone existing config
        world_state_config.seed = final_seed;
        world_state_config.world_size = (final_width, final_height);
        // Update generation parameters if they could be changed by options? Seems unlikely for now.
        world_manager_guard.update_config(world_state_config); // Assuming this method exists

        godot_print!("ConfigurationService: Initializing WorldStateManager with Seed: {}, Size: ({}, {})", final_seed, final_width, final_height);

        // Initialize world based on mode (no changes here)
        match mode {
            NetworkMode::Standalone | NetworkMode::Host => { /* ... */ },
            NetworkMode::Client => { /* ... */ }
        }
        // Call the actual initialization method on WorldStateManager
        world_manager_guard.initialize()
             .map_err(|e| format!("World initialization failed: {}", e))?;

        godot_print!("World initialized");
        Ok(())
    }

    /// Finalize initialization by marking game manager
    fn finalize_initialization(&mut self, mode: &NetworkMode) -> Result<(), String> {
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
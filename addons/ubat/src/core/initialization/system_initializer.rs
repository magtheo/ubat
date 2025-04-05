use godot::prelude::*;
use std::error::Error;
use std::fmt;
use std::result::Result;
use std::sync::{Arc, Mutex, Once, OnceLock};

use crate::bridge::config::ConfigBridge;
use crate::bridge::game::GameManagerBridge;
use crate::bridge::network::NetworkManagerBridge;
use crate::bridge::event::EventBridge;

// Import your managers as Rust modules
use crate::core::config_manager;
use crate::core::event_bus;
use crate::core::game_manager;
use crate::core::world_manager::{WorldStateManager, WorldStateConfig};
use crate::networking::network_manager::{NetworkHandler, NetworkConfig, NetworkMode};

// Import the new configuration service
use crate::core::initialization::configuration_service::ConfigurationService;

// Custom error type for system initialization
#[derive(Debug)]
pub enum SystemInitError {
    ConfigError(String),
    NetworkError(String),
    GameError(String),
    BridgeError(String),
    ManagerError(String),
}

impl fmt::Display for SystemInitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SystemInitError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            SystemInitError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            SystemInitError::GameError(msg) => write!(f, "Game error: {}", msg),
            SystemInitError::BridgeError(msg) => write!(f, "Bridge error: {}", msg),
            SystemInitError::ManagerError(msg) => write!(f, "Manager error: {}", msg),
        }
    }
}

impl Error for SystemInitError {}

// Static singleton instance
static INSTANCE: OnceLock<Mutex<SystemInitializer>> = OnceLock::new();
static INIT: Once = Once::new();

pub struct SystemInitializer {
    // Bridges as Option<Gd> to manage Godot object lifecycle
    game_bridge: Option<Arc<Mutex<Gd<GameManagerBridge>>>>,
    config_bridge: Option<Arc<Mutex<Gd<ConfigBridge>>>>,
    network_bridge: Option<Arc<Mutex<Gd<NetworkManagerBridge>>>>,
    event_bridge: Option<Arc<Mutex<Gd<EventBridge>>>>,
    
    // Configuration service
    configuration_service: Option<ConfigurationService>,
    
    // Core managers with Arc<Mutex> for thread safety
    game_manager: Option<Arc<Mutex<game_manager::GameManager>>>,
    config_manager: Option<Arc<Mutex<config_manager::ConfigurationManager>>>,
    network_manager: Option<Arc<Mutex<NetworkHandler>>>,
    world_manager: Option<Arc<Mutex<WorldStateManager>>>,
    event_bus: Option<Arc<event_bus::EventBus>>,
    
    // Tracks initialization status
    initialized: bool,
}

impl SystemInitializer {
    /// Create a new system initializer
    pub fn new() -> Self {
        Self {
            game_bridge: None,
            config_bridge: None,
            network_bridge: None,
            event_bridge: None,
            
            configuration_service: None,
            
            game_manager: None,
            config_manager: None,
            network_manager: None,
            world_manager: None,
            event_bus: None,
            
            initialized: false,
        }
    }
    
    /// Get or create the singleton instance
    pub fn get_instance() -> &'static Mutex<SystemInitializer> {
        INSTANCE.get_or_init(|| {
            Mutex::new(SystemInitializer::new())
        })
    }
    
    /// Initialize core managers and bridges
    fn initialize_core_systems(&mut self) -> Result<(), SystemInitError> {
        godot_print!("SystemInitializer: Initializing core systems");
        
        // Initialize event bus
        let event_bus = Arc::new(event_bus::EventBus::new());
        self.event_bus = Some(event_bus.clone());
        
        // Initialize configuration manager
        let config_manager = Arc::new(Mutex::new(config_manager::ConfigurationManager::default()));
        self.config_manager = Some(config_manager.clone());
        
        // Prepare default world configuration
        let default_world_config = WorldStateConfig {
            seed: 12345, // Default seed
            world_size: (1024, 1024), // Default world size
            generation_parameters: Default::default(), // Use default generation rules
        };
        
        // Initialize world manager with default configuration
        let world_manager = Arc::new(Mutex::new(WorldStateManager::new_with_dependencies(
            default_world_config,
            Some(event_bus.clone()),
            None // TerrainWorldIntegration will be created later if needed
        )));
        self.world_manager = Some(world_manager.clone());
        
        // Prepare default network configuration
        let default_network_config = NetworkConfig {
            mode: NetworkMode::Standalone,
            port: 0,
            max_connections: 0,
            server_address: None,
        };
        
        // Initialize network manager
        let network_manager = Arc::new(Mutex::new(
            NetworkHandler::new(default_network_config)
                .map_err(|e| SystemInitError::NetworkError(format!("{:?}", e)))?
        ));
        self.network_manager = Some(network_manager.clone());
        
        // Initialize game manager with dependencies
        let game_manager = Arc::new(Mutex::new(game_manager::GameManager::new_with_dependencies(
            config_manager.clone(),
            event_bus.clone(),
            Some(world_manager.clone()),
            Some(network_manager.clone()),
        )));
        self.game_manager = Some(game_manager.clone());
        
        // Initialize bridges (wrap in Arc<Mutex> for thread safety)
        self.event_bridge = Some(Arc::new(Mutex::new(EventBridge::new_alloc())));
        self.config_bridge = Some(Arc::new(Mutex::new(ConfigBridge::new_alloc())));
        self.game_bridge = Some(Arc::new(Mutex::new(GameManagerBridge::new_alloc())));
        self.network_bridge = Some(Arc::new(Mutex::new(NetworkManagerBridge::new_alloc())));
        
        // Create configuration service
        let configuration_service = ConfigurationService::new(
            game_manager.clone(),
            config_manager.clone(),
            network_manager.clone(),
            world_manager.clone(),
            event_bus.clone(),
        );
        self.configuration_service = Some(configuration_service);
        
        godot_print!("SystemInitializer: Core systems initialized");
        Ok(())
    }
    
    /// Initialize the system in standalone mode
    pub fn initialize_standalone(&mut self, options: &Dictionary) -> Result<(), SystemInitError> {
        godot_print!("SystemInitializer: Initializing standalone mode");
        
        // Initialize core systems if not already done
        if !self.initialized {
            self.initialize_core_systems()?;
        }
        
        // Configure system using the configuration service
        if let Some(config_service) = &mut self.configuration_service {
            config_service.configure(options)
                .map_err(|e| SystemInitError::ManagerError(e))?;
        } else {
            return Err(SystemInitError::ManagerError("Configuration service not initialized".into()));
        }
        
        self.initialized = true;
        godot_print!("SystemInitializer: Standalone initialization complete");
        Ok(())
    }
    
    /// Initialize the system in host mode
    pub fn initialize_host(&mut self, options: &Dictionary) -> Result<(), SystemInitError> {
        godot_print!("SystemInitializer: Initializing host mode");
        
        // Initialize core systems if not already done
        if !self.initialized {
            self.initialize_core_systems()?;
        }
        
        // Configure system using the configuration service
        if let Some(config_service) = &mut self.configuration_service {
            config_service.configure(options)
                .map_err(|e| SystemInitError::ManagerError(e))?;
        } else {
            return Err(SystemInitError::ManagerError("Configuration service not initialized".into()));
        }
        
        // Set network mode to Host
        if let Some(network_manager) = &mut self.network_manager {
            let mut manager = network_manager.lock()
                .map_err(|_| SystemInitError::ManagerError("Failed to lock network manager".into()))?;
            
            // Update network mode to Host with port from options
            let port = options.get("port")
                .and_then(|v| v.try_to::<i64>().ok())
                .unwrap_or(7878) as u16;
            
            manager.update_config(NetworkConfig {
                mode: NetworkMode::Host,
                port,
                max_connections: 64,
                server_address: None,
            })?;
        }
        
        self.initialized = true;
        godot_print!("SystemInitializer: Host initialization complete");
        Ok(())
    }
    
    /// Initialize the system in client mode
    pub fn initialize_client(&mut self, options: &Dictionary) -> Result<(), SystemInitError> {
        godot_print!("SystemInitializer: Initializing client mode");
        
        // Initialize core systems if not already done
        if !self.initialized {
            self.initialize_core_systems()?;
        }
        
        // Configure system using the configuration service
        if let Some(config_service) = &mut self.configuration_service {
            config_service.configure(options)
                .map_err(|e| SystemInitError::ManagerError(e))?;
        } else {
            return Err(SystemInitError::ManagerError("Configuration service not initialized".into()));
        }
        
        // Set network mode to Client
        if let Some(network_manager) = &mut self.network_manager {
            let mut manager = network_manager.lock()
                .map_err(|_| SystemInitError::ManagerError("Failed to lock network manager".into()))?;
            
            // Get server address from options
            let server_address = options.get("server_address")
                .and_then(|v| v.try_to::<GString>().ok())
                .unwrap_or("127.0.0.1:7878".to_string());
            
            manager.update_config(NetworkConfig {
                mode: NetworkMode::Client,
                port: 0,
                max_connections: 1,
                server_address: Some(server_address.to_string()),
            })?;
        }
        
        self.initialized = true;
        godot_print!("SystemInitializer: Client initialization complete");
        Ok(())
    }
    
    /// Get the game bridge
    pub fn get_game_bridge(&self) -> Option<Arc<Mutex<Gd<GameManagerBridge>>>> {
        self.game_bridge.clone()
    }
    
    /// Get the config bridge
    pub fn get_config_bridge(&self) -> Option<Arc<Mutex<Gd<ConfigBridge>>>> {
        self.config_bridge.clone()
    }
    
    /// Get the network bridge
    pub fn get_network_bridge(&self) -> Option<Arc<Mutex<Gd<NetworkManagerBridge>>>> {
        self.network_bridge.clone()
    }
    
    /// Get the event bridge
    pub fn get_event_bridge(&self) -> Option<Arc<Mutex<Gd<EventBridge>>>> {
        self.event_bridge.clone()
    }
    
    /// Check if initialization is complete
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
    
    /// Shutdown and clean up all systems
    pub fn shutdown(&mut self) -> Result<(), SystemInitError> {
        godot_print!("SystemInitializer: Shutting down systems");
        
        // Attempt to shutdown each component
        if let Some(game_manager) = &self.game_manager {
            if let Ok(mut manager) = game_manager.lock() {
                manager.shutdown();
            }
        }
        
        if let Some(network_manager) = &self.network_manager {
            if let Ok(mut manager) = network_manager.lock() {
                manager.shutdown()?;
            }
        }
        
        if let Some(world_manager) = &self.world_manager {
            if let Ok(mut manager) = world_manager.lock() {
                manager.shutdown();
            }
        }
        
        if let Some(config_manager) = &self.config_manager {
            if let Ok(mut manager) = config_manager.lock() {
                if let Err(e) = manager.save_to_file() {
                    godot_print!("Failed to save configuration: {:?}", e);
                }
            }
        }
        
        // Explicitly free Godot bridges
        if let Some(game_bridge) = &self.game_bridge {
            if let Ok(bridge) = game_bridge.lock() {
                bridge.free();
            }
        }
        
        if let Some(network_bridge) = &self.network_bridge {
            if let Ok(bridge) = network_bridge.lock() {
                bridge.free();
            }
        }
        
        if let Some(config_bridge) = &self.config_bridge {
            if let Ok(bridge) = config_bridge.lock() {
                bridge.free();
            }
        }
        
        if let Some(event_bridge) = &self.event_bridge {
            if let Ok(bridge) = event_bridge.lock() {
                bridge.free();
            }
        }
        
        // Clear all references
        self.game_bridge = None;
        self.config_bridge = None;
        self.network_bridge = None;
        self.event_bridge = None;
        
        // Reset initialization state
        self.initialized = false;
        
        godot_print!("SystemInitializer: Systems shutdown complete");
        Ok(())
    }
}

// Implement Default for easy initialization
impl Default for SystemInitializer {
    fn default() -> Self {
        Self::new()
    }
}

// Implement Drop trait for cleanup
impl Drop for SystemInitializer {
    fn drop(&mut self) {
        // Attempt to shutdown systems if not already done
        if self.initialized {
            let _ = self.shutdown();
        }
    }
}
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
use crate::core::world_manager;

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
    // Bridges are Godot objects
    game_bridge: Option<Gd<GameManagerBridge>>,
    config_bridge: Option<Gd<ConfigBridge>>,
    network_bridge: Option<Gd<NetworkManagerBridge>>,
    event_bridge: Option<Gd<EventBridge>>,
    
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
            initialized: false,
        }
    }
    
    /// Get or create the singleton instance
    pub fn get_instance() -> &'static Mutex<SystemInitializer> {
        INSTANCE.get_or_init(|| {
            Mutex::new(SystemInitializer::new())
        })
    }
    
    /// Initialize the system in standalone mode
    pub fn initialize_standalone(&mut self, options: &Dictionary) -> Result<(), SystemInitError> {
        godot_print!("SystemInitializer: Initializing in standalone mode");
        
        // Initialize core systems
        self.initialize_core_systems()?;
        
        // Configure for standalone mode
        self.configure_standalone(options)?;
        
        // Start the game
        self.start_game()?;
        
        self.initialized = true;
        godot_print!("SystemInitializer: Standalone initialization complete");
        Ok(())
    }
    
    /// Initialize the system in host mode
    pub fn initialize_host(&mut self, options: &Dictionary) -> Result<(), SystemInitError> {
        godot_print!("SystemInitializer: Initializing in host mode");
        
        // Initialize core systems
        self.initialize_core_systems()?;
        
        // Configure for host mode
        self.configure_host(options)?;
        
        // Start the game
        self.start_game()?;
        
        self.initialized = true;
        godot_print!("SystemInitializer: Host initialization complete");
        Ok(())
    }
    
    /// Initialize the system in client mode
    pub fn initialize_client(&mut self, options: &Dictionary) -> Result<(), SystemInitError> {
        godot_print!("SystemInitializer: Initializing in client mode");
        
        // Initialize core systems
        self.initialize_core_systems()?;
        
        // Configure for client mode
        self.configure_client(options)?;
        
        // Start the game
        self.start_game()?;
        
        self.initialized = true;
        godot_print!("SystemInitializer: Client initialization complete");
        Ok(())
    }
    
    /// Initialize core systems (common for all modes)
    fn initialize_core_systems(&mut self) -> Result<(), SystemInitError> {
        godot_print!("SystemInitializer: Initializing core systems");
        
        // Initialize event bus first (Rust module)
        if let Err(err) = event_bus::initialize() {
            return Err(SystemInitError::ManagerError(format!("Failed to initialize event_bus: {}", err)));
        }
        
        // Initialize config manager (Rust module)
        if let Err(err) = config_manager::initialize() {
            return Err(SystemInitError::ManagerError(format!("Failed to initialize config_manager: {}", err)));
        }
        
        // Initialize bridges (Godot objects)
        let event_bridge = EventBridge::new_alloc();
        let config_bridge = ConfigBridge::new_alloc();
        let game_bridge = GameManagerBridge::new_alloc();
        let network_bridge = NetworkManagerBridge::new_alloc();
        
        self.event_bridge = Some(event_bridge);
        self.config_bridge = Some(config_bridge);
        self.game_bridge = Some(game_bridge);
        self.network_bridge = Some(network_bridge);
        
        // Initialize other managers (Rust modules)
        if let Err(err) = game_manager::initialize() {
            return Err(SystemInitError::ManagerError(format!("Failed to initialize game_manager: {}", err)));
        }
        
        if let Err(err) = world_manager::initialize() {
            return Err(SystemInitError::ManagerError(format!("Failed to initialize world_manager: {}", err)));
        }
        
        godot_print!("SystemInitializer: Core systems initialized");
        Ok(())
    }
    
    /// Configure for standalone mode
    fn configure_standalone(&mut self, options: &Dictionary) -> Result<(), SystemInitError> {
        if let Some(config_bridge) = &mut self.config_bridge {
            let mut config_bridge_mut = config_bridge.clone();
            
            // Apply configuration
            if !config_bridge_mut.bind_mut().apply_multiple_settings(options.clone(), true) {
                return Err(SystemInitError::ConfigError("Failed to apply configuration for standalone mode".into()));
            }
            
            // Configure the game manager
            if let Err(err) = game_manager::configure_standalone() {
                return Err(SystemInitError::ManagerError(format!("Failed to configure game manager for standalone mode: {}", err)));
            }
            
            Ok(())
        } else {
            Err(SystemInitError::BridgeError("ConfigBridge not initialized".into()))
        }
    }
    
    /// Configure for host mode
    fn configure_host(&mut self, options: &Dictionary) -> Result<(), SystemInitError> {
        if let Some(config_bridge) = &mut self.config_bridge {
            let mut config_bridge_mut = config_bridge.clone();
            
            // Apply configuration
            if !config_bridge_mut.bind_mut().apply_multiple_settings(options.clone(), true) {
                return Err(SystemInitError::ConfigError("Failed to apply configuration for host mode".into()));
            }
            
            // Initialize network for host mode
            if let Some(network_bridge) = &mut self.network_bridge {
                let mut network_bridge_mut = network_bridge.clone();
                let port = options.get("server_port".to_variant())
                    .and_then(|v| v.try_to::<i64>().ok())
                    .unwrap_or(7878);
                
                if !network_bridge_mut.bind_mut().initialize_network(1, port, "".into()) {
                    return Err(SystemInitError::NetworkError("Failed to initialize network for host mode".into()));
                }
            }
            
            // Configure the game manager
            if let Err(err) = game_manager::configure_host() {
                return Err(SystemInitError::ManagerError(format!("Failed to configure game manager for host mode: {}", err)));
            }
            
            Ok(())
        } else {
            Err(SystemInitError::BridgeError("ConfigBridge not initialized".into()))
        }
    }
    
    /// Configure for client mode
    fn configure_client(&mut self, options: &Dictionary) -> Result<(), SystemInitError> {
        if let Some(config_bridge) = &mut self.config_bridge {
            let mut config_bridge_mut = config_bridge.clone();
            
            // Apply configuration
            if !config_bridge_mut.bind_mut().apply_multiple_settings(options.clone(), true) {
                return Err(SystemInitError::ConfigError("Failed to apply configuration for client mode".into()));
            }
            
            // Initialize network for client mode
            if let Some(network_bridge) = &mut self.network_bridge {
                let mut network_bridge_mut = network_bridge.clone();
                let server_address = options.get("server_address".to_variant())
                    .and_then(|v| v.try_to::<GString>().ok())
                    .unwrap_or_else(|| "127.0.0.1:7878".into());
                
                if !network_bridge_mut.bind_mut().initialize_network(2, 0, server_address) {
                    return Err(SystemInitError::NetworkError("Failed to initialize network for client mode".into()));
                }
            }
            
            // Configure the game manager
            if let Err(err) = game_manager::configure_client() {
                return Err(SystemInitError::ManagerError(format!("Failed to configure game manager for client mode: {}", err)));
            }
            
            Ok(())
        } else {
            Err(SystemInitError::BridgeError("ConfigBridge not initialized".into()))
        }
    }
    
    /// Start the game
    fn start_game(&mut self) -> Result<(), SystemInitError> {
        if let Some(game_bridge) = &mut self.game_bridge {
            let mut game_bridge_mut = game_bridge.clone();
            if !game_bridge_mut.bind_mut().start_game() {
                return Err(SystemInitError::GameError("Failed to start game".into()));
            }
            Ok(())
        } else {
            Err(SystemInitError::BridgeError("GameBridge not initialized".into()))
        }
    }
    
    /// Get the game bridge
    pub fn get_game_bridge(&self) -> Option<Gd<GameManagerBridge>> {
        self.game_bridge.clone()
    }
    
    /// Get the config bridge
    pub fn get_config_bridge(&self) -> Option<Gd<ConfigBridge>> {
        self.config_bridge.clone()
    }
    
    /// Get the network bridge
    pub fn get_network_bridge(&self) -> Option<Gd<NetworkManagerBridge>> {
        self.network_bridge.clone()
    }
    
    /// Get the event bridge
    pub fn get_event_bridge(&self) -> Option<Gd<EventBridge>> {
        self.event_bridge.clone()
    }
    
    /// Check if initialization is complete
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}
// File: system_initializer.rs

use godot::prelude::*;
use std::error::Error;
use std::fmt;
use std::result::Result;
use std::sync::{Arc, Mutex};
use std::cell::RefCell;
use std::thread_local;

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

use crate::initialization::world::world_initializer::WorldInitializer;

// Import the configuration service
use crate::initialization::configuration_service::ConfigurationService;

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

// Thread-local storage for the SystemInitializer singleton
thread_local! {
    static SYSTEM_INITIALIZER: RefCell<Option<Arc<Mutex<SystemInitializer>>>> = RefCell::new(None);
}

pub struct SystemInitializer {
    // Godot objects (not thread-safe)
    game_bridge: Option<Gd<GameManagerBridge>>,
    config_bridge: Option<Gd<ConfigBridge>>,
    network_bridge: Option<Gd<NetworkManagerBridge>>,
    event_bridge: Option<Gd<EventBridge>>,

    // Core managers with Arc<Mutex> for thread safety
    game_manager: Option<Arc<Mutex<game_manager::GameManager>>>,
    config_manager: Option<Arc<Mutex<config_manager::ConfigurationManager>>>,
    network_manager: Option<Arc<Mutex<NetworkHandler>>>,
    world_manager: Option<Arc<Mutex<WorldStateManager>>>,
    event_bus: Option<Arc<event_bus::EventBus>>,
    
    // Configuration service
    configuration_service: Option<ConfigurationService>,
    
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

            game_manager: None,
            config_manager: None,
            network_manager: None,
            world_manager: None,
            event_bus: None,
            
            configuration_service: None,
            
            initialized: false,
        }
    }
    
    /// Initialize the singleton instance if not already initialized
    pub fn ensure_initialized() -> Arc<Mutex<SystemInitializer>> {
        let mut instance = None;
        
        SYSTEM_INITIALIZER.with(|cell| {
            if cell.borrow().is_none() {
                let new_instance = Arc::new(Mutex::new(SystemInitializer::new()));
                *cell.borrow_mut() = Some(new_instance.clone());
                instance = Some(new_instance);
            } else {
                instance = Some(cell.borrow().clone().unwrap());
            }
        });
        
        instance.unwrap()
    }
    
    /// Get the singleton instance
    pub fn get_instance() -> Option<Arc<Mutex<SystemInitializer>>> {
        let mut result = None;
        SYSTEM_INITIALIZER.with(|cell| {
            if let Some(initializer) = &*cell.borrow() {
                result = Some(initializer.clone());
            }
        });
        result
    }
    
    /// Set the singleton instance
    pub fn set_instance(initializer: Arc<Mutex<SystemInitializer>>) {
        SYSTEM_INITIALIZER.with(|cell| {
            *cell.borrow_mut() = Some(initializer);
        });
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
        
        // Create and use WorldInitializer
        let mut world_initializer = WorldInitializer::new(
            config_manager.clone(),
            event_bus.clone()
        );
        
        // Initialize all world-related systems
        if let Err(e) = world_initializer.initialize() {
            return Err(SystemInitError::ManagerError(format!("World initialization failed: {}", e)));
        }
        
        // Get initialized world manager and store it
        if let Some(world_manager) = world_initializer.get_world_manager() {
            self.world_manager = Some(world_manager);
        } else {
            return Err(SystemInitError::ManagerError("Failed to get world manager from initializer".to_string()));
        }
        
        // Initialize game manager with dependencies
        let game_manager = Arc::new(Mutex::new(game_manager::GameManager::new_with_dependencies(
            config_manager.clone(),
            event_bus.clone(),
            self.world_manager.clone(),
            Some(network_manager.clone()),
        )));
        self.game_manager = Some(game_manager.clone());
        
        // Set the game manager in the thread-local storage so it can be accessed from anywhere
        crate::core::game_manager::set_instance(game_manager.clone());
        
        // Create configuration service (optional, remove if not needed)
        let configuration_service = ConfigurationService::new(
            game_manager.clone(),
            config_manager.clone(),
            network_manager.clone(),
            self.world_manager.clone().unwrap(),
            event_bus.clone(),
        );
        self.configuration_service = Some(configuration_service);
        
        godot_print!("SystemInitializer: Core systems initialized");
        
        // Initialize bridges after all systems are ready
        self.initialize_bridges()?;
        
        Ok(())
    }
    
    /// Initialize bridges for GDScript communication
    pub fn initialize_bridges(&mut self) -> Result<(), SystemInitError> {
        godot_print!("SystemInitializer: Initializing bridges");
        
        // Create bridges by direct allocation since they're Node-based (not RefCounted)
        let mut game_bridge = GameManagerBridge::new_alloc();
        let mut config_bridge = ConfigBridge::new_alloc();
        let mut network_bridge = NetworkManagerBridge::new_alloc();
        let mut event_bridge = EventBridge::new_alloc();
        
        // Initialize bridges with their respective managers
        if let Some(game_manager) = &self.game_manager {
            // Set game manager reference on the bridge
            game_bridge.bind_mut().set_config_manager(game_manager.clone());
        }
        
        if let Some(config_manager) = &self.config_manager {
            // Set config manager reference on the bridge
            config_bridge.bind_mut().set_config_manager(config_manager.clone());
        }
        
        if let Some(network_manager) = &self.network_manager {
            // Initialize network bridge
            // Using the existing initialize_network method with standalone mode
            network_bridge.bind_mut().initialize_network(0, 0, "".into());
        }
        
        if let Some(event_bus) = &self.event_bus {
            // Set event bus reference on the bridge
            event_bridge.bind_mut().set_event_bus(event_bus.clone());
        }
        
        // Store the bridges
        self.game_bridge = Some(game_bridge);
        self.config_bridge = Some(config_bridge);
        self.network_bridge = Some(network_bridge);
        self.event_bridge = Some(event_bridge);
        
        godot_print!("SystemInitializer: Bridges initialized");
        Ok(())
    }
    
    /// Initialize the system in standalone mode
    pub fn initialize_standalone(&mut self, options: &Dictionary) -> Result<(), SystemInitError> {
        godot_print!("SystemInitializer: Initializing standalone mode");
        
        // Initialize core systems if not already done
        if !self.initialized {
            self.initialize_core_systems()?;
            self.initialize_bridges()?;
        }
        
        // Configure system using the configuration service
        if let Some(ref mut config_service) = self.configuration_service {
            config_service.configure(options)
                .map_err(|e| SystemInitError::ManagerError(e))?;
        } else {
            return Err(SystemInitError::ManagerError("Configuration service not initialized".into()));
        }
        
        self.initialized = true;
        
        // Note: We no longer need to update the singleton instance here since
        // we're using Arc<Mutex<>> and already modifying the instance in place
        
        godot_print!("SystemInitializer: Standalone initialization complete");
        Ok(())
    }
    
    /// Initialize the system in host mode
    pub fn initialize_host(&mut self, options: &Dictionary) -> Result<(), SystemInitError> {
        godot_print!("SystemInitializer: Initializing host mode");
        
        // Initialize core systems if not already done
        if !self.initialized {
            self.initialize_core_systems()?;
            self.initialize_bridges()?;
        }
        
        // Configure system using the configuration service
        if let Some(ref mut config_service) = self.configuration_service {
            config_service.configure(options)
                .map_err(|e| SystemInitError::ManagerError(e))?;
        } else {
            return Err(SystemInitError::ManagerError("Configuration service not initialized".into()));
        }
        
        self.initialized = true;
        
        // Note: We no longer need to update the singleton instance here since
        // we're using Arc<Mutex<>> and already modifying the instance in place
        
        godot_print!("SystemInitializer: Host initialization complete");
        Ok(())
    }
    
    /// Initialize the system in client mode
    pub fn initialize_client(&mut self, options: &Dictionary) -> Result<(), SystemInitError> {
        godot_print!("SystemInitializer: Initializing client mode");
        
        // Initialize core systems if not already done
        if !self.initialized {
            self.initialize_core_systems()?;
            self.initialize_bridges()?;
        }
        
        // Configure system using the configuration service
        if let Some(ref mut config_service) = self.configuration_service {
            config_service.configure(options)
                .map_err(|e| SystemInitError::ManagerError(e))?;
        } else {
            return Err(SystemInitError::ManagerError("Configuration service not initialized".into()));
        }
        
        self.initialized = true;
        
        // Note: We no longer need to update the singleton instance here since
        // we're using Arc<Mutex<>> and already modifying the instance in place
        
        godot_print!("SystemInitializer: Client initialization complete");
        Ok(())
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
                // Just drop the manager since we don't have an explicit shutdown method
                // Any cleanup would happen in the NetworkHandler's Drop implementation
                drop(manager);
            }
        }
        
        if let Some(world_manager) = &self.world_manager {
            if let Ok(mut manager) = world_manager.lock() {
                // Just initialize the world manager to its default state
                // since we don't have an explicit shutdown method
                *manager = WorldStateManager::new(WorldStateConfig {
                    seed: 0,
                    world_size: (0, 0),
                    generation_parameters: Default::default(),
                });
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
        if let Some(bridge) = &self.game_bridge {
            bridge.clone().free();
            self.game_bridge = None;
        }
        if let Some(bridge) = &self.config_bridge {
            bridge.clone().free();
            self.config_bridge = None;
        }
        if let Some(bridge) = &self.network_bridge {
            bridge.clone().free();
            self.network_bridge = None;
        }
        if let Some(bridge) = &self.event_bridge {
            bridge.clone().free();
            self.event_bridge = None;
        }
        
        // Reset initialization state
        self.initialized = false;
        
        // Clear the singleton instance
        SYSTEM_INITIALIZER.with(|cell| {
            *cell.borrow_mut() = None;
        });
        
        godot_print!("SystemInitializer: Systems shutdown complete");
        Ok(())
    }
}

// We no longer need to implement Clone for SystemInitializer
// Since we're now working with Arc<Mutex<SystemInitializer>> which already provides shared ownership
// This removes a potentially problematic pattern where configuration_service couldn't be cloned

// Implement Default for easy initialization
impl Default for SystemInitializer {
    fn default() -> Self {
        Self::new()
    }
}

// Implement Drop trait for cleanup
impl Drop for SystemInitializer {
    fn drop(&mut self) {
        if self.initialized {
            godot_print!("SystemInitializer: Dropping initialized instance - performing cleanup");
            let _ = self.shutdown();
        }
    }
}
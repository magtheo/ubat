use godot::prelude::*;
use std::sync::{Arc, Mutex};
use crate::core::system_initializer::SystemInitializer;
use crate::bridge::{ConfigBridge, GameManagerBridge, NetworkManagerBridge, EventBridge};

/// Helper class for simplified game initialization
///
/// This class provides a simple interface for initializing the game
/// with different modes and configurations.
#[derive(GodotClass)]
#[class(base=Node)]
pub struct GameInitHelper {
    base: Base<Node>,
    
    #[export]
    debug_mode: bool,
    
    // Use a thread-safe approach with Arc<Mutex>
    system_initializer: Arc<Mutex<SystemInitializer>>,
}

#[godot_api]
impl INode for GameInitHelper {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            debug_mode: false,
            system_initializer: Arc::new(Mutex::new(SystemInitializer::new())),
        }
    }
    
    fn ready(&mut self) {
        if self.debug_mode {
            godot_print!("GameInitHelper: SystemInitializer created");
        }
    }
}

#[godot_api]
impl GameInitHelper {
    /// Initialize the game in standalone mode
    fn initialize_game(&mut self, mode: i64, options: Dictionary) -> bool {
        godot_print!("GameInitHelper: Initializing game with mode: {}", mode);
        
        // Use a blocking lock instead of try_lock
        let mut system_init = match self.system_initializer.lock() {
            Ok(guard) => guard,
            Err(_) => {
                godot_error!("GameInitHelper: Could not acquire lock on SystemInitializer");
                return false;
            }
        };
        
        // Prepare options with network mode
        let mut full_options = options.clone();
        full_options.insert("network_mode".to_variant(), mode.to_variant());
        
        // Select initialization method based on mode
        let init_result = match mode {
            0 => system_init.initialize_standalone(&full_options),
            1 => system_init.initialize_host(&full_options),
            2 => system_init.initialize_client(&full_options),
            _ => {
                godot_error!("GameInitHelper: Invalid network mode");
                return false;
            }
        };
        
        // Handle initialization result
        match init_result {
            Ok(_) => {
                godot_print!("GameInitHelper: Game initialization successful");
                true
            },
            Err(err) => {
                godot_error!("GameInitHelper: Initialization failed: {}", err);
                false
            }
        }
    }

    /// Standalone mode initialization
    #[func]
    pub fn init_standalone(&mut self, options: Dictionary) -> bool {
        self.initialize_game(0, options)
    }

    /// Host mode initialization
    #[func]
    pub fn init_host(&mut self, options: Dictionary) -> bool {
        self.initialize_game(1, options)
    }

    /// Client mode initialization
    #[func]
    pub fn init_client(&mut self, options: Dictionary) -> bool {
        self.initialize_game(2, options)
    }
    
    /// Check if the system is ready
    #[func]
    pub fn is_system_ready(&self) -> bool {
        // Check if we can acquire a lock, which implies the system is initialized
        self.system_initializer.try_lock().is_ok()
    }


    /// Get bridge methods with shared access strategy
    #[func]
    pub fn get_game_bridge(&self) -> Variant {
        match self.system_initializer.lock() {
            Ok(system_init) => {
                system_init.get_game_bridge()
                    .map(|bridge| bridge.to_variant())
                    .unwrap_or(Variant::nil())
            },
            Err(_) =>{
                godot_error!("Could not acquire lock to get game bridge");
                Variant::nil()
            }
        }
    }

    // Similar implementations for other bridge getters (config, network, event)
    #[func]
    pub fn get_config_bridge(&self) -> Variant {
        match self.system_initializer.lock() {
            Ok(system_init) => {
                system_init.get_config_bridge()
                    .map(|bridge| bridge.to_variant())
                    .unwrap_or(Variant::nil())
            },
            Err(_) => {
                godot_error!("Could not acquire lock to get config bridge");
                Variant::nil()
            }
        }
    }

    #[func]
    pub fn get_network_bridge(&self) -> Variant {
        match self.system_initializer.lock() {
            Ok(system_init) => {
                system_init.get_network_bridge()
                    .map(|bridge| bridge.to_variant())
                    .unwrap_or(Variant::nil())
            },
            Err(_) => {
                godot_error!("Could not acquire lock to get network bridge");
                Variant::nil()
            }
        }
    }

    #[func]
    pub fn get_event_bridge(&self) -> Variant {
        match self.system_initializer.lock() {
            Ok(system_init) => {
                system_init.get_event_bridge()
                    .map(|bridge| bridge.to_variant())
                    .unwrap_or(Variant::nil())
            },
            Err(_) => {
                godot_error!("Could not acquire lock to get event bridge");
                Variant::nil()
            }
        }
    }
}
use godot::prelude::*;
use std::sync::{Arc, Mutex};
use crate::initialization::system_initializer::SystemInitializer;
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
}

#[godot_api]
impl INode for GameInitHelper {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            debug_mode: false,
        }
    }
    
    fn ready(&mut self) {
        if self.debug_mode {
            godot_print!("GameInitHelper: Ready, will use SystemInitializer singleton");
        }
        
        // Ensure the SystemInitializer is properly initialized once at startup
        SystemInitializer::ensure_initialized();
    }
}

#[godot_api]
impl GameInitHelper {
    /// Initialize the game in standalone mode
    fn initialize_game(&mut self, mode: i64, options: Dictionary) -> bool {
        godot_print!("GameInitHelper: Initializing game with mode: {}", mode);
        
        // Get or create the SystemInitializer singleton
        let system_initializer = SystemInitializer::ensure_initialized();
        
        // Use a blocking lock to access the initializer
        let mut system_init = match system_initializer.lock() {
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
        // Get the singleton instance
        match SystemInitializer::get_instance() {
            Some(system_initializer) => {
                // Check if we can lock the initializer and if it's initialized
                match system_initializer.try_lock() {
                    Ok(initializer) => initializer.is_initialized(),
                    Err(_) => {
                        // If we can't lock it, it's probably in use, which means it exists
                        true
                    }
                }
            },
            None => false
        }
    }

    /// Get bridge methods with shared access strategy
    #[func]
    pub fn get_game_bridge(&self) -> Variant {
        match SystemInitializer::get_instance() {
            Some(system_initializer) => {
                match system_initializer.lock() {
                    Ok(system_init) => {
                        system_init.get_game_bridge()
                            .map(|bridge| bridge.to_variant())
                            .unwrap_or(Variant::nil())
                    },
                    Err(_) => {
                        godot_error!("Could not acquire lock to get game bridge");
                        Variant::nil()
                    }
                }
            },
            None => {
                godot_error!("SystemInitializer not initialized");
                Variant::nil()
            }
        }
    }

    // Similar implementations for other bridge getters (config, network, event)
    #[func]
    pub fn get_config_bridge(&self) -> Variant {
        match SystemInitializer::get_instance() {
            Some(system_initializer) => {
                match system_initializer.lock() {
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
            },
            None => {
                godot_error!("SystemInitializer not initialized");
                Variant::nil()
            }
        }
    }

    #[func]
    pub fn get_network_bridge(&self) -> Variant {
        match SystemInitializer::get_instance() {
            Some(system_initializer) => {
                match system_initializer.lock() {
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
            },
            None => {
                godot_error!("SystemInitializer not initialized");
                Variant::nil()
            }
        }
    }

    #[func]
    pub fn get_event_bridge(&self) -> Variant {
        match SystemInitializer::get_instance() {
            Some(system_initializer) => {
                match system_initializer.lock() {
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
            },
            None => {
                godot_error!("SystemInitializer not initialized");
                Variant::nil()
            }
        }
    }
}
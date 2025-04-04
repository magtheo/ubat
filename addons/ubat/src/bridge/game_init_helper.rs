use godot::prelude::*;
use crate::core::system_initializer::SystemInitializer;

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
    
    // Reference to the system initializer
    // Now using a standard Rust module rather than a Godot object
    system_initializer: Option<SystemInitializer>,
}

#[godot_api]
impl INode for GameInitHelper {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            debug_mode: false,
            system_initializer: None,
        }
    }
    
    fn ready(&mut self) {
        // Initialize the system initializer
        if self.system_initializer.is_none() {
            self.system_initializer = Some(SystemInitializer::new());
            
            if self.debug_mode {
                godot_print!("GameInitHelper: SystemInitializer created");
            }
        }
    }
}

#[godot_api]
impl GameInitHelper {
    /// Initialize the game in standalone mode
    #[func]
    pub fn init_standalone(&mut self, options: Dictionary) -> bool {
        godot_print!("GameInitHelper: init_standalone called with options: {:?}", options);
        
        if let Some(system_init) = &mut self.system_initializer {
            // Add network_mode to options
            let mut full_options = options.clone();
            full_options.insert("network_mode".to_variant(), 0.to_variant());
            
            // Use the system initializer directly
            match system_init.initialize_standalone(&full_options) {
                Ok(_) => {
                    godot_print!("GameInitHelper: Standalone mode initialized successfully");
                    true
                },
                Err(err) => {
                    godot_error!("GameInitHelper: Failed to initialize standalone mode: {}", err);
                    false
                }
            }
        } else {
            godot_error!("GameInitHelper: SystemInitializer not initialized");
            false
        }
    }
    
    /// Initialize the game in host mode
    #[func]
    pub fn init_host(&mut self, options: Dictionary) -> bool {
        godot_print!("GameInitHelper: init_host called with options: {:?}", options);
        
        if let Some(system_init) = &mut self.system_initializer {
            // Add network_mode to options
            let mut full_options = options.clone();
            full_options.insert("network_mode".to_variant(), 1.to_variant());
            
            // Use the system initializer directly
            match system_init.initialize_host(&full_options) {
                Ok(_) => {
                    godot_print!("GameInitHelper: Host mode initialized successfully");
                    true
                },
                Err(err) => {
                    godot_error!("GameInitHelper: Failed to initialize host mode: {}", err);
                    false
                }
            }
        } else {
            godot_error!("GameInitHelper: SystemInitializer not initialized");
            false
        }
    }
    
    /// Initialize the game in client mode
    #[func]
    pub fn init_client(&mut self, options: Dictionary) -> bool {
        godot_print!("GameInitHelper: init_client called with options: {:?}", options);
        
        if let Some(system_init) = &mut self.system_initializer {
            // Add network_mode to options
            let mut full_options = options.clone();
            full_options.insert("network_mode".to_variant(), 2.to_variant());
            
            // Use the system initializer directly
            match system_init.initialize_client(&full_options) {
                Ok(_) => {
                    godot_print!("GameInitHelper: Client mode initialized successfully");
                    true
                },
                Err(err) => {
                    godot_error!("GameInitHelper: Failed to initialize client mode: {}", err);
                    false
                }
            }
        } else {
            godot_error!("GameInitHelper: SystemInitializer not initialized");
            false
        }
    }
    
    /// Check if the system is ready
    #[func]
    pub fn is_system_ready(&self) -> bool {
        self.system_initializer.is_some()
    }
}
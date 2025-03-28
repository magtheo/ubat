
use godot::prelude::*;
use std::collections::HashMap;

use crate::bridge::config_bridge::ConfigBridge;
use crate::bridge::game_bridge::GameManagerBridge;

/// Helper class for simplified game initialization
///
/// This class provides a simple interface for initializing the game
/// with different modes and configurations.
#[derive(GodotClass)]
#[class(base=Node)]
pub struct GameInitHelper {
    base: Base<Node>,
    
    // References to bridges
    config_bridge: Option<Gd<ConfigBridge>>,
    game_bridge: Option<Gd<GameManagerBridge>>,
    
    #[export]
    debug_mode: bool,
}

#[godot_api]
impl INode for GameInitHelper {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            config_bridge: None,
            game_bridge: None,
            debug_mode: false,
        }
    }
}

#[godot_api]
impl GameInitHelper {
    /// Set the bridges used by this helper
    #[func]
    pub fn set_bridges(&mut self, config_bridge: Gd<ConfigBridge>, game_bridge: Gd<GameManagerBridge>) {
        self.config_bridge = Some(config_bridge);
        self.game_bridge = Some(game_bridge);
        
        if self.debug_mode {
            godot_print!("GameInitHelper: Bridges set successfully");
        }
    }
    
    /// Initialize the game in standalone mode
    ///
    /// Parameters:
    /// - config_path: Path to the configuration file
    /// - options: Dictionary of configuration options
    ///
    /// Returns true if initialization was successful, false otherwise
    #[func]
    pub fn init_standalone(&self, config_path: GString, options: Dictionary) -> bool {
        godot_print!("GameInitHelper: init_standalone called with path: {}, options: {:?}", config_path, options);
        
        // Check bridges
        if self.config_bridge.is_none() {
            godot_error!("GameInitHelper: ConfigBridge not set");
            return false;
        }
        
        if self.game_bridge.is_none() {
            godot_error!("GameInitHelper: GameBridge not set");
            return false;
        }
        
        // Clone bridges to avoid borrowing issues
        let config_bridge = self.config_bridge.as_ref().unwrap().clone();
        let game_bridge = self.game_bridge.as_ref().unwrap().clone();
        
        // Create a new dictionary with network_mode set to 0 (Standalone)
        let mut full_options = options.clone();
        full_options.insert("network_mode".to_variant(), 0.to_variant());
        
        godot_print!("GameInitHelper: Applying configuration options: {:?}", full_options);
        
        // Step 1: Apply settings with better error reporting
        let apply_result = {
            let mut config_bridge_mut = config_bridge.clone();
            let result = config_bridge_mut.bind_mut().apply_multiple_settings(full_options, true);
            if !result {
                godot_error!("GameInitHelper: Failed to apply configuration options");
            }
            result
        };
        
        if !apply_result {
            // Try to get more specific information - separate operation
            let mut config_bridge_mut = config_bridge.clone();
            let game_mode_result = config_bridge_mut.bind_mut().set_game_mode(0, true);
            godot_error!("GameInitHelper: Direct set_game_mode(0) result: {}", game_mode_result);
            
            return false;
        }
        
        godot_print!("GameInitHelper: Successfully applied configuration options");
        godot_print!("GameInitHelper: Initializing game with config path: {}", config_path);
        
        // Step 2: Initialize the game with better error reporting
        let init_result = {
            let mut game_bridge_mut = game_bridge.clone();
            let result = game_bridge_mut.bind_mut().initialize(config_path.clone());
            if !result {
                godot_error!("GameInitHelper: Failed to initialize game with path: {}", config_path);
            }
            result
        };
        
        if !init_result {
            // Try with default initialization as fallback
            let mut game_bridge_mut = game_bridge.clone();
            let default_result = game_bridge_mut.bind_mut().initialize_default();
            godot_error!("GameInitHelper: Fallback initialize_default result: {}", default_result);
            
            return false;
        }
        
        godot_print!("GameInitHelper: Game initialized successfully, now starting game");
        
        // Step 3: Start the game with better error reporting
        let start_result = {
            let mut game_bridge_mut = game_bridge.clone();
            let result = game_bridge_mut.bind_mut().start_game();
            if !result {
                godot_error!("GameInitHelper: Failed to start game");
            }
            result
        };
        
        if !start_result {
            return false;
        }
        
        godot_print!("GameInitHelper: Complete initialization successful");
        
        true
    }

    /// Initialize the game in host mode
    ///
    /// Parameters:
    /// - config_path: Path to the configuration file
    /// - options: Dictionary of configuration options
    ///
    /// Returns true if initialization was successful, false otherwise
    #[func]
    pub fn init_host(&self, config_path: GString, options: Dictionary) -> bool {
        if self.config_bridge.is_none() || self.game_bridge.is_none() {
            godot_error!("GameInitHelper: Bridges not set");
            return false;
        }
        
        let mut config_bridge = self.config_bridge.as_ref().unwrap().clone();
        let mut game_bridge = self.game_bridge.as_ref().unwrap().clone();
        
        // Create a new dictionary with network_mode set to 1 (Host)
        let mut full_options = options.clone();
        full_options.insert("network_mode".to_variant(), 1.to_variant());
        
        // Apply options to the config bridge
        let apply_result = config_bridge.bind_mut().apply_multiple_settings(full_options, true);
        if !apply_result {
            godot_error!("GameInitHelper: Failed to apply configuration options");
            return false;
        }
        
        // Validate required fields for host mode
        let validate_result = config_bridge.bind().validate_for_mode(1);
        if !validate_result {
            godot_error!("GameInitHelper: Invalid configuration for host mode");
            return false;
        }
        
        // Initialize the game
        let init_result = game_bridge.bind_mut().initialize(config_path);
        if !init_result {
            godot_error!("GameInitHelper: Failed to initialize game");
            return false;
        }
        
        // Start the game
        let start_result = game_bridge.bind_mut().start_game();
        if !start_result {
            godot_error!("GameInitHelper: Failed to start game");
            return false;
        }
        
        true
    }
    
    /// Initialize the game in client mode
    ///
    /// Parameters:
    /// - config_path: Path to the configuration file
    /// - options: Dictionary of configuration options
    ///
    /// Returns true if initialization was successful, false otherwise
    #[func]
    pub fn init_client(&self, config_path: GString, options: Dictionary) -> bool {
        if self.config_bridge.is_none() || self.game_bridge.is_none() {
            godot_error!("GameInitHelper: Bridges not set");
            return false;
        }
        
        let mut config_bridge = self.config_bridge.as_ref().unwrap().clone();
        let mut game_bridge = self.game_bridge.as_ref().unwrap().clone();
        
        // Create a new dictionary with network_mode set to 2 (Client)
        let mut full_options = options.clone();
        full_options.insert("network_mode".to_variant(), 2.to_variant());
        
        // Apply options to the config bridge
        let apply_result = config_bridge.bind_mut().apply_multiple_settings(full_options, true);
        if !apply_result {
            godot_error!("GameInitHelper: Failed to apply configuration options");
            return false;
        }
        
        // Validate required fields for client mode
        let validate_result = config_bridge.bind().validate_for_mode(2);
        if !validate_result {
            godot_error!("GameInitHelper: Invalid configuration for client mode");
            return false;
        }
        
        // Initialize the game
        let init_result = game_bridge.bind_mut().initialize(config_path);
        if !init_result {
            godot_error!("GameInitHelper: Failed to initialize game");
            return false;
        }
        
        // Start the game
        let start_result = game_bridge.bind_mut().start_game();
        if !start_result {
            godot_error!("GameInitHelper: Failed to start game");
            return false;
        }
        
        true
    }
}
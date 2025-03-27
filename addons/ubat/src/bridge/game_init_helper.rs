
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
        if self.config_bridge.is_none() || self.game_bridge.is_none() {
            godot_error!("GameInitHelper: Bridges not set");
            return false;
        }
        
        let mut config_bridge = self.config_bridge.as_ref().unwrap().clone();
        let mut game_bridge = self.game_bridge.as_ref().unwrap().clone();
        
        // Create a new dictionary with network_mode set to 0 (Standalone)
        let mut full_options = options.clone();
        full_options.insert("network_mode".to_variant(), 0.to_variant());
        
        // Apply options to the config bridge
        let apply_result = config_bridge.bind_mut().apply_multiple_settings(full_options, true);
        if !apply_result {
            godot_error!("GameInitHelper: Failed to apply configuration options");
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
use godot::prelude::*;
use std::sync::{Arc, Mutex};
use std::path::Path;

use crate::core::config_manager::{
    ConfigurationManager, 
    GameConfiguration, 
    GameModeConfig,
    NetworkConfig,
    ConfigValue,
    WorldSize,
    HostConfig,
    ClientConfig,
    ConfigurationError
};

/// ConfigBridge connects Rust configuration to Godot
///
/// This bridge provides an interface for loading, saving, and modifying
/// game configuration from both Rust and GDScript.
///
/// Usage:
/// 1. Add to your scene tree as a node
/// 2. Call load_config() to load configuration from a file
/// 3. Access configuration values through getters
/// 4. Modify configuration values through setters
/// 5. Call save_config() to save changes
///
/// Example:
/// ```gdscript
/// func _ready():
///     $ConfigBridge.connect("config_loaded", self, "_on_config_loaded")
///     $ConfigBridge.load_config("res://game_config.toml")
///
/// func _on_config_loaded(success):
///     if success:
///         print("World seed: ", $ConfigBridge.world_seed)
///         $ConfigBridge.world_seed = 12345
///         $ConfigBridge.save_config()
/// ```
#[derive(GodotClass)]
#[class(base=Node)]
pub struct ConfigBridge {
    base: Base<Node>,
    
    // Core configuration manager
    config_manager: Option<Arc<Mutex<ConfigurationManager>>>,
    
    // Common configuration properties exposed to the editor
    #[export]
    config_path: GString,
    
    #[export]
    world_seed: i64,
    
    #[export]
    world_width: i32,
    
    #[export]
    world_height: i32,
    
    #[export]
    max_players: i32,
    
    #[export]
    server_port: i32,
    
    #[export]
    network_mode: i32, // 0=Standalone, 1=Host, 2=Client
    
    #[export]
    server_address: GString,
    
    #[export]
    debug_mode: bool,
}

#[godot_api]
impl INode for ConfigBridge {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            config_manager: None,
            config_path: "res://game_config.toml".into(),
            world_seed: 0,
            world_width: 10000,
            world_height: 10000,
            max_players: 64,
            server_port: 7878,
            network_mode: 0, // Standalone by default
            server_address: "127.0.0.1:7878".into(),
            debug_mode: false,
        }
    }
    
    fn ready(&mut self) {
        if self.debug_mode {
            godot_print!("ConfigBridge: Ready");
        }
    }
}

#[godot_api]
impl ConfigBridge {
    // Signal declarations
    #[signal]
    fn config_loaded(success: bool);
    
    #[signal]
    fn config_saved(success: bool);
    
    #[signal]
    fn config_updated(key: GString, value: Variant);
    
    /// Load configuration from a file
    /// 
    /// Returns true if loading was successful, false otherwise
    #[func]
    pub fn load_config(&mut self, path: GString) -> bool {
        // Store the path
        self.config_path = path.clone();
        
        let success = match ConfigurationManager::load_from_file(path.to_string()) {
            Ok(manager) => {
                self.config_manager = Some(Arc::new(Mutex::new(manager)));
                
                // Update editor properties
                self.update_editor_properties_from_config();
                
                if self.debug_mode {
                    godot_print!("ConfigBridge: Configuration loaded successfully");
                }
                
                true
            },
            Err(e) => {
                godot_error!("Failed to load config: {}", e);
                false
            }
        };
        
        // Use base_mut() for signal emission
        self.base_mut().emit_signal(
            &StringName::from("config_loaded"), 
            &[success.to_variant()]
        );

        success
    }
    
    /// Create a new configuration with default values
    /// 
    /// Returns true if creation was successful
    #[func]
    pub fn create_default_config(&mut self) -> bool {
        self.config_manager = Some(Arc::new(Mutex::new(
            ConfigurationManager::new(None)
        )));
        
        // Update editor properties
        self.update_editor_properties_from_config();
        
        if self.debug_mode {
            godot_print!("ConfigBridge: Created default configuration");
        }
        
        true
    }
    
    /// Save configuration to the current path
    /// 
    /// Returns true if saving was successful, false otherwise
    #[func]
    pub fn save_config(&mut self) -> bool {
        let success = if let Some(config_manager) = &self.config_manager {
            if let Ok(manager) = config_manager.lock() {
                match manager.save_to_file() {
                    Ok(_) => {
                        if self.debug_mode {
                            godot_print!("ConfigBridge: Configuration saved successfully");
                        }
                        true
                    },
                    Err(e) => {
                        godot_error!("Failed to save config: {}", e);
                        false
                    }
                }
            } else {
                godot_error!("Failed to lock config manager");
                false
            }
        } else {
            godot_error!("Config manager not initialized");
            false
        };
        
        self.base_mut().emit_signal(
            &StringName::from("config_saved"), 
            &[success.to_variant()]
        );

        success
    }
    
    /// Save configuration to a specific path
    /// 
    /// Returns true if saving was successful, false otherwise
    #[func]
    pub fn save_config_to(&mut self, path: GString) -> bool {
        // Store the new path
        self.config_path = path.clone();
        
        // Save to the new path
        self.save_config()
    }
    
    /// Validate the current configuration
    /// 
    /// Returns true if the configuration is valid, false otherwise
    #[func]
    pub fn validate_config(&self) -> bool {
        if let Some(config_manager) = &self.config_manager {
            if let Ok(manager) = config_manager.lock() {
                match manager.validate() {
                    Ok(_) => {
                        if self.debug_mode {
                            godot_print!("ConfigBridge: Configuration is valid");
                        }
                        true
                    },
                    Err(e) => {
                        godot_error!("Configuration error: {:?}", e);
                        false
                    }
                }
            } else {
                godot_error!("Failed to lock config manager");
                false
            }
        } else {
            godot_error!("Config manager not initialized");
            false
        }
    }
    
    /// Apply the world seed property to the configuration
    #[func]
    pub fn apply_world_seed(&mut self) {
        self.sync_property_to_config("world_seed", self.world_seed.to_variant());
    }
    
    /// Apply the world width property to the configuration
    #[func]
    pub fn apply_world_width(&mut self) {
        self.sync_property_to_config("world_width", self.world_width.to_variant());
    }
    
    /// Apply the world height property to the configuration
    #[func]
    pub fn apply_world_height(&mut self) {
        self.sync_property_to_config("world_height", self.world_height.to_variant());
    }
    
    /// Apply the max players property to the configuration
    #[func]
    pub fn apply_max_players(&mut self) {
        self.sync_property_to_config("max_players", self.max_players.to_variant());
    }
    
    /// Apply the server port property to the configuration
    #[func]
    pub fn apply_server_port(&mut self) {
        self.sync_property_to_config("server_port", self.server_port.to_variant());
    }
    
    /// Apply the network mode property to the configuration
    #[func]
    pub fn apply_network_mode(&mut self) {
        self.sync_property_to_config("network_mode", self.network_mode.to_variant());
    }
    
    /// Apply the server address property to the configuration
    #[func]
    pub fn apply_server_address(&mut self) {
        self.sync_property_to_config("server_address", self.server_address.to_variant());
    }
    
    /// Get a custom configuration value
    #[func]
    pub fn get_custom_value(&self, key: GString) -> Variant {
        if let Some(config_manager) = &self.config_manager {
            if let Ok(manager) = config_manager.lock() {
                let config = manager.get_config();
                
                if let Some(value) = config.custom_settings.get(&key.to_string()) {
                    match value {
                        ConfigValue::String(s) => s.to_variant(),
                        ConfigValue::Integer(i) => i.to_variant(),
                        ConfigValue::Float(f) => f.to_variant(),
                        ConfigValue::Boolean(b) => b.to_variant(),
                    }
                } else {
                    Variant::nil()
                }
            } else {
                Variant::nil()
            }
        } else {
            Variant::nil()
        }
    }
    
    /// Set a custom configuration value
    #[func]
    pub fn set_custom_value(&mut self, key: GString, value: Variant) -> bool {
        // Use a separate scope to manage borrows
        let result = if let Some(config_manager) = &self.config_manager {
            // Take a lock, create a clone of the current config
            config_manager.lock().map(|mut manager| {
                let mut config = manager.get_config().clone();
                
                // Convert Variant to ConfigValue
                let config_value = match value.get_type() {
                    VariantType::STRING => ConfigValue::String(value.to::<GString>().to_string()),
                    VariantType::INT => ConfigValue::Integer(value.to::<i64>()),
                    VariantType::FLOAT => ConfigValue::Float(value.to::<f64>()),
                    VariantType::BOOL => ConfigValue::Boolean(value.to::<bool>()),
                    _ => {
                        godot_error!("Unsupported variant type for custom value: {:?}", value.get_type());
                        return false;
                    }
                };
                
                // Add to custom settings
                config.custom_settings.insert(key.to_string(), config_value);
                
                // Update the configuration
                manager.update_config(config);
                
                true
            }).unwrap_or(false)
        } else {
            godot_error!("Config manager not initialized");
            false
        };

        // Emit signal AFTER releasing all locks
        if result {
            self.base_mut().emit_signal(
                &StringName::from("config_updated"), 
                &[key.to_variant(), value]
            );
        }

        result
    }
    
    /// Synchronize a configuration property with its corresponding editor property
    fn sync_property_to_config(&mut self, property_name: &str, value: Variant) {
        // Use a separate scope to manage borrows and compute result
        let result = if let Some(config_manager) = &self.config_manager {
            config_manager.lock().map(|mut manager| {
                // Clone the current configuration
                let mut config = manager.get_config().clone();
                
                // Update the appropriate configuration property
                match property_name {
                    "world_seed" => {
                        config.world_seed = self.world_seed as u64;
                    },
                    "world_width" => {
                        config.world_size.width = self.world_width as u32;
                    },
                    "world_height" => {
                        config.world_size.height = self.world_height as u32;
                    },
                    "max_players" => {
                        config.network.max_players = self.max_players as u8;
                    },
                    "server_port" => {
                        config.network.server_port = self.server_port as u16;
                    },
                    "network_mode" => {
                        // Convert mode to GameModeConfig
                        match self.network_mode {
                            0 => {
                                config.game_mode = GameModeConfig::Standalone;
                            },
                            1 => {
                                // If it's not already a host, create a new host config
                                if let GameModeConfig::Host(_) = &config.game_mode {
                                    // Keep existing host config
                                } else {
                                    config.game_mode = GameModeConfig::Host(HostConfig {
                                        world_generation_seed: config.world_seed,
                                        admin_password: None,
                                    });
                                }
                            },
                            2 => {
                                // If it's not already a client, create a new client config
                                if let GameModeConfig::Client(_) = &config.game_mode {
                                    // Keep existing client config
                                } else {
                                    config.game_mode = GameModeConfig::Client(ClientConfig {
                                        server_address: self.server_address.to_string(),
                                        username: "Player".to_string(),
                                    });
                                }
                            },
                            _ => {
                                godot_error!("Invalid network mode: {}", self.network_mode);
                                return false;
                            }
                        }
                    },
                    "server_address" => {
                        // Only update if in client mode
                        if let GameModeConfig::Client(ref mut client_config) = config.game_mode {
                            client_config.server_address = self.server_address.to_string();
                        } else if self.debug_mode {
                            godot_print!("ConfigBridge: Not in client mode, server address not updated");
                        }
                    },
                    _ => {
                        godot_error!("Unknown property: {}", property_name);
                        return false;
                    }
                }
                
                // Update the configuration
                manager.update_config(config);
                
                true
            }).unwrap_or(false)
        } else {
            godot_error!("Config manager not initialized");
            false
        };

        // Emit signal AFTER releasing all locks
        if result {
            // Convert property_name to GString
            let property_key = GString::from(property_name);
            
            self.base_mut().emit_signal(
                &StringName::from("config_updated"), 
                &[property_key.to_variant(), value]
            );
            
            if self.debug_mode {
                godot_print!("ConfigBridge: Property '{}' updated", property_name);
            }
        }
    }
    
    /// Update editor properties from the current configuration
    fn update_editor_properties_from_config(&mut self) {
        if let Some(config_manager) = &self.config_manager {
            if let Ok(manager) = config_manager.lock() {
                let config = manager.get_config();
                
                // Update basic properties
                self.world_seed = config.world_seed as i64;
                self.world_width = config.world_size.width as i32;
                self.world_height = config.world_size.height as i32;
                self.max_players = config.network.max_players as i32;
                self.server_port = config.network.server_port as i32;
                
                // Update network mode
                match &config.game_mode {
                    GameModeConfig::Standalone => {
                        self.network_mode = 0;
                    },
                    GameModeConfig::Host(_) => {
                        self.network_mode = 1;
                    },
                    GameModeConfig::Client(client_config) => {
                        self.network_mode = 2;
                        self.server_address = client_config.server_address.clone().into();
                    },
                }
                
                if self.debug_mode {
                    godot_print!("ConfigBridge: Updated editor properties from configuration");
                }
            }
        }
    }
}
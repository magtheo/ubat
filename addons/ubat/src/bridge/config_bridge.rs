use godot::prelude::*;
use std::sync::{Arc, Mutex};

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
    pub config_path: GString,
    
    #[export]
    pub world_seed: i64,
    
    #[export]
    pub world_width: i32,
    
    #[export]
    pub world_height: i32,
    
    #[export]
    pub max_players: i32,
    
    #[export]
    pub server_port: i32,
    
    #[export]
    pub network_mode: i32, // 0=Standalone, 1=Host, 2=Client
    
    #[export]
    pub server_address: GString,
    
    #[export]
    pub debug_mode: bool,
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
    
    // Add this method to set the ConfigManager reference from SystemInitializer
    pub fn set_config_manager(&mut self, config_manager: Arc<Mutex<ConfigurationManager>>) {
        self.config_manager = Some(config_manager);
        
        // Update editor properties from the new config
        self.update_editor_properties_from_config();
        
        if self.debug_mode {
            godot_print!("ConfigBridge: Config manager reference set externally");
        }
    }

    // Add this method to check initialization status
    #[func]
    pub fn is_initialized(&self) -> bool {
        self.config_manager.is_some()
    }

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

    /// Set the game mode with associated configuration
    /// 
    /// This is a convenience method that sets the network mode and
    /// ensures all related configuration is properly set.
    /// 
    /// Parameters:
    /// - mode: 0=Standalone, 1=Host, 2=Client
    /// - save_config: Whether to save the configuration immediately
    /// 
    /// Returns true if successful, false otherwise
    #[func]
    pub fn set_game_mode(&mut self, mode: i32, save_config: bool) -> bool {
        // Update the network mode
        self.network_mode = mode;
        
        // Apply the network mode to the configuration
        let result = self.sync_property_to_config("network_mode", self.network_mode.to_variant());
        
        // If requested, save the configuration
        if result && save_config {
            return self.save_config();
        }
        
        result
    }
    
    /// Validate configuration for a specific game mode
    ///
    /// Checks if the configuration has all required properties for the specified mode
    /// 
    /// Parameters:
    /// - mode: 0=Standalone, 1=Host, 2=Client, -1=Current mode
    /// 
    /// Returns true if configuration is valid for the mode, false otherwise
    #[func]
    pub fn validate_for_mode(&self, mode: i32) -> bool {
        // Use current mode if -1 is passed
        let check_mode = if mode < 0 { self.network_mode } else { mode };
        
        // Check basic requirements for all modes
        if self.world_seed == 0 {
            godot_error!("World seed is required");
            return false;
        }
        
        // Check mode-specific requirements
        match check_mode {
            0 => true, // Standalone mode has minimal requirements
            1 => {
                // Host mode requirements
                if self.server_port <= 0 || self.server_port > 65535 {
                    godot_error!("Invalid server port (must be 1-65535)");
                    return false;
                }
                if self.max_players <= 0 {
                    godot_error!("Invalid max players (must be positive)");
                    return false;
                }
                true
            },
            2 => {
                // Client mode requirements
                if self.server_address.is_empty() {
                    godot_error!("Server address is required for client mode");
                    return false;
                }
                true
            },
            _ => {
                godot_error!("Invalid network mode: {}", check_mode);
                false
            }
        }
    }

    /// Apply multiple configuration settings at once
    ///
    /// This allows setting multiple properties and only saving once
    /// 
    /// Parameters:
    /// - property_map: Dictionary mapping property names to values
    /// - save_after: Whether to save the configuration after applying changes
    /// 
    /// Returns true if all properties were applied successfully, false otherwise
    #[func]
    pub fn apply_multiple_settings(&mut self, property_map: Dictionary, save_after: bool) -> bool {
        // Track whether all properties were applied successfully
        let mut all_successful = true;
        
        // Keep track of which properties were modified
        let mut modified_properties = vec![];
        
        // Apply each property
        for (key_variant, value) in property_map.iter_shared() {
            // Convert key to string
            if let Ok(key) = key_variant.try_to::<GString>() {
                let property_name = key.to_string();
                let result = match property_name.as_str() {
                    "world_seed" => {
                        if let Ok(seed) = value.try_to::<i64>() {
                            self.world_seed = seed;
                            self.sync_property_to_config("world_seed", self.world_seed.to_variant())
                        } else {
                            godot_error!("Invalid value type for world_seed");
                            false
                        }
                    },
                    "world_width" => {
                        if let Ok(width) = value.try_to::<i32>() {
                            self.world_width = width;
                            self.sync_property_to_config("world_width", self.world_width.to_variant())
                        } else {
                            godot_error!("Invalid value type for world_width");
                            false
                        }
                    },
                    "world_height" => {
                        if let Ok(height) = value.try_to::<i32>() {
                            self.world_height = height;
                            self.sync_property_to_config("world_height", self.world_height.to_variant())
                        } else {
                            godot_error!("Invalid value type for world_height");
                            false
                        }
                    },
                    "max_players" => {
                        if let Ok(max) = value.try_to::<i32>() {
                            self.max_players = max;
                            self.sync_property_to_config("max_players", self.max_players.to_variant())
                        } else {
                            godot_error!("Invalid value type for max_players");
                            false
                        }
                    },
                    "server_port" => {
                        if let Ok(port) = value.try_to::<i32>() {
                            self.server_port = port;
                            self.sync_property_to_config("server_port", self.server_port.to_variant())
                        } else {
                            godot_error!("Invalid value type for server_port");
                            false
                        }
                    },
                    "network_mode" => {
                        if let Ok(mode) = value.try_to::<i32>() {
                            self.network_mode = mode;
                            self.sync_property_to_config("network_mode", self.network_mode.to_variant())
                        } else {
                            godot_error!("Invalid value type for network_mode");
                            false
                        }
                    },
                    "server_address" => {
                        if let Ok(address) = value.try_to::<GString>() {
                            self.server_address = address;
                            self.sync_property_to_config("server_address", self.server_address.to_variant())
                        } else {
                            godot_error!("Invalid value type for server_address");
                            false
                        }
                    },
                    _ => {
                        // For any other key, treat it as a custom value
                        self.set_custom_value(key, value)
                    }
                };
                
                if result {
                    modified_properties.push(property_name);
                } else {
                    all_successful = false;
                }
            } else {
                godot_error!("Invalid property key type (must be string)");
                all_successful = false;
            }
        }
        
        // Save configuration if requested and if anything was modified
        if save_after && !modified_properties.is_empty() {
            let save_result = self.save_config();
            if !save_result {
                godot_error!("Failed to save configuration after applying multiple settings");
                return false;
            }
        }
        
        // If we're in debug mode and properties were modified, log them
        if self.debug_mode && !modified_properties.is_empty() {
            godot_print!("ConfigBridge: Applied {} configuration settings: {:?}", 
                        modified_properties.len(), modified_properties);
        }
        
        all_successful
    }

    /// Get multiple configuration properties at once
    ///
    /// Returns a Dictionary containing the requested properties
    ///
    /// Parameters:
    /// - property_names: Array of property names to fetch
    ///
    /// Returns a Dictionary with property names as keys and their values
    #[func]
    pub fn get_multiple_settings(&self, property_names: VariantArray) -> Dictionary {
        let mut result = Dictionary::new();
        
        for property_variant in property_names.iter_shared() {
            if let Ok(property_name) = property_variant.try_to::<GString>() {
                let value = match property_name.to_string().as_str() {
                    "world_seed" => self.world_seed.to_variant(),
                    "world_width" => self.world_width.to_variant(),
                    "world_height" => self.world_height.to_variant(),
                    "max_players" => self.max_players.to_variant(),
                    "server_port" => self.server_port.to_variant(),
                    "network_mode" => self.network_mode.to_variant(),
                    "server_address" => self.server_address.to_variant(),
                    _ => {
                        // Try to get it as a custom value
                        let custom_value = self.get_custom_value(property_name.clone());
                        if custom_value.is_nil() {
                            continue; // Skip this property
                        }
                        custom_value
                    }
                };
                
                result.insert(property_name.to_variant(), value);
            }
        }
        
        result
    }    

    /// Get the configuration as a complete dictionary
    ///
    /// Returns all configuration values as a Dictionary
    ///
    /// Returns a Dictionary with property names as keys and their values
    #[func]
    pub fn get_configuration_dictionary(&self) -> Dictionary {
        let mut result = Dictionary::new();
        
        // Add basic properties
        result.insert("world_seed".to_variant(), self.world_seed.to_variant());
        result.insert("world_width".to_variant(), self.world_width.to_variant());
        result.insert("world_height".to_variant(), self.world_height.to_variant());
        result.insert("max_players".to_variant(), self.max_players.to_variant());
        result.insert("server_port".to_variant(), self.server_port.to_variant());
        result.insert("network_mode".to_variant(), self.network_mode.to_variant());
        result.insert("server_address".to_variant(), self.server_address.to_variant());
        
        // Add all custom settings
        if let Some(config_manager) = &self.config_manager {
            if let Ok(manager) = config_manager.lock() {
                let config = manager.get_config();
                for (key, value) in &config.custom_settings {
                    let key_gstring = GString::from(key);
                    let variant_value = match value {
                        ConfigValue::String(s) => s.to_variant(),
                        ConfigValue::Integer(i) => i.to_variant(),
                        ConfigValue::Float(f) => f.to_variant(),
                        ConfigValue::Boolean(b) => b.to_variant(),
                    };
                    result.insert(key_gstring.to_variant(), variant_value);
                }
            }
        }
        
        result
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
    fn sync_property_to_config(&mut self, property_name: &str, value: Variant) -> bool {
        // Use a separate scope to manage borrows and compute result
        let result = if let Some(config_manager) = &self.config_manager {
            config_manager.lock().map(|mut manager| {
                // Clone the current configuration
                let mut config = manager.get_config().clone();
                
                // Update the appropriate configuration property
                let property_updated = match property_name {
                    "world_seed" => {
                        config.world_seed = self.world_seed as u64;
                        true
                    },
                    "world_width" => {
                        config.world_size.width = self.world_width as u32;
                        true
                    },
                    "world_height" => {
                        config.world_size.height = self.world_height as u32;
                        true
                    },
                    "max_players" => {
                        config.network.max_players = self.max_players as u8;
                        true
                    },
                    "server_port" => {
                        config.network.server_port = self.server_port as u16;
                        true
                    },
                    "network_mode" => {
                        // Convert mode to GameModeConfig
                        match self.network_mode {
                            0 => {
                                config.game_mode = GameModeConfig::Standalone;
                                true
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
                                true
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
                                true
                            },
                            _ => {
                                godot_error!("Invalid network mode: {}", self.network_mode);
                                false
                            }
                        }
                    },
                    "server_address" => {
                        // Only update if in client mode
                        if let GameModeConfig::Client(ref mut client_config) = config.game_mode {
                            client_config.server_address = self.server_address.to_string();
                            true
                        } else {
                            if self.debug_mode {
                                godot_print!("ConfigBridge: Not in client mode, server address not updated");
                            }
                            false
                        }
                    },
                    _ => {
                        godot_error!("Unknown property: {}", property_name);
                        false
                    }
                };
                
                // Only update the configuration if the property was successfully updated
                if property_updated {
                    manager.update_config(config);
                }
                
                property_updated
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
        
        result
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
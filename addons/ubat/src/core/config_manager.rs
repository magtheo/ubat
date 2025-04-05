use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::terrain::generation_rules::GenerationRules;

// Core configuration structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameConfiguration {
    // World Generation Parameters
    pub world_seed: u64,
    pub world_size: WorldSize,
    pub generation_rules: GenerationRules, 
    
    // Networking Configuration
    pub network: NetworkConfig,
    
    // Game Mode Specific Settings
    pub game_mode: GameModeConfig,
    
    // Custom configuration sections
    pub custom_settings: HashMap<String, ConfigValue>,
}

// World size representation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorldSize {
    pub width: u32,
    pub height: u32,
}

// Network configuration
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct NetworkConfig {
    pub max_players: u8,
    pub server_port: u16,
    pub connection_timeout: u32, // milliseconds
}

// Game mode specific configurations
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum GameModeConfig {
    Standalone,
    Host(HostConfig),
    Client(ClientConfig),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HostConfig {
    pub world_generation_seed: u64,
    pub admin_password: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClientConfig {
    pub server_address: String,
    pub username: String,
}

// Flexible configuration value
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ConfigValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    // Extensible for more complex types
}

// Configuration Manager
pub struct ConfigurationManager {
    current_config: GameConfiguration,
    config_path: Option<String>,
    is_initialized: bool,
}

impl ConfigurationManager {  
    // Create a new configuration with specific config
    pub fn with_config(config: GameConfiguration, config_path: Option<String>) -> Self {
        Self {
            current_config: config,
            config_path,
            is_initialized: true,
        }
    }

    // Create a default configuration
    fn create_default_config() -> GameConfiguration {
        GameConfiguration {
            world_seed: Self::generate_default_seed(),
            world_size: WorldSize {
                width: 10000,
                height: 10000,
            },
            generation_rules: GenerationRules::default(),
            network: NetworkConfig {
                max_players: 64,
                server_port: 7878,
                connection_timeout: 5000,
            },
            game_mode: GameModeConfig::Standalone,
            custom_settings: HashMap::new(),
        }
    }

    // Generate a deterministic default seed
    fn generate_default_seed() -> u64 {
        // Use a combination of system time and some entropy
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    // Load configuration from a file, returns a new ConfigurationManager
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        // Read and parse TOML as before
        let config_str = fs::read_to_string(path.as_ref())?;
        
        // Try to parse it as is
        let config: GameConfiguration = match toml::from_str(&config_str) {
            Ok(cfg) => cfg,
            Err(e) => {
                // Check if the error is about missing game_mode
                if e.to_string().contains("missing field `game_mode`") {
                    // Try to parse without that field
                    #[derive(Deserialize)]
                    struct PartialConfig {
                        world_seed: u64,
                        world_size: WorldSize,
                        generation_rules: GenerationRules,
                        network: NetworkConfig,
                        #[serde(default)]
                        custom_settings: HashMap<String, ConfigValue>,
                    }
                    
                    let partial: PartialConfig = toml::from_str(&config_str)
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                    
                    // Create a complete config with default game mode
                    GameConfiguration {
                        world_seed: partial.world_seed,
                        world_size: partial.world_size,
                        generation_rules: partial.generation_rules,
                        network: partial.network,
                        game_mode: GameModeConfig::Standalone, // Default
                        custom_settings: partial.custom_settings,
                    }
                } else {
                    // If it's some other error, propagate it
                    return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e));
                }
            }
        };
        
        Ok(Self {
            current_config: config,
            config_path: Some(path.as_ref().to_string_lossy().into_owned()),
            is_initialized: true,
        })
    }

    // Save configuration to file
    pub fn save_to_file(&self) -> Result<(), std::io::Error> {
        if let Some(path) = &self.config_path {
            let toml_string = toml::to_string(&self.current_config)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            fs::write(path, toml_string)?;
        }
        Ok(())
    }

    // Set a new config path
    pub fn set_config_path<P: AsRef<Path>>(&mut self, path: P) {
        self.config_path = Some(path.as_ref().to_string_lossy().into_owned());
    }

    // Update configuration
    pub fn update_config(&mut self, updates: GameConfiguration) {
        self.current_config = updates;
    }

    // Get a specific configuration value
    pub fn get<T: Clone>(&self, key: &str) -> Option<T> 
    where ConfigValue: Into<T> {
        self.current_config.custom_settings.get(key)
            .and_then(|val| Some(val.clone().into()))
    }

    // Get all config values
    pub fn get_config(&self) -> &GameConfiguration {
        &self.current_config
    }


    // Set a custom configuration value
    pub fn set(&mut self, key: String, value: ConfigValue) {
        self.current_config.custom_settings.insert(key, value);
    }

    // Check if manager is initialized
    pub fn is_initialized(&self) -> bool {
        self.is_initialized
    }

    // Validate configuration
    pub fn validate(&self) -> Result<(), ConfigurationError> {
        // Add validation logic
        match &self.current_config.game_mode {
            GameModeConfig::Host(host_config) => {
                if host_config.world_generation_seed == 0 {
                    return Err(ConfigurationError::InvalidSeed);
                }
            },
            GameModeConfig::Client(client_config) => {
                if client_config.server_address.is_empty() {
                    return Err(ConfigurationError::InvalidServerAddress);
                }
            },
            _ => {}
        }
        Ok(())
    }
}

impl Default for ConfigurationManager {
    fn default() -> Self {
        Self {
            current_config: Self::create_default_config(), // Use the existing method
            config_path: None,
            is_initialized: true,
        }
    }
}

// Custom error type for configuration errors
#[derive(Debug)]
pub enum ConfigurationError {
    InvalidSeed,
    InvalidServerAddress,
    NetworkConfigError,
}

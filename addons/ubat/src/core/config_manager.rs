use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

// Core configuration structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameConfiguration {
    // World Generation Parameters
    pub world_seed: u64,
    pub world_size: WorldSize,
    
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
#[derive(Debug, Serialize, Deserialize, Clone)]
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
}

impl ConfigurationManager {
    // Create a new configuration manager
    pub fn new(default_config: Option<GameConfiguration>) -> Self {
        Self {
            current_config: default_config.unwrap_or_else(|| Self::create_default_config()),
            config_path: None,
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

    // Load configuration from a file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let config_str = fs::read_to_string(path.as_ref())?;
        let config: GameConfiguration = toml::from_str(&config_str)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        
        Ok(Self {
            current_config: config,
            config_path: Some(path.as_ref().to_string_lossy().into_owned()),
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

    // Set a custom configuration value
    pub fn set(&mut self, key: String, value: ConfigValue) {
        self.current_config.custom_settings.insert(key, value);
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

// Custom error type for configuration errors
#[derive(Debug)]
pub enum ConfigurationError {
    InvalidSeed,
    InvalidServerAddress,
    NetworkConfigError,
}

// Example usage
fn demonstrate_configuration_management() {
    // Create default configuration
    let mut config_manager = ConfigurationManager::new(None);

    // Modify configuration
    config_manager.update_config(GameConfiguration {
        game_mode: GameModeConfig::Host(HostConfig {
            world_generation_seed: 12345,
            admin_password: Some("admin123".to_string()),
        }),
        ..config_manager.current_config.clone()
    });

    // Save configuration
    config_manager.save_to_file().unwrap();

    // Load configuration
    let loaded_config = ConfigurationManager::load_from_file("config.toml").unwrap();

    // Validate configuration
    if let Err(e) = loaded_config.validate() {
        println!("Configuration error: {:?}", e);
    }
}
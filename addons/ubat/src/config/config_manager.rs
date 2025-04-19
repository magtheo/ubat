// src/core/config_manager.rs

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::io; // For io::Error

use crate::terrain::generation_rules::{GenerationRules}; // Ensure this path is correct and derives traits

// Default values
pub fn default_server_address() -> String { "127.0.0.1:7878".to_string() }
pub fn default_username() -> String { "Player".to_string() }


// --- Struct Definitions ---

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TerrainInitialConfigData { // Represents the data loaded from TOML [terrain] section
    pub max_threads: usize,
    pub chunk_size: u32,
    pub blend_distance: f32,
    pub use_parallel_processing: bool,
    pub chunk_cache_size: usize,
    pub chunks_per_frame: usize,
    pub render_distance: i32,
    #[serde(default)]
    pub noise_paths: HashMap<String, String>,
}

// Default for TerrainInitialConfigData - used if file/section missing
impl Default for TerrainInitialConfigData {
    fn default() -> Self {
        let cpu_count = num_cpus::get();
        godot::prelude::godot_print!("Creating default TerrainInitialConfigData"); // Use Godot print
        TerrainInitialConfigData {
            max_threads: std::cmp::max(1, cpu_count.saturating_sub(1)),
            chunk_size: 32,
            blend_distance: 200.0,
            use_parallel_processing: true,
            chunk_cache_size: 400,
            chunks_per_frame: 4,
            render_distance: 2,
            noise_paths: HashMap::new(), // Default to empty
        }
    }
}

// World size representation (Keep as is)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorldSize {
    pub width: u32,
    pub height: u32,
}
impl Default for WorldSize {
    fn default() -> Self { WorldSize { width: 10000, height: 10000 } }
}

// Network configuration (Keep as is, maybe rename to avoid clash with NetworkConfig enum?)
// Renaming to NetworkInitialConfigData to be clear it comes from TOML
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkInitialConfigData {
    pub default_port: u16,
    pub max_players: u8,
    pub connection_timeout_ms: u32,
}
impl Default for NetworkInitialConfigData {
     fn default() -> Self {
         NetworkInitialConfigData {
             default_port: 7878,
             max_players: 64,
             connection_timeout_ms: 5000,
         }
     }
}

// Game mode specific configurations (Keep as is)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum GameModeConfig {
    Standalone,
    Host(HostConfig),
    Client(ClientConfig),
}
impl Default for GameModeConfig { // Need a default for GameConfiguration default
    fn default() -> Self { GameModeConfig::Standalone }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct HostConfig {
    // These are typically set at runtime, maybe remove from default config?
    // Keep if you want defaults, but they'll likely be overwritten by options Dict
    #[serde(default = "ConfigurationManager::generate_default_seed")]
    pub world_generation_seed: u64,
    #[serde(default)]
    pub admin_password: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ClientConfig {
    #[serde(default = "default_server_address")]
    pub server_address: String,
    #[serde(default = "default_username")]
    pub username: String,
}

// Flexible configuration value (Keep as is)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ConfigValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

// --- Main GameConfiguration Struct ---
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GameConfiguration {
    #[serde(default)]
    pub debug_mode: bool,

    // World Generation Parameters (using WorldSize default)
    #[serde(default = "ConfigurationManager::generate_default_seed")]
    pub world_seed: u64,
    #[serde(default)]
    pub world_size: WorldSize,
    #[serde(default)]
    pub generation_rules: GenerationRules, // Assumes GenerationRules::default() exists

    // Initial Network Config from TOML
    #[serde(default)]
    pub network: NetworkInitialConfigData, // Use the renamed struct

    // Initial Terrain Config from TOML
    #[serde(default)]
    pub terrain: TerrainInitialConfigData, // Use the new struct

    // Custom configuration sections
    #[serde(default)]
    pub custom_settings: HashMap<String, ConfigValue>,

    // --- Runtime State (Not serialized) ---
    #[serde(skip, default)]
    pub game_mode: GameModeConfig,
}


// Configuration Manager (Keep most methods, update load/save/default)
pub struct ConfigurationManager {
    current_config: GameConfiguration,
    config_path: Option<String>, // Path used for loading/saving
    is_initialized: bool, // Keep this? Global init handles it mostly. Maybe remove.
}

impl ConfigurationManager {
    // Keep with_config if manual creation is needed elsewhere
    pub fn with_config(config: GameConfiguration, config_path: Option<String>) -> Self {
        Self {
            current_config: config,
            config_path,
            is_initialized: true,
        }
    }

    // Update create_default_config to use struct defaults
    fn create_default_config() -> GameConfiguration {
         godot::prelude::godot_print!("Creating default GameConfiguration in ConfigurationManager"); // Use Godot print
         GameConfiguration::default() // Rely on derive(Default) and sub-struct defaults
    }

    // Helper for seed generation
    pub fn generate_default_seed() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    // Load configuration from a file - updated error type
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, io::Error> {
        let path_ref = path.as_ref();
        godot::prelude::godot_print!("Loading config from: {:?}", path_ref); // Use Godot print
        let config_str = fs::read_to_string(path_ref)?;

        let config: GameConfiguration = toml::from_str(&config_str)
            .map_err(|e| {
                godot::prelude::godot_error!("Failed to parse TOML config: {}", e); // Use Godot print
                io::Error::new(io::ErrorKind::InvalidData, e)
            })?;

        Ok(Self {
            current_config: config,
            config_path: Some(path_ref.to_string_lossy().into_owned()),
            is_initialized: true,
        })
    }

    // Save configuration to file (ensure it uses the stored path)
    pub fn save_to_file(&self) -> Result<(), io::Error> {
        if let Some(path) = &self.config_path {
            godot::prelude::godot_print!("Saving config to: {}", path); // Use Godot print
            let toml_string = toml::to_string_pretty(&self.current_config) // Use pretty print
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            fs::write(path, toml_string)?;
            Ok(())
        } else {
            godot::prelude::godot_warn!("Cannot save configuration: No config path set."); // Use Godot print
            // Optionally return an error here if saving without a path is invalid
            Ok(()) // Or Err(io::Error::new(io::ErrorKind::NotFound, "Config path not set"))
        }
    }

    // Set config path (Keep as is)
    pub fn set_config_path<P: AsRef<Path>>(&mut self, path: P) {
        self.config_path = Some(path.as_ref().to_string_lossy().into_owned());
    }

    // Update configuration (Applies a whole new config struct)
    pub fn update_config(&mut self, updates: GameConfiguration) {
        self.current_config = updates;
        // Maybe trigger a save here? Or leave it manual.
        // let _ = self.save_to_file();
    }

    // Get the whole config struct (mutable access needed for ConfigurationService)
    pub fn get_config_mut(&mut self) -> &mut GameConfiguration {
        &mut self.current_config
    }

    // Get immutable reference
    pub fn get_config(&self) -> &GameConfiguration {
        &self.current_config
    }

    // Set custom value (Keep as is)
    pub fn set(&mut self, key: String, value: ConfigValue) {
        self.current_config.custom_settings.insert(key, value);
    }

    // Validate configuration (Keep as is, maybe enhance)
    pub fn validate(&self) -> Result<(), ConfigurationError> {
        // Add validation logic based on GameConfiguration fields
        match &self.current_config.game_mode {
            GameModeConfig::Host(_host_config) => {
                // Validation for host...
            },
            GameModeConfig::Client(client_config) => {
                if client_config.server_address.is_empty() {
                    return Err(ConfigurationError::InvalidServerAddress);
                }
            },
            _ => {}
        }
        if self.current_config.world_size.width == 0 || self.current_config.world_size.height == 0 {
             return Err(ConfigurationError::InvalidWorldSize);
        }
        Ok(())
    }

    // is_initialized might be less relevant now global init handles it
    pub fn is_initialized(&self) -> bool {
        // This now just indicates if *this instance* holds data.
        // The global initialization state is managed by OnceCell.
        true // Assuming an instance always holds data (either loaded or default)
    }
}

// Default for ConfigurationManager - uses the default GameConfiguration
impl Default for ConfigurationManager {
    fn default() -> Self {
        Self {
            current_config: Self::create_default_config(),
            config_path: None, // No path known when using default
            is_initialized: true, // Instance is ready with default data
        }
    }
}

// Custom error type for configuration errors (add more variants as needed)
#[derive(Debug)]
pub enum ConfigurationError {
    InvalidSeed,
    InvalidServerAddress,
    NetworkConfigError,
    InvalidWorldSize, // Added example
}

// Conversion for ConfigValue get - You might need to adjust this based on T
impl Into<String> for ConfigValue { fn into(self) -> String { if let ConfigValue::String(s) = self { s } else { String::new() } } }
impl Into<i64> for ConfigValue { fn into(self) -> i64 { if let ConfigValue::Integer(i) = self { i } else { 0 } } }
impl Into<f64> for ConfigValue { fn into(self) -> f64 { if let ConfigValue::Float(f) = self { f } else { 0.0 } } }
impl Into<bool> for ConfigValue { fn into(self) -> bool { if let ConfigValue::Boolean(b) = self { b } else { false } } }
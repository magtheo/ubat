// system_initializer.rs
use std::sync::{Arc, Mutex, RwLock};
use std::path::Path;

use godot::prelude::*;

use crate::core::event_bus::{EventBus, WorldGeneratedEvent};
use crate::core::config_manager::{ConfigurationManager, GameConfiguration, GameModeConfig};
use crate::core::game_manager::{GameManager, GameState, GameError};
use crate::core::world_manager::{WorldStateManager, WorldStateConfig};
use crate::networking::network_manager::{NetworkHandler, NetworkConfig, NetworkMode};
use crate::terrain::{BiomeManager, ChunkManager, TerrainWorldIntegration, TerrainInitializationState};
use crate::utils::error_logger::{ErrorLogger, ErrorSeverity};

/// Represents the current state of system initialization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitializationState {
    Uninitialized,
    CoreServicesInitialized,
    GameSystemsInitialized,
    TerrainSystemsInitialized,
    NetworkInitialized,
    Complete,
    Error(InitializationError),
}

/// Specific error types that can occur during initialization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitializationError {
    ConfigError,
    WorldError,
    TerrainError,
    NetworkError,
    ResourceError,
    GeneralError,
}

/// Container for all initialized system references
pub struct SystemBundle {
    // Core services
    pub event_bus: Arc<EventBus>,
    pub config_manager: Arc<Mutex<ConfigurationManager>>,
    pub error_logger: Arc<ErrorLogger>,
    
    // Game systems
    pub game_manager: Option<Arc<Mutex<GameManager>>>,
    pub world_manager: Option<Arc<Mutex<WorldStateManager>>>,
    
    // Network system
    pub network_handler: Option<Arc<Mutex<NetworkHandler>>>,
    
    // Terrain systems
    pub biome_manager: Option<Gd<BiomeManager>>,
    pub chunk_manager: Option<Gd<ChunkManager>>,
    pub terrain_integration: Option<Arc<TerrainWorldIntegration>>,
    
    // Initialization state
    pub state: InitializationState,
}

/// Options for system initialization
pub struct InitializationOptions {
    pub config_path: Option<String>,
    pub default_seed: Option<u64>,
    pub network_mode: NetworkMode,
    pub debug_mode: bool,
    pub terrain_enabled: bool,
}

impl Default for InitializationOptions {
    fn default() -> Self {
        Self {
            config_path: None,
            default_seed: None,
            network_mode: NetworkMode::Standalone,
            debug_mode: false,
            terrain_enabled: true,
        }
    }
}

/// Main system initializer that handles phased initialization of all game systems
pub struct SystemInitializer {
    options: InitializationOptions,
    state: InitializationState,
    error_logger: Arc<ErrorLogger>,
}

impl SystemInitializer {
    /// Create a new system initializer with default options
    pub fn new() -> Self {
        Self::with_options(InitializationOptions::default())
    }
    
    /// Create a new system initializer with custom options
    pub fn with_options(options: InitializationOptions) -> Self {
        // Initialize error logger first so we can use it during initialization
        let error_logger = Arc::new(ErrorLogger::new(100));
        
        Self {
            options,
            state: InitializationState::Uninitialized,
            error_logger,
        }
    }
    
    /// Initialize all systems and return a bundle of system references
    pub fn initialize(&mut self) -> Result<SystemBundle, InitializationError> {
        // Step 1: Initialize core services
        let (event_bus, config_manager) = self.initialize_core_services()?;
        self.state = InitializationState::CoreServicesInitialized;
        
        // Step 2: Initialize game systems
        let (game_manager, world_manager) = self.initialize_game_systems(&event_bus, &config_manager)?;
        self.state = InitializationState::GameSystemsInitialized;
        
        // Step 3: Initialize terrain system if enabled
        let (biome_manager, chunk_manager, terrain_integration) = 
            if self.options.terrain_enabled {
                self.initialize_terrain_systems(&event_bus, &world_manager)?
            } else {
                (None, None, None)
            };
        self.state = InitializationState::TerrainSystemsInitialized;
        
        // Step 4: Initialize network if not in standalone mode
        let network_handler = if self.options.network_mode != NetworkMode::Standalone {
            match self.initialize_network_system(&event_bus, &config_manager) {
                Ok(handler) => Some(handler),
                Err(e) => {
                    self.error_logger.log_error(
                        "SystemInitializer", 
                        &format!("Failed to initialize network: {:?}", e),
                        ErrorSeverity::Warning,
                        None
                    );
                    None
                }
            }
        } else {
            None
        };
        self.state = InitializationState::NetworkInitialized;
        
        // Step 5: Connect all systems via event subscriptions
        self.connect_system_events(
            &event_bus, 
            &world_manager, 
            network_handler.as_ref(),
            terrain_integration.as_ref()
        );
        
        // Complete initialization
        self.state = InitializationState::Complete;
        
        // Return the bundle of system references
        Ok(SystemBundle {
            event_bus,
            config_manager,
            error_logger: self.error_logger.clone(),
            game_manager,
            world_manager,
            network_handler,
            biome_manager,
            chunk_manager,
            terrain_integration,
            state: self.state,
        })
    }
    
    /// Initialize core services (EventBus, ConfigManager)
    fn initialize_core_services(&self) 
        -> Result<(Arc<EventBus>, Arc<Mutex<ConfigurationManager>>), InitializationError> {
        // Create event bus
        let event_bus = Arc::new(EventBus::new());
        
        // Create configuration manager
        let config_manager = if let Some(config_path) = &self.options.config_path {
            // Try to load configuration from file
            if Path::new(config_path).exists() {
                match ConfigurationManager::load_from_file(config_path) {
                    Ok(manager) => Arc::new(Mutex::new(manager)),
                    Err(e) => {
                        self.error_logger.log_error(
                            "SystemInitializer", 
                            &format!("Failed to load configuration: {}", e),
                            ErrorSeverity::Error,
                            None
                        );
                        
                        // Fall back to default configuration
                        Arc::new(Mutex::new(ConfigurationManager::new(None)))
                    }
                }
            } else {
                // Path doesn't exist, create default config
                Arc::new(Mutex::new(ConfigurationManager::new(None)))
            }
        } else {
            // No path specified, create default config
            Arc::new(Mutex::new(ConfigurationManager::new(None)))
        };
        
        // Apply default seed if provided
        if let Some(seed) = self.options.default_seed {
            if let Ok(mut manager) = config_manager.lock() {
                let mut config = manager.get_config().clone();
                config.world_seed = seed;
                manager.update_config(config);
            }
        }
        
        Ok((event_bus, config_manager))
    }
    
    /// Initialize game systems (GameManager, WorldStateManager)
    fn initialize_game_systems(
        &self,
        event_bus: &Arc<EventBus>,
        config_manager: &Arc<Mutex<ConfigurationManager>>
    ) -> Result<(
        Option<Arc<Mutex<GameManager>>>, 
        Arc<Mutex<WorldStateManager>>
    ), InitializationError> {
        // Create WorldStateManager
        let world_config = self.create_world_config(config_manager)?;
        let world_manager = Arc::new(Mutex::new(WorldStateManager::new(world_config)));
        
        // Initialize WorldStateManager
        {
            let mut manager = world_manager.lock().map_err(|_| InitializationError::WorldError)?;
            if let Err(e) = manager.initialize() {
                self.error_logger.log_error(
                    "SystemInitializer", 
                    &format!("Failed to initialize world manager: {}", e),
                    ErrorSeverity::Error,
                    None
                );
                return Err(InitializationError::WorldError);
            }
            
            // Set event bus
            manager.set_event_bus(event_bus.clone());
        }
        
        // Create GameManager if needed
        let game_manager = match GameManager::new() {
            ok_manager => {
                let mut manager = ok_manager;
                // Set references
                // Note: We're not using the lock pattern here because initialize() doesn't take &mut self
                if let Err(e) = manager.initialize() {
                    self.error_logger.log_error(
                        "SystemInitializer", 
                        &format!("Failed to initialize game manager: {:?}", e),
                        ErrorSeverity::Error,
                        None
                    );
                    None
                } else {
                    Some(Arc::new(Mutex::new(manager)))
                }
            }
        };
        
        Ok((game_manager, world_manager))
    }
    
    /// Initialize terrain systems (BiomeManager, ChunkManager, TerrainWorldIntegration)
    fn initialize_terrain_systems(
        &self,
        event_bus: &Arc<EventBus>,
        world_manager: &Arc<Mutex<WorldStateManager>>
    ) -> Result<(
        Option<Gd<BiomeManager>>, 
        Option<Gd<ChunkManager>>, 
        Option<Arc<TerrainWorldIntegration>>
    ), InitializationError> {
        // Create BiomeManager
        let mut biome_manager = BiomeManager::new_alloc();
        
        // Create ChunkManager
        let mut chunk_manager = ChunkManager::new_alloc();
        
        // Get world configuration data
        let (world_seed, world_size) = {
            let manager = world_manager.lock().map_err(|_| InitializationError::WorldError)?;
            let config = manager.get_config();
            (config.seed, (config.world_size.0, config.world_size.1))
        };
        
        // Configure BiomeManager
        {
            let mut bm = biome_manager.bind_mut();
            bm.set_seed(world_seed as u32);
            bm.set_world_dimensions(world_size.0 as f32, world_size.1 as f32);
            if !bm.comprehensive_initialization() {
                self.error_logger.log_error(
                    "SystemInitializer",
                    "Failed to initialize BiomeManager",
                    ErrorSeverity::Error,
                    None
                );
                return Err(InitializationError::TerrainError);
            }
        }
        
        // Configure ChunkManager
        {
            let mut cm = chunk_manager.bind_mut();
            cm.set_biome_manager(biome_manager.clone());
            cm.update_thread_safe_biome_data();
        }
        
        // Create TerrainWorldIntegration
        let terrain_integration = {
            let world_manager_clone = world_manager.clone();
            let terrain_integration = Arc::new(TerrainWorldIntegration::new(world_manager_clone));
            
            // Initialize terrain integration
            if let Err(e) = terrain_integration.initialize_terrain(biome_manager.clone(), chunk_manager.clone()) {
                self.error_logger.log_error(
                    "SystemInitializer",
                    &format!("Failed to initialize terrain integration: {}", e),
                    ErrorSeverity::Error,
                    None
                );
                return Err(InitializationError::TerrainError);
            }
            
            // Connect to event bus
            terrain_integration.connect_to_event_bus(event_bus.clone());
            
            terrain_integration
        };
        
        Ok((Some(biome_manager), Some(chunk_manager), Some(terrain_integration)))
    }
    
    /// Initialize network system
    fn initialize_network_system(
        &self,
        event_bus: &Arc<EventBus>,
        config_manager: &Arc<Mutex<ConfigurationManager>>
    ) -> Result<Arc<Mutex<NetworkHandler>>, InitializationError> {
        // Get configuration
        let network_config = {
            let manager = config_manager.lock().map_err(|_| InitializationError::ConfigError)?;
            let config = manager.get_config();
            
            match &config.game_mode {
                GameModeConfig::Standalone => {
                    NetworkConfig {
                        mode: NetworkMode::Standalone,
                        port: 0,
                        max_connections: 0,
                        server_address: None,
                    }
                },
                GameModeConfig::Host(host_config) => {
                    NetworkConfig {
                        mode: NetworkMode::Host,
                        port: config.network.server_port,
                        max_connections: config.network.max_players as usize,
                        server_address: None,
                    }
                },
                GameModeConfig::Client(client_config) => {
                    NetworkConfig {
                        mode: NetworkMode::Client,
                        port: 0,
                        max_connections: 1,
                        server_address: Some(client_config.server_address.clone()),
                    }
                },
            }
        };
        
        // Create network handler
        match NetworkHandler::new(network_config) {
            Ok(handler) => Ok(Arc::new(Mutex::new(handler))),
            Err(e) => {
                self.error_logger.log_error(
                    "SystemInitializer",
                    &format!("Failed to initialize network: {:?}", e),
                    ErrorSeverity::Error,
                    None
                );
                Err(InitializationError::NetworkError)
            }
        }
    }
    
    /// Connect systems via event subscriptions
    fn connect_system_events(
        &self,
        event_bus: &Arc<EventBus>,
        world_manager: &Arc<Mutex<WorldStateManager>>,
        network_handler: Option<&Arc<Mutex<NetworkHandler>>>,
        terrain_integration: Option<&Arc<TerrainWorldIntegration>>
    ) {
        // Set up world generation event subscriptions
        // This is just an example - add more as needed
        let world_manager_clone = world_manager.clone();
        let world_gen_handler = Arc::new(move |event: &WorldGeneratedEvent| {
            if let Ok(mut manager) = world_manager_clone.lock() {
                manager.set_pending_world_init(event.seed, event.world_size);
            }
        });
        
        event_bus.subscribe(world_gen_handler);
    }
    
    /// Helper to create WorldStateConfig from ConfigurationManager
    fn create_world_config(
        &self,
        config_manager: &Arc<Mutex<ConfigurationManager>>
    ) -> Result<WorldStateConfig, InitializationError> {
        let config = config_manager.lock()
            .map_err(|_| InitializationError::ConfigError)?
            .get_config()
            .clone();
        
        Ok(WorldStateConfig {
            seed: config.world_seed,
            world_size: (config.world_size.width, config.world_size.height),
            generation_parameters: config.generation_rules.clone(),
        })
    }
    
    /// Get current initialization state
    pub fn get_state(&self) -> InitializationState {
        self.state
    }
    
    /// Check if initialization is complete
    pub fn is_initialized(&self) -> bool {
        self.state == InitializationState::Complete
    }
}

/// Create a builder for the SystemInitializer
pub fn system_initializer() -> SystemInitializerBuilder {
    SystemInitializerBuilder::new()
}

/// Builder for SystemInitializer to provide a fluent API
pub struct SystemInitializerBuilder {
    options: InitializationOptions,
}

impl SystemInitializerBuilder {
    /// Create a new builder with default options
    pub fn new() -> Self {
        Self {
            options: InitializationOptions::default(),
        }
    }
    
    /// Set the configuration file path
    pub fn with_config_path(mut self, path: impl Into<String>) -> Self {
        self.options.config_path = Some(path.into());
        self
    }
    
    /// Set the default seed for world generation
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.options.default_seed = Some(seed);
        self
    }
    
    /// Set the network mode
    pub fn with_network_mode(mut self, mode: NetworkMode) -> Self {
        self.options.network_mode = mode;
        self
    }
    
    /// Enable or disable debug mode
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.options.debug_mode = debug;
        self
    }
    
    /// Enable or disable terrain system
    pub fn with_terrain(mut self, enabled: bool) -> Self {
        self.options.terrain_enabled = enabled;
        self
    }
    
    /// Build the SystemInitializer
    pub fn build(self) -> SystemInitializer {
        SystemInitializer::with_options(self.options)
    }
}
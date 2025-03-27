use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::thread;
use std::path::Path;

use crate::core::config_manager::{ConfigurationManager, GameConfiguration, GameModeConfig};
use crate::core::event_bus::{EventBus, PlayerConnectedEvent, WorldGeneratedEvent};
use crate::core::world_manager::{WorldStateManager, WorldStateConfig};
use crate::networking::network_manager::{NetworkHandler, NetworkConfig, NetworkMode, NetworkEvent, PeerId};

// Game state enum
#[derive(Debug, Clone, PartialEq)]
pub enum GameState {
    Initializing,
    MainMenu,
    Loading,
    Running,
    Paused,
    Exiting,
}

// Game initialization error
#[derive(Debug)]
pub enum GameError {
    ConfigError(String),
    NetworkError(String),
    WorldError(String),
    SystemError(String),
}

// Game event types specific to game logic
#[derive(Debug, Clone)]
pub enum GameEvent {
    StateChanged(GameState),
    PlayerJoined(String),
    PlayerLeft(String),
    WorldLoaded,
    ErrorOccurred(String),
    // Other game-specific events
}

// Main game manager
pub struct GameManager {
    // Game state
    state: GameState,
    running: bool,
    
    // Game configuration
    config_manager: Arc<Mutex<ConfigurationManager>>,
    
    // Event communication
    event_bus: Arc<EventBus>,
    
    // World state
    world_manager: Option<Arc<Mutex<WorldStateManager>>>,
    
    // Network handler
    network_handler: Option<Arc<Mutex<NetworkHandler>>>,
    
    // Game loop timing
    frame_rate: u32,
    last_update: Instant,
}

impl GameManager {
    // Create a new game manager
    pub fn new() -> Self {
        // Create with default configuration
        let config_manager = Arc::new(Mutex::new(
            ConfigurationManager::new(None)
        ));
        
        // Create event bus
        let event_bus = Arc::new(EventBus::new());
        
        Self {
            state: GameState::Initializing,
            running: false,
            config_manager,
            event_bus,
            world_manager: None,
            network_handler: None,
            frame_rate: 60, // Default frame rate
            last_update: Instant::now(),
        }
    }
    
    // Initialize from configuration file (accepts a String)
    pub fn init_from_config<S: AsRef<str>>(config_path: S) -> Result<Self, GameError> {
        // Convert to a Path
        let path = std::path::Path::new(config_path.as_ref());
        println!("GameManager: Loading from filesystem path: {:?}", path);
        
        // Check if the file exists
        if !path.exists() {
            return Err(GameError::ConfigError(format!("File not found: {:?}", path)));
        }
        
        // Try to load the configuration
        match ConfigurationManager::load_from_file(path) {
            Ok(config_manager) => {
                // Create event bus
                let event_bus = Arc::new(EventBus::new());
                
                Ok(Self {
                    state: GameState::Initializing,
                    running: false,
                    config_manager: Arc::new(Mutex::new(config_manager)),
                    event_bus,
                    world_manager: None,
                    network_handler: None,
                    frame_rate: 60,
                    last_update: Instant::now(),
                })
            },
            Err(e) => {
                Err(GameError::ConfigError(format!("Failed to load config: {}", e)))
            }
        }
    }
    
    // Initialize the game systems
    pub fn initialize(&mut self) -> Result<(), GameError> {
        self.transition_state(GameState::Initializing);
        
        // Get configuration
        let config = {
            let config_manager = self.config_manager.lock()
                .map_err(|_| GameError::SystemError("Failed to lock config manager".into()))?;
            config_manager.get_config().clone()
        };
        
        // Set up network based on game mode
        self.initialize_network(&config)?;
        
        // Set up world state manager
        self.initialize_world(&config)?;
        
        // Set up event handlers
        self.register_event_handlers();
        
        // Initial state
        self.transition_state(GameState::MainMenu);
        
        Ok(())
    }
    
    // Initialize networking based on game mode
    fn initialize_network(&mut self, config: &GameConfiguration) -> Result<(), GameError> {
        // Set up network configuration based on game mode
        let network_config = match &config.game_mode {
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
        };
        
        // Only create network handler if not in standalone mode
        // Using matches! is a more idiomatic way to check enum variants in Rust
        // This checks if network_config.mode matches the NetworkMode::Standalone pattern
        if !matches!(network_config.mode, NetworkMode::Standalone) {
            let handler = NetworkHandler::new(network_config)
                .map_err(|e| GameError::NetworkError(format!("Failed to initialize network: {:?}", e)))?;
            
            self.network_handler = Some(Arc::new(Mutex::new(handler)));
        }
        
        Ok(())
    }
    
    // Initialize world state manager
    fn initialize_world(&mut self, config: &GameConfiguration) -> Result<(), GameError> {
        // Create world state configuration
        let world_config = WorldStateConfig {
            seed: config.world_seed,
            world_size: (config.world_size.width, config.world_size.height),
            generation_parameters: config.generation_rules.clone(), // Use default rules
        };
        
        let world_manager = WorldStateManager::new(world_config.clone());
        self.world_manager = Some(Arc::new(Mutex::new(world_manager)));
        
        // If we're in host mode or standalone, generate the world
        match config.game_mode {
            GameModeConfig::Standalone | GameModeConfig::Host(_) => {
                if let Some(world_manager) = &self.world_manager {
                    let mut manager = world_manager.lock()
                        .map_err(|_| GameError::SystemError("Failed to lock world manager".into()))?;
                    
                    manager.generate_initial_world();
                    
                    // Publish world generated event
                    self.event_bus.publish(WorldGeneratedEvent {
                        seed: world_config.seed,
                        world_size: world_config.world_size,
                    });
                }
            },
            _ => {} // Client will receive world state from host
        }
        
        Ok(())
    }
    
    // Register event handlers
    fn register_event_handlers(&self) {
        let event_bus = self.event_bus.clone();
        let game_manager = self.clone();
        
        // Handle player connected events
        let player_handler = Arc::new(move |event: &PlayerConnectedEvent| {
            println!("Player connected: {}", event.player_id);
            // Additional logic can be added here
        });
        event_bus.subscribe(player_handler);
        
        // Handle world generation events
        let world_handler = Arc::new(|event: &WorldGeneratedEvent| {
            println!(
                "World generated with seed: {} and size: {:?}", 
                event.seed, 
                event.world_size
            );
        });
        event_bus.subscribe(world_handler);
        
        // Additional event handlers can be registered here
    }
    
    // Start the game
    pub fn start(&mut self) -> Result<(), GameError> {
        if self.state == GameState::Initializing {
            return Err(GameError::SystemError("Game not initialized".into()));
        }
        
        self.running = true;
        self.transition_state(GameState::Running);
        
        // Main game loop
        while self.running {
            self.update()?;
            
            // Control frame rate
            let frame_duration = Duration::from_secs_f32(1.0 / self.frame_rate as f32);
            let elapsed = self.last_update.elapsed();
            
            if elapsed < frame_duration {
                thread::sleep(frame_duration - elapsed);
            }
            
            self.last_update = Instant::now();
        }
        
        // Perform cleanup
        self.shutdown();
        
        Ok(())
    }
    
    // Update game state
    pub fn update(&mut self) -> Result<(), GameError> {
        // Process network events first
        if let Some(network_handler) = &self.network_handler {
            let handler = network_handler.lock()
                .map_err(|_| GameError::SystemError("Failed to lock network handler".into()))?;
            
            // Process all pending network events
            while let Some(event) = handler.poll_events() {
                self.handle_network_event(event)?;
            }
        }
        
        // Update world state
        if let Some(world_manager) = &self.world_manager {
            let mut manager = world_manager.lock()
                .map_err(|_| GameError::SystemError("Failed to lock world manager".into()))?;
            
            // Update world logic
            // manager.update(); // You would need to implement this
        }
        
        // Check for game state changes or exit conditions
        
        Ok(())
    }

    pub fn set_frame_rate(&mut self, fps: u32) {
        self.frame_rate = fps;
        println!("Game frame rate set to {}", fps);
    }
    
    // Handle network events
    fn handle_network_event(&self, event: NetworkEvent) -> Result<(), GameError> {
        match event {
            NetworkEvent::Connected(peer_id) => {
                // Clone peer_id before moving it into the event
                let peer_id_clone = peer_id.clone();

                // Publish player connected event
                self.event_bus.publish(PlayerConnectedEvent {
                    player_id: peer_id,
                });
                
                // In host mode, send world state to new client
                if let (Some(world_manager), Some(network_handler)) = 
                    (&self.world_manager, &self.network_handler) 
                {
                    let world = world_manager.lock()
                        .map_err(|_| GameError::SystemError("Failed to lock world manager".into()))?;
                    
                    let serialized_state = world.serialize_world_state();
                    
                    let handler = network_handler.lock()
                        .map_err(|_| GameError::SystemError("Failed to lock network handler".into()))?;
                    
                    handler.send_to_peer(&peer_id_clone, "world_state", &serialized_state)
                        .map_err(|e| GameError::NetworkError(format!("Failed to send world state: {:?}", e)))?;
                }
            },
            NetworkEvent::Disconnected(peer_id) => {
                println!("Peer disconnected: {}", peer_id);
                // Handle disconnection, cleanup, etc.
            },
            NetworkEvent::DataReceived { peer_id, payload } => {
                // Process received data
                // This would typically dispatch to the appropriate handler
                // based on message type
            },
            NetworkEvent::ConnectionError(error) => {
                return Err(GameError::NetworkError(format!("Connection error: {:?}", error)));
            },
        }
        
        Ok(())
    }
    
    // Change game state with event notification
    pub fn transition_state(&mut self, new_state: GameState) {
        let old_state = self.state.clone();
        self.state = new_state.clone();
        
        println!("Game state changed: {:?} -> {:?}", old_state, new_state);
        
        // Publish state change event
        self.event_bus.publish(GameEvent::StateChanged(new_state));
    }
    
    // Get current game state
    pub fn get_state(&self) -> GameState {
        self.state.clone()
    }
    
    // Pause the game
    pub fn pause(&mut self) {
        if self.state == GameState::Running {
            self.transition_state(GameState::Paused);
        }
    }
    
    // Resume the game
    pub fn resume(&mut self) {
        if self.state == GameState::Paused {
            self.transition_state(GameState::Running);
        }
    }
    
    // Stop the game
    pub fn stop(&mut self) {
        self.running = false;
        self.transition_state(GameState::Exiting);
    }
    
    // Clean shutdown
    fn shutdown(&mut self) {
        println!("Shutting down game systems...");
        
        // Save configuration
        if let Ok(config_manager) = self.config_manager.lock() {
            if let Err(e) = config_manager.save_to_file() {
                eprintln!("Failed to save configuration: {}", e);
            }
        }
        
        // Other cleanup actions
        
        println!("Game shutdown complete");
    }
    
    // Clone for event handling
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            running: self.running,
            config_manager: self.config_manager.clone(),
            event_bus: self.event_bus.clone(),
            world_manager: self.world_manager.clone(),
            network_handler: self.network_handler.clone(),
            frame_rate: self.frame_rate,
            last_update: self.last_update,
        }
    }
}

// Example usage
fn demonstrate_game_manager() {
    // Create and initialize game manager
    let mut game_manager = match GameManager::init_from_config("game.toml") {
        Ok(manager) => manager,
        Err(e) => {
            eprintln!("Failed to initialize game: {:?}", e);
            return;
        }
    };
    
    if let Err(e) = game_manager.initialize() {
        eprintln!("Failed to initialize game systems: {:?}", e);
        return;
    }
    
    println!("Game initialized successfully. Starting game...");
    
    // Start the game
    if let Err(e) = game_manager.start() {
        eprintln!("Game error: {:?}", e);
    }
}
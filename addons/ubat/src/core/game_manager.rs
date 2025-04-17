use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use std::thread_local;
use std::cell::RefCell;

use crate::config::config_manager::{ConfigurationManager, GameConfiguration, GameModeConfig};
use crate::core::event_bus::{EventBus, PlayerConnectedEvent, WorldGeneratedEvent};
use crate::core::world_manager::{WorldStateManager, WorldStateConfig};
use crate::networking::network_manager::{NetworkHandler, NetworkConfig, NetworkMode, NetworkEvent};

// Static singleton instance
static mut INSTANCE: Option<Arc<Mutex<GameManager>>> = None;

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

impl From<String> for GameError {
    fn from(s: String) -> Self {
        GameError::WorldError(s)
    }
}

// Game event types specific to game logic
#[derive(Debug, Clone)]
pub enum GameEvent {
    StateChanged(GameState),
    WorldLoaded,
    ErrorOccurred(String),
}

// Main game manager
pub struct GameManager {
    // Game state
    state: GameState,
    running: bool,
    
    // Game configuration
    config_manager: Arc<RwLock<ConfigurationManager>>,
    
    // Event communication
    event_bus: Arc<EventBus>,
    
    // World state
    world_manager: Option<Arc<Mutex<WorldStateManager>>>,
    
    // Network handler
    network_handler: Option<Arc<Mutex<NetworkHandler>>>,
    
    // Game loop timing
    frame_rate: u32,
    last_update: Instant,

    // Initialization state
    initialized: bool,
}

thread_local! {
    static GAME_MANAGER_INSTANCE: RefCell<Option<Arc<Mutex<GameManager>>>> = RefCell::new(None);
}

pub fn get_instance() -> Option<Arc<Mutex<GameManager>>> {
    let mut result = None;
    GAME_MANAGER_INSTANCE.with(|cell| {
        if let Some(instance) = &*cell.borrow() {
            result = Some(instance.clone());
        }
    });
    result
}

pub fn set_instance(instance: Arc<Mutex<GameManager>>) {
    GAME_MANAGER_INSTANCE.with(|cell| {
        *cell.borrow_mut() = Some(instance);
    });
}


impl GameManager {
    // Create a new game manager without configuration - for initialization by system_initializer
    pub fn new() -> Self {
        Self {
            state: GameState::Initializing,
            running: false,
            config_manager: Arc::new(RwLock::new(ConfigurationManager::default())),
            event_bus: Arc::new(EventBus::new()),
            world_manager: None,
            network_handler: None,
            frame_rate: 60, // Default frame rate
            last_update: Instant::now(),
            initialized: false,
        }
    }

    // Create a new game manager with dependencies
    pub fn new_with_dependencies(
        config_manager: Arc<RwLock<ConfigurationManager>>,
        event_bus: Arc<EventBus>,
        world_manager: Option<Arc<Mutex<WorldStateManager>>>,
        network_handler: Option<Arc<Mutex<NetworkHandler>>>,
    ) -> Self {
        Self {
            state: GameState::Initializing,
            running: false,
            config_manager,
            event_bus,
            world_manager,
            network_handler,
            frame_rate: 60, // Default frame rate
            last_update: Instant::now(),
            initialized: false,
        }
    }

    // Setters for dependencies
    pub fn set_world_manager(&mut self, world_manager: Arc<Mutex<WorldStateManager>>) {
        self.world_manager = Some(world_manager);
    }
    
    pub fn set_network_handler(&mut self, network_handler: Arc<Mutex<NetworkHandler>>) {
        self.network_handler = Some(network_handler);
    }
    
    pub fn set_config_manager(&mut self, config_manager: Arc<RwLock<ConfigurationManager>>) {
        self.config_manager = config_manager;
    }
    
    pub fn set_event_bus(&mut self, event_bus: Arc<EventBus>) {
        self.event_bus = event_bus;
    }

    // Mark the manager as initialized 
    pub fn mark_initialized(&mut self) {
        self.initialized = true;
        self.transition_state(GameState::MainMenu);
    }

    // Ensure world initialization is complete
    pub fn ensure_world_initialized(&mut self) -> Result<(), GameError> {
        if self.world_manager.is_none() {
            return Err(GameError::WorldError("World manager not created".into()));
        }
    
        // Get the world configuration
        let config = {
            let config_manager = self.config_manager.read()
                .map_err(|_| GameError::SystemError("Failed to lock config manager".into()))?;
            config_manager.get_config().clone()
        };
        
        // Force full initialization of terrain if needed
        if let Some(world_manager) = &mut self.world_manager {
            let mut manager = world_manager.lock()
                .map_err(|_| GameError::SystemError("Failed to lock world manager".into()))?;
            
            // Check if we need to call initialize
            let initialization_needed = manager.initialize()
                .map_err(|e| GameError::WorldError(e))?;
                
            if matches!(config.game_mode, GameModeConfig::Standalone | GameModeConfig::Host(_)) {
                println!("GameManager: Ensuring world generation in ensure_world_initialized");
                // manager.generate_initial_world();
            }
        }
        
        Ok(())
    }
    
    // Start the game
    pub fn start_game(&mut self) -> Result<(), GameError> {
        if !self.initialized {
            return Err(GameError::SystemError("Game not initialized".into()));
        }
        
        // Ensure world is fully initialized before starting
        self.ensure_world_initialized()?;
        
        self.running = true;
        self.transition_state(GameState::Running);
        
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
            
            // Update world logic if needed
        }
        
        Ok(())
    }

    // Handle network events
    fn handle_network_event(&self, event: NetworkEvent) -> Result<(), GameError> {
        match event {
            NetworkEvent::Connected(peer_id) => {
                let peer_id_clone = peer_id.clone();

                // Publish player connected event
                self.event_bus.publish(PlayerConnectedEvent {
                    player_id: peer_id,
                });
                
                // Send world state to new client if in host mode
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
            },
            NetworkEvent::DataReceived { peer_id, payload } => {
                // Process received data
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
    pub fn shutdown(&mut self) {
        println!("Shutting down game systems...");
        
        // Save configuration
        if let Ok(config_manager) = self.config_manager.read() {
            if let Err(e) = config_manager.save_to_file() {
                eprintln!("Failed to save configuration: {}", e);
            }
        }
        
        // Reset state
        self.running = false;
        self.transition_state(GameState::Exiting);
        self.initialized = false;

        println!("Game shutdown complete");
    }
    
    // Check if manager is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
    
    // Setter for frame rate
    pub fn set_frame_rate(&mut self, fps: u32) {
        self.frame_rate = fps;
        println!("Game frame rate set to {}", fps);
    }
}
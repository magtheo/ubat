use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::thread;

use crate::core::config_manager::{ConfigurationManager, GameConfiguration, GameModeConfig};
use crate::core::event_bus::{EventBus, PlayerConnectedEvent, WorldGeneratedEvent};
use crate::core::world_manager::{WorldStateManager, WorldStateConfig};
use crate::networking::network_manager::{NetworkHandler, NetworkConfig, NetworkMode, NetworkEvent};
use crate::terrain::TerrainWorldIntegration;

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

    // Initialization state
    initialized: bool,
}

// Static module functions for system_initializer compatibility
pub fn initialize() -> Result<(), String> {
    // Create a default game manager if it doesn't exist
    unsafe {
        if INSTANCE.is_none() {
            let manager = GameManager::new();
            INSTANCE = Some(Arc::new(Mutex::new(manager)));
        }
    }
    Ok(())
}

pub fn configure_standalone() -> Result<(), String> {
    unsafe {
        if let Some(instance) = &INSTANCE {
            match instance.lock() {
                Ok(mut manager) => match manager.configure_standalone() {
                    Ok(_) => Ok(()),
                    Err(e) => Err(format!("Error configuring standalone: {:?}", e)),
                },
                Err(_) => Err("Failed to lock game manager".to_string()),
            }
        } else {
            Err("Game manager not initialized".to_string())
        }
    }
}

pub fn configure_host() -> Result<(), String> {
    unsafe {
        if let Some(instance) = &INSTANCE {
            match instance.lock() {
                Ok(mut manager) => match manager.configure_host() {
                    Ok(_) => Ok(()),
                    Err(e) => Err(format!("Error configuring host: {:?}", e)),
                },
                Err(_) => Err("Failed to lock game manager".to_string()),
            }
        } else {
            Err("Game manager not initialized".to_string())
        }
    }
}

pub fn configure_client() -> Result<(), String> {
    unsafe {
        if let Some(instance) = &INSTANCE {
            match instance.lock() {
                Ok(mut manager) => match manager.configure_client() {
                    Ok(_) => Ok(()),
                    Err(e) => Err(format!("Error configuring client: {:?}", e)),
                },
                Err(_) => Err("Failed to lock game manager".to_string()),
            }
        } else {
            Err("Game manager not initialized".to_string())
        }
    }
}

// Returns a reference to the game manager instance (for system_initializer)
pub fn get_instance() -> Option<Arc<Mutex<GameManager>>> {
    unsafe {
        INSTANCE.clone()
    }
}

impl GameManager {
    // Create a new game manager without configuration - for initialization by system_initializer
    pub fn new() -> Self {
        // Default construction for SystemInitializer to configure later
        Self {
            state: GameState::Initializing,
            running: false,
            config_manager: Arc::new(Mutex::new(ConfigurationManager::default())),
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
        config_manager: Arc<Mutex<ConfigurationManager>>,
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

    // Setters for dependencies - for the SystemInitializer to use
    pub fn set_world_manager(&mut self, world_manager: Arc<Mutex<WorldStateManager>>) {
        self.world_manager = Some(world_manager);
    }
    
    pub fn set_network_handler(&mut self, network_handler: Arc<Mutex<NetworkHandler>>) {
        self.network_handler = Some(network_handler);
    }
    
    pub fn set_config_manager(&mut self, config_manager: Arc<Mutex<ConfigurationManager>>) {
        self.config_manager = config_manager;
    }
    
    pub fn set_event_bus(&mut self, event_bus: Arc<EventBus>) {
        self.event_bus = event_bus;
    }

    // Mark the manager as initialized (called by the system initializer)
    pub fn mark_initialized(&mut self) {
        self.initialized = true;
        self.transition_state(GameState::MainMenu);
    }
    
    // Configure for standalone mode
    pub fn configure_standalone(&mut self) -> Result<(), GameError> {
        // Configure for standalone mode
        println!("GameManager: Configuring for standalone mode");
        
        // Register event handlers
        self.register_event_handlers();
        
        Ok(())
    }
    
    // Configure for host mode
    pub fn configure_host(&mut self) -> Result<(), GameError> {
        // Configure for host mode
        println!("GameManager: Configuring for host mode");
        
        // Register event handlers
        self.register_event_handlers();
        
        Ok(())
    }
    
    // Configure for client mode
    pub fn configure_client(&mut self) -> Result<(), GameError> {
        // Configure for client mode
        println!("GameManager: Configuring for client mode");
        
        // Register event handlers
        self.register_event_handlers();
        
        Ok(())
    }

    // Ensure world initialization is complete - can be called separately if needed
    pub fn ensure_world_initialized(&mut self) -> Result<(), GameError> {
        if self.world_manager.is_none() {
            // Something went wrong during initialization
            return Err(GameError::WorldError("World manager not created".into()));
        }
    
        // Get the world configuration
        let config = {
            let config_manager = self.config_manager.lock()
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
                manager.generate_initial_world();
            }
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
    pub fn start_game(&mut self) -> Result<(), GameError> {
        if !self.initialized {
            return Err(GameError::SystemError("Game not initialized".into()));
        }
        
        self.running = true;
        self.transition_state(GameState::Running);
        
        // In a real implementation, you might want to spawn a separate thread for the game loop
        // or use Godot's process callback to drive updates
        
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
    pub fn shutdown(&mut self) {
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
    
    // Check if manager is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
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
            initialized: self.initialized,
        }
    }
}
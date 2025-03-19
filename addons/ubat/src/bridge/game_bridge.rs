use godot::prelude::*;
use std::sync::{Arc, Mutex};
use std::path::Path;

use crate::core::game_manager::{GameManager, GameState, GameError};
use crate::bridge::EventBridge;


#[derive(GodotClass)]
#[class(base=Node)]
pub struct GameManagerBridge {
    // Base class must be first field
    base: Base<Node>,
    
    // Internal reference to the game manager
    game_manager: Option<Arc<Mutex<GameManager>>>,

    // Reference to the event bridge
    event_bridge: Option<Gd<EventBridge>>,
    
    // Configuration properties exposed to the editor
    #[export]
    config_path: GString,
    
    #[export]
    frame_rate: i32,
    
    #[export]
    debug_mode: bool,
    
    // Current game state for property access
    #[export]
    current_state: i32,
    
    // Flag to control automatic updates
    #[export]
    auto_update: bool,
}

#[godot_api]
impl INode for GameManagerBridge {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            game_manager: None,
            event_bridge: None,
            config_path: "res://game_config.toml".into(),
            frame_rate: 60,
            debug_mode: false,
            current_state: -1, // Not initialized
            auto_update: true,
        }
    }
    
    fn ready(&mut self) {
        // Find the event bridge in the scene tree
        if let Some(tree) = self.base().get_tree() {
            if let Some(root) = tree.get_root() {
                // Using get_node_as which returns Option<Gd<T>>
                self.event_bridge = root.get_node_as::<EventBridge>("EventBridge");
                
                if self.event_bridge.is_some() && self.debug_mode {
                    godot_print!("GameManagerBridge: Found EventBridge");
                }
            }
        }
        
        // Automatically initialize if config path is set
        if !self.config_path.is_empty() && self.debug_mode {
            godot_print!("GameManagerBridge: Config path set to {}", self.config_path);
        }
    }
    
    fn process(&mut self, delta: f64) {
        // Update game state if running and auto-update is enabled
        if self.auto_update {
            self.update_game(delta);
        }
    }
}

#[godot_api]
impl GameManagerBridge {
    // Signal declarations
    #[signal]
    fn game_state_changed(old_state: i32, new_state: i32, state_name: GString);
    
    #[signal]
    fn game_error(error_message: GString);
    
    /// Initialize the game with the given configuration file path
    /// 
    /// Returns true if initialization was successful, false otherwise
    #[func]
    pub fn initialize(&mut self, config_path: GString) -> bool {
        // Store the config path
        self.config_path = config_path.clone();
        
        // Create and initialize the game manager
        match GameManager::init_from_config(config_path.to_string()) {
            Ok(mut manager) => {
                // Initialize the manager
                match manager.initialize() {
                    Ok(_) => {
                        // Store the initialized manager in an Arc<Mutex>
                        self.game_manager = Some(Arc::new(Mutex::new(manager)));
                        
                        // Update the current state
                        self.update_state_property();
                        
                        if self.debug_mode {
                            godot_print!("GameManagerBridge: Game initialized successfully");
                        }
                        
                        true
                    },
                    Err(e) => {
                        let error_msg = format!("Failed to initialize game: {:?}", e);
                        godot_error!("{}", error_msg);
                        
                        // Emit error signal
                        self.base.emit_signal("game_error".into(), &[error_msg.to_variant()]);
                        
                        false
                    }
                }
            },
            Err(e) => {
                let error_msg = format!("Failed to create game manager: {:?}", e);
                godot_error!("{}", error_msg);
                
                // Emit error signal
                self.base.emit_signal("game_error".into(), &[error_msg.to_variant()]);
                
                false
            }
        }
    }
    
    /// Initialize the game with default configuration
    /// 
    /// Returns true if initialization was successful, false otherwise
    #[func]
    pub fn initialize_default(&mut self) -> bool {
        let mut manager = GameManager::new();
        
        match manager.initialize() {
            Ok(_) => {
                // Store the initialized manager
                self.game_manager = Some(Arc::new(Mutex::new(manager)));
                
                // Update the current state
                self.update_state_property();
                
                if self.debug_mode {
                    godot_print!("GameManagerBridge: Game initialized with default config");
                }
                
                true
            },
            Err(e) => {
                let error_msg = format!("Failed to initialize game: {:?}", e);
                godot_error!("{}", error_msg);
                
                // Emit error signal
                self.base.emit_signal("game_error".into(), &[error_msg.to_variant()]);
                
                false
            }
        }
    }
    
    /// Start the game (non-blocking)
    /// 
    /// Returns true if the game was started successfully, false otherwise
    #[func]
    pub fn start_game(&mut self) -> bool {
        if let Some(game_manager) = &self.game_manager {
            if let Ok(mut manager) = game_manager.lock() {
                // Check the current state
                let current_state = manager.get_state();
                
                // Only start if not already running
                if current_state != GameState::Running {
                    // Changed from manager.start() which was blocking
                    // to just setting the state to Running
                    manager.transition_state(GameState::Running);
                    
                    // Update the state property
                    self.update_state_property();
                    
                    if self.debug_mode {
                        godot_print!("GameManagerBridge: Game started");
                    }
                    
                    return true;
                } else {
                    if self.debug_mode {
                        godot_print!("GameManagerBridge: Game already running");
                    }
                    
                    return true; // Already running is not an error
                }
            }
        }
        
        godot_error!("GameManagerBridge: Game manager not initialized");
        false
    }
    
    /// Update the game state (should be called every frame for non-blocking operation)
    /// 
    /// Returns true if the update was successful, false otherwise
    #[func]
    pub fn update_game(&mut self, delta: f64) -> bool {
        if let Some(game_manager) = &self.game_manager {
            if let Ok(mut manager) = game_manager.lock() {
                // Only update if the game is running
                if manager.get_state() == GameState::Running {
                    // Call the update method
                    if let Err(e) = manager.update() {
                        let error_msg = format!("Game update error: {:?}", e);
                        godot_error!("{}", error_msg);
                        
                        // Emit error signal
                        self.base.emit_signal("game_error".into(), &[error_msg.to_variant()]);
                        
                        return false;
                    }
                    
                    // Update was successful
                    return true;
                }
            }
        }
        
        // No updates were performed
        false
    }
    
    /// Get the current game state as an integer
    /// 
    /// Returns:
    /// - 0: Initializing
    /// - 1: MainMenu
    /// - 2: Loading
    /// - 3: Running
    /// - 4: Paused
    /// - 5: Exiting
    /// - -1: Not initialized
    #[func]
    pub fn get_game_state(&self) -> i32 {
        self.current_state
    }
    
    /// Get the current game state as a string
    /// 
    /// Returns a descriptive string for the current state
    #[func]
    pub fn get_game_state_name(&self) -> GString {
        match self.current_state {
            0 => "Initializing",
            1 => "MainMenu",
            2 => "Loading",
            3 => "Running",
            4 => "Paused",
            5 => "Exiting",
            _ => "Not Initialized",
        }.into()
    }
    
    /// Pause the game
    /// 
    /// Returns true if the game was paused successfully
    #[func]
    pub fn pause_game(&mut self) -> bool {
        if let Some(game_manager) = &self.game_manager {
            if let Ok(mut manager) = game_manager.lock() {
                manager.pause();
                
                // Update the state property
                self.update_state_property();
                
                if self.debug_mode {
                    godot_print!("GameManagerBridge: Game paused");
                }
                
                return true;
            }
        }
        
        godot_error!("GameManagerBridge: Game manager not initialized");
        false
    }
    
    /// Resume the game
    /// 
    /// Returns true if the game was resumed successfully
    #[func]
    pub fn resume_game(&mut self) -> bool {
        if let Some(game_manager) = &self.game_manager {
            if let Ok(mut manager) = game_manager.lock() {
                manager.resume();
                
                // Update the state property
                self.update_state_property();
                
                if self.debug_mode {
                    godot_print!("GameManagerBridge: Game resumed");
                }
                
                return true;
            }
        }
        
        godot_error!("GameManagerBridge: Game manager not initialized");
        false
    }
    
    /// Stop the game
    /// 
    /// Returns true if the game was stopped successfully
    #[func]
    pub fn stop_game(&mut self) -> bool {
        if let Some(game_manager) = &self.game_manager {
            if let Ok(mut manager) = game_manager.lock() {
                manager.stop();
                
                // Update the state property
                self.update_state_property();
                
                if self.debug_mode {
                    godot_print!("GameManagerBridge: Game stopped");
                }
                
                return true;
            }
        }
        
        godot_error!("GameManagerBridge: Game manager not initialized");
        false
    }
    
    /// Set the maximum frames per second
    #[func]
    pub fn set_frame_rate(&mut self, fps: i32) {
        self.frame_rate = fps;
        
        if let Some(game_manager) = &self.game_manager {
            if let Ok(mut manager) = game_manager.lock() {
                // Update the frame rate in the game manager
                manager.set_frame_rate(fps as u32);
                
                if self.debug_mode {
                    godot_print!("GameManagerBridge: Frame rate set to {}", fps);
                }
            }
        }
    }
    
    /// Update the current_state property based on the game manager state
    fn update_state_property(&mut self) {
        if let Some(game_manager) = &self.game_manager {
            if let Ok(manager) = game_manager.lock() {
                let old_state = self.current_state;
                
                // Map game state to integer
                self.current_state = match manager.get_state() {
                    GameState::Initializing => 0,
                    GameState::MainMenu => 1,
                    GameState::Loading => 2,
                    GameState::Running => 3,
                    GameState::Paused => 4,
                    GameState::Exiting => 5,
                };
                
                // Emit signal if state changed
                if old_state != self.current_state {
                    let state_name = self.get_game_state_name();
                    
                    self.base.emit_signal("game_state_changed".into(), &[
                        old_state.to_variant(),
                        self.current_state.to_variant(),
                        state_name.to_variant(),
                    ]);
                    
                    if self.debug_mode {
                        godot_print!(
                            "GameManagerBridge: Game state changed from {} to {}", 
                            if old_state >= 0 { self.state_to_string(old_state) } else { "Not Initialized" },
                            self.state_to_string(self.current_state),
                        );
                    }
                }
            }
        }
    }
    
    /// Helper function to convert state integer to string
    fn state_to_string(&self, state: i32) -> &'static str {
        match state {
            0 => "Initializing",
            1 => "MainMenu",
            2 => "Loading",
            3 => "Running",
            4 => "Paused",
            5 => "Exiting",
            _ => "Unknown",
        }
    }
}
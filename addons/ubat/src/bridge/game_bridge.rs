use godot::prelude::*;
use std::sync::{Arc, Mutex};

use crate::core::game_manager::{GameManager, GameState, GameError};
use crate::bridge::EventBridge;


#[derive(GodotClass)]
#[class(base=Node)]
pub struct GameManagerBridge {
    // Base class must be first field
    base: Base<Node>,
    
    // Internal reference to the game manager
    game_manager: Option<Arc<Mutex<GameManager>>>,
 
    // Configuration properties exposed to the editor
    #[export]
    config_path: GString,
    
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
            config_path: "res://game_config.toml".into(),
            frame_rate: 60,
            debug_mode: false,
            current_state: -1, // Not initialized
            auto_update: true,
        }
    }
    
    fn ready(&mut self) {
        
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
        
        // Convert Godot resource path to filesystem path
        let global_path = godot::classes::ProjectSettings::singleton().globalize_path(&config_path);
        let path_str = global_path.to_string();
        
        if self.debug_mode {
            println!("GameManagerBridge: Converting path '{}' to '{}'", config_path, path_str);
        }
        
        // Check if the file exists
        if !std::path::Path::new(&path_str).exists() {
            let error_msg = format!("Config file not found at: {}", path_str);
            println!("GameManagerBridge: {}", error_msg);
            self.base_mut().emit_signal("game_error", &[error_msg.to_variant()]);
            return false;
        }
        
        // IMPORTANT: Pass the globalized path string, not the original Godot path
        let (initialization_successful, error_msg) = match GameManager::init_from_config(path_str) {
            Ok(mut manager) => {
                // Initialize the manager
                match manager.initialize() {
                    Ok(_) => {
                        // Store the initialized manager in an Arc<Mutex>
                        self.game_manager = Some(Arc::new(Mutex::new(manager)));
                        (true, None)
                    },
                    Err(e) => {
                        let error_msg = format!("Failed to initialize game: {:?}", e);
                        println!("{}", error_msg);
                        (false, Some(error_msg))
                    }
                }
            },
            Err(e) => {
                let error_msg = format!("Failed to create game manager: {:?}", e);
                println!("{}", error_msg);
                (false, Some(error_msg))
            }
        };
        
        // Now handle the result and emit signals
        if let Some(msg) = error_msg {
            self.base_mut().emit_signal("game_error", &[msg.to_variant()]);
        }
        
        if initialization_successful {
            // Update the current state
            self.update_state_property();
            
            if self.debug_mode {
                println!("GameManagerBridge: Game initialized successfully");
            }
        }
        
        initialization_successful
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
                self.base_mut().emit_signal("game_error", &[error_msg.to_variant()]);
                
                false
            }
        }
    }

    /// Initialize with configuration options
    /// 
    /// This method takes a dictionary of configuration options,
    /// applies them to the ConfigBridge, and then initializes the game.
    /// 
    /// Parameters:
    /// - config_path: Path to the configuration file
    /// - options: Dictionary of configuration options to apply
    ///
    /// Returns true if initialization was successful, false otherwise
    #[func]
    pub fn initialize_with_options(&mut self, config_path: GString, options: Dictionary) -> bool {
        // Store the config path
        self.config_path = config_path.clone();
        
        // Try to load the configuration
        let global_path = godot::classes::ProjectSettings::singleton().globalize_path(&config_path);
        let path_str = global_path.to_string();
        
        if self.debug_mode {
            godot_print!("GameManagerBridge: Loading from filesystem path: {}", path_str);
        }
        
        // Check if the configuration file exists
        let path_exists = std::path::Path::new(&path_str).exists();
        
        // Define a variable to store the error message, if any
        let mut error_msg_opt: Option<String> = None;
        
        // First phase: Create the game manager
        let initialization_result = if path_exists {
            // If the path exists, try to load it
            match GameManager::init_from_config(path_str) {
                Ok(mut manager) => {
                    // Store the initialized manager in an Arc<Mutex>
                    self.game_manager = Some(Arc::new(Mutex::new(manager)));
                    true
                },
                Err(e) => {
                    error_msg_opt = Some(format!("Failed to load game config: {:?}", e));
                    false
                }
            }
        } else {
            // If the path doesn't exist, create a default configuration
            let manager = GameManager::new();
            self.game_manager = Some(Arc::new(Mutex::new(manager)));
            true
        };
        
        // If initialization failed, display error and return early
        if !initialization_result {
            // Safe to unwrap because we've checked initialization_result
            let error_msg = error_msg_opt.unwrap();
            godot_error!("{}", error_msg);
            self.base_mut().emit_signal("game_error", &[error_msg.to_variant()]);
            return false;
        }
        
        // Second phase: Initialize the game
        let mut success = false;
        let mut phase2_error: Option<String> = None;
        
        // This block makes sure any game_manager lock is dropped before we try
        // to borrow self mutably later
        {
            if let Some(ref game_manager) = self.game_manager {
                let lock_result = game_manager.lock();
                
                match lock_result {
                    Ok(mut manager) => {
                        match manager.initialize() {
                            Ok(_) => {
                                success = true;
                            },
                            Err(e) => {
                                phase2_error = Some(format!("Failed to initialize game: {:?}", e));
                            }
                        }
                    },
                    Err(_) => {
                        phase2_error = Some("Failed to lock game manager".to_string());
                    }
                }
            }
        } // End of scope - all locks on game_manager are released here
        
        // Now we can safely borrow self as mutable
        if success {
            // Update the state property
            self.update_state_property();
            
            if self.debug_mode {
                godot_print!("GameManagerBridge: Game initialized successfully");
            }
            
            true
        } else {
            // Safe to unwrap because success is false, which means phase2_error is Some
            let error_msg = phase2_error.unwrap();
            godot_error!("{}", error_msg);
            self.base_mut().emit_signal("game_error", &[error_msg.to_variant()]);
            
            false
        }
    }
    
    /// Start the game (non-blocking)
    /// 
    /// Returns true if the game was started successfully, false otherwise
    #[func]
    pub fn start_game(&mut self) -> bool {
        // Step 1: Execute operation with immutable borrow
        let start_result = if let Some(game_manager) = &self.game_manager {
            if let Ok(mut manager) = game_manager.lock() {
                // Check the current state
                let current_state = manager.get_state();
                
                // Only start if not already running
                if current_state != GameState::Running {
                    // Set state to Running
                    manager.transition_state(GameState::Running);
                    true
                } else {
                    // Already running is not an error
                    true
                }
            } else {
                false
            }
        } else {
            false
        };
        
        // Step 2: Now handle operations requiring mutable borrow
        if start_result {
            // Update the state property
            self.update_state_property();
            
            if self.debug_mode {
                godot_print!("GameManagerBridge: Game started");
            }
            
            true
        } else {
            godot_error!("GameManagerBridge: Game manager not initialized");
            false
        }
    }
    
    /// Update the game state (should be called every frame for non-blocking operation)
    /// 
    /// Returns true if the update was successful, false otherwise
    #[func]
    pub fn update_game(&mut self, delta: f64) -> bool {
        // Step 1: Extract operation result and error message if any
        let (update_result, error_msg) = {
            if let Some(game_manager) = &self.game_manager {
                // Store the lock result in a local variable to control its lifetime
                let lock_result = game_manager.lock();
                
                if let Ok(mut manager) = lock_result {
                    // Only update if the game is running
                    if manager.get_state() == GameState::Running {
                        // Call the update method
                        match manager.update() {
                            Ok(_) => (true, None),
                            Err(e) => {
                                let msg = format!("Game update error: {:?}", e);
                                (false, Some(msg))
                            }
                        }
                    } else {
                        (false, None) // Not running, no update performed
                    }
                } else {
                    (false, None) // Failed to lock
                }
            } else {
                (false, None) // No game manager
            }
        }; // End of scope for any temporary borrows
        
        // Step 2: Handle results and emit signals
        if let Some(msg) = error_msg {
            godot_error!("{}", msg);
            self.base_mut().emit_signal(&StringName::from("game_error"), &[msg.to_variant()]);
        }
        
        update_result
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
        // Step 1: Execute operation with immutable borrow
        let pause_successful = if let Some(game_manager) = &self.game_manager {
            if let Ok(mut manager) = game_manager.lock() {
                manager.pause();
                true
            } else {
                false
            }
        } else {
            false
        };
        
        // Step 2: Now handle operations requiring mutable borrow
        if pause_successful {
            // Update the state property
            self.update_state_property();
            
            if self.debug_mode {
                godot_print!("GameManagerBridge: Game paused");
            }
            
            true
        } else {
            godot_error!("GameManagerBridge: Game manager not initialized");
            false
        }
    }
    
    /// Resume the game
    /// 
    /// Returns true if the game was resumed successfully
    #[func]
    pub fn resume_game(&mut self) -> bool {
        // Step 1: Execute operation with immutable borrow
        let resume_successful = if let Some(game_manager) = &self.game_manager {
            if let Ok(mut manager) = game_manager.lock() {
                manager.resume();
                true
            } else {
                false
            }
        } else {
            false
        };
        
        // Step 2: Now handle operations requiring mutable borrow
        if resume_successful {
            // Update the state property
            self.update_state_property();
            
            if self.debug_mode {
                godot_print!("GameManagerBridge: Game resumed");
            }
            
            true
        } else {
            godot_error!("GameManagerBridge: Game manager not initialized");
            false
        }
    }

    #[func]
    pub fn stop_game(&mut self) -> bool {
        // Step 1: Execute operation with immutable borrow
        let stop_successful = if let Some(game_manager) = &self.game_manager {
            if let Ok(mut manager) = game_manager.lock() {
                manager.stop();
                true
            } else {
                false
            }
        } else {
            false
        };
        
        // Step 2: Now handle operations requiring mutable borrow
        if stop_successful {
            // Update the state property
            self.update_state_property();
            
            if self.debug_mode {
                godot_print!("GameManagerBridge: Game stopped");
            }
            
            true
        } else {
            godot_error!("GameManagerBridge: Game manager not initialized");
            false
        }
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
        // First gather data with immutable borrow
        let (old_state, new_state, state_changed) = if let Some(game_manager) = &self.game_manager {
            if let Ok(manager) = game_manager.lock() {
                let old_state = self.current_state;
                
                // Map game state to integer
                let new_state = match manager.get_state() {
                    GameState::Initializing => 0,
                    GameState::MainMenu => 1,
                    GameState::Loading => 2,
                    GameState::Running => 3,
                    GameState::Paused => 4,
                    GameState::Exiting => 5,
                };
                
                (old_state, new_state, old_state != new_state)
            } else {
                return; // Failed to lock
            }
        } else {
            return; // No game manager
        };
        
        // Update the state
        self.current_state = new_state;
        
        // Emit signal if state changed
        if state_changed {
            let state_name = self.get_game_state_name();
            
            self.base_mut().emit_signal(&StringName::from("game_state_changed"), &[
                old_state.to_variant(),
                new_state.to_variant(),
                state_name.to_variant(),
            ]);
            
            if self.debug_mode {
                godot_print!(
                    "GameManagerBridge: Game state changed from {} to {}", 
                    if old_state >= 0 { self.state_to_string(old_state) } else { "Not Initialized" },
                    self.state_to_string(new_state),
                );
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
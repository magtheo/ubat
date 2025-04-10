use godot::prelude::*;
use crate::core::game_manager::{self, GameManager, GameState, GameError};
use std::sync::{Arc, Mutex};

/// Bridge between Godot and the Rust game manager
/// 
/// This class provides an interface for Godot to interact with the Rust game manager.
/// It primarily forwards calls to the game manager and emits signals for Godot to handle.
#[derive(GodotClass)]
#[class(base=Node)]
pub struct GameManagerBridge {
    // Base class must be first field
    base: Base<Node>,

    // Game manager reference
    game_manager: Option<Arc<Mutex<GameManager>>>,
    
    // Configuration properties exposed to the editor
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
            debug_mode: false,
            current_state: -1, // Not initialized
            auto_update: true,
        }
    }
    
    fn ready(&mut self) {
        if self.debug_mode {
            godot_print!("GameManagerBridge: Ready");
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
    fn game_world_initialized();
    
    #[signal]
    fn game_state_changed(old_state: i32, new_state: i32, state_name: GString);
    
    #[signal]
    fn game_error(error_message: GString);

    /// Set the game manager reference
    pub fn set_config_manager(&mut self, game_manager: Arc<Mutex<GameManager>>) {
        // Store a clone of the game manager
        let manager_clone = game_manager.clone();
        self.game_manager = Some(game_manager);
        
        // Try to update the state property using the cloned reference
        if let Ok(locked_manager) = manager_clone.lock() {
            self.update_state_property(&locked_manager);
        }
        
        if self.debug_mode {
            godot_print!("GameManagerBridge: Game manager reference set externally");
        }
    }
    
    /// Start the game after initialization
    /// 
    /// Returns true if the game was started successfully, false otherwise
    #[func]
    pub fn start_game(&mut self) -> bool {
        // Use our own reference to the game manager if available
        if let Some(game_manager_arc) = &self.game_manager {
            // Create a local variable to store the game state
            let mut success = false;
            let mut current_state = GameState::Initializing;
            let mut error_message = None;
            
            // Lock the game manager in a separate scope to avoid borrowing issues
            {
                // Lock the game manager
                match game_manager_arc.lock() {
                    Ok(mut game_manager) => {
                        // Attempt to start the game
                        match game_manager.start_game() {
                            Ok(_) => {
                                // Store the current state for later use
                                current_state = game_manager.get_state();
                                success = true;
                            },
                            Err(e) => {
                                error_message = Some(format!("Failed to start game: {:?}", e));
                            }
                        }
                    },
                    Err(_) => {
                        error_message = Some("Failed to lock game manager".to_string());
                    }
                }
            }
            
            // Now update the state property and emit signals
            if success {
                self.update_state_from_enum(current_state);
                
                // Emit signal
                self.base_mut().emit_signal(
                    &StringName::from("game_world_initialized"), 
                    &[]
                );
                
                if self.debug_mode {
                    godot_print!("GameManagerBridge: Game started");
                }
            } else if let Some(error_msg) = error_message {
                godot_error!("{}", error_msg);
                self.base_mut().emit_signal("game_error", &[error_msg.to_variant()]);
            }
            
            success
        } else {
            let error_msg = "GameManagerBridge: Game manager reference not set";
            godot_error!("{}", error_msg);
            self.base_mut().emit_signal("game_error", &[error_msg.to_variant()]);
            false
        }
    }
    
    /// Update the game state (called from process or manually)
    /// 
    /// Returns true if the update was successful, false otherwise
    #[func]
    pub fn update_game(&mut self, delta: f64) -> bool {
        // Use our own reference to the game manager if available
        if let Some(game_manager_arc) = &self.game_manager {
            // Variables to store the results outside the lock scope
            let mut success = false;
            let mut current_state = GameState::Initializing;
            let mut error_msg = None;
            
            // Use a separate scope for the lock to avoid borrowing issues
            {
                // Lock the game manager
                match game_manager_arc.lock() {
                    Ok(mut game_manager) => {
                        // Only update if the game is running
                        if game_manager.get_state() == GameState::Running {
                            // Call the update method
                            match game_manager.update() {
                                Ok(_) => {
                                    // Store state for later use
                                    current_state = game_manager.get_state();
                                    success = true;
                                },
                                Err(e) => {
                                    error_msg = Some(format!("Game update error: {:?}", e));
                                }
                            }
                        }
                    },
                    Err(_) => {
                        // Failed to lock the game manager
                    }
                }
            }
            
            // Now we can safely update our state and emit signals
            if success {
                self.update_state_from_enum(current_state);
            } else if let Some(msg) = error_msg {
                godot_error!("{}", msg);
                self.base_mut().emit_signal("game_error", &[msg.to_variant()]);
            }
            
            success
        } else {
            // No game manager reference
            false
        }
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
        // Use our own reference to the game manager if available
        if let Some(game_manager_arc) = &self.game_manager {
            let mut success = false;
            let mut current_state = GameState::Initializing;
            
            // Use a separate scope for the lock to avoid borrowing issues
            {
                // Lock the game manager
                match game_manager_arc.lock() {
                    Ok(mut game_manager) => {
                        // Pause the game
                        game_manager.pause();
                        current_state = game_manager.get_state();
                        success = true;
                    },
                    Err(_) => {
                        godot_error!("GameManagerBridge: Failed to lock game manager");
                    }
                }
            }
            
            // Now safely update state outside the lock scope
            if success {
                self.update_state_from_enum(current_state);
                
                if self.debug_mode {
                    godot_print!("GameManagerBridge: Game paused");
                }
            }
            
            success
        } else {
            godot_error!("GameManagerBridge: Game manager reference not set");
            false
        }
    }
    
    /// Resume the game
    /// 
    /// Returns true if the game was resumed successfully
    #[func]
    pub fn resume_game(&mut self) -> bool {
        // Use our own reference to the game manager if available
        if let Some(game_manager_arc) = &self.game_manager {
            let mut success = false;
            let mut current_state = GameState::Initializing;
            
            // Use a separate scope for the lock to avoid borrowing issues
            {
                // Lock the game manager
                match game_manager_arc.lock() {
                    Ok(mut game_manager) => {
                        // Resume the game
                        game_manager.resume();
                        current_state = game_manager.get_state();
                        success = true;
                    },
                    Err(_) => {
                        godot_error!("GameManagerBridge: Failed to lock game manager");
                    }
                }
            }
            
            // Now safely update state outside the lock scope
            if success {
                self.update_state_from_enum(current_state);
                
                if self.debug_mode {
                    godot_print!("GameManagerBridge: Game resumed");
                }
            }
            
            success
        } else {
            godot_error!("GameManagerBridge: Game manager reference not set");
            false
        }
    }

    /// Stop the game
    /// 
    /// Returns true if the game was stopped successfully
    #[func]
    pub fn stop_game(&mut self) -> bool {
        // Use our own reference to the game manager if available
        if let Some(game_manager_arc) = &self.game_manager {
            let mut success = false;
            let mut current_state = GameState::Initializing;
            
            // Use a separate scope for the lock to avoid borrowing issues
            {
                // Lock the game manager
                match game_manager_arc.lock() {
                    Ok(mut game_manager) => {
                        // Stop the game
                        game_manager.stop();
                        current_state = game_manager.get_state();
                        success = true;
                    },
                    Err(_) => {
                        godot_error!("GameManagerBridge: Failed to lock game manager");
                    }
                }
            }
            
            // Now safely update state outside the lock scope
            if success {
                self.update_state_from_enum(current_state);
                
                if self.debug_mode {
                    godot_print!("GameManagerBridge: Game stopped");
                }
            }
            
            success
        } else {
            godot_error!("GameManagerBridge: Game manager reference not set");
            false
        }
    }
    
    /// Set the maximum frames per second
    #[func]
    pub fn set_frame_rate(&mut self, fps: i32) {
        // Use our own reference to the game manager if available
        if let Some(game_manager_arc) = &self.game_manager {
            // Lock the game manager
            if let Ok(mut game_manager) = game_manager_arc.lock() {
                // Update the frame rate in the game manager
                game_manager.set_frame_rate(fps as u32);
                
                if self.debug_mode {
                    godot_print!("GameManagerBridge: Frame rate set to {}", fps);
                }
            }
        } else {
            godot_error!("GameManagerBridge: Game manager reference not set");
        }
    }
    
    /// Check if the game is initialized and ready
    #[func]
    pub fn is_initialized(&self) -> bool {
        // Use our own reference to the game manager if available
        if let Some(game_manager_arc) = &self.game_manager {
            // Lock the game manager
            match game_manager_arc.lock() {
                Ok(game_manager) => game_manager.is_initialized(),
                Err(_) => false,
            }
        } else {
            false
        }
    }
    
    /// Update the state property based on the GameState enum
    fn update_state_from_enum(&mut self, game_state: GameState) {
        let old_state = self.current_state;
        
        // Map game state to integer
        let new_state = match game_state {
            GameState::Initializing => 0,
            GameState::MainMenu => 1,
            GameState::Loading => 2,
            GameState::Running => 3,
            GameState::Paused => 4,
            GameState::Exiting => 5,
        };
        
        // Update the state
        self.current_state = new_state;
        
        // Emit signal if state changed
        if old_state != new_state {
            let state_name = self.get_game_state_name();
            
            self.base_mut().emit_signal("game_state_changed", &[
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
    
    /// Update the current_state property based on the game manager state
    fn update_state_property(&mut self, game_manager: &GameManager) {
        self.update_state_from_enum(game_manager.get_state());
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
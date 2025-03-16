use godot::prelude::*;

use crate::core::GameManager;


#[derive(GodotClass)]
#[class(base=Node)]
pub struct GameManagerBridge {
    // Base class must be first field
    base: Base<Node>,
    
    // Internal reference to your game manager
    game_manager: Option<GameManager>,
}

#[godot_api]
impl GameManagerBridge {
    #[func]
    fn initialize(&mut self, config_path: GString) -> bool {
        match GameManager::init_from_config(config_path.to_string()) {
            Ok(manager) => {
                self.game_manager = Some(manager);
                if let Some(manager) = &mut self.game_manager {
                    match manager.initialize() {
                        Ok(_) => true,
                        Err(e) => {
                            godot_print!("Failed to initialize game: {:?}", e);
                            false
                        }
                    }
                } else {
                    false
                }
            },
            Err(e) => {
                godot_print!("Failed to create game manager: {:?}", e);
                false
            }
        }
    }
    
    #[func]
    fn start_game(&mut self) -> bool {
        if let Some(manager) = &mut self.game_manager {
            match manager.start() {
                Ok(_) => true,
                Err(e) => {
                    godot_print!("Game error: {:?}", e);
                    false
                }
            }
        } else {
            godot_print!("Game manager not initialized");
            false
        }
    }
    
    #[func]
    fn get_game_state(&self) -> i32 {
        if let Some(manager) = &self.game_manager {
            match manager.get_state() {
                GameState::Initializing => 0,
                GameState::MainMenu => 1,
                GameState::Loading => 2,
                GameState::Running => 3,
                GameState::Paused => 4,
                GameState::Exiting => 5,
            }
        } else {
            -1 // Not initialized
        }
    }
    
    #[func]
    fn pause_game(&mut self) {
        if let Some(manager) = &mut self.game_manager {
            manager.pause();
        }
    }
    
    #[func]
    fn resume_game(&mut self) {
        if let Some(manager) = &mut self.game_manager {
            manager.resume();
        }
    }
    
    #[func]
    fn stop_game(&mut self) {
        if let Some(manager) = &mut self.game_manager {
            manager.stop();
        }
    }
}
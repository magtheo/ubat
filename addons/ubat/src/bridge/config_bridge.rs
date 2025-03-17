use godot::prelude::*;

use std::sync::{Arc, Mutex};

use crate::core::config_manager::{ConfigurationManager};
use crate::bridge::EventBridge;

#[derive(GodotClass)]
#[class(base=RefCounted)]
pub struct ConfigBridge {
    base: Base<RefCounted>,
    config_manager: Option<Arc<Mutex<ConfigurationManager>>>,
}

#[godot_api]
impl IRefCounted for ConfigBridge {
    fn init(base: Base<RefCounted>) -> Self {
        Self {
            base,
            config_manager: None,
        }
    }
}

#[godot_api]
impl ConfigBridge {
    #[func]
    fn load_config(&mut self, path: GString) -> bool {
        match ConfigurationManager::load_from_file(path.to_string()) {
            Ok(manager) => {
                self.config_manager = Some(Arc::new(Mutex::new(manager)));
                true
            },
            Err(e) => {
                godot_print!("Failed to load config: {}", e);
                false
            }
        }
    }
    
    #[func]
    fn get_world_seed(&self) -> i64 {
        if let Some(config) = &self.config_manager {
            if let Ok(config) = config.lock() {
                return config.get_config().world_seed as i64;
            }
        }
        0
    }

    // TODO: add more getter funcitons when needed
    
    
    // Add getters/setters for other important configuration values
}
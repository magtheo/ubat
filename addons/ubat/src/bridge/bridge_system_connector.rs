// system_bundle_initializer.rs
use std::sync::{Arc, Mutex};
use godot::prelude::*;

use crate::networking::network_manager::NetworkMode;
use crate::core::initialization::system_initializer::{SystemInitializer, SystemBundle, InitializationError, InitializationOptions};
use crate::bridge::{ConfigBridge, GameManagerBridge, EventBridge, NetworkManagerBridge};

/// Provides bridge initialization services and manages system lifecycle
pub struct SystemBundleInitializer {
    // The main system bundle with all core systems
    bundle: Option<SystemBundle>,
    
    // Initialization options
    config_path: Option<String>,
    network_mode: i32,
    debug_mode: bool,
}

impl SystemBundleInitializer {
    /// Create a new SystemBundleInitializer
    pub fn new() -> Self {
        Self {
            bundle: None,
            config_path: None,
            network_mode: 0, // Standalone by default
            debug_mode: false,
        }
    }
    
    /// Initialize all systems with the given configuration
    /// 
    /// This is the main entry point for Godot bridges to initialize the Rust backend.
    pub fn initialize_all_systems(&mut self, config_path: GString) -> bool {
        // Store configuration path for future use
        self.config_path = Some(config_path.to_string());
        
        // Build initialization options
        let options = InitializationOptions {
            config_path: self.config_path.clone(),
            default_seed: None, // Will be loaded from config or generated
            network_mode: self.get_network_mode(),
            debug_mode: self.debug_mode,
            terrain_enabled: true,
        };
        
        // Create and initialize the system
        let mut initializer = SystemInitializer::with_options(options);
        
        match initializer.initialize() {
            Ok(bundle) => {
                if self.debug_mode {
                    godot_print!("SystemBundleInitializer: Successfully initialized all systems");
                }
                self.bundle = Some(bundle);
                true
            },
            Err(e) => {
                godot_error!("SystemBundleInitializer: Failed to initialize systems: {:?}", e);
                false
            }
        }
    }
    
    /// Initialize the ConfigBridge with the system bundle
    pub fn initialize_config_bridge(&self, mut bridge: Gd<ConfigBridge>) -> bool {
        if let Some(bundle) = &self.bundle {
            let mut bridge_obj = bridge.bind_mut();
            bridge_obj.set_config_manager(bundle.config_manager.clone());
            
            if self.debug_mode {
                godot_print!("SystemBundleInitializer: Initialized ConfigBridge");
            }
            true
        } else {
            godot_error!("SystemBundleInitializer: Cannot initialize ConfigBridge - system bundle not available");
            false
        }
    }
    
    /// Initialize the GameManagerBridge with the system bundle
    pub fn initialize_game_bridge(&self, mut bridge: Gd<GameManagerBridge>) -> bool {
        if let Some(bundle) = &self.bundle {
            let mut bridge_obj = bridge.bind_mut();
            
            // Only proceed if game manager is available
            if let Some(game_manager) = &bundle.game_manager {
                bridge_obj.set_game_manager(game_manager.clone());
                
                if self.debug_mode {
                    godot_print!("SystemBundleInitializer: Initialized GameManagerBridge");
                }
                true
            } else {
                godot_error!("SystemBundleInitializer: Cannot initialize GameManagerBridge - game manager not available");
                false
            }
        } else {
            godot_error!("SystemBundleInitializer: Cannot initialize GameManagerBridge - system bundle not available");
            false
        }
    }
    
    /// Initialize the EventBridge with the system bundle
    pub fn initialize_event_bridge(&self, mut bridge: Gd<EventBridge>) -> bool {
        if let Some(bundle) = &self.bundle {
            let mut bridge_obj = bridge.bind_mut();
            bridge_obj.set_event_bus(bundle.event_bus.clone());
            
            if self.debug_mode {
                godot_print!("SystemBundleInitializer: Initialized EventBridge");
            }
            true
        } else {
            godot_error!("SystemBundleInitializer: Cannot initialize EventBridge - system bundle not available");
            false
        }
    }
    
    /// Initialize the NetworkManagerBridge with the system bundle
    pub fn initialize_network_bridge(&self, mut bridge: Gd<NetworkManagerBridge>) -> bool {
        if let Some(bundle) = &self.bundle {
            // Only proceed if network handler is available
            if let Some(network_handler) = &bundle.network_handler {
                let mut bridge_obj = bridge.bind_mut();
                bridge_obj.set_network_handler(network_handler.clone());
                
                if self.debug_mode {
                    godot_print!("SystemBundleInitializer: Initialized NetworkManagerBridge");
                }
                true
            } else {
                // This is not necessarily an error - might be in standalone mode
                if self.debug_mode {
                    godot_print!("SystemBundleInitializer: NetworkManagerBridge not initialized - network handler not available");
                }
                false
            }
        } else {
            godot_error!("SystemBundleInitializer: Cannot initialize NetworkManagerBridge - system bundle not available");
            false
        }
    }
    
    /// Initialize all bridges at once
    pub fn initialize_all_bridges(
        &self,
        config_bridge: Gd<ConfigBridge>,
        game_bridge: Gd<GameManagerBridge>,
        event_bridge: Gd<EventBridge>,
        network_bridge: Option<Gd<NetworkManagerBridge>>,
    ) -> bool {
        let config_result = self.initialize_config_bridge(config_bridge);
        let game_result = self.initialize_game_bridge(game_bridge);
        let event_result = self.initialize_event_bridge(event_bridge);
        
        // Network is optional
        let network_result = if let Some(bridge) = network_bridge {
            self.initialize_network_bridge(bridge)
        } else {
            true // Not an error if network bridge is not provided
        };
        
        // Everything must succeed
        config_result && game_result && event_result
    }
    
    /// Set the network mode (0=Standalone, 1=Host, 2=Client)
    pub fn set_network_mode(&mut self, mode: i32) {
        self.network_mode = mode;
    }
    
    /// Set debug mode
    pub fn set_debug_mode(&mut self, debug: bool) {
        self.debug_mode = debug;
    }
    
    /// Get NetworkMode from integer representation
    fn get_network_mode(&self) -> NetworkMode {
        match self.network_mode {
            1 => NetworkMode::Host,
            2 => NetworkMode::Client,
            _ => NetworkMode::Standalone,
        }
    }
    
    /// Check if initialization is complete
    pub fn is_initialized(&self) -> bool {
        self.bundle.is_some()
    }
    
    /// Get initialization state as a string
    pub fn get_initialization_state(&self) -> GString {
        if let Some(bundle) = &self.bundle {
            match bundle.state {
                crate::core::system_initializer::InitializationState::Uninitialized => 
                    "Uninitialized".into(),
                crate::core::system_initializer::InitializationState::CoreServicesInitialized => 
                    "Core Services Initialized".into(),
                crate::core::system_initializer::InitializationState::GameSystemsInitialized => 
                    "Game Systems Initialized".into(),
                crate::core::system_initializer::InitializationState::TerrainSystemsInitialized => 
                    "Terrain Systems Initialized".into(),
                crate::core::system_initializer::InitializationState::NetworkInitialized => 
                    "Network Initialized".into(),
                crate::core::system_initializer::InitializationState::Complete => 
                    "Initialization Complete".into(),
                crate::core::system_initializer::InitializationState::Error(e) => 
                    format!("Initialization Error: {:?}", e).into(),
            }
        } else {
            "Not Initialized".into()
        }
    }
}

#[derive(GodotClass)]
#[class(base=Node)]
pub struct SystemInitializerBridge {
    #[base]
    base: Base<Node>,
    
    initializer: SystemBundleInitializer,
    
    #[export]
    debug_mode: bool,
}

#[godot_api]
impl INode for SystemInitializerBridge {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            initializer: SystemBundleInitializer::new(),
            debug_mode: false,
        }
    }
    
    fn ready(&mut self) {
        self.initializer.set_debug_mode(self.debug_mode);
        
        if self.debug_mode {
            godot_print!("SystemInitializerBridge: Ready");
        }
    }
}

#[godot_api]
impl SystemInitializerBridge {
    #[signal]
    fn initialization_complete(success: bool);
    
    #[signal]
    fn initialization_status(state: GString);
    
    /// Initialize all systems
    #[func]
    pub fn initialize_systems(&mut self, config_path: GString) -> bool {
        if self.debug_mode {
            godot_print!("SystemInitializerBridge: Initializing systems with config: {}", config_path);
        }
        
        let result = self.initializer.initialize_all_systems(config_path);
        
        // Emit signals
        self.base_mut().emit_signal("initialization_complete", &[result.to_variant()]);
        self.base_mut().emit_signal("initialization_status", &[self.initializer.get_initialization_state().to_variant()]);
        
        result
    }
    
    /// Set the network mode for initialization
    #[func]
    pub fn set_network_mode(&mut self, mode: i32) {
        self.initializer.set_network_mode(mode);
        
        if self.debug_mode {
            godot_print!("SystemInitializerBridge: Network mode set to {}", mode);
        }
    }
    
    /// Initialize a ConfigBridge
    #[func]
    pub fn initialize_config_bridge(&self, bridge: Gd<ConfigBridge>) -> bool {
        self.initializer.initialize_config_bridge(bridge)
    }
    
    /// Initialize a GameManagerBridge
    #[func]
    pub fn initialize_game_bridge(&self, bridge: Gd<GameManagerBridge>) -> bool {
        self.initializer.initialize_game_bridge(bridge)
    }
    
    /// Initialize an EventBridge
    #[func]
    pub fn initialize_event_bridge(&self, bridge: Gd<EventBridge>) -> bool {
        self.initializer.initialize_event_bridge(bridge)
    }
    
    /// Initialize a NetworkManagerBridge
    #[func]
    pub fn initialize_network_bridge(&self, bridge: Gd<NetworkManagerBridge>) -> bool {
        self.initializer.initialize_network_bridge(bridge)
    }
    
    /// Initialize all bridges at once
    #[func]
    pub fn initialize_all_bridges(
        &self,
        config_bridge: Gd<ConfigBridge>,
        game_bridge: Gd<GameManagerBridge>,
        event_bridge: Gd<EventBridge>,
        network_bridge: Variant, // Optional parameter
    ) -> bool {
        let network_bridge_opt = if network_bridge.get_type() == VariantType::NIL {
            None
        } else {
            match network_bridge.try_to::<Gd<NetworkManagerBridge>>() {
                Ok(bridge) => Some(bridge),
                Err(_) => None,
            }
        };
        
        self.initializer.initialize_all_bridges(
            config_bridge,
            game_bridge,
            event_bridge,
            network_bridge_opt
        )
    }
    
    /// Check if initialization is complete
    #[func]
    pub fn is_initialized(&self) -> bool {
        self.initializer.is_initialized()
    }
    
    /// Get initialization state as a string
    #[func]
    pub fn get_initialization_state(&self) -> GString {
        self.initializer.get_initialization_state()
    }
}
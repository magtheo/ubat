// network_bridge.rs
use godot::prelude::*;
use std::sync::{Arc, Mutex};

use crate::networking::network_manager::{NetworkHandler, NetworkConfig, NetworkMode, NetworkEvent, PeerId};

#[derive(GodotClass)]
#[class(base=Node)]
pub struct NetworkManagerBridge {
    base: Base<Node>,
    
    // Network handler
    network_handler: Option<Arc<Mutex<NetworkHandler>>>,
    
    // Network status properties
    #[export]
    connected: bool,
    
    #[export]
    peer_count: i32,
    
    #[export]
    debug_mode: bool,
}

#[godot_api]
impl INode for NetworkManagerBridge {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            network_handler: None,
            connected: false,
            peer_count: 0,
            debug_mode: false,
        }
    }
    
    fn process(&mut self, _delta: f64) {
        // Poll for network events
        self.process_network_events();
    }
}

#[godot_api]
impl NetworkManagerBridge {
    // Signal declarations
    #[signal]
    fn peer_connected(peer_id: GString);
    
    #[signal]
    fn peer_disconnected(peer_id: GString);
    
    #[signal]
    fn connection_failed(error_message: GString);
    
    // Initialize network handler
    #[func]
    pub fn initialize_network(&mut self, mode: i32, port: i32, server_address: GString) -> bool {
        // Convert mode to NetworkMode
        let network_mode = match mode {
            0 => NetworkMode::Standalone,
            1 => NetworkMode::Host,
            2 => NetworkMode::Client,
            _ => {
                godot_error!("Invalid network mode: {}", mode);
                return false;
            }
        };
        
        // Create network configuration
        let network_config = NetworkConfig {
            mode: network_mode.clone(),
            port: port as u16,
            max_connections: 64,  // Default, could be configurable
            server_address: if mode == 2 { Some(server_address.to_string()) } else { None },
        };
        
        // Create network handler
        let result = if network_mode == NetworkMode::Standalone {
            // No network needed for standalone
            true
        } else {
            match NetworkHandler::new(network_config) {
                Ok(handler) => {
                    self.network_handler = Some(Arc::new(Mutex::new(handler)));
                    self.connected = true;
                    
                    if self.debug_mode {
                        godot_print!("NetworkManagerBridge: Initialized in {:?} mode", network_mode);
                    }
                    
                    true
                },
                Err(e) => {
                    let error_msg = format!("Failed to initialize network: {:?}", e);
                    godot_error!("{}", error_msg);
                    
                    // Emit signal
                    self.base_mut().emit_signal(
                        &StringName::from("connection_failed"), 
                        &[error_msg.to_variant()]
                    );
                    
                    false
                }
            }
        };
        
        result
    }
    
    // Process network events
    fn process_network_events(&mut self) {
        // Step 1: Collect all events with immutable borrow
        let mut events_to_process = Vec::new();
        
        if let Some(network_handler) = &self.network_handler {
            if let Ok(handler) = network_handler.lock() {
                // Collect all pending events
                while let Some(event) = handler.poll_events() {
                    events_to_process.push(event);
                }
            }
        }
        
        // Step 2: Process collected events with mutable borrow
        for event in events_to_process {
            self.handle_single_event(event);
        }
    }
    
    // Helper method to handle a single event
    fn handle_single_event(&mut self, event: NetworkEvent) {
        match event {
            NetworkEvent::Connected(peer_id) => {
                // Update peer count
                self.peer_count += 1;
                
                // Convert peer_id to GString
                let peer_id_gstring = GString::from(peer_id.clone());
                
                // Emit signal
                self.base_mut().emit_signal(
                    &StringName::from("peer_connected"), 
                    &[peer_id_gstring.to_variant()]
                );
                
                if self.debug_mode {
                    godot_print!("NetworkManagerBridge: Peer connected: {}", peer_id);
                }
            },
            NetworkEvent::Disconnected(peer_id) => {
                // Update peer count
                self.peer_count -= 1;
                
                // Convert peer_id to GString
                let peer_id_gstring = GString::from(peer_id.clone());
                
                // Emit signal
                self.base_mut().emit_signal(
                    &StringName::from("peer_disconnected"), 
                    &[peer_id_gstring.to_variant()]
                );
                
                if self.debug_mode {
                    godot_print!("NetworkManagerBridge: Peer disconnected: {}", peer_id);
                }
            },
            NetworkEvent::DataReceived { peer_id, payload } => {
                // Process received data
                if self.debug_mode {
                    godot_print!("NetworkManagerBridge: Received data from {}: {} bytes", 
                        peer_id, payload.len());
                }
                
                // Here you'd typically decode the payload and dispatch to appropriate handlers
            },
            NetworkEvent::ConnectionError(error) => {
                let error_msg = format!("Connection error: {:?}", error);
                godot_error!("{}", error_msg);
                
                // Emit signal
                self.base_mut().emit_signal(
                    &StringName::from("connection_failed"), 
                    &[error_msg.to_variant()]
                );
            },
        }
    }
    
    // Get connection status
    #[func]
    pub fn is_connected(&self) -> bool {
        self.connected
    }
    
    // Get peer count - Remove this if you have another definition elsewhere
    #[func]
    pub fn get_peer_count_Ubat(&self) -> i32 {
        self.peer_count
    }
    
    // Disconnect from network
    #[func]
    pub fn disconnect(&mut self) {
        self.network_handler = None;
        self.connected = false;
        self.peer_count = 0;
        
        if self.debug_mode {
            godot_print!("NetworkManagerBridge: Disconnected");
        }
    }
}
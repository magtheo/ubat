use godot::prelude::*;

use std::sync::{Arc};

use crate::core::event_bus::{EventBus, PlayerConnectedEvent};


#[derive(GodotClass)]
#[class(base=Node)]
pub struct EventBridge {
    base: Base<Node>,
    event_bus: Option<Arc<EventBus>>,

    player_connected_receiver: Option<std::sync::mpsc::Receiver<String>>,
    player_connected_target: Option<Callable>,
    

}

#[godot_api]
impl INode for EventBridge {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            event_bus: None,
            player_connected_receiver: None,
            player_connected_target: None,

        }
    }
    
    fn ready(&mut self) {
        // Initialize the event bus
        self.event_bus = Some(Arc::new(EventBus::new()));
    }
}

#[godot_api]
impl EventBridge {
    // Method to share the event bus with other Rust components
    pub fn get_event_bus(&self) -> Option<Arc<EventBus>> {
        self.event_bus.clone()
    }

    // Method to set the event bus from another component
    pub fn set_event_bus(&mut self, event_bus: Arc<EventBus>) {
        self.event_bus = Some(event_bus);
    }

    
    #[func]
    fn register_player_connected_callback(&mut self, target: Callable) {
        // Store the target first
        self.player_connected_target = Some(target.clone());

        // Now proceed with event bus subscription
        if let Some(event_bus) = &self.event_bus {
            // Create a channel to send events back to the main thread
            let (sender, receiver) = std::sync::mpsc::channel();

            // Store the receiver
            self.player_connected_receiver = Some(receiver);

            // Clone event bus and target for move closure
            let event_bus_clone = event_bus.clone();

            // Create a thread-safe handler that sends events through the channel
            let handler = Arc::new(move |event: &PlayerConnectedEvent| {
                let player_id = event.player_id.clone();
                // Ignore send errors, as they can happen if the receiver is dropped
                let _ = sender.send(player_id);
            });

            // Subscribe to the event
            event_bus_clone.subscribe(handler);
        }
    }

    #[func]
    fn process_events(&mut self) {
        // Process any pending events from the channels
        self.process_player_connected_events();
    }

    // You'll need to add fields and methods to handle the receivers
    fn store_event_receiver(&mut self, receiver: std::sync::mpsc::Receiver<String>) {
        // Store the receiver in a field
        self.player_connected_receiver = Some(receiver);
    }

    fn process_player_connected_events(&mut self) {
        // Check if we have a receiver and a target
        if let (Some(receiver), Some(target)) = 
            (&self.player_connected_receiver, &self.player_connected_target) {
            // Try to receive all pending events
            while let Ok(player_id) = receiver.try_recv() {
                // Convert player ID to Godot variant
                let player_id_gd = player_id.to_variant();
                
                // Call the target with the player ID
                let _ = target.call(&[player_id_gd]);
            }
        }
    }

    // Add more event registrations as needed
}
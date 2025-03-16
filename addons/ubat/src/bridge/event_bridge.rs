use godot::prelude::*;

use std::sync::{Arc, Mutex};

use crate::core::event_bus::{EventBus, PlayerConnectedEvent};

#[derive(GodotClass)]
#[class(base=Node)]
pub struct EventBridge {
    base: Base<Node>,
    event_bus: Option<Arc<EventBus>>,
}

#[godot_api]
impl EventBridge {
    #[func]
    fn register_player_connected_callback(&mut self, target: Callable) {
        if let Some(event_bus) = &self.event_bus {
            let event_bus_clone = event_bus.clone();
            
            // Create a handler that will invoke the Godot callable
            let handler = Arc::new(move |event: &PlayerConnectedEvent| {
                let player_id = event.player_id.clone();
                
                // Convert to Godot type
                let player_id_gd = player_id.to_variant();
                
                // Call the Godot function
                let _ = target.call(&[player_id_gd]);
            });
            
            event_bus_clone.subscribe(handler);
        }
    }
    
    // Add more event registrations as needed
}
use std::collections::HashMap;
use std::any::{Any, TypeId};
use std::sync::{Arc, Mutex};

/// EventBus
/// 
/// Think of the EventBus like a communication infrastructure for systemlevel events:
// Handles high-level, system-wide events
// Facilitates loose coupling between components
// Provides a way for different systems to broadcast and listen to events

// Examples of EventBus Events

// Game started
// World generated
// Network connection established
// Configuration changed

// Boxed event handler type
type BoxedHandler = Arc<dyn Fn(&dyn Any) + Send + Sync>;

// Trait for event handling
trait EventHandler {
    fn handle_event(&self, event: &dyn Any);
}

// Generic event bus for type-safe event handling
pub struct EventBus {
    handlers: Mutex<HashMap<TypeId, Vec<BoxedHandler>>>,
    initialized: bool,
}

impl EventBus {
    pub fn new() -> Self {
        EventBus {
            handlers: Mutex::new(HashMap::new()),
            initialized: true,
        }
    }

    // Subscribe to a specific event type
    pub fn subscribe<T: 'static>(&self, handler: Arc<dyn Fn(&T) + Send + Sync + 'static>) 
    where
        T: Send + Sync
    {

        let mut handlers = self.handlers.lock().unwrap();
        
        let type_id = TypeId::of::<T>();
        
        // Create a type-erased handler
        let boxed_handler: BoxedHandler = Arc::new(move |event: &dyn Any| {
            if let Some(specific_event) = event.downcast_ref::<T>() {
                handler(specific_event);
            }
        });

        handlers.entry(type_id).or_insert_with(Vec::new).push(boxed_handler);
    }

    // Publish an event to all relevant handlers
    pub fn publish<T: 'static>(&self, event: T) 
    where T: Send + Sync {
        let handlers = self.handlers.lock().unwrap();
        
        let type_id = TypeId::of::<T>();
        
        if let Some(event_handlers) = handlers.get(&type_id) {
            for handler in event_handlers {
                handler(&event);
            }
        }
    }
    // Check if EventBus is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

}

// Example Event Types
#[derive(Debug)]
pub struct PlayerConnectedEvent {
    pub player_id: String,
}

#[derive(Debug)]
pub struct WorldGeneratedEvent {
    pub seed: u64,
    pub world_size: (u32, u32),
}


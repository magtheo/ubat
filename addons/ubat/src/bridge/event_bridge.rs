use godot::prelude::*;
use std::sync::{Arc, mpsc};

use crate::core::event_bus::{EventBus, PlayerConnectedEvent, WorldGeneratedEvent};

/// Event data resource for structured event information
/// 
/// This resource wraps event data in a format that can be easily
/// passed to and from GDScript, with type information preserved.
#[derive(GodotClass)]
#[class(base=Resource)]
pub struct EventData {
    base: Base<Resource>,
    
    // The type of event, used for filtering
    #[export]
    pub event_type: GString,
    
    // Dictionary to store event-specific data
    #[export]
    pub data: Dictionary,
}

#[godot_api]
impl IResource for EventData {
    fn init(base: Base<Resource>) -> Self {
        Self {
            base,
            event_type: "none".into(),
            data: Dictionary::new(),
        }
    }
}

/// EventBridge connects the Rust EventBus system to Godot
///
/// This bridge acts as an interface between the Rust event system and Godot.
/// It provides both signal-based and callable-based event forwarding mechanisms.
///
/// Usage:
/// 1. Add to your scene tree as a node
/// 2. Connect to signals in GDScript: connect("player_connected", self, "_on_player_connected")
/// 3. Or register callbacks: register_player_connected_callback(Callable.new(self, "_on_player_connected"))
/// 4. Call process_events() in your _process function or enable auto_process
/// 5. Handle events in your GDScript callbacks
///
/// Example:
/// ```gdscript
/// func _ready():
///     $EventBridge.connect("player_connected", self, "_on_player_connected")
///     $EventBridge.connect("world_generated", self, "_on_world_generated")
///
/// func _on_player_connected(player_id):
///     print("Player connected: ", player_id)
///
/// func _on_world_generated(seed, width, height):
///     print("World generated with seed:", seed, " size:", width, "x", height)
/// ```
#[derive(GodotClass)]
#[class(base=Node)]
pub struct EventBridge {
    base: Base<Node>,
    
    // Core event bus
    event_bus: Option<Arc<EventBus>>,

    // Channels for thread-safe event passing
    player_connected_receiver: Option<mpsc::Receiver<String>>,
    world_generated_receiver: Option<mpsc::Receiver<(u64, (u32, u32))>>,
    
    // Direct callable targets
    player_connected_target: Option<Callable>,
    world_generated_target: Option<Callable>,
    
    // Configuration options
    #[export]
    auto_process: bool,
    
    #[export]
    debug_mode: bool,
}

#[godot_api]
impl INode for EventBridge {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            event_bus: None,
            player_connected_receiver: None,
            world_generated_receiver: None,
            player_connected_target: None,
            world_generated_target: None,
            auto_process: true,
            debug_mode: false,
        }
    }
    
    fn ready(&mut self) {
        // Initialize the event bus if not already set
        if self.event_bus.is_none() {
            self.event_bus = Some(Arc::new(EventBus::new()));
            
            if self.debug_mode {
                godot_print!("EventBridge: Created new EventBus");
            }
        }
    }
    
    fn process(&mut self, _delta: f64) {
        // Automatically process events each frame if enabled
        if self.auto_process {
            self.process_events();
        }
    }
}

#[godot_api]
impl EventBridge {
    // Signal declarations for all event types
    #[signal]
    fn player_connected(player_id: GString);
    
    #[signal]
    fn player_connected_data(event_data: Gd<EventData>);
    
    #[signal]
    fn world_generated(seed: u64, width: u32, height: u32);
    
    #[signal]
    fn world_generated_data(event_data: Gd<EventData>);
    
    /// Retrieves the internal event bus for other Rust components
    /// 
    /// This method allows sharing the EventBus across multiple Rust components
    pub fn get_event_bus(&self) -> Option<Arc<EventBus>> {
        self.event_bus.clone()
    }

    /// Sets the event bus from an external component
    /// 
    /// This method allows sharing an existing EventBus from elsewhere in the codebase
    pub fn set_event_bus(&mut self, event_bus: Arc<EventBus>) {
        self.event_bus = Some(event_bus);
        
        if self.debug_mode {
            godot_print!("EventBridge: External EventBus set");
        }
    }

    /// Register a callable to be called when a player connects
    /// 
    /// The callable will receive a GString with the player ID
    #[func]
    pub fn register_player_connected_callback(&mut self, target: Callable) {
        // Store the target first
        self.player_connected_target = Some(target);

        // Now set up the event subscription if needed
        if self.player_connected_receiver.is_none() {
            if let Some(event_bus) = &self.event_bus {
                // Create a channel to send events back to the main thread
                let (sender, receiver) = mpsc::channel();

                // Store the receiver
                self.player_connected_receiver = Some(receiver);

                // Subscribe to the event
                let sender = sender.clone();
                let handler = Arc::new(move |event: &PlayerConnectedEvent| {
                    let player_id = event.player_id.clone();
                    // Ignore send errors, as they can happen if the receiver is dropped
                    let _ = sender.send(player_id);
                });

                // Subscribe to the event
                event_bus.subscribe(handler);
                
                if self.debug_mode {
                    godot_print!("EventBridge: Registered PlayerConnectedEvent handler");
                }
            }
        }
    }
    
    /// Register a callable to be called when the world is generated
    /// 
    /// The callable will receive the seed, width, and height parameters
    #[func]
    pub fn register_world_generated_callback(&mut self, target: Callable) {
        // Store the target
        self.world_generated_target = Some(target);
        
        // Set up the event subscription if needed
        if self.world_generated_receiver.is_none() {
            if let Some(event_bus) = &self.event_bus {
                // Create a channel
                let (sender, receiver) = mpsc::channel();
                
                // Store the receiver
                self.world_generated_receiver = Some(receiver);
                
                // Subscribe to the event
                let sender = sender.clone();
                let handler = Arc::new(move |event: &WorldGeneratedEvent| {
                    // Ignore send errors
                    let _ = sender.send((event.seed, event.world_size));
                });
                
                // Subscribe to the event
                event_bus.subscribe(handler);
                
                if self.debug_mode {
                    godot_print!("EventBridge: Registered WorldGeneratedEvent handler");
                }
            }
        }
    }

    /// Process all pending events
    /// 
    /// Call this method in your _process function if auto_process is disabled
    #[func]
    pub fn process_events(&mut self) {
        // Process all event types
        self.process_player_connected_events();
        self.process_world_generated_events();
    }

    /// Process player connected events
    fn process_player_connected_events(&mut self) {
        if let Some(receiver) = &self.player_connected_receiver {
            // Try to receive all pending events
            while let Ok(player_id) = receiver.try_recv() {
                // First emit the simple signal
                self.base_mut().emit_signal(
                    &StringName::from("player_connected"), 
                    &[player_id.clone().to_variant()]
                );
                
                // Create and emit the structured data
                let event_data = Gd::from_init_fn(|base| {
                    let mut event = EventData::init(base);
                    event.event_type = GString::from("player_connected");
                    
                    let mut dict = Dictionary::new();
                    // Convert to GString explicitly
                    dict.set::<Variant, Variant>(
                        GString::from("player_id").to_variant(), 
                        player_id.clone().to_variant()
                    );
                    event.data = dict;
                    
                    event
                });
                
                // Emit the structured data signal
                self.base_mut().emit_signal(
                    &StringName::from("player_connected_data"), 
                    &[event_data.to_variant()]
                );
                
                // Call the target callable if set
                if let Some(target) = &self.player_connected_target {
                    let _ = target.call(&[player_id.to_variant()]);
                }
                
                if self.debug_mode {
                    godot_print!("EventBridge: Processed PlayerConnectedEvent: {}", player_id);
                }
            }
        }
    }
    
    fn process_world_generated_events(&mut self) {
        if let Some(receiver) = &self.world_generated_receiver {
            // Try to receive all pending events
            while let Ok((seed, (width, height))) = receiver.try_recv() {
                // First emit the simple signal
                self.base_mut().emit_signal(
                    &StringName::from("world_generated"), 
                    &[
                        seed.to_variant(),
                        width.to_variant(),
                        height.to_variant()
                    ]
                );
                
                // Create and emit the structured data
                let event_data = Gd::from_init_fn(|base| {
                    let mut event = EventData::init(base);
                    event.event_type = GString::from("world_generated");
                    
                    let mut dict = Dictionary::new();
                    // Explicitly convert keys and values
                    dict.set::<Variant, Variant>(
                        GString::from("seed").to_variant(), 
                        seed.to_variant()
                    );
                    dict.set::<Variant, Variant>(
                        GString::from("width").to_variant(), 
                        width.to_variant()
                    );
                    dict.set::<Variant, Variant>(
                        GString::from("height").to_variant(), 
                        height.to_variant()
                    );
                    event.data = dict;
                    
                    event
                });
                
                // Emit the structured data signal
                self.base_mut().emit_signal(
                    &StringName::from("world_generated_data"), 
                    &[event_data.to_variant()]
                );
                
                // Call the target callable if set
                if let Some(target) = &self.world_generated_target {
                    let _ = target.call(&[
                        seed.to_variant(),
                        width.to_variant(),
                        height.to_variant()
                    ]);
                }
                
                if self.debug_mode {
                    godot_print!("EventBridge: Processed WorldGeneratedEvent: seed={}, size={}x{}", 
                        seed, width, height);
                }
            }
        }
    }
    
    /// Publish a player connected event from GDScript
    #[func]
    pub fn publish_player_connected(&self, player_id: GString) {
        if let Some(event_bus) = &self.event_bus {
            event_bus.publish(PlayerConnectedEvent {
                player_id: player_id.to_string(),
            });
            
            if self.debug_mode {
                godot_print!("EventBridge: Published PlayerConnectedEvent: {}", player_id);
            }
        }
    }
    
    /// Publish a world generated event from GDScript
    #[func]
    pub fn publish_world_generated(&self, seed: u64, width: u32, height: u32) {
        if let Some(event_bus) = &self.event_bus {
            event_bus.publish(WorldGeneratedEvent {
                seed,
                world_size: (width, height),
            });
            
            if self.debug_mode {
                godot_print!("EventBridge: Published WorldGeneratedEvent: seed={}, size={}x{}", 
                    seed, width, height);
            }
        }
    }
}
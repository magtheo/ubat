use godot::prelude::*;

mod core;
mod resource;
mod networking;
mod bridge;
// mod terrain;

// The entry point of your extension library.
struct UbatExtension;


#[gdextension]
unsafe impl ExtensionLibrary for UbatExtension {}

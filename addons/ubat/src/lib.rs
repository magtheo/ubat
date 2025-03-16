use godot::prelude::*;

mod core;
// mod networking;
mod resource;
mod networking;
// mod terrain;

// The entry point of your extension library.
struct UbatExtension;


#[gdextension]
unsafe impl ExtensionLibrary for UbatExtension {}

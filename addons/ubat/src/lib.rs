use godot::prelude::*;

mod core;
mod resource;
mod networking;
mod terrain;
mod initialization;
mod threading;

mod bridge;
mod utils;
mod config;

// The entry point of your extension library.
struct UbatExtension;


#[gdextension]
unsafe impl ExtensionLibrary for UbatExtension {}

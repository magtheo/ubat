// src/global_config.rs
use crate::config::config_manager::{ConfigurationManager, GameConfiguration};
use once_cell::sync::OnceCell; // Add `once_cell` crate: cargo add once_cell
use std::sync::{Arc, RwLock};
use godot::prelude::*; // For godot_print
use godot::classes::ProjectSettings;

// Global static variable to hold the initialized ConfigurationManager
static GLOBAL_CONFIG_MANAGER: OnceCell<Arc<RwLock<ConfigurationManager>>> = OnceCell::new();

const DEFAULT_CONFIG_PATH: &str = "res://game_config.toml";


/// Internal function to perform the actual initialization logic.
fn internal_initialize() -> Arc<RwLock<ConfigurationManager>> {
    godot_print!("Attempting to initialize global configuration lazily...");

    // Use Godot's ProjectSettings to potentially make the path more robust
    // let settings = ProjectSettings::singleton();
    // let global_path = settings.globalize_path(DEFAULT_CONFIG_PATH.into()).to_string();
    // Using globalize_path might be needed if running from outside the project root
    // For now, assume relative path works or adjust DEFAULT_CONFIG_PATH as needed.
    let settings = ProjectSettings::singleton();
    let config_path = settings.globalize_path(DEFAULT_CONFIG_PATH).to_string(); // Get absolute path

    let config_manager = match ConfigurationManager::load_from_file(&config_path) {
        Ok(manager) => {
            godot_print!("Successfully loaded global config from {}", config_path);
            manager
        },
        Err(e) => {
            godot_warn!(
                "Failed to load global config from {}: {}. Using default configuration.",
                config_path, e
            );
            ConfigurationManager::default()
        }
    };
    godot_print!("Global configuration manager initialized via internal_initialize.");
    Arc::new(RwLock::new(config_manager))
}

/// Gets a static reference to the globally initialized ConfigurationManager.
/// Initializes it on the first call if necessary.
pub fn get_config_manager() -> &'static Arc<RwLock<ConfigurationManager>> {
    // get_or_init ensures internal_initialize is called only once, the first time needed.
    GLOBAL_CONFIG_MANAGER.get_or_init(internal_initialize)
}


/// Gets a read-only reference to the current GameConfiguration.
/// Convenience function. Panics if not initialized.
pub fn get_config() -> std::sync::RwLockReadGuard<'static, GameConfiguration> {
    let manager_lock = get_config_manager();
    // We acquire the lock and expect it to succeed. If it's poisoned, something is very wrong.
    let manager_guard = manager_lock.read().expect("Failed to acquire read lock on global config manager (poisoned?)");
    // We need to leak the guard slightly to satisfy the lifetime requirements,
    // ensuring the guard lives as long as the reference it returns.
    // This is generally safe IF the global state isn't destroyed while the guard is held,
    // which is true for a static OnceCell.
    // A cleaner way might involve passing the lock guard around, but this is simpler for read-only access.
    // Alternatively, return the Arc<RwLock<...>> and let callers lock it. Let's do that instead.
    // manager_guard
    // --> Let's change this to return the manager lock instead for safer lock management by caller.
    // This function's signature and purpose changes if we do that. Let's stick to the original intent
    // but acknowledge the lifetime complexity. A read guard is usually fine for quick access.
    // Re-evaluating: Returning the guard directly is tricky with lifetimes.
    // Let's revert to the safer approach: return the manager lock itself.

    // ---- SAFER APPROACH ----
    // Callers will need to lock this themselves.
    // get_config_manager() // Callers use this and then .read() or .write()

    // ---- ORIGINAL INTENT (More complex lifetimes, potentially unsafe if misused) ----
    // If you absolutely need direct access via a simple function:
    // 1. Ensure your GameConfiguration struct implements Clone if not already.
    // 2. Return a clone:
    //    get_config_manager().read().expect("Config lock poisoned").get_config().clone()
    // This avoids lifetime issues but involves a clone each time.

    // Let's provide a specific getter for the terrain config data for TerrainConfigManager init
    // This is safer as it clones just the needed part.
    unimplemented!("get_config() is complex with lifetimes; use get_config_manager() and lock manually, or implement specific getters like get_terrain_config_data()");
}


// Specific getter example
use crate::config::config_manager::TerrainInitialConfigData; // Adjust path if needed
pub fn get_terrain_config_data() -> TerrainInitialConfigData {
    get_config_manager()
        .read()
        .expect("Config lock poisoned")
        .get_config() // Get reference to GameConfiguration
        .terrain // Access the terrain field
        .clone() // Clone the TerrainInitialConfigData
}

// You might add more specific getters here for commonly accessed, cloneable parts.
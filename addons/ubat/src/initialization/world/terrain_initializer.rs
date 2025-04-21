use bincode::Options;
// File: terrain_initializer.rs
use godot::prelude::*;
use godot::classes::{Node, Engine, SceneTree};
use std::sync::{Arc};
use std::time::Instant;
use std::collections::HashMap;

use crate::bridge::{terrain, TerrainBridge};
use crate::terrain::biome_manager::{BiomeManager, ThreadSafeBiomeData};
use crate::initialization::world::terrainInitState::{TerrainInitializationTiming, TerrainInitializationState};
use crate::terrain::ChunkManager;
use crate::terrain::ChunkController;
use crate::utils::error_logger::{ErrorLogger, ErrorSeverity};
use crate::core::event_bus::EventBus;
use crate::terrain::noise::noise_manager::NoiseManager; 


// TerrainSystemContext stores references to initialized terrain components
pub struct TerrainSystemContext {
    pub biome_manager: Option<Gd<BiomeManager>>,
    pub chunk_manager: Option<Gd<ChunkManager>>,
    pub noise_manager: Option<Gd<NoiseManager>>,
    pub thread_safe_biome_data: Option<Arc<ThreadSafeBiomeData>>,
}

pub struct TerrainInitializer {

    biome_manager: Option<Gd<BiomeManager>>,
    chunk_manager: Option<Gd<ChunkManager>>,
    chunk_controller: Option<Gd<ChunkController>>,
    noise_manager: Option<Gd<NoiseManager>>,
    terrain_bridge: Option<Gd<TerrainBridge>>,

    timing: TerrainInitializationTiming,
    error_logger: Arc<ErrorLogger>,
    event_bus: Option<Arc<EventBus>>,

    // COnfigurable values
    world_width: f32,
    world_height: f32,
    seed: u32,
    noise_paths: HashMap<String, String>,
    render_distance: i32,
    
    initialized: bool,
}



impl TerrainInitializer {
    pub fn new() -> Self {
        Self {
            biome_manager: None,
            chunk_manager: None,
            chunk_controller: None,
            noise_manager: None,
            event_bus: None,
            terrain_bridge: None,
            timing: TerrainInitializationTiming::new(),
            error_logger: Arc::new(ErrorLogger::new(100)),

            // Config values
            noise_paths: HashMap::new(),
            world_width: 10000.0,
            world_height: 10000.0,
            seed: 12345,
            render_distance: 4,

            initialized: false,
        }
    }

    // This is the main method to initialize the terrain system
    pub fn initialize_terrain_system(&mut self) -> Result<(), String> {
        if self.initialized {
             godot_warn!("TerrainInitializer: Attempted to initialize terrain system again.");
             return Ok(()); // Already done
        }
        godot_print!("TerrainInitializer: Starting initialization...");
        let start_time = Instant::now();

        // 1. Create parent node for our terrain system
        let mut parent_node = Node::new_alloc();
        parent_node.set_name("TerrainSystem"); // Use GString

        // --- Create and Configure NoiseManager FIRST ---
        let mut noise_manager = NoiseManager::new_alloc();
        noise_manager.set_name("NoiseManager"); // Use GString

        // **IMPORTANT:** Populate noise paths from the stored map
        {
            let mut nm_bind = noise_manager.bind_mut();
            // Convert HashMap<String, String> to Godot Dictionary
            let mut noise_paths_dict = Dictionary::new();
            for (key, path) in &self.noise_paths {
                noise_paths_dict.insert(key.to_variant(), path.to_variant()); // Use GString::from(path) if needed
            }

            // Print regardless, to confirm what's being set
            godot_print!("TerrainInitializer: Setting noise paths on NoiseManager (Count: {}): {:?}",
                noise_paths_dict.len(),
                &self.noise_paths // Log the original HashMap for clarity
            );

            // Assuming NoiseManager has a setter like `set_noise_resource_paths`
            nm_bind.set_noise_resource_paths(noise_paths_dict);
        }
        // Note: NoiseManager's _ready() will run *after* it's added to the scene,
        // where it will then use the paths set above to load parameters.

        // --- Create BiomeManager ---
        let mut biome_manager = BiomeManager::new_alloc();
        biome_manager.set_name("BiomeManager");
        {
            let mut biome_mgr_mut = biome_manager.bind_mut();
            let init_result = biome_mgr_mut.initialize(
                self.world_width,
                self.world_height,
                self.seed
            );
            if !init_result {
                let err_msg = "Failed to initialize BiomeManager".to_string();
                self.error_logger.log_error(
                    "TerrainInitializer", // Module name
                    &err_msg,             // Message
                    ErrorSeverity::Critical, // Severity
                    None                  // Optional context
                );
                return Err(err_msg);
            }
        }

        // --- Create ChunkManager ---
        let mut chunk_manager = ChunkManager::new_alloc();
        chunk_manager.set_name("ChunkManager");

        // --- Create ChunkController ---
        let mut chunk_controller = ChunkController::new_alloc();
        chunk_controller.set_name("ChunkController");

        // --- !!! Create TerrainBridge !!! ---
        let mut terrain_bridge = TerrainBridge::new_alloc();
        terrain_bridge.set_name("TerrainBridge"); // CRUCIAL: Name needed for finding later

        // --- Add all nodes to the parent ---
        // It's generally better to add children *before* adding the parent to the main scene tree
        parent_node.add_child(&noise_manager);
        parent_node.add_child(&biome_manager);
        parent_node.add_child(&chunk_manager);
        parent_node.add_child(&chunk_controller);
        parent_node.add_child(&terrain_bridge); // bridge

        // --- Add parent to scene root ---
        if let Some(mut root) = Self::get_scene_root() {
            godot_print!("TerrainInitializer: Adding TerrainSystem node to scene root.");
            root.add_child(&parent_node); // Add parent_node itself
            // Set owner *after* adding to the loaded scene tree
            parent_node.set_owner(&root); // Owner for TerrainSystem
            // Children likely inherit owner or don't strictly need it set manually here
            // unless you have specific save/instancing requirements.
        } else {
            let err_msg = "Failed to retrieve the scene root node.".to_string();
            self.error_logger.log_error(
                "TerrainInitializer",
                &err_msg,
                ErrorSeverity::Critical,
                None
            );
            return Err(err_msg);
        }

        // Store references
        self.noise_manager = Some(noise_manager.clone()); // <-- Store reference
        self.biome_manager = Some(biome_manager.clone());
        self.chunk_manager = Some(chunk_manager.clone());
        self.chunk_controller = Some(chunk_controller.clone());

        // --- !!! Link Managers TO the TerrainBridge !!! ---
        {
            let mut bridge_bind = terrain_bridge.bind_mut();
            bridge_bind.set_terrain_nodes(
                chunk_manager,    // Pass the Gd
                chunk_controller, // Pass the Gd
                biome_manager,    // Pass the Gd
                // noise_manager, // Add if needed
            );
        }

        // Update initialization state
        self.timing.update_state(TerrainInitializationState::Ready); // Assuming this tracks internal state
        self.initialized = true; // Mark this initializer as having run

        godot_print!("TerrainInitializer: Terrain system nodes created and added to scene in {}ms.", start_time.elapsed().as_millis());
        Ok(())
    }

    // Get the terrain context (components needed by the world manager)
    pub fn get_terrain_context(&self) -> TerrainSystemContext {
        TerrainSystemContext {
            biome_manager: self.biome_manager.clone(),
            chunk_manager: self.chunk_manager.clone(),
            noise_manager: self.noise_manager.clone(), // Pass the Option<Gd<NoiseManager>>
    
            // Check both options before creating ThreadSafeBiomeData
            thread_safe_biome_data: match (&self.biome_manager, &self.noise_manager) {
                (Some(biome_mgr), Some(noise_mgr)) => {
                    // Both managers are Some, proceed to create the data
                    Some(Arc::new(ThreadSafeBiomeData::from_biome_manager(
                        &biome_mgr.bind(),
                        &noise_mgr.bind() // Correctly use the matched 'noise_mgr'
                    )))
                }
                _ => {
                    // Either biome_manager or noise_manager (or both) are None
                    godot_warn!("get_terrain_context: BiomeManager or NoiseManager is None, cannot create ThreadSafeBiomeData.");
                    None // Cannot create data if dependencies are missing
                }
            },
        }
    }

    fn get_scene_root() -> Option<Gd<Node>> {
        // Access the root node of the scene tree
        Engine::singleton()
            .get_main_loop()
            .and_then(|main_loop| Some(main_loop.cast::<SceneTree>())) // Returns Option<Gd<SceneTree>>
            .and_then(|scene_tree| scene_tree.get_root())             // Returns Option<Gd<Window>>
            .map(|root_window| root_window.upcast::<Node>())      // Converts Gd<Window> to Gd<Node>
    }
    
    

    pub fn get_initialization_status(&self) -> Dictionary {
        let mut result = Dictionary::new();

        // Get status of each component
        let biome_initialized = self.biome_manager.is_some() && 
            self.biome_manager.as_ref().unwrap().bind().is_fully_initialized();

        let chunk_manager_initialized = self.chunk_manager.is_some() && 
            self.chunk_manager.as_ref().unwrap().bind().is_initialized();

        let controller_initialized = self.chunk_controller.is_some();

        result.insert("biome_initialized", biome_initialized);
        result.insert("chunk_manager_initialized", chunk_manager_initialized);
        result.insert("controller_initialized", controller_initialized);
        result.insert("fully_initialized", biome_initialized && chunk_manager_initialized && controller_initialized);

        result
    }
    
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
    
    // Configuration setters
    pub fn set_world_dimensions(&mut self, width: f32, height: f32) {
        self.world_width = width;
        self.world_height = height;
    }
    
    pub fn set_seed(&mut self, seed: u32) {
        self.seed = seed;
    }

    // Setter for noise paths
    pub fn set_noise_paths(&mut self, paths: HashMap<String, String>) {
        self.noise_paths = paths;
    }
    
    pub fn set_render_distance(&mut self, distance: i32) {
        self.render_distance = distance;
    }
}
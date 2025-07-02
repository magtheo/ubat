use bincode::Options;
// File: terrain_initializer.rs
use godot::classes::{Engine, FileAccess, Node, ProjectSettings, SceneTree};
use godot::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use crate::bridge::{TerrainBridge, terrain};
use crate::config::global_config;
use crate::core::event_bus::EventBus;
use crate::initialization::world::terrainInitState::{
    TerrainInitializationState, TerrainInitializationTiming,
};
use crate::terrain::ChunkController;
use crate::terrain::ChunkManager;
use crate::terrain::noise::noise_manager::NoiseManager;
use crate::utils::error_logger::{ErrorLogger, ErrorSeverity};

use crate::terrain::section::{SectionManager, ThreadSafeSectionData};

// TerrainSystemContext stores references to initialized terrain components
#[derive(Clone)]
pub struct TerrainSystemContext {
    pub section_manager: Option<Gd<SectionManager>>,
    pub chunk_manager: Option<Gd<ChunkManager>>,
    pub noise_manager: Option<Gd<NoiseManager>>,
    pub thread_safe_section_data: Option<Arc<ThreadSafeSectionData>>,
}

pub struct TerrainInitializer {
    section_manager: Option<Gd<SectionManager>>,
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
            section_manager: None,
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
            return Ok(());
        }
        godot_print!("TerrainInitializer: Starting initialization...");
        let start_time = Instant::now();

        // 1. Create parent node for our terrain system
        let mut parent_node = Node::new_alloc();
        parent_node.set_name("TerrainSystem");

        // 2. Add the parent container to the scene root so that any children you add
        //    later will be considered "ready"
        let mut root = Self::get_scene_root().ok_or_else(|| {
            let msg = "Failed to retrieve the scene root node.".to_string();
            self.error_logger
                .log_error("TerrainInitializer", &msg, ErrorSeverity::Critical, None);
            msg
        })?;
        root.add_child(&parent_node.clone().upcast::<Node>());
        parent_node.set_owner(&root.clone().upcast::<Node>());

        // --- Create & attach NoiseManager ---
        let mut noise_manager = NoiseManager::new_alloc();
        noise_manager.set_name("NoiseManager");
        parent_node.add_child(&noise_manager.clone().upcast::<Node>());
        noise_manager.set_owner(&parent_node.clone().upcast::<Node>());

        // Now that NoiseManager is in the tree and 'ready', setting paths will auto-load
        {
            let mut nm_bind = noise_manager.bind_mut();
            let mut noise_paths_dict = Dictionary::new();
            for (key, path) in &self.noise_paths {
                noise_paths_dict.insert(key.to_variant(), path.to_variant());
            }
            godot_print!(
                "TerrainInitializer: Setting noise paths on NoiseManager (Count: {}): {:?}",
                noise_paths_dict.len(),
                &self.noise_paths
            );
            nm_bind.set_noise_resource_paths(noise_paths_dict);
            // no need to call load_and_extract_all_parameters manually
        }

        // --- Fetch TOML‐loaded section & biome configs + seed ---
        let (sections_config_var, biomes_config_var, seed) = {
            let settings = ProjectSettings::singleton();
            let path = settings.globalize_path("res://game_config.toml");

            // Better error handling
            let file = match FileAccess::open(&path, godot::classes::file_access::ModeFlags::READ) {
                Some(f) => f,
                None => {
                    godot_error!(
                        "TerrainInitializer: Failed to open config file at: {}",
                        path
                    );
                    return Err("Failed to open config file".to_string());
                }
            };

            let raw_config = file.get_as_text().to_string();

            let config: toml::Value = match raw_config.parse() {
                Ok(c) => c,
                Err(e) => {
                    godot_error!("TerrainInitializer: Failed to parse TOML: {}", e);
                    return Err(format!("Failed to parse TOML: {}", e));
                }
            };

            // Safer field access with defaults
            let world_seed = config
                .get("world")
                .and_then(|w| w.get("seed"))
                .and_then(|s| s.as_integer())
                .unwrap_or(12345) as u64;

            godot_print!("TerrainInitializer: Using world seed: {}", world_seed);

            // Parse sections with error checking
            let sections_toml = match config.get("sections").and_then(|v| v.as_array()) {
                Some(arr) => arr,
                None => {
                    godot_error!("TerrainInitializer: No [sections] array found in config");
                    return Err("No sections found in config".to_string());
                }
            };

            let mut sections_array = VariantArray::new();
            for (i, section_toml) in sections_toml.iter().enumerate() {
                let mut dict = Dictionary::new();

                // Safe field extraction with defaults
                let id = section_toml
                    .get("id")
                    .and_then(|v| v.as_integer())
                    .unwrap_or(i as i64) as u8;

                let length = section_toml
                    .get("length")
                    .and_then(|v| v.as_float())
                    .unwrap_or(200.0) as f32;

                let transition_zone = section_toml
                    .get("transition_zone")
                    .and_then(|v| v.as_float())
                    .unwrap_or(50.0) as f32;

                let blend_noise_key = section_toml
                    .get("blend_noise_key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("default_blend");

                dict.set("id", Variant::from(id));
                dict.set("length", Variant::from(length));
                dict.set("transition_zone", Variant::from(transition_zone));
                dict.set("blend_noise_key", Variant::from(blend_noise_key));

                // Parse possible_biomes array
                let mut biomes = VariantArray::new();
                if let Some(biomes_array) = section_toml
                    .get("possible_biomes")
                    .and_then(|v| v.as_array())
                {
                    for biome_id in biomes_array {
                        if let Some(id) = biome_id.as_integer() {
                            biomes.push(&Variant::from(id as u8));
                        }
                    }
                } else {
                    godot_warn!(
                        "TerrainInitializer: Section {} has no possible_biomes, using default [0]",
                        id
                    );
                    biomes.push(&Variant::from(0u8));
                }

                // FIX: Get count before moving
                let biomes_count = biomes.len();
                dict.set("possible_biomes", Variant::from(biomes)); // Move happens here
                sections_array.push(&dict.to_variant());

                // FIX: Use the saved count
                godot_print!(
                    "TerrainInitializer: Parsed section {}: length={}, biomes_count={}",
                    id,
                    length,
                    biomes_count
                );
            }

            // Parse biomes with error checking
            let biomes_toml = match config.get("biomes").and_then(|v| v.as_array()) {
                Some(arr) => arr,
                None => {
                    godot_error!("TerrainInitializer: No [biomes] array found in config");
                    return Err("No biomes found in config".to_string());
                }
            };

            let mut biomes_array = VariantArray::new();
            for (i, biome_toml) in biomes_toml.iter().enumerate() {
                let mut biome_dict = Dictionary::new();

                let id = biome_toml
                    .get("id")
                    .and_then(|v| v.as_integer())
                    .unwrap_or(i as i64) as u8;

                let name = biome_toml
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");

                let primary_noise_key = biome_toml
                    .get("primary_noise_key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("default");

                biome_dict.set("id", Variant::from(id));
                biome_dict.set("name", Variant::from(name));
                biome_dict.set("primary_noise_key", Variant::from(primary_noise_key));
                biome_dict.set("texture_params", Variant::from(Dictionary::new()));
                biome_dict.set("secondary_noise_keys", Variant::from(VariantArray::new()));
                biomes_array.push(&biome_dict.to_variant());

                godot_print!("TerrainInitializer: Parsed biome {}: {}", id, name);
            }

            (
                sections_array.to_variant(),
                biomes_array.to_variant(),
                world_seed,
            )
        };

        // --- Create & attach SectionManager ---
        let mut section_manager = SectionManager::new_alloc();
        section_manager.set_name("SectionManager");
        parent_node.add_child(&section_manager.clone().upcast::<Node>());
        section_manager.set_owner(&parent_node.clone().upcast::<Node>());

        // Initialize SectionManager with our noise_manager
        let init_ok = section_manager.bind_mut().initialize(
            sections_config_var,
            biomes_config_var,
            seed,
            noise_manager.clone(),
        );
        if !init_ok {
            let err_msg = "Failed to initialize SectionManager".to_string();
            godot_error!("TerrainInitializer: {}", err_msg);
            self.error_logger.log_error(
                "TerrainInitializer",
                &err_msg,
                ErrorSeverity::Critical,
                None,
            );
            return Err(err_msg);
        }

        // --- Create & attach ChunkManager, ChunkController, TerrainBridge ---
        let mut chunk_manager = ChunkManager::new_alloc();
        chunk_manager.set_name("ChunkManager");
        parent_node.add_child(&chunk_manager.clone().upcast::<Node>());
        chunk_manager.set_owner(&parent_node.clone().upcast::<Node>());

        let mut chunk_controller = ChunkController::new_alloc();
        chunk_controller.set_name("ChunkController");
        parent_node.add_child(&chunk_controller.clone().upcast::<Node>());
        chunk_controller.set_owner(&parent_node.clone().upcast::<Node>());

        let mut terrain_bridge = TerrainBridge::new_alloc();
        terrain_bridge.set_name("TerrainBridge");
        parent_node.add_child(&terrain_bridge.clone().upcast::<Node>());
        terrain_bridge.set_owner(&parent_node.clone().upcast::<Node>());

        // Link managers into the TerrainBridge
        {
            let mut bridge_bind = terrain_bridge.bind_mut();
            bridge_bind.set_terrain_nodes(
                chunk_manager.clone(),
                chunk_controller.clone(),
                section_manager.clone(),
            );
        }

        // --- Finalize ---
        self.noise_manager = Some(noise_manager);
        self.section_manager = Some(section_manager);
        self.chunk_manager = Some(chunk_manager);
        self.chunk_controller = Some(chunk_controller);
        self.terrain_bridge = Some(terrain_bridge);

        self.timing.update_state(TerrainInitializationState::Ready);
        self.initialized = true;

        godot_print!(
            "TerrainInitializer: Terrain system initialized and added to scene in {}ms.",
            start_time.elapsed().as_millis()
        );
        Ok(())
    }

    // Get the terrain context (components needed by the world manager)
    pub fn get_terrain_context(&self) -> TerrainSystemContext {
        TerrainSystemContext {
            section_manager: self.section_manager.clone(),
            chunk_manager: self.chunk_manager.clone(),
            noise_manager: self.noise_manager.clone(),

            thread_safe_section_data: match (&self.section_manager, &self.noise_manager) {
                (Some(section_mgr_gd), Some(noise_mgr_gd)) => {
                    // Renamed to avoid conflict
                    // --- DEBUG LOGGING START ---
                    godot_print!(
                        "DEBUG: Creating ThreadSafeSectionData. Current SectionManager state:"
                    );
                    // We need to bind the Gd to access its methods
                    let sm_bind = section_mgr_gd.bind(); // Bind here
                    godot_print!("DEBUG:   World Length: {}", sm_bind.get_world_length());
                    godot_print!("DEBUG:   World Width: {}", sm_bind.get_world_width()); // Access internal field if pub or via getter
                    godot_print!(
                        "DEBUG:   Sections Count: {}",
                        sm_bind.get_sections_internal().len()
                    );

                    let sections = sm_bind.get_sections_internal(); // Get internal ref
                    if sections.is_empty() {
                        godot_print!("DEBUG:   No sections defined in SectionManager yet.");
                    } else {
                        for (i, section) in sections.iter().enumerate() {
                            godot_print!(
                                "DEBUG:   Section {}: ID={}, Start={:.2}, End={:.2}, Length={:.2}, Transition={:.2}-{:.2}",
                                i,
                                section.id,
                                section.start_position,
                                section.end_position,
                                section.end_position - section.start_position,
                                section.transition_start,
                                section.transition_end
                            );
                        }
                    }
                    // --- DEBUG LOGGING END ---

                    // Both managers are Some, proceed to create the data
                    Some(Arc::new(ThreadSafeSectionData::from_section_manager(
                        &sm_bind,             // Use the bound reference
                        &noise_mgr_gd.bind(), // Bind noise manager too
                    )))
                }
                _ => {
                    godot_warn!(
                        "get_terrain_context: SectionManager or NoiseManager is None, cannot create ThreadSafeBiomeData."
                    );
                    None
                }
            },
        }
    }

    fn get_scene_root() -> Option<Gd<Node>> {
        // Access the root node of the scene tree
        Engine::singleton()
            .get_main_loop()
            .and_then(|main_loop| Some(main_loop.cast::<SceneTree>())) // Returns Option<Gd<SceneTree>>
            .and_then(|scene_tree| scene_tree.get_root()) // Returns Option<Gd<Window>>
            .map(|root_window| root_window.upcast::<Node>()) // Converts Gd<Window> to Gd<Node>
    }

    pub fn get_initialization_status(&self) -> Dictionary {
        let mut result = Dictionary::new();

        // Get status of each component
        let section_initialized = self.section_manager.is_some()
            && self
                .section_manager
                .as_ref()
                .unwrap()
                .bind()
                .is_fully_initialized();

        let chunk_manager_initialized = self.chunk_manager.is_some()
            && self.chunk_manager.as_ref().unwrap().bind().is_initialized();

        let controller_initialized = self.chunk_controller.is_some();

        result.insert("section_initialized", section_initialized);
        result.insert("chunk_manager_initialized", chunk_manager_initialized);
        result.insert("controller_initialized", controller_initialized);
        result.insert(
            "fully_initialized",
            section_initialized && chunk_manager_initialized && controller_initialized,
        );

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

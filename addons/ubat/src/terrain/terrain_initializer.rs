use godot::prelude::*;
  use godot::classes::Node;
  use std::sync::{Arc, Mutex};
  use std::time::Instant;

  use crate::terrain::TerrainInitializationState;
  use crate::terrain::terrainInitState::TerrainInitializationTiming;
  use crate::terrain::BiomeManager;
  use crate::terrain::ChunkManager;
  use crate::terrain::ChunkController;
  use crate::utils::error_logger::{ErrorLogger, ErrorSeverity};

  #[derive(GodotClass)]
  #[class(base=Node)]
  pub struct TerrainInitializer {
      #[base]
      base: Base<Node>,

      biome_manager: Option<Gd<BiomeManager>>,
      chunk_manager: Option<Gd<ChunkManager>>,
      chunk_controller: Option<Gd<ChunkController>>,

      timing: TerrainInitializationTiming,
      error_logger: Option<Arc<ErrorLogger>>,

      #[export]
      world_width: f32,

      #[export]
      world_height: f32,

      #[export]
      seed: u32,

      #[export]
      render_distance: i32,
  }

  #[godot_api]
  impl INode for TerrainInitializer {
      fn init(base: Base<Node>) -> Self {
          Self {
              base,
              biome_manager: None,
              chunk_manager: None,
              chunk_controller: None,
              timing: TerrainInitializationTiming::new(),
              error_logger: None,
              world_width: 10000.0,
              world_height: 10000.0,
              seed: 12345,
              render_distance: 8,
          }
      }

      fn ready(&mut self) {
          godot_print!("TERRAIN: TerrainInitializer starting 
  initialization...");

          // Setup error logger
          self.error_logger = Some(Arc::new(ErrorLogger::new(100)));

          // Start initialization process
          self.initialize_terrain_system();
      }
  }

  #[godot_api]
  impl TerrainInitializer {
      fn initialize_terrain_system(&mut self) {
          // Create BiomeManager
          let mut biome_manager = BiomeManager::new_alloc();

          // Configure BiomeManager
          {
              let mut biome_mgr_mut = biome_manager.bind_mut();
              biome_mgr_mut.set_world_dimensions(self.world_width,self.world_height);
              biome_mgr_mut.set_seed(self.seed);
          }

          // Add to scene tree
          let node_ref = biome_manager.clone().upcast::<Node>();
          self.base_mut().add_child(&node_ref);

          // Store reference
          self.biome_manager = Some(biome_manager);

          // Update initialization state
          self.timing.update_state(TerrainInitializationState::BiomeInitialized);

          // Create ChunkManager
          let mut chunk_manager = ChunkManager::new_alloc();

          // Add to scene tree
          let node_ref = chunk_manager.clone().upcast::<Node>();
          self.base_mut().add_child(&node_ref);

          // Connect the ChunkManager to BiomeManager
          if let Some(biome_mgr) = &self.biome_manager {
              let mut chunk_mgr_mut = chunk_manager.bind_mut();
              chunk_mgr_mut.set_biome_manager(biome_mgr.clone());
              chunk_mgr_mut.set_render_distance(self.render_distance);
          }

          // Store reference
          self.chunk_manager = Some(chunk_manager);

          // Update initialization state
          self.timing.update_state(TerrainInitializationState::ChunkManagerInitialized);

          // Create ChunkController
          let mut chunk_controller = ChunkController::new_alloc();

          // Add to scene tree
          let node_ref = chunk_controller.clone().upcast::<Node>();
          self.base_mut().add_child(&node_ref);

          // Connect the ChunkController to other components
          if let (Some(chunk_mgr), Some(biome_mgr)) = (&self.chunk_manager, &self.biome_manager) {
              // No direct connections needed here, they'll use node paths
          }

          // Store reference
          self.chunk_controller = Some(chunk_controller);

          // Update initialization state
          self.timing.update_state(TerrainInitializationState::Ready);

          godot_print!("TERRAIN: Terrain system fully initialized in Rust");
      }

      #[func]
      pub fn get_initialization_status(&self) -> Dictionary {
          let mut result = Dictionary::new();

          // Get status of each component
          let biome_initialized = self.biome_manager.is_some() && self.biome_manager.as_ref().unwrap().bind().is_fully_initialized();

          let chunk_manager_initialized = self.chunk_manager.is_some() && self.chunk_manager.as_ref().unwrap().bind().is_initialized();

          let controller_initialized = self.chunk_controller.is_some();

          result.insert("biome_initialized", biome_initialized);
          result.insert("chunk_manager_initialized", chunk_manager_initialized);
          result.insert("controller_initialized", controller_initialized);
          result.insert("fully_initialized", biome_initialized && chunk_manager_initialized && controller_initialized);

          result
      }
  }

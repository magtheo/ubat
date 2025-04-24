// File: src/bridge/terrain_bridge.rs

use godot::prelude::*;
use godot::classes::Node;
use std::sync::{Arc, Mutex};

use crate::terrain::{ChunkController, ChunkManager};
use crate::terrain::section::SectionManager;

#[derive(GodotClass)]
#[class(base=Node)]
pub struct TerrainBridge {
    #[base]
    base: Base<Node>,
    
    // Reference to the ChunkController
    chunk_controller: Option<Gd<ChunkController>>,
    chunk_manager: Option<Gd<ChunkManager>>,
    section_manager: Option<Gd<SectionManager>>,
    // NoiseManager might also be useful to expose if debugger needs noise info
    // noise_manager: Option<Gd<NoiseManager>>,
    
}

#[godot_api]
impl INode for TerrainBridge {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            chunk_controller: None,
            chunk_manager: None,
            section_manager: None,
            // noise_manager: None,
        }
    }
    
    fn ready(&mut self) {
        godot_print!("TerrainBridge: Ready");
    }
}

#[godot_api]
impl TerrainBridge {
    /// Called by TerrainInitializer (or similar) after creating the nodes.
    #[func]
    pub fn set_terrain_nodes(
        &mut self,
        chunk_manager: Gd<ChunkManager>,
        chunk_controller: Gd<ChunkController>,
        section_manager: Gd<SectionManager>,
        // noise_manager: Gd<NoiseManager>, // Add if needed
    ) {
        godot_print!("TerrainBridge: Setting terrain node references...");
        self.chunk_manager = Some(chunk_manager);
        self.chunk_controller = Some(chunk_controller);
        self.section_manager = Some(section_manager);
        // self.noise_manager = Some(noise_manager);
        godot_print!("TerrainBridge: References set.");
    }

    // --- Getter functions for GDScript ---

    #[func]
    pub fn get_chunk_manager(&self) -> Variant {
        // Return as Variant, GDScript can call methods if it's a valid GodotObject
        match &self.chunk_manager {
            Some(cm) => cm.clone().to_variant(),
            None => Variant::nil(),
        }
        // // Alternative: Return Option<Gd<ChunkManager>> if needed, but Variant is easier for GDScript
        // self.chunk_manager.clone()
    }

    #[func]
    pub fn get_chunk_controller(&self) -> Variant {
        match &self.chunk_controller {
            Some(cc) => cc.clone().to_variant(),
            None => Variant::nil(),
        }
        // self.chunk_controller.clone()
    }

    #[func]
    pub fn get_biome_manager(&self) -> Variant {
        match &self.section_manager {
            Some(bm) => bm.clone().to_variant(),
            None => Variant::nil(),
        }
        // self.section_manager.clone()
    }

    // #[func]
    // pub fn get_noise_manager(&self) -> Variant {
    //     match &self.noise_manager {
    //         Some(nm) => nm.clone().to_variant(),
    //         None => Variant::nil(),
    //     }
    // }
}
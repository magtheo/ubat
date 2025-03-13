use godot::prelude::*;
use godot::classes::{Node, CharacterBody2D, Camera2D};
use std::collections::HashSet;

use crate::terrain::SectionReader;
// This will be defined in the future
use crate::world::chunk_handler::ChunkHandler;

/// Configuration for PlayerReader
#[derive(Clone, Copy)]
struct PlayerReaderConfig {
    /// How far in chunks to load around player
    chunk_load_distance: i32,
    /// How far the player needs to move (in world units) to trigger a position update
    position_update_threshold: f32,
    /// Size of each chunk in world units
    chunk_size: f32,
}

/// PlayerReader monitors player position and manages chunk loading
#[derive(GodotClass)]
#[class(base=Node)]
pub struct PlayerReader {
    #[base]
    base: Base<Node>,
    
    // Config
    config: PlayerReaderConfig,
    
    // References to other nodes
    player: Option<Gd<CharacterBody2D>>,
    camera: Option<Gd<Camera2D>>,
    biome_mask: Option<Gd<BiomeMask>>,
    chunk_handler: Option<Gd<ChunkHandler>>,
    
    // State tracking
    last_player_position: Vector2,
    loaded_chunk_coords: HashSet<(i32, i32)>,
    current_biome_color: Color,
}

#[godot_api]
impl INode for PlayerReader {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            config: PlayerReaderConfig {
                chunk_load_distance: 3,
                position_update_threshold: 10.0,
                chunk_size: 256.0,
            },
            player: None,
            camera: None,
            biome_mask: None,
            chunk_handler: None,
            last_player_position: Vector2::ZERO,
            loaded_chunk_coords: HashSet::new(),
            current_biome_color: Color::from_rgba(0.0, 0.0, 0.0, 1.0),
        }
    }

    fn ready(&mut self) {
        // Find required nodes
        self.player = self.base.get_node_as::<CharacterBody2D>("../Player");
        self.camera = self.base.get_node_as::<Camera2D>("../Player/Camera2D");
        self.biome_mask = self.base.get_node_as::<BiomeMask>("../BiomeMask");
        self.chunk_handler = self.base.get_node_as::<ChunkHandler>("../ChunkHandler");
        
        // Log initialization status
        if self.player.is_some() && self.biome_mask.is_some() && self.chunk_handler.is_some() {
            godot_print!("PlayerReader initialized successfully");
        } else {
            godot_error!("PlayerReader failed to find required nodes");
            if self.player.is_none() { godot_error!("Player node not found"); }
            if self.biome_mask.is_none() { godot_error!("BiomeMask node not found"); }
            if self.chunk_handler.is_none() { godot_error!("ChunkHandler node not found"); }
        }
        
        // Set initial player position
        if let Some(player) = &self.player {
            self.last_player_position = player.bind().get_global_position();
            self.update_chunks(true); // Force initial chunk loading
        }
    }

    fn process(&mut self, _delta: f64) {
        self.check_player_position();
    }
}

#[godot_api]
impl PlayerReader {
    #[func]
    fn check_player_position(&mut self) {
        if let Some(player) = &self.player {
            let current_position = player.bind().get_global_position();
            
            // Check if player has moved enough to trigger an update
            let distance_moved = current_position.distance_to(self.last_player_position);
            if distance_moved > self.config.position_update_threshold {
                self.last_player_position = current_position;
                self.update_chunks(false);
                self.update_biome_info();
            }
        }
    }
    
    #[func]
    fn update_chunks(&mut self, force_reload: bool) {
        if let (Some(player), Some(chunk_handler)) = (&self.player, &self.chunk_handler) {
            let player_pos = player.bind().get_global_position();
            
            // Convert player position to chunk coordinates
            let center_chunk_x = (player_pos.x / self.config.chunk_size).floor() as i32;
            let center_chunk_y = (player_pos.y / self.config.chunk_size).floor() as i32;
            
            // Calculate which chunks should be loaded
            let mut chunks_to_load = HashSet::new();
            for x in -self.config.chunk_load_distance..=self.config.chunk_load_distance {
                for y in -self.config.chunk_load_distance..=self.config.chunk_load_distance {
                    let chunk_x = center_chunk_x + x;
                    let chunk_y = center_chunk_y + y;
                    chunks_to_load.insert((chunk_x, chunk_y));
                }
            }
            
            // Find chunks to unload (currently loaded but not in the new set)
            if !force_reload {
                let chunks_to_unload: Vec<(i32, i32)> = self.loaded_chunk_coords
                    .difference(&chunks_to_load)
                    .cloned()
                    .collect();
                
                // Unload chunks that are now too far away
                for chunk_coords in chunks_to_unload {
                    chunk_handler.bind_mut().unload_chunk(chunk_coords.0, chunk_coords.1);
                    self.loaded_chunk_coords.remove(&chunk_coords);
                }
            }
            
            // Load new chunks
            let mut chunk_handler = chunk_handler.bind_mut();
            for chunk_coords in &chunks_to_load {
                if !self.loaded_chunk_coords.contains(chunk_coords) || force_reload {
                    // Get biome color for this chunk from BiomeMask
                    let chunk_world_x = chunk_coords.0 as f32 * self.config.chunk_size;
                    let chunk_world_y = chunk_coords.1 as f32 * self.config.chunk_size;
                    
                    let biome_color = if let Some(biome_mask) = &self.biome_mask {
                        biome_mask.bind_mut().get_biome_color(chunk_world_x, chunk_world_y)
                    } else {
                        // Default color if biome mask isn't available
                        Color::from_rgba(0.5, 0.5, 0.5, 1.0)
                    };
                    
                    // Request chunk from ChunkHandler
                    chunk_handler.load_chunk(
                        chunk_coords.0, 
                        chunk_coords.1, 
                        biome_color
                    );
                    
                    // Add to loaded chunks set
                    self.loaded_chunk_coords.insert(*chunk_coords);
                }
            }
        }
    }
    
    #[func]
    fn update_biome_info(&mut self) {
        if let (Some(player), Some(biome_mask)) = (&self.player, &self.biome_mask) {
            let player_pos = player.bind().get_global_position();
            
            // Get biome color at player's position
            let biome_color = biome_mask.bind_mut().get_biome_color(player_pos.x, player_pos.y);
            
            // Only update if color changed
            if self.current_biome_color != biome_color {
                self.current_biome_color = biome_color;
                
                // Emit signal for biome change if needed
                self.base.emit_signal("biome_changed".into(), &[Variant::from(biome_color)]);
                
                // Log biome change
                godot_print!(
                    "Player entered new biome: R={}, G={}, B={}", 
                    biome_color.r, 
                    biome_color.g, 
                    biome_color.b
                );
            }
        }
    }
    
    // Configuration methods
    #[func]
    pub fn set_chunk_load_distance(&mut self, distance: i32) {
        self.config.chunk_load_distance = distance;
        self.update_chunks(false);
    }
    
    #[func]
    pub fn set_chunk_size(&mut self, size: f32) {
        self.config.chunk_size = size;
        self.update_chunks(true); // Force reload when chunk size changes
    }
    
    #[func]
    pub fn get_current_biome_color(&self) -> Color {
        self.current_biome_color
    }
    
    #[func]
    pub fn force_update(&mut self) {
        self.update_chunks(true);
        self.update_biome_info();
    }
}
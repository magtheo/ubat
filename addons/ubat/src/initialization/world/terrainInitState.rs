use std::time::Instant;
use godot::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainInitializationState {
    Uninitialized,
    ConfigLoaded,
    BiomeInitialized,
    ChunkManagerInitialized,
    Ready,
    Error
}

// Tracks timing data for initialization stages
#[derive(Debug, Clone)]
pub struct TerrainInitializationTiming {
    pub start_time: Instant,
    pub config_loaded_time: Option<Instant>,
    pub biome_initialized_time: Option<Instant>,
    pub chunk_manager_initialized_time: Option<Instant>,
    pub ready_time: Option<Instant>,
    pub current_state: TerrainInitializationState,
}

impl TerrainInitializationTiming {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            config_loaded_time: None,
            biome_initialized_time: None,
            chunk_manager_initialized_time: None,
            ready_time: None,
            current_state: TerrainInitializationState::Uninitialized,
        }
    }
    
    pub fn update_state(&mut self, state: TerrainInitializationState) {
        self.current_state = state;
        let now = Instant::now();
        
        match state {
            TerrainInitializationState::ConfigLoaded => {
                self.config_loaded_time = Some(now);
                godot_print!("TerrainInitState: Config loaded in {}ms", now.duration_since(self.start_time).as_millis());
            },
            TerrainInitializationState::BiomeInitialized => {
                self.biome_initialized_time = Some(now);
                let config_time = self.config_loaded_time.unwrap_or(self.start_time);
                godot_print!("TerrainInitState: Biomes initialized in {}ms", now.duration_since(config_time).as_millis());
            },
            TerrainInitializationState::ChunkManagerInitialized => {
                self.chunk_manager_initialized_time = Some(now);
                let biome_time = self.biome_initialized_time.unwrap_or(self.start_time);
                godot_print!("TerrainInitState: Chunk manager initialized in {}ms", now.duration_since(biome_time).as_millis());
            },
            TerrainInitializationState::Ready => {
                self.ready_time = Some(now);
                let total_time = now.duration_since(self.start_time).as_millis();
                godot_print!("TerrainInitState: Full initialization completed in {}ms", total_time);
                
                // If we have all timestamps, print detailed breakdown
                if self.config_loaded_time.is_some() && self.biome_initialized_time.is_some() && 
                   self.chunk_manager_initialized_time.is_some() {
                    let config_time = self.config_loaded_time.unwrap().duration_since(self.start_time).as_millis();
                    let biome_time = self.biome_initialized_time.unwrap().duration_since(self.config_loaded_time.unwrap()).as_millis();
                    let chunk_time = self.chunk_manager_initialized_time.unwrap().duration_since(self.biome_initialized_time.unwrap()).as_millis();
                    let final_time = now.duration_since(self.chunk_manager_initialized_time.unwrap()).as_millis();
                    
                    godot_print!("TerrainInitState: Detailed timing breakdown:");
                    godot_print!("TerrainInitState: - Config loading: {}ms ({}%)", config_time, config_time * 100 / total_time);
                    godot_print!("TerrainInitState: - Biome initialization: {}ms ({}%)", biome_time, biome_time * 100 / total_time);
                    godot_print!("TerrainInitState: - Chunk manager setup: {}ms ({}%)", chunk_time, chunk_time * 100 / total_time);
                    godot_print!("TerrainInitState: - Final preparation: {}ms ({}%)", final_time, final_time * 100 / total_time);
                }
            },
            TerrainInitializationState::Error => {
                godot_error!("TerrainInitState: Initialization failed after {}ms", now.duration_since(self.start_time).as_millis());
            },
            _ => {},
        }
    }
}
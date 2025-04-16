// src/terrain/noise_manager.rs
use godot::prelude::*;
use godot::classes::{Node, ResourceLoader, FastNoiseLite, NoiseTexture2D, Resource}; // Added Resource
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

// Corrected import path using super:: since it's likely in the same noise/ module
use super::noise_parameters::{NoiseParameters, map_godot_noise_type, map_godot_fractal_type};

#[derive(GodotClass)]
#[class(base=Node)]
pub struct NoiseManager {
    #[base]
    base: Base<Node>,

    noise_resource_paths: Dictionary,

    noise_parameters_cache: Arc<RwLock<HashMap<String, NoiseParameters>>>,
}

#[godot_api]
impl INode for NoiseManager {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            noise_resource_paths: Dictionary::new(),
            noise_parameters_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn ready(&mut self) {
        godot_print!("NoiseManager: Initializing...");
        self.load_and_extract_all_parameters();
        godot_print!("NoiseManager: Initialization complete. Loaded {} noise configurations.",
            self.noise_parameters_cache.read().unwrap().len());
    }
}

#[godot_api]
impl NoiseManager {
    pub fn get_noise_cache_handle(&self) -> Arc<RwLock<HashMap<String, NoiseParameters>>> {
        Arc::clone(&self.noise_parameters_cache)
    }


    pub fn set_noise_resource_paths(&mut self, paths: Dictionary) {
        // Check if already initialized? Optional, might want to allow reloading.
        // if self.noise_parameters_cache.read().unwrap().is_empty() {
             godot_print!("NoiseManager: Setting noise resource paths programmatically.");
             self.noise_resource_paths = paths;
             // Optionally, immediately load/reload parameters here,
             // or rely on _ready() to load them after the node is added to scene.
             // self.load_and_extract_all_parameters(); // Call here if needed before _ready
        // } else {
        //      godot_warn!("NoiseManager: Attempted to set paths after initialization.");
        // }
    }

    fn load_and_extract_all_parameters(&mut self) {
        // **FIXED:** Declare loader as mutable
        let mut loader = ResourceLoader::singleton();
        let mut cache_writer = self.noise_parameters_cache.write().unwrap();
        cache_writer.clear();

        if self.noise_resource_paths.is_empty() {
            godot_warn!("NoiseManager: noise_resource_paths dictionary is empty. No noise will be loaded.");
            return;
        }

        for (key_variant, path_variant) in self.noise_resource_paths.iter_shared() {
            let key = key_variant.to::<GString>().to_string();
            let path = path_variant.to::<GString>();

            if key.is_empty() || path.to_string().is_empty() {
                godot_warn!("NoiseManager: Skipping empty key or path in noise_resource_paths.");
                continue;
            }

            godot_print!("NoiseManager: Loading noise for key '{}' from path: {}", key, path);

            match loader.load(&path) { // Pass reference
                Some(resource) => {
                    // Pass path by reference if needed later
                    if let Some(params) = self.try_extract_parameters_from_resource(resource, &path) {
                         cache_writer.insert(key, params);
                    }
                }
                None => { // This is the None pattern lint was likely complaining about
                    godot_error!("NoiseManager: Failed to load resource at path: {}", path);
                }
            }
        }
    }

    fn try_extract_parameters_from_resource(&self, resource: Gd<Resource>, path: &GString) -> Option<NoiseParameters> {
        let noise_gd: Option<Gd<FastNoiseLite>> = if resource.is_class("NoiseTexture2D") {
            resource.cast::<NoiseTexture2D>()
                .get_noise()
                // **FIXED:** Use .ok() to convert Result from try_cast to Option
                .and_then(|noise_base| noise_base.try_cast::<FastNoiseLite>().ok())
        } else if resource.is_class("FastNoiseLite") {
            Some(resource.cast::<FastNoiseLite>())
        } else {
            godot_error!("NoiseManager: Resource at {} is not NoiseTexture2D or FastNoiseLite", path);
            None
        };

        if let Some(noise_gd) = noise_gd {
            Some(self.extract_parameters(noise_gd))
        } else {
            godot_error!("NoiseManager: Could not get/cast FastNoiseLite from resource: {}", path);
            None
        }
    }

    // Extracts parameters from a FastNoiseLite object
    fn extract_parameters(&self, noise_gd: Gd<FastNoiseLite>) -> NoiseParameters {
        let offset_gd = noise_gd.get_offset();
        NoiseParameters {
            seed: noise_gd.get_seed(),
            frequency: noise_gd.get_frequency(),
            noise_type: map_godot_noise_type(noise_gd.get_noise_type()),
            offset: (offset_gd.x, offset_gd.y, offset_gd.z),
            fractal_type: map_godot_fractal_type(noise_gd.get_fractal_type()),
            fractal_octaves: noise_gd.get_fractal_octaves(),
            fractal_lacunarity: noise_gd.get_fractal_lacunarity(),
            fractal_gain: noise_gd.get_fractal_gain(),
            fractal_weighted_strength: noise_gd.get_fractal_weighted_strength(),
            fractal_ping_pong_strength: noise_gd.get_fractal_ping_pong_strength(),
            // Extract other parameters...
        }
    }
}

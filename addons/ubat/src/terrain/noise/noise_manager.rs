// src/terrain/noise_manager.rs
use godot::prelude::*;
use godot::classes::{Node, ResourceLoader, FastNoiseLite, NoiseTexture2D, Resource}; // Added Resource
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use noise::NoiseFn;

// Corrected import path using super:: since it's likely in the same noise/ module
use super::noise_parameters::{NoiseParameters, map_godot_noise_type, map_godot_fractal_type};
use crate::terrain::noise::noise_utils::create_noise_function_from_params;

#[derive(GodotClass)]
#[class(base=Node)]
pub struct NoiseManager {
    #[base]
    base: Base<Node>,

    noise_resource_paths: Dictionary,

    noise_parameters_cache: Arc<RwLock<HashMap<String, NoiseParameters>>>,

    noise_functions_cache: Arc<RwLock<HashMap<String, Arc<dyn NoiseFn<f64, 2> + Send + Sync>>>>,
}

#[godot_api]
impl INode for NoiseManager {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            noise_resource_paths: Dictionary::new(),
            noise_parameters_cache: Arc::new(RwLock::new(HashMap::new())),
            noise_functions_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn ready(&mut self) {
        godot_print!("NoiseManager: Initializing...");
        self.load_and_extract_all_parameters();
        let param_count = self.noise_parameters_cache.read().unwrap().len();
        let func_count = self.noise_functions_cache.read().unwrap().len();
        godot_print!(
            "NoiseManager: Initialization complete. Loaded {} param configs, created {} function objects.",
            param_count, func_count
        );
    }
}

#[godot_api]
impl NoiseManager {
    pub fn get_noise_cache_handle(&self) -> Arc<RwLock<HashMap<String, NoiseParameters>>> {
        Arc::clone(&self.noise_parameters_cache)
    }


    pub fn set_noise_resource_paths(&mut self, paths: Dictionary) {
        godot_print!("NoiseManager: Setting noise resource paths programmatically.");
        self.noise_resource_paths = paths;
        // Reload parameters and functions if paths are set after ready()
        if self.base().is_node_ready() { // Call base() first
            self.load_and_extract_all_parameters();
        }
    }


   fn load_and_extract_all_parameters(&mut self) {
        let mut loader = ResourceLoader::singleton();
        // Lock both caches for writing
        let Ok(mut params_writer) = self.noise_parameters_cache.write() else {
            godot_error!("NoiseManager: Failed to lock parameters cache for writing.");
            return;
        };
        let Ok(mut funcs_writer) = self.noise_functions_cache.write() else {
            godot_error!("NoiseManager: Failed to lock functions cache for writing.");
            return; // Release params_writer lock implicitly on return
        };

        params_writer.clear();
        funcs_writer.clear(); // Clear function cache too

        if self.noise_resource_paths.is_empty() { /* ... warning ... */ return; }

        for (key_variant, path_variant) in self.noise_resource_paths.iter_shared() {
            let key = key_variant.to::<GString>().to_string();
            let path = path_variant.to::<GString>();
            if key.is_empty() || path.to_string().is_empty() { /* ... warning ... */ continue; }
            godot_print!("NoiseManager: Loading noise for key '{}' from path: {}", key, path);

            match loader.load(&path) {
                Some(resource) => {
                    if let Some(params) = self.try_extract_parameters_from_resource(resource, &path) {
                        // --- ADDED: Create and cache the function object ---
                        let noise_fn_boxed = create_noise_function_from_params(&params);
                        let noise_fn_arc = Arc::from(noise_fn_boxed); // Convert Box to Arc
                        funcs_writer.insert(key.clone(), noise_fn_arc); // Store function Arc
                        // --- END ADDED ---

                        // Store parameters (original logic)
                        params_writer.insert(key, params);
                    }
                }
                None => { godot_error!("NoiseManager: Failed to load resource at path: {}", path); }
            }
        }
    }

    pub fn get_parameters(&self, key: &str) -> Option<NoiseParameters> {
        // Read lock the cache
        match self.noise_parameters_cache.read() {
            Ok(cache) => cache.get(key).cloned(), // Access cache and clone result
            Err(e) => {
                godot_error!("NoiseManager::get_parameters - Failed to lock cache: {}", e);
                None
            }
        }
    }

    pub fn get_function_cache_handle(&self) -> Arc<RwLock<HashMap<String, Arc<dyn NoiseFn<f64, 2> + Send + Sync>>>> {
        Arc::clone(&self.noise_functions_cache)
    }

    pub fn get_noise_function(&self, key: &str) -> Option<Arc<dyn NoiseFn<f64, 2> + Send + Sync>> {
        match self.noise_functions_cache.read() {
             Ok(cache) => cache.get(key).cloned(), // Clones the Arc, not the function
             Err(e) => { godot_error!("NoiseManager::get_noise_function - Failed to lock cache: {}", e); None }
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
            seed: noise_gd.get_seed() as u32,
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

use godot::prelude::*;
use godot::classes::{Texture2D, Shader, ResourceLoader};
use std::cell::RefCell;
use std::rc::Rc;

/// The ResourceManager handles loading, caching, and managing game assets.

pub struct ResourceManager {
    // Caches for textures and shaders.
    texture_cache: Dictionary,
    shader_cache: Dictionary,
    // Base path for assets.
    base_asset_path: GString,
}

impl ResourceManager {
    /// Convenience constructor that wraps init in a Gd pointer.
    pub fn new() -> Self {
        Self {
            texture_cache: Dictionary::new(),
            shader_cache: Dictionary::new(),
            base_asset_path: GString::from("res://assets/"),
        }
    }
    
    // generic resource funciton exposed to godot
    pub fn load_resource(&mut self, path: GString) -> Option<Gd<Resource>> {
        ResourceLoader::singleton().load(&path)
    }

    /// Private helper method (not exposed to Godot)
    /// // For FastNoiseLite resources
    /// let noise = self.load_and_cast::<FastNoiseLite>(&path);

    /// // For textures
    /// let texture = self.load_and_cast::<Texture2D>(&path);

    /// // For shaders
    /// let shader = self.load_and_cast::<Shader>(&path);
    fn load_and_cast<T: GodotClass>(&mut self, path: &GString) -> Option<Gd<T>>
    where 
        T: GodotClass + Inherits<Resource>
        {
        if let Some(resource) = self.load_resource(path.clone()) {
            resource.try_cast::<T>().ok()
        } else {
            None
        }
    }
    
    
    pub fn load_texture(&mut self, path: GString) -> Option<Gd<Texture2D>> {
        // Use key as Variant without an extra '&'
        if let Some(texture_variant) = self.texture_cache.get(path.to_variant()) {
            return texture_variant.try_to::<Gd<Texture2D>>().ok();
        }

        // First check if we can load the resource
        let resource_opt = ResourceLoader::singleton().load(&path);
        if resource_opt.is_none() {
            godot_error!("Failed to load texture {}", path);
            return None;
        }

        // Then try to cast it to a Texture2D
        let resource = resource_opt.unwrap();
        let texture_result = resource.try_cast::<Texture2D>();
        if texture_result.is_err() {
            godot_error!("Resource at {} is not a Texture2D", path);
            return None;
        }

        // If all goes well, cache and return the texture
        let texture = texture_result.unwrap();
        self.texture_cache.insert(path.clone(), texture.clone());
        Some(texture)
    }

    
    pub fn load_shader(&mut self, path: GString) -> Option<Gd<Shader>> {
        if let Some(shader_variant) = self.shader_cache.get(path.to_variant()) {
            return shader_variant.try_to::<Gd<Shader>>().ok();
        }

        // First check if we can load the resource
        let resource_opt = ResourceLoader::singleton().load(&path);
        if resource_opt.is_none() {
            godot_error!("Failed to load shader {}", path);
            return None;
        }

        // Then try to cast it to a Shader
        let resource = resource_opt.unwrap();
        let shader_result = resource.try_cast::<Shader>();
        if shader_result.is_err() {
            godot_error!("Resource at {} is not a Shader", path);
            return None;
        }

        // If all goes well, cache and return the shader
        let shader = shader_result.unwrap();
        self.shader_cache.insert(path.clone(), shader.clone());
        Some(shader)
    }

    
    pub fn clear_cache(&mut self) {
        self.texture_cache.clear();
        self.shader_cache.clear();
    }

    
    pub fn remove_texture(&mut self, path: GString) {
        self.texture_cache.remove(path.to_variant());
    }

    
    pub fn remove_shader(&mut self, path: GString) {
        self.shader_cache.remove(path.to_variant());
    }

    
    pub fn set_asset_base_path(&mut self, path: GString) {
        self.base_asset_path = path;
    }

    
    pub fn get_asset_base_path(&self) -> GString {
        self.base_asset_path.clone()
    }
}

// Since Godot objects aren't thread-safe (they contain raw pointers that
// aren't Send/Sync), we need to handle the singleton differently.
// Instead of a static OnceLock, we'll use a thread-local pattern.

thread_local! {
    static RESOURCE_MANAGER: RefCell<ResourceManager> = RefCell::new(ResourceManager::new());
}

/// Helper functions to access the resource manager singleton
pub mod resource_manager {
    use super::*;

    /// Initialize the resource manager - no-op since it's created on first access
    pub fn init() {
        // Already initialized via thread_local
    }

    /// Execute a function with borrowed access to the resource manager
    pub fn with<F, R>(f: F) -> R
    where
        F: FnOnce(&ResourceManager) -> R,
    {
        RESOURCE_MANAGER.with(|cell| {
            let resource_manager = cell.borrow();
            f(&resource_manager)
        })
    }

    /// Execute a function with mutable borrowed access to the resource manager
    pub fn with_mut<F, R>(f: F) -> R
    where
        F: FnOnce(&mut ResourceManager) -> R,
    {
        RESOURCE_MANAGER.with(|cell| {
            let mut resource_manager = cell.borrow_mut();
            f(&mut resource_manager)
        })
    }

    /// Generic function that can load any resource type
    pub fn load_and_cast<T>(path: GString) -> Option<Gd<T>> 
    where 
        T: GodotClass + Inherits<Resource>
    {
        with_mut(|manager| manager.load_and_cast::<T>(&path))
    }
}
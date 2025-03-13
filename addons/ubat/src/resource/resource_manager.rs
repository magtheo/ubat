use godot::prelude::*;
use godot::classes::{Texture2D, Shader, ResourceLoader};
use std::cell::RefCell;

/// The ResourceManager handles loading, caching, and managing game assets.
#[derive(GodotClass)]
#[class(base=Object, no_init)]
pub struct ResourceManager {
    // The base Godot object.
    base: Base<Object>,
    // Caches for textures and shaders.
    texture_cache: Dictionary,
    shader_cache: Dictionary,
    // Base path for assets.
    base_asset_path: GString,
}

#[godot_api]
impl IObject for ResourceManager {
    // Required default constructor.
    fn init(base: Base<Object>) -> Self {
        Self {
            base,
            texture_cache: Dictionary::new(),
            shader_cache: Dictionary::new(),
            base_asset_path: GString::from("res://assets/"),
        }
    }
}

#[godot_api]
impl ResourceManager {
    /// Convenience constructor that wraps init in a Gd pointer.
    #[func]
    pub fn new() -> Gd<Self> {
        Gd::from_init_fn(|base| Self::init(base))
    }
    
    // generic resource funciton exposed to godot
    #[func]
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
    
    #[func]
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

    #[func]
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

    #[func]
    pub fn clear_cache(&mut self) {
        self.texture_cache.clear();
        self.shader_cache.clear();
    }

    #[func]
    pub fn remove_texture(&mut self, path: GString) {
        self.texture_cache.remove(path.to_variant());
    }

    #[func]
    pub fn remove_shader(&mut self, path: GString) {
        self.shader_cache.remove(path.to_variant());
    }

    #[func]
    pub fn set_asset_base_path(&mut self, path: GString) {
        self.base_asset_path = path;
    }

    #[func]
    pub fn get_asset_base_path(&self) -> GString {
        self.base_asset_path.clone()
    }
}

// Since Godot objects aren't thread-safe (they contain raw pointers that
// aren't Send/Sync), we need to handle the singleton differently.
// Instead of a static OnceLock, we'll use a thread-local pattern.

thread_local! {
    static RESOURCE_MANAGER_INSTANCE: RefCell<Option<Gd<ResourceManager>>> = RefCell::new(None);
}

/// Helper functions to access the resource manager singleton
pub mod resource_manager {
    use super::*;

    /// Initialize the resource manager singleton
    pub fn init() {
        RESOURCE_MANAGER_INSTANCE.with(|cell| {
            let mut instance = cell.borrow_mut();
            if instance.is_none() {
                *instance = Some(ResourceManager::new());
            }
        });
    }

    /// Get the resource manager singleton
    pub fn get() -> Option<Gd<ResourceManager>> {
        let mut result = None;
        RESOURCE_MANAGER_INSTANCE.with(|cell| {
            if let Some(instance) = &*cell.borrow() {
                result = Some(instance.clone());
            }
        });
        result
    }
}
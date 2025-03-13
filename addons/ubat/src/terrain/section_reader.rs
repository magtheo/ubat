use godot::prelude::*;
use godot::classes::{Image, Node, Texture2D, ResourceLoader, Rect2};
use std::collections::HashMap;

/// SectionReader handles loading and accessing a bitmap that defines biome regions
#[derive(GodotClass)]
#[class(base=Node)]
pub struct SectionReader {
    #[base]
    base: Base<Node>,
    
    // 🖼️ Biome Mask Texture
    biome_image: Option<Gd<Image>>,
    mask_width: i32,
    mask_height: i32,
    
    // 🌎 World Size (Determined from the mask)
    world_width: f32,
    world_height: f32,
    
    // ⚙️ Performance Cache
    color_cache: HashMap<String, Color>,
    
    // 🗺️ Biome mask image path
    biome_mask_image: GString,
}

#[godot_api]
impl INode for SectionReader {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            biome_image: None,
            mask_width: 0,
            mask_height: 0,
            world_width: 10000.0,
            world_height: 10000.0,
            color_cache: HashMap::new(),
            biome_mask_image: GString::from("res://textures/biomeMask_image.png"),
        }
    }

    // 🚀 Initialize
    fn ready(&mut self) {
        self.load_mask(self.biome_mask_image.clone());
    }
}

#[godot_api]
impl SectionReader {
    // 📂 Load Biome Mask
    #[func]
    pub fn load_mask(&mut self, path: GString) -> bool {
        let resource_loader = ResourceLoader::singleton();
        match resource_loader.load(&path) {
            Some(texture) => {
                godot_print!("image texture: {}", texture);
                
                match texture.try_cast::<Texture2D>() {
                    Ok(texture_2d) => {
                        let image = texture_2d.bind().get_image();
                        self.biome_image = Some(image.clone());
                        
                        let image_bind = image.bind();
                        self.mask_width = image_bind.get_width();
                        self.mask_height = image_bind.get_height();
                        
                        godot_print!("Biome image dimensions: {}x{}", self.mask_width, self.mask_height);
                        godot_print!("Biome image format: {}", image_bind.get_format());
                        
                        true
                    },
                    Err(_) => {
                        godot_error!("Resource is not a Texture2D: {}", path);
                        false
                    }
                }
            },
            None => {
                godot_error!("Failed to load biome mask at: {}", path);
                false
            }
        }
    }
    
    // 🌎 Map World Coordinates to Biome Mask Coordinates
    #[func]
    pub fn world_to_mask_coords(&self, world_x: f32, world_y: f32) -> Vector2i {
        let mask_x = ((world_x / self.world_width) * self.mask_width as f32) as i32;
        let mask_y = ((world_y / self.world_height) * self.mask_height as f32) as i32;
        
        Vector2i::new(
            mask_x.clamp(0, self.mask_width - 1),
            mask_y.clamp(0, self.mask_height - 1)
        )
    }
    
    // 🎨 Get the Biome Color from the Mask
    #[func]
    pub fn get_biome_color(&mut self, world_x: f32, world_y: f32) -> Color {
        let coords = self.world_to_mask_coords(world_x, world_y);
        let key = format!("{}_{}", coords.x, coords.y);
        
        // 🚀 Use Cache for Performance
        if let Some(color) = self.color_cache.get(&key) {
            return *color;
        }
        
        // Get pixel color and cache it
        if let Some(image) = &self.biome_image {
            let color = image.bind().get_pixel(coords.x, coords.y);
            self.color_cache.insert(key, color);
            color
        } else {
            // Return a default color if image isn't loaded
            Color::from_rgba(1.0, 0.0, 1.0, 1.0) // Magenta as error color
        }
    }
    
    // 📏 Get World Boundaries
    #[func]
    pub fn get_world_bounds(&self) -> Rect2 {
        Rect2::from_position_and_size(
            Vector2::new(0.0, 0.0),
            Vector2::new(self.world_width, self.world_height)
        )
    }
    
    // 🧹 Clear Cache (useful if the mask is updated)
    #[func]
    pub fn clear_cache(&mut self) {
        self.color_cache.clear();
    }
    
    // Setters and getters for world dimensions
    #[func]
    pub fn set_world_dimensions(&mut self, width: f32, height: f32) {
        self.world_width = width;
        self.world_height = height;
        self.clear_cache(); // Cache is no longer valid with new dimensions
    }
    
    #[func]
    pub fn get_world_width(&self) -> f32 {
        self.world_width
    }
    
    #[func]
    pub fn get_world_height(&self) -> f32 {
        self.world_height
    }
}
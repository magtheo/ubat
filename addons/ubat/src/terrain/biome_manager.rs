use godot::prelude::*;
use godot::classes::{Image, Node, Texture2D, FastNoiseLite};
use godot::classes::RandomNumberGenerator;
use godot::builtin::{Color, Rect2, Vector2, Vector2i};
use std::collections::HashMap;
use std::cmp::Ordering;
use std::sync::{Arc, Mutex};

use crate::resource::resource_manager::resource_manager;
use crate::terrain::chunk_manager::ChunkManager;

// Structure to define a Voronoi point for biome distribution
struct VoronoiPoint {
    position: Vector2,
    biome_id: u8,
}

// Structure to define a section with its associated biomes
struct BiomeSection {
    section_id: u8,
    possible_biomes: Vec<u8>,
    voronoi_points: Vec<VoronoiPoint>,
    point_density: f32, // Points per 1000x1000 world units
}

// Thread-safe versions of biome structures
pub struct ThreadSafeBiomeData {
    world_width: f32,
    world_height: f32,
    seed: u32,
    sections: Vec<ThreadSafeBiomeSection>,
    blend_distance: f32,
}

struct ThreadSafeBiomeSection {
    section_id: u8,
    possible_biomes: Vec<u8>,
    voronoi_points: Vec<ThreadSafeVoronoiPoint>,
}

struct ThreadSafeVoronoiPoint {
    position: (f32, f32),
    biome_id: u8,
}


// BiomeManager handles loading and accessing a bitmap that defines biome regions
#[derive(GodotClass)]
#[class(base=Node)]
pub struct BiomeManager {
    #[base]
    base: Base<Node>,
    
    // Biome Mask Texture
    biome_image: Option<Gd<Image>>,
    mask_width: i32,
    mask_height: i32,
    
    // World Size
    world_width: f32,
    world_height: f32,
    
    // Performance Cache
    color_cache: Arc<Mutex<HashMap<String, Color>>>,
    section_cache: Arc<Mutex<HashMap<String, u8>>>,
    biome_cache: Arc<Mutex<HashMap<String, u8>>>,
    
    // Biome mask image path
    biome_mask_image_path: GString,
    noise_path: GString,
    
    // Biome configuration
    sections: Vec<BiomeSection>,
    blend_distance: f32,   // Distance over which biomes blend
    noise: Option<Gd<FastNoiseLite>>, // Noise for biome blending
    
    // Is the system initialized
    initialized: bool,
    seed: u32,
    
    // Random number generator for voronoi points
    rng: Gd<RandomNumberGenerator>,
}

#[godot_api]
impl INode for BiomeManager {
    fn init(base: Base<Node>) -> Self {
        let mut rng = RandomNumberGenerator::new_gd();
        rng.randomize();
        
        Self {
            base,
            biome_image: None,
            mask_width: 0,
            mask_height: 0,
            world_width: 10000.0,
            world_height: 10000.0,
            color_cache: Arc::new(Mutex::new(HashMap::new())),
            section_cache: Arc::new(Mutex::new(HashMap::new())),
            biome_cache: Arc::new(Mutex::new(HashMap::new())),
            biome_mask_image_path: GString::from("res://textures/biomeMask_image.png"),
            noise_path: GString::from("res://resources/noise/biome_blend_noise.tres"),
            sections: Vec::new(),
            blend_distance: 200.0,
            noise: None,
            initialized: false,
            seed: 12345,
            rng,
        }
    }

    // Initialize
    fn ready(&mut self) {
        godot_print!("BiomeManager: Initializing...");
        
        // Initialize resource manager if needed
        resource_manager::init();
        
        godot_print!("BiomeManager: Loading mask from {}", self.biome_mask_image_path);
        self.load_mask(self.biome_mask_image_path.clone());
        
        godot_print!("BiomeManager: Loading noise from {}", self.noise_path);
        self.load_noise(self.noise_path.clone());
        
        godot_print!("BiomeManager: Setting up biome sections");
        self.setup_biome_sections();
        
        godot_print!("BiomeManager: Initializing Voronoi points with seed {}", self.seed);
        self.initialize_voronoi_points();
        
        self.initialized = true;
        godot_print!("BiomeManager: Initialization complete");
    }
}

#[godot_api]
impl BiomeManager {
    // Load Biome Mask from image
    #[func]
    pub fn load_mask(&mut self, path: GString) -> bool {
        // Try to load using resource manager
        let texture = match resource_manager::load_and_cast::<Texture2D>(path.clone()) {
            Some(tex) => tex,
            none => {
                godot_error!("Failed to load texture from path: {}", path);
                return false;
            }
        };

        // Get image from texture
        let image = match texture.get_image() {
            Some(img) => img,
            none => {
                godot_error!("Failed to get image from texture");
                return false;
            }
        };

        // Store the image
        self.biome_image = Some(image.clone());
        
        // Update dimensions
        self.mask_width = image.get_width();
        self.mask_height = image.get_height();
        
        godot_print!("Biome image loaded: {}x{}", self.mask_width, self.mask_height);
        true
    }
    
    // Load FastNoiseLite from resource
    #[func]
    pub fn load_noise(&mut self, path: GString) -> bool {
        // Try to load using resource manager
        match resource_manager::load_and_cast::<FastNoiseLite>(path.clone()) {
            Some(noise) => {
                self.noise = Some(noise);
                godot_print!("Loaded FastNoiseLite from: {}", path);
                true
            },
            none => {
                godot_error!("Failed to load FastNoiseLite from path: {}", path);
                // Create a new noise as fallback
                let mut noise = FastNoiseLite::new_gd();
                noise.set_seed(self.seed as i32);
                noise.set_frequency(0.01);
                noise.set_fractal_octaves(4);
                self.noise = Some(noise);
                false
            }
        }
    }
    
    // Setup biome sections
    fn setup_biome_sections(&mut self) {
        // Clear existing sections
        self.sections.clear();
        
        // Define sections with their possible biomes
        // Section 1:
        self.sections.push(BiomeSection {
            section_id: 1,
            possible_biomes: vec![1, 2],  // sand, Corral
            voronoi_points: Vec::new(),
            point_density: 5.0,  // 5 points per 1000x1000 area
        });
        
        // Section 2: 
        self.sections.push(BiomeSection {
            section_id: 2,
            possible_biomes: vec![3, 4],  // rock, kelp
            voronoi_points: Vec::new(),
            point_density: 3.0,  // 3 points per 1000x1000 area
        });
        
        // Section 3: 
        self.sections.push(BiomeSection {
            section_id: 3,
            possible_biomes: vec![3, 5],  // rock, lavarock
            voronoi_points: Vec::new(),
            point_density: 4.0,  // 4 points per 1000x1000 area
        });
        
        // Make sure noise is initialized
        if self.noise.is_none() {
            let mut noise = FastNoiseLite::new_gd();
            noise.set_seed(self.seed as i32);
            noise.set_frequency(0.01);
            noise.set_fractal_octaves(4);
            self.noise = Some(noise);
        }
        
        godot_print!("Biome sections initialized");
    }
    
    // Initialize Voronoi points for each section
    fn initialize_voronoi_points(&mut self) {
        // Set the RNG seed
        self.rng.set_seed(self.seed as u64);
        
        // For each section
        for section in &mut self.sections {
            section.voronoi_points.clear();
            
            // Calculate how many points for each section
            let points_count = ((self.world_width * self.world_height) / 1_000_000.0 * section.point_density) as usize;
            
            for _ in 0..points_count {
                // Generate random position within world bounds
                let pos_x = self.rng.randf_range(0.0, self.world_width);
                let pos_y = self.rng.randf_range(0.0, self.world_height);
                
                // Select random biome from possible biomes for this section
                let biome_idx = self.rng.randi_range(0, section.possible_biomes.len() as i32 - 1) as usize;
                let biome_id = section.possible_biomes[biome_idx];
                
                section.voronoi_points.push(VoronoiPoint {
                    position: Vector2::new(pos_x, pos_y),
                    biome_id,
                });
            }
        }
        
        godot_print!("Voronoi points initialized for all sections ({} total sections)", self.sections.len());
    }
    
    // Map World Coordinates to Biome Mask Coordinates
    #[func]
    pub fn world_to_mask_coords(&self, world_x: f32, world_y: f32) -> Vector2i {
        let mask_x = ((world_x / self.world_width) * self.mask_width as f32) as i32;
        let mask_y = ((world_y / self.world_height) * self.mask_height as f32) as i32;
        
        Vector2i::new(
            mask_x.clamp(0, self.mask_width - 1),
            mask_y.clamp(0, self.mask_height - 1)
        )
    }

    #[func]
    pub fn get_seed(&self) -> u32 {
        self.seed
    }
    
    // Get the Section Color from the Mask
    #[func]
    pub fn get_biome_color(&mut self, world_x: f32, world_y: f32) -> Color {
        let coords = self.world_to_mask_coords(world_x, world_y);
        let key = format!("{}_{}", coords.x, coords.y);
        
        // Use Cache for Performance
        // Safe cache access
        {
            let cache = self.color_cache.lock().unwrap();
            if let Some(color) = cache.get(&key) {
                return *color;
            }
        }

        // Get pixel color and cache it
        match &self.biome_image {
            Some(image) => {
                let color = image.get_pixel(coords.x, coords.y);
                
                // Thread-safe cache insertion
                {
                    let mut cache = self.color_cache.lock().unwrap();
                    cache.insert(key, color);
                }
                
                color
            },
            _none => Color::from_rgba(1.0, 0.0, 1.0, 1.0) // Magenta as error color
        }
    }
    // TODO: this needs rework to allow for the secitons defined previously
    // Get the section ID from color
    #[func]
    pub fn get_section_id(&mut self, world_x: f32, world_y: f32) -> u8 {
        let key = format!("section_{}_{}", world_x as i32, world_y as i32);
        
        // Thread-safe cache check
    {
        let cache = self.section_cache.lock().unwrap();
        if let Some(&section_id) = cache.get(&key) {
            return section_id;
        }
    }
        
        // Get the color from the biome mask
        let color = self.get_biome_color(world_x, world_y);
        
        // Map colors to sections based on your biomeMask_image.png
        // Red components (r > 0.7) = Section 1 (Coral/Sand)
        // Green components (g > 0.7) = Section 2 (Rock/Kelp)
        // Blue components (b > 0.7) = Section 3 (Rock/Lavarock)
        
        let section_id = if color.r > 0.7 {
            1 // Section 1: Coral & Sand
        } else if color.g > 0.7 {
            2 // Section 2: Rock & Kelp
        } else if color.b > 0.7 {
            3 // Section 3: Rock & Lavarock
        } else {
            // For mixed colors or undefined areas, determine section by dominance
            let max_component = f32::max(f32::max(color.r, color.g), color.b);
            
            if max_component < 0.1 {
                0 // Very dark: undefined section
            } else if color.r >= color.g && color.r >= color.b {
                1 // Red dominant: Section 1
            } else if color.g >= color.r && color.g >= color.b {
                2 // Green dominant: Section 2
            } else {
                3 // Blue dominant: Section 3
            }
        };
        
        // Thread-safe cache insertion
        {
            let mut cache = self.section_cache.lock().unwrap();
            cache.insert(key, section_id);
        }

        section_id
    }
    
    // Get the biome ID at a specific world position
    #[func]
    pub fn get_biome_id(&mut self, world_x: f32, world_y: f32) -> u8 {
        if !self.initialized {
            return 0;
        }
        
        let cache_key = format!("biome_{}_{}", (world_x * 0.1) as i32, (world_y * 0.1) as i32);
        
        // Check cache first in a separate scope so the lock is released
        {
            let cache = self.biome_cache.lock().unwrap();
            if let Some(&biome_id) = cache.get(&cache_key) {
                return biome_id;
            }
        }// Lock is released here when cache goes out of scope

        // Get the section ID for this position
        let section_id = self.get_section_id(world_x, world_y);
        
        // Find the section
        let section_idx = self.sections.iter().position(|s| s.section_id == section_id);
        
        if let Some(section_idx) = section_idx {
            let section = &self.sections[section_idx];
            
            // Find closest Voronoi points for this section
            if section.voronoi_points.is_empty() {
                // No points in this section, return the first possible biome
                let default_biome = section.possible_biomes.first().unwrap_or(&0);
                
                // Lock again in a new scope
                {
                    let mut cache = self.biome_cache.lock().unwrap();
                    cache.insert(cache_key, *default_biome);
                } // Lock is released here
                
                return *default_biome;
            }
            
            // Create a position vector
            let pos = Vector2::new(world_x, world_y);
            
            // Calculate distances to all Voronoi points in this section
            let mut distances: Vec<(f32, &VoronoiPoint)> = section.voronoi_points.iter()
                .map(|point| {
                    let distance = pos.distance_to(point.position);
                    (distance, point)
                })
                .collect();
            
            // Sort by distance
            distances.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
            
            // Check if we need to blend between two closest points
            if distances.len() >= 2 {
                let (dist1, point1) = distances[0];
                let (dist2, point2) = distances[1];
                
                // If the points are close enough, blend between them
                if (dist2 - dist1) < self.blend_distance {
                    // Calculate blend factor with noise influence for natural borders
                    let noise_val = if let Some(ref noise) = self.noise {
                        // Use Godot's FastNoiseLite
                        noise.get_noise_2d(world_x * 0.01, world_y * 0.01) * 0.5 + 0.5
                    } else {
                        // Fallback if noise is not available
                        0.5
                    };
                    
                    let blend_factor = ((dist2 - dist1) / self.blend_distance).min(1.0);
                    let adjusted_blend = blend_factor * (1.0 - noise_val * 0.3); // Noise influence
                    
                    // If blend is needed, choose randomly between the two biomes
                    // with probability based on distance
                    if self.rng.randf() > adjusted_blend {
                        {
                            let mut cache = self.biome_cache.lock().unwrap();
                            cache.insert(cache_key, point1.biome_id);
                        }
                        return point1.biome_id;
                    } else {
                        {
                            let mut cache = self.biome_cache.lock().unwrap();
                            cache.insert(cache_key, point2.biome_id);
                        }
                        return point2.biome_id;
                    }
                }
            }
            
            // If no blending or only one point, return the closest
            if !distances.is_empty() {
                let biome_id = distances[0].1.biome_id;
                {  
                    let mut cache = self.biome_cache.lock().unwrap();
                    cache.insert(cache_key, biome_id);
                }
                return biome_id;
            }
        }
        
        // Default biome if no section found or other error
        0
    }
    
    // Get World Boundaries
    #[func]
    pub fn get_world_bounds(&self) -> Rect2 {
        Rect2::new(
            Vector2::new(0.0, 0.0),
            Vector2::new(self.world_width, self.world_height)
        )
    }
    
    // Clear Cache
    #[func]
    pub fn clear_cache(&mut self) {
        // Thread-safe cache clearing
        self.color_cache.lock().unwrap().clear();
        self.section_cache.lock().unwrap().clear();
        self.biome_cache.lock().unwrap().clear();
    }
    
    // Set world dimensions
    #[func]
    pub fn set_world_dimensions(&mut self, width: f32, height: f32) {
        self.world_width = width;
        self.world_height = height;
        self.clear_cache();
        self.initialize_voronoi_points(); // Recreate Voronoi points for new dimensions
    
        // Notify ChunkManager if possible
        self.notify_data_change();
    }
    
    // Set seed for procedural generation
    #[func]
    pub fn set_seed(&mut self, new_seed: u32) {
        self.seed = new_seed;
        
        // Update noise seed if we have a FastNoiseLite
        if let Some(ref mut noise) = self.noise {
            noise.set_seed(new_seed as i32);
        }
        
        self.clear_cache();
        self.initialize_voronoi_points();
        
        // Notify ChunkManager if possible
        self.notify_data_change();
    }

    
    // Set blend distance for smoother transitions
    #[func]
    pub fn set_blend_distance(&mut self, distance: f32) {
        self.blend_distance = distance;
        self.clear_cache();

        // Notify ChunkManager if possible
        self.notify_data_change();
    }
    
   // Helper method to notify ChunkManager
    fn notify_data_change(&self) {
        // Try to find ChunkManager in the scene tree
        if let Some(parent) = self.base().get_parent() {
            // Use a string literal directly
            let node_path = "ChunkManager";
            if let Some(chunk_manager) = parent.get_node_or_null(node_path) {
                // Use match for Result instead of if let for Option
                match chunk_manager.try_cast::<ChunkManager>() {
                    Ok(mut chunk_manager) => {
                        chunk_manager.bind_mut().update_thread_safe_biome_data();
                    },
                    Err(_) => {
                        godot_print!("Failed to cast node to ChunkManager");
                    }
                }
            }
        }
    }

    // Get a biome name for display
    #[func]
    pub fn get_biome_name(&self, biome_id: u8) -> GString {
        match biome_id {
            0 => "Unknown".into(),
            1 => "Coral".into(),
            2 => "Sand".into(),
            3 => "Rock".into(),
            4 => "Kelp".into(),
            5 => "Lavarock".into(),
            _ => format!("Biome {}", biome_id).into(),
        }
    }
    
    // Debug method to visualize a specific section's Voronoi points
    #[func]
    pub fn debug_get_voronoi_points(&self, section_id: u8) -> PackedVector2Array {
        let mut points = PackedVector2Array::new();
        
        if let Some(section) = self.sections.iter().find(|s| s.section_id == section_id) {
            for point in &section.voronoi_points {
                points.push(point.position);
            }
        }
        
        points
    }
    
    // Set the noise resource path and reload it
    #[func]
    pub fn set_noise_resource(&mut self, path: GString) -> bool {
        self.noise_path = path.clone();
        self.load_noise(path)
    }
    
    // Export section data for debugging
    #[func]
    pub fn debug_section_info(&self) -> Dictionary {
        let mut result = Dictionary::new();
        
        for (i, section) in self.sections.iter().enumerate() {
            let mut section_dict = Dictionary::new();
            section_dict.insert("section_id", section.section_id);
            section_dict.insert("point_count", section.voronoi_points.len() as i64);
            section_dict.insert("biome_count", section.possible_biomes.len() as i64);
            
            let mut biomes_array = VariantArray::new();
            for biome in &section.possible_biomes {
                let value = (*biome as i64).to_variant();
                biomes_array.push(&value);
            }
            section_dict.insert("biomes", biomes_array);
            
            result.insert(format!("section_{}", i), section_dict);
        }
        
        result
    }
}

impl ThreadSafeBiomeData {
    pub fn from_biome_manager(biome_mgr: &BiomeManager) -> Self {
        let mut sections = Vec::new();
        
        // Clone all sections and their Voronoi points
        for section in &biome_mgr.sections {
            let mut voronoi_points = Vec::new();
            
            for point in &section.voronoi_points {
                voronoi_points.push(ThreadSafeVoronoiPoint {
                    position: (point.position.x, point.position.y),
                    biome_id: point.biome_id,
                });
            }
            
            sections.push(ThreadSafeBiomeSection {
                section_id: section.section_id,
                possible_biomes: section.possible_biomes.clone(),
                voronoi_points,
            });
        }
        
        ThreadSafeBiomeData {
            world_width: biome_mgr.world_width,
            world_height: biome_mgr.world_height,
            seed: biome_mgr.seed,
            sections,
            blend_distance: biome_mgr.blend_distance,
        }
    }
    
    // Get section ID based on world coordinates
    pub fn get_section_id(&self, world_x: f32, world_y: f32) -> u8 {
        // Simplified version - you might need a more complex algorithm
        // based on your original get_section_id implementation
        
        // Basic section determination based on position
        let relative_x = world_x / self.world_width;
        let relative_y = world_y / self.world_height;
        
        if relative_x < 0.33 {
            1 // Section 1: Forest
        } else if relative_x < 0.66 {
            2 // Section 2: Mountain
        } else {
            3 // Section 3: Plains
        }
    }
    
    // Get biome ID at world coordinates
    pub fn get_biome_id(&self, world_x: f32, world_y: f32) -> u8 {
        // Get the section for this position
        let section_id = self.get_section_id(world_x, world_y);
        
        // Find the section
        if let Some(section) = self.sections.iter().find(|s| s.section_id == section_id) {
            // If no Voronoi points, return first possible biome
            if section.voronoi_points.is_empty() {
                return *section.possible_biomes.first().unwrap_or(&0);
            }
            
            // Calculate distances to all Voronoi points in this section
            let pos = (world_x, world_y);
            let mut distances: Vec<(f32, &ThreadSafeVoronoiPoint)> = section.voronoi_points.iter()
                .map(|point| {
                    let dx = pos.0 - point.position.0;
                    let dy = pos.1 - point.position.1;
                    let distance = (dx * dx + dy * dy).sqrt();
                    (distance, point)
                })
                .collect();
            
            // Sort by distance
            distances.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            
            // Return the biome ID of the closest point
            if !distances.is_empty() {
                return distances[0].1.biome_id;
            }
        }
        
        // Default biome
        0
    }
    
    // Get biome color based on biome ID
    pub fn get_biome_color(&self, world_x: f32, world_y: f32) -> Color {
        let biome_id = self.get_biome_id(world_x, world_y);
        
        // Generate a color based on biome ID
        match biome_id {
            1 => Color::from_rgba(0.8, 0.2, 0.2, 1.0), // Coral - reddish
            2 => Color::from_rgba(0.9, 0.9, 0.2, 1.0), // Sand - yellowish
            3 => Color::from_rgba(0.5, 0.5, 0.5, 1.0), // Rock - gray
            4 => Color::from_rgba(0.2, 0.8, 0.2, 1.0), // Kelp - greenish
            5 => Color::from_rgba(0.8, 0.4, 0.1, 1.0), // Lavarock - orange
            _ => Color::from_rgba(1.0, 0.0, 1.0, 1.0), // Magenta for unknown
        }
    }
}
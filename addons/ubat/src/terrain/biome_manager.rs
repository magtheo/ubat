use godot::prelude::*;
use godot::classes::{Image, Node, Texture2D, FastNoiseLite};
use godot::classes::RandomNumberGenerator;
use godot::builtin::{Color, Rect2, Vector2, Vector2i};
use std::collections::HashMap;
use std::cmp::Ordering;
use std::sync::{Arc, RwLock};

use crate::terrain::noise::noise_parameters::NoiseParameters; // Assuming you have this struct
use noise::{NoiseFn, Seedable, Perlin}; // Import necessary noise-rs items
use rand::{SeedableRng, Rng}; // For deterministic PRNG
use rand_chacha::ChaCha8Rng; // A good deterministic PRNG
use std::hash::{Hash, Hasher}; // For hashing option
use std::collections::hash_map::DefaultHasher; // For hashing option

use crate::resource::resource_manager::resource_manager;
use crate::terrain::chunk_manager::ChunkManager;
use crate::terrain::generation_rules::GenerationRules;

use crate::utils::error_logger::{ErrorLogger, ErrorSeverity};

use super::noise::NoiseManager;

#[derive(Debug, Clone, Copy, PartialEq)]
enum BiomeManagerState {
    Uninitialized,
    Initializing,
    Initialized,
    Error,
}

// Structure to define a Voronoi point for biome distribution
struct VoronoiPoint {
    position: Vector2,
    biome_id: u8,
}

// Grid cell for spatial partitioning
struct SpatialCell {
    voronoi_points: Vec<(usize, usize)>, // (section_index, point_index)
}

// Spatial partitioning grid
struct SpatialGrid {
    cells: Vec<Vec<SpatialCell>>,
    cell_size: f32,
    grid_width: usize,
    grid_height: usize,
}

impl SpatialGrid {
    fn new(world_width: f32, world_height: f32, cell_size: f32) -> Self {
        let grid_width = (world_width / cell_size).ceil() as usize + 1;
        let grid_height = (world_height / cell_size).ceil() as usize + 1;
        
        let mut cells = Vec::with_capacity(grid_width);
        for _ in 0..grid_width {
            let mut column = Vec::with_capacity(grid_height);
            for _ in 0..grid_height {
                column.push(SpatialCell { voronoi_points: Vec::new() });
            }
            cells.push(column);
        }
        
        SpatialGrid {
            cells,
            cell_size,
            grid_width,
            grid_height,
        }
    }
    
    fn add_point(&mut self, section_index: usize, point_index: usize, position: Vector2) {
        let grid_x = (position.x / self.cell_size).floor() as usize;
        let grid_y = (position.y / self.cell_size).floor() as usize;
        
        if grid_x < self.grid_width && grid_y < self.grid_height {
            self.cells[grid_x][grid_y].voronoi_points.push((section_index, point_index));
        }
    }
    
    fn get_nearby_points(&self, x: f32, y: f32, radius: f32) -> Vec<(usize, usize)> {
        let mut result = Vec::new();
        let grid_x = (x / self.cell_size).floor() as usize;
        let grid_y = (y / self.cell_size).floor() as usize;
        
        // Calculate cell radius based on query radius
        let cell_radius = (radius / self.cell_size).ceil() as usize;
        
        // Check cells in a square around the query point
        for dx in 0..=cell_radius*2 {
            let cx = if grid_x >= cell_radius {
                grid_x + dx - cell_radius
            } else {
                dx
            };
            
            if cx >= self.grid_width {
                continue;
            }
            
            for dy in 0..=cell_radius*2 {
                let cy = if grid_y >= cell_radius {
                    grid_y + dy - cell_radius
                } else {
                    dy
                };
                
                if cy >= self.grid_height {
                    continue;
                }
                
                // Add all points from this cell
                for &(section_index, point_index) in &self.cells[cx][cy].voronoi_points {
                    result.push((section_index, point_index));
                }
            }
        }
        
        result
    }
}


// TODO: this is and more structs is not implemented, find out why
// Structure to define a section with its associated biomes
struct BiomeSection {
    section_id: u8,
    possible_biomes: Vec<u8>,
    voronoi_points: Vec<VoronoiPoint>,
    point_density: f32, // Points per 1000x1000 world units
}

// Thread-safe versions of biome structures
#[derive(Clone)]
pub struct ThreadSafeBiomeData {
    world_width: f32,
    world_height: f32,
    seed: u32,
    pub blend_distance: i32,
    blend_noise_params: Option<NoiseParameters>, // Store blend noise config

    // Add reference to image data
    image_data: Vec<u8>,
    image_width: i32,
    image_height: i32,

    /// All Voronoi points from all sections, flattened into one list. Arc for cheap cloning.
    points: Arc<Vec<ThreadSafeVoronoiPoint>>,
    /// Grid containing indices into the `points` vector. Arc for cheap cloning.
    /// Structure: grid[grid_x][grid_y] -> Vec<point_index>
    spatial_grid_indices: Arc<Vec<Vec<Vec<usize>>>>,
    /// The size of each square cell in the spatial grid.
    grid_cell_size: f32,
    /// Width of the spatial grid in cells.
    grid_width: usize,
    /// Height of the spatial grid in cells.
    grid_height: usize,
}

#[derive(Clone)]
struct ThreadSafeBiomeSection {
    section_id: u8,
    possible_biomes: Vec<u8>,
    voronoi_points: Vec<ThreadSafeVoronoiPoint>,
}

#[derive(Clone)]
struct ThreadSafeVoronoiPoint {
    position: (f32, f32),
    biome_id: u8,
    section_id: u8,
}


// BiomeManager handles loading and accessing a bitmap that defines biome regions
#[derive(GodotClass)]
#[class(base=Node)]
pub struct BiomeManager {
    #[base]
    base: Base<Node>,

    initialization_state: BiomeManagerState,
    error_logger: Option<Arc<ErrorLogger>>,


    // Biome Mask Texture
    biome_image: Option<Gd<Image>>,
    mask_width: i32,
    mask_height: i32,
    
    // World Size
    world_width: f32,
    world_height: f32,
    
    // Performance Cache
    color_cache: Arc<RwLock<HashMap<String, Color>>>,
    section_cache: Arc<RwLock<HashMap<String, u8>>>,
    biome_cache: Arc<RwLock<HashMap<String, u8>>>,
    
    // Biome mask image path
    biome_mask_image_path: GString,
    noise_path: GString,
    
    // Biome configuration
    sections: Vec<BiomeSection>,
    blend_distance: i32,   // Distance over which biomes blend
    noise: Option<Gd<FastNoiseLite>>, // Noise for biome blending
    
    // Is the system initialized
    initialized: bool,
    seed: u32,
    
    // Random number generator for voronoi points
    rng: Gd<RandomNumberGenerator>,

    // Spatial partitioning grid
    spatial_grid: Option<SpatialGrid>,
}

#[godot_api]
impl INode for BiomeManager {
    fn init(base: Base<Node>) -> Self {
        let mut rng = RandomNumberGenerator::new_gd();
        rng.randomize();
        
        Self {
            base,
            initialization_state: BiomeManagerState::Uninitialized,
            error_logger: None,
            biome_image: None,
            mask_width: 0,
            mask_height: 0,
            world_width: 10000.0,
            world_height: 10000.0,
            color_cache: Arc::new(RwLock::new(HashMap::new())),
            section_cache: Arc::new(RwLock::new(HashMap::new())),
            biome_cache: Arc::new(RwLock::new(HashMap::new())),
            biome_mask_image_path: GString::from("res://textures/biomeMask_image.png"),
            noise_path: GString::from("res://project/terrain/noise/blendNoise.tres"),
            sections: Vec::new(),
            blend_distance: 200,
            noise: None,
            initialized: false,
            seed: 12345,
            rng,
            spatial_grid: None,
        }
    }

    // Initialize
    fn ready(&mut self) {
        let error_logger = Arc::new(ErrorLogger::new(100)); // 100 max log entries
        self.error_logger = Some(error_logger.clone());
        
        godot_print!("BiomeManager: Created but waiting for explicit initialization");
        // Don't auto-initialize - wait for TerrainInitializer to initialize
    }
}

#[godot_api]
impl BiomeManager {

    //initialize_explicitly
    #[func]
    pub fn initialize(&mut self, world_width: f32, world_height: f32, seed: u32) -> bool {
        // Prevent re-initialization
        if self.initialization_state != BiomeManagerState::Uninitialized {
            godot_print!("BiomeManager: Already initialized");
            return false;
        }

        // Set initialization state
        self.initialization_state = BiomeManagerState::Initializing;
        
        // Update world parameters
        self.world_width = world_width;
        self.world_height = world_height;
        self.seed = seed;
        
        // Initialize resource manager
        resource_manager::init();
        
        // Perform initialization steps with error handling
        let result = self.perform_initialization();
        
        // Update final state
        match result {
            Ok(_) => {
                self.initialization_state = BiomeManagerState::Initialized;
                godot_print!("BiomeManager: Initialization complete");
                true
            },
            Err(e) => {
                self.initialization_state = BiomeManagerState::Error;
                
                // Log the error
                if let Some(logger) = &self.error_logger {
                    logger.log_error(
                        "BiomeManager",
                        &format!("Initialization failed: {}", e),
                        ErrorSeverity::Critical,
                        None
                    );
                }
                
                godot_error!("BiomeManager initialization failed: {}", e);
                false
            }
        }
    }

    // Internal initialization method
    fn perform_initialization(&mut self) -> Result<(), String> {
        // Load mask image
        self.load_mask(self.biome_mask_image_path.clone())
            .map_err(|e| format!("Mask loading failed: {}", e))?;
        
        // Load noise
        self.load_noise(self.noise_path.clone())
            .map_err(|e| format!("Noise loading failed: {}", e))?;
        
        // Setup biome sections
        self.setup_biome_sections();
        
        // Initialize Voronoi points
        self.initialize_voronoi_points();
        
        // Validate initialization
        if !self.validate_initialization() {
            return Err("Incomplete initialization".to_string());
        }
        
        Ok(())
    }

    // Validate initialization
    fn validate_initialization(&self) -> bool {
        self.noise.is_some() && 
        !self.sections.is_empty() && 
        self.biome_image.is_some() && 
        self.spatial_grid.is_some()
    }

    // Modify existing load_mask and load_noise to return bool
    fn load_mask(&mut self, path: GString) -> Result<(), String> {
        godot_print!("BiomeManager: Loading biome mask from: {}", path);
        
        // Load texture
        let texture = resource_manager::load_and_cast::<Texture2D>(path.clone())
            .ok_or_else(|| format!("Failed to load texture from path: {}", path))?;
        
        let image = texture.get_image()
            .ok_or_else(|| "Failed to get image from texture".to_string())?;
        
        self.biome_image = Some(image.clone());
        let width = image.get_width();
        let height = image.get_height();
        
        godot_print!("Biome image loaded: {}x{}", width, height);
        
        // Warn about small images
        if width < 100 || height < 100 {
            godot_print!("WARNING: Biome mask image is very small ({}x{})", width, height);
        }
        
        Ok(())
    }
  

    fn load_noise(&mut self, path: GString) -> Result<(), String> {
        match resource_manager::load_and_cast::<FastNoiseLite>(path.clone()) {
            Some(noise) => {
                self.noise = Some(noise);
                godot_print!("Loaded FastNoiseLite from: {}", path);
                Ok(())
            },
            None => {
                // Create fallback noise
                let mut noise = FastNoiseLite::new_gd();
                noise.set_seed(self.seed as i32);
                noise.set_frequency(0.01);
                noise.set_fractal_octaves(4);
                self.noise = Some(noise);
                
                Err(format!("Failed to load noise from: {}, using fallback", path))
            }
        }
    }

    // Enhanced initialization check
    pub fn is_fully_initialized(&self) -> bool {
        self.noise.is_some() && 
        !self.sections.is_empty() && 
        self.biome_image.is_some() && 
        self.spatial_grid.is_some()
    }
    
    #[func]
    pub fn get_initialization_state(&self) -> i32 {
        match self.initialization_state {
            BiomeManagerState::Uninitialized => 0,
            BiomeManagerState::Initializing => 1,
            BiomeManagerState::Initialized => 2,
            BiomeManagerState::Error => -1,
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
            possible_biomes: vec![1, 2],  // sand, Coral
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
        
        godot_print!("BiomeManager: Biome sections initialized");
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
        godot_print!("BiomeManager: Voronoi points initialized for all sections ({} total sections)", self.sections.len());
        
        // Build the spatial grid
        self.build_spatial_grid();
    }
    
    // Build the spatial partitioning grid
    fn build_spatial_grid(&mut self) {
        // Create a new spatial grid with cell size of 200 (adjust as needed)
        let mut grid = SpatialGrid::new(self.world_width, self.world_height, 200.0);
        
        // Add all Voronoi points to the grid
        for (section_index, section) in self.sections.iter().enumerate() {
            for (point_index, point) in section.voronoi_points.iter().enumerate() {
                grid.add_point(section_index, point_index, point.position);
            }
        }
        
        self.spatial_grid = Some(grid);
        godot_print!("Spatial grid built for efficient point lookup");
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
        
        // Use Cache for Performance - Read lock
        {
            let cache = self.color_cache.read().expect("Failed to acquire read lock on color cache");
            if let Some(color) = cache.get(&key) {
                return *color;
            }
        }
    
        // Get pixel color and cache it
        match &self.biome_image {
            Some(image) => {
                let color = image.get_pixel(coords.x, coords.y);
                
                // Thread-safe cache insertion - Write lock
                {
                    let mut cache = self.color_cache.write().expect("Failed to acquire write lock on color cache");
                    cache.insert(key, color);
                }
                
                color
            },
            _none => Color::from_rgba(1.0, 0.0, 1.0, 1.0) // Magenta as error color
        }
    }
    
    // Get the section ID from color
    #[func]
    pub fn get_section_id(&self, world_x: f32, world_y: f32) -> u8 {
        // If we have a biome image, use it for section detection
        if let Some(ref img) = self.biome_image {
            let coords = self.world_to_mask_coords(world_x, world_y);
            
            // Get the pixel color
            let color = img.get_pixel(coords.x, coords.y);
            
            // Map color to section ID
            if color.r > 0.7 && color.g < 0.3 && color.b < 0.3 {
                return 1; // Red section
            } else if color.r < 0.3 && color.g > 0.7 && color.b < 0.3 {
                return 2; // Green section
            } else if color.r < 0.3 && color.g < 0.3 && color.b > 0.7 {
                return 3; // Blue section
            } else if color.r > 0.7 && color.g > 0.7 && color.b < 0.3 {
                return 4; // Yellow section
            } else if color.r > 0.7 && color.g < 0.3 && color.b > 0.7 {
                return 5; // Purple section  
            } else if color.r < 0.3 && color.g > 0.7 && color.b > 0.7 {
                return 6; // Cyan section
            } else {
                // Handle mixed colors
                let max_component = f32::max(f32::max(color.r, color.g), color.b);

                if max_component < 0.1 {
                    return 0; // Very dark: undefined section
                } else if color.r >= color.g && color.r >= color.b {
                    return 1; // Red dominant: Section 1
                } else if color.g >= color.r && color.g >= color.b {
                    return 2; // Green dominant: Section 2
                } else {
                    return 3; // Blue dominant: Section 3
                }
            }
        }

        // Fallback to original behavior if no image data or other error
        let relative_x = world_x / self.world_width;
        let relative_y = world_y / self.world_height;

        if relative_x < 0.33 {
            1 // Section 1
        } else if relative_x < 0.66 {
            2 // Section 2
        } else {
            3 // Section 3
        }
    }
    
    // Get the biome ID at a specific world position
    #[func]
    pub fn get_biome_id(&mut self, world_x: f32, world_y: f32) -> u8 {
        if !self.initialized {
            return 0;
        }
        
        let cache_key = format!("biome_{}_{}", (world_x * 0.1) as i32, (world_y * 0.1) as i32);
        
        // Check cache first with read lock
        {
            let cache = self.biome_cache.read().expect("Failed to acquire read lock on biome cache");
            if let Some(&biome_id) = cache.get(&cache_key) {
                return biome_id;
            }
        }
        
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
                
                // Lock for cache write
                {
                    let mut cache = self.biome_cache.write().expect("Failed to acquire write lock on biome cache");
                    cache.insert(cache_key, *default_biome);
                }
                
                return *default_biome;
            }
            
            // Create a position vector
            let pos = Vector2::new(world_x, world_y);
            
            // Use spatial grid for efficient lookup if available
            if let Some(grid) = &self.spatial_grid {
                let nearby_indices = grid.get_nearby_points(world_x, world_y, self.blend_distance as f32 * 2.0);
                
                // Filter to only points in the current section
                let section_points: Vec<_> = nearby_indices.iter()
                    .filter(|(idx, _)| *idx == section_idx)
                    .collect();
                
                if !section_points.is_empty() {
                    // Calculate distances
                    let mut distances: Vec<(f32, u8)> = Vec::new();
                    
                    for &(_, point_idx) in &section_points {
                        let point = &section.voronoi_points[*point_idx];
                        let distance = pos.distance_to(point.position);
                        distances.push((distance, point.biome_id));
                    }
                    
                    // Sort by distance
                    distances.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
                    
                    // Check if we need to blend between two closest points
                    if distances.len() >= 2 {
                        let (dist1, biome1) = distances[0];
                        let (dist2, biome2) = distances[1];
                        
                        // If the points are close enough, blend between them
                        if (dist2 - dist1) < self.blend_distance as f32 {
                            // Calculate blend factor with noise influence for natural borders
                            let noise_val = if let Some(ref noise) = self.noise {
                                // Use Godot's FastNoiseLite
                                noise.get_noise_2d(world_x * 0.01, world_y * 0.01) * 0.5 + 0.5
                            } else {
                                // Fallback if noise is not available
                                0.5
                            };
                            
                            let blend_factor = ((dist2 - dist1) / self.blend_distance as f32).min(1.0);
                            let adjusted_blend = blend_factor * (1.0 - noise_val * 0.3); // Noise influence
                            
                            // Choose biome based on blend factor
                            let selected_biome = if self.rng.randf() > adjusted_blend {
                                biome1
                            } else {
                                biome2
                            };
                            
                            // Write to cache
                            {
                                let mut cache = self.biome_cache.write().expect("Failed to acquire write lock on biome cache");
                                cache.insert(cache_key, selected_biome);
                            }
                            
                            return selected_biome;
                        }
                    }
                    
                    // If no blending needed, return closest
                    if !distances.is_empty() {
                        let biome_id = distances[0].1;
                        {
                            let mut cache = self.biome_cache.write().expect("Failed to acquire write lock on biome cache");
                            cache.insert(cache_key, biome_id);
                        }
                        return biome_id;
                    }
                }
            }
            
            // Fallback to original algorithm if spatial grid not available or no nearby points found
            let mut distances: Vec<(f32, &VoronoiPoint)> = section.voronoi_points.iter()
                .map(|point| {
                    let distance = pos.distance_to(point.position);
                    (distance, point)
                })
                .collect();
            
            // Sort by distance
            distances.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
            
            // If no blending or only one point, return the closest
            if !distances.is_empty() {
                let biome_id = distances[0].1.biome_id;
                {  
                    let mut cache = self.biome_cache.write().expect("Failed to acquire write lock on biome cache");
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
        // Thread-safe cache clearing with write locks
        self.color_cache.write().expect("Failed to acquire write lock on color cache").clear();
        self.section_cache.write().expect("Failed to acquire write lock on section cache").clear();
        self.biome_cache.write().expect("Failed to acquire write lock on biome cache").clear();
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
        self.blend_distance = distance as i32;
        self.clear_cache();

        // Notify ChunkManager if possible
        self.notify_data_change();
    }
    
    // Apply generation rules to the biome manager
    #[func]
    pub fn apply_generation_rules(&mut self, rules_dict: Dictionary) -> VariantArray {
        // Convert Dictionary to GenerationRules
        let mut validated_rules = GenerationRules::from_dictionary(&rules_dict);
        let warnings = validated_rules.validate_and_fix();
        
        // Apply the validated rules
        self.blend_distance = validated_rules.biome_blend_distance as i32;
        
        // Update noise settings if available
        if let Some(ref mut noise) = self.noise {
            noise.set_fractal_octaves(validated_rules.terrain_octaves as i32);
            noise.set_frequency(1.0 / validated_rules.terrain_scale);
            // Set other noise parameters as needed
        }
        
        // Clear caches since we've changed parameters
        self.clear_cache();
        
        // Convert warnings to VariantArray for GDScript
        let mut result = VariantArray::new();
        for warning in warnings {
            result.push(&warning.to_variant());
        }
        
        result
    }
    
    // Helper method to notify ChunkManager
    fn notify_data_change(&self) {
        godot_print!("BiomeManager: Data changed (notification)");
        // REMOVED THE CALL TO CHUNK MANAGER UPDATE HERE

        // Optional: If you want to use signals (more complex setup needed)
        // self.base.emit_signal("biome_data_changed".into(), &[]);

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
        match self.load_noise(path) {
            Ok(_) => true,
            Err(e) => {
                godot_error!("Failed to set noise resource: {}", e);
                false
            }
        }
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
    // Update only changed properties
    pub fn update_from_biome_manager(&mut self, biome_mgr: &BiomeManager, noise_manager: &NoiseManager) {
        let mut rebuild_grid = false;
        if self.seed != biome_mgr.seed { self.seed = biome_mgr.seed; rebuild_grid = true; }
        if self.world_width != biome_mgr.world_width || self.world_height != biome_mgr.world_height {
            self.world_width = biome_mgr.world_width;
            self.world_height = biome_mgr.world_height;
            rebuild_grid = true;
        }

        if rebuild_grid {
            println!("ThreadSafeBiomeData: Rebuilding points and spatial grid due to changes.");

            // Declare temporary variables to hold the new data
            let new_points_arc: Arc<Vec<ThreadSafeVoronoiPoint>>;
            let new_spatial_grid_indices_arc: Arc<Vec<Vec<Vec<usize>>>>;
            let new_grid_cell_size: f32;
            let new_grid_width: usize;
            let new_grid_height: usize;

            if let Some(grid) = &biome_mgr.spatial_grid {
                let mut all_points = Vec::new();
                for section in &biome_mgr.sections {
                    for point in &section.voronoi_points {
                         all_points.push(ThreadSafeVoronoiPoint {
                             position: (point.position.x, point.position.y),
                             biome_id: point.biome_id,
                             section_id: section.section_id,
                         });
                    }
                }
                new_points_arc = Arc::new(all_points);

                new_grid_cell_size = grid.cell_size;
                new_grid_width = grid.grid_width;
                new_grid_height = grid.grid_height;

                let mut grid_indices = vec![vec![Vec::new(); new_grid_height]; new_grid_width];
                for (point_index, point) in new_points_arc.iter().enumerate() {
                    let grid_x = (point.position.0 / new_grid_cell_size).floor() as usize;
                    let grid_y = (point.position.1 / new_grid_cell_size).floor() as usize;
                    if grid_x < new_grid_width && grid_y < new_grid_height {
                        grid_indices[grid_x][grid_y].push(point_index);
                    }
                }
                new_spatial_grid_indices_arc = Arc::new(grid_indices);
            } else {
                 godot_error!("ThreadSafeBiomeData: BiomeManager spatial_grid is None during update!");
                 new_points_arc = Arc::new(Vec::new());
                 new_spatial_grid_indices_arc = Arc::new(Vec::new());
                 new_grid_cell_size = 1.0; // Default
                 new_grid_width = 0;
                 new_grid_height = 0;
            }

            // Assign the newly built data to self fields
            self.points = new_points_arc;
            self.spatial_grid_indices = new_spatial_grid_indices_arc;
            self.grid_cell_size = new_grid_cell_size;
            self.grid_width = new_grid_width;
            self.grid_height = new_grid_height;
        }

        // Update other fields (image data, blend distance, noise params) as before
        if let Some(ref img) = biome_mgr.biome_image {
            let image_width = img.get_width();
            let image_height = img.get_height();
            if self.image_width != image_width || self.image_height != image_height {
                self.image_width = image_width;
                self.image_height = image_height;
                self.image_data = img.get_data().to_vec();
            }
        }
        self.blend_distance = biome_mgr.blend_distance;
        let current_blend_params = noise_manager.get_parameters("biome_blend");
        if self.blend_noise_params != current_blend_params {
            godot_print!("ThreadSafeBiomeData: Updating blend noise parameters."); // Use println! if preferred
            self.blend_noise_params = current_blend_params;
            if self.blend_noise_params.is_none() {
                godot_warn!("ThreadSafeBiomeData: 'biome_blend' noise parameters not found during update. Biome blending noise disabled."); // Use eprintln! if preferred
            }
        }
    }
    

    pub fn from_biome_manager(biome_mgr: &BiomeManager, noise_manager: &NoiseManager) -> Self {

        // --- Declare variables in the outer scope ---
        let points_arc: Arc<Vec<ThreadSafeVoronoiPoint>>;
        let spatial_grid_indices_arc: Arc<Vec<Vec<Vec<usize>>>>;
        let grid_cell_size: f32;
        let grid_width: usize;
        let grid_height: usize;
        // --- End Declaration ---

        if let Some(grid) = &biome_mgr.spatial_grid {
            let mut all_points = Vec::new();
            for section in &biome_mgr.sections {
                for point in &section.voronoi_points {
                    all_points.push(ThreadSafeVoronoiPoint {
                        position: (point.position.x, point.position.y),
                        biome_id: point.biome_id,
                        section_id: section.section_id,
                    });
                }
            }
            // Assign to the outer scope variable
            points_arc = Arc::new(all_points);

            // Assign to outer scope variables
            grid_cell_size = grid.cell_size;
            grid_width = grid.grid_width;
            grid_height = grid.grid_height;

            let mut grid_indices: Vec<Vec<Vec<usize>>> = vec![vec![Vec::new(); grid_height]; grid_width];
            for (point_index, point) in points_arc.iter().enumerate() {
                let grid_x = (point.position.0 / grid_cell_size).floor() as usize;
                let grid_y = (point.position.1 / grid_cell_size).floor() as usize;
                if grid_x < grid_width && grid_y < grid_height {
                    grid_indices[grid_x][grid_y].push(point_index);
                }
            }
            // Assign to the outer scope variable
            spatial_grid_indices_arc = Arc::new(grid_indices);

        } else {
            // Assign default values to outer scope variables in the error case
            godot_error!("ThreadSafeBiomeData: BiomeManager spatial_grid is None during creation!");
            points_arc = Arc::new(Vec::new());
            spatial_grid_indices_arc = Arc::new(Vec::new());
            grid_cell_size = 1.0; // Avoid division by zero later, use a default
            grid_width = 0;
            grid_height = 0;
        }

        // Copy image data (remains the same)
        let mut image_data = Vec::new();
        let mut image_width = 0;
        let mut image_height = 0;
        if let Some(ref img) = biome_mgr.biome_image {
            image_width = img.get_width();
            image_height = img.get_height();
            image_data = img.get_data().to_vec();
        }

        // Fetch Blend Noise Parameters (remains the same)
        let blend_noise_params = noise_manager.get_parameters("biome_blend");
        if blend_noise_params.is_none() {
            godot_warn!("ThreadSafeBiomeData: Could not find 'biome_blend' noise parameters in NoiseManager. Biome blending noise disabled.");
        }


        // Construct the struct - variables are now correctly in scope
        ThreadSafeBiomeData {
            world_width: biome_mgr.world_width,
            world_height: biome_mgr.world_height,
            seed: biome_mgr.seed,
            blend_distance: biome_mgr.blend_distance,
            blend_noise_params,
            image_data,
            image_width,
            image_height,
            points: points_arc, // Use outer variable
            spatial_grid_indices: spatial_grid_indices_arc, // Use outer variable
            grid_cell_size, // Use outer variable
            grid_width, // Use outer variable
            grid_height, // Use outer variable
        }
    }

    fn create_blend_noise_fn(params: &NoiseParameters) -> Option<Box<dyn noise::NoiseFn<f64, 2> + Send + Sync>> {
        // Use noise-rs based on params. Adjust noise type as needed for blending.
        // Example using simple Perlin:
        let noise_fn = Perlin::new(params.seed)
            .set_seed(params.seed); // Use the seed from params
   
        // Wrap with ScalePoint for frequency ONLY IF params represent simple noise.
        // If params include fractal settings, use Fbm, RidgedMulti etc. like in ChunkManager.
        // Let's assume simple Perlin scaled by frequency for blend noise for now.
        // You might need to adjust this based on your actual blend noise settings.
        let scaled_noise = noise::ScalePoint::new(noise_fn)
            .set_scale(params.frequency as f64); // Apply frequency
   
        Some(Box::new(scaled_noise))
   
        // If using fractals for blending noise:
        /*
        match params.fractal_type {
            // ... cases for Fbm<Perlin>, RidgedMulti<Perlin>, etc. ...
            // Use params.frequency, params.fractal_octaves, etc.
            RustFractalType::None => {
                // Just Perlin scaled by frequency
                let noise_fn = Perlin::new(params.seed);
                let scaled_noise = noise::ScalePoint::new(noise_fn)
                    .set_scale(params.frequency as f64);
                Some(Box::new(scaled_noise))
            }
            _ => { // Handle FBM, Ridged etc.
               // ... create Fbm<Perlin>::new(params.seed).set_frequency(...) etc. ...
            }
        }
        */
    }

    // Helper for deterministic random value [0.0, 1.0) using ChaCha8Rng
    fn get_deterministic_random(&self, world_x: f32, world_y: f32) -> f32 {
        // Combine seed and coordinates for a unique seed per position
        // Using XOR and to_bits for a simple combination
        let pos_hash_low = world_x.to_bits() ^ world_y.to_bits();
        let seed64 = (self.seed as u64) << 32 | (pos_hash_low as u64);

        // Create a ChaCha8Rng seeded with this value
        let mut rng = ChaCha8Rng::seed_from_u64(seed64);

        // Generate a random f32 in [0.0, 1.0)
        rng.r#gen::<f32>()
    }
    
    // Get section ID based on world coordinates
    pub fn get_section_id(&self, world_x: f32, world_y: f32) -> u8 {
        // Use image data if available
        if !self.image_data.is_empty() && self.image_width > 0 && self.image_height > 0 {
            let mask_x = ((world_x / self.world_width) * self.image_width as f32) as i32;
            let mask_y = ((world_y / self.world_height) * self.image_height as f32) as i32;
  
            let x = mask_x.clamp(0, self.image_width - 1) as usize;
            let y = mask_y.clamp(0, self.image_height - 1) as usize;
  
            // Get the pixel data (RGBA format)
            let idx = (y * self.image_width as usize + x) * 4;
  
            if idx + 2 < self.image_data.len() {
                let r = self.image_data[idx] as f32 / 255.0;
                let g = self.image_data[idx + 1] as f32 / 255.0;
                let b = self.image_data[idx + 2] as f32 / 255.0;
  
                // Use the same section detection logic as BiomeManager
                if r > 0.7 && g < 0.3 && b < 0.3 {
                    return 1; // Red section
                } else if r < 0.3 && g > 0.7 && b < 0.3 {
                    return 2; // Green section
                } else if r < 0.3 && g < 0.3 && b > 0.7 {
                    return 3; // Blue section
                } else {
                    // For mixed colors, use dominant component
                    let max_component = f32::max(f32::max(r, g), b);
  
                    if max_component < 0.1 {
                        return 0; // Very dark: undefined section
                    } else if r >= g && r >= b {
                        return 1; // Red dominant: Section 1
                    } else if g >= r && g >= b {
                        return 2; // Green dominant: Section 2
                    } else {
                        return 3; // Blue dominant: Section 3
                    }
                }
            }
        }
  
        // Fallback to simpler logic
        // (same as current implementation)
        let relative_x = world_x / self.world_width;
        let relative_y = world_y / self.world_height;
  
        if relative_x < 0.33 {
            1 // Section 1
        } else if relative_x < 0.66 {
            2 // Section 2
        } else {
            3 // Section 3
        }
    }
  
    // Get biome ID at world coordinates
    pub fn get_biome_id(&self, world_x: f32, world_y: f32) -> u8 {
        // Determine the target section for the query location
        let target_section_id = self.get_section_id(world_x, world_y);
        if target_section_id == 0 { return 0; } // Undefined section

        if self.points.is_empty() || self.grid_width == 0 || self.grid_height == 0 {
            // Handle cases with no points or invalid grid
            // Maybe return a default biome for the section? Needs design decision.
            godot_warn!("get_biome_id called with no points or invalid grid for section {}.", target_section_id);
            return 0; // Or find the section definition and return its first possible biome
        }

        // --- Use Spatial Grid ---
        let mut closest1: Option<(usize, f32)> = None; // (point_index, dist_sq)
        let mut closest2: Option<(usize, f32)> = None; // (point_index, dist_sq)
        let pos = (world_x, world_y);

        // Calculate query point's grid cell
        let grid_x = (world_x / self.grid_cell_size).floor() as usize;
        let grid_y = (world_y / self.grid_cell_size).floor() as usize;

        // Determine search radius in cells (e.g., cover blend distance + 1 cell buffer)
        // Search slightly larger area than blend distance to ensure we find the correct neighbours
        let search_radius_world = self.blend_distance as f32 * 1.5; // Adjust multiplier as needed
        let cell_radius = (search_radius_world / self.grid_cell_size).ceil() as usize + 1;

        // Iterate through nearby grid cells
        let min_cx = grid_x.saturating_sub(cell_radius);
        let max_cx = (grid_x + cell_radius).min(self.grid_width - 1);
        let min_cy = grid_y.saturating_sub(cell_radius);
        let max_cy = (grid_y + cell_radius).min(self.grid_height - 1);

        for cx in min_cx..=max_cx {
            for cy in min_cy..=max_cy {
                // Access the list of point indices for this cell
                // Cloning Arc is cheap, direct indexing is fine for read access
                let point_indices_in_cell = &self.spatial_grid_indices[cx][cy];

                for point_index in point_indices_in_cell {
                    let candidate_point = &self.points[*point_index];

                    // *** Filter by target section ID ***
                    if candidate_point.section_id != target_section_id {
                        continue; // Skip points not in the target section
                    }

                    // Calculate squared distance (cheaper than sqrt initially)
                    let dx = pos.0 - candidate_point.position.0;
                    let dy = pos.1 - candidate_point.position.1;
                    let dist_sq = dx * dx + dy * dy;

                    // Update closest points found *so far* within the target section
                    if closest1.is_none() || dist_sq < closest1.unwrap().1 {
                        closest2 = closest1;
                        closest1 = Some((*point_index, dist_sq));
                    } else if closest2.is_none() || dist_sq < closest2.unwrap().1 {
                         // Avoid selecting the same point twice
                         if closest1.unwrap().0 != *point_index {
                              closest2 = Some((*point_index, dist_sq));
                         }
                    }
                }
            }
        }

        // --- Process the closest points found ---
        if let (Some((p1_idx, dist1_sq)), Some((p2_idx, dist2_sq))) = (closest1, closest2) {
            // Calculate real distances now for blending check
            let dist1 = dist1_sq.sqrt();
            let dist2 = dist2_sq.sqrt();
            let blend_dist_f32 = self.blend_distance as f32;

            // Use the same blending logic as before
            if dist1 < blend_dist_f32 && (dist2 - dist1) < blend_dist_f32 {
                // Blend noise calculation (using create_blend_noise_fn for now)
                let mut noise_influence = 0.0;
                if let Some(ref params) = self.blend_noise_params {
                    if let Some(noise_fn) = Self::create_blend_noise_fn(params) { // Will be replaced in Step 4
                         let noise_val = noise_fn.get([world_x as f64, world_y as f64]);
                         let normalized_noise = (noise_val as f32 * 0.5) + 0.5;
                         let noise_factor = 0.3;
                         noise_influence = (normalized_noise - 0.5) * noise_factor;
                    }
                }
                // Blend factor calculation
                let blend_factor = (dist1 / blend_dist_f32).clamp(0.0, 1.0);
                let adjusted_blend = (blend_factor + noise_influence).clamp(0.0, 1.0);

                // Deterministic random choice
                let rand_val = self.get_deterministic_random(world_x, world_y);
                let p1_biome_id = self.points[p1_idx].biome_id;
                let p2_biome_id = self.points[p2_idx].biome_id;

                return if rand_val < adjusted_blend { p2_biome_id } else { p1_biome_id };

            } else {
                // No blending needed, return closest point's biome
                return self.points[p1_idx].biome_id;
            }

        } else if let Some((p1_idx, _)) = closest1 {
            // Only one relevant point found in the search area
            return self.points[p1_idx].biome_id;
        }

        // Fallback if no relevant points found in search radius (should be rare if radius is sufficient)
        // Find the absolute closest point in the section using a linear scan as a last resort?
        // Or return a default for the section. Let's return 0 for now.
        godot_warn!("get_biome_id: No Voronoi points found in spatial grid search radius for section {}.", target_section_id);
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

    pub fn seed(&self) -> u32 {
        self.seed
    }
    
    pub fn blend_distance(&self) -> i32 {
        self.blend_distance
    }

}
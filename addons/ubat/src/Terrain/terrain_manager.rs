pub mod terrain {
    /// Manages all chunks in the world
    pub struct ChunkManager {
        active_chunks: HashMap<Vector2i, Chunk>,
        chunk_cache: LruCache<Vector2i, ChunkData>,
        chunk_generator: ChunkGenerator,
        network_role: NetworkRole,
    }

    impl ChunkManager {
        /// Loads or generates a chunk at the specified position
        pub fn get_or_load_chunk(&mut self, position: Vector2i) -> Option<&Chunk> {
            // Retrieves from active chunks, cache, or generates new
        }

        /// Update loop for host simulation
        pub fn host_update(&mut self, delta: f32) {
            // Process chunk loading/unloading based on players
            // Run simulation for active chunks
            // Send updates to clients
        }

        /// Update loop for client visualization and prediction
        pub fn client_update(&mut self, delta: f32) {
            // Apply chunk updates from host
            // Request chunks needed for rendering
            // Handle local modifications
        }

        /// Unloads a chunk, saving its state appropriately
        pub fn unload_chunk(&mut self, position: Vector2i) {
            // Saves state and removes from active chunks
        }
    }

    /// Represents a loaded chunk in the world
    pub struct Chunk {
        position: Vector2i,
        mesh_instance: Option<Gd<MeshInstance3D>>,
        entities: Vec<EntityRef>,
        modifications: HashMap<Vector3i, BlockModification>,
        is_dirty: bool,
    }

    /// Data-only representation of a chunk for serialization
    pub struct ChunkData {
        position: Vector2i,
        terrain_seed: i64,
        height_data: Option<Vec<f32>>,
        modifications: HashMap<Vector3i, BlockModification>,
        persistent_state: Dictionary,
    }

    /// Handles procedural generation of terrain
    pub struct ChunkGenerator {
        biome_system: BiomeSystem,
        noise_cache: HashMap<String, NoiseInstance>,
        shader_resources: ShaderResources,
    }

    impl ChunkGenerator {
        /// Generate mesh data for a chunk
        pub fn generate_mesh(&self, position: Vector2i, lod_level: u32) -> MeshData {
            // Creates optimized mesh based on position and LOD
        }

        /// Generate biome data for a chunk
        pub fn generate_biome_data(&self, position: Vector2i) -> BiomeData {
            // Determines biome distribution within chunk
        }

        /// Generate height data for a chunk
        pub fn generate_height_data(&self, position: Vector2i, biome_data: &BiomeData) -> HeightData {
            // Calculates height values for chunk vertices
        }

        /// Generate collision shape for a chunk
        pub fn generate_collision(&self, height_data: &HeightData) -> CollisionShape {
            // Creates collision data from height field
        }
    }

    /// Manages biome types, distribution and transitions
    pub struct BiomeSystem {
        biome_types: HashMap<String, BiomeType>,
        transition_rules: HashMap<(String, String), TransitionRule>,
        biome_mask: Option<BiomeMask>,
    }

    impl BiomeSystem {
        /// Determine which biome(s) are at a world position
        pub fn get_biome_weights_at(&self, position: Vector2) -> HashMap<String, f32> {
            // Returns map of biome -> weight at position
        }

        /// Calculate biome distribution for a chunk
        pub fn calculate_chunk_biomes(&self, chunk_pos: Vector2i) -> BiomeDistribution {
            // Efficiently determines biome distribution
        }

        /// Get noise parameters for a specific biome
        pub fn get_biome_noise_params(&self, biome_name: &str) -> NoiseParameters {
            // Returns noise settings for height generation
        }

        /// Calculate blend weights between biomes
        pub fn calculate_blend_weights(&self, 
            position: Vector2, 
            biomes: &[&str]
        ) -> Vec<f32> {
            // Determines how to blend between biomes
        }
    }
}
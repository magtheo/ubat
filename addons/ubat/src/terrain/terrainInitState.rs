#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainInitializationState {
    Uninitialized,
    ConfigLoaded,
    BiomeInitialized,
    ChunkManagerInitialized,
    Ready,
    Error
}
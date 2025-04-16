// src/terrain/noise/noise_parameters.rs
use godot::prelude::*;

// --- Rust equivalents for Godot Enums ---
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustNoiseType { Value, ValueCubic, Perlin, Cellular, Simplex, SimplexSmooth }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustFractalType { None, Fbm, Ridged, PingPong }
// Add other enums (CellularDistanceFunction, etc.) if needed

// --- Main Parameter Struct ---
#[derive(Debug, Clone)]
pub struct NoiseParameters {
    pub seed: i32,
    pub frequency: f32,
    pub noise_type: RustNoiseType,
    pub offset: (f32, f32, f32),
    pub fractal_type: RustFractalType,
    pub fractal_octaves: i32,
    pub fractal_lacunarity: f32,
    pub fractal_gain: f32,
    pub fractal_weighted_strength: f32,
    pub fractal_ping_pong_strength: f32,
    // Add other extracted fields...
}

// --- Mapping Functions ---
// (These are now correctly defined in the same file/module as NoiseParameters)
pub fn map_godot_noise_type(godot_enum: godot::classes::fast_noise_lite::NoiseType) -> RustNoiseType {
    // **FIXED:** Use full path for enum variants in match arms
    match godot_enum {
        godot::classes::fast_noise_lite::NoiseType::VALUE => RustNoiseType::Value,
        godot::classes::fast_noise_lite::NoiseType::VALUE_CUBIC => RustNoiseType::ValueCubic,
        godot::classes::fast_noise_lite::NoiseType::PERLIN => RustNoiseType::Perlin,
        godot::classes::fast_noise_lite::NoiseType::CELLULAR => RustNoiseType::Cellular,
        godot::classes::fast_noise_lite::NoiseType::SIMPLEX => RustNoiseType::Simplex,
        godot::classes::fast_noise_lite::NoiseType::SIMPLEX_SMOOTH => RustNoiseType::SimplexSmooth,
        _ => {
            // Use godot_warn! only if this function is called from the main thread,
            // otherwise consider logging the warning differently or removing it.
            // For simplicity, let's remove the warn here as it's mainly for parameter setup.
            // println!("Warning: Unsupported Godot NoiseType: {:?}. Falling back to Perlin.", godot_enum);
            RustNoiseType::Perlin
        }
    }
}

pub fn map_godot_fractal_type(godot_enum: godot::classes::fast_noise_lite::FractalType) -> RustFractalType {
     // **FIXED:** Use full path for enum variants in match arms
     match godot_enum {
        godot::classes::fast_noise_lite::FractalType::NONE => RustFractalType::None,
        godot::classes::fast_noise_lite::FractalType::FBM => RustFractalType::Fbm,
        godot::classes::fast_noise_lite::FractalType::RIDGED => RustFractalType::Ridged,
        godot::classes::fast_noise_lite::FractalType::PING_PONG => RustFractalType::PingPong,
        _ => {
            // println!("Warning: Unsupported Godot FractalType: {:?}. Falling back to None.", godot_enum);
            RustFractalType::None
        }
     }
}

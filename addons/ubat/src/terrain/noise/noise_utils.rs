// e.g., src/terrain/noise/noise_utils.rs
use crate::terrain::noise::noise_parameters::{NoiseParameters, RustFractalType};
use noise::{NoiseFn, Perlin, Fbm, Billow, RidgedMulti, ScalePoint, MultiFractal}; // Ensure imports

// Helper function to create a noise-rs boxed trait object from parameters
pub fn create_noise_function_from_params(
    params: &NoiseParameters
) -> Box<dyn NoiseFn<f64, 2> + Send + Sync> {
    // Use Perlin as the base, adjust if other base types are needed via params.noise_type
    let base_noise_generator = Perlin::new(params.seed); // Pass seed directly

    let final_noise: Box<dyn NoiseFn<f64, 2> + Send + Sync> = match params.fractal_type {
        RustFractalType::Fbm => {
            Box::new(Fbm::<Perlin>::new(params.seed) // Use seed here too
                .set_frequency(params.frequency as f64)
                .set_octaves(params.fractal_octaves as usize)
                .set_lacunarity(params.fractal_lacunarity as f64)
                .set_persistence(params.fractal_gain as f64)) // Gain is Persistence in noise-rs Fbm
        }
        RustFractalType::Ridged => {
            Box::new(RidgedMulti::<Perlin>::new(params.seed)
                .set_frequency(params.frequency as f64)
                .set_octaves(params.fractal_octaves as usize)
                .set_lacunarity(params.fractal_lacunarity as f64))
                // Note: noise-rs RidgedMulti doesn't have direct gain/weighted strength params
                // You might need custom implementations or different fractal types if those are critical
        }
        RustFractalType::PingPong => {
             // PingPong often maps better to Billow noise in noise-rs
             // Adjust persistence/gain mapping as needed. fractal_gain often maps to persistence.
            Box::new(Billow::<Perlin>::new(params.seed)
                .set_frequency(params.frequency as f64)
                .set_octaves(params.fractal_octaves as usize)
                .set_lacunarity(params.fractal_lacunarity as f64)
                .set_persistence(params.fractal_gain as f64)) // Check if gain mapping is correct for Billow
        }
        RustFractalType::None => {
            // No fractal, just base noise scaled by frequency
            // Apply offset by wrapping the noise function or adding offset after sampling
            let base = ScalePoint::new(base_noise_generator)
                        .set_scale(params.frequency as f64);
            // TODO: Handle offset. Simplest is often adding offset *after* getting noise value.
            // Wrapping the function requires a custom NoiseFn impl or using Combine/Add modules if available.
             Box::new(base)
        }
    };

     // TODO: Handle offset and domain warp if needed, potentially by wrapping final_noise further.
     // Example: Offset might be applied where the noise value is used:
     // let base_val = final_noise.get(point);
     // let final_val = base_val + params.offset.y; // Assuming offset applies to height (y)

    final_noise
}
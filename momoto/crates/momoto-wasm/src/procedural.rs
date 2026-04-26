// =============================================================================
// momoto-wasm: Procedural Variation WASM Bindings
// File: crates/momoto-wasm/src/procedural.rs
//
// Exposes PerlinNoise and procedural variation utilities via wasm-bindgen.
// All functions are deterministic for the same seed — reproducible across frames.
// =============================================================================

use wasm_bindgen::prelude::*;

use momoto_materials::glass_physics::perlin_noise::{
    presets as noise_presets, PerlinNoise as CorePerlinNoise,
};

// =============================================================================
// ProceduralNoise — wraps CorePerlinNoise
// =============================================================================

/// Perlin-noise generator for procedural material variation.
///
/// Uses improved Perlin noise with configurable octaves for natural-looking
/// frosted glass textures. All output is in [0, 1] (normalised from [-1, 1]).
///
/// # Reproducibility
///
/// The same `seed` always produces the same noise field — safe for animations
/// where you need consistent texture between frames.
#[wasm_bindgen]
pub struct ProceduralNoise {
    inner: CorePerlinNoise,
}

#[wasm_bindgen]
impl ProceduralNoise {
    /// Create a noise generator with explicit parameters.
    ///
    /// # Arguments
    /// * `seed` — deterministic seed (u32)
    /// * `octaves` — number of noise layers (1=simple, 6=detailed)
    /// * `persistence` — amplitude falloff per octave (0.5 is standard)
    /// * `lacunarity` — frequency growth per octave (2.0 is standard)
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u32, octaves: u32, persistence: f64, lacunarity: f64) -> Self {
        Self {
            inner: CorePerlinNoise::new(seed, octaves, persistence, lacunarity),
        }
    }

    /// Preset: frosted glass (6 octaves — high detail).
    pub fn frosted() -> Self {
        Self {
            inner: noise_presets::frosted_glass(),
        }
    }

    /// Preset: regular glass (3 octaves — balanced).
    pub fn regular() -> Self {
        Self {
            inner: noise_presets::regular_glass(),
        }
    }

    /// Preset: clear glass (1 octave — minimal texture).
    pub fn clear() -> Self {
        Self {
            inner: noise_presets::clear_glass(),
        }
    }

    /// Preset: thick glass (4 octaves — more visible texture).
    pub fn thick() -> Self {
        Self {
            inner: noise_presets::thick_glass(),
        }
    }

    /// Sample fractional Brownian motion noise at (x, y).
    ///
    /// Returns value in `[0, 1]` (normalised — raw Perlin output is [-1, 1]).
    pub fn sample(&self, x: f64, y: f64) -> f64 {
        let raw = self.inner.fractal_noise_2d(x, y);
        // Normalise [-1,1] → [0,1]
        (raw + 1.0) * 0.5
    }

    /// Sample the raw Perlin value at (x, y) without normalisation.
    ///
    /// Returns value in approximately `[-1, 1]`.
    #[wasm_bindgen(js_name = "sampleRaw")]
    pub fn sample_raw(&self, x: f64, y: f64) -> f64 {
        self.inner.fractal_noise_2d(x, y)
    }

    /// Generate a 2D noise field.
    ///
    /// Returns a flat array of `cols * rows` values in `[0, 1]`,
    /// row-major order (left-to-right, top-to-bottom).
    ///
    /// # Arguments
    /// * `cols` — width in samples
    /// * `rows` — height in samples
    /// * `scale` — spatial frequency (0.05 = large features, 0.5 = fine detail)
    #[wasm_bindgen(js_name = "generateField")]
    pub fn generate_field(&self, cols: u32, rows: u32, scale: f64) -> Box<[f64]> {
        let n = (cols * rows) as usize;
        let mut out = Vec::with_capacity(n);
        for row in 0..rows {
            for col in 0..cols {
                let x = col as f64 * scale;
                let y = row as f64 * scale;
                let v = self.inner.fractal_noise_2d(x, y);
                out.push((v + 1.0) * 0.5); // normalise to [0,1]
            }
        }
        out.into_boxed_slice()
    }
}

// =============================================================================
// Free functions
// =============================================================================

/// Generate a 2D IOR variation field for procedural material texturing.
///
/// Models micro-scale IOR variation across a glass surface, useful for
/// frosted-glass distortion maps.
///
/// # Arguments
/// * `base_ior` — central IOR value (e.g. 1.5 for glass)
/// * `variation` — maximum deviation from base IOR (e.g. 0.05)
/// * `cols` — width in samples
/// * `rows` — height in samples
/// * `seed` — noise seed for reproducibility
///
/// # Returns
///
/// Flat array of `cols * rows` IOR values in `[base_ior - variation, base_ior + variation]`.
#[wasm_bindgen(js_name = "variationField")]
pub fn variation_field(
    base_ior: f64,
    variation: f64,
    cols: u32,
    rows: u32,
    seed: u32,
) -> Box<[f64]> {
    let noise = CorePerlinNoise::new(seed, 4, 0.5, 2.0);
    let n = (cols * rows) as usize;
    let mut out = Vec::with_capacity(n);
    let scale = 0.05f64;
    for row in 0..rows {
        for col in 0..cols {
            let x = col as f64 * scale;
            let y = row as f64 * scale;
            let v = noise.fractal_noise_2d(x, y); // in [-1, 1]
            out.push(base_ior + variation * v);
        }
    }
    out.into_boxed_slice()
}

/// Generate a roughness variation field for procedural surface micro-texture.
///
/// # Arguments
/// * `base_roughness` — central roughness (0=mirror, 1=fully diffuse)
/// * `variation` — maximum deviation
/// * `cols`, `rows` — grid dimensions
/// * `seed` — noise seed
///
/// # Returns
///
/// Flat array clamped to `[0, 1]`.
#[wasm_bindgen(js_name = "roughnessVariationField")]
pub fn roughness_variation_field(
    base_roughness: f64,
    variation: f64,
    cols: u32,
    rows: u32,
    seed: u32,
) -> Box<[f64]> {
    let noise = CorePerlinNoise::new(seed, 6, 0.5, 2.0);
    let n = (cols * rows) as usize;
    let mut out = Vec::with_capacity(n);
    let scale = 0.08f64;
    for row in 0..rows {
        for col in 0..cols {
            let x = col as f64 * scale;
            let y = row as f64 * scale;
            let v = noise.fractal_noise_2d(x, y);
            let r = (base_roughness + variation * v).clamp(0.0, 1.0);
            out.push(r);
        }
    }
    out.into_boxed_slice()
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reproducibility() {
        let n1 = ProceduralNoise::new(42, 4, 0.5, 2.0);
        let n2 = ProceduralNoise::new(42, 4, 0.5, 2.0);
        // Same seed must give same values
        assert_eq!(n1.sample(1.5, 2.3), n2.sample(1.5, 2.3));
        assert_eq!(n1.sample(0.0, 0.0), n2.sample(0.0, 0.0));
    }

    #[test]
    fn test_different_seeds() {
        let n1 = ProceduralNoise::new(1, 4, 0.5, 2.0);
        let n2 = ProceduralNoise::new(2, 4, 0.5, 2.0);
        // Different seeds should give different values (with high probability)
        assert_ne!(n1.sample(1.5, 2.3), n2.sample(1.5, 2.3));
    }

    #[test]
    fn test_output_in_range() {
        let noise = ProceduralNoise::frosted();
        for i in 0..50 {
            let v = noise.sample(i as f64 * 0.1, i as f64 * 0.07);
            assert!(v >= 0.0 && v <= 1.0, "Value {} out of [0,1]", v);
        }
    }

    #[test]
    fn test_generate_field_size() {
        let noise = ProceduralNoise::regular();
        let field = noise.generate_field(8, 6, 0.1);
        assert_eq!(field.len(), 8 * 6);
    }

    #[test]
    fn test_variation_field_range() {
        let base_ior = 1.5;
        let variation = 0.05;
        let field = variation_field(base_ior, variation, 8, 8, 42);
        for &v in field.iter() {
            assert!(
                v >= base_ior - variation - 1e-10 && v <= base_ior + variation + 1e-10,
                "IOR {} out of expected range",
                v
            );
        }
    }
}

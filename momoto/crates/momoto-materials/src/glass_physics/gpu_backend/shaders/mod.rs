//! # WGSL Compute Shaders
//!
//! WGSL shader sources for GPU-accelerated BSDF evaluation.
//!
//! ## Shaders
//!
//! - `unified_bsdf.wgsl` - Dielectric and Conductor BSDF evaluation
//! - `anisotropic.wgsl` - Anisotropic GGX microfacet model
//! - `thin_film.wgsl` - Thin-film interference effects
//! - `neural_inference.wgsl` - Neural MLP forward pass (SIREN)
//!
//! ## Design Principles
//!
//! 1. **Parity with CPU**: Shaders implement the same algorithms as Rust code
//! 2. **Energy Conservation**: R + T + A = 1 is enforced in shaders
//! 3. **f32 Precision**: GPU uses f32, CPU uses f64 - perceptual parity is goal
//! 4. **Workgroup Size**: 256 threads per workgroup for broad compatibility

/// Embedded WGSL source for unified BSDF (dielectric + conductor).
pub const UNIFIED_BSDF_WGSL: &str = include_str!("unified_bsdf.wgsl");

/// Embedded WGSL source for anisotropic GGX.
pub const ANISOTROPIC_WGSL: &str = include_str!("anisotropic.wgsl");

/// Embedded WGSL source for thin-film interference.
pub const THIN_FILM_WGSL: &str = include_str!("thin_film.wgsl");

/// Embedded WGSL source for neural MLP inference.
pub const NEURAL_INFERENCE_WGSL: &str = include_str!("neural_inference.wgsl");

/// Get shader source by name.
pub fn get_shader_source(name: &str) -> Option<&'static str> {
    match name {
        "unified_bsdf" | "dielectric" | "conductor" => Some(UNIFIED_BSDF_WGSL),
        "anisotropic" => Some(ANISOTROPIC_WGSL),
        "thin_film" => Some(THIN_FILM_WGSL),
        "neural_inference" | "neural" => Some(NEURAL_INFERENCE_WGSL),
        _ => None,
    }
}

/// List all available shader names.
pub fn available_shaders() -> &'static [&'static str] {
    &[
        "unified_bsdf",
        "anisotropic",
        "thin_film",
        "neural_inference",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_shader_source() {
        assert!(get_shader_source("unified_bsdf").is_some());
        assert!(get_shader_source("anisotropic").is_some());
        assert!(get_shader_source("thin_film").is_some());
        assert!(get_shader_source("neural_inference").is_some());
        assert!(get_shader_source("nonexistent").is_none());
    }

    #[test]
    fn test_available_shaders() {
        let shaders = available_shaders();
        assert_eq!(shaders.len(), 4);
    }

    #[test]
    fn test_shader_aliases() {
        // Test that aliases work
        assert_eq!(
            get_shader_source("dielectric"),
            get_shader_source("unified_bsdf")
        );
        assert_eq!(
            get_shader_source("conductor"),
            get_shader_source("unified_bsdf")
        );
        assert_eq!(
            get_shader_source("neural"),
            get_shader_source("neural_inference")
        );
    }
}

//! # BSDF Types
//!
//! Re-exports of stable BSDF types from the unified_bsdf module.

// Re-export core BSDF types from unified_bsdf
pub use super::super::super::unified_bsdf::{
    BSDFContext, BSDFResponse, BSDFSample, ConductorBSDF, DielectricBSDF, EnergyValidation,
    LambertianBSDF, LayeredBSDF, ThinFilmBSDF, BSDF,
};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dielectric_creation() {
        let bsdf = DielectricBSDF::new(1.5, 0.0);
        assert_eq!(bsdf.name(), "DielectricBSDF");
    }

    #[test]
    fn test_conductor_creation() {
        let bsdf = ConductorBSDF::new(0.18, 3.0, 0.0);
        assert_eq!(bsdf.name(), "ConductorBSDF");
    }

    #[test]
    fn test_response_energy_conservation() {
        let response = BSDFResponse::default();
        let total = response.reflectance + response.transmittance + response.absorption;
        assert!((total - 1.0).abs() < 0.01);
    }
}

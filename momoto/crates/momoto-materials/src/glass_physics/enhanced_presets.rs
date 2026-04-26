//! # Enhanced Material Presets with PBR Phase 1 Parameters
//!
//! Material definitions using the new physically-based parameters:
//! - Chromatic dispersion (Cauchy/Sellmeier)
//! - Directional scattering (Henyey-Greenstein)
//! - Per-channel Fresnel effects
//!
//! These presets demonstrate the improved physics models and
//! serve as validation test cases.

use super::dispersion::{CauchyDispersion, DispersionModel, SellmeierDispersion};
use super::scattering::{presets as scatter_presets, ScatteringParams};

// ============================================================================
// ENHANCED GLASS MATERIAL
// ============================================================================

/// Enhanced glass material with PBR Phase 1 parameters
///
/// Combines traditional glass properties with new physics models.
#[derive(Debug, Clone)]
pub struct EnhancedGlassMaterial {
    /// Material name
    pub name: &'static str,

    // --- Traditional Parameters ---
    /// Base index of refraction (at d-line)
    pub ior: f64,
    /// Surface roughness (0.0 = mirror, 1.0 = diffuse)
    pub roughness: f64,
    /// Optical thickness in mm
    pub thickness: f64,
    /// Absorption coefficient per mm
    pub absorption: f64,

    // --- PBR Phase 1: Dispersion ---
    /// Chromatic dispersion model
    pub dispersion: DispersionModel,

    // --- PBR Phase 1: Scattering ---
    /// Scattering phase function parameters
    pub scattering: ScatteringParams,

    // --- Quality Hints ---
    /// Suggested quality tier for this material
    pub quality_hint: QualityTier,
}

/// Quality tier for adaptive rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QualityTier {
    /// Fast: Original Momoto (Schlick, no spectral)
    Fast,
    #[default]
    /// Standard: RGB spectral, single H-G
    Standard,
    /// High: Full spectral, double H-G, Phase 3+4 features
    High,
    /// UltraHigh: Research-grade with SIMD, combined effects, perceptual calibration (Phase 6)
    UltraHigh,
    /// Experimental: Phase 7 ultra-realistic with advanced parallelization, spectral rendering,
    /// real-time calibration, dynamic effects, and experimental presets
    Experimental,
    /// Reference: Full physics validation, no optimizations
    Reference,
}

impl EnhancedGlassMaterial {
    /// Create material with default settings
    pub fn new(name: &'static str, ior: f64) -> Self {
        Self {
            name,
            ior,
            roughness: 0.02,
            thickness: 5.0,
            absorption: 0.01,
            dispersion: DispersionModel::from_ior(ior),
            scattering: ScatteringParams::isotropic(),
            quality_hint: QualityTier::Standard,
        }
    }

    /// Get Abbe number (dispersion strength)
    pub fn abbe_number(&self) -> f64 {
        use super::dispersion::Dispersion;
        self.dispersion.abbe_number()
    }

    /// Check if material has significant dispersion
    pub fn is_dispersive(&self) -> bool {
        self.dispersion.is_dispersive()
    }

    /// Get scattering radius in mm
    pub fn scattering_radius(&self) -> f64 {
        self.scattering
            .scattering_radius_mm(self.roughness, self.thickness)
    }

    /// Builder pattern methods
    pub fn with_roughness(mut self, roughness: f64) -> Self {
        self.roughness = roughness;
        self
    }

    pub fn with_thickness(mut self, thickness: f64) -> Self {
        self.thickness = thickness;
        self
    }

    pub fn with_absorption(mut self, absorption: f64) -> Self {
        self.absorption = absorption;
        self
    }

    pub fn with_dispersion(mut self, dispersion: DispersionModel) -> Self {
        self.dispersion = dispersion;
        self
    }

    pub fn with_scattering(mut self, scattering: ScatteringParams) -> Self {
        self.scattering = scattering;
        self
    }

    pub fn with_quality(mut self, quality: QualityTier) -> Self {
        self.quality_hint = quality;
        self
    }
}

// ============================================================================
// MATERIAL PRESETS
// ============================================================================

/// Crown Glass (BK7) - Low dispersion optical glass
///
/// Characteristics:
/// - Low dispersion (Abbe ~64)
/// - Common in windows, lenses
/// - Minimal chromatic effects
pub fn crown_glass() -> EnhancedGlassMaterial {
    EnhancedGlassMaterial {
        name: "Crown Glass (BK7)",
        ior: 1.5168,
        roughness: 0.02,
        thickness: 5.0,
        absorption: 0.01,
        dispersion: DispersionModel::Cauchy(CauchyDispersion::crown_glass()),
        scattering: ScatteringParams::forward(0.0),
        quality_hint: QualityTier::Standard,
    }
}

/// Flint Glass (SF11) - High dispersion optical glass
///
/// Characteristics:
/// - High dispersion (Abbe ~25)
/// - Strong chromatic effects
/// - Used in prisms, decorative items
pub fn flint_glass() -> EnhancedGlassMaterial {
    EnhancedGlassMaterial {
        name: "Flint Glass (SF11)",
        ior: 1.7847,
        roughness: 0.02,
        thickness: 5.0,
        absorption: 0.02,
        dispersion: DispersionModel::Cauchy(CauchyDispersion::flint_glass()),
        scattering: ScatteringParams::forward(0.0),
        quality_hint: QualityTier::High, // Chromatic effects worth showing
    }
}

/// Fused Silica - Ultra-low dispersion
///
/// Characteristics:
/// - Very low dispersion (Abbe ~68)
/// - High purity
/// - Used in precision optics
pub fn fused_silica() -> EnhancedGlassMaterial {
    EnhancedGlassMaterial {
        name: "Fused Silica",
        ior: 1.4585,
        roughness: 0.01,
        thickness: 3.0,
        absorption: 0.005,
        dispersion: DispersionModel::Sellmeier(SellmeierDispersion::fused_silica()),
        scattering: ScatteringParams::isotropic(),
        quality_hint: QualityTier::High,
    }
}

/// Diamond - High IOR, high dispersion
///
/// Characteristics:
/// - Very high IOR (2.42)
/// - Strong "fire" (dispersion)
/// - Brilliant edge effects
pub fn diamond() -> EnhancedGlassMaterial {
    EnhancedGlassMaterial {
        name: "Diamond",
        ior: 2.417,
        roughness: 0.005,
        thickness: 2.0,
        absorption: 0.001,
        dispersion: DispersionModel::Cauchy(CauchyDispersion::diamond()),
        scattering: ScatteringParams::isotropic(),
        quality_hint: QualityTier::High,
    }
}

/// Frosted Glass - Diffuse scattering
///
/// Characteristics:
/// - Moderate scattering
/// - Privacy glass effect
/// - Soft diffuse edges
pub fn frosted_glass() -> EnhancedGlassMaterial {
    EnhancedGlassMaterial {
        name: "Frosted Glass",
        ior: 1.52,
        roughness: 0.5,
        thickness: 5.0,
        absorption: 0.05,
        dispersion: DispersionModel::Cauchy(CauchyDispersion::crown_glass()),
        scattering: scatter_presets::frosted_glass(),
        quality_hint: QualityTier::Standard,
    }
}

/// Opal Glass - Complex translucent scattering
///
/// Characteristics:
/// - Forward scattering with backscatter
/// - Milky/opalescent appearance
/// - Complex light interaction
pub fn opal_glass() -> EnhancedGlassMaterial {
    EnhancedGlassMaterial {
        name: "Opal Glass",
        ior: 1.45,
        roughness: 0.3,
        thickness: 4.0,
        absorption: 0.15,
        dispersion: DispersionModel::from_ior(1.45),
        scattering: scatter_presets::opal(),
        quality_hint: QualityTier::High,
    }
}

/// Polycarbonate - High dispersion plastic
///
/// Characteristics:
/// - High dispersion (Abbe ~30)
/// - Common in eyewear, displays
/// - Noticeable chromatic aberration
pub fn polycarbonate() -> EnhancedGlassMaterial {
    EnhancedGlassMaterial {
        name: "Polycarbonate",
        ior: 1.585,
        roughness: 0.03,
        thickness: 3.0,
        absorption: 0.02,
        dispersion: DispersionModel::Cauchy(CauchyDispersion::polycarbonate()),
        scattering: ScatteringParams::forward(0.1),
        quality_hint: QualityTier::Standard,
    }
}

/// PMMA (Acrylic) - Low dispersion plastic
///
/// Characteristics:
/// - Low dispersion (Abbe ~57)
/// - Common in displays, signage
/// - Clean, clear appearance
pub fn pmma() -> EnhancedGlassMaterial {
    EnhancedGlassMaterial {
        name: "PMMA (Acrylic)",
        ior: 1.492,
        roughness: 0.02,
        thickness: 4.0,
        absorption: 0.015,
        dispersion: DispersionModel::Cauchy(CauchyDispersion::pmma()),
        scattering: ScatteringParams::isotropic(),
        quality_hint: QualityTier::Fast,
    }
}

/// Water - Reference liquid
///
/// Characteristics:
/// - Lower IOR (1.33)
/// - Moderate dispersion
/// - Reference for liquid effects
pub fn water() -> EnhancedGlassMaterial {
    EnhancedGlassMaterial {
        name: "Water",
        ior: 1.333,
        roughness: 0.0,
        thickness: 10.0,
        absorption: 0.01,
        dispersion: DispersionModel::Cauchy(CauchyDispersion::water()),
        scattering: ScatteringParams::isotropic(),
        quality_hint: QualityTier::Standard,
    }
}

/// Sapphire - High quality optical crystal
///
/// Characteristics:
/// - High IOR (1.77)
/// - Very hard, scratch resistant
/// - Used in watch faces, windows
pub fn sapphire() -> EnhancedGlassMaterial {
    EnhancedGlassMaterial {
        name: "Sapphire",
        ior: 1.77,
        roughness: 0.005,
        thickness: 1.0,
        absorption: 0.002,
        dispersion: DispersionModel::Sellmeier(SellmeierDispersion::sapphire()),
        scattering: ScatteringParams::isotropic(),
        quality_hint: QualityTier::High,
    }
}

/// Ice - Frozen water
///
/// Characteristics:
/// - Similar to water but crystalline
/// - Can have internal fractures
/// - Slight scattering from inclusions
pub fn ice() -> EnhancedGlassMaterial {
    EnhancedGlassMaterial {
        name: "Ice",
        ior: 1.31,
        roughness: 0.1,
        thickness: 20.0,
        absorption: 0.005,
        dispersion: DispersionModel::from_ior(1.31),
        scattering: ScatteringParams::forward(0.15),
        quality_hint: QualityTier::Standard,
    }
}

/// Milk Glass - Strong subsurface scattering
///
/// Characteristics:
/// - Opaque white appearance
/// - Strong forward + back scatter
/// - Common in lighting diffusers
pub fn milk_glass() -> EnhancedGlassMaterial {
    EnhancedGlassMaterial {
        name: "Milk Glass",
        ior: 1.52,
        roughness: 0.4,
        thickness: 3.0,
        absorption: 0.3,
        dispersion: DispersionModel::from_ior(1.52),
        scattering: scatter_presets::milk(),
        quality_hint: QualityTier::High,
    }
}

// ============================================================================
// PRESET COLLECTION
// ============================================================================

/// Get all available presets
pub fn all_presets() -> Vec<EnhancedGlassMaterial> {
    vec![
        crown_glass(),
        flint_glass(),
        fused_silica(),
        diamond(),
        frosted_glass(),
        opal_glass(),
        polycarbonate(),
        pmma(),
        water(),
        sapphire(),
        ice(),
        milk_glass(),
    ]
}

/// Get presets by quality tier
pub fn presets_by_quality(tier: QualityTier) -> Vec<EnhancedGlassMaterial> {
    all_presets()
        .into_iter()
        .filter(|p| p.quality_hint == tier)
        .collect()
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_presets_valid() {
        for preset in all_presets() {
            assert!(preset.ior > 1.0, "{} IOR should be > 1", preset.name);
            assert!(preset.ior < 3.0, "{} IOR should be < 3", preset.name);
            assert!(
                preset.roughness >= 0.0,
                "{} roughness should be >= 0",
                preset.name
            );
            assert!(
                preset.roughness <= 1.0,
                "{} roughness should be <= 1",
                preset.name
            );
            assert!(
                preset.thickness > 0.0,
                "{} thickness should be > 0",
                preset.name
            );
            assert!(
                preset.absorption >= 0.0,
                "{} absorption should be >= 0",
                preset.name
            );
        }
    }

    #[test]
    fn test_dispersion_ordering() {
        let crown = crown_glass();
        let flint = flint_glass();

        // Flint should have lower Abbe number (more dispersion)
        assert!(
            flint.abbe_number() < crown.abbe_number(),
            "Flint should have lower Abbe number"
        );
    }

    #[test]
    fn test_scattering_radius() {
        let clear = crown_glass();
        let frosted = frosted_glass();

        // Frosted should have larger scattering radius
        assert!(
            frosted.scattering_radius() > clear.scattering_radius(),
            "Frosted should scatter more"
        );
    }

    #[test]
    fn test_quality_tiers() {
        let fast = presets_by_quality(QualityTier::Fast);
        let high = presets_by_quality(QualityTier::High);

        // Should have materials in different tiers
        assert!(!fast.is_empty() || !high.is_empty());
    }

    #[test]
    fn test_builder_pattern() {
        let custom = EnhancedGlassMaterial::new("Custom", 1.6)
            .with_roughness(0.1)
            .with_thickness(3.0)
            .with_absorption(0.05)
            .with_quality(QualityTier::High);

        assert_eq!(custom.roughness, 0.1);
        assert_eq!(custom.thickness, 3.0);
        assert_eq!(custom.quality_hint, QualityTier::High);
    }
}

//! # Combined Effects Module
//!
//! Unified effect compositor for combining multiple optical effects into coherent materials.
//!
//! ## Features
//!
//! - **Effect Stacking**: Combine Fresnel, Thin-Film, Metal, Mie in one material
//! - **Blend Modes**: Physically-based and artistic blending options
//! - **Presets**: Common combined materials (soap bubble, opal, patina)
//! - **CSS Output**: Generate gradients for all combined effects
//!
//! ## Usage
//!
//! ```rust
//! use momoto_materials::glass_physics::combined_effects::{
//!     CombinedMaterial, EffectLayer, BlendMode, presets,
//! };
//!
//! let material = CombinedMaterial::builder()
//!     .add_fresnel(1.5)
//!     .add_roughness(0.05)
//!     .blend_mode(BlendMode::PhysicallyBased)
//!     .build();
//!
//! let rgb = material.evaluate_rgb(0.7);
//! ```

use std::f64::consts::PI;

use super::enhanced_presets::QualityTier;
use super::fresnel::fresnel_schlick;

// ============================================================================
// EFFECT LAYER DEFINITIONS
// ============================================================================

/// Individual effect layer in the stack
#[derive(Debug, Clone)]
pub enum EffectLayer {
    /// Base Fresnel reflection
    Fresnel { ior: f64, spectral: bool },

    /// Thin-film interference
    ThinFilm {
        n_film: f64,
        thickness_nm: f64,
        n_substrate: f64,
    },

    /// Metal with complex IOR
    Metal { n: f64, k: f64 },

    /// Mie scattering from particles
    Mie {
        g: f64,          // Asymmetry parameter
        extinction: f64, // Extinction coefficient
    },

    /// Surface roughness
    Roughness {
        value: f64, // 0-1 roughness
        model: RoughnessModel,
    },

    /// Absorption (Beer-Lambert)
    Absorption { coefficient: f64, thickness: f64 },

    /// Oxidation layer
    Oxidation {
        oxide_n: f64,
        oxide_k: f64,
        thickness_nm: f64,
    },
}

/// Roughness model selection
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum RoughnessModel {
    /// GGX/Trowbridge-Reitz
    #[default]
    GGX,
    /// Beckmann distribution
    Beckmann,
    /// Blinn-Phong (legacy)
    BlinnPhong,
}

/// Blend mode for combining effects
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum BlendMode {
    /// Add reflectance values (clamped to 1)
    Additive,
    /// Multiply reflectance values
    Multiplicative,
    /// Weight by Fresnel term
    FresnelWeighted,
    /// Physics-based layering (default)
    #[default]
    PhysicallyBased,
}

// ============================================================================
// COMBINED MATERIAL
// ============================================================================

/// Combined material with multiple effects
#[derive(Debug, Clone)]
pub struct CombinedMaterial {
    /// Effect layers (evaluated bottom to top)
    pub layers: Vec<EffectLayer>,
    /// Blend mode
    pub blend_mode: BlendMode,
    /// Quality tier for feature selection
    pub quality_tier: QualityTier,
    /// Cached base IOR
    base_ior: f64,
}

impl CombinedMaterial {
    /// Create builder
    pub fn builder() -> CombinedMaterialBuilder {
        CombinedMaterialBuilder::new()
    }

    /// Evaluate reflectance at wavelength and angle
    pub fn evaluate(&self, wavelength_nm: f64, cos_theta: f64) -> f64 {
        let cos_theta = cos_theta.clamp(0.0, 1.0);

        match self.blend_mode {
            BlendMode::Additive => self.evaluate_additive(wavelength_nm, cos_theta),
            BlendMode::Multiplicative => self.evaluate_multiplicative(wavelength_nm, cos_theta),
            BlendMode::FresnelWeighted => self.evaluate_fresnel_weighted(wavelength_nm, cos_theta),
            BlendMode::PhysicallyBased => self.evaluate_physically_based(wavelength_nm, cos_theta),
        }
    }

    /// Evaluate RGB reflectance at angle
    pub fn evaluate_rgb(&self, cos_theta: f64) -> [f64; 3] {
        [
            self.evaluate(650.0, cos_theta),
            self.evaluate(550.0, cos_theta),
            self.evaluate(450.0, cos_theta),
        ]
    }

    /// Evaluate full spectrum (31 points, 400-700nm)
    pub fn evaluate_spectral(&self, cos_theta: f64) -> Vec<(f64, f64)> {
        (0..31)
            .map(|i| {
                let w = 400.0 + i as f64 * 10.0;
                (w, self.evaluate(w, cos_theta))
            })
            .collect()
    }

    /// Additive blend mode
    fn evaluate_additive(&self, wavelength_nm: f64, cos_theta: f64) -> f64 {
        let mut total = 0.0;

        for layer in &self.layers {
            total += self.evaluate_layer(layer, wavelength_nm, cos_theta);
        }

        total.min(1.0)
    }

    /// Multiplicative blend mode
    fn evaluate_multiplicative(&self, wavelength_nm: f64, cos_theta: f64) -> f64 {
        let mut total = 1.0;

        for layer in &self.layers {
            total *= self.evaluate_layer(layer, wavelength_nm, cos_theta);
        }

        total
    }

    /// Fresnel-weighted blend mode
    fn evaluate_fresnel_weighted(&self, wavelength_nm: f64, cos_theta: f64) -> f64 {
        let fresnel_weight = fresnel_schlick(1.0, self.base_ior, cos_theta);
        let mut total = 0.0;

        for layer in &self.layers {
            let layer_value = self.evaluate_layer(layer, wavelength_nm, cos_theta);
            total += layer_value * fresnel_weight;
        }

        total.min(1.0)
    }

    /// Physically-based blend mode (layered model)
    fn evaluate_physically_based(&self, wavelength_nm: f64, cos_theta: f64) -> f64 {
        let mut reflectance = 0.0;
        let mut transmittance = 1.0;

        // Process layers from top to bottom
        for layer in &self.layers {
            let layer_r = self.evaluate_layer(layer, wavelength_nm, cos_theta);

            // Reflected light that made it through upper layers
            reflectance += transmittance * transmittance * layer_r;

            // Remaining transmission
            transmittance *= 1.0 - layer_r;

            if transmittance < 0.001 {
                break;
            }
        }

        reflectance.min(1.0)
    }

    /// Evaluate single layer
    fn evaluate_layer(&self, layer: &EffectLayer, wavelength_nm: f64, cos_theta: f64) -> f64 {
        match layer {
            EffectLayer::Fresnel { ior, spectral: _ } => fresnel_schlick(1.0, *ior, cos_theta),

            EffectLayer::ThinFilm {
                n_film,
                thickness_nm,
                n_substrate,
            } => thin_film_reflectance(
                wavelength_nm,
                *n_film,
                *thickness_nm,
                *n_substrate,
                cos_theta,
            ),

            EffectLayer::Metal { n, k } => metal_fresnel(*n, *k, cos_theta),

            EffectLayer::Mie { g, extinction } => {
                henyey_greenstein_phase(cos_theta, *g) * (1.0 - (-*extinction).exp())
            }

            EffectLayer::Roughness { value, model } => roughness_factor(cos_theta, *value, *model),

            EffectLayer::Absorption {
                coefficient,
                thickness,
            } => (-*coefficient * *thickness).exp(),

            EffectLayer::Oxidation {
                oxide_n,
                oxide_k,
                thickness_nm,
            } => oxide_reflectance(*oxide_n, *oxide_k, *thickness_nm, wavelength_nm),
        }
    }

    /// Generate CSS gradient
    pub fn to_css(&self, angle_deg: f64) -> String {
        let cos_theta = (angle_deg * PI / 180.0).cos();
        let rgb = self.evaluate_rgb(cos_theta);

        let r = (rgb[0] * 255.0).round() as u8;
        let g = (rgb[1] * 255.0).round() as u8;
        let b = (rgb[2] * 255.0).round() as u8;

        // Generate radial gradient for material appearance
        let center_rgb = self.evaluate_rgb(1.0);
        let cr = (center_rgb[0] * 255.0).round() as u8;
        let cg = (center_rgb[1] * 255.0).round() as u8;
        let cb = (center_rgb[2] * 255.0).round() as u8;

        format!(
            "radial-gradient(ellipse at 30% 30%, rgb({}, {}, {}) 0%, rgb({}, {}, {}) 100%)",
            cr, cg, cb, r, g, b
        )
    }

    /// Generate CSS with iridescence animation
    pub fn to_css_animated(&self, duration_s: f64) -> String {
        let mut keyframes = String::new();
        keyframes.push_str("@keyframes iridescence {\n");

        for i in 0..=10 {
            let t = i as f64 / 10.0;
            let angle = t * 60.0; // 0 to 60 degrees
            let cos_theta = (angle * PI / 180.0).cos();
            let rgb = self.evaluate_rgb(cos_theta);

            let r = (rgb[0] * 255.0).round() as u8;
            let g = (rgb[1] * 255.0).round() as u8;
            let b = (rgb[2] * 255.0).round() as u8;

            keyframes.push_str(&format!(
                "  {}% {{ background-color: rgb({}, {}, {}); }}\n",
                (t * 100.0).round() as u32,
                r,
                g,
                b
            ));
        }

        keyframes.push_str("}\n\n");
        keyframes.push_str(&format!(
            ".iridescent {{ animation: iridescence {}s ease-in-out infinite alternate; }}",
            duration_s
        ));

        keyframes
    }

    /// Number of layers
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    /// Get base IOR
    pub fn base_ior(&self) -> f64 {
        self.base_ior
    }
}

impl Default for CombinedMaterial {
    fn default() -> Self {
        Self {
            layers: vec![EffectLayer::Fresnel {
                ior: 1.5,
                spectral: false,
            }],
            blend_mode: BlendMode::PhysicallyBased,
            quality_tier: QualityTier::High,
            base_ior: 1.5,
        }
    }
}

// ============================================================================
// BUILDER
// ============================================================================

/// Builder for CombinedMaterial
#[derive(Debug, Clone, Default)]
pub struct CombinedMaterialBuilder {
    layers: Vec<EffectLayer>,
    blend_mode: BlendMode,
    quality_tier: QualityTier,
    base_ior: f64,
}

impl CombinedMaterialBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            layers: Vec::new(),
            blend_mode: BlendMode::PhysicallyBased,
            quality_tier: QualityTier::High,
            base_ior: 1.5,
        }
    }

    /// Add Fresnel layer
    pub fn add_fresnel(mut self, ior: f64) -> Self {
        self.base_ior = ior;
        self.layers.push(EffectLayer::Fresnel {
            ior,
            spectral: false,
        });
        self
    }

    /// Add spectral Fresnel layer
    pub fn add_fresnel_spectral(mut self, ior: f64) -> Self {
        self.base_ior = ior;
        self.layers.push(EffectLayer::Fresnel {
            ior,
            spectral: true,
        });
        self
    }

    /// Add thin-film layer
    pub fn add_thin_film(mut self, n_film: f64, thickness_nm: f64, n_substrate: f64) -> Self {
        self.layers.push(EffectLayer::ThinFilm {
            n_film,
            thickness_nm,
            n_substrate,
        });
        self
    }

    /// Add metal layer
    pub fn add_metal(mut self, n: f64, k: f64) -> Self {
        self.layers.push(EffectLayer::Metal { n, k });
        self
    }

    /// Add Mie scattering layer
    pub fn add_mie(mut self, g: f64, extinction: f64) -> Self {
        self.layers.push(EffectLayer::Mie { g, extinction });
        self
    }

    /// Add roughness layer
    pub fn add_roughness(mut self, value: f64) -> Self {
        self.layers.push(EffectLayer::Roughness {
            value,
            model: RoughnessModel::GGX,
        });
        self
    }

    /// Add roughness with specific model
    pub fn add_roughness_model(mut self, value: f64, model: RoughnessModel) -> Self {
        self.layers.push(EffectLayer::Roughness { value, model });
        self
    }

    /// Add absorption layer
    pub fn add_absorption(mut self, coefficient: f64, thickness: f64) -> Self {
        self.layers.push(EffectLayer::Absorption {
            coefficient,
            thickness,
        });
        self
    }

    /// Add oxidation layer
    pub fn add_oxidation(mut self, oxide_n: f64, oxide_k: f64, thickness_nm: f64) -> Self {
        self.layers.push(EffectLayer::Oxidation {
            oxide_n,
            oxide_k,
            thickness_nm,
        });
        self
    }

    /// Set blend mode
    pub fn blend_mode(mut self, mode: BlendMode) -> Self {
        self.blend_mode = mode;
        self
    }

    /// Set quality tier
    pub fn quality_tier(mut self, tier: QualityTier) -> Self {
        self.quality_tier = tier;
        self
    }

    /// Build the material
    pub fn build(self) -> CombinedMaterial {
        CombinedMaterial {
            layers: self.layers,
            blend_mode: self.blend_mode,
            quality_tier: self.quality_tier,
            base_ior: self.base_ior,
        }
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Thin-film interference reflectance (simplified)
fn thin_film_reflectance(
    wavelength_nm: f64,
    n_film: f64,
    thickness_nm: f64,
    n_substrate: f64,
    cos_theta: f64,
) -> f64 {
    // Phase difference
    let delta = 4.0 * PI * n_film * thickness_nm * cos_theta / wavelength_nm;

    // Fresnel coefficients at interfaces
    let r1 = (1.0 - n_film) / (1.0 + n_film);
    let r2 = (n_film - n_substrate) / (n_film + n_substrate);

    // Airy formula (simplified)
    let numerator = r1 * r1 + r2 * r2 + 2.0 * r1 * r2 * delta.cos();
    let denominator = 1.0 + r1 * r1 * r2 * r2 + 2.0 * r1 * r2 * delta.cos();

    (numerator / denominator).abs().min(1.0)
}

/// Metal Fresnel reflectance (normal incidence approximation)
fn metal_fresnel(n: f64, k: f64, cos_theta: f64) -> f64 {
    // Schlick approximation adjusted for metals
    let f0 = ((n - 1.0).powi(2) + k.powi(2)) / ((n + 1.0).powi(2) + k.powi(2));
    let one_minus_cos = 1.0 - cos_theta;
    let pow5 = one_minus_cos.powi(5);
    f0 + (1.0 - f0) * pow5
}

/// Henyey-Greenstein phase function
fn henyey_greenstein_phase(cos_theta: f64, g: f64) -> f64 {
    if g.abs() < 1e-10 {
        return 1.0 / (4.0 * PI);
    }

    let g2 = g * g;
    let denom = 1.0 + g2 - 2.0 * g * cos_theta;
    (1.0 - g2) / (4.0 * PI * denom * denom.sqrt())
}

/// Roughness factor (GGX-based)
fn roughness_factor(cos_theta: f64, roughness: f64, model: RoughnessModel) -> f64 {
    let alpha = roughness * roughness;

    match model {
        RoughnessModel::GGX => {
            // GGX geometry term
            let k = alpha / 2.0;
            cos_theta / (cos_theta * (1.0 - k) + k)
        }
        RoughnessModel::Beckmann => {
            // Beckmann geometry term
            let c = cos_theta / (alpha * (1.0 - cos_theta * cos_theta).sqrt());
            if c < 1.6 {
                (3.535 * c + 2.181 * c * c) / (1.0 + 2.276 * c + 2.577 * c * c)
            } else {
                1.0
            }
        }
        RoughnessModel::BlinnPhong => {
            // Simple power law
            cos_theta.powf(1.0 / roughness.max(0.01))
        }
    }
}

/// Oxide layer reflectance
fn oxide_reflectance(n: f64, k: f64, thickness_nm: f64, wavelength_nm: f64) -> f64 {
    // Phase from oxide thickness
    let phase = 4.0 * PI * n * thickness_nm / wavelength_nm;

    // Fresnel at air-oxide interface
    let r1 = ((n - 1.0).powi(2) + k.powi(2)) / ((n + 1.0).powi(2) + k.powi(2));

    // Interference modulation
    let interference = 0.5 * (1.0 + phase.cos() * 0.3);

    // Absorption in oxide
    let absorption = (-k * thickness_nm / 100.0).exp();

    r1 * interference * absorption
}

// ============================================================================
// PRESETS
// ============================================================================

/// Combined effect presets
pub mod presets {
    use super::*;

    /// Soap bubble with thin-film interference and Mie scattering
    pub fn soap_bubble() -> CombinedMaterial {
        CombinedMaterial::builder()
            .add_fresnel(1.33)
            .add_thin_film(1.33, 350.0, 1.0) // Water film ~350nm
            .add_mie(0.8, 0.1) // Forward scattering from microbubbles
            .blend_mode(BlendMode::PhysicallyBased)
            .build()
    }

    /// Aged copper with patina (oxidation + thin-film)
    pub fn copper_patina() -> CombinedMaterial {
        CombinedMaterial::builder()
            .add_metal(0.27, 3.41) // Copper base
            .add_oxidation(2.63, 0.5, 50.0) // CuO layer
            .add_oxidation(1.73, 0.1, 200.0) // Patina (basic copper carbonate)
            .blend_mode(BlendMode::PhysicallyBased)
            .quality_tier(QualityTier::High)
            .build()
    }

    /// Opal glass (milk glass + thin-film + Mie)
    pub fn opal_glass() -> CombinedMaterial {
        CombinedMaterial::builder()
            .add_fresnel(1.52)
            .add_mie(0.7, 0.3) // Strong scattering
            .add_thin_film(1.45, 150.0, 1.52) // Surface film
            .add_roughness(0.1)
            .blend_mode(BlendMode::PhysicallyBased)
            .build()
    }

    /// Morpho butterfly wing (multi-layer thin-film)
    pub fn morpho_wing() -> CombinedMaterial {
        CombinedMaterial::builder()
            .add_fresnel(1.56) // Chitin
            .add_thin_film(1.56, 80.0, 1.0) // Air gap
            .add_thin_film(1.56, 70.0, 1.0)
            .add_thin_film(1.56, 75.0, 1.0)
            .add_roughness(0.05)
            .blend_mode(BlendMode::PhysicallyBased)
            .build()
    }

    /// Titanium alloy at elevated temperature
    pub fn titanium_alloy(temp_k: f64) -> CombinedMaterial {
        // Temperature affects oxidation
        let oxide_thickness = 10.0 + (temp_k - 293.0) * 0.1;

        CombinedMaterial::builder()
            .add_metal(2.73, 3.82) // Titanium
            .add_oxidation(2.5, 0.01, oxide_thickness.max(10.0)) // TiO2
            .add_roughness(0.1)
            .blend_mode(BlendMode::PhysicallyBased)
            .build()
    }

    /// Weathered bronze
    pub fn weathered_bronze() -> CombinedMaterial {
        CombinedMaterial::builder()
            .add_metal(0.35, 3.30) // Bronze (Cu-Sn)
            .add_oxidation(2.65, 0.3, 30.0) // SnO2 + CuO
            .add_oxidation(1.70, 0.05, 100.0) // Patina
            .add_mie(0.5, 0.05) // Surface dust
            .blend_mode(BlendMode::PhysicallyBased)
            .quality_tier(QualityTier::High)
            .build()
    }

    /// Oil film on water
    pub fn oil_on_water() -> CombinedMaterial {
        CombinedMaterial::builder()
            .add_fresnel(1.33) // Water
            .add_thin_film(1.47, 300.0, 1.33) // Oil film
            .add_roughness(0.02) // Slight surface ripples
            .blend_mode(BlendMode::PhysicallyBased)
            .build()
    }

    /// Pearl (nacre)
    pub fn pearl() -> CombinedMaterial {
        CombinedMaterial::builder()
            .add_fresnel(1.53) // Aragonite
            .add_thin_film(1.53, 400.0, 1.34) // Nacre layers
            .add_thin_film(1.34, 30.0, 1.53) // Protein
            .add_thin_film(1.53, 400.0, 1.34)
            .add_roughness(0.03)
            .blend_mode(BlendMode::PhysicallyBased)
            .build()
    }

    /// Simple glass
    pub fn glass() -> CombinedMaterial {
        CombinedMaterial::builder()
            .add_fresnel(1.52)
            .blend_mode(BlendMode::PhysicallyBased)
            .build()
    }

    /// Frosted glass
    pub fn frosted_glass() -> CombinedMaterial {
        CombinedMaterial::builder()
            .add_fresnel(1.52)
            .add_roughness(0.3)
            .add_mie(0.6, 0.2)
            .blend_mode(BlendMode::PhysicallyBased)
            .build()
    }
}

// ============================================================================
// MEMORY ESTIMATE
// ============================================================================

/// Estimate memory usage
pub fn total_combined_memory() -> usize {
    // CombinedMaterial base: ~64 bytes
    // Each EffectLayer: ~48 bytes
    // Typical material with 4 layers: ~256 bytes
    // 10 presets cached: ~2.5KB
    3_000
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder() {
        let material = CombinedMaterial::builder()
            .add_fresnel(1.5)
            .add_roughness(0.1)
            .blend_mode(BlendMode::PhysicallyBased)
            .build();

        assert_eq!(material.layer_count(), 2);
        assert_eq!(material.blend_mode, BlendMode::PhysicallyBased);
    }

    #[test]
    fn test_evaluate_fresnel() {
        let material = CombinedMaterial::builder().add_fresnel(1.5).build();

        let r_normal = material.evaluate(550.0, 1.0);
        let r_grazing = material.evaluate(550.0, 0.0);

        // Grazing should be higher than normal
        assert!(r_grazing > r_normal);
        assert!(r_normal > 0.0 && r_normal < 1.0);
    }

    #[test]
    fn test_evaluate_rgb() {
        let material = CombinedMaterial::builder().add_fresnel(1.5).build();

        let rgb = material.evaluate_rgb(0.7);

        assert!(rgb[0] >= 0.0 && rgb[0] <= 1.0);
        assert!(rgb[1] >= 0.0 && rgb[1] <= 1.0);
        assert!(rgb[2] >= 0.0 && rgb[2] <= 1.0);
    }

    #[test]
    fn test_spectral() {
        let material = CombinedMaterial::builder().add_fresnel(1.5).build();

        let spectrum = material.evaluate_spectral(0.7);

        assert_eq!(spectrum.len(), 31);
        assert!((spectrum[0].0 - 400.0).abs() < 0.1);
        assert!((spectrum[30].0 - 700.0).abs() < 0.1);
    }

    #[test]
    fn test_thin_film_iridescence() {
        let material = CombinedMaterial::builder()
            .add_fresnel(1.33)
            .add_thin_film(1.33, 350.0, 1.0)
            .build();

        let r_blue = material.evaluate(450.0, 0.8);
        let r_red = material.evaluate(650.0, 0.8);

        // Thin-film should show wavelength dependence
        assert!(r_blue != r_red);
    }

    #[test]
    fn test_metal() {
        let material = CombinedMaterial::builder()
            .add_metal(0.18, 3.0) // Gold-like
            .build();

        let r = material.evaluate(550.0, 1.0);
        assert!(r > 0.5); // Metals have high reflectance
    }

    #[test]
    fn test_blend_modes() {
        let base_material = CombinedMaterial::builder()
            .add_fresnel(1.5)
            .add_roughness(0.1);

        let additive = base_material
            .clone()
            .blend_mode(BlendMode::Additive)
            .build();
        let multiplicative = base_material
            .clone()
            .blend_mode(BlendMode::Multiplicative)
            .build();
        let physical = base_material.blend_mode(BlendMode::PhysicallyBased).build();

        let r_add = additive.evaluate(550.0, 0.7);
        let r_mul = multiplicative.evaluate(550.0, 0.7);
        let r_phys = physical.evaluate(550.0, 0.7);

        // All should produce valid results
        assert!(r_add >= 0.0 && r_add <= 1.0);
        assert!(r_mul >= 0.0 && r_mul <= 1.0);
        assert!(r_phys >= 0.0 && r_phys <= 1.0);
    }

    #[test]
    fn test_presets() {
        let soap = presets::soap_bubble();
        let patina = presets::copper_patina();
        let opal = presets::opal_glass();
        let morpho = presets::morpho_wing();

        // All presets should evaluate without error
        assert!(soap.evaluate(550.0, 0.7) >= 0.0);
        assert!(patina.evaluate(550.0, 0.7) >= 0.0);
        assert!(opal.evaluate(550.0, 0.7) >= 0.0);
        assert!(morpho.evaluate(550.0, 0.7) >= 0.0);
    }

    #[test]
    fn test_css_output() {
        let material = presets::glass();
        let css = material.to_css(30.0);

        assert!(css.contains("radial-gradient"));
        assert!(css.contains("rgb"));
    }

    #[test]
    fn test_css_animated() {
        let material = presets::soap_bubble();
        let css = material.to_css_animated(2.0);

        assert!(css.contains("@keyframes"));
        assert!(css.contains("iridescence"));
        assert!(css.contains("2s"));
    }

    #[test]
    fn test_memory_estimate() {
        let mem = total_combined_memory();
        assert!(mem > 0 && mem < 10_000);
    }
}

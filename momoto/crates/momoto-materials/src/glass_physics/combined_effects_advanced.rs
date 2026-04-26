//! # Advanced Combined Effects - Phase 7
//!
//! Extended effect compositor combining Phase 5 dynamics with Phase 6 effects
//! for ultra-realistic material rendering.
//!
//! ## New Effect Layers
//!
//! - **DynamicThinFilm**: Temperature/stress-responsive thin-film stacks
//! - **DynamicOxidation**: Time-evolving metal oxidation
//! - **MiePolydisperse**: Size-distribution particle scattering
//! - **SpectralDispersion**: Wavelength-dependent IOR (Cauchy/Sellmeier)
//! - **MechanicalDeformation**: Height-mapped surface effects
//! - **TemperatureGradient**: Spatially-varying thermal effects
//!
//! ## Usage
//!
//! ```rust
//! use momoto_materials::glass_physics::combined_effects_advanced::{
//!     AdvancedCombinedMaterial, AdvancedEffectLayer, PhysicalState,
//! };
//!
//! let material = AdvancedCombinedMaterial::builder()
//!     .add_dynamic_thin_film(1.33, 300.0, 1.0)
//!     .with_temperature(310.0)
//!     .add_mie_polydisperse(0.8, 0.1, 0.2)
//!     .build();
//!
//! let rgb = material.evaluate_rgb(0.7);
//! ```

use std::f64::consts::PI;

use super::combined_effects::{BlendMode, RoughnessModel};
use super::enhanced_presets::QualityTier;
use super::fresnel::fresnel_schlick;
use super::metal_oxidation_dynamic::{AlloyComposition, DynamicOxidizedMetal, Element};
use super::thin_film_dynamic::{
    DynamicFilmLayer, DynamicThinFilmStack, HeightMap, SubstrateProperties, Vec2,
};

// ============================================================================
// DISPERSION MODELS
// ============================================================================

/// Dispersion model for wavelength-dependent IOR
#[derive(Debug, Clone, Copy)]
pub enum DispersionModel {
    /// Cauchy: n(λ) = A + B/λ² + C/λ⁴
    Cauchy { a: f64, b: f64, c: f64 },
    /// Sellmeier: n²(λ) = 1 + Σ(B_i λ² / (λ² - C_i))
    Sellmeier { b1: f64, c1: f64, b2: f64, c2: f64 },
    /// Conrady: n(λ) = A + B/λ + C/λ^3.5
    Conrady { a: f64, b: f64, c: f64 },
    /// Constant IOR (no dispersion)
    Constant { n: f64 },
}

impl DispersionModel {
    /// BK7 crown glass (Sellmeier)
    pub fn bk7() -> Self {
        Self::Sellmeier {
            b1: 1.03961212,
            c1: 0.00600069867,
            b2: 0.231792344,
            c2: 0.0200179144,
        }
    }

    /// Fused silica (Sellmeier)
    pub fn fused_silica() -> Self {
        Self::Sellmeier {
            b1: 0.6961663,
            c1: 0.0684043,
            b2: 0.4079426,
            c2: 0.1162414,
        }
    }

    /// Water (Cauchy)
    pub fn water() -> Self {
        Self::Cauchy {
            a: 1.3199,
            b: 6878.0, // nm²
            c: 0.0,
        }
    }

    /// Diamond (high dispersion)
    pub fn diamond() -> Self {
        Self::Cauchy {
            a: 2.3789,
            b: 12800.0,
            c: 0.0,
        }
    }

    /// Evaluate IOR at wavelength (nm)
    pub fn evaluate(&self, wavelength_nm: f64) -> f64 {
        let lambda = wavelength_nm / 1000.0; // Convert to μm for Sellmeier
        let lambda_nm = wavelength_nm;

        match self {
            Self::Cauchy { a, b, c } => a + b / (lambda_nm * lambda_nm) + c / (lambda_nm.powi(4)),
            Self::Sellmeier { b1, c1, b2, c2 } => {
                let l2 = lambda * lambda;
                let n2 = 1.0 + (b1 * l2) / (l2 - c1) + (b2 * l2) / (l2 - c2);
                n2.sqrt()
            }
            Self::Conrady { a, b, c } => a + b / lambda_nm + c / lambda_nm.powf(3.5),
            Self::Constant { n } => *n,
        }
    }

    /// Calculate Abbe number (V_d)
    pub fn abbe_number(&self) -> f64 {
        let n_d = self.evaluate(587.6); // Yellow (helium d-line)
        let n_f = self.evaluate(486.1); // Blue (hydrogen F-line)
        let n_c = self.evaluate(656.3); // Red (hydrogen C-line)

        if (n_f - n_c).abs() < 1e-10 {
            0.0
        } else {
            (n_d - 1.0) / (n_f - n_c)
        }
    }
}

impl Default for DispersionModel {
    fn default() -> Self {
        Self::Constant { n: 1.5 }
    }
}

// ============================================================================
// SIZE DISTRIBUTION FOR MIE
// ============================================================================

/// Particle size distribution for polydisperse Mie scattering
#[derive(Debug, Clone, Copy)]
pub enum SizeDistribution {
    /// Log-normal distribution: mode radius (μm), geometric std dev
    LogNormal { r_mode: f64, sigma_g: f64 },
    /// Gamma distribution: effective radius, effective variance
    Gamma { r_eff: f64, v_eff: f64 },
    /// Monodisperse (single size)
    Monodisperse { radius: f64 },
}

impl SizeDistribution {
    /// Milk particles
    pub fn milk() -> Self {
        Self::LogNormal {
            r_mode: 0.5,
            sigma_g: 1.5,
        }
    }

    /// Fog droplets
    pub fn fog() -> Self {
        Self::LogNormal {
            r_mode: 5.0,
            sigma_g: 1.3,
        }
    }

    /// Dust particles
    pub fn dust() -> Self {
        Self::LogNormal {
            r_mode: 1.0,
            sigma_g: 2.0,
        }
    }

    /// Calculate effective g parameter for Henyey-Greenstein
    pub fn effective_g(&self, base_g: f64) -> f64 {
        match self {
            Self::LogNormal { sigma_g, .. } => {
                // Broader distribution -> more isotropic scattering
                base_g * (1.0 / sigma_g).clamp(0.5, 1.0)
            }
            Self::Gamma { v_eff, .. } => {
                // Higher variance -> more isotropic
                base_g * (1.0 - v_eff).clamp(0.3, 1.0)
            }
            Self::Monodisperse { .. } => base_g,
        }
    }

    /// Calculate extinction scaling factor
    pub fn extinction_factor(&self) -> f64 {
        match self {
            Self::LogNormal { sigma_g, .. } => {
                // Polydisperse enhances extinction
                1.0 + 0.2 * (sigma_g - 1.0)
            }
            Self::Gamma { v_eff, .. } => 1.0 + 0.3 * v_eff,
            Self::Monodisperse { .. } => 1.0,
        }
    }
}

impl Default for SizeDistribution {
    fn default() -> Self {
        Self::Monodisperse { radius: 1.0 }
    }
}

// ============================================================================
// TEMPERATURE GRADIENT
// ============================================================================

/// Temperature gradient types
#[derive(Debug, Clone, Copy)]
pub enum GradientType {
    /// Linear from center to edge
    Linear,
    /// Radial (circular symmetry)
    Radial,
    /// Gaussian profile
    Gaussian { sigma: f64 },
}

impl Default for GradientType {
    fn default() -> Self {
        Self::Linear
    }
}

/// Temperature gradient configuration
#[derive(Debug, Clone, Copy)]
pub struct TemperatureGradientConfig {
    pub center_temp_k: f64,
    pub edge_temp_k: f64,
    pub gradient_type: GradientType,
}

impl TemperatureGradientConfig {
    /// Evaluate temperature at position (0-1 normalized)
    pub fn temperature_at(&self, pos: Vec2) -> f64 {
        let r = ((pos.x - 0.5).powi(2) + (pos.y - 0.5).powi(2)).sqrt() * 2.0;
        let r = r.clamp(0.0, 1.0);

        let t = match self.gradient_type {
            GradientType::Linear => r,
            GradientType::Radial => r,
            GradientType::Gaussian { sigma } => 1.0 - (-r * r / (2.0 * sigma * sigma)).exp(),
        };

        self.center_temp_k + t * (self.edge_temp_k - self.center_temp_k)
    }
}

impl Default for TemperatureGradientConfig {
    fn default() -> Self {
        Self {
            center_temp_k: 293.0,
            edge_temp_k: 293.0,
            gradient_type: GradientType::Linear,
        }
    }
}

// ============================================================================
// PHYSICAL STATE
// ============================================================================

/// Physical state of the material
#[derive(Debug, Clone)]
pub struct PhysicalState {
    /// Temperature (K)
    pub temperature_k: f64,
    /// Relative humidity (0-1)
    pub humidity: f64,
    /// Age since exposure (seconds)
    pub age_seconds: f64,
    /// Stress tensor in Voigt notation (MPa)
    pub stress: [f64; 6],
    /// Atmospheric oxygen partial pressure (atm)
    pub oxygen_pressure: f64,
    /// Temperature gradient (optional)
    pub temp_gradient: Option<TemperatureGradientConfig>,
}

impl Default for PhysicalState {
    fn default() -> Self {
        Self {
            temperature_k: 293.0,
            humidity: 0.5,
            age_seconds: 0.0,
            stress: [0.0; 6],
            oxygen_pressure: 0.21,
            temp_gradient: None,
        }
    }
}

impl PhysicalState {
    /// Create with temperature
    pub fn with_temperature(mut self, temp_k: f64) -> Self {
        self.temperature_k = temp_k;
        self
    }

    /// Create with humidity
    pub fn with_humidity(mut self, humidity: f64) -> Self {
        self.humidity = humidity.clamp(0.0, 1.0);
        self
    }

    /// Create with age
    pub fn with_age_seconds(mut self, age: f64) -> Self {
        self.age_seconds = age;
        self
    }

    /// Create with stress
    pub fn with_stress(mut self, stress: [f64; 6]) -> Self {
        self.stress = stress;
        self
    }

    /// Age in days
    pub fn age_days(&self) -> f64 {
        self.age_seconds / 86400.0
    }

    /// Age in years
    pub fn age_years(&self) -> f64 {
        self.age_seconds / (86400.0 * 365.25)
    }
}

// ============================================================================
// ADVANCED EFFECT LAYERS
// ============================================================================

/// Advanced effect layer with Phase 5 dynamics
#[derive(Debug, Clone)]
pub enum AdvancedEffectLayer {
    // === Phase 6 layers (inherited) ===
    /// Base Fresnel reflection
    Fresnel { ior: f64, spectral: bool },

    /// Thin-film interference (static)
    ThinFilm {
        n_film: f64,
        thickness_nm: f64,
        n_substrate: f64,
    },

    /// Metal with complex IOR
    Metal { n: f64, k: f64 },

    /// Mie scattering (monodisperse)
    Mie { g: f64, extinction: f64 },

    /// Surface roughness
    Roughness { value: f64, model: RoughnessModel },

    /// Absorption (Beer-Lambert)
    Absorption { coefficient: f64, thickness: f64 },

    /// Static oxidation layer
    Oxidation {
        oxide_n: f64,
        oxide_k: f64,
        thickness_nm: f64,
    },

    // === Phase 7 NEW layers ===
    /// Dynamic thin-film stack with temperature/stress response
    DynamicThinFilm { stack: DynamicThinFilmStack },

    /// Dynamic metal oxidation with time evolution
    DynamicOxidation { metal: DynamicOxidizedMetal },

    /// Polydisperse Mie scattering
    MiePolydisperse {
        distribution: SizeDistribution,
        g_mean: f64,
        extinction: f64,
    },

    /// Spectral dispersion (wavelength-dependent IOR)
    SpectralDispersion { dispersion: DispersionModel },

    /// Mechanical deformation via height map
    MechanicalDeformation {
        height_map: HeightMap,
        amplitude: f64,
    },

    /// Temperature gradient effects
    TemperatureGradient {
        config: TemperatureGradientConfig,
        dn_dt: f64, // Thermo-optic coefficient
    },
}

// ============================================================================
// ADVANCED COMBINED MATERIAL
// ============================================================================

/// Advanced combined material with Phase 7 features
#[derive(Debug, Clone)]
pub struct AdvancedCombinedMaterial {
    /// Effect layers
    pub layers: Vec<AdvancedEffectLayer>,
    /// Blend mode
    pub blend_mode: BlendMode,
    /// Quality tier
    pub quality_tier: QualityTier,
    /// Physical state
    pub physical_state: PhysicalState,
    /// Base IOR
    base_ior: f64,
}

impl AdvancedCombinedMaterial {
    /// Create builder
    pub fn builder() -> AdvancedCombinedMaterialBuilder {
        AdvancedCombinedMaterialBuilder::new()
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

    /// Evaluate at position (for spatially-varying effects)
    pub fn evaluate_at(&self, wavelength_nm: f64, cos_theta: f64, pos: Vec2) -> f64 {
        let cos_theta = cos_theta.clamp(0.0, 1.0);
        self.evaluate_physically_based_at(wavelength_nm, cos_theta, pos)
    }

    /// Evaluate RGB reflectance
    pub fn evaluate_rgb(&self, cos_theta: f64) -> [f64; 3] {
        [
            self.evaluate(650.0, cos_theta),
            self.evaluate(550.0, cos_theta),
            self.evaluate(450.0, cos_theta),
        ]
    }

    /// Evaluate RGB at position
    pub fn evaluate_rgb_at(&self, cos_theta: f64, pos: Vec2) -> [f64; 3] {
        [
            self.evaluate_at(650.0, cos_theta, pos),
            self.evaluate_at(550.0, cos_theta, pos),
            self.evaluate_at(450.0, cos_theta, pos),
        ]
    }

    /// Evaluate full spectrum (31 points)
    pub fn evaluate_spectral(&self, cos_theta: f64) -> Vec<(f64, f64)> {
        (0..31)
            .map(|i| {
                let w = 400.0 + i as f64 * 10.0;
                (w, self.evaluate(w, cos_theta))
            })
            .collect()
    }

    // Blend mode implementations

    fn evaluate_additive(&self, wavelength_nm: f64, cos_theta: f64) -> f64 {
        let mut total = 0.0;
        for layer in &self.layers {
            total += self.evaluate_layer(layer, wavelength_nm, cos_theta, Vec2::new(0.5, 0.5));
        }
        total.min(1.0)
    }

    fn evaluate_multiplicative(&self, wavelength_nm: f64, cos_theta: f64) -> f64 {
        let mut total = 1.0;
        for layer in &self.layers {
            total *= self.evaluate_layer(layer, wavelength_nm, cos_theta, Vec2::new(0.5, 0.5));
        }
        total
    }

    fn evaluate_fresnel_weighted(&self, wavelength_nm: f64, cos_theta: f64) -> f64 {
        let fresnel_weight = fresnel_schlick(1.0, self.base_ior, cos_theta);
        let mut total = 0.0;
        for layer in &self.layers {
            let layer_value =
                self.evaluate_layer(layer, wavelength_nm, cos_theta, Vec2::new(0.5, 0.5));
            total += layer_value * fresnel_weight;
        }
        total.min(1.0)
    }

    fn evaluate_physically_based(&self, wavelength_nm: f64, cos_theta: f64) -> f64 {
        self.evaluate_physically_based_at(wavelength_nm, cos_theta, Vec2::new(0.5, 0.5))
    }

    fn evaluate_physically_based_at(&self, wavelength_nm: f64, cos_theta: f64, pos: Vec2) -> f64 {
        let mut reflectance = 0.0;
        let mut transmittance = 1.0;

        for layer in &self.layers {
            let layer_r = self.evaluate_layer(layer, wavelength_nm, cos_theta, pos);

            reflectance += transmittance * transmittance * layer_r;
            transmittance *= 1.0 - layer_r;

            if transmittance < 0.001 {
                break;
            }
        }

        reflectance.min(1.0)
    }

    /// Evaluate single layer
    fn evaluate_layer(
        &self,
        layer: &AdvancedEffectLayer,
        wavelength_nm: f64,
        cos_theta: f64,
        pos: Vec2,
    ) -> f64 {
        match layer {
            // Phase 6 layers
            AdvancedEffectLayer::Fresnel { ior, spectral: _ } => {
                fresnel_schlick(1.0, *ior, cos_theta)
            }

            AdvancedEffectLayer::ThinFilm {
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

            AdvancedEffectLayer::Metal { n, k } => metal_fresnel(*n, *k, cos_theta),

            AdvancedEffectLayer::Mie { g, extinction } => {
                henyey_greenstein_phase(cos_theta, *g) * (1.0 - (-*extinction).exp())
            }

            AdvancedEffectLayer::Roughness { value, model } => {
                roughness_factor(cos_theta, *value, *model)
            }

            AdvancedEffectLayer::Absorption {
                coefficient,
                thickness,
            } => (-*coefficient * *thickness).exp(),

            AdvancedEffectLayer::Oxidation {
                oxide_n,
                oxide_k,
                thickness_nm,
            } => oxide_reflectance(*oxide_n, *oxide_k, *thickness_nm, wavelength_nm),

            // Phase 7 layers
            AdvancedEffectLayer::DynamicThinFilm { stack } => {
                stack.reflectance_at(pos, wavelength_nm, self.angle_from_cos(cos_theta))
            }

            AdvancedEffectLayer::DynamicOxidation { metal } => metal.reflectance(wavelength_nm),

            AdvancedEffectLayer::MiePolydisperse {
                distribution,
                g_mean,
                extinction,
            } => {
                let effective_g = distribution.effective_g(*g_mean);
                let effective_ext = extinction * distribution.extinction_factor();
                henyey_greenstein_phase(cos_theta, effective_g) * (1.0 - (-effective_ext).exp())
            }

            AdvancedEffectLayer::SpectralDispersion { dispersion } => {
                let n = dispersion.evaluate(wavelength_nm);
                fresnel_schlick(1.0, n, cos_theta)
            }

            AdvancedEffectLayer::MechanicalDeformation {
                height_map,
                amplitude,
            } => {
                let h = height_map.sample(pos);
                let normal = height_map.normal(pos);
                // Adjust effective angle based on surface normal
                let effective_cos = (cos_theta * normal[2]).max(0.0);
                // Modulate reflectance by height
                let height_factor = 1.0 + h * amplitude * 0.1;
                fresnel_schlick(1.0, self.base_ior, effective_cos) * height_factor.clamp(0.5, 2.0)
            }

            AdvancedEffectLayer::TemperatureGradient { config, dn_dt } => {
                let temp = config.temperature_at(pos);
                let delta_t = temp - 293.0; // Reference temp
                let n_effective = self.base_ior + dn_dt * delta_t;
                fresnel_schlick(1.0, n_effective, cos_theta)
            }
        }
    }

    fn angle_from_cos(&self, cos_theta: f64) -> f64 {
        cos_theta.acos().to_degrees()
    }

    /// Update physical state
    pub fn set_physical_state(&mut self, state: PhysicalState) {
        self.physical_state = state.clone();

        // Propagate state to dynamic layers
        for layer in &mut self.layers {
            match layer {
                AdvancedEffectLayer::DynamicThinFilm { stack } => {
                    stack.set_environment(
                        state.temperature_k,
                        101325.0, // Standard pressure
                        state.humidity,
                    );
                    stack.apply_stress(state.stress);
                }
                AdvancedEffectLayer::DynamicOxidation { metal } => {
                    metal.set_environment(
                        state.temperature_k,
                        state.humidity,
                        state.oxygen_pressure,
                    );
                    if state.age_seconds > 0.0 {
                        metal.advance_time(state.age_seconds);
                    }
                }
                _ => {}
            }
        }
    }

    /// Advance time for dynamic effects
    pub fn advance_time(&mut self, dt_seconds: f64) {
        self.physical_state.age_seconds += dt_seconds;

        for layer in &mut self.layers {
            if let AdvancedEffectLayer::DynamicOxidation { metal } = layer {
                metal.advance_time(dt_seconds);
            }
        }
    }

    /// Number of layers
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    /// Base IOR
    pub fn base_ior(&self) -> f64 {
        self.base_ior
    }

    /// Generate CSS gradient
    pub fn to_css(&self, angle_deg: f64) -> String {
        let cos_theta = (angle_deg * PI / 180.0).cos();
        let rgb = self.evaluate_rgb(cos_theta);

        let r = (rgb[0] * 255.0).round() as u8;
        let g = (rgb[1] * 255.0).round() as u8;
        let b = (rgb[2] * 255.0).round() as u8;

        let center_rgb = self.evaluate_rgb(1.0);
        let cr = (center_rgb[0] * 255.0).round() as u8;
        let cg = (center_rgb[1] * 255.0).round() as u8;
        let cb = (center_rgb[2] * 255.0).round() as u8;

        format!(
            "radial-gradient(ellipse at 30% 30%, rgb({}, {}, {}) 0%, rgb({}, {}, {}) 100%)",
            cr, cg, cb, r, g, b
        )
    }
}

impl Default for AdvancedCombinedMaterial {
    fn default() -> Self {
        Self {
            layers: vec![AdvancedEffectLayer::Fresnel {
                ior: 1.5,
                spectral: false,
            }],
            blend_mode: BlendMode::PhysicallyBased,
            quality_tier: QualityTier::High,
            physical_state: PhysicalState::default(),
            base_ior: 1.5,
        }
    }
}

// ============================================================================
// BUILDER
// ============================================================================

/// Builder for AdvancedCombinedMaterial
#[derive(Debug, Clone)]
pub struct AdvancedCombinedMaterialBuilder {
    layers: Vec<AdvancedEffectLayer>,
    blend_mode: BlendMode,
    quality_tier: QualityTier,
    physical_state: PhysicalState,
    base_ior: f64,
}

impl AdvancedCombinedMaterialBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            layers: Vec::new(),
            blend_mode: BlendMode::PhysicallyBased,
            quality_tier: QualityTier::High,
            physical_state: PhysicalState::default(),
            base_ior: 1.5,
        }
    }

    // Phase 6 layer methods

    /// Add Fresnel layer
    pub fn add_fresnel(mut self, ior: f64) -> Self {
        self.base_ior = ior;
        self.layers.push(AdvancedEffectLayer::Fresnel {
            ior,
            spectral: false,
        });
        self
    }

    /// Add thin-film layer
    pub fn add_thin_film(mut self, n_film: f64, thickness_nm: f64, n_substrate: f64) -> Self {
        self.layers.push(AdvancedEffectLayer::ThinFilm {
            n_film,
            thickness_nm,
            n_substrate,
        });
        self
    }

    /// Add metal layer
    pub fn add_metal(mut self, n: f64, k: f64) -> Self {
        self.layers.push(AdvancedEffectLayer::Metal { n, k });
        self
    }

    /// Add Mie scattering layer
    pub fn add_mie(mut self, g: f64, extinction: f64) -> Self {
        self.layers.push(AdvancedEffectLayer::Mie { g, extinction });
        self
    }

    /// Add roughness layer
    pub fn add_roughness(mut self, value: f64) -> Self {
        self.layers.push(AdvancedEffectLayer::Roughness {
            value,
            model: RoughnessModel::GGX,
        });
        self
    }

    /// Add absorption layer
    pub fn add_absorption(mut self, coefficient: f64, thickness: f64) -> Self {
        self.layers.push(AdvancedEffectLayer::Absorption {
            coefficient,
            thickness,
        });
        self
    }

    /// Add oxidation layer
    pub fn add_oxidation(mut self, oxide_n: f64, oxide_k: f64, thickness_nm: f64) -> Self {
        self.layers.push(AdvancedEffectLayer::Oxidation {
            oxide_n,
            oxide_k,
            thickness_nm,
        });
        self
    }

    // Phase 7 layer methods

    /// Add dynamic thin-film stack
    pub fn add_dynamic_thin_film(
        mut self,
        n_film: f64,
        thickness_nm: f64,
        n_substrate: f64,
    ) -> Self {
        let substrate = SubstrateProperties {
            n: n_substrate,
            k: 0.0,
            alpha: 5e-6,
        };
        let mut stack = DynamicThinFilmStack::new(1.0, substrate);
        stack.add_layer(DynamicFilmLayer::new(n_film, thickness_nm));
        self.layers
            .push(AdvancedEffectLayer::DynamicThinFilm { stack });
        self.base_ior = n_film;
        self
    }

    /// Add dynamic thin-film stack with custom stack
    pub fn add_dynamic_thin_film_stack(mut self, stack: DynamicThinFilmStack) -> Self {
        self.layers
            .push(AdvancedEffectLayer::DynamicThinFilm { stack });
        self
    }

    /// Add dynamic oxidation layer
    pub fn add_dynamic_oxidation(mut self, element: Element) -> Self {
        let metal = DynamicOxidizedMetal::pure(element);
        self.layers
            .push(AdvancedEffectLayer::DynamicOxidation { metal });
        self
    }

    /// Add dynamic oxidation with alloy
    pub fn add_dynamic_oxidation_alloy(mut self, composition: AlloyComposition) -> Self {
        let metal = DynamicOxidizedMetal::new(composition);
        self.layers
            .push(AdvancedEffectLayer::DynamicOxidation { metal });
        self
    }

    /// Add polydisperse Mie scattering
    pub fn add_mie_polydisperse(mut self, g_mean: f64, extinction: f64, sigma: f64) -> Self {
        let distribution = SizeDistribution::LogNormal {
            r_mode: 1.0,
            sigma_g: 1.0 + sigma,
        };
        self.layers.push(AdvancedEffectLayer::MiePolydisperse {
            distribution,
            g_mean,
            extinction,
        });
        self
    }

    /// Add polydisperse Mie with custom distribution
    pub fn add_mie_polydisperse_custom(
        mut self,
        distribution: SizeDistribution,
        g_mean: f64,
        extinction: f64,
    ) -> Self {
        self.layers.push(AdvancedEffectLayer::MiePolydisperse {
            distribution,
            g_mean,
            extinction,
        });
        self
    }

    /// Add spectral dispersion
    pub fn add_spectral_dispersion(mut self, dispersion: DispersionModel) -> Self {
        self.layers
            .push(AdvancedEffectLayer::SpectralDispersion { dispersion });
        self
    }

    /// Add spectral dispersion with Abbe number
    pub fn add_dispersion_abbe(mut self, n_d: f64, v_d: f64) -> Self {
        // Approximate Cauchy coefficients from Abbe number
        let b = (n_d - 1.0) / v_d * 1e6; // Convert to nm²
        let dispersion = DispersionModel::Cauchy { a: n_d, b, c: 0.0 };
        self.base_ior = n_d;
        self.layers
            .push(AdvancedEffectLayer::SpectralDispersion { dispersion });
        self
    }

    /// Add mechanical deformation
    pub fn add_mechanical_deformation(mut self, height_map: HeightMap, amplitude: f64) -> Self {
        self.layers
            .push(AdvancedEffectLayer::MechanicalDeformation {
                height_map,
                amplitude,
            });
        self
    }

    /// Add temperature gradient
    pub fn add_temperature_gradient(
        mut self,
        center_temp: f64,
        edge_temp: f64,
        dn_dt: f64,
    ) -> Self {
        let config = TemperatureGradientConfig {
            center_temp_k: center_temp,
            edge_temp_k: edge_temp,
            gradient_type: GradientType::Radial,
        };
        self.layers
            .push(AdvancedEffectLayer::TemperatureGradient { config, dn_dt });
        self
    }

    // State methods

    /// Set temperature
    pub fn with_temperature(mut self, temp_k: f64) -> Self {
        self.physical_state.temperature_k = temp_k;
        self
    }

    /// Set humidity
    pub fn with_humidity(mut self, humidity: f64) -> Self {
        self.physical_state.humidity = humidity.clamp(0.0, 1.0);
        self
    }

    /// Set age
    pub fn with_age_seconds(mut self, age: f64) -> Self {
        self.physical_state.age_seconds = age;
        self
    }

    /// Set stress
    pub fn with_stress(mut self, stress: [f64; 6]) -> Self {
        self.physical_state.stress = stress;
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
    pub fn build(self) -> AdvancedCombinedMaterial {
        let mut material = AdvancedCombinedMaterial {
            layers: self.layers,
            blend_mode: self.blend_mode,
            quality_tier: self.quality_tier,
            physical_state: self.physical_state.clone(),
            base_ior: self.base_ior,
        };

        // Apply initial physical state
        material.set_physical_state(self.physical_state);

        material
    }
}

impl Default for AdvancedCombinedMaterialBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// HELPER FUNCTIONS (copied from combined_effects.rs)
// ============================================================================

/// Thin-film interference reflectance
fn thin_film_reflectance(
    wavelength_nm: f64,
    n_film: f64,
    thickness_nm: f64,
    n_substrate: f64,
    cos_theta: f64,
) -> f64 {
    let delta = 4.0 * PI * n_film * thickness_nm * cos_theta / wavelength_nm;
    let r1 = (1.0 - n_film) / (1.0 + n_film);
    let r2 = (n_film - n_substrate) / (n_film + n_substrate);
    let numerator = r1 * r1 + r2 * r2 + 2.0 * r1 * r2 * delta.cos();
    let denominator = 1.0 + r1 * r1 * r2 * r2 + 2.0 * r1 * r2 * delta.cos();
    (numerator / denominator).abs().min(1.0)
}

/// Metal Fresnel reflectance
fn metal_fresnel(n: f64, k: f64, cos_theta: f64) -> f64 {
    let f0 = ((n - 1.0).powi(2) + k.powi(2)) / ((n + 1.0).powi(2) + k.powi(2));
    let one_minus_cos = 1.0 - cos_theta;
    f0 + (1.0 - f0) * one_minus_cos.powi(5)
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

/// Roughness factor
fn roughness_factor(cos_theta: f64, roughness: f64, model: RoughnessModel) -> f64 {
    let alpha = roughness * roughness;
    match model {
        RoughnessModel::GGX => {
            let k = alpha / 2.0;
            cos_theta / (cos_theta * (1.0 - k) + k)
        }
        RoughnessModel::Beckmann => {
            let c = cos_theta / (alpha * (1.0 - cos_theta * cos_theta).sqrt().max(0.001));
            if c < 1.6 {
                (3.535 * c + 2.181 * c * c) / (1.0 + 2.276 * c + 2.577 * c * c)
            } else {
                1.0
            }
        }
        RoughnessModel::BlinnPhong => cos_theta.powf(1.0 / roughness.max(0.01)),
    }
}

/// Oxide layer reflectance
fn oxide_reflectance(n: f64, k: f64, thickness_nm: f64, wavelength_nm: f64) -> f64 {
    let phase = 4.0 * PI * n * thickness_nm / wavelength_nm;
    let r1 = ((n - 1.0).powi(2) + k.powi(2)) / ((n + 1.0).powi(2) + k.powi(2));
    let interference = 0.5 * (1.0 + phase.cos() * 0.3);
    let absorption = (-k * thickness_nm / 100.0).exp();
    r1 * interference * absorption
}

// ============================================================================
// MEMORY ESTIMATE
// ============================================================================

/// Estimate memory usage for advanced effects
pub fn total_advanced_memory() -> usize {
    // Base AdvancedCombinedMaterial: ~128 bytes
    // Each advanced layer: ~64-256 bytes depending on type
    // DynamicThinFilmStack: ~512 bytes
    // DynamicOxidizedMetal: ~256 bytes
    // PhysicalState: ~64 bytes
    // Typical advanced material: ~1KB
    // Cache for experimental presets: ~8KB
    8_000
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispersion_model_bk7() {
        let bk7 = DispersionModel::bk7();

        let n_d = bk7.evaluate(587.6);
        assert!(n_d > 1.5 && n_d < 1.6, "BK7 n_d = {}", n_d);

        let abbe = bk7.abbe_number();
        assert!(abbe > 60.0 && abbe < 70.0, "BK7 V_d = {}", abbe);
    }

    #[test]
    fn test_dispersion_model_water() {
        let water = DispersionModel::water();

        let n_d = water.evaluate(550.0);
        // Water IOR at 550nm is approximately 1.33-1.35 depending on model
        assert!(n_d > 1.32 && n_d < 1.36, "Water n = {}", n_d);
    }

    #[test]
    fn test_size_distribution_effective_g() {
        let mono = SizeDistribution::Monodisperse { radius: 1.0 };
        let poly = SizeDistribution::LogNormal {
            r_mode: 1.0,
            sigma_g: 1.5,
        };

        let g_mono = mono.effective_g(0.8);
        let g_poly = poly.effective_g(0.8);

        // Polydisperse should have lower effective g (more isotropic)
        assert!(g_poly <= g_mono);
    }

    #[test]
    fn test_temperature_gradient() {
        let config = TemperatureGradientConfig {
            center_temp_k: 400.0,
            edge_temp_k: 300.0,
            gradient_type: GradientType::Radial,
        };

        let t_center = config.temperature_at(Vec2::new(0.5, 0.5));
        let t_edge = config.temperature_at(Vec2::new(0.0, 0.5));

        assert!((t_center - 400.0).abs() < 1.0);
        assert!(t_edge < t_center);
    }

    #[test]
    fn test_physical_state() {
        let state = PhysicalState::default()
            .with_temperature(350.0)
            .with_humidity(0.8)
            .with_age_seconds(86400.0);

        assert!((state.temperature_k - 350.0).abs() < 0.1);
        assert!((state.humidity - 0.8).abs() < 0.01);
        assert!((state.age_days() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_builder_basic() {
        let material = AdvancedCombinedMaterial::builder()
            .add_fresnel(1.5)
            .add_roughness(0.1)
            .build();

        assert_eq!(material.layer_count(), 2);
        assert_eq!(material.blend_mode, BlendMode::PhysicallyBased);
    }

    #[test]
    fn test_builder_dynamic_thin_film() {
        let material = AdvancedCombinedMaterial::builder()
            .add_dynamic_thin_film(1.33, 300.0, 1.0)
            .with_temperature(310.0)
            .build();

        assert_eq!(material.layer_count(), 1);
        assert!((material.physical_state.temperature_k - 310.0).abs() < 0.1);
    }

    #[test]
    fn test_builder_dynamic_oxidation() {
        let material = AdvancedCombinedMaterial::builder()
            .add_dynamic_oxidation(Element::Cu)
            .with_age_seconds(86400.0)
            .with_humidity(0.7)
            .build();

        assert_eq!(material.layer_count(), 1);
    }

    #[test]
    fn test_builder_polydisperse_mie() {
        let material = AdvancedCombinedMaterial::builder()
            .add_fresnel(1.5)
            .add_mie_polydisperse(0.8, 0.1, 0.5)
            .build();

        assert_eq!(material.layer_count(), 2);
    }

    #[test]
    fn test_builder_spectral_dispersion() {
        let material = AdvancedCombinedMaterial::builder()
            .add_spectral_dispersion(DispersionModel::bk7())
            .build();

        // Should show wavelength dependence
        let r_blue = material.evaluate(450.0, 0.8);
        let r_red = material.evaluate(650.0, 0.8);
        assert!(r_blue != r_red);
    }

    #[test]
    fn test_evaluate_rgb() {
        let material = AdvancedCombinedMaterial::builder().add_fresnel(1.5).build();

        let rgb = material.evaluate_rgb(0.7);

        assert!(rgb[0] >= 0.0 && rgb[0] <= 1.0);
        assert!(rgb[1] >= 0.0 && rgb[1] <= 1.0);
        assert!(rgb[2] >= 0.0 && rgb[2] <= 1.0);
    }

    #[test]
    fn test_evaluate_spectral() {
        let material = AdvancedCombinedMaterial::builder().add_fresnel(1.5).build();

        let spectrum = material.evaluate_spectral(0.7);
        assert_eq!(spectrum.len(), 31);
    }

    #[test]
    fn test_css_output() {
        let material = AdvancedCombinedMaterial::builder().add_fresnel(1.5).build();

        let css = material.to_css(30.0);
        assert!(css.contains("radial-gradient"));
    }

    #[test]
    fn test_advance_time() {
        let mut material = AdvancedCombinedMaterial::builder()
            .add_dynamic_oxidation(Element::Cu)
            .with_humidity(0.7)
            .build();

        let initial_age = material.physical_state.age_seconds;
        material.advance_time(86400.0);

        assert!(material.physical_state.age_seconds > initial_age);
    }

    #[test]
    fn test_memory_estimate() {
        let mem = total_advanced_memory();
        assert!(mem > 0 && mem < 20_000);
    }
}

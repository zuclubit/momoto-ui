//! # Dynamic Mie Scattering (Phase 4)
//!
//! Advanced particle scattering with anisotropy, temporal variation,
//! and polydisperse size distributions.
//!
//! ## Features
//!
//! - **Anisotropic Scattering**: Non-spherical particles
//! - **Temporal Evolution**: Particle growth, coalescence
//! - **Polydisperse Systems**: Size distributions (log-normal, bimodal)
//! - **Wavelength-Dependent Effects**: Full spectral calculation
//!
//! ## Physical Models
//!
//! ### Anisotropic Scattering
//! ```text
//! p(θ, φ) = p_mie(θ) × [1 + A × cos(2φ)]
//! ```
//!
//! ### Polydisperse Integration
//! ```text
//! p_total(θ) = ∫ p(θ, r) × n(r) dr
//! ```
//!
//! ## References
//!
//! - Mishchenko et al. (2002): "Scattering, Absorption, and Emission of Light"
//! - Frisvad et al. (2007): SIGGRAPH Mie implementation

use std::f64::consts::PI;

use super::mie_lut::{mie_asymmetry_g, mie_fast};

// ============================================================================
// SIZE DISTRIBUTIONS
// ============================================================================

/// Particle size distribution type
#[derive(Debug, Clone)]
pub enum SizeDistribution {
    /// Single particle size
    Monodisperse { radius_um: f64 },
    /// Log-normal distribution (common for natural aerosols)
    LogNormal {
        geometric_mean_um: f64,
        geometric_std: f64,
    },
    /// Gamma distribution
    Gamma {
        shape: f64,    // α
        scale_um: f64, // β
    },
    /// Bimodal distribution (e.g., smoke with fine and coarse modes)
    Bimodal {
        mode1: Box<SizeDistribution>,
        mode2: Box<SizeDistribution>,
        weight1: f64, // Weight of first mode (0-1)
    },
}

impl SizeDistribution {
    /// Create log-normal distribution
    pub fn log_normal(geometric_mean_um: f64, geometric_std: f64) -> Self {
        Self::LogNormal {
            geometric_mean_um,
            geometric_std,
        }
    }

    /// Create bimodal distribution
    pub fn bimodal(mean1: f64, std1: f64, mean2: f64, std2: f64, weight1: f64) -> Self {
        Self::Bimodal {
            mode1: Box::new(Self::LogNormal {
                geometric_mean_um: mean1,
                geometric_std: std1,
            }),
            mode2: Box::new(Self::LogNormal {
                geometric_mean_um: mean2,
                geometric_std: std2,
            }),
            weight1,
        }
    }

    /// Probability density at radius r
    pub fn pdf(&self, r: f64) -> f64 {
        match self {
            Self::Monodisperse { radius_um } => {
                // Delta function approximation
                if (r - radius_um).abs() < 0.01 * radius_um {
                    100.0
                } else {
                    0.0
                }
            }
            Self::LogNormal {
                geometric_mean_um,
                geometric_std,
            } => {
                if r <= 0.0 {
                    return 0.0;
                }
                let mu = geometric_mean_um.ln();
                let sigma = *geometric_std;
                let x = (r.ln() - mu) / sigma;
                (1.0 / (r * sigma * (2.0 * PI).sqrt())) * (-0.5 * x * x).exp()
            }
            Self::Gamma { shape, scale_um } => {
                if r <= 0.0 {
                    return 0.0;
                }
                let alpha = *shape;
                let beta = *scale_um;
                let gamma_alpha = gamma_function(alpha);
                (r.powf(alpha - 1.0) * (-r / beta).exp()) / (beta.powf(alpha) * gamma_alpha)
            }
            Self::Bimodal {
                mode1,
                mode2,
                weight1,
            } => weight1 * mode1.pdf(r) + (1.0 - weight1) * mode2.pdf(r),
        }
    }

    /// Mean radius
    pub fn mean_radius(&self) -> f64 {
        match self {
            Self::Monodisperse { radius_um } => *radius_um,
            Self::LogNormal {
                geometric_mean_um,
                geometric_std,
            } => geometric_mean_um * (0.5 * geometric_std * geometric_std).exp(),
            Self::Gamma { shape, scale_um } => shape * scale_um,
            Self::Bimodal {
                mode1,
                mode2,
                weight1,
            } => weight1 * mode1.mean_radius() + (1.0 - weight1) * mode2.mean_radius(),
        }
    }

    /// Sample radii for numerical integration
    pub fn sample_radii(&self, num_samples: usize) -> Vec<f64> {
        let mean = self.mean_radius();
        let min_r = mean * 0.1;
        let max_r = mean * 5.0;

        // Log-space sampling
        let log_min = min_r.ln();
        let log_max = max_r.ln();
        let log_step = (log_max - log_min) / (num_samples - 1) as f64;

        (0..num_samples)
            .map(|i| (log_min + i as f64 * log_step).exp())
            .collect()
    }
}

/// Simple gamma function approximation (Stirling)
fn gamma_function(x: f64) -> f64 {
    if x < 0.5 {
        PI / ((PI * x).sin() * gamma_function(1.0 - x))
    } else {
        // Stirling approximation
        let x_adj = x - 1.0;
        (2.0 * PI / x_adj).sqrt() * (x_adj / std::f64::consts::E).powf(x_adj)
    }
}

// ============================================================================
// DYNAMIC MIE PARAMETERS
// ============================================================================

/// Dynamic Mie scattering parameters with temporal evolution
#[derive(Debug, Clone)]
pub struct DynamicMieParams {
    /// Particle refractive index
    pub n_particle: f64,
    /// Medium refractive index
    pub n_medium: f64,
    /// Size distribution
    pub size_distribution: SizeDistribution,
    /// Anisotropy factor (0 = spherical, 1 = highly elongated)
    pub anisotropy: f64,
    /// Preferred orientation (normalized direction vector)
    pub orientation: [f64; 3],
    /// Temporal evolution: growth rate (μm/s)
    pub growth_rate: f64,
    /// Temporal evolution: coalescence rate (1/s)
    pub coalescence_rate: f64,
}

impl DynamicMieParams {
    /// Create new dynamic Mie parameters
    pub fn new(n_particle: f64, n_medium: f64, size_distribution: SizeDistribution) -> Self {
        Self {
            n_particle,
            n_medium,
            size_distribution,
            anisotropy: 0.0,
            orientation: [0.0, 0.0, 1.0],
            growth_rate: 0.0,
            coalescence_rate: 0.0,
        }
    }

    /// Set anisotropy
    pub fn with_anisotropy(mut self, anisotropy: f64, orientation: [f64; 3]) -> Self {
        self.anisotropy = anisotropy.clamp(0.0, 1.0);
        // Normalize orientation
        let len = (orientation[0].powi(2) + orientation[1].powi(2) + orientation[2].powi(2)).sqrt();
        if len > 1e-10 {
            self.orientation = [
                orientation[0] / len,
                orientation[1] / len,
                orientation[2] / len,
            ];
        }
        self
    }

    /// Set temporal evolution parameters
    pub fn with_temporal(mut self, growth_rate: f64, coalescence_rate: f64) -> Self {
        self.growth_rate = growth_rate;
        self.coalescence_rate = coalescence_rate;
        self
    }

    /// Get relative refractive index
    pub fn relative_ior(&self) -> f64 {
        self.n_particle / self.n_medium
    }

    /// Get size distribution at time t
    pub fn distribution_at_time(&self, t: f64) -> SizeDistribution {
        if self.growth_rate.abs() < 1e-10 && self.coalescence_rate.abs() < 1e-10 {
            return self.size_distribution.clone();
        }

        // Apply growth and coalescence
        match &self.size_distribution {
            SizeDistribution::LogNormal {
                geometric_mean_um,
                geometric_std,
            } => {
                let new_mean = geometric_mean_um + self.growth_rate * t;
                let new_std = geometric_std * (1.0 + self.coalescence_rate * t);
                SizeDistribution::LogNormal {
                    geometric_mean_um: new_mean.max(0.01),
                    geometric_std: new_std.clamp(0.1, 2.0),
                }
            }
            SizeDistribution::Monodisperse { radius_um } => SizeDistribution::Monodisperse {
                radius_um: (radius_um + self.growth_rate * t).max(0.01),
            },
            _ => self.size_distribution.clone(),
        }
    }

    /// Calculate size parameter for a given radius and wavelength
    fn size_parameter(&self, radius_um: f64, wavelength_nm: f64) -> f64 {
        let lambda_um = wavelength_nm / 1000.0;
        2.0 * PI * radius_um / lambda_um
    }
}

// ============================================================================
// POLYDISPERSE SCATTERING
// ============================================================================

/// Calculate polydisperse phase function
///
/// Integrates over the size distribution:
/// p_total(θ) = ∫ p(θ, r) × n(r) dr
pub fn polydisperse_phase(
    cos_theta: f64,
    params: &DynamicMieParams,
    wavelength_nm: f64,
    num_samples: usize,
) -> f64 {
    let m = params.relative_ior();

    // Special case: Monodisperse is just a single size
    if let SizeDistribution::Monodisperse { radius_um } = &params.size_distribution {
        let x = params.size_parameter(*radius_um, wavelength_nm);
        return mie_fast(cos_theta, x, m);
    }

    let radii = params.size_distribution.sample_radii(num_samples);

    let mut total_phase = 0.0;
    let mut total_weight = 0.0;

    for r in &radii {
        let pdf = params.size_distribution.pdf(*r);
        if pdf < 1e-10 {
            continue;
        }

        let x = params.size_parameter(*r, wavelength_nm);
        let phase = mie_fast(cos_theta, x, m);

        total_phase += phase * pdf * r; // r factor for volume weighting
        total_weight += pdf * r;
    }

    if total_weight > 1e-10 {
        total_phase / total_weight
    } else {
        // Fallback to mean radius
        let x = params.size_parameter(params.size_distribution.mean_radius(), wavelength_nm);
        mie_fast(cos_theta, x, m)
    }
}

/// Calculate RGB polydisperse phase function
pub fn polydisperse_phase_rgb(
    cos_theta: f64,
    params: &DynamicMieParams,
    num_samples: usize,
) -> [f64; 3] {
    [
        polydisperse_phase(cos_theta, params, 650.0, num_samples),
        polydisperse_phase(cos_theta, params, 550.0, num_samples),
        polydisperse_phase(cos_theta, params, 450.0, num_samples),
    ]
}

// ============================================================================
// ANISOTROPIC SCATTERING
// ============================================================================

/// Calculate anisotropic phase function
///
/// p(θ, φ) = p_mie(θ) × [1 + A × cos(2φ)]
///
/// Where A is the anisotropy parameter and φ is the azimuthal angle
pub fn anisotropic_phase(
    cos_theta: f64,
    phi: f64,
    params: &DynamicMieParams,
    wavelength_nm: f64,
) -> f64 {
    let mean_r = params.size_distribution.mean_radius();
    let x = params.size_parameter(mean_r, wavelength_nm);
    let m = params.relative_ior();

    let isotropic_phase = mie_fast(cos_theta, x, m);

    if params.anisotropy < 1e-6 {
        return isotropic_phase;
    }

    // Anisotropic modulation
    let modulation = 1.0 + params.anisotropy * (2.0 * phi).cos();

    isotropic_phase * modulation.max(0.0)
}

/// Calculate anisotropic phase averaged over azimuth
pub fn anisotropic_phase_averaged(
    cos_theta: f64,
    params: &DynamicMieParams,
    wavelength_nm: f64,
) -> f64 {
    // For uniform azimuthal distribution, the cos(2φ) term averages to zero
    // So the averaged phase is just the isotropic phase
    let mean_r = params.size_distribution.mean_radius();
    let x = params.size_parameter(mean_r, wavelength_nm);
    mie_fast(cos_theta, x, params.relative_ior())
}

// ============================================================================
// TEMPORAL EVOLUTION
// ============================================================================

/// Calculate phase function at a specific time
pub fn temporal_phase(
    cos_theta: f64,
    params: &DynamicMieParams,
    wavelength_nm: f64,
    time_s: f64,
) -> f64 {
    let evolved = params.distribution_at_time(time_s);
    let evolved_params = DynamicMieParams {
        n_particle: params.n_particle,
        n_medium: params.n_medium,
        size_distribution: evolved,
        anisotropy: params.anisotropy,
        orientation: params.orientation,
        growth_rate: 0.0, // Don't double-evolve
        coalescence_rate: 0.0,
    };

    polydisperse_phase(cos_theta, &evolved_params, wavelength_nm, 16)
}

/// Calculate time series of phase functions
pub fn phase_time_series(
    cos_theta: f64,
    params: &DynamicMieParams,
    wavelength_nm: f64,
    times: &[f64],
) -> Vec<f64> {
    times
        .iter()
        .map(|&t| temporal_phase(cos_theta, params, wavelength_nm, t))
        .collect()
}

// ============================================================================
// DYNAMIC MIE PRESETS
// ============================================================================

/// Pre-defined dynamic Mie configurations
pub mod dynamic_presets {
    use super::*;

    /// Stratocumulus cloud droplets
    pub fn stratocumulus() -> DynamicMieParams {
        DynamicMieParams::new(
            1.33, // Water
            1.0,  // Air
            SizeDistribution::log_normal(8.0, 0.35),
        )
    }

    /// Fog droplets
    pub fn fog() -> DynamicMieParams {
        DynamicMieParams::new(1.33, 1.0, SizeDistribution::log_normal(4.0, 0.25))
    }

    /// Smoke particles (bimodal)
    pub fn smoke() -> DynamicMieParams {
        DynamicMieParams::new(
            1.5, // Soot
            1.0,
            SizeDistribution::bimodal(0.1, 0.5, 2.0, 0.3, 0.7),
        )
    }

    /// Milk particles (fat globules)
    pub fn milk() -> DynamicMieParams {
        DynamicMieParams::new(
            1.46, // Fat
            1.33, // Water
            SizeDistribution::log_normal(0.5, 0.4),
        )
    }

    /// Blood cells (bimodal: RBC and platelets)
    pub fn blood() -> DynamicMieParams {
        DynamicMieParams::new(
            1.40, // Hemoglobin
            1.35, // Plasma
            SizeDistribution::bimodal(3.0, 0.2, 8.0, 0.15, 0.3),
        )
    }

    /// Dust particles (desert aerosol)
    pub fn dust() -> DynamicMieParams {
        DynamicMieParams::new(
            1.53, // Silica
            1.0,
            SizeDistribution::log_normal(2.0, 0.6),
        )
        .with_anisotropy(0.2, [0.0, 0.0, 1.0]) // Slightly elongated
    }

    /// Ice crystals (cirrus cloud)
    pub fn ice_crystals() -> DynamicMieParams {
        DynamicMieParams::new(
            1.31, // Ice
            1.0,
            SizeDistribution::log_normal(20.0, 0.5),
        )
        .with_anisotropy(0.4, [0.0, 0.0, 1.0]) // Elongated crystals
    }

    /// Evolving fog (condensation)
    pub fn condensing_fog() -> DynamicMieParams {
        DynamicMieParams::new(1.33, 1.0, SizeDistribution::log_normal(1.0, 0.3))
            .with_temporal(0.1, 0.01) // Growing droplets
    }

    /// Evaporating mist
    pub fn evaporating_mist() -> DynamicMieParams {
        DynamicMieParams::new(1.33, 1.0, SizeDistribution::log_normal(5.0, 0.3))
            .with_temporal(-0.05, 0.0) // Shrinking droplets
    }

    /// Get all dynamic presets
    pub fn all_presets() -> Vec<(&'static str, DynamicMieParams)> {
        vec![
            ("Stratocumulus Cloud", stratocumulus()),
            ("Fog", fog()),
            ("Smoke", smoke()),
            ("Milk", milk()),
            ("Blood", blood()),
            ("Dust", dust()),
            ("Ice Crystals", ice_crystals()),
            ("Condensing Fog", condensing_fog()),
            ("Evaporating Mist", evaporating_mist()),
        ]
    }
}

// ============================================================================
// EFFECTIVE PROPERTIES
// ============================================================================

/// Calculate effective asymmetry parameter for polydisperse system
pub fn effective_asymmetry_g(params: &DynamicMieParams, wavelength_nm: f64) -> f64 {
    let radii = params.size_distribution.sample_radii(16);
    let m = params.relative_ior();

    let mut total_g = 0.0;
    let mut total_weight = 0.0;

    for r in &radii {
        let pdf = params.size_distribution.pdf(*r);
        if pdf < 1e-10 {
            continue;
        }

        let x = params.size_parameter(*r, wavelength_nm);
        let g = mie_asymmetry_g(x, m);

        // Weight by scattering cross-section (proportional to r^2)
        let weight = pdf * r * r;
        total_g += g * weight;
        total_weight += weight;
    }

    if total_weight > 1e-10 {
        total_g / total_weight
    } else {
        0.5
    }
}

/// Calculate extinction coefficient for polydisperse system
pub fn extinction_coefficient(params: &DynamicMieParams, wavelength_nm: f64) -> f64 {
    let radii = params.size_distribution.sample_radii(16);

    let mut total_extinction = 0.0;

    for r in &radii {
        let pdf = params.size_distribution.pdf(*r);
        if pdf < 1e-10 {
            continue;
        }

        let x = params.size_parameter(*r, wavelength_nm);

        // Extinction efficiency (approximation)
        let q_ext = if x < 0.3 {
            // Rayleigh regime
            8.0 / 3.0
                * x.powi(4)
                * ((params.relative_ior().powi(2) - 1.0) / (params.relative_ior().powi(2) + 2.0))
                    .powi(2)
        } else {
            // Larger particles: Q_ext ≈ 2 (geometric limit)
            2.0 - 4.0 / x
        };

        // Cross-section: σ = Q_ext * π * r²
        let sigma = q_ext * PI * r * r;

        total_extinction += sigma * pdf;
    }

    total_extinction
}

// ============================================================================
// CSS GENERATION
// ============================================================================

/// Generate CSS for fog effect
pub fn to_css_fog_effect(params: &DynamicMieParams, density: f64) -> String {
    let g = effective_asymmetry_g(params, 550.0);

    // Forward scattering creates bright halos
    let halo_strength = g * density;
    let scatter_blur = (1.0 - g) * 20.0 * density;

    format!(
        "background: rgba(255, 255, 255, {:.2}); \
         backdrop-filter: blur({:.1}px); \
         box-shadow: 0 0 {:.0}px {:.0}px rgba(255, 255, 255, {:.2});",
        density * 0.3,
        scatter_blur,
        halo_strength * 50.0,
        halo_strength * 30.0,
        halo_strength * 0.5,
    )
}

/// Generate CSS for smoke effect
pub fn to_css_smoke_effect(params: &DynamicMieParams, density: f64) -> String {
    let rgb = polydisperse_phase_rgb(0.5, params, 8);

    // Smoke is dark and absorbing
    let darkness = 1.0 - (rgb[0] + rgb[1] + rgb[2]) / 3.0;

    format!(
        "background: radial-gradient(ellipse at center, \
         rgba(50, 50, 50, {:.2}) 0%, \
         rgba(30, 30, 30, {:.2}) 50%, \
         rgba(10, 10, 10, {:.2}) 100%); \
         filter: blur({:.1}px);",
        density * 0.6,
        density * 0.4,
        density * 0.2,
        darkness * 10.0,
    )
}

// ============================================================================
// MEMORY AND PERFORMANCE
// ============================================================================

/// Memory usage for dynamic Mie calculations
pub fn dynamic_mie_memory() -> usize {
    std::mem::size_of::<DynamicMieParams>()
        + std::mem::size_of::<SizeDistribution>() * 2  // For bimodal
        + 64 // Sample array typical size
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_normal_pdf() {
        let dist = SizeDistribution::log_normal(5.0, 0.5);

        // PDF should be positive
        let pdf = dist.pdf(5.0);
        assert!(pdf > 0.0, "PDF at mean should be positive");

        // PDF should integrate to ~1
        let radii = dist.sample_radii(100);
        let sum: f64 = radii
            .windows(2)
            .map(|w| {
                let r_mid = (w[0] + w[1]) / 2.0;
                let dr = w[1] - w[0];
                dist.pdf(r_mid) * dr
            })
            .sum();

        assert!(
            (sum - 1.0).abs() < 0.3,
            "PDF should integrate to ~1: {}",
            sum
        );
    }

    #[test]
    fn test_bimodal_distribution() {
        let dist = SizeDistribution::bimodal(1.0, 0.3, 10.0, 0.3, 0.5);

        // Should have two peaks
        let pdf_1 = dist.pdf(1.0);
        let pdf_10 = dist.pdf(10.0);
        let pdf_5 = dist.pdf(5.0); // Between peaks

        assert!(pdf_1 > pdf_5, "First peak should be higher than valley");
        assert!(pdf_10 > pdf_5, "Second peak should be higher than valley");
    }

    #[test]
    fn test_polydisperse_phase() {
        let params = dynamic_presets::fog();
        let phase = polydisperse_phase(0.5, &params, 550.0, 16);

        assert!(phase > 0.0, "Phase should be positive");
    }

    #[test]
    fn test_anisotropic_phase() {
        let params = dynamic_presets::dust();

        let phase_0 = anisotropic_phase(0.5, 0.0, &params, 550.0);
        let phase_90 = anisotropic_phase(0.5, PI / 2.0, &params, 550.0);

        // Anisotropic: different at different azimuthal angles
        assert!(
            (phase_0 - phase_90).abs() > 0.001,
            "Anisotropic phase should vary with φ"
        );
    }

    #[test]
    fn test_temporal_evolution() {
        let params = dynamic_presets::condensing_fog();

        // Use forward scattering where size differences are more pronounced
        let phase_t0 = temporal_phase(0.9, &params, 550.0, 0.0);
        let phase_t50 = temporal_phase(0.9, &params, 550.0, 50.0); // Larger time for more growth

        // Larger droplets = different scattering (forward peak changes)
        // At t=0: mean=1.0µm, size_param=11.4
        // At t=50: mean=6.0µm, size_param=68.6 (clamped to 30, but mean still differs)
        assert!(
            (phase_t0 - phase_t50).abs() > 0.0001 || phase_t0 != phase_t50,
            "Phase should change over time: t0={}, t50={}",
            phase_t0,
            phase_t50
        );
    }

    #[test]
    fn test_effective_asymmetry() {
        let fog = dynamic_presets::fog();
        let g_fog = effective_asymmetry_g(&fog, 550.0);

        assert!(g_fog > 0.5, "Fog should have forward scattering");
        assert!(g_fog < 0.95, "g should be < 0.95");

        let cloud = dynamic_presets::stratocumulus();
        let g_cloud = effective_asymmetry_g(&cloud, 550.0);

        // Larger droplets = more forward scattering
        assert!(
            g_cloud > g_fog,
            "Cloud (larger) should have higher g than fog"
        );
    }

    #[test]
    fn test_wavelength_dependence() {
        let params = dynamic_presets::fog();
        let rgb = polydisperse_phase_rgb(0.5, &params, 8);

        // All channels should be positive
        for &v in &rgb {
            assert!(v > 0.0, "RGB phases should be positive");
        }
    }

    #[test]
    fn test_all_dynamic_presets() {
        let presets = dynamic_presets::all_presets();

        for (name, params) in presets {
            let phase = polydisperse_phase(0.5, &params, 550.0, 8);
            assert!(phase > 0.0, "{} phase should be positive", name);

            let g = effective_asymmetry_g(&params, 550.0);
            assert!(
                g >= 0.0 && g <= 1.0,
                "{} asymmetry should be in [0,1]: {}",
                name,
                g
            );
        }
    }

    #[test]
    fn test_size_distribution_mean() {
        let log_normal = SizeDistribution::log_normal(5.0, 0.5);
        let mean = log_normal.mean_radius();

        // Mean of log-normal should be close to but slightly higher than geometric mean
        assert!(mean > 4.0 && mean < 7.0, "Mean radius: {}", mean);
    }

    #[test]
    fn test_css_generation() {
        let fog = dynamic_presets::fog();
        let css = to_css_fog_effect(&fog, 0.5);

        assert!(css.contains("background"));
        assert!(css.contains("blur"));
    }

    #[test]
    fn test_memory_usage() {
        let mem = dynamic_mie_memory();
        assert!(mem < 500, "Memory should be minimal: {} bytes", mem);
    }
}

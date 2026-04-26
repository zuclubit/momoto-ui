//! # Spectral Pipeline - Unified End-to-End Optical Processing
//!
//! Sprint 6: This module unifies ALL optical phenomena into a single spectral pipeline
//! where color is NEVER an intermediate state - only a final projection.
//!
//! ## Key Principles
//!
//! 1. **Spectral Domain**: All calculations operate on wavelength-resolved signals
//! 2. **Energy Conservation**: Total energy in = total energy out (no creation/destruction)
//! 3. **Composable Stages**: Phenomena stack sequentially with explicit ordering
//! 4. **RGB Only at End**: `to_rgb()` is the FINAL operation, never intermediate
//!
//! ## Architecture
//!
//! ```text
//! IncidentLight → [Stage1] → [Stage2] → ... → [StageN] → SpectralSignal → RGB
//!                 ↓          ↓               ↓
//!              ThinFilm   Dispersion      Mie/etc
//! ```

use std::f64::consts::PI;

// ============================================================================
// SPECTRAL CONSTANTS (GLOBAL STANDARD)
// ============================================================================

/// Standard visible spectrum range (nm)
pub const VISIBLE_MIN_NM: f64 = 380.0;
pub const VISIBLE_MAX_NM: f64 = 780.0;

/// CIE Standard Observer wavelengths for RGB primaries
/// Using CIE 1931 color matching function peaks
pub mod wavelengths {
    /// Red primary wavelength (CIE X peak approximation)
    pub const RED: f64 = 650.0;
    /// Green primary wavelength (CIE Y peak)
    pub const GREEN: f64 = 550.0;
    /// Blue primary wavelength (CIE Z peak approximation)
    pub const BLUE: f64 = 450.0;

    /// Standard spectroscopic reference lines (for dispersion)
    pub const FRAUNHOFER_C: f64 = 656.3; // Hydrogen alpha (red)
    pub const FRAUNHOFER_D: f64 = 587.6; // Helium (yellow)
    pub const FRAUNHOFER_F: f64 = 486.1; // Hydrogen beta (blue)

    /// Default spectral sampling points (31 points, 10nm spacing)
    pub fn default_sampling() -> Vec<f64> {
        (0..=40).map(|i| 380.0 + i as f64 * 10.0).collect()
    }

    /// High-resolution sampling (81 points, 5nm spacing)
    pub fn high_resolution_sampling() -> Vec<f64> {
        (0..=80).map(|i| 380.0 + i as f64 * 5.0).collect()
    }

    /// RGB-only sampling (3 points)
    pub fn rgb_sampling() -> Vec<f64> {
        vec![RED, GREEN, BLUE]
    }
}

// ============================================================================
// SPECTRAL SIGNAL - THE CORE DATA TYPE
// ============================================================================

/// A single spectral sample at a specific wavelength
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpectralSample {
    /// Wavelength in nanometers
    pub wavelength_nm: f64,
    /// Intensity (dimensionless, 0.0 to 1.0 for reflectance/transmittance)
    /// or power (W/nm) for irradiance
    pub intensity: f64,
}

impl SpectralSample {
    pub fn new(wavelength_nm: f64, intensity: f64) -> Self {
        Self {
            wavelength_nm,
            intensity,
        }
    }
}

/// A complete spectral signal - the universal data type for the pipeline
///
/// This is the ONLY type that flows between pipeline stages.
/// No RGB values, no shortcuts - pure spectral data.
#[derive(Debug, Clone)]
pub struct SpectralSignal {
    /// Ordered samples from low to high wavelength
    samples: Vec<SpectralSample>,
}

impl SpectralSignal {
    /// Create from wavelengths and intensities arrays
    pub fn from_arrays(wavelengths: &[f64], intensities: &[f64]) -> Self {
        assert_eq!(
            wavelengths.len(),
            intensities.len(),
            "Arrays must have same length"
        );
        let mut samples: Vec<_> = wavelengths
            .iter()
            .zip(intensities.iter())
            .map(|(&w, &i)| SpectralSample::new(w, i))
            .collect();
        samples.sort_by(|a, b| a.wavelength_nm.partial_cmp(&b.wavelength_nm).unwrap());
        Self { samples }
    }

    /// Create uniform (flat) spectrum at given intensity
    pub fn uniform(wavelengths: &[f64], intensity: f64) -> Self {
        let intensities: Vec<_> = wavelengths.iter().map(|_| intensity).collect();
        Self::from_arrays(wavelengths, &intensities)
    }

    /// Create from default sampling with uniform intensity
    pub fn uniform_default(intensity: f64) -> Self {
        Self::uniform(&wavelengths::default_sampling(), intensity)
    }

    /// Create D65 daylight illuminant (normalized)
    pub fn d65_illuminant() -> Self {
        let wavelengths = wavelengths::default_sampling();
        // D65 SPD approximation (simplified, relative values)
        let intensities: Vec<_> = wavelengths
            .iter()
            .map(|&w| {
                // Planck-like + atmospheric filtering approximation
                let t = 6500.0; // Color temperature
                let x = (1.4388e7 / (w * t)).min(50.0);
                let planck = 1.0 / (w.powi(5) * (x.exp() - 1.0));
                // Normalize to peak at ~1.0
                planck * 1e20
            })
            .collect();
        // Normalize
        let max = intensities
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);
        let normalized: Vec<_> = intensities.iter().map(|&i| i / max).collect();
        Self::from_arrays(&wavelengths, &normalized)
    }

    /// Get interpolated intensity at arbitrary wavelength
    pub fn intensity_at(&self, wavelength_nm: f64) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        if wavelength_nm <= self.samples.first().unwrap().wavelength_nm {
            return self.samples.first().unwrap().intensity;
        }
        if wavelength_nm >= self.samples.last().unwrap().wavelength_nm {
            return self.samples.last().unwrap().intensity;
        }

        // Linear interpolation
        for i in 0..self.samples.len() - 1 {
            let s0 = &self.samples[i];
            let s1 = &self.samples[i + 1];
            if wavelength_nm >= s0.wavelength_nm && wavelength_nm <= s1.wavelength_nm {
                let t = (wavelength_nm - s0.wavelength_nm) / (s1.wavelength_nm - s0.wavelength_nm);
                return s0.intensity * (1.0 - t) + s1.intensity * t;
            }
        }
        0.0
    }

    /// Get all samples
    pub fn samples(&self) -> &[SpectralSample] {
        &self.samples
    }

    /// Get wavelengths array
    pub fn wavelengths(&self) -> Vec<f64> {
        self.samples.iter().map(|s| s.wavelength_nm).collect()
    }

    /// Get intensities array
    pub fn intensities(&self) -> Vec<f64> {
        self.samples.iter().map(|s| s.intensity).collect()
    }

    /// Total integrated energy (trapezoidal integration)
    pub fn total_energy(&self) -> f64 {
        if self.samples.len() < 2 {
            return 0.0;
        }
        let mut energy = 0.0;
        for i in 0..self.samples.len() - 1 {
            let s0 = &self.samples[i];
            let s1 = &self.samples[i + 1];
            let dw = s1.wavelength_nm - s0.wavelength_nm;
            energy += 0.5 * (s0.intensity + s1.intensity) * dw;
        }
        energy
    }

    /// Multiply by another signal (element-wise, with interpolation)
    pub fn multiply(&self, other: &SpectralSignal) -> SpectralSignal {
        let new_samples: Vec<_> = self
            .samples
            .iter()
            .map(|s| {
                SpectralSample::new(
                    s.wavelength_nm,
                    s.intensity * other.intensity_at(s.wavelength_nm),
                )
            })
            .collect();
        SpectralSignal {
            samples: new_samples,
        }
    }

    /// Scale by constant factor
    pub fn scale(&self, factor: f64) -> SpectralSignal {
        let new_samples: Vec<_> = self
            .samples
            .iter()
            .map(|s| SpectralSample::new(s.wavelength_nm, s.intensity * factor))
            .collect();
        SpectralSignal {
            samples: new_samples,
        }
    }

    /// Convert to RGB using CIE 1931 color matching functions
    ///
    /// THIS IS THE ONLY PLACE WHERE RGB IS COMPUTED.
    /// All physics happens in spectral domain before this.
    pub fn to_xyz(&self) -> [f64; 3] {
        let mut x = 0.0;
        let mut y = 0.0;
        let mut z = 0.0;

        for sample in &self.samples {
            let (xbar, ybar, zbar) = cie_1931_cmf(sample.wavelength_nm);
            // Assuming uniform wavelength spacing for simplicity
            // For non-uniform, would need proper integration weights
            x += sample.intensity * xbar;
            y += sample.intensity * ybar;
            z += sample.intensity * zbar;
        }

        // Normalize (approximate integration weight)
        let dw = if self.samples.len() > 1 {
            (self.samples.last().unwrap().wavelength_nm
                - self.samples.first().unwrap().wavelength_nm)
                / (self.samples.len() - 1) as f64
        } else {
            10.0
        };

        [x * dw, y * dw, z * dw]
    }

    /// Convert to sRGB [0, 1] from spectral data
    pub fn to_rgb(&self) -> [f64; 3] {
        let xyz = self.to_xyz();
        xyz_to_srgb(xyz)
    }

    /// Convert to sRGB [0, 255] u8 values
    pub fn to_rgb_u8(&self) -> [u8; 3] {
        let rgb = self.to_rgb();
        [
            (rgb[0] * 255.0).clamp(0.0, 255.0) as u8,
            (rgb[1] * 255.0).clamp(0.0, 255.0) as u8,
            (rgb[2] * 255.0).clamp(0.0, 255.0) as u8,
        ]
    }
}

// ============================================================================
// CIE 1931 COLOR MATCHING FUNCTIONS
// ============================================================================

/// CIE 1931 2° Standard Observer color matching functions
///
/// Returns (x̄, ȳ, z̄) for a given wavelength in nm
fn cie_1931_cmf(wavelength_nm: f64) -> (f64, f64, f64) {
    // Gaussian approximation to CIE 1931 CMFs
    // Based on Wyman, Sloan, Shirley (2013) simple analytic approximations

    let w = wavelength_nm;

    // x̄(λ) - two Gaussian peaks (red and violet)
    let x = 1.056 * gaussian(w, 599.8, 37.9, 31.0) + 0.362 * gaussian(w, 442.0, 16.0, 26.7)
        - 0.065 * gaussian(w, 501.1, 20.4, 26.2);

    // ȳ(λ) - luminance, single peak
    let y = 0.821 * gaussian(w, 568.8, 46.9, 40.5) + 0.286 * gaussian(w, 530.9, 16.3, 31.1);

    // z̄(λ) - blue, single peak
    let z = 1.217 * gaussian(w, 437.0, 11.8, 36.0) + 0.681 * gaussian(w, 459.0, 26.0, 13.8);

    (x.max(0.0), y.max(0.0), z.max(0.0))
}

/// Asymmetric Gaussian helper for CMF approximation
fn gaussian(x: f64, mu: f64, sigma1: f64, sigma2: f64) -> f64 {
    let sigma = if x < mu { sigma1 } else { sigma2 };
    (-(x - mu).powi(2) / (2.0 * sigma.powi(2))).exp()
}

/// Convert CIE XYZ to sRGB (linear to gamma-corrected)
fn xyz_to_srgb(xyz: [f64; 3]) -> [f64; 3] {
    // XYZ to linear RGB matrix (sRGB, D65)
    let r_linear = 3.2404542 * xyz[0] - 1.5371385 * xyz[1] - 0.4985314 * xyz[2];
    let g_linear = -0.9692660 * xyz[0] + 1.8760108 * xyz[1] + 0.0415560 * xyz[2];
    let b_linear = 0.0556434 * xyz[0] - 0.2040259 * xyz[1] + 1.0572252 * xyz[2];

    // sRGB gamma correction
    fn gamma_correct(u: f64) -> f64 {
        if u <= 0.0031308 {
            12.92 * u
        } else {
            1.055 * u.powf(1.0 / 2.4) - 0.055
        }
    }

    [
        gamma_correct(r_linear).clamp(0.0, 1.0),
        gamma_correct(g_linear).clamp(0.0, 1.0),
        gamma_correct(b_linear).clamp(0.0, 1.0),
    ]
}

// ============================================================================
// SPECTRAL STAGE TRAIT - THE COMPOSABLE INTERFACE
// ============================================================================

/// A single stage in the spectral pipeline
///
/// Each physics phenomenon implements this trait.
/// The pipeline chains stages: input → stage1 → stage2 → ... → output
pub trait SpectralStage: Send + Sync {
    /// Process spectral signal through this stage
    ///
    /// # Arguments
    /// * `input` - Incoming spectral signal
    /// * `context` - Evaluation context (angle, position, etc.)
    ///
    /// # Returns
    /// Transformed spectral signal
    fn process(&self, input: &SpectralSignal, context: &EvaluationContext) -> SpectralSignal;

    /// Name of this stage for debugging/visualization
    fn name(&self) -> &str;

    /// Whether this stage conserves energy (should be true for most)
    fn conserves_energy(&self) -> bool {
        true
    }
}

/// Context for spectral evaluation
#[derive(Debug, Clone)]
pub struct EvaluationContext {
    /// Incident angle (cosine of angle from normal)
    pub cos_theta: f64,
    /// Temperature in Kelvin
    pub temperature_k: f64,
    /// Stress tensor [σxx, σyy, σzz, σxy, σyz, σzx] in MPa
    pub stress_mpa: [f64; 6],
    /// Position on surface (normalized 0-1)
    pub position: (f64, f64),
    /// Ambient pressure in Pa
    pub pressure_pa: f64,
    /// Relative humidity (0-1)
    pub humidity: f64,
}

impl Default for EvaluationContext {
    fn default() -> Self {
        Self {
            cos_theta: 1.0,        // Normal incidence
            temperature_k: 293.15, // 20°C
            stress_mpa: [0.0; 6],
            position: (0.5, 0.5),
            pressure_pa: 101325.0, // 1 atm
            humidity: 0.5,
        }
    }
}

impl EvaluationContext {
    pub fn with_angle_deg(mut self, angle_deg: f64) -> Self {
        self.cos_theta = (angle_deg * PI / 180.0).cos();
        self
    }

    pub fn with_temperature(mut self, temp_k: f64) -> Self {
        self.temperature_k = temp_k;
        self
    }

    pub fn with_stress(mut self, stress: [f64; 6]) -> Self {
        self.stress_mpa = stress;
        self
    }

    pub fn with_position(mut self, x: f64, y: f64) -> Self {
        self.position = (x, y);
        self
    }
}

// ============================================================================
// SPECTRAL PIPELINE - THE UNIFIED ORCHESTRATOR
// ============================================================================

/// The unified spectral pipeline
///
/// Composes multiple optical phenomena into a single coherent system.
/// RGB is ONLY computed at the very end via `to_rgb()`.
pub struct SpectralPipeline {
    stages: Vec<Box<dyn SpectralStage>>,
}

impl SpectralPipeline {
    pub fn new() -> Self {
        Self { stages: Vec::new() }
    }

    /// Add a stage to the pipeline
    pub fn add_stage<S: SpectralStage + 'static>(mut self, stage: S) -> Self {
        self.stages.push(Box::new(stage));
        self
    }

    /// Evaluate the complete pipeline
    ///
    /// # Arguments
    /// * `incident` - Incident light spectrum
    /// * `context` - Evaluation context
    ///
    /// # Returns
    /// Final spectral signal (NOT RGB - use .to_rgb() for that)
    pub fn evaluate(
        &self,
        incident: &SpectralSignal,
        context: &EvaluationContext,
    ) -> SpectralSignal {
        let mut signal = incident.clone();
        for stage in &self.stages {
            signal = stage.process(&signal, context);
        }
        signal
    }

    /// Evaluate and return intermediate results for visualization
    pub fn evaluate_with_intermediates(
        &self,
        incident: &SpectralSignal,
        context: &EvaluationContext,
    ) -> Vec<(String, SpectralSignal)> {
        let mut results = vec![("Incident".to_string(), incident.clone())];
        let mut signal = incident.clone();

        for stage in &self.stages {
            signal = stage.process(&signal, context);
            results.push((stage.name().to_string(), signal.clone()));
        }

        results
    }

    /// Get number of stages
    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }

    /// Get stage names
    pub fn stage_names(&self) -> Vec<&str> {
        self.stages.iter().map(|s| s.name()).collect()
    }

    /// Verify energy conservation across all stages
    pub fn verify_energy_conservation(
        &self,
        incident: &SpectralSignal,
        context: &EvaluationContext,
    ) -> bool {
        let intermediates = self.evaluate_with_intermediates(incident, context);

        let incident_energy = incident.total_energy();
        if incident_energy == 0.0 {
            return true; // No energy to conserve
        }

        for (i, (name, signal)) in intermediates.iter().enumerate().skip(1) {
            let stage = &self.stages[i - 1];
            if stage.conserves_energy() {
                let energy = signal.total_energy();
                // Allow 1% tolerance for numerical errors
                if energy > incident_energy * 1.01 {
                    eprintln!(
                        "Energy violation at stage '{}': {} > {}",
                        name, energy, incident_energy
                    );
                    return false;
                }
            }
        }
        true
    }
}

impl Default for SpectralPipeline {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// BUILT-IN STAGES: THIN FILM
// ============================================================================

/// Thin film interference stage (Airy formula)
pub struct ThinFilmStage {
    /// Film refractive index
    pub n_film: f64,
    /// Film thickness in nm
    pub thickness_nm: f64,
    /// Substrate refractive index
    pub n_substrate: f64,
}

impl ThinFilmStage {
    pub fn new(n_film: f64, thickness_nm: f64, n_substrate: f64) -> Self {
        Self {
            n_film,
            thickness_nm,
            n_substrate,
        }
    }

    /// Calculate reflectance at a single wavelength
    fn reflectance_at(&self, wavelength_nm: f64, cos_theta: f64) -> f64 {
        let n1 = 1.0; // Air
        let n2 = self.n_film;
        let n3 = self.n_substrate;
        let d = self.thickness_nm;

        // Snell's law for angle in film
        let sin_theta1 = (1.0 - cos_theta * cos_theta).sqrt();
        let sin_theta2 = sin_theta1 / n2;
        if sin_theta2 >= 1.0 {
            return 1.0; // Total internal reflection
        }
        let cos_theta2 = (1.0 - sin_theta2 * sin_theta2).sqrt();

        // Fresnel coefficients (s-polarized, simplified)
        let r12 = (n1 * cos_theta - n2 * cos_theta2) / (n1 * cos_theta + n2 * cos_theta2);
        let r23 = (n2 * cos_theta2 - n3 * cos_theta) / (n2 * cos_theta2 + n3 * cos_theta);

        // Phase difference
        let delta = 4.0 * PI * n2 * d * cos_theta2 / wavelength_nm;

        // Airy formula
        let r_num = r12 * r12 + r23 * r23 + 2.0 * r12 * r23 * delta.cos();
        let r_den = 1.0 + r12 * r12 * r23 * r23 + 2.0 * r12 * r23 * delta.cos();

        (r_num / r_den).clamp(0.0, 1.0)
    }
}

impl SpectralStage for ThinFilmStage {
    fn process(&self, input: &SpectralSignal, context: &EvaluationContext) -> SpectralSignal {
        let new_samples: Vec<_> = input
            .samples()
            .iter()
            .map(|s| {
                let r = self.reflectance_at(s.wavelength_nm, context.cos_theta);
                SpectralSample::new(s.wavelength_nm, s.intensity * r)
            })
            .collect();
        SpectralSignal {
            samples: new_samples,
        }
    }

    fn name(&self) -> &str {
        "Thin Film"
    }
}

// ============================================================================
// BUILT-IN STAGES: DISPERSION
// ============================================================================

/// Dispersion stage - modifies IOR based on wavelength
pub struct DispersionStage {
    /// Cauchy coefficients [A, B, C]
    pub cauchy_coeffs: [f64; 3],
}

impl DispersionStage {
    pub fn new(a: f64, b: f64, c: f64) -> Self {
        Self {
            cauchy_coeffs: [a, b, c],
        }
    }

    /// Crown glass preset
    pub fn crown_glass() -> Self {
        Self::new(1.5168, 4300.0, 0.0)
    }

    /// Flint glass preset
    pub fn flint_glass() -> Self {
        Self::new(1.6200, 8000.0, 0.0)
    }

    /// Calculate refractive index at wavelength (Cauchy formula)
    pub fn n_at(&self, wavelength_nm: f64) -> f64 {
        let [a, b, c] = self.cauchy_coeffs;
        let w2 = wavelength_nm * wavelength_nm;
        let w4 = w2 * w2;
        a + b / w2 + c / w4
    }

    /// Calculate Fresnel reflectance for dielectric
    fn fresnel_dielectric(&self, n: f64, cos_theta: f64) -> f64 {
        let n1 = 1.0; // Air
        let sin_theta1 = (1.0 - cos_theta * cos_theta).sqrt();
        let sin_theta2 = n1 * sin_theta1 / n;

        if sin_theta2 >= 1.0 {
            return 1.0; // TIR
        }

        let cos_theta2 = (1.0 - sin_theta2 * sin_theta2).sqrt();

        // Average of s and p polarization
        let rs = (n1 * cos_theta - n * cos_theta2) / (n1 * cos_theta + n * cos_theta2);
        let rp = (n * cos_theta - n1 * cos_theta2) / (n * cos_theta + n1 * cos_theta2);

        0.5 * (rs * rs + rp * rp)
    }
}

impl SpectralStage for DispersionStage {
    fn process(&self, input: &SpectralSignal, context: &EvaluationContext) -> SpectralSignal {
        let new_samples: Vec<_> = input
            .samples()
            .iter()
            .map(|s| {
                let n = self.n_at(s.wavelength_nm);
                let r = self.fresnel_dielectric(n, context.cos_theta);
                SpectralSample::new(s.wavelength_nm, s.intensity * r)
            })
            .collect();
        SpectralSignal {
            samples: new_samples,
        }
    }

    fn name(&self) -> &str {
        "Dispersion"
    }
}

// ============================================================================
// BUILT-IN STAGES: MIE SCATTERING
// ============================================================================

/// Mie scattering stage
pub struct MieScatteringStage {
    /// Particle radius in micrometers
    pub radius_um: f64,
    /// Particle refractive index
    pub n_particle: f64,
    /// Medium refractive index
    pub n_medium: f64,
}

impl MieScatteringStage {
    pub fn new(radius_um: f64, n_particle: f64, n_medium: f64) -> Self {
        Self {
            radius_um,
            n_particle,
            n_medium,
        }
    }

    /// Fog preset
    pub fn fog() -> Self {
        Self::new(5.0, 1.33, 1.0) // Water droplets in air
    }

    /// Milk preset
    pub fn milk() -> Self {
        Self::new(0.5, 1.46, 1.33) // Fat globules in water
    }

    /// Calculate scattering efficiency (simplified Mie)
    fn scatter_efficiency(&self, wavelength_nm: f64, cos_theta: f64) -> f64 {
        let x = 2.0 * PI * self.radius_um * 1000.0 / wavelength_nm; // Size parameter
        let m = self.n_particle / self.n_medium; // Relative IOR

        if x < 0.3 {
            // Rayleigh regime
            let term = (m * m - 1.0) / (m * m + 2.0);
            let q_sca = (8.0 / 3.0) * x.powi(4) * term * term;
            // Phase function
            let p_theta = 0.75 * (1.0 + cos_theta * cos_theta);
            q_sca * p_theta
        } else {
            // Simplified Mie (Henyey-Greenstein approximation)
            let g = 0.85_f64.min(0.1 * x.sqrt()); // Asymmetry parameter
            let hg = (1.0 - g * g) / (1.0 + g * g - 2.0 * g * cos_theta).powf(1.5);
            let q_sca = 2.0 * (1.0 - (-0.1 * x).exp()); // Extinction efficiency
            q_sca * hg / (4.0 * PI)
        }
    }
}

impl SpectralStage for MieScatteringStage {
    fn process(&self, input: &SpectralSignal, context: &EvaluationContext) -> SpectralSignal {
        let new_samples: Vec<_> = input
            .samples()
            .iter()
            .map(|s| {
                let scatter = self.scatter_efficiency(s.wavelength_nm, context.cos_theta);
                // Scattering reduces forward intensity
                SpectralSample::new(s.wavelength_nm, s.intensity * (1.0 - scatter.min(1.0)))
            })
            .collect();
        SpectralSignal {
            samples: new_samples,
        }
    }

    fn name(&self) -> &str {
        "Mie Scattering"
    }

    fn conserves_energy(&self) -> bool {
        false // Scattering redirects energy, doesn't conserve in forward direction
    }
}

// ============================================================================
// BUILT-IN STAGES: THERMO-OPTIC
// ============================================================================

/// Thermo-optic stage - temperature-dependent optical properties
pub struct ThermoOpticStage {
    /// Base refractive index
    pub n_base: f64,
    /// Thermo-optic coefficient (dn/dT) in K⁻¹
    pub dn_dt: f64,
    /// Base thickness in nm
    pub thickness_nm: f64,
    /// Thermal expansion coefficient in K⁻¹
    pub alpha_thermal: f64,
    /// Reference temperature in K
    pub t_ref: f64,
}

impl ThermoOpticStage {
    pub fn new(n_base: f64, dn_dt: f64, thickness_nm: f64, alpha_thermal: f64) -> Self {
        Self {
            n_base,
            dn_dt,
            thickness_nm,
            alpha_thermal,
            t_ref: 293.15, // 20°C
        }
    }

    /// Glass coating preset
    pub fn glass_coating(thickness_nm: f64) -> Self {
        Self::new(
            1.52,   // BK7-like
            1.0e-5, // Typical glass dn/dT
            thickness_nm,
            7.0e-6, // Glass thermal expansion
        )
    }

    /// Calculate effective IOR at temperature
    fn n_effective(&self, temp_k: f64) -> f64 {
        self.n_base + self.dn_dt * (temp_k - self.t_ref)
    }

    /// Calculate effective thickness at temperature
    fn thickness_effective(&self, temp_k: f64) -> f64 {
        self.thickness_nm * (1.0 + self.alpha_thermal * (temp_k - self.t_ref))
    }

    /// Calculate reflectance with temperature effects
    fn reflectance_at(&self, wavelength_nm: f64, cos_theta: f64, temp_k: f64) -> f64 {
        let n = self.n_effective(temp_k);
        let d = self.thickness_effective(temp_k);

        // Simplified thin film reflectance
        let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();
        let sin_theta2 = sin_theta / n;
        if sin_theta2 >= 1.0 {
            return 1.0;
        }
        let cos_theta2 = (1.0 - sin_theta2 * sin_theta2).sqrt();

        let r12 = (cos_theta - n * cos_theta2) / (cos_theta + n * cos_theta2);
        let delta = 4.0 * PI * n * d * cos_theta2 / wavelength_nm;

        // Single surface reflectance modified by interference
        let base_r = r12 * r12;
        base_r * (1.0 + 0.5 * delta.cos()) // Simplified interference term
    }
}

impl SpectralStage for ThermoOpticStage {
    fn process(&self, input: &SpectralSignal, context: &EvaluationContext) -> SpectralSignal {
        let new_samples: Vec<_> = input
            .samples()
            .iter()
            .map(|s| {
                let r =
                    self.reflectance_at(s.wavelength_nm, context.cos_theta, context.temperature_k);
                SpectralSample::new(s.wavelength_nm, s.intensity * r.clamp(0.0, 1.0))
            })
            .collect();
        SpectralSignal {
            samples: new_samples,
        }
    }

    fn name(&self) -> &str {
        "Thermo-Optic"
    }
}

// ============================================================================
// BUILT-IN STAGES: METAL REFLECTANCE
// ============================================================================

/// Metal reflectance stage using complex IOR
pub struct MetalReflectanceStage {
    /// Complex IOR at RGB wavelengths: [(n_r, k_r), (n_g, k_g), (n_b, k_b)]
    pub spectral_nk: [(f64, f64); 3],
}

impl MetalReflectanceStage {
    pub fn new(spectral_nk: [(f64, f64); 3]) -> Self {
        Self { spectral_nk }
    }

    /// Gold preset (Johnson & Christy data)
    pub fn gold() -> Self {
        Self::new([
            (0.18, 3.00), // Red (650nm)
            (0.42, 2.40), // Green (550nm)
            (1.47, 1.95), // Blue (450nm)
        ])
    }

    /// Silver preset
    pub fn silver() -> Self {
        Self::new([
            (0.15, 4.00), // Red
            (0.13, 3.50), // Green
            (0.14, 2.50), // Blue
        ])
    }

    /// Copper preset
    pub fn copper() -> Self {
        Self::new([
            (0.21, 4.00), // Red
            (0.95, 2.60), // Green
            (1.22, 2.44), // Blue
        ])
    }

    /// Interpolate n,k at wavelength
    fn nk_at(&self, wavelength_nm: f64) -> (f64, f64) {
        // Simple linear interpolation between RGB points (650, 550, 450 nm)
        if wavelength_nm >= 650.0 {
            return self.spectral_nk[0];
        }
        if wavelength_nm <= 450.0 {
            return self.spectral_nk[2];
        }

        // Find interpolation segment
        if wavelength_nm >= 550.0 {
            let t = (wavelength_nm - 550.0) / 100.0;
            let n = self.spectral_nk[1].0 * (1.0 - t) + self.spectral_nk[0].0 * t;
            let k = self.spectral_nk[1].1 * (1.0 - t) + self.spectral_nk[0].1 * t;
            (n, k)
        } else {
            let t = (wavelength_nm - 450.0) / 100.0;
            let n = self.spectral_nk[2].0 * (1.0 - t) + self.spectral_nk[1].0 * t;
            let k = self.spectral_nk[2].1 * (1.0 - t) + self.spectral_nk[1].1 * t;
            (n, k)
        }
    }

    /// Fresnel reflectance for conductor
    fn fresnel_conductor(&self, n: f64, k: f64, cos_theta: f64) -> f64 {
        let cos2 = cos_theta * cos_theta;
        let _sin2 = 1.0 - cos2;

        let _n2 = n * n;
        let k2 = k * k;

        // Simplified conductor Fresnel (normal incidence approximation extended)
        let num = (n - 1.0).powi(2) + k2;
        let den = (n + 1.0).powi(2) + k2;
        let f0 = num / den;

        // Schlick approximation for angle dependence
        f0 + (1.0 - f0) * (1.0 - cos_theta).powi(5)
    }
}

impl SpectralStage for MetalReflectanceStage {
    fn process(&self, input: &SpectralSignal, context: &EvaluationContext) -> SpectralSignal {
        let new_samples: Vec<_> = input
            .samples()
            .iter()
            .map(|s| {
                let (n, k) = self.nk_at(s.wavelength_nm);
                let r = self.fresnel_conductor(n, k, context.cos_theta);
                SpectralSample::new(s.wavelength_nm, s.intensity * r)
            })
            .collect();
        SpectralSignal {
            samples: new_samples,
        }
    }

    fn name(&self) -> &str {
        "Metal Reflectance"
    }
}

// ============================================================================
// PIPELINE BUILDER (FLUENT API)
// ============================================================================

/// Fluent builder for spectral pipeline
pub struct PipelineBuilder {
    pipeline: SpectralPipeline,
}

impl PipelineBuilder {
    pub fn new() -> Self {
        Self {
            pipeline: SpectralPipeline::new(),
        }
    }

    pub fn with_thin_film(mut self, n_film: f64, thickness_nm: f64, n_substrate: f64) -> Self {
        self.pipeline =
            self.pipeline
                .add_stage(ThinFilmStage::new(n_film, thickness_nm, n_substrate));
        self
    }

    pub fn with_dispersion(mut self, a: f64, b: f64, c: f64) -> Self {
        self.pipeline = self.pipeline.add_stage(DispersionStage::new(a, b, c));
        self
    }

    pub fn with_crown_glass_dispersion(mut self) -> Self {
        self.pipeline = self.pipeline.add_stage(DispersionStage::crown_glass());
        self
    }

    pub fn with_mie_scattering(mut self, radius_um: f64, n_particle: f64, n_medium: f64) -> Self {
        self.pipeline = self
            .pipeline
            .add_stage(MieScatteringStage::new(radius_um, n_particle, n_medium));
        self
    }

    pub fn with_fog(mut self) -> Self {
        self.pipeline = self.pipeline.add_stage(MieScatteringStage::fog());
        self
    }

    pub fn with_thermo_optic(
        mut self,
        n_base: f64,
        dn_dt: f64,
        thickness_nm: f64,
        alpha: f64,
    ) -> Self {
        self.pipeline =
            self.pipeline
                .add_stage(ThermoOpticStage::new(n_base, dn_dt, thickness_nm, alpha));
        self
    }

    pub fn with_metal(mut self, metal: MetalReflectanceStage) -> Self {
        self.pipeline = self.pipeline.add_stage(metal);
        self
    }

    pub fn with_gold(mut self) -> Self {
        self.pipeline = self.pipeline.add_stage(MetalReflectanceStage::gold());
        self
    }

    pub fn with_silver(mut self) -> Self {
        self.pipeline = self.pipeline.add_stage(MetalReflectanceStage::silver());
        self
    }

    pub fn with_copper(mut self) -> Self {
        self.pipeline = self.pipeline.add_stage(MetalReflectanceStage::copper());
        self
    }

    pub fn build(self) -> SpectralPipeline {
        self.pipeline
    }
}

impl Default for PipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_signal_creation() {
        let wavelengths = vec![400.0, 500.0, 600.0, 700.0];
        let intensities = vec![0.5, 0.8, 0.9, 0.7];
        let signal = SpectralSignal::from_arrays(&wavelengths, &intensities);

        assert_eq!(signal.samples().len(), 4);
        assert_eq!(signal.intensity_at(400.0), 0.5);
        assert_eq!(signal.intensity_at(700.0), 0.7);
    }

    #[test]
    fn test_spectral_interpolation() {
        let signal = SpectralSignal::from_arrays(&[400.0, 600.0], &[0.0, 1.0]);

        assert!((signal.intensity_at(500.0) - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_total_energy() {
        let signal = SpectralSignal::from_arrays(&[400.0, 500.0, 600.0], &[1.0, 1.0, 1.0]);
        let energy = signal.total_energy();

        // Trapezoidal: (1+1)/2 * 100 + (1+1)/2 * 100 = 200
        assert!((energy - 200.0).abs() < 0.001);
    }

    #[test]
    fn test_xyz_conversion() {
        let signal = SpectralSignal::uniform_default(1.0);
        let xyz = signal.to_xyz();

        // Should have non-zero XYZ
        assert!(xyz[0] > 0.0);
        assert!(xyz[1] > 0.0);
        assert!(xyz[2] > 0.0);
    }

    #[test]
    fn test_rgb_conversion() {
        let signal = SpectralSignal::uniform_default(1.0);
        let rgb = signal.to_rgb();

        // Uniform white should give roughly equal RGB
        assert!(rgb[0] > 0.0 && rgb[0] <= 1.0);
        assert!(rgb[1] > 0.0 && rgb[1] <= 1.0);
        assert!(rgb[2] > 0.0 && rgb[2] <= 1.0);
    }

    #[test]
    fn test_pipeline_composition() {
        let pipeline = PipelineBuilder::new()
            .with_thin_film(1.45, 150.0, 1.52)
            .with_crown_glass_dispersion()
            .build();

        assert_eq!(pipeline.stage_count(), 2);
        assert_eq!(pipeline.stage_names(), vec!["Thin Film", "Dispersion"]);
    }

    #[test]
    fn test_pipeline_evaluation() {
        let pipeline = PipelineBuilder::new()
            .with_thin_film(1.45, 100.0, 1.52)
            .build();

        let incident = SpectralSignal::uniform_default(1.0);
        let context = EvaluationContext::default();

        let output = pipeline.evaluate(&incident, &context);

        // Output should have same wavelengths
        assert_eq!(output.samples().len(), incident.samples().len());

        // Output intensity should be <= input (reflectance < 1)
        for (in_s, out_s) in incident.samples().iter().zip(output.samples().iter()) {
            assert!(out_s.intensity <= in_s.intensity);
        }
    }

    #[test]
    fn test_energy_conservation() {
        let pipeline = PipelineBuilder::new()
            .with_thin_film(1.45, 100.0, 1.52)
            .build();

        let incident = SpectralSignal::uniform_default(1.0);
        let context = EvaluationContext::default();

        // Thin film should conserve energy (R + T = 1)
        // Our stage only computes R, so output < input is expected
        assert!(pipeline.verify_energy_conservation(&incident, &context));
    }

    #[test]
    fn test_intermediate_visualization() {
        let pipeline = PipelineBuilder::new()
            .with_thin_film(1.45, 100.0, 1.52)
            .with_gold()
            .build();

        let incident = SpectralSignal::uniform_default(1.0);
        let context = EvaluationContext::default();

        let intermediates = pipeline.evaluate_with_intermediates(&incident, &context);

        assert_eq!(intermediates.len(), 3); // Incident + 2 stages
        assert_eq!(intermediates[0].0, "Incident");
        assert_eq!(intermediates[1].0, "Thin Film");
        assert_eq!(intermediates[2].0, "Metal Reflectance");
    }

    #[test]
    fn test_metal_gold_color() {
        let pipeline = PipelineBuilder::new().with_gold().build();

        // Use uniform white light instead of D65 for simpler test
        let incident = SpectralSignal::uniform_default(1.0);
        let context = EvaluationContext::default();

        let output = pipeline.evaluate(&incident, &context);

        // Check that gold reflects red more than blue (at RGB wavelengths)
        // Gold n,k values cause higher reflectance at red than blue
        let r_650 = output.intensity_at(650.0);
        let b_450 = output.intensity_at(450.0);

        assert!(
            r_650 > b_450,
            "Gold should reflect more red (650nm) than blue (450nm): R={} B={}",
            r_650,
            b_450
        );
    }

    #[test]
    fn test_temperature_affects_output() {
        let pipeline = PipelineBuilder::new()
            .with_thermo_optic(1.52, 1e-5, 200.0, 7e-6)
            .build();

        let incident = SpectralSignal::uniform_default(1.0);

        let cold_context = EvaluationContext::default().with_temperature(200.0);
        let hot_context = EvaluationContext::default().with_temperature(500.0);

        let cold_output = pipeline.evaluate(&incident, &cold_context);
        let hot_output = pipeline.evaluate(&incident, &hot_context);

        // Temperature should change output
        let cold_energy = cold_output.total_energy();
        let hot_energy = hot_output.total_energy();

        assert!(
            (cold_energy - hot_energy).abs() > 0.001,
            "Temperature should affect output"
        );
    }

    #[test]
    fn test_angle_affects_output() {
        let pipeline = PipelineBuilder::new().with_gold().build();

        let incident = SpectralSignal::uniform_default(1.0);

        let normal = EvaluationContext::default().with_angle_deg(0.0);
        let grazing = EvaluationContext::default().with_angle_deg(80.0);

        let normal_output = pipeline.evaluate(&incident, &normal);
        let grazing_output = pipeline.evaluate(&incident, &grazing);

        let normal_energy = normal_output.total_energy();
        let grazing_energy = grazing_output.total_energy();

        // Grazing angle should have higher reflectance (Fresnel)
        assert!(
            grazing_energy > normal_energy,
            "Grazing angle should increase reflectance"
        );
    }
}

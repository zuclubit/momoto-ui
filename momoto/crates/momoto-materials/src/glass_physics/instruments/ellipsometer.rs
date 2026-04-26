//! # Virtual Ellipsometer
//!
//! Simulates spectroscopic ellipsometry measurements for thin-film characterization.
//! Produces metrologically-traceable optical constants and film thickness data.

use crate::glass_physics::metrology::{
    Measurement, MeasurementId, MeasurementQuality, MeasurementSource, TraceabilityChain,
    Uncertainty, Unit,
};

use super::common::{EnvironmentConditions, InstrumentConfig, NoiseModel, Resolution, SimpleRng};

// ============================================================================
// ELLIPSOMETRY CONFIGURATION
// ============================================================================

/// Virtual ellipsometer for thin-film characterization.
#[derive(Debug, Clone)]
pub struct VirtualEllipsometer {
    /// Instrument configuration.
    pub config: InstrumentConfig,
    /// Angle of incidence in degrees.
    pub angle_of_incidence_deg: f64,
    /// Wavelength range (min, max) in nm.
    pub wavelength_range: (f64, f64),
    /// Wavelength step in nm.
    pub wavelength_step: f64,
    /// Ellipsometer type.
    pub ellipsometer_type: EllipsometerType,
    /// Random number generator.
    rng: SimpleRng,
}

/// Type of ellipsometer.
#[derive(Debug, Clone, Copy)]
pub enum EllipsometerType {
    /// Rotating analyzer ellipsometer.
    RotatingAnalyzer,
    /// Rotating polarizer ellipsometer.
    RotatingPolarizer,
    /// Rotating compensator ellipsometer.
    RotatingCompensator,
    /// Phase modulation ellipsometer.
    PhaseModulation,
}

impl VirtualEllipsometer {
    /// Create new ellipsometer with configuration.
    pub fn new(config: InstrumentConfig) -> Self {
        Self {
            config,
            angle_of_incidence_deg: 70.0,
            wavelength_range: (300.0, 1000.0),
            wavelength_step: 5.0,
            ellipsometer_type: EllipsometerType::RotatingCompensator,
            rng: SimpleRng::new(42),
        }
    }

    /// Create ideal (no noise) ellipsometer.
    pub fn ideal() -> Self {
        Self::new(InstrumentConfig::ideal("Ideal Ellipsometer"))
    }

    /// Create research-grade spectroscopic ellipsometer.
    pub fn research_grade() -> Self {
        let config = InstrumentConfig::new("Research Ellipsometer", "SE-2000")
            .with_noise(NoiseModel::gaussian(0.01)) // 0.01° uncertainty in Psi/Delta
            .with_resolution(Resolution::research_grade());

        Self {
            config,
            angle_of_incidence_deg: 70.0,
            wavelength_range: (190.0, 1700.0),
            wavelength_step: 2.0,
            ellipsometer_type: EllipsometerType::RotatingCompensator,
            rng: SimpleRng::new(42),
        }
    }

    /// Create industrial thin-film ellipsometer.
    pub fn industrial() -> Self {
        let config = InstrumentConfig::new("Industrial Ellipsometer", "IE-500")
            .with_noise(NoiseModel::gaussian(0.05))
            .with_resolution(Resolution::standard());

        Self {
            config,
            angle_of_incidence_deg: 75.0,
            wavelength_range: (400.0, 800.0),
            wavelength_step: 10.0,
            ellipsometer_type: EllipsometerType::RotatingAnalyzer,
            rng: SimpleRng::new(42),
        }
    }

    /// Set angle of incidence.
    pub fn with_angle(mut self, angle_deg: f64) -> Self {
        self.angle_of_incidence_deg = angle_deg;
        self
    }

    /// Set wavelength range.
    pub fn with_wavelength_range(mut self, min_nm: f64, max_nm: f64) -> Self {
        self.wavelength_range = (min_nm, max_nm);
        self
    }

    /// Set random seed for reproducibility.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.rng = SimpleRng::new(seed);
        self
    }

    /// Measure Psi and Delta at single wavelength.
    ///
    /// # Arguments
    /// * `optical_constants_fn` - Function (wavelength) -> (n, k)
    /// * `wavelength_nm` - Measurement wavelength
    pub fn measure_single<F>(
        &mut self,
        optical_constants_fn: F,
        wavelength_nm: f64,
    ) -> EllipsometryPoint
    where
        F: Fn(f64) -> (f64, f64), // wavelength -> (n, k)
    {
        let (n, k) = optical_constants_fn(wavelength_nm);

        // Calculate Psi and Delta from Fresnel equations
        let (psi, delta) = self.calculate_psi_delta(n, k, wavelength_nm);

        // Apply noise
        let (noisy_psi, psi_std) = self.config.noise_model.apply(psi, &mut || self.rng.next());
        let (noisy_delta, delta_std) = self
            .config
            .noise_model
            .apply(delta, &mut || self.rng.next());

        // Constrain to valid ranges
        let clamped_psi = noisy_psi.clamp(0.0, 90.0);
        let clamped_delta = ((noisy_delta % 360.0) + 360.0) % 360.0; // Wrap to 0-360

        let quality = if self.config.is_calibrated() {
            MeasurementQuality::Calibrated
        } else {
            MeasurementQuality::Validated
        };

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        EllipsometryPoint {
            wavelength_nm,
            psi: Measurement {
                id: MeasurementId::generate(),
                value: clamped_psi,
                uncertainty: Uncertainty::TypeA {
                    std_error: psi_std.abs(),
                    n_samples: 100,
                },
                unit: Unit::Degrees,
                confidence_level: 0.95,
                quality,
                timestamp,
                source: MeasurementSource::Instrument {
                    name: self.config.name.clone(),
                    model: self.config.model.clone(),
                },
            },
            delta: Measurement {
                id: MeasurementId::generate(),
                value: clamped_delta,
                uncertainty: Uncertainty::TypeA {
                    std_error: delta_std.abs(),
                    n_samples: 100,
                },
                unit: Unit::Degrees,
                confidence_level: 0.95,
                quality,
                timestamp,
                source: MeasurementSource::Instrument {
                    name: self.config.name.clone(),
                    model: self.config.model.clone(),
                },
            },
            angle_of_incidence_deg: self.angle_of_incidence_deg,
        }
    }

    /// Calculate Psi and Delta from optical constants.
    fn calculate_psi_delta(&self, n: f64, k: f64, _wavelength_nm: f64) -> (f64, f64) {
        let theta_i = self.angle_of_incidence_deg.to_radians();
        let cos_i = theta_i.cos();
        let sin_i = theta_i.sin();

        // Complex refractive index
        let n_complex = Complex::new(n, -k);
        let n0 = Complex::new(1.0, 0.0); // Air

        // Snell's law for complex
        let sin_t_sq = (n0.re / n_complex.re).powi(2) * sin_i.powi(2);
        let cos_t = (1.0 - sin_t_sq).sqrt();

        // Fresnel coefficients (simplified for real part)
        let rs = (n0.re * cos_i - n_complex.re * cos_t) / (n0.re * cos_i + n_complex.re * cos_t);
        let rp = (n_complex.re * cos_i - n0.re * cos_t) / (n_complex.re * cos_i + n0.re * cos_t);

        // rho = rp / rs = tan(Psi) * exp(i * Delta)
        let rho_magnitude = (rp / rs).abs();
        let psi_rad = rho_magnitude.atan();

        // For real coefficients, delta is 0 or 180
        let delta_deg = if rp * rs > 0.0 { 0.0 } else { 180.0 };

        (psi_rad.to_degrees(), delta_deg)
    }

    /// Perform spectroscopic ellipsometry measurement.
    pub fn measure_spectrum<F>(&mut self, optical_constants_fn: F) -> EllipsometryResult
    where
        F: Fn(f64) -> (f64, f64),
    {
        let mut points = Vec::new();

        let mut wl = self.wavelength_range.0;
        while wl <= self.wavelength_range.1 {
            let point = self.measure_single(&optical_constants_fn, wl);
            points.push(point);
            wl += self.wavelength_step;
        }

        let mut traceability = TraceabilityChain::new();
        traceability.record_measurement(
            &self.config.name,
            &format!(
                "Spectroscopic scan {:.0}-{:.0} nm at {:.1}°",
                self.wavelength_range.0, self.wavelength_range.1, self.angle_of_incidence_deg
            ),
            MeasurementId::generate(),
        );

        EllipsometryResult {
            points,
            angle_of_incidence_deg: self.angle_of_incidence_deg,
            ellipsometer_type: self.ellipsometer_type,
            traceability,
            environment: self.config.environment.clone(),
        }
    }

    /// Measure thin film on substrate.
    pub fn measure_thin_film<F, S>(
        &mut self,
        film_constants_fn: F,
        substrate_constants_fn: S,
        thickness_nm: f64,
    ) -> ThinFilmResult
    where
        F: Fn(f64) -> (f64, f64), // Film optical constants
        S: Fn(f64) -> (f64, f64), // Substrate optical constants
    {
        let mut points = Vec::new();

        let mut wl = self.wavelength_range.0;
        while wl <= self.wavelength_range.1 {
            let (n_film, k_film) = film_constants_fn(wl);
            let (n_sub, k_sub) = substrate_constants_fn(wl);

            // Calculate thin-film modified Psi/Delta (simplified model)
            let (psi, delta) =
                self.calculate_thin_film_psi_delta(n_film, k_film, n_sub, k_sub, thickness_nm, wl);

            // Apply noise
            let (noisy_psi, psi_std) = self.config.noise_model.apply(psi, &mut || self.rng.next());
            let (noisy_delta, delta_std) = self
                .config
                .noise_model
                .apply(delta, &mut || self.rng.next());

            let quality = if self.config.is_calibrated() {
                MeasurementQuality::Calibrated
            } else {
                MeasurementQuality::Validated
            };

            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            points.push(EllipsometryPoint {
                wavelength_nm: wl,
                psi: Measurement {
                    id: MeasurementId::generate(),
                    value: noisy_psi.clamp(0.0, 90.0),
                    uncertainty: Uncertainty::TypeA {
                        std_error: psi_std.abs(),
                        n_samples: 100,
                    },
                    unit: Unit::Degrees,
                    confidence_level: 0.95,
                    quality,
                    timestamp,
                    source: MeasurementSource::Instrument {
                        name: self.config.name.clone(),
                        model: self.config.model.clone(),
                    },
                },
                delta: Measurement {
                    id: MeasurementId::generate(),
                    value: ((noisy_delta % 360.0) + 360.0) % 360.0,
                    uncertainty: Uncertainty::TypeA {
                        std_error: delta_std.abs(),
                        n_samples: 100,
                    },
                    unit: Unit::Degrees,
                    confidence_level: 0.95,
                    quality,
                    timestamp,
                    source: MeasurementSource::Instrument {
                        name: self.config.name.clone(),
                        model: self.config.model.clone(),
                    },
                },
                angle_of_incidence_deg: self.angle_of_incidence_deg,
            });

            wl += self.wavelength_step;
        }

        // Estimate thickness from phase variation
        let estimated_thickness = self.estimate_thickness(&points);

        let mut traceability = TraceabilityChain::new();
        traceability.record_measurement(
            &self.config.name,
            "Thin-film measurement",
            MeasurementId::generate(),
        );

        ThinFilmResult {
            points,
            thickness: Measurement {
                id: MeasurementId::generate(),
                value: estimated_thickness,
                uncertainty: Uncertainty::Combined {
                    type_a: estimated_thickness * 0.02, // 2% precision
                    type_b: 0.5,                        // 0.5 nm systematic
                },
                unit: Unit::ThicknessNm,
                confidence_level: 0.95,
                quality: MeasurementQuality::Estimated,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                source: MeasurementSource::Calculated {
                    method: "Model fit".to_string(),
                },
            },
            traceability,
            environment: self.config.environment.clone(),
        }
    }

    /// Calculate thin-film Psi/Delta (simplified model).
    fn calculate_thin_film_psi_delta(
        &self,
        n_film: f64,
        k_film: f64,
        n_sub: f64,
        _k_sub: f64,
        thickness_nm: f64,
        wavelength_nm: f64,
    ) -> (f64, f64) {
        let theta_i = self.angle_of_incidence_deg.to_radians();
        let cos_i = theta_i.cos();

        // Phase shift from film
        let phase = 4.0 * std::f64::consts::PI * n_film * thickness_nm * cos_i / wavelength_nm;

        // Simplified: modulate base Psi/Delta by thin-film interference
        let base_psi = 35.0; // Typical Psi
        let base_delta = 90.0; // Typical Delta

        let psi = base_psi + 10.0 * (phase).sin() * (1.0 - k_film);
        let delta = base_delta + 90.0 * (phase).cos() * (n_sub - n_film).abs() / n_sub;

        (psi.clamp(0.0, 90.0), delta)
    }

    /// Estimate film thickness from spectral data.
    fn estimate_thickness(&self, points: &[EllipsometryPoint]) -> f64 {
        if points.len() < 3 {
            return 0.0;
        }

        // Find oscillation period in Delta
        let deltas: Vec<f64> = points.iter().map(|p| p.delta.value).collect();
        let wls: Vec<f64> = points.iter().map(|p| p.wavelength_nm).collect();

        // Simple: estimate from delta variation
        let delta_range = deltas.iter().cloned().fold(0.0, f64::max)
            - deltas.iter().cloned().fold(f64::INFINITY, f64::min);

        let wl_range = wls.last().unwrap() - wls.first().unwrap();

        // Very rough estimate (real fitting would be more complex)
        if delta_range > 10.0 {
            wl_range / (delta_range / 360.0 * 2.0)
        } else {
            100.0 // Default estimate
        }
    }
}

// ============================================================================
// ELLIPSOMETRY RESULTS
// ============================================================================

/// Single ellipsometry measurement point.
#[derive(Debug, Clone)]
pub struct EllipsometryPoint {
    /// Wavelength in nm.
    pub wavelength_nm: f64,
    /// Psi (amplitude ratio) in degrees.
    pub psi: Measurement<f64>,
    /// Delta (phase difference) in degrees.
    pub delta: Measurement<f64>,
    /// Angle of incidence in degrees.
    pub angle_of_incidence_deg: f64,
}

impl EllipsometryPoint {
    /// Calculate complex reflectance ratio rho = tan(Psi) * exp(i*Delta).
    pub fn rho(&self) -> (f64, f64) {
        let psi_rad = self.psi.value.to_radians();
        let delta_rad = self.delta.value.to_radians();

        let magnitude = psi_rad.tan();
        let real = magnitude * delta_rad.cos();
        let imag = magnitude * delta_rad.sin();

        (real, imag)
    }

    /// Calculate pseudo-dielectric function (approximate).
    pub fn pseudo_dielectric(&self) -> (f64, f64) {
        let theta_i = self.angle_of_incidence_deg.to_radians();
        let sin_i = theta_i.sin();

        let (rho_re, rho_im) = self.rho();
        let rho_sq = rho_re * rho_re + rho_im * rho_im;

        // <epsilon> = sin^2(theta) * (1 + tan^2(theta) * ((1-rho)/(1+rho))^2)
        let factor = (1.0 - rho_sq) / (1.0 + rho_sq + 2.0 * rho_re);

        let epsilon_re = sin_i * sin_i * (1.0 + theta_i.tan().powi(2) * factor);
        let epsilon_im = 0.0; // Simplified

        (epsilon_re, epsilon_im)
    }
}

/// Result from spectroscopic ellipsometry.
#[derive(Debug, Clone)]
pub struct EllipsometryResult {
    /// Measured points.
    pub points: Vec<EllipsometryPoint>,
    /// Angle of incidence in degrees.
    pub angle_of_incidence_deg: f64,
    /// Ellipsometer type.
    pub ellipsometer_type: EllipsometerType,
    /// Traceability chain.
    pub traceability: TraceabilityChain,
    /// Environmental conditions.
    pub environment: EnvironmentConditions,
}

impl EllipsometryResult {
    /// Get point at specific wavelength (interpolated).
    pub fn at_wavelength(&self, wavelength_nm: f64) -> Option<(f64, f64)> {
        for i in 0..self.points.len().saturating_sub(1) {
            if wavelength_nm >= self.points[i].wavelength_nm
                && wavelength_nm <= self.points[i + 1].wavelength_nm
            {
                let t = (wavelength_nm - self.points[i].wavelength_nm)
                    / (self.points[i + 1].wavelength_nm - self.points[i].wavelength_nm);

                let psi = self.points[i].psi.value * (1.0 - t) + self.points[i + 1].psi.value * t;
                let delta =
                    self.points[i].delta.value * (1.0 - t) + self.points[i + 1].delta.value * t;

                return Some((psi, delta));
            }
        }
        None
    }

    /// Get mean Psi over spectrum.
    pub fn mean_psi(&self) -> f64 {
        if self.points.is_empty() {
            return 0.0;
        }
        self.points.iter().map(|p| p.psi.value).sum::<f64>() / self.points.len() as f64
    }

    /// Get mean Delta over spectrum.
    pub fn mean_delta(&self) -> f64 {
        if self.points.is_empty() {
            return 0.0;
        }
        self.points.iter().map(|p| p.delta.value).sum::<f64>() / self.points.len() as f64
    }

    /// Generate measurement report.
    pub fn report(&self) -> String {
        let mut report = String::new();
        report.push_str("Ellipsometry Measurement Report\n");
        report.push_str(&format!(
            "Angle of Incidence: {:.1}°\n",
            self.angle_of_incidence_deg
        ));
        report.push_str(&format!(
            "Ellipsometer Type: {:?}\n",
            self.ellipsometer_type
        ));
        report.push_str(&format!("Number of Points: {}\n", self.points.len()));

        if !self.points.is_empty() {
            report.push_str(&format!(
                "Wavelength Range: {:.0}-{:.0} nm\n",
                self.points.first().map(|p| p.wavelength_nm).unwrap_or(0.0),
                self.points.last().map(|p| p.wavelength_nm).unwrap_or(0.0)
            ));
        }

        report.push_str(&format!("Mean Psi: {:.2}°\n", self.mean_psi()));
        report.push_str(&format!("Mean Delta: {:.2}°\n", self.mean_delta()));
        report
    }
}

/// Result from thin-film ellipsometry.
#[derive(Debug, Clone)]
pub struct ThinFilmResult {
    /// Measured points.
    pub points: Vec<EllipsometryPoint>,
    /// Estimated film thickness.
    pub thickness: Measurement<f64>,
    /// Traceability chain.
    pub traceability: TraceabilityChain,
    /// Environmental conditions.
    pub environment: EnvironmentConditions,
}

impl ThinFilmResult {
    /// Generate thin-film report.
    pub fn report(&self) -> String {
        let mut report = String::new();
        report.push_str("Thin-Film Ellipsometry Report\n");
        report.push_str(&format!(
            "Thickness: {:.1} ± {:.1} nm\n",
            self.thickness.value,
            self.thickness.uncertainty.standard()
        ));
        report.push_str(&format!("Number of Points: {}\n", self.points.len()));
        report.push_str(&format!("Quality: {:?}\n", self.thickness.quality));
        report
    }
}

// ============================================================================
// HELPER TYPES
// ============================================================================

/// Simple complex number for Fresnel calculations.
/// Note: Used for construction and Debug display only in this module.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct Complex {
    re: f64,
    im: f64,
}

impl Complex {
    fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }
}

// ============================================================================
// OPTICAL CONSTANT MODELS FOR TESTING
// ============================================================================

/// Constant optical constants (n, k).
pub fn constant_optical_constants(n: f64, k: f64) -> impl Fn(f64) -> (f64, f64) {
    move |_wavelength: f64| (n, k)
}

/// Cauchy dispersion model for dielectrics.
pub fn cauchy_dispersion(a: f64, b: f64, c: f64) -> impl Fn(f64) -> (f64, f64) {
    move |wavelength_nm: f64| {
        let wl_um = wavelength_nm / 1000.0;
        let n = a + b / wl_um.powi(2) + c / wl_um.powi(4);
        (n, 0.0) // k = 0 for transparent materials
    }
}

/// Silicon-like optical constants (approximate).
pub fn silicon_optical_constants() -> impl Fn(f64) -> (f64, f64) {
    move |wavelength_nm: f64| {
        // Very simplified Si model
        let n = 3.5 + 1000.0 / wavelength_nm;
        let k = if wavelength_nm < 400.0 {
            1.0
        } else if wavelength_nm < 600.0 {
            0.5
        } else {
            0.1
        };
        (n.min(6.0), k)
    }
}

/// Glass substrate (SiO2) optical constants.
pub fn glass_optical_constants() -> impl Fn(f64) -> (f64, f64) {
    cauchy_dispersion(1.458, 0.00354, 0.0)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ideal_ellipsometer() {
        let mut ellip = VirtualEllipsometer::ideal();
        let constants = constant_optical_constants(1.5, 0.0);

        let point = ellip.measure_single(constants, 550.0);

        assert!(point.psi.value >= 0.0 && point.psi.value <= 90.0);
        assert!(point.delta.value >= 0.0 && point.delta.value < 360.0);
    }

    #[test]
    fn test_spectroscopic_measurement() {
        let mut ellip = VirtualEllipsometer::ideal()
            .with_wavelength_range(400.0, 700.0)
            .with_seed(42);

        let constants = cauchy_dispersion(1.5, 0.004, 0.0);
        let result = ellip.measure_spectrum(constants);

        assert!(!result.points.is_empty());
        assert!(result.points.len() > 50); // Default step 5nm
    }

    #[test]
    fn test_angle_variation() {
        let constants = constant_optical_constants(1.5, 0.0);

        let mut ellip_70 = VirtualEllipsometer::ideal().with_angle(70.0);
        let mut ellip_75 = VirtualEllipsometer::ideal().with_angle(75.0);

        let point_70 = ellip_70.measure_single(&constants, 550.0);
        let point_75 = ellip_75.measure_single(&constants, 550.0);

        // Different angles should give different Psi
        assert!((point_70.psi.value - point_75.psi.value).abs() > 0.1);
    }

    #[test]
    fn test_thin_film_measurement() {
        let mut ellip = VirtualEllipsometer::ideal()
            .with_wavelength_range(400.0, 800.0)
            .with_seed(42);

        let film = constant_optical_constants(1.46, 0.0); // SiO2-like
        let substrate = silicon_optical_constants();

        let result = ellip.measure_thin_film(film, substrate, 100.0);

        assert!(result.thickness.value > 0.0);
        assert!(!result.points.is_empty());
    }

    #[test]
    fn test_reproducibility() {
        let constants = constant_optical_constants(1.5, 0.1);

        let mut ellip1 = VirtualEllipsometer::research_grade().with_seed(123);
        let mut ellip2 = VirtualEllipsometer::research_grade().with_seed(123);

        let point1 = ellip1.measure_single(&constants, 550.0);
        let point2 = ellip2.measure_single(&constants, 550.0);

        assert!((point1.psi.value - point2.psi.value).abs() < 1e-10);
        assert!((point1.delta.value - point2.delta.value).abs() < 1e-10);
    }

    #[test]
    fn test_rho_calculation() {
        let point = EllipsometryPoint {
            wavelength_nm: 550.0,
            psi: Measurement {
                id: MeasurementId::generate(),
                value: 45.0,
                uncertainty: Uncertainty::TypeA {
                    std_error: 0.1,
                    n_samples: 100,
                },
                unit: Unit::Degrees,
                confidence_level: 0.95,
                quality: MeasurementQuality::Calibrated,
                timestamp: 0,
                source: MeasurementSource::Unknown,
            },
            delta: Measurement {
                id: MeasurementId::generate(),
                value: 0.0,
                uncertainty: Uncertainty::TypeA {
                    std_error: 0.1,
                    n_samples: 100,
                },
                unit: Unit::Degrees,
                confidence_level: 0.95,
                quality: MeasurementQuality::Calibrated,
                timestamp: 0,
                source: MeasurementSource::Unknown,
            },
            angle_of_incidence_deg: 70.0,
        };

        let (rho_re, rho_im) = point.rho();
        // tan(45°) = 1, delta = 0 => rho = (1, 0)
        assert!((rho_re - 1.0).abs() < 0.01);
        assert!(rho_im.abs() < 0.01);
    }

    #[test]
    fn test_interpolation() {
        let mut ellip = VirtualEllipsometer::ideal()
            .with_wavelength_range(400.0, 700.0)
            .with_seed(42);

        let constants = constant_optical_constants(1.5, 0.0);
        let result = ellip.measure_spectrum(constants);

        // Interpolate at 550nm
        let (psi, delta) = result.at_wavelength(550.0).unwrap();
        assert!(psi > 0.0);
        assert!(delta >= 0.0);

        // Out of range
        assert!(result.at_wavelength(300.0).is_none());
    }

    #[test]
    fn test_result_report() {
        let mut ellip = VirtualEllipsometer::ideal()
            .with_wavelength_range(400.0, 600.0)
            .with_seed(42);

        let constants = constant_optical_constants(1.5, 0.0);
        let result = ellip.measure_spectrum(constants);
        let report = result.report();

        assert!(report.contains("Angle of Incidence"));
        assert!(report.contains("Mean Psi"));
        assert!(report.contains("Mean Delta"));
    }

    #[test]
    fn test_ellipsometer_types() {
        let ellip = VirtualEllipsometer::research_grade();
        assert!(matches!(
            ellip.ellipsometer_type,
            EllipsometerType::RotatingCompensator
        ));

        let industrial = VirtualEllipsometer::industrial();
        assert!(matches!(
            industrial.ellipsometer_type,
            EllipsometerType::RotatingAnalyzer
        ));
    }

    #[test]
    fn test_cauchy_dispersion() {
        let cauchy = cauchy_dispersion(1.5, 0.004, 0.0);

        let (n_400, k_400) = cauchy(400.0);
        let (n_700, k_700) = cauchy(700.0);

        // n should decrease with wavelength for normal dispersion
        assert!(n_400 > n_700);
        assert!((k_400).abs() < 1e-10);
        assert!((k_700).abs() < 1e-10);
    }

    #[test]
    fn test_silicon_model() {
        let si = silicon_optical_constants();

        let (n, k) = si(500.0);
        assert!(n > 3.0); // Si has high n
        assert!(k > 0.0); // Si absorbs in visible
    }
}

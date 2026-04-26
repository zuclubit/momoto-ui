//! # Virtual Gonioreflectometer
//!
//! Simulates angular-resolved reflectance measurements (BRDF).
//! Produces metrologically-traceable measurement arrays.

use crate::glass_physics::metrology::{
    Measurement, MeasurementArray, MeasurementId, MeasurementQuality, MeasurementSource,
    TraceabilityChain, Uncertainty, Unit,
};

use super::common::{
    DetectorGeometry, EnvironmentConditions, InstrumentConfig, LightSource, NoiseModel, Resolution,
    SimpleRng,
};

// ============================================================================
// GONIOREFLECTOMETER CONFIGURATION
// ============================================================================

/// Virtual gonioreflectometer for BRDF measurements.
#[derive(Debug, Clone)]
pub struct VirtualGonioreflectometer {
    /// Instrument configuration.
    pub config: InstrumentConfig,
    /// Incident angle range (min, max) in degrees.
    pub incident_range: (f64, f64),
    /// Reflected angle range (min, max) in degrees.
    pub reflected_range: (f64, f64),
    /// Angular step size in degrees.
    pub angular_step: f64,
    /// Detector geometry.
    pub detector: DetectorGeometry,
    /// Light source configuration.
    pub light_source: LightSource,
    /// Random number generator for noise.
    rng: SimpleRng,
}

impl VirtualGonioreflectometer {
    /// Create new gonioreflectometer with configuration.
    pub fn new(config: InstrumentConfig) -> Self {
        Self {
            config,
            incident_range: (0.0, 85.0),
            reflected_range: (0.0, 85.0),
            angular_step: 5.0,
            detector: DetectorGeometry::Point {
                solid_angle_sr: 0.001,
            },
            light_source: LightSource::default(),
            rng: SimpleRng::new(42),
        }
    }

    /// Create ideal (no noise) gonioreflectometer.
    pub fn ideal() -> Self {
        Self::new(InstrumentConfig::ideal("Ideal Gonioreflectometer"))
    }

    /// Create research-grade gonioreflectometer.
    pub fn research_grade() -> Self {
        let config = InstrumentConfig::new("Research Gonioreflectometer", "RG-2000")
            .with_noise(NoiseModel::combined(0.0005, 0.0001))
            .with_resolution(Resolution::research_grade());

        Self {
            config,
            incident_range: (0.0, 89.0),
            reflected_range: (0.0, 89.0),
            angular_step: 1.0,
            detector: DetectorGeometry::Point {
                solid_angle_sr: 0.0001,
            },
            light_source: LightSource::default(),
            rng: SimpleRng::new(42),
        }
    }

    /// Create industrial-grade gonioreflectometer.
    pub fn industrial_grade() -> Self {
        let config = InstrumentConfig::new("Industrial Gonioreflectometer", "IG-500")
            .with_noise(NoiseModel::combined(0.002, 0.0005))
            .with_resolution(Resolution::standard());

        Self {
            config,
            incident_range: (5.0, 80.0),
            reflected_range: (5.0, 80.0),
            angular_step: 5.0,
            detector: DetectorGeometry::Point {
                solid_angle_sr: 0.005,
            },
            light_source: LightSource::default(),
            rng: SimpleRng::new(42),
        }
    }

    /// Set angular step.
    pub fn with_angular_step(mut self, step_deg: f64) -> Self {
        self.angular_step = step_deg;
        self
    }

    /// Set incident angle range.
    pub fn with_incident_range(mut self, min_deg: f64, max_deg: f64) -> Self {
        self.incident_range = (min_deg, max_deg);
        self
    }

    /// Set detector geometry.
    pub fn with_detector(mut self, detector: DetectorGeometry) -> Self {
        self.detector = detector;
        self
    }

    /// Set random seed for reproducibility.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.rng = SimpleRng::new(seed);
        self
    }

    /// Measure specular reflectance at a single angle.
    pub fn measure_specular<F>(
        &mut self,
        brdf_fn: F,
        incident_deg: f64,
        wavelength_nm: f64,
    ) -> Measurement<f64>
    where
        F: Fn(f64, f64, f64) -> f64, // (theta_i, theta_o, wavelength) -> BRDF
    {
        // Specular: theta_o = theta_i
        let theta_i_rad = incident_deg.to_radians();
        let theta_o_rad = theta_i_rad;

        let true_value = brdf_fn(theta_i_rad, theta_o_rad, wavelength_nm);

        // Apply bias
        let biased = self.config.bias.apply_angular(true_value, incident_deg);

        // Apply noise
        let (noisy, noise_std) = self
            .config
            .noise_model
            .apply(biased, &mut || self.rng.next());

        // Clamp to valid range
        let clamped = noisy.clamp(
            self.config.resolution.detection_limit,
            self.config.resolution.saturation_limit,
        );

        // Quantize
        let quantized = (clamped / 1e-6).round() * 1e-6;

        let quality = if self.config.is_calibrated() {
            MeasurementQuality::Calibrated
        } else {
            MeasurementQuality::Validated
        };

        Measurement {
            id: MeasurementId::generate(),
            value: quantized,
            uncertainty: Uncertainty::Combined {
                type_a: noise_std,
                type_b: self.config.bias.offset.abs() * 0.1,
            },
            unit: Unit::PerSteradian,
            confidence_level: 0.95,
            quality,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            source: MeasurementSource::Instrument {
                name: self.config.name.clone(),
                model: self.config.model.clone(),
            },
        }
    }

    /// Measure angular reflectance distribution.
    pub fn measure_angular<F>(
        &mut self,
        brdf_fn: F,
        incident_deg: f64,
        wavelength_nm: f64,
    ) -> GoniometerResult
    where
        F: Fn(f64, f64, f64) -> f64,
    {
        let theta_i_rad = incident_deg.to_radians();

        let mut angles = Vec::new();
        let mut measurements = Vec::new();

        let mut theta_o = self.reflected_range.0;
        while theta_o <= self.reflected_range.1 {
            let theta_o_rad = theta_o.to_radians();
            let true_value = brdf_fn(theta_i_rad, theta_o_rad, wavelength_nm);

            // Apply instrument effects
            let biased = self.config.bias.apply_angular(true_value, theta_o);
            let (noisy, noise_std) = self
                .config
                .noise_model
                .apply(biased, &mut || self.rng.next());
            let clamped = noisy.clamp(
                self.config.resolution.detection_limit,
                self.config.resolution.saturation_limit,
            );

            angles.push(theta_o);
            measurements.push((clamped, noise_std));

            theta_o += self.angular_step;
        }

        // Create measurement array
        let values: Vec<f64> = measurements.iter().map(|(v, _)| *v).collect();
        let uncertainties: Vec<f64> = measurements.iter().map(|(_, u)| *u).collect();

        let quality = if self.config.is_calibrated() {
            MeasurementQuality::Calibrated
        } else {
            MeasurementQuality::Validated
        };

        let array = MeasurementArray {
            values,
            uncertainties,
            unit: Unit::PerSteradian,
            quality,
            domain: angles.clone(),
            domain_unit: Unit::Degrees,
        };

        // Build traceability chain
        let mut traceability = TraceabilityChain::new();
        traceability.record_measurement(
            &self.config.name,
            &format!("Angular scan at {}° incidence", incident_deg),
            MeasurementId::generate(),
        );

        GoniometerResult {
            incident_angle_deg: incident_deg,
            wavelength_nm,
            reflected_angles_deg: angles,
            measurements: array,
            traceability,
            environment: self.config.environment.clone(),
        }
    }

    /// Measure full BRDF hemisphere.
    pub fn measure_hemisphere<F>(&mut self, brdf_fn: F, wavelength_nm: f64) -> Vec<GoniometerResult>
    where
        F: Fn(f64, f64, f64) -> f64,
    {
        let mut results = Vec::new();

        let mut theta_i = self.incident_range.0;
        while theta_i <= self.incident_range.1 {
            let result = self.measure_angular(&brdf_fn, theta_i, wavelength_nm);
            results.push(result);
            theta_i += self.angular_step;
        }

        results
    }

    /// Compute directional-hemispherical reflectance (DHR) from angular scan.
    pub fn compute_dhr(&self, result: &GoniometerResult) -> Measurement<f64> {
        // Integrate over hemisphere using cosine-weighted integration
        let mut integral = 0.0;
        let mut variance = 0.0;

        for (i, &theta_deg) in result.reflected_angles_deg.iter().enumerate() {
            let theta_rad = theta_deg.to_radians();
            let cos_theta = theta_rad.cos();
            let sin_theta = theta_rad.sin();

            // Weight by solid angle element
            let d_omega = 2.0 * std::f64::consts::PI * sin_theta * self.angular_step.to_radians();

            let brdf = result.measurements.values[i];
            let brdf_unc = result.measurements.uncertainties[i];

            integral += brdf * cos_theta * d_omega;
            variance += (brdf_unc * cos_theta * d_omega).powi(2);
        }

        // DHR = integral of BRDF * cos(theta) over hemisphere
        let dhr = integral;
        let dhr_uncertainty = variance.sqrt();

        Measurement {
            id: MeasurementId::generate(),
            value: dhr.clamp(0.0, 1.0),
            uncertainty: Uncertainty::Combined {
                type_a: dhr_uncertainty,
                type_b: 0.005, // Systematic from integration method
            },
            unit: Unit::Reflectance,
            confidence_level: 0.95,
            quality: result.measurements.quality,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            source: MeasurementSource::Calculated {
                method: "DHR integration".to_string(),
            },
        }
    }
}

// ============================================================================
// GONIOMETER RESULT
// ============================================================================

/// Result from gonioreflectometer measurement.
#[derive(Debug, Clone)]
pub struct GoniometerResult {
    /// Incident angle in degrees.
    pub incident_angle_deg: f64,
    /// Measurement wavelength in nm.
    pub wavelength_nm: f64,
    /// Reflected angles measured.
    pub reflected_angles_deg: Vec<f64>,
    /// BRDF measurements at each angle.
    pub measurements: MeasurementArray,
    /// Traceability chain.
    pub traceability: TraceabilityChain,
    /// Environmental conditions.
    pub environment: EnvironmentConditions,
}

impl GoniometerResult {
    /// Get specular peak value.
    pub fn specular_peak(&self) -> Option<(f64, f64)> {
        // Find maximum near specular angle
        let specular_angle = self.incident_angle_deg;

        self.reflected_angles_deg
            .iter()
            .enumerate()
            .filter(|(_, &angle)| (angle - specular_angle).abs() < 5.0)
            .max_by(|a, b| {
                self.measurements.values[a.0]
                    .partial_cmp(&self.measurements.values[b.0])
                    .unwrap()
            })
            .map(|(i, &angle)| (angle, self.measurements.values[i]))
    }

    /// Get mean BRDF over angular range.
    pub fn mean_brdf(&self) -> f64 {
        if self.measurements.values.is_empty() {
            return 0.0;
        }
        self.measurements.values.iter().sum::<f64>() / self.measurements.values.len() as f64
    }

    /// Get maximum uncertainty.
    pub fn max_uncertainty(&self) -> f64 {
        self.measurements
            .uncertainties
            .iter()
            .cloned()
            .fold(0.0, f64::max)
    }

    /// Check if measurement has good quality.
    pub fn is_high_quality(&self) -> bool {
        matches!(
            self.measurements.quality,
            MeasurementQuality::Calibrated | MeasurementQuality::Validated
        )
    }

    /// Generate measurement report.
    pub fn report(&self) -> String {
        let mut report = String::new();
        report.push_str(&format!("Gonioreflectometer Measurement Report\n"));
        report.push_str(&format!(
            "Incident Angle: {:.1}°\n",
            self.incident_angle_deg
        ));
        report.push_str(&format!("Wavelength: {:.1} nm\n", self.wavelength_nm));
        report.push_str(&format!(
            "Angular Range: {:.1}° to {:.1}°\n",
            self.reflected_angles_deg.first().unwrap_or(&0.0),
            self.reflected_angles_deg.last().unwrap_or(&0.0)
        ));
        report.push_str(&format!(
            "Number of Points: {}\n",
            self.reflected_angles_deg.len()
        ));
        report.push_str(&format!("Mean BRDF: {:.6} sr⁻¹\n", self.mean_brdf()));
        report.push_str(&format!("Max Uncertainty: {:.6}\n", self.max_uncertainty()));

        if let Some((angle, value)) = self.specular_peak() {
            report.push_str(&format!(
                "Specular Peak: {:.6} sr⁻¹ at {:.1}°\n",
                value, angle
            ));
        }

        report.push_str(&format!("Quality: {:?}\n", self.measurements.quality));
        report
    }
}

// ============================================================================
// BRDF MODELS FOR TESTING
// ============================================================================

/// Simple Lambertian BRDF for testing.
pub fn lambertian_brdf(albedo: f64) -> impl Fn(f64, f64, f64) -> f64 {
    move |_theta_i: f64, _theta_o: f64, _wavelength: f64| albedo / std::f64::consts::PI
}

/// Phong-like BRDF for testing.
pub fn phong_brdf(diffuse: f64, specular: f64, exponent: f64) -> impl Fn(f64, f64, f64) -> f64 {
    move |theta_i: f64, theta_o: f64, _wavelength: f64| {
        let diffuse_term = diffuse / std::f64::consts::PI;

        // Specular: maximum when theta_o = theta_i
        let angle_diff = (theta_o - theta_i).abs();
        let specular_term = specular * angle_diff.cos().max(0.0).powf(exponent);

        diffuse_term + specular_term
    }
}

/// Fresnel reflectance for dielectric.
pub fn fresnel_brdf(n: f64) -> impl Fn(f64, f64, f64) -> f64 {
    move |theta_i: f64, theta_o: f64, _wavelength: f64| {
        // Only return value at specular angle
        if (theta_o - theta_i).abs() > 0.1 {
            return 0.0;
        }

        let cos_i = theta_i.cos();
        let sin_t = (1.0 - cos_i * cos_i) / (n * n);

        if sin_t > 1.0 {
            // Total internal reflection
            return 1.0 / std::f64::consts::PI;
        }

        let cos_t = (1.0 - sin_t).sqrt();

        let rs = ((cos_i - n * cos_t) / (cos_i + n * cos_t)).powi(2);
        let rp = ((n * cos_i - cos_t) / (n * cos_i + cos_t)).powi(2);

        (rs + rp) / 2.0 / std::f64::consts::PI
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ideal_gonioreflectometer() {
        let mut gonio = VirtualGonioreflectometer::ideal();
        let brdf = lambertian_brdf(0.5);

        let measurement = gonio.measure_specular(brdf, 45.0, 550.0);
        let expected = 0.5 / std::f64::consts::PI;

        assert!((measurement.value - expected).abs() < 1e-4);
    }

    #[test]
    fn test_angular_measurement() {
        let mut gonio = VirtualGonioreflectometer::ideal()
            .with_angular_step(10.0)
            .with_incident_range(0.0, 80.0);

        let brdf = lambertian_brdf(0.8);
        let result = gonio.measure_angular(brdf, 30.0, 550.0);

        assert!(!result.reflected_angles_deg.is_empty());
        assert_eq!(
            result.reflected_angles_deg.len(),
            result.measurements.values.len()
        );

        // Lambertian should be constant
        let expected = 0.8 / std::f64::consts::PI;
        for &value in &result.measurements.values {
            assert!((value - expected).abs() < 0.01);
        }
    }

    #[test]
    fn test_noisy_measurement() {
        let mut gonio = VirtualGonioreflectometer::industrial_grade().with_seed(123);
        let brdf = lambertian_brdf(0.5);

        let m1 = gonio.measure_specular(&brdf, 45.0, 550.0);

        // Reset seed for same noise
        gonio = gonio.with_seed(123);
        let m2 = gonio.measure_specular(&brdf, 45.0, 550.0);

        // Should be reproducible with same seed
        assert!((m1.value - m2.value).abs() < 1e-10);
    }

    #[test]
    fn test_phong_specular() {
        let mut gonio = VirtualGonioreflectometer::ideal().with_angular_step(1.0);
        let brdf = phong_brdf(0.2, 0.8, 50.0);

        let result = gonio.measure_angular(brdf, 45.0, 550.0);

        // Should have peak near specular angle
        let (peak_angle, peak_value) = result.specular_peak().unwrap();
        assert!((peak_angle - 45.0).abs() < 2.0);
        assert!(peak_value > result.mean_brdf());
    }

    #[test]
    fn test_dhr_computation() {
        let mut gonio = VirtualGonioreflectometer::ideal()
            .with_angular_step(2.0)
            .with_incident_range(0.0, 85.0);

        let brdf = lambertian_brdf(0.6);
        let result = gonio.measure_angular(brdf, 0.0, 550.0);

        let dhr = gonio.compute_dhr(&result);

        // Lambertian DHR should equal albedo
        assert!((dhr.value - 0.6).abs() < 0.1);
    }

    #[test]
    fn test_hemisphere_measurement() {
        let mut gonio = VirtualGonioreflectometer::ideal()
            .with_angular_step(15.0)
            .with_incident_range(0.0, 60.0);

        let brdf = lambertian_brdf(0.5);
        let results = gonio.measure_hemisphere(brdf, 550.0);

        assert!(!results.is_empty());
        assert_eq!(results.len(), 5); // 0, 15, 30, 45, 60
    }

    #[test]
    fn test_fresnel_brdf() {
        let brdf = fresnel_brdf(1.5);
        let value = brdf(0.0, 0.0, 550.0); // Normal incidence

        // Fresnel at normal incidence: ((n-1)/(n+1))^2 = 0.04
        // Divided by pi for BRDF
        let expected = 0.04 / std::f64::consts::PI;
        assert!((value - expected).abs() < 0.01);
    }

    #[test]
    fn test_result_report() {
        let mut gonio = VirtualGonioreflectometer::ideal().with_angular_step(10.0);
        let brdf = lambertian_brdf(0.5);
        let result = gonio.measure_angular(brdf, 30.0, 550.0);

        let report = result.report();
        assert!(report.contains("Incident Angle: 30.0°"));
        assert!(report.contains("Wavelength: 550.0 nm"));
        assert!(report.contains("Mean BRDF"));
    }

    #[test]
    fn test_calibrated_quality() {
        let calibration = crate::glass_physics::metrology::CalibrationReference::new(
            "NIST Standard",
            "CAL-2024-001",
        );

        let config = InstrumentConfig::new("Calibrated Gonio", "1.0").with_calibration(calibration);

        let mut gonio = VirtualGonioreflectometer::new(config);
        let brdf = lambertian_brdf(0.5);
        let measurement = gonio.measure_specular(brdf, 45.0, 550.0);

        assert_eq!(measurement.quality, MeasurementQuality::Calibrated);
    }

    #[test]
    #[ignore = "Requires TraceabilityOperation type implementation"]
    fn test_traceability() {
        // TODO: Implement when TraceabilityOperation is available
    }
}

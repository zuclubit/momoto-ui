//! # Virtual Spectrophotometer
//!
//! Simulates spectral reflectance and transmittance measurements.
//! Produces metrologically-traceable spectral data.

use crate::glass_physics::metrology::{
    Measurement, MeasurementArray, MeasurementId, MeasurementQuality, MeasurementSource,
    TraceabilityChain, Uncertainty, Unit,
};

use super::common::{
    EnvironmentConditions, InstrumentConfig, LightSource, LightSourceType, NoiseModel, Resolution,
    SimpleRng,
};

// ============================================================================
// SPECTROPHOTOMETER CONFIGURATION
// ============================================================================

/// Virtual spectrophotometer for spectral measurements.
#[derive(Debug, Clone)]
pub struct VirtualSpectrophotometer {
    /// Instrument configuration.
    pub config: InstrumentConfig,
    /// Wavelength range (min, max) in nm.
    pub wavelength_range: (f64, f64),
    /// Wavelength step in nm.
    pub wavelength_step: f64,
    /// Integration time in milliseconds.
    pub integration_time_ms: f64,
    /// Slit width in nm (spectral bandwidth).
    pub slit_width_nm: f64,
    /// Light source.
    pub light_source: LightSource,
    /// Measurement geometry.
    pub geometry: SpectroGeometry,
    /// Random number generator.
    rng: SimpleRng,
}

/// Measurement geometry.
#[derive(Debug, Clone, Copy)]
pub enum SpectroGeometry {
    /// Normal incidence (0°/0°).
    Normal,
    /// 8° incidence, specular included (8°/d).
    D8Specular,
    /// 8° incidence, specular excluded (8°/de).
    D8Diffuse,
    /// Integrating sphere total reflectance.
    IntegratingSphere,
    /// Variable angle reflectance.
    VariableAngle {
        /// Incident angle in degrees.
        incident_deg: f64,
    },
}

impl VirtualSpectrophotometer {
    /// Create new spectrophotometer with configuration.
    pub fn new(config: InstrumentConfig) -> Self {
        Self {
            config,
            wavelength_range: (380.0, 780.0),
            wavelength_step: 5.0,
            integration_time_ms: 100.0,
            slit_width_nm: 2.0,
            light_source: LightSource::default(),
            geometry: SpectroGeometry::Normal,
            rng: SimpleRng::new(42),
        }
    }

    /// Create ideal (no noise) spectrophotometer.
    pub fn ideal() -> Self {
        Self::new(InstrumentConfig::ideal("Ideal Spectrophotometer"))
    }

    /// Create research-grade UV-Vis spectrophotometer.
    pub fn research_uv_vis() -> Self {
        let config = InstrumentConfig::new("Research UV-Vis", "UV-2600")
            .with_noise(NoiseModel::combined(0.0002, 0.00005))
            .with_resolution(Resolution::research_grade());

        Self {
            config,
            wavelength_range: (200.0, 1100.0),
            wavelength_step: 1.0,
            integration_time_ms: 200.0,
            slit_width_nm: 1.0,
            light_source: LightSource {
                source_type: LightSourceType::Deuterium,
                wavelength_range: (200.0, 1100.0),
                power_density: 1.0,
                polarization: super::common::Polarization::Unpolarized,
            },
            geometry: SpectroGeometry::Normal,
            rng: SimpleRng::new(42),
        }
    }

    /// Create color measurement spectrophotometer.
    pub fn color_measurement() -> Self {
        let config = InstrumentConfig::new("Color Spectrophotometer", "CM-5")
            .with_noise(NoiseModel::gaussian(0.0005))
            .with_resolution(Resolution::high_precision());

        Self {
            config,
            wavelength_range: (360.0, 740.0),
            wavelength_step: 10.0,
            integration_time_ms: 50.0,
            slit_width_nm: 10.0,
            light_source: LightSource::default(),
            geometry: SpectroGeometry::D8Specular,
            rng: SimpleRng::new(42),
        }
    }

    /// Set wavelength range.
    pub fn with_wavelength_range(mut self, min_nm: f64, max_nm: f64) -> Self {
        self.wavelength_range = (min_nm, max_nm);
        self
    }

    /// Set wavelength step.
    pub fn with_wavelength_step(mut self, step_nm: f64) -> Self {
        self.wavelength_step = step_nm;
        self
    }

    /// Set measurement geometry.
    pub fn with_geometry(mut self, geometry: SpectroGeometry) -> Self {
        self.geometry = geometry;
        self
    }

    /// Set random seed for reproducibility.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.rng = SimpleRng::new(seed);
        self
    }

    /// Measure reflectance at single wavelength.
    pub fn measure_reflectance_single<F>(
        &mut self,
        reflectance_fn: F,
        wavelength_nm: f64,
    ) -> Measurement<f64>
    where
        F: Fn(f64) -> f64, // wavelength -> reflectance
    {
        let _angle_deg = match self.geometry {
            SpectroGeometry::Normal => 0.0,
            SpectroGeometry::D8Specular | SpectroGeometry::D8Diffuse => 8.0,
            SpectroGeometry::IntegratingSphere => 0.0,
            SpectroGeometry::VariableAngle { incident_deg } => incident_deg,
        };

        let true_value = reflectance_fn(wavelength_nm);

        // Apply spectral bandwidth averaging
        let averaged = if self.slit_width_nm > 0.1 {
            self.apply_bandwidth_averaging(&reflectance_fn, wavelength_nm)
        } else {
            true_value
        };

        // Apply bias
        let biased = self.config.bias.apply_spectral(averaged, wavelength_nm);

        // Apply noise (signal-dependent for photometric measurements)
        let (noisy, noise_std) = self
            .config
            .noise_model
            .apply(biased, &mut || self.rng.next());

        // Clamp and quantize
        let clamped = noisy.clamp(0.0, 1.0);

        let quality = if self.config.is_calibrated() {
            MeasurementQuality::Calibrated
        } else {
            MeasurementQuality::Validated
        };

        Measurement {
            id: MeasurementId::generate(),
            value: clamped,
            uncertainty: Uncertainty::Combined {
                type_a: noise_std,
                type_b: 0.001, // Typical photometric accuracy
            },
            unit: Unit::Reflectance,
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

    /// Apply bandwidth averaging (simulating slit width effect).
    fn apply_bandwidth_averaging<F>(&self, func: &F, center_nm: f64) -> f64
    where
        F: Fn(f64) -> f64,
    {
        let half_width = self.slit_width_nm / 2.0;
        let n_samples = 5;
        let step = self.slit_width_nm / n_samples as f64;

        let mut sum = 0.0;
        let mut weight_sum = 0.0;

        for i in 0..n_samples {
            let offset = -half_width + step * (i as f64 + 0.5);
            let wl = center_nm + offset;

            // Triangular slit function
            let weight = 1.0 - (offset / half_width).abs();

            sum += func(wl) * weight;
            weight_sum += weight;
        }

        if weight_sum > 0.0 {
            sum / weight_sum
        } else {
            func(center_nm)
        }
    }

    /// Measure spectral reflectance over wavelength range.
    pub fn measure_reflectance<F>(&mut self, reflectance_fn: F) -> SpectroResult
    where
        F: Fn(f64) -> f64,
    {
        let mut wavelengths = Vec::new();
        let mut values = Vec::new();
        let mut uncertainties = Vec::new();

        let mut wl = self.wavelength_range.0;
        while wl <= self.wavelength_range.1 {
            let measurement = self.measure_reflectance_single(&reflectance_fn, wl);

            wavelengths.push(wl);
            values.push(measurement.value);
            uncertainties.push(measurement.uncertainty.standard());

            wl += self.wavelength_step;
        }

        let quality = if self.config.is_calibrated() {
            MeasurementQuality::Calibrated
        } else {
            MeasurementQuality::Validated
        };

        let array = MeasurementArray {
            values,
            uncertainties,
            unit: Unit::Reflectance,
            quality,
            domain: wavelengths.clone(),
            domain_unit: Unit::Nanometers,
        };

        let mut traceability = TraceabilityChain::new();
        traceability.record_measurement(
            &self.config.name,
            &format!(
                "Spectral scan {:.0}-{:.0} nm",
                self.wavelength_range.0, self.wavelength_range.1
            ),
            MeasurementId::generate(),
        );

        SpectroResult {
            measurement_type: SpectroMeasurementType::Reflectance,
            geometry: self.geometry,
            wavelengths,
            measurements: array,
            traceability,
            environment: self.config.environment.clone(),
        }
    }

    /// Measure spectral transmittance.
    pub fn measure_transmittance<F>(&mut self, transmittance_fn: F) -> SpectroResult
    where
        F: Fn(f64) -> f64,
    {
        let mut wavelengths = Vec::new();
        let mut values = Vec::new();
        let mut uncertainties = Vec::new();

        let mut wl = self.wavelength_range.0;
        while wl <= self.wavelength_range.1 {
            let true_value = transmittance_fn(wl);

            // Apply bandwidth averaging
            let averaged = if self.slit_width_nm > 0.1 {
                self.apply_bandwidth_averaging(&transmittance_fn, wl)
            } else {
                true_value
            };

            // Apply bias and noise
            let biased = self.config.bias.apply_spectral(averaged, wl);
            let (noisy, noise_std) = self
                .config
                .noise_model
                .apply(biased, &mut || self.rng.next());
            let clamped = noisy.clamp(0.0, 1.0);

            wavelengths.push(wl);
            values.push(clamped);
            uncertainties.push(noise_std);

            wl += self.wavelength_step;
        }

        let quality = if self.config.is_calibrated() {
            MeasurementQuality::Calibrated
        } else {
            MeasurementQuality::Validated
        };

        let array = MeasurementArray {
            values,
            uncertainties,
            unit: Unit::Transmittance,
            quality,
            domain: wavelengths.clone(),
            domain_unit: Unit::Nanometers,
        };

        let mut traceability = TraceabilityChain::new();
        traceability.record_measurement(
            &self.config.name,
            "Transmittance scan",
            MeasurementId::generate(),
        );

        SpectroResult {
            measurement_type: SpectroMeasurementType::Transmittance,
            geometry: self.geometry,
            wavelengths,
            measurements: array,
            traceability,
            environment: self.config.environment.clone(),
        }
    }

    /// Measure absorbance (optical density).
    pub fn measure_absorbance<F>(&mut self, transmittance_fn: F) -> SpectroResult
    where
        F: Fn(f64) -> f64,
    {
        let mut wavelengths = Vec::new();
        let mut values = Vec::new();
        let mut uncertainties = Vec::new();

        let mut wl = self.wavelength_range.0;
        while wl <= self.wavelength_range.1 {
            let transmittance = transmittance_fn(wl);

            // Apply measurement
            let biased = self.config.bias.apply_spectral(transmittance, wl);
            let (noisy, noise_std) = self
                .config
                .noise_model
                .apply(biased, &mut || self.rng.next());
            let t_clamped = noisy.clamp(1e-6, 1.0);

            // Convert to absorbance: A = -log10(T)
            let absorbance = -t_clamped.log10();

            // Propagate uncertainty: dA = dT / (T * ln(10))
            let abs_uncertainty = noise_std / (t_clamped * std::f64::consts::LN_10);

            wavelengths.push(wl);
            values.push(absorbance);
            uncertainties.push(abs_uncertainty);

            wl += self.wavelength_step;
        }

        let quality = if self.config.is_calibrated() {
            MeasurementQuality::Calibrated
        } else {
            MeasurementQuality::Validated
        };

        let array = MeasurementArray {
            values,
            uncertainties,
            unit: Unit::Dimensionless, // Absorbance is dimensionless
            quality,
            domain: wavelengths.clone(),
            domain_unit: Unit::Nanometers,
        };

        let mut traceability = TraceabilityChain::new();
        traceability.record_measurement(
            &self.config.name,
            "Absorbance scan",
            MeasurementId::generate(),
        );

        SpectroResult {
            measurement_type: SpectroMeasurementType::Absorbance,
            geometry: self.geometry,
            wavelengths,
            measurements: array,
            traceability,
            environment: self.config.environment.clone(),
        }
    }
}

// ============================================================================
// SPECTROPHOTOMETER RESULT
// ============================================================================

/// Type of spectrophotometric measurement.
#[derive(Debug, Clone, Copy)]
pub enum SpectroMeasurementType {
    /// Reflectance (R).
    Reflectance,
    /// Transmittance (T).
    Transmittance,
    /// Absorbance (A = -log10(T)).
    Absorbance,
}

/// Result from spectrophotometer measurement.
#[derive(Debug, Clone)]
pub struct SpectroResult {
    /// Measurement type.
    pub measurement_type: SpectroMeasurementType,
    /// Measurement geometry.
    pub geometry: SpectroGeometry,
    /// Wavelengths measured.
    pub wavelengths: Vec<f64>,
    /// Spectral measurements.
    pub measurements: MeasurementArray,
    /// Traceability chain.
    pub traceability: TraceabilityChain,
    /// Environmental conditions.
    pub environment: EnvironmentConditions,
}

impl SpectroResult {
    /// Get value at specific wavelength (interpolated).
    pub fn at_wavelength(&self, wavelength_nm: f64) -> Option<f64> {
        if self.wavelengths.is_empty() {
            return None;
        }

        if wavelength_nm < self.wavelengths[0] || wavelength_nm > *self.wavelengths.last()? {
            return None;
        }

        // Find bracketing wavelengths
        for i in 0..self.wavelengths.len() - 1 {
            if wavelength_nm >= self.wavelengths[i] && wavelength_nm <= self.wavelengths[i + 1] {
                let t = (wavelength_nm - self.wavelengths[i])
                    / (self.wavelengths[i + 1] - self.wavelengths[i]);
                return Some(
                    self.measurements.values[i] * (1.0 - t) + self.measurements.values[i + 1] * t,
                );
            }
        }

        None
    }

    /// Get mean value over wavelength range.
    pub fn mean_value(&self) -> f64 {
        if self.measurements.values.is_empty() {
            return 0.0;
        }
        self.measurements.values.iter().sum::<f64>() / self.measurements.values.len() as f64
    }

    /// Get value at peak wavelength.
    pub fn peak(&self) -> Option<(f64, f64)> {
        self.wavelengths
            .iter()
            .enumerate()
            .max_by(|a, b| {
                self.measurements.values[a.0]
                    .partial_cmp(&self.measurements.values[b.0])
                    .unwrap()
            })
            .map(|(i, &wl)| (wl, self.measurements.values[i]))
    }

    /// Get wavelength-weighted average.
    pub fn weighted_average(&self, weights: &[f64]) -> f64 {
        if weights.len() != self.measurements.values.len() {
            return self.mean_value();
        }

        let weighted_sum: f64 = self
            .measurements
            .values
            .iter()
            .zip(weights.iter())
            .map(|(&v, &w)| v * w)
            .sum();
        let weight_sum: f64 = weights.iter().sum();

        if weight_sum > 0.0 {
            weighted_sum / weight_sum
        } else {
            0.0
        }
    }

    /// Compute tristimulus X (CIE 1931) - simplified.
    pub fn tristimulus_x(&self) -> f64 {
        // Simplified: just integrate with rough X-bar weighting
        let mut sum = 0.0;
        for (i, &wl) in self.wavelengths.iter().enumerate() {
            let x_bar = self.cie_x_bar(wl);
            sum += self.measurements.values[i] * x_bar;
        }
        sum * (self.wavelengths[1] - self.wavelengths[0])
    }

    /// Compute tristimulus Y (CIE 1931) - simplified.
    pub fn tristimulus_y(&self) -> f64 {
        let mut sum = 0.0;
        for (i, &wl) in self.wavelengths.iter().enumerate() {
            let y_bar = self.cie_y_bar(wl);
            sum += self.measurements.values[i] * y_bar;
        }
        sum * (self.wavelengths[1] - self.wavelengths[0])
    }

    /// Compute tristimulus Z (CIE 1931) - simplified.
    pub fn tristimulus_z(&self) -> f64 {
        let mut sum = 0.0;
        for (i, &wl) in self.wavelengths.iter().enumerate() {
            let z_bar = self.cie_z_bar(wl);
            sum += self.measurements.values[i] * z_bar;
        }
        sum * (self.wavelengths[1] - self.wavelengths[0])
    }

    /// Simplified CIE x-bar (Gaussian approximation).
    fn cie_x_bar(&self, wl: f64) -> f64 {
        1.056 * gaussian(wl, 599.8, 37.9) + 0.362 * gaussian(wl, 442.0, 16.0)
            - 0.065 * gaussian(wl, 501.1, 20.4)
    }

    /// Simplified CIE y-bar (Gaussian approximation).
    fn cie_y_bar(&self, wl: f64) -> f64 {
        0.821 * gaussian(wl, 568.8, 46.9) + 0.286 * gaussian(wl, 530.9, 16.3)
    }

    /// Simplified CIE z-bar (Gaussian approximation).
    fn cie_z_bar(&self, wl: f64) -> f64 {
        1.217 * gaussian(wl, 437.0, 11.8) + 0.681 * gaussian(wl, 459.0, 26.0)
    }

    /// Generate measurement report.
    pub fn report(&self) -> String {
        let mut report = String::new();
        report.push_str("Spectrophotometer Measurement Report\n");
        report.push_str(&format!("Type: {:?}\n", self.measurement_type));
        report.push_str(&format!("Geometry: {:?}\n", self.geometry));
        report.push_str(&format!(
            "Wavelength Range: {:.0}-{:.0} nm\n",
            self.wavelengths.first().unwrap_or(&0.0),
            self.wavelengths.last().unwrap_or(&0.0)
        ));
        report.push_str(&format!("Number of Points: {}\n", self.wavelengths.len()));
        report.push_str(&format!("Mean Value: {:.4}\n", self.mean_value()));

        if let Some((wl, val)) = self.peak() {
            report.push_str(&format!("Peak: {:.4} at {:.0} nm\n", val, wl));
        }

        report.push_str(&format!("Quality: {:?}\n", self.measurements.quality));
        report
    }
}

/// Gaussian function for CMF approximation.
fn gaussian(x: f64, mean: f64, std: f64) -> f64 {
    (-0.5 * ((x - mean) / std).powi(2)).exp()
}

// ============================================================================
// SPECTRAL MODELS FOR TESTING
// ============================================================================

/// Constant reflectance (neutral gray).
pub fn constant_reflectance(value: f64) -> impl Fn(f64) -> f64 {
    move |_wavelength: f64| value
}

/// Linear ramp reflectance.
pub fn linear_reflectance(start: f64, end: f64, wl_start: f64, wl_end: f64) -> impl Fn(f64) -> f64 {
    move |wavelength: f64| {
        let t = (wavelength - wl_start) / (wl_end - wl_start);
        start + t * (end - start)
    }
}

/// Gaussian absorption band.
pub fn gaussian_absorption(
    baseline: f64,
    peak_depth: f64,
    center_nm: f64,
    width_nm: f64,
) -> impl Fn(f64) -> f64 {
    move |wavelength: f64| {
        let absorption = peak_depth * (-0.5 * ((wavelength - center_nm) / width_nm).powi(2)).exp();
        baseline - absorption
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ideal_spectrophotometer() {
        let mut spectro = VirtualSpectrophotometer::ideal()
            .with_wavelength_range(400.0, 700.0)
            .with_wavelength_step(10.0);

        let result = spectro.measure_reflectance(constant_reflectance(0.5));

        for &value in &result.measurements.values {
            assert!((value - 0.5).abs() < 0.001);
        }
    }

    #[test]
    fn test_wavelength_range() {
        let mut spectro = VirtualSpectrophotometer::ideal()
            .with_wavelength_range(400.0, 500.0)
            .with_wavelength_step(25.0);

        let result = spectro.measure_reflectance(constant_reflectance(0.5));

        assert_eq!(result.wavelengths.len(), 5); // 400, 425, 450, 475, 500
        assert!((result.wavelengths[0] - 400.0).abs() < 1e-10);
        assert!((result.wavelengths[4] - 500.0).abs() < 1e-10);
    }

    #[test]
    fn test_transmittance() {
        let mut spectro = VirtualSpectrophotometer::ideal().with_wavelength_step(10.0);

        let result = spectro.measure_transmittance(constant_reflectance(0.9)); // 90% transmittance

        assert!(matches!(
            result.measurement_type,
            SpectroMeasurementType::Transmittance
        ));
        assert!((result.mean_value() - 0.9).abs() < 0.01);
    }

    #[test]
    fn test_absorbance() {
        let mut spectro = VirtualSpectrophotometer::ideal().with_wavelength_step(10.0);

        // 10% transmittance -> A = -log10(0.1) = 1.0
        let result = spectro.measure_absorbance(constant_reflectance(0.1));

        assert!(matches!(
            result.measurement_type,
            SpectroMeasurementType::Absorbance
        ));
        assert!((result.mean_value() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_bandwidth_averaging() {
        let mut spectro = VirtualSpectrophotometer::ideal()
            .with_wavelength_step(10.0)
            .with_seed(42);

        // Sharp step function
        let step_fn = |wl: f64| if wl < 500.0 { 0.2 } else { 0.8 };

        let result = spectro.measure_reflectance(step_fn);

        // At 500nm, should be smoothed
        if let Some(value) = result.at_wavelength(500.0) {
            assert!(value > 0.2 && value < 0.8);
        }
    }

    #[test]
    fn test_noisy_measurement() {
        let mut spectro = VirtualSpectrophotometer::color_measurement().with_seed(123);

        let result = spectro.measure_reflectance(constant_reflectance(0.5));

        // With noise, values should vary
        let variance: f64 = result
            .measurements
            .values
            .iter()
            .map(|&v| (v - 0.5).powi(2))
            .sum::<f64>()
            / result.measurements.values.len() as f64;

        // Should have some variance but not too much
        assert!(variance < 0.01);
    }

    #[test]
    fn test_interpolation() {
        let mut spectro = VirtualSpectrophotometer::ideal()
            .with_wavelength_range(400.0, 700.0)
            .with_wavelength_step(50.0);

        let result = spectro.measure_reflectance(linear_reflectance(0.2, 0.8, 400.0, 700.0));

        // Interpolate at 550nm (midpoint)
        let mid_value = result.at_wavelength(550.0).unwrap();
        assert!((mid_value - 0.5).abs() < 0.01);

        // Out of range
        assert!(result.at_wavelength(300.0).is_none());
    }

    #[test]
    fn test_gaussian_absorption() {
        let mut spectro = VirtualSpectrophotometer::ideal()
            .with_wavelength_range(400.0, 700.0)
            .with_wavelength_step(5.0);

        let absorption = gaussian_absorption(0.9, 0.7, 550.0, 30.0);
        let result = spectro.measure_reflectance(absorption);

        // Should have minimum near 550nm
        let (peak_wl, _) = result
            .wavelengths
            .iter()
            .enumerate()
            .min_by(|a, b| {
                result.measurements.values[a.0]
                    .partial_cmp(&result.measurements.values[b.0])
                    .unwrap()
            })
            .map(|(i, &wl)| (wl, result.measurements.values[i]))
            .unwrap();

        assert!((peak_wl - 550.0).abs() < 10.0);
    }

    #[test]
    fn test_tristimulus() {
        let mut spectro = VirtualSpectrophotometer::ideal()
            .with_wavelength_range(380.0, 780.0)
            .with_wavelength_step(5.0);

        let result = spectro.measure_reflectance(constant_reflectance(1.0)); // Perfect white

        let x = result.tristimulus_x();
        let y = result.tristimulus_y();
        let z = result.tristimulus_z();

        // For perfect white, tristimulus should be positive
        assert!(x > 0.0);
        assert!(y > 0.0);
        assert!(z > 0.0);
    }

    #[test]
    fn test_result_report() {
        let mut spectro = VirtualSpectrophotometer::ideal().with_wavelength_step(20.0);

        let result = spectro.measure_reflectance(constant_reflectance(0.5));
        let report = result.report();

        assert!(report.contains("Reflectance"));
        assert!(report.contains("Mean Value"));
    }

    #[test]
    fn test_measurement_geometries() {
        let mut spectro =
            VirtualSpectrophotometer::ideal().with_geometry(SpectroGeometry::D8Specular);

        assert!(matches!(spectro.geometry, SpectroGeometry::D8Specular));

        spectro = spectro.with_geometry(SpectroGeometry::VariableAngle { incident_deg: 45.0 });
        assert!(matches!(
            spectro.geometry,
            SpectroGeometry::VariableAngle { .. }
        ));
    }

    #[test]
    fn test_research_grade() {
        let spectro = VirtualSpectrophotometer::research_uv_vis();

        assert_eq!(spectro.wavelength_range.0, 200.0);
        assert_eq!(spectro.wavelength_range.1, 1100.0);
        assert!(spectro.wavelength_step <= 1.0);
    }
}

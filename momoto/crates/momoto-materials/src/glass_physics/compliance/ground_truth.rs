//! # Ground Truth Validation
//!
//! Comparison against measured reference data (MERL, published papers, etc.)
//! for validating material twin accuracy.

use std::collections::HashMap;

// ============================================================================
// GROUND TRUTH DATASET
// ============================================================================

/// Types of ground truth datasets.
#[derive(Debug, Clone)]
pub enum GroundTruthDataset {
    /// MERL BRDF database.
    MERL {
        /// Material names in dataset.
        materials: Vec<String>,
    },
    /// Published measurement data.
    Published {
        /// Citation reference.
        reference: String,
        /// Spectral measurements.
        data: Vec<SpectralMeasurement>,
    },
    /// Custom user-provided data.
    Custom {
        /// Dataset name.
        name: String,
        /// Measurement points.
        measurements: Vec<ReferenceMeasurement>,
    },
}

impl GroundTruthDataset {
    /// Create MERL dataset reference.
    pub fn merl(materials: Vec<&str>) -> Self {
        GroundTruthDataset::MERL {
            materials: materials.into_iter().map(String::from).collect(),
        }
    }

    /// Create published dataset reference.
    pub fn published(reference: impl Into<String>) -> Self {
        GroundTruthDataset::Published {
            reference: reference.into(),
            data: Vec::new(),
        }
    }

    /// Create custom dataset.
    pub fn custom(name: impl Into<String>) -> Self {
        GroundTruthDataset::Custom {
            name: name.into(),
            measurements: Vec::new(),
        }
    }

    /// Get dataset name.
    pub fn name(&self) -> String {
        match self {
            GroundTruthDataset::MERL { .. } => "MERL BRDF Database".to_string(),
            GroundTruthDataset::Published { reference, .. } => reference.clone(),
            GroundTruthDataset::Custom { name, .. } => name.clone(),
        }
    }

    /// Get number of materials/samples.
    pub fn sample_count(&self) -> usize {
        match self {
            GroundTruthDataset::MERL { materials } => materials.len(),
            GroundTruthDataset::Published { data, .. } => data.len(),
            GroundTruthDataset::Custom { measurements, .. } => measurements.len(),
        }
    }
}

// ============================================================================
// MEASUREMENT TYPES
// ============================================================================

/// Spectral measurement point.
#[derive(Debug, Clone)]
pub struct SpectralMeasurement {
    /// Wavelength in nm.
    pub wavelength_nm: f64,
    /// Measured value (reflectance, transmittance, etc.).
    pub value: f64,
    /// Measurement uncertainty.
    pub uncertainty: f64,
    /// Angle of measurement (if applicable).
    pub angle_deg: Option<f64>,
}

/// Generic reference measurement.
#[derive(Debug, Clone)]
pub struct ReferenceMeasurement {
    /// Measurement identifier.
    pub id: String,
    /// Input parameters (wavelength, angle, etc.).
    pub inputs: HashMap<String, f64>,
    /// Measured output value.
    pub output: f64,
    /// Measurement uncertainty.
    pub uncertainty: f64,
}

impl SpectralMeasurement {
    /// Create new spectral measurement.
    pub fn new(wavelength_nm: f64, value: f64) -> Self {
        Self {
            wavelength_nm,
            value,
            uncertainty: 0.0,
            angle_deg: None,
        }
    }

    /// Set uncertainty.
    pub fn with_uncertainty(mut self, uncertainty: f64) -> Self {
        self.uncertainty = uncertainty;
        self
    }

    /// Set angle.
    pub fn with_angle(mut self, angle_deg: f64) -> Self {
        self.angle_deg = Some(angle_deg);
        self
    }
}

// ============================================================================
// GROUND TRUTH VALIDATOR
// ============================================================================

/// Validator for comparing predictions against ground truth.
#[derive(Debug, Clone)]
pub struct GroundTruthValidator {
    /// Reference datasets.
    pub datasets: Vec<GroundTruthDataset>,
    /// Tolerance for comparison.
    pub tolerance: f64,
    /// Use perceptual (ΔE) metric.
    pub use_perceptual: bool,
}

impl GroundTruthValidator {
    /// Create new validator.
    pub fn new() -> Self {
        Self {
            datasets: Vec::new(),
            tolerance: 0.01,
            use_perceptual: true,
        }
    }

    /// Add dataset for validation.
    pub fn add_dataset(&mut self, dataset: GroundTruthDataset) {
        self.datasets.push(dataset);
    }

    /// Set tolerance.
    pub fn with_tolerance(mut self, tolerance: f64) -> Self {
        self.tolerance = tolerance;
        self
    }

    /// Enable/disable perceptual metric.
    pub fn with_perceptual(mut self, use_perceptual: bool) -> Self {
        self.use_perceptual = use_perceptual;
        self
    }

    /// Validate predictions against ground truth.
    pub fn validate<F>(&self, prediction_fn: F) -> ValidationReport
    where
        F: Fn(f64, Option<f64>) -> f64, // (wavelength, angle) -> value
    {
        let mut all_errors: Vec<f64> = Vec::new();
        let mut dataset_reports: Vec<DatasetValidationReport> = Vec::new();

        for dataset in &self.datasets {
            let report = self.validate_dataset(dataset, &prediction_fn);
            all_errors.extend(&report.errors);
            dataset_reports.push(report);
        }

        // Compute overall statistics
        let n = all_errors.len();
        let mean_error = if n > 0 {
            all_errors.iter().sum::<f64>() / n as f64
        } else {
            0.0
        };

        let max_error = all_errors.iter().cloned().fold(0.0, f64::max);

        let rmse = if n > 0 {
            (all_errors.iter().map(|e| e.powi(2)).sum::<f64>() / n as f64).sqrt()
        } else {
            0.0
        };

        // Simple ΔE approximation (for reflectance data)
        let delta_e_approx = if self.use_perceptual {
            rmse * 100.0 // Rough conversion
        } else {
            rmse
        };

        let passed = delta_e_approx <= self.tolerance * 100.0;

        ValidationReport {
            datasets: dataset_reports,
            delta_e_mean: mean_error * 100.0,
            delta_e_max: max_error * 100.0,
            rmse_spectral: rmse,
            angular_error_mean: 0.0, // Would compute from angular data
            total_samples: n,
            passed,
        }
    }

    /// Validate single dataset.
    fn validate_dataset<F>(
        &self,
        dataset: &GroundTruthDataset,
        prediction_fn: &F,
    ) -> DatasetValidationReport
    where
        F: Fn(f64, Option<f64>) -> f64,
    {
        let mut errors = Vec::new();

        match dataset {
            GroundTruthDataset::Published { data, .. } => {
                for measurement in data {
                    let predicted = prediction_fn(measurement.wavelength_nm, measurement.angle_deg);
                    let error = (predicted - measurement.value).abs();
                    errors.push(error);
                }
            }
            GroundTruthDataset::Custom { measurements, .. } => {
                for measurement in measurements {
                    let wavelength = measurement
                        .inputs
                        .get("wavelength")
                        .copied()
                        .unwrap_or(550.0);
                    let angle = measurement.inputs.get("angle").copied();
                    let predicted = prediction_fn(wavelength, angle);
                    let error = (predicted - measurement.output).abs();
                    errors.push(error);
                }
            }
            GroundTruthDataset::MERL { .. } => {
                // MERL validation would require actual BRDF data
                // For now, return empty (data not loaded)
            }
        }

        let n = errors.len();
        let mean_error = if n > 0 {
            errors.iter().sum::<f64>() / n as f64
        } else {
            0.0
        };

        let max_error = errors.iter().cloned().fold(0.0, f64::max);
        let rmse = if n > 0 {
            (errors.iter().map(|e| e.powi(2)).sum::<f64>() / n as f64).sqrt()
        } else {
            0.0
        };

        DatasetValidationReport {
            dataset_name: dataset.name(),
            sample_count: n,
            mean_error,
            max_error,
            rmse,
            errors,
            passed: rmse <= self.tolerance,
        }
    }
}

impl Default for GroundTruthValidator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// VALIDATION REPORTS
// ============================================================================

/// Report for single dataset validation.
#[derive(Debug, Clone)]
pub struct DatasetValidationReport {
    /// Dataset name.
    pub dataset_name: String,
    /// Number of samples compared.
    pub sample_count: usize,
    /// Mean absolute error.
    pub mean_error: f64,
    /// Maximum error.
    pub max_error: f64,
    /// Root mean square error.
    pub rmse: f64,
    /// Individual errors.
    pub errors: Vec<f64>,
    /// Whether validation passed.
    pub passed: bool,
}

impl DatasetValidationReport {
    /// Generate report string.
    pub fn report(&self) -> String {
        format!(
            "{}: {} samples | Mean: {:.6} | Max: {:.6} | RMSE: {:.6} | {}",
            self.dataset_name,
            self.sample_count,
            self.mean_error,
            self.max_error,
            self.rmse,
            if self.passed { "PASS" } else { "FAIL" }
        )
    }
}

/// Complete validation report.
#[derive(Debug, Clone)]
pub struct ValidationReport {
    /// Per-dataset reports.
    pub datasets: Vec<DatasetValidationReport>,
    /// Mean ΔE2000 (or approximation).
    pub delta_e_mean: f64,
    /// Maximum ΔE2000.
    pub delta_e_max: f64,
    /// Spectral RMSE.
    pub rmse_spectral: f64,
    /// Angular error mean.
    pub angular_error_mean: f64,
    /// Total samples validated.
    pub total_samples: usize,
    /// Overall pass/fail.
    pub passed: bool,
}

impl ValidationReport {
    /// Generate full report.
    pub fn report(&self) -> String {
        let mut report = String::new();

        report.push_str("Ground Truth Validation Report\n");
        report.push_str(&format!(
            "Overall: {} ({} samples)\n",
            if self.passed { "PASSED" } else { "FAILED" },
            self.total_samples
        ));
        report.push_str(&format!("ΔE Mean: {:.3}\n", self.delta_e_mean));
        report.push_str(&format!("ΔE Max:  {:.3}\n", self.delta_e_max));
        report.push_str(&format!("RMSE:    {:.6}\n\n", self.rmse_spectral));

        report.push_str("Datasets:\n");
        for ds_report in &self.datasets {
            report.push_str(&format!("  {}\n", ds_report.report()));
        }

        report
    }

    /// Check if achieves target ΔE.
    pub fn achieves_delta_e(&self, target: f64) -> bool {
        self.delta_e_mean <= target
    }
}

// ============================================================================
// BUILT-IN REFERENCE DATA
// ============================================================================

/// Create gold reference data (approximate).
pub fn gold_reference_data() -> GroundTruthDataset {
    let mut data = Vec::new();

    // Approximate gold reflectance (Johnson & Christy)
    let wavelengths = [400.0, 450.0, 500.0, 550.0, 600.0, 650.0, 700.0, 750.0];
    let reflectances = [0.39, 0.36, 0.35, 0.52, 0.88, 0.95, 0.97, 0.98];

    for (&wl, &r) in wavelengths.iter().zip(reflectances.iter()) {
        data.push(SpectralMeasurement::new(wl, r).with_uncertainty(0.02));
    }

    GroundTruthDataset::Published {
        reference: "Johnson & Christy 1972".to_string(),
        data,
    }
}

/// Create silver reference data (approximate).
pub fn silver_reference_data() -> GroundTruthDataset {
    let mut data = Vec::new();

    // Approximate silver reflectance
    let wavelengths = [400.0, 450.0, 500.0, 550.0, 600.0, 650.0, 700.0, 750.0];
    let reflectances = [0.91, 0.95, 0.97, 0.98, 0.98, 0.98, 0.99, 0.99];

    for (&wl, &r) in wavelengths.iter().zip(reflectances.iter()) {
        data.push(SpectralMeasurement::new(wl, r).with_uncertainty(0.01));
    }

    GroundTruthDataset::Published {
        reference: "Palik Handbook 1998".to_string(),
        data,
    }
}

/// Create glass reference data (BK7).
pub fn bk7_reference_data() -> GroundTruthDataset {
    let mut data = Vec::new();

    // BK7 normal incidence reflectance ≈ 4% across visible
    let wavelengths = [400.0, 450.0, 500.0, 550.0, 600.0, 650.0, 700.0];

    for &wl in &wavelengths {
        // Fresnel reflectance for n ≈ 1.52
        let r = ((1.52f64 - 1.0) / (1.52 + 1.0)).powi(2);
        data.push(SpectralMeasurement::new(wl, r).with_uncertainty(0.002));
    }

    GroundTruthDataset::Published {
        reference: "Schott Glass Catalog".to_string(),
        data,
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dataset_creation() {
        let merl = GroundTruthDataset::merl(vec!["gold", "silver", "chrome"]);
        assert_eq!(merl.sample_count(), 3);

        let published = GroundTruthDataset::published("Test Reference 2024");
        assert!(published.name().contains("Test Reference"));

        let custom = GroundTruthDataset::custom("My Dataset");
        assert!(custom.name().contains("My Dataset"));
    }

    #[test]
    fn test_spectral_measurement() {
        let m = SpectralMeasurement::new(550.0, 0.5)
            .with_uncertainty(0.01)
            .with_angle(45.0);

        assert!((m.wavelength_nm - 550.0).abs() < 1e-10);
        assert!((m.value - 0.5).abs() < 1e-10);
        assert!((m.uncertainty - 0.01).abs() < 1e-10);
        assert_eq!(m.angle_deg, Some(45.0));
    }

    #[test]
    fn test_validator_creation() {
        let validator = GroundTruthValidator::new()
            .with_tolerance(0.02)
            .with_perceptual(true);

        assert!((validator.tolerance - 0.02).abs() < 1e-10);
        assert!(validator.use_perceptual);
    }

    #[test]
    fn test_perfect_validation() {
        let mut validator = GroundTruthValidator::new().with_tolerance(0.01);

        // Create dataset with known values
        let mut measurements = Vec::new();
        for wl in [400.0, 500.0, 600.0, 700.0] {
            measurements.push(SpectralMeasurement::new(wl, 0.5));
        }

        validator.add_dataset(GroundTruthDataset::Published {
            reference: "Test".to_string(),
            data: measurements,
        });

        // Perfect prediction
        let report = validator.validate(|_wl, _angle| 0.5);

        assert!(report.rmse_spectral < 1e-10);
        assert!(report.passed);
    }

    #[test]
    fn test_validation_with_error() {
        let mut validator = GroundTruthValidator::new().with_tolerance(0.01);

        let mut measurements = Vec::new();
        for wl in [400.0, 500.0, 600.0, 700.0] {
            measurements.push(SpectralMeasurement::new(wl, 0.5));
        }

        validator.add_dataset(GroundTruthDataset::Published {
            reference: "Test".to_string(),
            data: measurements,
        });

        // Prediction with 0.1 offset
        let report = validator.validate(|_wl, _angle| 0.6);

        assert!((report.rmse_spectral - 0.1).abs() < 1e-10);
    }

    #[test]
    fn test_gold_reference() {
        let gold = gold_reference_data();

        match gold {
            GroundTruthDataset::Published { data, reference } => {
                assert!(reference.contains("Johnson"));
                assert!(!data.is_empty());
                // Gold should have low reflectance in blue, high in red
                let blue = data.iter().find(|m| m.wavelength_nm < 500.0).unwrap();
                let red = data.iter().find(|m| m.wavelength_nm > 600.0).unwrap();
                assert!(blue.value < red.value);
            }
            _ => panic!("Expected Published dataset"),
        }
    }

    #[test]
    fn test_bk7_reference() {
        let bk7 = bk7_reference_data();

        match bk7 {
            GroundTruthDataset::Published { data, .. } => {
                // BK7 reflectance should be ~4%
                for measurement in &data {
                    assert!(measurement.value > 0.03 && measurement.value < 0.05);
                }
            }
            _ => panic!("Expected Published dataset"),
        }
    }

    #[test]
    fn test_validation_report() {
        let report = ValidationReport {
            datasets: vec![],
            delta_e_mean: 0.5,
            delta_e_max: 1.2,
            rmse_spectral: 0.008,
            angular_error_mean: 0.0,
            total_samples: 100,
            passed: true,
        };

        let report_str = report.report();
        assert!(report_str.contains("PASSED"));
        assert!(report_str.contains("100 samples"));
    }

    #[test]
    fn test_achieves_delta_e() {
        let report = ValidationReport {
            datasets: vec![],
            delta_e_mean: 0.4,
            delta_e_max: 0.8,
            rmse_spectral: 0.004,
            angular_error_mean: 0.0,
            total_samples: 50,
            passed: true,
        };

        assert!(report.achieves_delta_e(0.5));
        assert!(report.achieves_delta_e(1.0));
        assert!(!report.achieves_delta_e(0.3));
    }
}

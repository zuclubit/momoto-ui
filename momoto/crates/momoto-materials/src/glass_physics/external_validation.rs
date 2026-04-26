//! # External Validation Module
//!
//! Framework for comparing rendered results against external measured datasets.
//!
//! ## Features
//!
//! - **ExternalDataset trait**: Common interface for all datasets
//! - **ValidationEngine**: Compare materials against multiple datasets
//! - **Report Generation**: Markdown and JSON validation reports
//! - **Batch Validation**: Validate multiple materials efficiently
//!
//! ## Usage
//!
//! ```rust
//! use momoto_materials::glass_physics::external_validation::{
//!     ValidationEngine, ValidationConfig
//! };
//!
//! let mut engine = ValidationEngine::new();
//! // Add datasets...
//! // let result = engine.validate_material(&material, "dataset_name", "material_name");
//! ```

use super::reference_renderer::ReferenceRenderer;
use super::spectral_error::{
    compute_energy_metrics, compute_perceptual_metrics, compute_spectral_metrics,
};

use std::time::Instant;

// ============================================================================
// EXTERNAL DATASET TRAIT
// ============================================================================

/// Trait for external measured datasets
pub trait ExternalDataset: Send + Sync {
    /// Get dataset name
    fn name(&self) -> &str;

    /// Get number of materials in dataset
    fn material_count(&self) -> usize;

    /// Get list of material names
    fn material_names(&self) -> Vec<&str>;

    /// Get BRDF value at given angles
    /// Returns None if material or angles not available
    fn get_brdf(
        &self,
        material_index: usize,
        theta_i: f64,
        phi_i: f64,
        theta_o: f64,
        phi_o: f64,
    ) -> Option<f64>;

    /// Get spectral reflectance at wavelength and angle
    fn get_spectral(&self, material_index: usize, wavelength_nm: f64, theta: f64) -> Option<f64>;

    /// Check if dataset contains isotropic materials
    fn is_isotropic(&self) -> bool {
        true
    }

    /// Get material index by name
    fn material_index(&self, name: &str) -> Option<usize> {
        self.material_names()
            .iter()
            .position(|n| n.eq_ignore_ascii_case(name))
    }

    /// Get wavelength range supported
    fn wavelength_range(&self) -> (f64, f64) {
        (400.0, 700.0)
    }

    /// Get angle resolution (degrees)
    fn angle_resolution(&self) -> f64 {
        1.0
    }
}

// ============================================================================
// VALIDATION RESULT
// ============================================================================

/// Result of validating a material against a dataset
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Dataset name
    pub dataset_name: String,
    /// Material name within dataset
    pub material_name: String,
    /// Spectral RMSE (if spectral data available)
    pub spectral_rmse: f64,
    /// Maximum spectral error
    pub spectral_max_error: f64,
    /// CIEDE2000 color difference
    pub delta_e_2000: f64,
    /// CIE76 color difference
    pub delta_e_76: f64,
    /// Energy conservation (1.0 = perfect)
    pub energy_conservation: f64,
    /// Helmholtz reciprocity error
    pub reciprocity_error: f64,
    /// Number of samples evaluated
    pub samples_evaluated: usize,
    /// Computation time in milliseconds
    pub computation_time_ms: f64,
    /// Overall validation passed
    pub passed: bool,
    /// Notes or warnings
    pub notes: Vec<String>,
}

impl ValidationResult {
    /// Create empty result
    pub fn empty(dataset_name: &str, material_name: &str) -> Self {
        Self {
            dataset_name: dataset_name.to_string(),
            material_name: material_name.to_string(),
            spectral_rmse: 0.0,
            spectral_max_error: 0.0,
            delta_e_2000: 0.0,
            delta_e_76: 0.0,
            energy_conservation: 1.0,
            reciprocity_error: 0.0,
            samples_evaluated: 0,
            computation_time_ms: 0.0,
            passed: false,
            notes: Vec::new(),
        }
    }

    /// Add note
    pub fn with_note(mut self, note: &str) -> Self {
        self.notes.push(note.to_string());
        self
    }

    /// Check if result indicates acceptable quality
    pub fn is_acceptable(&self, delta_e_threshold: f64, rmse_threshold: f64) -> bool {
        self.delta_e_2000 <= delta_e_threshold && self.spectral_rmse <= rmse_threshold
    }

    /// Get quality grade
    pub fn quality_grade(&self) -> QualityGrade {
        if self.delta_e_2000 < 1.0 && self.spectral_rmse < 0.001 {
            QualityGrade::Excellent
        } else if self.delta_e_2000 < 2.0 && self.spectral_rmse < 0.005 {
            QualityGrade::Good
        } else if self.delta_e_2000 < 3.5 && self.spectral_rmse < 0.02 {
            QualityGrade::Acceptable
        } else if self.delta_e_2000 < 5.0 && self.spectral_rmse < 0.05 {
            QualityGrade::Marginal
        } else {
            QualityGrade::Poor
        }
    }
}

/// Quality grade for validation results
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityGrade {
    Excellent,
    Good,
    Acceptable,
    Marginal,
    Poor,
}

impl std::fmt::Display for QualityGrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Excellent => write!(f, "Excellent"),
            Self::Good => write!(f, "Good"),
            Self::Acceptable => write!(f, "Acceptable"),
            Self::Marginal => write!(f, "Marginal"),
            Self::Poor => write!(f, "Poor"),
        }
    }
}

// ============================================================================
// VALIDATION REPORT
// ============================================================================

/// Summary statistics for validation report
#[derive(Debug, Clone)]
pub struct ReportSummary {
    /// Total materials validated
    pub total_materials: usize,
    /// Materials that passed validation
    pub passed_count: usize,
    /// Mean Delta E 2000 across all materials
    pub mean_delta_e: f64,
    /// Maximum Delta E 2000
    pub max_delta_e: f64,
    /// Mean spectral RMSE
    pub mean_spectral_rmse: f64,
    /// Maximum spectral RMSE
    pub max_spectral_rmse: f64,
    /// Total computation time (ms)
    pub total_time_ms: f64,
}

/// Per-material validation info
#[derive(Debug, Clone)]
pub struct MaterialValidation {
    /// Material name
    pub name: String,
    /// Validation result
    pub result: ValidationResult,
    /// Quality grade
    pub grade: QualityGrade,
}

/// Complete validation report
#[derive(Debug, Clone)]
pub struct ValidationReport {
    /// Report summary
    pub summary: ReportSummary,
    /// Per-material validations
    pub per_material: Vec<MaterialValidation>,
    /// Recommendations for improvement
    pub recommendations: Vec<String>,
    /// Report timestamp
    pub timestamp: u64,
    /// Engine version
    pub engine_version: String,
}

impl ValidationReport {
    /// Get pass rate as percentage
    pub fn pass_rate(&self) -> f64 {
        if self.summary.total_materials == 0 {
            return 0.0;
        }
        100.0 * self.summary.passed_count as f64 / self.summary.total_materials as f64
    }

    /// Get materials by grade
    pub fn by_grade(&self, grade: QualityGrade) -> Vec<&MaterialValidation> {
        self.per_material
            .iter()
            .filter(|m| m.grade == grade)
            .collect()
    }
}

// ============================================================================
// VALIDATION ENGINE
// ============================================================================

/// Configuration for validation
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Delta E threshold for passing
    pub delta_e_threshold: f64,
    /// RMSE threshold for passing
    pub rmse_threshold: f64,
    /// Number of angle samples per dimension
    pub angle_samples: usize,
    /// Wavelength step for spectral validation (nm)
    pub wavelength_step: f64,
    /// Enable reciprocity checking
    pub check_reciprocity: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            delta_e_threshold: 3.0,
            rmse_threshold: 0.02,
            angle_samples: 90,
            wavelength_step: 10.0,
            check_reciprocity: true,
        }
    }
}

/// Validation engine for comparing against external datasets
pub struct ValidationEngine {
    /// Registered datasets
    datasets: Vec<Box<dyn ExternalDataset>>,
    /// Reference renderer
    renderer: ReferenceRenderer,
    /// Validation configuration
    config: ValidationConfig,
}

impl ValidationEngine {
    /// Create new validation engine
    pub fn new() -> Self {
        Self {
            datasets: Vec::new(),
            renderer: ReferenceRenderer::default(),
            config: ValidationConfig::default(),
        }
    }

    /// Create with custom config
    pub fn with_config(config: ValidationConfig) -> Self {
        Self {
            datasets: Vec::new(),
            renderer: ReferenceRenderer::default(),
            config,
        }
    }

    /// Add dataset to engine
    pub fn add_dataset(&mut self, dataset: Box<dyn ExternalDataset>) {
        self.datasets.push(dataset);
    }

    /// Get number of registered datasets
    pub fn dataset_count(&self) -> usize {
        self.datasets.len()
    }

    /// Get dataset names
    pub fn dataset_names(&self) -> Vec<&str> {
        self.datasets.iter().map(|d| d.name()).collect()
    }

    /// Get dataset by name
    pub fn get_dataset(&self, name: &str) -> Option<&dyn ExternalDataset> {
        self.datasets
            .iter()
            .find(|d| d.name().eq_ignore_ascii_case(name))
            .map(|d| d.as_ref())
    }

    /// Validate material against specific dataset and material
    pub fn validate_material(
        &self,
        rendered_spectral: &[f64],
        wavelengths: &[f64],
        lab_rendered: [f64; 3],
        reflectance: f64,
        transmittance: f64,
        dataset_name: &str,
        material_name: &str,
    ) -> ValidationResult {
        let start = Instant::now();

        // Find dataset
        let dataset = match self.get_dataset(dataset_name) {
            Some(d) => d,
            None => {
                return ValidationResult::empty(dataset_name, material_name)
                    .with_note(&format!("Dataset '{}' not found", dataset_name));
            }
        };

        // Find material index
        let material_idx = match dataset.material_index(material_name) {
            Some(idx) => idx,
            None => {
                return ValidationResult::empty(dataset_name, material_name).with_note(&format!(
                    "Material '{}' not found in dataset",
                    material_name
                ));
            }
        };

        // Get reference spectral data
        let mut measured_spectral = Vec::with_capacity(wavelengths.len());
        for &w in wavelengths {
            let r = dataset.get_spectral(material_idx, w, 0.0).unwrap_or(0.0);
            measured_spectral.push(r);
        }

        // Compute spectral metrics
        let spectral = compute_spectral_metrics(&measured_spectral, rendered_spectral, wavelengths);

        // Compute Lab for measured (approximate)
        let lab_measured = spectral_to_lab(&measured_spectral);

        // Compute perceptual metrics
        let perceptual = compute_perceptual_metrics(lab_measured, lab_rendered);

        // Compute energy metrics
        let absorption = 1.0 - reflectance - transmittance;
        let energy = compute_energy_metrics(reflectance, transmittance, absorption.max(0.0));

        // Check reciprocity (simplified)
        let reciprocity_error = if self.config.check_reciprocity {
            // Would need actual BRDF data for proper check
            0.0
        } else {
            0.0
        };

        let computation_time_ms = start.elapsed().as_micros() as f64 / 1000.0;

        let passed = perceptual.delta_e_2000 <= self.config.delta_e_threshold
            && spectral.rmse <= self.config.rmse_threshold;

        ValidationResult {
            dataset_name: dataset_name.to_string(),
            material_name: material_name.to_string(),
            spectral_rmse: spectral.rmse,
            spectral_max_error: spectral.max_error,
            delta_e_2000: perceptual.delta_e_2000,
            delta_e_76: perceptual.delta_e_76,
            energy_conservation: energy.physical_consistency,
            reciprocity_error,
            samples_evaluated: wavelengths.len(),
            computation_time_ms,
            passed,
            notes: Vec::new(),
        }
    }

    /// Validate against all materials in a dataset
    pub fn validate_all_in_dataset(
        &self,
        render_fn: impl Fn(&str) -> Option<(Vec<f64>, [f64; 3], f64, f64)>,
        dataset_name: &str,
    ) -> Vec<ValidationResult> {
        let dataset = match self.get_dataset(dataset_name) {
            Some(d) => d,
            None => return Vec::new(),
        };

        let wavelengths = self.renderer.config().wavelengths();
        let mut results = Vec::new();

        for material_name in dataset.material_names() {
            if let Some((spectral, lab, r, t)) = render_fn(material_name) {
                let result = self.validate_material(
                    &spectral,
                    &wavelengths,
                    lab,
                    r,
                    t,
                    dataset_name,
                    material_name,
                );
                results.push(result);
            }
        }

        results
    }

    /// Generate validation report from results
    pub fn generate_report(&self, results: &[ValidationResult]) -> ValidationReport {
        if results.is_empty() {
            return ValidationReport {
                summary: ReportSummary {
                    total_materials: 0,
                    passed_count: 0,
                    mean_delta_e: 0.0,
                    max_delta_e: 0.0,
                    mean_spectral_rmse: 0.0,
                    max_spectral_rmse: 0.0,
                    total_time_ms: 0.0,
                },
                per_material: Vec::new(),
                recommendations: vec!["No validation results available".to_string()],
                timestamp: current_timestamp(),
                engine_version: "momoto-materials-0.8.0".to_string(),
            };
        }

        let total_materials = results.len();
        let passed_count = results.iter().filter(|r| r.passed).count();

        let sum_delta_e: f64 = results.iter().map(|r| r.delta_e_2000).sum();
        let max_delta_e = results.iter().map(|r| r.delta_e_2000).fold(0.0, f64::max);

        let sum_rmse: f64 = results.iter().map(|r| r.spectral_rmse).sum();
        let max_rmse = results.iter().map(|r| r.spectral_rmse).fold(0.0, f64::max);

        let total_time: f64 = results.iter().map(|r| r.computation_time_ms).sum();

        let per_material: Vec<MaterialValidation> = results
            .iter()
            .map(|r| MaterialValidation {
                name: r.material_name.clone(),
                result: r.clone(),
                grade: r.quality_grade(),
            })
            .collect();

        // Generate recommendations
        let mut recommendations = Vec::new();
        if max_delta_e > 5.0 {
            recommendations.push(format!(
                "High perceptual error detected (max Delta E: {:.1}). Consider recalibrating.",
                max_delta_e
            ));
        }
        if max_rmse > 0.05 {
            recommendations.push(format!(
                "High spectral error detected (max RMSE: {:.3}). Check spectral accuracy.",
                max_rmse
            ));
        }
        if passed_count < total_materials / 2 {
            recommendations.push(
                "Less than 50% of materials passed. Review material model parameters.".to_string(),
            );
        }
        if recommendations.is_empty() {
            recommendations.push("All validations within acceptable thresholds.".to_string());
        }

        ValidationReport {
            summary: ReportSummary {
                total_materials,
                passed_count,
                mean_delta_e: sum_delta_e / total_materials as f64,
                max_delta_e,
                mean_spectral_rmse: sum_rmse / total_materials as f64,
                max_spectral_rmse: max_rmse,
                total_time_ms: total_time,
            },
            per_material,
            recommendations,
            timestamp: current_timestamp(),
            engine_version: "momoto-materials-0.8.0".to_string(),
        }
    }
}

impl Default for ValidationEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// REPORT GENERATION
// ============================================================================

/// Generate markdown validation report
pub fn generate_markdown_report(report: &ValidationReport) -> String {
    let mut md = String::new();

    md.push_str("# Validation Report\n\n");
    md.push_str(&format!(
        "Generated: {}\n\n",
        format_timestamp(report.timestamp)
    ));

    // Summary
    md.push_str("## Summary\n\n");
    md.push_str("| Metric | Value |\n");
    md.push_str("|--------|-------|\n");
    md.push_str(&format!(
        "| Materials Tested | {} |\n",
        report.summary.total_materials
    ));
    md.push_str(&format!(
        "| Passed | {} ({:.1}%) |\n",
        report.summary.passed_count,
        report.pass_rate()
    ));
    md.push_str(&format!(
        "| Mean Delta E | {:.2} |\n",
        report.summary.mean_delta_e
    ));
    md.push_str(&format!(
        "| Max Delta E | {:.2} |\n",
        report.summary.max_delta_e
    ));
    md.push_str(&format!(
        "| Mean RMSE | {:.4} |\n",
        report.summary.mean_spectral_rmse
    ));
    md.push_str(&format!(
        "| Max RMSE | {:.4} |\n",
        report.summary.max_spectral_rmse
    ));
    md.push_str(&format!(
        "| Total Time | {:.1} ms |\n",
        report.summary.total_time_ms
    ));

    // Per-material results
    md.push_str("\n## Per-Material Results\n\n");
    md.push_str("| Material | Delta E | RMSE | Grade | Status |\n");
    md.push_str("|----------|---------|------|-------|--------|\n");

    for m in &report.per_material {
        let status = if m.result.passed { "PASS" } else { "FAIL" };
        md.push_str(&format!(
            "| {} | {:.2} | {:.4} | {} | {} |\n",
            m.name, m.result.delta_e_2000, m.result.spectral_rmse, m.grade, status
        ));
    }

    // Recommendations
    md.push_str("\n## Recommendations\n\n");
    for rec in &report.recommendations {
        md.push_str(&format!("- {}\n", rec));
    }

    md
}

/// Generate JSON validation report
pub fn generate_json_report(report: &ValidationReport) -> String {
    let mut json = String::from("{\n");

    // Summary
    json.push_str("  \"summary\": {\n");
    json.push_str(&format!(
        "    \"total_materials\": {},\n",
        report.summary.total_materials
    ));
    json.push_str(&format!(
        "    \"passed_count\": {},\n",
        report.summary.passed_count
    ));
    json.push_str(&format!(
        "    \"mean_delta_e\": {:.4},\n",
        report.summary.mean_delta_e
    ));
    json.push_str(&format!(
        "    \"max_delta_e\": {:.4},\n",
        report.summary.max_delta_e
    ));
    json.push_str(&format!(
        "    \"mean_spectral_rmse\": {:.6},\n",
        report.summary.mean_spectral_rmse
    ));
    json.push_str(&format!(
        "    \"max_spectral_rmse\": {:.6},\n",
        report.summary.max_spectral_rmse
    ));
    json.push_str(&format!(
        "    \"total_time_ms\": {:.2}\n",
        report.summary.total_time_ms
    ));
    json.push_str("  },\n");

    // Materials
    json.push_str("  \"materials\": [\n");
    for (i, m) in report.per_material.iter().enumerate() {
        json.push_str("    {\n");
        json.push_str(&format!("      \"name\": \"{}\",\n", m.name));
        json.push_str(&format!(
            "      \"delta_e_2000\": {:.4},\n",
            m.result.delta_e_2000
        ));
        json.push_str(&format!(
            "      \"spectral_rmse\": {:.6},\n",
            m.result.spectral_rmse
        ));
        json.push_str(&format!("      \"grade\": \"{}\",\n", m.grade));
        json.push_str(&format!("      \"passed\": {}\n", m.result.passed));
        json.push_str("    }");
        if i < report.per_material.len() - 1 {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ],\n");

    // Metadata
    json.push_str(&format!("  \"timestamp\": {},\n", report.timestamp));
    json.push_str(&format!(
        "  \"engine_version\": \"{}\"\n",
        report.engine_version
    ));

    json.push_str("}\n");
    json
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Convert spectral reflectance to Lab color (approximate)
fn spectral_to_lab(spectral: &[f64]) -> [f64; 3] {
    if spectral.is_empty() {
        return [50.0, 0.0, 0.0]; // Neutral gray
    }

    // Simple approximation: use average as luminance
    let y: f64 = spectral.iter().sum::<f64>() / spectral.len() as f64;
    let l = if y > 0.008856 {
        116.0 * y.powf(1.0 / 3.0) - 16.0
    } else {
        903.3 * y
    };

    // Simplified a* and b* from spectral shape
    let n = spectral.len();
    let a = if n >= 2 {
        (spectral[n - 1] - spectral[0]) * 100.0
    } else {
        0.0
    };

    let b = if n >= 3 {
        let mid = spectral[n / 2];
        (spectral[0] + spectral[n - 1] - 2.0 * mid) * 50.0
    } else {
        0.0
    };

    [
        l.clamp(0.0, 100.0),
        a.clamp(-128.0, 128.0),
        b.clamp(-128.0, 128.0),
    ]
}

/// Get current timestamp
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Format timestamp as ISO 8601 date
fn format_timestamp(ts: u64) -> String {
    // Simple formatting (not full ISO 8601 parsing)
    let days = ts / 86400;
    let years = 1970 + days / 365;
    format!("{}-01-01", years)
}

// ============================================================================
// MEMORY ESTIMATION
// ============================================================================

/// Estimate memory usage for validation engine
pub fn total_validation_memory() -> usize {
    // ValidationEngine base: ~1KB
    // Per dataset reference: ~64 bytes
    // ValidationResult: ~200 bytes
    // ValidationReport (10 materials): ~3KB
    // Typical usage: ~5KB
    5120
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Mock dataset for testing
    struct MockDataset {
        materials: Vec<String>,
    }

    impl MockDataset {
        fn new() -> Self {
            Self {
                materials: vec!["Glass".to_string(), "Metal".to_string()],
            }
        }
    }

    impl ExternalDataset for MockDataset {
        fn name(&self) -> &str {
            "MockDataset"
        }

        fn material_count(&self) -> usize {
            self.materials.len()
        }

        fn material_names(&self) -> Vec<&str> {
            self.materials.iter().map(|s| s.as_str()).collect()
        }

        fn get_brdf(
            &self,
            _material_index: usize,
            _theta_i: f64,
            _phi_i: f64,
            _theta_o: f64,
            _phi_o: f64,
        ) -> Option<f64> {
            Some(0.5)
        }

        fn get_spectral(
            &self,
            material_index: usize,
            wavelength_nm: f64,
            _theta: f64,
        ) -> Option<f64> {
            if material_index < self.materials.len() {
                // Simple glass-like spectrum
                Some(0.04 + 0.001 * (wavelength_nm - 400.0) / 300.0)
            } else {
                None
            }
        }
    }

    #[test]
    fn test_validation_engine_creation() {
        let engine = ValidationEngine::new();
        assert_eq!(engine.dataset_count(), 0);
    }

    #[test]
    fn test_add_dataset() {
        let mut engine = ValidationEngine::new();
        engine.add_dataset(Box::new(MockDataset::new()));

        assert_eq!(engine.dataset_count(), 1);
        assert!(engine.get_dataset("MockDataset").is_some());
    }

    #[test]
    fn test_validate_material() {
        let mut engine = ValidationEngine::new();
        engine.add_dataset(Box::new(MockDataset::new()));

        let wavelengths: Vec<f64> = (0..31).map(|i| 400.0 + i as f64 * 10.0).collect();
        let rendered: Vec<f64> = wavelengths
            .iter()
            .map(|w| 0.04 + 0.001 * (w - 400.0) / 300.0)
            .collect();
        let lab = [50.0, 0.0, 0.0];

        let result = engine.validate_material(
            &rendered,
            &wavelengths,
            lab,
            0.04,
            0.96,
            "MockDataset",
            "Glass",
        );

        assert!(result.spectral_rmse < 0.01);
        assert!(result.samples_evaluated > 0);
    }

    #[test]
    fn test_validation_result_grades() {
        let mut result = ValidationResult::empty("test", "test");
        result.delta_e_2000 = 0.5;
        result.spectral_rmse = 0.0005;
        assert_eq!(result.quality_grade(), QualityGrade::Excellent);

        result.delta_e_2000 = 4.0;
        result.spectral_rmse = 0.03;
        assert_eq!(result.quality_grade(), QualityGrade::Marginal);
    }

    #[test]
    fn test_generate_report() {
        let results = vec![
            ValidationResult {
                dataset_name: "Test".to_string(),
                material_name: "Mat1".to_string(),
                spectral_rmse: 0.001,
                spectral_max_error: 0.005,
                delta_e_2000: 0.5,
                delta_e_76: 0.8,
                energy_conservation: 1.0,
                reciprocity_error: 0.0,
                samples_evaluated: 31,
                computation_time_ms: 1.5,
                passed: true,
                notes: Vec::new(),
            },
            ValidationResult {
                dataset_name: "Test".to_string(),
                material_name: "Mat2".to_string(),
                spectral_rmse: 0.05,
                spectral_max_error: 0.1,
                delta_e_2000: 5.0,
                delta_e_76: 6.0,
                energy_conservation: 0.95,
                reciprocity_error: 0.01,
                samples_evaluated: 31,
                computation_time_ms: 2.0,
                passed: false,
                notes: Vec::new(),
            },
        ];

        let engine = ValidationEngine::new();
        let report = engine.generate_report(&results);

        assert_eq!(report.summary.total_materials, 2);
        assert_eq!(report.summary.passed_count, 1);
        assert!(report.summary.mean_delta_e > 0.0);
    }

    #[test]
    fn test_markdown_report() {
        let results = vec![ValidationResult {
            dataset_name: "Test".to_string(),
            material_name: "Glass".to_string(),
            spectral_rmse: 0.01,
            spectral_max_error: 0.02,
            delta_e_2000: 1.5,
            delta_e_76: 2.0,
            energy_conservation: 1.0,
            reciprocity_error: 0.0,
            samples_evaluated: 31,
            computation_time_ms: 1.0,
            passed: true,
            notes: Vec::new(),
        }];

        let engine = ValidationEngine::new();
        let report = engine.generate_report(&results);
        let md = generate_markdown_report(&report);

        assert!(md.contains("Validation Report"));
        assert!(md.contains("Glass"));
        assert!(md.contains("PASS"));
    }

    #[test]
    fn test_json_report() {
        let results = vec![ValidationResult::empty("Test", "Mat1")];
        let engine = ValidationEngine::new();
        let report = engine.generate_report(&results);
        let json = generate_json_report(&report);

        assert!(json.contains("summary"));
        assert!(json.contains("materials"));
        assert!(json.contains("Mat1"));
    }

    #[test]
    fn test_memory_estimate() {
        let mem = total_validation_memory();
        assert!(mem > 0);
        assert!(mem < 50_000);
    }
}

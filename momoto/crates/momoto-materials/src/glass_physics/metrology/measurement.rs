//! # Core Measurement Type
//!
//! Measurement<T> with value, uncertainty, unit, and metrological metadata.

use super::units::Unit;
use std::fmt;

// ============================================================================
// UNCERTAINTY TYPES
// ============================================================================

/// Uncertainty classification (GUM-compliant).
#[derive(Debug, Clone)]
pub enum Uncertainty {
    /// Type A: Statistical uncertainty from repeated measurements.
    TypeA {
        /// Standard error of the mean.
        std_error: f64,
        /// Number of samples.
        n_samples: usize,
    },
    /// Type B: Systematic uncertainty from other sources.
    TypeB {
        /// Estimated systematic uncertainty.
        systematic: f64,
        /// Source description.
        source: String,
    },
    /// Combined uncertainty (RSS of Type A and Type B).
    Combined {
        /// Type A component.
        type_a: f64,
        /// Type B component.
        type_b: f64,
    },
    /// Unknown uncertainty (not yet characterized).
    Unknown,
}

impl Uncertainty {
    /// Create Type A uncertainty.
    pub fn type_a(std_error: f64, n_samples: usize) -> Self {
        Self::TypeA {
            std_error,
            n_samples,
        }
    }

    /// Create Type B uncertainty.
    pub fn type_b(systematic: f64, source: &str) -> Self {
        Self::TypeB {
            systematic,
            source: source.to_string(),
        }
    }

    /// Create combined uncertainty.
    pub fn combined(type_a: f64, type_b: f64) -> Self {
        Self::Combined { type_a, type_b }
    }

    /// Get standard uncertainty (1σ).
    pub fn standard(&self) -> f64 {
        match self {
            Self::TypeA { std_error, .. } => *std_error,
            Self::TypeB { systematic, .. } => *systematic,
            Self::Combined { type_a, type_b } => (type_a * type_a + type_b * type_b).sqrt(),
            Self::Unknown => f64::INFINITY,
        }
    }

    /// Get expanded uncertainty with coverage factor k.
    pub fn expanded(&self, k: f64) -> f64 {
        self.standard() * k
    }

    /// Get 95% confidence interval half-width (k ≈ 2).
    pub fn u95(&self) -> f64 {
        self.expanded(1.96)
    }

    /// Get 99% confidence interval half-width (k ≈ 2.576).
    pub fn u99(&self) -> f64 {
        self.expanded(2.576)
    }

    /// Check if uncertainty is characterized.
    pub fn is_known(&self) -> bool {
        !matches!(self, Self::Unknown)
    }

    /// Get degrees of freedom (for Type A only).
    pub fn degrees_of_freedom(&self) -> Option<usize> {
        match self {
            Self::TypeA { n_samples, .. } => Some(n_samples.saturating_sub(1)),
            _ => None,
        }
    }
}

impl Default for Uncertainty {
    fn default() -> Self {
        Self::Unknown
    }
}

// ============================================================================
// MEASUREMENT QUALITY
// ============================================================================

/// Quality classification of a measurement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MeasurementQuality {
    /// Calibrated against traceable standard.
    Calibrated,
    /// Validated against reference but not formally calibrated.
    Validated,
    /// Estimated from model or calculation.
    Estimated,
    /// Interpolated between measured points.
    Interpolated,
    /// Extrapolated beyond measured range.
    Extrapolated,
    /// Unknown or unverified quality.
    Unknown,
}

impl MeasurementQuality {
    /// Quality score (1.0 = best, 0.0 = worst).
    pub fn score(&self) -> f64 {
        match self {
            Self::Calibrated => 1.0,
            Self::Validated => 0.9,
            Self::Estimated => 0.7,
            Self::Interpolated => 0.6,
            Self::Extrapolated => 0.4,
            Self::Unknown => 0.0,
        }
    }

    /// Is acceptable for industrial use?
    pub fn is_acceptable(&self) -> bool {
        matches!(self, Self::Calibrated | Self::Validated | Self::Estimated)
    }

    /// Is acceptable for reference use?
    pub fn is_reference_grade(&self) -> bool {
        matches!(self, Self::Calibrated)
    }
}

impl Default for MeasurementQuality {
    fn default() -> Self {
        Self::Unknown
    }
}

// ============================================================================
// MEASUREMENT SOURCE
// ============================================================================

/// Source of the measurement.
#[derive(Debug, Clone)]
pub enum MeasurementSource {
    /// Direct instrument measurement.
    Instrument { name: String, model: String },
    /// Model prediction.
    Model { name: String, version: String },
    /// Literature or database value.
    Reference { citation: String },
    /// User-provided value.
    UserInput,
    /// Calculated/derived value.
    Calculated { method: String },
    /// Neural network correction.
    NeuralCorrection { magnitude: f64 },
    /// Unknown source.
    Unknown,
}

impl MeasurementSource {
    /// Create instrument source.
    pub fn instrument(name: &str, model: &str) -> Self {
        Self::Instrument {
            name: name.to_string(),
            model: model.to_string(),
        }
    }

    /// Create model source.
    pub fn model(name: &str, version: &str) -> Self {
        Self::Model {
            name: name.to_string(),
            version: version.to_string(),
        }
    }

    /// Create reference source.
    pub fn reference(citation: &str) -> Self {
        Self::Reference {
            citation: citation.to_string(),
        }
    }

    /// Create calculated source.
    pub fn calculated(method: &str) -> Self {
        Self::Calculated {
            method: method.to_string(),
        }
    }

    /// Is from a physical instrument?
    pub fn is_measured(&self) -> bool {
        matches!(self, Self::Instrument { .. })
    }

    /// Is from a model/calculation?
    pub fn is_computed(&self) -> bool {
        matches!(
            self,
            Self::Model { .. } | Self::Calculated { .. } | Self::NeuralCorrection { .. }
        )
    }
}

impl Default for MeasurementSource {
    fn default() -> Self {
        Self::Unknown
    }
}

// ============================================================================
// MEASUREMENT ID
// ============================================================================

/// Unique identifier for a measurement (for traceability).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeasurementId(pub u64);

impl MeasurementId {
    /// Generate new ID.
    pub fn generate() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Create from raw value.
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get raw value.
    pub fn value(&self) -> u64 {
        self.0
    }
}

// ============================================================================
// CORE MEASUREMENT TYPE
// ============================================================================

/// A metrologically rigorous measurement.
#[derive(Debug, Clone)]
pub struct Measurement<T> {
    /// Unique identifier.
    pub id: MeasurementId,
    /// Measured value.
    pub value: T,
    /// Uncertainty characterization.
    pub uncertainty: Uncertainty,
    /// Physical unit.
    pub unit: Unit,
    /// Confidence level (e.g., 0.95 for 95% CI).
    pub confidence_level: f64,
    /// Quality classification.
    pub quality: MeasurementQuality,
    /// Timestamp (nanoseconds since epoch).
    pub timestamp: u64,
    /// Source of measurement.
    pub source: MeasurementSource,
}

impl<T: Clone> Measurement<T> {
    /// Create new measurement.
    pub fn new(value: T, unit: Unit) -> Self {
        Self {
            id: MeasurementId::generate(),
            value,
            uncertainty: Uncertainty::Unknown,
            unit,
            confidence_level: 0.95,
            quality: MeasurementQuality::Unknown,
            timestamp: current_timestamp(),
            source: MeasurementSource::Unknown,
        }
    }

    /// Builder: set uncertainty.
    pub fn with_uncertainty(mut self, uncertainty: Uncertainty) -> Self {
        self.uncertainty = uncertainty;
        self
    }

    /// Builder: set quality.
    pub fn with_quality(mut self, quality: MeasurementQuality) -> Self {
        self.quality = quality;
        self
    }

    /// Builder: set source.
    pub fn with_source(mut self, source: MeasurementSource) -> Self {
        self.source = source;
        self
    }

    /// Builder: set confidence level.
    pub fn with_confidence_level(mut self, level: f64) -> Self {
        self.confidence_level = level.clamp(0.5, 0.9999);
        self
    }

    /// Builder: set timestamp.
    pub fn with_timestamp(mut self, timestamp: u64) -> Self {
        self.timestamp = timestamp;
        self
    }

    /// Get the value.
    pub fn get_value(&self) -> &T {
        &self.value
    }
}

impl Measurement<f64> {
    /// Create dimensionless measurement.
    pub fn dimensionless(value: f64) -> Self {
        Self::new(value, Unit::Dimensionless)
    }

    /// Create reflectance measurement.
    pub fn reflectance(value: f64) -> Self {
        Self::new(value, Unit::Reflectance)
    }

    /// Create transmittance measurement.
    pub fn transmittance(value: f64) -> Self {
        Self::new(value, Unit::Transmittance)
    }

    /// Create wavelength measurement.
    pub fn wavelength_nm(value: f64) -> Self {
        Self::new(value, Unit::Nanometers)
    }

    /// Create angle measurement in degrees.
    pub fn angle_deg(value: f64) -> Self {
        Self::new(value, Unit::Degrees)
    }

    /// Create angle measurement in radians.
    pub fn angle_rad(value: f64) -> Self {
        Self::new(value, Unit::Radians)
    }

    /// Create delta E measurement.
    pub fn delta_e(value: f64) -> Self {
        Self::new(value, Unit::DeltaE)
    }

    /// Create a calibrated measurement with Type A uncertainty.
    pub fn calibrated(value: f64, uncertainty: f64, unit: Unit) -> Self {
        Self::new(value, unit)
            .with_uncertainty(Uncertainty::type_a(uncertainty, 100))
            .with_quality(MeasurementQuality::Calibrated)
    }

    /// Check if value is within tolerance of target.
    pub fn is_within_tolerance(&self, target: f64, tolerance: f64) -> bool {
        (self.value - target).abs() <= tolerance
    }

    /// Get relative uncertainty (std / value).
    pub fn relative_uncertainty(&self) -> f64 {
        if self.value.abs() < 1e-15 {
            f64::INFINITY
        } else {
            self.uncertainty.standard() / self.value.abs()
        }
    }

    /// Get confidence interval (lower, upper) at configured level.
    pub fn confidence_interval(&self) -> (f64, f64) {
        let k = confidence_factor(self.confidence_level);
        let half_width = self.uncertainty.expanded(k);
        (self.value - half_width, self.value + half_width)
    }

    /// Check if another value is within confidence interval.
    pub fn contains(&self, other: f64) -> bool {
        let (lower, upper) = self.confidence_interval();
        other >= lower && other <= upper
    }

    /// Get lower bound at confidence level.
    pub fn lower_bound(&self) -> f64 {
        self.confidence_interval().0
    }

    /// Get upper bound at confidence level.
    pub fn upper_bound(&self) -> f64 {
        self.confidence_interval().1
    }

    /// Expand uncertainty to different confidence level.
    pub fn expand_to(&self, level: f64) -> Measurement<f64> {
        let mut expanded = self.clone();
        expanded.confidence_level = level;
        expanded
    }
}

impl fmt::Display for Measurement<f64> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let u = self.uncertainty.standard();
        if u.is_finite() && u > 0.0 {
            write!(
                f,
                "{:.6} ± {:.6} {} ({}% CI)",
                self.value,
                self.uncertainty.u95(),
                self.unit.symbol(),
                (self.confidence_level * 100.0) as u32
            )
        } else {
            write!(f, "{:.6} {}", self.value, self.unit.symbol())
        }
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Get current timestamp in nanoseconds.
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

/// Get coverage factor for confidence level.
fn confidence_factor(level: f64) -> f64 {
    // Based on normal distribution
    match level {
        l if l >= 0.999 => 3.291,
        l if l >= 0.99 => 2.576,
        l if l >= 0.95 => 1.96,
        l if l >= 0.90 => 1.645,
        l if l >= 0.68 => 1.0,
        _ => 1.0,
    }
}

// ============================================================================
// MEASUREMENT ARRAY
// ============================================================================

/// Array of measurements (e.g., spectral data).
#[derive(Debug, Clone)]
pub struct MeasurementArray {
    /// Values.
    pub values: Vec<f64>,
    /// Per-value uncertainties.
    pub uncertainties: Vec<f64>,
    /// Unit.
    pub unit: Unit,
    /// Quality.
    pub quality: MeasurementQuality,
    /// Domain values (e.g., wavelengths).
    pub domain: Vec<f64>,
    /// Domain unit.
    pub domain_unit: Unit,
}

impl MeasurementArray {
    /// Create spectral measurement array.
    pub fn spectral(wavelengths: Vec<f64>, values: Vec<f64>) -> Self {
        let n = values.len();
        Self {
            values,
            uncertainties: vec![0.0; n],
            unit: Unit::Reflectance,
            quality: MeasurementQuality::Unknown,
            domain: wavelengths,
            domain_unit: Unit::Nanometers,
        }
    }

    /// Create angular measurement array.
    pub fn angular(angles_deg: Vec<f64>, values: Vec<f64>) -> Self {
        let n = values.len();
        Self {
            values,
            uncertainties: vec![0.0; n],
            unit: Unit::Reflectance,
            quality: MeasurementQuality::Unknown,
            domain: angles_deg,
            domain_unit: Unit::Degrees,
        }
    }

    /// Set uniform uncertainty.
    pub fn with_uniform_uncertainty(mut self, u: f64) -> Self {
        self.uncertainties = vec![u; self.values.len()];
        self
    }

    /// Set per-value uncertainties.
    pub fn with_uncertainties(mut self, uncertainties: Vec<f64>) -> Self {
        if uncertainties.len() == self.values.len() {
            self.uncertainties = uncertainties;
        }
        self
    }

    /// Set quality.
    pub fn with_quality(mut self, quality: MeasurementQuality) -> Self {
        self.quality = quality;
        self
    }

    /// Get number of points.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Get mean value.
    pub fn mean(&self) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }
        self.values.iter().sum::<f64>() / self.values.len() as f64
    }

    /// Alias for mean().
    pub fn mean_value(&self) -> f64 {
        self.mean()
    }

    /// Get maximum value.
    pub fn max(&self) -> f64 {
        self.values
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max)
    }

    /// Get minimum value.
    pub fn min(&self) -> f64 {
        self.values.iter().cloned().fold(f64::INFINITY, f64::min)
    }

    /// Get mean uncertainty.
    pub fn mean_uncertainty(&self) -> f64 {
        if self.uncertainties.is_empty() {
            return 0.0;
        }
        self.uncertainties.iter().sum::<f64>() / self.uncertainties.len() as f64
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uncertainty_type_a() {
        let u = Uncertainty::type_a(0.01, 10);
        assert!((u.standard() - 0.01).abs() < 1e-10);
        assert_eq!(u.degrees_of_freedom(), Some(9));
    }

    #[test]
    fn test_uncertainty_combined() {
        let u = Uncertainty::combined(0.03, 0.04);
        assert!((u.standard() - 0.05).abs() < 1e-10);
    }

    #[test]
    fn test_measurement_creation() {
        let m = Measurement::reflectance(0.95)
            .with_uncertainty(Uncertainty::type_a(0.01, 10))
            .with_quality(MeasurementQuality::Calibrated);

        assert!((m.value - 0.95).abs() < 1e-10);
        assert!(m.quality.is_reference_grade());
    }

    #[test]
    fn test_confidence_interval() {
        let m = Measurement::reflectance(0.5).with_uncertainty(Uncertainty::type_a(0.01, 100));

        let (lower, upper) = m.confidence_interval();
        assert!(lower < 0.5);
        assert!(upper > 0.5);
        assert!(m.contains(0.5));
    }

    #[test]
    fn test_relative_uncertainty() {
        let m = Measurement::reflectance(0.5).with_uncertainty(Uncertainty::type_a(0.05, 10));

        assert!((m.relative_uncertainty() - 0.1).abs() < 1e-10);
    }

    #[test]
    fn test_measurement_id_uniqueness() {
        let id1 = MeasurementId::generate();
        let id2 = MeasurementId::generate();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_measurement_array() {
        let wavelengths = vec![400.0, 500.0, 600.0, 700.0];
        let values = vec![0.1, 0.2, 0.3, 0.4];
        let arr = MeasurementArray::spectral(wavelengths, values).with_uniform_uncertainty(0.01);

        assert_eq!(arr.len(), 4);
        assert!((arr.mean() - 0.25).abs() < 1e-10);
    }
}

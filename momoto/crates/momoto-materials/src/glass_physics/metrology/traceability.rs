//! # Traceability Chain
//!
//! Audit trail for metrological measurements.

use super::measurement::MeasurementId;
use std::fmt;

// ============================================================================
// TRACEABILITY OPERATION
// ============================================================================

/// Operation type in the traceability chain.
#[derive(Debug, Clone)]
pub enum TraceabilityOperation {
    /// Direct measurement from instrument.
    DirectMeasurement { instrument: String, method: String },
    /// Calibration against reference.
    Calibration {
        reference: String,
        certificate_id: Option<String>,
    },
    /// Interpolation between points.
    Interpolation { method: String },
    /// Model-based prediction.
    ModelPrediction {
        model_name: String,
        model_version: String,
    },
    /// Neural network correction.
    NeuralCorrection { magnitude: f64, share: f64 },
    /// Aggregation of multiple sources.
    Aggregation { method: String, source_count: usize },
    /// Unit conversion.
    UnitConversion { from_unit: String, to_unit: String },
    /// Error propagation.
    ErrorPropagation { method: String },
    /// Manual entry or correction.
    ManualEntry { operator: String, reason: String },
}

impl TraceabilityOperation {
    /// Create direct measurement operation.
    pub fn measurement(instrument: &str, method: &str) -> Self {
        Self::DirectMeasurement {
            instrument: instrument.to_string(),
            method: method.to_string(),
        }
    }

    /// Create calibration operation.
    pub fn calibration(reference: &str) -> Self {
        Self::Calibration {
            reference: reference.to_string(),
            certificate_id: None,
        }
    }

    /// Create interpolation operation.
    pub fn interpolation(method: &str) -> Self {
        Self::Interpolation {
            method: method.to_string(),
        }
    }

    /// Create model prediction operation.
    pub fn model(name: &str, version: &str) -> Self {
        Self::ModelPrediction {
            model_name: name.to_string(),
            model_version: version.to_string(),
        }
    }

    /// Create neural correction operation.
    pub fn neural(magnitude: f64, share: f64) -> Self {
        Self::NeuralCorrection { magnitude, share }
    }

    /// Create aggregation operation.
    pub fn aggregation(method: &str, count: usize) -> Self {
        Self::Aggregation {
            method: method.to_string(),
            source_count: count,
        }
    }

    /// Get operation category.
    pub fn category(&self) -> &'static str {
        match self {
            Self::DirectMeasurement { .. } => "measurement",
            Self::Calibration { .. } => "calibration",
            Self::Interpolation { .. } => "interpolation",
            Self::ModelPrediction { .. } => "model",
            Self::NeuralCorrection { .. } => "neural",
            Self::Aggregation { .. } => "aggregation",
            Self::UnitConversion { .. } => "conversion",
            Self::ErrorPropagation { .. } => "propagation",
            Self::ManualEntry { .. } => "manual",
        }
    }

    /// Is this a computational operation?
    pub fn is_computational(&self) -> bool {
        matches!(
            self,
            Self::Interpolation { .. }
                | Self::ModelPrediction { .. }
                | Self::NeuralCorrection { .. }
                | Self::ErrorPropagation { .. }
        )
    }

    /// Is this a neural operation?
    pub fn is_neural(&self) -> bool {
        matches!(self, Self::NeuralCorrection { .. })
    }
}

impl fmt::Display for TraceabilityOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DirectMeasurement { instrument, method } => {
                write!(f, "Measured with {} ({})", instrument, method)
            }
            Self::Calibration { reference, .. } => {
                write!(f, "Calibrated against {}", reference)
            }
            Self::Interpolation { method } => {
                write!(f, "Interpolated using {}", method)
            }
            Self::ModelPrediction {
                model_name,
                model_version,
            } => {
                write!(f, "Predicted by {} v{}", model_name, model_version)
            }
            Self::NeuralCorrection { magnitude, share } => {
                write!(
                    f,
                    "Neural correction: {:.4} ({:.1}% share)",
                    magnitude,
                    share * 100.0
                )
            }
            Self::Aggregation {
                method,
                source_count,
            } => {
                write!(f, "Aggregated {} sources using {}", source_count, method)
            }
            Self::UnitConversion { from_unit, to_unit } => {
                write!(f, "Converted {} -> {}", from_unit, to_unit)
            }
            Self::ErrorPropagation { method } => {
                write!(f, "Error propagated via {}", method)
            }
            Self::ManualEntry { operator, reason } => {
                write!(f, "Manual entry by {}: {}", operator, reason)
            }
        }
    }
}

// ============================================================================
// TRACEABILITY ENTRY
// ============================================================================

/// Single entry in the traceability chain.
#[derive(Debug, Clone)]
pub struct TraceabilityEntry {
    /// Unique step ID.
    pub step_id: u32,
    /// Operation performed.
    pub operation: TraceabilityOperation,
    /// Input measurement IDs.
    pub input_measurements: Vec<MeasurementId>,
    /// Output measurement ID.
    pub output_measurement: MeasurementId,
    /// Timestamp (nanoseconds since epoch).
    pub timestamp: u64,
    /// Operator or system identifier.
    pub operator: String,
    /// Additional notes.
    pub notes: Option<String>,
}

impl TraceabilityEntry {
    /// Create new entry.
    pub fn new(
        step_id: u32,
        operation: TraceabilityOperation,
        inputs: Vec<MeasurementId>,
        output: MeasurementId,
    ) -> Self {
        Self {
            step_id,
            operation,
            input_measurements: inputs,
            output_measurement: output,
            timestamp: current_timestamp(),
            operator: "system".to_string(),
            notes: None,
        }
    }

    /// Set operator.
    pub fn with_operator(mut self, operator: &str) -> Self {
        self.operator = operator.to_string();
        self
    }

    /// Add notes.
    pub fn with_notes(mut self, notes: &str) -> Self {
        self.notes = Some(notes.to_string());
        self
    }

    /// Set timestamp.
    pub fn with_timestamp(mut self, timestamp: u64) -> Self {
        self.timestamp = timestamp;
        self
    }
}

// ============================================================================
// CALIBRATION REFERENCE
// ============================================================================

/// Reference to a calibration standard.
#[derive(Debug, Clone)]
pub struct CalibrationReference {
    /// Standard name.
    pub name: String,
    /// Certificate number.
    pub certificate_id: Option<String>,
    /// Issuing laboratory.
    pub laboratory: String,
    /// Calibration date (nanoseconds since epoch).
    pub date: u64,
    /// Validity period (days).
    pub validity_days: u32,
    /// Uncertainty of the reference.
    pub uncertainty: f64,
}

impl CalibrationReference {
    /// Create new calibration reference.
    pub fn new(name: &str, laboratory: &str) -> Self {
        Self {
            name: name.to_string(),
            certificate_id: None,
            laboratory: laboratory.to_string(),
            date: current_timestamp(),
            validity_days: 365,
            uncertainty: 0.0,
        }
    }

    /// Set certificate ID.
    pub fn with_certificate(mut self, id: &str) -> Self {
        self.certificate_id = Some(id.to_string());
        self
    }

    /// Set uncertainty.
    pub fn with_uncertainty(mut self, u: f64) -> Self {
        self.uncertainty = u;
        self
    }

    /// Set validity period.
    pub fn with_validity(mut self, days: u32) -> Self {
        self.validity_days = days;
        self
    }

    /// Check if calibration is still valid.
    pub fn is_valid(&self) -> bool {
        let now = current_timestamp();
        let validity_ns = (self.validity_days as u64) * 24 * 60 * 60 * 1_000_000_000;
        now < self.date + validity_ns
    }

    /// Days until expiration.
    pub fn days_until_expiry(&self) -> i64 {
        let now = current_timestamp();
        let expiry = self.date + (self.validity_days as u64) * 24 * 60 * 60 * 1_000_000_000;
        let diff_ns = expiry as i64 - now as i64;
        diff_ns / (24 * 60 * 60 * 1_000_000_000)
    }
}

// ============================================================================
// TRACEABILITY CHAIN
// ============================================================================

/// Complete traceability chain for audit.
#[derive(Debug, Clone, Default)]
pub struct TraceabilityChain {
    /// Chain entries in order.
    pub entries: Vec<TraceabilityEntry>,
    /// Root calibration reference.
    pub root_calibration: Option<CalibrationReference>,
    /// Chain metadata.
    pub metadata: ChainMetadata,
}

/// Metadata for the chain.
#[derive(Debug, Clone, Default)]
pub struct ChainMetadata {
    /// Chain creation time.
    pub created_at: u64,
    /// Last modification time.
    pub modified_at: u64,
    /// Chain version.
    pub version: u32,
    /// Description.
    pub description: Option<String>,
}

impl TraceabilityChain {
    /// Create new empty chain.
    pub fn new() -> Self {
        let now = current_timestamp();
        Self {
            entries: Vec::new(),
            root_calibration: None,
            metadata: ChainMetadata {
                created_at: now,
                modified_at: now,
                version: 1,
                description: None,
            },
        }
    }

    /// Create chain with root calibration.
    pub fn with_calibration(calibration: CalibrationReference) -> Self {
        let mut chain = Self::new();
        chain.root_calibration = Some(calibration);
        chain
    }

    /// Add entry to chain.
    pub fn add_entry(&mut self, entry: TraceabilityEntry) {
        self.entries.push(entry);
        self.metadata.modified_at = current_timestamp();
    }

    /// Add measurement operation.
    pub fn record_measurement(&mut self, instrument: &str, method: &str, output: MeasurementId) {
        let step_id = self.next_step_id();
        let entry = TraceabilityEntry::new(
            step_id,
            TraceabilityOperation::measurement(instrument, method),
            Vec::new(),
            output,
        );
        self.add_entry(entry);
    }

    /// Add model prediction operation.
    pub fn record_model_prediction(
        &mut self,
        model: &str,
        version: &str,
        inputs: Vec<MeasurementId>,
        output: MeasurementId,
    ) {
        let step_id = self.next_step_id();
        let entry = TraceabilityEntry::new(
            step_id,
            TraceabilityOperation::model(model, version),
            inputs,
            output,
        );
        self.add_entry(entry);
    }

    /// Add neural correction operation.
    pub fn record_neural_correction(
        &mut self,
        magnitude: f64,
        share: f64,
        input: MeasurementId,
        output: MeasurementId,
    ) {
        let step_id = self.next_step_id();
        let entry = TraceabilityEntry::new(
            step_id,
            TraceabilityOperation::neural(magnitude, share),
            vec![input],
            output,
        );
        self.add_entry(entry);
    }

    /// Get next step ID.
    fn next_step_id(&self) -> u32 {
        self.entries.len() as u32 + 1
    }

    /// Get number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Check if chain has root calibration.
    pub fn is_calibrated(&self) -> bool {
        self.root_calibration.is_some()
    }

    /// Check if calibration is valid.
    pub fn is_calibration_valid(&self) -> bool {
        self.root_calibration
            .as_ref()
            .map(|c| c.is_valid())
            .unwrap_or(false)
    }

    /// Get total neural correction share in chain.
    pub fn total_neural_share(&self) -> f64 {
        self.entries
            .iter()
            .filter_map(|e| match &e.operation {
                TraceabilityOperation::NeuralCorrection { share, .. } => Some(*share),
                _ => None,
            })
            .sum()
    }

    /// Count neural operations in chain.
    pub fn neural_operation_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.operation.is_neural())
            .count()
    }

    /// Get chain summary for reporting.
    pub fn summary(&self) -> ChainSummary {
        ChainSummary {
            total_steps: self.entries.len(),
            measurement_steps: self
                .entries
                .iter()
                .filter(|e| matches!(e.operation, TraceabilityOperation::DirectMeasurement { .. }))
                .count(),
            model_steps: self
                .entries
                .iter()
                .filter(|e| matches!(e.operation, TraceabilityOperation::ModelPrediction { .. }))
                .count(),
            neural_steps: self.neural_operation_count(),
            has_calibration: self.is_calibrated(),
            calibration_valid: self.is_calibration_valid(),
            total_neural_share: self.total_neural_share(),
        }
    }

    /// Generate human-readable report.
    pub fn report(&self) -> String {
        let mut lines = Vec::new();

        lines.push("=== Traceability Chain Report ===".to_string());
        lines.push(format!(
            "Created: {}",
            format_timestamp(self.metadata.created_at)
        ));
        lines.push(format!("Steps: {}", self.entries.len()));

        if let Some(ref cal) = self.root_calibration {
            lines.push(format!("\nRoot Calibration:"));
            lines.push(format!("  Standard: {}", cal.name));
            lines.push(format!("  Laboratory: {}", cal.laboratory));
            if let Some(ref cert) = cal.certificate_id {
                lines.push(format!("  Certificate: {}", cert));
            }
            lines.push(format!(
                "  Valid: {}",
                if cal.is_valid() { "Yes" } else { "No" }
            ));
        }

        lines.push(format!("\nChain Steps:"));
        for entry in &self.entries {
            lines.push(format!(
                "  [{}] {} ({})",
                entry.step_id, entry.operation, entry.operator
            ));
        }

        let summary = self.summary();
        lines.push(format!("\nSummary:"));
        lines.push(format!(
            "  Measurement steps: {}",
            summary.measurement_steps
        ));
        lines.push(format!("  Model steps: {}", summary.model_steps));
        lines.push(format!("  Neural steps: {}", summary.neural_steps));
        lines.push(format!(
            "  Total neural share: {:.1}%",
            summary.total_neural_share * 100.0
        ));

        lines.join("\n")
    }
}

/// Summary of chain contents.
#[derive(Debug, Clone)]
pub struct ChainSummary {
    /// Total number of steps.
    pub total_steps: usize,
    /// Number of direct measurements.
    pub measurement_steps: usize,
    /// Number of model predictions.
    pub model_steps: usize,
    /// Number of neural corrections.
    pub neural_steps: usize,
    /// Has root calibration.
    pub has_calibration: bool,
    /// Calibration is valid.
    pub calibration_valid: bool,
    /// Total neural correction share.
    pub total_neural_share: f64,
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

/// Format timestamp for display.
fn format_timestamp(ns: u64) -> String {
    let secs = ns / 1_000_000_000;
    format!("{} (epoch seconds)", secs)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_traceability_operation() {
        let op = TraceabilityOperation::measurement("Spectrophotometer", "diffuse reflectance");
        assert_eq!(op.category(), "measurement");
        assert!(!op.is_computational());
    }

    #[test]
    fn test_neural_operation() {
        let op = TraceabilityOperation::neural(0.05, 0.03);
        assert!(op.is_neural());
        assert!(op.is_computational());
    }

    #[test]
    fn test_calibration_reference() {
        let cal = CalibrationReference::new("NIST SRM 2003", "NIST")
            .with_certificate("CAL-2024-001")
            .with_validity(365);

        assert!(cal.is_valid());
        assert!(cal.days_until_expiry() > 0);
    }

    #[test]
    fn test_traceability_chain() {
        let mut chain = TraceabilityChain::new();

        let id1 = MeasurementId::generate();
        let id2 = MeasurementId::generate();

        chain.record_measurement("Gonioreflectometer", "BRDF", id1);
        chain.record_neural_correction(0.02, 0.01, id1, id2);

        assert_eq!(chain.len(), 2);
        assert_eq!(chain.neural_operation_count(), 1);
        assert!((chain.total_neural_share() - 0.01).abs() < 1e-10);
    }

    #[test]
    fn test_chain_summary() {
        let cal = CalibrationReference::new("Standard", "Lab");
        let mut chain = TraceabilityChain::with_calibration(cal);

        let id1 = MeasurementId::generate();
        chain.record_measurement("Instrument", "method", id1);

        let summary = chain.summary();
        assert!(summary.has_calibration);
        assert!(summary.calibration_valid);
        assert_eq!(summary.measurement_steps, 1);
    }
}

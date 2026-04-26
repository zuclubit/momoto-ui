//! # Twin Variants
//!
//! Different types of material twins for various use cases.

// ============================================================================
// TWIN VARIANT ENUM
// ============================================================================

/// Classification of material twin types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TwinVariant {
    /// Static twin - no temporal evolution.
    Static,
    /// Temporal twin - evolves over time.
    Temporal,
    /// Layered twin - multiple material layers.
    Layered { layer_count: usize },
    /// Measured twin - from real measurement data.
    Measured,
}

impl TwinVariant {
    /// Check if variant is static.
    pub fn is_static(&self) -> bool {
        matches!(self, TwinVariant::Static)
    }

    /// Check if variant is temporal.
    pub fn is_temporal(&self) -> bool {
        matches!(self, TwinVariant::Temporal)
    }

    /// Check if variant is layered.
    pub fn is_layered(&self) -> bool {
        matches!(self, TwinVariant::Layered { .. })
    }

    /// Check if variant is from measured data.
    pub fn is_measured(&self) -> bool {
        matches!(self, TwinVariant::Measured)
    }

    /// Get layer count (1 for non-layered).
    pub fn layer_count(&self) -> usize {
        match self {
            TwinVariant::Layered { layer_count } => *layer_count,
            _ => 1,
        }
    }

    /// Get description string.
    pub fn description(&self) -> &'static str {
        match self {
            TwinVariant::Static => "Static material (no evolution)",
            TwinVariant::Temporal => "Temporal material (time-evolving)",
            TwinVariant::Layered { .. } => "Layered material (multi-layer)",
            TwinVariant::Measured => "Measured material (from BRDF data)",
        }
    }
}

impl Default for TwinVariant {
    fn default() -> Self {
        TwinVariant::Static
    }
}

impl std::fmt::Display for TwinVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TwinVariant::Static => write!(f, "Static"),
            TwinVariant::Temporal => write!(f, "Temporal"),
            TwinVariant::Layered { layer_count } => write!(f, "Layered({})", layer_count),
            TwinVariant::Measured => write!(f, "Measured"),
        }
    }
}

// ============================================================================
// STATIC TWIN DATA
// ============================================================================

/// Additional data for static twins.
#[derive(Debug, Clone, Default)]
pub struct StaticTwinData {
    /// Whether the twin has been validated.
    pub validated: bool,
    /// Validation score (0-100).
    pub validation_score: Option<f64>,
    /// Energy conservation error.
    pub energy_error: Option<f64>,
    /// Notes about the twin.
    pub notes: Vec<String>,
}

impl StaticTwinData {
    /// Create new static twin data.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark as validated with score.
    pub fn validate(&mut self, score: f64, energy_error: f64) {
        self.validated = true;
        self.validation_score = Some(score);
        self.energy_error = Some(energy_error);
    }

    /// Add a note.
    pub fn add_note(&mut self, note: &str) {
        self.notes.push(note.to_string());
    }

    /// Check if validation passed (score >= 90).
    pub fn validation_passed(&self) -> bool {
        self.validated && self.validation_score.map_or(false, |s| s >= 90.0)
    }
}

// ============================================================================
// TEMPORAL TWIN DATA
// ============================================================================

/// Additional data for temporal twins.
#[derive(Debug, Clone)]
pub struct TemporalTwinData {
    /// Start time of evolution (seconds from creation).
    pub evolution_start: f64,
    /// Current evolution time.
    pub current_time: f64,
    /// Maximum evolution time (if bounded).
    pub max_time: Option<f64>,
    /// Number of evolution steps taken.
    pub steps_taken: u64,
    /// Cumulative drift observed.
    pub cumulative_drift: f64,
    /// Maximum allowed drift before warning.
    pub drift_threshold: f64,
    /// Whether evolution is paused.
    pub paused: bool,
    /// Evolution checkpoints (time -> fingerprint hash).
    pub checkpoints: Vec<(f64, [u8; 8])>,
}

impl Default for TemporalTwinData {
    fn default() -> Self {
        Self {
            evolution_start: 0.0,
            current_time: 0.0,
            max_time: None,
            steps_taken: 0,
            cumulative_drift: 0.0,
            drift_threshold: 0.01,
            paused: false,
            checkpoints: Vec::new(),
        }
    }
}

impl TemporalTwinData {
    /// Create new temporal twin data.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set drift threshold.
    pub fn with_drift_threshold(mut self, threshold: f64) -> Self {
        self.drift_threshold = threshold;
        self
    }

    /// Set maximum evolution time.
    pub fn with_max_time(mut self, max_time: f64) -> Self {
        self.max_time = Some(max_time);
        self
    }

    /// Advance evolution by time step.
    pub fn advance(&mut self, dt: f64, drift: f64) {
        if !self.paused {
            self.current_time += dt;
            self.steps_taken += 1;
            self.cumulative_drift += drift.abs();
        }
    }

    /// Pause evolution.
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Resume evolution.
    pub fn resume(&mut self) {
        self.paused = false;
    }

    /// Check if drift threshold exceeded.
    pub fn drift_exceeded(&self) -> bool {
        self.cumulative_drift > self.drift_threshold
    }

    /// Check if evolution is complete (reached max time).
    pub fn is_complete(&self) -> bool {
        self.max_time.map_or(false, |max| self.current_time >= max)
    }

    /// Get evolution progress (0-1).
    pub fn progress(&self) -> f64 {
        self.max_time
            .map_or(0.0, |max| (self.current_time / max).min(1.0))
    }

    /// Add checkpoint with current fingerprint hash (first 8 bytes).
    pub fn add_checkpoint(&mut self, fingerprint_hash: [u8; 8]) {
        self.checkpoints.push((self.current_time, fingerprint_hash));
    }

    /// Get most recent checkpoint.
    pub fn latest_checkpoint(&self) -> Option<&(f64, [u8; 8])> {
        self.checkpoints.last()
    }

    /// Reset evolution.
    pub fn reset(&mut self) {
        self.current_time = self.evolution_start;
        self.steps_taken = 0;
        self.cumulative_drift = 0.0;
        self.paused = false;
        self.checkpoints.clear();
    }
}

// ============================================================================
// LAYERED TWIN DATA
// ============================================================================

/// Information about a layer in a layered twin.
#[derive(Debug, Clone)]
pub struct LayerInfo {
    /// Layer index (0 = top/coating).
    pub index: usize,
    /// Layer name.
    pub name: String,
    /// Layer thickness (nm, if applicable).
    pub thickness_nm: Option<f64>,
    /// Layer contribution weight.
    pub weight: f64,
    /// Whether this is the substrate.
    pub is_substrate: bool,
}

impl LayerInfo {
    /// Create new layer info.
    pub fn new(index: usize, name: &str) -> Self {
        Self {
            index,
            name: name.to_string(),
            thickness_nm: None,
            weight: 1.0,
            is_substrate: false,
        }
    }

    /// Set thickness.
    pub fn with_thickness(mut self, thickness_nm: f64) -> Self {
        self.thickness_nm = Some(thickness_nm);
        self
    }

    /// Set weight.
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }

    /// Mark as substrate.
    pub fn as_substrate(mut self) -> Self {
        self.is_substrate = true;
        self
    }
}

/// Additional data for layered twins.
#[derive(Debug, Clone, Default)]
pub struct LayeredTwinData {
    /// Layer information.
    pub layers: Vec<LayerInfo>,
    /// Total thickness (nm).
    pub total_thickness_nm: Option<f64>,
    /// Inter-layer coupling strength.
    pub coupling: f64,
    /// Notes about the layer structure.
    pub notes: Vec<String>,
}

impl LayeredTwinData {
    /// Create new layered twin data.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a layer.
    pub fn add_layer(&mut self, layer: LayerInfo) {
        self.layers.push(layer);
        self.update_total_thickness();
    }

    /// Get layer count.
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    /// Get layer by index.
    pub fn get_layer(&self, index: usize) -> Option<&LayerInfo> {
        self.layers.get(index)
    }

    /// Get substrate layer (last layer marked as substrate).
    pub fn substrate(&self) -> Option<&LayerInfo> {
        self.layers.iter().rev().find(|l| l.is_substrate)
    }

    /// Update total thickness from layers.
    fn update_total_thickness(&mut self) {
        let total: Option<f64> = self
            .layers
            .iter()
            .map(|l| l.thickness_nm)
            .try_fold(0.0, |acc, t| t.map(|v| acc + v));
        self.total_thickness_nm = total;
    }

    /// Set coupling strength.
    pub fn with_coupling(mut self, coupling: f64) -> Self {
        self.coupling = coupling;
        self
    }

    /// Add note.
    pub fn add_note(&mut self, note: &str) {
        self.notes.push(note.to_string());
    }
}

// ============================================================================
// MEASURED TWIN DATA
// ============================================================================

/// Additional data for twins from measured data.
#[derive(Debug, Clone)]
pub struct MeasuredTwinData {
    /// Source dataset name.
    pub dataset: String,
    /// Material name in dataset.
    pub material_name: String,
    /// Number of measured angles.
    pub angle_count: usize,
    /// Number of measured wavelengths.
    pub wavelength_count: usize,
    /// Measurement date (if known).
    pub measurement_date: Option<String>,
    /// Measurement conditions.
    pub conditions: MeasurementConditions,
    /// Fit residual (ΔE2000).
    pub fit_residual: Option<f64>,
}

/// Measurement conditions.
#[derive(Debug, Clone, Default)]
pub struct MeasurementConditions {
    /// Temperature (Celsius).
    pub temperature_c: Option<f64>,
    /// Humidity (percent).
    pub humidity_pct: Option<f64>,
    /// Illuminant (e.g., "D65").
    pub illuminant: Option<String>,
    /// Integration time (ms).
    pub integration_time_ms: Option<f64>,
    /// Notes.
    pub notes: Vec<String>,
}

impl Default for MeasuredTwinData {
    fn default() -> Self {
        Self {
            dataset: "unknown".to_string(),
            material_name: "unknown".to_string(),
            angle_count: 0,
            wavelength_count: 0,
            measurement_date: None,
            conditions: MeasurementConditions::default(),
            fit_residual: None,
        }
    }
}

impl MeasuredTwinData {
    /// Create new measured twin data.
    pub fn new(dataset: &str, material_name: &str) -> Self {
        Self {
            dataset: dataset.to_string(),
            material_name: material_name.to_string(),
            ..Default::default()
        }
    }

    /// Set angle count.
    pub fn with_angles(mut self, count: usize) -> Self {
        self.angle_count = count;
        self
    }

    /// Set wavelength count.
    pub fn with_wavelengths(mut self, count: usize) -> Self {
        self.wavelength_count = count;
        self
    }

    /// Set fit residual.
    pub fn with_fit_residual(mut self, residual: f64) -> Self {
        self.fit_residual = Some(residual);
        self
    }

    /// Set measurement date.
    pub fn with_date(mut self, date: &str) -> Self {
        self.measurement_date = Some(date.to_string());
        self
    }

    /// Get total observation count.
    pub fn observation_count(&self) -> usize {
        self.angle_count * self.wavelength_count.max(1)
    }

    /// Check if fit quality is good (ΔE < 2.0).
    pub fn fit_quality_good(&self) -> bool {
        self.fit_residual.map_or(false, |r| r < 2.0)
    }

    /// Check if fit quality is reference-grade (ΔE < 1.0).
    pub fn fit_quality_reference(&self) -> bool {
        self.fit_residual.map_or(false, |r| r < 1.0)
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_twin_variant_classification() {
        assert!(TwinVariant::Static.is_static());
        assert!(TwinVariant::Temporal.is_temporal());
        assert!(TwinVariant::Layered { layer_count: 3 }.is_layered());
        assert!(TwinVariant::Measured.is_measured());
    }

    #[test]
    fn test_twin_variant_layer_count() {
        assert_eq!(TwinVariant::Static.layer_count(), 1);
        assert_eq!(TwinVariant::Layered { layer_count: 5 }.layer_count(), 5);
    }

    #[test]
    fn test_twin_variant_display() {
        assert_eq!(format!("{}", TwinVariant::Static), "Static");
        assert_eq!(
            format!("{}", TwinVariant::Layered { layer_count: 2 }),
            "Layered(2)"
        );
    }

    #[test]
    fn test_static_twin_data() {
        let mut data = StaticTwinData::new();
        assert!(!data.validated);
        assert!(!data.validation_passed());

        data.validate(95.0, 0.001);
        assert!(data.validated);
        assert!(data.validation_passed());
    }

    #[test]
    fn test_temporal_twin_data() {
        let mut data = TemporalTwinData::new()
            .with_drift_threshold(0.05)
            .with_max_time(10.0);

        assert!(!data.paused);
        assert!(!data.is_complete());
        assert_eq!(data.progress(), 0.0);

        data.advance(5.0, 0.01);
        assert_eq!(data.steps_taken, 1);
        assert!((data.progress() - 0.5).abs() < 0.01);

        data.advance(6.0, 0.01);
        assert!(data.is_complete());
    }

    #[test]
    fn test_temporal_twin_drift() {
        let mut data = TemporalTwinData::new().with_drift_threshold(0.02);

        data.advance(1.0, 0.01);
        assert!(!data.drift_exceeded());

        data.advance(1.0, 0.015);
        assert!(data.drift_exceeded());
    }

    #[test]
    fn test_temporal_twin_pause_resume() {
        let mut data = TemporalTwinData::new();

        data.advance(1.0, 0.0);
        assert_eq!(data.current_time, 1.0);

        data.pause();
        data.advance(1.0, 0.0);
        assert_eq!(data.current_time, 1.0); // Should not advance

        data.resume();
        data.advance(1.0, 0.0);
        assert_eq!(data.current_time, 2.0);
    }

    #[test]
    fn test_layered_twin_data() {
        let mut data = LayeredTwinData::new();

        data.add_layer(LayerInfo::new(0, "AR Coating").with_thickness(100.0));
        data.add_layer(
            LayerInfo::new(1, "Glass")
                .with_thickness(1000.0)
                .as_substrate(),
        );

        assert_eq!(data.layer_count(), 2);
        assert!(data.total_thickness_nm.is_some());
        assert!((data.total_thickness_nm.unwrap() - 1100.0).abs() < 0.01);

        let substrate = data.substrate();
        assert!(substrate.is_some());
        assert_eq!(substrate.unwrap().name, "Glass");
    }

    #[test]
    fn test_measured_twin_data() {
        let data = MeasuredTwinData::new("MERL-100", "gold")
            .with_angles(90)
            .with_wavelengths(31)
            .with_fit_residual(0.8);

        assert_eq!(data.observation_count(), 90 * 31);
        assert!(data.fit_quality_good());
        assert!(data.fit_quality_reference());
    }

    #[test]
    fn test_measured_fit_quality() {
        let good = MeasuredTwinData::new("test", "mat").with_fit_residual(1.5);
        let ref_grade = MeasuredTwinData::new("test", "mat").with_fit_residual(0.5);
        let poor = MeasuredTwinData::new("test", "mat").with_fit_residual(5.0);

        assert!(good.fit_quality_good());
        assert!(!good.fit_quality_reference());

        assert!(ref_grade.fit_quality_good());
        assert!(ref_grade.fit_quality_reference());

        assert!(!poor.fit_quality_good());
    }
}

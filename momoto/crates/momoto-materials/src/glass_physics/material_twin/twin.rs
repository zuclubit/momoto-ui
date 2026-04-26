//! # Core MaterialTwin Type
//!
//! The MaterialTwin struct and TwinBuilder for creating digital material twins.

use super::super::differentiable::DifferentiableBSDF;
use super::super::material_fingerprint::MaterialFingerprint;
use super::super::temporal::TemporalEvolution;
use super::identity::SpectralIdentity;
use super::variants::TwinVariant;

// ============================================================================
// TWIN ID
// ============================================================================

/// Unique identifier for a material twin (UUID v4 format).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TwinId([u8; 16]);

impl TwinId {
    /// Generate a new random TwinId.
    pub fn generate() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        // Simple pseudo-random generation using system time + counter
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);

        // Add monotonic counter to ensure uniqueness even in rapid succession
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);

        let mut bytes = [0u8; 16];

        // Mix timestamp and counter with simple PRNG
        let mut state = (timestamp as u64).wrapping_add(counter);
        for byte in &mut bytes {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            *byte = (state >> 32) as u8;
        }

        // Set UUID version (4) and variant (RFC 4122)
        bytes[6] = (bytes[6] & 0x0f) | 0x40; // Version 4
        bytes[8] = (bytes[8] & 0x3f) | 0x80; // Variant RFC 4122

        Self(bytes)
    }

    /// Create TwinId from bytes.
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Get raw bytes.
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// Format as UUID string.
    pub fn to_uuid_string(&self) -> String {
        format!(
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3],
            self.0[4], self.0[5],
            self.0[6], self.0[7],
            self.0[8], self.0[9],
            self.0[10], self.0[11], self.0[12], self.0[13], self.0[14], self.0[15],
        )
    }

    /// Get short form (first 8 hex chars).
    pub fn short(&self) -> String {
        format!(
            "{:02x}{:02x}{:02x}{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3]
        )
    }
}

impl std::fmt::Display for TwinId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_uuid_string())
    }
}

// ============================================================================
// CALIBRATION METADATA
// ============================================================================

/// Quality level of calibration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalibrationQuality {
    /// No calibration performed.
    Uncalibrated,
    /// Basic calibration with limited data.
    Basic,
    /// Standard calibration with good coverage.
    Standard,
    /// High-quality calibration with comprehensive data.
    High,
    /// Reference-grade calibration with measured BRDF data.
    Reference,
}

impl CalibrationQuality {
    /// Get numeric score (0-100).
    pub fn score(&self) -> u8 {
        match self {
            CalibrationQuality::Uncalibrated => 0,
            CalibrationQuality::Basic => 25,
            CalibrationQuality::Standard => 50,
            CalibrationQuality::High => 75,
            CalibrationQuality::Reference => 100,
        }
    }

    /// Check if calibration meets minimum quality.
    pub fn meets_minimum(&self, minimum: CalibrationQuality) -> bool {
        self.score() >= minimum.score()
    }
}

/// Metadata about how a twin was calibrated.
#[derive(Debug, Clone)]
pub struct CalibrationMetadata {
    /// Data source used for calibration.
    pub source: String,
    /// Dataset name (e.g., "MERL-100", "measured_gold_sample").
    pub dataset: String,
    /// Number of observations used.
    pub observation_count: usize,
    /// Final loss value after optimization.
    pub final_loss: f64,
    /// Perceptual error (ΔE2000).
    pub delta_e_2000: Option<f64>,
    /// Optimizer used.
    pub optimizer: String,
    /// Number of iterations.
    pub iterations: usize,
    /// Calibration quality.
    pub quality: CalibrationQuality,
    /// Timestamp of calibration (Unix nanos).
    pub calibrated_at: u64,
    /// Duration of calibration (ms).
    pub duration_ms: u64,
}

impl Default for CalibrationMetadata {
    fn default() -> Self {
        Self {
            source: "none".to_string(),
            dataset: "none".to_string(),
            observation_count: 0,
            final_loss: f64::INFINITY,
            delta_e_2000: None,
            optimizer: "none".to_string(),
            iterations: 0,
            quality: CalibrationQuality::Uncalibrated,
            calibrated_at: 0,
            duration_ms: 0,
        }
    }
}

impl CalibrationMetadata {
    /// Create metadata for an uncalibrated twin.
    pub fn uncalibrated() -> Self {
        Self::default()
    }

    /// Create metadata from calibration results.
    pub fn from_calibration(
        source: &str,
        dataset: &str,
        observation_count: usize,
        final_loss: f64,
        delta_e: Option<f64>,
        optimizer: &str,
        iterations: usize,
    ) -> Self {
        let quality = Self::infer_quality(observation_count, final_loss, delta_e);

        Self {
            source: source.to_string(),
            dataset: dataset.to_string(),
            observation_count,
            final_loss,
            delta_e_2000: delta_e,
            optimizer: optimizer.to_string(),
            iterations,
            quality,
            calibrated_at: current_timestamp(),
            duration_ms: 0,
        }
    }

    /// Infer calibration quality from metrics.
    fn infer_quality(
        observation_count: usize,
        final_loss: f64,
        delta_e: Option<f64>,
    ) -> CalibrationQuality {
        // Reference: ΔE < 1.0, high observation count
        if let Some(de) = delta_e {
            if de < 1.0 && observation_count >= 1000 {
                return CalibrationQuality::Reference;
            }
            if de < 2.0 && observation_count >= 100 {
                return CalibrationQuality::High;
            }
            if de < 5.0 && observation_count >= 10 {
                return CalibrationQuality::Standard;
            }
            if de < 10.0 {
                return CalibrationQuality::Basic;
            }
        }

        // Fallback to loss-based quality
        if final_loss < 0.001 && observation_count >= 100 {
            CalibrationQuality::High
        } else if final_loss < 0.01 && observation_count >= 10 {
            CalibrationQuality::Standard
        } else if final_loss < 0.1 {
            CalibrationQuality::Basic
        } else {
            CalibrationQuality::Uncalibrated
        }
    }

    /// Check if calibration is valid.
    pub fn is_valid(&self) -> bool {
        self.quality != CalibrationQuality::Uncalibrated
    }

    /// Set calibration duration.
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }
}

// ============================================================================
// MATERIAL TWIN
// ============================================================================

/// A Digital Material Twin with full physical specification.
///
/// The twin encapsulates:
/// - A differentiable BSDF model with physical parameters
/// - Unique identity (UUID) for tracking
/// - Content fingerprint for versioning
/// - Spectral identity for material matching
/// - Temporal evolution model (optional)
/// - Calibration metadata with quality metrics
#[derive(Debug, Clone)]
pub struct MaterialTwin<M: DifferentiableBSDF> {
    /// Unique identifier.
    pub id: TwinId,
    /// Content-based fingerprint.
    pub fingerprint: MaterialFingerprint,
    /// Physical BSDF model.
    pub physical: M,
    /// Temporal evolution model (if dynamic).
    pub temporal_model: Option<TemporalEvolution>,
    /// Spectral identity signature.
    pub spectral_identity: SpectralIdentity,
    /// Calibration metadata.
    pub calibration: CalibrationMetadata,
    /// Creation timestamp (Unix nanos).
    pub created_at: u64,
    /// Last modification timestamp.
    pub modified_at: u64,
    /// Twin variant type.
    pub variant: TwinVariant,
    /// User-defined tags.
    pub tags: Vec<String>,
    /// Human-readable name.
    pub name: Option<String>,
}

impl<M: DifferentiableBSDF + Clone> MaterialTwin<M> {
    /// Create a new uncalibrated twin.
    pub fn new(physical: M) -> Self {
        let params = physical.params_to_vec();
        let fingerprint = MaterialFingerprint::from_params(&params);
        let spectral_identity = SpectralIdentity::from_bsdf(&physical);
        let now = current_timestamp();

        Self {
            id: TwinId::generate(),
            fingerprint,
            physical,
            temporal_model: None,
            spectral_identity,
            calibration: CalibrationMetadata::uncalibrated(),
            created_at: now,
            modified_at: now,
            variant: TwinVariant::Static,
            tags: Vec::new(),
            name: None,
        }
    }

    /// Get twin name or short ID.
    pub fn display_name(&self) -> String {
        self.name
            .clone()
            .unwrap_or_else(|| format!("twin-{}", self.id.short()))
    }

    /// Check if twin is calibrated.
    pub fn is_calibrated(&self) -> bool {
        self.calibration.is_valid()
    }

    /// Check if twin has temporal evolution.
    pub fn is_temporal(&self) -> bool {
        self.temporal_model.is_some()
    }

    /// Update fingerprint after parameter changes.
    pub fn update_fingerprint(&mut self) {
        let params = self.physical.params_to_vec();
        self.fingerprint = MaterialFingerprint::from_params(&params);
        self.modified_at = current_timestamp();
    }

    /// Update spectral identity.
    pub fn update_spectral_identity(&mut self) {
        self.spectral_identity = SpectralIdentity::from_bsdf(&self.physical);
        self.modified_at = current_timestamp();
    }

    /// Add a tag.
    pub fn add_tag(&mut self, tag: &str) {
        if !self.tags.contains(&tag.to_string()) {
            self.tags.push(tag.to_string());
            self.modified_at = current_timestamp();
        }
    }

    /// Remove a tag.
    pub fn remove_tag(&mut self, tag: &str) {
        self.tags.retain(|t| t != tag);
        self.modified_at = current_timestamp();
    }

    /// Check if twin has a specific tag.
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// Set name.
    pub fn set_name(&mut self, name: &str) {
        self.name = Some(name.to_string());
        self.modified_at = current_timestamp();
    }

    /// Create derived twin with new physical model.
    pub fn derive(&self, new_physical: M, description: &str) -> Self {
        let params = new_physical.params_to_vec();
        let fingerprint = MaterialFingerprint::from_params(&params);
        let spectral_identity = SpectralIdentity::from_bsdf(&new_physical);
        let now = current_timestamp();

        let mut derived = Self {
            id: TwinId::generate(),
            fingerprint,
            physical: new_physical,
            temporal_model: self.temporal_model.clone(),
            spectral_identity,
            calibration: self.calibration.clone(),
            created_at: now,
            modified_at: now,
            variant: self.variant.clone(),
            tags: self.tags.clone(),
            name: self.name.as_ref().map(|n| format!("{} (derived)", n)),
        };

        derived.add_tag(&format!("derived_from:{}", self.id.short()));
        derived.add_tag(&format!("change:{}", description));

        derived
    }

    /// Get age of twin in seconds.
    pub fn age_seconds(&self) -> f64 {
        let now = current_timestamp();
        (now - self.created_at) as f64 / 1_000_000_000.0
    }

    /// Check if twin was modified since creation.
    pub fn is_modified(&self) -> bool {
        self.modified_at > self.created_at
    }
}

// ============================================================================
// TWIN BUILDER
// ============================================================================

/// Builder for creating MaterialTwin instances.
pub struct TwinBuilder<M: DifferentiableBSDF> {
    physical: M,
    temporal_model: Option<TemporalEvolution>,
    calibration: Option<CalibrationMetadata>,
    variant: TwinVariant,
    tags: Vec<String>,
    name: Option<String>,
}

impl<M: DifferentiableBSDF + Clone> TwinBuilder<M> {
    /// Create builder from physical model.
    pub fn new(physical: M) -> Self {
        Self {
            physical,
            temporal_model: None,
            calibration: None,
            variant: TwinVariant::Static,
            tags: Vec::new(),
            name: None,
        }
    }

    /// Set temporal evolution model.
    pub fn with_temporal(mut self, evolution: TemporalEvolution) -> Self {
        self.temporal_model = Some(evolution);
        self.variant = TwinVariant::Temporal;
        self
    }

    /// Set calibration metadata.
    pub fn with_calibration(mut self, calibration: CalibrationMetadata) -> Self {
        self.calibration = Some(calibration);
        self
    }

    /// Set variant type.
    pub fn with_variant(mut self, variant: TwinVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Add a tag.
    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string());
        self
    }

    /// Add multiple tags.
    pub fn with_tags(mut self, tags: &[&str]) -> Self {
        for tag in tags {
            self.tags.push((*tag).to_string());
        }
        self
    }

    /// Set name.
    pub fn with_name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    /// Mark as measured (from real data).
    pub fn as_measured(mut self, dataset: &str) -> Self {
        self.variant = TwinVariant::Measured;
        self.tags.push(format!("source:{}", dataset));
        self
    }

    /// Build the twin.
    pub fn build(self) -> MaterialTwin<M> {
        let mut twin = MaterialTwin::new(self.physical);

        twin.temporal_model = self.temporal_model;
        twin.variant = self.variant;
        twin.tags = self.tags;
        twin.name = self.name;

        if let Some(cal) = self.calibration {
            twin.calibration = cal;
        }

        twin
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

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::super::differentiable::DifferentiableDielectric;
    use super::*;

    #[test]
    fn test_twin_id_generate() {
        let id1 = TwinId::generate();
        let id2 = TwinId::generate();
        assert_ne!(id1, id2); // Should be unique
    }

    #[test]
    fn test_twin_id_format() {
        let id = TwinId::generate();
        let uuid = id.to_uuid_string();
        assert_eq!(uuid.len(), 36); // Standard UUID format
        assert!(uuid.contains('-'));
    }

    #[test]
    fn test_calibration_quality_score() {
        assert_eq!(CalibrationQuality::Uncalibrated.score(), 0);
        assert_eq!(CalibrationQuality::Reference.score(), 100);
        assert!(CalibrationQuality::High.meets_minimum(CalibrationQuality::Standard));
    }

    #[test]
    fn test_calibration_metadata_default() {
        let meta = CalibrationMetadata::default();
        assert!(!meta.is_valid());
        assert_eq!(meta.quality, CalibrationQuality::Uncalibrated);
    }

    #[test]
    fn test_calibration_metadata_from_calibration() {
        let meta = CalibrationMetadata::from_calibration(
            "MERL",
            "gold",
            1000,
            0.001,
            Some(0.5),
            "Adam",
            500,
        );
        assert!(meta.is_valid());
        assert_eq!(meta.quality, CalibrationQuality::Reference);
    }

    #[test]
    fn test_material_twin_new() {
        let glass = DifferentiableDielectric::glass();
        let twin = MaterialTwin::new(glass);

        assert!(!twin.is_calibrated());
        assert!(!twin.is_temporal());
        assert_eq!(twin.variant, TwinVariant::Static);
    }

    #[test]
    fn test_material_twin_display_name() {
        let glass = DifferentiableDielectric::glass();
        let mut twin = MaterialTwin::new(glass);

        // Default uses short ID
        let default_name = twin.display_name();
        assert!(default_name.starts_with("twin-"));

        // Custom name
        twin.set_name("BK7 Glass");
        assert_eq!(twin.display_name(), "BK7 Glass");
    }

    #[test]
    fn test_material_twin_tags() {
        let glass = DifferentiableDielectric::glass();
        let mut twin = MaterialTwin::new(glass);

        twin.add_tag("calibrated");
        twin.add_tag("optical");
        assert!(twin.has_tag("calibrated"));
        assert!(twin.has_tag("optical"));
        assert!(!twin.has_tag("missing"));

        twin.remove_tag("optical");
        assert!(!twin.has_tag("optical"));
    }

    #[test]
    fn test_material_twin_fingerprint_update() {
        let glass = DifferentiableDielectric::glass();
        let mut twin = MaterialTwin::new(glass.clone());

        let original_fingerprint = twin.fingerprint.clone();

        // Simulate parameter change (in real usage, physical would be modified)
        twin.physical = DifferentiableDielectric::new(1.52, 0.1);
        twin.update_fingerprint();

        assert_ne!(twin.fingerprint.hash, original_fingerprint.hash);
    }

    #[test]
    fn test_material_twin_derive() {
        let glass = DifferentiableDielectric::glass();
        let original = MaterialTwin::new(glass);

        let new_glass = DifferentiableDielectric::new(1.52, 0.05);
        let derived = original.derive(new_glass, "adjusted_ior");

        assert_ne!(original.id, derived.id);
        assert!(derived.has_tag(&format!("derived_from:{}", original.id.short())));
    }

    #[test]
    fn test_twin_builder() {
        let glass = DifferentiableDielectric::glass();

        let twin = TwinBuilder::new(glass)
            .with_name("Test Glass")
            .with_tag("test")
            .with_tags(&["optical", "calibrated"])
            .build();

        assert_eq!(twin.name, Some("Test Glass".to_string()));
        assert!(twin.has_tag("test"));
        assert!(twin.has_tag("optical"));
        assert!(twin.has_tag("calibrated"));
    }

    #[test]
    fn test_twin_builder_with_calibration() {
        let glass = DifferentiableDielectric::glass();

        let calibration = CalibrationMetadata::from_calibration(
            "synthetic",
            "bk7",
            100,
            0.01,
            Some(1.5),
            "Adam",
            200,
        );

        let twin = TwinBuilder::new(glass)
            .with_calibration(calibration)
            .build();

        assert!(twin.is_calibrated());
    }

    #[test]
    fn test_twin_builder_measured() {
        let glass = DifferentiableDielectric::glass();

        let twin = TwinBuilder::new(glass).as_measured("MERL-100").build();

        assert_eq!(twin.variant, TwinVariant::Measured);
        assert!(twin.has_tag("source:MERL-100"));
    }
}

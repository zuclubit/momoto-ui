//! # Material Fingerprint Module
//!
//! Deterministic material identification and versioning for reproducibility.
//!
//! ## Features
//!
//! - **Deterministic Hashing**: Platform-independent material fingerprints
//! - **Version Tracking**: Schema-aware material versioning
//! - **Calibration Logs**: Scientific reproducibility tracking
//! - **Bit-Exact Verification**: Ensures identical outputs
//!
//! ## Usage
//!
//! ```rust
//! use momoto_materials::glass_physics::material_fingerprint::{
//!     MaterialFingerprint, fingerprint_from_params
//! };
//!
//! // Create fingerprint from material parameters
//! let params = [1.5, 0.1, 0.0, 0.5]; // ior, roughness, metallic, etc.
//! let fingerprint = fingerprint_from_params(&params);
//!
//! println!("Hash: {}", fingerprint.to_hex());
//! println!("Schema: v{}", fingerprint.schema_version);
//! ```

use std::collections::VecDeque;

// ============================================================================
// CONSTANTS
// ============================================================================

/// Current schema version for fingerprinting
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Engine version string
pub const ENGINE_VERSION: &str = "momoto-materials-0.8.0";

/// Maximum calibration log entries before rotation
pub const MAX_LOG_ENTRIES: usize = 1000;

// ============================================================================
// MATERIAL FINGERPRINT
// ============================================================================

/// Material fingerprint for reproducibility and identification
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterialFingerprint {
    /// 256-bit content hash
    pub hash: [u8; 32],
    /// Schema version (for migration compatibility)
    pub schema_version: u32,
    /// Engine version that produced this fingerprint
    pub engine_version: String,
    /// Timestamp (optional, for audit trails)
    pub timestamp: Option<u64>,
}

impl MaterialFingerprint {
    /// Create new fingerprint with given hash
    pub fn new(hash: [u8; 32]) -> Self {
        Self {
            hash,
            schema_version: CURRENT_SCHEMA_VERSION,
            engine_version: ENGINE_VERSION.to_string(),
            timestamp: Some(current_timestamp()),
        }
    }

    /// Create fingerprint without timestamp (for deterministic comparison)
    pub fn new_deterministic(hash: [u8; 32]) -> Self {
        Self {
            hash,
            schema_version: CURRENT_SCHEMA_VERSION,
            engine_version: ENGINE_VERSION.to_string(),
            timestamp: None,
        }
    }

    /// Create fingerprint from f64 parameters
    pub fn from_params(params: &[f64]) -> Self {
        let hash = deterministic_hash_f64(params);
        Self::new(hash)
    }

    /// Verify that this fingerprint matches given parameters
    pub fn verify(&self, params: &[f64]) -> bool {
        let computed = deterministic_hash_f64(params);
        self.hash == computed
    }

    /// Convert hash to hex string
    pub fn to_hex(&self) -> String {
        self.hash
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    }

    /// Parse fingerprint from hex string
    pub fn from_hex(hex: &str) -> Result<Self, FingerprintError> {
        if hex.len() != 64 {
            return Err(FingerprintError::InvalidHexLength {
                expected: 64,
                actual: hex.len(),
            });
        }

        let mut hash = [0u8; 32];
        for i in 0..32 {
            let byte_str = &hex[i * 2..i * 2 + 2];
            hash[i] = u8::from_str_radix(byte_str, 16)
                .map_err(|_| FingerprintError::InvalidHexChar(byte_str.to_string()))?;
        }

        Ok(Self {
            hash,
            schema_version: CURRENT_SCHEMA_VERSION,
            engine_version: ENGINE_VERSION.to_string(),
            timestamp: None,
        })
    }

    /// Get short hash (first 8 hex chars)
    pub fn short_hash(&self) -> String {
        self.to_hex()[..8].to_string()
    }

    /// Check if fingerprints are equal (ignoring timestamp)
    pub fn content_equals(&self, other: &Self) -> bool {
        self.hash == other.hash && self.schema_version == other.schema_version
    }

    /// Check schema compatibility
    pub fn is_compatible(&self) -> bool {
        self.schema_version <= CURRENT_SCHEMA_VERSION
    }

    /// Create fingerprint from raw bytes (for testing).
    /// Pads or truncates to 32 bytes.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut hash = [0u8; 32];
        let len = bytes.len().min(32);
        hash[..len].copy_from_slice(&bytes[..len]);
        Self::new_deterministic(hash)
    }
}

impl std::fmt::Display for MaterialFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MaterialFingerprint({}..., v{}, {})",
            &self.to_hex()[..8],
            self.schema_version,
            self.engine_version
        )
    }
}

/// Fingerprint errors
#[derive(Debug, Clone)]
pub enum FingerprintError {
    /// Hex string has wrong length
    InvalidHexLength { expected: usize, actual: usize },
    /// Invalid hex character
    InvalidHexChar(String),
    /// Schema version mismatch
    IncompatibleSchema { expected: u32, actual: u32 },
    /// Verification failed
    VerificationFailed,
}

impl std::fmt::Display for FingerprintError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidHexLength { expected, actual } => {
                write!(
                    f,
                    "Invalid hex length: expected {}, got {}",
                    expected, actual
                )
            }
            Self::InvalidHexChar(s) => write!(f, "Invalid hex character: {}", s),
            Self::IncompatibleSchema { expected, actual } => {
                write!(
                    f,
                    "Incompatible schema: expected v{}, got v{}",
                    expected, actual
                )
            }
            Self::VerificationFailed => write!(f, "Fingerprint verification failed"),
        }
    }
}

impl std::error::Error for FingerprintError {}

// ============================================================================
// MATERIAL VERSION
// ============================================================================

/// Material version with edit history
#[derive(Debug, Clone)]
pub struct MaterialVersion {
    /// Current fingerprint
    pub fingerprint: MaterialFingerprint,
    /// Parent version (if any)
    pub parent: Option<Box<MaterialVersion>>,
    /// Description of change from parent
    pub change_description: Option<String>,
    /// Version number (auto-incremented from parent)
    pub version_number: u32,
}

impl MaterialVersion {
    /// Create initial version
    pub fn initial(fingerprint: MaterialFingerprint) -> Self {
        Self {
            fingerprint,
            parent: None,
            change_description: Some("Initial version".to_string()),
            version_number: 1,
        }
    }

    /// Create new version derived from this one
    pub fn derive(&self, new_fingerprint: MaterialFingerprint, description: &str) -> Self {
        Self {
            fingerprint: new_fingerprint,
            parent: Some(Box::new(self.clone())),
            change_description: Some(description.to_string()),
            version_number: self.version_number + 1,
        }
    }

    /// Get version history as list (oldest first)
    pub fn history(&self) -> Vec<&MaterialVersion> {
        let mut history = Vec::new();
        let mut current = Some(self);

        while let Some(version) = current {
            history.push(version);
            current = version.parent.as_deref();
        }

        history.reverse();
        history
    }

    /// Get depth of version history
    pub fn depth(&self) -> usize {
        self.history().len()
    }
}

// ============================================================================
// CALIBRATION LOG
// ============================================================================

/// Entry in calibration log
#[derive(Debug, Clone)]
pub struct CalibrationEntry {
    /// Timestamp of calibration
    pub timestamp: u64,
    /// Fingerprint before calibration
    pub input_fingerprint: MaterialFingerprint,
    /// Fingerprint after calibration
    pub output_fingerprint: MaterialFingerprint,
    /// Optimizer used
    pub optimizer: String,
    /// Number of iterations
    pub iterations: usize,
    /// Final loss value
    pub final_loss: f64,
    /// Target dataset name
    pub target_dataset: String,
    /// Parameters that were changed
    pub parameters_changed: Vec<String>,
}

impl CalibrationEntry {
    /// Create new calibration entry
    pub fn new(
        input_fingerprint: MaterialFingerprint,
        output_fingerprint: MaterialFingerprint,
        optimizer: &str,
        iterations: usize,
        final_loss: f64,
        target_dataset: &str,
    ) -> Self {
        Self {
            timestamp: current_timestamp(),
            input_fingerprint,
            output_fingerprint,
            optimizer: optimizer.to_string(),
            iterations,
            final_loss,
            target_dataset: target_dataset.to_string(),
            parameters_changed: Vec::new(),
        }
    }

    /// Add changed parameter
    pub fn with_changed_param(mut self, param: &str) -> Self {
        self.parameters_changed.push(param.to_string());
        self
    }

    /// Set changed parameters
    pub fn with_changed_params(mut self, params: Vec<String>) -> Self {
        self.parameters_changed = params;
        self
    }

    /// Check if material changed
    pub fn material_changed(&self) -> bool {
        !self
            .input_fingerprint
            .content_equals(&self.output_fingerprint)
    }

    /// Format as log line
    pub fn to_log_line(&self) -> String {
        format!(
            "[{}] {} -> {} | {} | iters={} loss={:.6} | {}",
            format_timestamp(self.timestamp),
            self.input_fingerprint.short_hash(),
            self.output_fingerprint.short_hash(),
            self.optimizer,
            self.iterations,
            self.final_loss,
            self.target_dataset,
        )
    }
}

/// Calibration log for scientific reproducibility
#[derive(Debug, Clone)]
pub struct CalibrationLog {
    /// Log entries (newest last)
    entries: VecDeque<CalibrationEntry>,
    /// Maximum entries before rotation
    max_entries: usize,
}

impl CalibrationLog {
    /// Create new empty log
    pub fn new() -> Self {
        Self {
            entries: VecDeque::new(),
            max_entries: MAX_LOG_ENTRIES,
        }
    }

    /// Create log with custom max entries
    pub fn with_capacity(max_entries: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(max_entries),
            max_entries,
        }
    }

    /// Add entry to log
    pub fn add_entry(&mut self, entry: CalibrationEntry) {
        if self.entries.len() >= self.max_entries {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    /// Get all entries
    pub fn entries(&self) -> &VecDeque<CalibrationEntry> {
        &self.entries
    }

    /// Get number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if log is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get entry by index
    pub fn get(&self, index: usize) -> Option<&CalibrationEntry> {
        self.entries.get(index)
    }

    /// Get latest entry
    pub fn latest(&self) -> Option<&CalibrationEntry> {
        self.entries.back()
    }

    /// Find entries by fingerprint
    pub fn find_by_fingerprint(&self, fingerprint: &MaterialFingerprint) -> Vec<&CalibrationEntry> {
        self.entries
            .iter()
            .filter(|e| {
                e.input_fingerprint.content_equals(fingerprint)
                    || e.output_fingerprint.content_equals(fingerprint)
            })
            .collect()
    }

    /// Clear log
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Convert to JSON string (simple format)
    pub fn to_json(&self) -> String {
        let mut json = String::from("{\n  \"calibration_log\": [\n");

        for (i, entry) in self.entries.iter().enumerate() {
            json.push_str("    {\n");
            json.push_str(&format!("      \"timestamp\": {},\n", entry.timestamp));
            json.push_str(&format!(
                "      \"input_hash\": \"{}\",\n",
                entry.input_fingerprint.to_hex()
            ));
            json.push_str(&format!(
                "      \"output_hash\": \"{}\",\n",
                entry.output_fingerprint.to_hex()
            ));
            json.push_str(&format!("      \"optimizer\": \"{}\",\n", entry.optimizer));
            json.push_str(&format!("      \"iterations\": {},\n", entry.iterations));
            json.push_str(&format!("      \"final_loss\": {},\n", entry.final_loss));
            json.push_str(&format!(
                "      \"target_dataset\": \"{}\"\n",
                entry.target_dataset
            ));
            json.push_str("    }");

            if i < self.entries.len() - 1 {
                json.push(',');
            }
            json.push('\n');
        }

        json.push_str("  ]\n}");
        json
    }

    /// Format as human-readable log
    pub fn to_log_format(&self) -> String {
        let mut log = String::from("=== Calibration Log ===\n");
        log.push_str(&format!("Entries: {}\n\n", self.entries.len()));

        for entry in &self.entries {
            log.push_str(&entry.to_log_line());
            log.push('\n');
        }

        log
    }
}

impl Default for CalibrationLog {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// HASHING FUNCTIONS
// ============================================================================

/// Deterministic hash of f64 array (platform-independent)
pub fn deterministic_hash_f64(data: &[f64]) -> [u8; 32] {
    // Convert f64 to canonical byte representation
    let bytes: Vec<u8> = data.iter().flat_map(|&f| f.to_be_bytes()).collect();

    deterministic_hash(&bytes)
}

/// Deterministic hash of byte array using simple hash function
///
/// Note: This is a simple hash for fingerprinting, not cryptographic security.
/// Uses a variation of FNV-1a extended to 256 bits.
pub fn deterministic_hash(data: &[u8]) -> [u8; 32] {
    // FNV-1a style hash extended to 256 bits
    // Using 4 independent 64-bit hashes with different offsets

    const FNV_PRIME: u64 = 0x100000001b3;
    const FNV_OFFSET_BASIS: [u64; 4] = [
        0xcbf29ce484222325,
        0x84222325cbf29ce4,
        0x22325cbf29ce4842,
        0x25cbf29ce4842232,
    ];

    let mut state: [u64; 4] = FNV_OFFSET_BASIS;

    for (i, &byte) in data.iter().enumerate() {
        // Mix byte into all 4 hash states with different rotations
        for j in 0..4 {
            state[j] ^= byte.wrapping_add((i as u8).wrapping_mul(j as u8 + 1)) as u64;
            state[j] = state[j].wrapping_mul(FNV_PRIME);
        }

        // Cross-mix states for better avalanche
        if i % 8 == 7 {
            let temp = state[0];
            state[0] ^= state[1].rotate_left(13);
            state[1] ^= state[2].rotate_left(17);
            state[2] ^= state[3].rotate_left(19);
            state[3] ^= temp.rotate_left(23);
        }
    }

    // Final mixing
    for i in 0..4 {
        state[i] ^= state[i] >> 33;
        state[i] = state[i].wrapping_mul(0xff51afd7ed558ccd);
        state[i] ^= state[i] >> 33;
    }

    // Convert to bytes
    let mut result = [0u8; 32];
    for i in 0..4 {
        result[i * 8..(i + 1) * 8].copy_from_slice(&state[i].to_be_bytes());
    }

    result
}

/// Hash string for fingerprinting
pub fn hash_string(s: &str) -> [u8; 32] {
    deterministic_hash(s.as_bytes())
}

// ============================================================================
// BIT-EXACT VERIFICATION
// ============================================================================

/// Verify bit-exact equality of two f64 arrays
pub fn verify_bit_exact(a: &[f64], b: &[f64]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    for (x, y) in a.iter().zip(b.iter()) {
        if x.to_bits() != y.to_bits() {
            return false;
        }
    }

    true
}

/// Verify bit-exact equality with tolerance for floating-point operations
pub fn verify_near_exact(a: &[f64], b: &[f64], ulp_tolerance: u64) -> bool {
    if a.len() != b.len() {
        return false;
    }

    for (x, y) in a.iter().zip(b.iter()) {
        let bits_a = x.to_bits();
        let bits_b = y.to_bits();

        // Handle NaN
        if x.is_nan() && y.is_nan() {
            continue;
        }

        // Handle different signs
        if (bits_a >> 63) != (bits_b >> 63) {
            if *x != 0.0 || *y != 0.0 {
                return false;
            }
            continue;
        }

        // Compare ULP distance
        let ulp_diff = if bits_a > bits_b {
            bits_a - bits_b
        } else {
            bits_b - bits_a
        };

        if ulp_diff > ulp_tolerance {
            return false;
        }
    }

    true
}

/// Count bit differences between two f64 values
pub fn bit_difference(a: f64, b: f64) -> u32 {
    (a.to_bits() ^ b.to_bits()).count_ones()
}

// ============================================================================
// CONVENIENCE FUNCTIONS
// ============================================================================

/// Create fingerprint from f64 parameters
pub fn fingerprint_from_params(params: &[f64]) -> MaterialFingerprint {
    MaterialFingerprint::from_params(params)
}

/// Create fingerprint from named parameters
pub fn fingerprint_from_named(params: &[(&str, f64)]) -> MaterialFingerprint {
    // Sort by name for determinism, then hash
    let mut sorted: Vec<_> = params.to_vec();
    sorted.sort_by(|a, b| a.0.cmp(b.0));

    // Combine names and values into single byte sequence
    let mut bytes = Vec::new();
    for (name, value) in sorted {
        bytes.extend_from_slice(name.as_bytes());
        bytes.push(0); // null separator
        bytes.extend_from_slice(&value.to_be_bytes());
    }

    MaterialFingerprint::new(deterministic_hash(&bytes))
}

/// Get current timestamp in nanoseconds
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

/// Format timestamp as ISO 8601 (approximate)
fn format_timestamp(ns: u64) -> String {
    let secs = ns / 1_000_000_000;
    let mins = secs / 60;
    let hours = mins / 60;
    let days = hours / 24;

    // Very approximate - just for log readability
    format!(
        "{}d {:02}:{:02}:{:02}",
        days,
        hours % 24,
        mins % 60,
        secs % 60
    )
}

// ============================================================================
// MEMORY ESTIMATION
// ============================================================================

/// Estimate memory usage for fingerprint module
pub fn total_fingerprint_memory() -> usize {
    // MaterialFingerprint: 32 + 4 + ~24 + 8 = ~68 bytes
    // CalibrationEntry: ~200 bytes
    // CalibrationLog (1000 entries): ~200KB
    // For typical usage (100 entries): ~20KB
    // Base overhead: ~1KB
    1024 + 100 * 200
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fingerprint_creation() {
        let params = vec![1.5, 0.1, 0.0, 0.5];
        let fp = MaterialFingerprint::from_params(&params);

        assert_eq!(fp.hash.len(), 32);
        assert_eq!(fp.schema_version, CURRENT_SCHEMA_VERSION);
        assert!(!fp.engine_version.is_empty());
    }

    #[test]
    fn test_fingerprint_determinism() {
        let params = vec![1.5, 0.1, 0.0, 0.5];

        let fp1 = MaterialFingerprint::new_deterministic(deterministic_hash_f64(&params));
        let fp2 = MaterialFingerprint::new_deterministic(deterministic_hash_f64(&params));

        assert_eq!(fp1.hash, fp2.hash);
        assert!(fp1.content_equals(&fp2));
    }

    #[test]
    fn test_fingerprint_uniqueness() {
        let params1 = vec![1.5, 0.1, 0.0, 0.5];
        let params2 = vec![1.5, 0.1, 0.0, 0.500001];

        let fp1 = MaterialFingerprint::from_params(&params1);
        let fp2 = MaterialFingerprint::from_params(&params2);

        assert_ne!(fp1.hash, fp2.hash);
    }

    #[test]
    fn test_hex_roundtrip() {
        let params = vec![1.5, 0.1, 0.0, 0.5];
        let fp = MaterialFingerprint::from_params(&params);

        let hex = fp.to_hex();
        assert_eq!(hex.len(), 64);

        let fp2 = MaterialFingerprint::from_hex(&hex).unwrap();
        assert_eq!(fp.hash, fp2.hash);
    }

    #[test]
    fn test_hex_invalid() {
        assert!(MaterialFingerprint::from_hex("abc").is_err());
        assert!(MaterialFingerprint::from_hex(
            "gg00000000000000000000000000000000000000000000000000000000000000"
        )
        .is_err());
    }

    #[test]
    fn test_verification() {
        let params = vec![1.5, 0.1, 0.0, 0.5];
        let fp = MaterialFingerprint::from_params(&params);

        assert!(fp.verify(&params));
        assert!(!fp.verify(&vec![1.5, 0.2, 0.0, 0.5]));
    }

    #[test]
    fn test_material_version() {
        let params1 = vec![1.5, 0.1, 0.0, 0.5];
        let fp1 = MaterialFingerprint::from_params(&params1);
        let v1 = MaterialVersion::initial(fp1);

        assert_eq!(v1.version_number, 1);
        assert!(v1.parent.is_none());

        let params2 = vec![1.5, 0.15, 0.0, 0.5];
        let fp2 = MaterialFingerprint::from_params(&params2);
        let v2 = v1.derive(fp2, "Increased roughness");

        assert_eq!(v2.version_number, 2);
        assert!(v2.parent.is_some());
        assert_eq!(v2.history().len(), 2);
    }

    #[test]
    fn test_calibration_log() {
        let mut log = CalibrationLog::new();

        let fp_in = MaterialFingerprint::from_params(&vec![1.5, 0.1]);
        let fp_out = MaterialFingerprint::from_params(&vec![1.52, 0.12]);

        let entry = CalibrationEntry::new(fp_in, fp_out, "Adam", 100, 0.001, "BK7");

        log.add_entry(entry);

        assert_eq!(log.len(), 1);
        assert!(log.latest().is_some());
    }

    #[test]
    fn test_calibration_log_rotation() {
        let mut log = CalibrationLog::with_capacity(3);

        for i in 0..5 {
            let fp = MaterialFingerprint::from_params(&vec![i as f64]);
            let entry = CalibrationEntry::new(fp.clone(), fp, "SGD", i, 0.1, "Test");
            log.add_entry(entry);
        }

        // Should only keep last 3
        assert_eq!(log.len(), 3);
        assert_eq!(log.get(0).unwrap().iterations, 2);
    }

    #[test]
    fn test_bit_exact_verification() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        let c = vec![1.0, 2.0000000001, 3.0];

        assert!(verify_bit_exact(&a, &b));
        assert!(!verify_bit_exact(&a, &c));
    }

    #[test]
    fn test_near_exact_verification() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];

        // Should pass with any tolerance
        assert!(verify_near_exact(&a, &b, 0));
        assert!(verify_near_exact(&a, &b, 100));
    }

    #[test]
    fn test_deterministic_hash_consistency() {
        let data = b"test data for hashing";

        let hash1 = deterministic_hash(data);
        let hash2 = deterministic_hash(data);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_named_params_fingerprint() {
        let fp1 = fingerprint_from_named(&[("ior", 1.5), ("roughness", 0.1)]);

        // Order shouldn't matter
        let fp2 = fingerprint_from_named(&[("roughness", 0.1), ("ior", 1.5)]);

        assert_eq!(fp1.hash, fp2.hash);
    }

    #[test]
    fn test_calibration_log_json() {
        let mut log = CalibrationLog::new();
        let fp = MaterialFingerprint::from_params(&vec![1.5]);
        let entry = CalibrationEntry::new(fp.clone(), fp, "Adam", 50, 0.01, "Test");
        log.add_entry(entry);

        let json = log.to_json();
        assert!(json.contains("calibration_log"));
        assert!(json.contains("Adam"));
    }

    #[test]
    fn test_memory_estimate() {
        let mem = total_fingerprint_memory();
        assert!(mem > 0);
        assert!(mem < 100_000); // Should be under 100KB
    }
}

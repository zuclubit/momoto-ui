//! # Spectral Identity
//!
//! Unique spectral signatures for material identification and matching.

use super::super::differentiable::DifferentiableBSDF;
use super::super::unified_bsdf::{BSDFContext, Vector3};

// ============================================================================
// CONSTANTS
// ============================================================================

/// Standard wavelengths for identity computation (nm).
/// Covers visible spectrum from 380nm to 780nm in 13nm steps (31 samples).
pub const IDENTITY_WAVELENGTHS: [f64; 31] = [
    380.0, 393.0, 406.0, 419.0, 432.0, 445.0, 458.0, 471.0, 484.0, 497.0, 510.0, 523.0, 536.0,
    549.0, 562.0, 575.0, 588.0, 601.0, 614.0, 627.0, 640.0, 653.0, 666.0, 679.0, 692.0, 705.0,
    718.0, 731.0, 744.0, 757.0, 770.0,
];

/// Number of standard angles for identity (θ from 0° to 85° in 5° steps).
pub const IDENTITY_ANGLES: usize = 18;

/// Tolerance for spectral identity matching.
pub const IDENTITY_TOLERANCE: f64 = 0.05;

// ============================================================================
// SPECTRAL SIGNATURE
// ============================================================================

/// Compact spectral signature at a single angle.
#[derive(Debug, Clone)]
pub struct SpectralSignature {
    /// Reflectance at each wavelength.
    pub reflectance: [f64; 31],
    /// Transmittance at each wavelength.
    pub transmittance: [f64; 31],
    /// Incidence angle (degrees).
    pub angle_deg: f64,
}

impl SpectralSignature {
    /// Create new signature.
    pub fn new(reflectance: [f64; 31], transmittance: [f64; 31], angle_deg: f64) -> Self {
        Self {
            reflectance,
            transmittance,
            angle_deg,
        }
    }

    /// Create zero signature.
    pub fn zero(angle_deg: f64) -> Self {
        Self {
            reflectance: [0.0; 31],
            transmittance: [0.0; 31],
            angle_deg,
        }
    }

    /// Compute average reflectance.
    pub fn avg_reflectance(&self) -> f64 {
        self.reflectance.iter().sum::<f64>() / 31.0
    }

    /// Compute average transmittance.
    pub fn avg_transmittance(&self) -> f64 {
        self.transmittance.iter().sum::<f64>() / 31.0
    }

    /// Compute spectral distance to another signature (Euclidean).
    pub fn distance(&self, other: &SpectralSignature) -> f64 {
        let mut sum_sq = 0.0;
        for i in 0..31 {
            let dr = self.reflectance[i] - other.reflectance[i];
            let dt = self.transmittance[i] - other.transmittance[i];
            sum_sq += dr * dr + dt * dt;
        }
        sum_sq.sqrt()
    }

    /// Compute spectral angle (cosine similarity).
    pub fn spectral_angle(&self, other: &SpectralSignature) -> f64 {
        let mut dot = 0.0;
        let mut norm_a = 0.0;
        let mut norm_b = 0.0;

        for i in 0..31 {
            dot += self.reflectance[i] * other.reflectance[i];
            dot += self.transmittance[i] * other.transmittance[i];
            norm_a += self.reflectance[i] * self.reflectance[i];
            norm_a += self.transmittance[i] * self.transmittance[i];
            norm_b += other.reflectance[i] * other.reflectance[i];
            norm_b += other.transmittance[i] * other.transmittance[i];
        }

        if norm_a < 1e-10 || norm_b < 1e-10 {
            return std::f64::consts::FRAC_PI_2; // 90 degrees
        }

        let cos_angle = dot / (norm_a.sqrt() * norm_b.sqrt());
        cos_angle.clamp(-1.0, 1.0).acos()
    }

    /// Get dominant wavelength (highest reflectance).
    pub fn dominant_wavelength(&self) -> f64 {
        let (max_idx, _) = self
            .reflectance
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .unwrap();
        IDENTITY_WAVELENGTHS[max_idx]
    }
}

// ============================================================================
// SPECTRAL IDENTITY
// ============================================================================

/// Complete spectral identity for a material.
///
/// Contains spectral signatures at multiple angles for robust
/// material identification and matching.
#[derive(Debug, Clone)]
pub struct SpectralIdentity {
    /// Signatures at different angles.
    pub signatures: Vec<SpectralSignature>,
    /// Compact hash for quick comparison.
    pub hash: [u8; 16],
    /// F0 (normal incidence reflectance) for matching.
    pub f0: f64,
    /// Characteristic angle (Brewster or peak absorption).
    pub characteristic_angle: Option<f64>,
    /// Dominant wavelength (nm).
    pub dominant_wavelength: f64,
    /// Spectral variance (measure of dispersion).
    pub spectral_variance: f64,
}

impl SpectralIdentity {
    /// Create identity from BSDF at standard angles.
    pub fn from_bsdf<B: DifferentiableBSDF>(bsdf: &B) -> Self {
        let mut signatures = Vec::with_capacity(IDENTITY_ANGLES);
        let mut f0 = 0.0;

        for i in 0..IDENTITY_ANGLES {
            let angle_deg = (i as f64) * 5.0;
            let angle_rad = angle_deg.to_radians();
            let cos_theta = angle_rad.cos();
            let sin_theta = angle_rad.sin();

            // Create context for this angle
            let ctx = BSDFContext {
                wi: Vector3::new(sin_theta, 0.0, cos_theta),
                wo: Vector3::new(-sin_theta, 0.0, cos_theta),
                normal: Vector3::new(0.0, 0.0, 1.0),
                tangent: Vector3::new(1.0, 0.0, 0.0),
                bitangent: Vector3::new(0.0, 1.0, 0.0),
                wavelength: 550.0,
                wavelengths: None,
            };

            // Evaluate at each wavelength
            let mut reflectance = [0.0; 31];
            let mut transmittance = [0.0; 31];

            for (j, &wavelength) in IDENTITY_WAVELENGTHS.iter().enumerate() {
                let ctx_wl = BSDFContext {
                    wavelength,
                    ..ctx.clone()
                };
                let response = bsdf.evaluate(&ctx_wl);
                reflectance[j] = response.reflectance;
                transmittance[j] = response.transmittance;
            }

            let sig = SpectralSignature::new(reflectance, transmittance, angle_deg);

            // Capture F0 at normal incidence
            if i == 0 {
                f0 = sig.avg_reflectance();
            }

            signatures.push(sig);
        }

        // Compute hash from signatures
        let hash = Self::compute_hash(&signatures);

        // Find dominant wavelength from normal incidence
        let dominant_wavelength = signatures
            .get(0)
            .map(|s| s.dominant_wavelength())
            .unwrap_or(550.0);

        // Compute spectral variance
        let spectral_variance = Self::compute_variance(&signatures);

        // Find characteristic angle (max gradient in reflectance)
        let characteristic_angle = Self::find_characteristic_angle(&signatures);

        Self {
            signatures,
            hash,
            f0,
            characteristic_angle,
            dominant_wavelength,
            spectral_variance,
        }
    }

    /// Create identity from single normal-incidence measurement.
    pub fn from_normal_incidence(reflectance: [f64; 31], transmittance: [f64; 31]) -> Self {
        let sig = SpectralSignature::new(reflectance, transmittance, 0.0);
        let f0 = sig.avg_reflectance();
        let dominant_wavelength = sig.dominant_wavelength();

        let signatures = vec![sig];
        let hash = Self::compute_hash(&signatures);

        Self {
            signatures,
            hash,
            f0,
            characteristic_angle: None,
            dominant_wavelength,
            spectral_variance: 0.0,
        }
    }

    /// Compute compact hash from signatures.
    fn compute_hash(signatures: &[SpectralSignature]) -> [u8; 16] {
        // Simple hash based on key values
        let mut bytes = Vec::new();

        for sig in signatures {
            // Sample key wavelengths (blue, green, red)
            bytes.extend_from_slice(&sig.reflectance[5].to_be_bytes()); // ~445nm
            bytes.extend_from_slice(&sig.reflectance[15].to_be_bytes()); // ~575nm
            bytes.extend_from_slice(&sig.reflectance[25].to_be_bytes()); // ~705nm
        }

        // FNV-1a hash to 128 bits
        let mut hash = [0u8; 16];
        let mut state: u64 = 0xcbf29ce484222325;
        let prime: u64 = 0x100000001b3;

        for (i, &byte) in bytes.iter().enumerate() {
            state ^= byte as u64;
            state = state.wrapping_mul(prime);

            if i % 8 == 7 {
                let idx = (i / 8) % 16;
                hash[idx] ^= (state >> 32) as u8;
            }
        }

        hash
    }

    /// Compute spectral variance across angles.
    fn compute_variance(signatures: &[SpectralSignature]) -> f64 {
        if signatures.len() < 2 {
            return 0.0;
        }

        // Variance of average reflectance across angles
        let values: Vec<f64> = signatures.iter().map(|s| s.avg_reflectance()).collect();

        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance =
            values.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / values.len() as f64;

        variance
    }

    /// Find characteristic angle (max reflectance gradient).
    fn find_characteristic_angle(signatures: &[SpectralSignature]) -> Option<f64> {
        if signatures.len() < 2 {
            return None;
        }

        let mut max_gradient = 0.0;
        let mut char_angle = None;

        for i in 1..signatures.len() {
            let r_prev = signatures[i - 1].avg_reflectance();
            let r_curr = signatures[i].avg_reflectance();
            let gradient = (r_curr - r_prev).abs();

            if gradient > max_gradient {
                max_gradient = gradient;
                char_angle = Some(signatures[i].angle_deg);
            }
        }

        if max_gradient > 0.01 {
            char_angle
        } else {
            None
        }
    }

    /// Get signature at normal incidence.
    pub fn normal_incidence(&self) -> Option<&SpectralSignature> {
        self.signatures.get(0)
    }

    /// Get signature nearest to given angle.
    pub fn at_angle(&self, angle_deg: f64) -> Option<&SpectralSignature> {
        self.signatures.iter().min_by(|a, b| {
            let diff_a = (a.angle_deg - angle_deg).abs();
            let diff_b = (b.angle_deg - angle_deg).abs();
            diff_a.partial_cmp(&diff_b).unwrap()
        })
    }

    /// Check if hash matches (quick comparison).
    pub fn hash_matches(&self, other: &SpectralIdentity) -> bool {
        self.hash == other.hash
    }

    /// Get number of angles in identity.
    pub fn angle_count(&self) -> usize {
        self.signatures.len()
    }

    /// Format hash as hex string.
    pub fn hash_hex(&self) -> String {
        self.hash.iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// Short hash (first 8 hex chars).
    pub fn short_hash(&self) -> String {
        self.hash_hex()[..8].to_string()
    }
}

// ============================================================================
// SPECTRAL DISTANCE
// ============================================================================

/// Distance metrics between spectral identities.
#[derive(Debug, Clone)]
pub struct SpectralDistance {
    /// Euclidean distance (signature space).
    pub euclidean: f64,
    /// Spectral angle mapper (SAM) distance.
    pub sam: f64,
    /// F0 difference (absolute).
    pub f0_diff: f64,
    /// Maximum per-wavelength difference.
    pub max_diff: f64,
    /// Overall similarity score (0-1).
    pub similarity: f64,
}

impl SpectralDistance {
    /// Create zero distance (identical materials).
    pub fn zero() -> Self {
        Self {
            euclidean: 0.0,
            sam: 0.0,
            f0_diff: 0.0,
            max_diff: 0.0,
            similarity: 1.0,
        }
    }

    /// Check if materials are considered identical.
    pub fn is_identical(&self) -> bool {
        self.euclidean < IDENTITY_TOLERANCE
    }

    /// Check if materials are similar.
    pub fn is_similar(&self) -> bool {
        self.similarity > 0.9
    }
}

/// Compute distance between two spectral identities.
pub fn compute_spectral_distance(a: &SpectralIdentity, b: &SpectralIdentity) -> SpectralDistance {
    // Quick check with hash
    if a.hash == b.hash {
        return SpectralDistance::zero();
    }

    let mut total_dist: f64 = 0.0;
    let mut total_sam: f64 = 0.0;
    let mut max_diff: f64 = 0.0;
    let mut count = 0;

    // Compare at matching angles
    for sig_a in &a.signatures {
        if let Some(sig_b) = b
            .signatures
            .iter()
            .find(|s| (s.angle_deg - sig_a.angle_deg).abs() < 1.0)
        {
            let dist = sig_a.distance(sig_b);
            let sam = sig_a.spectral_angle(sig_b);

            total_dist += dist;
            total_sam += sam;

            // Track max per-wavelength diff
            for i in 0..31 {
                let diff = (sig_a.reflectance[i] - sig_b.reflectance[i]).abs();
                max_diff = max_diff.max(diff);
            }

            count += 1;
        }
    }

    let avg_dist = if count > 0 {
        total_dist / count as f64
    } else {
        1.0
    };
    let avg_sam = if count > 0 {
        total_sam / count as f64
    } else {
        std::f64::consts::FRAC_PI_2
    };
    let f0_diff = (a.f0 - b.f0).abs();

    // Compute similarity (0-1)
    let dist_factor = (-avg_dist * 10.0).exp();
    let sam_factor = (std::f64::consts::FRAC_PI_2 - avg_sam) / std::f64::consts::FRAC_PI_2;
    let f0_factor = 1.0 - f0_diff.min(1.0);
    let similarity = (dist_factor + sam_factor + f0_factor) / 3.0;

    SpectralDistance {
        euclidean: avg_dist,
        sam: avg_sam,
        f0_diff,
        max_diff,
        similarity: similarity.clamp(0.0, 1.0),
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::super::differentiable::DifferentiableDielectric;
    use super::*;

    #[test]
    fn test_identity_wavelengths() {
        assert_eq!(IDENTITY_WAVELENGTHS.len(), 31);
        assert!((IDENTITY_WAVELENGTHS[0] - 380.0).abs() < 0.01);
        assert!((IDENTITY_WAVELENGTHS[30] - 770.0).abs() < 0.01);
    }

    #[test]
    fn test_spectral_signature_zero() {
        let sig = SpectralSignature::zero(45.0);
        assert_eq!(sig.angle_deg, 45.0);
        assert!((sig.avg_reflectance() - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_spectral_signature_distance() {
        let sig1 = SpectralSignature::new([0.1; 31], [0.8; 31], 0.0);
        let sig2 = SpectralSignature::new([0.2; 31], [0.7; 31], 0.0);

        let dist = sig1.distance(&sig2);
        assert!(dist > 0.0);

        // Same signature should have zero distance
        let same_dist = sig1.distance(&sig1);
        assert!(same_dist < 0.001);
    }

    #[test]
    fn test_spectral_identity_from_bsdf() {
        let glass = DifferentiableDielectric::glass();
        let identity = SpectralIdentity::from_bsdf(&glass);

        assert_eq!(identity.angle_count(), IDENTITY_ANGLES);
        assert!(identity.f0 > 0.0);
        assert!(identity.f0 < 0.1); // Glass has low F0
    }

    #[test]
    fn test_spectral_identity_hash() {
        let glass1 = DifferentiableDielectric::new(1.5, 0.0);
        let glass2 = DifferentiableDielectric::new(1.5, 0.0);
        let glass3 = DifferentiableDielectric::new(1.7, 0.1);

        let id1 = SpectralIdentity::from_bsdf(&glass1);
        let id2 = SpectralIdentity::from_bsdf(&glass2);
        let id3 = SpectralIdentity::from_bsdf(&glass3);

        // Same material should have same hash
        assert!(id1.hash_matches(&id2));

        // Different material should have different hash
        assert!(!id1.hash_matches(&id3));
    }

    #[test]
    fn test_spectral_identity_normal_incidence() {
        let glass = DifferentiableDielectric::glass();
        let identity = SpectralIdentity::from_bsdf(&glass);

        let normal = identity.normal_incidence();
        assert!(normal.is_some());
        assert!((normal.unwrap().angle_deg - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_spectral_identity_at_angle() {
        let glass = DifferentiableDielectric::glass();
        let identity = SpectralIdentity::from_bsdf(&glass);

        let at_45 = identity.at_angle(45.0);
        assert!(at_45.is_some());
        assert!((at_45.unwrap().angle_deg - 45.0).abs() < 1.0);
    }

    #[test]
    fn test_spectral_distance_identical() {
        let glass = DifferentiableDielectric::glass();
        let id1 = SpectralIdentity::from_bsdf(&glass);
        let id2 = SpectralIdentity::from_bsdf(&glass);

        let dist = compute_spectral_distance(&id1, &id2);
        assert!(dist.is_identical());
        assert!((dist.similarity - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_spectral_distance_different() {
        let glass = DifferentiableDielectric::glass();
        let diamond = DifferentiableDielectric::diamond();

        let id1 = SpectralIdentity::from_bsdf(&glass);
        let id2 = SpectralIdentity::from_bsdf(&diamond);

        let dist = compute_spectral_distance(&id1, &id2);
        assert!(!dist.is_identical());
        assert!(dist.f0_diff > 0.01); // Diamond has higher F0
    }

    #[test]
    fn test_dominant_wavelength() {
        let mut reflectance = [0.0; 31];
        reflectance[15] = 1.0; // Peak at ~575nm (yellow-green)

        let sig = SpectralSignature::new(reflectance, [0.0; 31], 0.0);
        let dominant = sig.dominant_wavelength();

        assert!((dominant - 575.0).abs() < 1.0);
    }

    #[test]
    fn test_spectral_identity_short_hash() {
        let glass = DifferentiableDielectric::glass();
        let identity = SpectralIdentity::from_bsdf(&glass);

        let short = identity.short_hash();
        assert_eq!(short.len(), 8);
        assert!(short.chars().all(|c| c.is_ascii_hexdigit()));
    }
}

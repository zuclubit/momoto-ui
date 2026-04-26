//! # Flicker Validation
//!
//! Frame-to-frame spectral flicker detection using ΔE2000.
//!
//! ## Key Features
//!
//! - **ΔE2000 Computation**: Perceptual color difference metric
//! - **Flicker Detection**: Frame-to-frame variation tracking
//! - **Violation Logging**: Track and report flicker events

use super::packet::SpectralPacket;

// ============================================================================
// FLICKER STATUS
// ============================================================================

/// Status of flicker validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlickerStatus {
    /// No flicker detected, stable.
    Stable,
    /// Minor flicker, within tolerance.
    Minor,
    /// Warning level flicker.
    Warning,
    /// Exceeded flicker threshold.
    Exceeded,
}

impl FlickerStatus {
    /// Check if status is acceptable.
    pub fn is_acceptable(&self) -> bool {
        matches!(self, FlickerStatus::Stable | FlickerStatus::Minor)
    }

    /// Get status from ΔE value.
    pub fn from_delta_e(delta_e: f64, config: &FlickerConfig) -> Self {
        if delta_e <= config.stable_threshold {
            FlickerStatus::Stable
        } else if delta_e <= config.minor_threshold {
            FlickerStatus::Minor
        } else if delta_e <= config.warning_threshold {
            FlickerStatus::Warning
        } else {
            FlickerStatus::Exceeded
        }
    }
}

// ============================================================================
// FLICKER CONFIGURATION
// ============================================================================

/// Configuration for flicker validation.
#[derive(Debug, Clone)]
pub struct FlickerConfig {
    /// Maximum ΔE2000 for stable (no flicker).
    pub stable_threshold: f64,
    /// Maximum ΔE2000 for minor flicker.
    pub minor_threshold: f64,
    /// Maximum ΔE2000 for warning level.
    pub warning_threshold: f64,
    /// History size for averaging.
    pub history_size: usize,
    /// Log violations.
    pub log_violations: bool,
}

impl Default for FlickerConfig {
    fn default() -> Self {
        Self {
            stable_threshold: 0.2,  // Imperceptible
            minor_threshold: 0.5,   // Just noticeable
            warning_threshold: 1.0, // Noticeable
            history_size: 10,
            log_violations: true,
        }
    }
}

impl FlickerConfig {
    /// Strict configuration for high quality.
    pub fn strict() -> Self {
        Self {
            stable_threshold: 0.1,
            minor_threshold: 0.3,
            warning_threshold: 0.5,
            ..Default::default()
        }
    }

    /// Relaxed configuration for performance.
    pub fn relaxed() -> Self {
        Self {
            stable_threshold: 0.5,
            minor_threshold: 1.0,
            warning_threshold: 2.0,
            ..Default::default()
        }
    }
}

// ============================================================================
// FRAME COMPARISON
// ============================================================================

/// Result of comparing two frames.
#[derive(Debug, Clone)]
pub struct FrameComparison {
    /// Frame indices compared.
    pub frame_a: u64,
    pub frame_b: u64,
    /// ΔE2000 value.
    pub delta_e: f64,
    /// Status.
    pub status: FlickerStatus,
    /// Maximum per-wavelength difference.
    pub max_spectral_diff: f64,
    /// Average per-wavelength difference.
    pub avg_spectral_diff: f64,
}

impl FrameComparison {
    /// Create new comparison.
    pub fn new(
        frame_a: u64,
        frame_b: u64,
        delta_e: f64,
        max_spectral_diff: f64,
        avg_spectral_diff: f64,
        config: &FlickerConfig,
    ) -> Self {
        Self {
            frame_a,
            frame_b,
            delta_e,
            status: FlickerStatus::from_delta_e(delta_e, config),
            max_spectral_diff,
            avg_spectral_diff,
        }
    }
}

// ============================================================================
// FLICKER REPORT
// ============================================================================

/// Summary report of flicker validation.
#[derive(Debug, Clone)]
pub struct FlickerReport {
    /// Total frames analyzed.
    pub total_frames: u64,
    /// Stable frames.
    pub stable_count: u64,
    /// Minor flicker frames.
    pub minor_count: u64,
    /// Warning frames.
    pub warning_count: u64,
    /// Exceeded frames.
    pub exceeded_count: u64,
    /// Maximum ΔE observed.
    pub max_delta_e: f64,
    /// Average ΔE.
    pub avg_delta_e: f64,
    /// Violations log.
    pub violations: Vec<FrameComparison>,
}

impl Default for FlickerReport {
    fn default() -> Self {
        Self {
            total_frames: 0,
            stable_count: 0,
            minor_count: 0,
            warning_count: 0,
            exceeded_count: 0,
            max_delta_e: 0.0,
            avg_delta_e: 0.0,
            violations: Vec::new(),
        }
    }
}

impl FlickerReport {
    /// Get pass rate (stable + minor).
    pub fn pass_rate(&self) -> f64 {
        if self.total_frames == 0 {
            1.0
        } else {
            (self.stable_count + self.minor_count) as f64 / self.total_frames as f64
        }
    }

    /// Check if validation passed.
    pub fn passed(&self) -> bool {
        self.exceeded_count == 0
    }

    /// Add a comparison result.
    pub fn add_comparison(&mut self, comparison: FrameComparison, config: &FlickerConfig) {
        self.total_frames += 1;

        match comparison.status {
            FlickerStatus::Stable => self.stable_count += 1,
            FlickerStatus::Minor => self.minor_count += 1,
            FlickerStatus::Warning => self.warning_count += 1,
            FlickerStatus::Exceeded => self.exceeded_count += 1,
        }

        self.max_delta_e = self.max_delta_e.max(comparison.delta_e);

        // Update running average
        let n = self.total_frames as f64;
        self.avg_delta_e = (self.avg_delta_e * (n - 1.0) + comparison.delta_e) / n;

        // Log violations
        if config.log_violations && !comparison.status.is_acceptable() {
            self.violations.push(comparison);
        }
    }
}

// ============================================================================
// DELTA E 2000
// ============================================================================

/// Compute ΔE2000 between two Lab colors.
///
/// This is the standard perceptual color difference metric.
pub fn delta_e_2000(lab1: [f64; 3], lab2: [f64; 3]) -> f64 {
    let [l1, a1, b1] = lab1;
    let [l2, a2, b2] = lab2;

    // Constants
    let k_l = 1.0;
    let k_c = 1.0;
    let k_h = 1.0;

    // Calculate C' and h'
    let c1 = (a1 * a1 + b1 * b1).sqrt();
    let c2 = (a2 * a2 + b2 * b2).sqrt();
    let c_bar = (c1 + c2) / 2.0;
    let c_bar_7 = c_bar.powi(7);
    let g = 0.5 * (1.0 - (c_bar_7 / (c_bar_7 + 25.0_f64.powi(7))).sqrt());

    let a1_prime = a1 * (1.0 + g);
    let a2_prime = a2 * (1.0 + g);

    let c1_prime = (a1_prime * a1_prime + b1 * b1).sqrt();
    let c2_prime = (a2_prime * a2_prime + b2 * b2).sqrt();
    let c_bar_prime = (c1_prime + c2_prime) / 2.0;

    let h1_prime = b1.atan2(a1_prime).to_degrees();
    let h1_prime = if h1_prime < 0.0 {
        h1_prime + 360.0
    } else {
        h1_prime
    };

    let h2_prime = b2.atan2(a2_prime).to_degrees();
    let h2_prime = if h2_prime < 0.0 {
        h2_prime + 360.0
    } else {
        h2_prime
    };

    // Calculate delta values
    let delta_l_prime = l2 - l1;
    let delta_c_prime = c2_prime - c1_prime;

    let h_diff = h2_prime - h1_prime;
    let delta_h_prime = if c1_prime * c2_prime == 0.0 {
        0.0
    } else if h_diff.abs() <= 180.0 {
        h_diff
    } else if h_diff > 180.0 {
        h_diff - 360.0
    } else {
        h_diff + 360.0
    };

    let delta_h_prime_rad =
        2.0 * (c1_prime * c2_prime).sqrt() * (delta_h_prime.to_radians() / 2.0).sin();

    // Calculate H'bar
    let h_bar_prime = if c1_prime * c2_prime == 0.0 {
        h1_prime + h2_prime
    } else if (h1_prime - h2_prime).abs() <= 180.0 {
        (h1_prime + h2_prime) / 2.0
    } else if h1_prime + h2_prime < 360.0 {
        (h1_prime + h2_prime + 360.0) / 2.0
    } else {
        (h1_prime + h2_prime - 360.0) / 2.0
    };

    // Calculate T
    let t = 1.0 - 0.17 * (h_bar_prime - 30.0).to_radians().cos()
        + 0.24 * (2.0 * h_bar_prime).to_radians().cos()
        + 0.32 * (3.0 * h_bar_prime + 6.0).to_radians().cos()
        - 0.20 * (4.0 * h_bar_prime - 63.0).to_radians().cos();

    let l_bar_prime = (l1 + l2) / 2.0;
    let l_minus_50_sq = (l_bar_prime - 50.0).powi(2);

    // Weighting functions
    let s_l = 1.0 + (0.015 * l_minus_50_sq) / (20.0 + l_minus_50_sq).sqrt();
    let s_c = 1.0 + 0.045 * c_bar_prime;
    let s_h = 1.0 + 0.015 * c_bar_prime * t;

    let c_bar_prime_7 = c_bar_prime.powi(7);
    let r_c = 2.0 * (c_bar_prime_7 / (c_bar_prime_7 + 25.0_f64.powi(7))).sqrt();
    let delta_theta = 30.0 * (-((h_bar_prime - 275.0) / 25.0).powi(2)).exp();
    let r_t = -r_c * (2.0 * delta_theta.to_radians()).sin();

    // Final calculation
    let term1 = delta_l_prime / (k_l * s_l);
    let term2 = delta_c_prime / (k_c * s_c);
    let term3 = delta_h_prime_rad / (k_h * s_h);

    (term1 * term1 + term2 * term2 + term3 * term3 + r_t * term2 * term3).sqrt()
}

/// Convert XYZ to Lab (D65 illuminant).
pub fn xyz_to_lab(xyz: [f64; 3]) -> [f64; 3] {
    // D65 white point
    let xn = 0.95047;
    let yn = 1.0;
    let zn = 1.08883;

    let f = |t: f64| -> f64 {
        let delta: f64 = 6.0 / 29.0;
        if t > delta.powi(3) {
            t.powf(1.0 / 3.0)
        } else {
            t / (3.0 * delta * delta) + 4.0 / 29.0
        }
    };

    let fx = f(xyz[0] / xn);
    let fy = f(xyz[1] / yn);
    let fz = f(xyz[2] / zn);

    [
        116.0 * fy - 16.0, // L*
        500.0 * (fx - fy), // a*
        200.0 * (fy - fz), // b*
    ]
}

/// Convert RGB to XYZ (sRGB, D65).
pub fn rgb_to_xyz(rgb: [f64; 3]) -> [f64; 3] {
    // Linearize sRGB
    let linearize = |v: f64| -> f64 {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };

    let r = linearize(rgb[0]);
    let g = linearize(rgb[1]);
    let b = linearize(rgb[2]);

    // sRGB to XYZ matrix (D65)
    [
        0.4124564 * r + 0.3575761 * g + 0.1804375 * b,
        0.2126729 * r + 0.7151522 * g + 0.0721750 * b,
        0.0193339 * r + 0.1191920 * g + 0.9503041 * b,
    ]
}

// ============================================================================
// FLICKER VALIDATOR
// ============================================================================

/// Flicker validator for spectral packets.
#[derive(Debug, Clone)]
pub struct FlickerValidator {
    /// Configuration.
    config: FlickerConfig,
    /// Previous packet for comparison.
    previous: Option<SpectralPacket>,
    /// Previous Lab color.
    previous_lab: Option<[f64; 3]>,
    /// Current frame index.
    frame_index: u64,
    /// Report.
    report: FlickerReport,
}

impl Default for FlickerValidator {
    fn default() -> Self {
        Self::new(FlickerConfig::default())
    }
}

impl FlickerValidator {
    /// Create new validator.
    pub fn new(config: FlickerConfig) -> Self {
        Self {
            config,
            previous: None,
            previous_lab: None,
            frame_index: 0,
            report: FlickerReport::default(),
        }
    }

    /// Create strict validator.
    pub fn strict() -> Self {
        Self::new(FlickerConfig::strict())
    }

    /// Create relaxed validator.
    pub fn relaxed() -> Self {
        Self::new(FlickerConfig::relaxed())
    }

    /// Validate a spectral packet.
    ///
    /// Returns the flicker status for this frame.
    pub fn validate(&mut self, packet: &mut SpectralPacket) -> FlickerStatus {
        self.frame_index += 1;

        // Convert to RGB then to Lab
        let rgb = packet.to_rgb();
        let xyz = rgb_to_xyz(rgb);
        let lab = xyz_to_lab(xyz);

        let status = if let Some(prev_lab) = self.previous_lab {
            let delta_e = delta_e_2000(prev_lab, lab);

            // Compute spectral differences
            let (max_diff, avg_diff) = if let Some(prev_packet) = &self.previous {
                self.compute_spectral_diffs(prev_packet, packet)
            } else {
                (0.0, 0.0)
            };

            let comparison = FrameComparison::new(
                self.frame_index - 1,
                self.frame_index,
                delta_e,
                max_diff,
                avg_diff,
                &self.config,
            );

            let status = comparison.status;
            self.report.add_comparison(comparison, &self.config);
            status
        } else {
            FlickerStatus::Stable
        };

        self.previous = Some(packet.clone());
        self.previous_lab = Some(lab);

        status
    }

    /// Compute spectral differences.
    fn compute_spectral_diffs(&self, prev: &SpectralPacket, curr: &SpectralPacket) -> (f64, f64) {
        let mut max_diff: f64 = 0.0;
        let mut total_diff: f64 = 0.0;
        let mut count: usize = 0;

        for (i, &curr_val) in curr.values.iter().enumerate() {
            if i < prev.values.len() {
                let diff = (curr_val - prev.values[i]).abs();
                max_diff = max_diff.max(diff);
                total_diff += diff;
                count += 1;
            }
        }

        let avg_diff = if count > 0 {
            total_diff / count as f64
        } else {
            0.0
        };
        (max_diff, avg_diff)
    }

    /// Get current report.
    pub fn report(&self) -> &FlickerReport {
        &self.report
    }

    /// Reset validator.
    pub fn reset(&mut self) {
        self.previous = None;
        self.previous_lab = None;
        self.frame_index = 0;
        self.report = FlickerReport::default();
    }

    /// Get current frame index.
    pub fn frame_index(&self) -> u64 {
        self.frame_index
    }

    /// Check if validation is passing.
    pub fn is_passing(&self) -> bool {
        self.report.passed()
    }

    /// Get config.
    pub fn config(&self) -> &FlickerConfig {
        &self.config
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delta_e_2000_identical() {
        let lab = [50.0, 25.0, -30.0];
        let de = delta_e_2000(lab, lab);
        assert!(de < 1e-6);
    }

    #[test]
    fn test_delta_e_2000_different() {
        let lab1 = [50.0, 0.0, 0.0];
        let lab2 = [55.0, 0.0, 0.0]; // Slight L difference

        let de = delta_e_2000(lab1, lab2);
        assert!(de > 0.0);
        assert!(de < 10.0); // Should be relatively small for 5 L* difference
    }

    #[test]
    fn test_xyz_to_lab_white() {
        // D65 white point
        let xyz = [0.95047, 1.0, 1.08883];
        let lab = xyz_to_lab(xyz);

        assert!((lab[0] - 100.0).abs() < 0.1); // L* = 100
        assert!(lab[1].abs() < 0.1); // a* = 0
        assert!(lab[2].abs() < 0.1); // b* = 0
    }

    #[test]
    fn test_rgb_to_xyz() {
        // White
        let xyz = rgb_to_xyz([1.0, 1.0, 1.0]);
        assert!(xyz[0] > 0.9 && xyz[0] < 1.0);
        assert!(xyz[1] > 0.99 && xyz[1] < 1.01);

        // Black
        let xyz_black = rgb_to_xyz([0.0, 0.0, 0.0]);
        assert!(xyz_black[0].abs() < 1e-6);
        assert!(xyz_black[1].abs() < 1e-6);
        assert!(xyz_black[2].abs() < 1e-6);
    }

    #[test]
    fn test_flicker_status_from_delta_e() {
        let config = FlickerConfig::default();

        assert_eq!(
            FlickerStatus::from_delta_e(0.1, &config),
            FlickerStatus::Stable
        );
        assert_eq!(
            FlickerStatus::from_delta_e(0.3, &config),
            FlickerStatus::Minor
        );
        assert_eq!(
            FlickerStatus::from_delta_e(0.7, &config),
            FlickerStatus::Warning
        );
        assert_eq!(
            FlickerStatus::from_delta_e(1.5, &config),
            FlickerStatus::Exceeded
        );
    }

    #[test]
    fn test_flicker_validator_stable() {
        let mut validator = FlickerValidator::default();

        // Same packet each frame should be stable
        for _ in 0..5 {
            let mut packet = SpectralPacket::uniform_31();
            for v in packet.values.iter_mut() {
                *v = 0.5;
            }
            let status = validator.validate(&mut packet);
            assert!(status.is_acceptable());
        }

        assert!(validator.is_passing());
    }

    #[test]
    fn test_flicker_validator_first_frame() {
        let mut validator = FlickerValidator::default();
        let mut packet = SpectralPacket::uniform_31();

        let status = validator.validate(&mut packet);
        assert_eq!(status, FlickerStatus::Stable);
    }

    #[test]
    fn test_flicker_report() {
        let mut report = FlickerReport::default();
        let config = FlickerConfig::default();

        report.add_comparison(
            FrameComparison::new(0, 1, 0.1, 0.05, 0.02, &config),
            &config,
        );
        report.add_comparison(FrameComparison::new(1, 2, 0.3, 0.1, 0.05, &config), &config);

        assert_eq!(report.total_frames, 2);
        assert_eq!(report.stable_count, 1);
        assert_eq!(report.minor_count, 1);
        assert!(report.passed());
        assert!(report.pass_rate() >= 0.99);
    }

    #[test]
    fn test_flicker_config_presets() {
        let strict = FlickerConfig::strict();
        let relaxed = FlickerConfig::relaxed();

        assert!(strict.stable_threshold < relaxed.stable_threshold);
        assert!(strict.warning_threshold < relaxed.warning_threshold);
    }

    #[test]
    fn test_validator_reset() {
        let mut validator = FlickerValidator::default();
        let mut packet = SpectralPacket::uniform_31();

        validator.validate(&mut packet);
        validator.validate(&mut packet);

        assert_eq!(validator.frame_index(), 2);

        validator.reset();

        assert_eq!(validator.frame_index(), 0);
        assert_eq!(validator.report().total_frames, 0);
    }
}

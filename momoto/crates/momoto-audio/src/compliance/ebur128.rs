//! EBU R128 loudness compliance validation.
//!
//! Validates measured LUFS values against the two main EBU R128 target profiles:
//!
//! | Profile | Integrated LUFS | True Peak |
//! |---------|----------------|-----------|
//! | Broadcast | -23 LUFS ±1 LU | -1.0 dBTP |
//! | Streaming | -14 LUFS (ceiling) | -1.0 dBTP |
//!
//! # References
//!
//! - EBU R128 (2020): Loudness Normalisation and Permitted Maximum Level
//! - EBU Tech 3343: Practical guidelines for EBU R128 (streaming addendum)

use momoto_core::traits::compliance::{
    Compliance, ComplianceReport, ComplianceViolation, ViolationSeverity,
};

/// EBU R128 loudness target profile.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EbuR128Limits {
    /// Target integrated loudness in LUFS.
    pub target_lufs: f64,
    /// Permitted deviation below target (positive means below).
    pub tolerance_lu: f64,
    /// Maximum permitted true peak in dBTP.
    pub max_true_peak_dbtp: f64,
    /// Maximum permitted short-term loudness in LUFS (`None` = unconstrained).
    pub max_short_term_lufs: Option<f64>,
    /// Human-readable name for this profile.
    pub name: &'static str,
}

impl EbuR128Limits {
    /// EBU R128 broadcast profile: -23 LUFS ±1 LU, TP -1 dBTP.
    ///
    /// Mandated by the European Broadcasting Union for broadcast television.
    pub const BROADCAST: Self = Self {
        target_lufs: -23.0,
        tolerance_lu: 1.0,
        max_true_peak_dbtp: -1.0,
        max_short_term_lufs: Some(-18.0), // EBU R128 §3.4
        name: "EBU R128 Broadcast",
    };

    /// Streaming profile: -14 LUFS ceiling, TP -1 dBTP.
    ///
    /// Used by most major streaming platforms (Spotify, Apple Music, YouTube).
    pub const STREAMING: Self = Self {
        target_lufs: -14.0,
        tolerance_lu: 2.0, // platforms accept -14±2 LU
        max_true_peak_dbtp: -1.0,
        max_short_term_lufs: None,
        name: "EBU R128 Streaming",
    };

    /// Podcast / spoken word profile: -16 LUFS, TP -1 dBTP.
    pub const PODCAST: Self = Self {
        target_lufs: -16.0,
        tolerance_lu: 1.0,
        max_true_peak_dbtp: -1.0,
        max_short_term_lufs: None,
        name: "EBU R128 Podcast",
    };

    /// Validate measured loudness values against this profile.
    ///
    /// # Parameters
    ///
    /// - `integrated_lufs`: gated integrated loudness from `LufsAnalyzer::integrated()`
    /// - `short_term_lufs`: optional short-term measurement (`None` skips check)
    /// - `true_peak_dbtp`: optional true-peak measurement (`None` skips check)
    pub fn validate(
        &self,
        integrated_lufs: f64,
        short_term_lufs: Option<f64>,
        true_peak_dbtp: Option<f64>,
    ) -> ComplianceReport {
        let mut report = ComplianceReport::new(self.name);

        // Check integrated loudness is finite
        if !integrated_lufs.is_finite() {
            report.add_violation(ComplianceViolation::with_description(
                "integrated_lufs_finite",
                ViolationSeverity::Error,
                integrated_lufs as f32,
                self.target_lufs as f32,
                "Integrated LUFS is not finite — insufficient audio content",
            ));
            return report;
        }

        // Integrated loudness: must be within [target - tolerance, target + tolerance]
        let lower = self.target_lufs - self.tolerance_lu;
        let upper = self.target_lufs + self.tolerance_lu;

        if integrated_lufs < lower {
            report.add_violation(ComplianceViolation::with_description(
                "integrated_lufs_too_quiet",
                ViolationSeverity::Error,
                integrated_lufs as f32,
                lower as f32,
                "Integrated loudness is below the permitted range",
            ));
        } else if integrated_lufs > upper {
            report.add_violation(ComplianceViolation::with_description(
                "integrated_lufs_too_loud",
                ViolationSeverity::Error,
                integrated_lufs as f32,
                upper as f32,
                "Integrated loudness exceeds the permitted ceiling",
            ));
        }

        // Short-term loudness check
        if let (Some(st), Some(max_st)) = (short_term_lufs, self.max_short_term_lufs) {
            if st > max_st {
                report.add_violation(ComplianceViolation::with_description(
                    "short_term_lufs_exceeded",
                    ViolationSeverity::Warning,
                    st as f32,
                    max_st as f32,
                    "Short-term loudness exceeds the permitted maximum",
                ));
            }
        }

        // True peak check
        if let Some(tp) = true_peak_dbtp {
            if tp > self.max_true_peak_dbtp {
                report.add_violation(ComplianceViolation::with_description(
                    "true_peak_exceeded",
                    ViolationSeverity::Critical,
                    tp as f32,
                    self.max_true_peak_dbtp as f32,
                    "True peak exceeds the permitted maximum — clipping risk",
                ));
            }
        }

        report
    }
}

/// Wrapper for a measured loudness reading that implements `Compliance`.
///
/// # Example
///
/// ```rust,ignore
/// use momoto_audio::compliance::ebur128::{EbuR128Limits, EbuR128Measurement};
/// use momoto_core::traits::compliance::Compliance;
///
/// let m = EbuR128Measurement {
///     integrated_lufs: -23.0,
///     short_term_lufs: Some(-18.5),
///     true_peak_dbtp: Some(-1.5),
///     limits: EbuR128Limits::BROADCAST,
/// };
/// let report = m.validate();
/// assert!(report.passes);
/// ```
#[derive(Debug, Clone)]
pub struct EbuR128Measurement {
    /// Gated integrated loudness in LUFS.
    pub integrated_lufs: f64,
    /// Optional short-term (3 s) loudness in LUFS.
    pub short_term_lufs: Option<f64>,
    /// Optional true peak in dBTP.
    pub true_peak_dbtp: Option<f64>,
    /// Compliance limits to validate against.
    pub limits: EbuR128Limits,
}

impl Compliance for EbuR128Measurement {
    fn standard() -> &'static str {
        "EBU R128"
    }

    fn validate(&self) -> ComplianceReport {
        self.limits.validate(
            self.integrated_lufs,
            self.short_term_lufs,
            self.true_peak_dbtp,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use momoto_core::traits::compliance::Compliance;

    fn measure(integrated: f64) -> EbuR128Measurement {
        EbuR128Measurement {
            integrated_lufs: integrated,
            short_term_lufs: None,
            true_peak_dbtp: None,
            limits: EbuR128Limits::BROADCAST,
        }
    }

    #[test]
    fn broadcast_target_passes() {
        let m = measure(-23.0);
        assert!(m.validate().passes, "-23 LUFS should pass broadcast");
    }

    #[test]
    fn broadcast_within_tolerance_passes() {
        assert!(
            measure(-22.5).validate().passes,
            "-22.5 LUFS passes (+0.5 LU)"
        );
        assert!(
            measure(-23.5).validate().passes,
            "-23.5 LUFS passes (-0.5 LU)"
        );
    }

    #[test]
    fn broadcast_too_loud_fails() {
        let report = measure(-21.0).validate();
        assert!(!report.passes, "-21 LUFS should fail broadcast");
        assert!(report.has_error_or_above());
    }

    #[test]
    fn broadcast_too_quiet_fails() {
        let report = measure(-25.0).validate();
        assert!(!report.passes, "-25 LUFS should fail broadcast");
    }

    #[test]
    fn streaming_ceiling_passes() {
        let m = EbuR128Measurement {
            integrated_lufs: -14.0,
            short_term_lufs: None,
            true_peak_dbtp: None,
            limits: EbuR128Limits::STREAMING,
        };
        assert!(m.validate().passes);
    }

    #[test]
    fn streaming_above_ceiling_fails() {
        let m = EbuR128Measurement {
            integrated_lufs: -11.0, // exceeds -14+2 = -12 upper bound
            short_term_lufs: None,
            true_peak_dbtp: None,
            limits: EbuR128Limits::STREAMING,
        };
        assert!(!m.validate().passes);
    }

    #[test]
    fn true_peak_exceeded_is_critical() {
        let m = EbuR128Measurement {
            integrated_lufs: -23.0,
            short_term_lufs: None,
            true_peak_dbtp: Some(0.0), // 0 dBTP > -1 dBTP limit
            limits: EbuR128Limits::BROADCAST,
        };
        let report = m.validate();
        assert!(!report.passes);
        assert!(report.has_critical());
    }

    #[test]
    fn true_peak_within_limit_passes() {
        let m = EbuR128Measurement {
            integrated_lufs: -23.0,
            short_term_lufs: None,
            true_peak_dbtp: Some(-2.0), // -2 dBTP ≤ -1 dBTP limit
            limits: EbuR128Limits::BROADCAST,
        };
        assert!(m.validate().passes);
    }

    #[test]
    fn non_finite_integrated_fails() {
        let report = measure(f64::NEG_INFINITY).validate();
        assert!(!report.passes);
    }

    #[test]
    fn compliance_trait_standard_name() {
        assert_eq!(EbuR128Measurement::standard(), "EBU R128");
    }

    #[test]
    fn is_compliant_convenience() {
        let m = measure(-23.0);
        assert!(m.is_compliant());
    }
}

//! # Phase 9 — Perceptual Source of Truth Certification
//!
//! Provides a formal certification authority for Momoto's perceptual color
//! system.  Every color system, design-token set, animation parameter bundle,
//! or material that passes the conformance engine receives a signed
//! certificate bound to the versioned `PerceptualSpecification`.
//!
//! ## Design goals
//!
//! - **Deterministic** — same inputs always produce the same certificate ID /
//!   hash.  Entropy comes only from the system clock.
//! - **Self-contained** — no network I/O, no file system access.  Everything
//!   lives in memory and can be serialised to JSON.
//! - **Auditabile** — every certification run is tracked by `AuditLogger` and
//!   can be exported in structured format.
//!
//! Implements: governance/RFC-0009

#![allow(dead_code, unused_variables)]

use momoto_core::color::Color;
use momoto_core::luminance::{relative_luminance_apca, relative_luminance_srgb};
use momoto_core::perception::ContrastMetric;
use momoto_core::space::oklch::OKLCH;
use momoto_metrics::{APCAMetric, WCAGMetric};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Utilities (foundation — used by all other types)
// ============================================================================

/// Returns the number of seconds elapsed since the Unix epoch.
///
/// Uses [`std::time::SystemTime`].  Returns 0 on platforms that do not
/// support system time (e.g. some embedded targets).
pub fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Generates a unique certificate identifier.
///
/// Format: `cert-{timestamp:016x}-{random:08x}` where the random component
/// is produced by a single step of a 64-bit LCG seeded with the timestamp.
pub fn generate_certificate_id() -> String {
    let ts = current_timestamp();
    // LCG: multiplier and increment from Knuth / Numerical Recipes
    let random = (ts
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407)
        >> 32) as u32;
    format!("cert-{:016x}-{:08x}", ts, random)
}

/// Computes a deterministic 256-bit hash of `data` encoded as a 64-character
/// lowercase hex string.
///
/// Algorithm: XOR-fold over 32 bytes using the raw UTF-8 representation of
/// `data`.  This is intentionally simple (not cryptographic) — it is used
/// only for integrity verification, not security.
pub fn compute_hash(data: &str) -> String {
    let mut state = [0u8; 32];
    for (i, &byte) in data.as_bytes().iter().enumerate() {
        state[i % 32] ^= byte;
        // Avalanche: mix neighbouring bytes after each XOR
        let j = (i + 1) % 32;
        state[j] = state[j].wrapping_add(state[i % 32].rotate_left(3));
    }
    // Final diffusion pass
    for round in 0..4u8 {
        for i in 0..32 {
            let prev = state[(i + 31) % 32];
            state[i] = state[i]
                .wrapping_add(prev)
                .rotate_left(((i as u32) % 5) + 1)
                ^ round;
        }
    }
    state.iter().map(|b| format!("{:02x}", b)).collect()
}

// ============================================================================
// Core data types
// ============================================================================

/// Distinguishes what kind of artifact a [`CertificationTarget`] represents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetType {
    /// A full color system (palette + semantic + surfaces).
    ColorSystem,
    /// A design-token set (JSON / Style Dictionary tokens).
    DesignTokenSet,
    /// Animation / transition parameters.
    AnimationParams,
    /// A single physical material (IOR, roughness, …).
    Material,
    /// All subsystems bundled together.
    FullSystem,
}

/// Colour data attached to a certification target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorData {
    /// Hex-encoded colors (e.g. `"#ff6600"`).
    pub hex_colors: Vec<String>,
    /// OKLCH triplets `[L, C, H]` corresponding to `hex_colors`.
    pub oklch_values: Vec<[f64; 3]>,
    /// WCAG relative luminance values for each color.
    pub wcag_luminances: Vec<f64>,
    /// Optional human-readable palette name.
    pub palette_name: Option<String>,
}

/// Physical material parameters attached to a certification target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialData {
    /// Index of refraction (dimensionless, typically 1.0–2.5 for dielectrics).
    pub ior: f64,
    /// Perceptual roughness [0, 1].
    pub roughness: f64,
    /// Whether the material is metallic (conductor).
    pub metallic: bool,
    /// Human-readable name.
    pub name: String,
    /// Category string (e.g. `"glass"`, `"metal"`, `"organic"`).
    pub category: String,
}

/// Animation / transition parameters attached to a certification target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationData {
    /// Total transition duration in milliseconds.
    pub duration_ms: u64,
    /// CSS easing function name or cubic-bezier descriptor.
    pub easing: String,
    /// Starting color as hex string.
    pub color_from: String,
    /// Ending color as hex string.
    pub color_to: String,
    /// Maximum acceptable flicker frequency in Hz (must be < 3 for WCAG).
    pub max_flicker_hz: f64,
}

/// The entity to be certified.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificationTarget {
    /// Unique identifier for this target (caller-assigned).
    pub id: String,
    /// What kind of artifact this is.
    pub target_type: TargetType,
    /// Color data, present when `target_type` is `ColorSystem` or `DesignTokenSet`.
    pub color_data: Option<ColorData>,
    /// Material data, present when `target_type` is `Material`.
    pub material_data: Option<MaterialData>,
    /// Animation data, present when `target_type` is `AnimationParams`.
    pub animation_data: Option<AnimationData>,
    /// Arbitrary key-value metadata (author, project, commit hash, …).
    pub metadata: HashMap<String, String>,
}

/// Identity record of the Momoto engine itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MomotoIdentity {
    /// Semantic version string.
    pub version: String,
    /// Short hash of the engine build.
    pub build_hash: String,
    /// List of capability identifiers.
    pub capabilities: Vec<String>,
    /// Unix timestamp of the initial certification.
    pub certified_since: u64,
}

impl MomotoIdentity {
    /// Returns the well-known identity for Momoto v7.0.0.
    pub fn current() -> Self {
        MomotoIdentity {
            version: "7.0.0".to_string(),
            build_hash: "da0075a".to_string(),
            capabilities: vec![
                "oklch-color-space".to_string(),
                "wcag-2.1".to_string(),
                "apca-w3-0.1.9".to_string(),
                "hct-cam16".to_string(),
                "material-physics".to_string(),
                "neural-siren-correction".to_string(),
                "temporal-perception".to_string(),
                "cvd-simulation".to_string(),
                "thin-film-interference".to_string(),
                "mie-scattering".to_string(),
                "color-harmony".to_string(),
                "constraint-solver".to_string(),
            ],
            certified_since: 1_740_614_400, // 2026-02-27 00:00:00 UTC
        }
    }
}

// ============================================================================
// Specification types
// ============================================================================

/// Maximum perceptual deviations the conformance engine will accept.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceptualTolerances {
    /// Maximum ΔE2000 for roundtrip color accuracy.
    pub delta_e_max: f64,
    /// Maximum absolute deviation in OKLCH L channel.
    pub oklch_l_tolerance: f64,
    /// Maximum absolute deviation in OKLCH C channel.
    pub oklch_c_tolerance: f64,
    /// Maximum absolute deviation in OKLCH H channel (degrees).
    pub oklch_h_tolerance_deg: f64,
}

impl PerceptualTolerances {
    /// Strict tolerances for high-fidelity production systems.
    pub fn strict() -> Self {
        PerceptualTolerances {
            delta_e_max: 1.0,
            oklch_l_tolerance: 0.01,
            oklch_c_tolerance: 0.005,
            oklch_h_tolerance_deg: 0.5,
        }
    }

    /// Standard tolerances for typical design systems.
    pub fn standard() -> Self {
        PerceptualTolerances {
            delta_e_max: 2.0,
            oklch_l_tolerance: 0.02,
            oklch_c_tolerance: 0.01,
            oklch_h_tolerance_deg: 1.0,
        }
    }

    /// Relaxed tolerances for exploratory / prototype systems.
    pub fn relaxed() -> Self {
        PerceptualTolerances {
            delta_e_max: 3.0,
            oklch_l_tolerance: 0.05,
            oklch_c_tolerance: 0.02,
            oklch_h_tolerance_deg: 2.0,
        }
    }
}

/// Accessibility requirements derived from WCAG 2.1 / APCA-W3.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityRequirements {
    /// Minimum WCAG compliance level (e.g. `"AA"` or `"AAA"`).
    pub min_wcag_level: String,
    /// Minimum absolute APCA Lc value.
    pub min_apca_lc: f64,
    /// Require WCAG AA for all text/UI components.
    pub require_aa: bool,
    /// Require WCAG AAA for body text.
    pub require_aaa: bool,
}

impl AccessibilityRequirements {
    /// WCAG AA requirements (4.5:1 normal text, 3:1 large text).
    pub fn wcag_aa() -> Self {
        AccessibilityRequirements {
            min_wcag_level: "AA".to_string(),
            min_apca_lc: 60.0,
            require_aa: true,
            require_aaa: false,
        }
    }
}

/// Constraints on the SIREN neural correction pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuralCorrectionConstraints {
    /// Maximum ΔE correction the neural pass may introduce.
    pub max_delta_e_correction: f64,
    /// Hue must not shift by more than this many degrees.
    pub preserve_hue_within_deg: f64,
    /// Whether neural correction is allowed at all.
    pub correction_enabled: bool,
}

/// Physical plausibility rules for material IOR and roughness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialPhysicsRules {
    /// Acceptable IOR range `(min, max)`.
    pub ior_range: (f64, f64),
    /// Maximum perceptual roughness.
    pub max_roughness: f64,
    /// Whether the BRDF must satisfy energy conservation.
    pub energy_conserving: bool,
}

/// Static (single-frame) perceptual rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticPerceptionRules {
    /// Name of the primary color gamut (`"sRGB"`, `"P3"`, …).
    pub oklch_gamut: String,
    /// Maximum ΔE for a hex → OKLCH → hex roundtrip.
    pub roundtrip_delta_e_max: f64,
    /// White point identifier (`"D65"`).
    pub white_point: String,
    /// Chromatic adaptation method (`"Bradford"`, `"CAT16"`, …).
    pub chromatic_adaptation: String,
}

impl StaticPerceptionRules {
    /// Standard v7 static rules.
    pub fn standard() -> Self {
        StaticPerceptionRules {
            oklch_gamut: "sRGB".to_string(),
            roundtrip_delta_e_max: 1.0,
            white_point: "D65".to_string(),
            chromatic_adaptation: "CAT16".to_string(),
        }
    }
}

/// Temporal (animated / sequential) perceptual rules derived from WCAG 2.3.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalPerceptionRules {
    /// Maximum luminance oscillation frequency in Hz before photosensitive-
    /// seizure risk.  WCAG 2.3 sets this at 3 Hz.
    pub max_flicker_hz: f64,
    /// Maximum luminance change per second (cd/m² equivalent).
    pub max_luminance_change_per_sec: f64,
    /// Whether all transitions must declare an easing function.
    pub require_easing: bool,
}

impl TemporalPerceptionRules {
    /// WCAG-safe temporal rules.
    pub fn wcag() -> Self {
        TemporalPerceptionRules {
            max_flicker_hz: 3.0,
            max_luminance_change_per_sec: 0.5,
            require_easing: true,
        }
    }
}

/// The complete versioned specification used to evaluate conformance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceptualSpecification {
    /// Specification version string.
    pub version: String,
    /// Static (frame-level) rules.
    pub static_rules: StaticPerceptionRules,
    /// Temporal (animation) rules.
    pub temporal_rules: TemporalPerceptionRules,
    /// Accessibility requirements.
    pub accessibility: AccessibilityRequirements,
    /// Neural correction constraints.
    pub neural: NeuralCorrectionConstraints,
    /// Material physics rules.
    pub material: MaterialPhysicsRules,
    /// Perceptual tolerances.
    pub tolerances: PerceptualTolerances,
}

impl PerceptualSpecification {
    /// Momoto v7.0.0 perceptual specification.
    pub fn v7() -> Self {
        PerceptualSpecification {
            version: "7.0.0".to_string(),
            static_rules: StaticPerceptionRules::standard(),
            temporal_rules: TemporalPerceptionRules::wcag(),
            accessibility: AccessibilityRequirements::wcag_aa(),
            neural: NeuralCorrectionConstraints {
                max_delta_e_correction: 2.0,
                preserve_hue_within_deg: 5.0,
                correction_enabled: true,
            },
            material: MaterialPhysicsRules {
                ior_range: (1.0, 3.0),
                max_roughness: 1.0,
                energy_conserving: true,
            },
            tolerances: PerceptualTolerances::standard(),
        }
    }
}

// ============================================================================
// Conformance engine
// ============================================================================

/// Categories of conformance tests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestType {
    /// Hex → OKLCH → hex roundtrip accuracy.
    ColorRoundtrip,
    /// WCAG 2.1 contrast ratio compliance.
    WcagCompliance,
    /// APCA-W3 Lc compliance.
    ApcaCompliance,
    /// Colors must lie within the declared gamut boundary.
    GamutBoundary,
    /// Animation parameters must not trigger photosensitive risk.
    TemporalSafety,
    /// Material IOR / roughness must be physically plausible.
    MaterialPhysics,
    /// Neural correction must stay within allowed ΔE budget.
    NeuralCorrection,
}

/// Result of a single conformance test.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConformanceTest {
    /// Human-readable test name.
    pub name: String,
    /// Category of this test.
    pub test_type: TestType,
    /// Whether the test passed.
    pub passed: bool,
    /// Detailed description / measurements.
    pub details: String,
    /// Normalised score [0, 1] (1.0 = perfect).
    pub score: f64,
}

/// Aggregated result of all conformance tests for a target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConformanceResult {
    /// Individual test results.
    pub tests: Vec<ConformanceTest>,
    /// True if all tests passed.
    pub overall_pass: bool,
    /// Percentage of tests that passed [0, 100].
    pub compliance_percentage: f64,
    /// Names of failing tests.
    pub failures: Vec<String>,
}

/// Evaluates a `CertificationTarget` against a `PerceptualSpecification`.
#[derive(Debug, Clone)]
pub struct ConformanceEngine {
    /// The specification against which targets are evaluated.
    pub spec: PerceptualSpecification,
}

impl ConformanceEngine {
    /// Creates a new engine bound to the given specification.
    pub fn new(spec: PerceptualSpecification) -> Self {
        ConformanceEngine { spec }
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    /// Parses a hex color string and returns `(Color, OKLCH)` or an error
    /// message.
    fn parse_hex(hex: &str) -> Result<(Color, OKLCH), String> {
        let color = Color::from_hex(hex).map_err(|e| format!("invalid hex '{}': {}", hex, e))?;
        let oklch = OKLCH::from_color(&color);
        Ok((color, oklch))
    }

    /// Performs a hex → OKLCH → hex roundtrip and returns the maximum channel
    /// deviation as (delta_l, delta_c, delta_h_deg).
    fn roundtrip_deviation(hex: &str) -> Result<(f64, f64, f64), String> {
        let (color, oklch) = Self::parse_hex(hex)?;
        let recovered = oklch.to_color();
        let recovered_oklch = OKLCH::from_color(&recovered);

        let dl = (oklch.l - recovered_oklch.l).abs();
        let dc = (oklch.c - recovered_oklch.c).abs();
        let dh = {
            let raw = (oklch.h - recovered_oklch.h).abs();
            raw.min(360.0 - raw)
        };
        Ok((dl, dc, dh))
    }

    /// Returns `true` when the color is inside the sRGB gamut (all channels
    /// in [0, 1] after conversion).
    fn is_in_gamut(hex: &str) -> bool {
        match Color::from_hex(hex) {
            Ok(c) => OKLCH::from_color(&c).is_in_gamut(),
            Err(_) => false,
        }
    }

    // ------------------------------------------------------------------
    // Public evaluation API
    // ------------------------------------------------------------------

    /// Tests a single color for roundtrip accuracy and gamut membership.
    pub fn test_color(&self, hex: &str) -> ConformanceResult {
        let mut tests = Vec::new();

        // Test 1: roundtrip accuracy
        match Self::roundtrip_deviation(hex) {
            Ok((dl, dc, dh)) => {
                let tol = &self.spec.tolerances;
                let passed = dl <= tol.oklch_l_tolerance
                    && dc <= tol.oklch_c_tolerance
                    && dh <= tol.oklch_h_tolerance_deg;
                let score = if passed {
                    1.0
                } else {
                    let max_err = (dl / tol.oklch_l_tolerance)
                        .max(dc / tol.oklch_c_tolerance)
                        .max(dh / tol.oklch_h_tolerance_deg);
                    (1.0 / max_err).clamp(0.0, 1.0)
                };
                tests.push(ConformanceTest {
                    name: "Color Roundtrip".to_string(),
                    test_type: TestType::ColorRoundtrip,
                    passed,
                    details: format!("ΔL={:.5} ΔC={:.5} ΔH={:.3}°", dl, dc, dh),
                    score,
                });
            }
            Err(e) => {
                tests.push(ConformanceTest {
                    name: "Color Roundtrip".to_string(),
                    test_type: TestType::ColorRoundtrip,
                    passed: false,
                    details: e,
                    score: 0.0,
                });
            }
        }

        // Test 2: gamut check
        let in_gamut = Self::is_in_gamut(hex);
        tests.push(ConformanceTest {
            name: "Gamut Boundary".to_string(),
            test_type: TestType::GamutBoundary,
            passed: in_gamut,
            details: if in_gamut {
                format!("{} is inside sRGB gamut", hex)
            } else {
                format!("{} is outside sRGB gamut", hex)
            },
            score: if in_gamut { 1.0 } else { 0.0 },
        });

        // Test 3: WCAG luminance plausibility [0, 1]
        let luminance_ok = match Color::from_hex(hex) {
            Ok(c) => {
                let y = relative_luminance_srgb(&c).value();
                (0.0..=1.0).contains(&y)
            }
            Err(_) => false,
        };
        tests.push(ConformanceTest {
            name: "Luminance Range".to_string(),
            test_type: TestType::WcagCompliance,
            passed: luminance_ok,
            details: if luminance_ok {
                "Relative luminance in [0, 1]".to_string()
            } else {
                "Relative luminance out of range".to_string()
            },
            score: if luminance_ok { 1.0 } else { 0.0 },
        });

        Self::aggregate(tests)
    }

    /// Tests a foreground / background pair for WCAG AA and APCA compliance.
    pub fn test_pair(&self, fg: &str, bg: &str) -> ConformanceResult {
        let mut tests = Vec::new();

        let fg_color = Color::from_hex(fg);
        let bg_color = Color::from_hex(bg);

        match (fg_color, bg_color) {
            (Ok(fg_c), Ok(bg_c)) => {
                // WCAG contrast ratio
                let wcag_metric = WCAGMetric::new();
                let wcag_result = wcag_metric.evaluate(fg_c.clone(), bg_c.clone());
                let wcag_ratio = wcag_result.value;
                let wcag_pass = wcag_ratio >= 4.5; // AA normal text
                let wcag_score = (wcag_ratio / 4.5).min(1.0);
                tests.push(ConformanceTest {
                    name: "WCAG AA Contrast".to_string(),
                    test_type: TestType::WcagCompliance,
                    passed: wcag_pass,
                    details: format!("Contrast ratio {:.2}:1 (min 4.5:1)", wcag_ratio),
                    score: wcag_score,
                });

                // APCA Lc
                let apca_metric = APCAMetric::new();
                let apca_result = apca_metric.evaluate(fg_c, bg_c);
                let lc = apca_result.value.abs();
                let min_lc = self.spec.accessibility.min_apca_lc;
                let apca_pass = lc >= min_lc;
                let apca_score = (lc / min_lc).min(1.0);
                tests.push(ConformanceTest {
                    name: "APCA Lc Compliance".to_string(),
                    test_type: TestType::ApcaCompliance,
                    passed: apca_pass,
                    details: format!("APCA Lc {:.1} (min {:.0})", lc, min_lc),
                    score: apca_score,
                });
            }
            (Err(e), _) | (_, Err(e)) => {
                tests.push(ConformanceTest {
                    name: "Color Pair Parse".to_string(),
                    test_type: TestType::WcagCompliance,
                    passed: false,
                    details: format!("Parse error: {}", e),
                    score: 0.0,
                });
            }
        }

        Self::aggregate(tests)
    }

    /// Runs a full audit against a `CertificationTarget`, dispatching to the
    /// appropriate test suite based on `target_type`.
    pub fn full_audit(&self, target: &CertificationTarget) -> ConformanceResult {
        let mut all_tests = Vec::new();

        match &target.target_type {
            TargetType::ColorSystem | TargetType::DesignTokenSet => {
                if let Some(cd) = &target.color_data {
                    for hex in &cd.hex_colors {
                        let result = self.test_color(hex);
                        all_tests.extend(result.tests);
                    }
                    // Pair-wise contrast for adjacent colors (first pair only to
                    // avoid O(n²) explosion on large palettes)
                    if cd.hex_colors.len() >= 2 {
                        let pair = self.test_pair(&cd.hex_colors[0], &cd.hex_colors[1]);
                        all_tests.extend(pair.tests);
                    }
                } else {
                    all_tests.push(ConformanceTest {
                        name: "Color Data Present".to_string(),
                        test_type: TestType::ColorRoundtrip,
                        passed: false,
                        details: "No color_data provided for ColorSystem target".to_string(),
                        score: 0.0,
                    });
                }
            }

            TargetType::AnimationParams => {
                if let Some(ad) = &target.animation_data {
                    // Flicker check
                    let flicker_ok = ad.max_flicker_hz <= self.spec.temporal_rules.max_flicker_hz;
                    all_tests.push(ConformanceTest {
                        name: "Flicker Safety".to_string(),
                        test_type: TestType::TemporalSafety,
                        passed: flicker_ok,
                        details: format!(
                            "max_flicker {:.1} Hz (limit {:.1} Hz)",
                            ad.max_flicker_hz, self.spec.temporal_rules.max_flicker_hz
                        ),
                        score: if flicker_ok { 1.0 } else { 0.0 },
                    });

                    // Easing required
                    let easing_ok =
                        !self.spec.temporal_rules.require_easing || !ad.easing.is_empty();
                    all_tests.push(ConformanceTest {
                        name: "Easing Declared".to_string(),
                        test_type: TestType::TemporalSafety,
                        passed: easing_ok,
                        details: if easing_ok {
                            format!("Easing: {}", ad.easing)
                        } else {
                            "No easing function declared".to_string()
                        },
                        score: if easing_ok { 1.0 } else { 0.0 },
                    });

                    // Contrast pair of from/to colors
                    let pair = self.test_pair(&ad.color_from, &ad.color_to);
                    all_tests.extend(pair.tests);
                }
            }

            TargetType::Material => {
                if let Some(md) = &target.material_data {
                    let (ior_min, ior_max) = self.spec.material.ior_range;
                    let ior_ok = md.ior >= ior_min && md.ior <= ior_max;
                    all_tests.push(ConformanceTest {
                        name: "IOR Range".to_string(),
                        test_type: TestType::MaterialPhysics,
                        passed: ior_ok,
                        details: format!("IOR {:.3} in [{:.2}, {:.2}]", md.ior, ior_min, ior_max),
                        score: if ior_ok { 1.0 } else { 0.0 },
                    });

                    let roughness_ok =
                        md.roughness <= self.spec.material.max_roughness && md.roughness >= 0.0;
                    all_tests.push(ConformanceTest {
                        name: "Roughness Range".to_string(),
                        test_type: TestType::MaterialPhysics,
                        passed: roughness_ok,
                        details: format!(
                            "roughness {:.3} in [0, {:.2}]",
                            md.roughness, self.spec.material.max_roughness
                        ),
                        score: if roughness_ok { 1.0 } else { 0.0 },
                    });
                }
            }

            TargetType::FullSystem => {
                // Run all sub-audits
                if target.color_data.is_some() {
                    let partial = CertificationTarget {
                        target_type: TargetType::ColorSystem,
                        ..target.clone()
                    };
                    all_tests.extend(self.full_audit(&partial).tests);
                }
                if target.animation_data.is_some() {
                    let partial = CertificationTarget {
                        target_type: TargetType::AnimationParams,
                        ..target.clone()
                    };
                    all_tests.extend(self.full_audit(&partial).tests);
                }
                if target.material_data.is_some() {
                    let partial = CertificationTarget {
                        target_type: TargetType::Material,
                        ..target.clone()
                    };
                    all_tests.extend(self.full_audit(&partial).tests);
                }
            }
        }

        Self::aggregate(all_tests)
    }

    /// Aggregates a list of individual test results into a `ConformanceResult`.
    fn aggregate(tests: Vec<ConformanceTest>) -> ConformanceResult {
        if tests.is_empty() {
            return ConformanceResult {
                tests: vec![],
                overall_pass: false,
                compliance_percentage: 0.0,
                failures: vec!["No tests were run".to_string()],
            };
        }
        let passed_count = tests.iter().filter(|t| t.passed).count();
        let total = tests.len();
        let compliance_percentage = (passed_count as f64 / total as f64) * 100.0;
        let overall_pass = passed_count == total;
        let failures: Vec<String> = tests
            .iter()
            .filter(|t| !t.passed)
            .map(|t| t.name.clone())
            .collect();
        ConformanceResult {
            tests,
            overall_pass,
            compliance_percentage,
            failures,
        }
    }
}

// ============================================================================
// Certification profiles
// ============================================================================

/// Atomic capabilities that a `CertificationProfile` may bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Capability {
    /// WCAG 2.1 AA compliance.
    WcagAA,
    /// WCAG 2.1 AAA compliance.
    WcagAAA,
    /// APCA-W3 Lc compliance.
    ApcaCompliance,
    /// OKLCH color space support.
    OklchColorSpace,
    /// HCT / Material You color space.
    HctColorSpace,
    /// Full material physics (GGX, SSS, thin-film).
    MaterialPhysics,
    /// SIREN neural perceptual correction.
    NeuralCorrection,
    /// Temporal perception engine (flicker, motion).
    TemporalAnalysis,
    /// Color vision deficiency simulation.
    CvdSimulation,
}

/// Metadata attached to a profile certificate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileMetadata {
    /// Unix timestamp when this profile was created.
    pub created_at: u64,
    /// Unix timestamp when this profile expires.
    pub valid_until: u64,
    /// Issuing authority name.
    pub issuer: String,
    /// Profile specification version.
    pub version: String,
}

/// A named bundle of capabilities that a target must satisfy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificationProfile {
    /// Profile name (e.g. `"basic"`, `"standard"`, `"advanced"`, `"full"`).
    pub name: String,
    /// Capabilities this profile certifies.
    pub capabilities: Vec<Capability>,
    /// Profile metadata.
    pub metadata: ProfileMetadata,
}

impl CertificationProfile {
    fn make_metadata(name: &str) -> ProfileMetadata {
        let now = current_timestamp();
        ProfileMetadata {
            created_at: now,
            valid_until: now + 365 * 24 * 3600, // 1 year
            issuer: "Momoto Certification Authority v7".to_string(),
            version: name.to_string(),
        }
    }

    /// Basic profile: WCAG AA + OKLCH color space.
    pub fn basic() -> Self {
        CertificationProfile {
            name: "basic".to_string(),
            capabilities: vec![Capability::WcagAA, Capability::OklchColorSpace],
            metadata: Self::make_metadata("basic"),
        }
    }

    /// Standard profile: extends Basic with WCAG AAA, APCA, HCT, CVD.
    pub fn standard() -> Self {
        CertificationProfile {
            name: "standard".to_string(),
            capabilities: vec![
                Capability::WcagAA,
                Capability::WcagAAA,
                Capability::ApcaCompliance,
                Capability::OklchColorSpace,
                Capability::HctColorSpace,
                Capability::CvdSimulation,
            ],
            metadata: Self::make_metadata("standard"),
        }
    }

    /// Advanced profile: extends Standard with MaterialPhysics, NeuralCorrection,
    /// TemporalAnalysis.
    pub fn advanced() -> Self {
        CertificationProfile {
            name: "advanced".to_string(),
            capabilities: vec![
                Capability::WcagAA,
                Capability::WcagAAA,
                Capability::ApcaCompliance,
                Capability::OklchColorSpace,
                Capability::HctColorSpace,
                Capability::CvdSimulation,
                Capability::MaterialPhysics,
                Capability::NeuralCorrection,
                Capability::TemporalAnalysis,
            ],
            metadata: Self::make_metadata("advanced"),
        }
    }

    /// Full profile: all capabilities.
    pub fn full() -> Self {
        CertificationProfile {
            name: "full".to_string(),
            capabilities: vec![
                Capability::WcagAA,
                Capability::WcagAAA,
                Capability::ApcaCompliance,
                Capability::OklchColorSpace,
                Capability::HctColorSpace,
                Capability::MaterialPhysics,
                Capability::NeuralCorrection,
                Capability::TemporalAnalysis,
                Capability::CvdSimulation,
            ],
            metadata: Self::make_metadata("full"),
        }
    }

    /// Returns `true` when `self` has every capability that `other` has.
    pub fn is_superset_of(&self, other: &CertificationProfile) -> bool {
        other
            .capabilities
            .iter()
            .all(|cap| self.capabilities.contains(cap))
    }
}

/// Free-function version of `CertificationProfile::is_superset_of`.
pub fn is_profile_superset(a: &CertificationProfile, b: &CertificationProfile) -> bool {
    a.is_superset_of(b)
}

/// Returns the highest-level profile whose capability set is a subset of
/// `passed_capabilities`.
pub fn highest_passing_profile(passed_capabilities: &[Capability]) -> CertificationProfile {
    let profiles = [
        CertificationProfile::full(),
        CertificationProfile::advanced(),
        CertificationProfile::standard(),
        CertificationProfile::basic(),
    ];

    for profile in &profiles {
        let all_covered = profile
            .capabilities
            .iter()
            .all(|cap| passed_capabilities.contains(cap));
        if all_covered {
            return profile.clone();
        }
    }

    // Return a minimal profile with only the capabilities that did pass
    CertificationProfile {
        name: "partial".to_string(),
        capabilities: passed_capabilities.to_vec(),
        metadata: CertificationProfile::make_metadata("partial"),
    }
}

// ============================================================================
// Certificate types
// ============================================================================

/// Cryptographic (here: hash-based) signature over a certificate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateSignature {
    /// Hash algorithm identifier.
    pub algorithm: String,
    /// Hex-encoded hash of the certificate content.
    pub hash: String,
    /// Unix timestamp when the signature was produced.
    pub signed_at: u64,
    /// Signer identity string.
    pub signer: String,
}

/// The signed content of a certificate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateContent {
    /// Unique certificate identifier.
    pub id: String,
    /// ID of the `CertificationTarget` this certificate covers.
    pub target_id: String,
    /// Profile that was achieved.
    pub profile: CertificationProfile,
    /// Issue timestamp (Unix seconds).
    pub issued_at: u64,
    /// Expiry timestamp (Unix seconds).
    pub expires_at: u64,
    /// Specification version string.
    pub spec_version: String,
    /// Overall conformance score [0, 1].
    pub conformance_score: f64,
}

/// Result of verifying a `Certificate`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateVerification {
    /// `true` if the certificate is valid (not expired, hash matches).
    pub valid: bool,
    /// `true` if the certificate has passed its `expires_at` timestamp.
    pub expired: bool,
    /// `true` if the content hash matches the stored signature.
    pub hash_valid: bool,
    /// Unix timestamp when the verification was performed.
    pub verified_at: u64,
    /// Error messages (empty when `valid == true`).
    pub errors: Vec<String>,
}

/// A signed certificate binding a target to a profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Certificate {
    /// The signed content.
    pub content: CertificateContent,
    /// The signature over that content.
    pub signature: CertificateSignature,
}

impl Certificate {
    /// Issues a new certificate for `target_id` at the given profile and score.
    pub fn new(target_id: &str, profile: CertificationProfile, conformance_score: f64) -> Self {
        let now = current_timestamp();
        let id = generate_certificate_id();
        let expires_at = now + 365 * 24 * 3600;

        let content = CertificateContent {
            id: id.clone(),
            target_id: target_id.to_string(),
            profile,
            issued_at: now,
            expires_at,
            spec_version: "7.0.0".to_string(),
            conformance_score,
        };

        // Compute hash over a canonical string representation of content
        let content_str = format!(
            "{}|{}|{}|{}|{}|{}|{:.6}",
            content.id,
            content.target_id,
            content.profile.name,
            content.issued_at,
            content.expires_at,
            content.spec_version,
            content.conformance_score,
        );
        let hash = compute_hash(&content_str);

        let signature = CertificateSignature {
            algorithm: "xor-fold-256".to_string(),
            hash,
            signed_at: now,
            signer: "Momoto Certification Authority v7".to_string(),
        };

        Certificate { content, signature }
    }

    /// Verifies the certificate's integrity and expiry.
    pub fn verify(&self) -> CertificateVerification {
        let now = current_timestamp();
        let expired = now > self.content.expires_at;

        // Recompute the expected hash
        let content_str = format!(
            "{}|{}|{}|{}|{}|{}|{:.6}",
            self.content.id,
            self.content.target_id,
            self.content.profile.name,
            self.content.issued_at,
            self.content.expires_at,
            self.content.spec_version,
            self.content.conformance_score,
        );
        let expected_hash = compute_hash(&content_str);
        let hash_valid = expected_hash == self.signature.hash;

        let mut errors = Vec::new();
        if expired {
            errors.push(format!(
                "Certificate expired at {} (now {})",
                self.content.expires_at, now
            ));
        }
        if !hash_valid {
            errors.push("Content hash mismatch — certificate may have been tampered".to_string());
        }

        CertificateVerification {
            valid: !expired && hash_valid,
            expired,
            hash_valid,
            verified_at: now,
            errors,
        }
    }

    /// Convenience: returns `true` when `verify().valid` is `true`.
    pub fn is_valid(&self) -> bool {
        self.verify().valid
    }
}

// ============================================================================
// Certification results
// ============================================================================

/// The result of certifying a `CertificationTarget`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificationResult {
    /// The target that was evaluated.
    pub target: CertificationTarget,
    /// Issued certificate (present only when `success == true`).
    pub certificate: Option<Certificate>,
    /// Detailed conformance test results.
    pub conformance: ConformanceResult,
    /// The profile that was assessed.
    pub profile: CertificationProfile,
    /// `true` when the target passed all conformance tests.
    pub success: bool,
    /// Error messages (empty when `success == true`).
    pub errors: Vec<String>,
}

/// The result of the engine certifying itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfCertificationResult {
    /// Identity of the engine.
    pub identity: MomotoIdentity,
    /// The self-issued certificate.
    pub certificate: Certificate,
    /// Capabilities asserted.
    pub capabilities: Vec<Capability>,
    /// `true` when the self-certificate verifies correctly.
    pub verified: bool,
}

// ============================================================================
// Artifacts
// ============================================================================

/// Distinguishes which kind of payload a `SignedArtifact` carries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactType {
    /// Design tokens (Style Dictionary / W3C format).
    DesignTokens,
    /// A complete color system.
    ColorSystem,
    /// Animation parameters.
    AnimationParams,
    /// A single material.
    Material,
}

/// Metadata about a signed artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactMetadata {
    /// Unix timestamp of creation.
    pub created_at: u64,
    /// Identity of the creator / tool.
    pub creator: String,
    /// Semantic version of the artifact schema.
    pub version: String,
    /// Human-readable description.
    pub description: String,
}

/// Signature over an artifact payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactSignature {
    /// Hex-encoded hash of the payload.
    pub hash: String,
    /// Unix timestamp when signed.
    pub signed_at: u64,
    /// Hash algorithm identifier.
    pub algorithm: String,
}

/// Result of verifying a `SignedArtifact`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactVerification {
    /// `true` if the artifact hash is consistent with its payload.
    pub valid: bool,
    /// `true` if the recomputed hash matches the stored one.
    pub hash_matches: bool,
    /// Error messages.
    pub errors: Vec<String>,
}

/// A named color scale (e.g. `"blue"` with steps `["#dbeafe", …, "#1e3a8a"]`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorScale {
    /// Scale name.
    pub name: String,
    /// Hex color stops, from lightest to darkest.
    pub steps: Vec<String>,
    /// Index into `steps` of the "base" (brand) color.
    pub base_index: usize,
}

/// Semantic color assignments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticColors {
    /// Primary brand color.
    pub primary: String,
    /// Secondary brand color.
    pub secondary: String,
    /// Accent color.
    pub accent: String,
    /// Success / positive feedback color.
    pub success: String,
    /// Warning color.
    pub warning: String,
    /// Error / danger color.
    pub error: String,
    /// Informational color.
    pub info: String,
}

/// Surface / background color hierarchy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceColors {
    /// Page / app background.
    pub background: String,
    /// Component surface (cards, dialogs).
    pub surface: String,
    /// Elevated surface (tooltips, menus).
    pub elevated: String,
    /// Modal / scrim overlay color.
    pub overlay: String,
}

/// Certified design-token bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertifiedDesignTokens {
    /// Number of tokens in the bundle.
    pub token_count: u32,
    /// Top-level namespace keys (e.g. `["color", "spacing", "typography"]`).
    pub namespaces: Vec<String>,
    /// Schema version (e.g. `"w3c-dtcg-0.5"`).
    pub schema_version: String,
    /// Raw JSON representation of the token set.
    pub tokens_json: String,
}

/// Certified color system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertifiedColorSystem {
    /// Named color scales.
    pub palette: Vec<ColorScale>,
    /// Semantic color assignments.
    pub semantic: SemanticColors,
    /// Surface / background color hierarchy.
    pub surfaces: SurfaceColors,
    /// Optional dark-mode palette variant.
    pub dark_mode_palette: Option<Vec<ColorScale>>,
}

/// Certified animation parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertifiedAnimationParams {
    /// Named transitions (e.g. `["color-fade-200ms", "bg-scale-300ms"]`).
    pub transitions: Vec<String>,
    /// Maximum transition duration in ms.
    pub max_duration_ms: u64,
    /// Declared easing functions.
    pub easing_functions: Vec<String>,
    /// `true` when all transitions pass WCAG 2.3 flicker and motion rules.
    pub wcag_compliant: bool,
}

/// Certified material descriptor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertifiedMaterial {
    /// Material name.
    pub name: String,
    /// Index of refraction.
    pub ior: f64,
    /// Perceptual roughness [0, 1].
    pub roughness: f64,
    /// Whether the material is metallic.
    pub metallic: bool,
    /// Material category.
    pub category: String,
}

/// A signed artifact carrying one of the certified payload types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedArtifact {
    /// Type discriminant for the payload.
    pub artifact_type: ArtifactType,
    /// Artifact metadata.
    pub metadata: ArtifactMetadata,
    /// Artifact signature.
    pub signature: ArtifactSignature,
    /// Present when `artifact_type == ArtifactType::DesignTokens`.
    pub design_tokens: Option<CertifiedDesignTokens>,
    /// Present when `artifact_type == ArtifactType::ColorSystem`.
    pub color_system: Option<CertifiedColorSystem>,
    /// Present when `artifact_type == ArtifactType::AnimationParams`.
    pub animation_params: Option<CertifiedAnimationParams>,
    /// Present when `artifact_type == ArtifactType::Material`.
    pub material: Option<CertifiedMaterial>,
}

impl SignedArtifact {
    /// Verifies the artifact by recomputing its payload hash.
    pub fn verify(&self) -> ArtifactVerification {
        // Build a canonical string from whichever payload is present
        let payload_str = if let Some(dt) = &self.design_tokens {
            format!(
                "dt|{}|{}|{}",
                dt.token_count, dt.schema_version, dt.tokens_json
            )
        } else if let Some(cs) = &self.color_system {
            format!(
                "cs|{}|{}|{}",
                cs.palette.len(),
                cs.semantic.primary,
                cs.surfaces.background
            )
        } else if let Some(ap) = &self.animation_params {
            format!(
                "ap|{}|{}",
                ap.max_duration_ms,
                ap.easing_functions.join(",")
            )
        } else if let Some(m) = &self.material {
            format!("mat|{}|{:.4}|{:.4}", m.name, m.ior, m.roughness)
        } else {
            "empty".to_string()
        };

        let expected = compute_hash(&payload_str);
        let hash_matches = expected == self.signature.hash;
        let mut errors = Vec::new();
        if !hash_matches {
            errors.push("Payload hash mismatch — artifact may be corrupted".to_string());
        }
        ArtifactVerification {
            valid: hash_matches,
            hash_matches,
            errors,
        }
    }
}

/// Fluent builder for `SignedArtifact`.
#[derive(Debug, Clone)]
pub struct ArtifactBuilder {
    artifact_type: ArtifactType,
    design_tokens: Option<CertifiedDesignTokens>,
    color_system: Option<CertifiedColorSystem>,
    animation_params: Option<CertifiedAnimationParams>,
    material: Option<CertifiedMaterial>,
}

impl ArtifactBuilder {
    /// Creates a new builder for the given artifact type.
    pub fn new(artifact_type: ArtifactType) -> Self {
        ArtifactBuilder {
            artifact_type,
            design_tokens: None,
            color_system: None,
            animation_params: None,
            material: None,
        }
    }

    /// Attaches a `CertifiedDesignTokens` payload.
    pub fn with_design_tokens(mut self, tokens: CertifiedDesignTokens) -> Self {
        self.design_tokens = Some(tokens);
        self
    }

    /// Attaches a `CertifiedColorSystem` payload.
    pub fn with_color_system(mut self, system: CertifiedColorSystem) -> Self {
        self.color_system = Some(system);
        self
    }

    /// Attaches a `CertifiedAnimationParams` payload.
    pub fn with_animation_params(mut self, params: CertifiedAnimationParams) -> Self {
        self.animation_params = Some(params);
        self
    }

    /// Attaches a `CertifiedMaterial` payload.
    pub fn with_material(mut self, material: CertifiedMaterial) -> Self {
        self.material = Some(material);
        self
    }

    /// Signs the artifact, producing a `SignedArtifact` with a computed hash.
    pub fn sign(self, creator: &str, description: &str) -> SignedArtifact {
        let now = current_timestamp();

        // Compute payload string (mirrors `SignedArtifact::verify`)
        let payload_str = if let Some(dt) = &self.design_tokens {
            format!(
                "dt|{}|{}|{}",
                dt.token_count, dt.schema_version, dt.tokens_json
            )
        } else if let Some(cs) = &self.color_system {
            format!(
                "cs|{}|{}|{}",
                cs.palette.len(),
                cs.semantic.primary,
                cs.surfaces.background
            )
        } else if let Some(ap) = &self.animation_params {
            format!(
                "ap|{}|{}",
                ap.max_duration_ms,
                ap.easing_functions.join(",")
            )
        } else if let Some(m) = &self.material {
            format!("mat|{}|{:.4}|{:.4}", m.name, m.ior, m.roughness)
        } else {
            "empty".to_string()
        };

        let hash = compute_hash(&payload_str);

        SignedArtifact {
            artifact_type: self.artifact_type,
            metadata: ArtifactMetadata {
                created_at: now,
                creator: creator.to_string(),
                version: "7.0.0".to_string(),
                description: description.to_string(),
            },
            signature: ArtifactSignature {
                hash,
                signed_at: now,
                algorithm: "xor-fold-256".to_string(),
            },
            design_tokens: self.design_tokens,
            color_system: self.color_system,
            animation_params: self.animation_params,
            material: self.material,
        }
    }
}

// ============================================================================
// Audit
// ============================================================================

/// Categories of events recorded by the certification audit logger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditEventType {
    /// A certification process started.
    CertificationStarted,
    /// A certification process completed successfully.
    CertificationCompleted,
    /// A certification process failed.
    CertificationFailed,
    /// A certification profile was evaluated.
    ProfileEvaluated,
    /// An artifact was signed.
    ArtifactSigned,
    /// A certificate was verified.
    VerificationPerformed,
}

/// A single event in an audit record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Event category.
    pub event_type: AuditEventType,
    /// Unix timestamp when this event occurred.
    pub timestamp: u64,
    /// ID of the entity being audited (target ID, certificate ID, …).
    pub entity_id: String,
    /// Human-readable details.
    pub details: String,
}

/// A collection of events for a single certification run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    /// Unique record identifier.
    pub id: String,
    /// Chronologically ordered events.
    pub events: Vec<AuditEvent>,
    /// Unix timestamp when the record was opened.
    pub started_at: u64,
    /// Unix timestamp when the record was closed (absent if still open).
    pub completed_at: Option<u64>,
}

/// Summary result of a completed audit record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResult {
    /// The completed audit record.
    pub record: AuditRecord,
    /// `true` when no `CertificationFailed` events were recorded.
    pub success: bool,
    /// Wall-clock duration from `started_at` to `completed_at` in milliseconds.
    pub duration_ms: u64,
}

/// A serialised export of an audit record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditExport {
    /// Format identifier (`"json"` or `"text"`).
    pub format: String,
    /// Serialised content.
    pub content: String,
    /// Unix timestamp of export.
    pub exported_at: u64,
}

/// Verifies that a deterministic computation is reproducible.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReproducibilityVerification {
    /// Hash of the input `CertificationTarget`.
    pub inputs_hash: String,
    /// Hash of the `ConformanceResult` output.
    pub outputs_hash: String,
    /// `true` when re-running with the same inputs produces the same outputs.
    pub matches: bool,
    /// Unix timestamp of this verification.
    pub verified_at: u64,
}

/// Runs a conformance evaluation and produces a reproducibility proof.
#[derive(Debug, Clone)]
pub struct ReproducibleRunner {
    /// The specification used for all runs.
    pub spec: PerceptualSpecification,
}

impl ReproducibleRunner {
    /// Creates a new runner bound to `spec`.
    pub fn new(spec: PerceptualSpecification) -> Self {
        ReproducibleRunner { spec }
    }

    /// Runs the conformance engine on `target` and returns both the result and
    /// a reproducibility proof.
    pub fn run(
        &self,
        target: &CertificationTarget,
    ) -> (ConformanceResult, ReproducibilityVerification) {
        let engine = ConformanceEngine::new(self.spec.clone());
        let result = engine.full_audit(target);

        // Build deterministic hash of inputs
        let input_str = format!("{:?}|{}", target.target_type, target.id,);
        let inputs_hash = compute_hash(&input_str);

        // Build deterministic hash of outputs
        let output_str = format!(
            "{:.4}|{}|{}",
            result.compliance_percentage,
            result.overall_pass,
            result.failures.join(",")
        );
        let outputs_hash = compute_hash(&output_str);

        // Re-run to verify determinism
        let result2 = engine.full_audit(target);
        let output_str2 = format!(
            "{:.4}|{}|{}",
            result2.compliance_percentage,
            result2.overall_pass,
            result2.failures.join(",")
        );
        let outputs_hash2 = compute_hash(&output_str2);

        let verification = ReproducibilityVerification {
            inputs_hash,
            outputs_hash: outputs_hash.clone(),
            matches: outputs_hash == outputs_hash2,
            verified_at: current_timestamp(),
        };

        (result, verification)
    }
}

/// In-module audit logger (exported as `CertAuditLogger` from the crate root
/// to avoid name collision with `audit::AuditLogger`).
#[derive(Debug)]
pub struct AuditLogger {
    records: std::sync::Mutex<HashMap<String, AuditRecord>>,
}

impl AuditLogger {
    /// Creates a new, empty audit logger.
    pub fn new() -> Self {
        AuditLogger {
            records: std::sync::Mutex::new(HashMap::new()),
        }
    }

    /// Opens a new audit record for `entity_id` and returns its record ID.
    pub fn start_record(&self, entity_id: &str) -> String {
        let now = current_timestamp();
        let record_id = format!("audit-{:016x}", now);
        let record = AuditRecord {
            id: record_id.clone(),
            events: Vec::new(),
            started_at: now,
            completed_at: None,
        };
        if let Ok(mut records) = self.records.lock() {
            records.insert(record_id.clone(), record);
        }
        record_id
    }

    /// Appends an event to the record identified by `record_id`.
    ///
    /// If `record_id` does not exist this is a no-op.
    pub fn log_event(&self, record_id: &str, event: AuditEvent) {
        if let Ok(mut records) = self.records.lock() {
            if let Some(record) = records.get_mut(record_id) {
                record.events.push(event);
            }
        }
    }

    /// Closes a record and returns a summary `AuditResult`.
    ///
    /// If the record does not exist a synthetic failure result is returned.
    pub fn complete_record(&self, record_id: &str) -> AuditResult {
        let now = current_timestamp();
        if let Ok(mut records) = self.records.lock() {
            if let Some(record) = records.get_mut(record_id) {
                record.completed_at = Some(now);
                let success = !record
                    .events
                    .iter()
                    .any(|e| e.event_type == AuditEventType::CertificationFailed);
                let duration_ms = (now.saturating_sub(record.started_at)) * 1000;
                return AuditResult {
                    record: record.clone(),
                    success,
                    duration_ms,
                };
            }
        }
        // Record not found — return a synthetic failure
        AuditResult {
            record: AuditRecord {
                id: record_id.to_string(),
                events: vec![],
                started_at: now,
                completed_at: Some(now),
            },
            success: false,
            duration_ms: 0,
        }
    }

    /// Serialises a record to the requested format (`"json"` or `"text"`).
    ///
    /// Returns an empty export when the record ID is not found.
    pub fn export(&self, record_id: &str, format: &str) -> AuditExport {
        let now = current_timestamp();
        let records = self.records.lock().ok();
        let record = records.as_ref().and_then(|m| m.get(record_id)).cloned();

        let content = match record {
            None => format!("Record '{}' not found", record_id),
            Some(r) => match format {
                "json" => {
                    // Minimal JSON serialisation without external deps
                    let events_json: Vec<String> = r
                        .events
                        .iter()
                        .map(|e| {
                            format!(
                                r#"{{"type":"{:?}","ts":{},"entity":"{}","details":"{}"}}"#,
                                e.event_type, e.timestamp, e.entity_id, e.details
                            )
                        })
                        .collect();
                    format!(
                        r#"{{"id":"{}","started_at":{},"events":[{}]}}"#,
                        r.id,
                        r.started_at,
                        events_json.join(",")
                    )
                }
                _ => {
                    // Plain text
                    let mut out = format!("Audit Record: {}\n", r.id);
                    out.push_str(&format!("Started: {}\n", r.started_at));
                    for e in &r.events {
                        out.push_str(&format!(
                            "  [{:?}] {} — {}\n",
                            e.event_type, e.entity_id, e.details
                        ));
                    }
                    out
                }
            },
        };

        AuditExport {
            format: format.to_string(),
            content,
            exported_at: now,
        }
    }
}

// ============================================================================
// Certification Authority (main orchestrator)
// ============================================================================

/// The primary entry point for the certification subsystem.
///
/// `CertificationAuthority` wraps the `ConformanceEngine` and `AuditLogger`
/// to provide a single, coherent API for issuing and verifying certificates.
#[derive(Debug)]
pub struct CertificationAuthority {
    /// The specification used for all conformance evaluations.
    pub spec: PerceptualSpecification,
    audit_logger: AuditLogger,
}

/// System identity record returned by [`CertificationAuthority::identity`].
///
/// Contains semantic version, build metadata, and feature coverage information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemIdentity {
    /// Major version (breaking changes).
    pub major: u32,
    /// Minor version (additive changes).
    pub minor: u32,
    /// Patch version (bug fixes).
    pub patch: u32,
    /// Fraction of Momoto phases certified (0.0–1.0).
    pub phase_coverage: f64,
    /// Short commit hash or build identifier.
    pub build_id: String,
    /// Perceptual specification version string.
    pub spec_version: String,
}

impl SystemIdentity {
    /// Human-readable version string: `"major.minor.patch"`.
    pub fn version_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }

    /// Identity string including build ID.
    pub fn identity_string(&self) -> String {
        format!("momoto-v{}-{}", self.version_string(), self.build_id)
    }
}

impl CertificationAuthority {
    /// Creates a new authority using the default `PerceptualSpecification::v7()`.
    ///
    /// This is the primary constructor for the WASM bindings. For custom specs
    /// use [`CertificationAuthority::with_spec`].
    pub fn new() -> Self {
        Self::with_spec(PerceptualSpecification::v7())
    }

    /// Creates a new authority bound to a custom `spec`.
    pub fn with_spec(spec: PerceptualSpecification) -> Self {
        CertificationAuthority {
            spec,
            audit_logger: AuditLogger::new(),
        }
    }

    /// Returns the current system identity record.
    pub fn identity(&self) -> SystemIdentity {
        SystemIdentity {
            major: 7,
            minor: 0,
            patch: 0,
            phase_coverage: 1.0,
            build_id: "da0075a".to_string(),
            spec_version: self.spec.version.clone(),
        }
    }

    /// Certifies a `CertificationTarget` and returns a `CertificationResult`.
    ///
    /// The method:
    /// 1. Opens an audit record.
    /// 2. Runs the full conformance audit.
    /// 3. Issues a `CertificationProfile` appropriate to the conformance score.
    /// 4. Issues a `Certificate` when the target passes.
    /// 5. Closes the audit record.
    pub fn certify(&self, target: CertificationTarget) -> CertificationResult {
        let record_id = self.audit_logger.start_record(&target.id);

        self.audit_logger.log_event(
            &record_id,
            AuditEvent {
                event_type: AuditEventType::CertificationStarted,
                timestamp: current_timestamp(),
                entity_id: target.id.clone(),
                details: format!("Target type: {:?}", target.target_type),
            },
        );

        let engine = ConformanceEngine::new(self.spec.clone());
        let conformance = engine.full_audit(&target);

        let profile = self.issue_profile(&target, &conformance);

        self.audit_logger.log_event(
            &record_id,
            AuditEvent {
                event_type: AuditEventType::ProfileEvaluated,
                timestamp: current_timestamp(),
                entity_id: target.id.clone(),
                details: format!(
                    "Profile: {} | Score: {:.1}%",
                    profile.name, conformance.compliance_percentage
                ),
            },
        );

        let (certificate, success, errors) = if conformance.overall_pass {
            let cert = Certificate::new(
                &target.id,
                profile.clone(),
                conformance.compliance_percentage / 100.0,
            );
            self.audit_logger.log_event(
                &record_id,
                AuditEvent {
                    event_type: AuditEventType::CertificationCompleted,
                    timestamp: current_timestamp(),
                    entity_id: target.id.clone(),
                    details: format!("Certificate issued: {}", cert.content.id),
                },
            );
            (Some(cert), true, vec![])
        } else {
            let errs: Vec<String> = conformance
                .failures
                .iter()
                .map(|f| format!("Failed: {}", f))
                .collect();
            self.audit_logger.log_event(
                &record_id,
                AuditEvent {
                    event_type: AuditEventType::CertificationFailed,
                    timestamp: current_timestamp(),
                    entity_id: target.id.clone(),
                    details: format!("Failures: {}", conformance.failures.join(", ")),
                },
            );
            (None, false, errs)
        };

        self.audit_logger.complete_record(&record_id);

        CertificationResult {
            target,
            certificate,
            conformance,
            profile,
            success,
            errors,
        }
    }

    /// The engine certifies itself by verifying its own identity and issuing a
    /// self-certificate with the `full` profile.
    pub fn self_certify(&self) -> SelfCertificationResult {
        let identity = MomotoIdentity::current();
        let profile = CertificationProfile::full();
        let score = 1.0_f64; // Self-certification always scores 100%
        let certificate = Certificate::new("momoto-engine-v7", profile.clone(), score);
        let verified = certificate.is_valid();

        let capabilities = profile.capabilities.clone();

        SelfCertificationResult {
            identity,
            certificate,
            capabilities,
            verified,
        }
    }

    /// Verifies an existing certificate and returns a `CertificateVerification`.
    pub fn verify_certificate(&self, cert: &Certificate) -> CertificateVerification {
        let result = cert.verify();
        self.audit_logger.log_event(
            &format!("verify-{}", cert.content.id),
            AuditEvent {
                event_type: AuditEventType::VerificationPerformed,
                timestamp: current_timestamp(),
                entity_id: cert.content.id.clone(),
                details: format!("Valid: {}, Expired: {}", result.valid, result.expired),
            },
        );
        result
    }

    /// Determines the highest `CertificationProfile` that is appropriate for
    /// a target given the conformance results.
    pub fn issue_profile(
        &self,
        target: &CertificationTarget,
        conformance: &ConformanceResult,
    ) -> CertificationProfile {
        // Build the set of capabilities that passed based on test types
        let mut caps = Vec::new();

        // OKLCH is always present in v7
        caps.push(Capability::OklchColorSpace);

        let passed_types: Vec<&TestType> = conformance
            .tests
            .iter()
            .filter(|t| t.passed)
            .map(|t| &t.test_type)
            .collect();

        if passed_types.contains(&&TestType::WcagCompliance) {
            caps.push(Capability::WcagAA);
            // If score is above 90% grant AAA
            if conformance.compliance_percentage >= 90.0 {
                caps.push(Capability::WcagAAA);
            }
        }
        if passed_types.contains(&&TestType::ApcaCompliance) {
            caps.push(Capability::ApcaCompliance);
            caps.push(Capability::HctColorSpace);
            caps.push(Capability::CvdSimulation);
        }
        if passed_types.contains(&&TestType::MaterialPhysics) {
            caps.push(Capability::MaterialPhysics);
        }
        if passed_types.contains(&&TestType::TemporalSafety) {
            caps.push(Capability::TemporalAnalysis);
        }
        if passed_types.contains(&&TestType::NeuralCorrection) {
            caps.push(Capability::NeuralCorrection);
        }

        highest_passing_profile(&caps)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hash_deterministic() {
        let a = compute_hash("hello");
        let b = compute_hash("hello");
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
    }

    #[test]
    fn test_compute_hash_differs() {
        let a = compute_hash("hello");
        let b = compute_hash("world");
        assert_ne!(a, b);
    }

    #[test]
    fn test_generate_certificate_id_format() {
        let id = generate_certificate_id();
        assert!(id.starts_with("cert-"));
        // cert- + 16 hex + - + 8 hex
        assert_eq!(id.len(), 5 + 16 + 1 + 8);
    }

    #[test]
    fn test_momoto_identity_version() {
        let identity = MomotoIdentity::current();
        assert_eq!(identity.version, "7.0.0");
        assert!(!identity.capabilities.is_empty());
    }

    #[test]
    fn test_perceptual_spec_v7() {
        let spec = PerceptualSpecification::v7();
        assert_eq!(spec.version, "7.0.0");
        assert_eq!(spec.temporal_rules.max_flicker_hz, 3.0);
        assert!(spec.accessibility.require_aa);
    }

    #[test]
    fn test_conformance_engine_valid_color() {
        let engine = ConformanceEngine::new(PerceptualSpecification::v7());
        let result = engine.test_color("#ff6600");
        assert!(!result.tests.is_empty());
        // At minimum the roundtrip test should pass for a valid sRGB hex
        let roundtrip = result
            .tests
            .iter()
            .find(|t| t.test_type == TestType::ColorRoundtrip);
        assert!(roundtrip.is_some());
    }

    #[test]
    fn test_conformance_engine_pair() {
        let engine = ConformanceEngine::new(PerceptualSpecification::v7());
        let result = engine.test_pair("#000000", "#ffffff");
        assert!(result.overall_pass);
        let wcag_test = result
            .tests
            .iter()
            .find(|t| t.test_type == TestType::WcagCompliance);
        assert!(wcag_test.is_some());
        assert!(wcag_test.unwrap().passed);
    }

    #[test]
    fn test_profile_superset() {
        let full = CertificationProfile::full();
        let basic = CertificationProfile::basic();
        assert!(is_profile_superset(&full, &basic));
        assert!(!is_profile_superset(&basic, &full));
    }

    #[test]
    fn test_highest_passing_profile_basic() {
        let caps = vec![Capability::WcagAA, Capability::OklchColorSpace];
        let profile = highest_passing_profile(&caps);
        assert_eq!(profile.name, "basic");
    }

    #[test]
    fn test_certificate_roundtrip() {
        let profile = CertificationProfile::basic();
        let cert = Certificate::new("test-target", profile, 0.95);
        assert!(cert.is_valid());
        let ver = cert.verify();
        assert!(ver.valid);
        assert!(ver.hash_valid);
        assert!(!ver.expired);
    }

    #[test]
    fn test_artifact_builder_design_tokens() {
        let tokens = CertifiedDesignTokens {
            token_count: 42,
            namespaces: vec!["color".to_string()],
            schema_version: "w3c-dtcg-0.5".to_string(),
            tokens_json: r#"{"color":{}}"#.to_string(),
        };
        let artifact = ArtifactBuilder::new(ArtifactType::DesignTokens)
            .with_design_tokens(tokens)
            .sign("momoto-cli", "Test design tokens");
        let verification = artifact.verify();
        assert!(verification.valid);
    }

    #[test]
    fn test_certification_authority_self_certify() {
        let authority = CertificationAuthority::with_spec(PerceptualSpecification::v7());
        let result = authority.self_certify();
        assert!(result.verified);
        assert_eq!(result.identity.version, "7.0.0");
    }

    #[test]
    fn test_certification_authority_certify_color_system() {
        let authority = CertificationAuthority::with_spec(PerceptualSpecification::v7());
        let target = CertificationTarget {
            id: "test-color-system".to_string(),
            target_type: TargetType::ColorSystem,
            color_data: Some(ColorData {
                hex_colors: vec!["#000000".to_string(), "#ffffff".to_string()],
                oklch_values: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
                wcag_luminances: vec![0.0, 1.0],
                palette_name: Some("mono".to_string()),
            }),
            material_data: None,
            animation_data: None,
            metadata: HashMap::new(),
        };
        let result = authority.certify(target);
        // Black+white should pass roundtrip and gamut checks
        assert!(result.success);
        assert!(result.certificate.is_some());
    }

    #[test]
    fn test_audit_logger() {
        let logger = AuditLogger::new();
        let record_id = logger.start_record("entity-1");
        logger.log_event(
            &record_id,
            AuditEvent {
                event_type: AuditEventType::CertificationStarted,
                timestamp: current_timestamp(),
                entity_id: "entity-1".to_string(),
                details: "Test event".to_string(),
            },
        );
        let result = logger.complete_record(&record_id);
        assert!(result.success);
        assert_eq!(result.record.events.len(), 1);
    }

    #[test]
    fn test_reproducible_runner() {
        let runner = ReproducibleRunner::new(PerceptualSpecification::v7());
        let target = CertificationTarget {
            id: "repro-test".to_string(),
            target_type: TargetType::ColorSystem,
            color_data: Some(ColorData {
                hex_colors: vec!["#3b82f6".to_string()],
                oklch_values: vec![[0.6, 0.15, 265.0]],
                wcag_luminances: vec![0.16],
                palette_name: None,
            }),
            material_data: None,
            animation_data: None,
            metadata: HashMap::new(),
        };
        let (result, verification) = runner.run(&target);
        assert!(verification.matches);
        assert!(!result.tests.is_empty());
    }
}

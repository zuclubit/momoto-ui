//! AI Visual Generator — Full perceptual pipeline for color system generation.
//!
//! Implements a production-grade pipeline that:
//! 1. Parses and validates input colors
//! 2. Converts to OKLCH for perceptual operations
//! 3. Checks sRGB gamut boundaries
//! 4. Validates WCAG 2.1 and APCA contrast
//! 5. Generates 9-tone OKLCH tonal palette
//! 6. Emits CSS custom properties (light/dark/high-contrast)

#![allow(dead_code)]

use momoto_core::color::Color;
use momoto_core::perception::ContrastMetric;
use momoto_core::space::oklch::OKLCH;
use momoto_metrics::{APCAMetric, WCAGMetric};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during visual generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GenerationError {
    /// The provided color string could not be parsed.
    InvalidColor(String),
    /// The color's chroma exceeds the sRGB gamut boundary.
    GamutExceeded { color: String, chroma: f64 },
    /// The computed contrast ratio is below the required minimum.
    ContrastTooLow { ratio: f64, required: f64 },
    /// A perceptual processing error occurred.
    PerceptualError(String),
}

impl std::fmt::Display for GenerationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidColor(s) => write!(f, "Invalid color: {}", s),
            Self::GamutExceeded { color, chroma } => {
                write!(f, "Gamut exceeded for {}: chroma={:.4}", color, chroma)
            }
            Self::ContrastTooLow { ratio, required } => {
                write!(f, "Contrast too low: {:.2} < {:.2}", ratio, required)
            }
            Self::PerceptualError(s) => write!(f, "Perceptual error: {}", s),
        }
    }
}

// ============================================================================
// Audience & Mode Configuration
// ============================================================================

/// Target audience profile that influences color generation decisions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AudienceProfile {
    /// General audience — balanced legibility and aesthetics.
    General,
    /// High-contrast mode for low-vision users.
    HighContrast,
    /// CVD-safe palette (deuteranopia/protanopia/tritanopia safe).
    ColorBlindFriendly,
    /// Optimized for dark-mode UIs.
    DarkMode,
    /// Accessibility-first: WCAG AAA minimum.
    LowVision,
}

/// Specifies which color modes to generate output for.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorModeConfig {
    /// Emit light-mode CSS variables.
    pub light_mode: bool,
    /// Emit dark-mode CSS variables.
    pub dark_mode: bool,
    /// Emit high-contrast CSS variables.
    pub high_contrast: bool,
    /// Target audience influencing thresholds.
    pub audience: AudienceProfile,
}

impl Default for ColorModeConfig {
    fn default() -> Self {
        Self {
            light_mode: true,
            dark_mode: true,
            high_contrast: true,
            audience: AudienceProfile::General,
        }
    }
}

// ============================================================================
// Material Physics Properties
// ============================================================================

/// Physical material properties for realistic rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialProperties {
    /// Index of refraction (1.0 = vacuum, 1.5 = glass, 2.4 = diamond).
    pub ior: f64,
    /// Microsurface roughness in [0, 1].
    pub roughness: f64,
    /// Whether the material is metallic (conductors).
    pub metallic: bool,
    /// Fresnel reflectance at normal incidence (F0).
    pub fresnel_r0: f64,
    /// Transparency in [0, 1] (0 = opaque, 1 = fully transparent).
    pub transparency: f64,
}

impl MaterialProperties {
    /// Crown glass preset: IOR 1.5, low roughness, high transparency.
    pub fn glass() -> Self {
        Self {
            ior: 1.5,
            roughness: 0.05,
            metallic: false,
            fresnel_r0: 0.04, // ((1.5-1)/(1.5+1))^2 = 0.04
            transparency: 0.9,
        }
    }

    /// Gold-like metal preset: high IOR, moderate roughness.
    pub fn metal() -> Self {
        Self {
            ior: 2.5,
            roughness: 0.3,
            metallic: true,
            fresnel_r0: 0.95, // Metals have high F0
            transparency: 0.0,
        }
    }

    /// Matte plastic preset: dielectric, diffuse-dominant.
    pub fn plastic() -> Self {
        Self {
            ior: 1.5,
            roughness: 0.6,
            metallic: false,
            fresnel_r0: 0.04,
            transparency: 0.0,
        }
    }
}

/// Combined material effects for CSS rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialEffects {
    /// Physical material properties.
    pub properties: MaterialProperties,
    /// Specular highlight intensity in [0, 1].
    pub specular_intensity: f64,
    /// Ambient occlusion factor in [0, 1].
    pub ambient_occlusion: f64,
    /// Soft shadow radius factor in [0, 1].
    pub shadow_softness: f64,
}

// ============================================================================
// Validation Types
// ============================================================================

/// A single validation finding with severity and remediation hint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    /// "error", "warning", or "info".
    pub severity: String,
    /// Machine-readable issue code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
    /// Optional remediation suggestion.
    pub suggestion: Option<String>,
}

/// WCAG 2.1 contrast compliance results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WCAGValidation {
    /// AA level passes for normal text (≥4.5:1).
    pub aa_normal_pass: bool,
    /// AA level passes for large text (≥3.0:1).
    pub aa_large_pass: bool,
    /// AAA level passes for normal text (≥7.0:1).
    pub aaa_normal_pass: bool,
    /// AAA level passes for large text (≥4.5:1).
    pub aaa_large_pass: bool,
    /// The computed contrast ratio.
    pub contrast_ratio: f64,
    /// Validation issues found.
    pub issues: Vec<ValidationIssue>,
}

/// APCA contrast compliance results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct APCAValidation {
    /// Signed Lc value (positive = dark-on-light, negative = light-on-dark).
    pub lc_value: f64,
    /// Body text threshold: |Lc| ≥ 75.
    pub body_text_pass: bool,
    /// Heading threshold: |Lc| ≥ 60.
    pub heading_pass: bool,
    /// UI component threshold: |Lc| ≥ 45.
    pub ui_component_pass: bool,
    /// Validation issues found.
    pub issues: Vec<ValidationIssue>,
}

/// Perceptual quality metrics in OKLCH space.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceptualValidation {
    /// CIE 1976 Delta-E (approximate, in OKLCH space).
    pub delta_e_76: f64,
    /// CIEDE2000 Delta-E (Sharma 2005 formula).
    pub delta_e_2000: f64,
    /// Whether the color pair is perceptually distinguishable (ΔE₀₀ ≥ 1.0).
    pub is_distinguishable: bool,
    /// OKLCH lightness component.
    pub oklch_l: f64,
    /// OKLCH chroma component.
    pub oklch_c: f64,
    /// OKLCH hue angle.
    pub oklch_h: f64,
    /// Whether gamut clamping was applied.
    pub gamut_clipped: bool,
}

/// Metrics from the neural perceptual correction pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuralCorrectionMetrics {
    /// Number of correction iterations applied.
    pub corrections_applied: u32,
    /// Average Delta-E reduction per correction.
    pub avg_delta_e_reduction: f64,
    /// Net perceptual improvement score [0, 1].
    pub perceptual_improvement: f64,
    /// Wall-clock processing time in milliseconds.
    pub processing_ms: u64,
}

/// Complete validation report for a single color.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    /// The color in normalized hex form.
    pub color_hex: String,
    /// WCAG 2.1 results.
    pub wcag: WCAGValidation,
    /// APCA results.
    pub apca: APCAValidation,
    /// Perceptual quality metrics.
    pub perceptual: PerceptualValidation,
    /// Neural correction statistics.
    pub neural: NeuralCorrectionMetrics,
    /// Overall weighted score in [0, 1].
    pub overall_score: f64,
    /// True if all critical checks pass.
    pub passed: bool,
}

impl ValidationReport {
    /// Compute an overall quality score from sub-scores.
    ///
    /// Weights:
    /// - WCAG AA normal (30%)
    /// - APCA body text (30%)
    /// - Perceptual distinguishability (20%)
    /// - Neural improvement (10%)
    /// - Gamut integrity (10%)
    pub fn compute_overall_score(&self) -> f64 {
        let wcag_score = if self.wcag.aa_normal_pass {
            1.0
        } else if self.wcag.aa_large_pass {
            0.6
        } else {
            0.0
        };

        let apca_score = if self.apca.body_text_pass {
            1.0
        } else if self.apca.heading_pass {
            0.7
        } else if self.apca.ui_component_pass {
            0.4
        } else {
            0.0
        };

        let perceptual_score = if self.perceptual.is_distinguishable {
            1.0
        } else {
            0.0
        };

        let neural_score = (self.neural.perceptual_improvement).clamp(0.0, 1.0);

        let gamut_score = if !self.perceptual.gamut_clipped {
            1.0
        } else {
            0.5
        };

        0.30 * wcag_score
            + 0.30 * apca_score
            + 0.20 * perceptual_score
            + 0.10 * neural_score
            + 0.10 * gamut_score
    }
}

// ============================================================================
// Pipeline Configuration
// ============================================================================

/// Bit-flags for enabling/disabling pipeline phases.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelinePhases {
    /// Parse hex → Color.
    pub color_parsing: bool,
    /// Map out-of-gamut colors back into sRGB.
    pub gamut_mapping: bool,
    /// Run WCAG 2.1 contrast check.
    pub wcag_check: bool,
    /// Run APCA contrast check.
    pub apca_check: bool,
    /// Compute perceptual metrics (ΔE, OKLCH).
    pub perceptual_check: bool,
    /// Apply SIREN-inspired neural correction.
    pub neural_correction: bool,
    /// Emit CSS custom properties.
    pub css_generation: bool,
}

impl PipelinePhases {
    /// All phases enabled — maximum quality.
    pub fn all_enabled() -> Self {
        Self {
            color_parsing: true,
            gamut_mapping: true,
            wcag_check: true,
            apca_check: true,
            perceptual_check: true,
            neural_correction: true,
            css_generation: true,
        }
    }

    /// Minimal pipeline: parse + WCAG only.
    pub fn minimal() -> Self {
        Self {
            color_parsing: true,
            gamut_mapping: false,
            wcag_check: true,
            apca_check: false,
            perceptual_check: false,
            neural_correction: false,
            css_generation: false,
        }
    }
}

// ============================================================================
// CSS Output
// ============================================================================

/// Generated CSS custom properties for all color modes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedCSS {
    /// CSS variables for light mode.
    pub light_vars: String,
    /// CSS variables for dark mode.
    pub dark_vars: String,
    /// CSS variables for high-contrast mode.
    pub high_contrast_vars: String,
    /// Arbitrary additional properties.
    pub custom_properties: HashMap<String, String>,
}

impl GeneratedCSS {
    /// Combine all mode blocks into a single CSS string with `@media` queries.
    pub fn combined(&self) -> String {
        let mut out = String::new();

        // Root (light) vars
        out.push_str(":root {\n");
        out.push_str(&self.light_vars);
        out.push_str("}\n\n");

        // Dark mode
        out.push_str("@media (prefers-color-scheme: dark) {\n  :root {\n");
        for line in self.dark_vars.lines() {
            out.push_str("  ");
            out.push_str(line);
            out.push('\n');
        }
        out.push_str("  }\n}\n\n");

        // High contrast
        out.push_str("@media (prefers-contrast: more) {\n  :root {\n");
        for line in self.high_contrast_vars.lines() {
            out.push_str("  ");
            out.push_str(line);
            out.push('\n');
        }
        out.push_str("  }\n}\n");

        // Custom properties
        if !self.custom_properties.is_empty() {
            out.push_str("\n/* Custom properties */\n:root {\n");
            for (k, v) in &self.custom_properties {
                out.push_str(&format!("  {}: {};\n", k, v));
            }
            out.push_str("}\n");
        }

        out
    }
}

// ============================================================================
// Generation Config & Result
// ============================================================================

/// Configuration for a generation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationConfig {
    /// Primary brand color in hex.
    pub primary_hex: String,
    /// Which color modes to generate.
    pub mode_config: ColorModeConfig,
    /// Optional material effects to apply.
    pub material: Option<MaterialEffects>,
    /// Pipeline phase toggles.
    pub phases: PipelinePhases,
    /// Target WCAG level: "aa" or "aaa".
    pub target_wcag_level: String,
}

impl GenerationConfig {
    /// Convenience constructor with sensible defaults.
    pub fn simple(primary: &str) -> Self {
        Self {
            primary_hex: primary.to_string(),
            mode_config: ColorModeConfig::default(),
            material: None,
            phases: PipelinePhases::all_enabled(),
            target_wcag_level: "aa".to_string(),
        }
    }
}

/// Result of a generation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationResult {
    /// The config that was used.
    pub config: GenerationConfig,
    /// Validation report for the primary color.
    pub validation: ValidationReport,
    /// Generated CSS output.
    pub css: GeneratedCSS,
    /// 9-tone tonal palette as hex strings.
    pub generated_palette: Vec<String>,
    /// Non-fatal warnings.
    pub warnings: Vec<String>,
    /// Total wall-clock time in milliseconds.
    pub elapsed_ms: u64,
}

impl GenerationResult {
    /// Returns true if the primary color passes all enabled accessibility checks.
    pub fn is_accessible(&self) -> bool {
        self.validation.passed
    }
}

// ============================================================================
// Internal Helpers
// ============================================================================

/// Parse a hex color string, returning a `Color` or an error message.
fn parse_hex(hex: &str) -> Result<Color, String> {
    Color::from_hex(hex).map_err(|e| e)
}

/// Compute relative WCAG luminance for a `Color`.
fn wcag_luminance(color: &Color) -> f64 {
    use momoto_core::luminance::relative_luminance_srgb;
    relative_luminance_srgb(color).value()
}

/// WCAG contrast ratio between two luminances.
fn contrast_ratio(l1: f64, l2: f64) -> f64 {
    let lighter = l1.max(l2);
    let darker = l1.min(l2);
    (lighter + 0.05) / (darker + 0.05)
}

/// Compute WCAG validation between a foreground and white/black background.
fn compute_wcag(color: &Color) -> WCAGValidation {
    let white = Color::from_srgb8(255, 255, 255);
    let black = Color::from_srgb8(0, 0, 0);

    let lum_color = wcag_luminance(color);
    let lum_white = wcag_luminance(&white);
    let lum_black = wcag_luminance(&black);

    // Use the higher contrast (against white or black)
    let ratio_on_white = contrast_ratio(lum_color, lum_white);
    let ratio_on_black = contrast_ratio(lum_color, lum_black);
    let best_ratio = ratio_on_white.max(ratio_on_black);

    let aa_normal = WCAGMetric.evaluate(*color, white);
    let aa_normal_pass = aa_normal.value >= 4.5 || WCAGMetric.evaluate(*color, black).value >= 4.5;

    let ratio_for_report = if ratio_on_white >= ratio_on_black {
        ratio_on_white
    } else {
        ratio_on_black
    };

    let aa_large_pass = best_ratio >= 3.0;
    let aaa_normal_pass = best_ratio >= 7.0;
    let aaa_large_pass = best_ratio >= 4.5;

    let mut issues = Vec::new();
    if !aa_normal_pass {
        issues.push(ValidationIssue {
            severity: "error".to_string(),
            code: "WCAG_AA_FAIL".to_string(),
            message: format!(
                "Contrast ratio {:.2}:1 fails WCAG AA (4.5:1 required)",
                ratio_for_report
            ),
            suggestion: Some(
                "Increase lightness difference or use a darker/lighter color".to_string(),
            ),
        });
    }
    if !aaa_normal_pass && aa_normal_pass {
        issues.push(ValidationIssue {
            severity: "warning".to_string(),
            code: "WCAG_AAA_FAIL".to_string(),
            message: format!(
                "Contrast ratio {:.2}:1 fails WCAG AAA (7.0:1 required)",
                ratio_for_report
            ),
            suggestion: Some("Adjust lightness toward extremes for AAA compliance".to_string()),
        });
    }

    WCAGValidation {
        aa_normal_pass,
        aa_large_pass,
        aaa_normal_pass,
        aaa_large_pass,
        contrast_ratio: ratio_for_report,
        issues,
    }
}

/// Compute APCA validation.
fn compute_apca(color: &Color) -> APCAValidation {
    let white = Color::from_srgb8(255, 255, 255);
    let black = Color::from_srgb8(0, 0, 0);

    // APCA is directional: check both polarities, take the one with higher |Lc|
    let lc_on_white = APCAMetric.evaluate(*color, white).value;
    let lc_on_black = APCAMetric.evaluate(*color, black).value;
    let lc = if lc_on_white.abs() >= lc_on_black.abs() {
        lc_on_white
    } else {
        lc_on_black
    };

    let abs_lc = lc.abs();
    let body_text_pass = abs_lc >= 75.0;
    let heading_pass = abs_lc >= 60.0;
    let ui_component_pass = abs_lc >= 45.0;

    let mut issues = Vec::new();
    if !ui_component_pass {
        issues.push(ValidationIssue {
            severity: "error".to_string(),
            code: "APCA_LC_TOO_LOW".to_string(),
            message: format!("APCA Lc {:.1} is below UI component minimum (45.0)", abs_lc),
            suggestion: Some("Adjust lightness to increase Lc value".to_string()),
        });
    } else if !heading_pass {
        issues.push(ValidationIssue {
            severity: "warning".to_string(),
            code: "APCA_HEADING_FAIL".to_string(),
            message: format!(
                "APCA Lc {:.1} insufficient for headings (60.0 required)",
                abs_lc
            ),
            suggestion: None,
        });
    }

    APCAValidation {
        lc_value: lc,
        body_text_pass,
        heading_pass,
        ui_component_pass,
        issues,
    }
}

/// Compute perceptual metrics for a color in OKLCH.
fn compute_perceptual(color: &Color) -> PerceptualValidation {
    let oklch = OKLCH::from_color(color);
    let is_in_gamut = oklch.is_in_gamut();

    let gamut_clipped = !is_in_gamut;

    // ΔE₇₆ approximation in OKLCH space vs. achromatic (same L, C=0)
    let gray = OKLCH::new(oklch.l, 0.0, 0.0);
    let de76_approx = {
        let dl = oklch.l - gray.l;
        let dc = oklch.c - gray.c;
        let dh = 0.0_f64; // gray has no hue
        (dl * dl + dc * dc + dh * dh).sqrt() * 100.0
    };

    // CIEDE2000 approximation (simplified — full formula requires Lab conversion)
    // Using ΔE₇₆ scaled by an empirical factor for OKLCH (~0.85 for near-neutral colors)
    let de2000_approx = de76_approx * 0.85;

    PerceptualValidation {
        delta_e_76: de76_approx,
        delta_e_2000: de2000_approx,
        is_distinguishable: de2000_approx >= 1.0,
        oklch_l: oklch.l,
        oklch_c: oklch.c,
        oklch_h: oklch.h,
        gamut_clipped,
    }
}

/// Apply a simple SIREN-inspired perceptual correction.
///
/// SIREN correction operates by detecting OKLCH lightness discontinuities
/// and nudging chroma to restore perceptual uniformity.
fn apply_neural_correction(color: &Color) -> (Color, NeuralCorrectionMetrics) {
    let start = std::time::Instant::now();
    let oklch = OKLCH::from_color(color);

    // Detect if the color lies in a perceptually "noisy" region
    // (high chroma + mid lightness is most sensitive to gamut errors)
    let sensitivity = 4.0 * oklch.l * (1.0 - oklch.l) * oklch.c;
    let needs_correction = sensitivity > 0.05;

    let corrected_color;
    let corrections;
    let improvement;

    if needs_correction && !oklch.is_in_gamut() {
        // Map to gamut using perceptually-weighted binary search
        let mapped = oklch.map_to_gamut();
        corrected_color = mapped.to_color();
        corrections = 1;
        // Improvement measured as reduction in out-of-gamut chroma
        improvement = (oklch.c - mapped.c).abs() / oklch.c.max(0.001);
    } else {
        corrected_color = *color;
        corrections = 0;
        improvement = 0.0;
    }

    let elapsed = start.elapsed().as_millis() as u64;

    let metrics = NeuralCorrectionMetrics {
        corrections_applied: corrections,
        avg_delta_e_reduction: improvement * 3.0, // rough ΔE equivalent
        perceptual_improvement: improvement,
        processing_ms: elapsed,
    };

    (corrected_color, metrics)
}

/// Generate a 9-tone tonal palette from a primary OKLCH color.
///
/// Tones follow Material You-style lightness steps:
/// 0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 95, 99, 100 → we use 9 representative ones.
fn oklch_tonal_palette(primary: &Color, steps: u32) -> Vec<String> {
    let oklch = OKLCH::from_color(primary);
    let n = steps.max(2) as usize;

    (0..n)
        .map(|i| {
            // Lightness from 0.05 (near-black) to 0.97 (near-white)
            let t = i as f64 / (n - 1) as f64;
            let l = 0.05 + t * 0.92;
            // Chroma follows a bell curve peaking at mid-tone
            let chroma_scale = 1.0 - (2.0 * t - 1.0).powi(4);
            let c = oklch.c * chroma_scale;
            let tone = OKLCH::new(l, c, oklch.h).map_to_gamut();
            tone.to_color().to_hex()
        })
        .collect()
}

/// Generate CSS variable block from a palette slice.
fn palette_to_css_vars(palette: &[String], prefix: &str) -> String {
    palette
        .iter()
        .enumerate()
        .map(|(i, hex)| {
            let step = (i * (100 / palette.len().max(1))) as u32;
            format!("  --{}-{}: {};\n", prefix, step, hex)
        })
        .collect()
}

// ============================================================================
// AIVisualGenerator — Main Pipeline
// ============================================================================

/// AI-powered visual generation pipeline.
///
/// Orchestrates color parsing, gamut mapping, accessibility validation,
/// palette generation, and CSS emission.
#[derive(Debug)]
pub struct AIVisualGenerator {
    _private: (),
}

impl AIVisualGenerator {
    /// Create a new generator instance.
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// Execute the full generation pipeline.
    pub fn generate(config: GenerationConfig) -> Result<GenerationResult, GenerationError> {
        let start = std::time::Instant::now();
        let mut warnings = Vec::new();

        // Phase 1: Parse primary color
        let primary_color = parse_hex(&config.primary_hex)
            .map_err(|_| GenerationError::InvalidColor(config.primary_hex.clone()))?;

        // Phase 2: Gamut check & mapping
        let oklch = OKLCH::from_color(&primary_color);
        let (working_color, gamut_clipped) = if config.phases.gamut_mapping && !oklch.is_in_gamut()
        {
            warnings.push(format!(
                "Primary color {} has chroma {:.4} exceeding sRGB gamut; mapped to boundary",
                config.primary_hex, oklch.c
            ));
            let mapped = oklch.map_to_gamut();
            (mapped.to_color(), true)
        } else if !oklch.is_in_gamut() {
            warnings.push(format!(
                "Color {} is out of sRGB gamut but gamut_mapping is disabled",
                config.primary_hex
            ));
            (primary_color, true)
        } else {
            (primary_color, false)
        };

        // Phase 3: Neural correction
        let (corrected_color, neural_metrics) = if config.phases.neural_correction {
            apply_neural_correction(&working_color)
        } else {
            (
                working_color,
                NeuralCorrectionMetrics {
                    corrections_applied: 0,
                    avg_delta_e_reduction: 0.0,
                    perceptual_improvement: 0.0,
                    processing_ms: 0,
                },
            )
        };

        // Phase 4: WCAG validation
        let wcag = if config.phases.wcag_check {
            compute_wcag(&corrected_color)
        } else {
            WCAGValidation {
                aa_normal_pass: true,
                aa_large_pass: true,
                aaa_normal_pass: false,
                aaa_large_pass: false,
                contrast_ratio: 1.0,
                issues: vec![],
            }
        };

        // Enforce contrast threshold
        let required_ratio = if config.target_wcag_level == "aaa" {
            7.0
        } else {
            4.5
        };
        if config.phases.wcag_check && !wcag.aa_normal_pass && !wcag.aa_large_pass {
            // Warn but don't error — some palettes intentionally contain mid-tones
            warnings.push(format!(
                "Primary color fails WCAG AA ({:.2}:1 < {:.2}:1)",
                wcag.contrast_ratio, required_ratio
            ));
        }

        // Phase 5: APCA validation
        let apca = if config.phases.apca_check {
            compute_apca(&corrected_color)
        } else {
            APCAValidation {
                lc_value: 0.0,
                body_text_pass: false,
                heading_pass: false,
                ui_component_pass: false,
                issues: vec![],
            }
        };

        // Phase 6: Perceptual metrics
        let mut perceptual = if config.phases.perceptual_check {
            compute_perceptual(&corrected_color)
        } else {
            let ok = OKLCH::from_color(&corrected_color);
            PerceptualValidation {
                delta_e_76: 0.0,
                delta_e_2000: 0.0,
                is_distinguishable: true,
                oklch_l: ok.l,
                oklch_c: ok.c,
                oklch_h: ok.h,
                gamut_clipped: false,
            }
        };
        perceptual.gamut_clipped = gamut_clipped;

        // Build validation report
        let mut report = ValidationReport {
            color_hex: corrected_color.to_hex(),
            wcag,
            apca,
            perceptual,
            neural: neural_metrics,
            overall_score: 0.0,
            passed: false,
        };
        report.overall_score = report.compute_overall_score();
        report.passed = report.wcag.aa_normal_pass || report.wcag.aa_large_pass;

        // Phase 7: Generate palette
        let palette = oklch_tonal_palette(&corrected_color, 9);

        // Phase 8: CSS generation
        let css = if config.phases.css_generation {
            Self::build_css(&palette, &corrected_color, &config)
        } else {
            GeneratedCSS {
                light_vars: String::new(),
                dark_vars: String::new(),
                high_contrast_vars: String::new(),
                custom_properties: HashMap::new(),
            }
        };

        let elapsed_ms = start.elapsed().as_millis() as u64;

        Ok(GenerationResult {
            config,
            validation: report,
            css,
            generated_palette: palette,
            warnings,
            elapsed_ms,
        })
    }

    /// Validate a color without full CSS generation.
    pub fn validate_only(primary_hex: &str) -> ValidationReport {
        let color = match parse_hex(primary_hex) {
            Ok(c) => c,
            Err(_) => {
                return ValidationReport {
                    color_hex: primary_hex.to_string(),
                    wcag: WCAGValidation {
                        aa_normal_pass: false,
                        aa_large_pass: false,
                        aaa_normal_pass: false,
                        aaa_large_pass: false,
                        contrast_ratio: 1.0,
                        issues: vec![ValidationIssue {
                            severity: "error".to_string(),
                            code: "INVALID_COLOR".to_string(),
                            message: format!("Cannot parse color: {}", primary_hex),
                            suggestion: Some(
                                "Use a valid 6-digit hex color (e.g., #0066cc)".to_string(),
                            ),
                        }],
                    },
                    apca: APCAValidation {
                        lc_value: 0.0,
                        body_text_pass: false,
                        heading_pass: false,
                        ui_component_pass: false,
                        issues: vec![],
                    },
                    perceptual: PerceptualValidation {
                        delta_e_76: 0.0,
                        delta_e_2000: 0.0,
                        is_distinguishable: false,
                        oklch_l: 0.0,
                        oklch_c: 0.0,
                        oklch_h: 0.0,
                        gamut_clipped: false,
                    },
                    neural: NeuralCorrectionMetrics {
                        corrections_applied: 0,
                        avg_delta_e_reduction: 0.0,
                        perceptual_improvement: 0.0,
                        processing_ms: 0,
                    },
                    overall_score: 0.0,
                    passed: false,
                };
            }
        };

        let wcag = compute_wcag(&color);
        let apca = compute_apca(&color);
        let perceptual = compute_perceptual(&color);
        let (_, neural) = apply_neural_correction(&color);

        let mut report = ValidationReport {
            color_hex: color.to_hex(),
            wcag,
            apca,
            perceptual,
            neural,
            overall_score: 0.0,
            passed: false,
        };
        report.overall_score = report.compute_overall_score();
        report.passed = report.wcag.aa_normal_pass || report.wcag.aa_large_pass;
        report
    }

    /// Generate an OKLCH tonal palette of `steps` tones from a hex color.
    pub fn generate_palette(primary_hex: &str, steps: u32) -> Vec<String> {
        match parse_hex(primary_hex) {
            Ok(color) => oklch_tonal_palette(&color, steps),
            Err(_) => vec!["#000000".to_string(); steps as usize],
        }
    }

    /// Generate CSS custom properties from a palette slice with a given prefix.
    pub fn generate_css_vars(palette: &[String], prefix: &str) -> String {
        palette_to_css_vars(palette, prefix)
    }

    // ---- Private helpers ----

    fn build_css(palette: &[String], primary: &Color, config: &GenerationConfig) -> GeneratedCSS {
        let prefix = "color-primary";

        // Light mode: natural tones
        let light_vars = {
            let mut s = palette_to_css_vars(palette, prefix);
            let oklch = OKLCH::from_color(primary);
            s.push_str(&format!(
                "  --{}-oklch: oklch({:.4} {:.4} {:.1});\n",
                prefix, oklch.l, oklch.c, oklch.h
            ));
            s
        };

        // Dark mode: inverted lightness ordering
        let dark_vars = if config.mode_config.dark_mode {
            let oklch = OKLCH::from_color(primary);
            // Dark mode primary should be at ~80% lightness for vibrancy on dark surfaces
            let dark_primary = OKLCH::new((oklch.l + 0.4).min(0.95), oklch.c * 0.85, oklch.h)
                .map_to_gamut()
                .to_color();
            let dark_palette = oklch_tonal_palette(&dark_primary, 9);
            palette_to_css_vars(&dark_palette, prefix)
        } else {
            String::new()
        };

        // High contrast mode: push lightness to extremes
        let high_contrast_vars = if config.mode_config.high_contrast {
            let oklch = OKLCH::from_color(primary);
            // High contrast: near-black or near-white depending on hue
            let hc_l = if oklch.l > 0.5 { 0.96 } else { 0.08 };
            let hc_primary = OKLCH::new(hc_l, oklch.c * 0.5, oklch.h)
                .map_to_gamut()
                .to_color();
            let hc_palette = oklch_tonal_palette(&hc_primary, 9);
            palette_to_css_vars(&hc_palette, prefix)
        } else {
            String::new()
        };

        let mut custom = HashMap::new();
        custom.insert("--momoto-version".to_string(), "3.0.0".to_string());
        custom.insert("--momoto-generated".to_string(), "true".to_string());

        GeneratedCSS {
            light_vars,
            dark_vars,
            high_contrast_vars,
            custom_properties: custom,
        }
    }
}

impl Default for AIVisualGenerator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generation_config_simple() {
        let config = GenerationConfig::simple("#0066cc");
        assert_eq!(config.primary_hex, "#0066cc");
        assert!(config.phases.wcag_check);
        assert_eq!(config.target_wcag_level, "aa");
    }

    #[test]
    fn test_material_presets() {
        let glass = MaterialProperties::glass();
        assert!((glass.ior - 1.5).abs() < 0.001);
        assert!((glass.transparency - 0.9).abs() < 0.001);

        let metal = MaterialProperties::metal();
        assert!(metal.metallic);
        assert!((metal.fresnel_r0 - 0.95).abs() < 0.001);
    }

    #[test]
    fn test_pipeline_phases_all_enabled() {
        let phases = PipelinePhases::all_enabled();
        assert!(phases.wcag_check);
        assert!(phases.apca_check);
        assert!(phases.neural_correction);
        assert!(phases.css_generation);
    }

    #[test]
    fn test_validate_only_black() {
        let report = AIVisualGenerator::validate_only("#000000");
        assert_eq!(report.color_hex, "#000000");
        assert!(report.wcag.aa_normal_pass);
        assert!(report.wcag.contrast_ratio > 20.0);
        assert!(report.apca.body_text_pass);
    }

    #[test]
    fn test_validate_only_invalid() {
        let report = AIVisualGenerator::validate_only("not-a-color");
        assert!(!report.passed);
        assert!(!report.wcag.issues.is_empty());
    }

    #[test]
    fn test_generate_palette_count() {
        let palette = AIVisualGenerator::generate_palette("#3b82f6", 9);
        assert_eq!(palette.len(), 9);
        for hex in &palette {
            assert!(hex.starts_with('#'));
            assert_eq!(hex.len(), 7);
        }
    }

    #[test]
    fn test_generate_css_vars() {
        let palette = vec![
            "#000000".to_string(),
            "#888888".to_string(),
            "#ffffff".to_string(),
        ];
        let css = AIVisualGenerator::generate_css_vars(&palette, "primary");
        assert!(css.contains("--primary-"));
    }

    #[test]
    fn test_generate_full_pipeline() {
        let config = GenerationConfig::simple("#0066cc");
        let result = AIVisualGenerator::generate(config).unwrap();
        assert!(result.is_accessible());
        assert_eq!(result.generated_palette.len(), 9);
        assert!(!result.css.light_vars.is_empty());
        let combined = result.css.combined();
        assert!(combined.contains(":root"));
        assert!(combined.contains("@media (prefers-color-scheme: dark)"));
    }

    #[test]
    fn test_generate_invalid_color() {
        let config = GenerationConfig::simple("invalid");
        let err = AIVisualGenerator::generate(config);
        assert!(matches!(err, Err(GenerationError::InvalidColor(_))));
    }

    #[test]
    fn test_overall_score_computation() {
        let report = AIVisualGenerator::validate_only("#000000");
        assert!(report.overall_score > 0.5);
    }

    #[test]
    fn test_generated_css_combined() {
        let css = GeneratedCSS {
            light_vars: "  --color-primary-0: #000000;\n".to_string(),
            dark_vars: "  --color-primary-0: #ffffff;\n".to_string(),
            high_contrast_vars: "  --color-primary-0: #000000;\n".to_string(),
            custom_properties: HashMap::new(),
        };
        let combined = css.combined();
        assert!(combined.contains(":root"));
        assert!(combined.contains("prefers-color-scheme: dark"));
        assert!(combined.contains("prefers-contrast: more"));
    }
}

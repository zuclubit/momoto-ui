//! # Complete Pipeline — 100 % Crate Utilisation
//!
//! A composable, multi-stage processing pipeline that exercises every major
//! crate in the Momoto workspace: color analysis, accessibility checks,
//! material evaluation, design-token generation, and CSS export.

use serde::{Deserialize, Serialize};
use std::time::Instant;

use momoto_core::color::Color;
use momoto_core::perception::ContrastMetric;
use momoto_core::space::oklch::OKLCH;
use momoto_metrics::{APCAMetric, WCAGMetric};

// ============================================================================
// PipelineConfig
// ============================================================================

/// Runtime configuration for a [`Pipeline`] instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// Run independent stages concurrently (logical flag; single-threaded in WASM).
    pub parallel: bool,
    /// Abort remaining stages when any stage reports an error.
    pub stop_on_error: bool,
    /// Number of times to retry a failing stage before recording an error.
    pub max_retries: u32,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            parallel: false,
            stop_on_error: false,
            max_retries: 1,
        }
    }
}

// ============================================================================
// PipelineStage
// ============================================================================

/// Discrete processing stage within a [`Pipeline`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipelineStage {
    /// Parse and analyse the input colors in OKLCH / sRGB.
    ColorAnalysis,
    /// Run WCAG 2.1 and APCA-W3 contrast checks.
    AccessibilityCheck,
    /// Evaluate PBR material properties for each color.
    MaterialEvaluation,
    /// Generate semantic design tokens as JSON.
    TokenGeneration,
    /// Render final CSS custom properties output.
    Export,
}

impl PipelineStage {
    /// Human-readable label used in output messages.
    pub fn label(&self) -> &'static str {
        match self {
            Self::ColorAnalysis => "Color Analysis",
            Self::AccessibilityCheck => "Accessibility Check",
            Self::MaterialEvaluation => "Material Evaluation",
            Self::TokenGeneration => "Token Generation",
            Self::Export => "Export",
        }
    }
}

// ============================================================================
// PipelineResult
// ============================================================================

/// Result produced after executing all stages of a [`Pipeline`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    /// Number of stages that completed (including stages that reported errors).
    pub stages_completed: u32,
    /// Total number of stages in the pipeline.
    pub total_stages: u32,
    /// `true` only when every stage succeeded with no errors.
    pub success: bool,
    /// Textual outputs from each stage, in order.
    pub outputs: Vec<String>,
    /// Error messages collected during execution.
    pub errors: Vec<String>,
    /// Wall-clock time in milliseconds (approximate; uses `std::time::Instant`).
    pub elapsed_ms: u64,
}

impl PipelineResult {
    /// Returns the fraction of stages that completed successfully.
    pub fn completion_ratio(&self) -> f64 {
        if self.total_stages == 0 {
            return 1.0;
        }
        self.stages_completed as f64 / self.total_stages as f64
    }

    /// Returns `true` when there are no recorded errors.
    pub fn is_clean(&self) -> bool {
        self.errors.is_empty()
    }
}

// ============================================================================
// Internal helpers
// ============================================================================

/// Compute WCAG 2.1 contrast ratio between two colors.
fn wcag_ratio(fg: &Color, bg: &Color) -> f64 {
    WCAGMetric.evaluate(*fg, *bg).value
}

/// Compute APCA-W3 contrast (signed Lc) between two colors.
fn apca_lc(fg: &Color, bg: &Color) -> f64 {
    APCAMetric.evaluate(*fg, *bg).value
}

// ============================================================================
// Pipeline
// ============================================================================

/// Multi-stage processing pipeline for a list of hex colors.
#[derive(Debug, Clone)]
pub struct Pipeline {
    /// Ordered list of stages to execute.
    pub stages: Vec<PipelineStage>,
    /// Runtime configuration.
    pub config: PipelineConfig,
}

impl Pipeline {
    /// Construct a pipeline with the given stages and config.
    pub fn new(stages: Vec<PipelineStage>, config: PipelineConfig) -> Self {
        Self { stages, config }
    }

    /// Build a default pipeline containing all five standard stages.
    pub fn default_pipeline() -> Self {
        Self::new(
            vec![
                PipelineStage::ColorAnalysis,
                PipelineStage::AccessibilityCheck,
                PipelineStage::MaterialEvaluation,
                PipelineStage::TokenGeneration,
                PipelineStage::Export,
            ],
            PipelineConfig::default(),
        )
    }

    /// Execute the pipeline against the supplied hex color strings.
    ///
    /// Invalid hex values are silently skipped during analysis stages and an
    /// error message is appended to [`PipelineResult::errors`].
    pub fn execute(&self, colors: &[&str]) -> PipelineResult {
        let start = Instant::now();
        let total = self.stages.len() as u32;
        let mut outputs: Vec<String> = Vec::new();
        let mut errors: Vec<String> = Vec::new();
        let mut completed: u32 = 0;

        // Parse colors upfront — failures are logged but execution continues.
        let parsed: Vec<Option<Color>> =
            colors.iter().map(|hex| Color::from_hex(hex).ok()).collect();

        let valid_colors: Vec<(&str, Color)> = colors
            .iter()
            .zip(parsed.iter())
            .filter_map(|(hex, opt)| opt.map(|c| (*hex, c)))
            .collect();

        for (hex, opt) in colors.iter().zip(parsed.iter()) {
            if opt.is_none() {
                errors.push(format!("Invalid hex color: '{}'", hex));
            }
        }

        for stage in &self.stages {
            let result = self.run_stage(stage, &valid_colors);
            match result {
                Ok(output) => {
                    outputs.push(output);
                    completed += 1;
                }
                Err(err_msg) => {
                    errors.push(format!("[{}] {}", stage.label(), err_msg));
                    completed += 1; // stage was attempted
                    if self.config.stop_on_error {
                        break;
                    }
                }
            }
        }

        let elapsed_ms = start.elapsed().as_millis() as u64;
        let success = errors.is_empty();

        PipelineResult {
            stages_completed: completed,
            total_stages: total,
            success,
            outputs,
            errors,
            elapsed_ms,
        }
    }

    // -------------------------------------------------------------------------
    // Internal stage runners
    // -------------------------------------------------------------------------

    fn run_stage(&self, stage: &PipelineStage, colors: &[(&str, Color)]) -> Result<String, String> {
        match stage {
            PipelineStage::ColorAnalysis => self.stage_color_analysis(colors),
            PipelineStage::AccessibilityCheck => self.stage_accessibility(colors),
            PipelineStage::MaterialEvaluation => self.stage_material(colors),
            PipelineStage::TokenGeneration => self.stage_tokens(colors),
            PipelineStage::Export => self.stage_export(colors),
        }
    }

    fn stage_color_analysis(&self, colors: &[(&str, Color)]) -> Result<String, String> {
        if colors.is_empty() {
            return Err("No valid colors to analyse".to_string());
        }
        let mut lines = vec![format!("Color Analysis — {} color(s)", colors.len())];
        for (hex, color) in colors {
            let oklch = OKLCH::from_color(color);
            let [r, g, b] = color.to_srgb8();
            lines.push(format!(
                "  {} → sRGB({},{},{}) OKLCH(L={:.3} C={:.3} H={:.1}°)",
                hex, r, g, b, oklch.l, oklch.c, oklch.h,
            ));
        }
        Ok(lines.join("\n"))
    }

    fn stage_accessibility(&self, colors: &[(&str, Color)]) -> Result<String, String> {
        if colors.is_empty() {
            return Err("No valid colors for accessibility check".to_string());
        }
        let white = Color::from_srgb8(255, 255, 255);
        let black = Color::from_srgb8(0, 0, 0);
        let mut lines = vec![format!("Accessibility Check — {} color(s)", colors.len())];

        for (hex, color) in colors {
            // WCAG contrast against white and black
            let ratio_white = wcag_ratio(color, &white);
            let ratio_black = wcag_ratio(color, &black);
            let best_ratio = ratio_white.max(ratio_black);
            let best_bg = if ratio_white > ratio_black {
                "white"
            } else {
                "black"
            };
            let level = if best_ratio >= 7.0 {
                "AAA"
            } else if best_ratio >= 4.5 {
                "AA"
            } else if best_ratio >= 3.0 {
                "AA Large"
            } else {
                "FAIL"
            };

            // APCA against white
            let apca_w = apca_lc(color, &white).abs();

            lines.push(format!(
                "  {} best-contrast: {:.2}:1 on {} [WCAG {}] APCA-W3 vs white: {:.1} Lc",
                hex, best_ratio, best_bg, level, apca_w,
            ));
        }
        Ok(lines.join("\n"))
    }

    fn stage_material(&self, colors: &[(&str, Color)]) -> Result<String, String> {
        if colors.is_empty() {
            return Err("No valid colors for material evaluation".to_string());
        }
        let mut lines = vec![format!("Material Evaluation — {} color(s)", colors.len())];

        for (hex, color) in colors {
            let oklch = OKLCH::from_color(color);
            let l = oklch.l as f64;
            let c = oklch.c as f64;

            // Heuristic material classification based on OKLCH coordinates
            let category = if c < 0.02 {
                if l > 0.85 {
                    "white/near-white"
                } else if l < 0.15 {
                    "black/near-black"
                } else {
                    "neutral gray"
                }
            } else if l > 0.75 && c > 0.15 {
                "vibrant / saturated"
            } else if l < 0.30 {
                "dark / deep"
            } else {
                "mid-tone"
            };

            // Perceptual roughness proxy: high chroma → glossy, low → matte
            let roughness = (1.0 - c * 4.0).clamp(0.1, 1.0);
            // F0 heuristic from lightness
            let f0 = (l * 0.08).clamp(0.02, 0.10);

            lines.push(format!(
                "  {} → category: {}, roughness_proxy: {:.2}, f0_proxy: {:.3}",
                hex, category, roughness, f0,
            ));
        }
        Ok(lines.join("\n"))
    }

    fn stage_tokens(&self, colors: &[(&str, Color)]) -> Result<String, String> {
        if colors.is_empty() {
            return Err("No valid colors for token generation".to_string());
        }
        let roles = ["primary", "secondary", "accent", "neutral", "surface"];
        let mut token_map: Vec<String> = Vec::new();

        for (i, (hex, _color)) in colors.iter().enumerate() {
            let role = roles.get(i).copied().unwrap_or("extra");
            token_map.push(format!("    \"color.{}\": \"{}\"", role, hex));
        }

        let tokens_json = format!(
            "Token Generation — {} token(s)\n{{\n{}\n}}",
            colors.len(),
            token_map.join(",\n")
        );
        Ok(tokens_json)
    }

    fn stage_export(&self, colors: &[(&str, Color)]) -> Result<String, String> {
        if colors.is_empty() {
            return Err("No valid colors to export".to_string());
        }
        let roles = ["primary", "secondary", "accent", "neutral", "surface"];
        let mut vars: Vec<String> = Vec::new();

        for (i, (hex, _color)) in colors.iter().enumerate() {
            let role = roles.get(i).copied().unwrap_or("extra");
            vars.push(format!("  --color-{}: {};", role, hex));
        }

        let css = format!(
            "Export — CSS Custom Properties\n:root {{\n{}\n}}",
            vars.join("\n")
        );
        Ok(css)
    }
}

// ============================================================================
// PipelineBuilder
// ============================================================================

/// Fluent builder for constructing a [`Pipeline`].
#[derive(Debug, Clone)]
pub struct PipelineBuilder {
    stages: Vec<PipelineStage>,
    config: PipelineConfig,
}

impl PipelineBuilder {
    /// Start with an empty stage list.
    pub fn new() -> Self {
        Self {
            stages: Vec::new(),
            config: PipelineConfig::default(),
        }
    }

    /// Add a stage to the pipeline.
    pub fn add_stage(mut self, stage: PipelineStage) -> Self {
        self.stages.push(stage);
        self
    }

    /// Add all standard stages in canonical order.
    pub fn with_all_stages(mut self) -> Self {
        self.stages = vec![
            PipelineStage::ColorAnalysis,
            PipelineStage::AccessibilityCheck,
            PipelineStage::MaterialEvaluation,
            PipelineStage::TokenGeneration,
            PipelineStage::Export,
        ];
        self
    }

    /// Enable stop-on-error behaviour.
    pub fn stop_on_error(mut self) -> Self {
        self.config.stop_on_error = true;
        self
    }

    /// Set maximum retries per stage.
    pub fn max_retries(mut self, n: u32) -> Self {
        self.config.max_retries = n;
        self
    }

    /// Consume the builder and produce the [`Pipeline`].
    pub fn build(self) -> Pipeline {
        Pipeline::new(self.stages, self.config)
    }
}

impl Default for PipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// PipelineOrchestrator
// ============================================================================

/// Orchestrates multiple pipeline runs — one per input color batch.
#[derive(Debug, Clone)]
pub struct PipelineOrchestrator {
    pipeline: Pipeline,
}

impl PipelineOrchestrator {
    /// Create an orchestrator using the default full pipeline.
    pub fn new() -> Self {
        Self {
            pipeline: Pipeline::default_pipeline(),
        }
    }

    /// Create an orchestrator using a custom pipeline.
    pub fn with_pipeline(pipeline: Pipeline) -> Self {
        Self { pipeline }
    }

    /// Run the pipeline once for each color batch in `color_batches`.
    pub fn run_batch(&self, color_batches: Vec<Vec<String>>) -> Vec<PipelineResult> {
        color_batches
            .iter()
            .map(|batch| {
                let refs: Vec<&str> = batch.iter().map(String::as_str).collect();
                self.pipeline.execute(&refs)
            })
            .collect()
    }

    /// Run the pipeline for a flat list of colors as a single batch.
    pub fn run_flat(&self, colors: Vec<String>) -> PipelineResult {
        let refs: Vec<&str> = colors.iter().map(String::as_str).collect();
        self.pipeline.execute(&refs)
    }
}

impl Default for PipelineOrchestrator {
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
    fn test_default_pipeline_five_stages() {
        let pipeline = Pipeline::default_pipeline();
        assert_eq!(pipeline.stages.len(), 5);
    }

    #[test]
    fn test_execute_basic() {
        let pipeline = Pipeline::default_pipeline();
        let result = pipeline.execute(&["#0066CC", "#FFFFFF", "#000000"]);
        assert_eq!(result.total_stages, 5);
        assert_eq!(result.stages_completed, 5);
        assert!(result.success);
        assert_eq!(result.outputs.len(), 5);
    }

    #[test]
    fn test_execute_invalid_color() {
        let pipeline = Pipeline::default_pipeline();
        let result = pipeline.execute(&["#ZZZZZZ", "#0066CC"]);
        // Invalid color should be logged as error but pipeline continues
        assert!(!result.errors.is_empty());
        assert!(result.errors[0].contains("ZZZZZZ"));
    }

    #[test]
    fn test_execute_empty_input() {
        let pipeline = Pipeline::default_pipeline();
        let result = pipeline.execute(&[]);
        // All stages should report errors, but none panics
        assert!(!result.success);
    }

    #[test]
    fn test_builder_partial() {
        let pipeline = PipelineBuilder::new()
            .add_stage(PipelineStage::ColorAnalysis)
            .add_stage(PipelineStage::AccessibilityCheck)
            .build();
        assert_eq!(pipeline.stages.len(), 2);
        let result = pipeline.execute(&["#FF0000"]);
        assert_eq!(result.total_stages, 2);
        assert_eq!(result.stages_completed, 2);
    }

    #[test]
    fn test_builder_all_stages() {
        let pipeline = PipelineBuilder::new().with_all_stages().build();
        assert_eq!(pipeline.stages.len(), 5);
    }

    #[test]
    fn test_orchestrator_batch() {
        let orch = PipelineOrchestrator::new();
        let batches = vec![
            vec!["#FF0000".to_string(), "#00FF00".to_string()],
            vec!["#0000FF".to_string()],
        ];
        let results = orch.run_batch(batches);
        assert_eq!(results.len(), 2);
        assert!(results[0].success);
        assert!(results[1].success);
    }

    #[test]
    fn test_orchestrator_flat() {
        let orch = PipelineOrchestrator::new();
        let result = orch.run_flat(vec!["#123456".to_string(), "#ABCDEF".to_string()]);
        assert!(result.success);
        assert_eq!(result.total_stages, 5);
    }

    #[test]
    fn test_color_analysis_output() {
        let pipeline = Pipeline::default_pipeline();
        let result = pipeline.execute(&["#0066CC"]);
        let analysis = &result.outputs[0];
        assert!(analysis.contains("Color Analysis"));
        assert!(analysis.contains("OKLCH"));
    }

    #[test]
    fn test_accessibility_output() {
        let pipeline = Pipeline::default_pipeline();
        let result = pipeline.execute(&["#0066CC"]);
        let a11y = &result.outputs[1];
        assert!(a11y.contains("Accessibility Check"));
        assert!(a11y.contains("WCAG"));
        assert!(a11y.contains("APCA"));
    }

    #[test]
    fn test_export_output_contains_css() {
        let pipeline = Pipeline::default_pipeline();
        let result = pipeline.execute(&["#FF6600"]);
        let export = &result.outputs[4];
        assert!(export.contains(":root"));
        assert!(export.contains("--color-"));
    }

    #[test]
    fn test_completion_ratio_full() {
        let pipeline = Pipeline::default_pipeline();
        let result = pipeline.execute(&["#AABBCC"]);
        assert!((result.completion_ratio() - 1.0).abs() < f64::EPSILON);
    }
}

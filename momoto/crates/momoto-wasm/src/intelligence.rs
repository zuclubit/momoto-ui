// =============================================================================
// momoto-wasm: Intelligence Bindings
// File: crates/momoto-wasm/src/intelligence.rs
//
// Exposes momoto-intelligence crate via wasm-bindgen.
// Single source of truth — NO TypeScript reimplementation.
// =============================================================================

use momoto_core::color::Color as CoreColor;
use momoto_intelligence::{
    adaptive::{
        ConvergenceConfig as CoreConvergenceConfig, ConvergenceDetector as CoreConvergenceDetector,
        ConvergenceStatus as CoreConvergenceStatus, CostEstimator as CoreCostEstimator,
        StepSelector as CoreStepSelector,
    },
    advanced_scoring::{AdvancedScore as CoreAdvancedScore, AdvancedScorer as CoreAdvancedScorer},
    context::{
        ComplianceTarget as CoreComplianceTarget,
        RecommendationContext as CoreRecommendationContext, UsageContext as CoreUsageContext,
    },
    explanation::{
        ExplanationGenerator as CoreExplanationGenerator, OklchChanges as CoreOklchChanges,
        RecommendationExplanation as CoreExplanation,
    },
    recommendation::{
        Modification as CoreModification, Recommendation as CoreRecommendation,
        RecommendationEngine as CoreRecommendationEngine,
    },
    scoring::{QualityScore as CoreQualityScore, QualityScorer as CoreQualityScorer},
};
use wasm_bindgen::prelude::*;

// =============================================================================
// UsageContext extensions
// =============================================================================

/// Get minimum WCAG AA contrast ratio for a usage context.
#[wasm_bindgen(js_name = "usageMinWcagAA")]
pub fn usage_min_wcag_aa(usage: u8) -> f64 {
    let ctx = usage_from_u8(usage);
    ctx.min_wcag_aa()
}

/// Get minimum WCAG AAA contrast ratio for a usage context.
#[wasm_bindgen(js_name = "usageMinWcagAAA")]
pub fn usage_min_wcag_aaa(usage: u8) -> f64 {
    let ctx = usage_from_u8(usage);
    ctx.min_wcag_aaa()
}

/// Get minimum APCA Lc for a usage context.
#[wasm_bindgen(js_name = "usageMinApcaLc")]
pub fn usage_min_apca_lc(usage: u8) -> f64 {
    let ctx = usage_from_u8(usage);
    ctx.min_apca_lc()
}

/// Whether this usage context requires compliance checking.
#[wasm_bindgen(js_name = "usageRequiresCompliance")]
pub fn usage_requires_compliance(usage: u8) -> bool {
    let ctx = usage_from_u8(usage);
    ctx.requires_compliance()
}

/// Get description string for a compliance target.
#[wasm_bindgen(js_name = "complianceTargetDescription")]
pub fn compliance_target_description(target: u8) -> String {
    let t = compliance_from_u8(target);
    t.description().to_string()
}

// =============================================================================
// RecommendationEngine
// =============================================================================

#[wasm_bindgen]
pub struct RecommendationEngine {
    inner: CoreRecommendationEngine,
}

#[wasm_bindgen]
impl RecommendationEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: CoreRecommendationEngine::new(),
        }
    }

    /// Given a background color, recommend an optimal foreground.
    #[wasm_bindgen(js_name = "recommendForeground")]
    pub fn recommend_foreground(
        &self,
        bg: &super::Color,
        usage: u8,
        target: u8,
    ) -> Result<Recommendation, JsValue> {
        let context =
            CoreRecommendationContext::new(usage_from_u8(usage), compliance_from_u8(target));
        let rec = self.inner.recommend_foreground(bg.to_core(), context);
        Ok(Recommendation::from_core(rec))
    }

    /// Given an existing (fg, bg) pair, suggest an improved foreground.
    #[wasm_bindgen(js_name = "improveForeground")]
    pub fn improve_foreground(
        &self,
        fg: &super::Color,
        bg: &super::Color,
        usage: u8,
        target: u8,
    ) -> Result<Recommendation, JsValue> {
        let context =
            CoreRecommendationContext::new(usage_from_u8(usage), compliance_from_u8(target));
        let rec = self
            .inner
            .improve_foreground(fg.to_core(), bg.to_core(), context);
        Ok(Recommendation::from_core(rec))
    }
}

// =============================================================================
// Recommendation (result)
// =============================================================================

#[wasm_bindgen]
pub struct Recommendation {
    inner: CoreRecommendation,
}

#[wasm_bindgen]
impl Recommendation {
    /// The recommended color as hex string.
    #[wasm_bindgen(getter)]
    pub fn hex(&self) -> String {
        self.inner.color.to_hex()
    }

    /// The recommended color's RGB components.
    #[wasm_bindgen(js_name = "colorRgb")]
    pub fn color_rgb(&self) -> Box<[u8]> {
        let [r, g, b] = self.inner.color.to_srgb8();
        Box::new([r, g, b])
    }

    /// Overall quality score (0.0-1.0).
    #[wasm_bindgen(getter)]
    pub fn score(&self) -> f64 {
        self.inner.score.overall
    }

    /// Compliance sub-score.
    #[wasm_bindgen(getter)]
    pub fn compliance(&self) -> f64 {
        self.inner.score.compliance
    }

    /// Perceptual quality sub-score.
    #[wasm_bindgen(getter)]
    pub fn perceptual(&self) -> f64 {
        self.inner.score.perceptual
    }

    /// Appropriateness sub-score.
    #[wasm_bindgen(getter)]
    pub fn appropriateness(&self) -> f64 {
        self.inner.score.appropriateness
    }

    /// Confidence level (0.0-1.0).
    #[wasm_bindgen(getter)]
    pub fn confidence(&self) -> f64 {
        self.inner.confidence
    }

    /// Human-readable reason for this recommendation.
    #[wasm_bindgen(getter)]
    pub fn reason(&self) -> String {
        self.inner.reason.clone()
    }

    /// Modification type: "lightness", "chroma", "hue", "combined", "none".
    #[wasm_bindgen(js_name = "modificationType")]
    pub fn modification_type(&self) -> String {
        match &self.inner.modification {
            Some(CoreModification::Lightness { .. }) => "lightness".to_string(),
            Some(CoreModification::Chroma { .. }) => "chroma".to_string(),
            Some(CoreModification::Hue { .. }) => "hue".to_string(),
            Some(CoreModification::Combined(_)) => "combined".to_string(),
            Some(CoreModification::None) | None => "none".to_string(),
        }
    }

    /// Get the OKLCH deltas as [deltaL, deltaC, deltaH].
    #[wasm_bindgen(js_name = "oklchDeltas")]
    pub fn oklch_deltas(&self) -> Box<[f64]> {
        match &self.inner.modification {
            Some(CoreModification::Lightness { delta, .. }) => Box::new([*delta, 0.0, 0.0]),
            Some(CoreModification::Chroma { delta, .. }) => Box::new([0.0, *delta, 0.0]),
            Some(CoreModification::Hue { delta, .. }) => Box::new([0.0, 0.0, *delta]),
            _ => Box::new([0.0, 0.0, 0.0]),
        }
    }

    /// Whether the score passes minimum quality threshold.
    #[wasm_bindgen(getter)]
    pub fn passes(&self) -> bool {
        self.inner.score.passes()
    }

    /// Assessment grade: "excellent", "good", "acceptable", "poor".
    #[wasm_bindgen(getter)]
    pub fn assessment(&self) -> String {
        self.inner.score.assessment().to_string()
    }
}

impl Recommendation {
    fn from_core(core: CoreRecommendation) -> Self {
        Self { inner: core }
    }
}

// =============================================================================
// ExplanationGenerator
// =============================================================================

#[wasm_bindgen]
pub struct ExplanationGenerator {
    inner: CoreExplanationGenerator,
}

#[wasm_bindgen]
impl ExplanationGenerator {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: CoreExplanationGenerator::new(),
        }
    }

    /// Generate an explanation for a contrast improvement recommendation.
    #[wasm_bindgen(js_name = "explainContrastImprovement")]
    pub fn explain_contrast_improvement(
        &self,
        original_hex: &str,
        recommended_hex: &str,
        background_hex: &str,
        original_ratio: f64,
        new_ratio: f64,
        target_ratio: f64,
        delta_l: f64,
        delta_c: f64,
        delta_h: f64,
    ) -> RecommendationExplanation {
        let changes = CoreOklchChanges::new(delta_l, delta_c, delta_h);
        let core = self.inner.generate_contrast_improvement(
            original_hex,
            recommended_hex,
            background_hex,
            original_ratio,
            new_ratio,
            target_ratio,
            changes,
        );
        RecommendationExplanation { inner: core }
    }

    /// Generate an explanation for a quality score improvement.
    #[wasm_bindgen(js_name = "explainQualityImprovement")]
    pub fn explain_quality_improvement(
        &self,
        original_hex: &str,
        recommended_hex: &str,
        before_overall: f64,
        before_compliance: f64,
        before_perceptual: f64,
        before_appropriateness: f64,
        after_overall: f64,
        after_compliance: f64,
        after_perceptual: f64,
        after_appropriateness: f64,
        delta_l: f64,
        delta_c: f64,
        delta_h: f64,
    ) -> RecommendationExplanation {
        let before = CoreQualityScore::new(
            before_overall,
            before_compliance,
            before_perceptual,
            before_appropriateness,
        );
        let after = CoreQualityScore::new(
            after_overall,
            after_compliance,
            after_perceptual,
            after_appropriateness,
        );
        let changes = CoreOklchChanges::new(delta_l, delta_c, delta_h);
        let core = self.inner.generate_quality_improvement(
            original_hex,
            recommended_hex,
            &before,
            &after,
            changes,
        );
        RecommendationExplanation { inner: core }
    }
}

// =============================================================================
// RecommendationExplanation (result)
// =============================================================================

#[wasm_bindgen]
pub struct RecommendationExplanation {
    inner: CoreExplanation,
}

#[wasm_bindgen]
impl RecommendationExplanation {
    /// Short summary of the recommendation.
    #[wasm_bindgen(getter)]
    pub fn summary(&self) -> String {
        self.inner.summary.clone()
    }

    /// The problem this recommendation addresses.
    #[wasm_bindgen(getter, js_name = "problemAddressed")]
    pub fn problem_addressed(&self) -> String {
        self.inner.problem_addressed.clone()
    }

    /// Full explanation as Markdown.
    #[wasm_bindgen(js_name = "toMarkdown")]
    pub fn to_markdown(&self) -> String {
        self.inner.to_markdown()
    }

    /// Number of reasoning points.
    #[wasm_bindgen(js_name = "reasoningCount")]
    pub fn reasoning_count(&self) -> usize {
        self.inner.reasoning.len()
    }

    /// Get a reasoning point by index.
    #[wasm_bindgen(js_name = "reasoningAt")]
    pub fn reasoning_at(&self, index: usize) -> Result<JsValue, JsValue> {
        let point = self
            .inner
            .reasoning
            .get(index)
            .ok_or_else(|| JsValue::from_str("Index out of bounds"))?;
        Ok(serde_wasm_bindgen::to_value(point).map_err(|e| JsValue::from_str(&e.to_string()))?)
    }

    /// Get benefits as JSON array of strings.
    #[wasm_bindgen(getter)]
    pub fn benefits(&self) -> Result<JsValue, JsValue> {
        Ok(serde_wasm_bindgen::to_value(&self.inner.benefits)
            .map_err(|e| JsValue::from_str(&e.to_string()))?)
    }

    /// Get trade-offs as JSON array of strings.
    #[wasm_bindgen(getter, js_name = "tradeOffs")]
    pub fn trade_offs(&self) -> Result<JsValue, JsValue> {
        Ok(serde_wasm_bindgen::to_value(&self.inner.trade_offs)
            .map_err(|e| JsValue::from_str(&e.to_string()))?)
    }

    /// Technical details as JSON.
    #[wasm_bindgen(getter)]
    pub fn technical(&self) -> Result<JsValue, JsValue> {
        Ok(serde_wasm_bindgen::to_value(&self.inner.technical)
            .map_err(|e| JsValue::from_str(&e.to_string()))?)
    }
}

// =============================================================================
// AdvancedScorer
// =============================================================================

#[wasm_bindgen]
pub struct AdvancedScorer {
    inner: CoreAdvancedScorer,
}

#[wasm_bindgen]
impl AdvancedScorer {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: CoreAdvancedScorer::new(),
        }
    }

    /// Score a recommendation with impact, effort, confidence analysis.
    #[wasm_bindgen(js_name = "scoreRecommendation")]
    pub fn score_recommendation(
        &self,
        category: &str,
        before_overall: f64,
        before_compliance: f64,
        before_perceptual: f64,
        before_appropriateness: f64,
        after_overall: f64,
        after_compliance: f64,
        after_perceptual: f64,
        after_appropriateness: f64,
        delta_l: f64,
        delta_c: f64,
        delta_h: f64,
    ) -> AdvancedScore {
        let before = CoreQualityScore::new(
            before_overall,
            before_compliance,
            before_perceptual,
            before_appropriateness,
        );
        let after = CoreQualityScore::new(
            after_overall,
            after_compliance,
            after_perceptual,
            after_appropriateness,
        );
        let core = self
            .inner
            .score_recommendation(category, &before, &after, delta_l, delta_c, delta_h);
        AdvancedScore { inner: core }
    }
}

// =============================================================================
// AdvancedScore (result)
// =============================================================================

#[wasm_bindgen]
pub struct AdvancedScore {
    inner: CoreAdvancedScore,
}

#[wasm_bindgen]
impl AdvancedScore {
    #[wasm_bindgen(getter, js_name = "qualityOverall")]
    pub fn quality_overall(&self) -> f64 {
        self.inner.quality_overall
    }

    #[wasm_bindgen(getter)]
    pub fn impact(&self) -> f64 {
        self.inner.impact
    }

    #[wasm_bindgen(getter)]
    pub fn effort(&self) -> f64 {
        self.inner.effort
    }

    #[wasm_bindgen(getter)]
    pub fn confidence(&self) -> f64 {
        self.inner.confidence
    }

    #[wasm_bindgen(getter)]
    pub fn priority(&self) -> f64 {
        self.inner.priority
    }

    #[wasm_bindgen(js_name = "recommendationStrength")]
    pub fn recommendation_strength(&self) -> f64 {
        self.inner.recommendation_strength()
    }

    #[wasm_bindgen(js_name = "isStrongRecommendation")]
    pub fn is_strong_recommendation(&self) -> bool {
        self.inner.is_strong_recommendation()
    }

    /// Returns "Critical", "High", "Medium", or "Low".
    #[wasm_bindgen(js_name = "priorityAssessment")]
    pub fn priority_assessment(&self) -> String {
        self.inner.priority_assessment().to_string()
    }

    /// Full breakdown as JSON.
    #[wasm_bindgen(getter)]
    pub fn breakdown(&self) -> Result<JsValue, JsValue> {
        Ok(serde_wasm_bindgen::to_value(&self.inner.breakdown)
            .map_err(|e| JsValue::from_str(&e.to_string()))?)
    }
}

// =============================================================================
// ConvergenceDetector
// =============================================================================

#[wasm_bindgen]
pub struct ConvergenceDetector {
    inner: CoreConvergenceDetector,
}

#[wasm_bindgen]
impl ConvergenceDetector {
    /// Create with default config.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: CoreConvergenceDetector::with_defaults(),
        }
    }

    /// Create with "fast" preset (fewer iterations, lower threshold).
    #[wasm_bindgen(js_name = "fast")]
    pub fn fast() -> Self {
        Self {
            inner: CoreConvergenceDetector::new(CoreConvergenceConfig::fast()),
        }
    }

    /// Create with "high_quality" preset (more iterations, tighter threshold).
    #[wasm_bindgen(js_name = "highQuality")]
    pub fn high_quality() -> Self {
        Self {
            inner: CoreConvergenceDetector::new(CoreConvergenceConfig::high_quality()),
        }
    }

    /// Create with "neural" preset (optimized for neural correction loops).
    #[wasm_bindgen(js_name = "neural")]
    pub fn neural() -> Self {
        Self {
            inner: CoreConvergenceDetector::new(CoreConvergenceConfig::neural()),
        }
    }

    /// Feed a new quality value. Returns status as JSON.
    #[wasm_bindgen]
    pub fn update(&mut self, quality: f64) -> Result<JsValue, JsValue> {
        let status = self.inner.update(quality);
        let serializable = convergence_status_to_js(&status);
        Ok(serde_wasm_bindgen::to_value(&serializable)
            .map_err(|e| JsValue::from_str(&e.to_string()))?)
    }

    /// Reset the detector to initial state.
    #[wasm_bindgen]
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    /// Current best quality observed.
    #[wasm_bindgen(js_name = "bestQuality")]
    pub fn best_quality(&self) -> f64 {
        self.inner.best_quality()
    }

    /// Total iterations so far.
    #[wasm_bindgen(js_name = "iterationCount")]
    pub fn iteration_count(&self) -> usize {
        self.inner.iteration_count()
    }

    /// Total quality improvement from start.
    #[wasm_bindgen(js_name = "totalImprovement")]
    pub fn total_improvement(&self) -> f64 {
        self.inner.total_improvement()
    }

    /// Average improvement per iteration.
    #[wasm_bindgen(js_name = "improvementRate")]
    pub fn improvement_rate(&self) -> f64 {
        self.inner.improvement_rate()
    }

    /// Whether quality is still improving.
    #[wasm_bindgen(js_name = "isProgressing")]
    pub fn is_progressing(&self) -> bool {
        self.inner.is_progressing()
    }

    /// Full stats as JSON.
    #[wasm_bindgen]
    pub fn stats(&self) -> Result<JsValue, JsValue> {
        let stats = self.inner.stats();
        Ok(serde_wasm_bindgen::to_value(&stats).map_err(|e| JsValue::from_str(&e.to_string()))?)
    }
}

// =============================================================================
// StepSelector (Adaptive Pipeline)
// =============================================================================

#[wasm_bindgen]
pub struct StepSelector {
    inner: CoreStepSelector,
}

#[wasm_bindgen]
impl StepSelector {
    #[wasm_bindgen(constructor)]
    pub fn new(goal_type: &str, target: f64) -> Self {
        Self {
            inner: CoreStepSelector::new(goal_type, target),
        }
    }

    /// Update current progress value.
    #[wasm_bindgen(js_name = "updateProgress")]
    pub fn update_progress(&mut self, value: f64) {
        self.inner.update_progress(value);
    }

    /// Record outcome of a step execution.
    #[wasm_bindgen(js_name = "recordOutcome")]
    pub fn record_outcome(&mut self, step_type: &str, improvement: f64, cost: f64, success: bool) {
        self.inner
            .record_outcome(step_type, improvement, cost, success);
    }

    /// Get the next recommended step as JSON, or null if goal achieved.
    #[wasm_bindgen(js_name = "recommendNextStep")]
    pub fn recommend_next_step(&self) -> Result<JsValue, JsValue> {
        match self.inner.recommend_next_step() {
            Some(rec) => Ok(serde_wasm_bindgen::to_value(&rec)
                .map_err(|e| JsValue::from_str(&e.to_string()))?),
            None => Ok(JsValue::NULL),
        }
    }

    /// Current goal progress (0.0-1.0).
    #[wasm_bindgen(js_name = "goalProgress")]
    pub fn goal_progress(&self) -> f64 {
        self.inner.goal_progress()
    }

    /// Whether the goal has been achieved.
    #[wasm_bindgen(js_name = "isGoalAchieved")]
    pub fn is_goal_achieved(&self) -> bool {
        self.inner.is_goal_achieved()
    }
}

// =============================================================================
// CostEstimator
// =============================================================================

#[wasm_bindgen]
pub struct CostEstimator {
    inner: CoreCostEstimator,
}

#[wasm_bindgen]
impl CostEstimator {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: CoreCostEstimator::new(),
        }
    }

    /// Estimate cost for a step type with given factors.
    #[wasm_bindgen]
    pub fn estimate(
        &self,
        step_type: &str,
        color_count: usize,
        spectral: bool,
        neural: bool,
        material: bool,
    ) -> Result<JsValue, JsValue> {
        use momoto_intelligence::adaptive::cost_estimator::CostFactors;
        let mut factors = CostFactors::new().with_color_count(color_count);
        if spectral {
            factors = factors.with_spectral();
        }
        if neural {
            factors = factors.with_neural();
        }
        if material {
            factors = factors.with_material();
        }
        let est = self.inner.estimate(step_type, &factors);
        Ok(serde_wasm_bindgen::to_value(&est).map_err(|e| JsValue::from_str(&e.to_string()))?)
    }

    /// Estimate sequential cost for multiple steps.
    #[wasm_bindgen(js_name = "estimateSequential")]
    pub fn estimate_sequential(&self, steps_json: JsValue) -> Result<JsValue, JsValue> {
        let steps: Vec<(String, usize, bool, bool, bool)> =
            serde_wasm_bindgen::from_value(steps_json)
                .map_err(|e| JsValue::from_str(&e.to_string()))?;

        use momoto_intelligence::adaptive::cost_estimator::CostFactors;
        let step_refs: Vec<(&str, CostFactors)> = steps
            .iter()
            .map(|(s, cc, sp, ne, ma)| {
                let mut f = CostFactors::new().with_color_count(*cc);
                if *sp {
                    f = f.with_spectral();
                }
                if *ne {
                    f = f.with_neural();
                }
                if *ma {
                    f = f.with_material();
                }
                (s.as_str(), f)
            })
            .collect();

        let pairs: Vec<(&str, &CostFactors)> = step_refs.iter().map(|(s, f)| (*s, f)).collect();
        let est = self.inner.estimate_sequential(&pairs);
        Ok(serde_wasm_bindgen::to_value(&est).map_err(|e| JsValue::from_str(&e.to_string()))?)
    }
}

// =============================================================================
// Batch API: Score multiple pairs at once
// =============================================================================

/// Score multiple (fg, bg) color pairs in a single WASM call.
#[wasm_bindgen(js_name = "scorePairsBatch")]
pub fn score_pairs_batch(pairs: &[f64]) -> Result<Box<[f64]>, JsValue> {
    if pairs.len() % 8 != 0 {
        return Err(JsValue::from_str(
            "Input must be multiple of 8: [fg_r, fg_g, fg_b, bg_r, bg_g, bg_b, usage, target]",
        ));
    }

    let scorer = CoreQualityScorer::new();
    let count = pairs.len() / 8;
    let mut results = Vec::with_capacity(count * 4);

    for i in 0..count {
        let base = i * 8;
        let fg = CoreColor::from_srgb8(
            pairs[base] as u8,
            pairs[base + 1] as u8,
            pairs[base + 2] as u8,
        );
        let bg = CoreColor::from_srgb8(
            pairs[base + 3] as u8,
            pairs[base + 4] as u8,
            pairs[base + 5] as u8,
        );
        let context = CoreRecommendationContext::new(
            usage_from_u8(pairs[base + 6] as u8),
            compliance_from_u8(pairs[base + 7] as u8),
        );
        let score = scorer.score(fg, bg, context);
        results.push(score.overall);
        results.push(score.compliance);
        results.push(score.perceptual);
        results.push(score.appropriateness);
    }

    Ok(results.into_boxed_slice())
}

/// Recommend foreground colors for multiple backgrounds in a single WASM call.
#[wasm_bindgen(js_name = "recommendForegroundBatch")]
pub fn recommend_foreground_batch(backgrounds: &[u8]) -> Result<js_sys::Array, JsValue> {
    if backgrounds.len() % 5 != 0 {
        return Err(JsValue::from_str(
            "Input must be multiple of 5: [bg_r, bg_g, bg_b, usage, target]",
        ));
    }

    let engine = CoreRecommendationEngine::new();
    let count = backgrounds.len() / 5;
    let result = js_sys::Array::new_with_length(count as u32);

    for i in 0..count {
        let base = i * 5;
        let bg = CoreColor::from_srgb8(
            backgrounds[base],
            backgrounds[base + 1],
            backgrounds[base + 2],
        );
        let context = CoreRecommendationContext::new(
            usage_from_u8(backgrounds[base + 3]),
            compliance_from_u8(backgrounds[base + 4]),
        );
        let rec = engine.recommend_foreground(bg, context);
        let [r, g, b] = rec.color.to_srgb8();
        let entry = js_sys::Array::new();
        entry.push(&JsValue::from(r));
        entry.push(&JsValue::from(g));
        entry.push(&JsValue::from(b));
        entry.push(&JsValue::from(rec.score.overall));
        entry.push(&JsValue::from(rec.confidence));
        result.set(i as u32, entry.into());
    }

    Ok(result)
}

// =============================================================================
// Helpers (internal)
// =============================================================================

fn usage_from_u8(v: u8) -> CoreUsageContext {
    match v {
        0 => CoreUsageContext::BodyText,
        1 => CoreUsageContext::LargeText,
        2 => CoreUsageContext::Interactive,
        3 => CoreUsageContext::Decorative,
        4 => CoreUsageContext::IconsGraphics,
        5 => CoreUsageContext::Disabled,
        _ => CoreUsageContext::BodyText,
    }
}

fn compliance_from_u8(v: u8) -> CoreComplianceTarget {
    match v {
        0 => CoreComplianceTarget::WCAG_AA,
        1 => CoreComplianceTarget::WCAG_AAA,
        2 => CoreComplianceTarget::APCA,
        3 => CoreComplianceTarget::Hybrid,
        _ => CoreComplianceTarget::WCAG_AA,
    }
}

fn convergence_status_to_js(status: &CoreConvergenceStatus) -> serde_json::Value {
    match status {
        CoreConvergenceStatus::Converging {
            rate,
            estimated_iterations,
        } => {
            serde_json::json!({
                "type": "Converging",
                "rate": rate,
                "estimatedIterations": estimated_iterations,
            })
        }
        CoreConvergenceStatus::Converged {
            iterations,
            final_value,
        } => {
            serde_json::json!({
                "type": "Converged",
                "iterations": iterations,
                "finalValue": final_value,
            })
        }
        CoreConvergenceStatus::Oscillating {
            amplitude,
            frequency,
        } => {
            serde_json::json!({
                "type": "Oscillating",
                "amplitude": amplitude,
                "frequency": frequency,
            })
        }
        CoreConvergenceStatus::Diverging { rate } => {
            serde_json::json!({
                "type": "Diverging",
                "rate": rate,
            })
        }
        CoreConvergenceStatus::Stalled {
            stuck_at,
            iterations_stuck,
        } => {
            serde_json::json!({
                "type": "Stalled",
                "stuckAt": stuck_at,
                "iterationsStuck": iterations_stuck,
            })
        }
        CoreConvergenceStatus::Undetermined {
            current,
            iterations,
        } => {
            serde_json::json!({
                "type": "Undetermined",
                "current": current,
                "iterations": iterations,
            })
        }
    }
}

// =============================================================================
// COLOR HARMONY BINDINGS
// =============================================================================

use momoto_core::space::oklch::OKLCH as CoreOKLCH;
use momoto_intelligence::harmony::{
    generate_palette as core_generate_palette, harmony_score as core_harmony_score, hex_to_oklch,
    oklch_to_hex, shades as core_shades, temperature_palette as core_temperature_palette,
    HarmonyType as CoreHarmonyType,
};

/// Harmony type selector for WASM.
#[wasm_bindgen]
pub enum WasmHarmonyType {
    Complementary = 0,
    SplitComplementary = 1,
    Triadic = 2,
    Tetradic = 3,
    Analogous = 4,
    Monochromatic = 5,
    TemperatureWarm = 6,
    TemperatureCool = 7,
}

fn harmony_from_wasm(t: WasmHarmonyType) -> CoreHarmonyType {
    match t {
        WasmHarmonyType::Complementary => CoreHarmonyType::Complementary,
        WasmHarmonyType::SplitComplementary => CoreHarmonyType::SplitComplementary,
        WasmHarmonyType::Triadic => CoreHarmonyType::Triadic,
        WasmHarmonyType::Tetradic => CoreHarmonyType::Tetradic,
        WasmHarmonyType::Analogous => CoreHarmonyType::Analogous { spread: 30.0 },
        WasmHarmonyType::Monochromatic => CoreHarmonyType::Monochromatic { steps: 9 },
        WasmHarmonyType::TemperatureWarm => CoreHarmonyType::Temperature { warm: true },
        WasmHarmonyType::TemperatureCool => CoreHarmonyType::Temperature { warm: false },
    }
}

/// Generate a color palette from an OKLCH seed.
///
/// # Arguments
/// * `l`, `c`, `h` — seed color in OKLCH
/// * `harmony` — harmony type enum value
/// * `_count` — reserved (harmony type determines count)
///
/// # Returns
///
/// Flat array `[L0, C0, H0, L1, C1, H1, ...]` for all generated colors.
/// All colors are gamut-safe.
#[wasm_bindgen(js_name = "generatePalette")]
pub fn generate_palette(
    l: f64,
    c: f64,
    h: f64,
    harmony: WasmHarmonyType,
    _count: u32,
) -> Box<[f64]> {
    let seed = CoreOKLCH::new(l, c, h);
    let palette = core_generate_palette(seed, harmony_from_wasm(harmony));
    let mut out = Vec::with_capacity(palette.colors.len() * 3);
    for color in &palette.colors {
        out.push(color.l);
        out.push(color.c);
        out.push(color.h);
    }
    out.into_boxed_slice()
}

/// Compute harmony score for a palette.
///
/// # Arguments
/// * `lch_flat` — flat array `[L0, C0, H0, L1, C1, H1, ...]`
///
/// # Returns
///
/// Score in [0, 1] — higher is more harmonious.
#[wasm_bindgen(js_name = "harmonyScore")]
pub fn harmony_score_wasm(lch_flat: &[f64]) -> f64 {
    let n = lch_flat.len() / 3;
    let colors: Vec<CoreOKLCH> = (0..n)
        .map(|i| CoreOKLCH::new(lch_flat[i * 3], lch_flat[i * 3 + 1], lch_flat[i * 3 + 2]))
        .collect();
    core_harmony_score(&colors)
}

/// Generate tonal shades for a base color.
///
/// Returns flat array `[L0, C0, H0, ..., L9, C9, H9]` for 10 shades
/// (lightness descending: ~0.97 to ~0.12).
#[wasm_bindgen(js_name = "generateShades")]
pub fn generate_shades(l: f64, c: f64, h: f64) -> Box<[f64]> {
    let seed = CoreOKLCH::new(l, c, h);
    let shades = core_shades(seed, 10);
    let mut out = Vec::with_capacity(shades.len() * 3);
    for s in &shades {
        out.push(s.l);
        out.push(s.c);
        out.push(s.h);
    }
    out.into_boxed_slice()
}

/// Generate a warm or cool palette.
///
/// Returns flat array `[L0, C0, H0, ...]` for 5 colors.
#[wasm_bindgen(js_name = "temperaturePalette")]
pub fn temperature_palette_wasm(warm: bool) -> Box<[f64]> {
    let colors = core_temperature_palette(warm, 5);
    let mut out = Vec::with_capacity(colors.len() * 3);
    for c in &colors {
        out.push(c.l);
        out.push(c.c);
        out.push(c.h);
    }
    out.into_boxed_slice()
}

/// Generate a palette from a hex seed color.
///
/// # Arguments
/// * `hex` — seed color as hex string (e.g. "#3a7bd5")
/// * `harmony` — harmony type
///
/// # Returns
///
/// Flat `[L, C, H]` array, or empty array if hex is invalid.
#[wasm_bindgen(js_name = "generatePaletteFromHex")]
pub fn generate_palette_from_hex(hex: &str, harmony: WasmHarmonyType) -> Box<[f64]> {
    let Some(seed) = hex_to_oklch(hex) else {
        return Box::new([]);
    };
    let palette = core_generate_palette(seed, harmony_from_wasm(harmony));
    let mut out = Vec::with_capacity(palette.colors.len() * 3);
    for color in &palette.colors {
        out.push(color.l);
        out.push(color.c);
        out.push(color.h);
    }
    out.into_boxed_slice()
}

/// Convert OKLCH to hex string.
#[wasm_bindgen(js_name = "oklchToHex")]
pub fn oklch_to_hex_wasm(l: f64, c: f64, h: f64) -> String {
    oklch_to_hex(CoreOKLCH::new(l, c, h))
}

/// Convert hex string to OKLCH flat array `[L, C, H]`, or empty on error.
#[wasm_bindgen(js_name = "hexToOklch")]
pub fn hex_to_oklch_wasm(hex: &str) -> Box<[f64]> {
    match hex_to_oklch(hex) {
        Some(c) => vec![c.l, c.c, c.h].into_boxed_slice(),
        None => Box::new([]),
    }
}

// =============================================================================
// CVD (Color Vision Deficiency) BINDINGS
// =============================================================================

use momoto_core::color::cvd::{
    cvd_delta_e, parse_hex, simulate_cvd, simulate_cvd_hex, to_hex, CVDType,
};

/// Simulate how a hex color appears to a dichromat.
///
/// # Arguments
/// * `hex` — input color as hex string (e.g. "#ff0000")
/// * `cvd_type` — "protanopia", "deuteranopia", or "tritanopia"
///
/// # Returns
///
/// Simulated hex string (what the dichromat perceives), or original hex on error.
#[wasm_bindgen(js_name = "simulateCVD")]
pub fn simulate_cvd_wasm(hex: &str, cvd_type: &str) -> String {
    simulate_cvd_hex(hex, cvd_type)
}

/// Compute the perceptual ΔE between a color and its CVD simulation.
///
/// Higher values = more problematic for the given CVD type.
/// Typical ranges: < 20 = mild, 20–60 = moderate, > 60 = severe.
#[wasm_bindgen(js_name = "cvdDeltaE")]
pub fn cvd_delta_e_wasm(hex: &str, cvd_type: &str) -> f64 {
    let Some(cvd) = CVDType::from_str(cvd_type) else {
        return 0.0;
    };
    let Some(color) = parse_hex(hex) else {
        return 0.0;
    };
    cvd_delta_e(&color, cvd)
}

/// Simulate CVD for an OKLCH color. Returns flat `[L, C, H]` of simulated color.
#[wasm_bindgen(js_name = "simulateCVDOklch")]
pub fn simulate_cvd_oklch(l: f64, c: f64, h: f64, cvd_type: &str) -> Box<[f64]> {
    use momoto_core::space::oklch::OKLCH;

    let Some(cvd) = CVDType::from_str(cvd_type) else {
        return vec![l, c, h].into_boxed_slice();
    };

    let color = CoreOKLCH::new(l, c, h).to_color();
    let simulated = simulate_cvd(&color, cvd);
    let sim_lch = CoreOKLCH::from_color(&simulated);

    vec![sim_lch.l, sim_lch.c, sim_lch.h].into_boxed_slice()
}

// =============================================================================
// CONSTRAINT SOLVER BINDINGS
// =============================================================================

use momoto_intelligence::constraints::{
    ColorConstraint as CoreColorConstraint, ConstraintKind as CoreConstraintKind,
    ConstraintSolver as CoreConstraintSolver, SolverConfig as CoreSolverConfig,
};

/// Solve a set of color constraints for a palette.
///
/// # Arguments
/// * `lch_flat` — flat `[L0, C0, H0, L1, C1, H1, ...]` palette in OKLCH
/// * `constraints_json` — JSON array of constraint specs (see format below)
/// * `max_iterations` — override max iterations (0 = use default 500)
///
/// # Constraint JSON format
/// ```json
/// [
///   {"colorIdx":0,"kind":"MinContrast","otherIdx":1,"target":4.5},
///   {"colorIdx":0,"kind":"MinAPCA","otherIdx":1,"target":60.0},
///   {"colorIdx":0,"kind":"HarmonyAngle","otherIdx":1,"expectedDeltaH":180,"tolerance":5},
///   {"colorIdx":0,"kind":"InGamut"},
///   {"colorIdx":0,"kind":"LightnessRange","min":0.7,"max":1.0},
///   {"colorIdx":0,"kind":"ChromaRange","min":0.0,"max":0.2}
/// ]
/// ```
///
/// # Returns
/// JSON `{colors:[L,C,H,...], converged:bool, iterations:number, finalPenalty:number, violations:[...]}`
#[wasm_bindgen(js_name = "solveColorConstraints")]
pub fn solve_color_constraints(
    lch_flat: &[f64],
    constraints_json: &str,
    max_iterations: u32,
) -> Result<JsValue, JsValue> {
    let n = lch_flat.len() / 3;
    let colors: Vec<CoreOKLCH> = (0..n)
        .map(|i| CoreOKLCH::new(lch_flat[i * 3], lch_flat[i * 3 + 1], lch_flat[i * 3 + 2]))
        .collect();

    let specs: Vec<serde_json::Value> = serde_json::from_str(constraints_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid constraints JSON: {}", e)))?;

    let constraints: Vec<CoreColorConstraint> =
        specs.iter().filter_map(parse_wasm_constraint).collect();

    let mut config = CoreSolverConfig::default();
    if max_iterations > 0 {
        config.max_iterations = max_iterations as usize;
    }

    let mut solver = CoreConstraintSolver::new(colors, constraints, config);
    let result = solver.solve();

    let colors_flat: Vec<f64> = result.colors.iter().flat_map(|c| [c.l, c.c, c.h]).collect();

    let violations_json: Vec<serde_json::Value> = result
        .violations
        .iter()
        .map(|v| {
            serde_json::json!({
                "colorIdx": v.color_idx,
                "description": v.description,
                "magnitude": v.magnitude,
            })
        })
        .collect();

    let out = serde_json::json!({
        "colors": colors_flat,
        "converged": result.converged,
        "iterations": result.iterations,
        "finalPenalty": result.final_penalty,
        "violations": violations_json,
    });

    serde_wasm_bindgen::to_value(&out).map_err(|e| JsValue::from_str(&e.to_string()))
}

fn parse_wasm_constraint(c: &serde_json::Value) -> Option<CoreColorConstraint> {
    let color_idx = c["colorIdx"].as_u64()? as usize;
    let kind_str = c["kind"].as_str()?;

    let kind = match kind_str {
        "MinContrast" => CoreConstraintKind::MinContrast {
            other_idx: c["otherIdx"].as_u64()? as usize,
            target: c["target"].as_f64()?,
        },
        "MinAPCA" => CoreConstraintKind::MinAPCA {
            other_idx: c["otherIdx"].as_u64()? as usize,
            target: c["target"].as_f64()?,
        },
        "HarmonyAngle" => CoreConstraintKind::HarmonyAngle {
            other_idx: c["otherIdx"].as_u64()? as usize,
            expected_delta_h: c["expectedDeltaH"].as_f64()?,
            tolerance: c["tolerance"].as_f64().unwrap_or(5.0),
        },
        "InGamut" => CoreConstraintKind::InGamut,
        "LightnessRange" => CoreConstraintKind::LightnessRange {
            min: c["min"].as_f64()?,
            max: c["max"].as_f64()?,
        },
        "ChromaRange" => CoreConstraintKind::ChromaRange {
            min: c["min"].as_f64()?,
            max: c["max"].as_f64()?,
        },
        _ => return None,
    };

    Some(CoreColorConstraint { color_idx, kind })
}

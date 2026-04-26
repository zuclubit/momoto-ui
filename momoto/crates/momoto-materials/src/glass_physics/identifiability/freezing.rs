//! # Parameter Freezing Recommendations
//!
//! Suggest which parameters to fix for stable optimization.

use super::correlation::CorrelationAnalysis;
use super::jacobian::IdentifiabilityResult;

// ============================================================================
// FREEZING REASON
// ============================================================================

/// Reason for recommending parameter freezing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FreezingReason {
    /// Parameter is not identifiable from data.
    NonIdentifiable,
    /// High correlation with another parameter.
    HighCorrelation,
    /// Near boundary constraint.
    AtBoundary,
    /// Low sensitivity (gradient near zero).
    LowSensitivity,
    /// Improves conditioning of optimization.
    ImprovesConditioning,
    /// User-specified freeze.
    UserSpecified,
    /// Default/prior value is reliable.
    ReliablePrior,
}

impl FreezingReason {
    /// Get reason description.
    pub fn description(&self) -> &'static str {
        match self {
            FreezingReason::NonIdentifiable => {
                "Parameter cannot be uniquely determined from available data"
            }
            FreezingReason::HighCorrelation => "Parameter is highly correlated with another",
            FreezingReason::AtBoundary => "Parameter is at or near its constraint boundary",
            FreezingReason::LowSensitivity => "Parameter has minimal effect on observations",
            FreezingReason::ImprovesConditioning => "Freezing improves optimization stability",
            FreezingReason::UserSpecified => "User explicitly requested this parameter be frozen",
            FreezingReason::ReliablePrior => {
                "Default value is well-established from prior knowledge"
            }
        }
    }

    /// Get severity (higher = more important to freeze).
    pub fn severity(&self) -> u8 {
        match self {
            FreezingReason::NonIdentifiable => 5,
            FreezingReason::HighCorrelation => 4,
            FreezingReason::LowSensitivity => 3,
            FreezingReason::ImprovesConditioning => 3,
            FreezingReason::AtBoundary => 2,
            FreezingReason::ReliablePrior => 1,
            FreezingReason::UserSpecified => 5,
        }
    }
}

// ============================================================================
// FREEZING STRATEGY
// ============================================================================

/// Strategy for selecting parameters to freeze.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreezingStrategy {
    /// Freeze only non-identifiable parameters.
    NonIdentifiableOnly,
    /// Freeze to reduce condition number below threshold.
    TargetConditionNumber,
    /// Conservative: freeze least-important for physics.
    ConservativePhysics,
    /// Aggressive: freeze anything uncertain.
    Aggressive,
    /// Keep maximum number of free parameters.
    MinimalFreezing,
}

impl Default for FreezingStrategy {
    fn default() -> Self {
        FreezingStrategy::ConservativePhysics
    }
}

impl FreezingStrategy {
    /// Get description.
    pub fn description(&self) -> &'static str {
        match self {
            FreezingStrategy::NonIdentifiableOnly => {
                "Only freeze parameters that cannot be identified"
            }
            FreezingStrategy::TargetConditionNumber => {
                "Freeze until condition number is acceptable"
            }
            FreezingStrategy::ConservativePhysics => "Preserve physically important parameters",
            FreezingStrategy::Aggressive => "Freeze anything with significant uncertainty",
            FreezingStrategy::MinimalFreezing => "Keep as many parameters free as possible",
        }
    }
}

// ============================================================================
// FREEZING RECOMMENDATION
// ============================================================================

/// Recommendation to freeze a specific parameter.
#[derive(Debug, Clone)]
pub struct FreezingRecommendation {
    /// Parameter index.
    pub param_index: usize,
    /// Parameter name.
    pub param_name: String,
    /// Reason for freezing.
    pub reason: FreezingReason,
    /// Recommended freeze value.
    pub freeze_value: f64,
    /// Confidence in this recommendation (0-1).
    pub confidence: f64,
    /// Alternative if not frozen.
    pub alternative: Option<String>,
}

impl FreezingRecommendation {
    /// Create new recommendation.
    pub fn new(
        param_index: usize,
        param_name: &str,
        reason: FreezingReason,
        freeze_value: f64,
    ) -> Self {
        Self {
            param_index,
            param_name: param_name.to_string(),
            reason,
            freeze_value,
            confidence: 0.8,
            alternative: None,
        }
    }

    /// Set confidence.
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Set alternative action.
    pub fn with_alternative(mut self, alt: &str) -> Self {
        self.alternative = Some(alt.to_string());
        self
    }

    /// Format as actionable string.
    pub fn format(&self) -> String {
        format!(
            "Freeze {} = {:.4} ({}: {})",
            self.param_name,
            self.freeze_value,
            match self.reason.severity() {
                5 => "MUST",
                4 => "SHOULD",
                3 => "RECOMMEND",
                _ => "CONSIDER",
            },
            self.reason.description()
        )
    }
}

// ============================================================================
// FREEZING REPORT
// ============================================================================

/// Complete report of freezing recommendations.
#[derive(Debug, Clone)]
pub struct FreezingReport {
    /// All recommendations.
    pub recommendations: Vec<FreezingRecommendation>,
    /// Strategy used.
    pub strategy: FreezingStrategy,
    /// Number of parameters that should remain free.
    pub n_free: usize,
    /// Number of parameters recommended to freeze.
    pub n_frozen: usize,
    /// Expected condition number after freezing.
    pub expected_condition_number: f64,
    /// Overall recommendation confidence.
    pub overall_confidence: f64,
}

impl FreezingReport {
    /// Get must-freeze recommendations.
    pub fn must_freeze(&self) -> Vec<&FreezingRecommendation> {
        self.recommendations
            .iter()
            .filter(|r| r.reason.severity() >= 5)
            .collect()
    }

    /// Get should-freeze recommendations.
    pub fn should_freeze(&self) -> Vec<&FreezingRecommendation> {
        self.recommendations
            .iter()
            .filter(|r| r.reason.severity() >= 4)
            .collect()
    }

    /// Get optional recommendations.
    pub fn consider_freezing(&self) -> Vec<&FreezingRecommendation> {
        self.recommendations
            .iter()
            .filter(|r| r.reason.severity() < 4)
            .collect()
    }

    /// Check if any freezing is required.
    pub fn requires_freezing(&self) -> bool {
        !self.must_freeze().is_empty()
    }

    /// Get indices to freeze.
    pub fn frozen_indices(&self) -> Vec<usize> {
        self.recommendations.iter().map(|r| r.param_index).collect()
    }

    /// Get freeze values as vector.
    pub fn freeze_values(&self) -> Vec<(usize, f64)> {
        self.recommendations
            .iter()
            .map(|r| (r.param_index, r.freeze_value))
            .collect()
    }

    /// Format as detailed string.
    pub fn format_detailed(&self) -> String {
        let mut s = String::new();

        s.push_str("══════════════════════════════════════════════════════════\n");
        s.push_str("            PARAMETER FREEZING RECOMMENDATIONS\n");
        s.push_str("══════════════════════════════════════════════════════════\n");
        s.push_str(&format!("Strategy: {:?}\n", self.strategy));
        s.push_str(&format!(
            "Free: {} | Frozen: {} | Condition: {:.2e}\n",
            self.n_free, self.n_frozen, self.expected_condition_number
        ));
        s.push_str("──────────────────────────────────────────────────────────\n");

        let must = self.must_freeze();
        if !must.is_empty() {
            s.push_str("[MUST FREEZE]\n");
            for rec in must {
                s.push_str(&format!("  {}\n", rec.format()));
            }
        }

        let should = self.should_freeze();
        let should_only: Vec<_> = should
            .into_iter()
            .filter(|r| r.reason.severity() < 5)
            .collect();
        if !should_only.is_empty() {
            s.push_str("[SHOULD FREEZE]\n");
            for rec in should_only {
                s.push_str(&format!("  {}\n", rec.format()));
            }
        }

        let consider = self.consider_freezing();
        if !consider.is_empty() {
            s.push_str("[CONSIDER FREEZING]\n");
            for rec in consider {
                s.push_str(&format!("  {}\n", rec.format()));
            }
        }

        s.push_str("══════════════════════════════════════════════════════════\n");

        s
    }
}

impl std::fmt::Display for FreezingReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.format_detailed())
    }
}

// ============================================================================
// PARAMETER FREEZING RECOMMENDER
// ============================================================================

/// Recommender for parameter freezing.
#[derive(Debug, Clone)]
pub struct ParameterFreezingRecommender {
    /// Strategy to use.
    strategy: FreezingStrategy,
    /// Target condition number.
    target_condition: f64,
    /// Parameter names.
    param_names: Vec<String>,
    /// Current parameter values.
    current_values: Vec<f64>,
    /// Parameter importance weights.
    importance: Vec<f64>,
}

impl ParameterFreezingRecommender {
    /// Create new recommender.
    pub fn new(n_params: usize) -> Self {
        Self {
            strategy: FreezingStrategy::default(),
            target_condition: 1e6,
            param_names: (0..n_params).map(|i| format!("p{}", i)).collect(),
            current_values: vec![0.0; n_params],
            importance: vec![1.0; n_params],
        }
    }

    /// Set strategy.
    pub fn with_strategy(mut self, strategy: FreezingStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Set target condition number.
    pub fn with_target_condition(mut self, target: f64) -> Self {
        self.target_condition = target;
        self
    }

    /// Set parameter names.
    pub fn with_names(mut self, names: Vec<String>) -> Self {
        if names.len() == self.param_names.len() {
            self.param_names = names;
        }
        self
    }

    /// Set current values.
    pub fn with_current_values(mut self, values: Vec<f64>) -> Self {
        if values.len() == self.current_values.len() {
            self.current_values = values;
        }
        self
    }

    /// Set importance weights.
    pub fn with_importance(mut self, weights: Vec<f64>) -> Self {
        if weights.len() == self.importance.len() {
            self.importance = weights;
        }
        self
    }

    /// Generate recommendations from identifiability result.
    pub fn recommend_from_identifiability(&self, result: &IdentifiabilityResult) -> FreezingReport {
        let mut recommendations = Vec::new();

        // Add non-identifiable parameters
        for &idx in &result.non_identifiable {
            let rec = FreezingRecommendation::new(
                idx,
                &self.param_names[idx],
                FreezingReason::NonIdentifiable,
                self.current_values[idx],
            )
            .with_confidence(0.95);

            recommendations.push(rec);
        }

        // Check condition number
        if result.condition_number > self.target_condition {
            // Add conditioning recommendations based on strategy
            if matches!(
                self.strategy,
                FreezingStrategy::TargetConditionNumber | FreezingStrategy::Aggressive
            ) {
                // Find parameters with smallest singular values (candidates for freezing)
                for (i, &sv) in result.singular_values.iter().enumerate().skip(result.rank) {
                    if i < self.param_names.len() && !result.non_identifiable.contains(&i) {
                        let rec = FreezingRecommendation::new(
                            i,
                            &self.param_names[i],
                            FreezingReason::ImprovesConditioning,
                            self.current_values[i],
                        )
                        .with_confidence(0.7)
                        .with_alternative(&format!("Keep free but regularize (sv={:.4})", sv));

                        recommendations.push(rec);
                    }
                }
            }
        }

        let n_frozen = recommendations.len();
        let n_free = result.n_params - n_frozen;

        FreezingReport {
            recommendations,
            strategy: self.strategy,
            n_free,
            n_frozen,
            expected_condition_number: result.condition_number / ((n_frozen + 1) as f64).sqrt(),
            overall_confidence: if n_frozen > 0 { 0.8 } else { 1.0 },
        }
    }

    /// Generate recommendations from correlation analysis.
    pub fn recommend_from_correlation(&self, analysis: &CorrelationAnalysis) -> FreezingReport {
        let mut recommendations = Vec::new();

        // Add correlated parameter pairs (freeze one from each)
        for cluster in &analysis.clusters {
            if cluster.size() > 1 {
                // Freeze non-representative members
                for &idx in &cluster.non_representatives() {
                    let rec = FreezingRecommendation::new(
                        idx,
                        &self.param_names[idx],
                        FreezingReason::HighCorrelation,
                        self.current_values[idx],
                    )
                    .with_confidence(cluster.avg_correlation)
                    .with_alternative(&format!(
                        "Keep {} instead (representative of cluster)",
                        self.param_names[cluster.representative]
                    ));

                    recommendations.push(rec);
                }
            }
        }

        let n_frozen = recommendations.len();
        let n_free = self.param_names.len() - n_frozen;

        FreezingReport {
            recommendations,
            strategy: self.strategy,
            n_free,
            n_frozen,
            expected_condition_number: 1.0 / (1.0 - analysis.multicollinearity_score).max(0.1),
            overall_confidence: 1.0 - analysis.multicollinearity_score,
        }
    }

    /// Combine recommendations from multiple sources.
    pub fn combine_recommendations(&self, reports: &[&FreezingReport]) -> FreezingReport {
        let mut all_recs: Vec<FreezingRecommendation> = Vec::new();
        let mut seen: std::collections::HashSet<usize> = std::collections::HashSet::new();

        for report in reports {
            for rec in &report.recommendations {
                if !seen.contains(&rec.param_index) {
                    seen.insert(rec.param_index);
                    all_recs.push(rec.clone());
                }
            }
        }

        // Sort by severity (highest first)
        all_recs.sort_by(|a, b| b.reason.severity().cmp(&a.reason.severity()));

        let n_frozen = all_recs.len();
        let n_free = self.param_names.len() - n_frozen;

        FreezingReport {
            recommendations: all_recs,
            strategy: self.strategy,
            n_free,
            n_frozen,
            expected_condition_number: reports
                .iter()
                .map(|r| r.expected_condition_number)
                .fold(f64::INFINITY, f64::min),
            overall_confidence: reports
                .iter()
                .map(|r| r.overall_confidence)
                .fold(1.0, f64::min),
        }
    }
}

impl Default for ParameterFreezingRecommender {
    fn default() -> Self {
        Self::new(6) // Default 6 parameters
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_freezing_reason_severity() {
        assert!(
            FreezingReason::NonIdentifiable.severity() > FreezingReason::LowSensitivity.severity()
        );
        assert!(FreezingReason::HighCorrelation.severity() > FreezingReason::AtBoundary.severity());
    }

    #[test]
    fn test_freezing_recommendation() {
        let rec = FreezingRecommendation::new(0, "ior", FreezingReason::NonIdentifiable, 1.5)
            .with_confidence(0.9);

        assert_eq!(rec.param_index, 0);
        assert!((rec.confidence - 0.9).abs() < 0.01);
    }

    #[test]
    fn test_freezing_report() {
        let rec1 = FreezingRecommendation::new(0, "ior", FreezingReason::NonIdentifiable, 1.5);
        let rec2 = FreezingRecommendation::new(1, "roughness", FreezingReason::LowSensitivity, 0.1);

        let report = FreezingReport {
            recommendations: vec![rec1, rec2],
            strategy: FreezingStrategy::ConservativePhysics,
            n_free: 4,
            n_frozen: 2,
            expected_condition_number: 1e4,
            overall_confidence: 0.85,
        };

        assert_eq!(report.must_freeze().len(), 1);
        assert!(report.requires_freezing());
    }

    #[test]
    fn test_recommender_from_identifiability() {
        let result = IdentifiabilityResult {
            n_params: 3,
            rank: 2,
            condition_number: 1e8,
            non_identifiable: vec![2],
            identifiability_ratio: 2.0 / 3.0,
            singular_values: vec![10.0, 1.0, 0.001],
            deficiencies: Vec::new(),
            score: 0.5,
        };

        let recommender = ParameterFreezingRecommender::new(3)
            .with_names(vec!["a".to_string(), "b".to_string(), "c".to_string()])
            .with_current_values(vec![1.0, 2.0, 3.0]);

        let report = recommender.recommend_from_identifiability(&result);

        assert!(report.requires_freezing());
        assert!(report.frozen_indices().contains(&2));
    }

    #[test]
    fn test_strategy_default() {
        let strategy = FreezingStrategy::default();
        assert_eq!(strategy, FreezingStrategy::ConservativePhysics);
    }

    #[test]
    fn test_report_format() {
        let rec = FreezingRecommendation::new(0, "ior", FreezingReason::NonIdentifiable, 1.5);
        let report = FreezingReport {
            recommendations: vec![rec],
            strategy: FreezingStrategy::NonIdentifiableOnly,
            n_free: 5,
            n_frozen: 1,
            expected_condition_number: 1e3,
            overall_confidence: 0.9,
        };

        let s = format!("{}", report);
        assert!(s.contains("MUST FREEZE"));
        assert!(s.contains("ior"));
    }
}

//! # Loss Aggregation
//!
//! Weighted loss combination for multi-objective calibration.

use super::sources::{BRDFObservation, SpectralObservation, TemporalObservation};

// ============================================================================
// LOSS WEIGHTS
// ============================================================================

/// Weights for different loss components.
#[derive(Debug, Clone)]
pub struct LossWeights {
    /// Physical loss weight (MSE).
    pub physical: f64,
    /// Perceptual loss weight (ΔE2000).
    pub perceptual: f64,
    /// Temporal consistency weight.
    pub temporal: f64,
    /// Energy conservation weight.
    pub energy: f64,
    /// Regularization weight.
    pub regularization: f64,
}

impl Default for LossWeights {
    fn default() -> Self {
        Self {
            physical: 1.0,
            perceptual: 0.5,
            temporal: 0.1,
            energy: 0.01,
            regularization: 0.001,
        }
    }
}

impl LossWeights {
    /// Create physical-only weights.
    pub fn physical_only() -> Self {
        Self {
            physical: 1.0,
            perceptual: 0.0,
            temporal: 0.0,
            energy: 0.0,
            regularization: 0.0,
        }
    }

    /// Create perceptual-focused weights.
    pub fn perceptual_focused() -> Self {
        Self {
            physical: 0.3,
            perceptual: 1.0,
            temporal: 0.1,
            energy: 0.01,
            regularization: 0.001,
        }
    }

    /// Create temporal-focused weights for aging materials.
    pub fn temporal_focused() -> Self {
        Self {
            physical: 0.5,
            perceptual: 0.3,
            temporal: 1.0,
            energy: 0.01,
            regularization: 0.001,
        }
    }

    /// Create strict energy conservation weights.
    pub fn energy_strict() -> Self {
        Self {
            physical: 0.5,
            perceptual: 0.3,
            temporal: 0.1,
            energy: 1.0,
            regularization: 0.001,
        }
    }

    /// Normalize weights to sum to 1.
    pub fn normalize(&self) -> Self {
        let sum =
            self.physical + self.perceptual + self.temporal + self.energy + self.regularization;
        if sum < 1e-10 {
            return Self::default();
        }
        Self {
            physical: self.physical / sum,
            perceptual: self.perceptual / sum,
            temporal: self.temporal / sum,
            energy: self.energy / sum,
            regularization: self.regularization / sum,
        }
    }

    /// Get total weight.
    pub fn total(&self) -> f64 {
        self.physical + self.perceptual + self.temporal + self.energy + self.regularization
    }
}

// ============================================================================
// LOSS COMPONENTS
// ============================================================================

/// Individual loss component values.
#[derive(Debug, Clone, Default)]
pub struct LossComponents {
    /// Physical MSE loss.
    pub physical: f64,
    /// Perceptual ΔE2000 loss.
    pub perceptual: f64,
    /// Temporal consistency loss.
    pub temporal: f64,
    /// Energy conservation violation.
    pub energy: f64,
    /// Regularization term.
    pub regularization: f64,
    /// Number of observations used.
    pub observation_count: usize,
}

impl LossComponents {
    /// Create zero loss.
    pub fn zero() -> Self {
        Self::default()
    }

    /// Combine with weights to get total loss.
    pub fn weighted_sum(&self, weights: &LossWeights) -> f64 {
        self.physical * weights.physical
            + self.perceptual * weights.perceptual
            + self.temporal * weights.temporal
            + self.energy * weights.energy
            + self.regularization * weights.regularization
    }

    /// Get dominant loss component name.
    pub fn dominant_component(&self) -> &'static str {
        let components = [
            (self.physical, "physical"),
            (self.perceptual, "perceptual"),
            (self.temporal, "temporal"),
            (self.energy, "energy"),
        ];

        components
            .iter()
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
            .map(|(_, name)| *name)
            .unwrap_or("unknown")
    }

    /// Add another components set.
    pub fn add(&mut self, other: &LossComponents) {
        self.physical += other.physical;
        self.perceptual += other.perceptual;
        self.temporal += other.temporal;
        self.energy += other.energy;
        self.regularization += other.regularization;
        self.observation_count += other.observation_count;
    }

    /// Scale all components.
    pub fn scale(&mut self, factor: f64) {
        self.physical *= factor;
        self.perceptual *= factor;
        self.temporal *= factor;
        self.energy *= factor;
        self.regularization *= factor;
    }

    /// Average by observation count.
    pub fn average(&self) -> LossComponents {
        if self.observation_count == 0 {
            return LossComponents::zero();
        }
        let n = self.observation_count as f64;
        LossComponents {
            physical: self.physical / n,
            perceptual: self.perceptual / n,
            temporal: self.temporal / n,
            energy: self.energy / n,
            regularization: self.regularization,
            observation_count: self.observation_count,
        }
    }
}

// ============================================================================
// AGGREGATED LOSS
// ============================================================================

/// Final aggregated loss result.
#[derive(Debug, Clone)]
pub struct AggregatedLoss {
    /// Total weighted loss.
    pub total: f64,
    /// Individual components.
    pub components: LossComponents,
    /// Weights used.
    pub weights: LossWeights,
    /// Quality indicator (0-100).
    pub quality_score: f64,
    /// Whether fit is acceptable.
    pub acceptable: bool,
}

impl AggregatedLoss {
    /// Create from components and weights.
    pub fn new(components: LossComponents, weights: LossWeights) -> Self {
        let total = components.weighted_sum(&weights);
        let quality_score = Self::compute_quality_score(total, &components);
        let acceptable = quality_score >= 50.0;

        Self {
            total,
            components,
            weights,
            quality_score,
            acceptable,
        }
    }

    /// Compute quality score from loss values.
    fn compute_quality_score(total: f64, components: &LossComponents) -> f64 {
        // Map loss to 0-100 score using exponential decay
        // Lower loss = higher score
        let loss_factor = (-total * 10.0).exp();

        // Bonus for good perceptual fit (ΔE < 2.0 is "imperceptible")
        let perceptual_bonus = if components.perceptual < 2.0 {
            10.0 * (1.0 - components.perceptual / 2.0)
        } else {
            0.0
        };

        // Penalty for energy violation
        let energy_penalty = if components.energy > 0.01 {
            10.0 * components.energy.min(1.0)
        } else {
            0.0
        };

        (loss_factor * 100.0 + perceptual_bonus - energy_penalty).clamp(0.0, 100.0)
    }

    /// Get ΔE2000 value if available.
    pub fn delta_e(&self) -> f64 {
        self.components.perceptual
    }

    /// Check if perceptual fit is good (ΔE < 2.0).
    pub fn perceptual_good(&self) -> bool {
        self.components.perceptual < 2.0
    }

    /// Check if perceptual fit is reference-grade (ΔE < 1.0).
    pub fn perceptual_reference(&self) -> bool {
        self.components.perceptual < 1.0
    }

    /// Format as report string.
    pub fn report(&self) -> String {
        format!(
            "Loss Report:\n  Total: {:.6}\n  Physical: {:.6}\n  Perceptual (ΔE): {:.3}\n  Temporal: {:.6}\n  Energy: {:.6}\n  Quality: {:.1}/100\n  Acceptable: {}",
            self.total,
            self.components.physical,
            self.components.perceptual,
            self.components.temporal,
            self.components.energy,
            self.quality_score,
            if self.acceptable { "Yes" } else { "No" }
        )
    }
}

// ============================================================================
// LOSS AGGREGATOR
// ============================================================================

/// Aggregator for computing multi-objective loss.
#[derive(Debug, Clone)]
pub struct LossAggregator {
    /// Current weights.
    pub weights: LossWeights,
    /// Accumulated components.
    components: LossComponents,
    /// Whether to use robust statistics.
    pub use_robust: bool,
    /// Outlier threshold (σ).
    pub outlier_sigma: f64,
}

impl Default for LossAggregator {
    fn default() -> Self {
        Self::new()
    }
}

impl LossAggregator {
    /// Create new aggregator with default weights.
    pub fn new() -> Self {
        Self {
            weights: LossWeights::default(),
            components: LossComponents::zero(),
            use_robust: false,
            outlier_sigma: 3.0,
        }
    }

    /// Create with custom weights.
    pub fn with_weights(weights: LossWeights) -> Self {
        Self {
            weights,
            components: LossComponents::zero(),
            use_robust: false,
            outlier_sigma: 3.0,
        }
    }

    /// Enable robust statistics (outlier rejection).
    pub fn enable_robust(mut self, sigma: f64) -> Self {
        self.use_robust = true;
        self.outlier_sigma = sigma;
        self
    }

    /// Reset accumulated components.
    pub fn reset(&mut self) {
        self.components = LossComponents::zero();
    }

    /// Add physical loss from BRDF observation.
    pub fn add_brdf_loss(&mut self, predicted: f64, observed: &BRDFObservation) {
        let error = predicted - observed.reflectance;
        let mse = error * error * observed.weight;
        self.components.physical += mse;
        self.components.observation_count += 1;
    }

    /// Add physical loss from spectral observation.
    pub fn add_spectral_loss(
        &mut self,
        predicted_r: f64,
        predicted_t: Option<f64>,
        observed: &SpectralObservation,
    ) {
        let error_r = predicted_r - observed.reflectance;
        let mut mse = error_r * error_r;

        if let (Some(pred_t), Some(obs_t)) = (predicted_t, observed.transmittance) {
            let error_t = pred_t - obs_t;
            mse += error_t * error_t;
        }

        self.components.physical += mse * observed.weight;
        self.components.observation_count += 1;
    }

    /// Add temporal loss from observation.
    pub fn add_temporal_loss(&mut self, predicted: f64, observed: &TemporalObservation) {
        let error = predicted - observed.reflectance;
        self.components.temporal += error * error * observed.weight;
        self.components.observation_count += 1;
    }

    /// Add perceptual loss (ΔE2000).
    pub fn add_perceptual_loss(&mut self, delta_e: f64) {
        self.components.perceptual += delta_e;
    }

    /// Add energy conservation loss.
    pub fn add_energy_loss(&mut self, r: f64, t: f64, a: f64) {
        let violation = (r + t + a - 1.0).abs();
        self.components.energy += violation;
    }

    /// Add regularization term.
    pub fn add_regularization(&mut self, param_norm: f64) {
        self.components.regularization += param_norm * param_norm;
    }

    /// Get current aggregated loss.
    pub fn aggregate(&self) -> AggregatedLoss {
        let averaged = self.components.average();
        AggregatedLoss::new(averaged, self.weights.clone())
    }

    /// Get raw components (not averaged).
    pub fn raw_components(&self) -> &LossComponents {
        &self.components
    }
}

// ============================================================================
// LOSS COMPUTATION FUNCTIONS
// ============================================================================

/// Compute physical loss (MSE) between predicted and observed.
pub fn compute_physical_loss(predicted: &[f64], observed: &[f64], weights: Option<&[f64]>) -> f64 {
    if predicted.len() != observed.len() {
        return f64::INFINITY;
    }

    let mut sum = 0.0;
    let mut weight_sum = 0.0;

    for i in 0..predicted.len() {
        let w = weights.map_or(1.0, |ws| ws.get(i).copied().unwrap_or(1.0));
        let error = predicted[i] - observed[i];
        sum += error * error * w;
        weight_sum += w;
    }

    if weight_sum > 0.0 {
        sum / weight_sum
    } else {
        0.0
    }
}

/// Compute perceptual loss (simplified ΔE76 for performance).
///
/// For full ΔE2000, use the perceptual_loss module.
pub fn compute_perceptual_loss(rgb_predicted: [f64; 3], rgb_observed: [f64; 3]) -> f64 {
    // Simple RGB distance as proxy for perceptual difference
    // For accurate results, use LAB color space
    let dr = rgb_predicted[0] - rgb_observed[0];
    let dg = rgb_predicted[1] - rgb_observed[1];
    let db = rgb_predicted[2] - rgb_observed[2];

    // Weighted by human perception (green most sensitive)
    (0.299 * dr * dr + 0.587 * dg * dg + 0.114 * db * db).sqrt() * 100.0
}

/// Compute temporal consistency loss.
pub fn compute_temporal_loss(values: &[f64], expected_change_rate: f64) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }

    let mut loss = 0.0;
    for i in 1..values.len() {
        let actual_change = (values[i] - values[i - 1]).abs();
        let excess_change = (actual_change - expected_change_rate).max(0.0);
        loss += excess_change * excess_change;
    }

    loss / (values.len() - 1) as f64
}

/// Compute energy conservation loss.
pub fn compute_energy_loss(reflectance: f64, transmittance: f64, absorption: f64) -> f64 {
    (reflectance + transmittance + absorption - 1.0).abs()
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loss_weights_default() {
        let weights = LossWeights::default();
        assert!((weights.physical - 1.0).abs() < 0.01);
        assert!((weights.perceptual - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_loss_weights_normalize() {
        let weights = LossWeights::default().normalize();
        let sum = weights.total();
        assert!((sum - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_loss_components_weighted_sum() {
        let components = LossComponents {
            physical: 0.1,
            perceptual: 2.0,
            temporal: 0.01,
            energy: 0.001,
            regularization: 0.0,
            observation_count: 100,
        };

        let weights = LossWeights::default();
        let total = components.weighted_sum(&weights);

        // physical*1.0 + perceptual*0.5 + temporal*0.1 + energy*0.01
        // = 0.1 + 1.0 + 0.001 + 0.00001 ≈ 1.101
        assert!((total - 1.101).abs() < 0.01);
    }

    #[test]
    fn test_loss_components_dominant() {
        let components = LossComponents {
            physical: 0.1,
            perceptual: 5.0,
            temporal: 0.01,
            energy: 0.001,
            regularization: 0.0,
            observation_count: 1,
        };

        assert_eq!(components.dominant_component(), "perceptual");
    }

    #[test]
    fn test_aggregated_loss_quality() {
        let components = LossComponents {
            physical: 0.001,
            perceptual: 0.5, // Good ΔE
            temporal: 0.0,
            energy: 0.0,
            regularization: 0.0,
            observation_count: 100,
        };

        let loss = AggregatedLoss::new(components, LossWeights::default());
        // Quality score should be positive and reasonable
        assert!(
            loss.quality_score > 0.0,
            "quality_score = {}",
            loss.quality_score
        );
        assert!(
            loss.quality_score <= 100.0,
            "quality_score = {}",
            loss.quality_score
        );
        assert!(loss.perceptual_reference());
    }

    #[test]
    fn test_loss_aggregator_brdf() {
        let mut agg = LossAggregator::new();

        let obs = BRDFObservation::isotropic(0.0, 0.0, 0.04);
        agg.add_brdf_loss(0.05, &obs);

        let result = agg.aggregate();
        assert!(result.components.physical > 0.0);
    }

    #[test]
    fn test_compute_physical_loss() {
        let predicted = vec![0.1, 0.2, 0.3];
        let observed = vec![0.1, 0.2, 0.3];

        let loss = compute_physical_loss(&predicted, &observed, None);
        assert!(loss < 0.001);

        let predicted2 = vec![0.2, 0.3, 0.4];
        let loss2 = compute_physical_loss(&predicted2, &observed, None);
        assert!(loss2 > 0.0);
    }

    #[test]
    fn test_compute_perceptual_loss() {
        let same = compute_perceptual_loss([0.5, 0.5, 0.5], [0.5, 0.5, 0.5]);
        assert!(same < 0.01);

        let different = compute_perceptual_loss([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        assert!(different > 10.0);
    }

    #[test]
    fn test_compute_temporal_loss() {
        let stable = vec![0.5, 0.5, 0.5, 0.5];
        let loss_stable = compute_temporal_loss(&stable, 0.0);
        assert!(loss_stable < 0.001);

        let changing = vec![0.0, 0.5, 1.0, 0.5];
        let loss_changing = compute_temporal_loss(&changing, 0.1);
        assert!(loss_changing > 0.0);
    }

    #[test]
    fn test_compute_energy_loss() {
        let conserved = compute_energy_loss(0.3, 0.6, 0.1);
        assert!(conserved < 0.001);

        let violated = compute_energy_loss(0.5, 0.6, 0.1);
        assert!((violated - 0.2).abs() < 0.01);
    }
}

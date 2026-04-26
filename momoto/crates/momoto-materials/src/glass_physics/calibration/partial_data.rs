//! # Partial Data Handling
//!
//! Imputation strategies and quality assessment for incomplete calibration data.

// ============================================================================
// DATA QUALITY
// ============================================================================

/// Quality level of calibration data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataQuality {
    /// Reference-grade data with full coverage.
    Reference,
    /// High quality with good coverage.
    High,
    /// Standard quality with acceptable gaps.
    Standard,
    /// Low quality with significant gaps.
    Low,
    /// Poor quality, may need imputation.
    Poor,
}

impl DataQuality {
    /// Get numeric score (0-100).
    pub fn score(&self) -> f64 {
        match self {
            DataQuality::Reference => 100.0,
            DataQuality::High => 80.0,
            DataQuality::Standard => 60.0,
            DataQuality::Low => 40.0,
            DataQuality::Poor => 20.0,
        }
    }

    /// Infer quality from coverage ratio.
    pub fn from_coverage(coverage: f64) -> Self {
        if coverage >= 0.95 {
            DataQuality::Reference
        } else if coverage >= 0.80 {
            DataQuality::High
        } else if coverage >= 0.60 {
            DataQuality::Standard
        } else if coverage >= 0.40 {
            DataQuality::Low
        } else {
            DataQuality::Poor
        }
    }

    /// Check if quality is acceptable for calibration.
    pub fn is_acceptable(&self) -> bool {
        matches!(
            self,
            DataQuality::Reference | DataQuality::High | DataQuality::Standard
        )
    }
}

// ============================================================================
// IMPUTATION STRATEGY
// ============================================================================

/// Strategy for imputing missing values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImputationStrategy {
    /// Linear interpolation between known values.
    Linear,
    /// Cubic spline interpolation.
    Spline,
    /// Use nearest neighbor value.
    NearestNeighbor,
    /// Use mean of available values.
    Mean,
    /// Use median of available values.
    Median,
    /// Use physical model prediction.
    PhysicalModel,
    /// Zero fill (for sparse data).
    Zero,
    /// Skip missing values (reduce dataset).
    Skip,
}

impl Default for ImputationStrategy {
    fn default() -> Self {
        ImputationStrategy::Linear
    }
}

impl ImputationStrategy {
    /// Get description.
    pub fn description(&self) -> &'static str {
        match self {
            ImputationStrategy::Linear => "Linear interpolation between neighbors",
            ImputationStrategy::Spline => "Cubic spline interpolation",
            ImputationStrategy::NearestNeighbor => "Copy from nearest known value",
            ImputationStrategy::Mean => "Use mean of all known values",
            ImputationStrategy::Median => "Use median of all known values",
            ImputationStrategy::PhysicalModel => "Predict from physical model",
            ImputationStrategy::Zero => "Fill with zeros",
            ImputationStrategy::Skip => "Exclude missing observations",
        }
    }

    /// Check if strategy requires physical model.
    pub fn needs_model(&self) -> bool {
        matches!(self, ImputationStrategy::PhysicalModel)
    }
}

// ============================================================================
// MISSING DATA REPORT
// ============================================================================

/// Report about missing data in a source.
#[derive(Debug, Clone)]
pub struct MissingDataReport {
    /// Total expected observations.
    pub expected_count: usize,
    /// Actual observations present.
    pub actual_count: usize,
    /// Number of missing values.
    pub missing_count: usize,
    /// Coverage ratio (0-1).
    pub coverage: f64,
    /// Indices of missing values.
    pub missing_indices: Vec<usize>,
    /// Detected quality level.
    pub quality: DataQuality,
    /// Recommended imputation strategy.
    pub recommended_strategy: ImputationStrategy,
    /// Detected outlier indices.
    pub outlier_indices: Vec<usize>,
}

impl MissingDataReport {
    /// Create report from counts.
    pub fn new(expected: usize, actual: usize, missing_indices: Vec<usize>) -> Self {
        let missing = expected.saturating_sub(actual);
        let coverage = if expected > 0 {
            actual as f64 / expected as f64
        } else {
            1.0
        };
        let quality = DataQuality::from_coverage(coverage);
        let recommended_strategy = Self::recommend_strategy(coverage, &missing_indices);

        Self {
            expected_count: expected,
            actual_count: actual,
            missing_count: missing,
            coverage,
            missing_indices,
            quality,
            recommended_strategy,
            outlier_indices: Vec::new(),
        }
    }

    /// Recommend imputation strategy based on coverage pattern.
    fn recommend_strategy(coverage: f64, missing_indices: &[usize]) -> ImputationStrategy {
        if coverage >= 0.95 {
            ImputationStrategy::Linear
        } else if coverage >= 0.80 {
            // Check if gaps are sparse or clustered
            if Self::are_gaps_sparse(missing_indices) {
                ImputationStrategy::Linear
            } else {
                ImputationStrategy::Spline
            }
        } else if coverage >= 0.50 {
            ImputationStrategy::PhysicalModel
        } else {
            ImputationStrategy::Skip
        }
    }

    /// Check if missing indices are sparsely distributed.
    fn are_gaps_sparse(indices: &[usize]) -> bool {
        if indices.len() < 2 {
            return true;
        }

        // Check for consecutive indices (clusters)
        let mut consecutive = 0;
        for i in 1..indices.len() {
            if indices[i] == indices[i - 1] + 1 {
                consecutive += 1;
            }
        }

        // If more than 20% are consecutive, not sparse
        (consecutive as f64) / (indices.len() as f64) < 0.2
    }

    /// Check if data is usable.
    pub fn is_usable(&self) -> bool {
        self.quality.is_acceptable()
    }

    /// Get gap description.
    pub fn gap_description(&self) -> String {
        if self.missing_count == 0 {
            "No missing data".to_string()
        } else if self.missing_count == 1 {
            format!("1 missing value at index {}", self.missing_indices[0])
        } else {
            format!(
                "{} missing values ({:.1}% coverage)",
                self.missing_count,
                self.coverage * 100.0
            )
        }
    }

    /// Set outlier indices.
    pub fn with_outliers(mut self, outliers: Vec<usize>) -> Self {
        self.outlier_indices = outliers;
        self
    }
}

// ============================================================================
// PARTIAL DATA HANDLER
// ============================================================================

/// Handler for incomplete calibration data.
#[derive(Debug, Clone)]
pub struct PartialDataHandler {
    /// Imputation strategy.
    pub strategy: ImputationStrategy,
    /// Confidence weights for imputed values.
    pub imputed_weight: f64,
    /// Outlier detection threshold (σ).
    pub outlier_sigma: f64,
    /// Whether to automatically detect and handle outliers.
    pub auto_outlier_detection: bool,
}

impl Default for PartialDataHandler {
    fn default() -> Self {
        Self {
            strategy: ImputationStrategy::Linear,
            imputed_weight: 0.5,
            outlier_sigma: 3.0,
            auto_outlier_detection: true,
        }
    }
}

impl PartialDataHandler {
    /// Create new handler.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set imputation strategy.
    pub fn with_strategy(mut self, strategy: ImputationStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Set imputed value weight.
    pub fn with_imputed_weight(mut self, weight: f64) -> Self {
        self.imputed_weight = weight.clamp(0.0, 1.0);
        self
    }

    /// Set outlier detection threshold.
    pub fn with_outlier_sigma(mut self, sigma: f64) -> Self {
        self.outlier_sigma = sigma;
        self.auto_outlier_detection = true;
        self
    }

    /// Disable outlier detection.
    pub fn no_outlier_detection(mut self) -> Self {
        self.auto_outlier_detection = false;
        self
    }

    /// Analyze data and generate report.
    pub fn analyze(&self, data: &[Option<f64>], expected_count: usize) -> MissingDataReport {
        let actual_count = data.iter().filter(|x| x.is_some()).count();
        let missing_indices: Vec<usize> = data
            .iter()
            .enumerate()
            .filter(|(_, x)| x.is_none())
            .map(|(i, _)| i)
            .collect();

        let mut report = MissingDataReport::new(expected_count, actual_count, missing_indices);

        // Detect outliers if enabled
        if self.auto_outlier_detection {
            let values: Vec<f64> = data.iter().filter_map(|x| *x).collect();
            let outliers = detect_outliers_indices(&values, self.outlier_sigma);
            report = report.with_outliers(outliers);
        }

        report
    }

    /// Impute missing values.
    pub fn impute(&self, data: &mut Vec<Option<f64>>, domain: &[f64]) {
        match self.strategy {
            ImputationStrategy::Linear => impute_linear(data, domain),
            ImputationStrategy::NearestNeighbor => impute_nearest(data),
            ImputationStrategy::Mean => impute_mean(data),
            ImputationStrategy::Median => impute_median(data),
            ImputationStrategy::Zero => impute_zero(data),
            ImputationStrategy::Skip => {} // Don't modify
            ImputationStrategy::Spline => impute_spline(data, domain),
            ImputationStrategy::PhysicalModel => {} // Requires external model
        }
    }

    /// Get weights for imputed data.
    pub fn get_weights(&self, data: &[Option<f64>], original_mask: &[bool]) -> Vec<f64> {
        data.iter()
            .zip(original_mask.iter())
            .map(|(_, was_original)| {
                if *was_original {
                    1.0
                } else {
                    self.imputed_weight
                }
            })
            .collect()
    }
}

// ============================================================================
// IMPUTATION FUNCTIONS
// ============================================================================

/// Impute missing values using linear interpolation.
fn impute_linear(data: &mut Vec<Option<f64>>, _domain: &[f64]) {
    let n = data.len();
    if n == 0 {
        return;
    }

    // Find first and last known values
    let first_known = data.iter().position(|x| x.is_some());
    let last_known = data.iter().rposition(|x| x.is_some());

    if first_known.is_none() || last_known.is_none() {
        return;
    }

    let first_idx = first_known.unwrap();
    let last_idx = last_known.unwrap();

    // Fill before first known with first value
    if let Some(first_val) = data[first_idx] {
        for i in 0..first_idx {
            data[i] = Some(first_val);
        }
    }

    // Fill after last known with last value
    if let Some(last_val) = data[last_idx] {
        for i in (last_idx + 1)..n {
            data[i] = Some(last_val);
        }
    }

    // Linear interpolation in between
    let mut prev_idx = first_idx;
    for i in (first_idx + 1)..=last_idx {
        if data[i].is_some() {
            // Fill gap between prev_idx and i
            if i > prev_idx + 1 {
                let prev_val = data[prev_idx].unwrap();
                let curr_val = data[i].unwrap();
                for j in (prev_idx + 1)..i {
                    let t = (j - prev_idx) as f64 / (i - prev_idx) as f64;
                    data[j] = Some(prev_val + t * (curr_val - prev_val));
                }
            }
            prev_idx = i;
        }
    }
}

/// Impute missing values using nearest neighbor.
fn impute_nearest(data: &mut Vec<Option<f64>>) {
    let n = data.len();
    if n == 0 {
        return;
    }

    // Forward pass: fill with previous known value
    let mut last_known = None;
    for i in 0..n {
        if let Some(v) = data[i] {
            last_known = Some(v);
        } else if let Some(v) = last_known {
            data[i] = Some(v);
        }
    }

    // Backward pass: fill remaining with next known value
    let mut next_known = None;
    for i in (0..n).rev() {
        if let Some(v) = data[i] {
            next_known = Some(v);
        } else if let Some(v) = next_known {
            data[i] = Some(v);
        }
    }
}

/// Impute missing values with mean.
fn impute_mean(data: &mut Vec<Option<f64>>) {
    let known: Vec<f64> = data.iter().filter_map(|x| *x).collect();
    if known.is_empty() {
        return;
    }

    let mean = known.iter().sum::<f64>() / known.len() as f64;

    for item in data.iter_mut() {
        if item.is_none() {
            *item = Some(mean);
        }
    }
}

/// Impute missing values with median.
fn impute_median(data: &mut Vec<Option<f64>>) {
    let mut known: Vec<f64> = data.iter().filter_map(|x| *x).collect();
    if known.is_empty() {
        return;
    }

    known.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = if known.len() % 2 == 0 {
        (known[known.len() / 2 - 1] + known[known.len() / 2]) / 2.0
    } else {
        known[known.len() / 2]
    };

    for item in data.iter_mut() {
        if item.is_none() {
            *item = Some(median);
        }
    }
}

/// Impute missing values with zeros.
fn impute_zero(data: &mut Vec<Option<f64>>) {
    for item in data.iter_mut() {
        if item.is_none() {
            *item = Some(0.0);
        }
    }
}

/// Impute missing values using cubic spline (simplified).
fn impute_spline(data: &mut Vec<Option<f64>>, _domain: &[f64]) {
    // Simplified: use linear for now
    impute_linear(data, _domain);
}

// ============================================================================
// PUBLIC IMPUTATION FUNCTIONS
// ============================================================================

/// Impute missing spectral values.
pub fn impute_spectral(values: &mut Vec<f64>, wavelengths: &[f64], missing_mask: &[bool]) {
    let mut data: Vec<Option<f64>> = values
        .iter()
        .zip(missing_mask.iter())
        .map(|(v, m)| if *m { None } else { Some(*v) })
        .collect();

    impute_linear(&mut data, wavelengths);

    for (i, item) in data.into_iter().enumerate() {
        if let Some(v) = item {
            values[i] = v;
        }
    }
}

/// Impute missing angular values.
pub fn impute_angular(values: &mut Vec<f64>, angles: &[f64], missing_mask: &[bool]) {
    let mut data: Vec<Option<f64>> = values
        .iter()
        .zip(missing_mask.iter())
        .map(|(v, m)| if *m { None } else { Some(*v) })
        .collect();

    impute_linear(&mut data, angles);

    for (i, item) in data.into_iter().enumerate() {
        if let Some(v) = item {
            values[i] = v;
        }
    }
}

/// Detect outliers using z-score method.
pub fn detect_outliers(values: &[f64], sigma_threshold: f64) -> Vec<f64> {
    let indices = detect_outliers_indices(values, sigma_threshold);
    indices.into_iter().map(|i| values[i]).collect()
}

/// Detect outlier indices using z-score method.
fn detect_outliers_indices(values: &[f64], sigma_threshold: f64) -> Vec<usize> {
    if values.len() < 3 {
        return Vec::new();
    }

    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
    let std_dev = variance.sqrt();

    if std_dev < 1e-10 {
        return Vec::new();
    }

    values
        .iter()
        .enumerate()
        .filter(|(_, v)| ((*v - mean) / std_dev).abs() > sigma_threshold)
        .map(|(i, _)| i)
        .collect()
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_quality_from_coverage() {
        assert_eq!(DataQuality::from_coverage(1.0), DataQuality::Reference);
        assert_eq!(DataQuality::from_coverage(0.85), DataQuality::High);
        assert_eq!(DataQuality::from_coverage(0.65), DataQuality::Standard);
        assert_eq!(DataQuality::from_coverage(0.45), DataQuality::Low);
        assert_eq!(DataQuality::from_coverage(0.2), DataQuality::Poor);
    }

    #[test]
    fn test_data_quality_acceptable() {
        assert!(DataQuality::Reference.is_acceptable());
        assert!(DataQuality::High.is_acceptable());
        assert!(DataQuality::Standard.is_acceptable());
        assert!(!DataQuality::Low.is_acceptable());
        assert!(!DataQuality::Poor.is_acceptable());
    }

    #[test]
    fn test_missing_data_report() {
        let report = MissingDataReport::new(100, 95, vec![5, 15, 25, 35, 45]);
        assert_eq!(report.missing_count, 5);
        assert!((report.coverage - 0.95).abs() < 0.01);
        assert!(report.is_usable());
    }

    #[test]
    fn test_partial_data_handler_analyze() {
        let handler = PartialDataHandler::new();
        let data = vec![Some(0.1), Some(0.2), None, Some(0.4), None, Some(0.6)];
        let report = handler.analyze(&data, 6);

        assert_eq!(report.actual_count, 4);
        assert_eq!(report.missing_count, 2);
        assert_eq!(report.missing_indices, vec![2, 4]);
    }

    #[test]
    fn test_impute_linear() {
        let mut data = vec![Some(0.0), None, Some(1.0)];
        impute_linear(&mut data, &[0.0, 0.5, 1.0]);

        assert!((data[1].unwrap() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_impute_nearest() {
        let mut data = vec![Some(0.5), None, None, Some(0.8)];
        impute_nearest(&mut data);

        assert!(data[1].unwrap() == 0.5);
        // Note: depends on forward/backward pass order
    }

    #[test]
    fn test_impute_mean() {
        let mut data = vec![Some(0.2), None, Some(0.4), None, Some(0.6)];
        impute_mean(&mut data);

        let mean = 0.4;
        assert!((data[1].unwrap() - mean).abs() < 0.01);
        assert!((data[3].unwrap() - mean).abs() < 0.01);
    }

    #[test]
    fn test_impute_median() {
        let mut data = vec![Some(0.1), None, Some(0.3), None, Some(0.9)];
        impute_median(&mut data);

        let median = 0.3;
        assert!((data[1].unwrap() - median).abs() < 0.01);
    }

    #[test]
    fn test_detect_outliers() {
        let values = vec![1.0, 1.1, 1.0, 0.9, 1.1, 10.0, 1.0]; // 10.0 is outlier
        let outliers = detect_outliers(&values, 2.0);

        assert_eq!(outliers.len(), 1);
        assert!((outliers[0] - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_impute_spectral() {
        let mut values = vec![0.1, 0.0, 0.3, 0.0, 0.5];
        let wavelengths = vec![400.0, 450.0, 500.0, 550.0, 600.0];
        let missing_mask = vec![false, true, false, true, false];

        impute_spectral(&mut values, &wavelengths, &missing_mask);

        assert!((values[1] - 0.2).abs() < 0.01); // Interpolated
        assert!((values[3] - 0.4).abs() < 0.01); // Interpolated
    }

    #[test]
    fn test_handler_get_weights() {
        let handler = PartialDataHandler::new().with_imputed_weight(0.3);
        let data = vec![Some(0.1), Some(0.2), Some(0.3)];
        let original_mask = vec![true, false, true];

        let weights = handler.get_weights(&data, &original_mask);

        assert!((weights[0] - 1.0).abs() < 0.01);
        assert!((weights[1] - 0.3).abs() < 0.01);
        assert!((weights[2] - 1.0).abs() < 0.01);
    }
}

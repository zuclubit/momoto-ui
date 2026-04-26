//! # Reproducibility Testing
//!
//! Validates deterministic behavior and cross-platform reproducibility.
//! Ensures material twins produce identical results across runs.

// ============================================================================
// REPRODUCIBILITY TEST
// ============================================================================

/// Test configuration for reproducibility validation.
#[derive(Debug, Clone)]
pub struct ReproducibilityTest {
    /// Number of test runs.
    pub n_runs: usize,
    /// Random seed for determinism.
    pub seed: u64,
    /// Maximum allowed variation.
    pub max_variation: f64,
    /// Number of test points.
    pub n_points: usize,
    /// Test wavelengths.
    pub wavelengths: Vec<f64>,
    /// Test angles.
    pub angles: Vec<f64>,
}

impl Default for ReproducibilityTest {
    fn default() -> Self {
        Self {
            n_runs: 10,
            seed: 42,
            max_variation: 1e-10,
            n_points: 100,
            wavelengths: vec![400.0, 450.0, 500.0, 550.0, 600.0, 650.0, 700.0],
            angles: vec![0.0, 15.0, 30.0, 45.0, 60.0, 75.0],
        }
    }
}

impl ReproducibilityTest {
    /// Create new test configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create strict test (reference level).
    pub fn strict() -> Self {
        Self {
            n_runs: 100,
            seed: 12345,
            max_variation: 1e-15,
            n_points: 1000,
            wavelengths: (380..=780).step_by(10).map(|w| w as f64).collect(),
            angles: (0..=85).step_by(5).map(|a| a as f64).collect(),
        }
    }

    /// Create industrial test.
    pub fn industrial() -> Self {
        Self {
            n_runs: 50,
            seed: 42,
            max_variation: 1e-10,
            n_points: 500,
            ..Default::default()
        }
    }

    /// Set number of runs.
    pub fn with_runs(mut self, n: usize) -> Self {
        self.n_runs = n;
        self
    }

    /// Set seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Set max variation.
    pub fn with_max_variation(mut self, max: f64) -> Self {
        self.max_variation = max;
        self
    }

    /// Verify reproducibility of a function.
    pub fn verify<F>(&self, mut func: F) -> ReproducibilityResult
    where
        F: FnMut(f64, f64) -> f64, // (wavelength, angle) -> value
    {
        let mut all_results: Vec<Vec<f64>> = Vec::new();
        let mut failing_evaluations = Vec::new();

        // Generate test points
        let test_points: Vec<(f64, f64)> = self
            .wavelengths
            .iter()
            .flat_map(|&wl| self.angles.iter().map(move |&angle| (wl, angle)))
            .take(self.n_points)
            .collect();

        // Run multiple times
        for _run in 0..self.n_runs {
            let mut run_results = Vec::new();

            for &(wl, angle) in &test_points {
                let value = func(wl, angle);
                run_results.push(value);
            }

            all_results.push(run_results);
        }

        // Compare all runs to first run
        let reference = &all_results[0];
        let mut max_variation = 0.0;

        for (run_idx, run_results) in all_results.iter().enumerate().skip(1) {
            for (point_idx, (&ref_val, &run_val)) in
                reference.iter().zip(run_results.iter()).enumerate()
            {
                let variation = (ref_val - run_val).abs();
                if variation > max_variation {
                    max_variation = variation;
                }

                if variation > self.max_variation {
                    failing_evaluations.push((
                        run_idx,
                        point_idx,
                        variation,
                        test_points[point_idx],
                    ));
                }
            }
        }

        let deterministic = failing_evaluations.is_empty();

        ReproducibilityResult {
            deterministic,
            max_variation,
            n_runs: self.n_runs,
            n_points: test_points.len(),
            failing_evaluations: failing_evaluations
                .iter()
                .map(|&(run, _point, var, _)| (run, var))
                .take(10)
                .collect(),
            failing_points: failing_evaluations
                .iter()
                .map(|&(_, _, _, point)| point)
                .take(10)
                .collect(),
        }
    }

    /// Verify bit-exact reproducibility.
    pub fn verify_bit_exact<F>(&self, mut func: F) -> ReproducibilityResult
    where
        F: FnMut(f64, f64) -> f64,
    {
        self.clone().with_max_variation(0.0).verify(&mut func)
    }

    /// Compare two implementations.
    pub fn compare<F1, F2>(&self, mut func1: F1, mut func2: F2) -> ComparisonResult
    where
        F1: FnMut(f64, f64) -> f64,
        F2: FnMut(f64, f64) -> f64,
    {
        let mut differences = Vec::new();
        let mut max_diff = 0.0;
        let mut sum_diff = 0.0;

        let test_points: Vec<(f64, f64)> = self
            .wavelengths
            .iter()
            .flat_map(|&wl| self.angles.iter().map(move |&angle| (wl, angle)))
            .take(self.n_points)
            .collect();

        for &(wl, angle) in &test_points {
            let v1 = func1(wl, angle);
            let v2 = func2(wl, angle);
            let diff = (v1 - v2).abs();

            if diff > max_diff {
                max_diff = diff;
            }
            sum_diff += diff;

            if diff > self.max_variation {
                differences.push(((wl, angle), v1, v2, diff));
            }
        }

        let mean_diff = sum_diff / test_points.len() as f64;
        let equivalent = max_diff <= self.max_variation;

        ComparisonResult {
            equivalent,
            max_difference: max_diff,
            mean_difference: mean_diff,
            n_points: test_points.len(),
            differing_points: differences.len(),
            sample_differences: differences.into_iter().take(5).collect(),
        }
    }
}

// ============================================================================
// REPRODUCIBILITY RESULT
// ============================================================================

/// Result of reproducibility test.
#[derive(Debug, Clone)]
pub struct ReproducibilityResult {
    /// Whether results were deterministic.
    pub deterministic: bool,
    /// Maximum variation observed.
    pub max_variation: f64,
    /// Number of runs performed.
    pub n_runs: usize,
    /// Number of points tested.
    pub n_points: usize,
    /// Failing evaluations (run, variation).
    pub failing_evaluations: Vec<(usize, f64)>,
    /// Failing points (wavelength, angle).
    pub failing_points: Vec<(f64, f64)>,
}

impl ReproducibilityResult {
    /// Get reproducibility score (1.0 = perfect).
    pub fn score(&self) -> f64 {
        if self.n_runs == 0 || self.n_points == 0 {
            return 0.0;
        }

        let total_comparisons = (self.n_runs - 1) * self.n_points;
        let failing = self.failing_evaluations.len();

        if total_comparisons == 0 {
            1.0
        } else {
            1.0 - (failing as f64 / total_comparisons as f64)
        }
    }

    /// Check if achieves target reproducibility.
    pub fn achieves(&self, min_score: f64) -> bool {
        self.score() >= min_score
    }

    /// Generate report.
    pub fn report(&self) -> String {
        let mut report = String::new();

        report.push_str("Reproducibility Test Report\n");
        report.push_str(&format!(
            "Status: {}\n",
            if self.deterministic {
                "DETERMINISTIC"
            } else {
                "NON-DETERMINISTIC"
            }
        ));
        report.push_str(&format!("Runs: {}\n", self.n_runs));
        report.push_str(&format!("Points: {}\n", self.n_points));
        report.push_str(&format!("Max Variation: {:.2e}\n", self.max_variation));
        report.push_str(&format!("Score: {:.4}\n", self.score()));

        if !self.deterministic {
            report.push_str(&format!(
                "Failing evaluations: {}\n",
                self.failing_evaluations.len()
            ));

            if !self.failing_points.is_empty() {
                report.push_str("Sample failing points:\n");
                for (wl, angle) in &self.failing_points {
                    report.push_str(&format!("  λ={:.0}nm, θ={:.0}°\n", wl, angle));
                }
            }
        }

        report
    }
}

// ============================================================================
// COMPARISON RESULT
// ============================================================================

/// Result of comparing two implementations.
#[derive(Debug, Clone)]
pub struct ComparisonResult {
    /// Whether implementations are equivalent (within tolerance).
    pub equivalent: bool,
    /// Maximum difference found.
    pub max_difference: f64,
    /// Mean difference.
    pub mean_difference: f64,
    /// Number of points compared.
    pub n_points: usize,
    /// Number of differing points.
    pub differing_points: usize,
    /// Sample of differences: ((wl, angle), v1, v2, diff).
    pub sample_differences: Vec<((f64, f64), f64, f64, f64)>,
}

impl ComparisonResult {
    /// Get agreement percentage.
    pub fn agreement(&self) -> f64 {
        if self.n_points == 0 {
            return 100.0;
        }
        ((self.n_points - self.differing_points) as f64 / self.n_points as f64) * 100.0
    }

    /// Generate report.
    pub fn report(&self) -> String {
        let mut report = String::new();

        report.push_str("Implementation Comparison Report\n");
        report.push_str(&format!(
            "Status: {}\n",
            if self.equivalent {
                "EQUIVALENT"
            } else {
                "DIFFERENT"
            }
        ));
        report.push_str(&format!("Points compared: {}\n", self.n_points));
        report.push_str(&format!("Agreement: {:.2}%\n", self.agreement()));
        report.push_str(&format!("Max difference: {:.6e}\n", self.max_difference));
        report.push_str(&format!("Mean difference: {:.6e}\n", self.mean_difference));

        if !self.sample_differences.is_empty() {
            report.push_str("\nSample differences:\n");
            for ((wl, angle), v1, v2, diff) in &self.sample_differences {
                report.push_str(&format!(
                    "  λ={:.0}nm θ={:.0}°: {:.6} vs {:.6} (Δ={:.2e})\n",
                    wl, angle, v1, v2, diff
                ));
            }
        }

        report
    }
}

// ============================================================================
// HASH-BASED VERIFICATION
// ============================================================================

/// Generate reproducibility hash from function outputs.
pub fn compute_reproducibility_hash<F>(func: &mut F, seed: u64) -> u64
where
    F: FnMut(f64, f64) -> f64,
{
    let wavelengths = [400.0, 500.0, 600.0, 700.0];
    let angles = [0.0, 30.0, 60.0];

    let mut hash: u64 = seed;

    for &wl in &wavelengths {
        for &angle in &angles {
            let value = func(wl, angle);
            let bits = value.to_bits();
            hash = hash.wrapping_mul(31).wrapping_add(bits);
        }
    }

    hash
}

/// Verify hash matches expected value.
pub fn verify_hash<F>(func: &mut F, expected: u64, seed: u64) -> bool
where
    F: FnMut(f64, f64) -> f64,
{
    compute_reproducibility_hash(func, seed) == expected
}

// ============================================================================
// CROSS-PLATFORM CHECKS
// ============================================================================

/// Known values for cross-platform verification.
#[derive(Debug, Clone)]
pub struct CrossPlatformReference {
    /// Platform identifier.
    pub platform: String,
    /// Reference values at standard points.
    pub reference_values: Vec<(f64, f64, f64)>, // (wavelength, angle, value)
    /// Tolerance for comparison.
    pub tolerance: f64,
}

impl CrossPlatformReference {
    /// Create reference for current platform.
    pub fn from_current<F>(platform: impl Into<String>, mut func: F) -> Self
    where
        F: FnMut(f64, f64) -> f64,
    {
        let standard_points = vec![
            (400.0, 0.0),
            (550.0, 0.0),
            (700.0, 0.0),
            (550.0, 45.0),
            (550.0, 60.0),
        ];

        let reference_values: Vec<(f64, f64, f64)> = standard_points
            .iter()
            .map(|&(wl, angle)| (wl, angle, func(wl, angle)))
            .collect();

        Self {
            platform: platform.into(),
            reference_values,
            tolerance: 1e-10,
        }
    }

    /// Verify against reference.
    pub fn verify<F>(&self, mut func: F) -> bool
    where
        F: FnMut(f64, f64) -> f64,
    {
        for &(wl, angle, expected) in &self.reference_values {
            let actual = func(wl, angle);
            if (actual - expected).abs() > self.tolerance {
                return false;
            }
        }
        true
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reproducibility_deterministic() {
        let test = ReproducibilityTest::new().with_runs(5);

        // Deterministic function
        let result = test.verify(|wl, angle| wl * 0.001 + angle * 0.01);

        assert!(result.deterministic);
        assert!(result.max_variation < 1e-10);
        assert!((result.score() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_reproducibility_non_deterministic() {
        let test = ReproducibilityTest::new()
            .with_runs(5)
            .with_max_variation(0.0);

        use std::cell::Cell;
        let counter = Cell::new(0u64);

        // Non-deterministic function (changes each call)
        let result = test.verify(|wl, angle| {
            let c = counter.get();
            counter.set(c + 1);
            wl * 0.001 + angle * 0.01 + (c as f64) * 1e-8
        });

        // May or may not be deterministic depending on counter behavior
        // Since counter increments, successive runs will differ
        assert!(!result.deterministic || result.max_variation > 0.0);
    }

    #[test]
    fn test_comparison_equivalent() {
        let test = ReproducibilityTest::new();

        let result = test.compare(
            |wl, angle| wl * 0.001 + angle * 0.01,
            |wl, angle| wl * 0.001 + angle * 0.01,
        );

        assert!(result.equivalent);
        assert!(result.max_difference < 1e-10);
        assert!((result.agreement() - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_comparison_different() {
        let test = ReproducibilityTest::new().with_max_variation(1e-6);

        let result = test.compare(
            |wl, angle| wl * 0.001 + angle * 0.01,
            |wl, angle| wl * 0.001 + angle * 0.01 + 0.001, // Small difference
        );

        assert!(!result.equivalent);
        assert!((result.max_difference - 0.001).abs() < 1e-10);
    }

    #[test]
    fn test_strict_test() {
        let test = ReproducibilityTest::strict();
        assert!(test.max_variation < 1e-10);
        assert!(test.n_runs >= 100);
    }

    #[test]
    fn test_reproducibility_score() {
        let result = ReproducibilityResult {
            deterministic: true,
            max_variation: 0.0,
            n_runs: 10,
            n_points: 100,
            failing_evaluations: vec![],
            failing_points: vec![],
        };

        assert!((result.score() - 1.0).abs() < 1e-10);
        assert!(result.achieves(0.99));
    }

    #[test]
    fn test_hash_computation() {
        let mut func = |wl: f64, angle: f64| wl * 0.001 + angle * 0.01;

        let hash1 = compute_reproducibility_hash(&mut func, 42);
        let hash2 = compute_reproducibility_hash(&mut func, 42);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_verification() {
        let mut func = |wl: f64, angle: f64| wl * 0.001 + angle * 0.01;

        let expected = compute_reproducibility_hash(&mut func, 42);
        assert!(verify_hash(&mut func, expected, 42));
        assert!(!verify_hash(&mut func, expected + 1, 42));
    }

    #[test]
    fn test_cross_platform_reference() {
        let func = |wl: f64, angle: f64| wl * 0.001 + angle * 0.01;

        let reference = CrossPlatformReference::from_current("test", func);

        assert!(!reference.reference_values.is_empty());
        assert!(reference.verify(func));
    }

    #[test]
    fn test_result_report() {
        let result = ReproducibilityResult {
            deterministic: true,
            max_variation: 1e-12,
            n_runs: 10,
            n_points: 100,
            failing_evaluations: vec![],
            failing_points: vec![],
        };

        let report = result.report();
        assert!(report.contains("DETERMINISTIC"));
        assert!(report.contains("10"));
    }

    #[test]
    fn test_comparison_report() {
        let test = ReproducibilityTest::new();
        let result = test.compare(|wl, _| wl, |wl, _| wl + 0.01);

        let report = result.report();
        assert!(report.contains("Comparison"));
    }
}

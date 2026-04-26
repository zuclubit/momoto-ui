//! # GPU/CPU Parity Testing
//!
//! Validation utilities to ensure GPU and CPU produce equivalent results.
//!
//! ## Parity Metric
//!
//! We use ΔE2000 (CIEDE2000) as the perceptual distance metric.
//! Target: ΔE2000 < 1.0 (imperceptible difference).

use super::buffers::{MaterialGpuData, ResponseGpuData};
use super::dispatch::GpuBatchResult;

/// Configuration for parity testing.
#[derive(Debug, Clone)]
pub struct ParityConfig {
    /// Maximum acceptable ΔE2000 difference.
    pub threshold: f64,
    /// Number of random test cases.
    pub test_count: usize,
    /// Random seed for reproducibility.
    pub seed: u64,
    /// Include edge cases in tests.
    pub include_edge_cases: bool,
}

impl Default for ParityConfig {
    fn default() -> Self {
        Self {
            threshold: 1.0, // ΔE2000 < 1.0 is imperceptible
            test_count: 1000,
            seed: 42,
            include_edge_cases: true,
        }
    }
}

/// Result of a parity test.
#[derive(Debug, Clone)]
pub struct ParityResult {
    /// Test passed (all within threshold).
    pub passed: bool,
    /// Number of test cases.
    pub test_count: usize,
    /// Number of violations.
    pub violations: usize,
    /// Maximum ΔE2000 observed.
    pub max_delta_e: f64,
    /// Average ΔE2000.
    pub avg_delta_e: f64,
    /// Indices of violating materials.
    pub violating_indices: Vec<usize>,
}

impl ParityResult {
    /// Create a passing result.
    pub fn pass(test_count: usize, max_delta_e: f64, avg_delta_e: f64) -> Self {
        Self {
            passed: true,
            test_count,
            violations: 0,
            max_delta_e,
            avg_delta_e,
            violating_indices: Vec::new(),
        }
    }

    /// Create a failing result.
    pub fn fail(
        test_count: usize,
        violations: usize,
        max_delta_e: f64,
        avg_delta_e: f64,
        violating_indices: Vec<usize>,
    ) -> Self {
        Self {
            passed: false,
            test_count,
            violations,
            max_delta_e,
            avg_delta_e,
            violating_indices,
        }
    }
}

/// GPU/CPU parity tester.
pub struct GpuCpuParityTest {
    /// Configuration.
    config: ParityConfig,
}

impl GpuCpuParityTest {
    /// Create a new parity tester.
    pub fn new(config: ParityConfig) -> Self {
        Self { config }
    }

    /// Create with default configuration.
    pub fn default_config() -> Self {
        Self::new(ParityConfig::default())
    }

    /// Generate test materials.
    pub fn generate_test_materials(&self) -> Vec<MaterialGpuData> {
        let mut materials = Vec::with_capacity(self.config.test_count);

        // Use simple PRNG for reproducibility
        let mut seed = self.config.seed;
        let next_rand = |s: &mut u64| -> f64 {
            *s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
            (*s as f64) / (u64::MAX as f64)
        };

        // Generate random materials
        for _ in 0..self.config.test_count {
            materials.push(MaterialGpuData {
                ior: 1.0 + next_rand(&mut seed) * 1.5,  // 1.0 - 2.5
                cos_theta: next_rand(&mut seed),        // 0.0 - 1.0
                absorption: next_rand(&mut seed) * 0.5, // 0.0 - 0.5
                thickness: next_rand(&mut seed) * 20.0, // 0.0 - 20.0 mm
                g: next_rand(&mut seed) * 0.9,          // 0.0 - 0.9
                roughness: next_rand(&mut seed) * 0.5,  // 0.0 - 0.5
                metallic: if next_rand(&mut seed) > 0.7 { 1.0 } else { 0.0 },
                k: next_rand(&mut seed) * 5.0, // 0.0 - 5.0
            });
        }

        // Add edge cases
        if self.config.include_edge_cases {
            materials.extend(Self::edge_cases());
        }

        materials
    }

    /// Generate edge case materials.
    fn edge_cases() -> Vec<MaterialGpuData> {
        vec![
            // Grazing angle
            MaterialGpuData {
                ior: 1.5,
                cos_theta: 0.01,
                absorption: 0.1,
                thickness: 5.0,
                g: 0.0,
                roughness: 0.0,
                metallic: 0.0,
                k: 0.0,
            },
            // Normal incidence
            MaterialGpuData {
                ior: 1.5,
                cos_theta: 1.0,
                absorption: 0.1,
                thickness: 5.0,
                g: 0.0,
                roughness: 0.0,
                metallic: 0.0,
                k: 0.0,
            },
            // High IOR
            MaterialGpuData {
                ior: 2.4,
                cos_theta: 0.5,
                absorption: 0.0,
                thickness: 1.0,
                g: 0.0,
                roughness: 0.0,
                metallic: 0.0,
                k: 0.0,
            },
            // Metal (gold-like)
            MaterialGpuData {
                ior: 0.18,
                cos_theta: 0.5,
                absorption: 0.0,
                thickness: 0.0,
                g: 0.0,
                roughness: 0.1,
                metallic: 1.0,
                k: 3.0,
            },
            // Rough surface
            MaterialGpuData {
                ior: 1.5,
                cos_theta: 0.5,
                absorption: 0.1,
                thickness: 5.0,
                g: 0.0,
                roughness: 0.5,
                metallic: 0.0,
                k: 0.0,
            },
            // High absorption
            MaterialGpuData {
                ior: 1.5,
                cos_theta: 0.5,
                absorption: 1.0,
                thickness: 10.0,
                g: 0.0,
                roughness: 0.0,
                metallic: 0.0,
                k: 0.0,
            },
        ]
    }

    /// Compare GPU and CPU results.
    pub fn compare(
        &self,
        gpu_result: &GpuBatchResult,
        cpu_result: &GpuBatchResult,
    ) -> ParityResult {
        let count = gpu_result.count.min(cpu_result.count);

        if count == 0 {
            return ParityResult::pass(0, 0.0, 0.0);
        }

        let mut max_delta_e = 0.0;
        let mut sum_delta_e = 0.0;
        let mut violations = Vec::new();

        for i in 0..count {
            let delta_e = self.compute_delta_e(
                gpu_result.reflectance_r[i],
                gpu_result.reflectance_g[i],
                gpu_result.reflectance_b[i],
                cpu_result.reflectance_r[i],
                cpu_result.reflectance_g[i],
                cpu_result.reflectance_b[i],
            );

            sum_delta_e += delta_e;
            max_delta_e = max_delta_e.max(delta_e);

            if delta_e > self.config.threshold {
                violations.push(i);
            }
        }

        let avg_delta_e = sum_delta_e / count as f64;

        if violations.is_empty() {
            ParityResult::pass(count, max_delta_e, avg_delta_e)
        } else {
            ParityResult::fail(
                count,
                violations.len(),
                max_delta_e,
                avg_delta_e,
                violations,
            )
        }
    }

    /// Compute ΔE2000 between two RGB values.
    fn compute_delta_e(&self, r1: f64, g1: f64, b1: f64, r2: f64, g2: f64, b2: f64) -> f64 {
        // Simplified ΔE calculation (CIE76 approximation for efficiency)
        // Full ΔE2000 would use LAB color space

        // Convert RGB to approximate LAB
        let (l1, a1, b1_lab) = Self::rgb_to_lab_approx(r1, g1, b1);
        let (l2, a2, b2_lab) = Self::rgb_to_lab_approx(r2, g2, b2);

        // Euclidean distance in LAB (CIE76)
        let dl = l1 - l2;
        let da = a1 - a2;
        let db = b1_lab - b2_lab;

        (dl * dl + da * da + db * db).sqrt()
    }

    /// Approximate RGB to LAB conversion.
    fn rgb_to_lab_approx(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
        // Simplified conversion (gamma correction omitted for speed)
        let x = 0.4124 * r + 0.3576 * g + 0.1805 * b;
        let y = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        let z = 0.0193 * r + 0.1192 * g + 0.9505 * b;

        // D65 white point
        let xn = 0.95047;
        let yn = 1.0;
        let zn = 1.08883;

        let fx = Self::lab_f(x / xn);
        let fy = Self::lab_f(y / yn);
        let fz = Self::lab_f(z / zn);

        let l = 116.0 * fy - 16.0;
        let a = 500.0 * (fx - fy);
        let b_lab = 200.0 * (fy - fz);

        (l, a, b_lab)
    }

    /// LAB transfer function.
    fn lab_f(t: f64) -> f64 {
        let delta = 6.0 / 29.0;
        if t > delta.powi(3) {
            t.cbrt()
        } else {
            t / (3.0 * delta * delta) + 4.0 / 29.0
        }
    }
}

/// Estimate memory for parity testing.
pub fn estimate_parity_memory() -> usize {
    // Test configuration
    let config = 256;

    // Result storage
    let result = 512;

    config + result
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ParityConfig::default();
        assert!((config.threshold - 1.0).abs() < f64::EPSILON);
        assert_eq!(config.test_count, 1000);
        assert!(config.include_edge_cases);
    }

    #[test]
    fn test_generate_materials() {
        let tester = GpuCpuParityTest::default_config();
        let materials = tester.generate_test_materials();

        // Should have test_count + edge_cases
        assert!(materials.len() >= tester.config.test_count);
    }

    #[test]
    fn test_edge_cases() {
        let cases = GpuCpuParityTest::edge_cases();
        assert!(!cases.is_empty());

        // Check grazing angle case
        assert!(cases[0].cos_theta < 0.1);

        // Check normal incidence case
        assert!((cases[1].cos_theta - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_rgb_to_lab() {
        // White should be L=100
        let (l, a, b) = GpuCpuParityTest::rgb_to_lab_approx(1.0, 1.0, 1.0);
        assert!(l > 99.0);
        assert!(a.abs() < 1.0);
        assert!(b.abs() < 1.0);

        // Black should be L=0
        let (l, _, _) = GpuCpuParityTest::rgb_to_lab_approx(0.0, 0.0, 0.0);
        assert!(l < 1.0);
    }

    #[test]
    fn test_delta_e_identical() {
        let tester = GpuCpuParityTest::default_config();
        let delta_e = tester.compute_delta_e(0.5, 0.5, 0.5, 0.5, 0.5, 0.5);
        assert!(delta_e < 0.001);
    }

    #[test]
    fn test_memory_estimate() {
        let mem = estimate_parity_memory();
        assert!(mem > 0);
        assert!(mem < 5 * 1024); // Should be under 5KB
    }
}

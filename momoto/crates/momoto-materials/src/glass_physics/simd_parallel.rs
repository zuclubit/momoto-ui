//! # SIMD Parallel Evaluation Module (Phase 7)
//!
//! Advanced CPU parallelization using chunked parallel iteration with SIMD inner loops.
//! Builds on Phase 6's `simd_batch.rs` to add multi-threaded batch evaluation.
//!
//! ## Features
//!
//! - Parallel batch evaluation with configurable chunk sizes
//! - Adaptive threshold for automatic parallel/sequential selection
//! - Combined effects parallel evaluation
//! - Parallel perceptual loss computation
//!
//! ## Performance
//!
//! Target: 2x throughput improvement over Phase 6 SIMD-only on multi-core systems.

use std::time::Instant;

use super::combined_effects::{presets as combined_presets, CombinedMaterial};
use super::perceptual_loss::{delta_e_2000, rgb_to_lab, Illuminant};
use super::simd_batch::{
    beer_lambert_scalar, fresnel_schlick_scalar, henyey_greenstein_scalar, SimdBatchInput,
    SimdBatchResult, SimdConfig,
};

// ============================================================================
// CONFIGURATION
// ============================================================================

/// Parallel evaluation configuration
#[derive(Debug, Clone)]
pub struct ParallelConfig {
    /// Number of threads (0 = auto-detect based on CPU cores)
    pub thread_count: usize,
    /// Work chunk size per thread (default: 256)
    pub chunk_size: usize,
    /// SIMD width for inner loop unrolling (4 or 8)
    pub simd_width: usize,
    /// Minimum batch size to use parallel evaluation (default: 1000)
    pub adaptive_threshold: usize,
    /// Enable parallel evaluation
    pub parallel_enabled: bool,
}

impl ParallelConfig {
    /// Create default configuration
    pub fn new() -> Self {
        Self {
            thread_count: 0, // Auto-detect
            chunk_size: 256,
            simd_width: 4,
            adaptive_threshold: 1000,
            parallel_enabled: true,
        }
    }

    /// Configure for maximum parallelism
    pub fn max_parallel() -> Self {
        Self {
            thread_count: 0, // Use all cores
            chunk_size: 512,
            simd_width: 8,
            adaptive_threshold: 500,
            parallel_enabled: true,
        }
    }

    /// Configure for sequential execution (for comparison)
    pub fn sequential() -> Self {
        Self {
            thread_count: 1,
            chunk_size: 1024,
            simd_width: 4,
            adaptive_threshold: usize::MAX,
            parallel_enabled: false,
        }
    }

    /// Set specific thread count
    pub fn with_threads(mut self, count: usize) -> Self {
        self.thread_count = count;
        self
    }

    /// Set chunk size
    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size.max(16);
        self
    }

    /// Get effective thread count
    pub fn effective_threads(&self) -> usize {
        if self.thread_count == 0 {
            num_cpus()
        } else {
            self.thread_count
        }
    }
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Get number of CPU cores (simplified cross-platform)
fn num_cpus() -> usize {
    // In production, use num_cpus crate or std::thread::available_parallelism
    // For now, default to 4 cores
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

// ============================================================================
// PARALLEL BATCH EVALUATOR
// ============================================================================

/// Parallel batch evaluator using chunked iteration
#[derive(Debug, Clone)]
pub struct ParallelBatchEvaluator {
    config: ParallelConfig,
    simd_config: SimdConfig,
}

impl ParallelBatchEvaluator {
    /// Create new parallel evaluator
    pub fn new(config: ParallelConfig) -> Self {
        let simd_config = SimdConfig {
            chunk_size: config.simd_width,
            parallel: false, // Inner SIMD doesn't need parallel
            thread_count: 1,
            use_luts: true,
        };

        Self {
            config,
            simd_config,
        }
    }

    /// Evaluate batch in parallel
    pub fn evaluate(&self, input: &SimdBatchInput) -> SimdBatchResult {
        let n = input.len();

        // Use sequential for small batches
        if !self.config.parallel_enabled || n < self.config.adaptive_threshold {
            return self.evaluate_sequential(input);
        }

        self.evaluate_parallel(input)
    }

    /// Sequential evaluation (baseline)
    fn evaluate_sequential(&self, input: &SimdBatchInput) -> SimdBatchResult {
        let n = input.len();
        let mut result = SimdBatchResult {
            fresnel: vec![0.0; n],
            transmittance: vec![0.0; n],
            phase: vec![0.0; n],
            combined: vec![0.0; n],
        };

        // 4x unrolled loop
        let chunks = n / 4;
        for chunk in 0..chunks {
            let base = chunk * 4;

            // Fresnel
            result.fresnel[base] = fresnel_schlick_scalar(input.cos_theta[base], input.ior[base]);
            result.fresnel[base + 1] =
                fresnel_schlick_scalar(input.cos_theta[base + 1], input.ior[base + 1]);
            result.fresnel[base + 2] =
                fresnel_schlick_scalar(input.cos_theta[base + 2], input.ior[base + 2]);
            result.fresnel[base + 3] =
                fresnel_schlick_scalar(input.cos_theta[base + 3], input.ior[base + 3]);

            // Beer-Lambert
            result.transmittance[base] =
                beer_lambert_scalar(input.absorption[base], input.thickness[base]);
            result.transmittance[base + 1] =
                beer_lambert_scalar(input.absorption[base + 1], input.thickness[base + 1]);
            result.transmittance[base + 2] =
                beer_lambert_scalar(input.absorption[base + 2], input.thickness[base + 2]);
            result.transmittance[base + 3] =
                beer_lambert_scalar(input.absorption[base + 3], input.thickness[base + 3]);

            // Henyey-Greenstein
            result.phase[base] = henyey_greenstein_scalar(input.cos_theta[base], input.g[base]);
            result.phase[base + 1] =
                henyey_greenstein_scalar(input.cos_theta[base + 1], input.g[base + 1]);
            result.phase[base + 2] =
                henyey_greenstein_scalar(input.cos_theta[base + 2], input.g[base + 2]);
            result.phase[base + 3] =
                henyey_greenstein_scalar(input.cos_theta[base + 3], input.g[base + 3]);

            // Combined
            result.combined[base] =
                result.fresnel[base] * result.transmittance[base] * result.phase[base];
            result.combined[base + 1] =
                result.fresnel[base + 1] * result.transmittance[base + 1] * result.phase[base + 1];
            result.combined[base + 2] =
                result.fresnel[base + 2] * result.transmittance[base + 2] * result.phase[base + 2];
            result.combined[base + 3] =
                result.fresnel[base + 3] * result.transmittance[base + 3] * result.phase[base + 3];
        }

        // Handle remainder
        for i in (chunks * 4)..n {
            result.fresnel[i] = fresnel_schlick_scalar(input.cos_theta[i], input.ior[i]);
            result.transmittance[i] = beer_lambert_scalar(input.absorption[i], input.thickness[i]);
            result.phase[i] = henyey_greenstein_scalar(input.cos_theta[i], input.g[i]);
            result.combined[i] = result.fresnel[i] * result.transmittance[i] * result.phase[i];
        }

        result
    }

    /// Parallel evaluation using thread pool
    fn evaluate_parallel(&self, input: &SimdBatchInput) -> SimdBatchResult {
        let n = input.len();
        let chunk_size = self.config.chunk_size;
        let num_chunks = (n + chunk_size - 1) / chunk_size;

        // Pre-allocate result vectors
        let mut fresnel = vec![0.0; n];
        let mut transmittance = vec![0.0; n];
        let mut phase = vec![0.0; n];
        let mut combined = vec![0.0; n];

        // Process chunks in parallel using scoped threads
        std::thread::scope(|s| {
            let handles: Vec<_> = (0..num_chunks)
                .map(|chunk_idx| {
                    let start = chunk_idx * chunk_size;
                    let end = (start + chunk_size).min(n);
                    let chunk_len = end - start;

                    // Get slices for this chunk
                    let cos_theta = &input.cos_theta[start..end];
                    let ior = &input.ior[start..end];
                    let absorption = &input.absorption[start..end];
                    let thickness = &input.thickness[start..end];
                    let g = &input.g[start..end];

                    s.spawn(move || {
                        let mut f_out = vec![0.0; chunk_len];
                        let mut t_out = vec![0.0; chunk_len];
                        let mut p_out = vec![0.0; chunk_len];
                        let mut c_out = vec![0.0; chunk_len];

                        // Process with 4x unrolling
                        let inner_chunks = chunk_len / 4;
                        for i in 0..inner_chunks {
                            let base = i * 4;

                            f_out[base] = fresnel_schlick_scalar(cos_theta[base], ior[base]);
                            f_out[base + 1] =
                                fresnel_schlick_scalar(cos_theta[base + 1], ior[base + 1]);
                            f_out[base + 2] =
                                fresnel_schlick_scalar(cos_theta[base + 2], ior[base + 2]);
                            f_out[base + 3] =
                                fresnel_schlick_scalar(cos_theta[base + 3], ior[base + 3]);

                            t_out[base] = beer_lambert_scalar(absorption[base], thickness[base]);
                            t_out[base + 1] =
                                beer_lambert_scalar(absorption[base + 1], thickness[base + 1]);
                            t_out[base + 2] =
                                beer_lambert_scalar(absorption[base + 2], thickness[base + 2]);
                            t_out[base + 3] =
                                beer_lambert_scalar(absorption[base + 3], thickness[base + 3]);

                            p_out[base] = henyey_greenstein_scalar(cos_theta[base], g[base]);
                            p_out[base + 1] =
                                henyey_greenstein_scalar(cos_theta[base + 1], g[base + 1]);
                            p_out[base + 2] =
                                henyey_greenstein_scalar(cos_theta[base + 2], g[base + 2]);
                            p_out[base + 3] =
                                henyey_greenstein_scalar(cos_theta[base + 3], g[base + 3]);

                            c_out[base] = f_out[base] * t_out[base] * p_out[base];
                            c_out[base + 1] = f_out[base + 1] * t_out[base + 1] * p_out[base + 1];
                            c_out[base + 2] = f_out[base + 2] * t_out[base + 2] * p_out[base + 2];
                            c_out[base + 3] = f_out[base + 3] * t_out[base + 3] * p_out[base + 3];
                        }

                        // Remainder
                        for i in (inner_chunks * 4)..chunk_len {
                            f_out[i] = fresnel_schlick_scalar(cos_theta[i], ior[i]);
                            t_out[i] = beer_lambert_scalar(absorption[i], thickness[i]);
                            p_out[i] = henyey_greenstein_scalar(cos_theta[i], g[i]);
                            c_out[i] = f_out[i] * t_out[i] * p_out[i];
                        }

                        (chunk_idx, f_out, t_out, p_out, c_out)
                    })
                })
                .collect();

            // Collect results
            for handle in handles {
                let (chunk_idx, f_out, t_out, p_out, c_out) = handle.join().unwrap();
                let start = chunk_idx * chunk_size;

                for (i, val) in f_out.iter().enumerate() {
                    fresnel[start + i] = *val;
                }
                for (i, val) in t_out.iter().enumerate() {
                    transmittance[start + i] = *val;
                }
                for (i, val) in p_out.iter().enumerate() {
                    phase[start + i] = *val;
                }
                for (i, val) in c_out.iter().enumerate() {
                    combined[start + i] = *val;
                }
            }
        });

        SimdBatchResult {
            fresnel,
            transmittance,
            phase,
            combined,
        }
    }

    /// Get configuration
    pub fn config(&self) -> &ParallelConfig {
        &self.config
    }
}

impl Default for ParallelBatchEvaluator {
    fn default() -> Self {
        Self::new(ParallelConfig::default())
    }
}

// ============================================================================
// PARALLEL COMBINED EFFECTS
// ============================================================================

/// Evaluate multiple combined materials in parallel
pub fn parallel_combined_effects(materials: &[CombinedMaterial], cos_theta: f64) -> Vec<[f64; 3]> {
    if materials.len() < 100 {
        // Sequential for small batches
        return materials
            .iter()
            .map(|m| m.evaluate_rgb(cos_theta))
            .collect();
    }

    // Parallel evaluation
    let chunk_size = 64;
    let mut results = vec![[0.0; 3]; materials.len()];

    std::thread::scope(|s| {
        let result_chunks: Vec<_> = results.chunks_mut(chunk_size).collect();
        let material_chunks: Vec<_> = materials.chunks(chunk_size).collect();

        let handles: Vec<_> = result_chunks
            .into_iter()
            .zip(material_chunks.into_iter())
            .enumerate()
            .map(|(idx, (result_chunk, material_chunk))| {
                s.spawn(move || {
                    for (i, material) in material_chunk.iter().enumerate() {
                        result_chunk[i] = material.evaluate_rgb(cos_theta);
                    }
                    idx
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }
    });

    results
}

// ============================================================================
// PARALLEL PERCEPTUAL LOSS
// ============================================================================

/// Compute perceptual loss in parallel across color pairs
pub fn parallel_perceptual_loss(rendered: &[[f64; 3]], reference: &[[f64; 3]]) -> f64 {
    assert_eq!(
        rendered.len(),
        reference.len(),
        "Arrays must have same length"
    );

    let n = rendered.len();
    if n == 0 {
        return 0.0;
    }

    if n < 100 {
        // Sequential for small batches
        let mut total = 0.0;
        for i in 0..n {
            let lab1 = rgb_to_lab(rendered[i], Illuminant::D65);
            let lab2 = rgb_to_lab(reference[i], Illuminant::D65);
            total += delta_e_2000(lab1, lab2);
        }
        return total / n as f64;
    }

    // Parallel evaluation
    let chunk_size = 64;
    let num_chunks = (n + chunk_size - 1) / chunk_size;
    let mut chunk_sums = vec![0.0; num_chunks];
    let mut chunk_counts = vec![0usize; num_chunks];

    std::thread::scope(|s| {
        let handles: Vec<_> = (0..num_chunks)
            .map(|chunk_idx| {
                let start = chunk_idx * chunk_size;
                let end = (start + chunk_size).min(n);
                let rendered_slice = &rendered[start..end];
                let reference_slice = &reference[start..end];

                s.spawn(move || {
                    let mut sum = 0.0;
                    for i in 0..rendered_slice.len() {
                        let lab1 = rgb_to_lab(rendered_slice[i], Illuminant::D65);
                        let lab2 = rgb_to_lab(reference_slice[i], Illuminant::D65);
                        sum += delta_e_2000(lab1, lab2);
                    }
                    (chunk_idx, sum, rendered_slice.len())
                })
            })
            .collect();

        for handle in handles {
            let (idx, sum, count) = handle.join().unwrap();
            chunk_sums[idx] = sum;
            chunk_counts[idx] = count;
        }
    });

    let total_sum: f64 = chunk_sums.iter().sum();
    let total_count: usize = chunk_counts.iter().sum();

    total_sum / total_count as f64
}

// ============================================================================
// BENCHMARKING
// ============================================================================

/// Parallel benchmark results
#[derive(Debug, Clone)]
pub struct ParallelBenchmark {
    /// Sequential throughput (materials/second)
    pub sequential_throughput: f64,
    /// Parallel throughput (materials/second)
    pub parallel_throughput: f64,
    /// Speedup factor (parallel / sequential)
    pub speedup: f64,
    /// Efficiency (speedup / thread_count)
    pub efficiency: f64,
    /// Number of threads used
    pub thread_count: usize,
    /// Batch size tested
    pub batch_size: usize,
}

impl ParallelBenchmark {
    /// Generate markdown report
    pub fn to_markdown(&self) -> String {
        format!(
            r#"### Parallel Benchmark Results

| Metric | Value |
|--------|-------|
| Sequential Throughput | {:.2}M materials/s |
| Parallel Throughput | {:.2}M materials/s |
| Speedup | {:.2}x |
| Efficiency | {:.1}% |
| Thread Count | {} |
| Batch Size | {} |
"#,
            self.sequential_throughput / 1_000_000.0,
            self.parallel_throughput / 1_000_000.0,
            self.speedup,
            self.efficiency * 100.0,
            self.thread_count,
            self.batch_size
        )
    }
}

/// Run parallel vs sequential benchmark
pub fn benchmark_parallel(batch_size: usize, iterations: usize) -> ParallelBenchmark {
    let input = SimdBatchInput::uniform(batch_size, 1.5, 0.8, 0.1, 10.0);

    // Sequential benchmark
    let seq_config = ParallelConfig::sequential();
    let seq_evaluator = ParallelBatchEvaluator::new(seq_config);

    let start = Instant::now();
    for _ in 0..iterations {
        let _ = seq_evaluator.evaluate(&input);
    }
    let seq_time = start.elapsed().as_secs_f64();
    let seq_throughput = (batch_size * iterations) as f64 / seq_time;

    // Parallel benchmark
    let par_config = ParallelConfig::max_parallel();
    let thread_count = par_config.effective_threads();
    let par_evaluator = ParallelBatchEvaluator::new(par_config);

    let start = Instant::now();
    for _ in 0..iterations {
        let _ = par_evaluator.evaluate(&input);
    }
    let par_time = start.elapsed().as_secs_f64();
    let par_throughput = (batch_size * iterations) as f64 / par_time;

    let speedup = par_throughput / seq_throughput;
    let efficiency = speedup / thread_count as f64;

    ParallelBenchmark {
        sequential_throughput: seq_throughput,
        parallel_throughput: par_throughput,
        speedup,
        efficiency,
        thread_count,
        batch_size,
    }
}

/// Estimate memory usage for parallel evaluation
pub fn estimate_parallel_memory(batch_size: usize, config: &ParallelConfig) -> usize {
    // Input: 5 vectors × batch_size × 8 bytes
    let input_mem = 5 * batch_size * 8;

    // Output: 4 vectors × batch_size × 8 bytes
    let output_mem = 4 * batch_size * 8;

    // Thread-local buffers: chunk_size × 4 vectors × thread_count × 8 bytes
    let thread_mem = config.chunk_size * 4 * config.effective_threads() * 8;

    // Config overhead
    let config_mem = std::mem::size_of::<ParallelConfig>() + std::mem::size_of::<SimdConfig>();

    input_mem + output_mem + thread_mem + config_mem
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_config_defaults() {
        let config = ParallelConfig::default();
        assert_eq!(config.chunk_size, 256);
        assert_eq!(config.simd_width, 4);
        assert!(config.parallel_enabled);
    }

    #[test]
    fn test_sequential_vs_parallel_consistency() {
        let input = SimdBatchInput::uniform(1000, 1.5, 0.8, 0.1, 10.0);

        let seq_eval = ParallelBatchEvaluator::new(ParallelConfig::sequential());
        let par_eval = ParallelBatchEvaluator::new(ParallelConfig::max_parallel());

        let seq_result = seq_eval.evaluate(&input);
        let par_result = par_eval.evaluate(&input);

        // Results should be identical
        for i in 0..1000 {
            assert!(
                (seq_result.fresnel[i] - par_result.fresnel[i]).abs() < 1e-10,
                "Fresnel mismatch at {}",
                i
            );
            assert!(
                (seq_result.transmittance[i] - par_result.transmittance[i]).abs() < 1e-10,
                "Transmittance mismatch at {}",
                i
            );
            assert!(
                (seq_result.phase[i] - par_result.phase[i]).abs() < 1e-10,
                "Phase mismatch at {}",
                i
            );
            assert!(
                (seq_result.combined[i] - par_result.combined[i]).abs() < 1e-10,
                "Combined mismatch at {}",
                i
            );
        }
    }

    #[test]
    fn test_adaptive_threshold() {
        let small_input = SimdBatchInput::uniform(100, 1.5, 0.8, 0.1, 10.0);
        let large_input = SimdBatchInput::uniform(5000, 1.5, 0.8, 0.1, 10.0);

        let config = ParallelConfig::new();
        let evaluator = ParallelBatchEvaluator::new(config);

        // Small batch should use sequential
        let _ = evaluator.evaluate(&small_input); // Should not spawn threads

        // Large batch should use parallel
        let _ = evaluator.evaluate(&large_input); // Should use threads
    }

    #[test]
    fn test_parallel_combined_effects() {
        let materials: Vec<CombinedMaterial> =
            (0..200).map(|_| combined_presets::glass()).collect();

        let results = parallel_combined_effects(&materials, 0.8);

        assert_eq!(results.len(), 200);
        for rgb in &results {
            assert!(rgb[0] >= 0.0 && rgb[0] <= 1.0);
            assert!(rgb[1] >= 0.0 && rgb[1] <= 1.0);
            assert!(rgb[2] >= 0.0 && rgb[2] <= 1.0);
        }
    }

    #[test]
    fn test_parallel_perceptual_loss() {
        let rendered: Vec<[f64; 3]> = (0..200)
            .map(|i| [0.5 + (i as f64 * 0.001), 0.5, 0.5])
            .collect();
        let reference: Vec<[f64; 3]> = vec![[0.5, 0.5, 0.5]; 200];

        let loss = parallel_perceptual_loss(&rendered, &reference);

        // Should have some small loss due to variations
        assert!(loss > 0.0);
        assert!(loss < 50.0); // Reasonable bound for Delta E calculations
    }

    #[test]
    fn test_benchmark_runs() {
        let result = benchmark_parallel(1000, 10);

        assert!(result.sequential_throughput > 0.0);
        assert!(result.parallel_throughput > 0.0);
        assert!(result.speedup > 0.0);
        assert!(result.efficiency > 0.0);
    }

    #[test]
    fn test_memory_estimate() {
        let config = ParallelConfig::new();
        let mem = estimate_parallel_memory(10000, &config);

        // Should be reasonable (< 1MB for 10k materials)
        assert!(mem < 1_000_000);
    }

    #[test]
    fn test_effective_threads() {
        let auto = ParallelConfig::new();
        assert!(auto.effective_threads() >= 1);

        let fixed = ParallelConfig::new().with_threads(8);
        assert_eq!(fixed.effective_threads(), 8);
    }
}

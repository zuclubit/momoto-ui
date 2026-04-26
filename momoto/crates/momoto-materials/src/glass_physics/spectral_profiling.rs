//! Sprint 7 - Spectral Pipeline Profiling
//!
//! Scientific profiling to establish baseline performance metrics.
//! All optimizations MUST be justified by these measurements.
//!
//! Metrics tracked:
//! - Time per spectral evaluation
//! - Cost per phenomenon (ThinFilm TMM, Dispersion, Mie, ThermoOptic)
//! - Cost per number of λ samples
//! - Memory usage per operation

use std::collections::HashMap;
use std::time::Instant;

use super::spectral_pipeline::*;

// ============================================================================
// Profiling Types
// ============================================================================

/// Individual timing measurement
#[derive(Debug, Clone)]
pub struct TimingMeasurement {
    pub duration_ns: u64,
    pub sample_count: usize,
    pub stage_name: String,
}

/// Aggregated statistics for a stage
#[derive(Debug, Clone, Default)]
pub struct StageStats {
    pub total_calls: u64,
    pub total_time_ns: u64,
    pub min_time_ns: u64,
    pub max_time_ns: u64,
    pub mean_time_ns: f64,
    pub std_dev_ns: f64,
    times: Vec<u64>,
}

impl StageStats {
    pub fn new() -> Self {
        Self {
            min_time_ns: u64::MAX,
            ..Default::default()
        }
    }

    pub fn record(&mut self, duration_ns: u64) {
        self.total_calls += 1;
        self.total_time_ns += duration_ns;
        self.min_time_ns = self.min_time_ns.min(duration_ns);
        self.max_time_ns = self.max_time_ns.max(duration_ns);
        self.times.push(duration_ns);

        // Update mean
        self.mean_time_ns = self.total_time_ns as f64 / self.total_calls as f64;

        // Update std dev
        if self.total_calls > 1 {
            let variance: f64 = self
                .times
                .iter()
                .map(|&t| {
                    let diff = t as f64 - self.mean_time_ns;
                    diff * diff
                })
                .sum::<f64>()
                / (self.total_calls - 1) as f64;
            self.std_dev_ns = variance.sqrt();
        }
    }

    /// Time per sample in nanoseconds
    pub fn time_per_sample_ns(&self, sample_count: usize) -> f64 {
        if sample_count == 0 {
            return 0.0;
        }
        self.mean_time_ns / sample_count as f64
    }

    /// Equivalent FPS for single evaluation
    pub fn equivalent_fps(&self) -> f64 {
        if self.mean_time_ns == 0.0 {
            return f64::INFINITY;
        }
        1_000_000_000.0 / self.mean_time_ns
    }
}

/// Complete profiling report
#[derive(Debug, Clone)]
pub struct ProfilingReport {
    pub stages: HashMap<String, StageStats>,
    pub total_pipeline: StageStats,
    pub sample_counts_tested: Vec<usize>,
    pub timestamp: std::time::SystemTime,
}

impl ProfilingReport {
    pub fn new() -> Self {
        Self {
            stages: HashMap::new(),
            total_pipeline: StageStats::new(),
            sample_counts_tested: Vec::new(),
            timestamp: std::time::SystemTime::now(),
        }
    }

    /// Generate markdown table
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        md.push_str("# Spectral Pipeline Profiling Report\n\n");

        // Stage breakdown
        md.push_str("## Stage Performance\n\n");
        md.push_str("| Stage | Calls | Mean (μs) | Min (μs) | Max (μs) | Std Dev | FPS equiv |\n");
        md.push_str("|-------|-------|-----------|----------|----------|---------|----------|\n");

        let mut stages: Vec<_> = self.stages.iter().collect();
        stages.sort_by(|a, b| b.1.total_time_ns.cmp(&a.1.total_time_ns));

        for (name, stats) in &stages {
            md.push_str(&format!(
                "| {} | {} | {:.2} | {:.2} | {:.2} | {:.2} | {:.0} |\n",
                name,
                stats.total_calls,
                stats.mean_time_ns / 1000.0,
                stats.min_time_ns as f64 / 1000.0,
                stats.max_time_ns as f64 / 1000.0,
                stats.std_dev_ns / 1000.0,
                stats.equivalent_fps()
            ));
        }

        // Total pipeline
        md.push_str("\n## Total Pipeline\n\n");
        md.push_str(&format!(
            "- **Mean time**: {:.2} μs\n",
            self.total_pipeline.mean_time_ns / 1000.0
        ));
        md.push_str(&format!(
            "- **Equivalent FPS**: {:.0}\n",
            self.total_pipeline.equivalent_fps()
        ));

        md
    }
}

impl Default for ProfilingReport {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Profiler
// ============================================================================

/// Spectral pipeline profiler
pub struct SpectralProfiler {
    report: ProfilingReport,
    warmup_iterations: usize,
    measurement_iterations: usize,
}

impl SpectralProfiler {
    pub fn new() -> Self {
        Self {
            report: ProfilingReport::new(),
            warmup_iterations: 10,
            measurement_iterations: 100,
        }
    }

    pub fn with_iterations(mut self, warmup: usize, measure: usize) -> Self {
        self.warmup_iterations = warmup;
        self.measurement_iterations = measure;
        self
    }

    /// Profile a single stage
    pub fn profile_stage<F>(&mut self, name: &str, mut f: F)
    where
        F: FnMut(),
    {
        // Warmup
        for _ in 0..self.warmup_iterations {
            f();
        }

        // Get or create stats
        let stats = self
            .report
            .stages
            .entry(name.to_string())
            .or_insert_with(StageStats::new);

        // Measure
        for _ in 0..self.measurement_iterations {
            let start = Instant::now();
            f();
            let elapsed = start.elapsed().as_nanos() as u64;
            stats.record(elapsed);
        }
    }

    /// Profile complete pipeline
    pub fn profile_pipeline(
        &mut self,
        pipeline: &SpectralPipeline,
        incident: &SpectralSignal,
        context: &EvaluationContext,
    ) {
        // Warmup
        for _ in 0..self.warmup_iterations {
            let _ = pipeline.evaluate(incident, context);
        }

        // Measure
        for _ in 0..self.measurement_iterations {
            let start = Instant::now();
            let _ = pipeline.evaluate(incident, context);
            let elapsed = start.elapsed().as_nanos() as u64;
            self.report.total_pipeline.record(elapsed);
        }
    }

    /// Get the report
    pub fn report(&self) -> &ProfilingReport {
        &self.report
    }

    /// Consume and return the report
    pub fn into_report(self) -> ProfilingReport {
        self.report
    }
}

impl Default for SpectralProfiler {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Benchmark Suite
// ============================================================================

/// Complete benchmark suite for spectral pipeline
pub struct SpectralBenchmarkSuite {
    pub sample_counts: Vec<usize>,
    pub iterations: usize,
}

impl Default for SpectralBenchmarkSuite {
    fn default() -> Self {
        Self {
            sample_counts: vec![3, 8, 16, 31, 81, 161, 401],
            iterations: 100,
        }
    }
}

/// Benchmark result for a specific configuration
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub sample_count: usize,
    pub stage_name: String,
    pub mean_time_us: f64,
    pub std_dev_us: f64,
    pub fps_equivalent: f64,
    pub time_per_sample_ns: f64,
}

impl SpectralBenchmarkSuite {
    pub fn new() -> Self {
        Self::default()
    }

    /// Run complete benchmark suite
    pub fn run(&self) -> Vec<BenchmarkResult> {
        let mut results = Vec::new();

        for &sample_count in &self.sample_counts {
            // Create sampling wavelengths
            let wavelengths: Vec<f64> = (0..sample_count)
                .map(|i| 380.0 + (400.0 * i as f64 / (sample_count - 1).max(1) as f64))
                .collect();

            let intensities: Vec<f64> = vec![1.0; sample_count];
            let incident = SpectralSignal::from_arrays(&wavelengths, &intensities);
            let context = EvaluationContext::default();

            // Benchmark each stage type
            results.extend(self.benchmark_thin_film(&incident, &context, sample_count));
            results.extend(self.benchmark_dispersion(&incident, &context, sample_count));
            results.extend(self.benchmark_mie(&incident, &context, sample_count));
            results.extend(self.benchmark_thermo_optic(&incident, &context, sample_count));
            results.extend(self.benchmark_metal(&incident, &context, sample_count));

            // Benchmark full pipeline
            results.extend(self.benchmark_full_pipeline(&incident, &context, sample_count));
        }

        results
    }

    fn benchmark_thin_film(
        &self,
        incident: &SpectralSignal,
        context: &EvaluationContext,
        sample_count: usize,
    ) -> Vec<BenchmarkResult> {
        let pipeline = SpectralPipeline::new().add_stage(ThinFilmStage::new(1.45, 300.0, 1.52));

        self.measure_pipeline("ThinFilm", &pipeline, incident, context, sample_count)
    }

    fn benchmark_dispersion(
        &self,
        incident: &SpectralSignal,
        context: &EvaluationContext,
        sample_count: usize,
    ) -> Vec<BenchmarkResult> {
        let pipeline = SpectralPipeline::new().add_stage(DispersionStage::crown_glass());

        self.measure_pipeline("Dispersion", &pipeline, incident, context, sample_count)
    }

    fn benchmark_mie(
        &self,
        incident: &SpectralSignal,
        context: &EvaluationContext,
        sample_count: usize,
    ) -> Vec<BenchmarkResult> {
        let pipeline = SpectralPipeline::new().add_stage(MieScatteringStage::fog());

        self.measure_pipeline("Mie", &pipeline, incident, context, sample_count)
    }

    fn benchmark_thermo_optic(
        &self,
        incident: &SpectralSignal,
        context: &EvaluationContext,
        sample_count: usize,
    ) -> Vec<BenchmarkResult> {
        let pipeline = SpectralPipeline::new().add_stage(ThermoOpticStage::glass_coating(100.0));

        self.measure_pipeline("ThermoOptic", &pipeline, incident, context, sample_count)
    }

    fn benchmark_metal(
        &self,
        incident: &SpectralSignal,
        context: &EvaluationContext,
        sample_count: usize,
    ) -> Vec<BenchmarkResult> {
        let pipeline = SpectralPipeline::new().add_stage(MetalReflectanceStage::gold());

        self.measure_pipeline("Metal", &pipeline, incident, context, sample_count)
    }

    fn benchmark_full_pipeline(
        &self,
        incident: &SpectralSignal,
        context: &EvaluationContext,
        sample_count: usize,
    ) -> Vec<BenchmarkResult> {
        let pipeline = SpectralPipeline::new()
            .add_stage(ThinFilmStage::new(1.45, 300.0, 1.52))
            .add_stage(DispersionStage::crown_glass())
            .add_stage(MieScatteringStage::fog());

        self.measure_pipeline("FullPipeline", &pipeline, incident, context, sample_count)
    }

    fn measure_pipeline(
        &self,
        name: &str,
        pipeline: &SpectralPipeline,
        incident: &SpectralSignal,
        context: &EvaluationContext,
        sample_count: usize,
    ) -> Vec<BenchmarkResult> {
        // Warmup
        for _ in 0..10 {
            let _ = pipeline.evaluate(incident, context);
        }

        // Measure
        let mut times_ns: Vec<u64> = Vec::with_capacity(self.iterations);
        for _ in 0..self.iterations {
            let start = Instant::now();
            let _ = pipeline.evaluate(incident, context);
            times_ns.push(start.elapsed().as_nanos() as u64);
        }

        // Calculate statistics
        let mean_ns = times_ns.iter().sum::<u64>() as f64 / times_ns.len() as f64;
        let variance = times_ns
            .iter()
            .map(|&t| {
                let diff = t as f64 - mean_ns;
                diff * diff
            })
            .sum::<f64>()
            / (times_ns.len() - 1) as f64;
        let std_dev_ns = variance.sqrt();

        vec![BenchmarkResult {
            sample_count,
            stage_name: name.to_string(),
            mean_time_us: mean_ns / 1000.0,
            std_dev_us: std_dev_ns / 1000.0,
            fps_equivalent: 1_000_000_000.0 / mean_ns,
            time_per_sample_ns: mean_ns / sample_count as f64,
        }]
    }

    /// Generate markdown report
    pub fn results_to_markdown(results: &[BenchmarkResult]) -> String {
        let mut md = String::new();

        md.push_str("# Spectral Pipeline Benchmark Results\n\n");
        md.push_str("## Performance by Stage and Sample Count\n\n");
        md.push_str("| Stage | Samples | Mean (μs) | Std Dev | FPS equiv | ns/sample |\n");
        md.push_str("|-------|---------|-----------|---------|-----------|----------|\n");

        for result in results {
            md.push_str(&format!(
                "| {} | {} | {:.2} | {:.2} | {:.0} | {:.2} |\n",
                result.stage_name,
                result.sample_count,
                result.mean_time_us,
                result.std_dev_us,
                result.fps_equivalent,
                result.time_per_sample_ns,
            ));
        }

        // Summary by stage
        md.push_str("\n## Summary by Stage (81 samples, default)\n\n");

        let default_sample_count = 81;
        let default_results: Vec<_> = results
            .iter()
            .filter(|r| r.sample_count == default_sample_count)
            .collect();

        if !default_results.is_empty() {
            md.push_str("| Stage | Time (μs) | FPS equiv |\n");
            md.push_str("|-------|-----------|----------|\n");

            for result in &default_results {
                md.push_str(&format!(
                    "| {} | {:.2} | {:.0} |\n",
                    result.stage_name, result.mean_time_us, result.fps_equivalent,
                ));
            }
        }

        md
    }
}

// ============================================================================
// Run Full Benchmark (callable from WASM/tests)
// ============================================================================

/// Run comprehensive benchmark and return structured results
pub fn run_full_benchmark() -> Vec<BenchmarkResult> {
    let suite = SpectralBenchmarkSuite::default();
    suite.run()
}

/// Run quick benchmark with fewer samples for WASM
pub fn run_quick_benchmark() -> Vec<BenchmarkResult> {
    let suite = SpectralBenchmarkSuite {
        sample_counts: vec![8, 31, 81],
        iterations: 50,
    };
    suite.run()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_suite_runs() {
        let suite = SpectralBenchmarkSuite {
            sample_counts: vec![8, 31],
            iterations: 10,
        };

        let results = suite.run();
        assert!(!results.is_empty());

        // Print results for inspection
        for result in &results {
            println!(
                "{} @ {} samples: {:.2} μs ({:.0} FPS)",
                result.stage_name, result.sample_count, result.mean_time_us, result.fps_equivalent
            );
        }
    }

    #[test]
    fn test_full_benchmark_comprehensive() {
        let results = run_full_benchmark();

        println!(
            "\n{}",
            SpectralBenchmarkSuite::results_to_markdown(&results)
        );

        // Verify we have results for all sample counts
        let sample_counts: std::collections::HashSet<_> =
            results.iter().map(|r| r.sample_count).collect();

        assert!(sample_counts.contains(&81), "Missing default 81 samples");
    }

    #[test]
    fn test_profiler_records_stats() {
        let mut profiler = SpectralProfiler::new().with_iterations(5, 20);

        profiler.profile_stage("test_stage", || {
            // Simulate some work
            let mut sum = 0.0;
            for i in 0..1000 {
                sum += (i as f64).sin();
            }
            std::hint::black_box(sum);
        });

        let report = profiler.report();
        let stats = report.stages.get("test_stage").unwrap();

        assert_eq!(stats.total_calls, 20);
        assert!(stats.mean_time_ns > 0.0);
    }

    #[test]
    fn test_stage_stats_calculations() {
        let mut stats = StageStats::new();

        stats.record(100);
        stats.record(200);
        stats.record(300);

        assert_eq!(stats.total_calls, 3);
        assert_eq!(stats.min_time_ns, 100);
        assert_eq!(stats.max_time_ns, 300);
        assert!((stats.mean_time_ns - 200.0).abs() < 0.01);
    }
}

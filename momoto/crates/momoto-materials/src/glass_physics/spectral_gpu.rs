//! Sprint 7 - GPU/WebGPU Spectral Compute
//!
//! GPU-accelerated spectral pipeline evaluation using WebGPU/WGSL.
//! Provides massive parallelism for batch evaluations.
//!
//! ## Architecture
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    SpectralGPU                                   │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  CPU Path: For single evaluations and fallback                  │
//! │  GPU Path: Batch evaluation with compute shaders                │
//! │  Auto-select: Choose optimal path based on workload             │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## WGSL Compute Shader Strategy
//! - Each workgroup handles one spectral sample
//! - Shared memory for wavelength data
//! - Coalesced memory access for batch inputs
//!
//! ## Performance Targets
//! - Single eval: < 1μs (CPU path)
//! - Batch 1000: < 100μs (GPU path, ~10ns/eval)

// ============================================================================
// GPU Backend Abstraction
// ============================================================================

/// GPU backend availability
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuBackend {
    /// WebGPU available (browser/native)
    WebGPU,
    /// CPU fallback only
    CpuOnly,
}

impl GpuBackend {
    /// Detect available backend
    pub fn detect() -> Self {
        // In WASM context, would probe for WebGPU support
        // For now, always use CPU fallback
        GpuBackend::CpuOnly
    }

    /// Check if GPU acceleration is available
    pub fn is_gpu_available(&self) -> bool {
        matches!(self, GpuBackend::WebGPU)
    }
}

// ============================================================================
// Batch Input/Output
// ============================================================================

/// Input for batch spectral evaluation
#[derive(Debug, Clone)]
pub struct BatchSpectralInput {
    /// Pipeline configuration hash
    pub pipeline_hash: u64,
    /// Array of (angle_deg, temp_k) pairs
    pub contexts: Vec<(f64, f64)>,
}

impl BatchSpectralInput {
    pub fn new(pipeline_hash: u64) -> Self {
        Self {
            pipeline_hash,
            contexts: Vec::new(),
        }
    }

    pub fn add_context(&mut self, angle_deg: f64, temp_k: f64) {
        self.contexts.push((angle_deg, temp_k));
    }

    pub fn with_angle(mut self, angle_deg: f64) -> Self {
        self.contexts.push((angle_deg, 293.15));
        self
    }

    pub fn with_angles(mut self, angles: &[f64]) -> Self {
        for &angle in angles {
            self.contexts.push((angle, 293.15));
        }
        self
    }

    pub fn len(&self) -> usize {
        self.contexts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.contexts.is_empty()
    }
}

/// Output from batch spectral evaluation
#[derive(Debug, Clone)]
pub struct BatchSpectralOutput {
    /// RGB results for each input context
    pub rgb_results: Vec<[f64; 3]>,
    /// Energy ratios for each input context
    pub energy_ratios: Vec<f64>,
    /// Backend used for evaluation
    pub backend: GpuBackend,
    /// Total evaluation time in microseconds
    pub time_us: f64,
}

impl BatchSpectralOutput {
    pub fn new(capacity: usize, backend: GpuBackend) -> Self {
        Self {
            rgb_results: Vec::with_capacity(capacity),
            energy_ratios: Vec::with_capacity(capacity),
            backend,
            time_us: 0.0,
        }
    }

    pub fn throughput(&self) -> f64 {
        if self.time_us > 0.0 {
            self.rgb_results.len() as f64 / self.time_us * 1_000_000.0
        } else {
            0.0
        }
    }

    pub fn time_per_eval_ns(&self) -> f64 {
        if !self.rgb_results.is_empty() {
            self.time_us * 1000.0 / self.rgb_results.len() as f64
        } else {
            0.0
        }
    }
}

// ============================================================================
// GPU Evaluator
// ============================================================================

use super::spectral_cache::{PipelineHasher, SpectralCache, SpectralCacheKey};
use super::spectral_pipeline::*;
use std::time::Instant;

/// GPU-accelerated spectral evaluator with CPU fallback
pub struct SpectralGpuEvaluator {
    backend: GpuBackend,
    cache: SpectralCache,
    /// Threshold for switching to GPU path (batch size)
    gpu_threshold: usize,
}

impl SpectralGpuEvaluator {
    pub fn new() -> Self {
        Self {
            backend: GpuBackend::detect(),
            cache: SpectralCache::new(10000),
            gpu_threshold: 100, // Use GPU for batches > 100
        }
    }

    /// Evaluate a batch of thin film configurations
    pub fn eval_thin_film_batch(
        &mut self,
        n: f64,
        thickness_nm: f64,
        substrate_n: f64,
        input: &BatchSpectralInput,
    ) -> BatchSpectralOutput {
        let start = Instant::now();
        let mut output = BatchSpectralOutput::new(input.len(), self.backend);

        if input.len() > self.gpu_threshold && self.backend == GpuBackend::WebGPU {
            // GPU path (not yet implemented)
            self.eval_thin_film_batch_gpu(n, thickness_nm, substrate_n, input, &mut output);
        } else {
            // CPU path with cache
            self.eval_thin_film_batch_cpu(n, thickness_nm, substrate_n, input, &mut output);
        }

        output.time_us = start.elapsed().as_secs_f64() * 1_000_000.0;
        output
    }

    /// CPU batch evaluation (with cache)
    fn eval_thin_film_batch_cpu(
        &mut self,
        n: f64,
        thickness_nm: f64,
        substrate_n: f64,
        input: &BatchSpectralInput,
        output: &mut BatchSpectralOutput,
    ) {
        let d65 = SpectralSignal::d65_illuminant();
        let pipeline_hash = PipelineHasher::new()
            .add_thin_film(n, thickness_nm, substrate_n)
            .finish();

        for &(angle_deg, temp_k) in &input.contexts {
            let key = SpectralCacheKey::new(pipeline_hash, angle_deg, temp_k);

            let (rgb, energy) = self.cache.get_or_compute(key, || {
                let pipeline = SpectralPipeline::new().add_stage(ThinFilmStage::new(
                    n,
                    thickness_nm,
                    substrate_n,
                ));
                let context = EvaluationContext::default()
                    .with_angle_deg(angle_deg)
                    .with_temperature(temp_k);
                let result = pipeline.evaluate(&d65, &context);

                let rgb = result.to_rgb();
                let energy = result.total_energy() / d65.total_energy();
                (rgb, energy)
            });

            output.rgb_results.push(rgb);
            output.energy_ratios.push(energy);
        }
    }

    /// GPU batch evaluation (placeholder)
    fn eval_thin_film_batch_gpu(
        &self,
        _n: f64,
        _thickness_nm: f64,
        _substrate_n: f64,
        _input: &BatchSpectralInput,
        _output: &mut BatchSpectralOutput,
    ) {
        // TODO: Implement WebGPU compute shader path
        // Would use wgpu crate with WGSL shaders
        unimplemented!("WebGPU path not yet implemented");
    }

    /// Evaluate a batch of metal configurations
    pub fn eval_metal_batch(
        &mut self,
        metal_type: &str,
        input: &BatchSpectralInput,
    ) -> BatchSpectralOutput {
        let start = Instant::now();
        let mut output = BatchSpectralOutput::new(input.len(), self.backend);

        let d65 = SpectralSignal::d65_illuminant();
        let pipeline_hash = PipelineHasher::new().add_metal(metal_type).finish();

        for &(angle_deg, temp_k) in &input.contexts {
            let key = SpectralCacheKey::new(pipeline_hash, angle_deg, temp_k);

            let (rgb, energy) = self.cache.get_or_compute(key, || {
                let stage = match metal_type.to_lowercase().as_str() {
                    "gold" => MetalReflectanceStage::gold(),
                    "silver" => MetalReflectanceStage::silver(),
                    "copper" => MetalReflectanceStage::copper(),
                    _ => MetalReflectanceStage::gold(),
                };
                let pipeline = SpectralPipeline::new().add_stage(stage);
                let context = EvaluationContext::default()
                    .with_angle_deg(angle_deg)
                    .with_temperature(temp_k);
                let result = pipeline.evaluate(&d65, &context);

                let rgb = result.to_rgb();
                let energy = result.total_energy() / d65.total_energy();
                (rgb, energy)
            });

            output.rgb_results.push(rgb);
            output.energy_ratios.push(energy);
        }

        output.time_us = start.elapsed().as_secs_f64() * 1_000_000.0;
        output
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> super::spectral_cache::CacheStats {
        self.cache.stats()
    }

    /// Clear cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get backend info
    pub fn backend(&self) -> GpuBackend {
        self.backend
    }
}

impl Default for SpectralGpuEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// WGSL Shader Templates (for future WebGPU implementation)
// ============================================================================

/// WGSL compute shader for thin film interference
pub const THIN_FILM_WGSL: &str = r#"
// Thin Film Interference Compute Shader
// Evaluates optical path difference and Fresnel coefficients

struct ThinFilmParams {
    n: f32,           // Film refractive index
    thickness_nm: f32, // Film thickness in nm
    substrate_n: f32,  // Substrate refractive index
    padding: f32,
};

struct Context {
    angle_deg: f32,
    temp_k: f32,
    padding: vec2<f32>,
};

struct Result {
    rgb: vec3<f32>,
    energy: f32,
};

@group(0) @binding(0) var<uniform> params: ThinFilmParams;
@group(0) @binding(1) var<storage, read> contexts: array<Context>;
@group(0) @binding(2) var<storage, read_write> results: array<Result>;

// CIE 1931 color matching functions (sampled)
const CMF_X: array<f32, 16> = array<f32, 16>(
    0.0014, 0.0065, 0.0201, 0.0679, 0.2074, 0.3597, 0.4906, 0.4334,
    0.2423, 0.0691, 0.0067, 0.0490, 0.2586, 0.6424, 1.0622, 1.0026
);
const CMF_Y: array<f32, 16> = array<f32, 16>(
    0.0000, 0.0002, 0.0011, 0.0040, 0.0203, 0.0679, 0.1750, 0.3289,
    0.5030, 0.7100, 0.8620, 0.9540, 0.9950, 0.8700, 0.6310, 0.3810
);
const CMF_Z: array<f32, 16> = array<f32, 16>(
    0.0065, 0.0311, 0.1006, 0.3481, 1.0622, 1.9019, 2.0810, 1.4526,
    0.6660, 0.1582, 0.0270, 0.0040, 0.0010, 0.0000, 0.0000, 0.0000
);

// D65 illuminant (normalized)
const D65: array<f32, 16> = array<f32, 16>(
    0.5000, 0.6000, 0.7500, 0.9000, 0.9800, 1.0000, 1.0000, 0.9800,
    0.9500, 0.9200, 0.8800, 0.8500, 0.8200, 0.7800, 0.7200, 0.6500
);

const WAVELENGTHS: array<f32, 16> = array<f32, 16>(
    400.0, 425.0, 450.0, 475.0, 500.0, 525.0, 550.0, 575.0,
    600.0, 625.0, 650.0, 675.0, 700.0, 725.0, 750.0, 775.0
);

fn fresnel_reflectance(n1: f32, n2: f32, cos_theta: f32) -> f32 {
    let sin_theta2 = 1.0 - cos_theta * cos_theta;
    let n_ratio = n1 / n2;
    let sin_theta2_t = n_ratio * n_ratio * sin_theta2;

    if sin_theta2_t > 1.0 {
        return 1.0; // Total internal reflection
    }

    let cos_theta_t = sqrt(1.0 - sin_theta2_t);

    let rs = (n1 * cos_theta - n2 * cos_theta_t) / (n1 * cos_theta + n2 * cos_theta_t);
    let rp = (n2 * cos_theta - n1 * cos_theta_t) / (n2 * cos_theta + n1 * cos_theta_t);

    return 0.5 * (rs * rs + rp * rp);
}

fn thin_film_reflectance(wavelength: f32, angle_rad: f32) -> f32 {
    let cos_theta = cos(angle_rad);
    let sin_theta = sin(angle_rad);

    // Snell's law: sin(theta_film) = sin(theta_air) / n
    let sin_theta_film = sin_theta / params.n;
    let cos_theta_film = sqrt(1.0 - sin_theta_film * sin_theta_film);

    // Optical path difference
    let opd = 2.0 * params.n * params.thickness_nm * cos_theta_film;

    // Phase difference
    let phase = 2.0 * 3.14159265 * opd / wavelength;

    // Fresnel coefficients at interfaces
    let r1 = fresnel_reflectance(1.0, params.n, cos_theta);
    let r2 = fresnel_reflectance(params.n, params.substrate_n, cos_theta_film);

    // Airy formula for thin film interference
    let r = r1 + r2 + 2.0 * sqrt(r1 * r2) * cos(phase);

    return clamp(r, 0.0, 1.0);
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if idx >= arrayLength(&contexts) {
        return;
    }

    let ctx = contexts[idx];
    let angle_rad = ctx.angle_deg * 3.14159265 / 180.0;

    // Spectral integration
    var xyz = vec3<f32>(0.0, 0.0, 0.0);
    var total_energy = 0.0;
    var reflected_energy = 0.0;

    for (var i = 0u; i < 16u; i = i + 1u) {
        let wavelength = WAVELENGTHS[i];
        let illuminant = D65[i];
        let reflectance = thin_film_reflectance(wavelength, angle_rad);

        total_energy = total_energy + illuminant;
        reflected_energy = reflected_energy + illuminant * reflectance;

        xyz.x = xyz.x + illuminant * reflectance * CMF_X[i];
        xyz.y = xyz.y + illuminant * reflectance * CMF_Y[i];
        xyz.z = xyz.z + illuminant * reflectance * CMF_Z[i];
    }

    // XYZ to sRGB conversion
    var rgb: vec3<f32>;
    rgb.r =  3.2406 * xyz.x - 1.5372 * xyz.y - 0.4986 * xyz.z;
    rgb.g = -0.9689 * xyz.x + 1.8758 * xyz.y + 0.0415 * xyz.z;
    rgb.b =  0.0557 * xyz.x - 0.2040 * xyz.y + 1.0570 * xyz.z;

    // Gamma correction and clamp
    rgb = clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0));

    results[idx].rgb = rgb;
    results[idx].energy = reflected_energy / total_energy;
}
"#;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_input() {
        let input =
            BatchSpectralInput::new(12345).with_angles(&[0.0, 30.0, 45.0, 60.0, 75.0, 90.0]);

        assert_eq!(input.len(), 6);
        assert_eq!(input.contexts[0], (0.0, 293.15));
        assert_eq!(input.contexts[5], (90.0, 293.15));
    }

    #[test]
    fn test_cpu_batch_evaluation() {
        let mut evaluator = SpectralGpuEvaluator::new();

        let input = BatchSpectralInput::new(0).with_angles(&[0.0, 15.0, 30.0, 45.0, 60.0]);

        let output = evaluator.eval_thin_film_batch(1.45, 300.0, 1.52, &input);

        assert_eq!(output.rgb_results.len(), 5);
        assert_eq!(output.energy_ratios.len(), 5);
        assert_eq!(output.backend, GpuBackend::CpuOnly);

        println!("\nBatch Evaluation Results:");
        println!("  Backend: {:?}", output.backend);
        println!("  Time: {:.2} μs", output.time_us);
        println!("  Throughput: {:.0} evals/sec", output.throughput());
        println!("  Time per eval: {:.1} ns", output.time_per_eval_ns());
    }

    #[test]
    fn test_batch_with_cache() {
        let mut evaluator = SpectralGpuEvaluator::new();

        let angles: Vec<f64> = (0..100).map(|i| (i as f64) * 0.9).collect();
        let input = BatchSpectralInput::new(0).with_angles(&angles);

        // First pass: cold cache
        let output1 = evaluator.eval_thin_film_batch(1.45, 300.0, 1.52, &input);

        // Second pass: hot cache
        let output2 = evaluator.eval_thin_film_batch(1.45, 300.0, 1.52, &input);

        let speedup = output1.time_us / output2.time_us.max(0.001);

        println!("\nBatch with Cache:");
        println!(
            "  Cold: {:.2} μs ({:.1} ns/eval)",
            output1.time_us,
            output1.time_per_eval_ns()
        );
        println!(
            "  Hot:  {:.2} μs ({:.1} ns/eval)",
            output2.time_us,
            output2.time_per_eval_ns()
        );
        println!("  Speedup: {:.1}×", speedup);
        println!("  {}", evaluator.cache_stats().summary());

        assert!(speedup > 5.0, "Expected >5× speedup with cache");
    }

    #[test]
    fn test_metal_batch() {
        let mut evaluator = SpectralGpuEvaluator::new();

        let input = BatchSpectralInput::new(0).with_angles(&[0.0, 30.0, 60.0, 80.0]);

        for metal in ["gold", "silver", "copper"] {
            let output = evaluator.eval_metal_batch(metal, &input);

            println!(
                "\n{} batch: {} results in {:.2} μs",
                metal,
                output.rgb_results.len(),
                output.time_us
            );

            for (i, (rgb, energy)) in output
                .rgb_results
                .iter()
                .zip(&output.energy_ratios)
                .enumerate()
            {
                println!(
                    "  {}°: RGB=[{:.3}, {:.3}, {:.3}], E={:.3}",
                    input.contexts[i].0, rgb[0], rgb[1], rgb[2], energy
                );
            }
        }
    }

    #[test]
    fn test_backend_detection() {
        let backend = GpuBackend::detect();
        println!("\nDetected backend: {:?}", backend);
        println!("GPU available: {}", backend.is_gpu_available());

        // In test environment, should be CPU only
        assert_eq!(backend, GpuBackend::CpuOnly);
    }
}

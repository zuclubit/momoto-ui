//! # GPU Compute Backend
//!
//! WebGPU-accelerated BSDF evaluation using wgpu and WGSL compute shaders.
//!
//! This module provides GPU-accelerated batch evaluation of BSDFs while maintaining
//! numerical parity with the CPU reference implementation.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                     GpuBatchEvaluator                           │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  device.rs    │  Adapter/device initialization                  │
//! │  buffers.rs   │  SOA buffer pool for material data              │
//! │  pipelines.rs │  Compute pipeline cache per BSDF type           │
//! │  dispatch.rs  │  Workgroup dispatch and result readback         │
//! │  parity.rs    │  CPU/GPU comparison with ΔE2000 metric          │
//! │  fallback.rs  │  Graceful degradation to CPU path               │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use momoto_materials::glass_physics::gpu_backend::{GpuContext, GpuBatchEvaluator};
//!
//! // Initialize GPU context (async)
//! let context = GpuContext::new().await?;
//!
//! // Create batch evaluator with CPU fallback
//! let evaluator = GpuBatchEvaluator::new(&context);
//!
//! // Evaluate batch - automatically uses GPU if available
//! let result = evaluator.evaluate(&input);
//! ```
//!
//! ## Feature Flag
//!
//! This module requires the `gpu` feature to be enabled:
//!
//! ```toml
//! [dependencies]
//! momoto-materials = { version = "5.0", features = ["gpu"] }
//! ```
//!
//! ## Parity Guarantee
//!
//! GPU results are validated against CPU reference using ΔE2000 perceptual metric.
//! Parity threshold: ΔE2000 < 1.0 (imperceptible difference).

// Submodules (conditionally compiled with gpu feature)
#[cfg(feature = "gpu")]
pub mod buffers;
#[cfg(feature = "gpu")]
pub mod device;
#[cfg(feature = "gpu")]
pub mod dispatch;
#[cfg(feature = "gpu")]
pub mod fallback;
#[cfg(feature = "gpu")]
pub mod parity;
#[cfg(feature = "gpu")]
pub mod pipelines;
#[cfg(feature = "gpu")]
pub mod shaders;

// Re-exports when gpu feature is enabled
#[cfg(feature = "gpu")]
pub use buffers::{BufferPool, MaterialBuffer, ResponseBuffer};
#[cfg(feature = "gpu")]
pub use device::{GpuCapabilities, GpuContext, GpuContextConfig};
#[cfg(feature = "gpu")]
pub use dispatch::{GpuBatchEvaluator, GpuBatchResult, GpuDispatchConfig};
#[cfg(feature = "gpu")]
pub use fallback::{AutoFallback, FallbackReason, FallbackStats};
#[cfg(feature = "gpu")]
pub use parity::{GpuCpuParityTest, ParityConfig, ParityResult};
#[cfg(feature = "gpu")]
pub use pipelines::{ComputePipelineCache, PipelineType};

// Stub types when gpu feature is disabled (allows code to compile without feature)
#[cfg(not(feature = "gpu"))]
pub mod stubs {
    //! Stub implementations when GPU feature is disabled.
    //!
    //! These types allow code to compile without the `gpu` feature,
    //! but will return errors at runtime.

    use std::fmt;

    /// Error returned when GPU operations are attempted without the `gpu` feature.
    #[derive(Debug, Clone)]
    pub struct GpuNotAvailable;

    impl fmt::Display for GpuNotAvailable {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "GPU feature not enabled. Enable with `features = [\"gpu\"]`"
            )
        }
    }

    impl std::error::Error for GpuNotAvailable {}

    /// Stub GPU context that always fails.
    #[derive(Debug, Clone)]
    pub struct GpuContext;

    impl GpuContext {
        /// Attempts to create a GPU context.
        ///
        /// # Errors
        ///
        /// Always returns `Err(GpuNotAvailable)` when `gpu` feature is disabled.
        pub fn new() -> Result<Self, GpuNotAvailable> {
            Err(GpuNotAvailable)
        }

        /// Check if GPU is available (always false without feature).
        pub fn is_available() -> bool {
            false
        }
    }

    /// Stub batch evaluator that falls back to CPU.
    #[derive(Debug, Clone)]
    pub struct GpuBatchEvaluator;

    impl GpuBatchEvaluator {
        /// Creates a stub evaluator.
        pub fn new() -> Self {
            Self
        }

        /// Check if GPU path is active (always false without feature).
        pub fn is_gpu_active(&self) -> bool {
            false
        }
    }

    impl Default for GpuBatchEvaluator {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(not(feature = "gpu"))]
pub use stubs::{GpuBatchEvaluator, GpuContext, GpuNotAvailable};

// ============================================================================
// COMMON TYPES (available regardless of feature)
// ============================================================================

/// GPU backend configuration.
#[derive(Debug, Clone)]
pub struct GpuBackendConfig {
    /// Maximum batch size for GPU dispatch.
    pub max_batch_size: usize,
    /// Enable parity validation (slower but safer).
    pub validate_parity: bool,
    /// ΔE2000 threshold for parity validation.
    pub parity_threshold: f64,
    /// Prefer CPU for small batches (threshold).
    pub cpu_threshold: usize,
    /// Enable async GPU operations.
    pub async_dispatch: bool,
}

impl Default for GpuBackendConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 65536,
            validate_parity: false,
            parity_threshold: 1.0,
            cpu_threshold: 256,
            async_dispatch: true,
        }
    }
}

/// GPU backend statistics.
#[derive(Debug, Clone, Default)]
pub struct GpuBackendStats {
    /// Total GPU evaluations performed.
    pub gpu_evaluations: u64,
    /// Total CPU fallback evaluations.
    pub cpu_fallbacks: u64,
    /// Total parity violations detected.
    pub parity_violations: u64,
    /// Average GPU evaluation time (microseconds).
    pub avg_gpu_time_us: f64,
    /// Average CPU fallback time (microseconds).
    pub avg_cpu_time_us: f64,
}

impl GpuBackendStats {
    /// GPU utilization percentage.
    pub fn gpu_utilization(&self) -> f64 {
        let total = self.gpu_evaluations + self.cpu_fallbacks;
        if total == 0 {
            0.0
        } else {
            (self.gpu_evaluations as f64 / total as f64) * 100.0
        }
    }
}

/// Memory usage for GPU backend.
pub fn estimate_gpu_backend_memory() -> usize {
    // Base overhead for GPU backend infrastructure
    let base_overhead = 8 * 1024; // 8 KB

    // Shader cache (compiled shaders)
    let shader_cache = 5 * 1024; // 5 KB

    // Pipeline cache
    let pipeline_cache = 3 * 1024; // 3 KB

    base_overhead + shader_cache + pipeline_cache
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GpuBackendConfig::default();
        assert_eq!(config.max_batch_size, 65536);
        assert!(!config.validate_parity);
        assert!((config.parity_threshold - 1.0).abs() < f64::EPSILON);
        assert_eq!(config.cpu_threshold, 256);
        assert!(config.async_dispatch);
    }

    #[test]
    fn test_stats_utilization() {
        let mut stats = GpuBackendStats::default();
        assert_eq!(stats.gpu_utilization(), 0.0);

        stats.gpu_evaluations = 80;
        stats.cpu_fallbacks = 20;
        assert!((stats.gpu_utilization() - 80.0).abs() < 0.01);
    }

    #[test]
    fn test_memory_estimate() {
        let mem = estimate_gpu_backend_memory();
        assert!(mem > 0);
        assert!(mem < 100 * 1024); // Should be under 100KB
    }

    #[cfg(not(feature = "gpu"))]
    #[test]
    fn test_stub_context() {
        assert!(!GpuContext::is_available());
        assert!(GpuContext::new().is_err());
    }

    #[cfg(not(feature = "gpu"))]
    #[test]
    fn test_stub_evaluator() {
        let evaluator = GpuBatchEvaluator::new();
        assert!(!evaluator.is_gpu_active());
    }

    #[test]
    fn test_config_custom_values() {
        let config = GpuBackendConfig {
            max_batch_size: 1024,
            validate_parity: true,
            parity_threshold: 0.5,
            cpu_threshold: 64,
            async_dispatch: false,
        };
        assert_eq!(config.max_batch_size, 1024);
        assert!(config.validate_parity);
        assert!((config.parity_threshold - 0.5).abs() < f64::EPSILON);
        assert_eq!(config.cpu_threshold, 64);
        assert!(!config.async_dispatch);
    }

    #[test]
    fn test_stats_zero_gpu() {
        let stats = GpuBackendStats {
            gpu_evaluations: 0,
            cpu_fallbacks: 100,
            parity_violations: 0,
            avg_gpu_time_us: 0.0,
            avg_cpu_time_us: 10.0,
        };
        assert_eq!(stats.gpu_utilization(), 0.0);
    }

    #[test]
    fn test_stats_full_gpu() {
        let stats = GpuBackendStats {
            gpu_evaluations: 100,
            cpu_fallbacks: 0,
            parity_violations: 0,
            avg_gpu_time_us: 5.0,
            avg_cpu_time_us: 0.0,
        };
        assert!((stats.gpu_utilization() - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_stats_clone() {
        let stats = GpuBackendStats {
            gpu_evaluations: 50,
            cpu_fallbacks: 50,
            parity_violations: 2,
            avg_gpu_time_us: 3.5,
            avg_cpu_time_us: 7.0,
        };
        let cloned = stats.clone();
        assert_eq!(cloned.gpu_evaluations, 50);
        assert_eq!(cloned.parity_violations, 2);
    }

    #[test]
    fn test_config_clone() {
        let config = GpuBackendConfig::default();
        let cloned = config.clone();
        assert_eq!(cloned.max_batch_size, config.max_batch_size);
    }

    #[test]
    fn test_memory_reasonable_bounds() {
        let mem = estimate_gpu_backend_memory();
        // Should be at least 1KB
        assert!(mem >= 1024);
        // Should be under 50KB for Phase 11 budget
        assert!(mem < 50 * 1024);
    }
}

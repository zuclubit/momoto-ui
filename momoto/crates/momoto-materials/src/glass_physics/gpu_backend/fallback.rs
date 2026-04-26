//! # GPU Fallback System
//!
//! Graceful degradation to CPU when GPU is unavailable or fails.
//!
//! ## Design Principles
//!
//! 1. **Silent Fallback**: User code doesn't need to handle GPU failures
//! 2. **Logging**: Fallback reasons are logged for debugging
//! 3. **Statistics**: Track fallback frequency for performance analysis
//! 4. **Explicit Control**: Users can force CPU-only mode if needed

use std::sync::atomic::{AtomicU64, Ordering};

/// Reason for falling back to CPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackReason {
    /// GPU feature not compiled in.
    FeatureDisabled,
    /// No GPU adapter found.
    NoAdapter,
    /// GPU device creation failed.
    DeviceCreationFailed,
    /// Shader compilation failed.
    ShaderCompilationFailed,
    /// Buffer allocation failed.
    BufferAllocationFailed,
    /// Compute dispatch failed.
    DispatchFailed,
    /// Result readback failed.
    ReadbackFailed,
    /// Batch too small for GPU (CPU faster).
    BatchTooSmall,
    /// User explicitly requested CPU.
    UserRequested,
    /// Parity validation failed.
    ParityValidationFailed,
    /// Unknown error.
    Unknown,
}

impl std::fmt::Display for FallbackReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FeatureDisabled => write!(f, "GPU feature not enabled"),
            Self::NoAdapter => write!(f, "No GPU adapter found"),
            Self::DeviceCreationFailed => write!(f, "GPU device creation failed"),
            Self::ShaderCompilationFailed => write!(f, "Shader compilation failed"),
            Self::BufferAllocationFailed => write!(f, "GPU buffer allocation failed"),
            Self::DispatchFailed => write!(f, "Compute dispatch failed"),
            Self::ReadbackFailed => write!(f, "GPU result readback failed"),
            Self::BatchTooSmall => write!(f, "Batch too small for GPU"),
            Self::UserRequested => write!(f, "CPU requested by user"),
            Self::ParityValidationFailed => write!(f, "GPU/CPU parity validation failed"),
            Self::Unknown => write!(f, "Unknown error"),
        }
    }
}

/// Fallback statistics.
#[derive(Debug, Default)]
pub struct FallbackStats {
    /// Total evaluations.
    total_evaluations: AtomicU64,
    /// GPU evaluations.
    gpu_evaluations: AtomicU64,
    /// CPU fallback evaluations.
    cpu_fallbacks: AtomicU64,
    /// Fallbacks by reason.
    feature_disabled: AtomicU64,
    no_adapter: AtomicU64,
    device_failed: AtomicU64,
    shader_failed: AtomicU64,
    buffer_failed: AtomicU64,
    dispatch_failed: AtomicU64,
    readback_failed: AtomicU64,
    batch_too_small: AtomicU64,
    user_requested: AtomicU64,
    parity_failed: AtomicU64,
    unknown: AtomicU64,
}

impl FallbackStats {
    /// Create new stats.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a GPU evaluation.
    pub fn record_gpu(&self) {
        self.total_evaluations.fetch_add(1, Ordering::Relaxed);
        self.gpu_evaluations.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a CPU fallback with reason.
    pub fn record_fallback(&self, reason: FallbackReason) {
        self.total_evaluations.fetch_add(1, Ordering::Relaxed);
        self.cpu_fallbacks.fetch_add(1, Ordering::Relaxed);

        match reason {
            FallbackReason::FeatureDisabled => {
                self.feature_disabled.fetch_add(1, Ordering::Relaxed);
            }
            FallbackReason::NoAdapter => {
                self.no_adapter.fetch_add(1, Ordering::Relaxed);
            }
            FallbackReason::DeviceCreationFailed => {
                self.device_failed.fetch_add(1, Ordering::Relaxed);
            }
            FallbackReason::ShaderCompilationFailed => {
                self.shader_failed.fetch_add(1, Ordering::Relaxed);
            }
            FallbackReason::BufferAllocationFailed => {
                self.buffer_failed.fetch_add(1, Ordering::Relaxed);
            }
            FallbackReason::DispatchFailed => {
                self.dispatch_failed.fetch_add(1, Ordering::Relaxed);
            }
            FallbackReason::ReadbackFailed => {
                self.readback_failed.fetch_add(1, Ordering::Relaxed);
            }
            FallbackReason::BatchTooSmall => {
                self.batch_too_small.fetch_add(1, Ordering::Relaxed);
            }
            FallbackReason::UserRequested => {
                self.user_requested.fetch_add(1, Ordering::Relaxed);
            }
            FallbackReason::ParityValidationFailed => {
                self.parity_failed.fetch_add(1, Ordering::Relaxed);
            }
            FallbackReason::Unknown => {
                self.unknown.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Get total evaluations.
    pub fn total(&self) -> u64 {
        self.total_evaluations.load(Ordering::Relaxed)
    }

    /// Get GPU evaluations.
    pub fn gpu(&self) -> u64 {
        self.gpu_evaluations.load(Ordering::Relaxed)
    }

    /// Get CPU fallbacks.
    pub fn fallbacks(&self) -> u64 {
        self.cpu_fallbacks.load(Ordering::Relaxed)
    }

    /// Get GPU utilization percentage.
    pub fn gpu_utilization(&self) -> f64 {
        let total = self.total();
        if total == 0 {
            0.0
        } else {
            (self.gpu() as f64 / total as f64) * 100.0
        }
    }

    /// Generate report string.
    pub fn report(&self) -> String {
        let total = self.total();
        let gpu = self.gpu();
        let fallbacks = self.fallbacks();

        format!(
            "GPU Fallback Statistics:\n\
             - Total evaluations: {}\n\
             - GPU evaluations: {} ({:.1}%)\n\
             - CPU fallbacks: {} ({:.1}%)\n\
             \n\
             Fallback reasons:\n\
             - Feature disabled: {}\n\
             - No adapter: {}\n\
             - Device failed: {}\n\
             - Shader failed: {}\n\
             - Buffer failed: {}\n\
             - Dispatch failed: {}\n\
             - Readback failed: {}\n\
             - Batch too small: {}\n\
             - User requested: {}\n\
             - Parity failed: {}\n\
             - Unknown: {}",
            total,
            gpu,
            if total > 0 {
                (gpu as f64 / total as f64) * 100.0
            } else {
                0.0
            },
            fallbacks,
            if total > 0 {
                (fallbacks as f64 / total as f64) * 100.0
            } else {
                0.0
            },
            self.feature_disabled.load(Ordering::Relaxed),
            self.no_adapter.load(Ordering::Relaxed),
            self.device_failed.load(Ordering::Relaxed),
            self.shader_failed.load(Ordering::Relaxed),
            self.buffer_failed.load(Ordering::Relaxed),
            self.dispatch_failed.load(Ordering::Relaxed),
            self.readback_failed.load(Ordering::Relaxed),
            self.batch_too_small.load(Ordering::Relaxed),
            self.user_requested.load(Ordering::Relaxed),
            self.parity_failed.load(Ordering::Relaxed),
            self.unknown.load(Ordering::Relaxed),
        )
    }
}

/// Trait for types that can automatically fallback to CPU.
pub trait AutoFallback {
    /// Output type.
    type Output;

    /// Evaluate with automatic fallback.
    fn evaluate_with_fallback(&mut self) -> (Self::Output, Option<FallbackReason>);

    /// Force CPU evaluation.
    fn evaluate_cpu(&mut self) -> Self::Output;

    /// Check if GPU is available.
    fn is_gpu_available(&self) -> bool;
}

/// Fallback policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackPolicy {
    /// Always try GPU first, fall back to CPU on failure.
    Automatic,
    /// Force GPU (error if unavailable).
    GpuOnly,
    /// Force CPU (never use GPU).
    CpuOnly,
    /// Try GPU but validate with CPU (slow, for debugging).
    Validated,
}

impl Default for FallbackPolicy {
    fn default() -> Self {
        Self::Automatic
    }
}

/// Configuration for fallback behavior.
#[derive(Debug, Clone)]
pub struct FallbackConfig {
    /// Fallback policy.
    pub policy: FallbackPolicy,
    /// Minimum batch size for GPU (smaller batches use CPU).
    pub min_gpu_batch_size: usize,
    /// Enable logging of fallbacks.
    pub log_fallbacks: bool,
    /// Maximum consecutive failures before disabling GPU.
    pub max_consecutive_failures: usize,
}

impl Default for FallbackConfig {
    fn default() -> Self {
        Self {
            policy: FallbackPolicy::Automatic,
            min_gpu_batch_size: 256,
            log_fallbacks: false,
            max_consecutive_failures: 3,
        }
    }
}

/// Estimate memory for fallback infrastructure.
pub fn estimate_fallback_memory() -> usize {
    // Stats structure
    let stats = std::mem::size_of::<FallbackStats>();

    // Config
    let config = std::mem::size_of::<FallbackConfig>();

    stats + config
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_reason_display() {
        assert!(!FallbackReason::NoAdapter.to_string().is_empty());
        assert!(!FallbackReason::BatchTooSmall.to_string().is_empty());
    }

    #[test]
    fn test_stats_recording() {
        let stats = FallbackStats::new();

        stats.record_gpu();
        stats.record_gpu();
        stats.record_fallback(FallbackReason::BatchTooSmall);

        assert_eq!(stats.total(), 3);
        assert_eq!(stats.gpu(), 2);
        assert_eq!(stats.fallbacks(), 1);
    }

    #[test]
    fn test_gpu_utilization() {
        let stats = FallbackStats::new();

        // Empty stats
        assert!((stats.gpu_utilization() - 0.0).abs() < 0.01);

        // 50% utilization
        stats.record_gpu();
        stats.record_fallback(FallbackReason::BatchTooSmall);
        assert!((stats.gpu_utilization() - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_default_config() {
        let config = FallbackConfig::default();
        assert_eq!(config.policy, FallbackPolicy::Automatic);
        assert_eq!(config.min_gpu_batch_size, 256);
    }

    #[test]
    fn test_memory_estimate() {
        let mem = estimate_fallback_memory();
        assert!(mem > 0);
        assert!(mem < 5 * 1024); // Should be under 5KB
    }
}

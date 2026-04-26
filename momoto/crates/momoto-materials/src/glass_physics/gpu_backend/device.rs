//! # GPU Device Management
//!
//! WebGPU device initialization and capability detection.
//!
//! This module handles adapter selection, device creation, and capability
//! detection for GPU-accelerated BSDF evaluation.

use std::sync::Arc;

#[cfg(feature = "gpu")]
use wgpu;

/// GPU context holding the device and queue.
#[cfg(feature = "gpu")]
pub struct GpuContext {
    /// wgpu device for GPU operations.
    pub device: Arc<wgpu::Device>,
    /// Command queue for GPU submissions.
    pub queue: Arc<wgpu::Queue>,
    /// Detected GPU capabilities.
    pub capabilities: GpuCapabilities,
    /// Configuration used to create this context.
    pub config: GpuContextConfig,
}

/// Configuration for GPU context creation.
#[derive(Debug, Clone)]
pub struct GpuContextConfig {
    /// Preferred GPU backend (Vulkan, Metal, DX12, WebGPU).
    pub preferred_backend: Option<GpuBackend>,
    /// Require high-performance adapter (discrete GPU).
    pub high_performance: bool,
    /// Enable debug validation layers.
    pub debug: bool,
    /// Maximum buffer size in bytes.
    pub max_buffer_size: u64,
}

impl Default for GpuContextConfig {
    fn default() -> Self {
        Self {
            preferred_backend: None,
            high_performance: true,
            debug: cfg!(debug_assertions),
            max_buffer_size: 256 * 1024 * 1024, // 256 MB
        }
    }
}

/// GPU backend type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuBackend {
    /// Vulkan (Linux, Windows, Android).
    Vulkan,
    /// Metal (macOS, iOS).
    Metal,
    /// DirectX 12 (Windows).
    Dx12,
    /// WebGPU (Web browsers).
    WebGpu,
    /// OpenGL (fallback).
    Gl,
}

/// Detected GPU capabilities.
#[derive(Debug, Clone)]
pub struct GpuCapabilities {
    /// GPU vendor name.
    pub vendor: String,
    /// GPU device name.
    pub device_name: String,
    /// Backend in use.
    pub backend: GpuBackend,
    /// Maximum compute workgroup size.
    pub max_workgroup_size: u32,
    /// Maximum compute workgroups per dimension.
    pub max_workgroups_per_dimension: u32,
    /// Maximum storage buffer size.
    pub max_storage_buffer_size: u64,
    /// Supports f16 (half-precision) in shaders.
    pub supports_f16: bool,
    /// Supports timestamp queries for profiling.
    pub supports_timestamps: bool,
}

impl Default for GpuCapabilities {
    fn default() -> Self {
        Self {
            vendor: "Unknown".to_string(),
            device_name: "Unknown".to_string(),
            backend: GpuBackend::WebGpu,
            max_workgroup_size: 256,
            max_workgroups_per_dimension: 65535,
            max_storage_buffer_size: 128 * 1024 * 1024,
            supports_f16: false,
            supports_timestamps: false,
        }
    }
}

/// Error during GPU context creation.
#[derive(Debug, Clone)]
pub enum GpuContextError {
    /// No suitable GPU adapter found.
    NoAdapter,
    /// Device creation failed.
    DeviceCreation(String),
    /// Required feature not supported.
    UnsupportedFeature(String),
    /// Backend not available on this platform.
    BackendUnavailable(GpuBackend),
}

impl std::fmt::Display for GpuContextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoAdapter => write!(f, "No suitable GPU adapter found"),
            Self::DeviceCreation(msg) => write!(f, "GPU device creation failed: {}", msg),
            Self::UnsupportedFeature(feature) => {
                write!(f, "Required GPU feature not supported: {}", feature)
            }
            Self::BackendUnavailable(backend) => {
                write!(
                    f,
                    "GPU backend {:?} not available on this platform",
                    backend
                )
            }
        }
    }
}

impl std::error::Error for GpuContextError {}

#[cfg(feature = "gpu")]
impl GpuContext {
    /// Create a new GPU context with default configuration.
    ///
    /// # Errors
    ///
    /// Returns error if no suitable GPU is found or device creation fails.
    pub async fn new() -> Result<Self, GpuContextError> {
        Self::with_config(GpuContextConfig::default()).await
    }

    /// Create a GPU context with custom configuration.
    ///
    /// # Errors
    ///
    /// Returns error if no suitable GPU is found or device creation fails.
    pub async fn with_config(config: GpuContextConfig) -> Result<Self, GpuContextError> {
        // Create wgpu instance
        let backends = if let Some(backend) = config.preferred_backend {
            match backend {
                GpuBackend::Vulkan => wgpu::Backends::VULKAN,
                GpuBackend::Metal => wgpu::Backends::METAL,
                GpuBackend::Dx12 => wgpu::Backends::DX12,
                GpuBackend::WebGpu => wgpu::Backends::BROWSER_WEBGPU,
                GpuBackend::Gl => wgpu::Backends::GL,
            }
        } else {
            wgpu::Backends::all()
        };

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends,
            dx12_shader_compiler: wgpu::Dx12Compiler::default(),
            flags: if config.debug {
                wgpu::InstanceFlags::debugging()
            } else {
                wgpu::InstanceFlags::empty()
            },
            gles_minor_version: wgpu::Gles3MinorVersion::default(),
        });

        // Request adapter
        let power_preference = if config.high_performance {
            wgpu::PowerPreference::HighPerformance
        } else {
            wgpu::PowerPreference::LowPower
        };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or(GpuContextError::NoAdapter)?;

        // Get adapter info
        let info = adapter.get_info();
        let limits = adapter.limits();

        // Determine backend
        let backend = match info.backend {
            wgpu::Backend::Vulkan => GpuBackend::Vulkan,
            wgpu::Backend::Metal => GpuBackend::Metal,
            wgpu::Backend::Dx12 => GpuBackend::Dx12,
            wgpu::Backend::BrowserWebGpu => GpuBackend::WebGpu,
            wgpu::Backend::Gl => GpuBackend::Gl,
            _ => GpuBackend::WebGpu,
        };

        // Build capabilities
        let capabilities = GpuCapabilities {
            vendor: info.vendor.to_string(),
            device_name: info.name.clone(),
            backend,
            max_workgroup_size: limits.max_compute_workgroup_size_x,
            max_workgroups_per_dimension: limits.max_compute_workgroups_per_dimension,
            max_storage_buffer_size: limits.max_storage_buffer_binding_size as u64,
            supports_f16: adapter.features().contains(wgpu::Features::SHADER_F16),
            supports_timestamps: adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY),
        };

        // Request device with required features
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Momoto PBR GPU Context"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits {
                        max_storage_buffer_binding_size: config.max_buffer_size as u32,
                        ..wgpu::Limits::default()
                    },
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .map_err(|e| GpuContextError::DeviceCreation(e.to_string()))?;

        Ok(Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
            capabilities,
            config,
        })
    }

    /// Check if GPU context is valid and ready.
    pub fn is_ready(&self) -> bool {
        // Device exists means ready
        true
    }

    /// Get maximum batch size for this GPU.
    pub fn max_batch_size(&self) -> usize {
        // Calculate based on buffer limits and material struct size
        let material_size = 64; // ~64 bytes per material input
        let max_materials = self.capabilities.max_storage_buffer_size as usize / material_size;
        max_materials.min(self.config.max_buffer_size as usize / material_size)
    }
}

/// Check if GPU is available on the current platform.
#[cfg(feature = "gpu")]
pub fn is_gpu_available() -> bool {
    // This is a sync check - actual availability requires async adapter request
    #[cfg(target_arch = "wasm32")]
    {
        // Check if WebGPU is available in browser
        // In WASM, we need to check navigator.gpu
        true // Assume available, actual check happens in async context
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        // On native, wgpu supports Vulkan/Metal/DX12
        true
    }
}

#[cfg(not(feature = "gpu"))]
pub fn is_gpu_available() -> bool {
    false
}

/// Estimate memory for GPU context.
pub fn estimate_device_memory() -> usize {
    // Device + queue overhead
    let device_overhead = 1024; // 1 KB

    // Capabilities struct
    let capabilities = 256; // 256 bytes

    device_overhead + capabilities
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GpuContextConfig::default();
        assert!(config.preferred_backend.is_none());
        assert!(config.high_performance);
        assert_eq!(config.max_buffer_size, 256 * 1024 * 1024);
    }

    #[test]
    fn test_capabilities_default() {
        let caps = GpuCapabilities::default();
        assert_eq!(caps.max_workgroup_size, 256);
        assert_eq!(caps.max_workgroups_per_dimension, 65535);
    }

    #[test]
    fn test_error_display() {
        let err = GpuContextError::NoAdapter;
        assert!(err.to_string().contains("No suitable GPU"));

        let err = GpuContextError::DeviceCreation("test".to_string());
        assert!(err.to_string().contains("test"));
    }

    #[test]
    fn test_memory_estimate() {
        let mem = estimate_device_memory();
        assert!(mem > 0);
        assert!(mem < 10 * 1024); // Should be under 10KB
    }

    #[cfg(not(feature = "gpu"))]
    #[test]
    fn test_gpu_not_available_without_feature() {
        assert!(!is_gpu_available());
    }
}

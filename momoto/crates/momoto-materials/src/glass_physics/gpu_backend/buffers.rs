//! # GPU Buffer Management
//!
//! Structure-of-Arrays (SOA) buffer pool for efficient GPU data transfer.
//!
//! This module mirrors the layout from `simd_batch.rs` to ensure compatibility
//! between CPU and GPU evaluation paths.

use std::collections::HashMap;

#[cfg(feature = "gpu")]
use std::sync::Arc;
#[cfg(feature = "gpu")]
use wgpu;

/// Buffer pool for reusing GPU buffers.
#[cfg(feature = "gpu")]
pub struct BufferPool {
    /// Device reference for buffer creation.
    device: Arc<wgpu::Device>,
    /// Cached material input buffers by size.
    material_buffers: HashMap<usize, wgpu::Buffer>,
    /// Cached response output buffers by size.
    response_buffers: HashMap<usize, wgpu::Buffer>,
    /// Staging buffers for readback.
    staging_buffers: HashMap<usize, wgpu::Buffer>,
    /// Maximum buffer size in bytes.
    max_buffer_size: u64,
    /// Statistics.
    stats: BufferPoolStats,
}

/// Buffer pool statistics.
#[derive(Debug, Clone, Default)]
pub struct BufferPoolStats {
    /// Total buffers allocated.
    pub allocations: u64,
    /// Total buffer reuses.
    pub reuses: u64,
    /// Current memory usage in bytes.
    pub memory_bytes: u64,
    /// Peak memory usage in bytes.
    pub peak_memory_bytes: u64,
}

#[cfg(feature = "gpu")]
impl BufferPool {
    /// Create a new buffer pool.
    pub fn new(device: Arc<wgpu::Device>, max_buffer_size: u64) -> Self {
        Self {
            device,
            material_buffers: HashMap::new(),
            response_buffers: HashMap::new(),
            staging_buffers: HashMap::new(),
            max_buffer_size,
            stats: BufferPoolStats::default(),
        }
    }

    /// Get or create a material input buffer.
    pub fn get_material_buffer(&mut self, count: usize) -> &wgpu::Buffer {
        let size = count * std::mem::size_of::<MaterialGpuData>();
        self.material_buffers.entry(count).or_insert_with(|| {
            self.stats.allocations += 1;
            self.stats.memory_bytes += size as u64;
            self.stats.peak_memory_bytes =
                self.stats.peak_memory_bytes.max(self.stats.memory_bytes);
            self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("Material Buffer ({})", count)),
                size: size as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        })
    }

    /// Get or create a response output buffer.
    pub fn get_response_buffer(&mut self, count: usize) -> &wgpu::Buffer {
        let size = count * std::mem::size_of::<ResponseGpuData>();
        self.response_buffers.entry(count).or_insert_with(|| {
            self.stats.allocations += 1;
            self.stats.memory_bytes += size as u64;
            self.stats.peak_memory_bytes =
                self.stats.peak_memory_bytes.max(self.stats.memory_bytes);
            self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("Response Buffer ({})", count)),
                size: size as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            })
        })
    }

    /// Get or create a staging buffer for readback.
    pub fn get_staging_buffer(&mut self, count: usize) -> &wgpu::Buffer {
        let size = count * std::mem::size_of::<ResponseGpuData>();
        self.staging_buffers.entry(count).or_insert_with(|| {
            self.stats.allocations += 1;
            self.stats.memory_bytes += size as u64;
            self.stats.peak_memory_bytes =
                self.stats.peak_memory_bytes.max(self.stats.memory_bytes);
            self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("Staging Buffer ({})", count)),
                size: size as u64,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        })
    }

    /// Clear all cached buffers.
    pub fn clear(&mut self) {
        self.material_buffers.clear();
        self.response_buffers.clear();
        self.staging_buffers.clear();
        self.stats.memory_bytes = 0;
    }

    /// Get pool statistics.
    pub fn stats(&self) -> &BufferPoolStats {
        &self.stats
    }
}

// ============================================================================
// GPU DATA STRUCTURES (SOA layout matching simd_batch.rs)
// ============================================================================

/// Material input data for GPU (32-bit floats for GPU efficiency).
///
/// Matches the layout of `SimdBatchInput` from simd_batch.rs.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, bytemuck_derive::Pod, bytemuck_derive::Zeroable)]
#[cfg_attr(feature = "gpu", derive())]
pub struct MaterialGpuData {
    /// Index of Refraction.
    pub ior: f32,
    /// Cosine of incident angle.
    pub cos_theta: f32,
    /// Absorption coefficient.
    pub absorption: f32,
    /// Material thickness.
    pub thickness: f32,
    /// Scattering asymmetry parameter (Henyey-Greenstein g).
    pub g: f32,
    /// Roughness (GGX alpha).
    pub roughness: f32,
    /// Metallic factor (0 = dielectric, 1 = conductor).
    pub metallic: f32,
    /// Complex IOR imaginary part (extinction coefficient k).
    pub k: f32,
}

/// BSDF response data from GPU (32-bit floats).
///
/// Matches `BSDFResponse` from unified_bsdf.rs.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, bytemuck_derive::Pod, bytemuck_derive::Zeroable)]
#[cfg_attr(feature = "gpu", derive())]
pub struct ResponseGpuData {
    /// Reflectance (R channel).
    pub reflectance_r: f32,
    /// Reflectance (G channel).
    pub reflectance_g: f32,
    /// Reflectance (B channel).
    pub reflectance_b: f32,
    /// Transmittance (R channel).
    pub transmittance_r: f32,
    /// Transmittance (G channel).
    pub transmittance_g: f32,
    /// Transmittance (B channel).
    pub transmittance_b: f32,
    /// Absorption (computed as 1 - R - T for each channel).
    pub absorption_r: f32,
    pub absorption_g: f32,
}

impl MaterialGpuData {
    /// Create from CPU batch input at index.
    pub fn from_batch_input(
        ior: f64,
        cos_theta: f64,
        absorption: f64,
        thickness: f64,
        g: f64,
        roughness: f64,
        metallic: f64,
        k: f64,
    ) -> Self {
        Self {
            ior: ior as f32,
            cos_theta: cos_theta as f32,
            absorption: absorption as f32,
            thickness: thickness as f32,
            g: g as f32,
            roughness: roughness as f32,
            metallic: metallic as f32,
            k: k as f32,
        }
    }
}

impl ResponseGpuData {
    /// Convert to CPU f64 values.
    pub fn to_cpu_response(&self) -> (f64, f64, f64, f64, f64, f64) {
        (
            self.reflectance_r as f64,
            self.reflectance_g as f64,
            self.reflectance_b as f64,
            self.transmittance_r as f64,
            self.transmittance_g as f64,
            self.transmittance_b as f64,
        )
    }
}

/// Material buffer wrapper for type-safe GPU uploads.
#[cfg(feature = "gpu")]
pub struct MaterialBuffer {
    /// Underlying wgpu buffer.
    buffer: wgpu::Buffer,
    /// Number of materials in buffer.
    count: usize,
}

#[cfg(feature = "gpu")]
impl MaterialBuffer {
    /// Create a new material buffer.
    pub fn new(device: &wgpu::Device, materials: &[MaterialGpuData]) -> Self {
        use wgpu::util::DeviceExt;
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Material Buffer"),
            contents: bytemuck::cast_slice(materials),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        Self {
            buffer,
            count: materials.len(),
        }
    }

    /// Get the underlying buffer.
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Get material count.
    pub fn count(&self) -> usize {
        self.count
    }
}

/// Response buffer wrapper for type-safe GPU readback.
#[cfg(feature = "gpu")]
pub struct ResponseBuffer {
    /// Underlying wgpu buffer (storage).
    buffer: wgpu::Buffer,
    /// Staging buffer for readback.
    staging: wgpu::Buffer,
    /// Number of responses in buffer.
    count: usize,
}

#[cfg(feature = "gpu")]
impl ResponseBuffer {
    /// Create a new response buffer.
    pub fn new(device: &wgpu::Device, count: usize) -> Self {
        let size = (count * std::mem::size_of::<ResponseGpuData>()) as u64;

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Response Buffer"),
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Response Staging Buffer"),
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            buffer,
            staging,
            count,
        }
    }

    /// Get the storage buffer.
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Get the staging buffer for readback.
    pub fn staging(&self) -> &wgpu::Buffer {
        &self.staging
    }

    /// Get response count.
    pub fn count(&self) -> usize {
        self.count
    }
}

/// Estimate memory for buffer infrastructure.
pub fn estimate_buffer_memory() -> usize {
    // Buffer pool overhead
    let pool_overhead = 2 * 1024; // 2 KB

    // Typical cached buffers
    let cached_buffers = 4 * 1024; // 4 KB

    pool_overhead + cached_buffers
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_gpu_data_size() {
        // Verify struct is tightly packed for GPU
        assert_eq!(std::mem::size_of::<MaterialGpuData>(), 32);
    }

    #[test]
    fn test_response_gpu_data_size() {
        // Verify struct is tightly packed for GPU
        assert_eq!(std::mem::size_of::<ResponseGpuData>(), 32);
    }

    #[test]
    fn test_material_conversion() {
        let mat = MaterialGpuData::from_batch_input(1.5, 0.7, 0.1, 10.0, 0.8, 0.2, 0.0, 0.0);
        assert!((mat.ior - 1.5).abs() < 0.001);
        assert!((mat.cos_theta - 0.7).abs() < 0.001);
    }

    #[test]
    fn test_response_conversion() {
        let resp = ResponseGpuData {
            reflectance_r: 0.5,
            reflectance_g: 0.4,
            reflectance_b: 0.3,
            transmittance_r: 0.3,
            transmittance_g: 0.4,
            transmittance_b: 0.5,
            absorption_r: 0.2,
            absorption_g: 0.2,
        };
        let (rr, rg, rb, tr, tg, tb) = resp.to_cpu_response();
        assert!((rr - 0.5).abs() < 0.001);
        assert!((tb - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_buffer_pool_stats() {
        let stats = BufferPoolStats::default();
        assert_eq!(stats.allocations, 0);
        assert_eq!(stats.memory_bytes, 0);
    }

    #[test]
    fn test_memory_estimate() {
        let mem = estimate_buffer_memory();
        assert!(mem > 0);
        assert!(mem < 20 * 1024); // Should be under 20KB
    }
}

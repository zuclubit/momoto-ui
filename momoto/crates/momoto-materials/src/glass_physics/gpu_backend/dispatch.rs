//! # GPU Dispatch
//!
//! Workgroup dispatch and batch evaluation on GPU.

#[cfg(feature = "gpu")]
use std::sync::Arc;

#[cfg(feature = "gpu")]
use wgpu;

use super::buffers::{MaterialGpuData, ResponseGpuData};
#[cfg(feature = "gpu")]
use super::device::GpuContext;
#[cfg(feature = "gpu")]
use super::pipelines::{ComputePipelineCache, PipelineType};

/// Configuration for GPU dispatch.
#[derive(Debug, Clone)]
pub struct GpuDispatchConfig {
    /// Workgroup size (threads per workgroup).
    pub workgroup_size: u32,
    /// Maximum batch size per dispatch.
    pub max_batch_size: usize,
    /// Enable async dispatch.
    pub async_dispatch: bool,
    /// Pipeline type to use.
    pub pipeline_type: PipelineType,
}

impl Default for GpuDispatchConfig {
    fn default() -> Self {
        Self {
            workgroup_size: 256,
            max_batch_size: 65536,
            async_dispatch: true,
            pipeline_type: PipelineType::Unified,
        }
    }
}

/// Result of GPU batch evaluation.
#[derive(Debug, Clone)]
pub struct GpuBatchResult {
    /// Reflectance R channel.
    pub reflectance_r: Vec<f64>,
    /// Reflectance G channel.
    pub reflectance_g: Vec<f64>,
    /// Reflectance B channel.
    pub reflectance_b: Vec<f64>,
    /// Transmittance R channel.
    pub transmittance_r: Vec<f64>,
    /// Transmittance G channel.
    pub transmittance_g: Vec<f64>,
    /// Transmittance B channel.
    pub transmittance_b: Vec<f64>,
    /// Number of materials evaluated.
    pub count: usize,
    /// Evaluation time in microseconds.
    pub time_us: f64,
    /// Whether GPU was actually used (vs CPU fallback).
    pub used_gpu: bool,
}

impl GpuBatchResult {
    /// Create empty result.
    pub fn empty() -> Self {
        Self {
            reflectance_r: Vec::new(),
            reflectance_g: Vec::new(),
            reflectance_b: Vec::new(),
            transmittance_r: Vec::new(),
            transmittance_g: Vec::new(),
            transmittance_b: Vec::new(),
            count: 0,
            time_us: 0.0,
            used_gpu: false,
        }
    }

    /// Create from GPU response data.
    pub fn from_gpu_responses(responses: &[ResponseGpuData], time_us: f64) -> Self {
        let count = responses.len();
        let mut result = Self {
            reflectance_r: Vec::with_capacity(count),
            reflectance_g: Vec::with_capacity(count),
            reflectance_b: Vec::with_capacity(count),
            transmittance_r: Vec::with_capacity(count),
            transmittance_g: Vec::with_capacity(count),
            transmittance_b: Vec::with_capacity(count),
            count,
            time_us,
            used_gpu: true,
        };

        for resp in responses {
            result.reflectance_r.push(resp.reflectance_r as f64);
            result.reflectance_g.push(resp.reflectance_g as f64);
            result.reflectance_b.push(resp.reflectance_b as f64);
            result.transmittance_r.push(resp.transmittance_r as f64);
            result.transmittance_g.push(resp.transmittance_g as f64);
            result.transmittance_b.push(resp.transmittance_b as f64);
        }

        result
    }
}

/// GPU batch evaluator with CPU fallback.
#[cfg(feature = "gpu")]
pub struct GpuBatchEvaluator {
    /// GPU context.
    context: Arc<GpuContext>,
    /// Pipeline cache.
    pipeline_cache: ComputePipelineCache,
    /// Dispatch configuration.
    config: GpuDispatchConfig,
    /// CPU fallback evaluator.
    cpu_fallback: super::super::simd_batch::SimdBatchEvaluator,
    /// Statistics.
    stats: GpuEvaluatorStats,
}

/// GPU evaluator statistics.
#[derive(Debug, Clone, Default)]
pub struct GpuEvaluatorStats {
    /// Total GPU dispatches.
    pub gpu_dispatches: u64,
    /// Total CPU fallbacks.
    pub cpu_fallbacks: u64,
    /// Total materials evaluated on GPU.
    pub gpu_materials: u64,
    /// Total materials evaluated on CPU.
    pub cpu_materials: u64,
    /// Average GPU time per material (microseconds).
    pub avg_gpu_time_per_material: f64,
}

#[cfg(feature = "gpu")]
impl GpuBatchEvaluator {
    /// Create a new GPU batch evaluator.
    pub fn new(context: Arc<GpuContext>) -> Self {
        let device = context.device.clone();
        Self {
            context,
            pipeline_cache: ComputePipelineCache::new(device),
            config: GpuDispatchConfig::default(),
            cpu_fallback: super::super::simd_batch::SimdBatchEvaluator::new(
                super::super::simd_batch::SimdConfig::default(),
            ),
            stats: GpuEvaluatorStats::default(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(context: Arc<GpuContext>, config: GpuDispatchConfig) -> Self {
        let device = context.device.clone();
        Self {
            context,
            pipeline_cache: ComputePipelineCache::new(device),
            config,
            cpu_fallback: super::super::simd_batch::SimdBatchEvaluator::new(
                super::super::simd_batch::SimdConfig::default(),
            ),
            stats: GpuEvaluatorStats::default(),
        }
    }

    /// Check if GPU is active.
    pub fn is_gpu_active(&self) -> bool {
        self.context.is_ready()
    }

    /// Evaluate batch of materials.
    pub fn evaluate(&mut self, materials: &[MaterialGpuData]) -> GpuBatchResult {
        let count = materials.len();

        if count == 0 {
            return GpuBatchResult::empty();
        }

        // For small batches, prefer CPU
        if count < 256 {
            self.stats.cpu_fallbacks += 1;
            self.stats.cpu_materials += count as u64;
            return self.evaluate_cpu_fallback(materials);
        }

        // GPU evaluation
        let start = std::time::Instant::now();
        let result = self.dispatch_gpu(materials);
        let elapsed = start.elapsed().as_micros() as f64;

        self.stats.gpu_dispatches += 1;
        self.stats.gpu_materials += count as u64;
        self.stats.avg_gpu_time_per_material = (self.stats.avg_gpu_time_per_material
            * (self.stats.gpu_dispatches - 1) as f64
            + elapsed / count as f64)
            / self.stats.gpu_dispatches as f64;

        result
    }

    /// Dispatch compute shader on GPU.
    fn dispatch_gpu(&mut self, materials: &[MaterialGpuData]) -> GpuBatchResult {
        use wgpu::util::DeviceExt;

        let count = materials.len();
        let device = &self.context.device;
        let queue = &self.context.queue;

        // Create input buffer
        let material_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Material Input Buffer"),
            contents: bytemuck::cast_slice(materials),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Create output buffer
        let response_size = (count * std::mem::size_of::<ResponseGpuData>()) as u64;
        let response_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Response Output Buffer"),
            size: response_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create staging buffer for readback
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging Buffer"),
            size: response_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create uniform buffer for params
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct GpuParams {
            count: u32,
            wavelength: f32,
            padding0: f32,
            padding1: f32,
        }

        let params = GpuParams {
            count: count as u32,
            wavelength: 550.0, // Default green wavelength
            padding0: 0.0,
            padding1: 0.0,
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Get or create pipeline
        let pipeline = self.pipeline_cache.get_pipeline(self.config.pipeline_type);
        let bind_group_layout = self
            .pipeline_cache
            .get_bind_group_layout(self.config.pipeline_type);

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("BSDF Bind Group"),
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: material_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: response_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        // Create command encoder
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("BSDF Compute Encoder"),
        });

        // Dispatch compute
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("BSDF Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            let workgroups =
                (count as u32 + self.config.workgroup_size - 1) / self.config.workgroup_size;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // Copy to staging buffer
        encoder.copy_buffer_to_buffer(&response_buffer, 0, &staging_buffer, 0, response_size);

        // Submit
        queue.submit(std::iter::once(encoder.finish()));

        // Read back results
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });

        device.poll(wgpu::Maintain::Wait);

        match receiver.recv() {
            Ok(Ok(())) => {
                let data = buffer_slice.get_mapped_range();
                let responses: &[ResponseGpuData] = bytemuck::cast_slice(&data);
                let result = GpuBatchResult::from_gpu_responses(responses, 0.0);
                drop(data);
                staging_buffer.unmap();
                result
            }
            _ => {
                // Fallback to CPU on GPU error
                self.evaluate_cpu_fallback(materials)
            }
        }
    }

    /// CPU fallback evaluation.
    fn evaluate_cpu_fallback(&self, materials: &[MaterialGpuData]) -> GpuBatchResult {
        let count = materials.len();
        let mut result = GpuBatchResult {
            reflectance_r: Vec::with_capacity(count),
            reflectance_g: Vec::with_capacity(count),
            reflectance_b: Vec::with_capacity(count),
            transmittance_r: Vec::with_capacity(count),
            transmittance_g: Vec::with_capacity(count),
            transmittance_b: Vec::with_capacity(count),
            count,
            time_us: 0.0,
            used_gpu: false,
        };

        // Simple CPU evaluation (matches GPU logic)
        for mat in materials {
            let cos_theta = mat.cos_theta.abs() as f64;
            let f0 = ((mat.ior - 1.0) / (mat.ior + 1.0)).powi(2) as f64;
            let one_minus_cos = 1.0 - cos_theta;
            let pow5 = one_minus_cos.powi(5);
            let reflectance = f0 + (1.0 - f0) * pow5;

            let path = mat.thickness as f64 / cos_theta.max(1e-7);
            let transmittance = (1.0 - reflectance) * (-mat.absorption as f64 * path).exp();

            result.reflectance_r.push(reflectance.clamp(0.0, 1.0));
            result.reflectance_g.push(reflectance.clamp(0.0, 1.0));
            result.reflectance_b.push(reflectance.clamp(0.0, 1.0));
            result.transmittance_r.push(transmittance.clamp(0.0, 1.0));
            result.transmittance_g.push(transmittance.clamp(0.0, 1.0));
            result.transmittance_b.push(transmittance.clamp(0.0, 1.0));
        }

        result
    }

    /// Get statistics.
    pub fn stats(&self) -> &GpuEvaluatorStats {
        &self.stats
    }
}

/// Estimate memory for dispatch infrastructure.
pub fn estimate_dispatch_memory() -> usize {
    // Command encoder overhead
    let encoder = 1024; // 1 KB

    // Bind group overhead
    let bind_group = 512; // 512 bytes

    encoder + bind_group
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GpuDispatchConfig::default();
        assert_eq!(config.workgroup_size, 256);
        assert_eq!(config.max_batch_size, 65536);
        assert!(config.async_dispatch);
    }

    #[test]
    fn test_empty_result() {
        let result = GpuBatchResult::empty();
        assert_eq!(result.count, 0);
        assert!(!result.used_gpu);
    }

    #[test]
    fn test_from_gpu_responses() {
        let responses = vec![ResponseGpuData {
            reflectance_r: 0.5,
            reflectance_g: 0.5,
            reflectance_b: 0.5,
            transmittance_r: 0.3,
            transmittance_g: 0.3,
            transmittance_b: 0.3,
            absorption_r: 0.2,
            absorption_g: 0.2,
        }];

        let result = GpuBatchResult::from_gpu_responses(&responses, 100.0);
        assert_eq!(result.count, 1);
        assert!(result.used_gpu);
        assert!((result.reflectance_r[0] - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_memory_estimate() {
        let mem = estimate_dispatch_memory();
        assert!(mem > 0);
        assert!(mem < 10 * 1024); // Should be under 10KB
    }
}

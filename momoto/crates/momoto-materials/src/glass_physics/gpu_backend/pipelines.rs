//! # Compute Pipeline Management
//!
//! Cached compute pipelines for different BSDF types.

use std::collections::HashMap;

#[cfg(feature = "gpu")]
use std::sync::Arc;
#[cfg(feature = "gpu")]
use wgpu;

/// Type of compute pipeline for different BSDFs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PipelineType {
    /// DielectricBSDF evaluation (glass, water, crystal).
    Dielectric,
    /// ConductorBSDF evaluation (metals).
    Conductor,
    /// AnisotropicGGX evaluation (brushed metals).
    Anisotropic,
    /// ThinFilmBSDF evaluation (coatings, soap bubbles).
    ThinFilm,
    /// NeuralCorrectedBSDF inference (Phase 10 neural MLP).
    NeuralInference,
    /// Combined/Unified evaluation.
    Unified,
}

impl PipelineType {
    /// Get the WGSL shader source for this pipeline type.
    pub fn shader_source(&self) -> &'static str {
        match self {
            Self::Dielectric => include_str!("shaders/unified_bsdf.wgsl"),
            Self::Conductor => include_str!("shaders/unified_bsdf.wgsl"),
            Self::Anisotropic => include_str!("shaders/anisotropic.wgsl"),
            Self::ThinFilm => include_str!("shaders/thin_film.wgsl"),
            Self::NeuralInference => include_str!("shaders/neural_inference.wgsl"),
            Self::Unified => include_str!("shaders/unified_bsdf.wgsl"),
        }
    }

    /// Get entry point name for this pipeline.
    pub fn entry_point(&self) -> &'static str {
        match self {
            Self::Dielectric => "evaluate_dielectric",
            Self::Conductor => "evaluate_conductor",
            Self::Anisotropic => "evaluate_anisotropic",
            Self::ThinFilm => "evaluate_thin_film",
            Self::NeuralInference => "neural_forward",
            Self::Unified => "evaluate_unified",
        }
    }

    /// Get workgroup size for this pipeline.
    pub fn workgroup_size(&self) -> (u32, u32, u32) {
        // All pipelines use 256 threads per workgroup for now
        (256, 1, 1)
    }
}

/// Cached compute pipelines.
#[cfg(feature = "gpu")]
pub struct ComputePipelineCache {
    /// Device reference.
    device: Arc<wgpu::Device>,
    /// Cached pipelines by type.
    pipelines: HashMap<PipelineType, wgpu::ComputePipeline>,
    /// Cached bind group layouts.
    bind_group_layouts: HashMap<PipelineType, wgpu::BindGroupLayout>,
    /// Pipeline layout (shared).
    pipeline_layout: Option<wgpu::PipelineLayout>,
}

#[cfg(feature = "gpu")]
impl ComputePipelineCache {
    /// Create a new pipeline cache.
    pub fn new(device: Arc<wgpu::Device>) -> Self {
        Self {
            device,
            pipelines: HashMap::new(),
            bind_group_layouts: HashMap::new(),
            pipeline_layout: None,
        }
    }

    /// Get or create a compute pipeline.
    pub fn get_pipeline(&mut self, pipeline_type: PipelineType) -> &wgpu::ComputePipeline {
        if !self.pipelines.contains_key(&pipeline_type) {
            let pipeline = self.create_pipeline(pipeline_type);
            self.pipelines.insert(pipeline_type, pipeline);
        }
        self.pipelines.get(&pipeline_type).unwrap()
    }

    /// Get or create a bind group layout.
    pub fn get_bind_group_layout(&mut self, pipeline_type: PipelineType) -> &wgpu::BindGroupLayout {
        if !self.bind_group_layouts.contains_key(&pipeline_type) {
            let layout = self.create_bind_group_layout(pipeline_type);
            self.bind_group_layouts.insert(pipeline_type, layout);
        }
        self.bind_group_layouts.get(&pipeline_type).unwrap()
    }

    /// Create a compute pipeline.
    fn create_pipeline(&mut self, pipeline_type: PipelineType) -> wgpu::ComputePipeline {
        // Create shader module
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(&format!("{:?} Shader", pipeline_type)),
                source: wgpu::ShaderSource::Wgsl(pipeline_type.shader_source().into()),
            });

        // Create bind group layout
        let bind_group_layout = self.get_bind_group_layout(pipeline_type).clone();

        // Create pipeline layout
        let pipeline_layout = self.get_or_create_pipeline_layout(&bind_group_layout);

        // Create compute pipeline
        self.device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(&format!("{:?} Pipeline", pipeline_type)),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some(pipeline_type.entry_point()),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            })
    }

    /// Create a bind group layout for material/response buffers.
    fn create_bind_group_layout(&self, _pipeline_type: PipelineType) -> wgpu::BindGroupLayout {
        self.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("BSDF Bind Group Layout"),
                entries: &[
                    // Material input buffer (read-only storage)
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Response output buffer (read-write storage)
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Uniform buffer for global parameters
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            })
    }

    /// Get or create the pipeline layout.
    fn get_or_create_pipeline_layout(
        &mut self,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::PipelineLayout {
        if self.pipeline_layout.is_none() {
            self.pipeline_layout = Some(self.device.create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("BSDF Pipeline Layout"),
                    bind_group_layouts: &[bind_group_layout],
                    push_constant_ranges: &[],
                },
            ));
        }
        // Clone the layout (wgpu layouts are reference counted internally)
        self.device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("BSDF Pipeline Layout"),
                bind_group_layouts: &[bind_group_layout],
                push_constant_ranges: &[],
            })
    }

    /// Clear cached pipelines.
    pub fn clear(&mut self) {
        self.pipelines.clear();
        self.bind_group_layouts.clear();
        self.pipeline_layout = None;
    }

    /// Get number of cached pipelines.
    pub fn cached_count(&self) -> usize {
        self.pipelines.len()
    }
}

/// Estimate memory for pipeline cache.
pub fn estimate_pipeline_memory() -> usize {
    // Pipeline cache overhead
    let cache_overhead = 1024; // 1 KB

    // Compiled shader cache (per pipeline type)
    let shader_cache = 6 * 1024; // 6 KB for all pipeline types

    cache_overhead + shader_cache
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_type_entry_points() {
        assert_eq!(
            PipelineType::Dielectric.entry_point(),
            "evaluate_dielectric"
        );
        assert_eq!(PipelineType::Conductor.entry_point(), "evaluate_conductor");
        assert_eq!(
            PipelineType::Anisotropic.entry_point(),
            "evaluate_anisotropic"
        );
        assert_eq!(PipelineType::ThinFilm.entry_point(), "evaluate_thin_film");
        assert_eq!(
            PipelineType::NeuralInference.entry_point(),
            "neural_forward"
        );
    }

    #[test]
    fn test_workgroup_size() {
        let (x, y, z) = PipelineType::Unified.workgroup_size();
        assert_eq!(x, 256);
        assert_eq!(y, 1);
        assert_eq!(z, 1);
    }

    #[test]
    fn test_memory_estimate() {
        let mem = estimate_pipeline_memory();
        assert!(mem > 0);
        assert!(mem < 20 * 1024); // Should be under 20KB
    }
}

// Note: Allow unused items in research/experimental modules
// These are reserved for future implementations and advanced use cases
#![allow(dead_code, unused_imports, unused_variables)]

//! # Glass Physics Engine
//!
//! Physical simulation of light interaction with glass materials.
//!
//! This module provides the foundation for realistic glass rendering by modeling:
//! - **Transmittance**: How light passes through glass (Beer-Lambert law)
//! - **Refraction**: How light bends at glass interfaces (Snell's law)
//! - **Lighting**: How light scatters and reflects from surfaces
//!
//! ## Architecture
//!
//! The glass physics engine separates concerns:
//!
//! ```text
//! ┌─────────────────────────────────────────────────┐
//! │          Glass Physics Engine                    │
//! ├─────────────────────────────────────────────────┤
//! │  transmittance.rs  │  Light passing through     │
//! │  refraction.rs     │  Light bending             │
//! │  light_model.rs    │  Light scattering          │
//! └─────────────────────────────────────────────────┘
//!          ↓                   ↓                   ↓
//! ┌─────────────────────────────────────────────────┐
//! │       Material Layers (composition)              │
//! └─────────────────────────────────────────────────┘
//!                      ↓
//! ┌─────────────────────────────────────────────────┐
//! │          Rendering (CSS/Canvas)                  │
//! └─────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust
//! use momoto_materials::glass_physics::{
//!     transmittance::{OpticalProperties, calculate_multi_layer_transmittance},
//!     refraction_index::{RefractionParams, calculate_refraction},
//!     light_model::{LightingEnvironment, derive_gradient},
//! };
//!
//! // 1. Define glass optical properties
//! let optical = OpticalProperties {
//!     absorption_coefficient: 0.2,
//!     scattering_coefficient: 0.3,
//!     thickness: 1.5,
//!     refractive_index: 1.5,
//! };
//!
//! // 2. Calculate light transmission
//! let layers = calculate_multi_layer_transmittance(&optical, 1.0);
//! println!("Surface: {:.2}%", layers.surface * 100.0);
//! println!("Volume: {:.2}%", layers.volume * 100.0);
//! println!("Substrate: {:.2}%", layers.substrate * 100.0);
//!
//! // 3. Calculate refraction distortion
//! let refraction_params = RefractionParams::default();
//! let refraction = calculate_refraction(&refraction_params, 0.5, 0.5, 30.0);
//!
//! // 4. Generate physics-based gradient
//! let environment = LightingEnvironment::default();
//! let gradient = derive_gradient(&environment, 0.3, 32.0, 10);
//! ```
//!
//! ## Design Philosophy
//!
//! **Perceptual > Physical**: We prioritize perceptual correctness over
//! strict physical accuracy. Real glass physics involves complex phenomena
//! (Fresnel equations, dispersion, caustics) that are expensive to compute
//! and often imperceptible in UI contexts.
//!
//! Instead, we use **simplified physical models** that capture the essential
//! characteristics while remaining fast enough for real-time UI rendering.

pub mod batch; // ✅ NEW - Batch evaluation for 7-10x performance boost
pub mod blinn_phong;
pub mod complex_ior; // ✅ PBR Phase 3 - Complex IOR for metals (Gold, Silver, Copper, etc.)
pub mod context;
pub mod dhg_lut; // ✅ PBR Phase 2 - Double Henyey-Greenstein LUT
pub mod dispersion; // ✅ PBR Phase 1 - Wavelength-dependent IOR (Cauchy, Sellmeier)
pub mod enhanced_presets; // ✅ PBR Phase 1 - Enhanced material presets with new parameters
pub mod fresnel;
pub mod light_model;
pub mod lut; // ✅ NEW - Lookup Tables for 5x performance boost
pub mod mie_lut; // ✅ PBR Phase 3 - Mie scattering approximation for particles
pub mod pbr_validation; // ✅ PBR Phase 1 - Validation tests and benchmarks
pub mod perlin_noise;
pub mod phase2_validation; // ✅ PBR Phase 2 - Comparison benchmarks (DHG vs H-G, Sellmeier vs Cauchy)
pub mod phase3_validation; // ✅ PBR Phase 3 - Complex IOR, Mie, Thin-Film validation
pub mod phase4_validation; // ✅ PBR Phase 4 - Compression, Advanced TF, Temp Metals, Dynamic Mie
pub mod quality_tiers; // ✅ PBR Phase 2 - Quality tier auto-selection (+ Phase 4 extensions)
pub mod refraction_index;

// PBR Phase 4 - Advanced Features
pub mod lut_compression; // ✅ PBR Phase 4 - LUT compression for memory optimization
pub mod metal_temp; // ✅ PBR Phase 4 - Temperature-dependent metal IOR + oxidation
pub mod mie_dynamic; // ✅ PBR Phase 4 - Dynamic Mie with polydisperse/anisotropic scattering
pub mod scattering; // ✅ PBR Phase 1 - Henyey-Greenstein phase function with LUT
pub mod spectral_fresnel; // ✅ PBR Phase 1 - RGB spectral Fresnel evaluation
pub mod thin_film; // ✅ PBR Phase 3 - Thin-film interference for iridescent effects
pub mod thin_film_advanced; // ✅ PBR Phase 4 - Transfer matrix multi-layer thin-film
pub mod transmittance; // ✅ NEW - Formal context system for environment-aware evaluation

// Sprint 6 - Unified Spectral Pipeline
pub mod spectral_pipeline; // ✅ Sprint 6 - End-to-end spectral processing (no RGB intermediate)
#[cfg(test)]
mod spectral_pipeline_validation; // ✅ Sprint 6 - Physical validation tests

// Sprint 7 - Performance Optimization
pub mod spectral_cache; // ✅ Sprint 7 - Deterministic cache for ΔE=0 with O(1) lookup
pub mod spectral_gpu;
pub mod spectral_lut; // ✅ Sprint 7 - LUTs for ΔE < 1 with 10x+ speedup
pub mod spectral_optimization; // ✅ Sprint 7 - Quality tiers and adaptive spectral sampling
pub mod spectral_profiling; // ✅ Sprint 7 - Profiling and benchmarking // ✅ Sprint 7 - GPU/WebGPU batch evaluation foundation

// GGX Microfacet BRDF — Cook-Torrance + Oren-Nayar
pub mod microfacet; // ✅ FASE D - GGX NDF, Smith G2, Cook-Torrance, Oren-Nayar

// Sprint 8 - Scientific Validation
pub mod scientific_validation; // ✅ Sprint 8 - Publication-ready validation vs analytical references

// Re-export commonly used types
pub use transmittance::{
    calculate_multi_layer_transmittance, calculate_transmittance,
    GlassPresets as TransmittancePresets, LayerTransmittance, OpticalProperties,
    TransmittanceResult,
};

pub use refraction_index::{
    apply_refraction_to_color, calculate_refraction, generate_distortion_map, RefractionParams,
    RefractionPresets, RefractionResult,
};

pub use light_model::{
    calculate_lighting, derive_gradient, gradient_to_css, LightSource, LightingEnvironment,
    LightingResult, Vec3,
};

pub use fresnel::{
    brewster_angle, calculate_view_angle, edge_intensity, fresnel_full, fresnel_outer_glow_params,
    fresnel_schlick, generate_fresnel_gradient, to_css_fresnel_gradient, to_css_luminous_border,
};

pub use blinn_phong::{
    blinn_phong_specular, calculate_highlight_position, calculate_specular_layers,
    roughness_to_shininess, to_css_inner_glow, to_css_inner_highlight, to_css_secondary_specular,
    to_css_specular_highlight,
};

pub use perlin_noise::{presets as NoisePresets, PerlinNoise};

pub use lut::{beer_lambert_fast, fresnel_fast, total_lut_memory, BeerLambertLUT, FresnelLUT};

pub use batch::{evaluate_batch, BatchEvaluator, BatchMaterialInput, BatchResult};

pub use context::{
    BackgroundContext, ContextPresets, LightingContext, MaterialContext, ViewContext,
};

// PBR Phase 1 - Dispersion models
pub use dispersion::{
    chromatic_aberration_strength, f0_from_ior, f0_rgb, wavelengths, CauchyDispersion, Dispersion,
    DispersionModel, SellmeierDispersion,
};

// PBR Phase 1 - Scattering phase functions
pub use scattering::{
    double_henyey_greenstein, henyey_greenstein, hg_fast, presets as scattering_presets, sample_hg,
    HenyeyGreensteinLUT, ScatteringParams,
};

// PBR Phase 1 - Spectral Fresnel
pub use spectral_fresnel::{
    edge_intensity_rgb, fresnel_rgb, fresnel_rgb_fast, fresnel_rgb_lut, to_css_chromatic_border,
    to_css_chromatic_fresnel, SpectralFresnelLUT, SpectralFresnelResult,
};

// PBR Phase 1 - Enhanced material presets
pub use enhanced_presets::{
    all_presets, crown_glass, diamond, flint_glass, frosted_glass, fused_silica, ice, milk_glass,
    opal_glass, pmma, polycarbonate, presets_by_quality, sapphire, water, EnhancedGlassMaterial,
    QualityTier,
};

// PBR Phase 2 - Double Henyey-Greenstein LUT
pub use dhg_lut::{
    dhg_fast, dhg_preset, scattering_params_for_preset, total_dhg_memory, CompactDHGLUT, DHGPreset,
    DoubleHGLUT,
};

// PBR Phase 2 - Quality Tier System
pub use quality_tiers::{
    select_tier, DeviceCapabilities, MaterialComplexity, QualityConfig, TierFeatures, TierMetrics,
};

// PBR Phase 2 - Validation and Benchmarks
pub use phase2_validation::{
    benchmark_quality_tiers, compare_dhg_lut_vs_direct, compare_dhg_vs_hg,
    compare_sellmeier_vs_cauchy, full_phase2_report, memory_analysis, ComparisonResult,
    TierBenchmark,
};

// PBR Phase 3 - Complex IOR for Metals
pub use complex_ior::{
    fresnel_conductor, fresnel_conductor_schlick, fresnel_conductor_unpolarized, metals,
    to_css_metallic_gradient, to_css_metallic_surface, Complex, ComplexIOR, SpectralComplexIOR,
};

// PBR Phase 3 - Mie Scattering Approximation
pub use mie_lut::{
    mie_asymmetry_g, mie_efficiencies, mie_fast, mie_particle, mie_particle_rgb, mie_phase_hg,
    particles as mie_particles, rayleigh_efficiency, rayleigh_intensity_rgb, rayleigh_phase,
    total_mie_memory, MieLUT, MieParams,
};

// PBR Phase 3 - Thin-Film Interference
pub use thin_film::{
    ar_coating_thickness, dominant_wavelength, presets as thin_film_presets, thin_film_to_rgb,
    to_css_iridescent_gradient, to_css_oil_slick, to_css_soap_bubble, total_thin_film_memory,
    ThinFilm, ThinFilmStack,
};

// PBR Phase 3 - Validation and Benchmarks
pub use phase3_validation::{
    benchmark_thin_film, compare_complex_vs_dielectric_fresnel, compare_metal_schlick_vs_full,
    compare_mie_lut_vs_direct, compare_rayleigh_vs_mie, full_phase3_report, phase3_memory_analysis,
    validate_thin_film_physics, Phase3ComparisonResult, Phase3MemoryAnalysis,
};

// PBR Phase 4 - LUT Compression
pub use lut_compression::{
    calculate_memory_savings, dequantize_u16_to_f32, dequantize_u8_to_f32, quantize_f32_to_u16,
    quantize_f32_to_u8, CompressedFresnelLUT, CompressedHGLUT, CompressedLUT1D, CompressedLUT2D,
    CompressionAnalysis, DeltaEncodedLUT, EvaluationMethod, HybridEvaluator, SparseLUT1D,
};

// PBR Phase 4 - Advanced Thin-Film (Transfer Matrix)
pub use thin_film_advanced::{
    advanced_presets as thin_film_advanced_presets, FilmLayer, Matrix2x2, TransferMatrixFilm,
};

// PBR Phase 4 - Temperature-Dependent Metals
pub use metal_temp::{
    drude_metals, oxides, oxidized_presets, DrudeParams, OxideLayer, TempOxidizedMetal,
};

// PBR Phase 4 - Dynamic Mie Scattering
pub use mie_dynamic::{dynamic_presets, DynamicMieParams, SizeDistribution};

// PBR Phase 4 - Validation and Benchmarks
pub use phase4_validation::{
    benchmark_lut_compression, compare_dynamic_mie_presets, compare_metal_temperatures,
    compare_thin_film_methods, full_phase4_report, phase4_memory_analysis,
    validate_oxidation_effects, validate_polydisperse_scattering, validate_thin_film_advanced,
    CompressionBenchmark, DynamicMieResult, MetalTempResult, Phase4MemoryAnalysis,
    ThinFilmComparisonResult,
};

// PBR Phase 5 - Advanced Features
pub mod differentiable_render; // ✅ PBR Phase 5 - Auto-calibration with differentiable rendering
pub mod metal_oxidation_dynamic; // ✅ PBR Phase 5 - Dynamic metal oxidation with time evolution
pub mod mie_physics; // ✅ PBR Phase 5 - Advanced Mie physics with particle interactions
pub mod phase5_validation;
pub mod thin_film_dynamic; // ✅ PBR Phase 5 - Dynamic thin-film with physical deformations // ✅ PBR Phase 5 - Validation and benchmarks

// PBR Phase 5 - Differentiable Rendering / Auto-Calibration
pub use differentiable_render::{
    beer_lambert_diff, forward_dielectric, forward_metal, forward_thin_film, fresnel_schlick_diff,
    henyey_greenstein_diff, reference_presets, thin_film_reflectance_diff, AdamOptimizer,
    AutoCalibrator, LossConfig, MaterialParams, Optimizer, ParamGradient, SgdOptimizer,
};

// PBR Phase 5 - Dynamic Thin-Film
pub use thin_film_dynamic::{
    dynamic_presets as thin_film_dynamic_presets, DynamicFilmLayer, DynamicThinFilmStack,
    HeightMap, IridescenceMap, SubstrateProperties,
};

// PBR Phase 5 - Dynamic Metal Oxidation
pub use metal_oxidation_dynamic::{
    oxidation_presets, to_css_oxidation_animation, to_css_oxidized, AlloyComposition,
    DynamicOxidizedMetal, Element, OxidationKinetics, OxidationSimulation, OxidationState,
    OxideLayerProperties, OxideStructure,
};

// PBR Phase 5 - Advanced Mie Physics
pub use mie_physics::{
    ensemble_phase_function, ensemble_presets, henyey_greenstein as hg_phase, mie_approximation,
    to_css_scattering, to_css_scattering_animation, MediumProperties, Particle, ParticleDynamics,
    ParticleEnsemble, ParticleSpecies, ScatteringField, SizeStatistics, TurbulenceParams,
};

// PBR Phase 5 - Validation and Benchmarks
pub use phase5_validation::{
    analyze_memory as phase5_memory_analysis, benchmark_phase5, generate_validation_report,
    run_all_validations, validate_differentiable_gradients, validate_dynamic_thin_film,
    validate_integration, validate_material_params, validate_metal_oxidation, validate_mie_physics,
    BenchmarkResults, MemoryAnalysis, ValidationResult,
};

// PBR Phase 6 - Performance Optimization & Research-Grade Features
pub mod combined_effects; // ✅ PBR Phase 6 - Unified effect compositor
pub mod material_datasets; // ✅ PBR Phase 6 - Reference spectral data for calibration
pub mod perceptual_loss; // ✅ PBR Phase 6 - LAB color space & Delta E metrics
pub mod phase6_validation;
pub mod simd_batch; // ✅ PBR Phase 6 - SIMD-accelerated batch evaluation // ✅ PBR Phase 6 - Benchmarks and validation

// PBR Phase 6 - Perceptual Loss Functions
pub use perceptual_loss::{
    delta_e_2000, delta_e_76, delta_e_94, lab_to_rgb, lab_to_xyz, perceptual_loss,
    perceptual_loss_gradient, rgb_to_lab, rgb_to_xyz, xyz_to_lab, xyz_to_rgb, DeltaEFormula,
    Illuminant, LabColor, PerceptualLossConfig, XyzColor,
};

// PBR Phase 6 - Reference Material Datasets
pub use material_datasets::{
    MaterialCategory, MaterialDatabase, MeasurementMetadata, SpectralMeasurement,
};

// PBR Phase 6 - SIMD Batch Evaluation
pub use simd_batch::{
    beer_lambert_8, fresnel_schlick_8, henyey_greenstein_8, SimdBatchEvaluator, SimdBatchInput,
    SimdBatchResult, SimdConfig,
};

// PBR Phase 6 - Combined Effects Compositor
pub use combined_effects::{
    presets as combined_presets, BlendMode, CombinedMaterial, CombinedMaterialBuilder, EffectLayer,
    RoughnessModel,
};

// PBR Phase 6 - Validation and Benchmarks
pub use phase6_validation::{
    benchmark_phase6, validate_combined_effects, validate_material_datasets,
    validate_perceptual_loss, validate_simd_batch, Phase6MemoryAnalysis, SimdBenchmarks,
    ValidationResult as Phase6ValidationResult,
};

// PBR Phase 7 - Ultra-Realistic Rendering & Advanced Parallelization
pub mod auto_calibration_realtime; // ✅ PBR Phase 7 - Real-time perceptual calibration
pub mod combined_effects_advanced; // ✅ PBR Phase 7 - Extended effect layers with Phase 5 dynamics
pub mod phase7_validation;
pub mod presets_experimental; // ✅ PBR Phase 7 - 8 ultra-realistic experimental presets
pub mod simd_parallel; // ✅ PBR Phase 7 - CPU parallelization with SIMD inner loops
pub mod spectral_render; // ✅ PBR Phase 7 - Full spectral rendering with CIE CMF // ✅ PBR Phase 7 - Benchmarks and validation

// PBR Phase 7 - Parallel Batch Evaluation
pub use simd_parallel::{ParallelBatchEvaluator, ParallelBenchmark, ParallelConfig};

// PBR Phase 7 - Full Spectral Rendering
pub use spectral_render::{
    ColorMatchingLUT, SpectralMaterialEvaluator, SpectralRadiance, SpectralRenderConfig,
    WAVELENGTH_COUNT,
};

// PBR Phase 7 - Real-Time Auto-Calibration
pub use auto_calibration_realtime::{
    compare_to_dataset, perceptual_match_score, realtime_calibrate, CalibrationFeedbackLoop,
    ConvergenceStatus, RealtimeCalibrationConfig,
};

// PBR Phase 7 - Advanced Combined Effects
pub use combined_effects_advanced::{
    total_advanced_memory, AdvancedCombinedMaterial, AdvancedCombinedMaterialBuilder,
    AdvancedEffectLayer, DispersionModel as AdvancedDispersionModel, GradientType, PhysicalState,
    SizeDistribution as AdvancedSizeDistribution, TemperatureGradientConfig,
};

// PBR Phase 7 - Experimental Presets
pub use presets_experimental::{
    ancient_bronze, copper_aging, create_default as create_experimental_preset,
    dynamic_soap_bubble, list_presets as list_experimental_presets, morpho_dynamic,
    oil_on_water_dynamic, opalescent_suspension, preset_catalog, stressed_crystal, titanium_heated,
    total_presets_memory, PresetInfo,
};

// PBR Phase 7 - Validation and Benchmarks
pub use phase7_validation::{
    analyze_phase7_memory, benchmark_calibration, benchmark_parallel_performance, benchmark_phase7,
    benchmark_spectral_rendering, compare_phase6_vs_phase7, generate_phase7_report,
    validate_experimental_presets, validate_parallel_correctness, validate_perceptual_accuracy,
    validate_spectral_correctness, CalibrationMetrics, ParallelComparison, PerceptualValidation,
    Phase7BenchmarkResults, Phase7Comparison, Phase7MemoryAnalysis, SpectralComparison,
};

// PBR Phase 8 - Reference-Grade Scientific Validation & Ecosystem Integration
pub mod canonical_demos;
pub mod dataset_merl; // ✅ PBR Phase 8 - MERL BRDF dataset support
pub mod external_validation; // ✅ PBR Phase 8 - External dataset validation framework
pub mod material_export; // ✅ PBR Phase 8 - GLSL/WGSL/MaterialX/CSS export
pub mod material_fingerprint; // ✅ PBR Phase 8 - Deterministic material hashing & versioning
pub mod material_import; // ✅ PBR Phase 8 - MaterialX/glTF/JSON import
pub mod phase8_validation; // ✅ PBR Phase 8 - Benchmarks and reports
pub mod plugin_api; // ✅ PBR Phase 8 - Versioned plugin system
pub mod reference_renderer; // ✅ PBR Phase 8 - IEEE754 full precision rendering
pub mod research_api; // ✅ PBR Phase 8 - ML integration & optimization hooks
pub mod spectral_error; // ✅ PBR Phase 8 - Comprehensive spectral/perceptual error metrics
pub mod tier_validation; // ✅ PBR Phase 8 - Cross-tier validation // ✅ PBR Phase 8 - Reproducible scientific demos

// PBR Phase 8 - Reference Renderer
pub use reference_renderer::{
    total_reference_memory, LutVsReferenceComparison, PrecisionMode, ReferenceRenderConfig,
    ReferenceRenderResult, ReferenceRenderer,
};

// PBR Phase 8 - Spectral Error Metrics
pub use spectral_error::{
    compute_comprehensive, compute_energy_metrics, compute_perceptual_metrics,
    compute_spectral_angle, compute_spectral_metrics, total_error_memory, ComprehensiveMetrics,
    EnergyMetrics, PerceptualErrorMetrics, PerceptualQualityGrade, SpectralErrorMetrics,
    SpectralQualityGrade, ValidationStatus,
};

// PBR Phase 8 - Material Fingerprinting
pub use material_fingerprint::{
    deterministic_hash, fingerprint_from_named, fingerprint_from_params, total_fingerprint_memory,
    CalibrationEntry, CalibrationLog, MaterialFingerprint, MaterialVersion,
};

// PBR Phase 8 - External Validation Framework
pub use external_validation::{
    total_validation_memory, ExternalDataset, MaterialValidation, ReportSummary, ValidationConfig,
    ValidationEngine, ValidationReport, ValidationResult as ExternalValidationResult,
};

// PBR Phase 8 - MERL BRDF Dataset
pub use dataset_merl::{
    total_merl_memory, MaterialCategory as MerlCategory, MerlDataset, MerlMaterial,
};

// PBR Phase 8 - Material Export
pub use material_export::{
    total_export_memory, ExportOptions, ExportTarget, GlslVersion, MaterialDescriptor,
    MaterialExporter, ThinFilmDescriptor,
};

// PBR Phase 8 - Material Import
pub use material_import::{
    total_import_memory, ImportAdapter, ImportError, ImportSource, MaterialImporter,
};

// PBR Phase 8 - Plugin System
pub use plugin_api::{
    estimate_plugin_api_memory, DatasetPlugin, EvaluationContext, LambertianPlugin, MaterialType,
    MetricPlugin, PluginError, PluginInfo, PluginInventory, PluginMaterialParams, PluginRegistry,
    PluginRenderOutput, RenderPlugin, RmseMetricPlugin, SamMetricPlugin,
    SpectralMeasurement as PluginSpectralMeasurement, PLUGIN_API_VERSION,
    PLUGIN_API_VERSION_STRING,
};

// PBR Phase 8 - Research API
pub use research_api::{
    estimate_research_api_memory, Constraint, ConstraintType, ForwardFunction, GridSearchOptimizer,
    MaterialForwardFunction, MultiObjectiveTarget, ObjectiveFunction, ObjectiveType,
    OptimizationResult, ParameterBounds, ParameterMapping,
};

// PBR Phase 8 - Validation and Benchmarks
pub use phase8_validation::{
    analyze_phase8_memory, benchmark_export_performance, benchmark_fingerprint_consistency,
    benchmark_merl_validation, benchmark_phase8, benchmark_plugin_overhead,
    benchmark_reference_accuracy, generate_phase8_json_report, generate_phase8_report,
    ExportTimingResults, FingerprintResults, LutVsReferenceResults, MerlValidationResults,
    Phase8BenchmarkResults, Phase8MemoryAnalysis, PluginOverheadResults,
};

// PBR Phase 8 - Tier Cross-Validation
pub use tier_validation::{
    generate_full_validation_report, get_validation_summary, run_cross_validation,
    run_dispersion_validation, run_metal_validation, CrossValidationReport, TierValidationResult,
};

// PBR Phase 8 - Canonical Demos
pub use canonical_demos::{
    demo_ar_coating, demo_copper_patina, demo_dielectric_vs_conductor, demo_fog_vs_smoke,
    demo_spectral_vs_rgb, demo_thin_film_soap_bubble, run_all_demos, run_demo, DemoResult,
    DemoSuite,
};

// PBR Phase 9 - Unified BSDF + Perceptual Rendering Loop
pub mod anisotropic_brdf; // ✅ PBR Phase 9 - Anisotropic GGX microfacet model
pub mod perceptual_loop; // ✅ PBR Phase 9 - Closed-loop perceptual optimization
pub mod phase9_validation;
pub mod subsurface_scattering; // ✅ PBR Phase 9 - Diffusion BSSRDF for translucent materials
pub mod unified_bsdf; // ✅ PBR Phase 9 - Unified BSDF trait with energy conservation // ✅ PBR Phase 9 - Comprehensive validation suite

// PBR Phase 9 - Unified BSDF
pub use unified_bsdf::{
    evaluate_rgb as bsdf_evaluate_rgb, evaluate_spectral as bsdf_evaluate_spectral,
    total_unified_bsdf_memory, validate_energy_conservation as bsdf_validate_energy, BSDFContext,
    BSDFResponse, BSDFSample, ConductorBSDF, DielectricBSDF,
    DispersionModel as BSDFDispersionModel, EnergyValidation, LambertianBSDF, LayeredBSDF,
    ThinFilmBSDF, Vector3 as BSDFVector3, BSDF,
};

// PBR Phase 9 - Anisotropic BRDF
pub use anisotropic_brdf::{
    anisotropy_strength, presets as anisotropic_presets, strength_to_alphas,
    total_anisotropic_memory, AnisotropicConductor, AnisotropicGGX, FiberBSDF,
};

// PBR Phase 9 - Subsurface Scattering
pub use subsurface_scattering::{
    sss_presets, total_sss_memory, DiffusionBSSRDF, SubsurfaceBSDF, SubsurfaceParams,
};

// PBR Phase 9 - Perceptual Rendering Loop
pub use perceptual_loop::{
    quick_match_color, total_perceptual_loop_memory, AdamState,
    ConvergenceStatus as PerceptualConvergenceStatus, MaterialParams as PerceptualMaterialParams,
    OptimizationResult as PerceptualOptResult, ParameterBounds as PerceptualBounds,
    PerceptualLoopConfig, PerceptualRenderingLoop, PerceptualTarget,
};

// PBR Phase 9 - Validation
pub use phase9_validation::{
    analyze_memory as analyze_phase9_memory, generate_report as generate_phase9_report,
    run_full_validation as run_phase9_validation, validate_anisotropic,
    validate_energy_conservation_all as validate_phase9_energy, validate_perceptual_loop,
    validate_sss, validate_unified_vs_legacy, AnisotropicValidation, BSDFComparisonResults,
    ConvergenceResults, EnergyConservationReport, Phase9MemoryAnalysis, Phase9ValidationReport,
    SSSValidation,
};

// PBR Phase 10 - Neural Correction Layers & Hybrid Physical-Neural Rendering
pub mod neural_constraints; // ✅ PBR Phase 10 - Physics-based constraints enforcement
pub mod neural_correction; // ✅ PBR Phase 10 - SIREN MLP for physics residuals
pub mod phase10_validation;
pub mod training_dataset; // ✅ PBR Phase 10 - Synthetic + MERL training data generation
pub mod training_pipeline; // ✅ PBR Phase 10 - Adam training loop with perceptual loss // ✅ PBR Phase 10 - Comprehensive validation suite

// PBR Phase 10 - Neural Correction MLP
pub use neural_correction::{
    total_neural_correction_memory, CorrectionInput, CorrectionOutput, NeuralCorrectedBSDF,
    NeuralCorrectionMLP,
};

// PBR Phase 10 - Physics Constraints
pub use neural_constraints::{
    total_neural_constraints_memory, ConstraintConfig, ConstraintType as NeuralConstraintType,
    ConstraintValidator, ConstraintViolationReport, RegularizationTerms,
};

// PBR Phase 10 - Training Dataset
pub use training_dataset::{
    estimate_dataset_memory, AugmentationConfig, DatasetMetadata, DatasetSource, TrainingDataset,
    TrainingSample,
};

// PBR Phase 10 - Training Pipeline
pub use training_pipeline::{
    total_training_pipeline_memory, LossWeights, TrainingConfig, TrainingPipeline, TrainingResult,
};

// PBR Phase 10 - Validation
pub use phase10_validation::{
    analyze_phase10_memory, compute_network_stats, generate_report as generate_phase10_report,
    run_full_validation as run_phase10_validation,
    validate_energy_conservation as validate_phase10_energy, validate_perceptual_improvement,
    validate_physical_vs_hybrid, ComparisonResults, EnergyValidation as Phase10EnergyValidation,
    NetworkStats, PerceptualImprovement, Phase10MemoryAnalysis, Phase10ValidationReport,
};

// PBR Phase 11 - Production Readiness, GPU Acceleration & Public Canonicalization
pub mod gpu_backend; // ✅ PBR Phase 11 - GPU compute backend (wgpu/WGSL)
pub mod pbr_api; // ✅ PBR Phase 11 - Stable public API v1.0
pub mod phase11_validation; // ✅ PBR Phase 11 - Validation suite

// PBR Phase 11 - GPU Backend
pub use gpu_backend::{estimate_gpu_backend_memory, GpuBackendConfig, GpuBackendStats};

#[cfg(feature = "gpu")]
pub use gpu_backend::{
    AutoFallback, BSDFResponseGpu, BufferHandle, BufferPool, ComputePipelineCache, DeviceLimits,
    FallbackBatchEvaluator, FallbackReason, FallbackStats, GpuBatchEvaluator, GpuBatchResult,
    GpuCapabilities, GpuContext, GpuContextError, GpuCpuParityTest, GpuDispatchConfig,
    MaterialGpuData, ParityConfig, ParityResult, PipelineType,
};

// PBR Phase 11 - Stable Public API v1.0
pub use pbr_api::v1::{
    is_compatible, AnisotropicGGX as PbrAnisotropicGGX, EvaluationContext as PbrEvaluationContext,
    Layer, Material, MaterialBuilder, MaterialPreset, QualityTier as PbrQualityTier,
    Vector3 as PbrVector3, API_VERSION, API_VERSION_STRING,
};

// PBR Phase 11 - Prelude for convenient imports
pub mod pbr_prelude {
    pub use super::pbr_api::v1::prelude::*;
}

// PBR Phase 11 - Validation
pub use phase11_validation::{
    analyze_phase11_memory, generate_report as generate_phase11_report,
    run_full_validation as run_phase11_validation, validate_api_stability,
    validate_energy_conservation as validate_phase11_energy, validate_gpu_parity, validate_memory,
    ApiStabilityResults, EnergyResults, GpuParityResults, MemoryResults, Phase11MemoryAnalysis,
    Phase11ValidationReport,
};

// PBR Phase 12 - Temporal Light Transport, Spectral Coherence & Differentiable Foundations
pub mod neural_temporal_correction; // ✅ PBR Phase 12 - Neural correction with cumulative drift bounding
pub mod phase12_validation;
pub mod spectral_coherence; // ✅ PBR Phase 12 - Spectral flicker prevention and coherent sampling
pub mod temporal; // ✅ PBR Phase 12 - Temporal BSDF evaluation with time-aware materials // ✅ PBR Phase 12 - Validation suite for temporal stability

// PBR Phase 12 - Temporal Material Model
pub use temporal::{
    estimate_temporal_memory, inverse_lerp, lerp, remap, smootherstep, smoothstep,
    ConductorEvolution, DielectricEvolution, DriftConfig, DriftStatus, DriftTracker, EvolutionRate,
    ExponentialMovingAverage, Interpolation, InterpolationMode, RateLimiter, TemporalBSDF,
    TemporalBSDFInfo, TemporalConductor, TemporalContext, TemporalContextBuilder,
    TemporalDielectric, TemporalEvolution, TemporalThinFilm, ThinFilmEvolution,
};

// PBR Phase 12 - Spectral Coherence
pub use spectral_coherence::{
    estimate_spectral_coherence_memory, BlendConfig, CoherenceMetadata, CoherentSampler,
    FlickerConfig, FlickerReport, FlickerStatus, FlickerValidator, FrameComparison,
    GradientLimiter, JitteredSampler, SamplingStrategy, SpectralInterpolator, SpectralPacket,
    SpectralPacketBuilder, StratifiedSampler, WavelengthBand,
};

// PBR Phase 12 - Neural Temporal Correction
pub use neural_temporal_correction::{
    estimate_temporal_neural_memory, CumulativeDriftTracker, DriftLimitConfig,
    TemporalCorrectionInput, TemporalNeuralConfig, TemporalNeuralCorrectedBSDF,
    TemporalNeuralCorrection,
};

// PBR Phase 12 - Validation
pub use phase12_validation::{
    run_full_validation as run_phase12_full, run_quick_validation as run_phase12_quick,
    run_strict_validation as run_phase12_strict, Phase12ValidationConfig, Phase12ValidationReport,
    Phase12ValidationSuite, ValidationResult as Phase12ValidationResult,
};

// PBR Phase 13 - Differentiable Rendering, Inverse Materials & Physical Parameter Recovery
pub mod differentiable; // ✅ PBR Phase 13 - DifferentiableBSDF trait with analytical gradients
pub mod gradient_validation; // ✅ PBR Phase 13 - Analytical vs numerical gradient verification
pub mod inverse_material; // ✅ PBR Phase 13 - Adam/L-BFGS optimization for parameter recovery
pub mod phase13_validation;
pub mod spectral_gradients; // ✅ PBR Phase 13 - Per-wavelength gradients and ΔE2000
pub mod temporal_differentiable; // ✅ PBR Phase 13 - BPTT and evolution gradients // ✅ PBR Phase 13 - Comprehensive validation suite

// PBR Phase 13 - Differentiable BSDF Rendering
pub use differentiable::{
    beer_lambert_gradient, estimate_differentiable_memory, fresnel_conductor_gradient,
    fresnel_schlick_gradient, ggx_distribution_gradient, smith_g_gradient, DifferentiableBSDF,
    DifferentiableConductor, DifferentiableDielectric, DifferentiableLayered,
    DifferentiableResponse, DifferentiableThinFilm, GradientConfig, GradientVerification, Jacobian,
    JacobianBuilder, LayerConfig, ParameterBounds as DifferentiableBounds, ParameterGradients,
};

// PBR Phase 13 - Inverse Material Solver
pub use inverse_material::{
    estimate_inverse_memory, recover_ior_from_normal_reflectance,
    recover_roughness_from_glossiness, AdamConfig as InverseAdamConfig,
    AdamOptimizer as InverseAdamOptimizer, BoundsConfig as InverseBoundsConfig, BoundsEnforcer,
    ConvergenceReason, DifferentiableOptimizer, InverseMaterialSolver, InverseResult,
    InverseSolverConfig, LBFGSConfig, LBFGSOptimizer, LossFunction, ProjectionMethod,
    ReferenceData, ReferenceObservation, TemporalFitResult, TemporalFitter, TemporalFitterConfig,
    TemporalSequence,
};

// PBR Phase 13 - Temporal Differentiable
pub use temporal_differentiable::{
    compute_evolution_gradient, estimate_temporal_differentiable_memory, BPTTConfig, BPTTState,
    EvolutionGradient, EvolutionGradients, EvolutionType, ExponentialEvolutionGradient,
    GradientStabilizer, LinearEvolutionGradient, OscillatingEvolutionGradient, StabilizerConfig,
    TemporalGradientAccumulator, BPTT,
};

// PBR Phase 13 - Spectral Gradients
pub use spectral_gradients::{
    compute_spectral_gradient, delta_e_2000 as spectral_delta_e_2000, delta_e_2000_gradient,
    estimate_spectral_gradients_memory, CauchyDispersion as SpectralCauchyDispersion,
    DeltaE2000Gradient, Lab, LabGradient, PerceptualLoss as SpectralPerceptualLoss,
    SellmeierDispersion as SpectralSellmeierDispersion, SpectralGradient, SpectralJacobian,
    WavelengthGradient, VISIBLE_WAVELENGTHS,
};

// PBR Phase 13 - Gradient Validation
pub use gradient_validation::{
    full_verify_with_report, numerical_gradient_central, numerical_gradient_forward,
    numerical_jacobian, quick_verify, verify_bsdf_gradients, BatchVerification, GradientCheck,
    GradientVerificationResult, VerificationConfig,
};

// PBR Phase 13 - Validation
pub use phase13_validation::{
    run_phase13_validation, Phase13ValidationConfig, Phase13ValidationReport, ValidationTest,
};

// PBR Phase 14 - Digital Material Twins & Calibration
pub mod calibration; // ✅ PBR Phase 14 - Multi-source calibration (BRDF, spectral, time-series)
pub mod identifiability; // ✅ PBR Phase 14 - Jacobian rank analysis, parameter freezing
pub mod material_twin; // ✅ PBR Phase 14 - MaterialTwin abstraction with UUID, fingerprint, variants
pub mod phase14_validation;
pub mod twin_validation; // ✅ PBR Phase 14 - TwinValidator, DriftMonitor
pub mod uncertainty; // ✅ PBR Phase 14 - Covariance, Fisher information, bootstrap CI // ✅ PBR Phase 14 - Comprehensive validation suite

// PBR Phase 14 - Material Twin Core
pub use material_twin::{
    CalibrationMetadata, CalibrationQuality, LayeredTwinData, MaterialTwin, MeasuredTwinData,
    SpectralDistance, SpectralIdentity, SpectralSignature, StaticTwinData, TemporalTwinData,
    TwinBuilder, TwinId, TwinVariant,
};

// PBR Phase 14 - Calibration Pipeline
pub use calibration::{
    estimate_calibration_memory, AggregatedLoss, BRDFObservation, BRDFSource, CalibrationSource,
    CombinedSource, DataQuality, ImputationStrategy, LossAggregator, LossComponents,
    LossWeights as CalibrationLossWeights, MissingDataReport, PartialDataHandler,
    SpectralObservation, SpectralSource, TemporalObservation, TimeSeriesSource,
};

// PBR Phase 14 - Uncertainty Estimation
pub use uncertainty::{
    estimate_uncertainty_memory, BootstrapConfig, BootstrapResampler, BootstrapResult,
    ConfidenceInterval, ConfidenceLevel, ConfidenceWarning, CovarianceEstimator,
    FisherInformationEstimator, FisherInformationMatrix, ParameterCovarianceMatrix,
    ParameterUncertainty, TwinConfidenceReport,
};

// PBR Phase 14 - Identifiability Analysis
pub use identifiability::{
    compute_vif, estimate_identifiability_memory, find_correlation_clusters, CorrelationAnalysis,
    CorrelationCluster, FreezingReason, FreezingRecommendation, FreezingReport, FreezingStrategy,
    IdentifiabilityResult, JacobianRankAnalyzer, ParameterCorrelationMatrix,
    ParameterFreezingRecommender, RankDeficiency,
};

// PBR Phase 14 - Twin Validation & Drift Monitoring
pub use twin_validation::{
    estimate_validation_memory, DriftMonitor, DriftObservation, DriftReport, DriftStatistics,
    IssueCategory, TwinValidator, ValidationConfig as TwinValidationConfig, ValidationIssue,
    ValidationRecord, ValidationResult as TwinValidationResult,
};

// PBR Phase 15 - Certifiable Material Twins
pub mod certification; // ✅ PBR Phase 15 - Certification levels, profiles, and auditing
pub mod compliance; // ✅ PBR Phase 15 - Ground truth validation, neural audit, export
pub mod instruments; // ✅ PBR Phase 15 - Virtual measurement instruments
pub mod metrology; // ✅ PBR Phase 15 - Formal metrology layer (measurement, uncertainty, traceability)
pub mod phase15_validation; // ✅ PBR Phase 15 - Comprehensive validation suite (67+ tests)

// PBR Phase 15 - Metrology Layer
pub use metrology::{
    celsius_to_kelvin,
    convert_unit,
    deg_to_rad,
    // Memory
    estimate_memory_footprint as estimate_metrology_memory,
    fraction_to_percent,
    identity_correlation,
    kelvin_to_celsius,
    nm_to_um,
    percent_to_fraction,
    rad_to_deg,
    um_to_nm,
    uniform_correlation,
    units_compatible,
    validate_correlation_matrix,
    CalibrationReference,
    CertificationTolerance,
    ChainMetadata,
    ComponentValidation,
    // Measurement
    Measurement,
    MeasurementArray,
    MeasurementId,
    MeasurementQuality,
    MeasurementSource,
    MetrologyMemoryEstimate,
    PropagationMethod,
    SensitivityAnalysis,
    // Tolerance
    ToleranceBudget,
    ToleranceCategory,
    ToleranceComponent,
    ToleranceValidation,
    // Traceability
    TraceabilityChain,
    TraceabilityEntry,
    TraceabilityOperation,
    Uncertainty,
    // Propagation
    UncertaintyPropagator,
    // Units
    Unit,
    UnitValue,
};

// PBR Phase 15 - Virtual Instruments
pub use instruments::{
    cauchy_dispersion,
    constant_optical_constants,
    constant_reflectance,
    // Memory
    estimate_memory_footprint as estimate_instruments_memory,
    fresnel_brdf,
    gaussian_absorption,
    glass_optical_constants,
    lambertian_brdf,
    linear_reflectance,
    phong_brdf,
    silicon_optical_constants,
    BiasModel,
    DetectorGeometry,
    EllipsometerType,
    EllipsometryPoint,
    EllipsometryResult,
    EnvironmentConditions,
    GoniometerResult,
    InstrumentConfig,
    InstrumentsMemoryEstimate,
    LightSource as InstrumentLightSource,
    LightSourceType,
    // Common
    NoiseModel,
    Polarization,
    Resolution,
    SimpleRng,
    SpectroGeometry,
    SpectroMeasurementType,
    SpectroResult,
    ThinFilmResult,
    // Ellipsometer
    VirtualEllipsometer,
    // Gonioreflectometer
    VirtualGonioreflectometer,
    // Spectrophotometer
    VirtualSpectrophotometer,
};

// PBR Phase 15 - Certification System
pub use certification::{
    // Quick functions
    can_achieve_level,
    // Memory
    estimate_memory_footprint as estimate_certification_memory,
    highest_level,
    quick_certify_experimental,
    required_test_count,
    required_tests,
    // Auditor
    CertificationAuditor,
    // Levels
    CertificationLevel,
    CertificationMemoryEstimate,
    CertificationMetrics,
    CertificationResult,
    // Profiles
    CertifiedTwinProfile,
    LevelCheck,
    LevelRequirements,
    // Requirements
    MandatoryTest,
    MaterialAuditData,
    NeuralCorrectionStats,
    NeuralViolation,
    TestResult as CertificationTestResult,
    TestSuiteResult,
};

// PBR Phase 15 - Compliance & Export
pub use compliance::{
    batch_export,
    bk7_reference_data,
    compute_reproducibility_hash,
    estimate_memory_footprint as estimate_compliance_memory,
    full_compliance_check,
    gold_reference_data,
    // Quick checks
    quick_ground_truth_check,
    quick_neural_audit,
    quick_reproducibility_check,
    silver_reference_data,
    // Validation
    validate_module as validate_compliance_module,
    verify_hash,
    AuditFinding,
    ComparisonResult as ReproducibilityComparisonResult,
    ComplianceMemoryEstimate,
    ComplianceValidation,
    CorrectionCheck,
    CrossPlatformReference,
    DatasetValidationReport,
    ExportFormat,
    FindingCategory,
    FindingSeverity,
    FullComplianceResult,
    GroundTruthDataset,
    // Ground Truth
    GroundTruthValidator,
    // Export
    MetrologicalExporter,
    NeuralAuditResult,
    // Neural Audit
    NeuralAuditor,
    ReferenceMeasurement,
    ReproducibilityResult,
    // Reproducibility
    ReproducibilityTest,
    SpectralMeasurement as ComplianceSpectralMeasurement,
    ValidationReport as GroundTruthValidationReport,
};

// PBR Phase 15 - Validation Suite
pub use phase15_validation::{run_phase15_validation, Phase15ValidationResult};

// Sprint 8 - Scientific Validation
pub use scientific_validation::{
    airy_thin_film_reflectance,
    bk7_sellmeier,
    cauchy_dispersion as cauchy_dispersion_analytical,
    copper_optical_constants,
    fresnel_conductor_exact,
    // Analytical References
    fresnel_dielectric_exact,
    // Reference Data
    gold_optical_constants,
    mie_asymmetry_g as mie_asymmetry_g_analytical,
    rayleigh_scattering,
    run_full_validation as run_sprint8_validation,
    sellmeier_dispersion,
    silver_optical_constants,
    transfer_matrix_multilayer,
    validate_dispersion_bk7,
    validate_energy_conservation as validate_sprint8_energy_conservation,
    validate_fresnel_dielectric,
    validate_gold_reflectance,
    validate_mie_rayleigh_limit,
    // Validation Functions
    validate_thin_film_vs_airy,
    PhenomenonValidation,
    // Statistical Metrics
    ValidationMetrics as ScientificValidationMetrics,
    ValidationReport as ScientificValidationReport,
};

// Phase 5 Intelligence - Enhanced Neural Corrections
pub mod phase13_neural;

// Enterprise Phase 7 - Advanced Materials Extension
pub mod advanced_material_presets;
pub mod anisotropic; // ✅ Enterprise Phase 7 - Anisotropic BSDF (brushed metals, hair, fabric)
pub mod meta_materials; // ✅ Enterprise Phase 7 - Photonic crystals, structural color
pub mod plasmonic; // ✅ Enterprise Phase 7 - Plasmonic nanoparticles (LSPR) // ✅ Enterprise Phase 7 - Extended presets (architectural, automotive, natural, technical)

// Phase 5 Intelligence - Wavelength-Specific Corrections
pub use phase13_neural::{
    BandInterpolator, BandStats, SpectralCorrectionResult, WavelengthBand as NeuralWavelengthBand,
    WavelengthCorrectionConfig, WavelengthCorrectionMLP,
};

// Phase 5 Intelligence - Polarization-Aware Corrections
pub use phase13_neural::{
    PolarizationCorrectionConfig, PolarizationCorrectionMLP, PolarizationDifference,
    PolarizationState, PolarizedResponse,
};

// Phase 5 Intelligence - Training Data Collection
pub use phase13_neural::{
    CollectionFilter, CollectionStatistics, DataSource, MaterialType as TrainingMaterialType,
    MemoryStorage, SampleMetadata, StorageError, TrainingDataCollector,
    TrainingSample as NeuralTrainingSample, TrainingSampleInput, TrainingSampleTarget,
    TrainingStorage,
};

// Phase 5 Intelligence - Confidence Scoring
pub use phase13_neural::{
    estimate_phase13_neural_memory, thresholds as confidence_thresholds,
    ConfidenceLevel as NeuralConfidenceLevel, ConfidenceScorer, ConfidenceScorerConfig,
    CorrectionConfidence, InputDistribution, TrainingStatistics,
};

// Enterprise Phase 7 - Anisotropic Materials (brushed metals, hair, fabric)
pub use anisotropic::{
    estimate_anisotropic_memory, presets as anisotropic_material_presets, AnisotropicBSDF,
    AshikhminShirleyBSDF, Color as AnisotropicColor, HairBSDF,
};

// Enterprise Phase 7 - Meta-Materials (photonic crystals, structural color)
pub use meta_materials::{
    estimate_meta_materials_memory, presets as meta_material_presets, DiffractionGrating,
    LatticeType, MaterialRef, NanostructureType, PhotonicCrystal, StructuralColor,
};

// Enterprise Phase 7 - Plasmonic Materials (nanoparticles, LSPR)
pub use plasmonic::{
    estimate_plasmonic_memory, presets as plasmonic_presets, MetalType as PlasmonicMetalType,
    Ordering as PlasmonicOrdering, ParticleShape, PlasmonicArray, PlasmonicFilm,
    PlasmonicNanoparticle,
};

// Enterprise Phase 7 - Advanced Material Presets (architectural, automotive, natural, technical)
pub use advanced_material_presets::{
    // Technical coatings
    anti_reflective_coating,
    beetle_shell,
    // Automotive finishes
    car_paint_metallic,
    // Catalog functions
    catalog as advanced_material_catalog,
    chrome_finish,
    dichroic_filter,
    electrochromic_glass,
    estimate_advanced_presets_memory,
    get_preset_info,
    holographic,
    list_by_category,
    // Architectural glass
    low_e_coating,
    mother_of_pearl,
    // Natural materials
    opal,
    pearlescent_paint,
    smart_glass_pdlc,
    AdvancedMaterialCategory,
    AdvancedMaterialInfo,
};

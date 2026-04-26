//! # Momoto Materials - Advanced Glass Physics System
//!
//! Physically-inspired material effects for perceptual color systems.
//!
//! ## Features
//!
//! ### Core Systems
//! - **Liquid Glass**: Modern translucent materials with perceptual color science
//! - **Glass Physics**: Beer-Lambert transmittance, Snell's law refraction, physical light models
//! - **Shadow Engine**: Multi-layer shadows (contact, ambient, elevation)
//! - **Material Composition**: Edge highlights, frost layers, translucency
//!
//! ### Advanced Capabilities
//! - **Multi-layer composition**: Highlight, shadow, and illumination layers
//! - **Adaptive tinting**: Context-aware color adaptation
//! - **Contrast validation**: Ensures text readability on glass surfaces
//! - **Perceptual uniformity**: All adjustments in OKLCH space
//! - **Physics-derived gradients**: Real light interaction, not decorative
//!
//! ## Quick Start
//!
//! ```rust
//! use momoto_core::color::Color;
//! use momoto_materials::glass::{LiquidGlass, GlassVariant};
//!
//! // Create glass surface
//! let background = Color::from_srgb8(59, 130, 246); // Blue
//! let glass = LiquidGlass::new(GlassVariant::Regular);
//!
//! // Get recommended text color
//! let text_color = glass.recommend_text_color(background, true);
//! ```
//!
//! ## Advanced Usage
//!
//! ```rust
//! use momoto_materials::glass_physics::{
//!     transmittance::{OpticalProperties, calculate_multi_layer_transmittance},
//!     light_model::{LightingEnvironment, derive_gradient},
//! };
//! use momoto_materials::shadow_engine::{
//!     elevation_shadow::{calculate_elevation_shadow, ElevationPresets},
//! };
//! use momoto_core::space::oklch::OKLCH;
//!
//! // 1. Calculate physical transmittance
//! let optical = OpticalProperties::default();
//! let layers = calculate_multi_layer_transmittance(&optical, 1.0);
//!
//! // 2. Generate physics-based gradient
//! let environment = LightingEnvironment::default();
//! let gradient = derive_gradient(&environment, 0.3, 32.0, 10);
//!
//! // 3. Calculate elevation shadow
//! let background = OKLCH::new(0.95, 0.01, 240.0);
//! let shadow = calculate_elevation_shadow(ElevationPresets::LEVEL_2, background, 1.0);
//! ```

// TODO: Re-enable missing_docs after Phase 15 documentation sprint
#![allow(missing_docs)]
#![deny(unsafe_code)]

// Core modules
pub mod blur;
pub mod elevation;
pub mod glass;
pub mod vibrancy;

// Advanced physics modules
pub mod glass_physics;
pub mod shadow_engine;

// Enhanced CSS generation
pub mod css_enhanced;

// Re-exports - Core
pub use elevation::{Elevation, MaterialSurface};
pub use glass::{GlassProperties, GlassVariant, LiquidGlass};
pub use vibrancy::{VibrancyEffect, VibrancyLevel};

// Re-exports - Glass Physics
pub use glass_physics::{
    light_model::{LightingEnvironment, LightingResult},
    refraction_index::{RefractionParams, RefractionResult},
    transmittance::{LayerTransmittance, OpticalProperties, TransmittanceResult},
};

// Re-exports - Shadow Engine
pub use shadow_engine::{
    ambient_shadow::{AmbientShadow, AmbientShadowParams},
    contact_shadow::{ContactShadow, ContactShadowParams},
    elevation_shadow::{ElevationShadow, ElevationTransition, InteractiveState},
};

// Re-exports - Enhanced CSS
pub use css_enhanced::{render_enhanced_css, render_premium_css, EnhancedCssBackend};

// ============================================================================
// Tier 1 - Essential APIs (Core shading, context, presets, performance)
// ============================================================================

// Fresnel reflectance - fundamental for glass/metal rendering
pub use glass_physics::{
    brewster_angle, edge_intensity, fresnel_full, fresnel_schlick, generate_fresnel_gradient,
    to_css_fresnel_gradient, to_css_luminous_border,
};

// Blinn-Phong specular - core highlight calculations
pub use glass_physics::{
    blinn_phong_specular, calculate_highlight_position, calculate_specular_layers,
    roughness_to_shininess, to_css_inner_highlight, to_css_specular_highlight,
};

// Context system - environment-aware material evaluation
pub use glass_physics::{
    BackgroundContext, ContextPresets, LightingContext, MaterialContext, ViewContext,
};

// Enhanced material presets with quality tiers
pub use glass_physics::{
    all_presets, crown_glass, diamond, flint_glass, frosted_glass, fused_silica, ice, milk_glass,
    opal_glass, pmma, polycarbonate, presets_by_quality, sapphire, water, EnhancedGlassMaterial,
    QualityTier,
};

// LUT system for performance (5-10x speedup)
pub use glass_physics::{
    beer_lambert_fast, fresnel_fast, total_lut_memory, BeerLambertLUT, FresnelLUT,
};

// Batch evaluation for bulk operations (7-10x speedup)
pub use glass_physics::{evaluate_batch, BatchEvaluator, BatchMaterialInput, BatchResult};

// Quality tier selection system
pub use glass_physics::{
    select_tier, DeviceCapabilities, MaterialComplexity, QualityConfig, TierFeatures, TierMetrics,
};

// ============================================================================
// Tier 2 - Advanced APIs (BSDF, digital twins, thin film, metals)
// ============================================================================

// Unified BSDF system - physically-based material evaluation
pub use glass_physics::{
    bsdf_evaluate_rgb, bsdf_evaluate_spectral, bsdf_validate_energy, BSDFContext, BSDFResponse,
    BSDFSample, ConductorBSDF, DielectricBSDF, EnergyValidation, LambertianBSDF, LayeredBSDF,
    ThinFilmBSDF, BSDF,
};

// Digital Material Twins - calibrated material instances
pub use glass_physics::{
    CalibrationMetadata, CalibrationQuality, LayeredTwinData, MaterialTwin, MeasuredTwinData,
    SpectralDistance, SpectralIdentity, SpectralSignature, StaticTwinData, TemporalTwinData,
    TwinBuilder, TwinId, TwinVariant,
};

// Thin film interference - iridescent effects
pub use glass_physics::{
    ar_coating_thickness, dominant_wavelength, thin_film_to_rgb, to_css_iridescent_gradient,
    to_css_oil_slick, to_css_soap_bubble, total_thin_film_memory, ThinFilm, ThinFilmStack,
};

// Complex IOR for metals (gold, silver, copper, etc.)
pub use glass_physics::{
    fresnel_conductor, fresnel_conductor_schlick, fresnel_conductor_unpolarized, metals,
    to_css_metallic_gradient, to_css_metallic_surface, Complex, ComplexIOR, SpectralComplexIOR,
};

// Dispersion models - wavelength-dependent IOR
pub use glass_physics::{
    chromatic_aberration_strength, f0_from_ior, f0_rgb, CauchyDispersion, Dispersion,
    DispersionModel, SellmeierDispersion,
};

// Scattering phase functions
pub use glass_physics::{
    double_henyey_greenstein, henyey_greenstein, hg_fast, sample_hg, HenyeyGreensteinLUT,
    ScatteringParams,
};

// Mie scattering for particles
pub use glass_physics::{
    mie_asymmetry_g, mie_efficiencies, mie_fast, mie_particle, mie_particle_rgb, mie_phase_hg,
    rayleigh_efficiency, rayleigh_intensity_rgb, rayleigh_phase, total_mie_memory, MieLUT,
    MieParams,
};

// ============================================================================
// Tier 3 - Research & Production APIs (differentiable, calibration, GPU)
// ============================================================================

// Differentiable rendering - gradient computation for optimization
pub use glass_physics::{
    DifferentiableBSDF, DifferentiableConductor, DifferentiableDielectric, DifferentiableLayered,
    DifferentiableResponse, DifferentiableThinFilm, GradientConfig, GradientVerification, Jacobian,
    JacobianBuilder, LayerConfig, ParameterGradients,
};

// Inverse material solver - parameter recovery from measurements
pub use glass_physics::{
    recover_ior_from_normal_reflectance, recover_roughness_from_glossiness, ConvergenceReason,
    InverseMaterialSolver, InverseResult, InverseSolverConfig, LossFunction, ReferenceData,
    ReferenceObservation,
};

// Calibration pipeline - multi-source material calibration
pub use glass_physics::{
    AggregatedLoss, BRDFObservation, BRDFSource, CalibrationSource, CombinedSource, LossAggregator,
    LossComponents, SpectralObservation, SpectralSource, TemporalObservation, TimeSeriesSource,
};

// Uncertainty estimation - confidence intervals and covariance
pub use glass_physics::{
    BootstrapConfig, BootstrapResampler, BootstrapResult, ConfidenceInterval, ConfidenceLevel,
    ConfidenceWarning, CovarianceEstimator, FisherInformationEstimator, FisherInformationMatrix,
    ParameterCovarianceMatrix, ParameterUncertainty, TwinConfidenceReport,
};

// Perceptual loss functions - LAB color space and Delta E
pub use glass_physics::{
    delta_e_2000, delta_e_76, delta_e_94, lab_to_rgb, lab_to_xyz, perceptual_loss,
    perceptual_loss_gradient, rgb_to_lab, rgb_to_xyz, xyz_to_lab, xyz_to_rgb, DeltaEFormula,
    Illuminant, LabColor, PerceptualLossConfig, XyzColor,
};

// Combined effects compositor
pub use glass_physics::{
    BlendMode, CombinedMaterial, CombinedMaterialBuilder, EffectLayer, RoughnessModel,
};

// Anisotropic BRDF - brushed metals, hair, fabric
pub use glass_physics::{
    anisotropy_strength, strength_to_alphas, AnisotropicConductor, AnisotropicGGX, FiberBSDF,
};

// Subsurface scattering - translucent materials
pub use glass_physics::{sss_presets, DiffusionBSSRDF, SubsurfaceBSDF, SubsurfaceParams};

// PBR API v1 - stable public interface
pub use glass_physics::{
    is_compatible, Layer, Material, MaterialBuilder, MaterialPreset, API_VERSION,
    API_VERSION_STRING,
};

// GPU backend configuration (feature-gated types available via glass_physics)
pub use glass_physics::{estimate_gpu_backend_memory, GpuBackendConfig, GpuBackendStats};

// ============================================================================
// Tier 4 - Scientific & Metrology APIs (certification, instruments, validation)
// ============================================================================

// Metrology - formal measurement system
pub use glass_physics::{
    Measurement, MeasurementArray, MeasurementId, MeasurementQuality, MeasurementSource,
    ToleranceBudget, ToleranceCategory, ToleranceComponent, TraceabilityChain, TraceabilityEntry,
    TraceabilityOperation, Uncertainty, Unit, UnitValue,
};

// Virtual instruments - simulation of physical measurement devices
pub use glass_physics::{
    EllipsometryResult, GoniometerResult, SpectroGeometry, SpectroResult,
    ThinFilmResult as EllipsometryThinFilmResult, VirtualEllipsometer, VirtualGonioreflectometer,
    VirtualSpectrophotometer,
};

// Certification system - material validation levels
pub use glass_physics::{
    can_achieve_level, highest_level, quick_certify_experimental, CertificationAuditor,
    CertificationLevel, CertificationMetrics, CertificationResult, LevelRequirements,
    MaterialAuditData,
};

// Ground truth validation
pub use glass_physics::{
    bk7_reference_data, gold_reference_data, silver_reference_data, GroundTruthDataset,
    GroundTruthValidator,
};

// Material export/import
pub use glass_physics::{
    ExportOptions, ExportTarget, GlslVersion, ImportAdapter, ImportError, ImportSource,
    MaterialDescriptor, MaterialExporter, MaterialImporter,
};

// Plugin system
pub use glass_physics::{
    DatasetPlugin, MaterialType, MetricPlugin, PluginError, PluginInfo, PluginInventory,
    PluginMaterialParams, PluginRegistry, PluginRenderOutput, RenderPlugin, PLUGIN_API_VERSION,
    PLUGIN_API_VERSION_STRING,
};

// Reference renderer - IEEE754 precision
pub use glass_physics::{
    PrecisionMode, ReferenceRenderConfig, ReferenceRenderResult, ReferenceRenderer,
};

// Spectral error metrics
pub use glass_physics::{
    compute_comprehensive, compute_perceptual_metrics, compute_spectral_metrics,
    ComprehensiveMetrics, EnergyMetrics, PerceptualErrorMetrics, PerceptualQualityGrade,
    SpectralErrorMetrics, SpectralQualityGrade, ValidationStatus,
};

// Scientific validation
pub use glass_physics::{
    airy_thin_film_reflectance, cauchy_dispersion_analytical, fresnel_conductor_exact,
    fresnel_dielectric_exact, rayleigh_scattering, sellmeier_dispersion,
    transfer_matrix_multilayer,
};

// ============================================================================
// Enterprise Phase 7 - Advanced Material Presets
// ============================================================================

// Advanced material presets catalog
pub use glass_physics::{
    advanced_material_catalog,
    // Technical coatings
    anti_reflective_coating,
    beetle_shell,
    // Automotive finishes
    car_paint_metallic,
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

// Anisotropic materials (brushed metals, hair, fabric)
pub use glass_physics::{
    anisotropic_material_presets, estimate_anisotropic_memory, AnisotropicBSDF,
    AshikhminShirleyBSDF, HairBSDF,
};

// Meta-materials (photonic crystals, structural color)
pub use glass_physics::{
    estimate_meta_materials_memory, meta_material_presets, DiffractionGrating, LatticeType,
    MaterialRef, NanostructureType, PhotonicCrystal, StructuralColor,
};

// Plasmonic materials (nanoparticles, LSPR)
pub use glass_physics::{
    estimate_plasmonic_memory, plasmonic_presets, ParticleShape, PlasmonicArray, PlasmonicFilm,
    PlasmonicMetalType, PlasmonicNanoparticle, PlasmonicOrdering,
};

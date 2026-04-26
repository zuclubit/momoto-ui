//! # Quality Tier System (Phase 2 + Phase 3 + Phase 4 + Phase 5 + Phase 8)
//!
//! Adaptive rendering quality based on device capabilities and material complexity.
//!
//! ## Overview
//!
//! The quality tier system allows Momoto to automatically select the appropriate
//! level of physical accuracy based on:
//!
//! 1. **Device Capabilities** - CPU speed, available memory, GPU presence
//! 2. **Material Complexity** - Dispersion, scattering, special effects
//! 3. **Performance Budget** - Target frame time, batch size
//! 4. **User Preferences** - Quality vs performance tradeoff
//!
//! ## Quality Tiers
//!
//! | Tier | Features | Performance | Use Case |
//! |------|----------|-------------|----------|
//! | Fast | Schlick, no spectral, compressed LUTs | >100M ops/s | Mobile, animations |
//! | Standard | RGB Fresnel, single H-G, basic metals | >50M ops/s | Desktop default |
//! | High | Full spectral, DHG, Phase 3+4 features | >20M ops/s | High-end displays |
//! | Reference | Full physics, no compression | >5M ops/s | Validation only |
//!
//! ## Phase 3 Features
//!
//! | Feature | Tier Required | Memory | Performance Impact |
//! |---------|--------------|--------|-------------------|
//! | Complex IOR (Metals) | Standard+ | ~1KB | +30% per metal |
//! | Mie Scattering | High+ | ~128KB | +50% vs H-G |
//! | Thin-Film Interference | High+ | ~0KB | +40% per interface |
//!
//! ## Phase 4 Features
//!
//! | Feature | Tier Required | Memory | Performance Impact |
//! |---------|--------------|--------|-------------------|
//! | LUT Compression | All | -60% | Same or better |
//! | Multi-Layer Thin-Film | High+ | +10KB | +80% per stack |
//! | Temperature Metals | Standard+ | +5KB | +20% per eval |
//! | Dynamic Mie | High+ | +20KB | +60% vs static |
//! | Polydisperse Scattering | Reference | +50KB | +200% vs mono |
//!
//! ## Phase 5 Features
//!
//! | Feature | Tier Required | Memory | Performance Impact |
//! |---------|--------------|--------|-------------------|
//! | Differentiable Rendering | High+ | ~5KB | +100% per calibration iter |
//! | Auto-Calibration | Reference | ~10KB | +500% for optimization |
//! | Dynamic Thin-Film | High+ | ~20KB | +150% vs static |
//! | Particle Physics | High+ | ~50KB | +300% vs static particles |
//! | Scattering Fields | Reference | ~200KB | +400% vs point samples |
//! | Oxidation Kinetics | Standard+ | ~8KB | +50% per time step |
//!
//! ## Phase 8 Features (Reference-Grade & Ecosystem)
//!
//! | Feature | Tier Required | Memory | Performance Impact |
//! |---------|--------------|--------|-------------------|
//! | Reference Renderer | Reference | ~8KB | +1000% (no LUTs) |
//! | External Validation | Reference | ~15KB | +200% per dataset |
//! | MERL BRDF Dataset | Reference | ~50KB | +100% per material |
//! | Material Export | All | ~20KB | Minimal |
//! | Material Import | All | ~10KB | Minimal |
//! | Plugin System | High+ | ~12KB | +10% per plugin |
//! | Research API (ML) | Reference | ~10KB | +50% for forward pass |
//! | Material Fingerprint | All | ~6KB | +5% per hash |
//!
//! ## Auto-Selection
//!
//! The tier is automatically selected based on:
//!
//! ```text
//! if is_mobile || animation_active:
//!     Fast
//! elif has_webgpu && high_refresh_display:
//!     High
//! elif material.has_dhg_scattering || material.has_metal || material.has_mie:
//!     High (required for accurate effects)
//! else:
//!     Standard
//! ```

use super::dispersion::DispersionModel;
use super::enhanced_presets::QualityTier;
use super::scattering::ScatteringParams;

// ============================================================================
// DEVICE CAPABILITIES
// ============================================================================

/// Device capability profile for quality tier selection
#[derive(Debug, Clone)]
pub struct DeviceCapabilities {
    /// Estimated CPU performance tier (1-10)
    pub cpu_tier: u8,
    /// Available memory in MB
    pub memory_mb: u32,
    /// GPU/WebGPU available
    pub has_gpu: bool,
    /// High refresh rate display (>60Hz)
    pub high_refresh: bool,
    /// Mobile device
    pub is_mobile: bool,
    /// Touch-primary interaction
    pub is_touch: bool,
    /// Reduced motion preference
    pub reduced_motion: bool,
    /// Power saver mode active
    pub power_saver: bool,
}

impl DeviceCapabilities {
    /// Default desktop profile
    pub const fn desktop() -> Self {
        Self {
            cpu_tier: 7,
            memory_mb: 8192,
            has_gpu: true,
            high_refresh: false,
            is_mobile: false,
            is_touch: false,
            reduced_motion: false,
            power_saver: false,
        }
    }

    /// Default mobile profile
    pub const fn mobile() -> Self {
        Self {
            cpu_tier: 4,
            memory_mb: 4096,
            has_gpu: true,
            high_refresh: false,
            is_mobile: true,
            is_touch: true,
            reduced_motion: false,
            power_saver: false,
        }
    }

    /// High-end desktop profile
    pub const fn high_end() -> Self {
        Self {
            cpu_tier: 9,
            memory_mb: 32768,
            has_gpu: true,
            high_refresh: true,
            is_mobile: false,
            is_touch: false,
            reduced_motion: false,
            power_saver: false,
        }
    }

    /// Low-end/embedded profile
    pub const fn low_end() -> Self {
        Self {
            cpu_tier: 2,
            memory_mb: 1024,
            has_gpu: false,
            high_refresh: false,
            is_mobile: true,
            is_touch: true,
            reduced_motion: true,
            power_saver: true,
        }
    }

    /// Compute a performance score (0.0 to 1.0)
    pub fn performance_score(&self) -> f64 {
        let mut score = 0.0;

        // CPU tier (40% weight)
        score += (self.cpu_tier as f64 / 10.0) * 0.4;

        // Memory (20% weight)
        let mem_score = (self.memory_mb as f64 / 16384.0).min(1.0);
        score += mem_score * 0.2;

        // GPU (20% weight)
        if self.has_gpu {
            score += 0.2;
        }

        // High refresh (10% weight)
        if self.high_refresh {
            score += 0.1;
        }

        // Penalties
        if self.is_mobile {
            score *= 0.8;
        }
        if self.power_saver {
            score *= 0.6;
        }
        if self.reduced_motion {
            score *= 0.9;
        }

        score.clamp(0.0, 1.0)
    }
}

impl Default for DeviceCapabilities {
    fn default() -> Self {
        Self::desktop()
    }
}

// ============================================================================
// MATERIAL COMPLEXITY
// ============================================================================

/// Material complexity analysis for tier selection
#[derive(Debug, Clone)]
pub struct MaterialComplexity {
    /// Material uses wavelength-dependent dispersion
    pub has_dispersion: bool,
    /// Uses Sellmeier (more expensive than Cauchy)
    pub uses_sellmeier: bool,
    /// Has double-lobe scattering (DHG)
    pub has_dhg: bool,
    /// Scattering intensity (0.0 to 1.0)
    pub scattering_intensity: f64,
    /// Number of light interactions to simulate
    pub bounce_count: u8,
    /// Requires spectral (per-channel) evaluation
    pub needs_spectral: bool,
    // Phase 3 features
    /// Material is a metal (requires complex IOR)
    pub is_metal: bool,
    /// Uses Mie scattering for particles
    pub has_mie: bool,
    /// Has thin-film interference coating
    pub has_thin_film: bool,
    /// Thin-film layer count (for multi-layer coatings)
    pub thin_film_layers: u8,
    // Phase 4 features
    /// Uses transfer matrix thin-film (multi-layer stacks)
    pub has_transfer_matrix_tf: bool,
    /// Uses temperature-dependent metal IOR
    pub has_temp_dependent_metal: bool,
    /// Uses dynamic Mie (polydisperse/anisotropic)
    pub has_dynamic_mie: bool,
    /// Uses polydisperse size distribution
    pub has_polydisperse: bool,
    /// Has oxidation layer effects
    pub has_oxidation: bool,
    // Phase 5 features
    /// Uses differentiable rendering for gradient computation
    pub has_differentiable: bool,
    /// Uses auto-calibration optimization
    pub has_auto_calibration: bool,
    /// Uses dynamic thin-film with deformations
    pub has_dynamic_thin_film: bool,
    /// Uses particle physics simulation
    pub has_particle_physics: bool,
    /// Uses 3D scattering field computation
    pub has_scattering_field: bool,
    /// Uses dynamic oxidation kinetics
    pub has_oxidation_kinetics: bool,
    // Phase 8 features
    /// Requires reference-grade rendering (no LUT approximations)
    pub requires_reference_mode: bool,
    /// Uses external validation datasets (MERL, etc.)
    pub uses_external_validation: bool,
    /// Uses plugin system for custom physics
    pub uses_plugins: bool,
    /// Uses research API for ML integration
    pub uses_research_api: bool,
}

impl MaterialComplexity {
    /// Analyze a material's dispersion model
    pub fn from_dispersion(dispersion: &DispersionModel) -> Self {
        let (has_dispersion, uses_sellmeier) = match dispersion {
            DispersionModel::None(_) => (false, false),
            DispersionModel::Cauchy(c) => (c.b != 0.0 || c.c != 0.0, false),
            DispersionModel::Sellmeier(_) => (true, true),
        };

        Self {
            has_dispersion,
            uses_sellmeier,
            has_dhg: false,
            scattering_intensity: 0.0,
            bounce_count: 1,
            needs_spectral: has_dispersion,
            // Phase 3 defaults
            is_metal: false,
            has_mie: false,
            has_thin_film: false,
            thin_film_layers: 0,
            // Phase 4 defaults
            has_transfer_matrix_tf: false,
            has_temp_dependent_metal: false,
            has_dynamic_mie: false,
            has_polydisperse: false,
            has_oxidation: false,
            // Phase 5 defaults
            has_differentiable: false,
            has_auto_calibration: false,
            has_dynamic_thin_film: false,
            has_particle_physics: false,
            has_scattering_field: false,
            has_oxidation_kinetics: false,
            // Phase 8 defaults
            requires_reference_mode: false,
            uses_external_validation: false,
            uses_plugins: false,
            uses_research_api: false,
        }
    }

    /// Analyze full material parameters
    pub fn from_material(dispersion: &DispersionModel, scattering: &ScatteringParams) -> Self {
        let mut complexity = Self::from_dispersion(dispersion);

        complexity.has_dhg = scattering.double_lobe;
        complexity.scattering_intensity = scattering.surface_scatter + scattering.volume_scatter;

        // DHG and high scattering require spectral for best results
        if complexity.has_dhg || complexity.scattering_intensity > 0.3 {
            complexity.needs_spectral = true;
        }

        complexity
    }

    /// Create complexity for a metallic material
    pub const fn metal() -> Self {
        Self {
            has_dispersion: true,
            uses_sellmeier: false,
            has_dhg: false,
            scattering_intensity: 0.0,
            bounce_count: 1,
            needs_spectral: true,
            is_metal: true,
            has_mie: false,
            has_thin_film: false,
            thin_film_layers: 0,
            // Phase 4
            has_transfer_matrix_tf: false,
            has_temp_dependent_metal: false,
            has_dynamic_mie: false,
            has_polydisperse: false,
            has_oxidation: false,
            // Phase 5
            has_differentiable: false,
            has_auto_calibration: false,
            has_dynamic_thin_film: false,
            has_particle_physics: false,
            has_scattering_field: false,
            has_oxidation_kinetics: false,
            // Phase 8
            requires_reference_mode: false,
            uses_external_validation: false,
            uses_plugins: false,
            uses_research_api: false,
        }
    }

    /// Create complexity for temperature-dependent metallic material (Phase 4)
    pub const fn temp_metal() -> Self {
        Self {
            has_dispersion: true,
            uses_sellmeier: false,
            has_dhg: false,
            scattering_intensity: 0.0,
            bounce_count: 1,
            needs_spectral: true,
            is_metal: true,
            has_mie: false,
            has_thin_film: false,
            thin_film_layers: 0,
            // Phase 4
            has_transfer_matrix_tf: false,
            has_temp_dependent_metal: true,
            has_dynamic_mie: false,
            has_polydisperse: false,
            has_oxidation: false,
            // Phase 5
            has_differentiable: false,
            has_auto_calibration: false,
            has_dynamic_thin_film: false,
            has_particle_physics: false,
            has_scattering_field: false,
            has_oxidation_kinetics: false,
            // Phase 8
            requires_reference_mode: false,
            uses_external_validation: false,
            uses_plugins: false,
            uses_research_api: false,
        }
    }

    /// Create complexity for oxidized metallic material (Phase 4)
    pub const fn oxidized_metal() -> Self {
        Self {
            has_dispersion: true,
            uses_sellmeier: false,
            has_dhg: false,
            scattering_intensity: 0.0,
            bounce_count: 1,
            needs_spectral: true,
            is_metal: true,
            has_mie: false,
            has_thin_film: true, // Oxide layer acts as thin-film
            thin_film_layers: 1,
            // Phase 4
            has_transfer_matrix_tf: false,
            has_temp_dependent_metal: true,
            has_dynamic_mie: false,
            has_polydisperse: false,
            has_oxidation: true,
            // Phase 5
            has_differentiable: false,
            has_auto_calibration: false,
            has_dynamic_thin_film: false,
            has_particle_physics: false,
            has_scattering_field: false,
            has_oxidation_kinetics: true, // Uses dynamic oxidation
            // Phase 8
            requires_reference_mode: false,
            uses_external_validation: false,
            uses_plugins: false,
            uses_research_api: false,
        }
    }

    /// Create complexity for particle scattering (fog, milk, etc.)
    pub const fn particle_scattering() -> Self {
        Self {
            has_dispersion: true,
            uses_sellmeier: false,
            has_dhg: false,
            scattering_intensity: 0.5,
            bounce_count: 2,
            needs_spectral: true,
            is_metal: false,
            has_mie: true,
            has_thin_film: false,
            thin_film_layers: 0,
            // Phase 4
            has_transfer_matrix_tf: false,
            has_temp_dependent_metal: false,
            has_dynamic_mie: false,
            has_polydisperse: false,
            has_oxidation: false,
            // Phase 5
            has_differentiable: false,
            has_auto_calibration: false,
            has_dynamic_thin_film: false,
            has_particle_physics: false,
            has_scattering_field: false,
            has_oxidation_kinetics: false,
            // Phase 8
            requires_reference_mode: false,
            uses_external_validation: false,
            uses_plugins: false,
            uses_research_api: false,
        }
    }

    /// Create complexity for dynamic particle scattering (Phase 4/5)
    pub const fn dynamic_scattering() -> Self {
        Self {
            has_dispersion: true,
            uses_sellmeier: false,
            has_dhg: false,
            scattering_intensity: 0.6,
            bounce_count: 2,
            needs_spectral: true,
            is_metal: false,
            has_mie: true,
            has_thin_film: false,
            thin_film_layers: 0,
            // Phase 4
            has_transfer_matrix_tf: false,
            has_temp_dependent_metal: false,
            has_dynamic_mie: true,
            has_polydisperse: true,
            has_oxidation: false,
            // Phase 5
            has_differentiable: false,
            has_auto_calibration: false,
            has_dynamic_thin_film: false,
            has_particle_physics: true, // Uses particle dynamics
            has_scattering_field: false,
            has_oxidation_kinetics: false,
            // Phase 8
            requires_reference_mode: false,
            uses_external_validation: false,
            uses_plugins: false,
            uses_research_api: false,
        }
    }

    /// Create complexity for iridescent material (thin-film)
    pub const fn iridescent(layers: u8) -> Self {
        Self {
            has_dispersion: true,
            uses_sellmeier: false,
            has_dhg: false,
            scattering_intensity: 0.0,
            bounce_count: 1,
            needs_spectral: true,
            is_metal: false,
            has_mie: false,
            has_thin_film: true,
            thin_film_layers: layers,
            // Phase 4
            has_transfer_matrix_tf: false,
            has_temp_dependent_metal: false,
            has_dynamic_mie: false,
            has_polydisperse: false,
            has_oxidation: false,
            // Phase 5
            has_differentiable: false,
            has_auto_calibration: false,
            has_dynamic_thin_film: false,
            has_particle_physics: false,
            has_scattering_field: false,
            has_oxidation_kinetics: false,
            // Phase 8
            requires_reference_mode: false,
            uses_external_validation: false,
            uses_plugins: false,
            uses_research_api: false,
        }
    }

    /// Create complexity for advanced multi-layer thin-film (Phase 4)
    pub const fn multi_layer_iridescent(layers: u8) -> Self {
        Self {
            has_dispersion: true,
            uses_sellmeier: false,
            has_dhg: false,
            scattering_intensity: 0.0,
            bounce_count: 1,
            needs_spectral: true,
            is_metal: false,
            has_mie: false,
            has_thin_film: true,
            thin_film_layers: layers,
            // Phase 4
            has_transfer_matrix_tf: true,
            has_temp_dependent_metal: false,
            has_dynamic_mie: false,
            has_polydisperse: false,
            has_oxidation: false,
            // Phase 5
            has_differentiable: false,
            has_auto_calibration: false,
            has_dynamic_thin_film: false,
            has_particle_physics: false,
            has_scattering_field: false,
            has_oxidation_kinetics: false,
            // Phase 8
            requires_reference_mode: false,
            uses_external_validation: false,
            uses_plugins: false,
            uses_research_api: false,
        }
    }

    /// Create complexity for dynamic iridescent surface (Phase 5)
    pub const fn dynamic_iridescent(layers: u8) -> Self {
        Self {
            has_dispersion: true,
            uses_sellmeier: false,
            has_dhg: false,
            scattering_intensity: 0.0,
            bounce_count: 1,
            needs_spectral: true,
            is_metal: false,
            has_mie: false,
            has_thin_film: true,
            thin_film_layers: layers,
            // Phase 4
            has_transfer_matrix_tf: true,
            has_temp_dependent_metal: false,
            has_dynamic_mie: false,
            has_polydisperse: false,
            has_oxidation: false,
            // Phase 5
            has_differentiable: false,
            has_auto_calibration: false,
            has_dynamic_thin_film: true, // Dynamic thin-film with deformations
            has_particle_physics: false,
            has_scattering_field: false,
            has_oxidation_kinetics: false,
            // Phase 8
            requires_reference_mode: false,
            uses_external_validation: false,
            uses_plugins: false,
            uses_research_api: false,
        }
    }

    /// Create complexity for auto-calibrated material (Phase 5)
    pub const fn auto_calibrated() -> Self {
        Self {
            has_dispersion: true,
            uses_sellmeier: true,
            has_dhg: false,
            scattering_intensity: 0.0,
            bounce_count: 1,
            needs_spectral: true,
            is_metal: false,
            has_mie: false,
            has_thin_film: false,
            thin_film_layers: 0,
            // Phase 4
            has_transfer_matrix_tf: false,
            has_temp_dependent_metal: false,
            has_dynamic_mie: false,
            has_polydisperse: false,
            has_oxidation: false,
            // Phase 5
            has_differentiable: true,
            has_auto_calibration: true,
            has_dynamic_thin_film: false,
            has_particle_physics: false,
            has_scattering_field: false,
            has_oxidation_kinetics: false,
            // Phase 8
            requires_reference_mode: false,
            uses_external_validation: false,
            uses_plugins: false,
            uses_research_api: true, // ML integration for calibration
        }
    }

    /// Create complexity for volumetric scattering (Phase 5)
    pub const fn volumetric_scattering() -> Self {
        Self {
            has_dispersion: true,
            uses_sellmeier: false,
            has_dhg: false,
            scattering_intensity: 0.8,
            bounce_count: 3,
            needs_spectral: true,
            is_metal: false,
            has_mie: true,
            has_thin_film: false,
            thin_film_layers: 0,
            // Phase 4
            has_transfer_matrix_tf: false,
            has_temp_dependent_metal: false,
            has_dynamic_mie: true,
            has_polydisperse: true,
            has_oxidation: false,
            // Phase 5
            has_differentiable: false,
            has_auto_calibration: false,
            has_dynamic_thin_film: false,
            has_particle_physics: true,
            has_scattering_field: true, // Full 3D scattering field
            has_oxidation_kinetics: false,
            // Phase 8
            requires_reference_mode: true, // Requires reference for accuracy
            uses_external_validation: false,
            uses_plugins: false,
            uses_research_api: false,
        }
    }

    /// Compute complexity score (0.0 to 1.0)
    pub fn complexity_score(&self) -> f64 {
        let mut score = 0.0;

        // Phase 1+2 features
        if self.has_dispersion {
            score += 0.10;
        }
        if self.uses_sellmeier {
            score += 0.08;
        }
        if self.has_dhg {
            score += 0.10;
        }
        if self.needs_spectral {
            score += 0.08;
        }

        score += self.scattering_intensity * 0.08;
        score += (self.bounce_count as f64 - 1.0) * 0.04;

        // Phase 3 features
        if self.is_metal {
            score += 0.10; // Complex Fresnel arithmetic
        }
        if self.has_mie {
            score += 0.10; // Mie LUT lookup + wavelength dependence
        }
        if self.has_thin_film {
            score += 0.08 + (self.thin_film_layers as f64 * 0.03); // Each layer adds cost
        }

        // Phase 4 features
        if self.has_transfer_matrix_tf {
            score += 0.10; // Transfer matrix computation
        }
        if self.has_temp_dependent_metal {
            score += 0.06; // Drude model evaluation
        }
        if self.has_dynamic_mie {
            score += 0.08; // Dynamic Mie with anisotropy
        }
        if self.has_polydisperse {
            score += 0.10; // Size distribution integration
        }
        if self.has_oxidation {
            score += 0.05; // Oxide layer thin-film effect
        }

        // Phase 5 features
        if self.has_differentiable {
            score += 0.08; // Gradient computation
        }
        if self.has_auto_calibration {
            score += 0.12; // Optimization loop
        }
        if self.has_dynamic_thin_film {
            score += 0.10; // Deformation + temperature effects
        }
        if self.has_particle_physics {
            score += 0.12; // Brownian, settling, coalescence
        }
        if self.has_scattering_field {
            score += 0.15; // 3D field computation
        }
        if self.has_oxidation_kinetics {
            score += 0.06; // Time evolution
        }

        // Phase 8 features
        if self.requires_reference_mode {
            score += 0.15; // Full precision rendering
        }
        if self.uses_external_validation {
            score += 0.08; // Dataset validation overhead
        }
        if self.uses_plugins {
            score += 0.04; // Plugin dispatch
        }
        if self.uses_research_api {
            score += 0.06; // ML integration
        }

        score.clamp(0.0, 1.0)
    }

    /// Simple complexity (no dispersion, no DHG, no Phase 3/4/5/8)
    pub const fn simple() -> Self {
        Self {
            has_dispersion: false,
            uses_sellmeier: false,
            has_dhg: false,
            scattering_intensity: 0.0,
            bounce_count: 1,
            needs_spectral: false,
            is_metal: false,
            has_mie: false,
            has_thin_film: false,
            thin_film_layers: 0,
            // Phase 4
            has_transfer_matrix_tf: false,
            has_temp_dependent_metal: false,
            has_dynamic_mie: false,
            has_polydisperse: false,
            has_oxidation: false,
            // Phase 5
            has_differentiable: false,
            has_auto_calibration: false,
            has_dynamic_thin_film: false,
            has_particle_physics: false,
            has_scattering_field: false,
            has_oxidation_kinetics: false,
            // Phase 8
            requires_reference_mode: false,
            uses_external_validation: false,
            uses_plugins: false,
            uses_research_api: false,
        }
    }

    /// Standard complexity (Cauchy dispersion, single H-G)
    pub const fn standard() -> Self {
        Self {
            has_dispersion: true,
            uses_sellmeier: false,
            has_dhg: false,
            scattering_intensity: 0.2,
            bounce_count: 1,
            needs_spectral: true,
            is_metal: false,
            has_mie: false,
            has_thin_film: false,
            thin_film_layers: 0,
            // Phase 4
            has_transfer_matrix_tf: false,
            has_temp_dependent_metal: false,
            has_dynamic_mie: false,
            has_polydisperse: false,
            has_oxidation: false,
            // Phase 5
            has_differentiable: false,
            has_auto_calibration: false,
            has_dynamic_thin_film: false,
            has_particle_physics: false,
            has_scattering_field: false,
            has_oxidation_kinetics: false,
            // Phase 8
            requires_reference_mode: false,
            uses_external_validation: false,
            uses_plugins: false,
            uses_research_api: false,
        }
    }

    /// High complexity (Sellmeier, DHG, Phase 3+4+5 features)
    pub const fn high() -> Self {
        Self {
            has_dispersion: true,
            uses_sellmeier: true,
            has_dhg: true,
            scattering_intensity: 0.5,
            bounce_count: 2,
            needs_spectral: true,
            is_metal: false,
            has_mie: true,       // Phase 3 Mie
            has_thin_film: true, // Phase 3 thin-film
            thin_film_layers: 1,
            // Phase 4
            has_transfer_matrix_tf: true,
            has_temp_dependent_metal: true,
            has_dynamic_mie: true,
            has_polydisperse: false,
            has_oxidation: true,
            // Phase 5
            has_differentiable: true,
            has_auto_calibration: false, // Only Reference tier
            has_dynamic_thin_film: true,
            has_particle_physics: true,
            has_scattering_field: false, // Only Reference tier
            has_oxidation_kinetics: true,
            // Phase 8
            requires_reference_mode: false,
            uses_external_validation: false,
            uses_plugins: true, // Plugin support enabled
            uses_research_api: false,
        }
    }

    /// Maximum complexity (all Phase 3+4+5+8 features enabled)
    pub const fn maximum() -> Self {
        Self {
            has_dispersion: true,
            uses_sellmeier: true,
            has_dhg: true,
            scattering_intensity: 0.7,
            bounce_count: 3,
            needs_spectral: true,
            is_metal: true,
            has_mie: true,
            has_thin_film: true,
            thin_film_layers: 4,
            // Phase 4 - all enabled
            has_transfer_matrix_tf: true,
            has_temp_dependent_metal: true,
            has_dynamic_mie: true,
            has_polydisperse: true,
            has_oxidation: true,
            // Phase 5 - all enabled
            has_differentiable: true,
            has_auto_calibration: true,
            has_dynamic_thin_film: true,
            has_particle_physics: true,
            has_scattering_field: true,
            has_oxidation_kinetics: true,
            // Phase 8 - all enabled
            requires_reference_mode: true,
            uses_external_validation: true,
            uses_plugins: true,
            uses_research_api: true,
        }
    }
}

impl Default for MaterialComplexity {
    fn default() -> Self {
        Self::standard()
    }
}

// ============================================================================
// QUALITY TIER SELECTOR
// ============================================================================

/// Configuration for quality tier selection
#[derive(Debug, Clone)]
pub struct QualityConfig {
    /// Device capabilities
    pub device: DeviceCapabilities,
    /// User preference override (None = auto)
    pub preferred_tier: Option<QualityTier>,
    /// Animation currently active
    pub animation_active: bool,
    /// Target frame time in ms (default 16.67 for 60fps)
    pub target_frame_ms: f64,
    /// Batch size (number of materials per frame)
    pub batch_size: u32,
    /// Allow automatic tier downgrade during animation
    pub allow_downgrade: bool,
}

impl QualityConfig {
    /// Create with defaults
    pub fn new() -> Self {
        Self {
            device: DeviceCapabilities::default(),
            preferred_tier: None,
            animation_active: false,
            target_frame_ms: 16.67,
            batch_size: 1,
            allow_downgrade: true,
        }
    }

    /// Set device capabilities
    pub fn with_device(mut self, device: DeviceCapabilities) -> Self {
        self.device = device;
        self
    }

    /// Set preferred tier
    pub fn with_preferred_tier(mut self, tier: QualityTier) -> Self {
        self.preferred_tier = Some(tier);
        self
    }

    /// Set animation state
    pub fn with_animation(mut self, active: bool) -> Self {
        self.animation_active = active;
        self
    }

    /// Set batch size
    pub fn with_batch_size(mut self, size: u32) -> Self {
        self.batch_size = size;
        self
    }
}

impl Default for QualityConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Select optimal quality tier
///
/// # Arguments
///
/// * `config` - Quality configuration
/// * `material_complexity` - Complexity of the material being rendered
///
/// # Returns
///
/// The recommended quality tier
pub fn select_tier(
    config: &QualityConfig,
    material_complexity: &MaterialComplexity,
) -> QualityTier {
    // User preference takes priority (unless animation requires downgrade)
    if let Some(preferred) = config.preferred_tier {
        if !config.animation_active || !config.allow_downgrade {
            return preferred;
        }
        // During animation, may downgrade from preferred
        if config.animation_active
            && matches!(
                preferred,
                QualityTier::High
                    | QualityTier::UltraHigh
                    | QualityTier::Experimental
                    | QualityTier::Reference
            )
        {
            return QualityTier::Standard;
        }
        return preferred;
    }

    // Compute scores
    let device_score = config.device.performance_score();
    let _complexity_score = material_complexity.complexity_score();

    // Estimate required operations per frame
    let ops_per_material = estimate_ops(material_complexity);
    let total_ops = ops_per_material * config.batch_size as u64;

    // Estimate available ops based on device and frame budget
    let max_ops = estimate_max_ops(&config.device, config.target_frame_ms);

    // Special cases
    if config.device.power_saver || config.device.reduced_motion {
        return QualityTier::Fast;
    }

    if config.animation_active && config.allow_downgrade {
        // During animation, prefer speed
        if config.device.is_mobile {
            return QualityTier::Fast;
        }
        return QualityTier::Standard;
    }

    // Reference tier only when explicitly requested
    if matches!(config.preferred_tier, Some(QualityTier::Reference)) {
        return QualityTier::Reference;
    }

    // Select based on device capability and material needs
    if device_score < 0.3 || total_ops > max_ops * 2 {
        QualityTier::Fast
    } else if device_score < 0.6 || total_ops > max_ops {
        QualityTier::Standard
    } else if material_complexity.has_dhg || material_complexity.uses_sellmeier {
        // Phase 1+2: High complexity materials benefit from High tier
        QualityTier::High
    } else if material_complexity.has_mie || material_complexity.has_thin_film {
        // Phase 3: Mie and thin-film require High tier
        QualityTier::High
    } else if material_complexity.is_metal && device_score > 0.5 {
        // Metals can use Standard tier (has metal_fresnel)
        QualityTier::Standard
    } else if device_score > 0.8 && config.device.has_gpu {
        QualityTier::High
    } else {
        QualityTier::Standard
    }
}

/// Estimate operations needed for a material
fn estimate_ops(complexity: &MaterialComplexity) -> u64 {
    let mut ops: u64 = 100; // Base Fresnel

    // Phase 1+2 features
    if complexity.has_dispersion {
        ops += 50; // RGB evaluation
    }

    if complexity.uses_sellmeier {
        ops += 100; // Sellmeier is more expensive
    }

    if complexity.has_dhg {
        ops += 200; // DHG evaluation
    } else if complexity.scattering_intensity > 0.0 {
        ops += 80; // Single H-G
    }

    if complexity.needs_spectral {
        ops += ops / 2; // Spectral adds ~50%
    }

    // Phase 3 features
    if complexity.is_metal {
        ops += 150; // Complex arithmetic for Fresnel
    }

    if complexity.has_mie {
        ops += 180; // Mie LUT lookup + interpolation
    }

    if complexity.has_thin_film {
        let layer_ops = 120 * complexity.thin_film_layers.max(1) as u64;
        ops += layer_ops; // Per-interface calculation
    }

    // Phase 4 features
    if complexity.has_transfer_matrix_tf {
        // Transfer matrix: 2x2 complex matrix per layer
        let tm_ops = 200 * complexity.thin_film_layers.max(1) as u64;
        ops += tm_ops;
    }

    if complexity.has_temp_dependent_metal {
        ops += 80; // Drude model evaluation
    }

    if complexity.has_oxidation {
        ops += 100; // Oxide thin-film effect
    }

    if complexity.has_dynamic_mie {
        ops += 120; // Anisotropic + temporal variation
    }

    if complexity.has_polydisperse {
        ops += 300; // Size distribution integration (expensive)
    }

    // Phase 5 features
    if complexity.has_differentiable {
        ops += 150; // Gradient computation overhead
    }

    if complexity.has_auto_calibration {
        ops += 500; // Full optimization loop
    }

    if complexity.has_dynamic_thin_film {
        ops += 200; // Deformation + temperature + curvature
    }

    if complexity.has_particle_physics {
        ops += 400; // Brownian + settling + coalescence
    }

    if complexity.has_scattering_field {
        ops += 600; // 3D field computation
    }

    if complexity.has_oxidation_kinetics {
        ops += 100; // Time evolution step
    }

    // Phase 8 features
    if complexity.requires_reference_mode {
        ops *= 2; // Reference mode is ~2x slower (no LUTs)
    }
    if complexity.uses_external_validation {
        ops += 200; // Dataset comparison overhead
    }
    if complexity.uses_plugins {
        ops += 50; // Plugin dispatch overhead
    }
    if complexity.uses_research_api {
        ops += 100; // Forward function evaluation
    }

    ops * complexity.bounce_count as u64
}

/// Estimate maximum operations per frame
fn estimate_max_ops(device: &DeviceCapabilities, target_frame_ms: f64) -> u64 {
    // Base estimate: 1M ops per ms on mid-tier device
    let base_ops_per_ms = 1_000_000u64;

    let cpu_multiplier = (device.cpu_tier as f64 / 5.0).max(0.5);
    let gpu_multiplier = if device.has_gpu { 1.5 } else { 1.0 };
    let mobile_multiplier = if device.is_mobile { 0.6 } else { 1.0 };

    let ops_per_ms = base_ops_per_ms as f64 * cpu_multiplier * gpu_multiplier * mobile_multiplier;

    (ops_per_ms * target_frame_ms * 0.5) as u64 // Use 50% of budget for materials
}

// ============================================================================
// TIER FEATURE SET
// ============================================================================

/// Features enabled for each quality tier
#[derive(Debug, Clone)]
pub struct TierFeatures {
    /// Use spectral (RGB) Fresnel
    pub spectral_fresnel: bool,
    /// Use Sellmeier dispersion (vs Cauchy)
    pub sellmeier_dispersion: bool,
    /// Use DHG scattering (vs single H-G)
    pub dhg_scattering: bool,
    /// Use LUT acceleration
    pub use_luts: bool,
    /// Number of light bounces
    pub max_bounces: u8,
    /// CSS gradient complexity (stops)
    pub max_gradient_stops: u8,
    /// Enable chromatic aberration effects
    pub chromatic_effects: bool,
    // Phase 3 features
    /// Use complex IOR for metals (vs Schlick approximation)
    pub metal_fresnel: bool,
    /// Use Mie scattering for particles (vs H-G approximation)
    pub mie_scattering: bool,
    /// Enable thin-film interference effects
    pub thin_film_interference: bool,
    /// Maximum thin-film layers to compute
    pub max_thin_film_layers: u8,
    // Phase 4 features
    /// Use compressed LUTs for memory optimization
    pub use_compressed_luts: bool,
    /// Use transfer matrix method for multi-layer thin-film
    pub transfer_matrix_thin_film: bool,
    /// Enable temperature-dependent metal IOR (Drude model)
    pub temp_dependent_metals: bool,
    /// Enable oxidation layer effects on metals
    pub oxidation_effects: bool,
    /// Use dynamic Mie scattering (anisotropic, temporal)
    pub dynamic_mie: bool,
    /// Use polydisperse size distributions
    pub polydisperse_scattering: bool,
    // Phase 5 features
    /// Enable differentiable rendering for gradient computation
    pub differentiable_rendering: bool,
    /// Enable auto-calibration optimization
    pub auto_calibration: bool,
    /// Enable dynamic thin-film with physical deformations
    pub dynamic_thin_film: bool,
    /// Enable particle physics simulation
    pub particle_physics: bool,
    /// Enable 3D scattering field computation
    pub scattering_fields: bool,
    /// Enable oxidation kinetics time evolution
    pub oxidation_kinetics: bool,
    // Phase 8 features
    /// Enable reference-grade rendering (IEEE754 full precision)
    pub reference_mode: bool,
    /// Enable external dataset validation (MERL, etc.)
    pub external_validation: bool,
    /// Enable plugin system for custom physics
    pub plugin_support: bool,
    /// Enable research API for ML integration
    pub research_api: bool,
    /// Enable material fingerprinting for reproducibility
    pub material_fingerprinting: bool,
    /// Enable material export (GLSL/WGSL/MaterialX)
    pub material_export: bool,
    /// Enable material import (MaterialX/glTF/JSON)
    pub material_import: bool,
}

impl TierFeatures {
    /// Get features for a quality tier
    pub fn for_tier(tier: QualityTier) -> Self {
        match tier {
            QualityTier::Fast => Self {
                spectral_fresnel: false,
                sellmeier_dispersion: false,
                dhg_scattering: false,
                use_luts: true,
                max_bounces: 1,
                max_gradient_stops: 4,
                chromatic_effects: false,
                // Phase 3: Minimal support
                metal_fresnel: false,  // Use Schlick approximation
                mie_scattering: false, // Use H-G approximation
                thin_film_interference: false,
                max_thin_film_layers: 0,
                // Phase 4: Memory optimization only
                use_compressed_luts: true, // Always use compressed for Fast
                transfer_matrix_thin_film: false,
                temp_dependent_metals: false,
                oxidation_effects: false,
                dynamic_mie: false,
                polydisperse_scattering: false,
                // Phase 5: None
                differentiable_rendering: false,
                auto_calibration: false,
                dynamic_thin_film: false,
                particle_physics: false,
                scattering_fields: false,
                oxidation_kinetics: false,
                // Phase 8: Basic export/import only
                reference_mode: false,
                external_validation: false,
                plugin_support: false,
                research_api: false,
                material_fingerprinting: true, // Always enabled
                material_export: true,         // Always enabled
                material_import: true,         // Always enabled
            },
            QualityTier::Standard => Self {
                spectral_fresnel: true,
                sellmeier_dispersion: false,
                dhg_scattering: false,
                use_luts: true,
                max_bounces: 1,
                max_gradient_stops: 8,
                chromatic_effects: true,
                // Phase 3: Basic support
                metal_fresnel: true,   // Full complex Fresnel for metals
                mie_scattering: false, // Use H-G approximation
                thin_film_interference: false,
                max_thin_film_layers: 0,
                // Phase 4: Basic features
                use_compressed_luts: true,
                transfer_matrix_thin_film: false,
                temp_dependent_metals: true, // Basic Drude model
                oxidation_effects: false,
                dynamic_mie: false,
                polydisperse_scattering: false,
                // Phase 5: Basic oxidation kinetics only
                differentiable_rendering: false,
                auto_calibration: false,
                dynamic_thin_film: false,
                particle_physics: false,
                scattering_fields: false,
                oxidation_kinetics: true, // Basic time evolution
                // Phase 8: Basic features
                reference_mode: false,
                external_validation: false,
                plugin_support: false,
                research_api: false,
                material_fingerprinting: true,
                material_export: true,
                material_import: true,
            },
            QualityTier::High => Self {
                spectral_fresnel: true,
                sellmeier_dispersion: true,
                dhg_scattering: true,
                use_luts: true,
                max_bounces: 2,
                max_gradient_stops: 12,
                chromatic_effects: true,
                // Phase 3: Full support
                metal_fresnel: true,
                mie_scattering: true,
                thin_film_interference: true,
                max_thin_film_layers: 4,
                // Phase 4: Advanced features
                use_compressed_luts: true,
                transfer_matrix_thin_film: true,
                temp_dependent_metals: true,
                oxidation_effects: true,
                dynamic_mie: true,
                polydisperse_scattering: false, // Too expensive for High
                // Phase 5: Most features enabled
                differentiable_rendering: true,
                auto_calibration: false, // Only Reference tier
                dynamic_thin_film: true,
                particle_physics: true,
                scattering_fields: false, // Too expensive for High
                oxidation_kinetics: true,
                // Phase 8: Plugin support enabled
                reference_mode: false,
                external_validation: false,
                plugin_support: true, // Plugin system enabled
                research_api: false,
                material_fingerprinting: true,
                material_export: true,
                material_import: true,
            },
            QualityTier::UltraHigh => Self {
                // Phase 6: Research-grade with all optimizations
                spectral_fresnel: true,
                sellmeier_dispersion: true,
                dhg_scattering: true,
                use_luts: true,
                max_bounces: 3,
                max_gradient_stops: 16,
                chromatic_effects: true,
                // Phase 3: Full support
                metal_fresnel: true,
                mie_scattering: true,
                thin_film_interference: true,
                max_thin_film_layers: 8,
                // Phase 4: All advanced features
                use_compressed_luts: true,
                transfer_matrix_thin_film: true,
                temp_dependent_metals: true,
                oxidation_effects: true,
                dynamic_mie: true,
                polydisperse_scattering: true, // Enabled for UltraHigh
                // Phase 5: All features enabled
                differentiable_rendering: true,
                auto_calibration: true, // Enabled for UltraHigh
                dynamic_thin_film: true,
                particle_physics: true,
                scattering_fields: true, // Enabled for UltraHigh
                oxidation_kinetics: true,
                // Phase 8: Most features enabled
                reference_mode: false,
                external_validation: true, // Validation enabled
                plugin_support: true,
                research_api: true, // ML integration
                material_fingerprinting: true,
                material_export: true,
                material_import: true,
            },
            QualityTier::Experimental => Self {
                // Phase 7: Ultra-realistic with advanced parallelization
                spectral_fresnel: true,
                sellmeier_dispersion: true,
                dhg_scattering: true,
                use_luts: true,
                max_bounces: 4,
                max_gradient_stops: 24,
                chromatic_effects: true,
                // Phase 3: Full support
                metal_fresnel: true,
                mie_scattering: true,
                thin_film_interference: true,
                max_thin_film_layers: 12,
                // Phase 4: All advanced features
                use_compressed_luts: true,
                transfer_matrix_thin_film: true,
                temp_dependent_metals: true,
                oxidation_effects: true,
                dynamic_mie: true,
                polydisperse_scattering: true,
                // Phase 5: All features enabled
                differentiable_rendering: true,
                auto_calibration: true,
                dynamic_thin_film: true,
                particle_physics: true,
                scattering_fields: true,
                oxidation_kinetics: true,
                // Phase 8: All features enabled
                reference_mode: false, // Still use LUTs for performance
                external_validation: true,
                plugin_support: true,
                research_api: true,
                material_fingerprinting: true,
                material_export: true,
                material_import: true,
            },
            QualityTier::Reference => Self {
                spectral_fresnel: true,
                sellmeier_dispersion: true,
                dhg_scattering: true,
                use_luts: false, // Direct calculation for accuracy
                max_bounces: 4,
                max_gradient_stops: 16,
                chromatic_effects: true,
                // Phase 3: Maximum quality
                metal_fresnel: true,
                mie_scattering: true,
                thin_film_interference: true,
                max_thin_film_layers: 16, // Unlimited multi-layer coatings
                // Phase 4: All features enabled
                use_compressed_luts: false, // Full precision for reference
                transfer_matrix_thin_film: true,
                temp_dependent_metals: true,
                oxidation_effects: true,
                dynamic_mie: true,
                polydisperse_scattering: true, // Full polydisperse support
                // Phase 5: All features enabled
                differentiable_rendering: true,
                auto_calibration: true,
                dynamic_thin_film: true,
                particle_physics: true,
                scattering_fields: true,
                oxidation_kinetics: true,
                // Phase 8: All features enabled (reference-grade)
                reference_mode: true,      // IEEE754 full precision, no LUTs
                external_validation: true, // MERL and external datasets
                plugin_support: true,
                research_api: true, // ML integration
                material_fingerprinting: true,
                material_export: true,
                material_import: true,
            },
        }
    }

    /// Check if this tier can render a material at full quality
    pub fn can_render_full_quality(&self, complexity: &MaterialComplexity) -> bool {
        // Phase 1+2 checks
        if complexity.uses_sellmeier && !self.sellmeier_dispersion {
            return false;
        }
        if complexity.has_dhg && !self.dhg_scattering {
            return false;
        }
        if complexity.needs_spectral && !self.spectral_fresnel {
            return false;
        }
        if complexity.bounce_count > self.max_bounces {
            return false;
        }

        // Phase 3 checks
        if complexity.is_metal && !self.metal_fresnel {
            return false;
        }
        if complexity.has_mie && !self.mie_scattering {
            return false;
        }
        if complexity.has_thin_film && !self.thin_film_interference {
            return false;
        }
        if complexity.thin_film_layers > self.max_thin_film_layers {
            return false;
        }

        // Phase 4 checks
        if complexity.has_transfer_matrix_tf && !self.transfer_matrix_thin_film {
            return false;
        }
        if complexity.has_temp_dependent_metal && !self.temp_dependent_metals {
            return false;
        }
        if complexity.has_oxidation && !self.oxidation_effects {
            return false;
        }
        if complexity.has_dynamic_mie && !self.dynamic_mie {
            return false;
        }
        if complexity.has_polydisperse && !self.polydisperse_scattering {
            return false;
        }

        // Phase 5 checks
        if complexity.has_differentiable && !self.differentiable_rendering {
            return false;
        }
        if complexity.has_auto_calibration && !self.auto_calibration {
            return false;
        }
        if complexity.has_dynamic_thin_film && !self.dynamic_thin_film {
            return false;
        }
        if complexity.has_particle_physics && !self.particle_physics {
            return false;
        }
        if complexity.has_scattering_field && !self.scattering_fields {
            return false;
        }
        if complexity.has_oxidation_kinetics && !self.oxidation_kinetics {
            return false;
        }

        // Phase 8 checks
        if complexity.requires_reference_mode && !self.reference_mode {
            return false;
        }
        if complexity.uses_external_validation && !self.external_validation {
            return false;
        }
        if complexity.uses_plugins && !self.plugin_support {
            return false;
        }
        if complexity.uses_research_api && !self.research_api {
            return false;
        }

        true
    }
}

// ============================================================================
// PERFORMANCE METRICS
// ============================================================================

/// Performance metrics for quality tier evaluation
#[derive(Debug, Clone, Default)]
pub struct TierMetrics {
    /// Average evaluation time in nanoseconds
    pub avg_eval_ns: f64,
    /// Peak evaluation time
    pub peak_eval_ns: f64,
    /// Memory usage in bytes
    pub memory_bytes: usize,
    /// Operations per second achieved
    pub throughput: f64,
    /// Frame time budget used (percentage)
    pub budget_usage: f64,
}

impl TierMetrics {
    /// Check if tier is performing within budget
    pub fn is_within_budget(&self, target_ms: f64) -> bool {
        self.budget_usage < 100.0 && self.avg_eval_ns * 1e-6 < target_ms
    }

    /// Suggest tier adjustment based on metrics
    pub fn suggest_adjustment(&self, current_tier: QualityTier) -> Option<QualityTier> {
        if self.budget_usage > 120.0 {
            // Over budget, suggest downgrade
            match current_tier {
                QualityTier::Reference => Some(QualityTier::Experimental),
                QualityTier::Experimental => Some(QualityTier::UltraHigh),
                QualityTier::UltraHigh => Some(QualityTier::High),
                QualityTier::High => Some(QualityTier::Standard),
                QualityTier::Standard => Some(QualityTier::Fast),
                QualityTier::Fast => None, // Can't go lower
            }
        } else if self.budget_usage < 30.0 {
            // Under-utilizing, suggest upgrade
            match current_tier {
                QualityTier::Fast => Some(QualityTier::Standard),
                QualityTier::Standard => Some(QualityTier::High),
                QualityTier::High => Some(QualityTier::UltraHigh),
                QualityTier::UltraHigh => Some(QualityTier::Experimental),
                QualityTier::Experimental => None, // Don't auto-upgrade to Reference
                QualityTier::Reference => None,
            }
        } else {
            None
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_performance_scores() {
        let high_end = DeviceCapabilities::high_end();
        let desktop = DeviceCapabilities::desktop();
        let mobile = DeviceCapabilities::mobile();
        let low_end = DeviceCapabilities::low_end();

        assert!(high_end.performance_score() > desktop.performance_score());
        assert!(desktop.performance_score() > mobile.performance_score());
        assert!(mobile.performance_score() > low_end.performance_score());
    }

    #[test]
    fn test_material_complexity_scores() {
        let simple = MaterialComplexity::simple();
        let standard = MaterialComplexity::standard();
        let high = MaterialComplexity::high();

        assert!(simple.complexity_score() < standard.complexity_score());
        assert!(standard.complexity_score() < high.complexity_score());
    }

    #[test]
    fn test_tier_selection_mobile() {
        let config = QualityConfig::new()
            .with_device(DeviceCapabilities::mobile())
            .with_animation(true);

        let tier = select_tier(&config, &MaterialComplexity::standard());
        assert_eq!(tier, QualityTier::Fast);
    }

    #[test]
    fn test_tier_selection_high_end() {
        let config = QualityConfig::new().with_device(DeviceCapabilities::high_end());

        let complexity = MaterialComplexity::high();
        let tier = select_tier(&config, &complexity);
        assert_eq!(tier, QualityTier::High);
    }

    #[test]
    fn test_tier_selection_preferred() {
        let config = QualityConfig::new().with_preferred_tier(QualityTier::Reference);

        let tier = select_tier(&config, &MaterialComplexity::simple());
        assert_eq!(tier, QualityTier::Reference);
    }

    #[test]
    fn test_tier_features() {
        let fast = TierFeatures::for_tier(QualityTier::Fast);
        let high = TierFeatures::for_tier(QualityTier::High);

        assert!(!fast.spectral_fresnel);
        assert!(!fast.dhg_scattering);

        assert!(high.spectral_fresnel);
        assert!(high.dhg_scattering);
        assert!(high.sellmeier_dispersion);
    }

    #[test]
    fn test_tier_can_render() {
        let fast = TierFeatures::for_tier(QualityTier::Fast);
        let high = TierFeatures::for_tier(QualityTier::High);

        let dhg_material = MaterialComplexity::high();

        assert!(!fast.can_render_full_quality(&dhg_material));
        assert!(high.can_render_full_quality(&dhg_material));
    }

    #[test]
    fn test_metrics_adjustment() {
        let mut metrics = TierMetrics::default();

        // Over budget
        metrics.budget_usage = 150.0;
        assert_eq!(
            metrics.suggest_adjustment(QualityTier::High),
            Some(QualityTier::Standard)
        );

        // Under budget
        metrics.budget_usage = 20.0;
        assert_eq!(
            metrics.suggest_adjustment(QualityTier::Fast),
            Some(QualityTier::Standard)
        );

        // Within budget
        metrics.budget_usage = 50.0;
        assert_eq!(metrics.suggest_adjustment(QualityTier::Standard), None);
    }
}

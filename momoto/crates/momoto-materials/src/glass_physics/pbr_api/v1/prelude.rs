//! # PBR API v1 Prelude
//!
//! Common imports for working with the PBR API.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use momoto_materials::glass_physics::pbr_api::v1::prelude::*;
//! ```

// Material types
pub use super::material::{Layer, Material, MaterialBuilder, MaterialPreset};

// BSDF types
pub use super::bsdf::{
    BSDFResponse, BSDFSample, ConductorBSDF, DielectricBSDF, EnergyValidation, LambertianBSDF,
    LayeredBSDF, ThinFilmBSDF, BSDF,
};

// Context types
pub use super::context::{EvaluationContext, Vector3};

// Quality tiers
pub use super::super::super::enhanced_presets::QualityTier;

// Anisotropic
pub use super::AnisotropicGGX;

// Version info
pub use super::{is_compatible, API_VERSION, API_VERSION_STRING};

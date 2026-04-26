//! # Advanced Material Presets (Enterprise Phase 7)
//!
//! Extended material presets for architectural glass, automotive, natural materials,
//! and technical coatings. These presets utilize the Enterprise Phase 7 modules:
//! - Anisotropic BSDF (brushed metals, hair, fabric)
//! - Meta-materials (photonic crystals, structural color)
//! - Plasmonic materials (nanoparticle effects)
//!
//! ## Categories
//!
//! - **Architectural Glass**: Low-E coatings, electrochromic, smart glass
//! - **Automotive**: Metallic paint, pearlescent, chrome finishes
//! - **Natural Materials**: Opal, mother of pearl, beetle shell
//! - **Technical Coatings**: Anti-reflective, dichroic filters, holographic

#![allow(dead_code, unused_imports, unused_variables)]

use crate::glass_physics::{
    anisotropic::AnisotropicBSDF,
    complex_ior::metals,
    meta_materials::{
        DiffractionGrating, LatticeType, MaterialRef, NanostructureType, PhotonicCrystal,
        StructuralColor,
    },
    thin_film::{ThinFilm, ThinFilmStack},
    unified_bsdf::{
        BSDFContext, BSDFResponse, BSDFSample, ConductorBSDF, DielectricBSDF, LayeredBSDF,
        ThinFilmBSDF, BSDF,
    },
};

// ============================================================================
// Material Preset Enum and Trait
// ============================================================================

/// Categories of advanced materials
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AdvancedMaterialCategory {
    /// Architectural glass (low-E, electrochromic, smart glass)
    Architectural,
    /// Automotive finishes (metallic paint, pearlescent, chrome)
    Automotive,
    /// Natural materials (opal, pearl, beetle shell)
    Natural,
    /// Technical coatings (AR, dichroic, holographic)
    Technical,
}

impl std::fmt::Display for AdvancedMaterialCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Architectural => write!(f, "architectural"),
            Self::Automotive => write!(f, "automotive"),
            Self::Natural => write!(f, "natural"),
            Self::Technical => write!(f, "technical"),
        }
    }
}

/// Information about an advanced material preset
#[derive(Debug, Clone)]
pub struct AdvancedMaterialInfo {
    /// Unique identifier for the preset
    pub id: &'static str,
    /// Human-readable name
    pub name: &'static str,
    /// Description of the material
    pub description: &'static str,
    /// Category
    pub category: AdvancedMaterialCategory,
    /// Representative IOR (for display purposes)
    pub representative_ior: f64,
    /// Whether the material has angle-dependent color
    pub has_iridescence: bool,
    /// Whether the material has subsurface scattering
    pub has_scattering: bool,
    /// Approximate memory usage in bytes
    pub memory_bytes: usize,
}

// ============================================================================
// Architectural Glass
// ============================================================================

/// Low-E (low emissivity) coating for energy-efficient windows.
///
/// Consists of a thin metallic oxide layer that reflects infrared
/// while transmitting visible light.
pub fn low_e_coating() -> LayeredBSDF {
    // Silver-based Low-E with protective dielectric layers
    let base_glass = DielectricBSDF::new(1.52, 0.0); // Float glass

    // Simplified silver layer using preset
    let silver = ConductorBSDF::silver();

    // Thin film interference from oxide layers (substrate, film, thickness)
    let oxide_layer = ThinFilmBSDF::new(1.52, 1.8, 80.0); // Tin oxide, ~80nm

    LayeredBSDF::new()
        .push(Box::new(oxide_layer))
        .push(Box::new(silver))
        .push(Box::new(base_glass))
}

/// Electrochromic glass that changes tint with applied voltage.
///
/// # Arguments
/// * `tint_level` - Tint level from 0.0 (clear) to 1.0 (fully tinted)
pub fn electrochromic_glass(tint_level: f64) -> DielectricBSDF {
    // Model as variable absorption dielectric
    // Electrochromic effect modeled via roughness (simplified)
    let tint = tint_level.clamp(0.0, 1.0);
    let effective_ior = 1.52 + tint * 0.05; // Slight IOR change with tinting
    let roughness = tint * 0.1; // Tinting adds slight haze

    DielectricBSDF::new(effective_ior, roughness)
}

/// Smart glass with PDLC (Polymer Dispersed Liquid Crystal).
///
/// When voltage is applied, liquid crystals align and glass becomes clear.
/// Without voltage, scattering from misaligned crystals makes it opaque.
///
/// # Arguments
/// * `voltage` - Applied voltage normalized 0.0 (opaque) to 1.0 (clear)
pub fn smart_glass_pdlc(voltage: f64) -> DielectricBSDF {
    let v = voltage.clamp(0.0, 1.0);

    // Scattering decreases as voltage aligns LC droplets
    let scattering = 1.0 - v;
    let roughness = scattering * 0.8; // High roughness when opaque

    DielectricBSDF::new(1.52, roughness)
}

// ============================================================================
// Automotive
// ============================================================================

/// Metallic car paint with colored flakes.
///
/// Three-layer system: clear coat, metallic base coat, primer.
pub fn car_paint_metallic(base_hue: f64, flake_density: f64) -> LayeredBSDF {
    let density = flake_density.clamp(0.0, 1.0);

    // Clear coat (high gloss)
    let clear_coat = DielectricBSDF::new(1.5, 0.0);

    // Metallic flakes modeled as anisotropic (random orientations)
    let flake_layer = AnisotropicBSDF::new(
        0.1 + density * 0.2, // alpha_x
        0.1 + density * 0.2, // alpha_y
        2.5,                 // metallic IOR (aluminum-like)
        base_hue,            // rotation based on color
    );

    // Base color coat
    let base = DielectricBSDF::new(1.45, 0.05);

    LayeredBSDF::new()
        .push(Box::new(clear_coat))
        .push(Box::new(flake_layer))
        .push(Box::new(base))
}

/// Pearlescent car paint with interference pigments.
///
/// Mica flakes coated with metal oxide create angle-dependent color shifts.
pub fn pearlescent_paint(base_hue: f64) -> LayeredBSDF {
    // Clear coat
    let clear_coat = DielectricBSDF::new(1.5, 0.0);

    // Interference layer (mica + metal oxide)
    // ThinFilmBSDF::new(substrate_ior, film_ior, film_thickness)
    let interference = ThinFilmBSDF::new(
        1.58,             // Mica substrate
        2.2,              // TiO2 coating
        120.0 + base_hue, // Film thickness varies with color
    );

    // Mica substrate
    let mica = DielectricBSDF::new(1.58, 0.02);

    LayeredBSDF::new()
        .push(Box::new(clear_coat))
        .push(Box::new(interference))
        .push(Box::new(mica))
}

/// Chrome mirror finish.
pub fn chrome_finish() -> ConductorBSDF {
    // Use built-in chrome preset
    ConductorBSDF::chrome()
}

// ============================================================================
// Natural Materials
// ============================================================================

/// Opal with play-of-color from silica sphere diffraction.
pub fn opal() -> StructuralColor {
    StructuralColor::new(
        NanostructureType::PhotonicCrystal(
            PhotonicCrystal::new(
                LatticeType::Hexagonal,
                250.0, // ~250nm period for visible diffraction
                0.74,  // Close-packed spheres
            )
            .with_high_material(MaterialRef::Dielectric { ior: 1.45 }) // Silica
            .with_low_material(MaterialRef::Air),
        ),
        MaterialRef::Dielectric { ior: 1.45 }, // Silica substrate
    )
}

/// Mother of pearl (nacre) with iridescent layered aragonite.
pub fn mother_of_pearl() -> LayeredBSDF {
    // Nacre has ~300-500nm aragonite tablets separated by organic layers
    let aragonite = ThinFilmBSDF::new(1.45, 1.68, 400.0);
    let organic = DielectricBSDF::new(1.45, 0.01);
    let aragonite2 = ThinFilmBSDF::new(1.45, 1.68, 400.0);

    LayeredBSDF::new()
        .push(Box::new(aragonite))
        .push(Box::new(organic))
        .push(Box::new(aragonite2))
}

/// Beetle shell (scarab) with structural coloration.
///
/// Helicoidal chitin structure creates circularly polarized reflection.
pub fn beetle_shell() -> StructuralColor {
    StructuralColor::new(
        NanostructureType::ThinFilmStack {
            layers: vec![
                (120.0, 1.56), // Chitin layer
                (80.0, 1.45),  // Spacing
                (120.0, 1.56), // Chitin layer
                (80.0, 1.45),  // Spacing
            ],
        },
        MaterialRef::Dielectric { ior: 1.56 }, // Chitin substrate
    )
}

// ============================================================================
// Technical Coatings
// ============================================================================

/// Multi-layer anti-reflective coating.
///
/// # Arguments
/// * `layers` - Number of layers (more = broader bandwidth AR)
pub fn anti_reflective_coating(layers: usize) -> ThinFilmStack {
    let layer_count = layers.clamp(1, 8);

    // V-coat or multi-layer design
    let mut film_layers = Vec::with_capacity(layer_count);

    // Alternate high/low index materials
    for i in 0..layer_count {
        let (n, d) = if i % 2 == 0 {
            (1.38, 100.0) // MgF2 (low index)
        } else {
            (2.3, 80.0) // TiO2 (high index)
        };
        film_layers.push(ThinFilm::new(n, d));
    }

    ThinFilmStack::new(film_layers, 1.52)
}

/// Dichroic filter that passes a specific wavelength band.
///
/// # Arguments
/// * `pass_band` - Tuple of (min_nm, max_nm) for the pass band
pub fn dichroic_filter(pass_band: (f64, f64)) -> ThinFilmStack {
    let center = (pass_band.0 + pass_band.1) / 2.0;
    let _bandwidth = pass_band.1 - pass_band.0;

    // Simplified bandpass filter using interference
    let qwot = center / 4.0; // Quarter-wave optical thickness

    let layers = vec![
        ThinFilm::new(2.3, qwot),       // High index
        ThinFilm::new(1.38, qwot),      // Low index
        ThinFilm::new(2.3, qwot * 2.0), // Cavity
        ThinFilm::new(1.38, qwot),
        ThinFilm::new(2.3, qwot),
    ];

    ThinFilmStack::new(layers, 1.52)
}

/// Holographic diffraction grating.
pub fn holographic() -> DiffractionGrating {
    // DiffractionGrating::new(period, depth)
    // 1000nm period = 1000 lines/mm
    DiffractionGrating::new(1000.0, 100.0).with_blaze(0.3)
}

// ============================================================================
// Catalog and Memory Estimation
// ============================================================================

/// Get information about all available advanced material presets.
pub fn catalog() -> Vec<AdvancedMaterialInfo> {
    vec![
        // Architectural
        AdvancedMaterialInfo {
            id: "low_e_coating",
            name: "Low-E Glass",
            description: "Energy-efficient glass with infrared-reflective coating",
            category: AdvancedMaterialCategory::Architectural,
            representative_ior: 1.52,
            has_iridescence: false,
            has_scattering: false,
            memory_bytes: estimate_low_e_memory(),
        },
        AdvancedMaterialInfo {
            id: "electrochromic_glass",
            name: "Electrochromic Glass",
            description: "Smart glass that changes tint with voltage",
            category: AdvancedMaterialCategory::Architectural,
            representative_ior: 1.52,
            has_iridescence: false,
            has_scattering: true,
            memory_bytes: estimate_electrochromic_memory(),
        },
        AdvancedMaterialInfo {
            id: "smart_glass_pdlc",
            name: "PDLC Smart Glass",
            description: "Privacy glass with polymer dispersed liquid crystal",
            category: AdvancedMaterialCategory::Architectural,
            representative_ior: 1.52,
            has_iridescence: false,
            has_scattering: true,
            memory_bytes: estimate_pdlc_memory(),
        },
        // Automotive
        AdvancedMaterialInfo {
            id: "car_paint_metallic",
            name: "Metallic Car Paint",
            description: "Three-layer metallic paint with aluminum flakes",
            category: AdvancedMaterialCategory::Automotive,
            representative_ior: 1.5,
            has_iridescence: true,
            has_scattering: false,
            memory_bytes: estimate_metallic_paint_memory(),
        },
        AdvancedMaterialInfo {
            id: "pearlescent_paint",
            name: "Pearlescent Paint",
            description: "Paint with mica interference pigments",
            category: AdvancedMaterialCategory::Automotive,
            representative_ior: 1.5,
            has_iridescence: true,
            has_scattering: false,
            memory_bytes: estimate_pearlescent_memory(),
        },
        AdvancedMaterialInfo {
            id: "chrome_finish",
            name: "Chrome Finish",
            description: "Mirror-like chromium plating",
            category: AdvancedMaterialCategory::Automotive,
            representative_ior: 3.18,
            has_iridescence: false,
            has_scattering: false,
            memory_bytes: estimate_chrome_memory(),
        },
        // Natural
        AdvancedMaterialInfo {
            id: "opal",
            name: "Opal",
            description: "Gemstone with play-of-color from silica sphere arrays",
            category: AdvancedMaterialCategory::Natural,
            representative_ior: 1.45,
            has_iridescence: true,
            has_scattering: false,
            memory_bytes: estimate_opal_memory(),
        },
        AdvancedMaterialInfo {
            id: "mother_of_pearl",
            name: "Mother of Pearl",
            description: "Iridescent nacre from mollusc shells",
            category: AdvancedMaterialCategory::Natural,
            representative_ior: 1.68,
            has_iridescence: true,
            has_scattering: false,
            memory_bytes: estimate_nacre_memory(),
        },
        AdvancedMaterialInfo {
            id: "beetle_shell",
            name: "Beetle Shell",
            description: "Structural coloration from chitin helicoidal layers",
            category: AdvancedMaterialCategory::Natural,
            representative_ior: 1.56,
            has_iridescence: true,
            has_scattering: false,
            memory_bytes: estimate_beetle_memory(),
        },
        // Technical
        AdvancedMaterialInfo {
            id: "anti_reflective_coating",
            name: "AR Coating",
            description: "Multi-layer anti-reflective coating",
            category: AdvancedMaterialCategory::Technical,
            representative_ior: 1.38,
            has_iridescence: false,
            has_scattering: false,
            memory_bytes: estimate_ar_memory(),
        },
        AdvancedMaterialInfo {
            id: "dichroic_filter",
            name: "Dichroic Filter",
            description: "Wavelength-selective interference filter",
            category: AdvancedMaterialCategory::Technical,
            representative_ior: 2.3,
            has_iridescence: true,
            has_scattering: false,
            memory_bytes: estimate_dichroic_memory(),
        },
        AdvancedMaterialInfo {
            id: "holographic",
            name: "Holographic Grating",
            description: "Diffraction grating for spectral dispersion",
            category: AdvancedMaterialCategory::Technical,
            representative_ior: 1.5,
            has_iridescence: true,
            has_scattering: false,
            memory_bytes: estimate_holographic_memory(),
        },
    ]
}

/// Get preset info by ID
pub fn get_preset_info(id: &str) -> Option<AdvancedMaterialInfo> {
    catalog().into_iter().find(|p| p.id == id)
}

/// List presets by category
pub fn list_by_category(category: AdvancedMaterialCategory) -> Vec<AdvancedMaterialInfo> {
    catalog()
        .into_iter()
        .filter(|p| p.category == category)
        .collect()
}

// Memory estimation functions
fn estimate_low_e_memory() -> usize {
    512
}
fn estimate_electrochromic_memory() -> usize {
    256
}
fn estimate_pdlc_memory() -> usize {
    256
}
fn estimate_metallic_paint_memory() -> usize {
    768
}
fn estimate_pearlescent_memory() -> usize {
    640
}
fn estimate_chrome_memory() -> usize {
    128
}
fn estimate_opal_memory() -> usize {
    1024
}
fn estimate_nacre_memory() -> usize {
    512
}
fn estimate_beetle_memory() -> usize {
    768
}
fn estimate_ar_memory() -> usize {
    384
}
fn estimate_dichroic_memory() -> usize {
    512
}
fn estimate_holographic_memory() -> usize {
    256
}

/// Estimate total memory for all advanced material presets
pub fn estimate_advanced_presets_memory() -> usize {
    catalog().iter().map(|p| p.memory_bytes).sum()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_low_e_coating_creation() {
        let material = low_e_coating();
        assert!(material.layer_count() >= 2);
    }

    #[test]
    fn test_electrochromic_range() {
        let clear = electrochromic_glass(0.0);
        let tinted = electrochromic_glass(1.0);

        // Tinted should have higher roughness
        assert!(tinted.roughness > clear.roughness);
    }

    #[test]
    fn test_smart_glass_pdlc_range() {
        let opaque = smart_glass_pdlc(0.0);
        let clear = smart_glass_pdlc(1.0);

        // Opaque should scatter more (higher roughness)
        assert!(opaque.roughness > clear.roughness);
    }

    #[test]
    fn test_car_paint_metallic() {
        let paint = car_paint_metallic(180.0, 0.5);
        assert!(paint.layer_count() >= 2);
    }

    #[test]
    fn test_pearlescent_paint() {
        let paint = pearlescent_paint(240.0);
        assert!(paint.layer_count() >= 2);
    }

    #[test]
    fn test_chrome_finish() {
        let chrome = chrome_finish();
        // Chrome should have high reflectivity (metallic)
        let ctx = BSDFContext::new_simple(1.0);
        let response = chrome.evaluate(&ctx);
        assert!(response.reflectance > 0.5);
    }

    #[test]
    fn test_opal() {
        let opal_material = opal();
        // viewing_angle_dependence defaults to true for StructuralColor
        assert!(opal_material.viewing_angle_dependence);
    }

    #[test]
    fn test_mother_of_pearl() {
        let nacre = mother_of_pearl();
        assert!(nacre.layer_count() >= 2);
    }

    #[test]
    fn test_beetle_shell() {
        let shell = beetle_shell();
        assert!(shell.viewing_angle_dependence);
    }

    #[test]
    fn test_anti_reflective_coating() {
        let ar = anti_reflective_coating(4);
        assert!(ar.layers.len() >= 4);
    }

    #[test]
    fn test_dichroic_filter() {
        let filter = dichroic_filter((500.0, 550.0));
        assert!(filter.layers.len() >= 4);
    }

    #[test]
    fn test_holographic() {
        let grating = holographic();
        // Just verify it creates without panic
        assert!(grating.period > 0.0);
    }

    #[test]
    fn test_catalog() {
        let cat = catalog();
        assert!(cat.len() >= 12);

        // Verify all categories are represented
        let arch_count = cat
            .iter()
            .filter(|p| p.category == AdvancedMaterialCategory::Architectural)
            .count();
        let auto_count = cat
            .iter()
            .filter(|p| p.category == AdvancedMaterialCategory::Automotive)
            .count();
        let natural_count = cat
            .iter()
            .filter(|p| p.category == AdvancedMaterialCategory::Natural)
            .count();
        let tech_count = cat
            .iter()
            .filter(|p| p.category == AdvancedMaterialCategory::Technical)
            .count();

        assert!(arch_count >= 3);
        assert!(auto_count >= 3);
        assert!(natural_count >= 3);
        assert!(tech_count >= 3);
    }

    #[test]
    fn test_get_preset_info() {
        let info = get_preset_info("opal");
        assert!(info.is_some());
        assert!(info.unwrap().has_iridescence);
    }

    #[test]
    fn test_list_by_category() {
        let automotive = list_by_category(AdvancedMaterialCategory::Automotive);
        assert!(automotive.len() >= 3);
        assert!(automotive
            .iter()
            .all(|p| p.category == AdvancedMaterialCategory::Automotive));
    }

    #[test]
    fn test_memory_estimation() {
        let total = estimate_advanced_presets_memory();
        assert!(total > 0);
        assert!(total < 100_000); // Should be reasonable
    }
}

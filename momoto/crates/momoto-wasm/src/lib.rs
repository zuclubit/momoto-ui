//! # Momoto WASM Bindings
//!
//! WebAssembly bindings for the **Momoto Multimodal Perceptual Physics Engine**.
//!
//! Provides JavaScript-friendly APIs for three perceptual domains:
//!
//! | Domain  | Feature flag | Capabilities |
//! |---------|-------------|--------------|
//! | Color   | `color`     | WCAG 2.1, APCA-W3, OKLCH, HCT, CVD, harmony |
//! | Audio   | `audio`     | K-weighting, LUFS (momentary/short-term/integrated), FFT, Mel |
//! | Haptics | `haptics`   | LRA/ERM/Piezo mapping, energy budget, waveform generation |
//!
//! ## Feature flags
//!
//! ```toml
//! # Cargo.toml
//! momoto-wasm = { features = ["color"]           }  # optical domain only
//! momoto-wasm = { features = ["audio"]           }  # acoustic domain only
//! momoto-wasm = { features = ["haptics"]         }  # vibrotactile domain only
//! momoto-wasm = { features = ["multimodal"]      }  # all three domains
//! ```
//!
//! ## Usage from JavaScript
//!
//! ```javascript
//! import init, { WCAGMetric, APCAMetric, Color } from './momoto_wasm';
//!
//! await init();
//!
//! // Single evaluation
//! const wcag = new WCAGMetric();
//! const black = Color.from_rgb(0, 0, 0);

// WASM bindings are still in development - allow some clippy warnings for scaffolding code
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(deprecated)]
#![allow(non_camel_case_types)]
#![allow(clippy::new_without_default)]
#![allow(clippy::clone_on_copy)]

// =============================================================================
// Sub-modules — expose additional crate APIs
// =============================================================================
mod agent;
#[cfg(feature = "audio")]
mod audio;
mod core_ext;
mod events;
mod hct;
mod intelligence;
mod materials_ext;
mod procedural;
mod siren;
mod temporal;

// ── Haptics module (gated by feature "haptics") ───────────────────────────────
// #[cfg(feature = "haptics")]
// mod haptics;

pub use agent::*;
#[cfg(feature = "audio")]
pub use audio::*;
pub use core_ext::*;
pub use events::*;
pub use hct::*;
pub use intelligence::*;
pub use materials_ext::*;
pub use procedural::*;
pub use siren::*;
pub use temporal::*;

// const white = Color.from_rgb(255, 255, 255);
// const ratio = wcag.evaluate(black, white);
// console.log(`Contrast ratio: ${ratio.value}`);
//
// // Batch evaluation (faster for multiple colors)
// const foregrounds = [black, black, black];
// const backgrounds = [white, white, white];
// const results = wcag.evaluate_batch(foregrounds, backgrounds);
// ```

use momoto_core::color::Color as CoreColor;
use momoto_core::material::GlassMaterial as CoreGlassMaterial;
use momoto_core::perception::{ContrastMetric as CoreContrastMetric, Polarity as CorePolarity};
use momoto_core::space::oklch::{HuePath as CoreHuePath, OKLCH as CoreOKLCH};
use momoto_intelligence::context::{
    ComplianceTarget as CoreComplianceTarget, RecommendationContext as CoreRecommendationContext,
    UsageContext as CoreUsageContext,
};
use momoto_intelligence::scoring::{
    QualityScore as CoreQualityScore, QualityScorer as CoreQualityScorer,
};
use momoto_materials::blur::BlurIntensity as CoreBlurIntensity;
use momoto_materials::elevation::{
    Elevation as CoreElevation, MaterialSurface as CoreMaterialSurface,
};
use momoto_materials::glass::{
    GlassLayers as CoreGlassLayers, GlassProperties as CoreGlassProperties,
    GlassVariant as CoreGlassVariant, LiquidGlass as CoreLiquidGlass,
};
use momoto_materials::glass_physics::{
    // NEW: Batch evaluation
    batch::{
        BatchEvaluator as CoreBatchEvaluator, BatchMaterialInput as CoreBatchInput,
        BatchResult as CoreBatchResult,
    },
    blinn_phong,
    // NEW: Sprint 2 - Complex IOR for Metals
    complex_ior::{
        fresnel_conductor as core_fresnel_conductor,
        fresnel_conductor_schlick as core_fresnel_conductor_schlick,
        fresnel_conductor_unpolarized as core_fresnel_conductor_unpolarized,
        metals as metal_presets, to_css_metallic_gradient as core_to_css_metallic_gradient,
        to_css_metallic_surface as core_to_css_metallic_surface, Complex as CoreComplex,
        ComplexIOR as CoreComplexIOR, SpectralComplexIOR as CoreSpectralComplexIOR,
    },
    // NEW: Context system
    context::{
        BackgroundContext as CoreBackgroundContext, ContextPresets as CoreContextPresets,
        LightingContext as CoreLightingContext, MaterialContext as CoreMaterialContext,
        ViewContext as CoreViewContext,
    },
    // NEW: Sprint 2 - Chromatic Dispersion
    dispersion::{
        chromatic_aberration_strength as core_chromatic_aberration_strength,
        f0_from_ior as core_f0_from_ior, f0_rgb as core_f0_rgb,
        wavelengths as dispersion_wavelengths, CauchyDispersion as CoreCauchyDispersion,
        Dispersion as DispersionTrait, DispersionModel as CoreDispersionModel,
        SellmeierDispersion as CoreSellmeierDispersion,
    },
    fresnel,
    light_model::Vec3 as CoreVec3,
    // NEW: LUT functions
    lut::{beer_lambert_fast, fresnel_fast, total_lut_memory},
    // NEW: Sprint 2 - Temperature-Dependent Metals (extended Sprint 5)
    metal_temp::{
        drude_metals as drude_presets, oxides as oxide_presets,
        oxidized_presets as oxidized_metal_presets, temp_metal_memory as core_temp_metal_memory,
        temperature_sensitivity as core_temperature_sensitivity,
        to_css_patina as core_to_css_patina, to_css_temp_metal as core_to_css_temp_metal,
        DrudeParams as CoreDrudeParams, OxideLayer as CoreOxideLayer,
        TempOxidizedMetal as CoreTempOxidizedMetal,
    },
    mie_dynamic::{
        anisotropic_phase as core_anisotropic_phase, dynamic_presets as mie_dynamic_presets,
        effective_asymmetry_g as core_effective_asymmetry_g,
        extinction_coefficient as core_extinction_coefficient,
        polydisperse_phase as core_polydisperse_phase,
        polydisperse_phase_rgb as core_polydisperse_phase_rgb,
        to_css_fog_effect as core_to_css_fog_effect,
        to_css_smoke_effect as core_to_css_smoke_effect, DynamicMieParams as CoreDynamicMieParams,
        SizeDistribution as CoreSizeDistribution,
    },
    // NEW: Sprint 3 - Mie Scattering (Volumetric)
    mie_lut::{
        mie_asymmetry_g as core_mie_asymmetry_g, mie_efficiencies as core_mie_efficiencies,
        mie_fast as core_mie_fast, mie_particle as core_mie_particle,
        mie_particle_rgb as core_mie_particle_rgb, particles as mie_particle_presets,
        rayleigh_efficiency as core_rayleigh_efficiency,
        rayleigh_intensity_rgb as core_rayleigh_intensity_rgb,
        rayleigh_phase as core_rayleigh_phase, MieLUT as CoreMieLUT, MieParams as CoreMieParams,
    },
    perlin_noise::{presets as noise_presets, PerlinNoise as CorePerlinNoise},
    scattering::{
        double_henyey_greenstein as core_double_henyey_greenstein,
        henyey_greenstein as core_henyey_greenstein, hg_fast as core_hg_fast,
        presets as scattering_presets, sample_hg as core_sample_hg,
        ScatteringParams as CoreScatteringParams,
    },
    // NEW: Sprint 6 - Unified Spectral Pipeline
    spectral_pipeline::{
        wavelengths as spectral_wavelengths, DispersionStage as CoreDispersionStage,
        EvaluationContext as CoreEvaluationContext,
        MetalReflectanceStage as CoreMetalReflectanceStage,
        MieScatteringStage as CoreMieScatteringStage, PipelineBuilder as CorePipelineBuilder,
        SpectralPipeline as CoreSpectralPipeline, SpectralSample as CoreSpectralSample,
        SpectralSignal as CoreSpectralSignal, SpectralStage as CoreSpectralStage,
        ThermoOpticStage as CoreThermoOpticStage, ThinFilmStage as CoreThinFilmStage,
    },
    // NEW: Sprint 1 - Thin-Film Interference
    thin_film::{
        self as thin_film_module, ar_coating_thickness as core_ar_coating_thickness,
        dominant_wavelength as core_dominant_wavelength, presets as thin_film_presets,
        thin_film_to_rgb as core_thin_film_to_rgb,
        to_css_iridescent_gradient as core_to_css_iridescent_gradient,
        to_css_oil_slick as core_to_css_oil_slick, to_css_soap_bubble as core_to_css_soap_bubble,
        ThinFilm as CoreThinFilm, ThinFilmStack as CoreThinFilmStack,
    },
    // NEW: Sprint 4 - Advanced Thin Film (Multilayer, Structural Color)
    thin_film_advanced::{
        advanced_presets as tmm_presets, calculate_color_shift as core_calculate_color_shift,
        find_peak_wavelength as core_find_peak_wavelength,
        to_css_bragg_mirror as core_to_css_bragg_mirror,
        to_css_structural_color as core_to_css_structural_color,
        transfer_matrix_memory as core_transfer_matrix_memory, FilmLayer as CoreFilmLayer,
        Polarization as CorePolarization, TransferMatrixFilm as CoreTransferMatrixFilm,
    },
    // NEW: Sprint 5 - Dynamic Optics (Thermo-Optic, Stress-Optic)
    thin_film_dynamic::{
        compute_iridescence_map as core_compute_iridescence_map,
        dynamic_presets as dynamic_thin_film_presets, DynamicFilmLayer as CoreDynamicFilmLayer,
        DynamicThinFilmStack as CoreDynamicThinFilmStack, HeightMap as CoreHeightMap,
        IridescenceMap as CoreIridescenceMap, SubstrateProperties as CoreSubstrateProperties,
        Vec2 as CoreVec2,
    },
    transmittance::{
        calculate_multi_layer_transmittance as core_calc_transmittance,
        LayerTransmittance as CoreLayerTransmittance, OpticalProperties as CoreOpticalProperties,
    },
};
use momoto_materials::shadow_engine::{
    ambient_shadow::AmbientShadow as CoreAmbientShadow,
    contact_shadow::ContactShadow as CoreContactShadow,
    elevation_shadow::{
        calculate_elevation_shadow as core_calculate_elevation_shadow,
        to_css as core_shadow_to_css, ElevationPresets as CoreElevationPresets,
        ElevationShadow as CoreElevationShadow,
    },
};
use momoto_materials::vibrancy::{
    VibrancyEffect as CoreVibrancyEffect, VibrancyLevel as CoreVibrancyLevel,
};
use momoto_metrics::apca::APCAMetric as CoreAPCAMetric;
use momoto_metrics::wcag::{
    TextSize as CoreTextSize, WCAGLevel as CoreWCAGLevel, WCAGMetric as CoreWCAGMetric,
};
use wasm_bindgen::prelude::*;
// NEW: Phase 3 evaluate() + render() pipeline
use momoto_core::backend::CssBackend as CoreCssBackend;
use momoto_core::evaluated::{
    Evaluable, EvaluatedMaterial as CoreEvaluatedMaterial, LinearRgba as CoreLinearRgba,
    MaterialContext as CoreEvalMaterialContext,
};
use momoto_core::render::{
    AccessibilityMode as CoreAccessibilityMode, ColorSpace as CoreColorSpace, RenderBackend,
    RenderContext as CoreRenderContext, RenderError as CoreRenderError,
    TargetMedium as CoreTargetMedium,
};
// NEW: Enhanced CSS rendering
use momoto_core::backend::css_config::CssRenderConfig as CoreCssRenderConfig;
use momoto_materials::css_enhanced::EnhancedCssBackend as CoreEnhancedCssBackend;
use momoto_materials::glass_physics::blinn_phong::{
    to_css_inner_highlight, to_css_secondary_specular, to_css_specular_highlight,
};
use momoto_materials::glass_physics::fresnel::to_css_fresnel_gradient;

// ============================================================================
// Color
// ============================================================================

/// RGB color value.
///
/// Represents a color in sRGB color space with 8-bit channels.
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct Color {
    pub(crate) inner: CoreColor,
}

#[wasm_bindgen]
impl Color {
    /// Create a color from RGB values (0-255).
    #[wasm_bindgen(constructor)]
    pub fn from_rgb(r: u8, g: u8, b: u8) -> Color {
        Color {
            inner: CoreColor::from_srgb8(r, g, b),
        }
    }

    /// Create a color from hex string (e.g., "#FF0000" or "FF0000").
    #[wasm_bindgen(js_name = fromHex)]
    pub fn from_hex(hex: &str) -> Result<Color, JsValue> {
        CoreColor::from_hex(hex)
            .map(|inner| Color { inner })
            .map_err(|e| JsValue::from_str(&e))
    }

    /// Get red channel (0-255).
    #[wasm_bindgen(getter)]
    pub fn r(&self) -> u8 {
        self.inner.to_srgb8()[0]
    }

    /// Get green channel (0-255).
    #[wasm_bindgen(getter)]
    pub fn g(&self) -> u8 {
        self.inner.to_srgb8()[1]
    }

    /// Get blue channel (0-255).
    #[wasm_bindgen(getter)]
    pub fn b(&self) -> u8 {
        self.inner.to_srgb8()[2]
    }

    /// Convert to hex string (e.g., "#FF0000").
    #[wasm_bindgen(js_name = toHex)]
    pub fn to_hex(&self) -> String {
        self.inner.to_hex()
    }

    // ========================================================================
    // Alpha Channel (Gap 1 - P1)
    // ========================================================================

    /// Get the alpha (opacity) value of this color (0.0-1.0).
    ///
    /// Returns 1.0 for fully opaque colors.
    #[wasm_bindgen(getter)]
    pub fn alpha(&self) -> f64 {
        self.inner.get_alpha()
    }

    /// Create a new Color with the specified alpha (opacity) value.
    ///
    /// # Arguments
    /// * `alpha` - Alpha value (0.0 = transparent, 1.0 = opaque)
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// const color = Color.fromHex("#FF0000");
    /// const semiTransparent = color.withAlpha(0.5);
    /// console.log(semiTransparent.alpha); // 0.5
    /// ```
    #[wasm_bindgen(js_name = withAlpha)]
    pub fn with_alpha(&self, alpha: f64) -> Color {
        Color {
            inner: self.inner.with_alpha(alpha),
        }
    }

    // ========================================================================
    // Color Manipulation (convenience methods)
    // ========================================================================

    /// Make the color lighter by the specified amount.
    ///
    /// # Arguments
    /// * `amount` - Lightness increase (0.0 to 1.0)
    pub fn lighten(&self, amount: f64) -> Color {
        Color {
            inner: self.inner.lighten(amount),
        }
    }

    /// Make the color darker by the specified amount.
    ///
    /// # Arguments
    /// * `amount` - Lightness decrease (0.0 to 1.0)
    pub fn darken(&self, amount: f64) -> Color {
        Color {
            inner: self.inner.darken(amount),
        }
    }

    /// Increase the saturation (chroma) of the color.
    ///
    /// # Arguments
    /// * `amount` - Chroma increase
    pub fn saturate(&self, amount: f64) -> Color {
        Color {
            inner: self.inner.saturate(amount),
        }
    }

    /// Decrease the saturation (chroma) of the color.
    ///
    /// # Arguments
    /// * `amount` - Chroma decrease
    pub fn desaturate(&self, amount: f64) -> Color {
        Color {
            inner: self.inner.desaturate(amount),
        }
    }
}

impl Color {
    pub(crate) fn to_core(&self) -> CoreColor {
        self.inner
    }
    pub(crate) fn from_core(inner: CoreColor) -> Self {
        Self { inner }
    }
}

// ============================================================================
// Contrast Result
// ============================================================================

/// Result of a contrast calculation.
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct ContrastResult {
    /// The contrast value.
    ///
    /// Interpretation depends on metric:
    /// - WCAG: 1.0 to 21.0 (contrast ratio)
    /// - APCA: -108 to +106 (Lc value, signed)
    pub value: f64,

    /// Polarity of the contrast (APCA only).
    ///
    /// - 1 = dark on light
    /// - -1 = light on dark
    /// - 0 = not applicable (WCAG)
    pub polarity: i8,
}

impl ContrastResult {
    fn from_core(result: momoto_core::perception::PerceptualResult) -> Self {
        let polarity = match result.polarity {
            Some(CorePolarity::DarkOnLight) => 1,
            Some(CorePolarity::LightOnDark) => -1,
            None => 0,
        };

        ContrastResult {
            value: result.value,
            polarity,
        }
    }
}

// ============================================================================
// WCAG 2.1 Metric
// ============================================================================

/// WCAG 2.1 Contrast Ratio metric.
///
/// Calculates symmetric contrast ratios from 1.0 (no contrast) to 21.0 (maximum).
#[wasm_bindgen]
pub struct WCAGMetric {
    inner: CoreWCAGMetric,
}

#[wasm_bindgen]
impl WCAGMetric {
    /// Create a new WCAG metric.
    #[wasm_bindgen(constructor)]
    pub fn new() -> WCAGMetric {
        WCAGMetric {
            inner: CoreWCAGMetric::new(),
        }
    }

    /// Evaluate contrast between foreground and background colors.
    ///
    /// Returns a contrast ratio from 1.0 to 21.0.
    pub fn evaluate(&self, foreground: &Color, background: &Color) -> ContrastResult {
        let result = self.inner.evaluate(foreground.inner, background.inner);
        ContrastResult::from_core(result)
    }

    /// Evaluate contrast for multiple color pairs (faster than calling evaluate in a loop).
    ///
    /// # Arguments
    ///
    /// * `foregrounds` - Array of foreground colors
    /// * `backgrounds` - Array of background colors (must match length)
    ///
    /// # Returns
    ///
    /// Array of contrast results
    #[wasm_bindgen(js_name = evaluateBatch)]
    pub fn evaluate_batch(
        &self,
        foregrounds: Vec<Color>,
        backgrounds: Vec<Color>,
    ) -> Result<Vec<ContrastResult>, JsValue> {
        if foregrounds.len() != backgrounds.len() {
            return Err(JsValue::from_str(
                "Foreground and background arrays must have the same length",
            ));
        }

        let fg_colors: Vec<CoreColor> = foregrounds.iter().map(|c| c.inner).collect();
        let bg_colors: Vec<CoreColor> = backgrounds.iter().map(|c| c.inner).collect();

        let results = self.inner.evaluate_batch(&fg_colors, &bg_colors);
        Ok(results.into_iter().map(ContrastResult::from_core).collect())
    }

    /// Check if contrast ratio passes WCAG level for text size.
    ///
    /// # Arguments
    ///
    /// * `ratio` - Contrast ratio to check
    /// * `level` - "AA" or "AAA"
    /// * `is_large_text` - Whether text is large (18pt+ or 14pt+ bold)
    #[wasm_bindgen]
    pub fn passes(ratio: f64, level: &str, is_large_text: bool) -> bool {
        let wcag_level = match level {
            "AA" => CoreWCAGLevel::AA,
            "AAA" => CoreWCAGLevel::AAA,
            _ => return false,
        };

        let text_size = if is_large_text {
            CoreTextSize::Large
        } else {
            CoreTextSize::Normal
        };

        CoreWCAGMetric::passes(ratio, wcag_level, text_size)
    }
}

impl Default for WCAGMetric {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// APCA Metric
// ============================================================================

/// APCA-W3 0.1.9 Contrast metric.
///
/// Calculates polarity-aware Lc values from -108 to +106.
/// Positive values = dark on light, negative = light on dark.
#[wasm_bindgen]
pub struct APCAMetric {
    inner: CoreAPCAMetric,
}

#[wasm_bindgen]
impl APCAMetric {
    /// Create a new APCA metric.
    #[wasm_bindgen(constructor)]
    pub fn new() -> APCAMetric {
        APCAMetric {
            inner: CoreAPCAMetric::new(),
        }
    }

    /// Evaluate APCA contrast (Lc value) between foreground and background.
    ///
    /// Returns Lc value:
    /// - Positive = dark text on light background
    /// - Negative = light text on dark background
    /// - Near zero = insufficient contrast
    pub fn evaluate(&self, foreground: &Color, background: &Color) -> ContrastResult {
        let result = self.inner.evaluate(foreground.inner, background.inner);
        ContrastResult::from_core(result)
    }

    /// Evaluate APCA contrast for multiple color pairs (faster than calling evaluate in a loop).
    ///
    /// # Arguments
    ///
    /// * `foregrounds` - Array of foreground colors
    /// * `backgrounds` - Array of background colors (must match length)
    ///
    /// # Returns
    ///
    /// Array of APCA results with Lc values and polarities
    #[wasm_bindgen(js_name = evaluateBatch)]
    pub fn evaluate_batch(
        &self,
        foregrounds: Vec<Color>,
        backgrounds: Vec<Color>,
    ) -> Result<Vec<ContrastResult>, JsValue> {
        if foregrounds.len() != backgrounds.len() {
            return Err(JsValue::from_str(
                "Foreground and background arrays must have the same length",
            ));
        }

        let fg_colors: Vec<CoreColor> = foregrounds.iter().map(|c| c.inner).collect();
        let bg_colors: Vec<CoreColor> = backgrounds.iter().map(|c| c.inner).collect();

        let results = self.inner.evaluate_batch(&fg_colors, &bg_colors);
        Ok(results.into_iter().map(ContrastResult::from_core).collect())
    }
}

impl Default for APCAMetric {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// OKLCH Color Space
// ============================================================================

/// OKLCH color space value.
///
/// Perceptually uniform cylindrical color space.
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct OKLCH {
    pub(crate) inner: CoreOKLCH,
}

#[wasm_bindgen]
impl OKLCH {
    /// Create OKLCH color from L, C, H values.
    ///
    /// # Arguments
    ///
    /// * `l` - Lightness (0.0 to 1.0)
    /// * `c` - Chroma (0.0 to ~0.4)
    /// * `h` - Hue (0.0 to 360.0 degrees)
    #[wasm_bindgen(constructor)]
    pub fn new(l: f64, c: f64, h: f64) -> OKLCH {
        OKLCH {
            inner: CoreOKLCH::new(l, c, h),
        }
    }

    /// Convert RGB color to OKLCH.
    #[wasm_bindgen(js_name = fromColor)]
    pub fn from_color(color: &Color) -> OKLCH {
        OKLCH {
            inner: CoreOKLCH::from_color(&color.inner),
        }
    }

    /// Convert OKLCH to RGB color.
    #[wasm_bindgen(js_name = toColor)]
    pub fn to_color(&self) -> Color {
        Color {
            inner: self.inner.to_color(),
        }
    }

    /// Get lightness (0.0 to 1.0).
    #[wasm_bindgen(getter)]
    pub fn l(&self) -> f64 {
        self.inner.l
    }

    /// Get chroma (0.0 to ~0.4).
    #[wasm_bindgen(getter)]
    pub fn c(&self) -> f64 {
        self.inner.c
    }

    /// Get hue (0.0 to 360.0).
    #[wasm_bindgen(getter)]
    pub fn h(&self) -> f64 {
        self.inner.h
    }

    /// Make color lighter by delta.
    pub fn lighten(&self, delta: f64) -> OKLCH {
        OKLCH {
            inner: self.inner.lighten(delta),
        }
    }

    /// Make color darker by delta.
    pub fn darken(&self, delta: f64) -> OKLCH {
        OKLCH {
            inner: self.inner.darken(delta),
        }
    }

    /// Increase chroma (saturation) by factor.
    pub fn saturate(&self, factor: f64) -> OKLCH {
        OKLCH {
            inner: self.inner.saturate(factor),
        }
    }

    /// Decrease chroma (saturation) by factor.
    pub fn desaturate(&self, factor: f64) -> OKLCH {
        OKLCH {
            inner: self.inner.desaturate(factor),
        }
    }

    /// Rotate hue by degrees.
    #[wasm_bindgen(js_name = rotateHue)]
    pub fn rotate_hue(&self, degrees: f64) -> OKLCH {
        OKLCH {
            inner: self.inner.rotate_hue(degrees),
        }
    }

    /// Map to sRGB gamut by reducing chroma if necessary.
    #[wasm_bindgen(js_name = mapToGamut)]
    pub fn map_to_gamut(&self) -> OKLCH {
        OKLCH {
            inner: self.inner.map_to_gamut(),
        }
    }

    /// Calculate perceptual difference (Delta E) between two colors.
    #[wasm_bindgen(js_name = deltaE)]
    pub fn delta_e(&self, other: &OKLCH) -> f64 {
        self.inner.delta_e(&other.inner)
    }

    /// Interpolate between two OKLCH colors.
    ///
    /// # Arguments
    ///
    /// * `a` - Start color
    /// * `b` - End color
    /// * `t` - Interpolation factor (0.0 to 1.0)
    /// * `hue_path` - "shorter" or "longer"
    pub fn interpolate(a: &OKLCH, b: &OKLCH, t: f64, hue_path: &str) -> OKLCH {
        let path = match hue_path {
            "longer" => CoreHuePath::Longer,
            _ => CoreHuePath::Shorter,
        };

        OKLCH {
            inner: CoreOKLCH::interpolate(&a.inner, &b.inner, t, path),
        }
    }
}

impl OKLCH {
    pub(crate) fn to_core_oklch(&self) -> CoreOKLCH {
        self.inner
    }
    pub(crate) fn from_core(inner: CoreOKLCH) -> Self {
        Self { inner }
    }
}

// ============================================================================
// Intelligence - Usage Context
// ============================================================================

/// Usage context for color recommendations.
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub enum UsageContext {
    /// Body text - primary content (18px or less, normal weight)
    BodyText,
    /// Large text - headings, titles (18pt+ or 14pt+ bold)
    LargeText,
    /// Interactive elements - buttons, links, form inputs
    Interactive,
    /// Decorative - non-essential visual elements
    Decorative,
    /// Icons and graphics - functional imagery
    IconsGraphics,
    /// Disabled state - reduced emphasis
    Disabled,
}

impl From<UsageContext> for CoreUsageContext {
    fn from(ctx: UsageContext) -> Self {
        match ctx {
            UsageContext::BodyText => CoreUsageContext::BodyText,
            UsageContext::LargeText => CoreUsageContext::LargeText,
            UsageContext::Interactive => CoreUsageContext::Interactive,
            UsageContext::Decorative => CoreUsageContext::Decorative,
            UsageContext::IconsGraphics => CoreUsageContext::IconsGraphics,
            UsageContext::Disabled => CoreUsageContext::Disabled,
        }
    }
}

// ============================================================================
// Intelligence - Compliance Target
// ============================================================================

/// Target compliance level for recommendations.
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub enum ComplianceTarget {
    /// WCAG 2.1 Level AA (minimum legal requirement in many jurisdictions)
    WCAG_AA,
    /// WCAG 2.1 Level AAA (enhanced accessibility)
    WCAG_AAA,
    /// APCA-based recommendations (modern perceptual contrast)
    APCA,
    /// Meet both WCAG AA and APCA minimums
    Hybrid,
}

impl From<ComplianceTarget> for CoreComplianceTarget {
    fn from(target: ComplianceTarget) -> Self {
        match target {
            ComplianceTarget::WCAG_AA => CoreComplianceTarget::WCAG_AA,
            ComplianceTarget::WCAG_AAA => CoreComplianceTarget::WCAG_AAA,
            ComplianceTarget::APCA => CoreComplianceTarget::APCA,
            ComplianceTarget::Hybrid => CoreComplianceTarget::Hybrid,
        }
    }
}

// ============================================================================
// Intelligence - Recommendation Context
// ============================================================================

/// Context for intelligent color recommendations.
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct RecommendationContext {
    inner: CoreRecommendationContext,
}

#[wasm_bindgen]
impl RecommendationContext {
    /// Create a new recommendation context.
    #[wasm_bindgen(constructor)]
    pub fn new(usage: UsageContext, target: ComplianceTarget) -> RecommendationContext {
        RecommendationContext {
            inner: CoreRecommendationContext::new(usage.into(), target.into()),
        }
    }

    /// Create context for body text (WCAG AA).
    #[wasm_bindgen(js_name = bodyText)]
    pub fn body_text() -> RecommendationContext {
        RecommendationContext {
            inner: CoreRecommendationContext::body_text(),
        }
    }

    /// Create context for large text (WCAG AA).
    #[wasm_bindgen(js_name = largeText)]
    pub fn large_text() -> RecommendationContext {
        RecommendationContext {
            inner: CoreRecommendationContext::large_text(),
        }
    }

    /// Create context for interactive elements (WCAG AA).
    #[wasm_bindgen(js_name = interactive)]
    pub fn interactive() -> RecommendationContext {
        RecommendationContext {
            inner: CoreRecommendationContext::new(
                CoreUsageContext::Interactive,
                CoreComplianceTarget::WCAG_AA,
            ),
        }
    }

    /// Create context for decorative elements (no requirements).
    #[wasm_bindgen(js_name = decorative)]
    pub fn decorative() -> RecommendationContext {
        RecommendationContext {
            inner: CoreRecommendationContext::new(
                CoreUsageContext::Decorative,
                CoreComplianceTarget::WCAG_AA,
            ),
        }
    }
}

// ============================================================================
// Intelligence - Quality Score
// ============================================================================

/// Score for a color combination (0.0 to 1.0).
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct QualityScore {
    /// Overall quality score (0.0 to 1.0)
    #[wasm_bindgen(readonly)]
    pub overall: f64,

    /// Compliance score (0.0 = fails, 1.0 = exceeds)
    #[wasm_bindgen(readonly)]
    pub compliance: f64,

    /// Perceptual quality score (0.0 = poor, 1.0 = optimal)
    #[wasm_bindgen(readonly)]
    pub perceptual: f64,

    /// Context appropriateness score (0.0 = inappropriate, 1.0 = perfect fit)
    #[wasm_bindgen(readonly)]
    pub appropriateness: f64,
}

#[wasm_bindgen]
impl QualityScore {
    /// Returns whether this score indicates the combination passes requirements.
    pub fn passes(&self) -> bool {
        self.compliance >= 1.0
    }

    /// Returns a qualitative assessment of the score.
    ///
    /// Returns: "Excellent", "Good", "Acceptable", "Marginal", or "Poor"
    pub fn assessment(&self) -> String {
        if self.overall >= 0.9 {
            "Excellent".to_string()
        } else if self.overall >= 0.7 {
            "Good".to_string()
        } else if self.overall >= 0.5 {
            "Acceptable".to_string()
        } else if self.overall >= 0.3 {
            "Marginal".to_string()
        } else {
            "Poor".to_string()
        }
    }

    /// Get confidence level (0.0 to 1.0).
    ///
    /// Higher confidence means the score is more reliable.
    /// For now, returns compliance score as proxy for confidence.
    pub fn confidence(&self) -> f64 {
        // Confidence is higher when compliance is clear (either very high or very low)
        if self.compliance >= 0.9 {
            0.95
        } else if self.compliance <= 0.3 {
            0.9
        } else {
            0.8
        }
    }

    /// Get human-readable explanation of the score.
    pub fn explanation(&self) -> String {
        let assessment = self.assessment();
        let passes = if self.passes() { "passes" } else { "fails" };

        format!(
            "{} quality ({}). Compliance: {:.0}%, Perceptual: {:.0}%, Appropriateness: {:.0}%",
            assessment,
            passes,
            self.compliance * 100.0,
            self.perceptual * 100.0,
            self.appropriateness * 100.0
        )
    }
}

impl From<CoreQualityScore> for QualityScore {
    fn from(score: CoreQualityScore) -> Self {
        QualityScore {
            overall: score.overall,
            compliance: score.compliance,
            perceptual: score.perceptual,
            appropriateness: score.appropriateness,
        }
    }
}

// ============================================================================
// Intelligence - Quality Scorer
// ============================================================================

/// Scorer for evaluating color combination quality.
#[wasm_bindgen]
pub struct QualityScorer {
    inner: CoreQualityScorer,
}

#[wasm_bindgen]
impl QualityScorer {
    /// Create a new quality scorer.
    #[wasm_bindgen(constructor)]
    pub fn new() -> QualityScorer {
        QualityScorer {
            inner: CoreQualityScorer::new(),
        }
    }

    /// Score a color combination for a given context.
    ///
    /// # Arguments
    ///
    /// * `foreground` - Foreground color
    /// * `background` - Background color
    /// * `context` - Usage context
    ///
    /// # Returns
    ///
    /// Quality score with overall, compliance, perceptual, and appropriateness scores
    pub fn score(
        &self,
        foreground: &Color,
        background: &Color,
        context: &RecommendationContext,
    ) -> QualityScore {
        let core_score = self
            .inner
            .score(foreground.inner, background.inner, context.inner);
        QualityScore::from(core_score)
    }
}

// ============================================================================
// Materials - Glass Variant
// ============================================================================

/// Glass variant defines the visual behavior of Liquid Glass.
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub enum GlassVariant {
    /// Regular glass - adaptive, most versatile
    Regular,
    /// Clear glass - permanently more transparent
    Clear,
}

impl From<GlassVariant> for CoreGlassVariant {
    fn from(variant: GlassVariant) -> Self {
        match variant {
            GlassVariant::Regular => CoreGlassVariant::Regular,
            GlassVariant::Clear => CoreGlassVariant::Clear,
        }
    }
}

impl From<CoreGlassVariant> for GlassVariant {
    fn from(variant: CoreGlassVariant) -> Self {
        match variant {
            CoreGlassVariant::Regular => GlassVariant::Regular,
            CoreGlassVariant::Clear => GlassVariant::Clear,
        }
    }
}

// ============================================================================
// Materials - Glass Properties
// ============================================================================

/// Glass properties defining the multi-layer composition.
#[wasm_bindgen]
#[derive(Clone)]
pub struct GlassProperties {
    inner: CoreGlassProperties,
}

#[wasm_bindgen]
impl GlassProperties {
    /// Create default glass properties.
    #[wasm_bindgen(constructor)]
    pub fn new() -> GlassProperties {
        GlassProperties {
            inner: CoreGlassProperties::default(),
        }
    }

    /// Get base tint color.
    #[wasm_bindgen(js_name = getBaseTint)]
    pub fn get_base_tint(&self) -> OKLCH {
        OKLCH {
            inner: self.inner.base_tint,
        }
    }

    /// Set base tint color.
    #[wasm_bindgen(js_name = setBaseTint)]
    pub fn set_base_tint(&mut self, tint: &OKLCH) {
        self.inner.base_tint = tint.inner;
    }

    /// Get opacity (0.0 = transparent, 1.0 = opaque).
    #[wasm_bindgen(getter)]
    pub fn opacity(&self) -> f64 {
        self.inner.opacity
    }

    /// Set opacity.
    #[wasm_bindgen(setter)]
    pub fn set_opacity(&mut self, value: f64) {
        self.inner.opacity = value.clamp(0.0, 1.0);
    }

    /// Get blur radius in pixels.
    #[wasm_bindgen(js_name = blurRadius, getter)]
    pub fn blur_radius(&self) -> f64 {
        self.inner.blur_radius
    }

    /// Set blur radius.
    #[wasm_bindgen(js_name = blurRadius, setter)]
    pub fn set_blur_radius(&mut self, value: f64) {
        self.inner.blur_radius = value.max(0.0);
    }

    /// Get reflectivity (0.0 = none, 1.0 = mirror).
    #[wasm_bindgen(getter)]
    pub fn reflectivity(&self) -> f64 {
        self.inner.reflectivity
    }

    /// Set reflectivity.
    #[wasm_bindgen(setter)]
    pub fn set_reflectivity(&mut self, value: f64) {
        self.inner.reflectivity = value.clamp(0.0, 1.0);
    }

    /// Get refraction index.
    #[wasm_bindgen(getter)]
    pub fn refraction(&self) -> f64 {
        self.inner.refraction
    }

    /// Set refraction index.
    #[wasm_bindgen(setter)]
    pub fn set_refraction(&mut self, value: f64) {
        self.inner.refraction = value.max(1.0);
    }

    /// Get depth/thickness.
    #[wasm_bindgen(getter)]
    pub fn depth(&self) -> f64 {
        self.inner.depth
    }

    /// Set depth/thickness.
    #[wasm_bindgen(setter)]
    pub fn set_depth(&mut self, value: f64) {
        self.inner.depth = value.clamp(0.0, 1.0);
    }

    /// Get noise scale.
    #[wasm_bindgen(js_name = noiseScale, getter)]
    pub fn noise_scale(&self) -> f64 {
        self.inner.noise_scale
    }

    /// Set noise scale.
    #[wasm_bindgen(js_name = noiseScale, setter)]
    pub fn set_noise_scale(&mut self, value: f64) {
        self.inner.noise_scale = value.max(0.0);
    }

    /// Get specular intensity.
    #[wasm_bindgen(js_name = specularIntensity, getter)]
    pub fn specular_intensity(&self) -> f64 {
        self.inner.specular_intensity
    }

    /// Set specular intensity.
    #[wasm_bindgen(js_name = specularIntensity, setter)]
    pub fn set_specular_intensity(&mut self, value: f64) {
        self.inner.specular_intensity = value.clamp(0.0, 1.0);
    }
}

impl Default for GlassProperties {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Materials - Glass Layers
// ============================================================================

/// Multi-layer glass composition.
#[wasm_bindgen]
#[derive(Clone)]
pub struct GlassLayers {
    /// Top layer: Specular highlights
    #[wasm_bindgen(readonly)]
    pub highlight: OKLCH,

    /// Middle layer: Base glass tint
    #[wasm_bindgen(readonly)]
    pub base: OKLCH,

    /// Bottom layer: Shadow for depth
    #[wasm_bindgen(readonly)]
    pub shadow: OKLCH,
}

impl From<CoreGlassLayers> for GlassLayers {
    fn from(layers: CoreGlassLayers) -> Self {
        GlassLayers {
            highlight: OKLCH {
                inner: layers.highlight,
            },
            base: OKLCH { inner: layers.base },
            shadow: OKLCH {
                inner: layers.shadow,
            },
        }
    }
}

// ============================================================================
// Materials - Liquid Glass
// ============================================================================

/// Liquid Glass surface with adaptive behavior.
///
/// Implementation of Apple's Liquid Glass material system from WWDC 2025.
#[wasm_bindgen]
pub struct LiquidGlass {
    inner: CoreLiquidGlass,
}

#[wasm_bindgen]
impl LiquidGlass {
    /// Create new Liquid Glass with specified variant.
    #[wasm_bindgen(constructor)]
    pub fn new(variant: GlassVariant) -> LiquidGlass {
        LiquidGlass {
            inner: CoreLiquidGlass::new(variant.into()),
        }
    }

    /// Create with custom properties.
    #[wasm_bindgen(js_name = withProperties)]
    pub fn with_properties(variant: GlassVariant, properties: &GlassProperties) -> LiquidGlass {
        LiquidGlass {
            inner: CoreLiquidGlass::with_properties(variant.into(), properties.inner.clone()),
        }
    }

    /// Calculate effective color when glass is over background.
    #[wasm_bindgen(js_name = effectiveColor)]
    pub fn effective_color(&self, background: &Color) -> Color {
        Color {
            inner: self.inner.effective_color(background.inner),
        }
    }

    /// Recommend text color for maximum readability.
    ///
    /// # Arguments
    ///
    /// * `background` - Background color behind the glass
    /// * `prefer_white` - Whether to prefer white text over dark text
    #[wasm_bindgen(js_name = recommendTextColor)]
    pub fn recommend_text_color(&self, background: &Color, prefer_white: bool) -> Color {
        Color {
            inner: self
                .inner
                .recommend_text_color(background.inner, prefer_white),
        }
    }

    /// Decompose into multi-layer structure.
    #[wasm_bindgen(js_name = getLayers)]
    pub fn get_layers(&self, background: &Color) -> GlassLayers {
        let layers = self.inner.get_layers(background.inner);
        GlassLayers::from(layers)
    }

    /// Adapt glass properties for dark mode.
    #[wasm_bindgen(js_name = adaptForDarkMode)]
    pub fn adapt_for_dark_mode(&mut self) {
        self.inner.adapt_for_dark_mode();
    }

    /// Adapt glass properties for light mode.
    #[wasm_bindgen(js_name = adaptForLightMode)]
    pub fn adapt_for_light_mode(&mut self) {
        self.inner.adapt_for_light_mode();
    }

    /// Get variant.
    #[wasm_bindgen(getter)]
    pub fn variant(&self) -> GlassVariant {
        self.inner.variant().into()
    }

    /// Get properties.
    #[wasm_bindgen(getter)]
    pub fn properties(&self) -> GlassProperties {
        GlassProperties {
            inner: self.inner.properties().clone(),
        }
    }
}

// ============================================================================
// Materials - Blur Intensity
// ============================================================================

/// Blur intensity levels matching Apple HIG.
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub enum BlurIntensity {
    /// No blur (0px)
    None,
    /// Light blur (10px)
    Light,
    /// Medium blur (20px)
    Medium,
    /// Heavy blur (30px)
    Heavy,
    /// Extra heavy blur (40px)
    ExtraHeavy,
}

// ============================================================================
// Materials - Vibrancy Level
// ============================================================================

/// Vibrancy level determines how much background color bleeds through.
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub enum VibrancyLevel {
    /// Primary vibrancy - most color through (75%)
    Primary,
    /// Secondary vibrancy - moderate color (50%)
    Secondary,
    /// Tertiary vibrancy - subtle color (30%)
    Tertiary,
    /// Divider vibrancy - minimal color (15%)
    Divider,
}

// ============================================================================
// Materials - Vibrancy Effect
// ============================================================================

/// Vibrancy effect applies background color to foreground.
#[wasm_bindgen]
pub struct VibrancyEffect {
    inner: CoreVibrancyEffect,
}

#[wasm_bindgen]
impl VibrancyEffect {
    /// Create new vibrancy effect.
    #[wasm_bindgen(constructor)]
    pub fn new(level: VibrancyLevel) -> VibrancyEffect {
        let core_level = match level {
            VibrancyLevel::Primary => CoreVibrancyLevel::Primary,
            VibrancyLevel::Secondary => CoreVibrancyLevel::Secondary,
            VibrancyLevel::Tertiary => CoreVibrancyLevel::Tertiary,
            VibrancyLevel::Divider => CoreVibrancyLevel::Divider,
        };
        VibrancyEffect {
            inner: CoreVibrancyEffect::new(core_level),
        }
    }

    /// Apply vibrancy to foreground color given background.
    pub fn apply(&self, foreground: &OKLCH, background: &OKLCH) -> OKLCH {
        OKLCH {
            inner: self.inner.apply(foreground.inner, background.inner),
        }
    }

    /// Get vibrancy level.
    #[wasm_bindgen(getter)]
    pub fn level(&self) -> VibrancyLevel {
        match self.inner.level() {
            CoreVibrancyLevel::Primary => VibrancyLevel::Primary,
            CoreVibrancyLevel::Secondary => VibrancyLevel::Secondary,
            CoreVibrancyLevel::Tertiary => VibrancyLevel::Tertiary,
            CoreVibrancyLevel::Divider => VibrancyLevel::Divider,
        }
    }
}

// ============================================================================
// Materials - Elevation
// ============================================================================

/// Material Design 3 elevation levels.
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub enum Elevation {
    /// Level 0 - Base surface
    Level0,
    /// Level 1 - 1dp elevation
    Level1,
    /// Level 2 - 3dp elevation
    Level2,
    /// Level 3 - 6dp elevation
    Level3,
    /// Level 4 - 8dp elevation
    Level4,
    /// Level 5 - 12dp elevation
    Level5,
}

impl From<Elevation> for CoreElevation {
    fn from(elevation: Elevation) -> Self {
        match elevation {
            Elevation::Level0 => CoreElevation::Level0,
            Elevation::Level1 => CoreElevation::Level1,
            Elevation::Level2 => CoreElevation::Level2,
            Elevation::Level3 => CoreElevation::Level3,
            Elevation::Level4 => CoreElevation::Level4,
            Elevation::Level5 => CoreElevation::Level5,
        }
    }
}

impl From<CoreElevation> for Elevation {
    fn from(elevation: CoreElevation) -> Self {
        match elevation {
            CoreElevation::Level0 => Elevation::Level0,
            CoreElevation::Level1 => Elevation::Level1,
            CoreElevation::Level2 => Elevation::Level2,
            CoreElevation::Level3 => Elevation::Level3,
            CoreElevation::Level4 => Elevation::Level4,
            CoreElevation::Level5 => Elevation::Level5,
        }
    }
}

// ============================================================================
// Materials - Material Surface
// ============================================================================

/// Material surface with elevation and optional glass effect.
#[wasm_bindgen]
pub struct MaterialSurface {
    inner: CoreMaterialSurface,
}

#[wasm_bindgen]
impl MaterialSurface {
    /// Create material surface from elevation and theme color.
    #[wasm_bindgen(constructor)]
    pub fn new(elevation: Elevation, theme_primary: &OKLCH) -> MaterialSurface {
        MaterialSurface {
            inner: CoreMaterialSurface::new(elevation.into(), theme_primary.inner),
        }
    }

    /// Apply glass overlay to elevated surface.
    #[wasm_bindgen(js_name = withGlass)]
    pub fn with_glass(self, glass: &LiquidGlass) -> MaterialSurface {
        MaterialSurface {
            inner: self.inner.with_glass(glass.inner.clone()),
        }
    }

    /// Calculate final surface color over base.
    #[wasm_bindgen(js_name = surfaceColor)]
    pub fn surface_color(&self, base_surface: &Color) -> Color {
        Color {
            inner: self.inner.surface_color(base_surface.inner),
        }
    }

    /// Get elevation.
    #[wasm_bindgen(getter)]
    pub fn elevation(&self) -> Elevation {
        self.inner.elevation().into()
    }

    /// Get surface tint.
    #[wasm_bindgen(js_name = surfaceTint, getter)]
    pub fn surface_tint(&self) -> OKLCH {
        OKLCH {
            inner: self.inner.surface_tint(),
        }
    }
}

// ============================================================================
// Shadow Engine - Elevation Shadow System
// ============================================================================

/// Elevation shadow result with CSS output
#[wasm_bindgen]
pub struct ElevationShadow {
    inner: CoreElevationShadow,
}

#[wasm_bindgen]
impl ElevationShadow {
    /// Get elevation level used
    #[wasm_bindgen(getter)]
    pub fn elevation(&self) -> u8 {
        self.inner.elevation
    }

    /// Convert to CSS box-shadow string
    #[wasm_bindgen(js_name = toCSS)]
    pub fn to_css(&self) -> String {
        core_shadow_to_css(&self.inner)
    }
}

/// Calculate elevation shadow for glass element
///
/// # Arguments
///
/// * `elevation` - Elevation level (0-24)
/// * `background` - Background color in OKLCH
/// * `glass_depth` - Perceived thickness of glass (0.0-2.0)
///
/// # Returns
///
/// Complete shadow system as CSS box-shadow string
#[wasm_bindgen(js_name = calculateElevationShadow)]
pub fn calculate_elevation_shadow(
    elevation: u8,
    background: &OKLCH,
    glass_depth: f64,
) -> ElevationShadow {
    ElevationShadow {
        inner: core_calculate_elevation_shadow(elevation, background.inner, glass_depth),
    }
}

/// Elevation presets following Apple Liquid Glass patterns
#[wasm_bindgen]
pub struct ElevationPresets;

#[wasm_bindgen]
impl ElevationPresets {
    /// Flush with surface (no elevation)
    #[wasm_bindgen(getter, js_name = LEVEL_0)]
    pub fn level_0() -> u8 {
        CoreElevationPresets::LEVEL_0
    }

    /// Subtle lift (standard buttons)
    #[wasm_bindgen(getter, js_name = LEVEL_1)]
    pub fn level_1() -> u8 {
        CoreElevationPresets::LEVEL_1
    }

    /// Hover state (interactive lift)
    #[wasm_bindgen(getter, js_name = LEVEL_2)]
    pub fn level_2() -> u8 {
        CoreElevationPresets::LEVEL_2
    }

    /// Floating cards
    #[wasm_bindgen(getter, js_name = LEVEL_3)]
    pub fn level_3() -> u8 {
        CoreElevationPresets::LEVEL_3
    }

    /// Modals, sheets
    #[wasm_bindgen(getter, js_name = LEVEL_4)]
    pub fn level_4() -> u8 {
        CoreElevationPresets::LEVEL_4
    }

    /// Dropdowns, tooltips
    #[wasm_bindgen(getter, js_name = LEVEL_5)]
    pub fn level_5() -> u8 {
        CoreElevationPresets::LEVEL_5
    }

    /// Drag state (maximum separation)
    #[wasm_bindgen(getter, js_name = LEVEL_6)]
    pub fn level_6() -> u8 {
        CoreElevationPresets::LEVEL_6
    }
}

// ============================================================================
// Shadow Engine - Contact Shadow System (Gap 4 - P1)
// ============================================================================

use momoto_materials::shadow_engine::contact_shadow::{
    calculate_contact_shadow as core_calculate_contact_shadow,
    to_css as core_contact_shadow_to_css, ContactShadowParams as CoreContactShadowParams,
    ContactShadowPresets as CoreContactShadowPresets,
};

/// Contact shadow configuration parameters.
///
/// Contact shadows are the sharp, dark shadows that appear where glass
/// touches the background, creating a sense of physical connection.
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct ContactShadowParams {
    inner: CoreContactShadowParams,
}

#[wasm_bindgen]
impl ContactShadowParams {
    /// Create contact shadow params with custom values.
    ///
    /// # Arguments
    ///
    /// * `darkness` - Shadow darkness (0.0 = no shadow, 1.0 = pure black)
    /// * `blur_radius` - Blur radius in pixels (typically 1-3px for contact shadows)
    /// * `offset_y` - Vertical offset in pixels (typically 0-1px)
    /// * `spread` - Shadow spread (typically 0 for contact shadows)
    #[wasm_bindgen(constructor)]
    pub fn new(darkness: f64, blur_radius: f64, offset_y: f64, spread: f64) -> ContactShadowParams {
        ContactShadowParams {
            inner: CoreContactShadowParams {
                darkness: darkness.clamp(0.0, 1.0),
                blur_radius: blur_radius.max(0.0),
                offset_y,
                spread,
            },
        }
    }

    /// Create default contact shadow params (standard glass contact shadow).
    #[wasm_bindgen(js_name = default)]
    pub fn default_params() -> ContactShadowParams {
        ContactShadowParams {
            inner: CoreContactShadowParams::default(),
        }
    }

    /// Standard glass contact shadow preset.
    #[wasm_bindgen(js_name = standard)]
    pub fn standard() -> ContactShadowParams {
        ContactShadowParams {
            inner: CoreContactShadowPresets::standard(),
        }
    }

    /// Floating glass preset (lighter contact shadow).
    #[wasm_bindgen(js_name = floating)]
    pub fn floating() -> ContactShadowParams {
        ContactShadowParams {
            inner: CoreContactShadowPresets::floating(),
        }
    }

    /// Grounded glass preset (heavier contact shadow).
    #[wasm_bindgen(js_name = grounded)]
    pub fn grounded() -> ContactShadowParams {
        ContactShadowParams {
            inner: CoreContactShadowPresets::grounded(),
        }
    }

    /// Subtle preset (barely visible contact shadow).
    #[wasm_bindgen(js_name = subtle)]
    pub fn subtle() -> ContactShadowParams {
        ContactShadowParams {
            inner: CoreContactShadowPresets::subtle(),
        }
    }

    // Getters
    #[wasm_bindgen(getter)]
    pub fn darkness(&self) -> f64 {
        self.inner.darkness
    }

    #[wasm_bindgen(getter, js_name = blurRadius)]
    pub fn blur_radius(&self) -> f64 {
        self.inner.blur_radius
    }

    #[wasm_bindgen(getter, js_name = offsetY)]
    pub fn offset_y(&self) -> f64 {
        self.inner.offset_y
    }

    #[wasm_bindgen(getter)]
    pub fn spread(&self) -> f64 {
        self.inner.spread
    }
}

/// Contact shadow result with computed properties.
///
/// Represents a calculated contact shadow ready for CSS rendering.
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct ContactShadow {
    inner: momoto_materials::shadow_engine::contact_shadow::ContactShadow,
}

#[wasm_bindgen]
impl ContactShadow {
    /// Get the shadow color as OKLCH.
    #[wasm_bindgen(getter)]
    pub fn color(&self) -> OKLCH {
        OKLCH {
            inner: self.inner.color,
        }
    }

    /// Get blur radius in pixels.
    #[wasm_bindgen(getter)]
    pub fn blur(&self) -> f64 {
        self.inner.blur
    }

    /// Get vertical offset in pixels.
    #[wasm_bindgen(getter, js_name = offsetY)]
    pub fn offset_y(&self) -> f64 {
        self.inner.offset_y
    }

    /// Get spread in pixels.
    #[wasm_bindgen(getter)]
    pub fn spread(&self) -> f64 {
        self.inner.spread
    }

    /// Get opacity (0.0-1.0).
    #[wasm_bindgen(getter)]
    pub fn opacity(&self) -> f64 {
        self.inner.opacity
    }

    /// Convert to CSS box-shadow string.
    ///
    /// # Example output
    ///
    /// `"0 0.5px 2.0px 0.0px oklch(0.050 0.003 240.0 / 0.75)"`
    #[wasm_bindgen(js_name = toCss)]
    pub fn to_css(&self) -> String {
        core_contact_shadow_to_css(&self.inner)
    }
}

/// Calculate contact shadow for a glass element.
///
/// Generates a sharp, dark shadow at the point where glass meets background.
///
/// # Arguments
///
/// * `params` - Contact shadow configuration
/// * `background` - Background color in OKLCH (affects shadow visibility)
/// * `glass_depth` - Perceived thickness of glass (affects shadow intensity, 0.0-2.0)
///
/// # Returns
///
/// Calculated contact shadow ready for CSS rendering.
///
/// # Example (JavaScript)
///
/// ```javascript
/// const params = ContactShadowParams.standard();
/// const background = new OKLCH(0.95, 0.01, 240.0); // Light background
/// const shadow = calculateContactShadow(params, background, 1.0);
///
/// element.style.boxShadow = shadow.toCss();
/// ```
#[wasm_bindgen(js_name = calculateContactShadow)]
pub fn calculate_contact_shadow(
    params: &ContactShadowParams,
    background: &OKLCH,
    glass_depth: f64,
) -> ContactShadow {
    ContactShadow {
        inner: core_calculate_contact_shadow(&params.inner, background.inner, glass_depth),
    }
}

// ============================================================================
// Glass Physics - Transmittance System
// ============================================================================

/// Optical properties of glass material
#[wasm_bindgen]
pub struct OpticalProperties {
    inner: CoreOpticalProperties,
}

#[wasm_bindgen]
impl OpticalProperties {
    /// Create with custom optical properties
    #[wasm_bindgen(constructor)]
    pub fn new(
        absorption_coefficient: f64,
        scattering_coefficient: f64,
        thickness: f64,
        refractive_index: f64,
    ) -> OpticalProperties {
        OpticalProperties {
            inner: CoreOpticalProperties {
                absorption_coefficient,
                scattering_coefficient,
                thickness,
                refractive_index,
            },
        }
    }

    /// Create default optical properties
    #[wasm_bindgen(js_name = default)]
    pub fn default_props() -> OpticalProperties {
        OpticalProperties {
            inner: CoreOpticalProperties::default(),
        }
    }

    /// Get absorption coefficient
    #[wasm_bindgen(getter, js_name = absorptionCoefficient)]
    pub fn absorption_coefficient(&self) -> f64 {
        self.inner.absorption_coefficient
    }

    /// Get scattering coefficient
    #[wasm_bindgen(getter, js_name = scatteringCoefficient)]
    pub fn scattering_coefficient(&self) -> f64 {
        self.inner.scattering_coefficient
    }

    /// Get thickness
    #[wasm_bindgen(getter)]
    pub fn thickness(&self) -> f64 {
        self.inner.thickness
    }

    /// Get refractive index
    #[wasm_bindgen(getter, js_name = refractiveIndex)]
    pub fn refractive_index(&self) -> f64 {
        self.inner.refractive_index
    }
}

/// Multi-layer transmittance result
#[wasm_bindgen]
pub struct LayerTransmittance {
    inner: CoreLayerTransmittance,
}

#[wasm_bindgen]
impl LayerTransmittance {
    /// Surface layer (edge highlight) - High reflectivity, bright
    #[wasm_bindgen(getter)]
    pub fn surface(&self) -> f64 {
        self.inner.surface
    }

    /// Volume layer (glass body) - Main transmittance value
    #[wasm_bindgen(getter)]
    pub fn volume(&self) -> f64 {
        self.inner.volume
    }

    /// Substrate layer (deep contact) - Darkest layer, creates depth
    #[wasm_bindgen(getter)]
    pub fn substrate(&self) -> f64 {
        self.inner.substrate
    }
}

/// Calculate multi-layer transmittance for realistic glass rendering
///
/// # Arguments
///
/// * `optical_props` - Optical properties of the glass
/// * `incident_intensity` - Incoming light intensity (0.0-1.0)
///
/// # Returns
///
/// Layer-separated transmittance values
#[wasm_bindgen(js_name = calculateMultiLayerTransmittance)]
pub fn calculate_multi_layer_transmittance(
    optical_props: &OpticalProperties,
    incident_intensity: f64,
) -> LayerTransmittance {
    LayerTransmittance {
        inner: core_calc_transmittance(&optical_props.inner, incident_intensity),
    }
}

// ============================================================================
// Glass Physics - Vec3 (3D Vector)
// ============================================================================

/// 3D vector for light direction and surface normals
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct Vec3 {
    inner: CoreVec3,
}

#[wasm_bindgen]
impl Vec3 {
    /// Create a new 3D vector
    #[wasm_bindgen(constructor)]
    pub fn new(x: f64, y: f64, z: f64) -> Vec3 {
        Vec3 {
            inner: CoreVec3::new(x, y, z),
        }
    }

    /// Get x component
    #[wasm_bindgen(getter)]
    pub fn x(&self) -> f64 {
        self.inner.x
    }

    /// Get y component
    #[wasm_bindgen(getter)]
    pub fn y(&self) -> f64 {
        self.inner.y
    }

    /// Get z component
    #[wasm_bindgen(getter)]
    pub fn z(&self) -> f64 {
        self.inner.z
    }

    /// Normalize vector to unit length
    pub fn normalize(&self) -> Vec3 {
        Vec3 {
            inner: self.inner.normalize(),
        }
    }

    /// Calculate dot product with another vector
    pub fn dot(&self, other: &Vec3) -> f64 {
        self.inner.dot(&other.inner)
    }

    /// Reflect vector around normal
    pub fn reflect(&self, normal: &Vec3) -> Vec3 {
        Vec3 {
            inner: self.inner.reflect(&normal.inner),
        }
    }
}

// ============================================================================
// Glass Physics - Glass Material
// ============================================================================

/// Physical glass material properties
#[wasm_bindgen]
pub struct GlassMaterial {
    inner: CoreGlassMaterial,
}

#[wasm_bindgen]
impl GlassMaterial {
    /// Create glass material with custom properties
    ///
    /// # Arguments
    ///
    /// * `ior` - Index of refraction (1.0-2.5, typical glass: 1.5)
    /// * `roughness` - Surface roughness (0.0-1.0, 0 = mirror-smooth)
    /// * `thickness` - Thickness in millimeters
    /// * `noise_scale` - Frosted texture amount (0.0-1.0)
    /// * `base_color` - Material tint color
    /// * `edge_power` - Fresnel edge sharpness (1.0-4.0)
    #[wasm_bindgen(constructor)]
    pub fn new(
        ior: f64,
        roughness: f64,
        thickness: f64,
        noise_scale: f64,
        base_color: &OKLCH,
        edge_power: f64,
    ) -> GlassMaterial {
        GlassMaterial {
            inner: CoreGlassMaterial {
                ior,
                roughness,
                thickness,
                noise_scale,
                base_color: base_color.inner,
                edge_power,
            },
        }
    }

    /// Create clear glass preset
    /// IOR: 1.5, Roughness: 0.05, Thickness: 2mm
    #[wasm_bindgen(js_name = clear)]
    pub fn clear() -> GlassMaterial {
        GlassMaterial {
            inner: CoreGlassMaterial::clear(),
        }
    }

    /// Create regular glass preset (Apple-like)
    /// IOR: 1.5, Roughness: 0.15, Thickness: 5mm
    #[wasm_bindgen(js_name = regular)]
    pub fn regular() -> GlassMaterial {
        GlassMaterial {
            inner: CoreGlassMaterial::regular(),
        }
    }

    /// Create thick glass preset
    /// IOR: 1.52, Roughness: 0.25, Thickness: 10mm
    #[wasm_bindgen(js_name = thick)]
    pub fn thick() -> GlassMaterial {
        GlassMaterial {
            inner: CoreGlassMaterial::thick(),
        }
    }

    /// Create frosted glass preset
    /// IOR: 1.5, Roughness: 0.6, Thickness: 8mm
    #[wasm_bindgen(js_name = frosted)]
    pub fn frosted() -> GlassMaterial {
        GlassMaterial {
            inner: CoreGlassMaterial::frosted(),
        }
    }

    /// Get index of refraction
    #[wasm_bindgen(getter)]
    pub fn ior(&self) -> f64 {
        self.inner.ior
    }

    /// Get surface roughness
    #[wasm_bindgen(getter)]
    pub fn roughness(&self) -> f64 {
        self.inner.roughness
    }

    /// Get thickness in millimeters
    #[wasm_bindgen(getter)]
    pub fn thickness(&self) -> f64 {
        self.inner.thickness
    }

    /// Get noise scale
    #[wasm_bindgen(getter, js_name = noiseScale)]
    pub fn noise_scale(&self) -> f64 {
        self.inner.noise_scale
    }

    /// Get base color
    #[wasm_bindgen(getter, js_name = baseColor)]
    pub fn base_color(&self) -> OKLCH {
        OKLCH {
            inner: self.inner.base_color,
        }
    }

    /// Get edge power
    #[wasm_bindgen(getter, js_name = edgePower)]
    pub fn edge_power(&self) -> f64 {
        self.inner.edge_power
    }

    /// Calculate Blinn-Phong shininess from roughness
    pub fn shininess(&self) -> f64 {
        self.inner.shininess()
    }

    /// Calculate scattering radius in millimeters (physical property)
    #[wasm_bindgen(js_name = scatteringRadiusMm)]
    pub fn scattering_radius_mm(&self) -> f64 {
        self.inner.scattering_radius_mm()
    }

    // REMOVED in v6.0.0: blurAmount() - Use scatteringRadiusMm() instead
    // JavaScript migration: const blurPx = scatteringRadiusMm() * (devicePixelRatio * 96 / 25.4);

    /// Calculate translucency (opacity 0-1)
    pub fn translucency(&self) -> f64 {
        self.inner.translucency()
    }

    /// Evaluate material properties based on context (Phase 3 pipeline)
    ///
    /// Performs full physics-based evaluation including Fresnel reflectance,
    /// Beer-Lambert absorption, and subsurface scattering.
    ///
    /// # Arguments
    ///
    /// * `context` - Material evaluation context (lighting, viewing angle, background)
    ///
    /// # Returns
    ///
    /// EvaluatedMaterial with all optical properties resolved
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// const glass = GlassMaterial.frosted();
    /// const context = EvalMaterialContext.default();
    /// const evaluated = glass.evaluate(context);
    /// console.log(`Opacity: ${evaluated.opacity}`);
    /// console.log(`Scattering: ${evaluated.scatteringRadiusMm}mm`);
    /// ```
    pub fn evaluate(&self, context: &EvalMaterialContext) -> EvaluatedMaterial {
        let evaluated = Evaluable::evaluate(&self.inner, &context.inner);
        EvaluatedMaterial { inner: evaluated }
    }

    /// Create a builder for custom glass materials (Gap 5 - P1).
    ///
    /// Provides a fluent API for creating glass materials with custom properties.
    /// Unset properties default to the "regular" preset values.
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// const custom = GlassMaterial.builder()
    ///     .ior(1.45)
    ///     .roughness(0.3)
    ///     .thickness(8.0)
    ///     .build();
    /// ```
    #[wasm_bindgen(js_name = builder)]
    pub fn builder_new() -> GlassMaterialBuilder {
        GlassMaterialBuilder::new()
    }
}

// ============================================================================
// Glass Physics - GlassMaterial Builder (Gap 5 - P1)
// ============================================================================

use momoto_core::material::GlassMaterialBuilder as CoreGlassMaterialBuilder;

/// Builder for creating custom GlassMaterial instances.
///
/// Provides a fluent API for creating glass materials with custom properties.
/// All unset properties default to the "regular" glass preset values.
///
/// # Example (JavaScript)
///
/// ```javascript
/// // Create a custom material
/// const custom = GlassMaterial.builder()
///     .ior(1.45)
///     .roughness(0.3)
///     .thickness(8.0)
///     .baseColor(new OKLCH(0.9, 0.05, 200.0))
///     .build();
///
/// // Or start from a preset and modify
/// const variant = GlassMaterial.builder()
///     .presetFrosted()
///     .thickness(12.0)
///     .build();
/// ```
#[wasm_bindgen]
pub struct GlassMaterialBuilder {
    inner: CoreGlassMaterialBuilder,
}

#[wasm_bindgen]
impl GlassMaterialBuilder {
    /// Create a new builder with no preset values.
    #[wasm_bindgen(constructor)]
    pub fn new() -> GlassMaterialBuilder {
        GlassMaterialBuilder {
            inner: CoreGlassMaterialBuilder::new(),
        }
    }

    /// Start from the "clear" preset.
    #[wasm_bindgen(js_name = presetClear)]
    pub fn preset_clear(self) -> GlassMaterialBuilder {
        GlassMaterialBuilder {
            inner: self.inner.preset_clear(),
        }
    }

    /// Start from the "regular" preset.
    #[wasm_bindgen(js_name = presetRegular)]
    pub fn preset_regular(self) -> GlassMaterialBuilder {
        GlassMaterialBuilder {
            inner: self.inner.preset_regular(),
        }
    }

    /// Start from the "thick" preset.
    #[wasm_bindgen(js_name = presetThick)]
    pub fn preset_thick(self) -> GlassMaterialBuilder {
        GlassMaterialBuilder {
            inner: self.inner.preset_thick(),
        }
    }

    /// Start from the "frosted" preset.
    #[wasm_bindgen(js_name = presetFrosted)]
    pub fn preset_frosted(self) -> GlassMaterialBuilder {
        GlassMaterialBuilder {
            inner: self.inner.preset_frosted(),
        }
    }

    /// Set the index of refraction (IOR).
    ///
    /// Valid range: 1.0 - 2.5
    pub fn ior(self, ior: f64) -> GlassMaterialBuilder {
        GlassMaterialBuilder {
            inner: self.inner.ior(ior),
        }
    }

    /// Set the surface roughness.
    ///
    /// Valid range: 0.0 - 1.0
    pub fn roughness(self, roughness: f64) -> GlassMaterialBuilder {
        GlassMaterialBuilder {
            inner: self.inner.roughness(roughness),
        }
    }

    /// Set the glass thickness in millimeters.
    pub fn thickness(self, mm: f64) -> GlassMaterialBuilder {
        GlassMaterialBuilder {
            inner: self.inner.thickness(mm),
        }
    }

    /// Set the noise scale for frosted texture.
    ///
    /// Valid range: 0.0 - 1.0
    #[wasm_bindgen(js_name = noiseScale)]
    pub fn noise_scale(self, scale: f64) -> GlassMaterialBuilder {
        GlassMaterialBuilder {
            inner: self.inner.noise_scale(scale),
        }
    }

    /// Set the base color tint.
    #[wasm_bindgen(js_name = baseColor)]
    pub fn base_color(self, color: &OKLCH) -> GlassMaterialBuilder {
        GlassMaterialBuilder {
            inner: self.inner.base_color(color.inner),
        }
    }

    /// Set the edge power for Fresnel glow.
    ///
    /// Valid range: 1.0 - 4.0
    #[wasm_bindgen(js_name = edgePower)]
    pub fn edge_power(self, power: f64) -> GlassMaterialBuilder {
        GlassMaterialBuilder {
            inner: self.inner.edge_power(power),
        }
    }

    /// Build the GlassMaterial.
    ///
    /// Any unset properties default to the "regular" preset values.
    pub fn build(self) -> GlassMaterial {
        GlassMaterial {
            inner: self.inner.build(),
        }
    }
}

impl Default for GlassMaterialBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Glass Physics - Perlin Noise
// ============================================================================

/// Perlin noise generator for frosted glass textures
#[wasm_bindgen]
pub struct PerlinNoise {
    inner: CorePerlinNoise,
}

#[wasm_bindgen]
impl PerlinNoise {
    /// Create new Perlin noise generator
    ///
    /// # Arguments
    ///
    /// * `seed` - Random seed for reproducibility
    /// * `octaves` - Number of noise layers (1-8)
    /// * `persistence` - Amplitude decrease per octave (0.0-1.0)
    /// * `lacunarity` - Frequency increase per octave (typically 2.0)
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u32, octaves: u32, persistence: f64, lacunarity: f64) -> PerlinNoise {
        PerlinNoise {
            inner: CorePerlinNoise::new(seed, octaves, persistence, lacunarity),
        }
    }

    /// Generate 2D noise value at position
    ///
    /// Returns value in range [-1.0, 1.0]
    #[wasm_bindgen(js_name = noise2D)]
    pub fn noise_2d(&self, x: f64, y: f64) -> f64 {
        self.inner.noise_2d(x, y)
    }

    /// Generate fractal (multi-octave) 2D noise
    ///
    /// Returns value in range [-1.0, 1.0]
    #[wasm_bindgen(js_name = fractalNoise2D)]
    pub fn fractal_noise_2d(&self, x: f64, y: f64) -> f64 {
        self.inner.fractal_noise_2d(x, y)
    }

    /// Generate RGBA texture buffer
    ///
    /// # Arguments
    ///
    /// * `width` - Texture width in pixels
    /// * `height` - Texture height in pixels
    /// * `scale` - Noise scale factor (typical: 0.01-0.1)
    ///
    /// # Returns
    ///
    /// Uint8Array with RGBA values (width * height * 4 bytes)
    #[wasm_bindgen(js_name = generateTexture)]
    pub fn generate_texture(&self, width: u32, height: u32, scale: f64) -> Vec<u8> {
        self.inner.generate_texture(width, height, scale)
    }

    /// Create clear glass noise preset (1 octave)
    #[wasm_bindgen(js_name = clearGlass)]
    pub fn clear_glass() -> PerlinNoise {
        PerlinNoise {
            inner: noise_presets::clear_glass(),
        }
    }

    /// Create regular glass noise preset (3 octaves)
    #[wasm_bindgen(js_name = regularGlass)]
    pub fn regular_glass() -> PerlinNoise {
        PerlinNoise {
            inner: noise_presets::regular_glass(),
        }
    }

    /// Create thick glass noise preset (4 octaves)
    #[wasm_bindgen(js_name = thickGlass)]
    pub fn thick_glass() -> PerlinNoise {
        PerlinNoise {
            inner: noise_presets::thick_glass(),
        }
    }

    /// Create frosted glass noise preset (6 octaves)
    #[wasm_bindgen(js_name = frostedGlass)]
    pub fn frosted_glass() -> PerlinNoise {
        PerlinNoise {
            inner: noise_presets::frosted_glass(),
        }
    }
}

// ============================================================================
// Glass Physics - Fresnel Calculations
// ============================================================================

/// Calculate Fresnel reflectance using Schlick's approximation
///
/// Fast approximation of angle-dependent reflectivity (<4% error vs full Fresnel).
///
/// # Arguments
///
/// * `ior1` - Refractive index of first medium (e.g., 1.0 for air)
/// * `ior2` - Refractive index of second medium (e.g., 1.5 for glass)
/// * `cos_theta` - Cosine of view angle (0 = grazing, 1 = perpendicular)
///
/// # Returns
///
/// Reflectance value (0.0 to 1.0)
#[wasm_bindgen(js_name = fresnelSchlick)]
pub fn fresnel_schlick(ior1: f64, ior2: f64, cos_theta: f64) -> f64 {
    fresnel::fresnel_schlick(ior1, ior2, cos_theta)
}

/// Calculate full Fresnel equations (s and p polarization)
///
/// More accurate than Schlick's approximation but slower.
///
/// # Arguments
///
/// * `ior1` - Refractive index of first medium
/// * `ior2` - Refractive index of second medium
/// * `cos_theta_i` - Cosine of incident angle
///
/// # Returns
///
/// Tuple of (Rs, Rp) - reflectance for s and p polarization
#[wasm_bindgen(js_name = fresnelFull)]
pub fn fresnel_full(ior1: f64, ior2: f64, cos_theta_i: f64) -> Vec<f64> {
    let (rs, rp) = fresnel::fresnel_full(ior1, ior2, cos_theta_i);
    vec![rs, rp]
}

/// Calculate Brewster's angle (minimum reflectance for p-polarization)
///
/// # Arguments
///
/// * `ior1` - Refractive index of first medium
/// * `ior2` - Refractive index of second medium
///
/// # Returns
///
/// Brewster's angle in degrees
#[wasm_bindgen(js_name = brewsterAngle)]
pub fn brewster_angle(ior1: f64, ior2: f64) -> f64 {
    fresnel::brewster_angle(ior1, ior2)
}

/// Calculate view angle between normal and view direction
///
/// # Arguments
///
/// * `normal` - Surface normal vector
/// * `view_dir` - View direction vector
///
/// # Returns
///
/// Cosine of angle (for use in Fresnel calculations)
#[wasm_bindgen(js_name = calculateViewAngle)]
pub fn calculate_view_angle(normal: &Vec3, view_dir: &Vec3) -> f64 {
    fresnel::calculate_view_angle(normal.inner, view_dir.inner)
}

/// Calculate edge intensity for edge glow effect
///
/// # Arguments
///
/// * `cos_theta` - Cosine of view angle
/// * `edge_power` - Power curve exponent (1.0-4.0, higher = sharper edge)
///
/// # Returns
///
/// Edge intensity (0.0 at center to 1.0 at edge)
#[wasm_bindgen(js_name = edgeIntensity)]
pub fn edge_intensity(cos_theta: f64, edge_power: f64) -> f64 {
    fresnel::edge_intensity(cos_theta, edge_power)
}

/// Generate CSS-ready Fresnel gradient
///
/// # Arguments
///
/// * `ior` - Index of refraction (e.g., 1.5 for glass)
/// * `samples` - Number of gradient stops (typically 8-16)
/// * `edge_power` - Edge sharpness (1.0-4.0)
///
/// # Returns
///
/// Flat array of [position, intensity, position, intensity, ...]
#[wasm_bindgen(js_name = generateFresnelGradient)]
pub fn generate_fresnel_gradient(ior: f64, samples: usize, edge_power: f64) -> Vec<f64> {
    let gradient = fresnel::generate_fresnel_gradient(ior, samples, edge_power);
    gradient
        .into_iter()
        .flat_map(|(pos, intensity)| vec![pos, intensity])
        .collect()
}

// ============================================================================
// Glass Physics - Blinn-Phong Specular
// ============================================================================

/// Calculate Blinn-Phong specular highlight
///
/// Uses halfway vector for faster and more accurate specular than Phong model.
///
/// # Arguments
///
/// * `normal` - Surface normal vector
/// * `light_dir` - Light direction vector (from surface to light)
/// * `view_dir` - View direction vector (from surface to camera)
/// * `shininess` - Material shininess (1-256, higher = sharper highlight)
///
/// # Returns
///
/// Specular intensity (0.0 to 1.0)
#[wasm_bindgen(js_name = blinnPhongSpecular)]
pub fn blinn_phong_specular(
    normal: &Vec3,
    light_dir: &Vec3,
    view_dir: &Vec3,
    shininess: f64,
) -> f64 {
    blinn_phong::blinn_phong_specular(normal.inner, light_dir.inner, view_dir.inner, shininess)
}

/// Calculate multi-layer specular highlights
///
/// Generates 4 layers: main, secondary, top edge, left edge
///
/// # Arguments
///
/// * `normal` - Surface normal vector
/// * `light_dir` - Light direction vector
/// * `view_dir` - View direction vector
/// * `base_shininess` - Base material shininess
///
/// # Returns
///
/// Flat array of [intensity1, x1, y1, size1, intensity2, x2, y2, size2, ...]
#[wasm_bindgen(js_name = calculateSpecularLayers)]
pub fn calculate_specular_layers(
    normal: &Vec3,
    light_dir: &Vec3,
    view_dir: &Vec3,
    base_shininess: f64,
) -> Vec<f64> {
    let layers = blinn_phong::calculate_specular_layers(
        normal.inner,
        light_dir.inner,
        view_dir.inner,
        base_shininess,
    );
    layers
        .into_iter()
        .flat_map(|(intensity, x, y, size)| vec![intensity, x, y, size])
        .collect()
}

/// Convert PBR roughness to Blinn-Phong shininess
///
/// Maps roughness (0.0-1.0) to shininess (1-256) using perceptually linear curve.
///
/// # Arguments
///
/// * `roughness` - Surface roughness (0.0 = smooth, 1.0 = rough)
///
/// # Returns
///
/// Shininess value for Blinn-Phong (1-256)
#[wasm_bindgen(js_name = roughnessToShininess)]
pub fn roughness_to_shininess(roughness: f64) -> f64 {
    blinn_phong::roughness_to_shininess(roughness)
}

/// Calculate CSS position for highlight from light direction
///
/// # Arguments
///
/// * `light_dir` - Light direction vector
///
/// # Returns
///
/// Array of [x, y] in percentage (-50 to 150)
#[wasm_bindgen(js_name = calculateHighlightPosition)]
pub fn calculate_highlight_position(light_dir: &Vec3) -> Vec<f64> {
    let (x, y) = blinn_phong::calculate_highlight_position(light_dir.inner);
    vec![x, y]
}

// ============================================================================
// Glass Physics - LUT Functions (Fast Approximations)
// ============================================================================

/// Fast Fresnel calculation using lookup table
///
/// 5x faster than direct calculation with <1% error.
/// Ideal for batch processing or performance-critical paths.
///
/// # Arguments
///
/// * `ior` - Index of refraction (1.0 to 2.5)
/// * `cos_theta` - Cosine of view angle (0.0 to 1.0)
///
/// # Returns
///
/// Fresnel reflectance (0.0 to 1.0)
#[wasm_bindgen(js_name = fresnelFast)]
pub fn fresnel_fast_wasm(ior: f64, cos_theta: f64) -> f64 {
    fresnel_fast(ior, cos_theta)
}

/// Fast Beer-Lambert attenuation using lookup table
///
/// 4x faster than exp() calculation with <1% error.
///
/// # Arguments
///
/// * `absorption` - Absorption coefficient per mm (0.0 to 1.0)
/// * `distance` - Path length in mm (0.0 to 100.0)
///
/// # Returns
///
/// Transmittance (0.0 to 1.0)
#[wasm_bindgen(js_name = beerLambertFast)]
pub fn beer_lambert_fast_wasm(absorption: f64, distance: f64) -> f64 {
    beer_lambert_fast(absorption, distance)
}

/// Get total LUT memory usage in bytes
#[wasm_bindgen(js_name = totalLutMemory)]
pub fn total_lut_memory_wasm() -> usize {
    total_lut_memory()
}

// ============================================================================
// Material Context System
// ============================================================================

/// Lighting context for material evaluation
#[wasm_bindgen]
pub struct LightingContext {
    inner: CoreLightingContext,
}

#[wasm_bindgen]
impl LightingContext {
    /// Create studio lighting preset
    #[wasm_bindgen(js_name = studio)]
    pub fn studio() -> LightingContext {
        LightingContext {
            inner: CoreLightingContext::studio(),
        }
    }

    /// Create outdoor lighting preset
    #[wasm_bindgen(js_name = outdoor)]
    pub fn outdoor() -> LightingContext {
        LightingContext {
            inner: CoreLightingContext::outdoor(),
        }
    }

    /// Create dramatic lighting preset
    #[wasm_bindgen(js_name = dramatic)]
    pub fn dramatic() -> LightingContext {
        LightingContext {
            inner: CoreLightingContext::dramatic(),
        }
    }

    /// Create soft lighting preset
    #[wasm_bindgen(js_name = soft)]
    pub fn soft() -> LightingContext {
        LightingContext {
            inner: CoreLightingContext::soft(),
        }
    }

    /// Create neutral lighting preset
    #[wasm_bindgen(js_name = neutral)]
    pub fn neutral() -> LightingContext {
        LightingContext {
            inner: CoreLightingContext::neutral(),
        }
    }
}

/// Background context (what's behind the material)
#[wasm_bindgen]
pub struct BackgroundContext {
    inner: CoreBackgroundContext,
}

#[wasm_bindgen]
impl BackgroundContext {
    /// White background preset
    #[wasm_bindgen(js_name = white)]
    pub fn white() -> BackgroundContext {
        BackgroundContext {
            inner: CoreBackgroundContext::white(),
        }
    }

    /// Black background preset
    #[wasm_bindgen(js_name = black)]
    pub fn black() -> BackgroundContext {
        BackgroundContext {
            inner: CoreBackgroundContext::black(),
        }
    }

    /// Gray background preset
    #[wasm_bindgen(js_name = gray)]
    pub fn gray() -> BackgroundContext {
        BackgroundContext {
            inner: CoreBackgroundContext::gray(),
        }
    }

    /// Colorful background preset
    #[wasm_bindgen(js_name = colorful)]
    pub fn colorful() -> BackgroundContext {
        BackgroundContext {
            inner: CoreBackgroundContext::colorful(),
        }
    }

    /// Sky background preset
    #[wasm_bindgen(js_name = sky)]
    pub fn sky() -> BackgroundContext {
        BackgroundContext {
            inner: CoreBackgroundContext::sky(),
        }
    }
}

/// View context (observer perspective)
#[wasm_bindgen]
pub struct ViewContext {
    inner: CoreViewContext,
}

#[wasm_bindgen]
impl ViewContext {
    /// Perpendicular view preset
    #[wasm_bindgen(js_name = perpendicular)]
    pub fn perpendicular() -> ViewContext {
        ViewContext {
            inner: CoreViewContext::perpendicular(),
        }
    }

    /// Oblique view preset (45° angle)
    #[wasm_bindgen(js_name = oblique)]
    pub fn oblique() -> ViewContext {
        ViewContext {
            inner: CoreViewContext::oblique(),
        }
    }

    /// Grazing angle view preset
    #[wasm_bindgen(js_name = grazing)]
    pub fn grazing() -> ViewContext {
        ViewContext {
            inner: CoreViewContext::grazing(),
        }
    }
}

/// Complete material context (lighting + background + view)
#[wasm_bindgen]
pub struct MaterialContext {
    inner: CoreMaterialContext,
}

#[wasm_bindgen]
impl MaterialContext {
    /// Create studio preset context
    #[wasm_bindgen(js_name = studio)]
    pub fn studio() -> MaterialContext {
        MaterialContext {
            inner: CoreMaterialContext::studio(),
        }
    }

    /// Create outdoor preset context
    #[wasm_bindgen(js_name = outdoor)]
    pub fn outdoor() -> MaterialContext {
        MaterialContext {
            inner: CoreMaterialContext::outdoor(),
        }
    }

    /// Create dramatic preset context
    #[wasm_bindgen(js_name = dramatic)]
    pub fn dramatic() -> MaterialContext {
        MaterialContext {
            inner: CoreMaterialContext::dramatic(),
        }
    }

    /// Create neutral preset context
    #[wasm_bindgen(js_name = neutral)]
    pub fn neutral() -> MaterialContext {
        MaterialContext {
            inner: CoreMaterialContext::neutral(),
        }
    }

    /// Create showcase preset context
    #[wasm_bindgen(js_name = showcase)]
    pub fn showcase() -> MaterialContext {
        MaterialContext {
            inner: CoreMaterialContext::showcase(),
        }
    }
}

// ============================================================================
// Batch Evaluation API
// ============================================================================

/// Batch material input for efficient multi-material evaluation
#[wasm_bindgen]
pub struct BatchMaterialInput {
    inner: CoreBatchInput,
}

#[wasm_bindgen]
impl BatchMaterialInput {
    /// Create new empty batch input
    #[wasm_bindgen(constructor)]
    pub fn new() -> BatchMaterialInput {
        BatchMaterialInput {
            inner: CoreBatchInput::new(),
        }
    }

    /// Add a material to the batch
    ///
    /// # Arguments
    ///
    /// * `ior` - Index of refraction
    /// * `roughness` - Surface roughness (0-1)
    /// * `thickness` - Thickness in mm
    /// * `absorption` - Absorption coefficient per mm
    pub fn push(&mut self, ior: f64, roughness: f64, thickness: f64, absorption: f64) {
        self.inner.push(ior, roughness, thickness, absorption);
    }

    /// Get number of materials in batch
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if batch is empty
    #[wasm_bindgen(js_name = isEmpty)]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

/// Batch evaluation result
#[wasm_bindgen]
pub struct BatchResult {
    inner: CoreBatchResult,
}

#[wasm_bindgen]
impl BatchResult {
    /// Number of materials evaluated
    #[wasm_bindgen(getter)]
    pub fn count(&self) -> usize {
        self.inner.count
    }

    /// Get opacity array
    #[wasm_bindgen(js_name = getOpacity)]
    pub fn get_opacity(&self) -> Vec<f64> {
        self.inner.opacity.clone()
    }

    /// Get blur array
    #[wasm_bindgen(js_name = getBlur)]
    pub fn get_blur(&self) -> Vec<f64> {
        self.inner.blur.clone()
    }

    /// Get Fresnel normal incidence array
    #[wasm_bindgen(js_name = getFresnelNormal)]
    pub fn get_fresnel_normal(&self) -> Vec<f64> {
        self.inner.fresnel_normal.clone()
    }

    /// Get Fresnel grazing angle array
    #[wasm_bindgen(js_name = getFresnelGrazing)]
    pub fn get_fresnel_grazing(&self) -> Vec<f64> {
        self.inner.fresnel_grazing.clone()
    }

    /// Get transmittance array
    #[wasm_bindgen(js_name = getTransmittance)]
    pub fn get_transmittance(&self) -> Vec<f64> {
        self.inner.transmittance.clone()
    }
}

/// Batch evaluator for efficient multi-material processing
#[wasm_bindgen]
pub struct BatchEvaluator {
    inner: CoreBatchEvaluator,
}

#[wasm_bindgen]
impl BatchEvaluator {
    /// Create new batch evaluator with default context
    #[wasm_bindgen(constructor)]
    pub fn new() -> BatchEvaluator {
        BatchEvaluator {
            inner: CoreBatchEvaluator::new(),
        }
    }

    /// Create batch evaluator with custom context
    #[wasm_bindgen(js_name = withContext)]
    pub fn with_context(context: &MaterialContext) -> BatchEvaluator {
        BatchEvaluator {
            inner: CoreBatchEvaluator::with_context(context.inner),
        }
    }

    /// Evaluate batch of materials
    ///
    /// Returns result object with arrays for each property.
    /// This is 7-10x faster than evaluating materials individually
    /// when called from JavaScript (reduces JS↔WASM crossings).
    pub fn evaluate(&self, input: &BatchMaterialInput) -> Result<BatchResult, JsValue> {
        self.inner
            .evaluate(&input.inner)
            .map(|result| BatchResult { inner: result })
            .map_err(|e| JsValue::from_str(&e))
    }

    /// Update context
    #[wasm_bindgen(js_name = setContext)]
    pub fn set_context(&mut self, context: &MaterialContext) {
        self.inner.set_context(context.inner);
    }
}

// ============================================================================
// Glass Physics Engine - High-Level API
// ============================================================================

/// High-level glass physics engine combining all calculations
#[wasm_bindgen]
pub struct GlassPhysicsEngine {
    material: CoreGlassMaterial,
    noise: CorePerlinNoise,
}

#[wasm_bindgen]
impl GlassPhysicsEngine {
    /// Create new glass physics engine with material preset
    ///
    /// # Arguments
    ///
    /// * `preset` - "clear", "regular", "thick", or "frosted"
    #[wasm_bindgen(constructor)]
    pub fn new(preset: &str) -> Result<GlassPhysicsEngine, JsValue> {
        let material = match preset {
            "clear" => CoreGlassMaterial::clear(),
            "regular" => CoreGlassMaterial::regular(),
            "thick" => CoreGlassMaterial::thick(),
            "frosted" => CoreGlassMaterial::frosted(),
            _ => {
                return Err(JsValue::from_str(
                    "Invalid preset. Use: clear, regular, thick, or frosted",
                ))
            }
        };

        let noise = match preset {
            "clear" => noise_presets::clear_glass(),
            "regular" => noise_presets::regular_glass(),
            "thick" => noise_presets::thick_glass(),
            "frosted" => noise_presets::frosted_glass(),
            _ => noise_presets::regular_glass(),
        };

        Ok(GlassPhysicsEngine { material, noise })
    }

    /// Create with custom material and noise
    #[wasm_bindgen(js_name = withCustom)]
    pub fn with_custom(material: &GlassMaterial, noise: &PerlinNoise) -> GlassPhysicsEngine {
        GlassPhysicsEngine {
            material: material.inner,
            noise: noise.inner.clone(),
        }
    }

    /// Get material
    #[wasm_bindgen(getter)]
    pub fn material(&self) -> GlassMaterial {
        GlassMaterial {
            inner: self.material,
        }
    }

    /// Calculate complete glass properties for rendering
    ///
    /// Returns object with all CSS-ready values:
    /// - opacity: Material translucency (0-1)
    /// - blur: Blur amount in pixels
    /// - fresnel: Array of gradient stops [position, intensity, ...]
    /// - specular: Array of layer data [intensity, x, y, size, ...]
    /// - noise: Noise texture scale
    #[wasm_bindgen(js_name = calculateProperties)]
    pub fn calculate_properties(
        &self,
        normal: &Vec3,
        light_dir: &Vec3,
        view_dir: &Vec3,
    ) -> js_sys::Object {
        let obj = js_sys::Object::new();

        // Basic properties
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("opacity"),
            &JsValue::from_f64(self.material.translucency()),
        )
        .unwrap();

        // v6.0.0: Return scatteringMm instead of blurPx - consumers convert using DPI
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("scatteringMm"),
            &JsValue::from_f64(self.material.scattering_radius_mm()),
        )
        .unwrap();

        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("noiseScale"),
            &JsValue::from_f64(self.material.noise_scale),
        )
        .unwrap();

        // Fresnel gradient
        let cos_theta = calculate_view_angle(
            &Vec3 {
                inner: normal.inner,
            },
            &Vec3 {
                inner: view_dir.inner,
            },
        );
        let fresnel_value = fresnel_schlick(1.0, self.material.ior, cos_theta);
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("fresnelValue"),
            &JsValue::from_f64(fresnel_value),
        )
        .unwrap();

        let gradient = generate_fresnel_gradient(self.material.ior, 12, self.material.edge_power);
        let gradient_array =
            js_sys::Array::from_iter(gradient.iter().map(|v| JsValue::from_f64(*v)));
        js_sys::Reflect::set(&obj, &JsValue::from_str("fresnelGradient"), &gradient_array).unwrap();

        // Specular layers
        let shininess = self.material.shininess();
        let specular_layers = calculate_specular_layers(
            &Vec3 {
                inner: normal.inner,
            },
            &Vec3 {
                inner: light_dir.inner,
            },
            &Vec3 {
                inner: view_dir.inner,
            },
            shininess,
        );
        let specular_array =
            js_sys::Array::from_iter(specular_layers.iter().map(|v| JsValue::from_f64(*v)));
        js_sys::Reflect::set(&obj, &JsValue::from_str("specularLayers"), &specular_array).unwrap();

        obj
    }

    /// Generate noise texture
    #[wasm_bindgen(js_name = generateNoiseTexture)]
    pub fn generate_noise_texture(&self, width: u32, height: u32, scale: f64) -> Vec<u8> {
        self.noise.generate_texture(width, height, scale)
    }
}

// ============================================================================
// Phase 3: Material Evaluation Context
// ============================================================================

/// Material evaluation context for physics-based rendering
///
/// Defines the viewing and lighting conditions for material evaluation.
#[wasm_bindgen]
pub struct EvalMaterialContext {
    inner: CoreEvalMaterialContext,
}

#[wasm_bindgen]
impl EvalMaterialContext {
    /// Create default evaluation context
    ///
    /// Uses standard viewing angle (0° = looking straight at surface),
    /// neutral background, and default lighting.
    #[wasm_bindgen(constructor)]
    pub fn new() -> EvalMaterialContext {
        EvalMaterialContext {
            inner: CoreEvalMaterialContext::default(),
        }
    }

    /// Create context with custom background color
    #[wasm_bindgen(js_name = withBackground)]
    pub fn with_background(background: &OKLCH) -> EvalMaterialContext {
        EvalMaterialContext {
            inner: CoreEvalMaterialContext::with_background(background.inner),
        }
    }

    /// Create context with custom viewing angle
    ///
    /// # Arguments
    ///
    /// * `angle_deg` - Viewing angle in degrees (0° = perpendicular, 90° = edge-on)
    #[wasm_bindgen(js_name = withViewingAngle)]
    pub fn with_viewing_angle(angle_deg: f64) -> EvalMaterialContext {
        EvalMaterialContext {
            inner: CoreEvalMaterialContext::at_angle(angle_deg),
        }
    }

    /// Get background color
    #[wasm_bindgen(getter)]
    pub fn background(&self) -> OKLCH {
        OKLCH {
            inner: self.inner.background,
        }
    }

    /// Get viewing angle in degrees
    #[wasm_bindgen(getter, js_name = viewingAngle)]
    pub fn viewing_angle(&self) -> f64 {
        self.inner.viewing_angle_deg
    }

    /// Get ambient light intensity
    #[wasm_bindgen(getter, js_name = ambientLight)]
    pub fn ambient_light(&self) -> f64 {
        self.inner.ambient_light
    }

    /// Get key light intensity
    #[wasm_bindgen(getter, js_name = keyLight)]
    pub fn key_light(&self) -> f64 {
        self.inner.key_light
    }
}

// ============================================================================
// Phase 3: Evaluated Material (Result of evaluate())
// ============================================================================

/// Evaluated material with all optical properties resolved
///
/// This is the output of GlassMaterial.evaluate() and contains
/// all computed physics-based properties ready for rendering.
#[wasm_bindgen]
pub struct EvaluatedMaterial {
    pub(crate) inner: CoreEvaluatedMaterial,
}

#[wasm_bindgen]
impl EvaluatedMaterial {
    /// Get base color (RGB in linear space)
    #[wasm_bindgen(js_name = baseColor)]
    pub fn base_color_rgb(&self) -> Vec<f64> {
        vec![
            self.inner.base_color.r,
            self.inner.base_color.g,
            self.inner.base_color.b,
        ]
    }

    /// Get final opacity (0.0-1.0)
    #[wasm_bindgen(getter)]
    pub fn opacity(&self) -> f64 {
        self.inner.opacity
    }

    /// Get Fresnel reflectance at normal incidence (F0)
    #[wasm_bindgen(getter, js_name = fresnelF0)]
    pub fn fresnel_f0(&self) -> f64 {
        self.inner.fresnel_f0
    }

    /// Get edge intensity for Fresnel glow
    #[wasm_bindgen(getter, js_name = fresnelEdgeIntensity)]
    pub fn fresnel_edge_intensity(&self) -> f64 {
        self.inner.fresnel_edge_intensity
    }

    /// Get index of refraction (if applicable)
    #[wasm_bindgen(getter)]
    pub fn ior(&self) -> Option<f64> {
        self.inner.index_of_refraction
    }

    /// Get surface roughness (0.0-1.0)
    #[wasm_bindgen(getter)]
    pub fn roughness(&self) -> f64 {
        self.inner.roughness
    }

    /// Get scattering radius in millimeters (physical property)
    #[wasm_bindgen(getter, js_name = scatteringRadiusMm)]
    pub fn scattering_radius_mm(&self) -> f64 {
        self.inner.scattering_radius_mm
    }

    /// Get blur amount in CSS pixels (DEPRECATED)
    ///
    /// **DEPRECATED:** Use scatteringRadiusMm instead and convert in your renderer.
    /// This method assumes 96 DPI and will be removed in v6.0.
    #[wasm_bindgen(getter, js_name = blurPx)]
    #[deprecated(since = "5.0.0", note = "Use scatteringRadiusMm instead")]
    pub fn blur_px(&self) -> f64 {
        const MM_TO_PX: f64 = 3.779527559;
        self.inner.scattering_radius_mm * MM_TO_PX
    }

    /// Get specular intensity
    #[wasm_bindgen(getter, js_name = specularIntensity)]
    pub fn specular_intensity(&self) -> f64 {
        self.inner.specular_intensity
    }

    /// Get specular shininess
    #[wasm_bindgen(getter, js_name = specularShininess)]
    pub fn specular_shininess(&self) -> f64 {
        self.inner.specular_shininess
    }

    /// Get thickness in millimeters
    #[wasm_bindgen(getter, js_name = thicknessMm)]
    pub fn thickness_mm(&self) -> f64 {
        self.inner.thickness_mm
    }

    /// Get absorption coefficients (RGB)
    #[wasm_bindgen(getter)]
    pub fn absorption(&self) -> Vec<f64> {
        self.inner.absorption.to_vec()
    }

    /// Get scattering coefficients (RGB)
    #[wasm_bindgen(getter)]
    pub fn scattering(&self) -> Vec<f64> {
        self.inner.scattering.to_vec()
    }
}

impl EvaluatedMaterial {
    pub(crate) fn to_core(&self) -> &CoreEvaluatedMaterial {
        &self.inner
    }
}

// ============================================================================
// Phase 3: Render Context
// ============================================================================

/// Rendering context for backend rendering
///
/// Defines the target environment and capabilities for rendering.
#[wasm_bindgen]
pub struct RenderContext {
    pub(crate) inner: CoreRenderContext,
}

#[wasm_bindgen]
impl RenderContext {
    /// Create desktop rendering context (1920x1080, sRGB)
    #[wasm_bindgen(js_name = desktop)]
    pub fn desktop() -> RenderContext {
        RenderContext {
            inner: CoreRenderContext::desktop(),
        }
    }

    /// Create mobile rendering context (375x667, Display P3 if supported)
    pub fn mobile() -> RenderContext {
        RenderContext {
            inner: CoreRenderContext::mobile(),
        }
    }

    /// Create 4K rendering context
    #[wasm_bindgen(js_name = fourK)]
    pub fn four_k() -> RenderContext {
        RenderContext {
            inner: CoreRenderContext::four_k(),
        }
    }

    /// Create custom rendering context
    ///
    /// # Arguments
    ///
    /// * `viewport_width` - Viewport width in CSS pixels
    /// * `viewport_height` - Viewport height in CSS pixels
    /// * `pixel_density` - Device pixel density (1.0 = standard, 2.0 = retina)
    #[wasm_bindgen(constructor)]
    pub fn new(viewport_width: u32, viewport_height: u32, pixel_density: f64) -> RenderContext {
        RenderContext {
            inner: CoreRenderContext {
                viewport_width,
                viewport_height,
                pixel_density,
                viewing_distance_m: 0.6,
                color_space: CoreColorSpace::SRgb,
                hdr: false,
                capabilities: std::collections::HashMap::new(),
                medium: CoreTargetMedium::Screen,
                background_luminance: 0.95,
                accessibility_mode: None,
            },
        }
    }

    /// Get viewport width
    #[wasm_bindgen(getter, js_name = viewportWidth)]
    pub fn viewport_width(&self) -> u32 {
        self.inner.viewport_width
    }

    /// Get viewport height
    #[wasm_bindgen(getter, js_name = viewportHeight)]
    pub fn viewport_height(&self) -> u32 {
        self.inner.viewport_height
    }

    /// Get pixel density
    #[wasm_bindgen(getter, js_name = pixelDensity)]
    pub fn pixel_density(&self) -> f64 {
        self.inner.pixel_density
    }
}

impl RenderContext {
    pub(crate) fn to_core(&self) -> &CoreRenderContext {
        &self.inner
    }
}

// ============================================================================
// Phase 3: CSS Backend
// ============================================================================

/// CSS rendering backend
///
/// Converts evaluated materials to CSS strings with
/// backdrop-filter, background, and other CSS properties.
#[wasm_bindgen]
pub struct CssBackend {
    inner: CoreCssBackend,
}

#[wasm_bindgen]
impl CssBackend {
    /// Create new CSS backend
    #[wasm_bindgen(constructor)]
    pub fn new() -> CssBackend {
        CssBackend {
            inner: CoreCssBackend::new(),
        }
    }

    /// Render evaluated material to CSS string
    ///
    /// # Arguments
    ///
    /// * `material` - Evaluated material with resolved properties
    /// * `context` - Rendering context
    ///
    /// # Returns
    ///
    /// CSS string with all material properties, or error
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// const glass = GlassMaterial.frosted();
    /// const evalCtx = EvalMaterialContext.new();
    /// const evaluated = glass.evaluate(evalCtx);
    ///
    /// const backend = new CssBackend();
    /// const renderCtx = RenderContext.desktop();
    /// const css = backend.render(evaluated, renderCtx);
    /// console.log(css); // "backdrop-filter: blur(24px); background: ..."
    /// ```
    pub fn render(
        &self,
        material: &EvaluatedMaterial,
        context: &RenderContext,
    ) -> Result<String, JsValue> {
        RenderBackend::render(&self.inner, &material.inner, &context.inner)
            .map_err(|e| JsValue::from_str(&format!("Render error: {:?}", e)))
    }

    /// Get backend name
    pub fn name(&self) -> String {
        RenderBackend::name(&self.inner).to_string()
    }
}

// ============================================================================
// Phase 3: Convenience Functions
// ============================================================================

/// Evaluate and render glass material to CSS in one call (convenience function)
///
/// This is a shortcut for:
/// 1. glass.evaluate(materialContext)
/// 2. backend.render(evaluated, renderContext)
///
/// # Arguments
///
/// * `glass` - Glass material to render
/// * `material_context` - Evaluation context (viewing angle, background, etc.)
/// * `render_context` - Rendering context (viewport, pixel ratio, etc.)
///
/// # Returns
///
/// CSS string ready to apply to DOM element
///
/// # Example (JavaScript)
///
/// ```javascript
/// const glass = GlassMaterial.frosted();
/// const materialCtx = EvalMaterialContext.new();
/// const renderCtx = RenderContext.desktop();
///
/// const css = evaluateAndRenderCss(glass, materialCtx, renderCtx);
/// document.getElementById('panel').style.cssText = css;
/// ```
#[wasm_bindgen(js_name = evaluateAndRenderCss)]
pub fn evaluate_and_render_css(
    glass: &GlassMaterial,
    material_context: &EvalMaterialContext,
    render_context: &RenderContext,
) -> Result<String, JsValue> {
    // Evaluate material
    let evaluated = Evaluable::evaluate(&glass.inner, &material_context.inner);

    // Render to CSS
    let backend = CoreCssBackend::new();
    RenderBackend::render(&backend, &evaluated, &render_context.inner)
        .map_err(|e| JsValue::from_str(&format!("Render error: {:?}", e)))
}

// ============================================================================
// Batch Rendering (Gap 3 - P1)
// ============================================================================

/// Batch evaluate and render multiple materials to CSS strings.
///
/// This is significantly more efficient than calling `evaluateAndRenderCss`
/// in a loop, especially for large numbers of materials.
///
/// # Arguments
///
/// * `materials` - Array of GlassMaterial instances
/// * `material_contexts` - Array of EvalMaterialContext instances (same length)
/// * `render_context` - Single RenderContext to use for all materials
///
/// # Returns
///
/// Array of CSS strings, one per material
///
/// # Example (JavaScript)
///
/// ```javascript
/// const materials = [
///     GlassMaterial.clear(),
///     GlassMaterial.frosted(),
///     GlassMaterial.thick()
/// ];
/// const contexts = materials.map(() => EvalMaterialContext.default());
/// const renderCtx = RenderContext.desktop();
///
/// const cssArray = evaluateAndRenderCssBatch(materials, contexts, renderCtx);
/// cssArray.forEach((css, i) => {
///     document.getElementById(`panel-${i}`).style.cssText = css;
/// });
/// ```
#[wasm_bindgen(js_name = evaluateAndRenderCssBatch)]
pub fn evaluate_and_render_css_batch(
    materials: Vec<GlassMaterial>,
    material_contexts: Vec<EvalMaterialContext>,
    render_context: &RenderContext,
) -> Result<Vec<String>, JsValue> {
    // Validate array lengths
    if materials.len() != material_contexts.len() {
        return Err(JsValue::from_str(
            "Materials and contexts arrays must have the same length",
        ));
    }

    if materials.is_empty() {
        return Ok(Vec::new());
    }

    // Evaluate all materials
    let evaluated: Vec<_> = materials
        .iter()
        .zip(material_contexts.iter())
        .map(|(mat, ctx)| Evaluable::evaluate(&mat.inner, &ctx.inner))
        .collect();

    // Render all to CSS
    let backend = CoreCssBackend::new();
    let results: Result<Vec<String>, _> = evaluated
        .iter()
        .map(|eval| {
            RenderBackend::render(&backend, eval, &render_context.inner)
                .map_err(|e| JsValue::from_str(&format!("Render error: {:?}", e)))
        })
        .collect();

    results
}

/// Batch evaluate and render with individual render contexts.
///
/// More flexible version that allows different render contexts per material.
///
/// # Arguments
///
/// * `materials` - Array of GlassMaterial instances
/// * `material_contexts` - Array of EvalMaterialContext instances
/// * `render_contexts` - Array of RenderContext instances (all arrays must match length)
///
/// # Returns
///
/// Array of CSS strings, one per material
#[wasm_bindgen(js_name = evaluateAndRenderCssBatchFull)]
pub fn evaluate_and_render_css_batch_full(
    materials: Vec<GlassMaterial>,
    material_contexts: Vec<EvalMaterialContext>,
    render_contexts: Vec<RenderContext>,
) -> Result<Vec<String>, JsValue> {
    // Validate array lengths
    let len = materials.len();
    if material_contexts.len() != len || render_contexts.len() != len {
        return Err(JsValue::from_str("All arrays must have the same length"));
    }

    if materials.is_empty() {
        return Ok(Vec::new());
    }

    // Evaluate and render each material
    let backend = CoreCssBackend::new();
    let results: Result<Vec<String>, _> = materials
        .iter()
        .zip(material_contexts.iter())
        .zip(render_contexts.iter())
        .map(|((mat, mat_ctx), render_ctx)| {
            let evaluated = Evaluable::evaluate(&mat.inner, &mat_ctx.inner);
            RenderBackend::render(&backend, &evaluated, &render_ctx.inner)
                .map_err(|e| JsValue::from_str(&format!("Render error: {:?}", e)))
        })
        .collect();

    results
}

// ============================================================================
// Enhanced CSS Rendering (Apple Liquid Glass Quality)
// ============================================================================

/// Configuration for enhanced glass CSS rendering.
///
/// Controls all physics-based visual effects:
/// - Specular highlights (Blinn-Phong)
/// - Fresnel edge glow
/// - Inner highlights
/// - Multi-layer elevation shadows
/// - Backdrop saturation
#[wasm_bindgen]
#[derive(Clone)]
pub struct GlassRenderOptions {
    inner: CoreCssRenderConfig,
}

#[wasm_bindgen]
impl GlassRenderOptions {
    /// Create options with default settings.
    #[wasm_bindgen(constructor)]
    pub fn new() -> GlassRenderOptions {
        GlassRenderOptions {
            inner: CoreCssRenderConfig::default(),
        }
    }

    /// Create minimal preset (no visual effects).
    pub fn minimal() -> GlassRenderOptions {
        GlassRenderOptions {
            inner: CoreCssRenderConfig::minimal(),
        }
    }

    /// Create premium preset (Apple Liquid Glass quality).
    pub fn premium() -> GlassRenderOptions {
        GlassRenderOptions {
            inner: CoreCssRenderConfig::premium(),
        }
    }

    /// Create modal preset (floating dialogs).
    pub fn modal() -> GlassRenderOptions {
        GlassRenderOptions {
            inner: CoreCssRenderConfig::modal(),
        }
    }

    /// Create subtle preset (content-focused cards).
    pub fn subtle() -> GlassRenderOptions {
        GlassRenderOptions {
            inner: CoreCssRenderConfig::subtle(),
        }
    }

    /// Create dark mode preset.
    #[wasm_bindgen(js_name = darkMode)]
    pub fn dark_mode() -> GlassRenderOptions {
        GlassRenderOptions {
            inner: CoreCssRenderConfig::dark_mode(),
        }
    }

    // Setters for customization

    /// Enable or disable specular highlights.
    #[wasm_bindgen(setter, js_name = specularEnabled)]
    pub fn set_specular_enabled(&mut self, value: bool) {
        self.inner.specular_enabled = value;
    }

    /// Set specular highlight intensity (0.0-1.0).
    #[wasm_bindgen(setter, js_name = specularIntensity)]
    pub fn set_specular_intensity(&mut self, value: f64) {
        self.inner.specular_intensity = value.clamp(0.0, 1.0);
    }

    /// Enable or disable Fresnel edge glow.
    #[wasm_bindgen(setter, js_name = fresnelEnabled)]
    pub fn set_fresnel_enabled(&mut self, value: bool) {
        self.inner.fresnel_enabled = value;
    }

    /// Set Fresnel edge intensity (0.0-1.0).
    #[wasm_bindgen(setter, js_name = fresnelIntensity)]
    pub fn set_fresnel_intensity(&mut self, value: f64) {
        self.inner.fresnel_intensity = value.clamp(0.0, 1.0);
    }

    /// Set elevation level (0-6).
    #[wasm_bindgen(setter)]
    pub fn set_elevation(&mut self, value: u8) {
        self.inner.elevation = value.min(6);
    }

    /// Enable or disable backdrop saturation boost.
    #[wasm_bindgen(setter)]
    pub fn set_saturate(&mut self, value: bool) {
        self.inner.saturate = value;
    }

    /// Set border radius in pixels.
    #[wasm_bindgen(setter, js_name = borderRadius)]
    pub fn set_border_radius(&mut self, value: f64) {
        self.inner.border_radius = value.max(0.0);
    }

    /// Set light mode (true) or dark mode (false).
    #[wasm_bindgen(setter, js_name = lightMode)]
    pub fn set_light_mode(&mut self, value: bool) {
        self.inner.light_mode = value;
    }

    /// Enable or disable inner highlight.
    #[wasm_bindgen(setter, js_name = innerHighlightEnabled)]
    pub fn set_inner_highlight_enabled(&mut self, value: bool) {
        self.inner.inner_highlight_enabled = value;
    }

    /// Enable or disable border.
    #[wasm_bindgen(setter, js_name = borderEnabled)]
    pub fn set_border_enabled(&mut self, value: bool) {
        self.inner.border_enabled = value;
    }
}

/// Render enhanced glass CSS with physics-based effects.
///
/// This generates complete CSS with:
/// - Multi-layer backgrounds with gradients
/// - Specular highlights (Blinn-Phong)
/// - Fresnel edge glow
/// - 4-layer elevation shadows
/// - Backdrop blur with saturation
///
/// # Example (JavaScript)
///
/// ```javascript
/// const glass = GlassMaterial.regular();
/// const ctx = EvalMaterialContext.new();
/// const rctx = RenderContext.desktop();
/// const options = GlassRenderOptions.premium();
///
/// const css = renderEnhancedGlassCss(glass, ctx, rctx, options);
/// document.getElementById('panel').style.cssText = css;
/// ```
#[wasm_bindgen(js_name = renderEnhancedGlassCss)]
pub fn render_enhanced_glass_css(
    glass: &GlassMaterial,
    material_context: &EvalMaterialContext,
    _render_context: &RenderContext,
    options: &GlassRenderOptions,
) -> Result<String, JsValue> {
    // Evaluate material
    let evaluated = Evaluable::evaluate(&glass.inner, &material_context.inner);

    // Render with enhanced backend
    Ok(CoreEnhancedCssBackend::render(&evaluated, &options.inner))
}

// ============================================================================
// Direct Physics Functions (for fine control)
// ============================================================================

/// Generate Fresnel edge gradient CSS.
///
/// Creates a radial gradient that simulates angle-dependent reflection
/// (Schlick's approximation). Edges appear brighter than center.
///
/// # Arguments
///
/// * `intensity` - Edge glow intensity (0.0-1.0)
/// * `light_mode` - Whether to use light mode colors
///
/// # Returns
///
/// CSS radial-gradient string
///
/// # Example (JavaScript)
///
/// ```javascript
/// const gradient = generateFresnelGradientCss(0.3, true);
/// element.style.background = gradient;
/// ```
#[wasm_bindgen(js_name = generateFresnelGradientCss)]
pub fn generate_fresnel_gradient_css(intensity: f64, light_mode: bool) -> String {
    to_css_fresnel_gradient(intensity, light_mode)
}

/// Generate specular highlight CSS.
///
/// Creates a positioned radial gradient for light reflection
/// based on Blinn-Phong model.
///
/// # Arguments
///
/// * `intensity` - Highlight intensity (0.0-1.0)
/// * `size` - Highlight size as percentage (20-60)
/// * `pos_x` - Horizontal position percentage (0-100)
/// * `pos_y` - Vertical position percentage (0-100)
///
/// # Returns
///
/// CSS radial-gradient string
#[wasm_bindgen(js_name = generateSpecularHighlightCss)]
pub fn generate_specular_highlight_css(
    intensity: f64,
    size: f64,
    pos_x: f64,
    pos_y: f64,
) -> String {
    to_css_specular_highlight(intensity, size, pos_x, pos_y)
}

/// Generate secondary specular (fill light) CSS.
///
/// Creates a weaker highlight at bottom-right to simulate
/// ambient/fill lighting.
///
/// # Arguments
///
/// * `intensity` - Highlight intensity (0.0-1.0)
/// * `size` - Highlight size as percentage (15-40)
///
/// # Returns
///
/// CSS radial-gradient string
#[wasm_bindgen(js_name = generateSecondarySpecularCss)]
pub fn generate_secondary_specular_css(intensity: f64, size: f64) -> String {
    to_css_secondary_specular(intensity, size)
}

/// Generate inner top highlight CSS.
///
/// Creates a linear gradient from top that simulates
/// light hitting the top edge.
///
/// # Arguments
///
/// * `intensity` - Highlight intensity (0.0-1.0)
/// * `light_mode` - Whether to use light mode colors
///
/// # Returns
///
/// CSS linear-gradient string
#[wasm_bindgen(js_name = generateInnerHighlightCss)]
pub fn generate_inner_highlight_css(intensity: f64, light_mode: bool) -> String {
    to_css_inner_highlight(intensity, light_mode)
}

// ============================================================================
// Sprint 1: Thin-Film Interference Physics
// ============================================================================

/// Thin-film interference coating parameters.
///
/// Models iridescent effects from thin transparent films like soap bubbles,
/// oil slicks, and anti-reflective coatings.
///
/// ## Physical Background
///
/// When light reflects from both surfaces of a thin transparent layer,
/// the path difference creates constructive or destructive interference
/// depending on wavelength and viewing angle.
///
/// ## Example (JavaScript)
///
/// ```javascript
/// // Create a soap bubble thin film
/// const film = ThinFilm.soapBubbleThin();
/// console.log(`Thickness: ${film.thicknessNm}nm`);
///
/// // Calculate reflectance at 550nm (green)
/// const r = film.reflectance(550.0, 1.0, 1.0);  // normal incidence
/// console.log(`Reflectance at 550nm: ${r}`);
///
/// // Get RGB reflectance (for rendering)
/// const rgb = film.reflectanceRgb(1.0, 0.8);  // n_substrate=1.0, cos_theta=0.8
/// console.log(`RGB: [${rgb[0]}, ${rgb[1]}, ${rgb[2]}]`);
///
/// // Generate CSS for visual effect
/// const css = film.toCssSoapBubble(100.0);
/// element.style.background = css;
/// ```
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct ThinFilm {
    inner: CoreThinFilm,
}

#[wasm_bindgen]
impl ThinFilm {
    /// Create a new thin film with custom parameters.
    ///
    /// # Arguments
    ///
    /// * `n_film` - Film refractive index (typically 1.3-1.7)
    /// * `thickness_nm` - Film thickness in nanometers (typically 50-500nm)
    ///
    /// # Example
    ///
    /// ```javascript
    /// // Custom thin film: n=1.45, thickness=180nm
    /// const film = new ThinFilm(1.45, 180.0);
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new(n_film: f64, thickness_nm: f64) -> ThinFilm {
        ThinFilm {
            inner: CoreThinFilm::new(n_film, thickness_nm),
        }
    }

    // ========================================================================
    // Presets - Soap Bubble
    // ========================================================================

    /// Thin soap bubble (~100nm water film).
    ///
    /// Creates subtle blue-violet interference colors.
    #[wasm_bindgen(js_name = soapBubbleThin)]
    pub fn soap_bubble_thin() -> ThinFilm {
        ThinFilm {
            inner: thin_film_presets::SOAP_BUBBLE_THIN,
        }
    }

    /// Medium soap bubble (~200nm water film).
    ///
    /// Creates balanced rainbow interference colors.
    #[wasm_bindgen(js_name = soapBubbleMedium)]
    pub fn soap_bubble_medium() -> ThinFilm {
        ThinFilm {
            inner: thin_film_presets::SOAP_BUBBLE_MEDIUM,
        }
    }

    /// Thick soap bubble (~400nm water film).
    ///
    /// Creates stronger yellow-red interference colors.
    #[wasm_bindgen(js_name = soapBubbleThick)]
    pub fn soap_bubble_thick() -> ThinFilm {
        ThinFilm {
            inner: thin_film_presets::SOAP_BUBBLE_THICK,
        }
    }

    // ========================================================================
    // Presets - Oil Slick
    // ========================================================================

    /// Thin oil slick on water (~150nm).
    ///
    /// Oil (n≈1.5) on water (n≈1.33) creates classic rainbow effect.
    #[wasm_bindgen(js_name = oilThin)]
    pub fn oil_thin() -> ThinFilm {
        ThinFilm {
            inner: thin_film_presets::OIL_THIN,
        }
    }

    /// Medium oil slick (~300nm).
    #[wasm_bindgen(js_name = oilMedium)]
    pub fn oil_medium() -> ThinFilm {
        ThinFilm {
            inner: thin_film_presets::OIL_MEDIUM,
        }
    }

    /// Thick oil slick (~500nm).
    #[wasm_bindgen(js_name = oilThick)]
    pub fn oil_thick() -> ThinFilm {
        ThinFilm {
            inner: thin_film_presets::OIL_THICK,
        }
    }

    // ========================================================================
    // Presets - Special Materials
    // ========================================================================

    /// Anti-reflective coating (MgF2 on glass).
    ///
    /// Quarter-wave thickness at 550nm for minimal reflection.
    #[wasm_bindgen(js_name = arCoating)]
    pub fn ar_coating() -> ThinFilm {
        ThinFilm {
            inner: thin_film_presets::AR_COATING,
        }
    }

    /// Thin oxide layer (SiO2 on silicon, ~50nm).
    ///
    /// Creates characteristic chip colors.
    #[wasm_bindgen(js_name = oxideThin)]
    pub fn oxide_thin() -> ThinFilm {
        ThinFilm {
            inner: thin_film_presets::OXIDE_THIN,
        }
    }

    /// Medium oxide layer (~150nm).
    #[wasm_bindgen(js_name = oxideMedium)]
    pub fn oxide_medium() -> ThinFilm {
        ThinFilm {
            inner: thin_film_presets::OXIDE_MEDIUM,
        }
    }

    /// Thick oxide layer (~300nm).
    #[wasm_bindgen(js_name = oxideThick)]
    pub fn oxide_thick() -> ThinFilm {
        ThinFilm {
            inner: thin_film_presets::OXIDE_THICK,
        }
    }

    /// Beetle shell coating (chitin-like material).
    ///
    /// Creates natural iridescence seen in jewel beetles.
    #[wasm_bindgen(js_name = beetleShell)]
    pub fn beetle_shell() -> ThinFilm {
        ThinFilm {
            inner: thin_film_presets::BEETLE_SHELL,
        }
    }

    /// Pearl nacre (aragonite layers).
    ///
    /// Creates lustrous pearl iridescence.
    #[wasm_bindgen(js_name = nacre)]
    pub fn nacre() -> ThinFilm {
        ThinFilm {
            inner: thin_film_presets::NACRE,
        }
    }

    // ========================================================================
    // Getters
    // ========================================================================

    /// Film refractive index.
    #[wasm_bindgen(getter, js_name = nFilm)]
    pub fn n_film(&self) -> f64 {
        self.inner.n_film
    }

    /// Film thickness in nanometers.
    #[wasm_bindgen(getter, js_name = thicknessNm)]
    pub fn thickness_nm(&self) -> f64 {
        self.inner.thickness_nm
    }

    // ========================================================================
    // Physics Calculations
    // ========================================================================

    /// Calculate optical path difference for given viewing angle.
    ///
    /// OPD = 2 * n_film * d * cos(theta_film)
    ///
    /// # Arguments
    ///
    /// * `cos_theta_air` - Cosine of incidence angle in air (1.0 = normal)
    ///
    /// # Returns
    ///
    /// Optical path difference in nanometers
    #[wasm_bindgen(js_name = opticalPathDifference)]
    pub fn optical_path_difference(&self, cos_theta_air: f64) -> f64 {
        self.inner.optical_path_difference(cos_theta_air)
    }

    /// Calculate phase difference for a given wavelength.
    ///
    /// delta = 2 * PI * OPD / lambda
    ///
    /// # Arguments
    ///
    /// * `wavelength_nm` - Wavelength in nanometers (visible: 400-700nm)
    /// * `cos_theta` - Cosine of incidence angle (1.0 = normal)
    ///
    /// # Returns
    ///
    /// Phase difference in radians
    #[wasm_bindgen(js_name = phaseDifference)]
    pub fn phase_difference(&self, wavelength_nm: f64, cos_theta: f64) -> f64 {
        self.inner.phase_difference(wavelength_nm, cos_theta)
    }

    /// Calculate reflectance at a single wavelength using the Airy formula.
    ///
    /// This is the core physics calculation that accounts for:
    /// - Fresnel reflection at both interfaces
    /// - Phase difference from optical path
    /// - Interference between reflected rays
    ///
    /// # Arguments
    ///
    /// * `wavelength_nm` - Wavelength in nanometers (visible: 400-700nm)
    /// * `n_substrate` - Substrate refractive index (air=1.0, water=1.33, glass=1.52)
    /// * `cos_theta` - Cosine of incidence angle (1.0 = normal, 0.0 = grazing)
    ///
    /// # Returns
    ///
    /// Reflectance (0.0-1.0)
    ///
    /// # Example
    ///
    /// ```javascript
    /// const film = ThinFilm.soapBubbleMedium();
    ///
    /// // Green light at normal incidence, air substrate
    /// const rGreen = film.reflectance(550.0, 1.0, 1.0);
    ///
    /// // Same but at 60° angle
    /// const rAngled = film.reflectance(550.0, 1.0, 0.5);  // cos(60°) = 0.5
    /// ```
    pub fn reflectance(&self, wavelength_nm: f64, n_substrate: f64, cos_theta: f64) -> f64 {
        self.inner
            .reflectance(wavelength_nm, n_substrate, cos_theta)
    }

    /// Calculate RGB reflectance (R=650nm, G=550nm, B=450nm).
    ///
    /// Returns reflectance values for rendering colored interference.
    ///
    /// # Arguments
    ///
    /// * `n_substrate` - Substrate refractive index
    /// * `cos_theta` - Cosine of incidence angle
    ///
    /// # Returns
    ///
    /// Array of 3 reflectance values [R, G, B] in range 0.0-1.0
    ///
    /// # Example
    ///
    /// ```javascript
    /// const film = ThinFilm.oilMedium();
    /// const rgb = film.reflectanceRgb(1.33, 0.8);  // oil on water
    /// console.log(`R=${rgb[0]}, G=${rgb[1]}, B=${rgb[2]}`);
    /// ```
    #[wasm_bindgen(js_name = reflectanceRgb)]
    pub fn reflectance_rgb(&self, n_substrate: f64, cos_theta: f64) -> Vec<f64> {
        self.inner.reflectance_rgb(n_substrate, cos_theta).to_vec()
    }

    /// Calculate full spectrum reflectance (8 wavelengths: 400-750nm).
    ///
    /// Returns wavelengths and corresponding reflectances for spectral rendering.
    ///
    /// # Arguments
    ///
    /// * `n_substrate` - Substrate refractive index
    /// * `cos_theta` - Cosine of incidence angle
    ///
    /// # Returns
    ///
    /// Object with `wavelengths` (8 values) and `reflectances` (8 values)
    #[wasm_bindgen(js_name = reflectanceSpectrum)]
    pub fn reflectance_spectrum(&self, n_substrate: f64, cos_theta: f64) -> js_sys::Object {
        let (wavelengths, reflectances) = self.inner.reflectance_spectrum(n_substrate, cos_theta);

        let obj = js_sys::Object::new();

        let w_array = js_sys::Array::from_iter(wavelengths.iter().map(|v| JsValue::from_f64(*v)));
        let r_array = js_sys::Array::from_iter(reflectances.iter().map(|v| JsValue::from_f64(*v)));

        js_sys::Reflect::set(&obj, &JsValue::from_str("wavelengths"), &w_array).unwrap();
        js_sys::Reflect::set(&obj, &JsValue::from_str("reflectances"), &r_array).unwrap();

        obj
    }

    /// Find wavelength of maximum constructive interference.
    ///
    /// For first-order maximum: OPD = lambda
    ///
    /// # Arguments
    ///
    /// * `cos_theta` - Cosine of incidence angle
    ///
    /// # Returns
    ///
    /// Wavelength in nanometers where reflectance is maximized
    #[wasm_bindgen(js_name = maxWavelength)]
    pub fn max_wavelength(&self, cos_theta: f64) -> f64 {
        self.inner.max_wavelength(cos_theta)
    }

    /// Find wavelength of maximum destructive interference.
    ///
    /// For first-order minimum: OPD = lambda/2
    ///
    /// # Arguments
    ///
    /// * `cos_theta` - Cosine of incidence angle
    ///
    /// # Returns
    ///
    /// Wavelength in nanometers where reflectance is minimized
    #[wasm_bindgen(js_name = minWavelength)]
    pub fn min_wavelength(&self, cos_theta: f64) -> f64 {
        self.inner.min_wavelength(cos_theta)
    }

    // ========================================================================
    // CSS Generation
    // ========================================================================

    /// Generate CSS for soap bubble effect.
    ///
    /// Creates a radial gradient that simulates angle-dependent
    /// interference colors with a highlight at the center.
    ///
    /// # Arguments
    ///
    /// * `size_percent` - Size scaling percentage (100 = full size)
    ///
    /// # Returns
    ///
    /// CSS radial-gradient string
    ///
    /// # Example
    ///
    /// ```javascript
    /// const film = ThinFilm.soapBubbleMedium();
    /// const css = film.toCssSoapBubble(100.0);
    /// element.style.background = css;
    /// ```
    #[wasm_bindgen(js_name = toCssSoapBubble)]
    pub fn to_css_soap_bubble(&self, size_percent: f64) -> String {
        core_to_css_soap_bubble(&self.inner, size_percent)
    }

    /// Generate CSS for oil slick effect.
    ///
    /// Creates a linear gradient that simulates rainbow-like
    /// interference patterns seen on oil films.
    ///
    /// # Returns
    ///
    /// CSS linear-gradient string
    ///
    /// # Example
    ///
    /// ```javascript
    /// const film = ThinFilm.oilMedium();
    /// const css = film.toCssOilSlick();
    /// element.style.background = css;
    /// ```
    #[wasm_bindgen(js_name = toCssOilSlick)]
    pub fn to_css_oil_slick(&self) -> String {
        core_to_css_oil_slick(&self.inner)
    }

    /// Generate CSS for general iridescent gradient.
    ///
    /// Creates a gradient with angle-dependent color shift over a base color.
    ///
    /// # Arguments
    ///
    /// * `n_substrate` - Substrate refractive index
    /// * `base_color` - Base CSS color string (e.g., "#000000")
    ///
    /// # Returns
    ///
    /// CSS gradient string
    #[wasm_bindgen(js_name = toCssIridescentGradient)]
    pub fn to_css_iridescent_gradient(&self, n_substrate: f64, base_color: &str) -> String {
        core_to_css_iridescent_gradient(&self.inner, n_substrate, base_color)
    }

    /// Convert thin-film reflectance to RGB color for given conditions.
    ///
    /// # Arguments
    ///
    /// * `n_substrate` - Substrate refractive index
    /// * `cos_theta` - Cosine of incidence angle
    ///
    /// # Returns
    ///
    /// Array [r, g, b] with values 0-255
    #[wasm_bindgen(js_name = toRgb)]
    pub fn to_rgb(&self, n_substrate: f64, cos_theta: f64) -> Vec<u8> {
        let (r, g, b) = core_thin_film_to_rgb(&self.inner, n_substrate, cos_theta);
        vec![r, g, b]
    }
}

// ============================================================================
// Thin-Film Utility Functions
// ============================================================================

/// Calculate optimal AR coating thickness for a given wavelength.
///
/// For quarter-wave AR coating: d = lambda / (4 * n_film)
///
/// # Arguments
///
/// * `wavelength_nm` - Design wavelength in nanometers (typically 550nm for visible)
/// * `n_film` - Film refractive index
///
/// # Returns
///
/// Optimal thickness in nanometers
///
/// # Example
///
/// ```javascript
/// // AR coating for green light on MgF2
/// const thickness = calculateArCoatingThickness(550.0, 1.38);
/// console.log(`Optimal thickness: ${thickness}nm`);  // ~99.6nm
/// ```
#[wasm_bindgen(js_name = calculateArCoatingThickness)]
pub fn calculate_ar_coating_thickness(wavelength_nm: f64, n_film: f64) -> f64 {
    core_ar_coating_thickness(wavelength_nm, n_film)
}

/// Find the dominant (brightest) wavelength for a thin film.
///
/// Returns the wavelength with maximum reflectance in the visible range.
///
/// # Arguments
///
/// * `film` - ThinFilm parameters
/// * `n_substrate` - Substrate refractive index
/// * `cos_theta` - Cosine of incidence angle
///
/// # Returns
///
/// Dominant wavelength in nanometers (400-700nm)
///
/// # Example
///
/// ```javascript
/// const film = ThinFilm.soapBubbleMedium();
/// const lambda = findDominantWavelength(film, 1.0, 1.0);
/// console.log(`Dominant color wavelength: ${lambda}nm`);
/// ```
#[wasm_bindgen(js_name = findDominantWavelength)]
pub fn find_dominant_wavelength(film: &ThinFilm, n_substrate: f64, cos_theta: f64) -> f64 {
    core_dominant_wavelength(&film.inner, n_substrate, cos_theta)
}

/// Get all thin-film presets with their names and recommended substrates.
///
/// # Returns
///
/// Array of objects with { name, nFilm, thicknessNm, suggestedSubstrate }
///
/// # Example
///
/// ```javascript
/// const presets = getThinFilmPresets();
/// for (const preset of presets) {
///     console.log(`${preset.name}: n=${preset.nFilm}, d=${preset.thicknessNm}nm`);
/// }
/// ```
#[wasm_bindgen(js_name = getThinFilmPresets)]
pub fn get_thin_film_presets() -> js_sys::Array {
    let presets = thin_film_presets::all_presets();
    let array = js_sys::Array::new();

    for (name, film, substrate) in presets {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &JsValue::from_str("name"), &JsValue::from_str(name)).unwrap();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("nFilm"),
            &JsValue::from_f64(film.n_film),
        )
        .unwrap();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("thicknessNm"),
            &JsValue::from_f64(film.thickness_nm),
        )
        .unwrap();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("suggestedSubstrate"),
            &JsValue::from_f64(substrate),
        )
        .unwrap();
        array.push(&obj);
    }

    array
}

// ============================================================================
// Sprint 2: Chromatic Dispersion Models
// ============================================================================

/// Cauchy dispersion model for wavelength-dependent refractive index.
///
/// Simple polynomial approximation for most transparent materials:
/// n(λ) = A + B/λ² + C/λ⁴
///
/// ## Example (JavaScript)
///
/// ```javascript
/// // Create crown glass dispersion
/// const crown = CauchyDispersion.crownGlass();
///
/// // Get IOR at specific wavelength
/// const n550 = crown.n(550.0);  // ~1.518 at green
///
/// // Get RGB IOR values
/// const nRgb = crown.nRgb();  // [n_red, n_green, n_blue]
///
/// // Calculate Abbe number (dispersion strength)
/// const abbe = crown.abbeNumber();  // ~64 for crown glass
/// ```
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct CauchyDispersion {
    inner: CoreCauchyDispersion,
}

#[wasm_bindgen]
impl CauchyDispersion {
    /// Create custom Cauchy dispersion model.
    ///
    /// # Arguments
    ///
    /// * `a` - Base refractive index (A coefficient, typically ~1.5)
    /// * `b` - First dispersion coefficient (B coefficient, nm²)
    /// * `c` - Second dispersion coefficient (C coefficient, nm⁴)
    #[wasm_bindgen(constructor)]
    pub fn new(a: f64, b: f64, c: f64) -> CauchyDispersion {
        CauchyDispersion {
            inner: CoreCauchyDispersion::new(a, b, c),
        }
    }

    /// Create from base IOR with default dispersion.
    ///
    /// Uses empirical relationship between IOR and dispersion.
    #[wasm_bindgen(js_name = fromIor)]
    pub fn from_ior(ior: f64) -> CauchyDispersion {
        CauchyDispersion {
            inner: CoreCauchyDispersion::from_ior(ior),
        }
    }

    /// Create non-dispersive model (constant IOR).
    #[wasm_bindgen(js_name = constant)]
    pub fn constant(ior: f64) -> CauchyDispersion {
        CauchyDispersion {
            inner: CoreCauchyDispersion::constant(ior),
        }
    }

    // ========================================================================
    // Presets
    // ========================================================================

    /// Crown glass (BK7) - Low dispersion optical glass.
    /// Abbe number ~64
    #[wasm_bindgen(js_name = crownGlass)]
    pub fn crown_glass() -> CauchyDispersion {
        CauchyDispersion {
            inner: CoreCauchyDispersion::crown_glass(),
        }
    }

    /// Flint glass (SF11) - High dispersion dense glass.
    /// Abbe number ~25
    #[wasm_bindgen(js_name = flintGlass)]
    pub fn flint_glass() -> CauchyDispersion {
        CauchyDispersion {
            inner: CoreCauchyDispersion::flint_glass(),
        }
    }

    /// Fused silica - Very low dispersion, pure SiO2.
    /// Abbe number ~68
    #[wasm_bindgen(js_name = fusedSilica)]
    pub fn fused_silica() -> CauchyDispersion {
        CauchyDispersion {
            inner: CoreCauchyDispersion::fused_silica(),
        }
    }

    /// Water at 20°C.
    /// Abbe number ~56
    pub fn water() -> CauchyDispersion {
        CauchyDispersion {
            inner: CoreCauchyDispersion::water(),
        }
    }

    /// Diamond - Very high dispersion ("fire").
    /// Abbe number ~44
    pub fn diamond() -> CauchyDispersion {
        CauchyDispersion {
            inner: CoreCauchyDispersion::diamond(),
        }
    }

    /// Polycarbonate (PC) - High dispersion plastic.
    /// Abbe number ~30
    pub fn polycarbonate() -> CauchyDispersion {
        CauchyDispersion {
            inner: CoreCauchyDispersion::polycarbonate(),
        }
    }

    /// PMMA (Acrylic) - Low dispersion plastic.
    /// Abbe number ~57
    pub fn pmma() -> CauchyDispersion {
        CauchyDispersion {
            inner: CoreCauchyDispersion::pmma(),
        }
    }

    // ========================================================================
    // Getters
    // ========================================================================

    /// Base coefficient A (approximate IOR at d-line).
    #[wasm_bindgen(getter)]
    pub fn a(&self) -> f64 {
        self.inner.a
    }

    /// First dispersion coefficient B (nm²).
    #[wasm_bindgen(getter)]
    pub fn b(&self) -> f64 {
        self.inner.b
    }

    /// Second dispersion coefficient C (nm⁴).
    #[wasm_bindgen(getter)]
    pub fn c(&self) -> f64 {
        self.inner.c
    }

    // ========================================================================
    // Core Physics
    // ========================================================================

    /// Calculate refractive index at given wavelength.
    ///
    /// # Arguments
    ///
    /// * `wavelength_nm` - Wavelength in nanometers (visible: 380-780nm)
    ///
    /// # Returns
    ///
    /// Refractive index n (typically 1.0 to 2.5)
    pub fn n(&self, wavelength_nm: f64) -> f64 {
        self.inner.n(wavelength_nm)
    }

    /// Calculate refractive indices for RGB channels.
    ///
    /// Uses standard wavelengths: R=656.3nm, G=587.6nm, B=486.1nm
    ///
    /// # Returns
    ///
    /// Array [n_red, n_green, n_blue]
    #[wasm_bindgen(js_name = nRgb)]
    pub fn n_rgb(&self) -> Vec<f64> {
        self.inner.n_rgb().to_vec()
    }

    /// Calculate Abbe number (dispersion strength).
    ///
    /// V_d = (n_d - 1) / (n_F - n_C)
    ///
    /// Higher values = less dispersion (crown glass ~60)
    /// Lower values = more dispersion (flint glass ~30)
    #[wasm_bindgen(js_name = abbeNumber)]
    pub fn abbe_number(&self) -> f64 {
        self.inner.abbe_number()
    }

    /// Get base refractive index (at sodium d-line, 589.3nm).
    #[wasm_bindgen(js_name = nBase)]
    pub fn n_base(&self) -> f64 {
        self.inner.n_base()
    }
}

/// Sellmeier dispersion model for high-accuracy wavelength-dependent IOR.
///
/// Resonance-based model with higher accuracy than Cauchy:
/// n²(λ) = 1 + Σᵢ (Bᵢ * λ²) / (λ² - Cᵢ)
///
/// ## Example (JavaScript)
///
/// ```javascript
/// // Create BK7 optical glass
/// const bk7 = SellmeierDispersion.bk7();
///
/// // Get IOR at HeNe laser wavelength
/// const n633 = bk7.n(632.8);  // ~1.457
///
/// // Get full dispersion curve
/// const wavelengths = [400, 450, 500, 550, 600, 650, 700];
/// const iors = wavelengths.map(λ => bk7.n(λ));
/// ```
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct SellmeierDispersion {
    inner: CoreSellmeierDispersion,
}

#[wasm_bindgen]
impl SellmeierDispersion {
    /// Create custom Sellmeier dispersion model.
    ///
    /// # Arguments
    ///
    /// * `b` - Array of 3 oscillator strengths [B1, B2, B3]
    /// * `c` - Array of 3 resonance wavelengths squared in μm² [C1, C2, C3]
    #[wasm_bindgen(constructor)]
    pub fn new(b: Vec<f64>, c: Vec<f64>) -> SellmeierDispersion {
        let b_arr = [
            b.get(0).copied().unwrap_or(0.0),
            b.get(1).copied().unwrap_or(0.0),
            b.get(2).copied().unwrap_or(0.0),
        ];
        let c_arr = [
            c.get(0).copied().unwrap_or(0.0),
            c.get(1).copied().unwrap_or(0.0),
            c.get(2).copied().unwrap_or(0.0),
        ];
        SellmeierDispersion {
            inner: CoreSellmeierDispersion::new(b_arr, c_arr),
        }
    }

    // ========================================================================
    // Presets
    // ========================================================================

    /// Fused silica (SiO2) - Malitson 1965.
    #[wasm_bindgen(js_name = fusedSilica)]
    pub fn fused_silica() -> SellmeierDispersion {
        SellmeierDispersion {
            inner: CoreSellmeierDispersion::fused_silica(),
        }
    }

    /// BK7 optical glass (Schott) - Common crown glass.
    pub fn bk7() -> SellmeierDispersion {
        SellmeierDispersion {
            inner: CoreSellmeierDispersion::bk7(),
        }
    }

    /// SF11 flint glass (Schott) - High dispersion.
    pub fn sf11() -> SellmeierDispersion {
        SellmeierDispersion {
            inner: CoreSellmeierDispersion::sf11(),
        }
    }

    /// Sapphire (Al2O3) - Ordinary ray.
    pub fn sapphire() -> SellmeierDispersion {
        SellmeierDispersion {
            inner: CoreSellmeierDispersion::sapphire(),
        }
    }

    /// Diamond (C).
    pub fn diamond() -> SellmeierDispersion {
        SellmeierDispersion {
            inner: CoreSellmeierDispersion::diamond(),
        }
    }

    // ========================================================================
    // Core Physics
    // ========================================================================

    /// Calculate refractive index at given wavelength.
    ///
    /// More accurate than Cauchy, especially in UV and IR ranges.
    pub fn n(&self, wavelength_nm: f64) -> f64 {
        self.inner.n(wavelength_nm)
    }

    /// Calculate refractive indices for RGB channels.
    #[wasm_bindgen(js_name = nRgb)]
    pub fn n_rgb(&self) -> Vec<f64> {
        self.inner.n_rgb().to_vec()
    }

    /// Calculate Abbe number.
    #[wasm_bindgen(js_name = abbeNumber)]
    pub fn abbe_number(&self) -> f64 {
        self.inner.abbe_number()
    }

    /// Get base refractive index (at sodium d-line).
    #[wasm_bindgen(js_name = nBase)]
    pub fn n_base(&self) -> f64 {
        self.inner.n_base()
    }
}

// ============================================================================
// Sprint 2: Complex IOR for Metals
// ============================================================================

/// Complex refractive index for metals and absorbing materials.
///
/// n_complex = n + i*k
///
/// Where:
/// - n = real part (refraction, phase velocity)
/// - k = imaginary part (extinction coefficient, absorption)
///
/// ## Example (JavaScript)
///
/// ```javascript
/// // Create gold-like IOR at 550nm
/// const goldIor = new ComplexIOR(0.42, 2.35);
///
/// // Calculate F0 (normal incidence reflectance)
/// const f0 = goldIor.f0();  // ~0.85 for gold
///
/// // Check if material is a conductor
/// const isMetal = goldIor.isConductor();  // true
/// ```
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct ComplexIOR {
    inner: CoreComplexIOR,
}

#[wasm_bindgen]
impl ComplexIOR {
    /// Create new complex IOR.
    ///
    /// # Arguments
    ///
    /// * `n` - Real part (refractive index)
    /// * `k` - Imaginary part (extinction coefficient)
    #[wasm_bindgen(constructor)]
    pub fn new(n: f64, k: f64) -> ComplexIOR {
        ComplexIOR {
            inner: CoreComplexIOR::new(n, k),
        }
    }

    /// Create dielectric (k = 0).
    pub fn dielectric(n: f64) -> ComplexIOR {
        ComplexIOR {
            inner: CoreComplexIOR::dielectric(n),
        }
    }

    /// Real part: refractive index.
    #[wasm_bindgen(getter)]
    pub fn n(&self) -> f64 {
        self.inner.n
    }

    /// Imaginary part: extinction coefficient.
    #[wasm_bindgen(getter)]
    pub fn k(&self) -> f64 {
        self.inner.k
    }

    /// Calculate F0 (normal incidence reflectance).
    ///
    /// F0 = ((n-1)² + k²) / ((n+1)² + k²)
    pub fn f0(&self) -> f64 {
        self.inner.f0()
    }

    /// Check if this is a conductor (has significant extinction).
    #[wasm_bindgen(js_name = isConductor)]
    pub fn is_conductor(&self) -> bool {
        self.inner.is_conductor()
    }

    /// Calculate penetration depth (skin depth) in nanometers.
    #[wasm_bindgen(js_name = penetrationDepthNm)]
    pub fn penetration_depth_nm(&self, wavelength_nm: f64) -> f64 {
        self.inner.penetration_depth_nm(wavelength_nm)
    }
}

/// Spectral complex IOR (RGB wavelength-dependent metal response).
///
/// Stores n+ik values at red, green, and blue wavelengths for
/// physically accurate metal coloration.
///
/// ## Example (JavaScript)
///
/// ```javascript
/// // Get gold preset
/// const gold = SpectralComplexIOR.gold();
///
/// // Get F0 for each channel - THIS IS THE METAL'S COLOR
/// const f0Rgb = gold.f0Rgb();  // [0.94, 0.78, 0.33] - yellow!
///
/// // Calculate full Fresnel reflectance at 45°
/// const cosTheta = 0.707;
/// const fresnelRgb = gold.fresnelRgb(1.0, cosTheta);
/// ```
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct SpectralComplexIOR {
    inner: CoreSpectralComplexIOR,
}

#[wasm_bindgen]
impl SpectralComplexIOR {
    /// Create spectral complex IOR from RGB values.
    ///
    /// # Arguments
    ///
    /// * `n_rgb` - Array of n values [n_red, n_green, n_blue]
    /// * `k_rgb` - Array of k values [k_red, k_green, k_blue]
    #[wasm_bindgen(constructor)]
    pub fn new(n_rgb: Vec<f64>, k_rgb: Vec<f64>) -> SpectralComplexIOR {
        let n_arr = [
            n_rgb.get(0).copied().unwrap_or(1.0),
            n_rgb.get(1).copied().unwrap_or(1.0),
            n_rgb.get(2).copied().unwrap_or(1.0),
        ];
        let k_arr = [
            k_rgb.get(0).copied().unwrap_or(0.0),
            k_rgb.get(1).copied().unwrap_or(0.0),
            k_rgb.get(2).copied().unwrap_or(0.0),
        ];
        SpectralComplexIOR {
            inner: CoreSpectralComplexIOR::from_arrays(n_arr, k_arr),
        }
    }

    // ========================================================================
    // Metal Presets - Measured Optical Constants
    // ========================================================================

    /// Gold (Au) - Warm yellow metal.
    /// Source: Johnson & Christy (1972)
    pub fn gold() -> SpectralComplexIOR {
        SpectralComplexIOR {
            inner: metal_presets::GOLD,
        }
    }

    /// Silver (Ag) - Neutral white metal, highest reflectivity.
    /// Source: Johnson & Christy (1972)
    pub fn silver() -> SpectralComplexIOR {
        SpectralComplexIOR {
            inner: metal_presets::SILVER,
        }
    }

    /// Copper (Cu) - Orange-red metal.
    /// Source: Johnson & Christy (1972)
    pub fn copper() -> SpectralComplexIOR {
        SpectralComplexIOR {
            inner: metal_presets::COPPER,
        }
    }

    /// Aluminum (Al) - Bright white metal, slight blue tint.
    /// Source: Rakic (1995)
    pub fn aluminum() -> SpectralComplexIOR {
        SpectralComplexIOR {
            inner: metal_presets::ALUMINUM,
        }
    }

    /// Iron (Fe) - Dark gray metal.
    /// Source: Johnson & Christy (1974)
    pub fn iron() -> SpectralComplexIOR {
        SpectralComplexIOR {
            inner: metal_presets::IRON,
        }
    }

    /// Chromium (Cr) - Bright silver metal.
    pub fn chromium() -> SpectralComplexIOR {
        SpectralComplexIOR {
            inner: metal_presets::CHROMIUM,
        }
    }

    /// Titanium (Ti) - Dark silver with yellow tint.
    pub fn titanium() -> SpectralComplexIOR {
        SpectralComplexIOR {
            inner: metal_presets::TITANIUM,
        }
    }

    /// Nickel (Ni) - Warm silver metal.
    pub fn nickel() -> SpectralComplexIOR {
        SpectralComplexIOR {
            inner: metal_presets::NICKEL,
        }
    }

    /// Platinum (Pt) - Dense silver-white metal.
    pub fn platinum() -> SpectralComplexIOR {
        SpectralComplexIOR {
            inner: metal_presets::PLATINUM,
        }
    }

    /// Brass (Cu-Zn alloy) - Yellow metal.
    pub fn brass() -> SpectralComplexIOR {
        SpectralComplexIOR {
            inner: metal_presets::BRASS,
        }
    }

    /// Bronze (Cu-Sn alloy) - Brown metal.
    pub fn bronze() -> SpectralComplexIOR {
        SpectralComplexIOR {
            inner: metal_presets::BRONZE,
        }
    }

    /// Tungsten (W) - Dense gray metal.
    pub fn tungsten() -> SpectralComplexIOR {
        SpectralComplexIOR {
            inner: metal_presets::TUNGSTEN,
        }
    }

    // ========================================================================
    // Core Physics
    // ========================================================================

    /// Get F0 (normal incidence reflectance) for each RGB channel.
    ///
    /// THIS IS THE METAL'S COLOR - it emerges from the spectral response!
    ///
    /// # Returns
    ///
    /// Array [F0_red, F0_green, F0_blue]
    #[wasm_bindgen(js_name = f0Rgb)]
    pub fn f0_rgb(&self) -> Vec<f64> {
        self.inner.f0_rgb().to_vec()
    }

    /// Calculate full Fresnel reflectance for each RGB channel.
    ///
    /// Uses exact complex Fresnel equations for conductors.
    ///
    /// # Arguments
    ///
    /// * `n_i` - Incident medium IOR (1.0 for air)
    /// * `cos_theta_i` - Cosine of incident angle
    ///
    /// # Returns
    ///
    /// Array [R_red, R_green, R_blue]
    #[wasm_bindgen(js_name = fresnelRgb)]
    pub fn fresnel_rgb(&self, n_i: f64, cos_theta_i: f64) -> Vec<f64> {
        self.inner.fresnel_rgb(n_i, cos_theta_i).to_vec()
    }

    /// Calculate Schlick approximation for each RGB channel.
    ///
    /// Faster than full Fresnel, ~10% error.
    #[wasm_bindgen(js_name = fresnelSchlickRgb)]
    pub fn fresnel_schlick_rgb(&self, cos_theta_i: f64) -> Vec<f64> {
        self.inner.fresnel_schlick_rgb(cos_theta_i).to_vec()
    }

    /// Get IOR at red wavelength (~650nm).
    #[wasm_bindgen(getter)]
    pub fn red(&self) -> ComplexIOR {
        ComplexIOR {
            inner: self.inner.red,
        }
    }

    /// Get IOR at green wavelength (~550nm).
    #[wasm_bindgen(getter)]
    pub fn green(&self) -> ComplexIOR {
        ComplexIOR {
            inner: self.inner.green,
        }
    }

    /// Get IOR at blue wavelength (~450nm).
    #[wasm_bindgen(getter)]
    pub fn blue(&self) -> ComplexIOR {
        ComplexIOR {
            inner: self.inner.blue,
        }
    }

    /// Generate CSS for metallic gradient effect.
    #[wasm_bindgen(js_name = toCssGradient)]
    pub fn to_css_gradient(&self, intensity: f64) -> String {
        core_to_css_metallic_gradient(&self.inner, intensity)
    }

    /// Generate CSS for metallic surface with light angle.
    #[wasm_bindgen(js_name = toCssSurface)]
    pub fn to_css_surface(&self, light_angle_deg: f64) -> String {
        core_to_css_metallic_surface(&self.inner, light_angle_deg)
    }
}

// ============================================================================
// Sprint 2: Temperature-Dependent Metal Physics (Drude Model)
// ============================================================================

/// Drude model parameters for temperature-dependent metal optics.
///
/// Models metal optical properties as a function of temperature:
/// ε(ω, T) = ε∞ - ωₚ²(T) / (ω² + iγ(T)ω)
///
/// ## Example (JavaScript)
///
/// ```javascript
/// // Get gold Drude parameters
/// const gold = DrudeParams.gold();
///
/// // Calculate IOR at 550nm, room temperature
/// const ior300 = gold.complexIor(550.0, 300.0);
///
/// // Calculate IOR at 550nm, hot (500K)
/// const ior500 = gold.complexIor(550.0, 500.0);
///
/// // Higher temperature = more damping = slightly different color
/// ```
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct DrudeParams {
    inner: CoreDrudeParams,
}

#[wasm_bindgen]
impl DrudeParams {
    /// Create custom Drude parameters.
    #[wasm_bindgen(constructor)]
    pub fn new(
        eps_inf: f64,
        omega_p: f64,
        gamma: f64,
        t_ref: f64,
        d_omega_p: f64,
        d_gamma: f64,
    ) -> DrudeParams {
        DrudeParams {
            inner: CoreDrudeParams::new(eps_inf, omega_p, gamma, t_ref, d_omega_p, d_gamma),
        }
    }

    // ========================================================================
    // Metal Presets
    // ========================================================================

    /// Gold (Au) - Drude model from Ordal et al. (1983).
    pub fn gold() -> DrudeParams {
        DrudeParams {
            inner: drude_presets::GOLD,
        }
    }

    /// Silver (Ag) - Drude model.
    pub fn silver() -> DrudeParams {
        DrudeParams {
            inner: drude_presets::SILVER,
        }
    }

    /// Copper (Cu) - Drude model.
    pub fn copper() -> DrudeParams {
        DrudeParams {
            inner: drude_presets::COPPER,
        }
    }

    /// Aluminum (Al) - Drude model.
    pub fn aluminum() -> DrudeParams {
        DrudeParams {
            inner: drude_presets::ALUMINUM,
        }
    }

    /// Iron (Fe) - Drude model.
    pub fn iron() -> DrudeParams {
        DrudeParams {
            inner: drude_presets::IRON,
        }
    }

    /// Platinum (Pt) - Drude model.
    pub fn platinum() -> DrudeParams {
        DrudeParams {
            inner: drude_presets::PLATINUM,
        }
    }

    /// Nickel (Ni) - Drude model.
    pub fn nickel() -> DrudeParams {
        DrudeParams {
            inner: drude_presets::NICKEL,
        }
    }

    // ========================================================================
    // Temperature-Dependent Physics
    // ========================================================================

    /// Calculate complex IOR at given wavelength and temperature.
    ///
    /// # Arguments
    ///
    /// * `wavelength_nm` - Wavelength in nanometers
    /// * `temp_k` - Temperature in Kelvin
    ///
    /// # Returns
    ///
    /// ComplexIOR with temperature-adjusted n and k
    #[wasm_bindgen(js_name = complexIor)]
    pub fn complex_ior(&self, wavelength_nm: f64, temp_k: f64) -> ComplexIOR {
        ComplexIOR {
            inner: self.inner.complex_ior(wavelength_nm, temp_k),
        }
    }

    /// Calculate spectral IOR (RGB) at given temperature.
    #[wasm_bindgen(js_name = spectralIor)]
    pub fn spectral_ior(&self, temp_k: f64) -> SpectralComplexIOR {
        SpectralComplexIOR {
            inner: self.inner.spectral_ior(temp_k),
        }
    }

    /// Get temperature-adjusted plasma frequency and damping.
    ///
    /// # Returns
    ///
    /// Object { omegaP, gamma } at the given temperature
    #[wasm_bindgen(js_name = atTemperature)]
    pub fn at_temperature(&self, temp_k: f64) -> js_sys::Object {
        let (omega_p, gamma) = self.inner.at_temperature(temp_k);
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("omegaP"),
            &JsValue::from_f64(omega_p),
        )
        .unwrap();
        js_sys::Reflect::set(&obj, &JsValue::from_str("gamma"), &JsValue::from_f64(gamma)).unwrap();
        obj
    }
}

// ============================================================================
// Sprint 2: Utility Functions
// ============================================================================

/// Calculate Fresnel F0 (normal incidence reflectance) from IOR.
///
/// F0 = ((n - 1) / (n + 1))²
#[wasm_bindgen(js_name = f0FromIor)]
pub fn f0_from_ior(ior: f64) -> f64 {
    core_f0_from_ior(ior)
}

/// Get all metal presets with their names.
///
/// # Returns
///
/// Array of objects with { name, nRgb, kRgb, f0Rgb }
#[wasm_bindgen(js_name = getMetalPresets)]
pub fn get_metal_presets() -> js_sys::Array {
    let presets = metal_presets::all_presets();
    let array = js_sys::Array::new();

    for (name, metal) in presets {
        let obj = js_sys::Object::new();
        let f0 = metal.f0_rgb();

        js_sys::Reflect::set(&obj, &JsValue::from_str("name"), &JsValue::from_str(name)).unwrap();

        // F0 RGB - THE METAL'S COLOR
        let f0_arr = js_sys::Array::from_iter(f0.iter().map(|v| JsValue::from_f64(*v)));
        js_sys::Reflect::set(&obj, &JsValue::from_str("f0Rgb"), &f0_arr).unwrap();

        array.push(&obj);
    }

    array
}

/// Get all Drude metal presets with temperature capability.
#[wasm_bindgen(js_name = getDrudeMetalPresets)]
pub fn get_drude_metal_presets() -> js_sys::Array {
    let presets = drude_presets::all_presets();
    let array = js_sys::Array::new();

    for (name, drude) in presets {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &JsValue::from_str("name"), &JsValue::from_str(name)).unwrap();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("epsInf"),
            &JsValue::from_f64(drude.eps_inf),
        )
        .unwrap();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("omegaP"),
            &JsValue::from_f64(drude.omega_p),
        )
        .unwrap();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("gamma"),
            &JsValue::from_f64(drude.gamma),
        )
        .unwrap();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("tRef"),
            &JsValue::from_f64(drude.t_ref),
        )
        .unwrap();
        array.push(&obj);
    }

    array
}

/// Get dispersion wavelength constants.
///
/// # Returns
///
/// Object with standard wavelengths { red, green, blue, sodiumD, visibleMin, visibleMax }
#[wasm_bindgen(js_name = getDispersionWavelengths)]
pub fn get_dispersion_wavelengths() -> js_sys::Object {
    let obj = js_sys::Object::new();
    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("red"),
        &JsValue::from_f64(dispersion_wavelengths::RED),
    )
    .unwrap();
    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("green"),
        &JsValue::from_f64(dispersion_wavelengths::GREEN),
    )
    .unwrap();
    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("blue"),
        &JsValue::from_f64(dispersion_wavelengths::BLUE),
    )
    .unwrap();
    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("sodiumD"),
        &JsValue::from_f64(dispersion_wavelengths::SODIUM_D),
    )
    .unwrap();
    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("visibleMin"),
        &JsValue::from_f64(dispersion_wavelengths::VISIBLE_MIN),
    )
    .unwrap();
    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("visibleMax"),
        &JsValue::from_f64(dispersion_wavelengths::VISIBLE_MAX),
    )
    .unwrap();
    obj
}

// ============================================================================
// Sprint 3: Mie Scattering (Volumetric)
// ============================================================================
//
// "La niebla no tiene color. Tiene partículas."
//
// Mie scattering describes light interaction with particles comparable to wavelength.
// Key physics:
// - Size parameter: x = 2πr/λ
// - Rayleigh regime (x << 1): Blue sky, λ⁻⁴ dependence
// - Mie regime (x ~ 1-10): Complex lobes, forward scattering
// - Geometric regime (x >> 10): Ray optics, weak λ dependence

/// Mie scattering parameters for a single particle type.
///
/// # Physical Parameters
///
/// - `radius_um`: Particle radius in micrometers (µm)
/// - `n_particle`: Particle refractive index (typically 1.33 for water, 1.5 for dust)
/// - `n_medium`: Surrounding medium IOR (1.0 for air, 1.33 for water)
///
/// # Size Parameter
///
/// x = 2πr/λ determines scattering regime:
/// - x < 0.3: Rayleigh (blue scattering, symmetric)
/// - x ~ 1-10: Mie resonance (forward scattering)
/// - x > 30: Geometric (white scattering)
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct MieParams {
    inner: CoreMieParams,
}

#[wasm_bindgen]
impl MieParams {
    /// Create new Mie parameters.
    ///
    /// # Arguments
    ///
    /// * `radius_um` - Particle radius in micrometers
    /// * `n_particle` - Particle refractive index
    /// * `n_medium` - Medium refractive index (default 1.0 for air)
    #[wasm_bindgen(constructor)]
    pub fn new(radius_um: f64, n_particle: f64, n_medium: f64) -> MieParams {
        MieParams {
            inner: CoreMieParams::new(radius_um, n_particle, n_medium),
        }
    }

    // ========================================================================
    // Particle Presets
    // ========================================================================

    /// Fine dust (Rayleigh regime, x ~ 0.3).
    /// Creates blue-ish scattering, responsible for blue sky.
    #[wasm_bindgen(js_name = fineDust)]
    pub fn fine_dust() -> MieParams {
        MieParams {
            inner: mie_particle_presets::FINE_DUST,
        }
    }

    /// Coarse dust (Mie regime).
    #[wasm_bindgen(js_name = coarseDust)]
    pub fn coarse_dust() -> MieParams {
        MieParams {
            inner: mie_particle_presets::COARSE_DUST,
        }
    }

    /// Small fog droplet (~2µm water).
    #[wasm_bindgen(js_name = fogSmall)]
    pub fn fog_small() -> MieParams {
        MieParams {
            inner: mie_particle_presets::FOG_SMALL,
        }
    }

    /// Large fog droplet (~10µm water).
    #[wasm_bindgen(js_name = fogLarge)]
    pub fn fog_large() -> MieParams {
        MieParams {
            inner: mie_particle_presets::FOG_LARGE,
        }
    }

    /// Cloud droplet (~8µm water).
    pub fn cloud() -> MieParams {
        MieParams {
            inner: mie_particle_presets::CLOUD,
        }
    }

    /// Fine mist (~3µm water).
    pub fn mist() -> MieParams {
        MieParams {
            inner: mie_particle_presets::MIST,
        }
    }

    /// Smoke particle (~0.3µm soot).
    pub fn smoke() -> MieParams {
        MieParams {
            inner: mie_particle_presets::SMOKE,
        }
    }

    /// Milk fat globule (~2.5µm in water medium).
    #[wasm_bindgen(js_name = milkGlobule)]
    pub fn milk_globule() -> MieParams {
        MieParams {
            inner: mie_particle_presets::MILK_GLOBULE,
        }
    }

    /// Pollen grain (~25µm, geometric regime).
    pub fn pollen() -> MieParams {
        MieParams {
            inner: mie_particle_presets::POLLEN,
        }
    }

    // ========================================================================
    // Getters
    // ========================================================================

    /// Particle radius in micrometers.
    #[wasm_bindgen(getter, js_name = radiusUm)]
    pub fn radius_um(&self) -> f64 {
        self.inner.radius_um
    }

    /// Particle refractive index.
    #[wasm_bindgen(getter, js_name = nParticle)]
    pub fn n_particle(&self) -> f64 {
        self.inner.n_particle
    }

    /// Medium refractive index.
    #[wasm_bindgen(getter, js_name = nMedium)]
    pub fn n_medium(&self) -> f64 {
        self.inner.n_medium
    }

    // ========================================================================
    // Physics Calculations
    // ========================================================================

    /// Calculate size parameter for a wavelength.
    ///
    /// x = 2πr/λ
    ///
    /// # Arguments
    ///
    /// * `wavelength_nm` - Wavelength in nanometers
    ///
    /// # Returns
    ///
    /// Size parameter x (dimensionless)
    #[wasm_bindgen(js_name = sizeParameter)]
    pub fn size_parameter(&self, wavelength_nm: f64) -> f64 {
        self.inner.size_parameter(wavelength_nm)
    }

    /// Calculate relative refractive index (m = n_particle / n_medium).
    #[wasm_bindgen(js_name = relativeIor)]
    pub fn relative_ior(&self) -> f64 {
        self.inner.relative_ior()
    }

    /// Size parameters at R/G/B wavelengths (650/550/450nm).
    #[wasm_bindgen(js_name = sizeParamRgb)]
    pub fn size_param_rgb(&self) -> Vec<f64> {
        self.inner.size_param_rgb().to_vec()
    }

    /// Calculate Mie phase function at given angle and wavelength.
    ///
    /// # Arguments
    ///
    /// * `cos_theta` - Cosine of scattering angle (-1 to 1)
    /// * `wavelength_nm` - Wavelength in nanometers
    ///
    /// # Returns
    ///
    /// Phase function value (probability density)
    #[wasm_bindgen(js_name = phaseFunction)]
    pub fn phase_function(&self, cos_theta: f64, wavelength_nm: f64) -> f64 {
        core_mie_particle(cos_theta, &self.inner, wavelength_nm)
    }

    /// Calculate RGB phase function (wavelength-dependent).
    ///
    /// Returns [p_red, p_green, p_blue] at 650/550/450nm.
    #[wasm_bindgen(js_name = phaseRgb)]
    pub fn phase_rgb(&self, cos_theta: f64) -> Vec<f64> {
        core_mie_particle_rgb(cos_theta, &self.inner).to_vec()
    }

    /// Calculate asymmetry parameter g.
    ///
    /// g = 0: Isotropic (Rayleigh)
    /// g > 0: Forward scattering (Mie)
    /// g ~ 0.85: Strong forward (clouds)
    #[wasm_bindgen(js_name = asymmetryG)]
    pub fn asymmetry_g(&self, wavelength_nm: f64) -> f64 {
        let x = self.size_parameter(wavelength_nm);
        let m = self.relative_ior();
        core_mie_asymmetry_g(x, m)
    }

    /// Calculate scattering and extinction efficiencies.
    ///
    /// # Returns
    ///
    /// Object { Qsca, Qext } efficiency factors
    #[wasm_bindgen(js_name = efficiencies)]
    pub fn efficiencies(&self, wavelength_nm: f64) -> js_sys::Object {
        let x = self.size_parameter(wavelength_nm);
        let m = self.relative_ior();
        let (q_sca, q_ext) = core_mie_efficiencies(x, m);

        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &JsValue::from_str("Qsca"), &JsValue::from_f64(q_sca)).unwrap();
        js_sys::Reflect::set(&obj, &JsValue::from_str("Qext"), &JsValue::from_f64(q_ext)).unwrap();
        obj
    }

    /// Calculate scattering coefficient (1/µm).
    ///
    /// σ_s = Q_sca × π × r²
    #[wasm_bindgen(js_name = scatteringCoeff)]
    pub fn scattering_coeff(&self, wavelength_nm: f64) -> f64 {
        let x = self.size_parameter(wavelength_nm);
        let m = self.relative_ior();
        let (q_sca, _) = core_mie_efficiencies(x, m);
        q_sca * std::f64::consts::PI * self.inner.radius_um.powi(2)
    }

    /// Calculate extinction coefficient (1/µm).
    ///
    /// σ_ext = Q_ext × π × r²
    #[wasm_bindgen(js_name = extinctionCoeff)]
    pub fn extinction_coeff(&self, wavelength_nm: f64) -> f64 {
        let x = self.size_parameter(wavelength_nm);
        let m = self.relative_ior();
        let (_, q_ext) = core_mie_efficiencies(x, m);
        q_ext * std::f64::consts::PI * self.inner.radius_um.powi(2)
    }
}

// ============================================================================
// Dynamic Mie (Polydisperse Particle Systems)
// ============================================================================

/// Dynamic Mie parameters with size distribution.
///
/// Models real volumetric media like fog, clouds, and smoke which have
/// a distribution of particle sizes, not a single size.
///
/// # Size Distributions
///
/// - **Monodisperse**: Single particle size (idealized)
/// - **Log-normal**: Most common for atmospheric aerosols
/// - **Bimodal**: Two size modes (e.g., smoke with fine/coarse particles)
#[wasm_bindgen]
pub struct DynamicMieParams {
    inner: CoreDynamicMieParams,
}

#[wasm_bindgen]
impl DynamicMieParams {
    /// Create monodisperse (single-size) particle distribution.
    ///
    /// # Arguments
    ///
    /// * `radius_um` - Particle radius in micrometers
    /// * `n_particle` - Particle refractive index
    /// * `n_medium` - Medium refractive index
    #[wasm_bindgen(constructor)]
    pub fn new(radius_um: f64, n_particle: f64, n_medium: f64) -> DynamicMieParams {
        DynamicMieParams {
            inner: CoreDynamicMieParams::new(
                n_particle,
                n_medium,
                CoreSizeDistribution::Monodisperse { radius_um },
            ),
        }
    }

    /// Create log-normal size distribution.
    ///
    /// Most realistic for atmospheric particles.
    ///
    /// # Arguments
    ///
    /// * `geometric_mean_um` - Geometric mean radius (µm)
    /// * `geometric_std` - Geometric standard deviation (dimensionless, typically 1.2-2.5)
    /// * `n_particle` - Particle refractive index
    /// * `n_medium` - Medium refractive index
    #[wasm_bindgen(js_name = logNormal)]
    pub fn log_normal(
        geometric_mean_um: f64,
        geometric_std: f64,
        n_particle: f64,
        n_medium: f64,
    ) -> DynamicMieParams {
        DynamicMieParams {
            inner: CoreDynamicMieParams::new(
                n_particle,
                n_medium,
                CoreSizeDistribution::log_normal(geometric_mean_um, geometric_std),
            ),
        }
    }

    /// Create bimodal size distribution.
    ///
    /// Useful for smoke (fine soot + coarse aggregates).
    ///
    /// # Arguments
    ///
    /// * `mean1_um`, `std1` - First mode parameters
    /// * `mean2_um`, `std2` - Second mode parameters
    /// * `weight1` - Weight of first mode (0-1)
    /// * `n_particle`, `n_medium` - Refractive indices
    #[wasm_bindgen]
    pub fn bimodal(
        mean1_um: f64,
        std1: f64,
        mean2_um: f64,
        std2: f64,
        weight1: f64,
        n_particle: f64,
        n_medium: f64,
    ) -> DynamicMieParams {
        DynamicMieParams {
            inner: CoreDynamicMieParams::new(
                n_particle,
                n_medium,
                CoreSizeDistribution::bimodal(mean1_um, std1, mean2_um, std2, weight1),
            ),
        }
    }

    // ========================================================================
    // Volumetric Presets
    // ========================================================================

    /// Stratocumulus cloud droplets (~8µm water, forward scattering).
    pub fn stratocumulus() -> DynamicMieParams {
        DynamicMieParams {
            inner: mie_dynamic_presets::stratocumulus(),
        }
    }

    /// Fog droplets (~4µm water).
    pub fn fog() -> DynamicMieParams {
        DynamicMieParams {
            inner: mie_dynamic_presets::fog(),
        }
    }

    /// Smoke particles (bimodal soot distribution).
    pub fn smoke() -> DynamicMieParams {
        DynamicMieParams {
            inner: mie_dynamic_presets::smoke(),
        }
    }

    /// Milk (fat globules in water).
    pub fn milk() -> DynamicMieParams {
        DynamicMieParams {
            inner: mie_dynamic_presets::milk(),
        }
    }

    /// Desert dust storm.
    pub fn dust() -> DynamicMieParams {
        DynamicMieParams {
            inner: mie_dynamic_presets::dust(),
        }
    }

    /// Ice crystals (cirrus clouds).
    #[wasm_bindgen(js_name = iceCrystals)]
    pub fn ice_crystals() -> DynamicMieParams {
        DynamicMieParams {
            inner: mie_dynamic_presets::ice_crystals(),
        }
    }

    /// Condensing fog (growing droplets).
    #[wasm_bindgen(js_name = condensingFog)]
    pub fn condensing_fog() -> DynamicMieParams {
        DynamicMieParams {
            inner: mie_dynamic_presets::condensing_fog(),
        }
    }

    /// Evaporating mist (shrinking droplets).
    #[wasm_bindgen(js_name = evaporatingMist)]
    pub fn evaporating_mist() -> DynamicMieParams {
        DynamicMieParams {
            inner: mie_dynamic_presets::evaporating_mist(),
        }
    }

    // ========================================================================
    // Polydisperse Scattering
    // ========================================================================

    /// Calculate polydisperse phase function.
    ///
    /// Integrates over the size distribution:
    /// p_total(θ) = ∫ p(θ, r) × n(r) dr
    ///
    /// # Arguments
    ///
    /// * `cos_theta` - Cosine of scattering angle
    /// * `wavelength_nm` - Wavelength in nanometers
    #[wasm_bindgen(js_name = phaseFunction)]
    pub fn phase_function(&self, cos_theta: f64, wavelength_nm: f64) -> f64 {
        core_polydisperse_phase(cos_theta, &self.inner, wavelength_nm, 16)
    }

    /// Calculate RGB polydisperse phase function.
    ///
    /// Returns [p_red, p_green, p_blue] integrated over size distribution.
    #[wasm_bindgen(js_name = phaseRgb)]
    pub fn phase_rgb(&self, cos_theta: f64) -> Vec<f64> {
        core_polydisperse_phase_rgb(cos_theta, &self.inner, 16).to_vec()
    }

    /// Calculate effective asymmetry parameter for the distribution.
    ///
    /// Weighted average of g over all particle sizes.
    #[wasm_bindgen(js_name = effectiveG)]
    pub fn effective_g(&self, wavelength_nm: f64) -> f64 {
        core_effective_asymmetry_g(&self.inner, wavelength_nm)
    }

    /// Calculate extinction coefficient for the distribution.
    ///
    /// Total extinction from all particle sizes.
    #[wasm_bindgen(js_name = extinctionCoeff)]
    pub fn extinction_coeff(&self, wavelength_nm: f64) -> f64 {
        core_extinction_coefficient(&self.inner, wavelength_nm)
    }

    /// Generate CSS for fog-like volumetric effect.
    ///
    /// # Arguments
    ///
    /// * `density` - Optical density (0-1)
    #[wasm_bindgen(js_name = toCssFog)]
    pub fn to_css_fog(&self, density: f64) -> String {
        core_to_css_fog_effect(&self.inner, density)
    }

    /// Generate CSS for smoke-like volumetric effect.
    ///
    /// # Arguments
    ///
    /// * `density` - Optical density (0-1)
    #[wasm_bindgen(js_name = toCssSmoke)]
    pub fn to_css_smoke(&self, density: f64) -> String {
        core_to_css_smoke_effect(&self.inner, density)
    }
}

// ============================================================================
// Phase Functions
// ============================================================================

/// Henyey-Greenstein phase function.
///
/// Standard phase function for volumetric scattering.
///
/// p(θ) = (1 - g²) / (4π × (1 + g² - 2g×cosθ)^1.5)
///
/// # Arguments
///
/// * `cos_theta` - Cosine of scattering angle (-1 to 1)
/// * `g` - Asymmetry parameter (-1 to 1)
///
/// # Properties
///
/// - g = 0: Isotropic (Rayleigh-like)
/// - g > 0: Forward scattering (typical for aerosols)
/// - g < 0: Backward scattering
#[wasm_bindgen(js_name = henyeyGreenstein)]
pub fn henyey_greenstein(cos_theta: f64, g: f64) -> f64 {
    core_hg_fast(cos_theta, g)
}

/// Double Henyey-Greenstein phase function.
///
/// Two-lobe model for materials with both forward and backward scatter.
///
/// p(θ) = w × p_HG(θ, g_f) + (1-w) × p_HG(θ, g_b)
///
/// # Arguments
///
/// * `cos_theta` - Cosine of scattering angle
/// * `g_forward` - Forward lobe asymmetry (positive)
/// * `g_backward` - Backward lobe asymmetry (negative)
/// * `weight` - Forward lobe weight (0-1)
#[wasm_bindgen(js_name = doubleHenyeyGreenstein)]
pub fn double_henyey_greenstein(
    cos_theta: f64,
    g_forward: f64,
    g_backward: f64,
    weight: f64,
) -> f64 {
    core_double_henyey_greenstein(cos_theta, g_forward, g_backward, weight)
}

/// Rayleigh phase function.
///
/// For particles much smaller than wavelength (x << 1).
///
/// p(θ) = (3/4) × (1 + cos²θ)
///
/// # Properties
///
/// - Symmetric (equal forward/backward)
/// - Responsible for blue sky (λ⁻⁴ dependence)
#[wasm_bindgen(js_name = rayleighPhase)]
pub fn rayleigh_phase(cos_theta: f64) -> f64 {
    core_rayleigh_phase(cos_theta)
}

/// Rayleigh scattering efficiency.
///
/// Q_sca = (8/3) × x⁴ × |((m²-1)/(m²+2))|²
#[wasm_bindgen(js_name = rayleighEfficiency)]
pub fn rayleigh_efficiency(size_param: f64, relative_ior: f64) -> f64 {
    core_rayleigh_efficiency(size_param, relative_ior)
}

/// Rayleigh RGB intensity (wavelength-dependent).
///
/// Shows λ⁻⁴ blue enhancement.
#[wasm_bindgen(js_name = rayleighIntensityRgb)]
pub fn rayleigh_intensity_rgb(cos_theta: f64) -> Vec<f64> {
    core_rayleigh_intensity_rgb(cos_theta).to_vec()
}

// ============================================================================
// Sprint 3: Utility Functions
// ============================================================================

/// Get all particle presets with their names and properties.
#[wasm_bindgen(js_name = getMieParticlePresets)]
pub fn get_mie_particle_presets() -> js_sys::Array {
    let presets = mie_particle_presets::all_presets();
    let array = js_sys::Array::new();

    for (name, params) in presets {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &JsValue::from_str("name"), &JsValue::from_str(name)).unwrap();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("radiusUm"),
            &JsValue::from_f64(params.radius_um),
        )
        .unwrap();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("nParticle"),
            &JsValue::from_f64(params.n_particle),
        )
        .unwrap();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("nMedium"),
            &JsValue::from_f64(params.n_medium),
        )
        .unwrap();

        // Size parameter at 550nm
        let x = params.size_parameter(550.0);
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("sizeParam550"),
            &JsValue::from_f64(x),
        )
        .unwrap();

        // Scattering regime
        let regime = if x < 0.3 {
            "Rayleigh"
        } else if x < 30.0 {
            "Mie"
        } else {
            "Geometric"
        };
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("regime"),
            &JsValue::from_str(regime),
        )
        .unwrap();

        array.push(&obj);
    }

    array
}

/// Get all dynamic (polydisperse) presets.
#[wasm_bindgen(js_name = getMieDynamicPresets)]
pub fn get_mie_dynamic_presets() -> js_sys::Array {
    let presets = mie_dynamic_presets::all_presets();
    let array = js_sys::Array::new();

    for (name, params) in presets {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &JsValue::from_str("name"), &JsValue::from_str(name)).unwrap();

        // Effective asymmetry at 550nm
        let g = core_effective_asymmetry_g(&params, 550.0);
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("effectiveG"),
            &JsValue::from_f64(g),
        )
        .unwrap();

        // Extinction coefficient
        let ext = core_extinction_coefficient(&params, 550.0);
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("extinction550"),
            &JsValue::from_f64(ext),
        )
        .unwrap();

        array.push(&obj);
    }

    array
}

/// Calculate scattering color based on particle size.
///
/// Demonstrates the key principle:
/// - Small particles (Rayleigh): Blue scattering
/// - Large particles (Geometric): White/gray scattering
///
/// # Arguments
///
/// * `radius_um` - Particle radius in micrometers
/// * `n_particle` - Particle refractive index
///
/// # Returns
///
/// Object { r, g, b, regime, explanation }
#[wasm_bindgen(js_name = scatteringColorFromRadius)]
pub fn scattering_color_from_radius(radius_um: f64, n_particle: f64) -> js_sys::Object {
    let params = CoreMieParams::new(radius_um, n_particle, 1.0);

    // Phase at 45° scattering angle
    let cos_theta = 0.707; // cos(45°)

    let p_r = core_mie_particle(cos_theta, &params, 650.0);
    let p_g = core_mie_particle(cos_theta, &params, 550.0);
    let p_b = core_mie_particle(cos_theta, &params, 450.0);

    // Normalize to get relative intensities
    let max_p = p_r.max(p_g).max(p_b);
    let (r, g, b) = if max_p > 1e-10 {
        (p_r / max_p, p_g / max_p, p_b / max_p)
    } else {
        (1.0, 1.0, 1.0)
    };

    let x = params.size_parameter(550.0);
    let (regime, explanation) = if x < 0.3 {
        ("Rayleigh", "Small particles scatter blue light more (λ⁻⁴)")
    } else if x < 10.0 {
        ("Mie", "Complex wavelength dependence, forward scattering")
    } else {
        (
            "Geometric",
            "Large particles scatter all wavelengths equally (white/gray)",
        )
    };

    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &JsValue::from_str("r"), &JsValue::from_f64(r)).unwrap();
    js_sys::Reflect::set(&obj, &JsValue::from_str("g"), &JsValue::from_f64(g)).unwrap();
    js_sys::Reflect::set(&obj, &JsValue::from_str("b"), &JsValue::from_f64(b)).unwrap();
    js_sys::Reflect::set(&obj, &JsValue::from_str("sizeParam"), &JsValue::from_f64(x)).unwrap();
    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("regime"),
        &JsValue::from_str(regime),
    )
    .unwrap();
    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("explanation"),
        &JsValue::from_str(explanation),
    )
    .unwrap();
    obj
}

/// Get memory usage of Mie LUT.
#[wasm_bindgen(js_name = getMieLutMemory)]
pub fn get_mie_lut_memory() -> usize {
    CoreMieLUT::global().memory_size()
}

// ============================================================================
// Sprint 4: Advanced Thin Film (Multilayer, Structural Color)
// ============================================================================

/// Polarization state for thin-film calculations
///
/// # Variants
/// - `S` (TE): Electric field perpendicular to plane of incidence
/// - `P` (TM): Electric field parallel to plane of incidence
/// - `Average`: Unpolarized (average of S and P)
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub enum Polarization {
    /// S-polarization (TE, perpendicular)
    S = 0,
    /// P-polarization (TM, parallel)
    P = 1,
    /// Average of S and P (unpolarized)
    Average = 2,
}

impl From<Polarization> for CorePolarization {
    fn from(p: Polarization) -> Self {
        match p {
            Polarization::S => CorePolarization::S,
            Polarization::P => CorePolarization::P,
            Polarization::Average => CorePolarization::Average,
        }
    }
}

/// Single layer in a multilayer thin-film stack
///
/// Each layer has:
/// - Complex refractive index (n + ik)
/// - Thickness in nanometers
///
/// # Physics
///
/// The phase accumulated through the layer is:
/// δ = 2π * n * d * cos(θ) / λ
///
/// where θ is the angle inside the layer (from Snell's law).
#[wasm_bindgen]
pub struct FilmLayer {
    inner: CoreFilmLayer,
}

#[wasm_bindgen]
impl FilmLayer {
    /// Create a dielectric (lossless) layer
    ///
    /// # Arguments
    /// * `n` - Real refractive index
    /// * `thickness_nm` - Layer thickness in nanometers
    ///
    /// # Example
    /// ```javascript
    /// // Quarter-wave MgF2 layer at 550nm
    /// const layer = FilmLayer.dielectric(1.38, 99.6);
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn dielectric(n: f64, thickness_nm: f64) -> FilmLayer {
        FilmLayer {
            inner: CoreFilmLayer::dielectric(n, thickness_nm),
        }
    }

    /// Create an absorbing layer with complex IOR (n + ik)
    ///
    /// # Arguments
    /// * `n` - Real part of refractive index
    /// * `k` - Extinction coefficient (imaginary part)
    /// * `thickness_nm` - Layer thickness in nanometers
    ///
    /// # Example
    /// ```javascript
    /// // Thin aluminum layer
    /// const al = FilmLayer.absorbing(0.15, 3.5, 50.0);
    /// ```
    #[wasm_bindgen(js_name = absorbing)]
    pub fn absorbing(n: f64, k: f64, thickness_nm: f64) -> FilmLayer {
        FilmLayer {
            inner: CoreFilmLayer::absorbing(n, k, thickness_nm),
        }
    }

    /// Get real part of refractive index
    #[wasm_bindgen(getter, js_name = n)]
    pub fn n(&self) -> f64 {
        self.inner.n.re
    }

    /// Get extinction coefficient (imaginary part of n)
    #[wasm_bindgen(getter, js_name = k)]
    pub fn k(&self) -> f64 {
        self.inner.n.im
    }

    /// Get layer thickness in nanometers
    #[wasm_bindgen(getter, js_name = thicknessNm)]
    pub fn thickness_nm(&self) -> f64 {
        self.inner.thickness_nm
    }
}

/// Transfer Matrix Method calculator for multilayer thin-film stacks
///
/// # Physics Principle
///
/// "El color no está en el material. Está en la estructura."
/// (Color is not in the material. It's in the structure.)
///
/// The transfer matrix method computes:
/// ```text
/// M = D₀⁻¹ · Π(i=1..N) [Dᵢ · Pᵢ · Dᵢ⁻¹] · Dₛ
/// ```
///
/// where:
/// - Dᵢ = dynamical matrix (interface boundary conditions)
/// - Pᵢ = propagation matrix (phase accumulation)
///
/// # Features
///
/// - Full spectral analysis (reflectance, transmittance)
/// - S and P polarization support
/// - Angle-dependent calculations
/// - Structural color emergence
///
/// # Example
/// ```javascript
/// import { TransferMatrixFilm, Polarization } from 'momoto-wasm';
///
/// // Create 5-pair Bragg mirror
/// const mirror = TransferMatrixFilm.braggMirror(2.35, 1.46, 550.0, 5);
///
/// // Check reflectance at design wavelength
/// const r = mirror.reflectance(550.0, 0.0, Polarization.Average);
/// console.log(`Reflectance at 550nm: ${(r * 100).toFixed(1)}%`);
///
/// // Color emerges from STRUCTURE, not pigments
/// const rgb = mirror.reflectanceRgb(0.0, Polarization.Average);
/// ```
#[wasm_bindgen]
pub struct TransferMatrixFilm {
    inner: CoreTransferMatrixFilm,
}

#[wasm_bindgen]
impl TransferMatrixFilm {
    /// Create a new empty film stack
    ///
    /// # Arguments
    /// * `n_incident` - Incident medium refractive index (typically 1.0 for air)
    /// * `n_substrate` - Substrate refractive index (e.g., 1.52 for glass)
    #[wasm_bindgen(constructor)]
    pub fn new(n_incident: f64, n_substrate: f64) -> TransferMatrixFilm {
        TransferMatrixFilm {
            inner: CoreTransferMatrixFilm::new(n_incident, n_substrate),
        }
    }

    // ========================================================================
    // Layer Management
    // ========================================================================

    /// Add a dielectric layer to the stack
    ///
    /// Layers are added from the incident medium side toward the substrate.
    ///
    /// # Arguments
    /// * `n` - Real refractive index of the layer
    /// * `thickness_nm` - Layer thickness in nanometers
    #[wasm_bindgen(js_name = addLayer)]
    pub fn add_layer(&mut self, n: f64, thickness_nm: f64) {
        self.inner.add_layer(n, thickness_nm);
    }

    /// Add an absorbing layer with complex IOR
    ///
    /// # Arguments
    /// * `n` - Real part of refractive index
    /// * `k` - Extinction coefficient
    /// * `thickness_nm` - Layer thickness in nanometers
    #[wasm_bindgen(js_name = addAbsorbingLayer)]
    pub fn add_absorbing_layer(&mut self, n: f64, k: f64, thickness_nm: f64) {
        self.inner.add_absorbing_layer(n, k, thickness_nm);
    }

    /// Get the number of layers in the stack
    #[wasm_bindgen(getter, js_name = layerCount)]
    pub fn layer_count(&self) -> usize {
        self.inner.layer_count()
    }

    /// Get incident medium refractive index
    #[wasm_bindgen(getter, js_name = nIncident)]
    pub fn n_incident(&self) -> f64 {
        self.inner.n_incident.re
    }

    /// Get substrate refractive index
    #[wasm_bindgen(getter, js_name = nSubstrate)]
    pub fn n_substrate(&self) -> f64 {
        self.inner.n_substrate.re
    }

    // ========================================================================
    // Presets - Bragg Mirrors & High Reflectors
    // ========================================================================

    /// Create a Bragg mirror (distributed Bragg reflector)
    ///
    /// Alternating high/low index quarter-wave layers create
    /// wavelength-selective high reflectance.
    ///
    /// # Arguments
    /// * `n_high` - High index material (e.g., TiO2 = 2.35)
    /// * `n_low` - Low index material (e.g., SiO2 = 1.46)
    /// * `design_lambda` - Design wavelength in nm
    /// * `pairs` - Number of layer pairs
    ///
    /// # Physics
    ///
    /// Stop band width ∝ (n_high - n_low) / (n_high + n_low)
    /// Peak reflectance increases exponentially with pairs.
    ///
    /// # Example
    /// ```javascript
    /// // Green-reflecting Bragg mirror
    /// const mirror = TransferMatrixFilm.braggMirror(2.35, 1.46, 550.0, 10);
    /// console.log(`R @ 550nm: ${mirror.reflectance(550.0, 0.0, Polarization.Average)}`);
    /// ```
    #[wasm_bindgen(js_name = braggMirror)]
    pub fn bragg_mirror(
        n_high: f64,
        n_low: f64,
        design_lambda: f64,
        pairs: usize,
    ) -> TransferMatrixFilm {
        TransferMatrixFilm {
            inner: tmm_presets::bragg_mirror(n_high, n_low, design_lambda, pairs),
        }
    }

    /// Create a broadband anti-reflection coating
    ///
    /// Two-layer V-coat design for glass substrates.
    ///
    /// # Arguments
    /// * `design_lambda` - Center wavelength in nm (typically 550)
    #[wasm_bindgen(js_name = arBroadband)]
    pub fn ar_broadband(design_lambda: f64) -> TransferMatrixFilm {
        TransferMatrixFilm {
            inner: tmm_presets::ar_broadband(design_lambda),
        }
    }

    /// Create a notch filter (narrow rejection band)
    ///
    /// # Arguments
    /// * `center_lambda` - Center wavelength to reject in nm
    /// * `bandwidth_nm` - Approximate bandwidth in nm
    #[wasm_bindgen(js_name = notchFilter)]
    pub fn notch_filter(center_lambda: f64, bandwidth_nm: f64) -> TransferMatrixFilm {
        TransferMatrixFilm {
            inner: tmm_presets::notch_filter(center_lambda, bandwidth_nm),
        }
    }

    // ========================================================================
    // Presets - Dichroic Filters
    // ========================================================================

    /// Create a dichroic filter that reflects blue, transmits red/green
    ///
    /// Used in color separation and stage lighting.
    #[wasm_bindgen(js_name = dichroicBlueReflect)]
    pub fn dichroic_blue_reflect() -> TransferMatrixFilm {
        TransferMatrixFilm {
            inner: tmm_presets::dichroic_blue_reflect(),
        }
    }

    /// Create a dichroic filter that reflects red, transmits blue/green
    #[wasm_bindgen(js_name = dichroicRedReflect)]
    pub fn dichroic_red_reflect() -> TransferMatrixFilm {
        TransferMatrixFilm {
            inner: tmm_presets::dichroic_red_reflect(),
        }
    }

    // ========================================================================
    // Presets - Structural Color (Biological)
    // ========================================================================

    /// Create a Morpho butterfly wing structure
    ///
    /// # Physics
    ///
    /// The brilliant blue of Morpho butterflies is NOT from pigment.
    /// It emerges from irregularly-spaced chitin/air layers that
    /// create broadband constructive interference for blue light.
    ///
    /// Key characteristics:
    /// - Chitin (n ≈ 1.56) and air (n = 1.0) layers
    /// - Irregular spacing creates broadband reflection
    /// - Strong angle dependence (iridescence)
    ///
    /// # Example
    /// ```javascript
    /// const morpho = TransferMatrixFilm.morphoButterfly();
    ///
    /// // Blue emerges from STRUCTURE
    /// const rgb = morpho.reflectanceRgb(0.0, Polarization.Average);
    /// // rgb[2] (blue) >> rgb[0] (red)
    ///
    /// // Color shifts with angle
    /// const rgb45 = morpho.reflectanceRgb(45.0, Polarization.Average);
    /// // Blue shifts toward UV at oblique angles
    /// ```
    #[wasm_bindgen(js_name = morphoButterfly)]
    pub fn morpho_butterfly() -> TransferMatrixFilm {
        TransferMatrixFilm {
            inner: tmm_presets::morpho_butterfly(),
        }
    }

    /// Create a beetle shell iridescence structure
    ///
    /// Gradual index variation creates metallic-looking iridescence.
    #[wasm_bindgen(js_name = beetleShell)]
    pub fn beetle_shell() -> TransferMatrixFilm {
        TransferMatrixFilm {
            inner: tmm_presets::beetle_shell(),
        }
    }

    /// Create a nacre (mother of pearl) structure
    ///
    /// # Physics
    ///
    /// Nacre is made of aragonite (CaCO3) platelets in a protein matrix.
    /// The alternating high/low index creates pearlescent iridescence.
    ///
    /// - Aragonite: n ≈ 1.68
    /// - Protein matrix: n ≈ 1.34
    /// - ~20 platelet layers
    #[wasm_bindgen(js_name = nacre)]
    pub fn nacre() -> TransferMatrixFilm {
        TransferMatrixFilm {
            inner: tmm_presets::nacre(),
        }
    }

    /// Create an optical disc (CD/DVD) approximation
    ///
    /// Polycarbonate with thin metallic reflection layer.
    #[wasm_bindgen(js_name = opticalDisc)]
    pub fn optical_disc() -> TransferMatrixFilm {
        TransferMatrixFilm {
            inner: tmm_presets::optical_disc(),
        }
    }

    // ========================================================================
    // Physics Calculations
    // ========================================================================

    /// Calculate reflectance at a single wavelength and angle
    ///
    /// # Arguments
    /// * `wavelength_nm` - Wavelength in nanometers
    /// * `angle_deg` - Incidence angle in degrees (0 = normal)
    /// * `pol` - Polarization state (S, P, or Average)
    ///
    /// # Returns
    /// Reflectance (0.0 to 1.0)
    ///
    /// # Example
    /// ```javascript
    /// const mirror = TransferMatrixFilm.braggMirror(2.35, 1.46, 550.0, 10);
    ///
    /// // At design wavelength
    /// const r = mirror.reflectance(550.0, 0.0, Polarization.Average);
    /// // r > 0.95 (high reflectance)
    ///
    /// // Off-band
    /// const r_off = mirror.reflectance(700.0, 0.0, Polarization.Average);
    /// // r_off << r (low reflectance outside stop band)
    /// ```
    pub fn reflectance(&self, wavelength_nm: f64, angle_deg: f64, pol: Polarization) -> f64 {
        self.inner.reflectance(wavelength_nm, angle_deg, pol.into())
    }

    /// Calculate transmittance at a single wavelength and angle
    ///
    /// # Arguments
    /// * `wavelength_nm` - Wavelength in nanometers
    /// * `angle_deg` - Incidence angle in degrees
    /// * `pol` - Polarization state
    ///
    /// # Returns
    /// Transmittance (0.0 to 1.0)
    ///
    /// # Note
    /// For lossless films: R + T ≈ 1
    pub fn transmittance(&self, wavelength_nm: f64, angle_deg: f64, pol: Polarization) -> f64 {
        self.inner
            .transmittance(wavelength_nm, angle_deg, pol.into())
    }

    /// Calculate RGB reflectance (R=650nm, G=550nm, B=450nm)
    ///
    /// # Arguments
    /// * `angle_deg` - Incidence angle in degrees
    /// * `pol` - Polarization state
    ///
    /// # Returns
    /// Array [R, G, B] reflectance values (0.0 to 1.0)
    ///
    /// # Example
    /// ```javascript
    /// const morpho = TransferMatrixFilm.morphoButterfly();
    ///
    /// // Color EMERGES from structure
    /// const rgb = morpho.reflectanceRgb(0.0, Polarization.Average);
    ///
    /// // For Morpho: B >> R (structural blue)
    /// console.log(`R=${rgb[0].toFixed(2)}, G=${rgb[1].toFixed(2)}, B=${rgb[2].toFixed(2)}`);
    /// ```
    #[wasm_bindgen(js_name = reflectanceRgb)]
    pub fn reflectance_rgb(&self, angle_deg: f64, pol: Polarization) -> Vec<f64> {
        self.inner.reflectance_rgb(angle_deg, pol.into()).to_vec()
    }

    /// Calculate full spectrum reflectance
    ///
    /// # Arguments
    /// * `angle_deg` - Incidence angle in degrees
    /// * `pol` - Polarization state
    /// * `num_points` - Number of spectral points
    ///
    /// # Returns
    /// Object { wavelengths: number[], reflectances: number[] }
    #[wasm_bindgen(js_name = reflectanceSpectrum)]
    pub fn reflectance_spectrum(
        &self,
        angle_deg: f64,
        pol: Polarization,
        num_points: usize,
    ) -> js_sys::Object {
        let wavelengths: Vec<f64> = (0..num_points)
            .map(|i| 400.0 + (i as f64 / (num_points - 1) as f64) * 300.0)
            .collect();

        let reflectances = self
            .inner
            .reflectance_spectrum(&wavelengths, angle_deg, pol.into());

        let obj = js_sys::Object::new();
        let w_array = js_sys::Array::from_iter(wavelengths.iter().map(|v| JsValue::from_f64(*v)));
        let r_array = js_sys::Array::from_iter(reflectances.iter().map(|v| JsValue::from_f64(*v)));

        js_sys::Reflect::set(&obj, &JsValue::from_str("wavelengths"), &w_array).unwrap();
        js_sys::Reflect::set(&obj, &JsValue::from_str("reflectances"), &r_array).unwrap();

        obj
    }

    // ========================================================================
    // CSS Generation
    // ========================================================================

    /// Generate CSS gradient for structural color effect
    ///
    /// Samples reflectance at multiple angles to create an
    /// iridescent gradient showing the angle-dependent color.
    ///
    /// # Returns
    /// CSS linear-gradient string
    ///
    /// # Example
    /// ```javascript
    /// const morpho = TransferMatrixFilm.morphoButterfly();
    /// element.style.background = morpho.toCssStructuralColor();
    /// ```
    #[wasm_bindgen(js_name = toCssStructuralColor)]
    pub fn to_css_structural_color(&self) -> String {
        core_to_css_structural_color(&self.inner)
    }
}

// ============================================================================
// Sprint 4: Utility Functions
// ============================================================================

/// Generate CSS for a Bragg mirror at a specific design wavelength
///
/// # Arguments
/// * `design_lambda` - Design wavelength in nm
///
/// # Returns
/// CSS radial-gradient string
#[wasm_bindgen(js_name = toCssBraggMirror)]
pub fn to_css_bragg_mirror(design_lambda: f64) -> String {
    core_to_css_bragg_mirror(design_lambda)
}

/// Find the peak reflectance wavelength for a film stack
///
/// # Arguments
/// * `film` - TransferMatrixFilm to analyze
/// * `angle_deg` - Incidence angle in degrees
///
/// # Returns
/// Wavelength (nm) where reflectance is maximum
#[wasm_bindgen(js_name = findPeakWavelength)]
pub fn find_peak_wavelength(film: &TransferMatrixFilm, angle_deg: f64) -> f64 {
    core_find_peak_wavelength(&film.inner, angle_deg)
}

/// Calculate color shift with viewing angle
///
/// # Arguments
/// * `film` - TransferMatrixFilm to analyze
///
/// # Returns
/// Array of { angle: number, rgb: [r, g, b] } objects
#[wasm_bindgen(js_name = calculateColorShift)]
pub fn calculate_color_shift(film: &TransferMatrixFilm) -> js_sys::Array {
    let shifts = core_calculate_color_shift(&film.inner);
    let array = js_sys::Array::new();

    for (angle, rgb) in shifts {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &JsValue::from_str("angle"), &JsValue::from_f64(angle)).unwrap();

        let rgb_array = js_sys::Array::new();
        rgb_array.push(&JsValue::from_f64(rgb[0]));
        rgb_array.push(&JsValue::from_f64(rgb[1]));
        rgb_array.push(&JsValue::from_f64(rgb[2]));
        js_sys::Reflect::set(&obj, &JsValue::from_str("rgb"), &rgb_array).unwrap();

        array.push(&obj);
    }

    array
}

/// Get all advanced thin-film preset names and descriptions
///
/// # Returns
/// Array of { name: string, layerCount: number } objects
#[wasm_bindgen(js_name = getAdvancedThinFilmPresets)]
pub fn get_advanced_thin_film_presets() -> js_sys::Array {
    let presets = tmm_presets::all_presets();
    let array = js_sys::Array::new();

    for (name, film) in presets {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &JsValue::from_str("name"), &JsValue::from_str(name)).unwrap();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("layerCount"),
            &JsValue::from_f64(film.layer_count() as f64),
        )
        .unwrap();

        // Sample reflectance at 550nm
        let r = film.reflectance(550.0, 0.0, CorePolarization::Average);
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("reflectance550"),
            &JsValue::from_f64(r),
        )
        .unwrap();

        array.push(&obj);
    }

    array
}

/// Get transfer matrix memory usage
#[wasm_bindgen(js_name = getTransferMatrixMemory)]
pub fn get_transfer_matrix_memory() -> usize {
    core_transfer_matrix_memory()
}

/// Calculate quarter-wave thickness for a material
///
/// # Arguments
/// * `n` - Refractive index
/// * `design_lambda` - Design wavelength in nm
///
/// # Returns
/// Thickness in nm for quarter-wave optical path
#[wasm_bindgen(js_name = quarterWaveThickness)]
pub fn quarter_wave_thickness(n: f64, design_lambda: f64) -> f64 {
    design_lambda / (4.0 * n)
}

/// Demonstrate structural color principle
///
/// Creates a comparison showing that the SAME materials
/// with DIFFERENT structures produce DIFFERENT colors.
///
/// # Returns
/// Object with two stacks and their resulting colors
#[wasm_bindgen(js_name = demonstrateStructuralColor)]
pub fn demonstrate_structural_color() -> js_sys::Object {
    // Same materials (chitin n=1.56, air n=1.0), different structures
    let morpho = tmm_presets::morpho_butterfly(); // Blue structure
    let beetle = tmm_presets::beetle_shell(); // Different structure

    let rgb_morpho = morpho.reflectance_rgb(0.0, CorePolarization::Average);
    let rgb_beetle = beetle.reflectance_rgb(0.0, CorePolarization::Average);

    let result = js_sys::Object::new();

    // Morpho butterfly
    let morpho_obj = js_sys::Object::new();
    js_sys::Reflect::set(
        &morpho_obj,
        &JsValue::from_str("name"),
        &JsValue::from_str("Morpho Butterfly"),
    )
    .unwrap();
    js_sys::Reflect::set(
        &morpho_obj,
        &JsValue::from_str("layers"),
        &JsValue::from_f64(morpho.layer_count() as f64),
    )
    .unwrap();
    let morpho_rgb = js_sys::Array::from_iter(rgb_morpho.iter().map(|&v| JsValue::from_f64(v)));
    js_sys::Reflect::set(&morpho_obj, &JsValue::from_str("rgb"), &morpho_rgb).unwrap();
    js_sys::Reflect::set(&result, &JsValue::from_str("morpho"), &morpho_obj).unwrap();

    // Beetle shell
    let beetle_obj = js_sys::Object::new();
    js_sys::Reflect::set(
        &beetle_obj,
        &JsValue::from_str("name"),
        &JsValue::from_str("Beetle Shell"),
    )
    .unwrap();
    js_sys::Reflect::set(
        &beetle_obj,
        &JsValue::from_str("layers"),
        &JsValue::from_f64(beetle.layer_count() as f64),
    )
    .unwrap();
    let beetle_rgb = js_sys::Array::from_iter(rgb_beetle.iter().map(|&v| JsValue::from_f64(v)));
    js_sys::Reflect::set(&beetle_obj, &JsValue::from_str("rgb"), &beetle_rgb).unwrap();
    js_sys::Reflect::set(&result, &JsValue::from_str("beetle"), &beetle_obj).unwrap();

    // Key principle
    js_sys::Reflect::set(
        &result,
        &JsValue::from_str("principle"),
        &JsValue::from_str("El color no está en el material. Está en la estructura."),
    )
    .unwrap();

    result
}

// ============================================================================
// Sprint 5: Dynamic Optics (Thermo-Optic, Stress-Optic Coupling)
// ============================================================================
//
// Key Principle:
// "Un material vivo no cambia porque pasa el tiempo.
//  Cambia porque cambia su estado físico."
//
// Color emerges from PHYSICAL STATE (T, σ), not from time-based animation.
// ============================================================================

/// Dynamic thin-film layer with thermo-optic and mechanical response
///
/// # Physics Models
///
/// **Thermo-Optic Effect**:
/// ```text
/// n(T) = n_base + (dn/dT) × (T - T_ref)
/// ```
/// Typical dn/dT: 10⁻⁵ to 10⁻⁴ /K for glasses
///
/// **Thermal Expansion**:
/// ```text
/// d(T) = d_base × (1 + α × (T - T_ref))
/// ```
/// Typical α: 10⁻⁵ to 10⁻⁴ /K
///
/// **Stress-Optic (Photoelastic) Effect**:
/// ```text
/// ε = σ / E (strain from stress)
/// d(σ) = d_base × (1 + ε)
/// ```
///
/// # Units
///
/// - Temperature: Kelvin (K)
/// - Stress: MPa
/// - Young's modulus: GPa
/// - dn/dT: K⁻¹
/// - α (thermal expansion): K⁻¹
#[wasm_bindgen]
pub struct DynamicFilmLayer {
    inner: CoreDynamicFilmLayer,
}

#[wasm_bindgen]
impl DynamicFilmLayer {
    /// Create a new dynamic film layer
    ///
    /// # Arguments
    /// * `n` - Base refractive index at reference temperature (293K)
    /// * `thickness_nm` - Base thickness in nanometers
    ///
    /// Default properties:
    /// - dn/dT = 10⁻⁵ K⁻¹ (typical glass)
    /// - α_thermal = 5×10⁻⁶ K⁻¹ (typical SiO₂)
    /// - Young's modulus = 70 GPa
    /// - Poisson ratio = 0.17
    #[wasm_bindgen(constructor)]
    pub fn new(n: f64, thickness_nm: f64) -> DynamicFilmLayer {
        DynamicFilmLayer {
            inner: CoreDynamicFilmLayer::new(n, thickness_nm),
        }
    }

    /// Set thermo-optic coefficient dn/dT
    ///
    /// # Arguments
    /// * `dn_dt` - Thermo-optic coefficient in K⁻¹
    ///
    /// Typical values:
    /// - SiO₂: 1.0×10⁻⁵ K⁻¹
    /// - BK7 glass: 2.3×10⁻⁶ K⁻¹
    /// - Water: 1.0×10⁻⁴ K⁻¹
    /// - Polycarbonate: -1.0×10⁻⁴ K⁻¹ (negative!)
    #[wasm_bindgen(js_name = withDnDt)]
    pub fn with_dn_dt(mut self, dn_dt: f64) -> DynamicFilmLayer {
        self.inner = self.inner.with_dn_dt(dn_dt);
        self
    }

    /// Set thermal expansion coefficient α
    ///
    /// # Arguments
    /// * `alpha` - Thermal expansion coefficient in K⁻¹
    ///
    /// Typical values:
    /// - SiO₂: 5×10⁻⁷ K⁻¹
    /// - BK7 glass: 7×10⁻⁶ K⁻¹
    /// - Water (film): 2×10⁻⁴ K⁻¹
    /// - Aluminum: 2.3×10⁻⁵ K⁻¹
    #[wasm_bindgen(js_name = withThermalExpansion)]
    pub fn with_thermal_expansion(mut self, alpha: f64) -> DynamicFilmLayer {
        self.inner = self.inner.with_thermal_expansion(alpha);
        self
    }

    /// Set mechanical properties
    ///
    /// # Arguments
    /// * `youngs_modulus` - Young's modulus in GPa
    /// * `poisson_ratio` - Poisson's ratio (typically 0.1-0.4)
    #[wasm_bindgen(js_name = withMechanical)]
    pub fn with_mechanical(mut self, youngs_modulus: f64, poisson_ratio: f64) -> DynamicFilmLayer {
        self.inner = self.inner.with_mechanical(youngs_modulus, poisson_ratio);
        self
    }

    /// Set extinction coefficient k
    #[wasm_bindgen(js_name = withK)]
    pub fn with_k(mut self, k: f64) -> DynamicFilmLayer {
        self.inner = self.inner.with_k(k);
        self
    }

    /// Set current temperature (K)
    ///
    /// # Arguments
    /// * `temp_k` - Temperature in Kelvin
    ///
    /// Valid range: 100K to 1000K (outside range may give unphysical results)
    #[wasm_bindgen(js_name = setTemperature)]
    pub fn set_temperature(&mut self, temp_k: f64) {
        self.inner.set_temperature(temp_k);
    }

    /// Set stress state (Voigt notation)
    ///
    /// # Arguments
    /// * `stress` - [σxx, σyy, σzz, σxy, σyz, σzx] in MPa
    ///
    /// For uniaxial stress σ in z-direction: [0, 0, σ, 0, 0, 0]
    /// For biaxial stress σ in xy-plane: [σ, σ, 0, 0, 0, 0]
    #[wasm_bindgen(js_name = setStress)]
    pub fn set_stress(&mut self, stress: &[f64]) {
        if stress.len() >= 6 {
            self.inner.set_stress([
                stress[0], stress[1], stress[2], stress[3], stress[4], stress[5],
            ]);
        }
    }

    /// Get effective refractive index at current temperature
    ///
    /// n_eff = n_base + dn/dT × (T - T_ref)
    #[wasm_bindgen(getter, js_name = effectiveN)]
    pub fn effective_n(&self) -> f64 {
        self.inner.effective_n()
    }

    /// Get effective thickness at current conditions (temperature + stress)
    ///
    /// d_eff = d_base × (1 + thermal_strain + stress_strain)
    #[wasm_bindgen(getter, js_name = effectiveThickness)]
    pub fn effective_thickness(&self) -> f64 {
        self.inner.effective_thickness()
    }

    /// Get all effective properties as [n, k, thickness]
    #[wasm_bindgen(js_name = effectiveProperties)]
    pub fn effective_properties(&self) -> Vec<f64> {
        let (n, k, d) = self.inner.effective_properties();
        vec![n, k, d]
    }

    // Getters for base properties
    #[wasm_bindgen(getter, js_name = nBase)]
    pub fn n_base(&self) -> f64 {
        self.inner.n_base
    }

    #[wasm_bindgen(getter, js_name = kBase)]
    pub fn k_base(&self) -> f64 {
        self.inner.k_base
    }

    #[wasm_bindgen(getter, js_name = baseThickness)]
    pub fn base_thickness(&self) -> f64 {
        self.inner.thickness_nm
    }

    #[wasm_bindgen(getter, js_name = dnDt)]
    pub fn dn_dt(&self) -> f64 {
        self.inner.dn_dt
    }

    #[wasm_bindgen(getter, js_name = alphaThermal)]
    pub fn alpha_thermal(&self) -> f64 {
        self.inner.alpha_thermal
    }

    #[wasm_bindgen(getter, js_name = temperature)]
    pub fn temperature(&self) -> f64 {
        self.inner.temperature
    }
}

/// Dynamic thin-film stack with environmental response
///
/// A multilayer thin-film system that responds to:
/// - Temperature (K)
/// - Pressure (Pa)
/// - Humidity (0-1)
/// - Applied stress (MPa)
/// - Surface curvature (HeightMap)
///
/// # Physics
///
/// Each layer's optical properties depend on temperature:
/// - n(T) via thermo-optic coefficient
/// - d(T) via thermal expansion
/// - d(σ) via stress-strain relationship
///
/// # Example
/// ```javascript
/// const stack = DynamicThinFilmStack.soapBubble(293.0);
///
/// // Heat the bubble
/// stack.setEnvironment(310.0, 101325.0, 0.8);
///
/// // Color CHANGES because PHYSICS changes
/// const rgb = stack.reflectanceRgbAt(0.5, 0.5, 0.0);
/// ```
#[wasm_bindgen]
pub struct DynamicThinFilmStack {
    inner: CoreDynamicThinFilmStack,
}

#[wasm_bindgen]
impl DynamicThinFilmStack {
    /// Create a new empty dynamic stack
    ///
    /// # Arguments
    /// * `n_ambient` - Ambient medium refractive index (1.0 for air)
    /// * `n_substrate` - Substrate refractive index
    #[wasm_bindgen(constructor)]
    pub fn new(n_ambient: f64, n_substrate: f64) -> DynamicThinFilmStack {
        DynamicThinFilmStack {
            inner: CoreDynamicThinFilmStack::new(
                n_ambient,
                CoreSubstrateProperties {
                    n: n_substrate,
                    k: 0.0,
                    alpha: 7e-6,
                },
            ),
        }
    }

    /// Add a dynamic layer to the stack
    #[wasm_bindgen(js_name = addLayer)]
    pub fn add_layer(&mut self, layer: DynamicFilmLayer) {
        self.inner.add_layer(layer.inner);
    }

    /// Set environmental conditions
    ///
    /// # Arguments
    /// * `temp_k` - Temperature in Kelvin
    /// * `pressure_pa` - Pressure in Pascals (standard: 101325 Pa)
    /// * `humidity` - Relative humidity (0.0 to 1.0)
    ///
    /// This updates ALL layers in the stack to the new temperature.
    #[wasm_bindgen(js_name = setEnvironment)]
    pub fn set_environment(&mut self, temp_k: f64, pressure_pa: f64, humidity: f64) {
        self.inner.set_environment(temp_k, pressure_pa, humidity);
    }

    /// Apply uniform stress to all layers
    ///
    /// # Arguments
    /// * `stress` - [σxx, σyy, σzz, σxy, σyz, σzx] in MPa
    #[wasm_bindgen(js_name = applyStress)]
    pub fn apply_stress(&mut self, stress: &[f64]) {
        if stress.len() >= 6 {
            self.inner.apply_stress([
                stress[0], stress[1], stress[2], stress[3], stress[4], stress[5],
            ]);
        }
    }

    /// Calculate reflectance at a surface position
    ///
    /// # Arguments
    /// * `pos_x` - Normalized x position (0.0 to 1.0)
    /// * `pos_y` - Normalized y position (0.0 to 1.0)
    /// * `wavelength_nm` - Wavelength in nanometers
    /// * `angle_deg` - Viewing angle in degrees
    ///
    /// # Returns
    /// Reflectance (0.0 to 1.0)
    #[wasm_bindgen(js_name = reflectanceAt)]
    pub fn reflectance_at(
        &self,
        pos_x: f64,
        pos_y: f64,
        wavelength_nm: f64,
        angle_deg: f64,
    ) -> f64 {
        self.inner
            .reflectance_at(CoreVec2::new(pos_x, pos_y), wavelength_nm, angle_deg)
    }

    /// Calculate RGB reflectance at a surface position
    ///
    /// # Arguments
    /// * `pos_x` - Normalized x position (0.0 to 1.0)
    /// * `pos_y` - Normalized y position (0.0 to 1.0)
    /// * `angle_deg` - Viewing angle in degrees
    ///
    /// # Returns
    /// [R, G, B] reflectance array (0.0 to 1.0)
    #[wasm_bindgen(js_name = reflectanceRgbAt)]
    pub fn reflectance_rgb_at(&self, pos_x: f64, pos_y: f64, angle_deg: f64) -> Vec<f64> {
        let rgb = self
            .inner
            .reflectance_rgb_at(CoreVec2::new(pos_x, pos_y), angle_deg);
        rgb.to_vec()
    }

    /// Calculate RGB reflectance at center of surface (convenience method)
    ///
    /// # Arguments
    /// * `angle_deg` - Viewing angle in degrees
    ///
    /// # Returns
    /// [R, G, B] reflectance array (0.0 to 1.0)
    #[wasm_bindgen(js_name = reflectanceRgb)]
    pub fn reflectance_rgb(&self, angle_deg: f64) -> Vec<f64> {
        // Use center position (0.5, 0.5)
        self.reflectance_rgb_at(0.5, 0.5, angle_deg)
    }

    /// Get total optical thickness of the stack at current conditions
    ///
    /// # Returns
    /// Total thickness in nanometers (sum of all layer thicknesses)
    #[wasm_bindgen(js_name = totalThickness)]
    pub fn total_thickness(&self) -> f64 {
        self.inner
            .layers
            .iter()
            .map(|l| l.effective_thickness())
            .sum()
    }

    // ========================================================================
    // Presets
    // ========================================================================

    /// Create a soap bubble with temperature response
    ///
    /// Water film (n=1.33) has high dn/dT (~10⁻⁴ K⁻¹) and
    /// significant thermal expansion (~2×10⁻⁴ K⁻¹).
    ///
    /// # Arguments
    /// * `temp_k` - Initial temperature in Kelvin
    #[wasm_bindgen(js_name = soapBubble)]
    pub fn soap_bubble(temp_k: f64) -> DynamicThinFilmStack {
        DynamicThinFilmStack {
            inner: dynamic_thin_film_presets::soap_bubble(temp_k),
        }
    }

    /// Create an AR coating with stress response
    ///
    /// MgF₂ coating on glass with mechanical properties.
    /// Stress affects thickness and therefore optical performance.
    ///
    /// # Arguments
    /// * `stress_mpa` - Applied biaxial stress in MPa
    #[wasm_bindgen(js_name = arCoatingStressed)]
    pub fn ar_coating_stressed(stress_mpa: f64) -> DynamicThinFilmStack {
        DynamicThinFilmStack {
            inner: dynamic_thin_film_presets::ar_coating_stressed(stress_mpa),
        }
    }

    /// Create an oil slick on water with ripple pattern
    #[wasm_bindgen(js_name = oilSlickRippled)]
    pub fn oil_slick_rippled() -> DynamicThinFilmStack {
        DynamicThinFilmStack {
            inner: dynamic_thin_film_presets::oil_slick_rippled(),
        }
    }

    // Getters
    #[wasm_bindgen(getter, js_name = ambientTemp)]
    pub fn ambient_temp(&self) -> f64 {
        self.inner.ambient_temp
    }

    #[wasm_bindgen(getter, js_name = ambientPressure)]
    pub fn ambient_pressure(&self) -> f64 {
        self.inner.ambient_pressure
    }

    #[wasm_bindgen(getter, js_name = humidity)]
    pub fn humidity(&self) -> f64 {
        self.inner.humidity
    }

    #[wasm_bindgen(getter, js_name = layerCount)]
    pub fn layer_count(&self) -> usize {
        self.inner.layers.len()
    }
}

/// Temperature-dependent metal with oxidation layer
///
/// # Physics Models
///
/// **Drude Model** (temperature-dependent):
/// ```text
/// ε(ω, T) = ε∞ - ωₚ²(T) / (ω² + iγ(T)ω)
///
/// ωₚ(T) = ωₚ₀ × (1 + dωₚ/dT × ΔT)
/// γ(T) = γ₀ × (1 + dγ/dT × ΔT)
/// ```
///
/// Higher temperature → higher damping (γ) → redder, less reflective
///
/// **Oxidation Effects**:
/// - Oxide layer adds thin-film interference
/// - Oxidation level controls oxide thickness
/// - Different metals have different oxides (Cu₂O, CuO, Al₂O₃, Fe₂O₃, Ag₂S)
///
/// # Example
/// ```javascript
/// // Fresh copper at room temperature
/// const copper = TempOxidizedMetal.copperFresh();
///
/// // Heat to 500K - color shifts redder
/// copper.setTemperature(500.0);
/// const rgb = copper.effectiveReflectanceRgb(0.9);
///
/// // Add oxidation - color shifts to patina
/// copper.setOxidation(0.5);
/// const rgb_oxidized = copper.effectiveReflectanceRgb(0.9);
/// ```
#[wasm_bindgen]
pub struct TempOxidizedMetal {
    inner: CoreTempOxidizedMetal,
}

#[wasm_bindgen]
impl TempOxidizedMetal {
    /// Set temperature
    ///
    /// # Arguments
    /// * `temp_k` - Temperature in Kelvin
    ///
    /// Valid range: 200K to 1500K (outside may be unphysical)
    #[wasm_bindgen(js_name = setTemperature)]
    pub fn set_temperature(&mut self, temp_k: f64) {
        self.inner.temperature_k = temp_k;
    }

    /// Set oxidation level
    ///
    /// # Arguments
    /// * `level` - Oxidation level (0.0 = fresh, 1.0 = heavily oxidized)
    ///
    /// 0.0 = bare metal, native oxide only
    /// 0.3 = light tarnish
    /// 0.7 = significant oxidation
    /// 1.0 = maximum oxidation (patina/rust)
    #[wasm_bindgen(js_name = setOxidation)]
    pub fn set_oxidation(&mut self, level: f64) {
        self.inner.oxidation_level = level.clamp(0.0, 1.0);
    }

    /// Get metal spectral IOR at current temperature
    ///
    /// Returns [n_R, k_R, n_G, k_G, n_B, k_B]
    #[wasm_bindgen(js_name = metalSpectralIor)]
    pub fn metal_spectral_ior(&self) -> Vec<f64> {
        let spectral = self.inner.metal_spectral_ior();
        vec![
            spectral.red.n,
            spectral.red.k,
            spectral.green.n,
            spectral.green.k,
            spectral.blue.n,
            spectral.blue.k,
        ]
    }

    /// Get effective reflectance at wavelength including oxide layer
    ///
    /// # Arguments
    /// * `wavelength_nm` - Wavelength in nanometers
    /// * `cos_theta` - Cosine of incidence angle (1.0 = normal)
    #[wasm_bindgen(js_name = effectiveReflectance)]
    pub fn effective_reflectance(&self, wavelength_nm: f64, cos_theta: f64) -> f64 {
        self.inner.effective_reflectance(wavelength_nm, cos_theta)
    }

    /// Get effective RGB reflectance
    ///
    /// # Arguments
    /// * `cos_theta` - Cosine of incidence angle
    ///
    /// # Returns
    /// [R, G, B] reflectance (0.0 to 1.0)
    #[wasm_bindgen(js_name = effectiveReflectanceRgb)]
    pub fn effective_reflectance_rgb(&self, cos_theta: f64) -> Vec<f64> {
        self.inner.effective_reflectance_rgb(cos_theta).to_vec()
    }

    /// Generate CSS for temperature-dependent metal effect
    ///
    /// # Arguments
    /// * `light_angle_deg` - Light angle in degrees
    #[wasm_bindgen(js_name = toCssTempMetal)]
    pub fn to_css_temp_metal(&self, light_angle_deg: f64) -> String {
        core_to_css_temp_metal(&self.inner, light_angle_deg)
    }

    /// Generate CSS for patina effect
    #[wasm_bindgen(js_name = toCssPatina)]
    pub fn to_css_patina(&self) -> String {
        core_to_css_patina(&self.inner)
    }

    /// Get F0 (normal incidence reflectance) at RGB wavelengths
    ///
    /// This is the key method for emergent color - color comes from
    /// the spectral F0 response, not hardcoded values.
    ///
    /// # Returns
    /// [R, G, B] reflectance at normal incidence (0.0 to 1.0)
    #[wasm_bindgen(js_name = f0Rgb)]
    pub fn f0_rgb(&self) -> Vec<f64> {
        // F0 is reflectance at normal incidence (cos_theta = 1.0)
        self.inner.effective_reflectance_rgb(1.0).to_vec()
    }

    /// Get effective oxide layer thickness in nanometers
    ///
    /// The oxide thickness varies with oxidation level:
    /// - 0.0 = native oxide only (~2-5nm)
    /// - 0.5 = moderate oxidation (~50nm)
    /// - 1.0 = heavy oxidation/patina (~200nm+)
    #[wasm_bindgen(js_name = effectiveOxideThickness)]
    pub fn effective_oxide_thickness(&self) -> f64 {
        self.inner.oxide_film().thickness_nm
    }

    /// Generate CSS gradient from temperature-dependent physics
    ///
    /// Convenience method that combines temperature and oxidation effects
    /// into a single CSS gradient suitable for UI display.
    #[wasm_bindgen(js_name = toCssGradient)]
    pub fn to_css_gradient(&self) -> String {
        // Use the temp metal CSS with a 45° light angle
        core_to_css_temp_metal(&self.inner, 45.0)
    }

    // ========================================================================
    // Presets
    // ========================================================================

    /// Fresh copper (no oxidation, room temperature)
    #[wasm_bindgen(js_name = copperFresh)]
    pub fn copper_fresh() -> TempOxidizedMetal {
        TempOxidizedMetal {
            inner: oxidized_metal_presets::copper_fresh(),
        }
    }

    /// Tarnished copper (light oxidation)
    #[wasm_bindgen(js_name = copperTarnished)]
    pub fn copper_tarnished() -> TempOxidizedMetal {
        TempOxidizedMetal {
            inner: oxidized_metal_presets::copper_tarnished(),
        }
    }

    /// Copper with patina (heavy oxidation)
    #[wasm_bindgen(js_name = copperPatina)]
    pub fn copper_patina() -> TempOxidizedMetal {
        TempOxidizedMetal {
            inner: oxidized_metal_presets::copper_patina(),
        }
    }

    /// Fresh silver
    #[wasm_bindgen(js_name = silverFresh)]
    pub fn silver_fresh() -> TempOxidizedMetal {
        TempOxidizedMetal {
            inner: oxidized_metal_presets::silver_fresh(),
        }
    }

    /// Tarnished silver
    #[wasm_bindgen(js_name = silverTarnished)]
    pub fn silver_tarnished() -> TempOxidizedMetal {
        TempOxidizedMetal {
            inner: oxidized_metal_presets::silver_tarnished(),
        }
    }

    /// Fresh aluminum (with native oxide)
    #[wasm_bindgen(js_name = aluminumFresh)]
    pub fn aluminum_fresh() -> TempOxidizedMetal {
        TempOxidizedMetal {
            inner: oxidized_metal_presets::aluminum_fresh(),
        }
    }

    /// Rusty iron
    #[wasm_bindgen(js_name = ironRusty)]
    pub fn iron_rusty() -> TempOxidizedMetal {
        TempOxidizedMetal {
            inner: oxidized_metal_presets::iron_rusty(),
        }
    }

    /// Hot gold (elevated temperature)
    #[wasm_bindgen(js_name = goldHot)]
    pub fn gold_hot() -> TempOxidizedMetal {
        TempOxidizedMetal {
            inner: oxidized_metal_presets::gold_hot(),
        }
    }

    // Getters
    #[wasm_bindgen(getter, js_name = temperatureK)]
    pub fn temperature_k(&self) -> f64 {
        self.inner.temperature_k
    }

    #[wasm_bindgen(getter, js_name = oxidationLevel)]
    pub fn oxidation_level(&self) -> f64 {
        self.inner.oxidation_level
    }
}

// ============================================================================
// Sprint 5: Utility Functions
// ============================================================================

/// Calculate temperature sensitivity of a Drude metal
///
/// Returns array of [temp_K, reflectance] pairs showing how
/// reflectance changes with temperature.
///
/// # Arguments
/// * `metal` - Metal name ("gold", "silver", "copper", etc.)
/// * `wavelength_nm` - Wavelength in nanometers
///
/// # Returns
/// Array of { temperatureK, reflectance } objects
#[wasm_bindgen(js_name = calculateTemperatureSensitivity)]
pub fn calculate_temperature_sensitivity(metal: &str, wavelength_nm: f64) -> js_sys::Array {
    let drude = match metal.to_lowercase().as_str() {
        "gold" | "au" => drude_presets::GOLD,
        "silver" | "ag" => drude_presets::SILVER,
        "copper" | "cu" => drude_presets::COPPER,
        "aluminum" | "al" => drude_presets::ALUMINUM,
        "iron" | "fe" => drude_presets::IRON,
        "platinum" | "pt" => drude_presets::PLATINUM,
        "nickel" | "ni" => drude_presets::NICKEL,
        _ => drude_presets::GOLD,
    };

    let sensitivity = core_temperature_sensitivity(&drude, wavelength_nm);
    let array = js_sys::Array::new();

    for (temp, r) in sensitivity {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("temperatureK"),
            &JsValue::from_f64(temp),
        )
        .unwrap();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("reflectance"),
            &JsValue::from_f64(r),
        )
        .unwrap();
        array.push(&obj);
    }

    array
}

/// Demonstrate thermo-optic effect
///
/// Shows how the SAME material changes color with temperature.
/// This is NOT animation - this is PHYSICS.
///
/// # Returns
/// Object with cold/room/hot states and their RGB values
#[wasm_bindgen(js_name = demonstrateThermoOpticEffect)]
pub fn demonstrate_thermo_optic_effect() -> js_sys::Object {
    let result = js_sys::Object::new();

    // Create a water film (soap bubble) - high dn/dT
    let cold_bubble = dynamic_thin_film_presets::soap_bubble(273.0); // 0°C
    let room_bubble = dynamic_thin_film_presets::soap_bubble(293.0); // 20°C
    let warm_bubble = dynamic_thin_film_presets::soap_bubble(313.0); // 40°C

    let states = [
        ("cold_273K", cold_bubble),
        ("room_293K", room_bubble),
        ("warm_313K", warm_bubble),
    ];

    for (name, stack) in states {
        let rgb = stack.reflectance_rgb_at(CoreVec2::new(0.5, 0.5), 0.0);
        let obj = js_sys::Object::new();

        let rgb_array = js_sys::Array::new();
        rgb_array.push(&JsValue::from_f64(rgb[0]));
        rgb_array.push(&JsValue::from_f64(rgb[1]));
        rgb_array.push(&JsValue::from_f64(rgb[2]));

        js_sys::Reflect::set(&obj, &JsValue::from_str("rgb"), &rgb_array).unwrap();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("temperature"),
            &JsValue::from_str(name),
        )
        .unwrap();
        js_sys::Reflect::set(&result, &JsValue::from_str(name), &obj).unwrap();
    }

    // Key principle
    js_sys::Reflect::set(
        &result,
        &JsValue::from_str("principle"),
        &JsValue::from_str("Un material vivo no cambia porque pasa el tiempo. Cambia porque cambia su estado físico.")
    ).unwrap();

    result
}

/// Demonstrate stress-optic effect
///
/// Shows how applied stress changes optical response.
/// Biaxial stress in MPa affects film thickness and therefore color.
#[wasm_bindgen(js_name = demonstrateStressOpticEffect)]
pub fn demonstrate_stress_optic_effect() -> js_sys::Object {
    let result = js_sys::Object::new();

    let stresses = [
        ("no_stress", 0.0),
        ("low_stress_10MPa", 10.0),
        ("high_stress_50MPa", 50.0),
    ];

    for (name, stress) in stresses {
        let stack = dynamic_thin_film_presets::ar_coating_stressed(stress);
        let rgb = stack.reflectance_rgb_at(CoreVec2::new(0.5, 0.5), 0.0);

        let obj = js_sys::Object::new();
        let rgb_array = js_sys::Array::new();
        rgb_array.push(&JsValue::from_f64(rgb[0]));
        rgb_array.push(&JsValue::from_f64(rgb[1]));
        rgb_array.push(&JsValue::from_f64(rgb[2]));

        js_sys::Reflect::set(&obj, &JsValue::from_str("rgb"), &rgb_array).unwrap();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("stressMPa"),
            &JsValue::from_f64(stress),
        )
        .unwrap();
        js_sys::Reflect::set(&result, &JsValue::from_str(name), &obj).unwrap();
    }

    result
}

/// Get all oxidized metal presets
#[wasm_bindgen(js_name = getOxidizedMetalPresets)]
pub fn get_oxidized_metal_presets() -> js_sys::Array {
    let presets = oxidized_metal_presets::all_presets();
    let array = js_sys::Array::new();

    for (name, metal) in presets {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &JsValue::from_str("name"), &JsValue::from_str(name)).unwrap();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("temperatureK"),
            &JsValue::from_f64(metal.temperature_k),
        )
        .unwrap();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("oxidationLevel"),
            &JsValue::from_f64(metal.oxidation_level),
        )
        .unwrap();

        let rgb = metal.effective_reflectance_rgb(0.9);
        let rgb_array = js_sys::Array::new();
        rgb_array.push(&JsValue::from_f64(rgb[0]));
        rgb_array.push(&JsValue::from_f64(rgb[1]));
        rgb_array.push(&JsValue::from_f64(rgb[2]));
        js_sys::Reflect::set(&obj, &JsValue::from_str("rgb"), &rgb_array).unwrap();

        array.push(&obj);
    }

    array
}

/// Get memory usage for dynamic optics
#[wasm_bindgen(js_name = getDynamicOpticsMemory)]
pub fn get_dynamic_optics_memory() -> usize {
    core_temp_metal_memory() + std::mem::size_of::<CoreDynamicFilmLayer>() * 10
}

// ============================================================================
// Sprint 6: Unified Spectral Pipeline
// ============================================================================

/// Spectral signal - the universal data type for the pipeline
///
/// # Key Principle
///
/// > "RGB is a projection. Physics lives in the spectrum."
///
/// This type carries full spectral information through the pipeline.
/// RGB conversion happens ONLY at the end via `toRgb()`.
///
/// # Example
///
/// ```javascript
/// // Create uniform white light
/// const light = SpectralSignal.uniformDefault(1.0);
///
/// // Create pipeline
/// const pipeline = new SpectralPipeline();
/// pipeline.addThinFilm(1.45, 150.0, 1.52);
/// pipeline.addMetal(MetalPreset.Gold);
///
/// // Evaluate - still spectral!
/// const result = pipeline.evaluate(light, new EvaluationContext());
///
/// // ONLY NOW convert to RGB
/// const rgb = result.toRgb();  // [r, g, b]
/// ```
#[wasm_bindgen]
pub struct SpectralSignal {
    inner: CoreSpectralSignal,
}

#[wasm_bindgen]
impl SpectralSignal {
    /// Create from wavelengths and intensities arrays
    ///
    /// # Arguments
    /// * `wavelengths` - Wavelengths in nm
    /// * `intensities` - Intensity values (0.0 to 1.0 for reflectance)
    #[wasm_bindgen(constructor)]
    pub fn new(wavelengths: &[f64], intensities: &[f64]) -> SpectralSignal {
        SpectralSignal {
            inner: CoreSpectralSignal::from_arrays(wavelengths, intensities),
        }
    }

    /// Create uniform (flat) spectrum at given intensity
    ///
    /// # Arguments
    /// * `intensity` - Uniform intensity value
    #[wasm_bindgen(js_name = uniformDefault)]
    pub fn uniform_default(intensity: f64) -> SpectralSignal {
        SpectralSignal {
            inner: CoreSpectralSignal::uniform_default(intensity),
        }
    }

    /// Create D65 daylight illuminant (normalized white light)
    #[wasm_bindgen(js_name = d65Illuminant)]
    pub fn d65_illuminant() -> SpectralSignal {
        SpectralSignal {
            inner: CoreSpectralSignal::d65_illuminant(),
        }
    }

    /// Get interpolated intensity at arbitrary wavelength
    ///
    /// # Arguments
    /// * `wavelength_nm` - Wavelength in nanometers
    #[wasm_bindgen(js_name = intensityAt)]
    pub fn intensity_at(&self, wavelength_nm: f64) -> f64 {
        self.inner.intensity_at(wavelength_nm)
    }

    /// Get all wavelengths
    pub fn wavelengths(&self) -> Vec<f64> {
        self.inner.wavelengths()
    }

    /// Get all intensities
    pub fn intensities(&self) -> Vec<f64> {
        self.inner.intensities()
    }

    /// Get total integrated energy
    #[wasm_bindgen(js_name = totalEnergy)]
    pub fn total_energy(&self) -> f64 {
        self.inner.total_energy()
    }

    /// Get number of samples
    #[wasm_bindgen(getter, js_name = sampleCount)]
    pub fn sample_count(&self) -> usize {
        self.inner.samples().len()
    }

    /// Convert to CIE XYZ color space
    ///
    /// # Returns
    /// [X, Y, Z] tristimulus values
    #[wasm_bindgen(js_name = toXyz)]
    pub fn to_xyz(&self) -> Vec<f64> {
        self.inner.to_xyz().to_vec()
    }

    /// Convert to sRGB
    ///
    /// # Returns
    /// [R, G, B] in 0.0-1.0 range
    ///
    /// **THIS IS THE ONLY PLACE WHERE RGB IS COMPUTED.**
    /// All physics happens in spectral domain before this.
    #[wasm_bindgen(js_name = toRgb)]
    pub fn to_rgb(&self) -> Vec<f64> {
        self.inner.to_rgb().to_vec()
    }

    /// Convert to sRGB as u8 values
    ///
    /// # Returns
    /// [R, G, B] in 0-255 range
    #[wasm_bindgen(js_name = toRgbU8)]
    pub fn to_rgb_u8(&self) -> Vec<u8> {
        self.inner.to_rgb_u8().to_vec()
    }

    /// Multiply by another signal (element-wise, with interpolation)
    pub fn multiply(&self, other: &SpectralSignal) -> SpectralSignal {
        SpectralSignal {
            inner: self.inner.multiply(&other.inner),
        }
    }

    /// Scale by constant factor
    pub fn scale(&self, factor: f64) -> SpectralSignal {
        SpectralSignal {
            inner: self.inner.scale(factor),
        }
    }
}

/// Evaluation context for spectral pipeline
///
/// Contains all physical parameters that affect optical phenomena.
#[wasm_bindgen]
pub struct EvaluationContext {
    inner: CoreEvaluationContext,
}

#[wasm_bindgen]
impl EvaluationContext {
    /// Create default context (normal incidence, room temperature)
    #[wasm_bindgen(constructor)]
    pub fn new() -> EvaluationContext {
        EvaluationContext {
            inner: CoreEvaluationContext::default(),
        }
    }

    /// Set viewing angle in degrees
    ///
    /// # Arguments
    /// * `angle_deg` - Angle from normal in degrees (0 = normal, 90 = grazing)
    #[wasm_bindgen(js_name = withAngle)]
    pub fn with_angle(mut self, angle_deg: f64) -> EvaluationContext {
        self.inner = self.inner.with_angle_deg(angle_deg);
        self
    }

    /// Set temperature in Kelvin
    ///
    /// # Arguments
    /// * `temp_k` - Temperature in Kelvin
    #[wasm_bindgen(js_name = withTemperature)]
    pub fn with_temperature(mut self, temp_k: f64) -> EvaluationContext {
        self.inner = self.inner.with_temperature(temp_k);
        self
    }

    /// Set stress tensor
    ///
    /// # Arguments
    /// * `stress` - [σxx, σyy, σzz, σxy, σyz, σzx] in MPa
    #[wasm_bindgen(js_name = withStress)]
    pub fn with_stress(mut self, stress: &[f64]) -> EvaluationContext {
        if stress.len() >= 6 {
            self.inner = self.inner.with_stress([
                stress[0], stress[1], stress[2], stress[3], stress[4], stress[5],
            ]);
        }
        self
    }

    /// Set surface position
    ///
    /// # Arguments
    /// * `x` - X position (0.0 to 1.0)
    /// * `y` - Y position (0.0 to 1.0)
    #[wasm_bindgen(js_name = withPosition)]
    pub fn with_position(mut self, x: f64, y: f64) -> EvaluationContext {
        self.inner = self.inner.with_position(x, y);
        self
    }

    // Getters
    #[wasm_bindgen(getter, js_name = cosTheta)]
    pub fn cos_theta(&self) -> f64 {
        self.inner.cos_theta
    }

    #[wasm_bindgen(getter, js_name = temperatureK)]
    pub fn temperature_k(&self) -> f64 {
        self.inner.temperature_k
    }
}

/// Unified spectral pipeline
///
/// Composes multiple optical phenomena into a single coherent system.
/// RGB is ONLY computed at the very end via `toRgb()` on the result.
///
/// # Key Principle
///
/// > "RGB is a projection. Physics lives in the spectrum."
///
/// # Example
///
/// ```javascript
/// const pipeline = new SpectralPipeline();
///
/// // Add phenomena in physical order
/// pipeline.addThinFilm(1.45, 150.0, 1.52);
/// pipeline.addDispersion(1.52, 4300.0, 0.0);
/// pipeline.addMieScattering(5.0, 1.33, 1.0);
///
/// // Evaluate with context
/// const incident = SpectralSignal.d65Illuminant();
/// const context = new EvaluationContext().withAngle(45);
///
/// const result = pipeline.evaluate(incident, context);
/// const rgb = result.toRgb();  // ONLY NOW convert to RGB
/// ```
#[wasm_bindgen]
pub struct SpectralPipeline {
    inner: CoreSpectralPipeline,
}

#[wasm_bindgen]
impl SpectralPipeline {
    /// Create empty pipeline
    #[wasm_bindgen(constructor)]
    pub fn new() -> SpectralPipeline {
        SpectralPipeline {
            inner: CoreSpectralPipeline::new(),
        }
    }

    /// Add thin film interference stage
    ///
    /// # Arguments
    /// * `n_film` - Film refractive index
    /// * `thickness_nm` - Film thickness in nanometers
    /// * `n_substrate` - Substrate refractive index
    #[wasm_bindgen(js_name = addThinFilm)]
    pub fn add_thin_film(&mut self, n_film: f64, thickness_nm: f64, n_substrate: f64) {
        self.inner = std::mem::take(&mut self.inner).add_stage(CoreThinFilmStage::new(
            n_film,
            thickness_nm,
            n_substrate,
        ));
    }

    /// Add dispersion stage (Cauchy model)
    ///
    /// # Arguments
    /// * `a` - Cauchy A coefficient
    /// * `b` - Cauchy B coefficient
    /// * `c` - Cauchy C coefficient
    #[wasm_bindgen(js_name = addDispersion)]
    pub fn add_dispersion(&mut self, a: f64, b: f64, c: f64) {
        self.inner = std::mem::take(&mut self.inner).add_stage(CoreDispersionStage::new(a, b, c));
    }

    /// Add crown glass dispersion
    #[wasm_bindgen(js_name = addCrownGlassDispersion)]
    pub fn add_crown_glass_dispersion(&mut self) {
        self.inner = std::mem::take(&mut self.inner).add_stage(CoreDispersionStage::crown_glass());
    }

    /// Add Mie scattering stage
    ///
    /// # Arguments
    /// * `radius_um` - Particle radius in micrometers
    /// * `n_particle` - Particle refractive index
    /// * `n_medium` - Medium refractive index
    #[wasm_bindgen(js_name = addMieScattering)]
    pub fn add_mie_scattering(&mut self, radius_um: f64, n_particle: f64, n_medium: f64) {
        self.inner = std::mem::take(&mut self.inner)
            .add_stage(CoreMieScatteringStage::new(radius_um, n_particle, n_medium));
    }

    /// Add fog scattering
    #[wasm_bindgen(js_name = addFog)]
    pub fn add_fog(&mut self) {
        self.inner = std::mem::take(&mut self.inner).add_stage(CoreMieScatteringStage::fog());
    }

    /// Add thermo-optic stage
    ///
    /// # Arguments
    /// * `n_base` - Base refractive index at reference temperature
    /// * `dn_dt` - Thermo-optic coefficient (dn/dT) in K⁻¹
    /// * `thickness_nm` - Film thickness in nm
    /// * `alpha_thermal` - Thermal expansion coefficient in K⁻¹
    #[wasm_bindgen(js_name = addThermoOptic)]
    pub fn add_thermo_optic(
        &mut self,
        n_base: f64,
        dn_dt: f64,
        thickness_nm: f64,
        alpha_thermal: f64,
    ) {
        self.inner = std::mem::take(&mut self.inner).add_stage(CoreThermoOpticStage::new(
            n_base,
            dn_dt,
            thickness_nm,
            alpha_thermal,
        ));
    }

    /// Add gold metal reflectance
    #[wasm_bindgen(js_name = addGold)]
    pub fn add_gold(&mut self) {
        self.inner = std::mem::take(&mut self.inner).add_stage(CoreMetalReflectanceStage::gold());
    }

    /// Add silver metal reflectance
    #[wasm_bindgen(js_name = addSilver)]
    pub fn add_silver(&mut self) {
        self.inner = std::mem::take(&mut self.inner).add_stage(CoreMetalReflectanceStage::silver());
    }

    /// Add copper metal reflectance
    #[wasm_bindgen(js_name = addCopper)]
    pub fn add_copper(&mut self) {
        self.inner = std::mem::take(&mut self.inner).add_stage(CoreMetalReflectanceStage::copper());
    }

    /// Evaluate the complete pipeline
    ///
    /// # Arguments
    /// * `incident` - Incident light spectrum
    /// * `context` - Evaluation context
    ///
    /// # Returns
    /// Final spectral signal (use `.toRgb()` to convert to color)
    pub fn evaluate(
        &self,
        incident: &SpectralSignal,
        context: &EvaluationContext,
    ) -> SpectralSignal {
        SpectralSignal {
            inner: self.inner.evaluate(&incident.inner, &context.inner),
        }
    }

    /// Evaluate and return intermediate results for visualization
    ///
    /// # Returns
    /// Array of { name: string, wavelengths: number[], intensities: number[] }
    #[wasm_bindgen(js_name = evaluateWithIntermediates)]
    pub fn evaluate_with_intermediates(
        &self,
        incident: &SpectralSignal,
        context: &EvaluationContext,
    ) -> js_sys::Array {
        let results = self
            .inner
            .evaluate_with_intermediates(&incident.inner, &context.inner);

        let array = js_sys::Array::new();
        for (name, signal) in results {
            let obj = js_sys::Object::new();

            js_sys::Reflect::set(&obj, &JsValue::from_str("name"), &JsValue::from_str(&name))
                .unwrap();

            let wavelengths = js_sys::Float64Array::from(&signal.wavelengths()[..]);
            js_sys::Reflect::set(&obj, &JsValue::from_str("wavelengths"), &wavelengths).unwrap();

            let intensities = js_sys::Float64Array::from(&signal.intensities()[..]);
            js_sys::Reflect::set(&obj, &JsValue::from_str("intensities"), &intensities).unwrap();

            // Also include RGB for convenience
            let rgb = signal.to_rgb();
            let rgb_array = js_sys::Float64Array::from(&rgb[..]);
            js_sys::Reflect::set(&obj, &JsValue::from_str("rgb"), &rgb_array).unwrap();

            array.push(&obj);
        }

        array
    }

    /// Get number of stages
    #[wasm_bindgen(getter, js_name = stageCount)]
    pub fn stage_count(&self) -> usize {
        self.inner.stage_count()
    }

    /// Get stage names
    #[wasm_bindgen(js_name = stageNames)]
    pub fn stage_names(&self) -> js_sys::Array {
        let array = js_sys::Array::new();
        for name in self.inner.stage_names() {
            array.push(&JsValue::from_str(name));
        }
        array
    }

    /// Verify energy conservation
    #[wasm_bindgen(js_name = verifyEnergyConservation)]
    pub fn verify_energy_conservation(
        &self,
        incident: &SpectralSignal,
        context: &EvaluationContext,
    ) -> bool {
        self.inner
            .verify_energy_conservation(&incident.inner, &context.inner)
    }
}

// ============================================================================
// Sprint 6: Utility Functions
// ============================================================================

/// Get default spectral sampling wavelengths (31 points, 380-780nm)
#[wasm_bindgen(js_name = getDefaultSpectralSampling)]
pub fn get_default_spectral_sampling() -> Vec<f64> {
    spectral_wavelengths::default_sampling()
}

/// Get high-resolution spectral sampling (81 points, 380-780nm)
#[wasm_bindgen(js_name = getHighResSpectralSampling)]
pub fn get_high_res_spectral_sampling() -> Vec<f64> {
    spectral_wavelengths::high_resolution_sampling()
}

/// Get RGB-only sampling wavelengths (3 points)
#[wasm_bindgen(js_name = getRgbSampling)]
pub fn get_rgb_sampling() -> Vec<f64> {
    spectral_wavelengths::rgb_sampling()
}

/// Demonstrate spectral pipeline with different configurations
///
/// Returns comparison data showing how the same material
/// produces different colors under different conditions.
#[wasm_bindgen(js_name = demonstrateSpectralPipeline)]
pub fn demonstrate_spectral_pipeline() -> js_sys::Object {
    let obj = js_sys::Object::new();

    // Create pipelines with different configurations
    let gold_pipeline = CorePipelineBuilder::new().with_gold().build();

    let gold_with_film = CorePipelineBuilder::new()
        .with_thin_film(1.45, 100.0, 1.52)
        .with_gold()
        .build();

    let incident = CoreSpectralSignal::uniform_default(1.0);
    let normal_context = CoreEvaluationContext::default();
    let grazing_context = CoreEvaluationContext::default().with_angle_deg(75.0);

    // Gold at normal incidence
    let gold_normal = gold_pipeline.evaluate(&incident, &normal_context);
    let gold_normal_rgb = gold_normal.to_rgb();

    // Gold at grazing angle
    let gold_grazing = gold_pipeline.evaluate(&incident, &grazing_context);
    let gold_grazing_rgb = gold_grazing.to_rgb();

    // Gold with thin film coating at normal
    let gold_coated = gold_with_film.evaluate(&incident, &normal_context);
    let gold_coated_rgb = gold_coated.to_rgb();

    // Build result object
    let gold_normal_obj = js_sys::Object::new();
    js_sys::Reflect::set(
        &gold_normal_obj,
        &JsValue::from_str("description"),
        &JsValue::from_str("Gold at normal incidence"),
    )
    .unwrap();
    js_sys::Reflect::set(
        &gold_normal_obj,
        &JsValue::from_str("rgb"),
        &js_sys::Float64Array::from(&gold_normal_rgb[..]),
    )
    .unwrap();

    let gold_grazing_obj = js_sys::Object::new();
    js_sys::Reflect::set(
        &gold_grazing_obj,
        &JsValue::from_str("description"),
        &JsValue::from_str("Gold at 75° grazing angle"),
    )
    .unwrap();
    js_sys::Reflect::set(
        &gold_grazing_obj,
        &JsValue::from_str("rgb"),
        &js_sys::Float64Array::from(&gold_grazing_rgb[..]),
    )
    .unwrap();

    let gold_coated_obj = js_sys::Object::new();
    js_sys::Reflect::set(
        &gold_coated_obj,
        &JsValue::from_str("description"),
        &JsValue::from_str("Gold with 100nm thin film coating"),
    )
    .unwrap();
    js_sys::Reflect::set(
        &gold_coated_obj,
        &JsValue::from_str("rgb"),
        &js_sys::Float64Array::from(&gold_coated_rgb[..]),
    )
    .unwrap();

    js_sys::Reflect::set(&obj, &JsValue::from_str("goldNormal"), &gold_normal_obj).unwrap();
    js_sys::Reflect::set(&obj, &JsValue::from_str("goldGrazing"), &gold_grazing_obj).unwrap();
    js_sys::Reflect::set(&obj, &JsValue::from_str("goldCoated"), &gold_coated_obj).unwrap();

    // Add principle
    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("principle"),
        &JsValue::from_str("RGB is a projection. Physics lives in the spectrum."),
    )
    .unwrap();

    obj
}

// ============================================================================
// Gamma Module - sRGB Transfer Functions
// ============================================================================

/// sRGB gamma correction utilities.
///
/// These functions convert between gamma-corrected sRGB and linear RGB values.
/// Essential for physically accurate color calculations.
#[wasm_bindgen]
pub struct Gamma;

#[wasm_bindgen]
impl Gamma {
    /// Convert sRGB channel value (0.0-1.0) to linear RGB.
    ///
    /// # Arguments
    /// * `channel` - sRGB channel value (0.0 to 1.0)
    ///
    /// # Returns
    /// Linear RGB channel value
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// const srgb = 0.5; // Mid gray in sRGB
    /// const linear = Gamma.srgbToLinear(srgb);
    /// console.log(linear); // ~0.214 (NOT 0.5!)
    /// ```
    #[wasm_bindgen(js_name = srgbToLinear)]
    pub fn srgb_to_linear(channel: f64) -> f64 {
        momoto_core::gamma::srgb_to_linear(channel)
    }

    /// Convert linear RGB channel value (0.0-1.0) to sRGB.
    ///
    /// # Arguments
    /// * `channel` - Linear RGB channel value (0.0 to 1.0)
    ///
    /// # Returns
    /// sRGB channel value
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// const linear = 0.214;
    /// const srgb = Gamma.linearToSrgb(linear);
    /// console.log(srgb); // ~0.5
    /// ```
    #[wasm_bindgen(js_name = linearToSrgb)]
    pub fn linear_to_srgb(channel: f64) -> f64 {
        momoto_core::gamma::linear_to_srgb(channel)
    }

    /// Convert RGB array from sRGB to linear.
    ///
    /// # Arguments
    /// * `r`, `g`, `b` - sRGB values (0.0 to 1.0)
    ///
    /// # Returns
    /// Linear RGB values as Float64Array
    #[wasm_bindgen(js_name = rgbToLinear)]
    pub fn rgb_to_linear(r: f64, g: f64, b: f64) -> js_sys::Float64Array {
        let linear = [
            momoto_core::gamma::srgb_to_linear(r),
            momoto_core::gamma::srgb_to_linear(g),
            momoto_core::gamma::srgb_to_linear(b),
        ];
        js_sys::Float64Array::from(&linear[..])
    }

    /// Convert RGB array from linear to sRGB.
    ///
    /// # Arguments
    /// * `r`, `g`, `b` - Linear RGB values (0.0 to 1.0)
    ///
    /// # Returns
    /// sRGB values as Float64Array
    #[wasm_bindgen(js_name = linearToRgb)]
    pub fn linear_to_rgb(r: f64, g: f64, b: f64) -> js_sys::Float64Array {
        let srgb = [
            momoto_core::gamma::linear_to_srgb(r),
            momoto_core::gamma::linear_to_srgb(g),
            momoto_core::gamma::linear_to_srgb(b),
        ];
        js_sys::Float64Array::from(&srgb[..])
    }
}

// ============================================================================
// Gamut Module - sRGB Gamut Boundary Utilities
// ============================================================================

/// sRGB gamut boundary estimation utilities.
///
/// Provides fast estimation of maximum achievable chroma for any
/// lightness/hue combination within the sRGB color gamut.
#[wasm_bindgen]
pub struct GamutUtils;

#[wasm_bindgen]
impl GamutUtils {
    /// Estimate maximum chroma for given lightness and hue.
    ///
    /// Uses parabolic approximation for fast gamut boundary estimation.
    ///
    /// # Arguments
    /// * `l` - Lightness (0.0 to 1.0)
    /// * `h` - Hue (0.0 to 360.0 degrees)
    ///
    /// # Returns
    /// Estimated maximum chroma that stays within sRGB gamut
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// const maxChroma = GamutUtils.estimateMaxChroma(0.5, 180.0); // Cyan at mid-L
    /// console.log(maxChroma); // ~0.06
    /// ```
    #[wasm_bindgen(js_name = estimateMaxChroma)]
    pub fn estimate_max_chroma(l: f64, h: f64) -> f64 {
        let oklch = CoreOKLCH::new(l, 0.1, h);
        oklch.estimate_max_chroma()
    }

    /// Check if OKLCH color is approximately within sRGB gamut.
    ///
    /// # Arguments
    /// * `l` - Lightness (0.0 to 1.0)
    /// * `c` - Chroma
    /// * `h` - Hue (0.0 to 360.0 degrees)
    ///
    /// # Returns
    /// true if color is within sRGB gamut (with 10% tolerance)
    #[wasm_bindgen(js_name = isInGamut)]
    pub fn is_in_gamut(l: f64, c: f64, h: f64) -> bool {
        CoreOKLCH::new(l, c, h).is_in_gamut()
    }

    /// Map OKLCH color to sRGB gamut by reducing chroma.
    ///
    /// Preserves lightness and hue while finding maximum achievable chroma.
    ///
    /// # Arguments
    /// * `l` - Lightness (0.0 to 1.0)
    /// * `c` - Chroma
    /// * `h` - Hue (0.0 to 360.0 degrees)
    ///
    /// # Returns
    /// OKLCH color with chroma reduced to fit within sRGB gamut
    #[wasm_bindgen(js_name = mapToGamut)]
    pub fn map_to_gamut(l: f64, c: f64, h: f64) -> OKLCH {
        let oklch = CoreOKLCH::new(l, c, h).map_to_gamut();
        OKLCH { inner: oklch }
    }
}

// ============================================================================
// Luminance Module - Relative Luminance Utilities
// ============================================================================

/// Relative luminance calculation utilities.
///
/// Provides luminance calculations for both WCAG (sRGB) and APCA methods.
#[wasm_bindgen]
pub struct LuminanceUtils;

#[wasm_bindgen]
impl LuminanceUtils {
    /// Calculate relative luminance using WCAG/sRGB coefficients.
    ///
    /// Uses ITU-R BT.709 coefficients: 0.2126 R + 0.7152 G + 0.0722 B
    ///
    /// # Arguments
    /// * `color` - The color to calculate luminance for
    ///
    /// # Returns
    /// Relative luminance (0.0 to 1.0)
    #[wasm_bindgen(js_name = relativeLuminanceSrgb)]
    pub fn relative_luminance_srgb(color: &Color) -> f64 {
        momoto_core::luminance::relative_luminance_srgb(&color.inner).value()
    }

    /// Calculate relative luminance using APCA coefficients.
    ///
    /// Uses APCA-specific coefficients for better perceptual accuracy.
    ///
    /// # Arguments
    /// * `color` - The color to calculate luminance for
    ///
    /// # Returns
    /// Relative luminance (0.0 to 1.0)
    #[wasm_bindgen(js_name = relativeLuminanceApca)]
    pub fn relative_luminance_apca(color: &Color) -> f64 {
        momoto_core::luminance::relative_luminance_apca(&color.inner).value()
    }
}

// ============================================================================
// Phase 4: Workflow, Session, and Report WASM Bindings
// ============================================================================

/// Execute a workflow from JSON specification.
///
/// # Arguments
/// * `workflow_json` - JSON string with workflow name or inline definition
/// * `input_json` - JSON string with colors, pairs, and backgrounds
///
/// # Returns
/// JSON string with workflow execution results
#[wasm_bindgen(js_name = executeWorkflow)]
pub fn execute_workflow(workflow_json: &str, input_json: &str) -> String {
    use momoto_agent::{
        query::{WorkflowInputSpec, WorkflowOptions, WorkflowSpec},
        AgentExecutor, Query,
    };

    // Parse workflow spec
    let workflow: WorkflowSpec = match serde_json::from_str(workflow_json) {
        Ok(w) => w,
        Err(e) => {
            return serde_json::json!({
                "error": format!("Invalid workflow specification: {}", e)
            })
            .to_string()
        }
    };

    // Parse input
    let input: WorkflowInputSpec = match serde_json::from_str(input_json) {
        Ok(i) => i,
        Err(e) => {
            return serde_json::json!({
                "error": format!("Invalid input specification: {}", e)
            })
            .to_string()
        }
    };

    // Execute
    let executor = AgentExecutor::new();
    let query = Query::ExecuteWorkflow {
        workflow,
        input,
        options: WorkflowOptions::default(),
    };

    match executor.execute(query) {
        momoto_agent::Response::Json(v) => v.to_string(),
        momoto_agent::Response::Error(e) => serde_json::json!({
            "error": e.message
        })
        .to_string(),
        other => serde_json::to_string(&other).unwrap_or_default(),
    }
}

/// Create a new session and return its ID.
///
/// # Arguments
/// * `context_json` - Optional JSON string with initial session context
///
/// # Returns
/// JSON string with session ID and status
#[wasm_bindgen(js_name = createSession)]
pub fn create_session(context_json: Option<String>) -> String {
    use momoto_agent::{SessionContext, SessionManager};

    let manager = SessionManager::default_manager();

    let context = context_json.and_then(|json| serde_json::from_str::<SessionContext>(&json).ok());

    let session_id = manager.create_session(context);

    serde_json::json!({
        "session_id": session_id,
        "status": "created"
    })
    .to_string()
}

/// Execute a query within a session context.
///
/// # Arguments
/// * `session_id` - The session ID to use
/// * `query_json` - JSON string with the query to execute
///
/// # Returns
/// JSON string with query results
#[wasm_bindgen(js_name = executeWithSession)]
pub fn execute_with_session(session_id: &str, query_json: &str) -> String {
    use momoto_agent::{AgentExecutor, Query};

    // Parse query
    let query: Query = match serde_json::from_str(query_json) {
        Ok(q) => q,
        Err(e) => {
            return serde_json::json!({
                "error": format!("Invalid query: {}", e)
            })
            .to_string()
        }
    };

    // Execute (session context would be loaded from manager in full impl)
    let executor = AgentExecutor::new();
    let session_query = Query::SessionQuery {
        session_id: session_id.to_string(),
        query: Box::new(query),
    };

    match executor.execute(session_query) {
        momoto_agent::Response::Json(v) => v.to_string(),
        momoto_agent::Response::Error(e) => serde_json::json!({
            "error": e.message
        })
        .to_string(),
        other => serde_json::to_string(&other).unwrap_or_default(),
    }
}

/// Generate a report from analysis data.
///
/// # Arguments
/// * `report_type` - Report type: comprehensive, accessibility, quality, physics
/// * `input_json` - JSON string with colors and pairs to analyze
/// * `format` - Output format: json, markdown, html
///
/// # Returns
/// Generated report content
#[wasm_bindgen(js_name = generateReport)]
pub fn generate_report(report_type: &str, input_json: &str, format: &str) -> String {
    use momoto_agent::{query::ReportInputSpec, AgentExecutor, Query};

    // Parse input
    let input: ReportInputSpec = match serde_json::from_str(input_json) {
        Ok(i) => i,
        Err(e) => {
            return serde_json::json!({
                "error": format!("Invalid input specification: {}", e)
            })
            .to_string()
        }
    };

    // Execute
    let executor = AgentExecutor::new();
    let query = Query::GenerateReport {
        report_type: report_type.to_string(),
        input,
        format: format.to_string(),
    };

    match executor.execute(query) {
        momoto_agent::Response::Json(v) => {
            // Extract the content field if present
            if let Some(content) = v.get("content").and_then(|c| c.as_str()) {
                content.to_string()
            } else {
                v.to_string()
            }
        }
        momoto_agent::Response::Error(e) => serde_json::json!({
            "error": e.message
        })
        .to_string(),
        other => serde_json::to_string(&other).unwrap_or_default(),
    }
}

/// List available preset workflows.
///
/// # Returns
/// JSON array of workflow names and descriptions
#[wasm_bindgen(js_name = listWorkflows)]
pub fn list_workflows() -> String {
    let workflows = momoto_agent::list_preset_workflows();
    serde_json::json!(workflows).to_string()
}

// ============================================================================
// Module initialization
// ============================================================================

#[wasm_bindgen(start)]
pub fn init() {
    // Set up better panic messages for debugging
    #[cfg(feature = "panic_hook")]
    console_error_panic_hook::set_once();
}

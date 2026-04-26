// =============================================================================
// momoto-wasm: Core & Metrics Additions
// File: crates/momoto-wasm/src/core_ext.rs
//
// Exposes missing items from momoto-core and momoto-metrics.
// =============================================================================

use momoto_core::{
    color::Color as CoreColor,
    evaluated::LinearRgba as CoreLinearRgba,
    luminance::{self as lum, RelativeLuminance as CoreRelativeLuminance},
    space::oklch::{OKLab as CoreOKLab, OKLCH as CoreOKLCH},
};
use momoto_metrics::wcag::{
    TextSize as CoreTextSize, WCAGLevel as CoreWCAGLevel, WCAGMetric as CoreWCAGMetric,
    WCAG_REQUIREMENTS,
};
use wasm_bindgen::prelude::*;

// =============================================================================
// OKLab (Cartesian Perceptual Space)
// =============================================================================

#[wasm_bindgen]
pub struct OKLab {
    inner: CoreOKLab,
}

#[wasm_bindgen]
impl OKLab {
    /// Create from Lightness, a (green-red), b (blue-yellow).
    #[wasm_bindgen(constructor)]
    pub fn new(l: f64, a: f64, b: f64) -> Self {
        Self {
            inner: CoreOKLab::new(l, a, b),
        }
    }

    /// Convert from Color.
    #[wasm_bindgen(js_name = "fromColor")]
    pub fn from_color(color: &super::Color) -> Self {
        Self {
            inner: CoreOKLab::from_color(&color.to_core()),
        }
    }

    /// Convert to Color (sRGB).
    #[wasm_bindgen(js_name = "toColor")]
    pub fn to_color(&self) -> super::Color {
        super::Color::from_core(self.inner.to_color())
    }

    /// Convert to OKLCH (cylindrical).
    #[wasm_bindgen(js_name = "toOklch")]
    pub fn to_oklch(&self) -> super::OKLCH {
        let oklch = CoreOKLCH::new(
            self.inner.l,
            (self.inner.a * self.inner.a + self.inner.b * self.inner.b).sqrt(),
            self.inner
                .b
                .atan2(self.inner.a)
                .to_degrees()
                .rem_euclid(360.0),
        );
        super::OKLCH::from_core(oklch)
    }

    #[wasm_bindgen(getter)]
    pub fn l(&self) -> f64 {
        self.inner.l
    }

    #[wasm_bindgen(getter)]
    pub fn a(&self) -> f64 {
        self.inner.a
    }

    #[wasm_bindgen(getter)]
    pub fn b(&self) -> f64 {
        self.inner.b
    }

    /// Linear interpolation in OKLab space.
    #[wasm_bindgen]
    pub fn interpolate(from: &OKLab, to: &OKLab, t: f64) -> OKLab {
        OKLab {
            inner: CoreOKLab::new(
                from.inner.l + (to.inner.l - from.inner.l) * t,
                from.inner.a + (to.inner.a - from.inner.a) * t,
                from.inner.b + (to.inner.b - from.inner.b) * t,
            ),
        }
    }

    /// Euclidean distance in OKLab.
    #[wasm_bindgen(js_name = "deltaE")]
    pub fn delta_e(&self, other: &OKLab) -> f64 {
        let dl = self.inner.l - other.inner.l;
        let da = self.inner.a - other.inner.a;
        let db = self.inner.b - other.inner.b;
        (dl * dl + da * da + db * db).sqrt()
    }
}

// =============================================================================
// LinearRgba
// =============================================================================

#[wasm_bindgen]
pub struct LinearRgba {
    inner: CoreLinearRgba,
}

#[wasm_bindgen]
impl LinearRgba {
    #[wasm_bindgen(constructor)]
    pub fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self {
            inner: CoreLinearRgba::new(r, g, b, a),
        }
    }

    /// Create from OKLCH color with alpha.
    #[wasm_bindgen(js_name = "fromOklch")]
    pub fn from_oklch(oklch: &super::OKLCH, alpha: f64) -> Self {
        Self {
            inner: CoreLinearRgba::from_oklch(oklch.to_core_oklch(), alpha),
        }
    }

    /// Create opaque from linear RGB.
    #[wasm_bindgen(js_name = "rgb")]
    pub fn rgb(r: f64, g: f64, b: f64) -> Self {
        Self {
            inner: CoreLinearRgba::rgb(r, g, b),
        }
    }

    #[wasm_bindgen(getter)]
    pub fn r(&self) -> f64 {
        self.inner.r
    }

    #[wasm_bindgen(getter)]
    pub fn g(&self) -> f64 {
        self.inner.g
    }

    #[wasm_bindgen(getter)]
    pub fn b(&self) -> f64 {
        self.inner.b
    }

    #[wasm_bindgen(getter, js_name = "a")]
    pub fn alpha(&self) -> f64 {
        self.inner.a
    }
}

// =============================================================================
// Luminance Functions
// =============================================================================

/// Calculate WCAG 2.1 relative luminance.
#[wasm_bindgen(js_name = "relativeLuminanceSrgb")]
pub fn relative_luminance_srgb(color: &super::Color) -> f64 {
    lum::relative_luminance_srgb(&color.to_core()).0
}

/// Calculate APCA relative luminance.
#[wasm_bindgen(js_name = "relativeLuminanceApca")]
pub fn relative_luminance_apca(color: &super::Color) -> f64 {
    lum::relative_luminance_apca(&color.to_core()).0
}

/// APCA soft-clamp function.
#[wasm_bindgen(js_name = "softClamp")]
pub fn soft_clamp(y: f64, threshold: f64, exponent: f64) -> f64 {
    lum::soft_clamp(CoreRelativeLuminance(y), threshold, exponent).0
}

/// sRGB gamma decoding.
#[wasm_bindgen(js_name = "srgbToLinear")]
pub fn srgb_to_linear(value: f64) -> f64 {
    momoto_core::color::gamma::srgb_to_linear(value)
}

/// sRGB gamma encoding.
#[wasm_bindgen(js_name = "linearToSrgb")]
pub fn linear_to_srgb(value: f64) -> f64 {
    momoto_core::color::gamma::linear_to_srgb(value)
}

/// Linear interpolation.
#[wasm_bindgen(js_name = "mathLerp")]
pub fn math_lerp(a: f64, b: f64, t: f64) -> f64 {
    momoto_core::math::lerp(a, b, t)
}

/// Inverse linear interpolation.
#[wasm_bindgen(js_name = "mathInverseLerp")]
pub fn math_inverse_lerp(a: f64, b: f64, value: f64) -> f64 {
    momoto_core::math::inverse_lerp(a, b, value)
}

// =============================================================================
// WCAG Helpers
// =============================================================================

/// Check if a contrast ratio passes a specific WCAG level.
#[wasm_bindgen(js_name = "wcagPasses")]
pub fn wcag_passes(ratio: f64, level: u8, is_large: bool) -> bool {
    let lvl = match level {
        0 => CoreWCAGLevel::AA,
        _ => CoreWCAGLevel::AAA,
    };
    let size = if is_large {
        CoreTextSize::Large
    } else {
        CoreTextSize::Normal
    };
    CoreWCAGMetric::passes(ratio, lvl, size)
}

/// Determine the highest WCAG level achieved.
#[wasm_bindgen(js_name = "wcagLevel")]
pub fn wcag_level(ratio: f64, is_large: bool) -> u8 {
    let size = if is_large {
        CoreTextSize::Large
    } else {
        CoreTextSize::Normal
    };
    match CoreWCAGMetric::level(ratio, size) {
        Some(CoreWCAGLevel::AA) => 1,
        Some(CoreWCAGLevel::AAA) => 2,
        None => 0,
    }
}

/// Check if text qualifies as "large text" per WCAG.
#[wasm_bindgen(js_name = "isLargeText")]
pub fn is_large_text(font_size_px: f64, font_weight: u16) -> bool {
    CoreWCAGMetric::is_large_text(font_size_px, font_weight)
}

/// Get the minimum required contrast ratio for a WCAG level + text size.
#[wasm_bindgen(js_name = "wcagRequirement")]
pub fn wcag_requirement(level: u8, is_large: bool) -> f64 {
    let lvl = match level {
        0 => CoreWCAGLevel::AA,
        _ => CoreWCAGLevel::AAA,
    };
    let size = if is_large {
        CoreTextSize::Large
    } else {
        CoreTextSize::Normal
    };
    lvl.requirement(size)
}

/// Get WCAG requirements matrix as flat array.
#[wasm_bindgen(js_name = "wcagRequirementsMatrix")]
pub fn wcag_requirements_matrix() -> Box<[f64]> {
    Box::new([
        WCAG_REQUIREMENTS[0][0],
        WCAG_REQUIREMENTS[0][1],
        WCAG_REQUIREMENTS[1][0],
        WCAG_REQUIREMENTS[1][1],
    ])
}

/// Calculate WCAG contrast ratio directly from two colors.
#[wasm_bindgen(js_name = "wcagContrastRatio")]
pub fn wcag_contrast_ratio(fg: &super::Color, bg: &super::Color) -> f64 {
    let fg_lum = lum::relative_luminance_srgb(&fg.to_core()).0;
    let bg_lum = lum::relative_luminance_srgb(&bg.to_core()).0;
    let lighter = fg_lum.max(bg_lum);
    let darker = fg_lum.min(bg_lum);
    (lighter + 0.05) / (darker + 0.05)
}

// =============================================================================
// APCA Constants
// =============================================================================

/// Get APCA algorithm constants as JSON.
#[wasm_bindgen(js_name = "apcaConstants")]
pub fn apca_constants() -> Result<JsValue, JsValue> {
    let constants = serde_json::json!({
        "mainTrc": 2.4,
        "sRco": 0.2126729,
        "sGco": 0.7151522,
        "sBco": 0.0721750,
        "blkThrs": 0.022,
        "blkClmp": 1.414,
        "normBg": 0.56,
        "normTxt": 0.57,
        "revBg": 0.65,
        "revTxt": 0.62,
        "scaleBow": 1.14,
        "scaleWob": 1.14,
        "loBowOffset": 0.027,
        "loWobOffset": 0.027,
        "loClip": 0.1,
        "deltaYMin": 0.0005,
    });
    Ok(serde_wasm_bindgen::to_value(&constants).map_err(|e| JsValue::from_str(&e.to_string()))?)
}

// =============================================================================
// CssRenderConfig
// =============================================================================

#[wasm_bindgen]
pub struct CssRenderConfig {
    inner: momoto_core::backend::css_config::CssRenderConfig,
}

#[wasm_bindgen]
impl CssRenderConfig {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: momoto_core::backend::css_config::CssRenderConfig::new(),
        }
    }

    #[wasm_bindgen]
    pub fn minimal() -> Self {
        Self {
            inner: momoto_core::backend::css_config::CssRenderConfig::minimal(),
        }
    }

    #[wasm_bindgen]
    pub fn premium() -> Self {
        Self {
            inner: momoto_core::backend::css_config::CssRenderConfig::premium(),
        }
    }

    #[wasm_bindgen]
    pub fn modal() -> Self {
        Self {
            inner: momoto_core::backend::css_config::CssRenderConfig::modal(),
        }
    }

    #[wasm_bindgen]
    pub fn subtle() -> Self {
        Self {
            inner: momoto_core::backend::css_config::CssRenderConfig::subtle(),
        }
    }

    #[wasm_bindgen(js_name = "darkMode")]
    pub fn dark_mode() -> Self {
        Self {
            inner: momoto_core::backend::css_config::CssRenderConfig::dark_mode(),
        }
    }

    #[wasm_bindgen(js_name = "withSpecularIntensity")]
    pub fn with_specular_intensity(mut self, intensity: f64) -> Self {
        self.inner = self.inner.with_specular_intensity(intensity);
        self
    }

    #[wasm_bindgen(js_name = "withFresnelIntensity")]
    pub fn with_fresnel_intensity(mut self, intensity: f64) -> Self {
        self.inner = self.inner.with_fresnel_intensity(intensity);
        self
    }

    #[wasm_bindgen(js_name = "withElevation")]
    pub fn with_elevation(mut self, level: u8) -> Self {
        self.inner = self.inner.with_elevation(level);
        self
    }

    #[wasm_bindgen(js_name = "withBorderRadius")]
    pub fn with_border_radius(mut self, radius: f64) -> Self {
        self.inner = self.inner.with_border_radius(radius);
        self
    }

    #[wasm_bindgen(js_name = "withLightMode")]
    pub fn with_light_mode(mut self, light_mode: bool) -> Self {
        self.inner = self.inner.with_light_mode(light_mode);
        self
    }

    #[wasm_bindgen(js_name = "withEffectsEnabled")]
    pub fn with_effects_enabled(mut self, enabled: bool) -> Self {
        self.inner = self.inner.with_effects_enabled(enabled);
        self
    }

    #[wasm_bindgen(js_name = "toJson")]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let c = &self.inner;
        let json = serde_json::json!({
            "specular_enabled": c.specular_enabled,
            "specular_intensity": c.specular_intensity,
            "specular_size": c.specular_size,
            "specular_position": [c.specular_position.0, c.specular_position.1],
            "fresnel_enabled": c.fresnel_enabled,
            "fresnel_intensity": c.fresnel_intensity,
            "fresnel_edge_power": c.fresnel_edge_power,
            "inner_highlight_enabled": c.inner_highlight_enabled,
            "inner_highlight_intensity": c.inner_highlight_intensity,
            "elevation": c.elevation,
            "shadow_color_tint": c.shadow_color_tint,
            "saturate": c.saturate,
            "saturation_factor": c.saturation_factor,
            "border_enabled": c.border_enabled,
            "border_radius": c.border_radius,
            "light_mode": c.light_mode,
        });
        Ok(serde_wasm_bindgen::to_value(&json).map_err(|e| JsValue::from_str(&e.to_string()))?)
    }
}

impl CssRenderConfig {
    pub(crate) fn to_core(&self) -> &momoto_core::backend::css_config::CssRenderConfig {
        &self.inner
    }
}

// =============================================================================
// Enums
// =============================================================================

#[wasm_bindgen]
pub enum ColorSpaceEnum {
    SRgb = 0,
    DisplayP3 = 1,
    Rec2020 = 2,
    LinearRgb = 3,
}

#[wasm_bindgen]
pub enum TargetMediumEnum {
    Screen = 0,
    Print = 1,
    Projection = 2,
}

#[wasm_bindgen]
pub enum AccessibilityModeEnum {
    HighContrast = 0,
    ReducedMotion = 1,
    ReducedTransparency = 2,
    InvertedColors = 3,
}

#[wasm_bindgen]
pub enum MaterialTypeEnum {
    Glass = 0,
    Metal = 1,
    Plastic = 2,
    Liquid = 3,
    Custom = 4,
}

// =============================================================================
// EvaluatedMaterial extensions
// =============================================================================

/// Check if an evaluated material is transparent.
#[wasm_bindgen(js_name = "materialIsTransparent")]
pub fn material_is_transparent(material: &super::EvaluatedMaterial) -> bool {
    material.to_core().is_transparent()
}

/// Check if an evaluated material has subsurface scattering.
#[wasm_bindgen(js_name = "materialHasScattering")]
pub fn material_has_scattering(material: &super::EvaluatedMaterial) -> bool {
    material.to_core().has_scattering()
}

/// Check if an evaluated material is emissive.
#[wasm_bindgen(js_name = "materialIsEmissive")]
pub fn material_is_emissive(material: &super::EvaluatedMaterial) -> bool {
    material.to_core().is_emissive()
}

/// Get effective specular intensity.
#[wasm_bindgen(js_name = "materialEffectiveSpecular")]
pub fn material_effective_specular(material: &super::EvaluatedMaterial) -> f64 {
    material.to_core().effective_specular()
}

// =============================================================================
// Batch: Luminance for multiple colors
// =============================================================================

/// Calculate WCAG relative luminance for multiple colors.
#[wasm_bindgen(js_name = "relativeLuminanceBatch")]
pub fn relative_luminance_batch(rgb_data: &[u8]) -> Result<Box<[f64]>, JsValue> {
    if rgb_data.len() % 3 != 0 {
        return Err(JsValue::from_str(
            "Input must be multiple of 3: [r, g, b, ...]",
        ));
    }
    let count = rgb_data.len() / 3;
    let mut results = Vec::with_capacity(count);
    for i in 0..count {
        let base = i * 3;
        let color = CoreColor::from_srgb8(rgb_data[base], rgb_data[base + 1], rgb_data[base + 2]);
        results.push(lum::relative_luminance_srgb(&color).0);
    }
    Ok(results.into_boxed_slice())
}

/// Calculate WCAG contrast ratios for multiple pairs.
#[wasm_bindgen(js_name = "wcagContrastRatioBatch")]
pub fn wcag_contrast_ratio_batch(pairs: &[u8]) -> Result<Box<[f64]>, JsValue> {
    if pairs.len() % 6 != 0 {
        return Err(JsValue::from_str(
            "Input must be multiple of 6: [fg_r, fg_g, fg_b, bg_r, bg_g, bg_b, ...]",
        ));
    }
    let count = pairs.len() / 6;
    let mut results = Vec::with_capacity(count);
    for i in 0..count {
        let base = i * 6;
        let fg = CoreColor::from_srgb8(pairs[base], pairs[base + 1], pairs[base + 2]);
        let bg = CoreColor::from_srgb8(pairs[base + 3], pairs[base + 4], pairs[base + 5]);
        let fg_lum = lum::relative_luminance_srgb(&fg).0;
        let bg_lum = lum::relative_luminance_srgb(&bg).0;
        let lighter = fg_lum.max(bg_lum);
        let darker = fg_lum.min(bg_lum);
        results.push((lighter + 0.05) / (darker + 0.05));
    }
    Ok(results.into_boxed_slice())
}

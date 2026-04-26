// =============================================================================
// momoto-wasm: Materials Extended Bindings
// File: crates/momoto-wasm/src/materials_ext.rs
//
// Exposes missing items from momoto-materials NOT already in lib.rs.
// =============================================================================

use serde_wasm_bindgen;
use wasm_bindgen::prelude::*;

// =============================================================================
// REFRACTION SYSTEM
// =============================================================================

use momoto_materials::glass_physics::refraction_index::{
    apply_refraction_to_color as core_apply_refraction_to_color,
    calculate_refraction as core_calculate_refraction,
    generate_distortion_map as core_generate_distortion_map,
    RefractionParams as CoreRefractionParams, RefractionPresets as CoreRefractionPresets,
};

#[wasm_bindgen]
pub struct RefractionParams {
    inner: CoreRefractionParams,
}

#[wasm_bindgen]
impl RefractionParams {
    #[wasm_bindgen(constructor)]
    pub fn new(
        index: f64,
        distortion_strength: f64,
        chromatic_aberration: f64,
        edge_lensing: f64,
    ) -> Self {
        Self {
            inner: CoreRefractionParams {
                index,
                distortion_strength,
                chromatic_aberration,
                edge_lensing,
            },
        }
    }

    pub fn clear() -> Self {
        Self {
            inner: CoreRefractionPresets::clear(),
        }
    }

    pub fn frosted() -> Self {
        Self {
            inner: CoreRefractionPresets::frosted(),
        }
    }

    pub fn thick() -> Self {
        Self {
            inner: CoreRefractionPresets::thick(),
        }
    }

    pub fn subtle() -> Self {
        Self {
            inner: CoreRefractionPresets::subtle(),
        }
    }

    #[wasm_bindgen(js_name = "highIndex")]
    pub fn high_index() -> Self {
        Self {
            inner: CoreRefractionPresets::high_index(),
        }
    }

    #[wasm_bindgen(getter)]
    pub fn index(&self) -> f64 {
        self.inner.index
    }

    #[wasm_bindgen(getter, js_name = "distortionStrength")]
    pub fn distortion_strength(&self) -> f64 {
        self.inner.distortion_strength
    }

    #[wasm_bindgen(getter, js_name = "chromaticAberration")]
    pub fn chromatic_aberration(&self) -> f64 {
        self.inner.chromatic_aberration
    }

    #[wasm_bindgen(getter, js_name = "edgeLensing")]
    pub fn edge_lensing(&self) -> f64 {
        self.inner.edge_lensing
    }
}

/// Calculate refraction at a position with incident angle. Returns [offset_x, offset_y, hue_shift, brightness_factor].
#[wasm_bindgen(js_name = "calculateRefraction")]
pub fn calculate_refraction(
    params: &RefractionParams,
    x: f64,
    y: f64,
    incident_angle: f64,
) -> Box<[f64]> {
    let result = core_calculate_refraction(&params.inner, x, y, incident_angle);
    Box::new([
        result.offset_x,
        result.offset_y,
        result.hue_shift,
        result.brightness_factor,
    ])
}

/// Apply refraction correction to an OKLCH color.
#[wasm_bindgen(js_name = "applyRefractionToColor")]
pub fn apply_refraction_to_color(
    params: &RefractionParams,
    l: f64,
    c: f64,
    h: f64,
    x: f64,
    y: f64,
    incident_angle: f64,
) -> Box<[f64]> {
    use momoto_core::space::oklch::OKLCH as CoreOKLCH;
    let color = CoreOKLCH { l, c, h };
    let refraction = core_calculate_refraction(&params.inner, x, y, incident_angle);
    let result = core_apply_refraction_to_color(color, &refraction);
    Box::new([result.l, result.c, result.h])
}

/// Generate a distortion map grid. Returns flat array [offset_x, offset_y, hue_shift, brightness, ...].
#[wasm_bindgen(js_name = "generateDistortionMap")]
pub fn generate_distortion_map(
    params: &RefractionParams,
    grid_size: usize,
) -> Result<Box<[f64]>, JsValue> {
    let map = core_generate_distortion_map(&params.inner, grid_size);
    let mut flat = Vec::with_capacity(grid_size * grid_size * 4);
    for row in &map {
        for result in row {
            flat.push(result.offset_x);
            flat.push(result.offset_y);
            flat.push(result.hue_shift);
            flat.push(result.brightness_factor);
        }
    }
    Ok(flat.into_boxed_slice())
}

// =============================================================================
// LIGHTING MODEL
// =============================================================================

use momoto_materials::glass_physics::light_model::{
    calculate_lighting as core_calculate_lighting, derive_gradient as core_derive_gradient,
    gradient_to_css as core_gradient_to_css, LightSource as CoreLightSource,
    LightingEnvironment as CoreLightingEnvironment, Vec3 as CoreVec3,
};

#[wasm_bindgen]
pub struct LightSource {
    inner: CoreLightSource,
}

#[wasm_bindgen]
impl LightSource {
    /// Create a light source. Color is specified as OKLCH (l, c, h).
    #[wasm_bindgen(constructor)]
    pub fn new(
        dir_x: f64,
        dir_y: f64,
        dir_z: f64,
        intensity: f64,
        color_l: f64,
        color_c: f64,
        color_h: f64,
    ) -> Self {
        use momoto_core::space::oklch::OKLCH as CoreOKLCH;
        Self {
            inner: CoreLightSource {
                direction: CoreVec3::new(dir_x, dir_y, dir_z),
                intensity,
                color: CoreOKLCH {
                    l: color_l,
                    c: color_c,
                    h: color_h,
                },
            },
        }
    }

    #[wasm_bindgen(js_name = "defaultKeyLight")]
    pub fn default_key_light() -> Self {
        Self {
            inner: CoreLightSource::default_key_light(),
        }
    }

    #[wasm_bindgen(js_name = "defaultFillLight")]
    pub fn default_fill_light() -> Self {
        Self {
            inner: CoreLightSource::default_fill_light(),
        }
    }

    #[wasm_bindgen(js_name = "dramaticTopLight")]
    pub fn dramatic_top_light() -> Self {
        Self {
            inner: CoreLightSource::dramatic_top_light(),
        }
    }
}

#[wasm_bindgen]
pub struct LightingEnvironment {
    inner: CoreLightingEnvironment,
}

#[wasm_bindgen]
impl LightingEnvironment {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: CoreLightingEnvironment::default(),
        }
    }
}

/// Calculate lighting for a surface. Returns lighting result as JSON.
#[wasm_bindgen(js_name = "calculateLighting")]
pub fn calculate_lighting(
    normal_x: f64,
    normal_y: f64,
    normal_z: f64,
    view_x: f64,
    view_y: f64,
    view_z: f64,
    env: &LightingEnvironment,
    shininess: f64,
) -> Result<JsValue, JsValue> {
    let normal = CoreVec3::new(normal_x, normal_y, normal_z);
    let view = CoreVec3::new(view_x, view_y, view_z);
    let result = core_calculate_lighting(&normal, &view, &env.inner, shininess);
    serde_wasm_bindgen::to_value(&serde_json::json!({
        "diffuse": result.diffuse,
        "specular": result.specular,
        "total": result.total,
    }))
    .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Derive a gradient from a lighting environment. Returns JSON array.
#[wasm_bindgen(js_name = "deriveGradient")]
pub fn derive_gradient(
    env: &LightingEnvironment,
    surface_curvature: f64,
    shininess: f64,
    samples: usize,
) -> Result<JsValue, JsValue> {
    let gradient = core_derive_gradient(&env.inner, surface_curvature, shininess, samples);
    let json_arr: Vec<serde_json::Value> = gradient
        .iter()
        .map(|r| {
            serde_json::json!({
                "diffuse": r.diffuse,
                "specular": r.specular,
                "total": r.total,
            })
        })
        .collect();
    serde_wasm_bindgen::to_value(&json_arr).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Convert a lighting gradient to CSS stops. Returns JSON array of [{position, l, c, h}, ...].
#[wasm_bindgen(js_name = "gradientToCss")]
pub fn gradient_to_css(
    env: &LightingEnvironment,
    surface_curvature: f64,
    shininess: f64,
    samples: usize,
    base_l: f64,
    base_c: f64,
    base_h: f64,
) -> Result<JsValue, JsValue> {
    use momoto_core::space::oklch::OKLCH as CoreOKLCH;
    let gradient = core_derive_gradient(&env.inner, surface_curvature, shininess, samples);
    let base_color = CoreOKLCH {
        l: base_l,
        c: base_c,
        h: base_h,
    };
    let stops = core_gradient_to_css(&gradient, base_color);
    let json: Vec<serde_json::Value> = stops
        .iter()
        .map(|(pos, color)| {
            serde_json::json!({
                "position": pos,
                "l": color.l,
                "c": color.c,
                "h": color.h,
            })
        })
        .collect();
    serde_wasm_bindgen::to_value(&json).map_err(|e| JsValue::from_str(&e.to_string()))
}

// =============================================================================
// AMBIENT SHADOWS
// =============================================================================

use momoto_materials::shadow_engine::ambient_shadow::to_css as ambient_to_css;
use momoto_materials::shadow_engine::elevation_shadow::to_css as elevation_to_css;
use momoto_materials::shadow_engine::{
    calculate_ambient_shadow as core_calculate_ambient_shadow,
    calculate_interactive_shadow as core_interactive_shadow,
    calculate_multi_scale_ambient as core_multi_scale_ambient,
    AmbientShadowParams as CoreAmbientShadowParams,
    AmbientShadowPresets as CoreAmbientShadowPresets, ElevationPresets as CoreElevationPresets,
    ElevationTransition as CoreElevationTransition, InteractiveState as CoreInteractiveState,
};

#[wasm_bindgen]
pub struct AmbientShadowParams {
    inner: CoreAmbientShadowParams,
}

#[wasm_bindgen]
impl AmbientShadowParams {
    #[wasm_bindgen(constructor)]
    pub fn new(base_opacity: f64, blur_radius: f64, offset_y: f64, spread: f64) -> Self {
        Self {
            inner: CoreAmbientShadowParams {
                base_opacity,
                blur_radius,
                offset_y,
                spread,
                color_tint: None,
            },
        }
    }

    pub fn standard() -> Self {
        Self {
            inner: CoreAmbientShadowPresets::standard(),
        }
    }

    pub fn elevated() -> Self {
        Self {
            inner: CoreAmbientShadowPresets::elevated(),
        }
    }

    pub fn subtle() -> Self {
        Self {
            inner: CoreAmbientShadowPresets::subtle(),
        }
    }

    pub fn dramatic() -> Self {
        Self {
            inner: CoreAmbientShadowPresets::dramatic(),
        }
    }
}

/// Calculate ambient shadow CSS string.
#[wasm_bindgen(js_name = "calculateAmbientShadow")]
pub fn calculate_ambient_shadow(
    params: &AmbientShadowParams,
    bg_l: f64,
    bg_c: f64,
    bg_h: f64,
    elevation: f64,
) -> String {
    use momoto_core::space::oklch::OKLCH as CoreOKLCH;
    let background = CoreOKLCH {
        l: bg_l,
        c: bg_c,
        h: bg_h,
    };
    let shadow = core_calculate_ambient_shadow(&params.inner, background, elevation);
    ambient_to_css(&shadow)
}

/// Calculate multi-scale ambient shadows (multiple layers). Returns comma-separated CSS.
#[wasm_bindgen(js_name = "calculateMultiScaleAmbient")]
pub fn calculate_multi_scale_ambient(
    params: &AmbientShadowParams,
    bg_l: f64,
    bg_c: f64,
    bg_h: f64,
    elevation: f64,
) -> String {
    use momoto_core::space::oklch::OKLCH as CoreOKLCH;
    let background = CoreOKLCH {
        l: bg_l,
        c: bg_c,
        h: bg_h,
    };
    let shadows = core_multi_scale_ambient(&params.inner, background, elevation);
    shadows
        .iter()
        .map(|s| ambient_to_css(s))
        .collect::<Vec<_>>()
        .join(", ")
}

// =============================================================================
// INTERACTIVE SHADOWS
// =============================================================================

#[wasm_bindgen]
pub struct ElevationTransition {
    inner: CoreElevationTransition,
}

#[wasm_bindgen]
impl ElevationTransition {
    /// Create from elevation dp values (raw u8).
    #[wasm_bindgen(constructor)]
    pub fn new(rest: u8, hover: u8, active: u8, focus: u8) -> Self {
        Self {
            inner: CoreElevationTransition {
                rest,
                hover,
                active,
                focus,
            },
        }
    }

    pub fn card() -> Self {
        Self {
            inner: CoreElevationTransition {
                rest: CoreElevationPresets::LEVEL_1,
                hover: CoreElevationPresets::LEVEL_4,
                active: CoreElevationPresets::LEVEL_1,
                focus: CoreElevationPresets::LEVEL_3,
            },
        }
    }

    pub fn fab() -> Self {
        Self {
            inner: CoreElevationTransition {
                rest: CoreElevationPresets::LEVEL_3,
                hover: CoreElevationPresets::LEVEL_5,
                active: CoreElevationPresets::LEVEL_3,
                focus: CoreElevationPresets::LEVEL_5,
            },
        }
    }

    pub fn flat() -> Self {
        Self {
            inner: CoreElevationTransition {
                rest: CoreElevationPresets::LEVEL_0,
                hover: CoreElevationPresets::LEVEL_2,
                active: CoreElevationPresets::LEVEL_0,
                focus: CoreElevationPresets::LEVEL_1,
            },
        }
    }
}

/// Calculate interactive shadow for a given state. Returns CSS box-shadow string.
#[wasm_bindgen(js_name = "calculateInteractiveShadow")]
pub fn calculate_interactive_shadow(
    transition: &ElevationTransition,
    state: u8,
    bg_l: f64,
    bg_c: f64,
    bg_h: f64,
    glass_depth: f64,
) -> String {
    use momoto_core::space::oklch::OKLCH as CoreOKLCH;
    let interaction_state = match state {
        0 => CoreInteractiveState::Rest,
        1 => CoreInteractiveState::Hover,
        2 => CoreInteractiveState::Active,
        _ => CoreInteractiveState::Focus,
    };
    let background = CoreOKLCH {
        l: bg_l,
        c: bg_c,
        h: bg_h,
    };
    let shadow = core_interactive_shadow(
        &transition.inner,
        interaction_state,
        background,
        glass_depth,
    );
    elevation_to_css(&shadow)
}

// =============================================================================
// ELEVATION (Material Design)
// =============================================================================

use momoto_materials::elevation::Elevation as CoreElevation;

#[wasm_bindgen(js_name = "elevationDp")]
pub fn elevation_dp(level: u8) -> f64 {
    let elev = match level {
        0 => CoreElevation::Level0,
        1 => CoreElevation::Level1,
        2 => CoreElevation::Level2,
        3 => CoreElevation::Level3,
        4 => CoreElevation::Level4,
        _ => CoreElevation::Level5,
    };
    elev.dp()
}

#[wasm_bindgen(js_name = "elevationTintOpacity")]
pub fn elevation_tint_opacity(level: u8) -> f64 {
    let elev = match level {
        0 => CoreElevation::Level0,
        1 => CoreElevation::Level1,
        2 => CoreElevation::Level2,
        3 => CoreElevation::Level3,
        4 => CoreElevation::Level4,
        _ => CoreElevation::Level5,
    };
    elev.tint_opacity()
}

// =============================================================================
// SPECTRAL COHERENCE
// =============================================================================

use momoto_materials::glass_physics::spectral_coherence::{
    FlickerConfig as CoreFlickerConfig, FlickerValidator as CoreFlickerValidator,
};

#[wasm_bindgen]
pub struct FlickerValidator {
    inner: CoreFlickerValidator,
}

#[wasm_bindgen]
impl FlickerValidator {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: CoreFlickerValidator::new(CoreFlickerConfig::default()),
        }
    }

    pub fn strict() -> Self {
        Self {
            inner: CoreFlickerValidator::strict(),
        }
    }

    pub fn relaxed() -> Self {
        Self {
            inner: CoreFlickerValidator::relaxed(),
        }
    }

    #[wasm_bindgen(js_name = "withThresholds")]
    pub fn with_thresholds(stable: f64, minor: f64, warning: f64) -> Self {
        Self {
            inner: CoreFlickerValidator::new(CoreFlickerConfig {
                stable_threshold: stable,
                minor_threshold: minor,
                warning_threshold: warning,
                ..CoreFlickerConfig::default()
            }),
        }
    }
}

// =============================================================================
// TEMPORAL INTERPOLATION
// =============================================================================

use momoto_materials::glass_physics::temporal::interpolation::{
    ExponentialMovingAverage as CoreEMA, InterpolationMode as CoreInterpolationMode,
    RateLimitConfig as CoreRateLimitConfig, RateLimiter as CoreRateLimiter,
};

/// Interpolation mode enum for JS.
#[wasm_bindgen]
pub enum InterpolationModeEnum {
    Linear = 0,
    Smoothstep = 1,
    Smootherstep = 2,
    EaseInOut = 3,
    Step = 4,
}

/// Apply interpolation mode to a t value (0.0-1.0).
#[wasm_bindgen(js_name = "applyInterpolation")]
pub fn apply_interpolation(mode: u8, t: f64) -> f64 {
    let m = match mode {
        0 => CoreInterpolationMode::Linear,
        1 => CoreInterpolationMode::Smoothstep,
        2 => CoreInterpolationMode::Smootherstep,
        3 => CoreInterpolationMode::EaseInOut,
        _ => CoreInterpolationMode::Step,
    };
    m.apply(t)
}

/// Interpolate between two values using a mode.
#[wasm_bindgen(js_name = "interpolateValues")]
pub fn interpolate_values(mode: u8, a: f64, b: f64, t: f64) -> f64 {
    let m = match mode {
        0 => CoreInterpolationMode::Linear,
        1 => CoreInterpolationMode::Smoothstep,
        2 => CoreInterpolationMode::Smootherstep,
        3 => CoreInterpolationMode::EaseInOut,
        _ => CoreInterpolationMode::Step,
    };
    m.interpolate(a, b, t)
}

#[wasm_bindgen]
pub struct RateLimiter {
    inner: CoreRateLimiter,
}

#[wasm_bindgen]
impl RateLimiter {
    #[wasm_bindgen(constructor)]
    pub fn new(initial: f64, max_rate: f64, smooth: bool) -> Self {
        Self {
            inner: CoreRateLimiter::new(initial, CoreRateLimitConfig { max_rate, smooth }),
        }
    }

    #[wasm_bindgen(js_name = "setTarget")]
    pub fn set_target(&mut self, target: f64) {
        self.inner.set_target(target);
    }

    pub fn update(&mut self, time: f64) -> f64 {
        self.inner.update(time)
    }

    #[wasm_bindgen(getter)]
    pub fn current(&self) -> f64 {
        self.inner.current()
    }

    #[wasm_bindgen(getter)]
    pub fn target(&self) -> f64 {
        self.inner.target()
    }

    #[wasm_bindgen(js_name = "atTarget")]
    pub fn at_target(&self) -> bool {
        self.inner.at_target()
    }

    pub fn reset(&mut self, value: f64, time: f64) {
        self.inner.reset(value, time);
    }
}

#[wasm_bindgen]
pub struct ExponentialMovingAverage {
    inner: CoreEMA,
}

#[wasm_bindgen]
impl ExponentialMovingAverage {
    #[wasm_bindgen(constructor)]
    pub fn new(alpha: f64) -> Self {
        Self {
            inner: CoreEMA::new(alpha),
        }
    }

    pub fn update(&mut self, value: f64) -> f64 {
        self.inner.update(value)
    }

    #[wasm_bindgen(getter)]
    pub fn value(&self) -> f64 {
        self.inner.value()
    }

    pub fn reset(&mut self) {
        self.inner.reset();
    }
}

// =============================================================================
// TEMPORAL BSDF (Time-Varying Materials)
// =============================================================================

use momoto_materials::glass_physics::temporal::materials::{
    ConductorEvolution as CoreConductorEvolution, DielectricEvolution as CoreDielectricEvolution,
    TemporalConductor as CoreTemporalConductor, TemporalDielectric as CoreTemporalDielectric,
    TemporalThinFilm as CoreTemporalThinFilm, ThinFilmEvolution as CoreThinFilmEvolution,
};

#[wasm_bindgen]
pub struct TemporalDielectric {
    inner: CoreTemporalDielectric,
}

#[wasm_bindgen]
impl TemporalDielectric {
    /// Create with drying paint preset.
    #[wasm_bindgen(js_name = "dryingPaint")]
    pub fn drying_paint() -> Self {
        Self {
            inner: CoreTemporalDielectric::drying_paint(),
        }
    }

    /// Create with weathering glass preset.
    #[wasm_bindgen(js_name = "weatheringGlass")]
    pub fn weathering_glass() -> Self {
        Self {
            inner: CoreTemporalDielectric::weathering_glass(),
        }
    }
}

#[wasm_bindgen]
pub struct TemporalThinFilm {
    inner: CoreTemporalThinFilm,
}

#[wasm_bindgen]
impl TemporalThinFilm {
    /// Create with soap bubble preset.
    #[wasm_bindgen(js_name = "soapBubble")]
    pub fn soap_bubble() -> Self {
        Self {
            inner: CoreTemporalThinFilm::soap_bubble(),
        }
    }
}

#[wasm_bindgen]
pub struct TemporalConductor {
    inner: CoreTemporalConductor,
}

#[wasm_bindgen]
impl TemporalConductor {
    /// Create with heated gold preset.
    #[wasm_bindgen(js_name = "heatedGold")]
    pub fn heated_gold() -> Self {
        Self {
            inner: CoreTemporalConductor::heated_gold(),
        }
    }
}

// =============================================================================
// NEURAL CONSTRAINTS
// =============================================================================

use momoto_materials::glass_physics::neural_constraints::{
    ConstraintConfig as CoreConstraintConfig, ConstraintValidator as CoreConstraintValidator,
};

#[wasm_bindgen]
pub struct ConstraintValidator {
    inner: CoreConstraintValidator,
}

#[wasm_bindgen]
impl ConstraintValidator {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: CoreConstraintValidator::new(),
        }
    }

    #[wasm_bindgen(js_name = "withConfig")]
    pub fn with_config(
        energy_tolerance: f64,
        reciprocity_tolerance: f64,
        max_spectral_gradient: f64,
        hard_clamp: bool,
    ) -> Self {
        Self {
            inner: CoreConstraintValidator::with_config(CoreConstraintConfig {
                energy_tolerance,
                reciprocity_tolerance,
                max_spectral_gradient,
                hard_clamp,
                ..CoreConstraintConfig::default()
            }),
        }
    }
}

// =============================================================================
// PBR API v1.0 — Unified BSDF
// =============================================================================

use momoto_materials::glass_physics::unified_bsdf::{
    BSDFContext as CoreBSDFContext, ConductorBSDF as CoreConductorBSDF,
    DielectricBSDF as CoreDielectricBSDF, LambertianBSDF as CoreLambertianBSDF,
    LayeredBSDF as CoreLayeredBSDF, ThinFilmBSDF as CoreThinFilmBSDF, Vector3, BSDF,
};

fn make_bsdf_context(
    wi_x: f64,
    wi_y: f64,
    wi_z: f64,
    wo_x: f64,
    wo_y: f64,
    wo_z: f64,
) -> CoreBSDFContext {
    CoreBSDFContext {
        wi: Vector3::new(wi_x, wi_y, wi_z),
        wo: Vector3::new(wo_x, wo_y, wo_z),
        normal: Vector3::unit_z(),
        tangent: Vector3::unit_x(),
        bitangent: Vector3::unit_y(),
        wavelength: 550.0,
        wavelengths: None,
    }
}

#[wasm_bindgen]
pub struct DielectricBSDF {
    inner: CoreDielectricBSDF,
}

#[wasm_bindgen]
impl DielectricBSDF {
    #[wasm_bindgen(constructor)]
    pub fn new(ior: f64, roughness: f64) -> Self {
        Self {
            inner: CoreDielectricBSDF::new(ior, roughness),
        }
    }

    pub fn glass() -> Self {
        Self {
            inner: CoreDielectricBSDF::glass(),
        }
    }

    pub fn water() -> Self {
        Self {
            inner: CoreDielectricBSDF::water(),
        }
    }

    pub fn diamond() -> Self {
        Self {
            inner: CoreDielectricBSDF::diamond(),
        }
    }

    #[wasm_bindgen(js_name = "frostedGlass")]
    pub fn frosted_glass() -> Self {
        Self {
            inner: CoreDielectricBSDF::frosted_glass(),
        }
    }

    pub fn evaluate(
        &self,
        wi_x: f64,
        wi_y: f64,
        wi_z: f64,
        wo_x: f64,
        wo_y: f64,
        wo_z: f64,
    ) -> Result<JsValue, JsValue> {
        let ctx = make_bsdf_context(wi_x, wi_y, wi_z, wo_x, wo_y, wo_z);
        let response = self.inner.evaluate(&ctx);
        serde_wasm_bindgen::to_value(&response).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = "validateEnergy")]
    pub fn validate_energy(&self) -> Result<JsValue, JsValue> {
        let ctx = make_bsdf_context(0.0, 0.0, 1.0, 0.0, 0.0, 1.0);
        let validation = self.inner.validate_energy(&ctx);
        serde_wasm_bindgen::to_value(&serde_json::json!({
            "conserved": validation.conserved,
            "error": validation.error,
            "details": validation.details,
        }))
        .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

#[wasm_bindgen]
pub struct ConductorBSDF {
    inner: CoreConductorBSDF,
}

#[wasm_bindgen]
impl ConductorBSDF {
    #[wasm_bindgen(constructor)]
    pub fn new(n: f64, k: f64, roughness: f64) -> Self {
        Self {
            inner: CoreConductorBSDF::new(n, k, roughness),
        }
    }

    pub fn gold() -> Self {
        Self {
            inner: CoreConductorBSDF::gold(),
        }
    }
    pub fn silver() -> Self {
        Self {
            inner: CoreConductorBSDF::silver(),
        }
    }
    pub fn copper() -> Self {
        Self {
            inner: CoreConductorBSDF::copper(),
        }
    }
    pub fn aluminum() -> Self {
        Self {
            inner: CoreConductorBSDF::aluminum(),
        }
    }
    pub fn chrome() -> Self {
        Self {
            inner: CoreConductorBSDF::chrome(),
        }
    }

    pub fn evaluate(
        &self,
        wi_x: f64,
        wi_y: f64,
        wi_z: f64,
        wo_x: f64,
        wo_y: f64,
        wo_z: f64,
    ) -> Result<JsValue, JsValue> {
        let ctx = make_bsdf_context(wi_x, wi_y, wi_z, wo_x, wo_y, wo_z);
        let response = self.inner.evaluate(&ctx);
        serde_wasm_bindgen::to_value(&response).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = "validateEnergy")]
    pub fn validate_energy(&self) -> Result<JsValue, JsValue> {
        let ctx = make_bsdf_context(0.0, 0.0, 1.0, 0.0, 0.0, 1.0);
        let validation = self.inner.validate_energy(&ctx);
        serde_wasm_bindgen::to_value(&serde_json::json!({
            "conserved": validation.conserved,
            "error": validation.error,
            "details": validation.details,
        }))
        .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

#[wasm_bindgen]
pub struct ThinFilmBSDF {
    inner: CoreThinFilmBSDF,
}

#[wasm_bindgen]
impl ThinFilmBSDF {
    #[wasm_bindgen(constructor)]
    pub fn new(substrate_ior: f64, film_ior: f64, film_thickness: f64) -> Self {
        Self {
            inner: CoreThinFilmBSDF::new(substrate_ior, film_ior, film_thickness),
        }
    }

    #[wasm_bindgen(js_name = "soapBubble")]
    pub fn soap_bubble(thickness: f64) -> Self {
        Self {
            inner: CoreThinFilmBSDF::soap_bubble(thickness),
        }
    }

    #[wasm_bindgen(js_name = "oilOnWater")]
    pub fn oil_on_water(thickness: f64) -> Self {
        Self {
            inner: CoreThinFilmBSDF::oil_on_water(thickness),
        }
    }

    #[wasm_bindgen(js_name = "arCoating")]
    pub fn ar_coating() -> Self {
        Self {
            inner: CoreThinFilmBSDF::ar_coating(),
        }
    }

    pub fn evaluate(
        &self,
        wi_x: f64,
        wi_y: f64,
        wi_z: f64,
        wo_x: f64,
        wo_y: f64,
        wo_z: f64,
    ) -> Result<JsValue, JsValue> {
        let ctx = make_bsdf_context(wi_x, wi_y, wi_z, wo_x, wo_y, wo_z);
        let response = self.inner.evaluate(&ctx);
        serde_wasm_bindgen::to_value(&response).map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

#[wasm_bindgen]
pub struct LambertianBSDF {
    inner: CoreLambertianBSDF,
}

#[wasm_bindgen]
impl LambertianBSDF {
    #[wasm_bindgen(constructor)]
    pub fn new(albedo: f64) -> Self {
        Self {
            inner: CoreLambertianBSDF::new(albedo),
        }
    }

    pub fn white() -> Self {
        Self {
            inner: CoreLambertianBSDF::white(),
        }
    }
    pub fn gray() -> Self {
        Self {
            inner: CoreLambertianBSDF::gray(),
        }
    }

    pub fn evaluate(
        &self,
        wi_x: f64,
        wi_y: f64,
        wi_z: f64,
        wo_x: f64,
        wo_y: f64,
        wo_z: f64,
    ) -> Result<JsValue, JsValue> {
        let ctx = make_bsdf_context(wi_x, wi_y, wi_z, wo_x, wo_y, wo_z);
        let response = self.inner.evaluate(&ctx);
        serde_wasm_bindgen::to_value(&response).map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

#[wasm_bindgen]
pub struct LayeredBSDF {
    inner: CoreLayeredBSDF,
}

#[wasm_bindgen]
impl LayeredBSDF {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: CoreLayeredBSDF::new(),
        }
    }

    /// Add a dielectric layer.
    #[wasm_bindgen(js_name = "pushDielectric")]
    pub fn push_dielectric(mut self, ior: f64, roughness: f64) -> LayeredBSDF {
        self.inner = self
            .inner
            .push(Box::new(CoreDielectricBSDF::new(ior, roughness)));
        self
    }

    /// Add a conductor layer.
    #[wasm_bindgen(js_name = "pushConductor")]
    pub fn push_conductor(mut self, n: f64, k: f64, roughness: f64) -> LayeredBSDF {
        self.inner = self
            .inner
            .push(Box::new(CoreConductorBSDF::new(n, k, roughness)));
        self
    }

    /// Add a thin film layer.
    #[wasm_bindgen(js_name = "pushThinFilm")]
    pub fn push_thin_film(
        mut self,
        substrate_ior: f64,
        film_ior: f64,
        thickness: f64,
    ) -> LayeredBSDF {
        self.inner = self.inner.push(Box::new(CoreThinFilmBSDF::new(
            substrate_ior,
            film_ior,
            thickness,
        )));
        self
    }

    /// Add a lambertian layer.
    #[wasm_bindgen(js_name = "pushLambertian")]
    pub fn push_lambertian(mut self, albedo: f64) -> LayeredBSDF {
        self.inner = self.inner.push(Box::new(CoreLambertianBSDF::new(albedo)));
        self
    }

    #[wasm_bindgen(js_name = "layerCount")]
    pub fn layer_count(&self) -> usize {
        self.inner.layer_count()
    }

    pub fn evaluate(
        &self,
        wi_x: f64,
        wi_y: f64,
        wi_z: f64,
        wo_x: f64,
        wo_y: f64,
        wo_z: f64,
    ) -> Result<JsValue, JsValue> {
        let ctx = make_bsdf_context(wi_x, wi_y, wi_z, wo_x, wo_y, wo_z);
        let response = self.inner.evaluate(&ctx);
        serde_wasm_bindgen::to_value(&response).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = "validateEnergy")]
    pub fn validate_energy(&self) -> Result<JsValue, JsValue> {
        let ctx = make_bsdf_context(0.0, 0.0, 1.0, 0.0, 0.0, 1.0);
        let validation = self.inner.validate_energy(&ctx);
        serde_wasm_bindgen::to_value(&serde_json::json!({
            "conserved": validation.conserved,
            "error": validation.error,
            "details": validation.details,
        }))
        .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

// =============================================================================
// PBR MATERIAL (High-Level API)
// =============================================================================

use momoto_materials::glass_physics::pbr_api::v1::{
    EvaluationContext as CoreEvaluationContext, Material as CorePBRMaterial,
    MaterialBuilder as CoreMaterialBuilder, MaterialPreset as CoreMaterialPreset,
};

#[wasm_bindgen]
pub struct PBRMaterial {
    inner: CorePBRMaterial,
}

#[wasm_bindgen]
impl PBRMaterial {
    #[wasm_bindgen(js_name = "fromPreset")]
    pub fn from_preset(preset: &str) -> Result<PBRMaterial, JsValue> {
        let p = match preset {
            "glass" => CoreMaterialPreset::Glass,
            "frosted_glass" | "frostedGlass" => CoreMaterialPreset::FrostedGlass,
            "water" => CoreMaterialPreset::Water,
            "diamond" => CoreMaterialPreset::Diamond,
            "gold" => CoreMaterialPreset::Gold,
            "silver" => CoreMaterialPreset::Silver,
            "copper" => CoreMaterialPreset::Copper,
            "soap_bubble" | "soapBubble" => CoreMaterialPreset::SoapBubble,
            "oil_slick" | "oilSlick" => CoreMaterialPreset::OilSlick,
            _ => return Err(JsValue::from_str(&format!("Unknown preset: {}", preset))),
        };
        Ok(PBRMaterial {
            inner: CorePBRMaterial::from_preset(p),
        })
    }

    pub fn builder() -> PBRMaterialBuilder {
        PBRMaterialBuilder {
            inner: CoreMaterialBuilder::new(),
        }
    }

    /// Evaluate the material with default context.
    pub fn evaluate(&self) -> Result<JsValue, JsValue> {
        let ctx = CoreEvaluationContext::default();
        let response = self.inner.evaluate(&ctx);
        serde_wasm_bindgen::to_value(&response).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Evaluate with custom incident angle (cos_theta = angle from normal).
    #[wasm_bindgen(js_name = "evaluateAtAngle")]
    pub fn evaluate_at_angle(&self, cos_theta: f64) -> Result<JsValue, JsValue> {
        use momoto_materials::glass_physics::unified_bsdf::Vector3;
        let sin_theta = (1.0 - cos_theta * cos_theta).max(0.0).sqrt();
        let mut ctx = CoreEvaluationContext::default();
        ctx.wi = Vector3::new(sin_theta, 0.0, cos_theta).into();
        let response = self.inner.evaluate(&ctx);
        serde_wasm_bindgen::to_value(&response).map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

#[wasm_bindgen]
pub struct PBRMaterialBuilder {
    inner: CoreMaterialBuilder,
}

#[wasm_bindgen]
impl PBRMaterialBuilder {
    #[wasm_bindgen(js_name = "addDielectric")]
    pub fn add_dielectric(mut self, ior: f64, roughness: f64) -> PBRMaterialBuilder {
        self.inner = self.inner.dielectric(ior, roughness);
        self
    }

    #[wasm_bindgen(js_name = "addConductor")]
    pub fn add_conductor(mut self, n: f64, k: f64, roughness: f64) -> PBRMaterialBuilder {
        self.inner = self.inner.conductor(n, k, roughness);
        self
    }

    #[wasm_bindgen(js_name = "addThinFilm")]
    pub fn add_thin_film(
        mut self,
        film_ior: f64,
        substrate_ior: f64,
        thickness_nm: f64,
    ) -> PBRMaterialBuilder {
        self.inner = self.inner.thin_film(film_ior, substrate_ior, thickness_nm);
        self
    }

    pub fn color(mut self, r: f64, g: f64, b: f64) -> PBRMaterialBuilder {
        self.inner = self.inner.color(r, g, b);
        self
    }

    pub fn opacity(mut self, opacity: f64) -> PBRMaterialBuilder {
        self.inner = self.inner.opacity(opacity);
        self
    }

    pub fn build(self) -> PBRMaterial {
        PBRMaterial {
            inner: self.inner.build(),
        }
    }
}

// =============================================================================
// EASING FUNCTIONS
// =============================================================================

use momoto_materials::glass_physics::temporal::interpolation::{
    ease_in_out as core_ease_in_out, remap as core_remap, smootherstep as core_smootherstep,
    smoothstep as core_smoothstep,
};

#[wasm_bindgen]
pub fn smoothstep(t: f64) -> f64 {
    core_smoothstep(t)
}

#[wasm_bindgen]
pub fn smootherstep(t: f64) -> f64 {
    core_smootherstep(t)
}

#[wasm_bindgen(js_name = "easeInOut")]
pub fn ease_in_out(t: f64) -> f64 {
    core_ease_in_out(t)
}

#[wasm_bindgen]
pub fn remap(value: f64, in_min: f64, in_max: f64, out_min: f64, out_max: f64) -> f64 {
    core_remap(value, in_min, in_max, out_min, out_max)
}

// =============================================================================
// BATCH EVALUATION
// =============================================================================

use momoto_materials::glass_physics::batch::{
    evaluate_batch as core_evaluate_batch, BatchMaterialInput as CoreBatchMaterialInput,
};

/// Evaluate materials in batch. Arrays must be same length.
#[wasm_bindgen(js_name = "evaluateMaterialBatch")]
pub fn evaluate_material_batch(
    iors: &[f64],
    roughnesses: &[f64],
    thicknesses: &[f64],
    absorptions: &[f64],
) -> Result<JsValue, JsValue> {
    let len = iors.len();
    if roughnesses.len() != len || thicknesses.len() != len || absorptions.len() != len {
        return Err(JsValue::from_str("All arrays must have the same length"));
    }

    let mut input = CoreBatchMaterialInput::with_capacity(len);
    for i in 0..len {
        input.push(iors[i], roughnesses[i], thicknesses[i], absorptions[i]);
    }

    match core_evaluate_batch(&input) {
        Ok(results) => serde_wasm_bindgen::to_value(&serde_json::json!({
            "count": results.count,
            "opacity": results.opacity,
            "blur": results.blur,
            "fresnel_normal": results.fresnel_normal,
        }))
        .map_err(|e| JsValue::from_str(&e.to_string())),
        Err(e) => Err(JsValue::from_str(&e)),
    }
}

// =============================================================================
// PERCEPTUAL LOSS FUNCTIONS (DeltaE)
// =============================================================================

use momoto_materials::glass_physics::perceptual_loss::{
    delta_e_2000 as core_delta_e_2000, delta_e_76 as core_delta_e_76,
    delta_e_94 as core_delta_e_94, lab_to_rgb as core_lab_to_rgb, rgb_to_lab as core_rgb_to_lab,
    Illuminant, LabColor,
};

#[wasm_bindgen(js_name = "deltaE76")]
pub fn delta_e_76(l1: f64, a1: f64, b1: f64, l2: f64, a2: f64, b2: f64) -> f64 {
    core_delta_e_76(LabColor::new(l1, a1, b1), LabColor::new(l2, a2, b2))
}

#[wasm_bindgen(js_name = "deltaE94")]
pub fn delta_e_94(l1: f64, a1: f64, b1: f64, l2: f64, a2: f64, b2: f64) -> f64 {
    core_delta_e_94(LabColor::new(l1, a1, b1), LabColor::new(l2, a2, b2))
}

#[wasm_bindgen(js_name = "deltaE2000")]
pub fn delta_e_2000(l1: f64, a1: f64, b1: f64, l2: f64, a2: f64, b2: f64) -> f64 {
    core_delta_e_2000(LabColor::new(l1, a1, b1), LabColor::new(l2, a2, b2))
}

#[wasm_bindgen(js_name = "rgbToLab")]
pub fn rgb_to_lab(r: f64, g: f64, b: f64) -> Box<[f64]> {
    let lab = core_rgb_to_lab([r, g, b], Illuminant::D65);
    Box::new([lab.l, lab.a, lab.b])
}

#[wasm_bindgen(js_name = "labToRgb")]
pub fn lab_to_rgb(l: f64, a: f64, b: f64) -> Box<[f64]> {
    let rgb = core_lab_to_rgb(LabColor::new(l, a, b), Illuminant::D65);
    Box::new(rgb)
}

#[wasm_bindgen(js_name = "deltaE2000Batch")]
pub fn delta_e_2000_batch(lab_pairs: &[f64]) -> Result<Box<[f64]>, JsValue> {
    if lab_pairs.len() % 6 != 0 {
        return Err(JsValue::from_str(
            "Input must be multiple of 6: [L1, a1, b1, L2, a2, b2, ...]",
        ));
    }
    let count = lab_pairs.len() / 6;
    let mut results = Vec::with_capacity(count);
    for i in 0..count {
        let base = i * 6;
        results.push(core_delta_e_2000(
            LabColor::new(lab_pairs[base], lab_pairs[base + 1], lab_pairs[base + 2]),
            LabColor::new(
                lab_pairs[base + 3],
                lab_pairs[base + 4],
                lab_pairs[base + 5],
            ),
        ));
    }
    Ok(results.into_boxed_slice())
}

// =============================================================================
// ENHANCED CSS BACKEND
// =============================================================================

use momoto_materials::css_enhanced::{
    render_enhanced_css as core_render_enhanced_css, render_premium_css as core_render_premium_css,
};

/// Render enhanced CSS for an evaluated material with a render config.
#[wasm_bindgen(js_name = "renderEnhancedCss")]
pub fn render_enhanced_css(
    material: &super::EvaluatedMaterial,
    context: &super::CssRenderConfig,
) -> String {
    core_render_enhanced_css(material.to_core(), context.to_core())
}

/// Render premium CSS with default config.
#[wasm_bindgen(js_name = "renderPremiumCss")]
pub fn render_premium_css(material: &super::EvaluatedMaterial) -> String {
    core_render_premium_css(material.to_core())
}

// =============================================================================
// MATERIAL PRESETS
// =============================================================================

use momoto_materials::glass_physics::enhanced_presets;
use momoto_materials::glass_physics::enhanced_presets::QualityTier;

/// Get all enhanced glass presets as JSON array.
#[wasm_bindgen(js_name = "getEnhancedGlassPresets")]
pub fn get_enhanced_glass_presets() -> Result<JsValue, JsValue> {
    let presets = enhanced_presets::all_presets();
    let json: Vec<serde_json::Value> = presets
        .iter()
        .map(|p| {
            serde_json::json!({
                "name": p.name,
                "ior": p.ior,
                "roughness": p.roughness,
                "thickness": p.thickness,
                "absorption": p.absorption,
            })
        })
        .collect();
    serde_wasm_bindgen::to_value(&json).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Get presets by quality tier. Tier: "fast", "standard", "high", "ultra_high", "experimental", "reference".
#[wasm_bindgen(js_name = "getPresetsByQuality")]
pub fn get_presets_by_quality(tier: &str) -> Result<JsValue, JsValue> {
    let qt = match tier {
        "fast" => QualityTier::Fast,
        "standard" => QualityTier::Standard,
        "high" => QualityTier::High,
        "ultra_high" | "ultraHigh" => QualityTier::UltraHigh,
        "experimental" => QualityTier::Experimental,
        "reference" => QualityTier::Reference,
        _ => {
            return Err(JsValue::from_str(&format!(
                "Unknown quality tier: {}",
                tier
            )))
        }
    };
    let presets = enhanced_presets::presets_by_quality(qt);
    let json: Vec<serde_json::Value> = presets
        .iter()
        .map(|p| {
            serde_json::json!({
                "name": p.name,
                "ior": p.ior,
                "roughness": p.roughness,
                "thickness": p.thickness,
                "absorption": p.absorption,
            })
        })
        .collect();
    serde_wasm_bindgen::to_value(&json).map_err(|e| JsValue::from_str(&e.to_string()))
}

// =============================================================================
// BATCH MATERIAL EVALUATION
// =============================================================================

/// Evaluate multiple materials in a single WASM call for maximum performance.
///
/// Evaluate a batch of dielectric materials using the full BSDF pipeline.
///
/// Avoids N×JS↔WASM boundary crossings by processing all materials in one call.
/// Each material is evaluated as a `DielectricBSDF` (Fresnel + roughness).
///
/// # Arguments
/// * `iors` — index of refraction for each material
/// * `roughnesses` — surface roughness for each material (0=smooth, 1=rough)
/// * `cos_thetas` — cosine of incident angle for each material
///
/// All three slices must have the same length N.
///
/// # Returns
///
/// Flat array of `[reflectance, transmittance, absorption] × N`
/// (length = 3 × N). Energy is always conserved per element.
#[wasm_bindgen(js_name = "evaluateDielectricBatch")]
pub fn evaluate_dielectric_batch(
    iors: &[f64],
    roughnesses: &[f64],
    cos_thetas: &[f64],
) -> Box<[f64]> {
    use momoto_materials::glass_physics::unified_bsdf::{BSDFContext, DielectricBSDF, BSDF};

    let n = iors.len().min(roughnesses.len()).min(cos_thetas.len());
    let mut out = Vec::with_capacity(n * 3);

    for i in 0..n {
        let bsdf = DielectricBSDF::new(iors[i], roughnesses[i]);
        let ctx = BSDFContext::new_simple(cos_thetas[i].clamp(0.0, 1.0));
        let resp = bsdf.evaluate(&ctx);
        out.push(resp.reflectance);
        out.push(resp.transmittance);
        out.push(resp.absorption);
    }

    out.into_boxed_slice()
}

/// Evaluate a single dielectric material. Returns `[reflectance, transmittance, absorption]`.
#[wasm_bindgen(js_name = "evaluateDielectricBSDF")]
pub fn evaluate_dielectric_bsdf(ior: f64, roughness: f64, cos_theta: f64) -> Box<[f64]> {
    use momoto_materials::glass_physics::unified_bsdf::{BSDFContext, DielectricBSDF, BSDF};

    let bsdf = DielectricBSDF::new(ior, roughness);
    let ctx = BSDFContext::new_simple(cos_theta.clamp(0.0, 1.0));
    let resp = bsdf.evaluate(&ctx);
    vec![resp.reflectance, resp.transmittance, resp.absorption].into_boxed_slice()
}

// =============================================================================
// MICROFACET (GGX) BRDF BINDINGS
// =============================================================================

/// Evaluate Cook-Torrance specular BRDF (GGX + Smith G2 + Schlick Fresnel).
///
/// # Arguments
/// * `n_dot_v` — cosine of view angle with surface normal (0=grazing, 1=normal)
/// * `n_dot_l` — cosine of light angle with normal
/// * `n_dot_h` — cosine of half-vector with normal
/// * `h_dot_v` — cosine of half-vector with view (for Fresnel)
/// * `roughness` — surface roughness in [0, 1]
/// * `f0` — Fresnel reflectance at normal incidence (0.04 for glass, 0.8+ for metals)
///
/// # Returns
///
/// BRDF value ≥ 0. Multiply by `n_dot_l` to get irradiance contribution.
#[wasm_bindgen(js_name = "cookTorranceBRDF")]
pub fn cook_torrance_brdf(
    n_dot_v: f64,
    n_dot_l: f64,
    n_dot_h: f64,
    h_dot_v: f64,
    roughness: f64,
    f0: f64,
) -> f64 {
    use momoto_materials::glass_physics::microfacet::cook_torrance_eval;
    cook_torrance_eval(n_dot_v, n_dot_l, n_dot_h, h_dot_v, roughness, f0)
}

/// Evaluate Oren-Nayar diffuse BRDF for rough surfaces.
///
/// # Arguments
/// * `n_dot_l` — cosine of light angle with normal
/// * `n_dot_v` — cosine of view angle with normal
/// * `l_dot_v` — cosine of angle between light and view directions
/// * `roughness` — surface roughness (0 = Lambertian, 1 = fully rough)
///
/// # Returns
///
/// BRDF value ≥ 0 (includes 1/π normalisation). At roughness=0 returns 1/π (Lambert).
#[wasm_bindgen(js_name = "orenNayarBRDF")]
pub fn oren_nayar_brdf(n_dot_l: f64, n_dot_v: f64, l_dot_v: f64, roughness: f64) -> f64 {
    use momoto_materials::glass_physics::microfacet::oren_nayar_eval;
    oren_nayar_eval(n_dot_l, n_dot_v, l_dot_v, roughness)
}

/// Evaluate a full microfacet material (Cook-Torrance specular + Oren-Nayar diffuse).
///
/// # Arguments
/// * `roughness` — surface roughness [0, 1]
/// * `metallic` — metallic factor (0 = dielectric, 1 = metallic)
/// * `f0` — Fresnel at normal incidence
/// * `cos_theta` — incident angle cosine
///
/// # Returns
///
/// `[reflectance, transmittance, absorption]` — energy conserved.
#[wasm_bindgen(js_name = "evaluateMicrofacetBSDF")]
pub fn evaluate_microfacet_bsdf(
    roughness: f64,
    metallic: f64,
    f0: f64,
    cos_theta: f64,
) -> Box<[f64]> {
    use momoto_materials::glass_physics::microfacet::MicrofacetBSDF;
    use momoto_materials::glass_physics::unified_bsdf::{BSDFContext, BSDF};

    let bsdf = MicrofacetBSDF::new(roughness, metallic, f0, 0.8);
    let ctx = BSDFContext::new_simple(cos_theta.clamp(0.0, 1.0));
    let resp = bsdf.evaluate(&ctx);
    vec![resp.reflectance, resp.transmittance, resp.absorption].into_boxed_slice()
}

/// Evaluate GGX Normal Distribution Function.
///
/// # Arguments
/// * `n_dot_h` — cosine of half-vector with normal
/// * `roughness` — surface roughness (alpha = roughness²)
///
/// # Returns
///
/// NDF value (unnormalised density). Peaks at n·h = 1.0.
#[wasm_bindgen(js_name = "ggxNDF")]
pub fn ggx_ndf_wasm(n_dot_h: f64, roughness: f64) -> f64 {
    use momoto_materials::glass_physics::microfacet::ggx_ndf;
    let alpha = roughness * roughness;
    ggx_ndf(n_dot_h, alpha)
}

// =============================================================================
// MATERIAL → COLOR BRIDGE
// =============================================================================

/// Convert a dielectric material to its dominant OKLCH color via spectral integration.
///
/// Evaluates the material's BSDF at 31 wavelengths (400–700 nm, 10 nm steps),
/// weights by the D65 illuminant, integrates with CIE 1931 2-degree CMFs,
/// and converts to OKLCH.
///
/// IOR dispersion is modeled via Cauchy: n(λ) = n₀ + 0.004/λ² (λ in μm).
///
/// # Arguments
/// * `ior` — base IOR at 589 nm (sodium D line)
/// * `roughness` — surface roughness in [0, 1]
/// * `cos_theta` — cosine of incidence angle
///
/// # Returns
///
/// Flat array `[L, C, H, reflectance, cct]` where:
/// - L, C, H are OKLCH components of the dominant color
/// - reflectance: spectrally-averaged reflectance (0–1)
/// - cct: Correlated Color Temperature in Kelvin (McCamy 1992)
#[wasm_bindgen(js_name = "materialToDominantColor")]
pub fn material_to_dominant_color(ior: f64, roughness: f64, cos_theta: f64) -> Box<[f64]> {
    use momoto_agent::material_bridge::bsdf_to_dominant_color;
    let result = bsdf_to_dominant_color(ior, roughness, cos_theta);
    vec![
        result.dominant.l,
        result.dominant.c,
        result.dominant.h,
        result.reflectance,
        result.cct,
    ]
    .into_boxed_slice()
}

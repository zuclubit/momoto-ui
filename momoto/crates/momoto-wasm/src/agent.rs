// =============================================================================
// momoto-wasm: Agent Bindings
// File: crates/momoto-wasm/src/agent.rs
//
// Exposes momoto-agent crate via wasm-bindgen.
// =============================================================================

use momoto_agent::{
    self as agent_lib, AgentExecutor as CoreAgentExecutor, ColorMetrics as CoreColorMetrics,
    ComplianceLevel as CoreComplianceLevel, Constraint as CoreConstraint, Contract as CoreContract,
    ContrastStandard as CoreContrastStandard, Gamut as CoreGamut, Query as CoreQuery,
    ValidationResponse as CoreValidationResponse,
};
use wasm_bindgen::prelude::*;

// =============================================================================
// AgentExecutor — The primary query/response interface
// =============================================================================

#[wasm_bindgen]
pub struct AgentExecutor {
    inner: CoreAgentExecutor,
}

#[wasm_bindgen]
impl AgentExecutor {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: CoreAgentExecutor::new(),
        }
    }

    /// Execute a query and return the response as JSON.
    #[wasm_bindgen]
    pub fn execute(&self, query_json: &str) -> Result<String, JsValue> {
        let query: CoreQuery = serde_json::from_str(query_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid query JSON: {}", e)))?;
        let response = self.inner.execute(query);
        serde_json::to_string(&response)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }
}

// =============================================================================
// Convenience functions
// =============================================================================

/// Validate a single color against a contract.
#[wasm_bindgen(js_name = "agentValidate")]
pub fn agent_validate(color_hex: &str, contract_json: &str) -> Result<String, JsValue> {
    let contract: CoreContract = serde_json::from_str(contract_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid contract: {}", e)))?;
    let resp = agent_lib::validate(color_hex, &contract);
    serde_json::to_string(&resp).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Validate a color pair for contrast compliance.
#[wasm_bindgen(js_name = "agentValidatePair")]
pub fn agent_validate_pair(
    fg_hex: &str,
    bg_hex: &str,
    standard: u8,
    level: u8,
) -> Result<String, JsValue> {
    let std = match standard {
        0 => CoreContrastStandard::Wcag,
        _ => CoreContrastStandard::Apca,
    };
    let lvl = match level {
        0 => CoreComplianceLevel::AA,
        _ => CoreComplianceLevel::AAA,
    };
    let resp = agent_lib::validate_pair(fg_hex, bg_hex, std, lvl);
    serde_json::to_string(&resp).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Get full color metrics for a hex color.
#[wasm_bindgen(js_name = "agentGetMetrics")]
pub fn agent_get_metrics(color_hex: &str) -> Result<String, JsValue> {
    let metrics = agent_lib::get_metrics(color_hex);
    serde_json::to_string(&metrics).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Get a material preset by name.
#[wasm_bindgen(js_name = "agentGetMaterial")]
pub fn agent_get_material(preset: &str) -> Result<JsValue, JsValue> {
    match agent_lib::get_material(preset) {
        Some(resp) => {
            let json =
                serde_json::to_string(&resp).map_err(|e| JsValue::from_str(&e.to_string()))?;
            Ok(JsValue::from_str(&json))
        }
        None => Ok(JsValue::NULL),
    }
}

/// List all available materials, optionally filtered by category.
#[wasm_bindgen(js_name = "agentListMaterials")]
pub fn agent_list_materials(category: Option<String>) -> Result<String, JsValue> {
    let resp = agent_lib::list_materials(category.as_deref());
    serde_json::to_string(&resp).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Recommend a foreground color for a given background.
#[wasm_bindgen(js_name = "agentRecommendForeground")]
pub fn agent_recommend_foreground(
    bg_hex: &str,
    context: &str,
    target: &str,
) -> Result<String, JsValue> {
    let resp = agent_lib::recommend_foreground(bg_hex, context, target);
    serde_json::to_string(&resp).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Improve an existing foreground color against a background.
#[wasm_bindgen(js_name = "agentImproveForeground")]
pub fn agent_improve_foreground(
    fg_hex: &str,
    bg_hex: &str,
    context: &str,
    target: &str,
) -> Result<String, JsValue> {
    let resp = agent_lib::improve_foreground(fg_hex, bg_hex, context, target);
    serde_json::to_string(&resp).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Score a color pair for quality.
#[wasm_bindgen(js_name = "agentScorePair")]
pub fn agent_score_pair(
    fg_hex: &str,
    bg_hex: &str,
    context: &str,
    target: &str,
) -> Result<String, JsValue> {
    let resp = agent_lib::score_pair(fg_hex, bg_hex, context, target);
    serde_json::to_string(&resp).map_err(|e| JsValue::from_str(&e.to_string()))
}

// =============================================================================
// Batch operations
// =============================================================================

/// Validate multiple color pairs in a single WASM call.
#[wasm_bindgen(js_name = "agentValidatePairsBatch")]
pub fn agent_validate_pairs_batch(pairs_json: &str) -> Result<String, JsValue> {
    #[derive(serde::Deserialize)]
    struct PairInput {
        fg: String,
        bg: String,
        standard: u8,
        level: u8,
    }

    let pairs: Vec<PairInput> = serde_json::from_str(pairs_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid JSON: {}", e)))?;

    let results: Vec<CoreValidationResponse> = pairs
        .iter()
        .map(|p| {
            let std = match p.standard {
                0 => CoreContrastStandard::Wcag,
                _ => CoreContrastStandard::Apca,
            };
            let lvl = match p.level {
                0 => CoreComplianceLevel::AA,
                _ => CoreComplianceLevel::AAA,
            };
            agent_lib::validate_pair(&p.fg, &p.bg, std, lvl)
        })
        .collect();

    serde_json::to_string(&results).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Get metrics for multiple colors in a single WASM call.
#[wasm_bindgen(js_name = "agentGetMetricsBatch")]
pub fn agent_get_metrics_batch(colors_json: &str) -> Result<String, JsValue> {
    let colors: Vec<String> = serde_json::from_str(colors_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid JSON: {}", e)))?;

    let results: Vec<CoreColorMetrics> = colors
        .iter()
        .map(|hex| agent_lib::get_metrics(hex))
        .collect();

    serde_json::to_string(&results).map_err(|e| JsValue::from_str(&e.to_string()))
}

// =============================================================================
// Contract Builder
// =============================================================================

#[wasm_bindgen]
pub struct ContractBuilder {
    constraints: Vec<CoreConstraint>,
}

#[wasm_bindgen]
impl ContractBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            constraints: Vec::new(),
        }
    }

    #[wasm_bindgen(js_name = "minContrastWcagAA")]
    pub fn min_contrast_wcag_aa(mut self, against_hex: &str) -> Self {
        self.constraints
            .push(CoreConstraint::min_contrast_wcag_aa(against_hex));
        self
    }

    #[wasm_bindgen(js_name = "minContrastWcagAAA")]
    pub fn min_contrast_wcag_aaa(mut self, against_hex: &str) -> Self {
        self.constraints
            .push(CoreConstraint::min_contrast_wcag_aaa(against_hex));
        self
    }

    #[wasm_bindgen(js_name = "inSrgb")]
    pub fn in_srgb(mut self) -> Self {
        self.constraints
            .push(CoreConstraint::in_gamut(CoreGamut::Srgb));
        self
    }

    #[wasm_bindgen(js_name = "inP3")]
    pub fn in_p3(mut self) -> Self {
        self.constraints
            .push(CoreConstraint::in_gamut(CoreGamut::P3));
        self
    }

    #[wasm_bindgen(js_name = "lightnessRange")]
    pub fn lightness_range(mut self, min: f32, max: f32) -> Self {
        self.constraints
            .push(CoreConstraint::lightness_range(min, max));
        self
    }

    #[wasm_bindgen(js_name = "chromaRange")]
    pub fn chroma_range(mut self, min: f32, max: f32) -> Self {
        self.constraints
            .push(CoreConstraint::chroma_range(min, max));
        self
    }

    #[wasm_bindgen(js_name = "hueRange")]
    pub fn hue_range(mut self, min: f32, max: f32) -> Self {
        self.constraints.push(CoreConstraint::hue_range(min, max));
        self
    }

    #[wasm_bindgen]
    pub fn build(&self) -> String {
        let contract = CoreContract {
            version: agent_lib::contract::Version { major: 1, minor: 0 },
            constraints: self.constraints.clone(),
        };
        contract.to_json()
    }

    #[wasm_bindgen(js_name = "buildAndValidate")]
    pub fn build_and_validate(&self, color_hex: &str) -> Result<String, JsValue> {
        let contract = CoreContract {
            version: agent_lib::contract::Version { major: 1, minor: 0 },
            constraints: self.constraints.clone(),
        };
        let resp = agent_lib::validate(color_hex, &contract);
        serde_json::to_string(&resp).map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

// =============================================================================
// Experience Generation
// =============================================================================

/// Generate a complete visual experience from brand colors.
#[wasm_bindgen(js_name = "generateExperience")]
pub fn generate_experience(
    preset: &str,
    primary_hex: &str,
    background_hex: &str,
) -> Result<String, JsValue> {
    let executor = CoreAgentExecutor::new();
    let query_json = serde_json::json!({
        "GenerateExperience": {
            "preset": preset,
            "primary": primary_hex,
            "background": background_hex,
        }
    });
    let query: CoreQuery = serde_json::from_value(query_json)
        .map_err(|e| JsValue::from_str(&format!("Query construction error: {}", e)))?;
    let response = executor.execute(query);
    serde_json::to_string(&response).map_err(|e| JsValue::from_str(&e.to_string()))
}

// =============================================================================
// Temporal Color (Animation)
// =============================================================================

/// Create a color transition sequence.
#[wasm_bindgen(js_name = "createColorTransition")]
pub fn create_color_transition(
    from_hex: &str,
    to_hex: &str,
    duration_ms: u64,
    easing: &str,
    frame_count: usize,
) -> Result<String, JsValue> {
    use momoto_agent::temporal::{ColorSequence, EasingFunction};
    use momoto_core::color::Color;

    let from = Color::from_hex(from_hex).map_err(|e| JsValue::from_str(&e))?;
    let to = Color::from_hex(to_hex).map_err(|e| JsValue::from_str(&e))?;

    let from_oklch = from.to_oklch();
    let to_oklch = to.to_oklch();

    let easing_fn = match easing {
        "ease_in" | "easeIn" => EasingFunction::EaseIn,
        "ease_out" | "easeOut" => EasingFunction::EaseOut,
        "ease_in_out" | "easeInOut" => EasingFunction::EaseInOut,
        "step" => EasingFunction::Step,
        _ => EasingFunction::Linear,
    };

    let mut seq = ColorSequence::new("transition", &format!("{} → {}", from_hex, to_hex));

    for i in 0..frame_count {
        let raw_t = i as f64 / (frame_count - 1).max(1) as f64;
        let t = easing_fn.apply(raw_t);
        let time_ms = (raw_t * duration_ms as f64) as u64;

        let l = from_oklch.l + (to_oklch.l - from_oklch.l) * t;
        let c = from_oklch.c + (to_oklch.c - from_oklch.c) * t;
        let mut dh = to_oklch.h - from_oklch.h;
        if dh > 180.0 {
            dh -= 360.0;
        }
        if dh < -180.0 {
            dh += 360.0;
        }
        let h = (from_oklch.h + dh * t).rem_euclid(360.0);

        seq.add_state(time_ms, l, c, h);
    }

    serde_json::to_string(&seq).map_err(|e| JsValue::from_str(&e.to_string()))
}

// =============================================================================
// Certification
// =============================================================================

/// Get the Momoto system identity and version.
#[wasm_bindgen(js_name = "getMomotoIdentity")]
pub fn get_momoto_identity() -> Result<String, JsValue> {
    use momoto_agent::certification::CertificationAuthority;
    let authority = CertificationAuthority::new();
    let identity = authority.identity();
    serde_json::to_string(&serde_json::json!({
        "version": identity.version_string(),
        "identity": identity.identity_string(),
        "major": identity.major,
        "minor": identity.minor,
        "patch": identity.patch,
        "phaseCoverage": identity.phase_coverage,
        "buildId": identity.build_id,
        "specVersion": identity.spec_version,
    }))
    .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Run self-certification to verify the Momoto engine integrity.
#[wasm_bindgen(js_name = "selfCertify")]
pub fn self_certify() -> Result<String, JsValue> {
    use momoto_agent::certification::CertificationAuthority;
    let mut authority = CertificationAuthority::new();
    let result = authority.self_certify();
    serde_json::to_string(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}

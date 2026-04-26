// =============================================================================
// momoto-wasm: Temporal Material WASM Bindings
// File: crates/momoto-wasm/src/temporal.rs
//
// Exposes the glass_physics/temporal module via wasm-bindgen.
// All temporal materials implement TemporalBSDF::eval_at_time(&TemporalContext).
// =============================================================================

use wasm_bindgen::prelude::*;

use momoto_materials::glass_physics::temporal::{
    ConductorEvolution, DielectricEvolution, TemporalBSDF, TemporalConductor, TemporalContext,
    TemporalDielectric, TemporalThinFilm, ThinFilmEvolution,
};
use momoto_materials::glass_physics::unified_bsdf::BSDFContext;

// =============================================================================
// TemporalMaterial — wrappers TemporalDielectric
// =============================================================================

/// Time-evolving dielectric material (roughness + IOR evolution).
///
/// Models materials like drying paint or weathering glass where
/// surface roughness changes continuously over time.
#[wasm_bindgen]
pub struct TemporalMaterial {
    inner: TemporalDielectric,
}

#[wasm_bindgen]
impl TemporalMaterial {
    /// Create a temporal dielectric with custom roughness evolution.
    ///
    /// # Arguments
    /// * `roughness_base` — initial roughness at t=0 (0=mirror, 1=fully rough)
    /// * `roughness_target` — final roughness as t→∞
    /// * `roughness_tau` — time constant in seconds (e.g. 60 = dries in ~1 min)
    /// * `ior_base` — index of refraction (e.g. 1.52 for glass)
    #[wasm_bindgen(constructor)]
    pub fn new(
        roughness_base: f64,
        roughness_target: f64,
        roughness_tau: f64,
        ior_base: f64,
    ) -> Self {
        Self {
            inner: TemporalDielectric::new(DielectricEvolution {
                roughness_base,
                roughness_target,
                roughness_tau,
                ior_base,
                ior_temp_coeff: -1e-5,
            }),
        }
    }

    /// Preset: drying paint (roughness 0.05→0.4 over ~60s).
    #[wasm_bindgen(js_name = "dryingPaint")]
    pub fn drying_paint() -> Self {
        Self {
            inner: TemporalDielectric::drying_paint(),
        }
    }

    /// Preset: weathering glass (roughness 0.01→0.15 over ~1h).
    #[wasm_bindgen(js_name = "weatheringGlass")]
    pub fn weathering_glass() -> Self {
        Self {
            inner: TemporalDielectric::weathering_glass(),
        }
    }

    /// Evaluate BSDF at given time and angle.
    ///
    /// Returns `[reflectance, transmittance, absorption]` — always sums to 1.0.
    ///
    /// # Arguments
    /// * `t` — simulation time in seconds
    /// * `cos_theta` — cosine of incident angle (0=grazing, 1=normal)
    #[wasm_bindgen(js_name = "evalAtTime")]
    pub fn eval_at_time(&self, t: f64, cos_theta: f64) -> Box<[f64]> {
        let base_ctx = BSDFContext::new_simple(cos_theta.clamp(0.0, 1.0));
        let ctx = TemporalContext::from_base(base_ctx).with_time(t);
        let resp = self.inner.eval_at_time(&ctx);
        vec![resp.reflectance, resp.transmittance, resp.absorption].into_boxed_slice()
    }

    /// Evaluate at t=0 (static fallback — backward-compatible).
    #[wasm_bindgen(js_name = "evalStatic")]
    pub fn eval_static(&self, cos_theta: f64) -> Box<[f64]> {
        self.eval_at_time(0.0, cos_theta)
    }

    /// Whether this material has time-varying behaviour.
    #[wasm_bindgen(getter, js_name = "supportsTemoral")]
    pub fn supports_temporal(&self) -> bool {
        self.inner.supports_temporal()
    }
}

// =============================================================================
// TemporalThinFilmMaterial — wraps TemporalThinFilm
// =============================================================================

/// Time-evolving thin-film material with oscillating thickness.
///
/// Models soap bubbles and oil slicks where film thickness oscillates,
/// producing iridescent color shifts over time.
#[wasm_bindgen]
pub struct TemporalThinFilmMaterial {
    inner: TemporalThinFilm,
}

#[wasm_bindgen]
impl TemporalThinFilmMaterial {
    /// Create a temporal thin-film with custom parameters.
    ///
    /// # Arguments
    /// * `thickness_base` — base film thickness in nm (e.g. 300)
    /// * `amplitude` — oscillation amplitude in nm
    /// * `frequency_hz` — oscillation frequency in Hz
    /// * `film_ior` — film IOR (e.g. 1.33 for water)
    /// * `substrate_ior` — substrate IOR (e.g. 1.0 for air)
    #[wasm_bindgen(constructor)]
    pub fn new(
        thickness_base: f64,
        amplitude: f64,
        frequency_hz: f64,
        film_ior: f64,
        substrate_ior: f64,
    ) -> Self {
        Self {
            inner: TemporalThinFilm::new(ThinFilmEvolution {
                thickness_base,
                thickness_amplitude: amplitude,
                thickness_frequency: frequency_hz,
                film_ior,
                substrate_ior,
                damping: 0.0,
            }),
        }
    }

    /// Preset: soap bubble (300nm base, 100nm amplitude, 2 Hz, damped).
    #[wasm_bindgen(js_name = "soapBubble")]
    pub fn soap_bubble() -> Self {
        Self {
            inner: TemporalThinFilm::soap_bubble(),
        }
    }

    /// Preset: oil slick (400nm base, slow oscillation).
    #[wasm_bindgen(js_name = "oilSlick")]
    pub fn oil_slick() -> Self {
        Self {
            inner: TemporalThinFilm::oil_slick(),
        }
    }

    /// Evaluate BSDF at given time and angle.
    ///
    /// Returns `[reflectance, transmittance, absorption]` — always sums to 1.0.
    #[wasm_bindgen(js_name = "evalAtTime")]
    pub fn eval_at_time(&self, t: f64, cos_theta: f64) -> Box<[f64]> {
        let base_ctx = BSDFContext::new_simple(cos_theta.clamp(0.0, 1.0));
        let ctx = TemporalContext::from_base(base_ctx).with_time(t);
        let resp = self.inner.eval_at_time(&ctx);
        vec![resp.reflectance, resp.transmittance, resp.absorption].into_boxed_slice()
    }

    /// Sample reflectance values across a time range.
    ///
    /// Returns flat `[t0, r0, t1, r1, ...]` for `samples` points in `[0, duration]`.
    #[wasm_bindgen(js_name = "sampleTimeline")]
    pub fn sample_timeline(&self, duration: f64, samples: u32, cos_theta: f64) -> Box<[f64]> {
        let n = samples.max(2) as usize;
        let mut out = Vec::with_capacity(n * 2);
        for i in 0..n {
            let t = duration * i as f64 / (n - 1) as f64;
            let base_ctx = BSDFContext::new_simple(cos_theta.clamp(0.0, 1.0));
            let ctx = TemporalContext::from_base(base_ctx).with_time(t);
            let resp = self.inner.eval_at_time(&ctx);
            out.push(t);
            out.push(resp.reflectance);
        }
        out.into_boxed_slice()
    }
}

// =============================================================================
// TemporalConductorMaterial — wraps TemporalConductor
// =============================================================================

/// Time-evolving conductor material with temperature-dependent optical constants.
///
/// Models heated metals where n and k change with temperature,
/// producing colour shifts as the metal heats or cools.
#[wasm_bindgen]
pub struct TemporalConductorMaterial {
    inner: TemporalConductor,
}

#[wasm_bindgen]
impl TemporalConductorMaterial {
    /// Create a temporal conductor with custom parameters.
    ///
    /// # Arguments
    /// * `n_base` — real part of IOR at reference temperature
    /// * `k_base` — extinction coefficient at reference temperature
    /// * `roughness` — surface roughness (0=mirror, 1=rough)
    /// * `n_temp_coeff` — dn/dT (temperature coefficient for n)
    /// * `k_temp_coeff` — dk/dT (temperature coefficient for k)
    #[wasm_bindgen(constructor)]
    pub fn new(
        n_base: f64,
        k_base: f64,
        roughness: f64,
        n_temp_coeff: f64,
        k_temp_coeff: f64,
    ) -> Self {
        Self {
            inner: TemporalConductor::new(ConductorEvolution {
                n_base,
                k_base,
                roughness_base: roughness,
                n_temp_coeff,
                k_temp_coeff,
                temp_ref: 293.15,
            }),
        }
    }

    /// Preset: heated gold (reddish at high T).
    #[wasm_bindgen(js_name = "heatedGold")]
    pub fn heated_gold() -> Self {
        Self {
            inner: TemporalConductor::heated_gold(),
        }
    }

    /// Preset: heated copper.
    #[wasm_bindgen(js_name = "heatedCopper")]
    pub fn heated_copper() -> Self {
        Self {
            inner: TemporalConductor::heated_copper(),
        }
    }

    /// Evaluate BSDF at given time and temperature.
    ///
    /// # Arguments
    /// * `temperature_k` — temperature in Kelvin (293.15 = 20°C)
    /// * `cos_theta` — cosine of incident angle
    ///
    /// Returns `[reflectance, transmittance, absorption]`.
    #[wasm_bindgen(js_name = "evalAtTemperature")]
    pub fn eval_at_temperature(&self, temperature_k: f64, cos_theta: f64) -> Box<[f64]> {
        let base_ctx = BSDFContext::new_simple(cos_theta.clamp(0.0, 1.0));
        let ctx = TemporalContext::from_base(base_ctx).with_temperature(temperature_k);
        let resp = self.inner.eval_at_time(&ctx);
        vec![resp.reflectance, resp.transmittance, resp.absorption].into_boxed_slice()
    }

    /// Evaluate at room temperature (293.15 K = 20°C).
    #[wasm_bindgen(js_name = "evalAtRoomTemp")]
    pub fn eval_at_room_temp(&self, cos_theta: f64) -> Box<[f64]> {
        self.eval_at_temperature(293.15, cos_theta)
    }
}

// =============================================================================
// Free functions
// =============================================================================

/// Evaluate a drying-paint temporal material at a given time.
///
/// Returns `[reflectance, transmittance, absorption]`.
#[wasm_bindgen(js_name = "temporalDryingPaint")]
pub fn temporal_drying_paint(t: f64, cos_theta: f64) -> Box<[f64]> {
    let mat = TemporalDielectric::drying_paint();
    let base_ctx = BSDFContext::new_simple(cos_theta.clamp(0.0, 1.0));
    let ctx = TemporalContext::from_base(base_ctx).with_time(t);
    let resp = mat.eval_at_time(&ctx);
    vec![resp.reflectance, resp.transmittance, resp.absorption].into_boxed_slice()
}

/// Evaluate a soap-bubble thin-film at a given time.
///
/// Returns `[reflectance, transmittance, absorption]`.
#[wasm_bindgen(js_name = "temporalSoapBubble")]
pub fn temporal_soap_bubble(t: f64, cos_theta: f64) -> Box<[f64]> {
    let mat = TemporalThinFilm::soap_bubble();
    let base_ctx = BSDFContext::new_simple(cos_theta.clamp(0.0, 1.0));
    let ctx = TemporalContext::from_base(base_ctx).with_time(t);
    let resp = mat.eval_at_time(&ctx);
    vec![resp.reflectance, resp.transmittance, resp.absorption].into_boxed_slice()
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temporal_material_energy_conservation() {
        let mat = TemporalMaterial::drying_paint();
        let result = mat.eval_at_time(0.0, 0.7);
        let sum = result[0] + result[1] + result[2];
        assert!(
            (sum - 1.0).abs() < 0.01,
            "Energy conservation failed: sum = {}",
            sum
        );
    }

    #[test]
    fn test_temporal_continuity_at_t0() {
        // At t=0, temporal material should match static evaluation
        let mat = TemporalMaterial::drying_paint();
        let r_t0 = mat.eval_at_time(0.0, 0.7);
        let r_static = mat.eval_static(0.7);
        // Should be very close (same roughness_base at t=0)
        assert!(
            (r_t0[0] - r_static[0]).abs() < 1e-10,
            "Temporal and static differ at t=0"
        );
    }

    #[test]
    fn test_thin_film_oscillation() {
        let mat = TemporalThinFilmMaterial::soap_bubble();
        let r0 = mat.eval_at_time(0.0, 0.7);
        let r_quarter = mat.eval_at_time(0.25, 0.7);
        // Both must be physically valid
        assert!(r0[0] >= 0.0 && r0[0] <= 1.0);
        assert!(r_quarter[0] >= 0.0 && r_quarter[0] <= 1.0);
    }

    #[test]
    fn test_conductor_temperature_effect() {
        let mat = TemporalConductorMaterial::heated_gold();
        let r_cold = mat.eval_at_temperature(293.15, 0.7);
        let r_hot = mat.eval_at_temperature(800.0, 0.7);
        // Both physically valid
        assert!(r_cold[0] >= 0.0);
        assert!(r_hot[0] >= 0.0);
        // Energy conservation
        assert!((r_cold[0] + r_cold[1] + r_cold[2] - 1.0).abs() < 0.01);
        assert!((r_hot[0] + r_hot[1] + r_hot[2] - 1.0).abs() < 0.01);
    }
}

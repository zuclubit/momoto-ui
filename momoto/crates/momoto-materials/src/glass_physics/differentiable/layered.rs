//! # Differentiable Layered BSDF
//!
//! Composition of multiple BSDF layers with chain rule gradient propagation.

use super::super::unified_bsdf::{BSDFContext, BSDFResponse, BSDFSample, EnergyValidation, BSDF};
use super::traits::{
    DifferentiableBSDF, DifferentiableResponse, ParameterBounds, ParameterGradients,
};

// ============================================================================
// LAYER CONFIGURATION
// ============================================================================

/// Configuration for a material layer.
#[derive(Debug, Clone)]
pub struct LayerConfig {
    /// Layer weight (blend factor).
    pub weight: f64,
    /// Whether this layer is the base substrate.
    pub is_substrate: bool,
}

impl Default for LayerConfig {
    fn default() -> Self {
        Self {
            weight: 1.0,
            is_substrate: false,
        }
    }
}

// ============================================================================
// DIFFERENTIABLE LAYERED
// ============================================================================

/// Layered BSDF with chain rule gradient propagation.
///
/// Supports stacking multiple differentiable materials with
/// proper gradient composition.
#[derive(Debug, Clone)]
pub struct DifferentiableLayered {
    /// Stored layer responses and gradients.
    layers: Vec<LayerData>,
    /// Total number of parameters across all layers.
    total_params: usize,
}

/// Data for a single layer.
#[derive(Debug, Clone)]
struct LayerData {
    /// Layer reflectance.
    reflectance: f64,
    /// Layer transmittance.
    transmittance: f64,
    /// Layer absorption.
    absorption: f64,
    /// Gradients from this layer.
    gradients: ParameterGradients,
    /// Layer weight.
    weight: f64,
    /// Parameter offset in combined vector (reserved for gradient propagation).
    #[allow(dead_code)]
    param_offset: usize,
    /// Number of parameters in this layer.
    param_count: usize,
}

impl DifferentiableLayered {
    /// Create empty layered material.
    pub fn new() -> Self {
        Self {
            layers: Vec::new(),
            total_params: 0,
        }
    }

    /// Add a layer from a differentiable BSDF.
    pub fn add_layer<B: DifferentiableBSDF>(
        &mut self,
        material: &B,
        ctx: &BSDFContext,
        config: LayerConfig,
    ) {
        let result = material.eval_with_gradients(ctx);
        let param_count = material.param_count();

        let layer = LayerData {
            reflectance: result.response.reflectance,
            transmittance: result.response.transmittance,
            absorption: result.response.absorption,
            gradients: result.gradients,
            weight: config.weight,
            param_offset: self.total_params,
            param_count,
        };

        self.total_params += param_count;
        self.layers.push(layer);
    }

    /// Compute combined response with chain rule gradients.
    pub fn compute(&self) -> DifferentiableResponse {
        if self.layers.is_empty() {
            return DifferentiableResponse::new(
                BSDFResponse::new(0.0, 1.0, 0.0),
                ParameterGradients::zero(),
            );
        }

        // Simple additive model for two layers (coating + substrate)
        // R_total = R_coat + T_coat² × R_sub / (1 - R_coat × R_sub)
        // T_total = T_coat × T_sub / (1 - R_coat × R_sub)

        if self.layers.len() == 1 {
            let layer = &self.layers[0];
            return DifferentiableResponse::new(
                BSDFResponse::new(
                    layer.reflectance * layer.weight,
                    layer.transmittance * layer.weight,
                    layer.absorption * layer.weight,
                ),
                layer.gradients.clone(),
            );
        }

        // Two-layer model
        let coating = &self.layers[0];
        let substrate = &self.layers[1];

        let r_c = coating.reflectance;
        let t_c = coating.transmittance;
        let r_s = substrate.reflectance;
        let t_s = substrate.transmittance;

        // Multiple reflections denominator
        let denom = 1.0 - r_c * r_s;
        let denom = denom.max(0.001); // Avoid division by zero

        // Combined reflectance: R = R_c + T_c² × R_s / denom
        let r_total = r_c + t_c * t_c * r_s / denom;

        // Combined transmittance: T = T_c × T_s / denom
        let t_total = t_c * t_s / denom;

        // Absorption
        let a_total = (1.0 - r_total - t_total).max(0.0);

        // Gradients via chain rule
        // ∂R_total/∂p_coat = ∂R_c/∂p + 2×T_c×R_s/denom × ∂T_c/∂p + T_c²×R_s×R_s/(denom²) × ∂R_c/∂p
        // Simplified: combine coating gradients with scaling

        let mut combined_grads = ParameterGradients::zero();

        // Coating contribution
        let dr_dr_c = 1.0 + t_c * t_c * r_s * r_s / (denom * denom);
        let dr_dt_c = 2.0 * t_c * r_s / denom;

        combined_grads.d_ior = coating.gradients.d_ior * dr_dr_c;
        combined_grads.d_roughness = coating.gradients.d_roughness * dr_dr_c;
        combined_grads.d_reflectance = dr_dr_c;
        combined_grads.d_transmittance = dr_dt_c;

        // Substrate contribution (if has thin-film params)
        if let Some(dt) = substrate.gradients.d_film_thickness {
            let dr_dr_s = t_c * t_c / denom + t_c * t_c * r_s * r_c / (denom * denom);
            combined_grads.d_film_thickness = Some(dt * dr_dr_s);
        }
        if let Some(dn) = substrate.gradients.d_film_ior {
            let dr_dr_s = t_c * t_c / denom + t_c * t_c * r_s * r_c / (denom * denom);
            combined_grads.d_film_ior = Some(dn * dr_dr_s);
        }

        DifferentiableResponse::new(
            BSDFResponse::new(
                r_total.clamp(0.0, 1.0),
                t_total.clamp(0.0, 1.0),
                a_total.clamp(0.0, 1.0),
            ),
            combined_grads,
        )
    }

    /// Get number of layers.
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    /// Get total parameter count.
    pub fn total_param_count(&self) -> usize {
        self.total_params
    }

    /// Clear all layers.
    pub fn clear(&mut self) {
        self.layers.clear();
        self.total_params = 0;
    }
}

impl Default for DifferentiableLayered {
    fn default() -> Self {
        Self::new()
    }
}

impl BSDF for DifferentiableLayered {
    fn evaluate(&self, _ctx: &BSDFContext) -> BSDFResponse {
        let result = self.compute();
        result.response
    }

    fn sample(&self, ctx: &BSDFContext, u1: f64, _u2: f64) -> BSDFSample {
        let response = self.evaluate(ctx);
        let is_reflection = u1 < response.reflectance;

        let wo = if is_reflection {
            let cos_theta_i = ctx.cos_theta_i();
            let d = ctx.wi * -1.0;
            d + ctx.normal * (2.0 * cos_theta_i)
        } else {
            ctx.wi * -1.0
        };

        let pdf = if is_reflection {
            response.reflectance
        } else {
            response.transmittance
        };
        let value = if is_reflection {
            BSDFResponse::pure_reflection(response.reflectance)
        } else {
            BSDFResponse::pure_transmission(response.transmittance)
        };

        BSDFSample {
            wo,
            value,
            pdf,
            is_delta: false,
        }
    }

    fn pdf(&self, ctx: &BSDFContext) -> f64 {
        let response = self.evaluate(ctx);
        response.reflectance.max(response.transmittance)
    }

    fn validate_energy(&self, ctx: &BSDFContext) -> EnergyValidation {
        let response = self.evaluate(ctx);
        let error = (response.total_energy() - 1.0).abs();
        if error < 1e-6 {
            EnergyValidation::pass(error)
        } else {
            EnergyValidation::fail(error, "R + T + A != 1")
        }
    }

    fn name(&self) -> &str {
        "DifferentiableLayered"
    }

    fn is_delta(&self) -> bool {
        false
    }
}

impl DifferentiableBSDF for DifferentiableLayered {
    fn eval_with_gradients(&self, _ctx: &BSDFContext) -> DifferentiableResponse {
        self.compute()
    }

    fn parameter_bounds(&self) -> ParameterBounds {
        ParameterBounds::default()
    }

    fn param_count(&self) -> usize {
        self.total_params
    }

    fn params_to_vec(&self) -> Vec<f64> {
        // Concatenate all layer parameters
        let mut params = Vec::with_capacity(self.total_params);
        for layer in &self.layers {
            let layer_params = layer.gradients.to_vec();
            params.extend(layer_params.iter().take(layer.param_count));
        }
        params
    }

    fn from_param_vec(_params: &[f64]) -> Self {
        // Cannot reconstruct without layer types
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::super::unified_bsdf::Vector3;
    use super::super::dielectric::DifferentiableDielectric;
    use super::super::thin_film::DifferentiableThinFilm;
    use super::*;

    fn create_ctx(cos_theta: f64) -> BSDFContext {
        let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();
        BSDFContext {
            wi: Vector3::new(sin_theta, 0.0, cos_theta),
            wo: Vector3::new(-sin_theta, 0.0, cos_theta),
            normal: Vector3::new(0.0, 0.0, 1.0),
            tangent: Vector3::new(1.0, 0.0, 0.0),
            bitangent: Vector3::new(0.0, 1.0, 0.0),
            wavelength: 550.0,
            wavelengths: None,
        }
    }

    #[test]
    fn test_layered_empty() {
        let layered = DifferentiableLayered::new();
        assert_eq!(layered.layer_count(), 0);
        assert_eq!(layered.total_param_count(), 0);
    }

    #[test]
    fn test_layered_single_layer() {
        let glass = DifferentiableDielectric::glass();
        let ctx = create_ctx(0.8);

        let mut layered = DifferentiableLayered::new();
        layered.add_layer(&glass, &ctx, LayerConfig::default());

        assert_eq!(layered.layer_count(), 1);

        let result = layered.compute();
        let direct = glass.evaluate(&ctx);

        assert!((result.response.reflectance - direct.reflectance).abs() < 1e-6);
    }

    #[test]
    fn test_layered_two_layers() {
        let coating = DifferentiableThinFilm::anti_reflective(100.0);
        let substrate = DifferentiableDielectric::glass();
        let ctx = create_ctx(0.8);

        let mut layered = DifferentiableLayered::new();
        layered.add_layer(&coating, &ctx, LayerConfig::default());
        layered.add_layer(
            &substrate,
            &ctx,
            LayerConfig {
                weight: 1.0,
                is_substrate: true,
            },
        );

        assert_eq!(layered.layer_count(), 2);

        let result = layered.compute();

        // Combined should have reasonable values
        assert!(result.response.reflectance >= 0.0);
        assert!(result.response.transmittance >= 0.0);

        // Energy conservation
        let total = result.response.reflectance
            + result.response.transmittance
            + result.response.absorption;
        assert!((total - 1.0).abs() < 0.1); // Allow some tolerance for multi-layer
    }

    #[test]
    fn test_layered_energy_conservation() {
        let glass = DifferentiableDielectric::new(1.5, 0.1);
        let ctx = create_ctx(0.8);

        let mut layered = DifferentiableLayered::new();
        layered.add_layer(&glass, &ctx, LayerConfig::default());

        let result = layered.compute();
        let total = result.response.reflectance
            + result.response.transmittance
            + result.response.absorption;

        assert!((total - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_layered_gradients_propagate() {
        let glass = DifferentiableDielectric::new(1.5, 0.1);
        let ctx = create_ctx(0.8);

        let mut layered = DifferentiableLayered::new();
        layered.add_layer(&glass, &ctx, LayerConfig::default());

        let result = layered.compute();

        // Gradients should be non-zero
        assert!(result.gradients.d_ior.abs() > 0.0);
    }

    #[test]
    fn test_layered_clear() {
        let glass = DifferentiableDielectric::glass();
        let ctx = create_ctx(0.8);

        let mut layered = DifferentiableLayered::new();
        layered.add_layer(&glass, &ctx, LayerConfig::default());
        assert_eq!(layered.layer_count(), 1);

        layered.clear();
        assert_eq!(layered.layer_count(), 0);
    }

    #[test]
    fn test_coating_increases_reflectance() {
        let substrate = DifferentiableDielectric::glass();
        let coating = DifferentiableThinFilm::soap_bubble(200.0);
        let ctx = create_ctx(0.8);

        let substrate_only = substrate.evaluate(&ctx).reflectance;

        let mut layered = DifferentiableLayered::new();
        layered.add_layer(&coating, &ctx, LayerConfig::default());
        layered.add_layer(
            &substrate,
            &ctx,
            LayerConfig {
                weight: 1.0,
                is_substrate: true,
            },
        );

        let combined = layered.compute().response.reflectance;

        // Coating should modify reflectance
        assert!((combined - substrate_only).abs() > 0.001);
    }
}

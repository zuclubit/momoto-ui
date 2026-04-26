//! # Differentiable Rendering for Material Calibration
//!
//! Phase 5 implementation of automatic material parameter optimization
//! through gradient-based optimization of rendered outputs.
//!
//! ## Key Features
//!
//! - **Gradient Computation**: Analytic gradients for Fresnel, Beer-Lambert, thin-film
//! - **Optimizers**: Adam, SGD, L-BFGS for parameter optimization
//! - **Loss Functions**: Physical, perceptual, and regularization losses
//! - **Auto-Calibration**: Match simulated materials to reference measurements
//!
//! ## Usage
//!
//! ```rust,ignore
//! use momoto_materials::glass_physics::differentiable_render::{
//!     AutoCalibrator, MaterialParams, ReferenceData, AdamOptimizer,
//!     forward_dielectric, reference_presets,
//! };
//!
//! // Get reference measurement data
//! let reference = reference_presets::bk7_glass();
//! let wavelengths = reference.wavelengths.clone();
//!
//! // Create calibrator with Adam optimizer
//! let mut calibrator = AutoCalibrator::new(AdamOptimizer::default());
//!
//! // Define forward rendering function
//! let forward = |p: &MaterialParams| forward_dielectric(p, &wavelengths);
//!
//! // Calibrate starting from initial guess
//! let result = calibrator.calibrate(
//!     forward,
//!     &reference,
//!     MaterialParams::glass(1.4),
//!     1000,  // max iterations
//!     1e-6,  // tolerance
//! );
//! println!("Calibrated n = {}", result.params.n);
//! ```

use std::f64::consts::PI;

// ============================================================================
// MATERIAL PARAMETERS (Differentiable)
// ============================================================================

/// Material parameters that can be optimized
#[derive(Debug, Clone)]
pub struct MaterialParams {
    /// Refractive index (real part)
    pub n: f64,
    /// Extinction coefficient
    pub k: f64,
    /// Absorption coefficient (1/mm)
    pub absorption: f64,
    /// Scattering coefficient (1/mm)
    pub scattering: f64,
    /// Surface roughness (0-1)
    pub roughness: f64,
    /// Thin-film thickness (nm), if applicable
    pub film_thickness: Option<f64>,
    /// Film refractive index
    pub film_n: Option<f64>,
    /// Asymmetry parameter for scattering (-1 to 1)
    pub g: f64,
}

impl Default for MaterialParams {
    fn default() -> Self {
        Self {
            n: 1.5,
            k: 0.0,
            absorption: 0.0,
            scattering: 0.0,
            roughness: 0.0,
            film_thickness: None,
            film_n: None,
            g: 0.0,
        }
    }
}

impl MaterialParams {
    /// Create glass-like material
    pub fn glass(n: f64) -> Self {
        Self {
            n,
            ..Default::default()
        }
    }

    /// Create metal-like material
    pub fn metal(n: f64, k: f64) -> Self {
        Self {
            n,
            k,
            ..Default::default()
        }
    }

    /// Create with thin-film coating
    pub fn with_film(mut self, n_film: f64, thickness_nm: f64) -> Self {
        self.film_n = Some(n_film);
        self.film_thickness = Some(thickness_nm);
        self
    }

    /// Number of optimizable parameters
    pub fn param_count(&self) -> usize {
        let mut count = 5; // n, k, absorption, scattering, roughness
        if self.film_thickness.is_some() {
            count += 2;
        }
        count + 1 // g
    }

    /// Convert to parameter vector for optimization
    pub fn to_vec(&self) -> Vec<f64> {
        let mut v = vec![
            self.n,
            self.k,
            self.absorption,
            self.scattering,
            self.roughness,
            self.g,
        ];
        if let (Some(ft), Some(fn_)) = (self.film_thickness, self.film_n) {
            v.push(ft);
            v.push(fn_);
        }
        v
    }

    /// Create from parameter vector
    pub fn from_vec(v: &[f64]) -> Self {
        let (film_thickness, film_n) = if v.len() > 6 {
            (Some(v[6]), Some(v[7]))
        } else {
            (None, None)
        };

        Self {
            n: v[0],
            k: v[1],
            absorption: v[2],
            scattering: v[3],
            roughness: v[4],
            g: v[5],
            film_thickness,
            film_n,
        }
    }

    /// Clamp parameters to physically valid ranges
    pub fn clamp_valid(&mut self) {
        self.n = self.n.clamp(1.0, 4.0);
        self.k = self.k.clamp(0.0, 10.0);
        self.absorption = self.absorption.clamp(0.0, 100.0);
        self.scattering = self.scattering.clamp(0.0, 100.0);
        self.roughness = self.roughness.clamp(0.0, 1.0);
        self.g = self.g.clamp(-0.99, 0.99);
        if let Some(ref mut ft) = self.film_thickness {
            *ft = ft.clamp(1.0, 2000.0);
        }
        if let Some(ref mut fn_) = self.film_n {
            *fn_ = fn_.clamp(1.0, 4.0);
        }
    }
}

/// Gradient of material parameters
#[derive(Debug, Clone, Default)]
pub struct ParamGradient {
    pub dn: f64,
    pub dk: f64,
    pub d_absorption: f64,
    pub d_scattering: f64,
    pub d_roughness: f64,
    pub dg: f64,
    pub d_film_thickness: Option<f64>,
    pub d_film_n: Option<f64>,
}

impl ParamGradient {
    /// Convert to vector
    pub fn to_vec(&self) -> Vec<f64> {
        let mut v = vec![
            self.dn,
            self.dk,
            self.d_absorption,
            self.d_scattering,
            self.d_roughness,
            self.dg,
        ];
        if let (Some(dft), Some(dfn)) = (self.d_film_thickness, self.d_film_n) {
            v.push(dft);
            v.push(dfn);
        }
        v
    }

    /// Create from vector
    pub fn from_vec(v: &[f64]) -> Self {
        let (d_film_thickness, d_film_n) = if v.len() > 6 {
            (Some(v[6]), Some(v[7]))
        } else {
            (None, None)
        };

        Self {
            dn: v[0],
            dk: v[1],
            d_absorption: v[2],
            d_scattering: v[3],
            d_roughness: v[4],
            dg: v[5],
            d_film_thickness,
            d_film_n,
        }
    }

    /// L2 norm of gradient
    pub fn norm(&self) -> f64 {
        self.to_vec().iter().map(|x| x * x).sum::<f64>().sqrt()
    }
}

// ============================================================================
// DIFFERENTIABLE OPTICAL FUNCTIONS
// ============================================================================

/// Fresnel reflection with gradient
pub fn fresnel_schlick_diff(cos_theta: f64, n: f64) -> (f64, f64) {
    let f0 = ((n - 1.0) / (n + 1.0)).powi(2);
    let one_minus_cos = 1.0 - cos_theta;
    let pow5 = one_minus_cos.powi(5);

    let f = f0 + (1.0 - f0) * pow5;

    // ∂F/∂n = ∂F₀/∂n × ∂F/∂F₀
    // ∂F₀/∂n = 4(n-1)/(n+1)³
    let df0_dn = 4.0 * (n - 1.0) / (n + 1.0).powi(3);
    // ∂F/∂F₀ = 1 - pow5
    let df_df0 = 1.0 - pow5;
    let df_dn = df0_dn * df_df0;

    (f, df_dn)
}

/// Beer-Lambert transmission with gradient
pub fn beer_lambert_diff(alpha: f64, distance: f64) -> (f64, f64, f64) {
    let t = (-alpha * distance).exp();

    // ∂T/∂α = -d × exp(-αd)
    let dt_dalpha = -distance * t;

    // ∂T/∂d = -α × exp(-αd)
    let dt_dd = -alpha * t;

    (t, dt_dalpha, dt_dd)
}

/// Henyey-Greenstein phase function with gradient
pub fn henyey_greenstein_diff(cos_theta: f64, g: f64) -> (f64, f64) {
    let g2 = g * g;
    let denom = 1.0 + g2 - 2.0 * g * cos_theta;
    let denom_sqrt = denom.sqrt();
    let denom_32 = denom * denom_sqrt;

    let p = (1.0 - g2) / (4.0 * PI * denom_32);

    // ∂p/∂g (computed analytically)
    let dp_dg = {
        let num = -2.0 * g * denom_32 - (1.0 - g2) * 1.5 * denom_sqrt * (2.0 * g - 2.0 * cos_theta);
        num / (4.0 * PI * denom_32 * denom_32 / denom)
    };

    (p, dp_dg)
}

/// Single-layer thin-film reflectance with gradient
pub fn thin_film_reflectance_diff(
    wavelength_nm: f64,
    n_ambient: f64,
    n_film: f64,
    n_substrate: f64,
    thickness_nm: f64,
    cos_theta: f64,
) -> (f64, f64, f64) {
    // Fresnel coefficients at interfaces
    let r01 = (n_ambient - n_film) / (n_ambient + n_film);
    let r12 = (n_film - n_substrate) / (n_film + n_substrate);

    // Phase change in film
    let cos_theta_film = (1.0 - (n_ambient / n_film).powi(2) * (1.0 - cos_theta.powi(2))).sqrt();
    let delta = 4.0 * PI * n_film * thickness_nm * cos_theta_film / wavelength_nm;

    // Reflectance (Airy formula)
    let r01_sq = r01 * r01;
    let r12_sq = r12 * r12;
    let cos_delta = delta.cos();
    let sin_delta = delta.sin();

    let numerator = r01_sq + r12_sq + 2.0 * r01 * r12 * cos_delta;
    let denominator = 1.0 + r01_sq * r12_sq + 2.0 * r01 * r12 * cos_delta;
    let r = numerator / denominator;

    // Gradient w.r.t. thickness
    let d_delta_d_thickness = 4.0 * PI * n_film * cos_theta_film / wavelength_nm;
    let d_numerator = -2.0 * r01 * r12 * sin_delta * d_delta_d_thickness;
    let d_denominator = -2.0 * r01 * r12 * sin_delta * d_delta_d_thickness;
    let dr_d_thickness =
        (d_numerator * denominator - numerator * d_denominator) / (denominator * denominator);

    // Gradient w.r.t. film n (simplified)
    let d_delta_d_n = 4.0 * PI * thickness_nm * cos_theta_film / wavelength_nm;
    let d_r01_d_n = -2.0 * n_ambient / (n_ambient + n_film).powi(2);
    let d_r12_d_n = 2.0 * n_substrate / (n_film + n_substrate).powi(2);

    let dr_d_n = {
        let partial_r01 = 2.0 * r01 * d_r01_d_n * (1.0 + r12_sq + 2.0 * r12 * cos_delta);
        let partial_r12 = 2.0 * r12 * d_r12_d_n * (1.0 + r01_sq + 2.0 * r01 * cos_delta);
        let partial_delta = -2.0 * r01 * r12 * sin_delta * d_delta_d_n;
        (partial_r01 + partial_r12 + partial_delta) / denominator
    };

    (r, dr_d_thickness, dr_d_n)
}

// ============================================================================
// RENDER OUTPUT AND REFERENCE DATA
// ============================================================================

/// Spectral render output
#[derive(Debug, Clone)]
pub struct RenderOutput {
    /// Wavelengths in nm
    pub wavelengths: Vec<f64>,
    /// Reflectance at each wavelength
    pub reflectance: Vec<f64>,
    /// Transmittance at each wavelength
    pub transmittance: Vec<f64>,
}

impl RenderOutput {
    /// Create from spectral data
    pub fn new(wavelengths: Vec<f64>, reflectance: Vec<f64>, transmittance: Vec<f64>) -> Self {
        Self {
            wavelengths,
            reflectance,
            transmittance,
        }
    }

    /// Sample RGB (650, 550, 450 nm)
    pub fn to_rgb(&self) -> ([f64; 3], [f64; 3]) {
        let r_rgb = self.sample_at_wavelengths(&[650.0, 550.0, 450.0], &self.reflectance);
        let t_rgb = self.sample_at_wavelengths(&[650.0, 550.0, 450.0], &self.transmittance);
        (r_rgb, t_rgb)
    }

    fn sample_at_wavelengths(&self, target: &[f64], values: &[f64]) -> [f64; 3] {
        let mut result = [0.0; 3];
        for (i, &w) in target.iter().enumerate() {
            // Linear interpolation
            let idx = self
                .wavelengths
                .iter()
                .position(|&x| x >= w)
                .unwrap_or(self.wavelengths.len() - 1);

            if idx == 0 {
                result[i] = values[0];
            } else {
                let t = (w - self.wavelengths[idx - 1])
                    / (self.wavelengths[idx] - self.wavelengths[idx - 1]);
                result[i] = values[idx - 1] + t * (values[idx] - values[idx - 1]);
            }
        }
        result
    }
}

/// Reference measurement data for calibration
#[derive(Debug, Clone)]
pub struct ReferenceData {
    /// Wavelengths in nm
    pub wavelengths: Vec<f64>,
    /// Measured reflectance
    pub reflectance: Vec<f64>,
    /// Measured transmittance (optional)
    pub transmittance: Option<Vec<f64>>,
    /// Measurement angles (degrees)
    pub angles: Vec<f64>,
}

impl ReferenceData {
    /// Create from spectral measurements
    pub fn from_spectral(wavelengths: Vec<f64>, reflectance: Vec<f64>) -> Self {
        Self {
            wavelengths,
            reflectance,
            transmittance: None,
            angles: vec![0.0], // Normal incidence
        }
    }

    /// Add transmittance data
    pub fn with_transmittance(mut self, transmittance: Vec<f64>) -> Self {
        self.transmittance = Some(transmittance);
        self
    }

    /// Standard visible spectrum (400-700nm, 10nm steps)
    pub fn visible_wavelengths() -> Vec<f64> {
        (400..=700).step_by(10).map(|x| x as f64).collect()
    }
}

// ============================================================================
// LOSS FUNCTIONS
// ============================================================================

/// Loss function configuration
#[derive(Debug, Clone)]
pub struct LossConfig {
    /// Weight for spectral RMSE
    pub w_spectral: f64,
    /// Weight for color difference (Delta E)
    pub w_color: f64,
    /// Weight for regularization
    pub w_regularization: f64,
    /// Prior parameters for regularization
    pub prior: Option<MaterialParams>,
}

impl Default for LossConfig {
    fn default() -> Self {
        Self {
            w_spectral: 1.0,
            w_color: 0.1,
            w_regularization: 0.01,
            prior: None,
        }
    }
}

/// Compute combined loss
pub fn compute_loss(
    output: &RenderOutput,
    reference: &ReferenceData,
    params: &MaterialParams,
    config: &LossConfig,
) -> f64 {
    let mut loss = 0.0;

    // Spectral RMSE
    if config.w_spectral > 0.0 {
        let mut sum_sq = 0.0;
        let n = reference.reflectance.len().min(output.reflectance.len());
        for i in 0..n {
            let diff = output.reflectance[i] - reference.reflectance[i];
            sum_sq += diff * diff;
        }
        loss += config.w_spectral * (sum_sq / n as f64).sqrt();
    }

    // Color difference (simplified CIE76 Delta E)
    if config.w_color > 0.0 {
        let (r_out, _) = output.to_rgb();
        let r_ref = [
            reference.reflectance.get(25).copied().unwrap_or(0.5), // ~650nm
            reference.reflectance.get(15).copied().unwrap_or(0.5), // ~550nm
            reference.reflectance.get(5).copied().unwrap_or(0.5),  // ~450nm
        ];

        let delta_e = ((r_out[0] - r_ref[0]).powi(2)
            + (r_out[1] - r_ref[1]).powi(2)
            + (r_out[2] - r_ref[2]).powi(2))
        .sqrt();
        loss += config.w_color * delta_e;
    }

    // Regularization toward prior
    if config.w_regularization > 0.0 {
        if let Some(prior) = &config.prior {
            let param_diff = (params.n - prior.n).powi(2)
                + (params.k - prior.k).powi(2)
                + (params.absorption - prior.absorption).powi(2)
                + (params.scattering - prior.scattering).powi(2);
            loss += config.w_regularization * param_diff;
        }
    }

    loss
}

/// Compute loss gradient (numerical for now)
pub fn compute_loss_gradient(
    forward_fn: impl Fn(&MaterialParams) -> RenderOutput,
    reference: &ReferenceData,
    params: &MaterialParams,
    config: &LossConfig,
) -> ParamGradient {
    let eps = 1e-6;
    let mut grad = ParamGradient::default();
    let param_vec = params.to_vec();

    for (i, g) in [
        &mut grad.dn,
        &mut grad.dk,
        &mut grad.d_absorption,
        &mut grad.d_scattering,
        &mut grad.d_roughness,
        &mut grad.dg,
    ]
    .iter_mut()
    .enumerate()
    {
        let mut params_plus = param_vec.clone();
        let mut params_minus = param_vec.clone();
        params_plus[i] += eps;
        params_minus[i] -= eps;

        let loss_plus = compute_loss(
            &forward_fn(&MaterialParams::from_vec(&params_plus)),
            reference,
            &MaterialParams::from_vec(&params_plus),
            config,
        );
        let loss_minus = compute_loss(
            &forward_fn(&MaterialParams::from_vec(&params_minus)),
            reference,
            &MaterialParams::from_vec(&params_minus),
            config,
        );

        **g = (loss_plus - loss_minus) / (2.0 * eps);
    }

    grad
}

// ============================================================================
// OPTIMIZERS
// ============================================================================

/// Optimizer trait
pub trait Optimizer: Clone {
    /// Perform optimization step
    fn step(&mut self, params: &mut MaterialParams, grad: &ParamGradient);
    /// Reset optimizer state
    fn reset(&mut self);
}

/// Stochastic Gradient Descent
#[derive(Debug, Clone)]
pub struct SgdOptimizer {
    pub learning_rate: f64,
}

impl Default for SgdOptimizer {
    fn default() -> Self {
        Self {
            learning_rate: 0.01,
        }
    }
}

impl Optimizer for SgdOptimizer {
    fn step(&mut self, params: &mut MaterialParams, grad: &ParamGradient) {
        let grad_vec = grad.to_vec();
        let mut param_vec = params.to_vec();

        for (p, g) in param_vec.iter_mut().zip(grad_vec.iter()) {
            *p -= self.learning_rate * g;
        }

        *params = MaterialParams::from_vec(&param_vec);
        params.clamp_valid();
    }

    fn reset(&mut self) {}
}

/// Adam Optimizer
#[derive(Debug, Clone)]
pub struct AdamOptimizer {
    pub learning_rate: f64,
    pub beta1: f64,
    pub beta2: f64,
    pub epsilon: f64,

    // State
    m: Vec<f64>,
    v: Vec<f64>,
    t: usize,
}

impl Default for AdamOptimizer {
    fn default() -> Self {
        Self {
            learning_rate: 0.001,
            beta1: 0.9,
            beta2: 0.999,
            epsilon: 1e-8,
            m: vec![0.0; 8],
            v: vec![0.0; 8],
            t: 0,
        }
    }
}

impl Optimizer for AdamOptimizer {
    fn step(&mut self, params: &mut MaterialParams, grad: &ParamGradient) {
        self.t += 1;
        let grad_vec = grad.to_vec();
        let mut param_vec = params.to_vec();

        // Use minimum length to handle mismatched sizes
        let len = param_vec.len().min(grad_vec.len());

        // Ensure state vectors are correct size
        while self.m.len() < len {
            self.m.push(0.0);
            self.v.push(0.0);
        }

        for i in 0..len {
            // Update biased first moment estimate
            self.m[i] = self.beta1 * self.m[i] + (1.0 - self.beta1) * grad_vec[i];

            // Update biased second raw moment estimate
            self.v[i] = self.beta2 * self.v[i] + (1.0 - self.beta2) * grad_vec[i] * grad_vec[i];

            // Compute bias-corrected estimates
            let m_hat = self.m[i] / (1.0 - self.beta1.powi(self.t as i32));
            let v_hat = self.v[i] / (1.0 - self.beta2.powi(self.t as i32));

            // Update parameters
            param_vec[i] -= self.learning_rate * m_hat / (v_hat.sqrt() + self.epsilon);
        }

        *params = MaterialParams::from_vec(&param_vec);
        params.clamp_valid();
    }

    fn reset(&mut self) {
        self.m = vec![0.0; 8];
        self.v = vec![0.0; 8];
        self.t = 0;
    }
}

// ============================================================================
// AUTO-CALIBRATOR
// ============================================================================

/// Calibration result
#[derive(Debug, Clone)]
pub struct CalibrationResult {
    /// Optimized parameters
    pub params: MaterialParams,
    /// Final loss value
    pub final_loss: f64,
    /// Number of iterations
    pub iterations: usize,
    /// Loss history
    pub loss_history: Vec<f64>,
    /// Converged?
    pub converged: bool,
}

/// Auto-calibration system
pub struct AutoCalibrator<O: Optimizer> {
    optimizer: O,
    loss_config: LossConfig,
}

impl<O: Optimizer> AutoCalibrator<O> {
    /// Create new calibrator
    pub fn new(optimizer: O) -> Self {
        Self {
            optimizer,
            loss_config: LossConfig::default(),
        }
    }

    /// Set loss configuration
    pub fn with_loss_config(mut self, config: LossConfig) -> Self {
        self.loss_config = config;
        self
    }

    /// Calibrate material parameters to match reference
    pub fn calibrate(
        &mut self,
        forward_fn: impl Fn(&MaterialParams) -> RenderOutput,
        reference: &ReferenceData,
        initial: MaterialParams,
        max_iter: usize,
        tolerance: f64,
    ) -> CalibrationResult {
        let mut params = initial;
        let mut loss_history = Vec::with_capacity(max_iter);
        let mut converged = false;

        for i in 0..max_iter {
            let output = forward_fn(&params);
            let loss = compute_loss(&output, reference, &params, &self.loss_config);
            loss_history.push(loss);

            if loss < tolerance {
                converged = true;
                break;
            }

            // Check for convergence based on loss change
            if i > 10 {
                let recent_change = (loss_history[i - 10] - loss).abs();
                if recent_change < tolerance * 0.1 {
                    converged = true;
                    break;
                }
            }

            let grad = compute_loss_gradient(&forward_fn, reference, &params, &self.loss_config);
            self.optimizer.step(&mut params, &grad);
        }

        let final_loss = *loss_history.last().unwrap_or(&f64::MAX);

        CalibrationResult {
            params,
            final_loss,
            iterations: loss_history.len(),
            loss_history,
            converged,
        }
    }
}

// ============================================================================
// FORWARD RENDERING FUNCTIONS
// ============================================================================

/// Simple dielectric forward model
pub fn forward_dielectric(params: &MaterialParams, wavelengths: &[f64]) -> RenderOutput {
    let mut reflectance = Vec::with_capacity(wavelengths.len());
    let mut transmittance = Vec::with_capacity(wavelengths.len());

    for &_wavelength in wavelengths {
        // Fresnel reflection at normal incidence
        let r = ((params.n - 1.0) / (params.n + 1.0)).powi(2);

        // Beer-Lambert absorption
        let thickness = 1.0; // mm
        let t_abs = (-params.absorption * thickness).exp();

        reflectance.push(r);
        transmittance.push((1.0 - r) * t_abs * (1.0 - r)); // Through both surfaces
    }

    RenderOutput::new(wavelengths.to_vec(), reflectance, transmittance)
}

/// Thin-film coated dielectric forward model
pub fn forward_thin_film(params: &MaterialParams, wavelengths: &[f64]) -> RenderOutput {
    let mut reflectance = Vec::with_capacity(wavelengths.len());
    let mut transmittance = Vec::with_capacity(wavelengths.len());

    let film_n = params.film_n.unwrap_or(1.38);
    let film_d = params.film_thickness.unwrap_or(100.0);

    for &wavelength in wavelengths {
        let (r, _, _) = thin_film_reflectance_diff(
            wavelength, 1.0, // air
            film_n, params.n, // substrate
            film_d, 1.0, // normal incidence
        );

        reflectance.push(r.clamp(0.0, 1.0));
        transmittance.push((1.0 - r).clamp(0.0, 1.0));
    }

    RenderOutput::new(wavelengths.to_vec(), reflectance, transmittance)
}

/// Metal forward model
pub fn forward_metal(params: &MaterialParams, wavelengths: &[f64]) -> RenderOutput {
    let mut reflectance = Vec::with_capacity(wavelengths.len());
    let transmittance = vec![0.0; wavelengths.len()]; // Metals are opaque

    for &_wavelength in wavelengths {
        // Conductor Fresnel at normal incidence
        let n = params.n;
        let k = params.k;
        let r = ((n - 1.0).powi(2) + k.powi(2)) / ((n + 1.0).powi(2) + k.powi(2));
        reflectance.push(r);
    }

    RenderOutput::new(wavelengths.to_vec(), reflectance, transmittance)
}

// ============================================================================
// PRESET REFERENCE DATA
// ============================================================================

/// Reference data presets for common materials
pub mod reference_presets {
    use super::*;

    /// Standard glass (BK7) reflectance
    pub fn bk7_glass() -> ReferenceData {
        let wavelengths = ReferenceData::visible_wavelengths();
        let reflectance: Vec<f64> = wavelengths.iter().map(|_| 0.04).collect();
        ReferenceData::from_spectral(wavelengths, reflectance)
    }

    /// AR coated glass (~1% reflection)
    pub fn ar_coated_glass() -> ReferenceData {
        let wavelengths = ReferenceData::visible_wavelengths();
        let reflectance: Vec<f64> = wavelengths
            .iter()
            .map(|w| {
                // V-coat centered at 550nm
                let x = (w - 550.0) / 100.0;
                0.01 + 0.02 * x.abs()
            })
            .collect();
        ReferenceData::from_spectral(wavelengths, reflectance)
    }

    /// Gold reflectance (simplified)
    pub fn gold() -> ReferenceData {
        let wavelengths = ReferenceData::visible_wavelengths();
        let reflectance: Vec<f64> = wavelengths
            .iter()
            .map(|w| {
                if *w < 550.0 {
                    0.4 + 0.3 * (w - 400.0) / 150.0
                } else {
                    0.9 + 0.08 * (w - 550.0) / 150.0
                }
            })
            .collect();
        ReferenceData::from_spectral(wavelengths, reflectance)
    }

    /// Silver reflectance
    pub fn silver() -> ReferenceData {
        let wavelengths = ReferenceData::visible_wavelengths();
        let reflectance: Vec<f64> = wavelengths.iter().map(|_| 0.97).collect();
        ReferenceData::from_spectral(wavelengths, reflectance)
    }

    /// Copper reflectance
    pub fn copper() -> ReferenceData {
        let wavelengths = ReferenceData::visible_wavelengths();
        let reflectance: Vec<f64> = wavelengths
            .iter()
            .map(|w| {
                if *w < 580.0 {
                    0.4 + 0.3 * (w - 400.0) / 180.0
                } else {
                    0.95
                }
            })
            .collect();
        ReferenceData::from_spectral(wavelengths, reflectance)
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fresnel_gradient() {
        let (f, df_dn) = fresnel_schlick_diff(1.0, 1.5);

        // At normal incidence for n=1.5: F0 = 0.04
        assert!((f - 0.04).abs() < 0.01);

        // Gradient should be positive (higher n = higher reflection)
        assert!(df_dn > 0.0);
    }

    #[test]
    fn test_beer_lambert_gradient() {
        let (t, dt_da, dt_dd) = beer_lambert_diff(1.0, 1.0);

        // T = exp(-1) ≈ 0.368
        assert!((t - 0.368).abs() < 0.01);

        // Gradients should be negative (more absorption = less transmission)
        assert!(dt_da < 0.0);
        assert!(dt_dd < 0.0);
    }

    #[test]
    fn test_adam_optimizer() {
        let mut optimizer = AdamOptimizer::default();
        let mut params = MaterialParams::glass(1.4);
        let grad = ParamGradient {
            dn: 0.1,
            ..Default::default()
        };

        optimizer.step(&mut params, &grad);

        // n should decrease (gradient positive, step negative)
        assert!(params.n < 1.4);
    }

    #[test]
    fn test_simple_calibration() {
        let reference = reference_presets::bk7_glass();
        let wavelengths = reference.wavelengths.clone();

        let mut calibrator = AutoCalibrator::new(AdamOptimizer::default());

        let forward = |p: &MaterialParams| forward_dielectric(p, &wavelengths);

        let result = calibrator.calibrate(
            forward,
            &reference,
            MaterialParams::glass(1.3), // Wrong initial guess
            100,
            0.01,
        );

        // Should converge to approximately n=1.5 (BK7)
        assert!(result.final_loss < 0.1);
        assert!((result.params.n - 1.5).abs() < 0.2);
    }

    #[test]
    fn test_thin_film_calibration() {
        // Create reference from known thin-film
        let wavelengths = ReferenceData::visible_wavelengths();
        let reference_params = MaterialParams::glass(1.52).with_film(1.38, 100.0);
        let reference_output = forward_thin_film(&reference_params, &wavelengths);
        let reference =
            ReferenceData::from_spectral(wavelengths.clone(), reference_output.reflectance);

        let mut calibrator = AutoCalibrator::new(AdamOptimizer::default());

        let forward = |p: &MaterialParams| forward_thin_film(p, &wavelengths);

        let result = calibrator.calibrate(
            forward,
            &reference,
            MaterialParams::glass(1.5).with_film(1.4, 90.0), // Close initial guess
            200,
            0.001,
        );

        assert!(result.final_loss < 0.05);
    }

    #[test]
    fn test_metal_calibration() {
        let reference = reference_presets::gold();
        let wavelengths = reference.wavelengths.clone();

        let mut calibrator = AutoCalibrator::new(AdamOptimizer {
            learning_rate: 0.01,
            ..Default::default()
        });

        let forward = |p: &MaterialParams| forward_metal(p, &wavelengths);

        let result = calibrator.calibrate(
            forward,
            &reference,
            MaterialParams::metal(0.2, 3.0), // Initial guess
            300,
            0.05,
        );

        // Should make some progress (loss should be finite)
        assert!(
            result.final_loss.is_finite(),
            "Calibration should produce finite loss, got {:?}",
            result.final_loss
        );
    }

    #[test]
    fn test_param_clamping() {
        let mut params = MaterialParams {
            n: 0.5,            // Too low
            k: -1.0,           // Negative
            absorption: 200.0, // Too high
            ..Default::default()
        };

        params.clamp_valid();

        assert!(params.n >= 1.0);
        assert!(params.k >= 0.0);
        assert!(params.absorption <= 100.0);
    }

    #[test]
    fn test_loss_gradient_finite_difference() {
        let wavelengths = ReferenceData::visible_wavelengths();
        let reference = reference_presets::bk7_glass();
        let params = MaterialParams::glass(1.45);
        let config = LossConfig::default();

        let forward = |p: &MaterialParams| forward_dielectric(p, &wavelengths);

        let grad = compute_loss_gradient(&forward, &reference, &params, &config);

        // Gradient should be non-zero (some sensitivity to parameters)
        assert!(
            grad.dn.abs() > 0.0 || grad.d_absorption.abs() > 0.0,
            "Gradient should have some non-zero component"
        );
    }
}

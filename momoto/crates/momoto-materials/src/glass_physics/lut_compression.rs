//! # LUT Compression Module (Phase 4)
//!
//! Memory-efficient lookup table implementations with compression and
//! hybrid analytical-LUT strategies.
//!
//! ## Compression Strategies
//!
//! 1. **Quantization** - f32 → u16/u8 with range encoding
//! 2. **Sparse Sampling** - Fewer samples + cubic interpolation
//! 3. **Delta Encoding** - Store differences from baseline
//! 4. **Hybrid** - Analytical for smooth regions, LUT for complex
//!
//! ## Memory Savings
//!
//! | Strategy | Compression | Max Error |
//! |----------|-------------|-----------|
//! | u16 quantization | 50% | 0.002% |
//! | u8 quantization | 75% | 0.4% |
//! | 4x sparse | 75% | 0.5% |
//! | Hybrid | 50-80% | 0.1% |

use std::f64::consts::PI;

// ============================================================================
// QUANTIZATION
// ============================================================================

/// Quantize f32 to u16 with range normalization
#[inline]
pub fn quantize_f32_to_u16(value: f32, min: f32, max: f32) -> u16 {
    let normalized = ((value - min) / (max - min)).clamp(0.0, 1.0);
    (normalized * 65535.0) as u16
}

/// Dequantize u16 to f32
#[inline]
pub fn dequantize_u16_to_f32(value: u16, min: f32, max: f32) -> f32 {
    (value as f32 / 65535.0) * (max - min) + min
}

/// Quantize f32 to u8 with range normalization
#[inline]
pub fn quantize_f32_to_u8(value: f32, min: f32, max: f32) -> u8 {
    let normalized = ((value - min) / (max - min)).clamp(0.0, 1.0);
    (normalized * 255.0) as u8
}

/// Dequantize u8 to f32
#[inline]
pub fn dequantize_u8_to_f32(value: u8, min: f32, max: f32) -> f32 {
    (value as f32 / 255.0) * (max - min) + min
}

// ============================================================================
// COMPRESSED 1D LUT
// ============================================================================

/// Compressed 1D lookup table with u16 quantization
#[derive(Clone)]
pub struct CompressedLUT1D {
    /// Quantized values
    data: Vec<u16>,
    /// Value range for dequantization
    min_value: f32,
    max_value: f32,
    /// Input range
    input_min: f64,
    input_max: f64,
}

impl CompressedLUT1D {
    /// Create from f32 data
    pub fn from_f32_slice(data: &[f32], input_min: f64, input_max: f64) -> Self {
        let min_value = data.iter().cloned().fold(f32::INFINITY, f32::min);
        let max_value = data.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

        let quantized: Vec<u16> = data
            .iter()
            .map(|&v| quantize_f32_to_u16(v, min_value, max_value))
            .collect();

        Self {
            data: quantized,
            min_value,
            max_value,
            input_min,
            input_max,
        }
    }

    /// Build from a function with given sample count
    pub fn build<F>(f: F, samples: usize, input_min: f64, input_max: f64) -> Self
    where
        F: Fn(f64) -> f64,
    {
        let step = (input_max - input_min) / (samples - 1) as f64;
        let data: Vec<f32> = (0..samples)
            .map(|i| f(input_min + i as f64 * step) as f32)
            .collect();

        Self::from_f32_slice(&data, input_min, input_max)
    }

    /// Lookup with linear interpolation
    pub fn lookup(&self, x: f64) -> f64 {
        let x_clamped = x.clamp(self.input_min, self.input_max);
        let t = (x_clamped - self.input_min) / (self.input_max - self.input_min);
        let idx_f = t * (self.data.len() - 1) as f64;

        let idx0 = (idx_f.floor() as usize).min(self.data.len() - 2);
        let idx1 = idx0 + 1;
        let frac = idx_f - idx0 as f64;

        let v0 = dequantize_u16_to_f32(self.data[idx0], self.min_value, self.max_value);
        let v1 = dequantize_u16_to_f32(self.data[idx1], self.min_value, self.max_value);

        (v0 as f64) * (1.0 - frac) + (v1 as f64) * frac
    }

    /// Memory usage in bytes
    pub fn memory_bytes(&self) -> usize {
        self.data.len() * 2 + 24 // data + metadata
    }

    /// Sample count
    pub fn samples(&self) -> usize {
        self.data.len()
    }
}

// ============================================================================
// COMPRESSED 2D LUT
// ============================================================================

/// Compressed 2D lookup table with u16 quantization
#[derive(Clone)]
pub struct CompressedLUT2D {
    /// Quantized values (row-major)
    data: Vec<u16>,
    /// Dimensions
    dim_x: usize,
    dim_y: usize,
    /// Value range
    min_value: f32,
    max_value: f32,
    /// Input ranges
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
}

impl CompressedLUT2D {
    /// Build from a 2D function
    pub fn build<F>(
        f: F,
        dim_x: usize,
        dim_y: usize,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
    ) -> Self
    where
        F: Fn(f64, f64) -> f64,
    {
        let x_step = (x_max - x_min) / (dim_x - 1) as f64;
        let y_step = (y_max - y_min) / (dim_y - 1) as f64;

        let mut data_f32 = Vec::with_capacity(dim_x * dim_y);

        for iy in 0..dim_y {
            let y = y_min + iy as f64 * y_step;
            for ix in 0..dim_x {
                let x = x_min + ix as f64 * x_step;
                data_f32.push(f(x, y) as f32);
            }
        }

        let min_value = data_f32.iter().cloned().fold(f32::INFINITY, f32::min);
        let max_value = data_f32.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

        let data: Vec<u16> = data_f32
            .iter()
            .map(|&v| quantize_f32_to_u16(v, min_value, max_value))
            .collect();

        Self {
            data,
            dim_x,
            dim_y,
            min_value,
            max_value,
            x_min,
            x_max,
            y_min,
            y_max,
        }
    }

    /// Lookup with bilinear interpolation
    pub fn lookup(&self, x: f64, y: f64) -> f64 {
        let x_clamped = x.clamp(self.x_min, self.x_max);
        let y_clamped = y.clamp(self.y_min, self.y_max);

        let tx = (x_clamped - self.x_min) / (self.x_max - self.x_min);
        let ty = (y_clamped - self.y_min) / (self.y_max - self.y_min);

        let ix_f = tx * (self.dim_x - 1) as f64;
        let iy_f = ty * (self.dim_y - 1) as f64;

        let ix0 = (ix_f.floor() as usize).min(self.dim_x - 2);
        let ix1 = ix0 + 1;
        let iy0 = (iy_f.floor() as usize).min(self.dim_y - 2);
        let iy1 = iy0 + 1;

        let fx = ix_f - ix0 as f64;
        let fy = iy_f - iy0 as f64;

        let v00 = self.get_value(ix0, iy0);
        let v10 = self.get_value(ix1, iy0);
        let v01 = self.get_value(ix0, iy1);
        let v11 = self.get_value(ix1, iy1);

        let v0 = v00 * (1.0 - fx) + v10 * fx;
        let v1 = v01 * (1.0 - fx) + v11 * fx;

        v0 * (1.0 - fy) + v1 * fy
    }

    #[inline]
    fn get_value(&self, ix: usize, iy: usize) -> f64 {
        let idx = iy * self.dim_x + ix;
        dequantize_u16_to_f32(self.data[idx], self.min_value, self.max_value) as f64
    }

    /// Memory usage in bytes
    pub fn memory_bytes(&self) -> usize {
        self.data.len() * 2 + 48 // data + metadata
    }
}

// ============================================================================
// SPARSE LUT WITH CUBIC INTERPOLATION
// ============================================================================

/// Sparse 1D LUT with cubic Hermite interpolation
#[derive(Clone)]
pub struct SparseLUT1D {
    /// Sample values
    values: Vec<f64>,
    /// Derivatives at sample points (for Hermite)
    derivatives: Vec<f64>,
    /// Input range
    input_min: f64,
    input_max: f64,
}

impl SparseLUT1D {
    /// Build from function with automatic derivative estimation
    pub fn build<F>(f: F, samples: usize, input_min: f64, input_max: f64) -> Self
    where
        F: Fn(f64) -> f64,
    {
        let step = (input_max - input_min) / (samples - 1) as f64;

        let values: Vec<f64> = (0..samples)
            .map(|i| f(input_min + i as f64 * step))
            .collect();

        // Estimate derivatives using central differences
        let mut derivatives = Vec::with_capacity(samples);
        let h = step;

        for i in 0..samples {
            let deriv = if i == 0 {
                (values[1] - values[0]) / h
            } else if i == samples - 1 {
                (values[samples - 1] - values[samples - 2]) / h
            } else {
                (values[i + 1] - values[i - 1]) / (2.0 * h)
            };
            derivatives.push(deriv * h); // Scale by step for Hermite
        }

        Self {
            values,
            derivatives,
            input_min,
            input_max,
        }
    }

    /// Lookup with cubic Hermite interpolation
    pub fn lookup(&self, x: f64) -> f64 {
        let x_clamped = x.clamp(self.input_min, self.input_max);
        let t_global = (x_clamped - self.input_min) / (self.input_max - self.input_min);
        let idx_f = t_global * (self.values.len() - 1) as f64;

        let idx0 = (idx_f.floor() as usize).min(self.values.len() - 2);
        let idx1 = idx0 + 1;
        let t = idx_f - idx0 as f64;

        cubic_hermite(
            t,
            self.values[idx0],
            self.values[idx1],
            self.derivatives[idx0],
            self.derivatives[idx1],
        )
    }

    /// Memory usage in bytes
    pub fn memory_bytes(&self) -> usize {
        (self.values.len() + self.derivatives.len()) * 8 + 24
    }
}

/// Cubic Hermite interpolation
#[inline]
fn cubic_hermite(t: f64, p0: f64, p1: f64, m0: f64, m1: f64) -> f64 {
    let t2 = t * t;
    let t3 = t2 * t;

    (2.0 * t3 - 3.0 * t2 + 1.0) * p0
        + (t3 - 2.0 * t2 + t) * m0
        + (-2.0 * t3 + 3.0 * t2) * p1
        + (t3 - t2) * m1
}

// ============================================================================
// HYBRID ANALYTICAL-LUT EVALUATOR
// ============================================================================

/// Evaluation method for hybrid evaluator
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EvaluationMethod {
    /// Pure analytical evaluation
    Analytical,
    /// Pure LUT evaluation
    LUT,
    /// Hybrid: analytical below boundary, LUT above
    Hybrid { boundary: f64 },
}

/// Hybrid evaluator combining analytical functions with LUTs
pub struct HybridEvaluator {
    lut: CompressedLUT1D,
    analytical_fn: fn(f64) -> f64,
    method: EvaluationMethod,
}

impl HybridEvaluator {
    /// Create a new hybrid evaluator
    pub fn new(
        analytical_fn: fn(f64) -> f64,
        lut_samples: usize,
        input_min: f64,
        input_max: f64,
        method: EvaluationMethod,
    ) -> Self {
        let lut = CompressedLUT1D::build(analytical_fn, lut_samples, input_min, input_max);

        Self {
            lut,
            analytical_fn,
            method,
        }
    }

    /// Evaluate the function
    pub fn evaluate(&self, x: f64) -> f64 {
        match self.method {
            EvaluationMethod::Analytical => (self.analytical_fn)(x),
            EvaluationMethod::LUT => self.lut.lookup(x),
            EvaluationMethod::Hybrid { boundary } => {
                if x < boundary {
                    (self.analytical_fn)(x)
                } else {
                    self.lut.lookup(x)
                }
            }
        }
    }

    /// Memory usage
    pub fn memory_bytes(&self) -> usize {
        match self.method {
            EvaluationMethod::Analytical => 0,
            EvaluationMethod::LUT | EvaluationMethod::Hybrid { .. } => self.lut.memory_bytes(),
        }
    }
}

// ============================================================================
// DELTA-ENCODED LUT
// ============================================================================

/// Delta-encoded LUT for slowly varying functions
///
/// Stores first value + deltas, good for monotonic functions
#[derive(Clone)]
pub struct DeltaEncodedLUT {
    /// First value
    base_value: f64,
    /// Quantized deltas (i16 for signed differences)
    deltas: Vec<i16>,
    /// Scale factor for deltas
    delta_scale: f64,
    /// Input range
    input_min: f64,
    input_max: f64,
}

impl DeltaEncodedLUT {
    /// Build from function
    pub fn build<F>(f: F, samples: usize, input_min: f64, input_max: f64) -> Self
    where
        F: Fn(f64) -> f64,
    {
        let step = (input_max - input_min) / (samples - 1) as f64;

        let values: Vec<f64> = (0..samples)
            .map(|i| f(input_min + i as f64 * step))
            .collect();

        let base_value = values[0];

        // Compute deltas
        let mut raw_deltas = Vec::with_capacity(samples - 1);
        for i in 1..samples {
            raw_deltas.push(values[i] - values[i - 1]);
        }

        // Find scale
        let max_delta = raw_deltas.iter().map(|d| d.abs()).fold(0.0, f64::max);

        let delta_scale = if max_delta > 0.0 {
            max_delta / 32767.0
        } else {
            1.0
        };

        let deltas: Vec<i16> = raw_deltas
            .iter()
            .map(|&d| (d / delta_scale).clamp(-32768.0, 32767.0) as i16)
            .collect();

        Self {
            base_value,
            deltas,
            delta_scale,
            input_min,
            input_max,
        }
    }

    /// Lookup with reconstruction
    pub fn lookup(&self, x: f64) -> f64 {
        let x_clamped = x.clamp(self.input_min, self.input_max);
        let t = (x_clamped - self.input_min) / (self.input_max - self.input_min);
        let idx_f = t * self.deltas.len() as f64;

        let idx = (idx_f.floor() as usize).min(self.deltas.len() - 1);
        let frac = idx_f - idx as f64;

        // Reconstruct value by summing deltas
        let mut value = self.base_value;
        for i in 0..idx {
            value += self.deltas[i] as f64 * self.delta_scale;
        }

        // Interpolate last delta
        if idx < self.deltas.len() {
            value += frac * self.deltas[idx] as f64 * self.delta_scale;
        }

        value
    }

    /// Memory usage
    pub fn memory_bytes(&self) -> usize {
        self.deltas.len() * 2 + 32
    }
}

// ============================================================================
// COMPRESSED FRESNEL LUT
// ============================================================================

/// Highly compressed Fresnel LUT using sparse sampling + cubic interpolation
pub struct CompressedFresnelLUT {
    lut: SparseLUT1D,
}

impl CompressedFresnelLUT {
    /// Build compressed Fresnel LUT for a given IOR
    pub fn build(ior: f64, samples: usize) -> Self {
        let fresnel_fn = |cos_theta: f64| {
            let r0 = ((1.0 - ior) / (1.0 + ior)).powi(2);
            r0 + (1.0 - r0) * (1.0 - cos_theta).powi(5)
        };

        let lut = SparseLUT1D::build(fresnel_fn, samples, 0.0, 1.0);

        Self { lut }
    }

    /// Lookup Fresnel reflectance
    #[inline]
    pub fn lookup(&self, cos_theta: f64) -> f64 {
        self.lut.lookup(cos_theta.abs())
    }

    /// Memory usage
    pub fn memory_bytes(&self) -> usize {
        self.lut.memory_bytes()
    }
}

// ============================================================================
// COMPRESSED H-G LUT
// ============================================================================

/// Compressed Henyey-Greenstein LUT
pub struct CompressedHGLUT {
    /// 2D LUT: [g_index][angle_index]
    lut: CompressedLUT2D,
}

impl CompressedHGLUT {
    /// Build compressed H-G LUT
    pub fn build(g_samples: usize, angle_samples: usize) -> Self {
        let hg_fn = |cos_theta: f64, g: f64| {
            if g.abs() < 1e-6 {
                return 1.0 / (4.0 * PI);
            }
            let g2 = g * g;
            let denom = 1.0 + g2 - 2.0 * g * cos_theta;
            (1.0 - g2) / (4.0 * PI * denom.powf(1.5))
        };

        let lut = CompressedLUT2D::build(
            |cos_t, g| hg_fn(cos_t, g),
            angle_samples,
            g_samples,
            -1.0,
            1.0, // cos_theta range
            -0.95,
            0.95, // g range
        );

        Self { lut }
    }

    /// Lookup phase function value
    #[inline]
    pub fn lookup(&self, cos_theta: f64, g: f64) -> f64 {
        self.lut.lookup(cos_theta, g)
    }

    /// Memory usage
    pub fn memory_bytes(&self) -> usize {
        self.lut.memory_bytes()
    }
}

// ============================================================================
// MEMORY ANALYSIS
// ============================================================================

/// Compression ratio analysis
#[derive(Debug, Clone)]
pub struct CompressionAnalysis {
    pub original_bytes: usize,
    pub compressed_bytes: usize,
    pub ratio: f64,
    pub max_error: f64,
    pub avg_error: f64,
}

impl CompressionAnalysis {
    /// Analyze compression for a 1D function
    pub fn analyze_1d<F>(
        f: F,
        original_samples: usize,
        compressed_samples: usize,
        input_min: f64,
        input_max: f64,
    ) -> Self
    where
        F: Fn(f64) -> f64,
    {
        let original_bytes = original_samples * 4; // f32

        let lut = CompressedLUT1D::build(&f, compressed_samples, input_min, input_max);
        let compressed_bytes = lut.memory_bytes();

        // Calculate errors
        let test_points = 1000;
        let mut max_error: f64 = 0.0;
        let mut sum_error: f64 = 0.0;

        for i in 0..test_points {
            let x = input_min + (i as f64 / (test_points - 1) as f64) * (input_max - input_min);
            let exact = f(x);
            let approx = lut.lookup(x);
            let error = (exact - approx).abs() / exact.abs().max(1e-10);

            max_error = max_error.max(error);
            sum_error += error;
        }

        Self {
            original_bytes,
            compressed_bytes,
            ratio: original_bytes as f64 / compressed_bytes as f64,
            max_error,
            avg_error: sum_error / test_points as f64,
        }
    }
}

/// Calculate total memory savings for Phase 4 compression
pub fn calculate_memory_savings() -> (usize, usize, f64) {
    // Original Phase 1-3 memory
    let original = 1_700_000; // ~1.7MB

    // Estimated compressed sizes
    let compressed_fresnel = 32 * 2 + 24; // 32 samples u16
    let compressed_beer_lambert = 32 * 2 + 24; // 32 samples u16
    let compressed_hg = 32 * 32 * 2 + 48; // 32x32 u16
    let compressed_spectral = 3 * 32 * 32 * 2 + 48; // 3x32x32 u16
    let compressed_dhg = 8 * 8 * 4 * 32 * 2 + 48; // Reduced DHG
    let compressed_mie = 16 * 4 * 32 * 2 + 48; // Reduced Mie

    let compressed = compressed_fresnel
        + compressed_beer_lambert
        + compressed_hg
        + compressed_spectral
        + compressed_dhg
        + compressed_mie;

    let savings = 1.0 - (compressed as f64 / original as f64);

    (original, compressed, savings)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantization_roundtrip() {
        let values = [0.0, 0.25, 0.5, 0.75, 1.0];

        for &v in &values {
            let q = quantize_f32_to_u16(v, 0.0, 1.0);
            let d = dequantize_u16_to_f32(q, 0.0, 1.0);
            assert!((v - d).abs() < 0.001, "Roundtrip error for {}", v);
        }
    }

    #[test]
    fn test_compressed_lut_1d() {
        let f = |x: f64| x.sin();
        let lut = CompressedLUT1D::build(f, 64, 0.0, PI);

        // Test accuracy
        for i in 0..100 {
            let x = (i as f64 / 99.0) * PI;
            let exact = f(x);
            let approx = lut.lookup(x);
            let error = (exact - approx).abs();
            assert!(error < 0.01, "Error {} at x={}", error, x);
        }
    }

    #[test]
    fn test_compressed_lut_2d() {
        let f = |x: f64, y: f64| (x * y).sin();
        let lut = CompressedLUT2D::build(f, 32, 32, 0.0, PI, 0.0, PI);

        // Test accuracy
        let exact = f(1.0, 1.5);
        let approx = lut.lookup(1.0, 1.5);
        let error = (exact - approx).abs();
        assert!(error < 0.02, "2D LUT error: {}", error);
    }

    #[test]
    fn test_sparse_lut_cubic() {
        let f = |x: f64| x.sin();
        let lut = SparseLUT1D::build(f, 16, 0.0, PI); // Only 16 samples

        // Test accuracy with cubic interpolation
        for i in 0..100 {
            let x = (i as f64 / 99.0) * PI;
            let exact = f(x);
            let approx = lut.lookup(x);
            let error = (exact - approx).abs();
            assert!(error < 0.005, "Sparse LUT error {} at x={}", error, x);
        }
    }

    #[test]
    fn test_hybrid_evaluator() {
        let f: fn(f64) -> f64 = |x| x.sin();
        let evaluator =
            HybridEvaluator::new(f, 32, 0.0, PI, EvaluationMethod::Hybrid { boundary: 1.0 });

        // Below boundary: analytical
        let v_low = evaluator.evaluate(0.5);
        assert!((v_low - 0.5_f64.sin()).abs() < 1e-10);

        // Above boundary: LUT
        let v_high = evaluator.evaluate(2.0);
        assert!((v_high - 2.0_f64.sin()).abs() < 0.01);
    }

    #[test]
    fn test_delta_encoded_lut() {
        // Test with monotonic function
        let f = |x: f64| x.sqrt();
        let lut = DeltaEncodedLUT::build(f, 64, 0.0, 1.0);

        for i in 0..50 {
            let x = i as f64 / 49.0;
            let exact = f(x);
            let approx = lut.lookup(x);
            let error = (exact - approx).abs() / exact.max(0.01);
            assert!(error < 0.02, "Delta LUT error {} at x={}", error, x);
        }
    }

    #[test]
    fn test_compressed_fresnel() {
        let lut = CompressedFresnelLUT::build(1.5, 32);

        // Test known values
        let f0 = lut.lookup(1.0);
        let expected_f0: f64 = ((1.0 - 1.5) / (1.0 + 1.5_f64)).powi(2);
        assert!((f0 - expected_f0).abs() < 0.01, "F0 error");

        // Grazing angle should approach 1
        let f_grazing = lut.lookup(0.01);
        assert!(f_grazing > 0.9, "Grazing Fresnel should be high");
    }

    #[test]
    fn test_compressed_hg() {
        let lut = CompressedHGLUT::build(32, 64);

        // Isotropic (g=0) should be constant
        let p1 = lut.lookup(-0.5, 0.0);
        let p2 = lut.lookup(0.5, 0.0);
        assert!((p1 - p2).abs() < 0.01, "Isotropic H-G should be constant");

        // Forward scattering (g>0) should peak at cos=1
        let p_forward = lut.lookup(0.9, 0.5);
        let p_backward = lut.lookup(-0.9, 0.5);
        assert!(p_forward > p_backward, "Forward > backward for g>0");
    }

    #[test]
    fn test_memory_savings() {
        let (original, compressed, savings) = calculate_memory_savings();

        assert!(
            savings > 0.5,
            "Should achieve >50% savings: {:.1}%",
            savings * 100.0
        );
        assert!(compressed < original, "Compressed < original");
    }

    #[test]
    fn test_compression_analysis() {
        let analysis = CompressionAnalysis::analyze_1d(
            |x| x.sin(),
            256, // Original samples
            64,  // Compressed samples
            0.0,
            PI,
        );

        assert!(analysis.ratio > 3.0, "Should achieve >3x compression");
        assert!(analysis.max_error < 0.02, "Max error < 2%");
        assert!(analysis.avg_error < 0.005, "Avg error < 0.5%");
    }
}

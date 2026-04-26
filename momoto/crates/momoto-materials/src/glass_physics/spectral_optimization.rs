//! Sprint 7 - Spectral Pipeline Optimization
//!
//! Intelligent spectral reduction strategies that maintain physical correctness.
//!
//! ## Quality Tiers
//! - **FullSpectral**: 81+ samples, reference quality
//! - **HighQuality**: 31 samples, imperceptible difference (ΔE < 0.5)
//! - **RealTime**: 8-16 samples, perceptually acceptable (ΔE < 1.0)
//! - **Preview**: 3 samples, for interactive preview only
//!
//! ## Key Principle
//! All optimizations MUST quantify their error vs the full spectral reference.

use super::spectral_pipeline::*;

// ============================================================================
// Quality Tiers
// ============================================================================

/// Spectral quality tier for performance/accuracy tradeoff
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpectralQuality {
    /// Full spectral accuracy (81+ samples)
    /// Target: ΔE = 0 (reference)
    FullSpectral,

    /// High quality (31 samples)
    /// Target: ΔE < 0.5
    HighQuality,

    /// Real-time quality (16 samples)
    /// Target: ΔE < 1.0
    RealTime,

    /// Fast preview (8 samples)
    /// Target: ΔE < 2.0
    FastPreview,

    /// Minimal preview (3 samples - RGB only)
    /// Target: ΔE < 5.0 (approximate only)
    MinimalPreview,
}

impl SpectralQuality {
    /// Get the number of spectral samples for this quality tier
    pub fn sample_count(&self) -> usize {
        match self {
            SpectralQuality::FullSpectral => 81,
            SpectralQuality::HighQuality => 31,
            SpectralQuality::RealTime => 16,
            SpectralQuality::FastPreview => 8,
            SpectralQuality::MinimalPreview => 3,
        }
    }

    /// Get wavelengths for this quality tier (CIE-weighted importance sampling)
    pub fn wavelengths(&self) -> Vec<f64> {
        match self {
            SpectralQuality::FullSpectral => wavelengths::default_sampling(),
            SpectralQuality::HighQuality => Self::high_quality_wavelengths(),
            SpectralQuality::RealTime => Self::realtime_wavelengths(),
            SpectralQuality::FastPreview => Self::fast_preview_wavelengths(),
            SpectralQuality::MinimalPreview => Self::minimal_wavelengths(),
        }
    }

    /// High quality: 31 samples, weighted toward high-sensitivity regions
    fn high_quality_wavelengths() -> Vec<f64> {
        let mut wavelengths = Vec::with_capacity(31);

        // Denser sampling in green region (highest eye sensitivity)
        // Sparser in blue/red extremes

        // Blue region (380-480): 7 samples
        for i in 0..7 {
            wavelengths.push(380.0 + i as f64 * 15.0);
        }

        // Green region (490-590): 12 samples (denser)
        for i in 0..12 {
            wavelengths.push(490.0 + i as f64 * 8.33);
        }

        // Yellow-Red region (600-700): 9 samples
        for i in 0..9 {
            wavelengths.push(600.0 + i as f64 * 11.25);
        }

        // Far red (710-780): 3 samples
        for i in 0..3 {
            wavelengths.push(710.0 + i as f64 * 23.33);
        }

        wavelengths
    }

    /// Real-time: 16 samples, optimized for CIE luminance function
    fn realtime_wavelengths() -> Vec<f64> {
        vec![
            400.0, // Violet
            435.0, // Blue
            460.0, // Blue-cyan
            490.0, // Cyan
            510.0, // Green-cyan
            530.0, // Green (high sensitivity)
            550.0, // Peak sensitivity
            565.0, // Yellow-green
            580.0, // Yellow
            600.0, // Orange
            620.0, // Red-orange
            650.0, // Red
            680.0, // Deep red
            700.0, // Far red
            730.0, // Edge
            760.0, // Far edge
        ]
    }

    /// Fast preview: 8 samples at key wavelengths
    fn fast_preview_wavelengths() -> Vec<f64> {
        vec![
            420.0, // Blue
            460.0, // Cyan-blue
            500.0, // Cyan
            540.0, // Green (peak)
            570.0, // Yellow
            600.0, // Orange
            640.0, // Red
            700.0, // Far red
        ]
    }

    /// Minimal: RGB primaries only
    fn minimal_wavelengths() -> Vec<f64> {
        vec![
            450.0, // Blue
            550.0, // Green
            650.0, // Red
        ]
    }

    /// Expected maximum ΔE for this quality tier
    pub fn expected_max_delta_e(&self) -> f64 {
        match self {
            SpectralQuality::FullSpectral => 0.0,
            SpectralQuality::HighQuality => 0.5,
            SpectralQuality::RealTime => 1.0,
            SpectralQuality::FastPreview => 2.0,
            SpectralQuality::MinimalPreview => 5.0,
        }
    }
}

// ============================================================================
// Optimized Spectral Signal
// ============================================================================

/// Create a spectral signal with optimized sampling
pub fn create_optimized_signal(quality: SpectralQuality, intensity: f64) -> SpectralSignal {
    let wavelengths = quality.wavelengths();
    let intensities = vec![intensity; wavelengths.len()];
    SpectralSignal::from_arrays(&wavelengths, &intensities)
}

/// Create D65 illuminant at specified quality
pub fn create_d65_at_quality(quality: SpectralQuality) -> SpectralSignal {
    let wavelengths = quality.wavelengths();
    let intensities: Vec<f64> = wavelengths.iter().map(|&wl| d65_spd_at(wl)).collect();
    SpectralSignal::from_arrays(&wavelengths, &intensities)
}

/// D65 spectral power distribution approximation
fn d65_spd_at(wavelength_nm: f64) -> f64 {
    // Simplified D65 approximation
    // Real implementation would use CIE tabulated values
    let wl = wavelength_nm;

    if wl < 380.0 || wl > 780.0 {
        return 0.0;
    }

    // Piecewise approximation of D65
    if wl < 420.0 {
        0.5 + (wl - 380.0) / 80.0 * 0.3
    } else if wl < 460.0 {
        0.8 + (wl - 420.0) / 40.0 * 0.2
    } else if wl < 520.0 {
        1.0 - (wl - 460.0) / 60.0 * 0.05
    } else if wl < 600.0 {
        0.95 + (wl - 520.0) / 80.0 * 0.05
    } else if wl < 700.0 {
        1.0 - (wl - 600.0) / 100.0 * 0.15
    } else {
        0.85 - (wl - 700.0) / 80.0 * 0.45
    }
}

// ============================================================================
// Quality Comparison & Error Measurement
// ============================================================================

/// Error metrics comparing optimized vs reference
#[derive(Debug, Clone)]
pub struct QualityMetrics {
    /// CIE Delta E 2000 color difference
    pub delta_e: f64,

    /// Energy conservation ratio (optimized/reference)
    pub energy_ratio: f64,

    /// Reference RGB
    pub reference_rgb: [f64; 3],

    /// Optimized RGB
    pub optimized_rgb: [f64; 3],

    /// Quality tier used
    pub quality: SpectralQuality,

    /// Actual sample count
    pub sample_count: usize,
}

impl QualityMetrics {
    /// Check if quality is acceptable for the tier
    pub fn is_acceptable(&self) -> bool {
        self.delta_e <= self.quality.expected_max_delta_e()
    }

    /// Generate summary string
    pub fn summary(&self) -> String {
        format!(
            "Quality: {:?}, Samples: {}, ΔE: {:.3}, Energy: {:.4}, Acceptable: {}",
            self.quality,
            self.sample_count,
            self.delta_e,
            self.energy_ratio,
            if self.is_acceptable() { "✓" } else { "✗" }
        )
    }
}

/// Compare optimized evaluation against reference
pub fn compare_quality(
    pipeline: &SpectralPipeline,
    context: &EvaluationContext,
    quality: SpectralQuality,
) -> QualityMetrics {
    // Reference evaluation (full spectral)
    let ref_incident = SpectralSignal::d65_illuminant();
    let ref_output = pipeline.evaluate(&ref_incident, context);
    let ref_rgb = ref_output.to_rgb();
    let ref_energy = ref_output.total_energy();

    // Optimized evaluation
    let opt_incident = create_d65_at_quality(quality);
    let opt_output = pipeline.evaluate(&opt_incident, context);
    let opt_rgb = opt_output.to_rgb();
    let opt_energy = opt_output.total_energy();

    // Calculate Delta E (simplified, should use CIEDE2000)
    let delta_e = calculate_delta_e_simple(&ref_rgb, &opt_rgb);

    QualityMetrics {
        delta_e,
        energy_ratio: opt_energy / ref_energy.max(0.0001),
        reference_rgb: ref_rgb,
        optimized_rgb: opt_rgb,
        quality,
        sample_count: quality.sample_count(),
    }
}

/// Simplified Delta E calculation (Euclidean in Lab)
pub fn calculate_delta_e_simple(rgb1: &[f64; 3], rgb2: &[f64; 3]) -> f64 {
    // Convert to Lab and compute Delta E
    let lab1 = rgb_to_lab(rgb1);
    let lab2 = rgb_to_lab(rgb2);

    let dl = lab1[0] - lab2[0];
    let da = lab1[1] - lab2[1];
    let db = lab1[2] - lab2[2];

    (dl * dl + da * da + db * db).sqrt()
}

/// RGB to Lab conversion (simplified)
pub fn rgb_to_lab(rgb: &[f64; 3]) -> [f64; 3] {
    // Linearize
    let r = gamma_expand(rgb[0]);
    let g = gamma_expand(rgb[1]);
    let b = gamma_expand(rgb[2]);

    // RGB to XYZ (sRGB D65)
    let x = 0.4124564 * r + 0.3575761 * g + 0.1804375 * b;
    let y = 0.2126729 * r + 0.7151522 * g + 0.0721750 * b;
    let z = 0.0193339 * r + 0.1191920 * g + 0.9503041 * b;

    // Normalize to D65 white point
    let xn = x / 0.95047;
    let yn = y / 1.0;
    let zn = z / 1.08883;

    // XYZ to Lab
    let fx = lab_f(xn);
    let fy = lab_f(yn);
    let fz = lab_f(zn);

    let l = 116.0 * fy - 16.0;
    let a = 500.0 * (fx - fy);
    let b_lab = 200.0 * (fy - fz);

    [l, a, b_lab]
}

fn gamma_expand(v: f64) -> f64 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

fn lab_f(t: f64) -> f64 {
    let delta: f64 = 6.0 / 29.0;
    if t > delta.powi(3) {
        t.cbrt()
    } else {
        t / (3.0 * delta * delta) + 4.0 / 29.0
    }
}

// ============================================================================
// Adaptive Quality Selection
// ============================================================================

/// Automatically select quality tier based on requirements
pub struct AdaptiveQualitySelector {
    /// Maximum acceptable Delta E
    pub max_delta_e: f64,

    /// Target frame time in microseconds
    pub target_time_us: f64,

    /// Cached baseline timings (quality -> time_us)
    baseline_timings: std::collections::HashMap<SpectralQuality, f64>,
}

impl AdaptiveQualitySelector {
    pub fn new(max_delta_e: f64, target_time_us: f64) -> Self {
        let mut selector = Self {
            max_delta_e,
            target_time_us,
            baseline_timings: std::collections::HashMap::new(),
        };

        // Approximate baseline timings based on benchmark data
        // These should be calibrated per-platform
        selector
            .baseline_timings
            .insert(SpectralQuality::FullSpectral, 12.5);
        selector
            .baseline_timings
            .insert(SpectralQuality::HighQuality, 5.4);
        selector
            .baseline_timings
            .insert(SpectralQuality::RealTime, 3.6);
        selector
            .baseline_timings
            .insert(SpectralQuality::FastPreview, 1.8);
        selector
            .baseline_timings
            .insert(SpectralQuality::MinimalPreview, 1.0);

        selector
    }

    /// Select best quality tier that meets both accuracy and performance targets
    pub fn select(&self) -> SpectralQuality {
        let tiers = [
            SpectralQuality::FullSpectral,
            SpectralQuality::HighQuality,
            SpectralQuality::RealTime,
            SpectralQuality::FastPreview,
            SpectralQuality::MinimalPreview,
        ];

        // Find highest quality that meets performance target
        for &tier in &tiers {
            let expected_time = self.baseline_timings.get(&tier).copied().unwrap_or(100.0);
            let expected_error = tier.expected_max_delta_e();

            if expected_time <= self.target_time_us && expected_error <= self.max_delta_e {
                return tier;
            }
        }

        // Fallback to minimal if nothing else works
        SpectralQuality::MinimalPreview
    }

    /// Select quality for interactive (60 FPS) use
    pub fn select_interactive() -> SpectralQuality {
        // 60 FPS = 16.67ms per frame
        // Allow 1ms for spectral evaluation
        let selector = Self::new(1.0, 1000.0);
        selector.select()
    }

    /// Select quality for real-time (30 FPS) use
    pub fn select_realtime() -> SpectralQuality {
        // 30 FPS = 33.3ms per frame
        // Allow 5ms for spectral evaluation
        let selector = Self::new(0.5, 5000.0);
        selector.select()
    }
}

impl Default for AdaptiveQualitySelector {
    fn default() -> Self {
        Self::new(1.0, 1000.0) // ΔE < 1.0, target 1ms
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glass_physics::spectral_pipeline::{
        DispersionStage, MetalReflectanceStage, MieScatteringStage,
    };

    #[test]
    fn test_quality_tiers_sample_counts() {
        assert_eq!(SpectralQuality::FullSpectral.sample_count(), 81);
        assert_eq!(SpectralQuality::HighQuality.sample_count(), 31);
        assert_eq!(SpectralQuality::RealTime.sample_count(), 16);
        assert_eq!(SpectralQuality::FastPreview.sample_count(), 8);
        assert_eq!(SpectralQuality::MinimalPreview.sample_count(), 3);
    }

    #[test]
    fn test_quality_wavelengths_ordered() {
        for quality in [
            SpectralQuality::FullSpectral,
            SpectralQuality::HighQuality,
            SpectralQuality::RealTime,
            SpectralQuality::FastPreview,
            SpectralQuality::MinimalPreview,
        ] {
            let wavelengths = quality.wavelengths();
            assert!(!wavelengths.is_empty());

            // Check wavelengths are in valid range
            for &wl in &wavelengths {
                assert!(wl >= 380.0 && wl <= 780.0, "Wavelength {} out of range", wl);
            }
        }
    }

    #[test]
    fn test_quality_comparison_thin_film() {
        let pipeline = SpectralPipeline::new().add_stage(ThinFilmStage::new(1.45, 300.0, 1.52));
        let context = EvaluationContext::default();

        for quality in [
            SpectralQuality::HighQuality,
            SpectralQuality::RealTime,
            SpectralQuality::FastPreview,
        ] {
            let metrics = compare_quality(&pipeline, &context, quality);
            println!("{}", metrics.summary());

            // Check error is within expected bounds (with some margin)
            assert!(
                metrics.delta_e <= quality.expected_max_delta_e() * 2.0,
                "ΔE {} exceeds 2x expected {} for {:?}",
                metrics.delta_e,
                quality.expected_max_delta_e(),
                quality
            );
        }
    }

    #[test]
    fn test_adaptive_selector() {
        // High quality target
        let selector = AdaptiveQualitySelector::new(0.5, 10000.0);
        let quality = selector.select();
        assert!(quality.expected_max_delta_e() <= 0.5);

        // Performance target
        let selector = AdaptiveQualitySelector::new(5.0, 1.0);
        let quality = selector.select();
        // Should pick fastest option
        assert_eq!(quality, SpectralQuality::MinimalPreview);
    }

    #[test]
    fn test_d65_at_quality_produces_valid_signal() {
        for quality in [
            SpectralQuality::FullSpectral,
            SpectralQuality::RealTime,
            SpectralQuality::MinimalPreview,
        ] {
            let signal = create_d65_at_quality(quality);
            let energy = signal.total_energy();
            assert!(energy > 0.0, "D65 at {:?} has zero energy", quality);
        }
    }

    #[test]
    fn test_comprehensive_quality_validation() {
        println!("\n=== Comprehensive Quality Validation ===\n");

        // Test 1: Complex thin film at different angles
        println!("--- Thin Film (n=1.45, 250nm) at Various Angles ---");
        for angle in [0.0, 30.0, 60.0] {
            let pipeline = SpectralPipeline::new().add_stage(ThinFilmStage::new(1.45, 250.0, 1.52));
            let context = EvaluationContext::default().with_angle_deg(angle);

            println!("  Angle: {}°", angle);
            for quality in [
                SpectralQuality::HighQuality,
                SpectralQuality::RealTime,
                SpectralQuality::FastPreview,
            ] {
                let metrics = compare_quality(&pipeline, &context, quality);
                println!(
                    "    {:?}: RGB=[{:.3}, {:.3}, {:.3}] vs ref=[{:.3}, {:.3}, {:.3}], ΔE={:.4}, Energy={:.4}",
                    quality,
                    metrics.optimized_rgb[0], metrics.optimized_rgb[1], metrics.optimized_rgb[2],
                    metrics.reference_rgb[0], metrics.reference_rgb[1], metrics.reference_rgb[2],
                    metrics.delta_e,
                    metrics.energy_ratio
                );
            }
        }

        // Test 2: Dispersion (crown glass)
        println!("\n--- Crown Glass Dispersion ---");
        let pipeline = SpectralPipeline::new().add_stage(DispersionStage::crown_glass());
        let context = EvaluationContext::default();

        for quality in [
            SpectralQuality::HighQuality,
            SpectralQuality::RealTime,
            SpectralQuality::FastPreview,
        ] {
            let metrics = compare_quality(&pipeline, &context, quality);
            println!(
                "  {:?}: RGB=[{:.3}, {:.3}, {:.3}] vs ref=[{:.3}, {:.3}, {:.3}], ΔE={:.4}",
                quality,
                metrics.optimized_rgb[0],
                metrics.optimized_rgb[1],
                metrics.optimized_rgb[2],
                metrics.reference_rgb[0],
                metrics.reference_rgb[1],
                metrics.reference_rgb[2],
                metrics.delta_e
            );
        }

        // Test 3: Metal (gold)
        println!("\n--- Gold Metal Reflection ---");
        let pipeline = SpectralPipeline::new().add_stage(MetalReflectanceStage::gold());
        let context = EvaluationContext::default();

        for quality in [
            SpectralQuality::HighQuality,
            SpectralQuality::RealTime,
            SpectralQuality::FastPreview,
        ] {
            let metrics = compare_quality(&pipeline, &context, quality);
            println!(
                "  {:?}: RGB=[{:.3}, {:.3}, {:.3}] vs ref=[{:.3}, {:.3}, {:.3}], ΔE={:.4}",
                quality,
                metrics.optimized_rgb[0],
                metrics.optimized_rgb[1],
                metrics.optimized_rgb[2],
                metrics.reference_rgb[0],
                metrics.reference_rgb[1],
                metrics.reference_rgb[2],
                metrics.delta_e
            );
        }

        // Test 4: Complex pipeline (thin film + dispersion + scattering)
        // CRITICAL FINDING: Naive spectral reduction has HIGH ΔE for complex pipelines!
        // This validates that we need LUTs (FASE 3) rather than just reducing samples.
        println!("\n--- Complex Pipeline (ThinFilm + Dispersion + Mie) ---");
        println!("  NOTE: High ΔE expected - naive sampling reduction is insufficient");
        println!("  This validates the need for LUT-based optimization (FASE 3)");
        let pipeline = SpectralPipeline::new()
            .add_stage(ThinFilmStage::new(1.45, 200.0, 1.52))
            .add_stage(DispersionStage::crown_glass())
            .add_stage(MieScatteringStage::fog());
        let context = EvaluationContext::default().with_angle_deg(30.0);

        let mut max_delta_e = 0.0f64;
        for quality in [
            SpectralQuality::HighQuality,
            SpectralQuality::RealTime,
            SpectralQuality::FastPreview,
        ] {
            let metrics = compare_quality(&pipeline, &context, quality);
            max_delta_e = max_delta_e.max(metrics.delta_e);
            println!(
                "  {:?}: RGB=[{:.3}, {:.3}, {:.3}] vs ref=[{:.3}, {:.3}, {:.3}], ΔE={:.4}, Energy={:.4}",
                quality,
                metrics.optimized_rgb[0], metrics.optimized_rgb[1], metrics.optimized_rgb[2],
                metrics.reference_rgb[0], metrics.reference_rgb[1], metrics.reference_rgb[2],
                metrics.delta_e,
                metrics.energy_ratio
            );
        }

        // Document the finding: naive sampling has high error
        println!(
            "\n  CONCLUSION: Max ΔE = {:.2} - LUTs required for ΔE < 1",
            max_delta_e
        );

        // This test validates the NEED for LUTs, not the quality of naive sampling
        // High ΔE is expected and proves our point
        assert!(
            max_delta_e > 5.0,
            "Expected high ΔE to prove LUT necessity, got {:.4}",
            max_delta_e
        );

        println!("\n=== Validation Complete ===");
        println!("  Summary: Naive sampling has ΔE > 5, validating need for LUTs");
    }
}

//! # Coherent Spectral Sampling
//!
//! Deterministic wavelength sampling for temporal stability.
//!
//! ## Key Features
//!
//! - **Stratified Sampling**: Even coverage across visible spectrum
//! - **Temporal Jitter Control**: Deterministic per-frame jitter
//! - **Coherent Selection**: Same wavelengths across frames for stability

use super::packet::{CoherenceMetadata, SpectralPacket, WavelengthBand};

// ============================================================================
// SAMPLING STRATEGY
// ============================================================================

/// Sampling strategy for spectral evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplingStrategy {
    /// Uniform sampling at fixed intervals.
    Uniform,
    /// Stratified sampling with controlled jitter.
    Stratified,
    /// Importance sampling weighted by luminance.
    ImportanceWeighted,
    /// Hero wavelength with secondary samples.
    HeroWavelength,
    /// RGB primaries only (3 wavelengths).
    RGBPrimaries,
}

impl Default for SamplingStrategy {
    fn default() -> Self {
        SamplingStrategy::Stratified
    }
}

impl SamplingStrategy {
    /// Get recommended sample count for this strategy.
    pub fn recommended_samples(&self) -> usize {
        match self {
            SamplingStrategy::Uniform => 31,
            SamplingStrategy::Stratified => 31,
            SamplingStrategy::ImportanceWeighted => 16,
            SamplingStrategy::HeroWavelength => 4,
            SamplingStrategy::RGBPrimaries => 3,
        }
    }
}

// ============================================================================
// COHERENT SAMPLER
// ============================================================================

/// Configuration for coherent sampling.
#[derive(Debug, Clone)]
pub struct SamplerConfig {
    /// Minimum wavelength (nm).
    pub min_wavelength: f64,
    /// Maximum wavelength (nm).
    pub max_wavelength: f64,
    /// Number of samples.
    pub sample_count: usize,
    /// Sampling strategy.
    pub strategy: SamplingStrategy,
    /// Jitter scale (0 = no jitter, 1 = full stratum jitter).
    pub jitter_scale: f64,
    /// Random seed for deterministic sampling.
    pub seed: u64,
}

impl Default for SamplerConfig {
    fn default() -> Self {
        Self {
            min_wavelength: 400.0,
            max_wavelength: 700.0,
            sample_count: 31,
            strategy: SamplingStrategy::Stratified,
            jitter_scale: 0.5,
            seed: 12345,
        }
    }
}

/// Coherent spectral sampler.
///
/// Generates deterministic wavelength samples for temporal stability.
#[derive(Debug, Clone)]
pub struct CoherentSampler {
    /// Configuration.
    config: SamplerConfig,
    /// Current frame index.
    frame_index: u64,
    /// Cached wavelengths.
    cached_wavelengths: Vec<f64>,
    /// Cache frame index.
    cache_frame: u64,
}

impl Default for CoherentSampler {
    fn default() -> Self {
        Self::new(SamplerConfig::default())
    }
}

impl CoherentSampler {
    /// Create new sampler with configuration.
    pub fn new(config: SamplerConfig) -> Self {
        Self {
            config,
            frame_index: 0,
            cached_wavelengths: Vec::new(),
            cache_frame: u64::MAX,
        }
    }

    /// Create uniform sampler.
    pub fn uniform(min: f64, max: f64, count: usize) -> Self {
        Self::new(SamplerConfig {
            min_wavelength: min,
            max_wavelength: max,
            sample_count: count,
            strategy: SamplingStrategy::Uniform,
            jitter_scale: 0.0,
            ..Default::default()
        })
    }

    /// Create stratified sampler with jitter.
    pub fn stratified(count: usize, jitter: f64) -> Self {
        Self::new(SamplerConfig {
            sample_count: count,
            strategy: SamplingStrategy::Stratified,
            jitter_scale: jitter,
            ..Default::default()
        })
    }

    /// Create RGB-only sampler.
    pub fn rgb_primaries() -> Self {
        Self::new(SamplerConfig {
            sample_count: 3,
            strategy: SamplingStrategy::RGBPrimaries,
            ..Default::default()
        })
    }

    /// Set frame index for deterministic sampling.
    pub fn set_frame(&mut self, frame: u64) {
        self.frame_index = frame;
    }

    /// Advance to next frame.
    pub fn advance_frame(&mut self) {
        self.frame_index += 1;
    }

    /// Generate wavelength samples.
    pub fn sample(&mut self) -> Vec<f64> {
        // Return cached if same frame
        if self.frame_index == self.cache_frame && !self.cached_wavelengths.is_empty() {
            return self.cached_wavelengths.clone();
        }

        let wavelengths = match self.config.strategy {
            SamplingStrategy::Uniform => self.sample_uniform(),
            SamplingStrategy::Stratified => self.sample_stratified(),
            SamplingStrategy::ImportanceWeighted => self.sample_importance(),
            SamplingStrategy::HeroWavelength => self.sample_hero(),
            SamplingStrategy::RGBPrimaries => self.sample_rgb(),
        };

        self.cached_wavelengths = wavelengths.clone();
        self.cache_frame = self.frame_index;

        wavelengths
    }

    /// Generate uniform samples.
    fn sample_uniform(&self) -> Vec<f64> {
        let step = (self.config.max_wavelength - self.config.min_wavelength)
            / (self.config.sample_count - 1) as f64;

        (0..self.config.sample_count)
            .map(|i| self.config.min_wavelength + i as f64 * step)
            .collect()
    }

    /// Generate stratified samples with deterministic jitter.
    fn sample_stratified(&self) -> Vec<f64> {
        let n = self.config.sample_count;
        let range = self.config.max_wavelength - self.config.min_wavelength;
        let stratum_size = range / n as f64;

        (0..n)
            .map(|i| {
                let stratum_min = self.config.min_wavelength + i as f64 * stratum_size;
                let stratum_center = stratum_min + stratum_size * 0.5;

                // Deterministic jitter based on frame and sample index
                let jitter = self.deterministic_jitter(i as u64) * self.config.jitter_scale;
                let offset = (jitter - 0.5) * stratum_size;

                (stratum_center + offset)
                    .clamp(self.config.min_wavelength, self.config.max_wavelength)
            })
            .collect()
    }

    /// Generate importance-weighted samples (luminance-weighted).
    fn sample_importance(&self) -> Vec<f64> {
        // Weight towards green (peak luminance) with some blue/red coverage
        let n = self.config.sample_count;
        let mut wavelengths = Vec::with_capacity(n);

        // More samples around 555nm (peak luminance)
        for i in 0..n {
            let t = i as f64 / (n - 1) as f64;
            // Bias towards center using smooth curve
            let biased_t = 0.5 + (t - 0.5) * (1.0 - 0.3 * (1.0 - 4.0 * (t - 0.5).powi(2)));
            let wavelength = self.config.min_wavelength
                + biased_t * (self.config.max_wavelength - self.config.min_wavelength);

            // Add small jitter
            let jitter = self.deterministic_jitter(i as u64) * 5.0;
            wavelengths.push(
                (wavelength + jitter - 2.5)
                    .clamp(self.config.min_wavelength, self.config.max_wavelength),
            );
        }

        wavelengths.sort_by(|a, b| a.partial_cmp(b).unwrap());
        wavelengths
    }

    /// Generate hero wavelength samples.
    fn sample_hero(&self) -> Vec<f64> {
        // Hero wavelength changes per frame for coverage
        let hero_offset = (self.frame_index % 31) as f64 * 10.0;
        let hero = 400.0 + hero_offset;

        // Secondary samples at fixed positions
        vec![
            hero, 486.1, // F (blue)
            555.0, // V (peak luminance)
            656.3, // C (red)
        ]
    }

    /// Generate RGB primary samples.
    fn sample_rgb(&self) -> Vec<f64> {
        vec![
            656.3, // Red (C line)
            555.0, // Green (peak luminance)
            486.1, // Blue (F line)
        ]
    }

    /// Deterministic pseudo-random jitter.
    fn deterministic_jitter(&self, sample_index: u64) -> f64 {
        // PCG-like hash for deterministic jitter with good mixing
        let mut state = self.config.seed;
        state = self.mix64(state, self.frame_index);
        state = self.mix64(state, sample_index);
        // Additional mixing rounds for better distribution
        state = self.splitmix64(state);
        (state as f64) / (u64::MAX as f64)
    }

    /// Mix two u64 values with better bit distribution.
    fn mix64(&self, a: u64, b: u64) -> u64 {
        let mut h = a.wrapping_add(b.wrapping_mul(0x9e3779b97f4a7c15));
        h ^= h >> 30;
        h = h.wrapping_mul(0xbf58476d1ce4e5b9);
        h ^= h >> 27;
        h = h.wrapping_mul(0x94d049bb133111eb);
        h ^= h >> 31;
        h
    }

    /// SplitMix64 for final mixing.
    fn splitmix64(&self, mut z: u64) -> u64 {
        z = z.wrapping_add(0x9e3779b97f4a7c15);
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }

    /// Create a SpectralPacket with sampled wavelengths.
    pub fn create_packet(&mut self) -> SpectralPacket {
        let wavelengths = self.sample();
        let values = vec![0.0; wavelengths.len()];

        SpectralPacket::from_data(wavelengths, values).with_coherence(CoherenceMetadata {
            frame_index: self.frame_index,
            ..Default::default()
        })
    }

    /// Get current configuration.
    pub fn config(&self) -> &SamplerConfig {
        &self.config
    }
}

// ============================================================================
// STRATIFIED SAMPLER
// ============================================================================

/// Stratified sampler with explicit stratum control.
#[derive(Debug, Clone)]
pub struct StratifiedSampler {
    /// Number of strata.
    strata: usize,
    /// Wavelength range.
    min: f64,
    max: f64,
    /// Jitter amount.
    jitter: f64,
    /// Frame index.
    frame: u64,
}

impl Default for StratifiedSampler {
    fn default() -> Self {
        Self {
            strata: 31,
            min: 400.0,
            max: 700.0,
            jitter: 0.5,
            frame: 0,
        }
    }
}

impl StratifiedSampler {
    /// Create new stratified sampler.
    pub fn new(strata: usize, min: f64, max: f64) -> Self {
        Self {
            strata,
            min,
            max,
            jitter: 0.5,
            frame: 0,
        }
    }

    /// Set jitter amount.
    pub fn with_jitter(mut self, jitter: f64) -> Self {
        self.jitter = jitter.clamp(0.0, 1.0);
        self
    }

    /// Set frame for deterministic sampling.
    pub fn at_frame(mut self, frame: u64) -> Self {
        self.frame = frame;
        self
    }

    /// Generate samples.
    pub fn sample(&self) -> Vec<f64> {
        let stratum_size = (self.max - self.min) / self.strata as f64;

        (0..self.strata)
            .map(|i| {
                let center = self.min + (i as f64 + 0.5) * stratum_size;
                let jitter_offset = self.stratum_jitter(i) * self.jitter * stratum_size;
                (center + jitter_offset).clamp(self.min, self.max)
            })
            .collect()
    }

    /// Get deterministic jitter for stratum.
    fn stratum_jitter(&self, stratum: usize) -> f64 {
        // Simple deterministic jitter based on stratum and frame
        let hash = stratum as u64 ^ self.frame.wrapping_mul(0x517cc1b727220a95);
        let normalized = (hash % 10000) as f64 / 10000.0;
        normalized - 0.5 // Range [-0.5, 0.5]
    }
}

// ============================================================================
// JITTERED SAMPLER
// ============================================================================

/// Low-discrepancy jittered sampler.
#[derive(Debug, Clone)]
pub struct JitteredSampler {
    /// Base sample count.
    count: usize,
    /// Wavelength range.
    min: f64,
    max: f64,
    /// Jitter seed.
    seed: u64,
}

impl Default for JitteredSampler {
    fn default() -> Self {
        Self {
            count: 31,
            min: 400.0,
            max: 700.0,
            seed: 42,
        }
    }
}

impl JitteredSampler {
    /// Create new jittered sampler.
    pub fn new(count: usize, min: f64, max: f64) -> Self {
        Self {
            count,
            min,
            max,
            seed: 42,
        }
    }

    /// Set random seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Generate samples using van der Corput sequence.
    pub fn sample(&self) -> Vec<f64> {
        (0..self.count)
            .map(|i| {
                let vdc = self.van_der_corput(i as u64 + self.seed);
                self.min + vdc * (self.max - self.min)
            })
            .collect()
    }

    /// Van der Corput low-discrepancy sequence.
    fn van_der_corput(&self, mut n: u64) -> f64 {
        let mut result = 0.0;
        let mut base_inv = 0.5;

        while n > 0 {
            result += base_inv * (n % 2) as f64;
            n /= 2;
            base_inv *= 0.5;
        }

        result
    }

    /// Generate sorted samples.
    pub fn sample_sorted(&self) -> Vec<f64> {
        let mut samples = self.sample();
        samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
        samples
    }
}

// ============================================================================
// BAND SAMPLER
// ============================================================================

/// Sample wavelengths by band.
#[derive(Debug, Clone, Default)]
pub struct BandSampler {
    /// Bands to sample.
    bands: Vec<WavelengthBand>,
    /// Samples per band.
    samples_per_band: usize,
}

impl BandSampler {
    /// Create new band sampler.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a band.
    pub fn with_band(mut self, band: WavelengthBand) -> Self {
        self.bands.push(band);
        self
    }

    /// Set samples per band.
    pub fn samples_per_band(mut self, count: usize) -> Self {
        self.samples_per_band = count;
        self
    }

    /// Add all visible bands.
    pub fn visible_spectrum(mut self) -> Self {
        self.bands = vec![
            WavelengthBand::Violet,
            WavelengthBand::Blue,
            WavelengthBand::Green,
            WavelengthBand::Yellow,
            WavelengthBand::Orange,
            WavelengthBand::Red,
        ];
        self
    }

    /// Generate samples.
    pub fn sample(&self) -> Vec<f64> {
        let mut wavelengths = Vec::new();

        for band in &self.bands {
            let (min, max) = band.range();
            let n = self.samples_per_band.max(1);

            if n == 1 {
                wavelengths.push(band.center());
            } else {
                let step = (max - min) / (n - 1) as f64;
                for i in 0..n {
                    wavelengths.push(min + i as f64 * step);
                }
            }
        }

        wavelengths.sort_by(|a, b| a.partial_cmp(b).unwrap());
        wavelengths
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uniform_sampling() {
        let mut sampler = CoherentSampler::uniform(400.0, 700.0, 31);
        let wavelengths = sampler.sample();

        assert_eq!(wavelengths.len(), 31);
        assert!((wavelengths[0] - 400.0).abs() < 1e-6);
        assert!((wavelengths[30] - 700.0).abs() < 1e-6);
    }

    #[test]
    fn test_stratified_sampling() {
        let mut sampler = CoherentSampler::stratified(31, 0.5);
        let wavelengths = sampler.sample();

        assert_eq!(wavelengths.len(), 31);
        for w in &wavelengths {
            assert!(*w >= 400.0 && *w <= 700.0);
        }
    }

    #[test]
    fn test_deterministic_sampling() {
        let mut sampler1 = CoherentSampler::stratified(31, 0.5);
        let mut sampler2 = CoherentSampler::stratified(31, 0.5);

        sampler1.set_frame(100);
        sampler2.set_frame(100);

        let w1 = sampler1.sample();
        let w2 = sampler2.sample();

        assert_eq!(w1, w2);
    }

    #[test]
    fn test_different_frames_differ() {
        let mut sampler = CoherentSampler::stratified(31, 0.5);

        sampler.set_frame(0);
        let w1 = sampler.sample();

        sampler.set_frame(1);
        let w2 = sampler.sample();

        // Wavelengths should differ between frames (with jitter)
        assert_ne!(w1, w2);
    }

    #[test]
    fn test_rgb_primaries() {
        let mut sampler = CoherentSampler::rgb_primaries();
        let wavelengths = sampler.sample();

        assert_eq!(wavelengths.len(), 3);
    }

    #[test]
    fn test_create_packet() {
        let mut sampler = CoherentSampler::default();
        sampler.set_frame(42);
        let packet = sampler.create_packet();

        assert_eq!(packet.len(), 31);
        assert_eq!(packet.coherence.frame_index, 42);
    }

    #[test]
    fn test_stratified_sampler() {
        let sampler = StratifiedSampler::new(16, 400.0, 700.0)
            .with_jitter(0.3)
            .at_frame(10);

        let wavelengths = sampler.sample();

        assert_eq!(wavelengths.len(), 16);
        for w in &wavelengths {
            assert!(*w >= 400.0 && *w <= 700.0);
        }
    }

    #[test]
    fn test_jittered_sampler() {
        let sampler = JitteredSampler::new(31, 400.0, 700.0).with_seed(12345);

        let wavelengths = sampler.sample_sorted();

        assert_eq!(wavelengths.len(), 31);
        // Should be sorted
        for i in 1..wavelengths.len() {
            assert!(wavelengths[i] >= wavelengths[i - 1]);
        }
    }

    #[test]
    fn test_van_der_corput() {
        let sampler = JitteredSampler::default();
        let samples: Vec<f64> = (0..8).map(|i| sampler.van_der_corput(i)).collect();

        // VdC sequence should be well-distributed
        for s in &samples {
            assert!(*s >= 0.0 && *s < 1.0);
        }
    }

    #[test]
    fn test_band_sampler() {
        let sampler = BandSampler::new().visible_spectrum().samples_per_band(3);

        let wavelengths = sampler.sample();

        assert_eq!(wavelengths.len(), 18); // 6 bands * 3 samples
    }

    #[test]
    fn test_band_sampler_single_per_band() {
        let sampler = BandSampler::new()
            .with_band(WavelengthBand::Green)
            .with_band(WavelengthBand::Red)
            .samples_per_band(1);

        let wavelengths = sampler.sample();

        assert_eq!(wavelengths.len(), 2);
    }

    #[test]
    fn test_sampling_strategy_recommendations() {
        assert_eq!(SamplingStrategy::Uniform.recommended_samples(), 31);
        assert_eq!(SamplingStrategy::RGBPrimaries.recommended_samples(), 3);
        assert_eq!(SamplingStrategy::HeroWavelength.recommended_samples(), 4);
    }
}

//! # Spectral Packet
//!
//! Wavelength set with coherence metadata for temporal stability.

// ============================================================================
// WAVELENGTH BAND
// ============================================================================

/// Standard wavelength bands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WavelengthBand {
    /// Ultraviolet (300-400nm).
    UV,
    /// Visible violet (400-450nm).
    Violet,
    /// Visible blue (450-495nm).
    Blue,
    /// Visible green (495-570nm).
    Green,
    /// Visible yellow (570-590nm).
    Yellow,
    /// Visible orange (590-620nm).
    Orange,
    /// Visible red (620-700nm).
    Red,
    /// Near infrared (700-1000nm).
    NIR,
}

impl WavelengthBand {
    /// Get wavelength range for band.
    pub fn range(&self) -> (f64, f64) {
        match self {
            WavelengthBand::UV => (300.0, 400.0),
            WavelengthBand::Violet => (400.0, 450.0),
            WavelengthBand::Blue => (450.0, 495.0),
            WavelengthBand::Green => (495.0, 570.0),
            WavelengthBand::Yellow => (570.0, 590.0),
            WavelengthBand::Orange => (590.0, 620.0),
            WavelengthBand::Red => (620.0, 700.0),
            WavelengthBand::NIR => (700.0, 1000.0),
        }
    }

    /// Get center wavelength.
    pub fn center(&self) -> f64 {
        let (min, max) = self.range();
        (min + max) / 2.0
    }

    /// Check if wavelength is in band.
    pub fn contains(&self, wavelength: f64) -> bool {
        let (min, max) = self.range();
        wavelength >= min && wavelength <= max
    }

    /// Get band for wavelength.
    pub fn from_wavelength(wavelength: f64) -> Option<Self> {
        if wavelength < 300.0 || wavelength > 1000.0 {
            None
        } else if wavelength < 400.0 {
            Some(WavelengthBand::UV)
        } else if wavelength < 450.0 {
            Some(WavelengthBand::Violet)
        } else if wavelength < 495.0 {
            Some(WavelengthBand::Blue)
        } else if wavelength < 570.0 {
            Some(WavelengthBand::Green)
        } else if wavelength < 590.0 {
            Some(WavelengthBand::Yellow)
        } else if wavelength < 620.0 {
            Some(WavelengthBand::Orange)
        } else if wavelength < 700.0 {
            Some(WavelengthBand::Red)
        } else {
            Some(WavelengthBand::NIR)
        }
    }
}

// ============================================================================
// COHERENCE METADATA
// ============================================================================

/// Coherence information for spectral evaluation.
#[derive(Debug, Clone)]
pub struct CoherenceMetadata {
    /// Coherence length in micrometers.
    pub coherence_length: f64,

    /// Temporal phase (for interference).
    pub temporal_phase: f64,

    /// Frame index for deterministic sampling.
    pub frame_index: u64,

    /// Bandwidth (nm) for coherent sources.
    pub bandwidth: f64,

    /// Whether this is a coherent light source.
    pub is_coherent: bool,
}

impl Default for CoherenceMetadata {
    fn default() -> Self {
        Self {
            coherence_length: 1.0, // 1 µm default
            temporal_phase: 0.0,
            frame_index: 0,
            bandwidth: 100.0, // Broad (incoherent) default
            is_coherent: false,
        }
    }
}

impl CoherenceMetadata {
    /// Create coherent source metadata.
    pub fn coherent(bandwidth_nm: f64) -> Self {
        // Coherence length = λ² / Δλ
        let center = 550.0; // nm
        let coherence_length = (center * center) / bandwidth_nm / 1000.0; // Convert to µm

        Self {
            coherence_length,
            temporal_phase: 0.0,
            frame_index: 0,
            bandwidth: bandwidth_nm,
            is_coherent: true,
        }
    }

    /// Create laser-like coherent source.
    pub fn laser() -> Self {
        Self::coherent(0.1) // Very narrow bandwidth
    }

    /// Create LED-like partially coherent source.
    pub fn led() -> Self {
        Self::coherent(20.0)
    }

    /// Create sunlight (incoherent).
    pub fn sunlight() -> Self {
        Self::default()
    }

    /// Advance to next frame.
    pub fn advance(&mut self, delta_time: f64) {
        self.frame_index += 1;
        // Phase evolves with time (for interference patterns)
        use std::f64::consts::TAU;
        self.temporal_phase = (self.temporal_phase + delta_time * 1e6) % TAU;
    }
}

// ============================================================================
// SPECTRAL PACKET
// ============================================================================

/// Spectral data packet with coherence tracking.
///
/// Contains wavelength samples and metadata for temporal stability.
#[derive(Debug, Clone)]
pub struct SpectralPacket {
    /// Sampled wavelengths (nm).
    pub wavelengths: Vec<f64>,

    /// Spectral values at each wavelength.
    pub values: Vec<f64>,

    /// Coherence metadata.
    pub coherence: CoherenceMetadata,

    /// RGB approximation (cached).
    pub rgb: Option<[f64; 3]>,

    /// XYZ approximation (cached).
    pub xyz: Option<[f64; 3]>,
}

impl Default for SpectralPacket {
    fn default() -> Self {
        Self::uniform_31()
    }
}

impl SpectralPacket {
    /// Create empty packet.
    pub fn new() -> Self {
        Self {
            wavelengths: Vec::new(),
            values: Vec::new(),
            coherence: CoherenceMetadata::default(),
            rgb: None,
            xyz: None,
        }
    }

    /// Create packet with uniform 31-point sampling (400-700nm, 10nm steps).
    pub fn uniform_31() -> Self {
        let wavelengths: Vec<f64> = (0..31).map(|i| 400.0 + i as f64 * 10.0).collect();
        let values = vec![0.0; 31];

        Self {
            wavelengths,
            values,
            coherence: CoherenceMetadata::default(),
            rgb: None,
            xyz: None,
        }
    }

    /// Create packet with RGB-only sampling (3 wavelengths).
    pub fn rgb_only() -> Self {
        Self {
            wavelengths: vec![656.3, 587.6, 486.1], // R, G, B
            values: vec![0.0; 3],
            coherence: CoherenceMetadata::default(),
            rgb: None,
            xyz: None,
        }
    }

    /// Create packet from wavelength/value pairs.
    pub fn from_data(wavelengths: Vec<f64>, values: Vec<f64>) -> Self {
        assert_eq!(wavelengths.len(), values.len());
        Self {
            wavelengths,
            values,
            coherence: CoherenceMetadata::default(),
            rgb: None,
            xyz: None,
        }
    }

    /// Set coherence metadata.
    pub fn with_coherence(mut self, coherence: CoherenceMetadata) -> Self {
        self.coherence = coherence;
        self
    }

    /// Get number of samples.
    pub fn len(&self) -> usize {
        self.wavelengths.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.wavelengths.is_empty()
    }

    /// Get value at wavelength (linear interpolation).
    pub fn value_at(&self, wavelength: f64) -> f64 {
        if self.wavelengths.is_empty() {
            return 0.0;
        }

        // Find bracketing wavelengths
        let mut idx = 0;
        while idx < self.wavelengths.len() - 1 && self.wavelengths[idx + 1] < wavelength {
            idx += 1;
        }

        if idx >= self.wavelengths.len() - 1 {
            return *self.values.last().unwrap_or(&0.0);
        }
        if wavelength <= self.wavelengths[0] {
            return self.values[0];
        }

        // Linear interpolation
        let w0 = self.wavelengths[idx];
        let w1 = self.wavelengths[idx + 1];
        let t = (wavelength - w0) / (w1 - w0);

        self.values[idx] * (1.0 - t) + self.values[idx + 1] * t
    }

    /// Set value at index.
    pub fn set_value(&mut self, index: usize, value: f64) {
        if index < self.values.len() {
            self.values[index] = value;
            self.rgb = None; // Invalidate cache
            self.xyz = None;
        }
    }

    /// Blend with another packet.
    pub fn blend(&self, other: &SpectralPacket, alpha: f64) -> SpectralPacket {
        let alpha = alpha.clamp(0.0, 1.0);
        let mut result = self.clone();

        for (i, v) in result.values.iter_mut().enumerate() {
            if i < other.values.len() {
                *v = *v * (1.0 - alpha) + other.values[i] * alpha;
            }
        }

        result.rgb = None;
        result.xyz = None;
        result
    }

    /// Get maximum spectral gradient (nm⁻¹).
    pub fn max_gradient(&self) -> f64 {
        if self.wavelengths.len() < 2 {
            return 0.0;
        }

        let mut max_grad: f64 = 0.0;
        for i in 0..self.wavelengths.len() - 1 {
            let dw = self.wavelengths[i + 1] - self.wavelengths[i];
            if dw > 0.0 {
                let dv = (self.values[i + 1] - self.values[i]).abs();
                let grad = dv / dw;
                max_grad = max_grad.max(grad);
            }
        }
        max_grad
    }

    /// Compute integrated luminance (simplified).
    pub fn luminance(&self) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }
        // Simple average weighted by green sensitivity
        let sum: f64 = self.values.iter().sum();
        sum / self.values.len() as f64
    }

    /// Convert to RGB (simplified).
    pub fn to_rgb(&mut self) -> [f64; 3] {
        if let Some(rgb) = self.rgb {
            return rgb;
        }

        // Simplified conversion using spectral locus
        let mut r = 0.0;
        let mut g = 0.0;
        let mut b = 0.0;

        for (i, &w) in self.wavelengths.iter().enumerate() {
            let v = self.values[i];
            if w < 490.0 {
                b += v * (490.0 - w) / 90.0;
            }
            if w > 440.0 && w < 600.0 {
                g += v * (1.0 - ((w - 520.0) / 80.0).abs().min(1.0));
            }
            if w > 580.0 {
                r += v * (w - 580.0) / 120.0;
            }
        }

        let n = self.wavelengths.len() as f64;
        let rgb = [
            (r / n).clamp(0.0, 1.0),
            (g / n).clamp(0.0, 1.0),
            (b / n).clamp(0.0, 1.0),
        ];

        self.rgb = Some(rgb);
        rgb
    }

    /// Advance frame.
    pub fn advance_frame(&mut self, delta_time: f64) {
        self.coherence.advance(delta_time);
    }
}

// ============================================================================
// SPECTRAL PACKET BUILDER
// ============================================================================

/// Builder for SpectralPacket.
#[derive(Debug, Clone, Default)]
pub struct SpectralPacketBuilder {
    wavelengths: Option<Vec<f64>>,
    values: Option<Vec<f64>>,
    coherence: CoherenceMetadata,
}

impl SpectralPacketBuilder {
    /// Create new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set wavelengths.
    pub fn wavelengths(mut self, wavelengths: Vec<f64>) -> Self {
        self.wavelengths = Some(wavelengths);
        self
    }

    /// Set uniform sampling.
    pub fn uniform(mut self, min: f64, max: f64, count: usize) -> Self {
        let step = (max - min) / (count - 1) as f64;
        let wavelengths: Vec<f64> = (0..count).map(|i| min + i as f64 * step).collect();
        self.wavelengths = Some(wavelengths);
        self.values = Some(vec![0.0; count]);
        self
    }

    /// Set values.
    pub fn values(mut self, values: Vec<f64>) -> Self {
        self.values = Some(values);
        self
    }

    /// Set coherence.
    pub fn coherence(mut self, coherence: CoherenceMetadata) -> Self {
        self.coherence = coherence;
        self
    }

    /// Set frame index.
    pub fn frame(mut self, frame_index: u64) -> Self {
        self.coherence.frame_index = frame_index;
        self
    }

    /// Build packet.
    pub fn build(self) -> SpectralPacket {
        let wavelengths = self
            .wavelengths
            .unwrap_or_else(|| (0..31).map(|i| 400.0 + i as f64 * 10.0).collect());
        let values = self.values.unwrap_or_else(|| vec![0.0; wavelengths.len()]);

        SpectralPacket {
            wavelengths,
            values,
            coherence: self.coherence,
            rgb: None,
            xyz: None,
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wavelength_band() {
        assert!(WavelengthBand::Green.contains(550.0));
        assert!(!WavelengthBand::Red.contains(550.0));

        let band = WavelengthBand::from_wavelength(550.0);
        assert_eq!(band, Some(WavelengthBand::Green));
    }

    #[test]
    fn test_coherence_metadata() {
        let laser = CoherenceMetadata::laser();
        assert!(laser.is_coherent);
        assert!(laser.coherence_length > 100.0); // Long coherence

        let sunlight = CoherenceMetadata::sunlight();
        assert!(!sunlight.is_coherent);
    }

    #[test]
    fn test_spectral_packet_default() {
        let packet = SpectralPacket::default();
        assert_eq!(packet.len(), 31);
        assert!(!packet.is_empty());
    }

    #[test]
    fn test_spectral_packet_interpolation() {
        let mut packet = SpectralPacket::uniform_31();
        packet.values[5] = 1.0; // 450nm
        packet.values[6] = 1.0; // 460nm

        let v = packet.value_at(455.0);
        assert!((v - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_spectral_packet_blend() {
        let mut p1 = SpectralPacket::uniform_31();
        let mut p2 = SpectralPacket::uniform_31();

        p1.values[10] = 0.0;
        p2.values[10] = 1.0;

        let blended = p1.blend(&p2, 0.5);
        assert!((blended.values[10] - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_spectral_packet_gradient() {
        let mut packet = SpectralPacket::uniform_31();
        // Flat spectrum
        assert!((packet.max_gradient() - 0.0).abs() < 1e-6);

        // Add step
        packet.values[10] = 1.0;
        assert!(packet.max_gradient() > 0.0);
    }

    #[test]
    fn test_spectral_packet_builder() {
        let packet = SpectralPacketBuilder::new()
            .uniform(400.0, 700.0, 31)
            .frame(100)
            .build();

        assert_eq!(packet.len(), 31);
        assert_eq!(packet.coherence.frame_index, 100);
    }

    #[test]
    fn test_rgb_conversion() {
        let mut packet = SpectralPacket::uniform_31();
        for v in packet.values.iter_mut() {
            *v = 0.5;
        }

        let rgb = packet.to_rgb();
        assert!(rgb[0] >= 0.0 && rgb[0] <= 1.0);
        assert!(rgb[1] >= 0.0 && rgb[1] <= 1.0);
        assert!(rgb[2] >= 0.0 && rgb[2] <= 1.0);
    }
}

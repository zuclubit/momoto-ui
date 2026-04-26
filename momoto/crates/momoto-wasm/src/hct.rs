// =============================================================================
// momoto-wasm: HCT Color Space Bindings
// File: crates/momoto-wasm/src/hct.rs
//
// Exposes HCT (Hue-Chroma-Tone) and CAM16 from momoto-core via wasm-bindgen.
// =============================================================================

use momoto_core::color::Color as CoreColor;
use momoto_core::space::hct::HCT as CoreHCT;
use wasm_bindgen::prelude::*;

// =============================================================================
// HCT struct
// =============================================================================

/// HCT (Hue, Chroma, Tone) — Google Material Design 3 perceptual color space.
///
/// - **Hue**: CAM16 hue angle (0–360°)
/// - **Chroma**: CAM16 chroma (≥ 0)
/// - **Tone**: CIELAB L* lightness (0–100)
#[wasm_bindgen]
pub struct HCT {
    inner: CoreHCT,
}

#[wasm_bindgen]
impl HCT {
    /// Create an HCT color from hue, chroma, and tone.
    #[wasm_bindgen(constructor)]
    pub fn new(hue: f64, chroma: f64, tone: f64) -> Self {
        Self {
            inner: CoreHCT::new(hue, chroma, tone),
        }
    }

    /// Convert an sRGB hex string to HCT.
    ///
    /// # Arguments
    /// * `hex` — hex color string (e.g. "#3a7bd5" or "3a7bd5")
    ///
    /// # Returns
    /// HCT instance, or HCT(0, 0, 0) if hex is invalid.
    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: &str) -> Self {
        use momoto_core::color::cvd::parse_hex;
        match parse_hex(hex) {
            Some(c) => Self {
                inner: CoreHCT::from_color(&c),
            },
            None => Self {
                inner: CoreHCT::new(0.0, 0.0, 0.0),
            },
        }
    }

    /// Convert from ARGB integer (0xAARRGGBB).
    #[wasm_bindgen(js_name = "fromArgb")]
    pub fn from_argb(argb: u32) -> Self {
        Self {
            inner: CoreHCT::from_argb(argb),
        }
    }

    /// CAM16 hue angle in degrees (0–360°).
    #[wasm_bindgen(getter)]
    pub fn hue(&self) -> f64 {
        self.inner.hue
    }

    /// CAM16 chroma (non-negative; maximum varies with tone and hue).
    #[wasm_bindgen(getter)]
    pub fn chroma(&self) -> f64 {
        self.inner.chroma
    }

    /// CIELAB L* tone (0 = black, 100 = white).
    #[wasm_bindgen(getter)]
    pub fn tone(&self) -> f64 {
        self.inner.tone
    }

    /// Convert HCT to an ARGB integer (0xFF_RR_GG_BB).
    #[wasm_bindgen(js_name = "toArgb")]
    pub fn to_argb(&self) -> u32 {
        self.inner.to_argb()
    }

    /// Convert HCT to a hex color string (e.g. "#3a7bd5").
    #[wasm_bindgen(js_name = "toHex")]
    pub fn to_hex(&self) -> String {
        let [r, g, b] = self.inner.to_color().to_srgb8();
        format!("#{:02x}{:02x}{:02x}", r, g, b)
    }

    /// Convert HCT to OKLCH flat array `[L, C, H]`.
    #[wasm_bindgen(js_name = "toOklch")]
    pub fn to_oklch(&self) -> Box<[f64]> {
        use momoto_core::space::oklch::OKLCH;
        let color = self.inner.to_color();
        let lch = OKLCH::from_color(&color);
        vec![lch.l, lch.c, lch.h].into_boxed_slice()
    }

    /// Clone with a different tone (preserves hue and chroma).
    #[wasm_bindgen(js_name = "withTone")]
    pub fn with_tone(&self, tone: f64) -> HCT {
        HCT {
            inner: CoreHCT::new(self.inner.hue, self.inner.chroma, tone.clamp(0.0, 100.0)),
        }
    }

    /// Clone with a different chroma (preserves hue and tone).
    #[wasm_bindgen(js_name = "withChroma")]
    pub fn with_chroma(&self, chroma: f64) -> HCT {
        HCT {
            inner: CoreHCT::new(self.inner.hue, chroma.max(0.0), self.inner.tone),
        }
    }

    /// Clone with a different hue (preserves chroma and tone).
    #[wasm_bindgen(js_name = "withHue")]
    pub fn with_hue(&self, hue: f64) -> HCT {
        HCT {
            inner: CoreHCT::new(hue.rem_euclid(360.0), self.inner.chroma, self.inner.tone),
        }
    }

    /// Clamp chroma to the maximum achievable in the sRGB gamut.
    #[wasm_bindgen(js_name = "clampToGamut")]
    pub fn clamp_to_gamut(&self) -> HCT {
        HCT {
            inner: self.inner.clamp_to_gamut(),
        }
    }
}

// =============================================================================
// Free functions
// =============================================================================

/// Convert a hex color string to HCT flat array `[hue, chroma, tone]`.
///
/// Returns `[0, 0, 0]` if the hex string is invalid.
#[wasm_bindgen(js_name = "hexToHct")]
pub fn hex_to_hct(hex: &str) -> Box<[f64]> {
    use momoto_core::color::cvd::parse_hex;
    match parse_hex(hex) {
        Some(c) => {
            let hct = CoreHCT::from_color(&c);
            vec![hct.hue, hct.chroma, hct.tone].into_boxed_slice()
        }
        None => vec![0.0, 0.0, 0.0].into_boxed_slice(),
    }
}

/// Convert HCT components to a hex color string.
///
/// Chroma may be clamped to the sRGB gamut boundary.
#[wasm_bindgen(js_name = "hctToHex")]
pub fn hct_to_hex(hue: f64, chroma: f64, tone: f64) -> String {
    let hct = CoreHCT::new(hue, chroma, tone);
    let [r, g, b] = hct.to_color().to_srgb8();
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

/// Convert HCT to OKLCH flat array `[L, C, H]`.
#[wasm_bindgen(js_name = "hctToOklch")]
pub fn hct_to_oklch(hue: f64, chroma: f64, tone: f64) -> Box<[f64]> {
    use momoto_core::space::oklch::OKLCH;
    let hct = CoreHCT::new(hue, chroma, tone);
    let lch = OKLCH::from_color(&hct.to_color());
    vec![lch.l, lch.c, lch.h].into_boxed_slice()
}

/// Convert OKLCH to HCT flat array `[hue, chroma, tone]`.
#[wasm_bindgen(js_name = "oklchToHct")]
pub fn oklch_to_hct(l: f64, c: f64, h: f64) -> Box<[f64]> {
    use momoto_core::space::oklch::OKLCH;
    let color = OKLCH::new(l, c, h).to_color();
    let hct = CoreHCT::from_color(&color);
    vec![hct.hue, hct.chroma, hct.tone].into_boxed_slice()
}

/// Generate a tonal palette in HCT space.
///
/// Returns flat array `[H0, C0, T0, H1, C1, T1, ...]` for tones:
/// 0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 95, 99, 100 (13 steps).
#[wasm_bindgen(js_name = "hctTonalPalette")]
pub fn hct_tonal_palette(hue: f64, chroma: f64) -> Box<[f64]> {
    const TONES: [f64; 13] = [
        0.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 95.0, 99.0, 100.0,
    ];
    let mut out = Vec::with_capacity(TONES.len() * 3);
    for &tone in &TONES {
        let hct = CoreHCT::new(hue, chroma, tone);
        let clamped = hct.clamp_to_gamut();
        out.push(clamped.hue);
        out.push(clamped.chroma);
        out.push(clamped.tone);
    }
    out.into_boxed_slice()
}

/// Get the maximum achievable chroma for a given hue and tone.
///
/// Useful for building a UI slider that shows the valid chroma range.
#[wasm_bindgen(js_name = "hctMaxChroma")]
pub fn hct_max_chroma(hue: f64, tone: f64) -> f64 {
    // Binary search: find max C such that HCT(hue, C, tone) is in gamut
    let mut lo = 0.0_f64;
    let mut hi = 200.0_f64;
    for _ in 0..50 {
        let mid = (lo + hi) / 2.0;
        let hct = CoreHCT::new(hue, mid, tone);
        let color = hct.to_color();
        let all_in_gamut = color.srgb.iter().all(|&ch| ch >= -0.001 && ch <= 1.001);
        if all_in_gamut {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

//! # Visual Design Experience Generator
//!
//! Generates coherent visual design experiences (themes) based on perceptual
//! color science using OKLCH color space for perceptually uniform manipulation.

use momoto_core::color::Color;
use momoto_core::space::oklch::OKLCH;
use serde::{Deserialize, Serialize};

// ============================================================================
// ThemePreset
// ============================================================================

/// A preset theme category that drives the color palette generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemePreset {
    /// Deep ocean blues and teals.
    Ocean,
    /// Natural forest greens and earth tones.
    Forest,
    /// Warm sunset oranges, reds, and purples.
    Sunset,
    /// Dark midnight blues and deep purples.
    Midnight,
    /// Cool arctic whites, icy blues, and silvers.
    Arctic,
    /// Warm desert sands, terracottas, and burnt oranges.
    Desert,
    /// Deep cosmic purples, nebula blues, and stellar accents.
    Cosmic,
    /// Japanese cherry blossom pinks, soft creams, and dusty roses.
    Sakura,
    /// Urban concrete grays, steel blues, and neon accents.
    Urban,
    /// Clean minimal whites, near-blacks, and a single accent.
    Minimal,
}

impl ThemePreset {
    /// Return the canonical display name of this preset.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Ocean => "Ocean",
            Self::Forest => "Forest",
            Self::Sunset => "Sunset",
            Self::Midnight => "Midnight",
            Self::Arctic => "Arctic",
            Self::Desert => "Desert",
            Self::Cosmic => "Cosmic",
            Self::Sakura => "Sakura",
            Self::Urban => "Urban",
            Self::Minimal => "Minimal",
        }
    }

    /// Parse a preset from a lowercase string identifier.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().trim() {
            "ocean" => Some(Self::Ocean),
            "forest" => Some(Self::Forest),
            "sunset" => Some(Self::Sunset),
            "midnight" => Some(Self::Midnight),
            "arctic" => Some(Self::Arctic),
            "desert" => Some(Self::Desert),
            "cosmic" => Some(Self::Cosmic),
            "sakura" => Some(Self::Sakura),
            "urban" => Some(Self::Urban),
            "minimal" => Some(Self::Minimal),
            _ => None,
        }
    }

    /// All known presets in canonical order.
    pub fn all() -> &'static [ThemePreset] {
        &[
            Self::Ocean,
            Self::Forest,
            Self::Sunset,
            Self::Midnight,
            Self::Arctic,
            Self::Desert,
            Self::Cosmic,
            Self::Sakura,
            Self::Urban,
            Self::Minimal,
        ]
    }
}

// ============================================================================
// OKLCH palette seeds — (L, C, H) per preset, 5 roles
// ============================================================================

/// Seed palette: (primary, secondary, accent, surface, text) as OKLCH tuples.
struct PaletteSeed {
    primary: (f64, f64, f64),
    secondary: (f64, f64, f64),
    accent: (f64, f64, f64),
    surface: (f64, f64, f64),
    text: (f64, f64, f64),
    description: &'static str,
}

fn seed_for(preset: ThemePreset) -> PaletteSeed {
    match preset {
        ThemePreset::Ocean => PaletteSeed {
            primary: (0.40, 0.18, 240.0),
            secondary: (0.55, 0.15, 192.0),
            accent: (0.70, 0.12, 200.0),
            surface: (0.97, 0.02, 220.0),
            text: (0.18, 0.05, 240.0),
            description: "Deep ocean depths with crystalline teal highlights",
        },
        ThemePreset::Forest => PaletteSeed {
            primary: (0.40, 0.15, 145.0),
            secondary: (0.55, 0.12, 110.0),
            accent: (0.65, 0.18, 80.0),
            surface: (0.96, 0.02, 130.0),
            text: (0.17, 0.04, 145.0),
            description: "Ancient forest greens rooted in earthy browns",
        },
        ThemePreset::Sunset => PaletteSeed {
            primary: (0.55, 0.22, 35.0),
            secondary: (0.60, 0.20, 15.0),
            accent: (0.72, 0.18, 60.0),
            surface: (0.97, 0.03, 40.0),
            text: (0.18, 0.06, 30.0),
            description: "Warm horizon fire with golden amber transitions",
        },
        ThemePreset::Midnight => PaletteSeed {
            primary: (0.28, 0.16, 265.0),
            secondary: (0.38, 0.18, 285.0),
            accent: (0.75, 0.20, 310.0),
            surface: (0.12, 0.03, 265.0),
            text: (0.93, 0.02, 265.0),
            description: "Deep midnight sky with ethereal violet accents",
        },
        ThemePreset::Arctic => PaletteSeed {
            primary: (0.72, 0.08, 210.0),
            secondary: (0.82, 0.05, 200.0),
            accent: (0.50, 0.14, 225.0),
            surface: (0.98, 0.01, 210.0),
            text: (0.15, 0.04, 215.0),
            description: "Glacial clarity with icy blue-silver luminescence",
        },
        ThemePreset::Desert => PaletteSeed {
            primary: (0.58, 0.16, 55.0),
            secondary: (0.65, 0.14, 40.0),
            accent: (0.50, 0.20, 30.0),
            surface: (0.97, 0.03, 60.0),
            text: (0.20, 0.06, 45.0),
            description: "Sun-baked terracotta and warm ochre dunes",
        },
        ThemePreset::Cosmic => PaletteSeed {
            primary: (0.30, 0.20, 280.0),
            secondary: (0.40, 0.22, 260.0),
            accent: (0.80, 0.18, 320.0),
            surface: (0.08, 0.04, 275.0),
            text: (0.92, 0.03, 280.0),
            description: "Infinite cosmos purple with nebula pink highlights",
        },
        ThemePreset::Sakura => PaletteSeed {
            primary: (0.72, 0.12, 355.0),
            secondary: (0.78, 0.10, 10.0),
            accent: (0.55, 0.18, 340.0),
            surface: (0.98, 0.02, 5.0),
            text: (0.20, 0.05, 350.0),
            description: "Japanese cherry blossom with soft petal softness",
        },
        ThemePreset::Urban => PaletteSeed {
            primary: (0.38, 0.06, 220.0),
            secondary: (0.52, 0.04, 210.0),
            accent: (0.65, 0.24, 195.0),
            surface: (0.95, 0.01, 220.0),
            text: (0.12, 0.02, 220.0),
            description: "Steel and concrete with electric teal accents",
        },
        ThemePreset::Minimal => PaletteSeed {
            primary: (0.20, 0.00, 0.0),
            secondary: (0.55, 0.00, 0.0),
            accent: (0.55, 0.20, 250.0),
            surface: (0.99, 0.00, 0.0),
            text: (0.10, 0.00, 0.0),
            description: "Surgical minimal with a single resonant accent",
        },
    }
}

// ============================================================================
// Internal helpers
// ============================================================================

/// Convert an OKLCH tuple to a clamped sRGB hex string.
fn oklch_to_hex(l: f64, c: f64, h: f64) -> String {
    let oklch = OKLCH::new(l, c, h);
    let color = oklch.to_color();
    let [r, g, b] = color.to_srgb8();
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

/// Adjust OKLCH seed toward a target color (hex string). Blends L/C/H.
fn blend_oklch_toward_hex(
    seed_l: f64,
    seed_c: f64,
    seed_h: f64,
    target_hex: &str,
    weight: f64, // 0.0 = pure seed, 1.0 = pure target
) -> (f64, f64, f64) {
    let w = weight.clamp(0.0, 1.0);
    if let Ok(target_color) = Color::from_hex(target_hex) {
        let t = OKLCH::from_color(&target_color);
        let tl = t.l as f64;
        let tc = t.c as f64;
        // Hue blending: take the shorter arc
        let th = t.h as f64;
        let dh = {
            let diff = th - seed_h;
            if diff > 180.0 {
                diff - 360.0
            } else if diff < -180.0 {
                diff + 360.0
            } else {
                diff
            }
        };
        (
            seed_l * (1.0 - w) + tl * w,
            seed_c * (1.0 - w) + tc * w,
            (seed_h + dh * w).rem_euclid(360.0),
        )
    } else {
        (seed_l, seed_c, seed_h)
    }
}

/// Generate CSS custom properties block from the five hex colors.
fn generate_css_variables(
    theme_name: &str,
    primary: &str,
    secondary: &str,
    accent: &str,
    surface: &str,
    text: &str,
) -> String {
    // Also derive muted surface variants
    let primary_color = Color::from_hex(primary).unwrap_or_else(|_| Color::from_srgb8(0, 0, 255));
    let p = OKLCH::from_color(&primary_color);
    let primary_muted = oklch_to_hex(
        (p.l as f64 + 0.30).min(0.95),
        (p.c as f64 * 0.35) as f64,
        p.h as f64,
    );
    let primary_dark = oklch_to_hex(
        (p.l as f64 - 0.12).max(0.05),
        (p.c as f64 * 0.85) as f64,
        p.h as f64,
    );

    format!(
        r#":root {{
  /* Momoto Theme: {theme_name} */
  --color-primary:        {primary};
  --color-primary-muted:  {primary_muted};
  --color-primary-dark:   {primary_dark};
  --color-secondary:      {secondary};
  --color-accent:         {accent};
  --color-surface:        {surface};
  --color-text:           {text};

  /* Semantic aliases */
  --color-background:     {surface};
  --color-foreground:     {text};
  --color-brand:          {primary};
  --color-interactive:    {accent};
  --color-muted:          {primary_muted};
}}"#,
        theme_name = theme_name,
        primary = primary,
        primary_muted = primary_muted,
        primary_dark = primary_dark,
        secondary = secondary,
        accent = accent,
        surface = surface,
        text = text,
    )
}

// ============================================================================
// VisualExperience
// ============================================================================

/// A fully-resolved visual design experience with all five color roles
/// plus generated CSS custom properties.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualExperience {
    /// Human-readable name for this experience.
    pub name: String,
    /// Short description of the visual feel.
    pub description: String,
    /// Primary brand color in 6-digit hex (`#RRGGBB`).
    pub primary_hex: String,
    /// Secondary supporting color.
    pub secondary_hex: String,
    /// Accent / interactive color.
    pub accent_hex: String,
    /// Surface / background color.
    pub surface_hex: String,
    /// Body text color.
    pub text_hex: String,
    /// Ready-to-use CSS custom properties block.
    pub css_variables: String,
    /// Canonical theme name string.
    pub theme_name: String,
    /// The source preset.
    pub preset: ThemePreset,
}

// ============================================================================
// ExperienceBuilder
// ============================================================================

/// Builder for constructing a [`VisualExperience`] step by step.
#[derive(Debug, Clone)]
pub struct ExperienceBuilder {
    preset: ThemePreset,
    override_color: Option<String>,
}

impl ExperienceBuilder {
    /// Create a new builder defaulting to [`ThemePreset::Minimal`].
    pub fn new() -> Self {
        Self {
            preset: ThemePreset::Minimal,
            override_color: None,
        }
    }

    /// Set the theme preset.
    pub fn with_preset(mut self, preset: ThemePreset) -> Self {
        self.preset = preset;
        self
    }

    /// Override the primary hue by blending toward the given hex color.
    /// This shifts the whole palette while preserving preset semantics.
    pub fn with_color(mut self, hex: &str) -> Self {
        self.override_color = Some(hex.to_string());
        self
    }

    /// Consume the builder and produce the [`VisualExperience`].
    pub fn build(self) -> VisualExperience {
        let gen = ExperienceGenerator::new();
        match self.override_color {
            Some(ref hex) => gen.generate_with_color(hex, self.preset),
            None => gen.generate(self.preset),
        }
    }
}

impl Default for ExperienceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// ExperienceGenerator
// ============================================================================

/// Stateless generator that converts a [`ThemePreset`] (and optional override
/// color) into a fully-resolved [`VisualExperience`].
#[derive(Debug, Clone)]
pub struct ExperienceGenerator;

impl ExperienceGenerator {
    /// Create a new generator.
    pub fn new() -> Self {
        Self
    }

    /// Generate a [`VisualExperience`] from a preset alone.
    pub fn generate(&self, preset: ThemePreset) -> VisualExperience {
        let seed = seed_for(preset);
        let (pl, pc, ph) = seed.primary;
        let (sl, sc, sh) = seed.secondary;
        let (al, ac, ah) = seed.accent;
        let (sul, suc, suh) = seed.surface;
        let (tl, tc, th) = seed.text;

        let primary_hex = oklch_to_hex(pl, pc, ph);
        let secondary_hex = oklch_to_hex(sl, sc, sh);
        let accent_hex = oklch_to_hex(al, ac, ah);
        let surface_hex = oklch_to_hex(sul, suc, suh);
        let text_hex = oklch_to_hex(tl, tc, th);
        let theme_name = preset.name().to_string();
        let css_variables = generate_css_variables(
            &theme_name,
            &primary_hex,
            &secondary_hex,
            &accent_hex,
            &surface_hex,
            &text_hex,
        );

        VisualExperience {
            name: format!("{} Theme", theme_name),
            description: seed.description.to_string(),
            primary_hex,
            secondary_hex,
            accent_hex,
            surface_hex,
            text_hex,
            css_variables,
            theme_name,
            preset,
        }
    }

    /// Generate a [`VisualExperience`] blending the preset with an override color.
    ///
    /// The override shifts the primary hue toward the supplied hex while keeping
    /// the secondary, accent, surface and text roles perceptually consistent
    /// with the original seed ratios.
    pub fn generate_with_color(&self, color_hex: &str, preset: ThemePreset) -> VisualExperience {
        let seed = seed_for(preset);
        let (pl, pc, ph) = seed.primary;
        let (sl, sc, sh) = seed.secondary;
        let (al, ac, ah) = seed.accent;
        let (sul, suc, suh) = seed.surface;
        let (tl, tc, th) = seed.text;

        // Blend primary 50% toward override color
        let (npl, npc, nph) = blend_oklch_toward_hex(pl, pc, ph, color_hex, 0.50);

        // Rotate secondary & accent by the same hue delta
        let hue_delta = {
            let raw = nph - ph;
            if raw > 180.0 {
                raw - 360.0
            } else if raw < -180.0 {
                raw + 360.0
            } else {
                raw
            }
        };
        let (nsl, nsc, nsh) = (sl, sc, (sh + hue_delta).rem_euclid(360.0));
        let (nal, nac, nah) = (al, ac, (ah + hue_delta).rem_euclid(360.0));
        // Surface and text get a very subtle hue shift
        let (nsul, nsuc, nsuh) = (sul, suc, (suh + hue_delta * 0.2).rem_euclid(360.0));
        let (ntl, ntc, nth) = (tl, tc, (th + hue_delta * 0.2).rem_euclid(360.0));

        let primary_hex = oklch_to_hex(npl, npc, nph);
        let secondary_hex = oklch_to_hex(nsl, nsc, nsh);
        let accent_hex = oklch_to_hex(nal, nac, nah);
        let surface_hex = oklch_to_hex(nsul, nsuc, nsuh);
        let text_hex = oklch_to_hex(ntl, ntc, nth);
        let theme_name = format!("{} (Custom)", preset.name());
        let css_variables = generate_css_variables(
            &theme_name,
            &primary_hex,
            &secondary_hex,
            &accent_hex,
            &surface_hex,
            &text_hex,
        );

        VisualExperience {
            name: format!("{} Theme (Custom Color)", preset.name()),
            description: format!("{} — adapted to {}", seed.description, color_hex),
            primary_hex,
            secondary_hex,
            accent_hex,
            surface_hex,
            text_hex,
            css_variables,
            theme_name,
            preset,
        }
    }
}

impl Default for ExperienceGenerator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Free functions
// ============================================================================

/// Generate a [`VisualExperience`] from a preset name string (case-insensitive).
///
/// Falls back to [`ThemePreset::Minimal`] if the name is not recognised.
pub fn generate_experience(preset_name: &str) -> VisualExperience {
    let preset = ThemePreset::from_str(preset_name).unwrap_or(ThemePreset::Minimal);
    ExperienceGenerator::new().generate(preset)
}

/// Generate a [`VisualExperience`] blending a custom color into a named preset.
///
/// Falls back to [`ThemePreset::Minimal`] if the preset name is not recognised.
pub fn generate_experience_with_color(color_hex: &str, preset_name: &str) -> VisualExperience {
    let preset = ThemePreset::from_str(preset_name).unwrap_or(ThemePreset::Minimal);
    ExperienceGenerator::new().generate_with_color(color_hex, preset)
}

/// Return the list of all known preset names in lowercase.
pub fn list_presets() -> Vec<&'static str> {
    vec![
        "ocean", "forest", "sunset", "midnight", "arctic", "desert", "cosmic", "sakura", "urban",
        "minimal",
    ]
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_ocean() {
        let exp = generate_experience("ocean");
        assert_eq!(exp.preset, ThemePreset::Ocean);
        assert!(exp.primary_hex.starts_with('#'));
        assert_eq!(exp.primary_hex.len(), 7);
        assert!(!exp.css_variables.is_empty());
        assert!(exp.css_variables.contains("--color-primary"));
    }

    #[test]
    fn test_generate_all_presets() {
        for name in list_presets() {
            let exp = generate_experience(name);
            assert_eq!(exp.primary_hex.len(), 7, "preset {} primary_hex bad", name);
            assert_eq!(exp.surface_hex.len(), 7, "preset {} surface_hex bad", name);
        }
    }

    #[test]
    fn test_generate_with_color() {
        let exp = generate_experience_with_color("#FF6600", "ocean");
        assert_eq!(exp.preset, ThemePreset::Ocean);
        assert!(exp.name.contains("Custom"));
    }

    #[test]
    fn test_builder() {
        let exp = ExperienceBuilder::new()
            .with_preset(ThemePreset::Cosmic)
            .with_color("#8800FF")
            .build();
        assert_eq!(exp.preset, ThemePreset::Cosmic);
        assert_eq!(exp.primary_hex.len(), 7);
    }

    #[test]
    fn test_list_presets_count() {
        assert_eq!(list_presets().len(), 10);
    }

    #[test]
    fn test_unknown_preset_fallback() {
        let exp = generate_experience("unknown_theme_xyz");
        assert_eq!(exp.preset, ThemePreset::Minimal);
    }

    #[test]
    fn test_css_variables_structure() {
        let exp = generate_experience("sunset");
        assert!(exp.css_variables.contains(":root"));
        assert!(exp.css_variables.contains("--color-primary:"));
        assert!(exp.css_variables.contains("--color-secondary:"));
        assert!(exp.css_variables.contains("--color-accent:"));
        assert!(exp.css_variables.contains("--color-surface:"));
        assert!(exp.css_variables.contains("--color-text:"));
    }
}

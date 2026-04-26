//! Enhanced CSS Backend for Apple Liquid Glass quality
//!
//! This module combines physics from `glass_physics` with CSS generation
//! to produce premium glass effects.
//!
//! ## Features
//!
//! - **Specular highlights** from Blinn-Phong model
//! - **Fresnel edge glow** from Schlick's approximation
//! - **Inner highlights** for top-light simulation
//! - **Multi-layer shadows** for depth perception
//! - **Backdrop saturation** boost
//!
//! ## Usage
//!
//! ```rust,ignore
//! use momoto_materials::css_enhanced::EnhancedCssBackend;
//! use momoto_core::backend::CssRenderConfig;
//! use momoto_core::material::GlassMaterial;
//! use momoto_core::evaluated::{Evaluable, MaterialContext};
//!
//! let glass = GlassMaterial::regular();
//! let ctx = MaterialContext::default();
//! let evaluated = glass.evaluate(&ctx);
//!
//! let config = CssRenderConfig::premium();
//! let css = EnhancedCssBackend::render(&evaluated, &config);
//! ```

use momoto_core::backend::css_config::CssRenderConfig;
use momoto_core::evaluated::{EvaluatedMaterial, LinearRgba};
use momoto_core::Color;

use crate::glass_physics::blinn_phong::{
    to_css_inner_glow, to_css_inner_highlight, to_css_secondary_specular, to_css_specular_highlight,
};
use crate::glass_physics::fresnel::{
    fresnel_outer_glow_params, to_css_fresnel_gradient, to_css_luminous_border,
};

/// Enhanced CSS Backend for Apple Liquid Glass quality
///
/// Generates complete CSS with physics-based effects:
/// - Multi-layer backgrounds with gradients
/// - Specular highlights positioned by light model
/// - Fresnel edge glow
/// - 4-layer elevation shadows
pub struct EnhancedCssBackend;

impl EnhancedCssBackend {
    /// Render material to enhanced CSS string
    ///
    /// Returns a complete CSS declaration block with all properties.
    pub fn render(material: &EvaluatedMaterial, config: &CssRenderConfig) -> String {
        let mut css_parts = Vec::new();

        // 1. Build background layers (ORDER: top to bottom)
        let background_layers = Self::generate_background_layers(material, config);
        if !background_layers.is_empty() {
            css_parts.push(format!("background: {};", background_layers.join(", ")));
        }

        // 2. Enhanced backdrop-filter
        if let Some(filter) = Self::generate_backdrop_filter(material, config) {
            css_parts.push(filter);
        }

        // 3. Box-shadow layers
        let shadows = Self::generate_box_shadows(config);
        if !shadows.is_empty() {
            css_parts.push(format!("box-shadow: {};", shadows.join(", ")));
        }

        // 4. Border
        if config.border_enabled {
            let border_color = if config.light_mode {
                "rgba(255, 255, 255, 0.22)"
            } else {
                "rgba(255, 255, 255, 0.12)"
            };
            css_parts.push(format!("border: 1px solid {};", border_color));
        }

        // 5. Border radius
        css_parts.push(format!("border-radius: {:.0}px;", config.border_radius));

        // 6. Opacity
        css_parts.push(format!("opacity: {:.2};", material.opacity));

        css_parts.join("\n")
    }

    /// Generate background layers as CSS gradients
    ///
    /// ## Apple Liquid Glass Layer Order (top to bottom)
    ///
    /// 1. **Primary Specular**: Bright light spot from Blinn-Phong
    /// 2. **Secondary Specular**: Fill light reflection
    /// 3. **Inner Highlight**: Top edge shine
    /// 4. **Inner Glow**: Soft internal luminosity (NEW)
    /// 5. **Fresnel Edge Glow**: Edge light catching
    /// 6. **Base Color**: Material tint
    fn generate_background_layers(
        material: &EvaluatedMaterial,
        config: &CssRenderConfig,
    ) -> Vec<String> {
        let mut layers = Vec::new();

        // Layer 1: Primary specular highlight (Blinn-Phong)
        if config.specular_enabled {
            layers.push(to_css_specular_highlight(
                config.specular_intensity,
                config.specular_size,
                config.specular_position.0,
                config.specular_position.1,
            ));

            // Secondary specular (fill light) - now included at lower threshold
            if config.specular_intensity > 0.15 {
                layers.push(to_css_secondary_specular(
                    config.specular_intensity * 0.5,
                    28.0,
                ));
            }
        }

        // Layer 2: Inner top highlight
        if config.inner_highlight_enabled {
            layers.push(to_css_inner_highlight(
                config.inner_highlight_intensity,
                config.light_mode,
            ));
        }

        // Layer 3: Inner glow (NEW - creates internal luminosity)
        // This is the signature Apple Liquid Glass effect
        if config.inner_highlight_enabled && config.inner_highlight_intensity > 0.2 {
            layers.push(to_css_inner_glow(
                config.inner_highlight_intensity * 0.8,
                config.light_mode,
            ));
        }

        // Layer 4: Fresnel edge glow
        if config.fresnel_enabled {
            layers.push(to_css_fresnel_gradient(
                config.fresnel_intensity,
                config.light_mode,
            ));
        }

        // Layer 5: Base color from material
        let base_color = Self::to_css_color(&material.base_color, material.opacity);
        layers.push(base_color);

        layers
    }

    /// Generate backdrop-filter CSS
    fn generate_backdrop_filter(
        material: &EvaluatedMaterial,
        config: &CssRenderConfig,
    ) -> Option<String> {
        // Convert scattering radius (mm) to CSS pixels
        // CSS standard: 96px = 1 inch = 25.4mm
        const MM_TO_PX: f64 = 3.779527559;
        let blur_px = material.scattering_radius_mm * MM_TO_PX;

        if blur_px < 0.5 {
            return None;
        }

        if config.saturate && config.saturation_factor > 1.0 {
            Some(format!(
                "backdrop-filter: blur({:.0}px) saturate({:.1});",
                blur_px, config.saturation_factor
            ))
        } else {
            Some(format!("backdrop-filter: blur({:.0}px);", blur_px))
        }
    }

    /// Generate box-shadow layers
    ///
    /// ## Apple Liquid Glass Shadow System
    ///
    /// Creates a multi-layer shadow system:
    /// 1. **Top edge shine**: Bright inset at top
    /// 2. **Luminous border**: Glowing inner edge (NEW)
    /// 3. **Fresnel outer glow**: Light halo around element
    /// 4. **4-layer elevation shadows**: Depth perception
    fn generate_box_shadows(config: &CssRenderConfig) -> Vec<String> {
        let mut shadows = Vec::new();

        // Inner highlight shadow (top edge shine) - ENHANCED
        if config.inner_highlight_enabled {
            let opacity = if config.light_mode { 0.65 } else { 0.35 };
            shadows.push(format!(
                "inset 0 1px 2px rgba(255, 255, 255, {:.2})",
                opacity
            ));
            // Add second top highlight for extra depth
            let opacity2 = if config.light_mode { 0.35 } else { 0.15 };
            shadows.push(format!(
                "inset 0 2px 4px rgba(255, 255, 255, {:.2})",
                opacity2
            ));
        }

        // Luminous border glow (NEW - creates the signature edge glow)
        if config.border_enabled {
            shadows.push(to_css_luminous_border(
                config.fresnel_intensity.max(0.4),
                config.light_mode,
                config.border_radius,
            ));
            // Secondary inner border for depth
            let border_opacity = if config.light_mode { 0.25 } else { 0.12 };
            shadows.push(format!(
                "inset 0 0 0 1px rgba(255, 255, 255, {:.2})",
                border_opacity
            ));
        }

        // Fresnel outer glow - ENHANCED
        if config.fresnel_enabled && config.fresnel_intensity > 0.08 {
            let (blur_radius, opacity) =
                fresnel_outer_glow_params(config.fresnel_intensity, config.light_mode);
            // Primary outer glow
            shadows.push(format!(
                "0 0 {:.1}px rgba(255, 255, 255, {:.3})",
                blur_radius,
                opacity * 1.3
            ));
            // Secondary subtle outer glow for softness
            shadows.push(format!(
                "0 0 {:.1}px rgba(255, 255, 255, {:.3})",
                blur_radius * 2.0,
                opacity * 0.5
            ));
        }

        // Elevation shadows (4-layer system)
        if config.elevation > 0 {
            let elevation_shadows =
                Self::generate_elevation_shadows(config.elevation, config.light_mode);
            shadows.extend(elevation_shadows);
        }

        shadows
    }

    /// Generate 4-layer elevation shadows (Apple Liquid Glass style)
    ///
    /// ## Apple Quality Enhancement
    ///
    /// Creates dramatic, multi-layer shadows that give the glass
    /// a floating, three-dimensional appearance:
    /// - Contact shadow: Tight, dark anchor to surface
    /// - Primary shadow: Main depth perception
    /// - Ambient shadow: Soft environmental shadow
    /// - Atmosphere: Very soft, creates floating effect
    fn generate_elevation_shadows(level: u8, light_mode: bool) -> Vec<String> {
        // ENHANCED: More dramatic shadows for Apple-quality depth
        let base_alpha = if light_mode { 0.18 } else { 0.35 };
        let factor = (level as f64 / 3.0).max(0.5); // Minimum factor for visibility

        vec![
            // Layer 1: Contact shadow (tight, dark anchor)
            format!(
                "0 {:.1}px {:.1}px rgba(0, 0, 0, {:.3})",
                1.0 * factor,
                2.0 * factor,
                base_alpha * 1.4
            ),
            // Layer 2: Primary shadow (main depth - ENHANCED)
            format!(
                "0 {:.1}px {:.1}px rgba(0, 0, 0, {:.3})",
                5.0 * factor,
                10.0 * factor,
                base_alpha * 1.0
            ),
            // Layer 3: Ambient shadow (soft, wide - ENHANCED)
            format!(
                "0 {:.1}px {:.1}px rgba(0, 0, 0, {:.3})",
                16.0 * factor,
                32.0 * factor,
                base_alpha * 0.7
            ),
            // Layer 4: Atmosphere (very soft, floating effect)
            format!(
                "0 {:.1}px {:.1}px rgba(0, 0, 0, {:.3})",
                32.0 * factor,
                64.0 * factor,
                base_alpha * 0.35
            ),
        ]
    }

    /// Convert LinearRgba to CSS oklch() color string
    fn to_css_color(color: &LinearRgba, alpha: f64) -> String {
        // Convert linear RGB to Color, then to OKLCH
        let rgb_color = Color::from_linear(color.r, color.g, color.b);
        let oklch = rgb_color.to_oklch();

        format!(
            "oklch({:.2} {:.2} {:.0} / {:.2})",
            oklch.l, oklch.c, oklch.h, alpha
        )
    }
}

/// Helper function to render enhanced CSS directly
///
/// Convenience function for quick rendering without creating backend instance.
pub fn render_enhanced_css(material: &EvaluatedMaterial, config: &CssRenderConfig) -> String {
    EnhancedCssBackend::render(material, config)
}

/// Helper function to render with default premium config
pub fn render_premium_css(material: &EvaluatedMaterial) -> String {
    let config = CssRenderConfig::premium();
    EnhancedCssBackend::render(material, &config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use momoto_core::evaluated::{EvaluationMetadata, MaterialType};

    fn create_test_material() -> EvaluatedMaterial {
        EvaluatedMaterial {
            base_color: LinearRgba::rgb(0.9, 0.9, 0.9),
            opacity: 0.85,
            scattering_radius_mm: 5.3,
            roughness: 0.15,
            fresnel_f0: 0.04,
            fresnel_edge_intensity: 0.8,
            index_of_refraction: Some(1.5),
            absorption: [0.1, 0.1, 0.1],
            scattering: [0.0, 0.0, 0.0],
            thickness_mm: 5.0,
            metallic: 0.0,
            specular_intensity: 0.8,
            specular_shininess: 50.0,
            anisotropy: None,
            emissive: [0.0, 0.0, 0.0],
            emissive_intensity: 0.0,
            clearcoat: None,
            iridescence: None,
            texture_noise: None,
            material_type: MaterialType::Glass,
            metadata: EvaluationMetadata::default(),
        }
    }

    #[test]
    fn test_enhanced_css_has_specular() {
        let material = create_test_material();
        let config = CssRenderConfig::premium();

        let css = EnhancedCssBackend::render(&material, &config);

        assert!(
            css.contains("radial-gradient"),
            "Should have specular gradient"
        );
        assert!(
            css.contains("linear-gradient"),
            "Should have inner highlight"
        );
    }

    #[test]
    fn test_enhanced_css_has_shadows() {
        let material = create_test_material();
        let config = CssRenderConfig::premium();

        let css = EnhancedCssBackend::render(&material, &config);

        // Should have multiple shadow layers
        let shadow_count = css.matches("rgba(0, 0, 0").count();
        assert!(
            shadow_count >= 4,
            "Should have 4+ shadow layers, found {}",
            shadow_count
        );
    }

    #[test]
    fn test_enhanced_css_has_backdrop_filter() {
        let material = create_test_material();
        let config = CssRenderConfig::premium();

        let css = EnhancedCssBackend::render(&material, &config);

        assert!(
            css.contains("backdrop-filter"),
            "Should have backdrop-filter"
        );
        assert!(css.contains("blur"), "Should have blur");
        assert!(css.contains("saturate"), "Should have saturate");
    }

    #[test]
    fn test_minimal_preset() {
        let material = create_test_material();
        let config = CssRenderConfig::minimal();

        let css = EnhancedCssBackend::render(&material, &config);

        // Should NOT have specular gradients
        assert!(
            !css.contains("radial-gradient"),
            "Minimal should not have specular"
        );
        // But should have basic blur
        assert!(
            css.contains("backdrop-filter"),
            "Should still have backdrop-filter"
        );
    }

    #[test]
    fn test_border_generation() {
        let material = create_test_material();
        let config = CssRenderConfig::premium();

        let css = EnhancedCssBackend::render(&material, &config);

        assert!(css.contains("border:"), "Should have border");
        assert!(css.contains("border-radius:"), "Should have border-radius");
    }

    #[test]
    fn test_dark_mode() {
        let material = create_test_material();
        let config = CssRenderConfig::dark_mode();

        let css = EnhancedCssBackend::render(&material, &config);

        // Should still render but with different opacities
        assert!(css.contains("background:"));
        assert!(css.contains("box-shadow:"));
    }
}

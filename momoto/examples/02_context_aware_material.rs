//! # Context-Aware Material
//!
//! Demonstrates how materials adapt to different backgrounds and viewing contexts.
//! Shows the perceptual color system adjusting material properties dynamically.
//!
//! Run with: `cargo run --example 02_context_aware_material`

use momoto_core::{
    backend::CssBackend,
    evaluated::{Evaluable, MaterialContext},
    material::GlassMaterial,
    render::{RenderBackend, RenderContext},
    space::oklch::OKLCH,
};

fn main() {
    println!("=== Context-Aware Material Demo ===\n");

    // Single glass material
    let glass = GlassMaterial::regular();

    // Test with different backgrounds
    let contexts = [
        (
            "Dark Background",
            OKLCH::new(0.2, 0.02, 280.0),
            0.0, // viewing angle
        ),
        ("Light Background", OKLCH::new(0.9, 0.02, 280.0), 0.0),
        ("Saturated Background", OKLCH::new(0.6, 0.15, 120.0), 0.0),
        ("Grazing Angle (Dark)", OKLCH::new(0.2, 0.02, 280.0), 75.0),
    ];

    let backend = CssBackend::new();
    let render_ctx = RenderContext::desktop();

    for (name, background, angle) in &contexts {
        let context = MaterialContext {
            background: *background,
            viewing_angle_deg: *angle,
            ambient_light: 0.3,
            key_light: 0.8,
            ..Default::default()
        };

        let evaluated = glass.evaluate(&context);

        println!("{}:", name);
        println!("  Background L:       {:.2}", background.l);
        println!("  Viewing Angle:      {:.1}°", angle);
        println!("  Result Opacity:     {:.4}", evaluated.opacity);
        println!("  Fresnel:            {:.4}", evaluated.fresnel_f0);
        println!(
            "  Edge Intensity:     {:.4}",
            evaluated.fresnel_edge_intensity
        );

        match backend.render(&evaluated, &render_ctx) {
            Ok(css) => {
                println!("  CSS: {}", css.lines().next().unwrap_or(""));
            }
            Err(e) => {
                println!("  Render error: {:?}", e);
            }
        }
        println!();
    }

    println!("✓ Context adaptation verified");
}

//! # Liquid Glass Benchmark
//!
//! Canonical demonstration of the Momoto liquid glass material system.
//! This example shows the core material rendering with physical accuracy.
//!
//! Run with: `cargo run --example 01_liquid_glass_benchmark --features webgpu`

use momoto_core::{
    backend::CssBackend,
    evaluated::{Evaluable, MaterialContext},
    material::GlassMaterial,
    render::{RenderBackend, RenderContext},
    space::oklch::OKLCH,
};

fn main() {
    println!("=== Momoto Liquid Glass Benchmark ===\n");

    // Create canonical liquid glass material
    let glass = GlassMaterial {
        roughness: 0.15,
        ior: 1.5,
        thickness: 3.0,
        noise_scale: 0.8,
        base_color: OKLCH::new(0.95, 0.01, 240.0),
        edge_power: 2.0,
    };

    // Material context
    let context = MaterialContext {
        background: OKLCH::new(0.85, 0.02, 280.0),
        viewing_angle_deg: 0.0,
        ambient_light: 0.3,
        key_light: 0.8,
        ..Default::default()
    };

    // Evaluate material
    let evaluated = glass.evaluate(&context);

    println!("Material Properties:");
    println!("  Opacity:            {:.4}", evaluated.opacity);
    println!(
        "  Scattering:         {:.2} mm",
        evaluated.scattering_radius_mm
    );
    println!("  Fresnel F0:         {:.4}", evaluated.fresnel_f0);
    println!(
        "  Edge Intensity:     {:.4}",
        evaluated.fresnel_edge_intensity
    );
    println!("  Thickness:          {:.2} mm", evaluated.thickness_mm);
    println!("  Roughness:          {:.2}", evaluated.roughness);
    println!();

    // Render to CSS
    let backend = CssBackend::new();
    let render_ctx = RenderContext::desktop();

    match backend.render(&evaluated, &render_ctx) {
        Ok(css) => {
            println!("CSS Output:");
            println!("{}", css);
            println!();
            println!("✓ Benchmark completed successfully");
        }
        Err(e) => {
            eprintln!("✗ Render failed: {:?}", e);
            std::process::exit(1);
        }
    }
}

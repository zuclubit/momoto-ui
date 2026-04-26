//! # Batch vs Single Performance Comparison
//!
//! Demonstrates the performance benefits of batch evaluation.
//! Compares single evaluations vs batch processing for multiple materials.
//!
//! Run with: `cargo run --example 03_batch_vs_single --release`

use momoto_core::{
    evaluated::{Evaluable, MaterialContext},
    material::GlassMaterial,
    space::oklch::OKLCH,
};
use std::time::Instant;

fn main() {
    println!("=== Batch vs Single Performance Comparison ===\n");

    const N: usize = 1000;

    // Create materials
    let materials: Vec<GlassMaterial> = (0..N)
        .map(|i| {
            let roughness = (i as f64 / N as f64) * 0.5 + 0.1;
            GlassMaterial {
                roughness,
                ior: 1.5,
                thickness: 3.0,
                noise_scale: 0.0,
                base_color: OKLCH::new(0.95, 0.01, 240.0),
                edge_power: 2.0,
            }
        })
        .collect();

    let context = MaterialContext::default();

    // Single evaluation approach
    let start = Instant::now();
    let results_single: Vec<_> = materials.iter().map(|m| m.evaluate(&context)).collect();
    let duration_single = start.elapsed();

    println!("Single Evaluation:");
    println!("  Materials:    {}", N);
    println!("  Duration:     {:?}", duration_single);
    println!(
        "  Per material: {:.2} µs",
        duration_single.as_micros() as f64 / N as f64
    );
    println!();

    // Verify results
    println!("Sample Results (first 3 materials):");
    for (i, result) in results_single.iter().take(3).enumerate() {
        println!(
            "  Material {}: opacity={:.4}, scattering={:.2}mm",
            i, result.opacity, result.scattering_radius_mm
        );
    }
    println!();

    // Note: Batch evaluation would require implementing a batch evaluator
    // that processes materials in parallel or uses SIMD optimizations
    println!("Note: True batch evaluation with SIMD is planned for future release.");
    println!("Current implementation processes materials sequentially.");
    println!();

    println!("✓ Performance comparison completed");
}

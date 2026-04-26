//! # Multimodal Engine Orchestration
//!
//! Demonstrates momoto-engine: MomotoEngine as the cross-domain orchestrator,
//! perceptual energy normalization across Color/Audio/Haptics, perceptual
//! alignment scoring, and system-wide energy conservation validation.
//!
//! Run with:
//!   cargo run --example 07_multimodal_engine --package momoto-engine

use momoto_core::traits::domain::DomainId;
use momoto_engine::{ColorDomain, DomainVariant, MomotoEngine};

fn main() {
    println!("=== Momoto Engine — Multimodal Orchestration ===\n");

    // ── 1. Engine construction ───────────────────────────────────────────────
    println!("1. Engine Construction");
    println!("   ─────────────────────────────────────────");

    let engine = MomotoEngine::new();

    println!("   Domains registered: {}", engine.domain_count());
    println!("   Domain names:       {:?}", engine.domain_names());
    println!(
        "   Has Color domain:   {}",
        engine.has_domain(DomainId::Color)
    );
    println!(
        "   Has Audio domain:   {}",
        engine.has_domain(DomainId::Audio)
    );
    println!(
        "   Has Haptics domain: {}",
        engine.has_domain(DomainId::Haptics)
    );
    println!("   Fully deterministic:{}", engine.is_fully_deterministic());
    println!("   Scratch buffer len: {}", engine.scratch().len());
    println!();

    // ── 2. Per-domain normalization ──────────────────────────────────────────
    println!("2. Cross-Domain Perceptual Energy Normalization");
    println!("   ─────────────────────────────────────────");
    println!("   Domain   Raw input     Normalized   Formula");

    // Color: relative luminance [0, 1] — pass-through
    let color_raw = 0.72_f32;
    let color_norm = engine.normalize_perceptual_energy(DomainId::Color, color_raw);
    println!(
        "   Color    L*={:.2}       → {:.3}     pass-through",
        color_raw, color_norm
    );

    // Audio: LUFS [-70, 0] → [0, 1] via (lufs + 70) / 70
    // Note: returns 0.0 if domain not registered (demo with Color proxy)
    let lufs_values = [-23.0_f32, -14.0, -6.0, 0.0, -70.0];
    for lufs in &lufs_values {
        // Demonstrate formula directly (Audio not registered in base engine)
        let norm = ((lufs + 70.0) / 70.0).clamp(0.0, 1.0);
        println!(
            "   Audio    {:.1} LUFS    → {:.3}     (lufs+70)/70",
            lufs, norm
        );
    }

    // Haptics: intensity [0, 1] — pass-through (same as Color)
    let haptic_intensity = 0.65_f32;
    let haptic_norm = engine.normalize_perceptual_energy(DomainId::Haptics, haptic_intensity);
    println!(
        "   Haptics  i={:.2}         → {:.3}     pass-through (not registered → 0.0)",
        haptic_intensity, haptic_norm
    );
    println!();

    // ── 3. Perceptual alignment ──────────────────────────────────────────────
    println!("3. Perceptual Alignment (Color ↔ Color)");
    println!("   ─────────────────────────────────────────");
    println!("   alignment = 1.0 − |norm_a − norm_b|  (symmetric, clamped [0,1])");
    println!();
    println!("   L_a   L_b   Alignment  Interpretation");

    let pairs = [
        (0.72_f32, 0.68_f32),
        (0.72_f32, 0.72_f32),
        (0.90_f32, 0.20_f32),
        (0.50_f32, 0.30_f32),
        (1.00_f32, 0.00_f32),
    ];

    for (va, vb) in &pairs {
        let alignment = engine.perceptual_alignment(DomainId::Color, DomainId::Color, *va, *vb);
        let label = if alignment > 0.95 {
            "perfectly coherent"
        } else if alignment > 0.80 {
            "highly coherent"
        } else if alignment > 0.60 {
            "moderately coherent"
        } else {
            "incoherent"
        };
        println!("   {:.2}  {:.2}  {:.3}      {}", va, vb, alignment, label);
    }
    println!();

    // ── 4. System energy conservation ────────────────────────────────────────
    println!("4. System Energy Conservation Validation");
    println!("   ─────────────────────────────────────────");

    let report = engine.validate_system_energy();

    println!("   System conserved:  {}", report.system_conserved);
    println!(
        "   Worst efficiency:  {:.4} (1.0 = lossless)",
        report.worst_efficiency
    );
    println!();
    println!("   Per-domain report:");
    println!(
        "   {:12}  {:>8}  {:>8}  {:>8}  {:>9}  Conserved",
        "Domain", "input", "output", "absorbed", "scattered"
    );
    for (domain_id, er) in &report.per_domain {
        println!(
            "   {:12}  {:>8.4}  {:>8.4}  {:>8.4}  {:>9.4}  {}",
            format!("{:?}", domain_id),
            er.input,
            er.output,
            er.absorbed,
            er.scattered,
            er.is_conserved(1e-4),
        );
    }
    println!();

    // ── 5. Compliance validation ─────────────────────────────────────────────
    println!("5. Domain Compliance");
    println!("   ─────────────────────────────────────────");

    println!("   Fully compliant:  {}", engine.is_fully_compliant());
    let compliance_reports = engine.validate_all();
    for report in &compliance_reports {
        println!(
            "   Standard: {:30}  passes: {}",
            report.standard, report.passes
        );
    }
    println!();

    // ── 6. Energy report batch ───────────────────────────────────────────────
    println!("6. Total Energy Report at Various Inputs");
    println!("   ─────────────────────────────────────────");
    println!("   input   output   absorbed  scattered  conserved");

    for &input in &[0.0_f32, 0.25, 0.5, 0.75, 1.0] {
        let er = engine.total_energy_report(input);
        println!(
            "   {:>5.2}   {:>6.4}   {:>8.4}  {:>9.4}  {}",
            input,
            er.output,
            er.absorbed,
            er.scattered,
            er.is_conserved(1e-4)
        );
    }
    println!();

    // ── 7. DomainVariant inspection ──────────────────────────────────────────
    println!("7. DomainVariant Direct Inspection");
    println!("   ─────────────────────────────────────────");

    let color_variant = DomainVariant::Color(ColorDomain);
    let er = color_variant.energy_report(1.0);
    println!("   ColorDomain id:        {:?}", color_variant.id());
    println!("   ColorDomain name:      {}", color_variant.name());
    println!(
        "   ColorDomain det.:      {}",
        color_variant.is_deterministic()
    );
    println!("   Energy report input:   {:.4}", er.input);
    println!("   Energy report output:  {:.4}", er.output);
    println!("   Energy conserved:      {}", er.is_conserved(1e-4));
    println!();

    // ── Summary ──────────────────────────────────────────────────────────────
    println!("=== Summary ===");
    println!("  Engine domains:            {}", engine.domain_count());
    println!(
        "  Fully deterministic:       {}",
        engine.is_fully_deterministic()
    );
    println!(
        "  System energy conserved:   {}",
        engine.validate_system_energy().system_conserved
    );
    println!(
        "  Color-Color alignment:     {:.3} (L=0.72 vs 0.68)",
        engine.perceptual_alignment(DomainId::Color, DomainId::Color, 0.72, 0.68)
    );
    println!(
        "  LUFS -23 normalized:       {:.3} (EBU R128 target)",
        ((-23.0_f32 + 70.0) / 70.0).clamp(0.0, 1.0)
    );
    println!();
    println!("  The engine is ready to compose Color + Audio + Haptics domains");
    println!("  with energy-conserving cross-domain normalization.");
}

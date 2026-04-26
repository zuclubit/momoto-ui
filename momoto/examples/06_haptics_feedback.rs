//! # Haptics Feedback Demonstration
//!
//! Demonstrates momoto-haptics: LRA energy budget, perceptual intensity →
//! vibration spec mapping (Weber's law), and 4 waveform kinds.
//!
//! Run with:
//!   cargo run --example 06_haptics_feedback --package momoto-haptics

use momoto_haptics::mapping::FrequencyForceMapper;
use momoto_haptics::{ActuatorModel, EnergyBudget, HapticWaveform, VibrationSpec, WaveformKind};

fn main() {
    println!("=== Momoto Haptics — Energy Budget & Waveforms ===\n");

    // ── 1. LRA Energy Budget ─────────────────────────────────────────────────
    println!("1. LRA Energy Budget");
    println!("   ─────────────────────────────────────────");

    // Typical smartphone LRA: 50 mJ capacity, 10 mJ/s passive recharge
    let mut budget = EnergyBudget::with_recharge(0.050, 0.010);
    println!("   Capacity:      {:.1} mJ", budget.capacity_j * 1000.0);
    println!(
        "   Recharge rate: {:.1} mJ/s",
        budget.recharge_rate_j_per_s * 1000.0
    );
    println!(
        "   Available:     {:.1} mJ  (load: {:.0}%)",
        budget.available_j() * 1000.0,
        budget.load_fraction() * 100.0
    );
    println!();

    // ── 2. 5-event haptic sequence ───────────────────────────────────────────
    println!("2. 5-Event Haptic Sequence");
    println!("   ─────────────────────────────────────────");

    let events: &[(&str, WaveformKind, f32, f32, f32)] = &[
        ("UI tap", WaveformKind::Pulse, 200.0, 30.0, 0.9),
        ("Texture sweep", WaveformKind::Sine, 150.0, 80.0, 0.5),
        ("Attack ramp", WaveformKind::Ramp, 180.0, 120.0, 0.7),
        ("Alert buzz", WaveformKind::Buzz, 220.0, 50.0, 0.8),
        ("Sustain", WaveformKind::Sine, 160.0, 200.0, 0.4),
    ];

    for (label, kind, freq_hz, dur_ms, amp) in events {
        let wave = HapticWaveform::generate(*kind, *freq_hz, *dur_ms, *amp, 8_000);
        // Simplified energy: E ≈ 0.5 * F² * t / k  (k=1000 N/m nominal LRA)
        let force_est = *amp * 0.5; // rough N estimate for demo
        let energy_j = 0.5 * force_est * force_est * (*dur_ms / 1000.0) / 1000.0;

        match budget.try_consume(energy_j) {
            Ok(()) => println!(
                "   ✓ {:16} {:?} {:.0} Hz {:.0} ms amp={:.1}  E={:.4} mJ  samples={}",
                label,
                kind,
                freq_hz,
                dur_ms,
                amp,
                energy_j * 1000.0,
                wave.samples.len()
            ),
            Err(e) => println!(
                "   ✗ {:16} {:?} — budget exceeded: req={:.4} mJ avail={:.4} mJ",
                label,
                kind,
                e.required_j * 1000.0,
                e.available_j * 1000.0
            ),
        }

        // Simulate 100 ms elapsed between events
        budget.tick(0.1);
    }

    println!();
    println!(
        "   Budget load: {:.1}%  ({:.2} mJ used, {:.2} mJ remaining)",
        budget.load_fraction() * 100.0,
        (budget.capacity_j - budget.available_j()) * 1000.0,
        budget.available_j() * 1000.0,
    );
    println!();

    // ── 3. Frequency-Force Mapping (Weber's Law) ─────────────────────────────
    println!("3. Frequency-Force Mapping — ActuatorModel::Lra");
    println!("   ─────────────────────────────────────────");

    let mapper = FrequencyForceMapper::new(ActuatorModel::Lra);
    let intensities = [0.1_f32, 0.25, 0.5, 0.75, 1.0];

    println!("   Intensity  FreqHz  Force(N)  Energy(mJ)");
    for &intensity in &intensities {
        let spec: VibrationSpec = mapper.map(intensity, 100.0);
        println!(
            "   {:>9.2}  {:>6.1}  {:>8.4}  {:>10.4}",
            intensity,
            spec.freq_hz,
            spec.force_n,
            spec.energy_j() * 1000.0
        );
    }
    println!();
    println!("   Note: freq scales as √intensity (Weber's law)");
    println!();

    // ── 4. All 4 Waveform Kinds ──────────────────────────────────────────────
    println!("4. Waveform Kinds at 150 Hz, 100 ms, amplitude=0.8, 8 kHz DAC");
    println!("   ─────────────────────────────────────────");

    let freq = 150.0_f32;
    let dur = 100.0_f32;
    let amp = 0.8_f32;
    let sr = 8_000u32;

    for kind in [
        WaveformKind::Sine,
        WaveformKind::Pulse,
        WaveformKind::Ramp,
        WaveformKind::Buzz,
    ] {
        let wave = HapticWaveform::generate(kind, freq, dur, amp, sr);
        let peak = wave
            .samples
            .iter()
            .cloned()
            .fold(0.0f32, |a, x| a.max(x.abs()));
        let rms = (wave.samples.iter().map(|&x| x * x).sum::<f32>()
            / wave.samples.len().max(1) as f32)
            .sqrt();
        println!(
            "   {:6?}  samples={:>4}  peak={:.3}  rms={:.3}",
            kind,
            wave.samples.len(),
            peak,
            rms
        );
    }
    println!();

    // ── 5. Three Actuator Models comparison ──────────────────────────────────
    println!("5. Actuator Model Comparison at intensity=0.7, 100 ms");
    println!("   ─────────────────────────────────────────");
    println!("   Model    FreqHz  Force(N)  Energy(mJ)");
    for model in [ActuatorModel::Lra, ActuatorModel::Erm, ActuatorModel::Piezo] {
        let m = FrequencyForceMapper::new(model);
        let spec = m.map(0.7, 100.0);
        println!(
            "   {:8?}  {:>6.1}  {:>8.4}  {:>10.4}",
            model,
            spec.freq_hz,
            spec.force_n,
            spec.energy_j() * 1000.0
        );
    }
    println!();

    // ── Summary ──────────────────────────────────────────────────────────────
    println!("=== Summary ===");
    println!("  EnergyBudget:  50 mJ LRA with recharge  ✓");
    println!("  Events:        5 haptic events sequenced ✓");
    println!("  Waveforms:     Sine/Pulse/Ramp/Buzz all generated ✓");
    println!("  Actuators:     LRA/ERM/Piezo compared ✓");
}

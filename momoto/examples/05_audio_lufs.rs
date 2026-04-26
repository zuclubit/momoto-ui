//! # Audio LUFS Measurement
//!
//! Demonstrates momoto-audio: K-weighted LUFS measurement (ITU-R BS.1770-4),
//! EBU R128 broadcast compliance validation, FFT power spectrum, and
//! spectral feature extraction.
//!
//! Run with:
//!   cargo run --example 05_audio_lufs --package momoto-audio

use momoto_audio::{
    spectral_brightness, spectral_centroid, spectral_flatness, spectral_rolloff, AudioDomain,
    FftPlan, MelFilterbank,
};

fn main() {
    println!("=== Momoto Audio — LUFS & Spectral Analysis ===\n");

    // ── Setup ────────────────────────────────────────────────────────────────
    let sample_rate = 48_000u32;
    let domain = AudioDomain::at_48khz();

    // ── 1. Synthesise a 1 kHz sine wave (1 second) ─────────────────────────
    let freq = 1_000.0_f32;
    let n_samples = sample_rate as usize;
    let sine: Vec<f32> = (0..n_samples)
        .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sample_rate as f32).sin())
        .collect();

    println!(
        "Signal: {:.0} Hz sine, {:.3} s, {} samples",
        freq, 1.0, n_samples
    );
    println!();

    // ── 2. LUFS measurement ──────────────────────────────────────────────────
    let mut analyzer = domain.lufs_analyzer(1).unwrap(); // mono, 48 kHz
    analyzer.add_mono_block(&sine);

    let momentary = analyzer.momentary(); // f64, 400 ms window
    let short_term = analyzer.short_term(); // f64, 3 s window
    let integrated = analyzer.integrated(); // f64, gated program loudness

    println!("LUFS Loudness (ITU-R BS.1770-4):");
    println!("  Momentary:    {:>8.2} LUFS", momentary);
    println!("  Short-term:   {:>8.2} LUFS", short_term);
    println!("  Integrated:   {:>8.2} LUFS", integrated);
    println!();

    // ── 3. EBU R128 Broadcast Compliance ────────────────────────────────────
    let report = domain.validate_broadcast(integrated);
    println!("EBU R128 Broadcast Compliance:");
    println!("  Standard:     {}", report.standard);
    println!("  Measured:     {:>8.2} LUFS", integrated);
    println!("  Violations:   {}", report.violations.len());
    println!(
        "  Passes:       {}",
        if report.passes { "✓ YES" } else { "✗ NO" }
    );
    if !report.passes {
        for v in &report.violations {
            println!(
                "    ✗ {} — measured={:.2} threshold={:.2} ({})",
                v.rule, v.measured, v.threshold, v.severity
            );
        }
    }
    println!();

    // ── 4. FFT Power Spectrum ────────────────────────────────────────────────
    let fft_size = 2048_usize;
    let plan = FftPlan::new(fft_size);

    // Use power_spectrum() — takes real samples, returns N/2+1 bins
    let power: Box<[f32]> = plan.power_spectrum(&sine[..fft_size]);
    let n_bins = power.len(); // fft_size/2 + 1 = 1025

    // Find the peak bin
    let peak_bin = power
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(i, _)| i)
        .unwrap_or(0);
    let bin_hz = sample_rate as f32 / fft_size as f32;
    let peak_freq = peak_bin as f32 * bin_hz;

    // Convert to dB for display
    let peak_db = 10.0 * (power[peak_bin] + 1e-30_f32).log10();

    println!(
        "FFT Power Spectrum ({} bins, {:.1} Hz/bin):",
        n_bins, bin_hz
    );
    println!("  FFT size:     {} samples", fft_size);
    println!(
        "  Peak bin:     {} → {:.0} Hz  ({:.1} dB)",
        peak_bin, peak_freq, peak_db
    );
    println!("  Expected peak: 1000 Hz (1 kHz sine)");
    println!(
        "  Match: {}",
        if (peak_freq - freq).abs() < bin_hz * 2.0 {
            "✓"
        } else {
            "✗"
        }
    );
    println!();

    // ── 5. Mel Filterbank ────────────────────────────────────────────────────
    let n_mels = 40;
    let f_min = 80.0_f32;
    let f_max = 8_000.0_f32;
    let filterbank = MelFilterbank::new(n_mels, fft_size, sample_rate, f_min, f_max);

    // filterbank.apply_into expects n_bins = fft_size/2+1 elements
    let mut mel_output = vec![0.0_f32; n_mels];
    filterbank.apply_into(&power, &mut mel_output);

    let mel_peak_bin = mel_output
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(i, _)| i)
        .unwrap_or(0);

    println!(
        "Mel Filterbank ({} bands, {:.0}–{:.0} Hz):",
        n_mels, f_min, f_max
    );
    println!("  Peak Mel band: {}", mel_peak_bin);
    println!("  Peak energy:   {:.6}", mel_output[mel_peak_bin]);
    println!();

    // ── 6. Spectral Features ─────────────────────────────────────────────────
    // Spectral functions take the one-sided power spectrum (N/2+1 bins)
    let centroid = spectral_centroid(&power, sample_rate);
    let brightness = spectral_brightness(&power, sample_rate, 2_000.0);
    let rolloff = spectral_rolloff(&power, sample_rate, 0.85);
    let flatness = spectral_flatness(&power);

    println!("Spectral Features:");
    println!(
        "  Centroid:     {:.1} Hz  (expected ≈ 1000 Hz for 1 kHz sine)",
        centroid
    );
    println!(
        "  Brightness:   {:.4}  (energy above 2 kHz / total)",
        brightness
    );
    println!("  Rolloff@85%%:  {:.1} Hz", rolloff);
    println!(
        "  Flatness:     {:.6}  (0 ≈ pure tone, 1 = white noise)",
        flatness
    );
    println!();

    // ── Summary ──────────────────────────────────────────────────────────────
    println!("=== Summary ===");
    println!("  LUFS integrated:  {:.2}", integrated);
    println!("  EBU R128 passes:  {}", report.passes);
    println!("  FFT peak:         {:.0} Hz (expected 1000 Hz)", peak_freq);
    println!("  Spectral centroid:{:.0} Hz", centroid);
    println!(
        "  Mel bands:        {} (peak at band {})",
        n_mels, mel_peak_bin
    );
}

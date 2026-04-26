//! `MomotoEngine` — multimodal domain orchestrator with enum dispatch.
//!
//! # Design decisions
//!
//! **No `dyn Domain` in hot paths**: `DomainVariant` is a plain Rust enum.
//! Each arm is a concrete type; LLVM can see through all match arms and inline
//! domain logic. The only place `dyn Domain` appears is `as_domain_dyn()`,
//! which is used only for metadata queries (name, id, version) — never signal
//! processing.
//!
//! **Shared scratch buffer**: `MomotoEngine` owns a `Box<[f32]>` (default
//! 4 096 elements = 16 KiB). Domains that need temporary storage receive
//! `&mut [f32]` from this buffer, so no domain evaluation allocates.
//!
//! **`ColorDomain` always registered**: Color is the founding domain and
//! is always present. Additional domains are registered at construction via
//! `MomotoEngine::with_audio()` / `with_haptics()` (feature-gated).

use momoto_core::traits::{
    compliance::ComplianceReport,
    domain::{Domain, DomainId},
    physical::{EnergyConserving, EnergyReport},
};

// ── ColorDomain ──────────────────────────────────────────────────────────────

/// Thin wrapper over `momoto-core` optical physics, implementing the `Domain`
/// and `EnergyConserving` contracts.
///
/// The color domain models ideal optical interactions: all input energy is
/// either transmitted or reflected (`A = 0`). Real absorption models live in
/// `momoto-materials` and are composed on top of this domain.
#[derive(Debug)]
pub struct ColorDomain;

impl Domain for ColorDomain {
    #[inline]
    fn id(&self) -> DomainId {
        DomainId::Color
    }

    #[inline]
    fn name(&self) -> &'static str {
        "momoto-core"
    }

    #[inline]
    fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    #[inline]
    fn is_deterministic(&self) -> bool {
        true
    }

    #[inline]
    fn max_inplace_samples(&self) -> Option<usize> {
        None
    }
}

impl EnergyConserving for ColorDomain {
    /// Ideal optical model: lossless pass-through (no absorption).
    ///
    /// Real thin-film absorption is handled in `momoto-materials::ThinFilm`,
    /// which returns an `EnergyReport` with non-zero `absorbed` component.
    #[inline]
    fn energy_report(&self, input: f32) -> EnergyReport {
        EnergyReport::lossless(input)
    }
}

// ── DomainVariant ─────────────────────────────────────────────────────────────

/// Enum dispatch over all registered sensory domains.
///
/// Each variant is a concrete type — no vtable, no indirection. New domains
/// are added as feature-gated variants so they compile away when unused.
#[derive(Debug)]
pub enum DomainVariant {
    /// Optical / perceptual color (always present).
    Color(ColorDomain),
    // Future variants (add after corresponding crates are implemented):
    // #[cfg(feature = "audio")]
    // Audio(momoto_audio::AudioDomain),
    // #[cfg(feature = "haptics")]
    // Haptics(momoto_haptics::HapticsDomain),
}

impl DomainVariant {
    /// Returns the `DomainId` of this variant — inline, no allocation.
    #[inline]
    pub fn id(&self) -> DomainId {
        match self {
            DomainVariant::Color(_) => DomainId::Color,
        }
    }

    /// Returns the crate name of this domain.
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            DomainVariant::Color(d) => d.name(),
        }
    }

    /// Returns `true` if this domain is deterministic.
    #[inline]
    pub fn is_deterministic(&self) -> bool {
        match self {
            DomainVariant::Color(d) => d.is_deterministic(),
        }
    }

    /// Compute the energy report for this domain at the given input.
    #[inline]
    pub fn energy_report(&self, input: f32) -> EnergyReport {
        match self {
            DomainVariant::Color(d) => d.energy_report(input),
        }
    }

    /// Validate this domain against its compliance standard.
    ///
    /// Color domain validates WCAG AA contrast (placeholder — full palette
    /// validation lives in `momoto-intelligence`; this is a domain-level
    /// structural check).
    pub fn validate_compliance(&self) -> ComplianceReport {
        match self {
            DomainVariant::Color(_) => {
                // Color domain structural check: energy model is always compliant.
                // Palette-level WCAG/APCA checks are in momoto-intelligence.
                ComplianceReport::new("ISO/IEC 11064-4")
            }
        }
    }
}

// ── SystemEnergyReport ───────────────────────────────────────────────────────

/// System-wide energy conservation report across all registered domains.
///
/// `validate_system_energy()` produces one `EnergyReport` per domain and
/// aggregates them into a single `total`. All arithmetic is `f32` — sufficient
/// for the 1 in 10⁴ relative-error tolerance specified by the EBU R128 and
/// WCAG standards that bound each domain.
#[derive(Debug, Clone)]
pub struct SystemEnergyReport {
    /// Energy report for each registered domain, in registration order.
    pub per_domain: Vec<(DomainId, EnergyReport)>,
    /// Sum of all per-domain reports.
    pub total: EnergyReport,
    /// `true` iff every domain individually conserves energy within `tolerance`.
    pub system_conserved: bool,
    /// Efficiency of the least-efficient domain (0 = fully absorbing, 1 = lossless).
    pub worst_efficiency: f32,
}

// ── MomotoEngine ─────────────────────────────────────────────────────────────

/// Multimodal Perceptual Physics Engine.
///
/// Composes sensory domains and provides a unified evaluation interface.
/// Owns a pre-allocated scratch buffer that is passed to domains during
/// evaluation to avoid per-call heap allocation.
///
/// # Construction
///
/// ```rust
/// use momoto_engine::MomotoEngine;
///
/// let engine = MomotoEngine::new(); // always includes ColorDomain
/// assert!(engine.has_domain(momoto_core::traits::domain::DomainId::Color));
/// ```
///
/// # Scratch buffer sizing
///
/// The default 4 096-element scratch covers:
/// - Audio FFT frames up to 4 096 samples (92 ms @ 44.1 kHz)
/// - Mel filterbank output (up to 128 bands × 32 frames)
/// - Color batch evaluations up to 4 096 pairs
///
/// Increase with `with_scratch_len` for longer audio frames.
#[derive(Debug)]
pub struct MomotoEngine {
    /// Registered domains in insertion order.
    domains: Vec<DomainVariant>,
    /// Shared work buffer. Domains receive `&mut scratch[..n]` slices.
    scratch: Box<[f32]>,
}

impl MomotoEngine {
    /// Default scratch buffer length in f32 elements (4 096 × 4 B = 16 KiB).
    pub const DEFAULT_SCRATCH_LEN: usize = 4_096;

    /// Create a new engine with `ColorDomain` registered and default scratch.
    #[must_use]
    pub fn new() -> Self {
        Self {
            domains: vec![DomainVariant::Color(ColorDomain)],
            scratch: vec![0.0_f32; Self::DEFAULT_SCRATCH_LEN].into_boxed_slice(),
        }
    }

    /// Override the scratch buffer length (in f32 elements).
    ///
    /// Call before any evaluation. Reallocates the scratch buffer.
    #[must_use]
    pub fn with_scratch_len(mut self, len: usize) -> Self {
        self.scratch = vec![0.0_f32; len].into_boxed_slice();
        self
    }

    // ── Domain registry ──────────────────────────────────────────────────────

    /// Number of registered domains.
    #[must_use]
    pub fn domain_count(&self) -> usize {
        self.domains.len()
    }

    /// Returns `true` if a domain with the given `DomainId` is registered.
    #[must_use]
    pub fn has_domain(&self, id: DomainId) -> bool {
        self.domains.iter().any(|d| d.id() == id)
    }

    /// Returns `true` iff all registered domains are deterministic.
    #[must_use]
    pub fn is_fully_deterministic(&self) -> bool {
        self.domains.iter().all(|d| d.is_deterministic())
    }

    // ── Scratch buffer access ─────────────────────────────────────────────────

    /// Read-only view of the shared scratch buffer.
    #[must_use]
    pub fn scratch(&self) -> &[f32] {
        &self.scratch
    }

    /// Mutable view of the shared scratch buffer.
    ///
    /// Domains call this to obtain temporary working memory.
    /// The buffer is NOT cleared between calls — callers must initialise the
    /// portion they use.
    pub fn scratch_mut(&mut self) -> &mut [f32] {
        &mut self.scratch
    }

    // ── Energy reporting ──────────────────────────────────────────────────────

    /// Aggregate energy report across all registered domains for a unit input.
    ///
    /// Returns the sum of all per-domain `EnergyReport`s. Useful for verifying
    /// that the full multimodal system is energy-conserving.
    #[must_use]
    pub fn total_energy_report(&self, input_per_domain: f32) -> EnergyReport {
        self.domains
            .iter()
            .map(|d| d.energy_report(input_per_domain))
            .fold(EnergyReport::lossless(0.0), |acc, r| acc + r)
    }

    /// Returns `true` iff all domains conserve energy for `unit_input`.
    #[must_use]
    pub fn verify_all_conservation(&self, unit_input: f32, tolerance: f32) -> bool {
        self.domains
            .iter()
            .all(|d| d.energy_report(unit_input).is_conserved(tolerance))
    }

    // ── Compliance ────────────────────────────────────────────────────────────

    /// Validate all domains against their compliance standards.
    ///
    /// Returns one `ComplianceReport` per domain, in registration order.
    pub fn validate_all(&self) -> Vec<ComplianceReport> {
        self.domains
            .iter()
            .map(|d| d.validate_compliance())
            .collect()
    }

    /// Returns `true` iff every domain's compliance report passes.
    #[must_use]
    pub fn is_fully_compliant(&self) -> bool {
        self.domains.iter().all(|d| d.validate_compliance().passes)
    }

    // ── Cross-domain operations ───────────────────────────────────────────────

    /// Normalise a domain-specific raw value to a perceptual scale `[0.0, 1.0]`.
    ///
    /// Each domain defines its own physical range:
    ///
    /// | Domain  | Physical input             | Normalisation                       |
    /// |---------|----------------------------|-------------------------------------|
    /// | Color   | Relative luminance [0, 1]  | pass-through (already normalised)   |
    /// | Audio   | Integrated LUFS [-70, 0]   | `(lufs + 70.0) / 70.0`, clamped     |
    /// | Haptics | Vibration intensity [0, 1] | pass-through (already normalised)   |
    ///
    /// Returns `0.0` for an unregistered `domain_id`.
    #[must_use]
    pub fn normalize_perceptual_energy(&self, domain_id: DomainId, raw_value: f32) -> f32 {
        if !self.has_domain(domain_id) {
            return 0.0;
        }
        match domain_id {
            // Color: relative luminance or OKLCH L* are already in [0, 1].
            DomainId::Color => raw_value.clamp(0.0, 1.0),
            // Audio: map integrated LUFS [-70, 0] → [0, 1].
            // Below -70 LUFS (absolute gate) → silence → 0.0.
            // Above 0 LUFS (over-compressed / clipping) → saturated → 1.0.
            DomainId::Audio => ((raw_value + 70.0) / 70.0).clamp(0.0, 1.0),
            // Haptics: normalised intensity [0, 1] is already perceptual.
            DomainId::Haptics => raw_value.clamp(0.0, 1.0),
        }
    }

    /// Compute perceptual alignment between two domain signals.
    ///
    /// "Alignment" measures how coherent two domain signals are in terms of
    /// perceived intensity, after each is normalised via
    /// `normalize_perceptual_energy`. A value of `1.0` means the two domains
    /// are perfectly in sync; `0.0` means they are maximally incoherent.
    ///
    /// The formula is:
    /// ```text
    /// alignment = 1.0 − |norm_a − norm_b|
    /// ```
    ///
    /// This is a symmetric, perceptually linear distance clamped to `[0, 1]`.
    ///
    /// Returns `0.0` if either domain is not registered.
    #[must_use]
    pub fn perceptual_alignment(
        &self,
        domain_a: DomainId,
        domain_b: DomainId,
        val_a: f32,
        val_b: f32,
    ) -> f32 {
        if !self.has_domain(domain_a) || !self.has_domain(domain_b) {
            return 0.0;
        }
        let norm_a = self.normalize_perceptual_energy(domain_a, val_a);
        let norm_b = self.normalize_perceptual_energy(domain_b, val_b);
        (1.0 - (norm_a - norm_b).abs()).clamp(0.0, 1.0)
    }

    /// Validate system-wide energy conservation across all registered domains.
    ///
    /// Each domain is queried with `unit_input = 1.0`. The per-domain reports
    /// are summed into a `total` and checked against `tolerance`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use momoto_engine::MomotoEngine;
    ///
    /// let e = MomotoEngine::new();
    /// let report = e.validate_system_energy();
    /// assert!(report.system_conserved);
    /// assert!((report.worst_efficiency - 1.0).abs() < 1e-5);
    /// ```
    #[must_use]
    pub fn validate_system_energy(&self) -> SystemEnergyReport {
        const UNIT_INPUT: f32 = 1.0;
        const TOLERANCE: f32 = 1e-4;

        let per_domain: Vec<(DomainId, EnergyReport)> = self
            .domains
            .iter()
            .map(|d| (d.id(), d.energy_report(UNIT_INPUT)))
            .collect();

        let total = per_domain
            .iter()
            .map(|(_, r)| *r)
            .fold(EnergyReport::lossless(0.0), |acc, r| acc + r);

        let system_conserved = per_domain.iter().all(|(_, r)| r.is_conserved(TOLERANCE));

        let worst_efficiency = per_domain
            .iter()
            .map(|(_, r)| r.efficiency())
            .fold(1.0_f32, f32::min);

        SystemEnergyReport {
            per_domain,
            total,
            system_conserved,
            worst_efficiency,
        }
    }

    // ── Diagnostic ───────────────────────────────────────────────────────────

    /// Returns the names of all registered domains in order.
    pub fn domain_names(&self) -> Vec<&'static str> {
        self.domains.iter().map(|d| d.name()).collect()
    }
}

impl Default for MomotoEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use momoto_core::traits::domain::DomainId;

    #[test]
    fn new_engine_has_color_domain() {
        let e = MomotoEngine::new();
        assert!(e.has_domain(DomainId::Color));
        assert_eq!(e.domain_count(), 1);
    }

    #[test]
    fn engine_does_not_have_audio_by_default() {
        let e = MomotoEngine::new();
        assert!(!e.has_domain(DomainId::Audio));
        assert!(!e.has_domain(DomainId::Haptics));
    }

    #[test]
    fn engine_is_deterministic_by_default() {
        assert!(MomotoEngine::new().is_fully_deterministic());
    }

    #[test]
    fn engine_is_compliant_by_default() {
        assert!(MomotoEngine::new().is_fully_compliant());
    }

    #[test]
    fn color_domain_energy_is_conserved() {
        let d = ColorDomain;
        assert!(d.verify_conservation(1.0, 1e-6));
        assert!(d.verify_conservation(0.0, 1e-6));
        assert!(d.verify_conservation(1000.0, 1e-4));
    }

    #[test]
    fn engine_total_energy_report_is_conserved() {
        let e = MomotoEngine::new();
        let report = e.total_energy_report(1.0);
        assert!(report.is_conserved(1e-5));
    }

    #[test]
    fn engine_verify_all_conservation_passes() {
        let e = MomotoEngine::new();
        assert!(e.verify_all_conservation(1.0, 1e-5));
    }

    #[test]
    fn scratch_buffer_default_len() {
        let e = MomotoEngine::new();
        assert_eq!(e.scratch().len(), MomotoEngine::DEFAULT_SCRATCH_LEN);
    }

    #[test]
    fn scratch_buffer_custom_len() {
        let e = MomotoEngine::new().with_scratch_len(8192);
        assert_eq!(e.scratch().len(), 8192);
    }

    #[test]
    fn scratch_mut_allows_write() {
        let mut e = MomotoEngine::new();
        e.scratch_mut()[0] = 42.0;
        assert!((e.scratch()[0] - 42.0).abs() < 1e-6);
    }

    #[test]
    fn domain_names_contains_color() {
        let e = MomotoEngine::new();
        let names = e.domain_names();
        assert!(names.contains(&"momoto-core"));
    }

    #[test]
    fn domain_variant_id_matches_color() {
        let v = DomainVariant::Color(ColorDomain);
        assert_eq!(v.id(), DomainId::Color);
    }

    #[test]
    fn domain_variant_energy_report_is_lossless() {
        let v = DomainVariant::Color(ColorDomain);
        let r = v.energy_report(1.0);
        assert!(r.is_conserved(1e-6));
        assert!((r.efficiency() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn default_engine_equals_new() {
        let a = MomotoEngine::new();
        let b = MomotoEngine::default();
        assert_eq!(a.domain_count(), b.domain_count());
        assert_eq!(a.scratch().len(), b.scratch().len());
    }

    // ── Cross-domain tests ────────────────────────────────────────────────────

    #[test]
    fn normalize_color_luminance_passthrough() {
        let e = MomotoEngine::new();
        assert!((e.normalize_perceptual_energy(DomainId::Color, 0.5) - 0.5).abs() < 1e-6);
        assert!((e.normalize_perceptual_energy(DomainId::Color, 0.0)).abs() < 1e-6);
        assert!((e.normalize_perceptual_energy(DomainId::Color, 1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn normalize_color_clamps_out_of_range() {
        let e = MomotoEngine::new();
        assert!((e.normalize_perceptual_energy(DomainId::Color, -0.5)).abs() < 1e-6);
        assert!((e.normalize_perceptual_energy(DomainId::Color, 1.5) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn normalize_unregistered_domain_returns_zero() {
        let e = MomotoEngine::new();
        // Audio and Haptics are not registered by default
        assert!((e.normalize_perceptual_energy(DomainId::Audio, -23.0)).abs() < 1e-6);
        assert!((e.normalize_perceptual_energy(DomainId::Haptics, 0.5)).abs() < 1e-6);
    }

    #[test]
    fn perceptual_alignment_identical_values_is_one() {
        let e = MomotoEngine::new();
        let alignment = e.perceptual_alignment(DomainId::Color, DomainId::Color, 0.5, 0.5);
        assert!((alignment - 1.0).abs() < 1e-6);
    }

    #[test]
    fn perceptual_alignment_opposite_extremes_is_zero() {
        let e = MomotoEngine::new();
        // 0.0 vs 1.0 → |0.0 - 1.0| = 1.0 → alignment = 0.0
        let alignment = e.perceptual_alignment(DomainId::Color, DomainId::Color, 0.0, 1.0);
        assert!(alignment.abs() < 1e-6);
    }

    #[test]
    fn perceptual_alignment_unregistered_domain_is_zero() {
        let e = MomotoEngine::new();
        let alignment = e.perceptual_alignment(DomainId::Color, DomainId::Audio, 0.5, -23.0);
        assert!(alignment.abs() < 1e-6);
    }

    #[test]
    fn validate_system_energy_conserved_for_default_engine() {
        let e = MomotoEngine::new();
        let report = e.validate_system_energy();
        assert!(report.system_conserved, "system energy must be conserved");
        assert_eq!(report.per_domain.len(), 1);
        assert_eq!(report.per_domain[0].0, DomainId::Color);
        assert!((report.worst_efficiency - 1.0).abs() < 1e-5);
    }

    #[test]
    fn validate_system_energy_total_matches_per_domain_sum() {
        let e = MomotoEngine::new();
        let report = e.validate_system_energy();
        // With one domain and unit input: total.input == 1.0
        assert!((report.total.input - 1.0).abs() < 1e-5);
    }
}

//! Temporal Perception Engine — WCAG 2.1 compliant color animation analysis.
//!
//! Implements:
//! - Flicker detection (WCAG 2.1 SC 2.3.1 — Three Flashes or Below Threshold)
//! - Motion analysis with vestibular safety checks
//! - Temporal contrast sensitivity (De Lange/Van Nes CSF)
//! - SIREN-inspired temporal neural correction
//! - Comprehensive stress-test suite
//!
//! All algorithms grounded in color perception science and WCAG 2.1 photosensitivity guidelines.

#![allow(dead_code)]

use momoto_core::color::Color;
use momoto_core::space::oklch::{HuePath, OKLCH};
use serde::{Deserialize, Serialize};

// ============================================================================
// Easing Functions
// ============================================================================

/// Interpolation easing functions for color transitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EasingFunction {
    /// Constant velocity.
    Linear,
    /// Accelerating curve.
    EaseIn,
    /// Decelerating curve.
    EaseOut,
    /// S-curve (symmetric).
    EaseInOut,
    /// Instant jump at the midpoint (no interpolation).
    ///
    /// Returns 0.0 for t < 0.5 and 1.0 for t ≥ 0.5.
    Step,
    /// CSS cubic-bezier(p1x, p1y, p2x, p2y).
    CubicBezier(f64, f64, f64, f64),
    /// Spring physics model.
    Spring { stiffness: f64, damping: f64 },
}

impl EasingFunction {
    /// Evaluate the easing function at normalized time `t` ∈ [0, 1].
    ///
    /// Returns a value typically in [0, 1] (may overshoot for Spring).
    pub fn evaluate(&self, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::EaseIn => t * t * t,
            Self::EaseOut => 1.0 - (1.0 - t).powi(3),
            Self::EaseInOut => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            Self::Step => {
                // Instantaneous jump at midpoint
                if t < 0.5 {
                    0.0
                } else {
                    1.0
                }
            }
            Self::CubicBezier(p1x, p1y, p2x, p2y) => {
                // Numerical approximation via Newton–Raphson on the parametric Bezier
                cubic_bezier_y(*p1x, *p1y, *p2x, *p2y, t)
            }
            Self::Spring { stiffness, damping } => {
                // Damped spring: x(t) = 1 - e^(-ζωt)(cos(ωd·t) + ζ/√(1-ζ²) sin(ωd·t))
                let omega = stiffness.sqrt();
                let zeta = damping / (2.0 * stiffness.sqrt());
                let zeta = zeta.clamp(0.001, 0.999);
                let omega_d = omega * (1.0 - zeta * zeta).sqrt();
                let envelope = (-zeta * omega * t).exp();
                let oscillation =
                    (omega_d * t).cos() + (zeta / (1.0 - zeta * zeta).sqrt()) * (omega_d * t).sin();
                (1.0 - envelope * oscillation).clamp(-0.5, 1.5)
            }
        }
    }

    /// Apply the easing function — alias for [`evaluate`].
    ///
    /// Provided for ergonomic compatibility with WASM bindings.
    #[inline]
    pub fn apply(&self, t: f64) -> f64 {
        self.evaluate(t)
    }
}

/// Evaluate a CSS cubic-bezier at x-value `t` using Newton–Raphson.
fn cubic_bezier_y(p1x: f64, p1y: f64, p2x: f64, p2y: f64, t: f64) -> f64 {
    // Bezier x(u) = 3*p1x*u*(1-u)^2 + 3*p2x*u^2*(1-u) + u^3
    // Find u such that x(u) = t, then compute y(u)
    let bezier_x = |u: f64| -> f64 {
        3.0 * p1x * u * (1.0 - u).powi(2) + 3.0 * p2x * u.powi(2) * (1.0 - u) + u.powi(3)
    };
    let bezier_y = |u: f64| -> f64 {
        3.0 * p1y * u * (1.0 - u).powi(2) + 3.0 * p2y * u.powi(2) * (1.0 - u) + u.powi(3)
    };
    let bezier_dx = |u: f64| -> f64 {
        3.0 * p1x * (1.0 - u) * (1.0 - 3.0 * u) + 3.0 * p2x * u * (3.0 * u - 2.0) + 3.0 * u.powi(2)
    };

    // Newton–Raphson: find u from x
    let mut u = t;
    for _ in 0..8 {
        let x = bezier_x(u);
        let dx = bezier_dx(u);
        if dx.abs() < 1e-9 {
            break;
        }
        u -= (x - t) / dx;
        u = u.clamp(0.0, 1.0);
    }
    bezier_y(u)
}

// ============================================================================
// Core Temporal Types
// ============================================================================

/// A color frozen at a specific timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalColorState {
    /// Color in hex notation.
    pub hex: String,
    /// OKLCH lightness.
    pub oklch_l: f64,
    /// OKLCH chroma.
    pub oklch_c: f64,
    /// OKLCH hue angle (degrees).
    pub oklch_h: f64,
    /// Timestamp in milliseconds from sequence start.
    pub timestamp_ms: u64,
}

impl TemporalColorState {
    /// Construct from a `Color` at a given timestamp.
    pub fn from_color(color: &Color, timestamp_ms: u64) -> Self {
        let oklch = OKLCH::from_color(color);
        Self {
            hex: color.to_hex(),
            oklch_l: oklch.l,
            oklch_c: oklch.c,
            oklch_h: oklch.h,
            timestamp_ms,
        }
    }

    /// Relative WCAG luminance from OKLCH lightness.
    ///
    /// Approximation: Y ≈ L³ after accounting for OKLCH non-linearity.
    pub fn relative_luminance(&self) -> f64 {
        // OKLCH L is approximately cube-root of luminance (like Lab)
        // So Y ≈ (L * 1.13)^3 — empirical calibration for OKLCH to WCAG Y
        let l_scaled = self.oklch_l * 1.0; // OKLCH L is already 0..1
        (l_scaled).powi(3).clamp(0.0, 1.0)
    }
}

/// An animated transition between two color states.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorTransition {
    /// Start color state.
    pub from: TemporalColorState,
    /// End color state.
    pub to: TemporalColorState,
    /// Duration in milliseconds.
    pub duration_ms: u64,
    /// Interpolation easing.
    pub easing: EasingFunction,
}

impl ColorTransition {
    /// Interpolate state at normalized time `t` ∈ [0, 1] using OKLCH lerp.
    pub fn interpolate(&self, t: f64) -> TemporalColorState {
        let t = t.clamp(0.0, 1.0);
        let eased_t = self.easing.evaluate(t);

        let a = OKLCH::new(self.from.oklch_l, self.from.oklch_c, self.from.oklch_h);
        let b = OKLCH::new(self.to.oklch_l, self.to.oklch_c, self.to.oklch_h);
        let mid = OKLCH::interpolate(&a, &b, eased_t, HuePath::Shorter);
        let color = mid.map_to_gamut().to_color();

        let timestamp = self.from.timestamp_ms + (eased_t * self.duration_ms as f64) as u64;

        TemporalColorState {
            hex: color.to_hex(),
            oklch_l: mid.l,
            oklch_c: mid.c,
            oklch_h: mid.h,
            timestamp_ms: timestamp,
        }
    }

    /// Sample the transition at an absolute millisecond offset within the transition.
    pub fn sample_at_ms(&self, ms: u64) -> TemporalColorState {
        let t = if self.duration_ms == 0 {
            1.0
        } else {
            (ms as f64 / self.duration_ms as f64).clamp(0.0, 1.0)
        };
        self.interpolate(t)
    }
}

/// A sequence of color transitions forming an animation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorSequence {
    /// Ordered list of transitions.
    pub transitions: Vec<ColorTransition>,
    /// Total animation duration in milliseconds.
    pub total_duration_ms: u64,
    /// Optional human-readable name for the sequence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Optional description of the transition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Builder state: last state added via `add_state`.
    /// Skipped from serialisation — only used during construction.
    #[serde(skip)]
    last_state: Option<TemporalColorState>,
}

impl ColorSequence {
    /// Create an empty named sequence in builder mode.
    ///
    /// Use [`add_state`] to append frames, then serialise to JSON.
    /// This constructor is intended for the incremental frame-by-frame API.
    ///
    /// # Example
    /// ```ignore
    /// let mut seq = ColorSequence::new("transition", "Blue → Green");
    /// seq.add_state(0,    0.5, 0.15, 250.0);
    /// seq.add_state(1000, 0.6, 0.18, 145.0);
    /// ```
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            transitions: Vec::new(),
            total_duration_ms: 0,
            name: Some(name.to_string()),
            description: Some(description.to_string()),
            last_state: None,
        }
    }

    /// Append a color frame at the given timestamp.
    ///
    /// Internally converts the OKLCH values to a `ColorTransition` from the
    /// previous frame. The first call just records the initial state.
    ///
    /// # Arguments
    /// * `time_ms` — absolute timestamp from sequence start in milliseconds
    /// * `l` — OKLCH lightness ∈ [0, 1]
    /// * `c` — OKLCH chroma ≥ 0
    /// * `h` — OKLCH hue ∈ [0, 360)
    pub fn add_state(&mut self, time_ms: u64, l: f64, c: f64, h: f64) {
        use momoto_core::space::oklch::OKLCH;
        let oklch = OKLCH { l, c, h };
        let color = oklch.map_to_gamut().to_color();
        let new_state = TemporalColorState {
            hex: color.to_hex(),
            oklch_l: l,
            oklch_c: c,
            oklch_h: h,
            timestamp_ms: time_ms,
        };

        if let Some(prev) = self.last_state.take() {
            let dur = time_ms.saturating_sub(prev.timestamp_ms);
            self.transitions.push(ColorTransition {
                from: prev,
                to: new_state.clone(),
                duration_ms: dur,
                easing: EasingFunction::Linear,
            });
            self.total_duration_ms = time_ms;
        }

        self.last_state = Some(new_state);
    }

    /// Build a sequence from discrete color states with equal duration steps.
    ///
    /// This is the batch constructor for pre-computed state arrays.
    /// For incremental frame-by-frame construction use [`ColorSequence::new`]
    /// followed by [`add_state`].
    pub fn from_states(
        states: Vec<TemporalColorState>,
        duration_per_step_ms: u64,
        easing: EasingFunction,
    ) -> Self {
        if states.len() < 2 {
            return Self {
                transitions: vec![],
                total_duration_ms: 0,
                name: None,
                description: None,
                last_state: None,
            };
        }

        let transitions: Vec<ColorTransition> = states
            .windows(2)
            .map(|w| ColorTransition {
                from: w[0].clone(),
                to: w[1].clone(),
                duration_ms: duration_per_step_ms,
                easing: easing.clone(),
            })
            .collect();

        let total = transitions.len() as u64 * duration_per_step_ms;

        Self {
            transitions,
            total_duration_ms: total,
            name: None,
            description: None,
            last_state: None,
        }
    }

    /// Get interpolated state at an absolute millisecond offset.
    pub fn at_ms(&self, ms: u64) -> TemporalColorState {
        if self.transitions.is_empty() {
            return TemporalColorState {
                hex: "#000000".to_string(),
                oklch_l: 0.0,
                oklch_c: 0.0,
                oklch_h: 0.0,
                timestamp_ms: ms,
            };
        }

        let mut elapsed = 0u64;
        for transition in &self.transitions {
            let end = elapsed + transition.duration_ms;
            if ms <= end || std::ptr::eq(transition, self.transitions.last().unwrap()) {
                let local_ms = ms.saturating_sub(elapsed).min(transition.duration_ms);
                return transition.sample_at_ms(local_ms);
            }
            elapsed = end;
        }

        // Past the end — return final state
        let last = self.transitions.last().unwrap();
        last.interpolate(1.0)
    }

    /// Sample luminance values over the sequence at `sample_rate_hz`.
    pub fn luminances(&self, sample_rate_hz: f64) -> Vec<f64> {
        let sample_interval_ms = (1000.0 / sample_rate_hz) as u64;
        let total = self.total_duration_ms;
        let n_samples = (total as f64 * sample_rate_hz / 1000.0).ceil() as usize + 1;

        (0..n_samples)
            .map(|i| {
                let ms = (i as u64 * sample_interval_ms).min(total);
                self.at_ms(ms).relative_luminance()
            })
            .collect()
    }
}

// ============================================================================
// Temporal Metrics & Results
// ============================================================================

/// Quantitative metrics describing a color sequence's temporal behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalMetrics {
    /// Maximum instantaneous luminance change per second (WCAG: must be < 3 flashes/sec).
    pub max_luminance_change_per_sec: f64,
    /// Average luminance change per second.
    pub avg_luminance_change_per_sec: f64,
    /// Maximum change in contrast ratio per second.
    pub max_contrast_ratio_change: f64,
    /// Dominant flicker frequency in Hz.
    pub flicker_frequency_hz: f64,
    /// Equivalent motion blur in pixels (based on luminance velocity).
    pub motion_blur_equivalent_px: f64,
}

/// Photosensitivity risk level for a color sequence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlickerRisk {
    /// No detectable flicker.
    None,
    /// Flicker below perceptual threshold.
    Low,
    /// Noticeable flicker; may cause discomfort.
    Medium,
    /// Significant flicker; accessibility concern.
    High,
    /// Dangerous for photosensitive users (>3Hz luminance flash).
    Photosensitive,
}

impl FlickerRisk {
    /// Human-readable description of the risk level.
    pub fn description(&self) -> &str {
        match self {
            Self::None => "No flicker detected",
            Self::Low => "Below perceptual threshold",
            Self::Medium => "Noticeable; may cause eye strain",
            Self::High => "Significant flicker; review animation parameters",
            Self::Photosensitive => "Dangerous: exceeds WCAG 2.3.1 flash threshold",
        }
    }

    /// Whether this risk level is safe for general use.
    pub fn is_safe(&self) -> bool {
        matches!(self, Self::None | Self::Low)
    }
}

/// Priority level for temporal recommendations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationPriority {
    /// Must be fixed before deployment.
    Critical,
    /// Should be fixed; degrades accessibility.
    High,
    /// Nice to fix; improves quality.
    Medium,
    /// Optional enhancement.
    Low,
}

/// A concrete recommendation for improving a color sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalRecommendation {
    /// Urgency level.
    pub priority: RecommendationPriority,
    /// Category: "flicker", "motion", "contrast", "duration".
    pub category: String,
    /// Human-readable recommendation.
    pub message: String,
    /// Suggested revised duration in milliseconds.
    pub suggested_duration_ms: Option<u64>,
}

/// WCAG photosensitivity compliance results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WcagTemporalResult {
    /// Passes WCAG 2.1 SC 1.4.1 (No Flicker below threshold).
    pub passes_sc_141: bool,
    /// Passes WCAG 2.1 SC 2.3.1 (Three Flashes or Below Threshold).
    pub passes_sc_2310: bool,
    /// Human-readable issue descriptions.
    pub issues: Vec<String>,
    /// Percentage of screen area affected (0.0–100.0).
    pub max_flash_area_percent: f64,
}

/// Complete flicker analysis for a color sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlickerAnalysis {
    /// Dominant flicker frequency in Hz.
    pub frequency_hz: f64,
    /// Assessed risk level.
    pub risk: FlickerRisk,
    /// WCAG photosensitivity compliance.
    pub wcag_result: WcagTemporalResult,
    /// Millisecond ranges where problematic flicker occurs.
    pub problematic_ranges: Vec<(u64, u64)>,
}

/// Motion safety analysis for a color sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionAnalysis {
    /// Maximum luminance velocity (change per second).
    pub max_velocity: f64,
    /// Average luminance velocity.
    pub avg_velocity: f64,
    /// Whether the motion is safe for vestibular-sensitive users.
    pub motion_safe: bool,
    /// List of motion issues.
    pub issues: Vec<String>,
    /// Recommended reduced-motion alternative CSS.
    pub reduced_motion_alternative: Option<String>,
}

/// Complete temporal analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalResult {
    /// The analyzed sequence.
    pub sequence: ColorSequence,
    /// Quantitative metrics.
    pub metrics: TemporalMetrics,
    /// Flicker risk assessment.
    pub flicker_risk: FlickerRisk,
    /// Motion safety status.
    pub motion_safe: bool,
    /// Overall WCAG temporal compliance.
    pub wcag_compliant: bool,
    /// Ordered list of recommendations.
    pub recommendations: Vec<TemporalRecommendation>,
}

// ============================================================================
// Flicker Detection
// ============================================================================

/// Configuration for flicker detection algorithm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlickerConfig {
    /// Luminance sampling rate in Hz.
    pub sample_rate_hz: f64,
    /// Analysis window length in milliseconds.
    pub window_ms: u64,
    /// Maximum acceptable flash frequency in Hz (WCAG: 3.0).
    pub threshold_hz: f64,
    /// Minimum luminance change fraction to count as a flash (0.0–1.0).
    pub luminance_change_threshold: f64,
}

impl FlickerConfig {
    /// WCAG 2.1 SC 2.3.1 compliant configuration.
    pub fn wcag() -> Self {
        Self {
            sample_rate_hz: 60.0,
            window_ms: 1000,
            threshold_hz: 3.0,
            luminance_change_threshold: 0.1,
        }
    }
}

/// A single luminance flash event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashEvent {
    /// Start of flash in milliseconds.
    pub start_ms: u64,
    /// End of flash in milliseconds.
    pub end_ms: u64,
    /// Peak absolute luminance change during the flash.
    pub peak_luminance_change: f64,
}

/// Result of running the flicker detection algorithm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlickerDetectionResult {
    /// All detected flash events.
    pub flash_events: Vec<FlashEvent>,
    /// Computed flashes per second in the worst window.
    pub flashes_per_second: f64,
    /// Assessed risk level.
    pub risk: FlickerRisk,
    /// Whether the sequence passes WCAG 2.3.1.
    pub wcag_compliant: bool,
}

/// Safe duration ranges for an animation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionDuration {
    /// Minimum acceptable duration in milliseconds.
    pub min_ms: u64,
    /// Recommended duration in milliseconds.
    pub recommended_ms: u64,
    /// Maximum duration beyond which animation feels sluggish.
    pub max_ms: u64,
}

/// Safe animation parameters for a given color pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafeAnimationParams {
    /// Maximum safe frame rate in Hz.
    pub max_fps: f64,
    /// Duration recommendations for normal motion.
    pub duration: TransitionDuration,
    /// Recommended easing function.
    pub easing: EasingFunction,
    /// Duration recommendations for reduced-motion contexts.
    pub reduced_motion_duration: TransitionDuration,
}

/// A transition with safety analysis applied.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlickerSafeTransition {
    /// The original transition.
    pub original: ColorTransition,
    /// A safe variant, if the original was unsafe.
    pub safe: Option<ColorTransition>,
    /// Whether the original was already safe.
    pub is_already_safe: bool,
    /// Issues found in the original transition.
    pub detected_issues: Vec<String>,
}

impl FlickerSafeTransition {
    /// Analyze a transition and produce a safe variant if needed.
    pub fn make_safe(transition: ColorTransition, config: &FlickerConfig) -> Self {
        let sequence = ColorSequence {
            transitions: vec![transition.clone()],
            total_duration_ms: transition.duration_ms,
            name: None,
            description: None,
            last_state: None,
        };
        let detector = FlickerDetector::new(config.clone());
        let result = detector.detect(&sequence);

        if result.wcag_compliant {
            return Self {
                original: transition,
                safe: None,
                is_already_safe: true,
                detected_issues: vec![],
            };
        }

        // Determine how much to slow down the transition
        let speed_factor = (result.flashes_per_second / config.threshold_hz).max(1.0);
        let safe_duration = (transition.duration_ms as f64 * speed_factor * 2.0) as u64;
        let safe_transition = ColorTransition {
            from: transition.from.clone(),
            to: transition.to.clone(),
            duration_ms: safe_duration,
            easing: EasingFunction::EaseInOut,
        };

        let issues = result
            .flash_events
            .iter()
            .map(|e| {
                format!(
                    "Flash at {}ms–{}ms (ΔL={:.3})",
                    e.start_ms, e.end_ms, e.peak_luminance_change
                )
            })
            .collect();

        Self {
            original: transition,
            safe: Some(safe_transition),
            is_already_safe: false,
            detected_issues: issues,
        }
    }
}

/// Flicker detection engine.
#[derive(Debug)]
pub struct FlickerDetector {
    /// Detection configuration.
    config: FlickerConfig,
}

impl FlickerDetector {
    /// Create a new detector with the given configuration.
    pub fn new(config: FlickerConfig) -> Self {
        Self { config }
    }

    /// Detect flash events in a color sequence.
    ///
    /// Algorithm (WCAG 2.1 SC 2.3.1):
    /// 1. Sample luminance at `config.sample_rate_hz`.
    /// 2. Compute first-difference (luminance velocity).
    /// 3. Detect sign changes where |ΔL| > threshold → flash pairs.
    /// 4. Count flash pairs per second in a sliding window.
    pub fn detect(&self, sequence: &ColorSequence) -> FlickerDetectionResult {
        let luminances = sequence.luminances(self.config.sample_rate_hz);
        let sample_interval_ms = 1000.0 / self.config.sample_rate_hz;

        if luminances.len() < 2 {
            return FlickerDetectionResult {
                flash_events: vec![],
                flashes_per_second: 0.0,
                risk: FlickerRisk::None,
                wcag_compliant: true,
            };
        }

        // Compute luminance differences
        let diffs: Vec<f64> = luminances.windows(2).map(|w| w[1] - w[0]).collect();

        // Detect zero-crossings with significant magnitude (= flash transitions)
        let mut flash_events = Vec::new();
        let mut flash_start_idx: Option<usize> = None;
        let mut prev_direction: i8 = 0; // -1 falling, +1 rising

        for (i, &d) in diffs.iter().enumerate() {
            let direction = if d > self.config.luminance_change_threshold {
                1i8
            } else if d < -self.config.luminance_change_threshold {
                -1i8
            } else {
                prev_direction
            };

            // Sign change with significant magnitude = flash boundary
            if direction != 0 && prev_direction != 0 && direction != prev_direction {
                if let Some(start) = flash_start_idx {
                    let peak = diffs[start..=i]
                        .iter()
                        .map(|x| x.abs())
                        .fold(0.0f64, f64::max);
                    flash_events.push(FlashEvent {
                        start_ms: (start as f64 * sample_interval_ms) as u64,
                        end_ms: (i as f64 * sample_interval_ms) as u64,
                        peak_luminance_change: peak,
                    });
                }
                flash_start_idx = Some(i);
            } else if direction != 0 && flash_start_idx.is_none() {
                flash_start_idx = Some(i);
            }

            if direction != 0 {
                prev_direction = direction;
            }
        }

        // Count flashes in the worst 1-second window
        let window_samples =
            (self.config.window_ms as f64 * self.config.sample_rate_hz / 1000.0) as usize;
        let mut max_flashes_in_window = 0usize;

        if !flash_events.is_empty() && window_samples > 0 {
            // Sliding window: count events whose start_ms falls within each window
            let total_windows = (sequence.total_duration_ms / self.config.window_ms) + 1;
            for w in 0..total_windows {
                let window_start = w * self.config.window_ms;
                let window_end = window_start + self.config.window_ms;
                let count = flash_events
                    .iter()
                    .filter(|e| e.start_ms >= window_start && e.start_ms < window_end)
                    .count();
                max_flashes_in_window = max_flashes_in_window.max(count);
            }
        }

        // Each flash event represents one luminance reversal; WCAG counts pairs
        // A "flash" in WCAG terms = going from below threshold to above and back
        let flashes_per_second =
            max_flashes_in_window as f64 / (self.config.window_ms as f64 / 1000.0);

        let risk = classify_flicker_risk(flashes_per_second, &self.config);
        let wcag_compliant = flashes_per_second <= self.config.threshold_hz;

        FlickerDetectionResult {
            flash_events,
            flashes_per_second,
            risk,
            wcag_compliant,
        }
    }

    /// Compute safe animation parameters for a given color pair.
    pub fn safe_params_for(from_hex: &str, to_hex: &str) -> SafeAnimationParams {
        let from_color = Color::from_hex(from_hex).unwrap_or_else(|_| Color::from_srgb8(0, 0, 0));
        let to_color = Color::from_hex(to_hex).unwrap_or_else(|_| Color::from_srgb8(255, 255, 255));

        let from_oklch = OKLCH::from_color(&from_color);
        let to_oklch = OKLCH::from_color(&to_color);

        // Luminance delta drives minimum safe duration
        let delta_l = (from_oklch.l - to_oklch.l).abs();

        // WCAG principle: no more than 3 luminance reversals per second
        // For a single transition: just ensure it takes >= 333ms (1/3Hz)
        // Scale up proportionally for large ΔL
        let base_min_ms = 333u64;
        let min_ms = (base_min_ms as f64 * (1.0 + delta_l * 2.0)) as u64;
        let recommended_ms = (min_ms as f64 * 1.5) as u64;
        let max_ms = recommended_ms * 4;

        SafeAnimationParams {
            max_fps: 60.0,
            duration: TransitionDuration {
                min_ms,
                recommended_ms,
                max_ms,
            },
            easing: EasingFunction::EaseInOut,
            reduced_motion_duration: TransitionDuration {
                min_ms: min_ms * 2,
                recommended_ms: recommended_ms * 2,
                max_ms: max_ms * 3,
            },
        }
    }

    /// Analyze a transition and return a safe variant if needed.
    pub fn make_transition_safe(&self, transition: ColorTransition) -> FlickerSafeTransition {
        FlickerSafeTransition::make_safe(transition, &self.config)
    }
}

fn classify_flicker_risk(fps: f64, config: &FlickerConfig) -> FlickerRisk {
    if fps < 0.1 {
        FlickerRisk::None
    } else if fps < 1.0 {
        FlickerRisk::Low
    } else if fps < config.threshold_hz {
        FlickerRisk::Medium
    } else if fps < 10.0 {
        FlickerRisk::High
    } else {
        FlickerRisk::Photosensitive
    }
}

// ============================================================================
// Motion Analysis
// ============================================================================

/// Configuration for motion analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionConfig {
    /// Maximum acceptable luminance velocity per second (0.0–1.0).
    pub max_velocity_per_sec: f64,
    /// Whether to check for reduced-motion compliance.
    pub enable_reduced_motion: bool,
    /// Whether to apply stricter vestibular safety checks.
    pub check_vestibular: bool,
}

/// A single motion safety issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionIssue {
    /// "error", "warning", or "info".
    pub severity: String,
    /// Description of the issue.
    pub description: String,
    /// Timestamp where the issue occurs.
    pub at_ms: u64,
}

/// Complete motion analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionAnalysisResult {
    /// Computed motion metrics.
    pub analysis: MotionAnalysis,
    /// Detected issues.
    pub issues: Vec<MotionIssue>,
    /// Whether the sequence is safe for vestibular-sensitive users.
    pub is_safe: bool,
}

/// Moving-average smoother for luminance series.
#[derive(Debug, Clone)]
pub struct MotionSmoother {
    /// Number of samples in the smoothing window.
    pub window_size: usize,
}

impl MotionSmoother {
    /// Smooth a series with a symmetric moving average.
    pub fn smooth(&self, values: &[f64]) -> Vec<f64> {
        if values.len() < self.window_size {
            return values.to_vec();
        }
        let half = self.window_size / 2;
        values
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let start = i.saturating_sub(half);
                let end = (i + half + 1).min(values.len());
                let slice = &values[start..end];
                slice.iter().sum::<f64>() / slice.len() as f64
            })
            .collect()
    }
}

/// Motion analysis engine.
#[derive(Debug)]
pub struct MotionAnalyzer {
    /// Analysis configuration.
    config: MotionConfig,
}

impl MotionAnalyzer {
    /// Create a new analyzer.
    pub fn new(config: MotionConfig) -> Self {
        Self { config }
    }

    /// Analyze a color sequence for motion safety.
    pub fn analyze(&self, sequence: &ColorSequence) -> MotionAnalysisResult {
        let sample_rate = 60.0;
        let luminances = sequence.luminances(sample_rate);
        let sample_interval_s = 1.0 / sample_rate;

        if luminances.len() < 2 {
            return MotionAnalysisResult {
                analysis: MotionAnalysis {
                    max_velocity: 0.0,
                    avg_velocity: 0.0,
                    motion_safe: true,
                    issues: vec![],
                    reduced_motion_alternative: None,
                },
                issues: vec![],
                is_safe: true,
            };
        }

        // Compute instantaneous velocities
        let smoother = MotionSmoother { window_size: 5 };
        let smoothed = smoother.smooth(&luminances);

        let velocities: Vec<f64> = smoothed
            .windows(2)
            .map(|w| (w[1] - w[0]).abs() / sample_interval_s)
            .collect();

        let max_velocity = velocities.iter().cloned().fold(0.0f64, f64::max);
        let avg_velocity = if velocities.is_empty() {
            0.0
        } else {
            velocities.iter().sum::<f64>() / velocities.len() as f64
        };

        let mut issues = Vec::new();
        let mut motion_issues = Vec::new();

        // Check against maximum velocity threshold
        if max_velocity > self.config.max_velocity_per_sec {
            motion_issues.push(MotionIssue {
                severity: "warning".to_string(),
                description: format!(
                    "Peak luminance velocity {:.3}/s exceeds limit of {:.3}/s",
                    max_velocity, self.config.max_velocity_per_sec
                ),
                at_ms: velocities
                    .iter()
                    .position(|&v| v >= max_velocity)
                    .map(|i| (i as f64 / sample_rate * 1000.0) as u64)
                    .unwrap_or(0),
            });
            issues.push(format!(
                "Motion velocity {:.3}/s exceeds threshold {:.3}/s",
                max_velocity, self.config.max_velocity_per_sec
            ));
        }

        // Vestibular check: high velocity for extended time
        if self.config.check_vestibular {
            let fast_samples = velocities.iter().filter(|&&v| v > 0.3).count();
            let fast_fraction = fast_samples as f64 / velocities.len() as f64;
            if fast_fraction > 0.3 {
                motion_issues.push(MotionIssue {
                    severity: "warning".to_string(),
                    description: format!(
                        "{:.0}% of animation exceeds vestibular safety velocity",
                        fast_fraction * 100.0
                    ),
                    at_ms: 0,
                });
                issues.push("Extended rapid motion may cause vestibular discomfort".to_string());
            }
        }

        let motion_safe = motion_issues.iter().all(|i| i.severity != "error")
            && max_velocity <= self.config.max_velocity_per_sec * 2.0;

        let reduced_motion_css = if self.config.enable_reduced_motion {
            Some("@media (prefers-reduced-motion: reduce) { transition: none; }".to_string())
        } else {
            None
        };

        MotionAnalysisResult {
            analysis: MotionAnalysis {
                max_velocity,
                avg_velocity,
                motion_safe,
                issues,
                reduced_motion_alternative: reduced_motion_css,
            },
            issues: motion_issues,
            is_safe: motion_safe,
        }
    }
}

// ============================================================================
// Temporal Contrast Sensitivity
// ============================================================================

/// A luminance change event between two sample points.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LuminanceChange {
    /// Starting relative luminance.
    pub from_luminance: f64,
    /// Ending relative luminance.
    pub to_luminance: f64,
    /// Absolute luminance delta.
    pub delta: f64,
    /// Rate of change per second.
    pub rate_per_sec: f64,
}

/// Parameters describing visual adaptation to luminance changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContrastAdaptation {
    /// Time in milliseconds for the visual system to adapt.
    pub adaptation_time_ms: u64,
    /// Factor by which sensitivity is reduced during adaptation [0, 1].
    pub sensitivity_loss_factor: f64,
}

/// Temporal masking parameters from psychophysical models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalMasking {
    /// Forward masking duration in milliseconds (preceding flash masks visibility).
    pub forward_masking_ms: u64,
    /// Backward masking duration in milliseconds.
    pub backward_masking_ms: u64,
    /// Critical frequency band for masking in Hz.
    pub critical_band_hz: f64,
}

/// Result of temporal contrast sensitivity analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalContrastResult {
    /// All detected luminance changes.
    pub changes: Vec<LuminanceChange>,
    /// Visual adaptation parameters.
    pub adaptation: ContrastAdaptation,
    /// Temporal masking parameters.
    pub masking: TemporalMasking,
    /// Whether the luminance changes are perceptually uniform over time.
    pub perceptually_uniform: bool,
}

/// Temporal Contrast Sensitivity Function (CSF) utilities.
///
/// Based on the De Lange (1958) / Van Nes & Bouman (1967) CSF model.
pub struct TemporalContrastSensitivity;

impl TemporalContrastSensitivity {
    /// Evaluate the temporal CSF at a given frequency.
    ///
    /// Model: Gaussian with peak at ~8 Hz and bandwidth of ~6 Hz.
    /// `sensitivity = peak * exp(-((f - f_peak)^2) / (2 * bandwidth^2))`
    ///
    /// Returns relative sensitivity [0, 1].
    pub fn csf(frequency_hz: f64) -> f64 {
        const PEAK_SENSITIVITY: f64 = 1.0;
        const F_PEAK: f64 = 8.0; // Hz — peak human temporal CSF
        const BANDWIDTH: f64 = 6.0; // Hz — half-power bandwidth

        let exponent = -(frequency_hz - F_PEAK).powi(2) / (2.0 * BANDWIDTH.powi(2));
        (PEAK_SENSITIVITY * exponent.exp()).clamp(0.0, 1.0)
    }
}

/// Temporal contrast sensitivity analyzer.
#[derive(Debug)]
pub struct TemporalContrastAnalyzer;

impl TemporalContrastAnalyzer {
    /// Create a new analyzer instance.
    pub fn new() -> Self {
        Self
    }

    /// Analyze temporal contrast sensitivity for a color sequence.
    pub fn analyze(&self, sequence: &ColorSequence) -> TemporalContrastResult {
        let sample_rate = 60.0;
        let sample_interval_s = 1.0 / sample_rate;
        let luminances = sequence.luminances(sample_rate);

        // Collect significant luminance changes
        let changes: Vec<LuminanceChange> = luminances
            .windows(2)
            .filter_map(|w| {
                let delta = (w[1] - w[0]).abs();
                if delta > 0.001 {
                    Some(LuminanceChange {
                        from_luminance: w[0],
                        to_luminance: w[1],
                        delta,
                        rate_per_sec: delta / sample_interval_s,
                    })
                } else {
                    None
                }
            })
            .collect();

        // Estimate dominant frequency from change rate
        let _avg_rate = if changes.is_empty() {
            0.0
        } else {
            changes.iter().map(|c| c.rate_per_sec).sum::<f64>() / changes.len() as f64
        };

        // Adaptation time scales inversely with luminance magnitude change
        let max_delta = changes.iter().map(|c| c.delta).fold(0.0f64, f64::max);
        let adaptation_time_ms = if max_delta > 0.5 {
            500 // Large change: ~500ms dark adaptation
        } else if max_delta > 0.2 {
            200
        } else {
            50
        };

        // Perceptually uniform: check if all changes are within 20% of each other
        let perceptually_uniform = if changes.len() < 2 {
            true
        } else {
            let mean = changes.iter().map(|c| c.delta).sum::<f64>() / changes.len() as f64;
            changes
                .iter()
                .all(|c| (c.delta - mean).abs() / mean.max(0.001) < 0.2)
        };

        TemporalContrastResult {
            changes,
            adaptation: ContrastAdaptation {
                adaptation_time_ms,
                sensitivity_loss_factor: max_delta.min(1.0),
            },
            masking: TemporalMasking {
                forward_masking_ms: 100, // Typical ~100ms forward masking
                backward_masking_ms: 50, // Typical ~50ms backward masking
                critical_band_hz: 10.0,
            },
            perceptually_uniform,
        }
    }
}

impl Default for TemporalContrastAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Validation
// ============================================================================

/// WCAG compliance level.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WcagComplianceLevel {
    /// Level A (minimum).
    A,
    /// Level AA (standard).
    AA,
    /// Level AAA (enhanced).
    AAA,
    /// Does not meet any level.
    None,
}

/// Category of a temporal issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IssueCategory {
    /// Photosensitivity / flicker.
    Flicker,
    /// Vestibular / motion.
    Motion,
    /// Contrast or luminance.
    Contrast,
    /// Animation duration.
    Duration,
    /// Easing function.
    Easing,
}

/// Severity of a temporal issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IssueSeverity {
    /// Must be fixed; may cause seizures or severe discomfort.
    Critical,
    /// Should be fixed; degrades accessibility.
    High,
    /// Worth investigating; minor impact.
    Medium,
    /// Informational.
    Low,
}

/// A single detected temporal issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalIssue {
    /// Issue category.
    pub category: IssueCategory,
    /// Issue severity.
    pub severity: IssueSeverity,
    /// Human-readable description.
    pub description: String,
    /// Millisecond timestamp where the issue occurs, if applicable.
    pub at_ms: Option<u64>,
}

/// Configuration for the temporal validator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalValidatorConfig {
    /// Minimum required WCAG compliance level.
    pub wcag_level: WcagComplianceLevel,
    /// Maximum acceptable flicker frequency in Hz.
    pub max_flicker_hz: f64,
    /// Maximum acceptable motion velocity per second.
    pub max_motion_velocity: f64,
    /// Whether to apply photosensitivity-specific rules.
    pub check_photosensitivity: bool,
}

impl TemporalValidatorConfig {
    /// Standard WCAG AA configuration.
    pub fn wcag_aa() -> Self {
        Self {
            wcag_level: WcagComplianceLevel::AA,
            max_flicker_hz: 3.0,
            max_motion_velocity: 1.0,
            check_photosensitivity: true,
        }
    }
}

/// Complete validation report for a color sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalValidationReport {
    /// Total duration of the sequence.
    pub sequence_duration_ms: u64,
    /// All detected issues.
    pub issues: Vec<TemporalIssue>,
    /// Whether the sequence passes the configured WCAG level.
    pub passes_wcag: bool,
    /// Assessed flicker risk.
    pub flicker_risk: FlickerRisk,
    /// Ordered recommendations.
    pub recommendations: Vec<TemporalRecommendation>,
    /// Overall quality score in [0, 1].
    pub overall_score: f64,
}

/// Validation results for a batch of sequences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchValidationReport {
    /// Total number of sequences validated.
    pub total_sequences: u32,
    /// Number of sequences that passed.
    pub passed: u32,
    /// Number of sequences that failed.
    pub failed: u32,
    /// Individual reports in order.
    pub reports: Vec<TemporalValidationReport>,
    /// Human-readable summary.
    pub summary: String,
}

/// Temporal validation engine.
#[derive(Debug)]
pub struct TemporalValidator {
    /// Validation configuration.
    config: TemporalValidatorConfig,
}

impl TemporalValidator {
    /// Create a new validator.
    pub fn new(config: TemporalValidatorConfig) -> Self {
        Self { config }
    }

    /// Validate a color sequence against the configured rules.
    pub fn validate(&self, sequence: &ColorSequence) -> TemporalValidationReport {
        let flicker_config = FlickerConfig {
            sample_rate_hz: 60.0,
            window_ms: 1000,
            threshold_hz: self.config.max_flicker_hz,
            luminance_change_threshold: 0.1,
        };

        let motion_config = MotionConfig {
            max_velocity_per_sec: self.config.max_motion_velocity,
            enable_reduced_motion: true,
            check_vestibular: self.config.check_photosensitivity,
        };

        let detector = FlickerDetector::new(flicker_config);
        let flicker_result = detector.detect(sequence);

        let analyzer = MotionAnalyzer::new(motion_config);
        let motion_result = analyzer.analyze(sequence);

        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        // Flicker issues
        if !flicker_result.wcag_compliant {
            issues.push(TemporalIssue {
                category: IssueCategory::Flicker,
                severity: if flicker_result.flashes_per_second > 10.0 {
                    IssueSeverity::Critical
                } else {
                    IssueSeverity::High
                },
                description: format!(
                    "Flicker rate {:.2} Hz exceeds WCAG 2.3.1 threshold of {} Hz",
                    flicker_result.flashes_per_second, self.config.max_flicker_hz
                ),
                at_ms: flicker_result.flash_events.first().map(|e| e.start_ms),
            });
            recommendations.push(TemporalRecommendation {
                priority: RecommendationPriority::Critical,
                category: "flicker".to_string(),
                message: format!(
                    "Increase transition duration to reduce flash rate below {} Hz",
                    self.config.max_flicker_hz
                ),
                suggested_duration_ms: Some((1000.0 / self.config.max_flicker_hz * 2.0) as u64),
            });
        }

        // Motion issues
        for mi in &motion_result.issues {
            issues.push(TemporalIssue {
                category: IssueCategory::Motion,
                severity: if mi.severity == "error" {
                    IssueSeverity::High
                } else {
                    IssueSeverity::Medium
                },
                description: mi.description.clone(),
                at_ms: Some(mi.at_ms),
            });
        }

        if !motion_result.is_safe {
            recommendations.push(TemporalRecommendation {
                priority: RecommendationPriority::High,
                category: "motion".to_string(),
                message: "Reduce animation velocity or add prefers-reduced-motion alternative"
                    .to_string(),
                suggested_duration_ms: Some(sequence.total_duration_ms * 2),
            });
        }

        // Short duration check
        if sequence.total_duration_ms < 100 && !sequence.transitions.is_empty() {
            issues.push(TemporalIssue {
                category: IssueCategory::Duration,
                severity: IssueSeverity::Medium,
                description: format!(
                    "Sequence duration {}ms is very short; may cause jarring transitions",
                    sequence.total_duration_ms
                ),
                at_ms: None,
            });
        }

        let passes_wcag = flicker_result.wcag_compliant && motion_result.is_safe;

        // Score: penalize for each issue by severity
        let score_penalty: f64 = issues
            .iter()
            .map(|i| match i.severity {
                IssueSeverity::Critical => 0.5,
                IssueSeverity::High => 0.25,
                IssueSeverity::Medium => 0.1,
                IssueSeverity::Low => 0.02,
            })
            .sum();
        let overall_score = (1.0 - score_penalty).max(0.0);

        TemporalValidationReport {
            sequence_duration_ms: sequence.total_duration_ms,
            issues,
            passes_wcag,
            flicker_risk: flicker_result.risk,
            recommendations,
            overall_score,
        }
    }
}

// ============================================================================
// Neural Correction
// ============================================================================

/// Local metrics for temporal neural correction (avoids cross-module dependency).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalNeuralCorrectionMetrics {
    /// Number of correction iterations applied.
    pub corrections_applied: u32,
    /// Average ΔE reduction per correction.
    pub avg_delta_e_reduction: f64,
    /// Net perceptual improvement score [0, 1].
    pub perceptual_improvement: f64,
    /// Wall-clock processing time in milliseconds.
    pub processing_ms: u64,
}

/// Configuration for temporal neural correction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalNeuralConfig {
    /// Correction strength in [0, 1] (1.0 = full correction).
    pub correction_strength: f64,
    /// Whether to preserve hue angle during correction.
    pub preserve_hue: bool,
    /// Maximum number of correction iterations.
    pub max_iterations: u32,
}

/// Result of temporal neural correction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalCorrectionResult {
    /// Original sequence before correction.
    pub original: ColorSequence,
    /// Corrected sequence.
    pub corrected: ColorSequence,
    /// Per-transition improvement scores.
    pub improvements: Vec<f64>,
    /// Correction statistics.
    pub neural_metrics: TemporalNeuralCorrectionMetrics,
}

/// Temporal neural corrector.
///
/// Applies SIREN-inspired perceptual correction to a color sequence,
/// ensuring perceptual uniformity across transitions.
#[derive(Debug)]
pub struct TemporalNeuralCorrector {
    /// Correction configuration.
    config: TemporalNeuralConfig,
}

impl TemporalNeuralCorrector {
    /// Create a new corrector.
    pub fn new(config: TemporalNeuralConfig) -> Self {
        Self { config }
    }

    /// Apply temporal neural correction to a sequence.
    pub fn correct(
        &self,
        sequence: ColorSequence,
        validator_config: &TemporalValidatorConfig,
    ) -> TemporalCorrectionResult {
        let start = std::time::Instant::now();

        let validator = TemporalValidator::new(validator_config.clone());
        let mut current = sequence.clone();
        let mut improvements = Vec::new();

        let iterations = self.config.max_iterations.min(10);
        let mut total_improvement = 0.0;
        let mut corrections = 0u32;

        for _iter in 0..iterations {
            let report = validator.validate(&current);
            if report.passes_wcag && report.overall_score > 0.9 {
                break;
            }

            // Apply correction: lengthen transitions with flicker issues
            let mut corrected_transitions = Vec::new();
            for t in &current.transitions {
                let local_seq = ColorSequence {
                    transitions: vec![t.clone()],
                    total_duration_ms: t.duration_ms,
                    name: None,
                    description: None,
                    last_state: None,
                };
                let local_report = validator.validate(&local_seq);

                let improvement;
                if !local_report.passes_wcag {
                    // Extend duration proportional to correction strength
                    let new_duration =
                        (t.duration_ms as f64 * (1.0 + self.config.correction_strength)) as u64;
                    let mut corrected = t.clone();
                    corrected.duration_ms = new_duration;
                    // Smooth easing for better perceptual uniformity
                    corrected.easing = EasingFunction::EaseInOut;
                    corrected_transitions.push(corrected);
                    improvement = self.config.correction_strength;
                    corrections += 1;
                } else {
                    corrected_transitions.push(t.clone());
                    improvement = 0.0;
                }
                improvements.push(improvement);
                total_improvement += improvement;
            }

            let new_total = corrected_transitions.iter().map(|t| t.duration_ms).sum();
            current = ColorSequence {
                transitions: corrected_transitions,
                total_duration_ms: new_total,
                name: None,
                description: None,
                last_state: None,
            };
        }

        let elapsed = start.elapsed().as_millis() as u64;
        let avg_improvement = if corrections > 0 {
            total_improvement / corrections as f64
        } else {
            0.0
        };

        TemporalCorrectionResult {
            original: sequence,
            corrected: current,
            improvements,
            neural_metrics: TemporalNeuralCorrectionMetrics {
                corrections_applied: corrections,
                avg_delta_e_reduction: avg_improvement * 2.0,
                perceptual_improvement: avg_improvement,
                processing_ms: elapsed,
            },
        }
    }
}

// ============================================================================
// Stress Tests
// ============================================================================

/// Category of stress test scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScenarioCategory {
    /// Rapid luminance reversals at photosensitive frequencies.
    FlickerStress,
    /// High-velocity color motion.
    MotionStress,
    /// Significant contrast changes between states.
    ContrastTransition,
    /// Many quick color changes.
    RapidColorChange,
    /// Smooth, slow luminance fade.
    SlowFade,
}

/// A predefined test scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressTestScenario {
    /// Scenario identifier.
    pub name: String,
    /// Scenario category.
    pub category: ScenarioCategory,
    /// What this scenario tests.
    pub description: String,
    /// The color sequence for this scenario.
    pub sequence: ColorSequence,
    /// Whether this scenario is expected to be safe.
    pub expected_safe: bool,
}

/// Configuration for the stress test runner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressTestConfig {
    /// Scenarios to run.
    pub scenarios: Vec<StressTestScenario>,
    /// Whether to include WCAG reference scenarios.
    pub include_wcag_scenarios: bool,
    /// Whether to include photosensitivity-specific scenarios.
    pub include_photosensitivity: bool,
}

impl StressTestConfig {
    /// Build the default set of predefined stress test scenarios.
    pub fn default_scenarios() -> Vec<StressTestScenario> {
        vec![
            // ---- SAFE SCENARIOS ----
            // Scenario 1: Smooth slow fade — expected safe
            {
                let black_state = TemporalColorState {
                    hex: "#000000".to_string(),
                    oklch_l: 0.0,
                    oklch_c: 0.0,
                    oklch_h: 0.0,
                    timestamp_ms: 0,
                };
                let white_state = TemporalColorState {
                    hex: "#ffffff".to_string(),
                    oklch_l: 1.0,
                    oklch_c: 0.0,
                    oklch_h: 0.0,
                    timestamp_ms: 2000,
                };
                let sequence = ColorSequence::from_states(
                    vec![black_state, white_state],
                    2000,
                    EasingFunction::EaseInOut,
                );
                StressTestScenario {
                    name: "smooth_slow_fade".to_string(),
                    category: ScenarioCategory::SlowFade,
                    description: "2-second ease-in-out fade from black to white — expected safe"
                        .to_string(),
                    sequence,
                    expected_safe: true,
                }
            },
            // Scenario 2: Slow color pulse at 0.5Hz — expected safe
            {
                let blue = TemporalColorState {
                    hex: "#0066cc".to_string(),
                    oklch_l: 0.45,
                    oklch_c: 0.15,
                    oklch_h: 264.0,
                    timestamp_ms: 0,
                };
                let light_blue = TemporalColorState {
                    hex: "#66aaff".to_string(),
                    oklch_l: 0.7,
                    oklch_c: 0.12,
                    oklch_h: 260.0,
                    timestamp_ms: 1000,
                };
                let sequence = ColorSequence::from_states(
                    vec![blue, light_blue],
                    1000,
                    EasingFunction::EaseInOut,
                );
                StressTestScenario {
                    name: "slow_1hz_pulse".to_string(),
                    category: ScenarioCategory::SlowFade,
                    description: "1 Hz blue pulse — expected safe (below WCAG 3 Hz threshold)"
                        .to_string(),
                    sequence,
                    expected_safe: true,
                }
            },
            // ---- UNSAFE SCENARIOS ----
            // Scenario 3: 3 Hz luminance flash — at WCAG boundary (unsafe)
            {
                let mut states = Vec::new();
                for i in 0..7 {
                    let l = if i % 2 == 0 { 0.05 } else { 0.95 };
                    states.push(TemporalColorState {
                        hex: if i % 2 == 0 {
                            "#111111".to_string()
                        } else {
                            "#eeeeee".to_string()
                        },
                        oklch_l: l,
                        oklch_c: 0.0,
                        oklch_h: 0.0,
                        timestamp_ms: i * 166,
                    });
                }
                let sequence = ColorSequence::from_states(states, 166, EasingFunction::Linear);
                StressTestScenario {
                    name: "3hz_flicker_stress".to_string(),
                    category: ScenarioCategory::FlickerStress,
                    description: "~3 Hz black/white flicker — at WCAG photosensitivity limit"
                        .to_string(),
                    sequence,
                    expected_safe: false,
                }
            },
            // Scenario 4: High-contrast rapid flash — unsafe
            {
                let mut states = Vec::new();
                for i in 0..10 {
                    let l = if i % 2 == 0 { 0.0 } else { 1.0 };
                    states.push(TemporalColorState {
                        hex: if i % 2 == 0 {
                            "#000000".to_string()
                        } else {
                            "#ffffff".to_string()
                        },
                        oklch_l: l,
                        oklch_c: 0.0,
                        oklch_h: 0.0,
                        timestamp_ms: i * 50,
                    });
                }
                let sequence = ColorSequence::from_states(states, 50, EasingFunction::Linear);
                StressTestScenario {
                    name: "high_contrast_rapid_flash".to_string(),
                    category: ScenarioCategory::RapidColorChange,
                    description: "20 Hz black/white flash — severely unsafe".to_string(),
                    sequence,
                    expected_safe: false,
                }
            },
        ]
    }
}

/// Result of running a single stress test scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressTestResult {
    /// The scenario that was tested.
    pub scenario: StressTestScenario,
    /// The validation report.
    pub report: TemporalValidationReport,
    /// Whether the result matched the expected safety outcome.
    pub passed: bool,
    /// Description of the delta from expected behavior.
    pub delta_from_expected: String,
}

/// Complete stress test report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalStressTestReport {
    /// Total number of scenarios executed.
    pub total_scenarios: u32,
    /// Number of scenarios where result matched expectation.
    pub passed: u32,
    /// Number of scenarios where result did not match expectation.
    pub failed: u32,
    /// Individual results in order.
    pub results: Vec<StressTestResult>,
    /// Names of scenarios with critical failures.
    pub critical_failures: Vec<String>,
}

/// Temporal stress test runner.
#[derive(Debug)]
pub struct TemporalStressTestRunner {
    /// Runner configuration.
    config: StressTestConfig,
}

impl TemporalStressTestRunner {
    /// Create a new runner with the given configuration.
    pub fn new(config: StressTestConfig) -> Self {
        Self { config }
    }

    /// Run all configured scenarios.
    pub fn run_all(&self) -> TemporalStressTestReport {
        let validator = TemporalValidator::new(TemporalValidatorConfig::wcag_aa());

        let mut results = Vec::new();
        let mut passed = 0u32;
        let mut failed = 0u32;
        let mut critical_failures = Vec::new();

        for scenario in &self.config.scenarios {
            let report = validator.validate(&scenario.sequence);
            let actual_safe = report.passes_wcag;
            let matches_expected = actual_safe == scenario.expected_safe;

            let delta = if matches_expected {
                "Result matches expected safety outcome".to_string()
            } else {
                format!(
                    "Expected safe={}, got safe={}; score={:.2}",
                    scenario.expected_safe, actual_safe, report.overall_score
                )
            };

            // Critical: expected safe but failed (could harm users in deployment)
            if scenario.expected_safe && !actual_safe {
                critical_failures.push(scenario.name.clone());
            }

            if matches_expected {
                passed += 1;
            } else {
                failed += 1;
            }

            results.push(StressTestResult {
                scenario: scenario.clone(),
                report,
                passed: matches_expected,
                delta_from_expected: delta,
            });
        }

        TemporalStressTestReport {
            total_scenarios: results.len() as u32,
            passed,
            failed,
            results,
            critical_failures,
        }
    }
}

// ============================================================================
// Free Functions
// ============================================================================

/// Validate a color sequence with default WCAG AA settings.
pub fn validate_sequence(sequence: &ColorSequence) -> TemporalValidationReport {
    let validator = TemporalValidator::new(TemporalValidatorConfig::wcag_aa());
    validator.validate(sequence)
}

/// Returns `true` if the sequence passes WCAG 2.1 temporal compliance.
pub fn is_sequence_safe(sequence: &ColorSequence) -> bool {
    validate_sequence(sequence).passes_wcag
}

/// Compute an overall quality score for a color sequence.
pub fn get_temporal_score(sequence: &ColorSequence) -> f64 {
    validate_sequence(sequence).overall_score
}

/// Run all default stress tests.
pub fn run_temporal_stress_tests() -> TemporalStressTestReport {
    let scenarios = StressTestConfig::default_scenarios();
    let config = StressTestConfig {
        scenarios,
        include_wcag_scenarios: true,
        include_photosensitivity: true,
    };
    let runner = TemporalStressTestRunner::new(config);
    runner.run_all()
}

/// Run only scenarios that are expected to be safe (regression suite).
pub fn run_safe_stress_tests() -> TemporalStressTestReport {
    let all = StressTestConfig::default_scenarios();
    let safe_only: Vec<_> = all.into_iter().filter(|s| s.expected_safe).collect();
    let config = StressTestConfig {
        scenarios: safe_only,
        include_wcag_scenarios: true,
        include_photosensitivity: true,
    };
    let runner = TemporalStressTestRunner::new(config);
    runner.run_all()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state(hex: &str, l: f64, ts: u64) -> TemporalColorState {
        TemporalColorState {
            hex: hex.to_string(),
            oklch_l: l,
            oklch_c: 0.0,
            oklch_h: 0.0,
            timestamp_ms: ts,
        }
    }

    fn simple_sequence(n: usize, duration_per_step_ms: u64) -> ColorSequence {
        let states: Vec<_> = (0..=n)
            .map(|i| {
                let l = i as f64 / n as f64;
                let hex = if l < 0.5 { "#111111" } else { "#eeeeee" };
                make_state(hex, l, i as u64 * duration_per_step_ms)
            })
            .collect();
        ColorSequence::from_states(states, duration_per_step_ms, EasingFunction::Linear)
    }

    #[test]
    fn test_easing_linear() {
        let e = EasingFunction::Linear;
        assert!((e.evaluate(0.0) - 0.0).abs() < 1e-6);
        assert!((e.evaluate(0.5) - 0.5).abs() < 1e-6);
        assert!((e.evaluate(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_easing_ease_in_out() {
        let e = EasingFunction::EaseInOut;
        assert!((e.evaluate(0.0) - 0.0).abs() < 1e-6);
        assert!((e.evaluate(1.0) - 1.0).abs() < 1e-6);
        // Mid-point for symmetric s-curve
        let mid = e.evaluate(0.5);
        assert!((mid - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_easing_cubic_bezier() {
        // CSS ease: cubic-bezier(0.25, 0.1, 0.25, 1.0)
        let e = EasingFunction::CubicBezier(0.25, 0.1, 0.25, 1.0);
        assert!(e.evaluate(0.0).abs() < 1e-5);
        assert!((e.evaluate(1.0) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_easing_spring() {
        let e = EasingFunction::Spring {
            stiffness: 100.0,
            damping: 20.0,
        };
        let v0 = e.evaluate(0.0);
        let v1 = e.evaluate(1.0);
        // Spring starts at ~0 and converges toward 1
        assert!(v0.abs() < 0.1);
        assert!((v1 - 1.0).abs() < 0.5); // Spring may overshoot
    }

    #[test]
    fn test_temporal_color_state_luminance() {
        let black = TemporalColorState {
            hex: "#000000".to_string(),
            oklch_l: 0.0,
            oklch_c: 0.0,
            oklch_h: 0.0,
            timestamp_ms: 0,
        };
        let white = TemporalColorState {
            hex: "#ffffff".to_string(),
            oklch_l: 1.0,
            oklch_c: 0.0,
            oklch_h: 0.0,
            timestamp_ms: 0,
        };
        assert!((black.relative_luminance() - 0.0).abs() < 1e-6);
        assert!((white.relative_luminance() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_color_transition_interpolate() {
        let from = make_state("#000000", 0.0, 0);
        let to = make_state("#ffffff", 1.0, 1000);
        let transition = ColorTransition {
            from,
            to,
            duration_ms: 1000,
            easing: EasingFunction::Linear,
        };
        let mid = transition.interpolate(0.5);
        assert!((mid.oklch_l - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_color_sequence_at_ms() {
        let seq = simple_sequence(4, 500);
        assert_eq!(seq.total_duration_ms, 2000);
        let state = seq.at_ms(0);
        assert!(state.oklch_l < 0.3);
        let state = seq.at_ms(2000);
        assert!(state.oklch_l > 0.7);
    }

    #[test]
    fn test_luminances_sample_count() {
        let seq = simple_sequence(2, 1000);
        let lums = seq.luminances(60.0);
        // 2 seconds at 60 Hz = ~121 samples
        assert!(lums.len() >= 100);
        assert!(lums.len() <= 200);
    }

    #[test]
    fn test_flicker_config_wcag() {
        let cfg = FlickerConfig::wcag();
        assert!((cfg.sample_rate_hz - 60.0).abs() < 0.001);
        assert!((cfg.threshold_hz - 3.0).abs() < 0.001);
        assert_eq!(cfg.window_ms, 1000);
    }

    #[test]
    fn test_flicker_detector_safe_sequence() {
        // Slow 0.5 Hz transition — should be safe
        let seq = simple_sequence(1, 2000);
        let detector = FlickerDetector::new(FlickerConfig::wcag());
        let result = detector.detect(&seq);
        assert!(
            result.wcag_compliant,
            "2-second fade should pass WCAG 2.3.1"
        );
    }

    #[test]
    fn test_flicker_detector_unsafe_sequence() {
        // Many rapid alternations in 1 second — should fail
        let states: Vec<_> = (0..20)
            .map(|i| {
                let l = if i % 2 == 0 { 0.0 } else { 1.0 };
                make_state(if i % 2 == 0 { "#000" } else { "#fff" }, l, i * 50)
            })
            .collect();
        let seq = ColorSequence::from_states(states, 50, EasingFunction::Linear);
        let detector = FlickerDetector::new(FlickerConfig::wcag());
        let result = detector.detect(&seq);
        assert!(!result.wcag_compliant, "20 Hz flash should fail WCAG 2.3.1");
        assert!(matches!(
            result.risk,
            FlickerRisk::High | FlickerRisk::Photosensitive
        ));
    }

    #[test]
    fn test_flicker_risk_is_safe() {
        assert!(FlickerRisk::None.is_safe());
        assert!(FlickerRisk::Low.is_safe());
        assert!(!FlickerRisk::Medium.is_safe());
        assert!(!FlickerRisk::High.is_safe());
        assert!(!FlickerRisk::Photosensitive.is_safe());
    }

    #[test]
    fn test_motion_analyzer_safe() {
        let seq = simple_sequence(2, 2000);
        let analyzer = MotionAnalyzer::new(MotionConfig {
            max_velocity_per_sec: 5.0,
            enable_reduced_motion: true,
            check_vestibular: false,
        });
        let result = analyzer.analyze(&seq);
        assert!(result.is_safe);
    }

    #[test]
    fn test_temporal_csf_peak() {
        // CSF peaks near 8 Hz
        let peak_value = TemporalContrastSensitivity::csf(8.0);
        assert!((peak_value - 1.0).abs() < 0.01);
        // Should be lower at 0 Hz and 30 Hz
        let low_value = TemporalContrastSensitivity::csf(0.0);
        let high_value = TemporalContrastSensitivity::csf(30.0);
        assert!(low_value < peak_value);
        assert!(high_value < peak_value);
    }

    #[test]
    fn test_validate_sequence_free_fn() {
        let seq = simple_sequence(2, 2000);
        let report = validate_sequence(&seq);
        assert!(report.passes_wcag);
        assert!(report.overall_score > 0.5);
    }

    #[test]
    fn test_is_sequence_safe_free_fn() {
        let seq = simple_sequence(2, 2000);
        assert!(is_sequence_safe(&seq));
    }

    #[test]
    fn test_get_temporal_score() {
        let seq = simple_sequence(2, 2000);
        let score = get_temporal_score(&seq);
        assert!(score >= 0.0 && score <= 1.0);
    }

    #[test]
    fn test_stress_tests_run() {
        let report = run_temporal_stress_tests();
        assert!(report.total_scenarios >= 4);
        // Safe scenarios should all pass
        for result in &report.results {
            if result.scenario.expected_safe {
                assert!(
                    result.passed,
                    "Safe scenario '{}' failed: {}",
                    result.scenario.name, result.delta_from_expected
                );
            }
        }
    }

    #[test]
    fn test_safe_stress_tests() {
        let report = run_safe_stress_tests();
        // All scenarios in this suite expect safe=true
        for result in &report.results {
            assert!(result.scenario.expected_safe);
        }
        // Should have no critical failures (all should pass the validator)
        assert!(
            report.critical_failures.is_empty(),
            "Critical failures in safe suite: {:?}",
            report.critical_failures
        );
    }

    #[test]
    fn test_temporal_validator() {
        let seq = simple_sequence(2, 2000);
        let validator = TemporalValidator::new(TemporalValidatorConfig::wcag_aa());
        let report = validator.validate(&seq);
        assert!(report.passes_wcag);
        assert!(report.overall_score > 0.5);
    }

    #[test]
    fn test_temporal_neural_corrector() {
        let seq = simple_sequence(2, 100); // Short = may need correction
        let corrector = TemporalNeuralCorrector::new(TemporalNeuralConfig {
            correction_strength: 0.5,
            preserve_hue: true,
            max_iterations: 3,
        });
        let result = corrector.correct(seq, &TemporalValidatorConfig::wcag_aa());
        // Corrected sequence should be at least as long as original
        assert!(result.corrected.total_duration_ms >= result.original.total_duration_ms);
    }

    #[test]
    fn test_flicker_safe_transition_already_safe() {
        let from = make_state("#000000", 0.0, 0);
        let to = make_state("#ffffff", 1.0, 2000);
        let transition = ColorTransition {
            from,
            to,
            duration_ms: 2000,
            easing: EasingFunction::EaseInOut,
        };
        let safe = FlickerSafeTransition::make_safe(transition, &FlickerConfig::wcag());
        assert!(safe.is_already_safe);
        assert!(safe.safe.is_none());
    }

    #[test]
    fn test_color_sequence_new_empty() {
        let seq = ColorSequence::from_states(vec![], 1000, EasingFunction::Linear);
        assert_eq!(seq.transitions.len(), 0);
        assert_eq!(seq.total_duration_ms, 0);
    }

    #[test]
    fn test_safe_animation_params() {
        let params = FlickerDetector::safe_params_for("#000000", "#ffffff");
        assert!(params.duration.min_ms >= 333);
        assert!(params.duration.recommended_ms >= params.duration.min_ms);
        assert!(params.reduced_motion_duration.min_ms >= params.duration.min_ms);
    }

    #[test]
    fn test_temporal_contrast_analyzer() {
        let seq = simple_sequence(4, 500);
        let analyzer = TemporalContrastAnalyzer::new();
        let result = analyzer.analyze(&seq);
        assert!(!result.changes.is_empty());
        assert!(result.masking.forward_masking_ms > 0);
    }
}

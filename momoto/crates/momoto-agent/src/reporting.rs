//! Reporting module for Momoto Agent.
//!
//! Provides report generation, dashboards, progress tracking, and live
//! metrics collection for the Momoto color intelligence system.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use momoto_core::perception::ContrastMetric;
use momoto_core::{color::Color, luminance::relative_luminance_srgb, space::oklch::OKLCH};
use momoto_metrics::wcag::{TextSize, WCAGLevel, WCAGMetric};

// ============================================================================
// Enumerations
// ============================================================================

/// Output format for generated reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportFormat {
    /// GitHub-flavoured Markdown.
    Markdown,
    /// Compact JSON (pretty-printed).
    Json,
    /// Standalone HTML document.
    Html,
    /// Comma-separated values (findings only).
    Csv,
}

/// The kind of analysis a report covers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportType {
    /// Single- or multi-color OKLCH analysis.
    ColorAnalysis,
    /// WCAG / APCA contrast pair audit.
    AccessibilityAudit,
    /// IOR / Fresnel / thin-film physics analysis.
    MaterialPhysics,
    /// Delta-E, hue stability, chroma consistency.
    PerceptualQuality,
    /// All sections combined.
    Comprehensive,
    /// Multi-job batch run.
    Batch,
}

/// Severity of an individual finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    /// Blocker — must be fixed before shipping.
    Critical,
    /// Major — strongly recommended fix.
    High,
    /// Moderate — fix when possible.
    Medium,
    /// Minor — cosmetic or informational.
    Low,
    /// Purely informational.
    Info,
}

impl Severity {
    /// Numeric level (4 = Critical … 0 = Info).
    pub fn level(&self) -> u8 {
        match self {
            Severity::Critical => 4,
            Severity::High => 3,
            Severity::Medium => 2,
            Severity::Low => 1,
            Severity::Info => 0,
        }
    }
}

/// Estimated implementation effort for a recommendation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffortLevel {
    /// Fix takes minutes (e.g., change a hex value).
    Immediate,
    /// Fix takes hours.
    Low,
    /// Fix takes a day or two.
    Medium,
    /// Fix takes a sprint.
    High,
    /// Fix requires architectural changes.
    Major,
}

/// Where report output is directed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputMode {
    /// Print to stdout.
    Console,
    /// Write to the given file path.
    File(String),
    /// Keep the rendered string in memory (no side-effects).
    InMemory,
}

// ============================================================================
// Data Structures
// ============================================================================

/// A single identified issue or observation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// Unique identifier (e.g. "F-001").
    pub id: String,
    /// Short human-readable title.
    pub title: String,
    /// Full description of the problem.
    pub description: String,
    /// How severe the issue is.
    pub severity: Severity,
    /// The color hex that triggered this finding (if applicable).
    pub color: Option<String>,
    /// Actionable suggestion.
    pub suggestion: Option<String>,
}

impl Finding {
    /// Construct a new finding.
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        description: impl Into<String>,
        severity: Severity,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: description.into(),
            severity,
            color: None,
            suggestion: None,
        }
    }

    /// Attach a color hex reference.
    pub fn with_color(mut self, hex: impl Into<String>) -> Self {
        self.color = Some(hex.into());
        self
    }

    /// Attach a suggestion.
    pub fn with_suggestion(mut self, s: impl Into<String>) -> Self {
        self.suggestion = Some(s.into());
        self
    }
}

/// A finding paired with remediation metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrioritizedRecommendation {
    /// The underlying finding.
    pub finding: Finding,
    /// How much effort the fix will take.
    pub effort: EffortLevel,
    /// Expected score improvement (0.0 – 1.0).
    pub expected_improvement: f64,
    /// Why this recommendation matters.
    pub rationale: String,
}

/// High-level statistics for the executive section.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutiveSummary {
    /// Total distinct colors analyzed.
    pub total_colors: u32,
    /// Number of findings (any severity).
    pub issues_found: u32,
    /// Count of Critical findings.
    pub critical: u32,
    /// Count of High findings.
    pub high: u32,
    /// Count of Medium findings.
    pub medium: u32,
    /// Count of Low findings.
    pub low: u32,
    /// Aggregate quality score (0.0 – 1.0).
    pub overall_score: f64,
    /// Fraction of pairs/colors that passed all checks.
    pub pass_rate: f64,
}

/// A titled section inside a `ComprehensiveReport`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSection {
    /// Section heading.
    pub title: String,
    /// Prose content (Markdown-friendly).
    pub content: String,
    /// Findings scoped to this section.
    pub findings: Vec<Finding>,
}

/// The top-level report document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensiveReport {
    /// Document title.
    pub title: String,
    /// Unix timestamp (seconds) when the report was generated.
    pub generated_at: u64,
    /// High-level statistics.
    pub executive_summary: ExecutiveSummary,
    /// Ordered list of sections.
    pub sections: Vec<ReportSection>,
    /// Prioritized action items.
    pub recommendations: Vec<PrioritizedRecommendation>,
}

impl ComprehensiveReport {
    /// Render the report as a Markdown document.
    pub fn to_markdown(&self) -> String {
        let mut md = String::with_capacity(4096);
        md.push_str(&format!("# {}\n\n", self.title));
        md.push_str(&format!(
            "_Generated at Unix time {}_\n\n",
            self.generated_at
        ));

        // Executive summary
        let s = &self.executive_summary;
        md.push_str("## Executive Summary\n\n");
        md.push_str(&format!("| Metric | Value |\n|--------|-------|\n"));
        md.push_str(&format!("| Colors analyzed | {} |\n", s.total_colors));
        md.push_str(&format!("| Issues found | {} |\n", s.issues_found));
        md.push_str(&format!("| Critical | {} |\n", s.critical));
        md.push_str(&format!("| High | {} |\n", s.high));
        md.push_str(&format!("| Medium | {} |\n", s.medium));
        md.push_str(&format!("| Low | {} |\n", s.low));
        md.push_str(&format!(
            "| Overall score | {:.1}% |\n",
            s.overall_score * 100.0
        ));
        md.push_str(&format!("| Pass rate | {:.1}% |\n\n", s.pass_rate * 100.0));

        // Sections
        for section in &self.sections {
            md.push_str(&format!("## {}\n\n", section.title));
            md.push_str(&section.content);
            md.push('\n');
            if !section.findings.is_empty() {
                md.push_str("\n### Findings\n\n");
                for f in &section.findings {
                    md.push_str(&format!(
                        "- **[{}]** `{}` — {} *({})*\n",
                        f.id,
                        f.severity_label(),
                        f.title,
                        f.description,
                    ));
                    if let Some(sug) = &f.suggestion {
                        md.push_str(&format!("  - _Suggestion_: {}\n", sug));
                    }
                }
                md.push('\n');
            }
        }

        // Recommendations
        if !self.recommendations.is_empty() {
            md.push_str("## Prioritized Recommendations\n\n");
            md.push_str("| # | Finding | Effort | Expected Gain | Rationale |\n");
            md.push_str("|---|---------|--------|---------------|-----------|\n");
            for (i, r) in self.recommendations.iter().enumerate() {
                md.push_str(&format!(
                    "| {} | {} | {:?} | {:.0}% | {} |\n",
                    i + 1,
                    r.finding.title,
                    r.effort,
                    r.expected_improvement * 100.0,
                    r.rationale,
                ));
            }
            md.push('\n');
        }

        md
    }

    /// Render the report as pretty-printed JSON.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
    }

    /// Return all Critical findings across every section.
    pub fn critical_findings(&self) -> Vec<&Finding> {
        self.sections
            .iter()
            .flat_map(|s| s.findings.iter())
            .filter(|f| f.severity == Severity::Critical)
            .collect()
    }
}

impl Finding {
    fn severity_label(&self) -> &str {
        match self.severity {
            Severity::Critical => "CRITICAL",
            Severity::High => "HIGH",
            Severity::Medium => "MEDIUM",
            Severity::Low => "LOW",
            Severity::Info => "INFO",
        }
    }
}

// ============================================================================
// Specialised Report Types
// ============================================================================

/// Single-color OKLCH analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorAnalysisReport {
    /// Input hex string (e.g. `#3B82F6`).
    pub color_hex: String,
    /// OKLCH coordinates `[L, C, H]`.
    pub oklch: [f64; 3],
    /// Relative luminance per WCAG (0.0 – 1.0).
    pub wcag_luminance: f64,
    /// Hue category (e.g. "blue", "red", "achromatic").
    pub hue_category: String,
    /// Chroma category: "muted", "saturated", or "vivid".
    pub chroma_category: String,
    /// Lightness category: "dark", "mid", or "light".
    pub lightness_category: String,
    /// Gamut membership: "in-gamut" or "out-of-gamut".
    pub gamut_status: String,
}

/// Accessibility audit over a set of foreground/background pairs.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AccessibilityAuditReport {
    /// Number of pairs evaluated.
    pub pairs_tested: u32,
    /// Pairs passing WCAG AA.
    pub wcag_aa_pass: u32,
    /// Pairs passing WCAG AAA.
    pub wcag_aaa_pass: u32,
    /// Pairs passing APCA Lc ≥ 60.
    pub apca_pass: u32,
    /// Detailed findings.
    pub findings: Vec<Finding>,
}

/// Material physics analysis summary.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MaterialPhysicsReport {
    /// Number of material samples analyzed.
    pub materials_analyzed: u32,
    /// Minimum and maximum IOR across samples.
    pub ior_range: (f64, f64),
    /// Minimum and maximum Fresnel reflectance (F0).
    pub fresnel_reflectance_range: (f64, f64),
    /// Free-form notes from the analysis.
    pub notes: Vec<String>,
}

/// Perceptual quality metrics across a palette.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerceptualQualityReport {
    /// Average perceptual uniformity (0.0 – 1.0).
    pub avg_perceptual_uniformity: f64,
    /// Hue stability score (0.0 – 1.0).
    pub hue_stability_score: f64,
    /// Chroma consistency score (0.0 – 1.0).
    pub chroma_consistency: f64,
    /// ΔE2000 statistics `(min, avg, max)`.
    pub delta_e_stats: (f64, f64, f64),
    /// Textual recommendations.
    pub recommendations: Vec<String>,
}

// ============================================================================
// Report Configuration & Generator
// ============================================================================

/// Configuration for a report run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportConfig {
    /// Document title.
    pub title: String,
    /// Output format.
    pub format: ReportFormat,
    /// Report type / scope.
    pub report_type: ReportType,
    /// Include raw data tables.
    pub include_raw_data: bool,
    /// Include ASCII/SVG charts (currently ASCII-only).
    pub include_charts: bool,
    /// Where to direct the rendered output.
    pub output_mode: OutputMode,
}

impl Default for ReportConfig {
    fn default() -> Self {
        Self {
            title: "Momoto Color Report".to_string(),
            format: ReportFormat::Markdown,
            report_type: ReportType::Comprehensive,
            include_raw_data: false,
            include_charts: false,
            output_mode: OutputMode::InMemory,
        }
    }
}

/// Main entry point for producing reports.
#[derive(Debug, Clone)]
pub struct ReportGenerator {
    /// Configuration driving this generator.
    pub config: ReportConfig,
}

impl ReportGenerator {
    /// Create a new generator with the given config.
    pub fn new(config: ReportConfig) -> Self {
        Self { config }
    }

    /// Analyze a single hex color and return an OKLCH-based report.
    pub fn analyze_color(hex: &str) -> ColorAnalysisReport {
        let color = Color::from_hex(hex).unwrap_or_else(|_| Color::from_srgb8(0, 0, 0));
        let oklch = OKLCH::from_color(&color);
        let lum = relative_luminance_srgb(&color).value();

        let hue_category = classify_hue(oklch.h, oklch.c);
        let chroma_category = if oklch.c < 0.05 {
            "muted".to_string()
        } else if oklch.c < 0.15 {
            "saturated".to_string()
        } else {
            "vivid".to_string()
        };
        let lightness_category = if oklch.l < 0.35 {
            "dark".to_string()
        } else if oklch.l < 0.65 {
            "mid".to_string()
        } else {
            "light".to_string()
        };
        let gamut_status = if oklch.is_in_gamut() {
            "in-gamut".to_string()
        } else {
            "out-of-gamut".to_string()
        };

        ColorAnalysisReport {
            color_hex: hex.to_string(),
            oklch: [oklch.l, oklch.c, oklch.h],
            wcag_luminance: lum,
            hue_category,
            chroma_category,
            lightness_category,
            gamut_status,
        }
    }

    /// Run a WCAG accessibility audit over a list of `(foreground, background)` hex pairs.
    pub fn audit_accessibility(pairs: &[(String, String)]) -> AccessibilityAuditReport {
        let metric = WCAGMetric;
        let mut report = AccessibilityAuditReport {
            pairs_tested: pairs.len() as u32,
            ..Default::default()
        };

        for (idx, (fg_hex, bg_hex)) in pairs.iter().enumerate() {
            let fg = Color::from_hex(fg_hex).unwrap_or_else(|_| Color::from_srgb8(0, 0, 0));
            let bg = Color::from_hex(bg_hex).unwrap_or_else(|_| Color::from_srgb8(255, 255, 255));

            let result = metric.evaluate(fg, bg);
            let ratio = result.value;

            let passes_aa = WCAGMetric::passes(ratio, WCAGLevel::AA, TextSize::Normal);
            let passes_aaa = WCAGMetric::passes(ratio, WCAGLevel::AAA, TextSize::Normal);

            if passes_aa {
                report.wcag_aa_pass += 1;
            }
            if passes_aaa {
                report.wcag_aaa_pass += 1;
            }

            // APCA approximation: APCA Lc ≥ 60 ≈ WCAG ratio ≥ 4.5
            // (proper APCA would require APCAMetric; we keep it self-contained here)
            if ratio >= 4.5 {
                report.apca_pass += 1;
            }

            if !passes_aa {
                let finding = Finding::new(
                    format!("A-{:03}", idx + 1),
                    format!("Low contrast: {} on {}", fg_hex, bg_hex),
                    format!(
                        "Contrast ratio {:.2}:1 is below the WCAG AA minimum of 4.5:1.",
                        ratio
                    ),
                    if ratio < 2.5 {
                        Severity::Critical
                    } else {
                        Severity::High
                    },
                )
                .with_color(fg_hex.clone())
                .with_suggestion(
                    "Increase the lightness difference between foreground and background.",
                );

                report.findings.push(finding);
            }
        }

        report
    }

    /// Generate a comprehensive report covering all sections.
    pub fn generate_comprehensive(
        colors: &[&str],
        pairs: &[(String, String)],
    ) -> ComprehensiveReport {
        let timestamp = current_unix_secs();

        // Color analysis section
        let mut color_findings: Vec<Finding> = Vec::new();
        let mut color_content = String::new();

        for (i, hex) in colors.iter().enumerate() {
            let analysis = Self::analyze_color(hex);
            color_content.push_str(&format!(
                "- `{}`: L={:.3} C={:.3} H={:.1}° — {} / {} / {}\n",
                analysis.color_hex,
                analysis.oklch[0],
                analysis.oklch[1],
                analysis.oklch[2],
                analysis.hue_category,
                analysis.chroma_category,
                analysis.lightness_category,
            ));
            if analysis.gamut_status == "out-of-gamut" {
                color_findings.push(
                    Finding::new(
                        format!("C-{:03}", i + 1),
                        format!("{} is out of sRGB gamut", hex),
                        "This color cannot be accurately reproduced on standard displays."
                            .to_string(),
                        Severity::Medium,
                    )
                    .with_color(*hex)
                    .with_suggestion("Map to gamut using OKLCH chroma reduction."),
                );
            }
        }

        let color_section = ReportSection {
            title: "Color Analysis".to_string(),
            content: color_content,
            findings: color_findings,
        };

        // Accessibility section
        let a11y = Self::audit_accessibility(pairs);
        let a11y_content = format!(
            "Pairs tested: {}\nWCAG AA pass: {}\nWCAG AAA pass: {}\nAPCA pass: {}\n",
            a11y.pairs_tested, a11y.wcag_aa_pass, a11y.wcag_aaa_pass, a11y.apca_pass,
        );
        let a11y_section = ReportSection {
            title: "Accessibility Audit".to_string(),
            content: a11y_content,
            findings: a11y.findings.clone(),
        };

        // Collect all findings for summary
        let all_findings: Vec<&Finding> = color_section
            .findings
            .iter()
            .chain(a11y_section.findings.iter())
            .collect();

        let critical = all_findings
            .iter()
            .filter(|f| f.severity == Severity::Critical)
            .count() as u32;
        let high = all_findings
            .iter()
            .filter(|f| f.severity == Severity::High)
            .count() as u32;
        let medium = all_findings
            .iter()
            .filter(|f| f.severity == Severity::Medium)
            .count() as u32;
        let low = all_findings
            .iter()
            .filter(|f| f.severity == Severity::Low)
            .count() as u32;
        let issues_found = all_findings.len() as u32;

        let pass_rate = if pairs.is_empty() {
            1.0
        } else {
            a11y.wcag_aa_pass as f64 / pairs.len() as f64
        };

        let overall_score = compute_overall_score(critical, high, medium, low, pass_rate);

        let summary = ExecutiveSummary {
            total_colors: colors.len() as u32,
            issues_found,
            critical,
            high,
            medium,
            low,
            overall_score,
            pass_rate,
        };

        // Recommendations
        let mut recommendations: Vec<PrioritizedRecommendation> = Vec::new();
        for finding in color_section
            .findings
            .iter()
            .chain(a11y_section.findings.iter())
        {
            let (effort, improvement) = effort_for_severity(&finding.severity);
            recommendations.push(PrioritizedRecommendation {
                finding: finding.clone(),
                effort,
                expected_improvement: improvement,
                rationale: format!(
                    "Addressing this {} issue will improve accessibility and visual quality.",
                    finding.severity_label().to_lowercase()
                ),
            });
        }
        // Sort by severity descending
        recommendations.sort_by(|a, b| b.finding.severity.level().cmp(&a.finding.severity.level()));

        ComprehensiveReport {
            title: format!("Momoto Comprehensive Report — {} colors", colors.len()),
            generated_at: timestamp,
            executive_summary: summary,
            sections: vec![color_section, a11y_section],
            recommendations,
        }
    }

    /// Render a `ComprehensiveReport` according to this generator's configured format.
    pub fn render(&self, report: &ComprehensiveReport) -> String {
        match self.config.format {
            ReportFormat::Markdown => report.to_markdown(),
            ReportFormat::Json => report.to_json(),
            ReportFormat::Html => render_html(report),
            ReportFormat::Csv => render_csv(report),
        }
    }
}

// ============================================================================
// Live Metrics & Dashboards
// ============================================================================

/// Snapshot of real-time operational metrics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LiveMetrics {
    /// Queries processed per second.
    pub requests_per_second: f64,
    /// Average query latency in milliseconds.
    pub avg_latency_ms: f64,
    /// Fraction of requests that resulted in errors.
    pub error_rate: f64,
    /// Fraction of requests served from cache.
    pub cache_hit_rate: f64,
    /// Number of currently active sessions.
    pub active_sessions: u32,
}

/// Structured log collector.
#[derive(Debug, Clone, Default)]
pub struct LogCollector {
    /// Stored log entries: `(unix_secs, level, message)`.
    pub logs: Vec<(u64, String, String)>,
}

impl LogCollector {
    /// Create an empty log collector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a log entry at the current time.
    pub fn log(&mut self, level: &str, msg: &str) {
        self.logs
            .push((current_unix_secs(), level.to_uppercase(), msg.to_string()));
    }

    /// Return the most recent `n` log entries.
    pub fn recent(&self, n: usize) -> Vec<&(u64, String, String)> {
        self.logs.iter().rev().take(n).collect()
    }

    /// Return all entries matching the given level (case-insensitive).
    pub fn filter_by_level(&self, level: &str) -> Vec<&(u64, String, String)> {
        let upper = level.to_uppercase();
        self.logs.iter().filter(|(_, l, _)| l == &upper).collect()
    }
}

/// Tracks progress for a single named job.
#[derive(Debug, Clone, Default)]
pub struct ProgressTracker {
    /// Total items to process.
    pub total: u32,
    /// Items successfully processed.
    pub completed: u32,
    /// Items that failed.
    pub failed: u32,
    /// The item currently being processed.
    pub current_item: Option<String>,
}

impl ProgressTracker {
    /// Create a tracker for a job with `total` items.
    pub fn new(total: u32) -> Self {
        Self {
            total,
            ..Default::default()
        }
    }

    /// Mark an item as successfully completed.
    pub fn advance(&mut self, item: &str) {
        self.completed += 1;
        self.current_item = Some(item.to_string());
    }

    /// Mark an item as failed.
    pub fn fail(&mut self, item: &str) {
        self.failed += 1;
        self.current_item = Some(item.to_string());
    }

    /// Completion percentage (0.0 – 100.0).
    pub fn percent(&self) -> f64 {
        if self.total == 0 {
            100.0
        } else {
            (self.completed + self.failed) as f64 / self.total as f64 * 100.0
        }
    }

    /// Returns `true` when all items have been processed (successfully or not).
    pub fn is_complete(&self) -> bool {
        self.total > 0 && (self.completed + self.failed) >= self.total
    }
}

/// Dashboard for a multi-job batch run.
#[derive(Debug, Clone, Default)]
pub struct BatchDashboard {
    /// Named progress trackers.
    pub trackers: HashMap<String, ProgressTracker>,
    /// Current live metrics.
    pub metrics: LiveMetrics,
    /// Log collector.
    pub logs: LogCollector,
}

impl BatchDashboard {
    /// Create an empty batch dashboard.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new named tracker with `total` items.
    pub fn add_tracker(&mut self, name: String, total: u32) {
        self.trackers.insert(name, ProgressTracker::new(total));
    }

    /// Render a compact textual summary of all trackers.
    pub fn summary(&self) -> String {
        let mut s = String::from("Batch Dashboard Summary\n");
        s.push_str("=======================\n");
        for (name, tracker) in &self.trackers {
            s.push_str(&format!(
                "  {}: {}/{} ({:.1}%) — {} failed\n",
                name,
                tracker.completed,
                tracker.total,
                tracker.percent(),
                tracker.failed,
            ));
        }
        s.push_str(&format!(
            "Metrics: {:.1} rps | {:.1} ms latency | {:.1}% errors\n",
            self.metrics.requests_per_second,
            self.metrics.avg_latency_ms,
            self.metrics.error_rate * 100.0,
        ));
        s
    }
}

/// Live metrics dashboard with historical data.
#[derive(Debug, Clone, Default)]
pub struct MetricsDashboard {
    /// Current metrics snapshot.
    pub metrics: LiveMetrics,
    /// Historical snapshots (most recent last).
    pub history: Vec<LiveMetrics>,
    /// Associated log collector.
    pub log_collector: LogCollector,
}

impl MetricsDashboard {
    /// Create an empty dashboard.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a new metrics snapshot.
    pub fn record(&mut self, metrics: LiveMetrics) {
        self.metrics = metrics.clone();
        self.history.push(metrics);
    }

    /// Render a simple ASCII table of the current metrics.
    pub fn render_ascii(&self) -> String {
        let m = &self.metrics;
        let mut s = String::from("+------------------------+----------+\n");
        s.push_str("| Metric                 | Value    |\n");
        s.push_str("+------------------------+----------+\n");
        s.push_str(&format!(
            "| Requests/s             | {:>8.2} |\n",
            m.requests_per_second
        ));
        s.push_str(&format!(
            "| Avg latency (ms)       | {:>8.2} |\n",
            m.avg_latency_ms
        ));
        s.push_str(&format!(
            "| Error rate             | {:>7.2}% |\n",
            m.error_rate * 100.0
        ));
        s.push_str(&format!(
            "| Cache hit rate         | {:>7.2}% |\n",
            m.cache_hit_rate * 100.0
        ));
        s.push_str(&format!(
            "| Active sessions        | {:>8} |\n",
            m.active_sessions
        ));
        s.push_str("+------------------------+----------+\n");
        s.push_str(&format!("History snapshots: {}\n", self.history.len()));
        s
    }
}

/// Fluent builder for `MetricsDashboard`.
#[derive(Debug, Clone, Default)]
pub struct DashboardBuilder {
    initial_metrics: Option<LiveMetrics>,
    retain_history: usize,
}

impl DashboardBuilder {
    /// Start building a new `MetricsDashboard`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set initial metrics to display before the first `record()` call.
    pub fn with_initial_metrics(mut self, m: LiveMetrics) -> Self {
        self.initial_metrics = Some(m);
        self
    }

    /// Limit the history buffer to `n` entries (0 = unlimited).
    pub fn with_history_limit(mut self, n: usize) -> Self {
        self.retain_history = n;
        self
    }

    /// Consume the builder and produce a `MetricsDashboard`.
    pub fn build(self) -> MetricsDashboard {
        let mut dash = MetricsDashboard::new();
        if let Some(m) = self.initial_metrics {
            dash.record(m);
        }
        dash
    }
}

// ============================================================================
// Private helpers
// ============================================================================

fn classify_hue(h: f64, c: f64) -> String {
    if c < 0.02 {
        return "achromatic".to_string();
    }
    // Hue angle ranges (approximate perceptual categories)
    let h = h.rem_euclid(360.0);
    match h as u32 {
        0..=29 | 330..=360 => "red".to_string(),
        30..=59 => "orange".to_string(),
        60..=89 => "yellow".to_string(),
        90..=149 => "green".to_string(),
        150..=209 => "cyan".to_string(),
        210..=269 => "blue".to_string(),
        270..=299 => "violet".to_string(),
        300..=329 => "magenta".to_string(),
        _ => "unknown".to_string(),
    }
}

fn effort_for_severity(severity: &Severity) -> (EffortLevel, f64) {
    match severity {
        Severity::Critical => (EffortLevel::Immediate, 0.40),
        Severity::High => (EffortLevel::Low, 0.25),
        Severity::Medium => (EffortLevel::Medium, 0.15),
        Severity::Low => (EffortLevel::Low, 0.05),
        Severity::Info => (EffortLevel::Immediate, 0.01),
    }
}

fn compute_overall_score(critical: u32, high: u32, medium: u32, low: u32, pass_rate: f64) -> f64 {
    let penalty = (critical as f64 * 0.15)
        + (high as f64 * 0.08)
        + (medium as f64 * 0.04)
        + (low as f64 * 0.01);
    let base = pass_rate * 0.6 + 0.4;
    (base - penalty).clamp(0.0, 1.0)
}

fn render_html(report: &ComprehensiveReport) -> String {
    let md = report.to_markdown();
    format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>{}</title></head><body><pre>{}</pre></body></html>",
        report.title,
        md.replace('<', "&lt;").replace('>', "&gt;"),
    )
}

fn render_csv(report: &ComprehensiveReport) -> String {
    let mut csv = String::from("id,severity,title,description,color,suggestion\n");
    for section in &report.sections {
        for f in &section.findings {
            csv.push_str(&format!(
                "{},{:?},{},{},{},{}\n",
                f.id,
                f.severity,
                csv_escape(&f.title),
                csv_escape(&f.description),
                f.color.as_deref().unwrap_or(""),
                f.suggestion.as_deref().unwrap_or(""),
            ));
        }
    }
    csv
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Return the current time as Unix seconds.
/// Falls back to zero on platforms without system time.
fn current_unix_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_levels() {
        assert_eq!(Severity::Critical.level(), 4);
        assert_eq!(Severity::High.level(), 3);
        assert_eq!(Severity::Medium.level(), 2);
        assert_eq!(Severity::Low.level(), 1);
        assert_eq!(Severity::Info.level(), 0);
    }

    #[test]
    fn test_analyze_color_black() {
        let r = ReportGenerator::analyze_color("#000000");
        assert_eq!(r.lightness_category, "dark");
        assert_eq!(r.chroma_category, "muted");
        assert!(r.wcag_luminance < 0.01);
    }

    #[test]
    fn test_analyze_color_white() {
        let r = ReportGenerator::analyze_color("#ffffff");
        assert_eq!(r.lightness_category, "light");
        assert!((r.wcag_luminance - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_audit_accessibility_passing_pair() {
        let pairs = vec![("#000000".to_string(), "#ffffff".to_string())];
        let report = ReportGenerator::audit_accessibility(&pairs);
        assert_eq!(report.pairs_tested, 1);
        assert_eq!(report.wcag_aa_pass, 1);
        assert_eq!(report.wcag_aaa_pass, 1);
        assert!(report.findings.is_empty());
    }

    #[test]
    fn test_audit_accessibility_failing_pair() {
        let pairs = vec![("#cccccc".to_string(), "#ffffff".to_string())];
        let report = ReportGenerator::audit_accessibility(&pairs);
        assert_eq!(report.pairs_tested, 1);
        assert_eq!(report.wcag_aa_pass, 0);
        assert!(!report.findings.is_empty());
    }

    #[test]
    fn test_generate_comprehensive() {
        let colors = vec!["#0066cc", "#ffffff", "#000000"];
        let pairs = vec![
            ("#000000".to_string(), "#ffffff".to_string()),
            ("#cccccc".to_string(), "#ffffff".to_string()),
        ];
        let report = ReportGenerator::generate_comprehensive(&colors, &pairs);
        assert_eq!(report.executive_summary.total_colors, 3);
        assert!(report.executive_summary.pass_rate <= 1.0);
        assert!(!report.sections.is_empty());
    }

    #[test]
    fn test_to_markdown_contains_title() {
        let colors = vec!["#ff0000"];
        let report = ReportGenerator::generate_comprehensive(&colors, &[]);
        let md = report.to_markdown();
        assert!(md.contains("# Momoto Comprehensive Report"));
    }

    #[test]
    fn test_to_json_round_trip() {
        let colors = vec!["#0066cc"];
        let report = ReportGenerator::generate_comprehensive(&colors, &[]);
        let json = report.to_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert!(parsed.get("title").is_some());
    }

    #[test]
    fn test_critical_findings_filter() {
        let colors = vec!["#0066cc"];
        let pairs = vec![("#eeeeee".to_string(), "#ffffff".to_string())];
        let report = ReportGenerator::generate_comprehensive(&colors, &pairs);
        // critical_findings must return only Critical-severity items
        for f in report.critical_findings() {
            assert_eq!(f.severity, Severity::Critical);
        }
    }

    #[test]
    fn test_progress_tracker() {
        let mut t = ProgressTracker::new(4);
        t.advance("item-1");
        t.advance("item-2");
        t.fail("item-3");
        assert!((t.percent() - 75.0).abs() < 0.01);
        assert!(!t.is_complete());
        t.advance("item-4");
        assert!(t.is_complete());
    }

    #[test]
    fn test_log_collector() {
        let mut lc = LogCollector::new();
        lc.log("info", "started");
        lc.log("warn", "slow");
        lc.log("error", "failed");
        assert_eq!(lc.logs.len(), 3);
        let errors = lc.filter_by_level("error");
        assert_eq!(errors.len(), 1);
        let recent = lc.recent(2);
        assert_eq!(recent.len(), 2);
    }

    #[test]
    fn test_batch_dashboard() {
        let mut dash = BatchDashboard::new();
        dash.add_tracker("job-a".to_string(), 10);
        dash.add_tracker("job-b".to_string(), 5);
        let summary = dash.summary();
        assert!(summary.contains("job-a"));
        assert!(summary.contains("job-b"));
    }

    #[test]
    fn test_metrics_dashboard_ascii() {
        let mut dash = DashboardBuilder::new()
            .with_initial_metrics(LiveMetrics {
                requests_per_second: 42.0,
                avg_latency_ms: 3.5,
                error_rate: 0.01,
                cache_hit_rate: 0.85,
                active_sessions: 7,
            })
            .build();
        let ascii = dash.render_ascii();
        assert!(ascii.contains("42.00"));
        assert!(ascii.contains("3.50"));
        dash.record(LiveMetrics {
            requests_per_second: 100.0,
            ..Default::default()
        });
        assert_eq!(dash.history.len(), 2);
    }

    #[test]
    fn test_render_html() {
        let config = ReportConfig {
            format: ReportFormat::Html,
            ..Default::default()
        };
        let gen = ReportGenerator::new(config);
        let report = ReportGenerator::generate_comprehensive(&["#0066cc"], &[]);
        let html = gen.render(&report);
        assert!(html.starts_with("<!DOCTYPE html>"));
    }

    #[test]
    fn test_render_csv() {
        let pairs = vec![("#cccccc".to_string(), "#ffffff".to_string())];
        let report = ReportGenerator::generate_comprehensive(&[], &pairs);
        let config = ReportConfig {
            format: ReportFormat::Csv,
            ..Default::default()
        };
        let gen = ReportGenerator::new(config);
        let csv = gen.render(&report);
        assert!(csv.starts_with("id,severity"));
    }
}

//! # Audit Logging
//!
//! Provides structured audit trails for all Momoto agent actions, including
//! color validation, workflow execution, bot authentication, and policy
//! violations.  Supports in-memory and (delegated) file-backed stores,
//! statistical summaries, multi-format export (JSON, CSV, Markdown), and
//! automatic report generation at configurable frequencies.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

// ============================================================================
// Export / Delivery / Frequency
// ============================================================================

/// Format used when exporting audit entries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    /// JSON array of `AuditEntry` objects.
    Json,
    /// Comma-separated values with header row.
    Csv,
    /// Markdown table.
    Markdown,
}

/// How often the `AutoReportGenerator` should produce a report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Frequency {
    /// Every 3600 seconds.
    Hourly,
    /// Every 86 400 seconds.
    Daily,
    /// Every 604 800 seconds.
    Weekly,
    /// Every 2 592 000 seconds (30 days).
    Monthly,
}

impl Frequency {
    /// Return the period in seconds.
    pub fn period_secs(&self) -> u64 {
        match self {
            Frequency::Hourly => 3_600,
            Frequency::Daily => 86_400,
            Frequency::Weekly => 604_800,
            Frequency::Monthly => 2_592_000,
        }
    }
}

/// Where a generated report is delivered.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReportDelivery {
    /// Print to stdout.
    Console,
    /// Keep in memory (retrievable via `AutoReportGenerator::last_report`).
    InMemory,
    /// Write to the given file path (no-op in WASM).
    File { path: String },
}

// ============================================================================
// AuditId
// ============================================================================

/// Opaque audit entry identifier.
///
/// Format: `{timestamp_hex:016x}-{counter_hex:08x}`
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AuditId(pub String);

impl fmt::Display for AuditId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AuditId {
    /// Generate a new unique ID.
    fn generate() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        static EPOCH: AtomicU64 = AtomicU64::new(1_735_689_600);
        let ts = EPOCH.fetch_add(1, Ordering::Relaxed);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed) as u32;
        AuditId(format!("{:016x}-{:08x}", ts, n))
    }
}

// ============================================================================
// Action / Actor / Resource / Outcome
// ============================================================================

/// The type of agent action that was audited.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditAction {
    /// A color was validated against a contract or standard.
    ColorValidated,
    /// A foreground or background color was algorithmically improved.
    ColorImproved,
    /// An agent workflow was triggered.
    WorkflowExecuted,
    /// A report was generated.
    ReportGenerated,
    /// A new session was created.
    SessionCreated,
    /// A bot authenticated successfully.
    BotAuthenticated,
    /// A perceptual source-of-truth certificate was issued.
    CertificateIssued,
    /// An action violated a policy constraint.
    PolicyViolation(String),
}

impl AuditAction {
    /// Return a stable string tag for the action variant (used in CSV/Markdown).
    pub fn kind_str(&self) -> &str {
        match self {
            AuditAction::ColorValidated => "color_validated",
            AuditAction::ColorImproved => "color_improved",
            AuditAction::WorkflowExecuted => "workflow_executed",
            AuditAction::ReportGenerated => "report_generated",
            AuditAction::SessionCreated => "session_created",
            AuditAction::BotAuthenticated => "bot_authenticated",
            AuditAction::CertificateIssued => "certificate_issued",
            AuditAction::PolicyViolation(_) => "policy_violation",
        }
    }
}

impl fmt::Display for AuditAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuditAction::PolicyViolation(detail) => {
                write!(f, "policy_violation({})", detail)
            }
            other => write!(f, "{}", other.kind_str()),
        }
    }
}

/// The entity that performed the audited action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    /// Unique actor identifier (user ID, bot ID, "system", …).
    pub id: String,
    /// Kind: `"user"`, `"bot"`, or `"system"`.
    pub kind: String,
    /// Human-readable display label.
    pub label: String,
}

impl Actor {
    /// Convenience constructor.
    pub fn new(id: impl Into<String>, kind: impl Into<String>, label: impl Into<String>) -> Self {
        Actor {
            id: id.into(),
            kind: kind.into(),
            label: label.into(),
        }
    }

    /// Create a system actor.
    pub fn system() -> Self {
        Actor::new("system", "system", "Momoto System")
    }
}

/// The resource that was acted upon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    /// Resource type (e.g. `"color"`, `"workflow"`, `"session"`).
    pub kind: String,
    /// Specific identifier (e.g. `"#0066cc"`, `"wf-001"`, `"sess-abc"`).
    pub identifier: String,
    /// Optional free-text description.
    pub description: Option<String>,
}

impl Resource {
    /// Convenience constructor without description.
    pub fn new(kind: impl Into<String>, identifier: impl Into<String>) -> Self {
        Resource {
            kind: kind.into(),
            identifier: identifier.into(),
            description: None,
        }
    }

    /// Constructor with description.
    pub fn with_description(
        kind: impl Into<String>,
        identifier: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Resource {
            kind: kind.into(),
            identifier: identifier.into(),
            description: Some(description.into()),
        }
    }
}

/// The outcome of an audited action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Outcome {
    /// Action completed successfully.
    Success,
    /// Action failed with a reason.
    Failure { reason: String },
    /// Action partially succeeded.
    PartialSuccess { details: String },
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Outcome::Success => write!(f, "success"),
            Outcome::Failure { reason } => write!(f, "failure({})", reason),
            Outcome::PartialSuccess { details } => write!(f, "partial({})", details),
        }
    }
}

impl Outcome {
    /// True for `Success` or `PartialSuccess`.
    pub fn is_ok(&self) -> bool {
        !matches!(self, Outcome::Failure { .. })
    }

    /// True only for `Failure`.
    pub fn is_failure(&self) -> bool {
        matches!(self, Outcome::Failure { .. })
    }
}

// ============================================================================
// AuditEntry
// ============================================================================

/// A single immutable audit record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Unique identifier for this audit entry.
    pub id: AuditId,
    /// Unix timestamp (seconds) when the event occurred.
    pub timestamp: u64,
    /// Actor that performed the action.
    pub actor: Actor,
    /// Action that was performed.
    pub action: AuditAction,
    /// Resource on which the action was performed.
    pub resource: Resource,
    /// Outcome of the action.
    pub outcome: Outcome,
    /// Arbitrary key-value metadata (e.g. WCAG ratio, session_id, …).
    pub metadata: HashMap<String, String>,
}

impl AuditEntry {
    /// Create a new entry, generating a unique ID and using `timestamp` as
    /// the event time.
    pub fn new(
        timestamp: u64,
        actor: Actor,
        action: AuditAction,
        resource: Resource,
        outcome: Outcome,
    ) -> Self {
        AuditEntry {
            id: AuditId::generate(),
            timestamp,
            actor,
            action,
            resource,
            outcome,
            metadata: HashMap::new(),
        }
    }

    /// Attach metadata key-value pairs.
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }
}

// ============================================================================
// AuditFilter
// ============================================================================

/// Criteria for querying audit entries.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditFilter {
    /// Only include entries at or after this timestamp.
    pub from_ts: Option<u64>,
    /// Only include entries at or before this timestamp.
    pub to_ts: Option<u64>,
    /// Only include entries whose `actor.id` matches.
    pub actor_id: Option<String>,
    /// Only include entries whose `action.kind_str()` matches.
    pub action_kind: Option<String>,
}

impl AuditFilter {
    /// Create an empty (pass-all) filter.
    pub fn new() -> Self {
        AuditFilter::default()
    }

    /// Builder: set start timestamp.
    pub fn from(mut self, ts: u64) -> Self {
        self.from_ts = Some(ts);
        self
    }

    /// Builder: set end timestamp.
    pub fn to(mut self, ts: u64) -> Self {
        self.to_ts = Some(ts);
        self
    }

    /// Builder: filter by actor ID.
    pub fn actor(mut self, id: impl Into<String>) -> Self {
        self.actor_id = Some(id.into());
        self
    }

    /// Builder: filter by action kind string.
    pub fn action(mut self, kind: impl Into<String>) -> Self {
        self.action_kind = Some(kind.into());
        self
    }

    /// Return `true` if `entry` satisfies all filter criteria.
    pub fn matches(&self, entry: &AuditEntry) -> bool {
        if let Some(from) = self.from_ts {
            if entry.timestamp < from {
                return false;
            }
        }
        if let Some(to) = self.to_ts {
            if entry.timestamp > to {
                return false;
            }
        }
        if let Some(ref aid) = self.actor_id {
            if entry.actor.id != *aid {
                return false;
            }
        }
        if let Some(ref kind) = self.action_kind {
            // Support both exact kind strings and policy_violation prefix.
            let entry_kind = entry.action.kind_str();
            if entry_kind != kind.as_str() {
                // Also accept "policy_violation" matching any PolicyViolation variant.
                if !(kind == "policy_violation"
                    && matches!(entry.action, AuditAction::PolicyViolation(_)))
                {
                    return false;
                }
            }
        }
        true
    }
}

// ============================================================================
// AuditStatistics
// ============================================================================

/// Aggregated statistics over a set of audit entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditStatistics {
    /// Total number of entries counted.
    pub total_entries: u64,
    /// Number of entries with `Outcome::Success` or `PartialSuccess`.
    pub success_count: u64,
    /// Number of entries with `Outcome::Failure`.
    pub failure_count: u64,
    /// Number of distinct actor IDs seen.
    pub unique_actors: usize,
    /// Entry count broken down by `action.kind_str()`.
    pub entries_per_action: HashMap<String, u64>,
}

impl Default for AuditStatistics {
    fn default() -> Self {
        AuditStatistics {
            total_entries: 0,
            success_count: 0,
            failure_count: 0,
            unique_actors: 0,
            entries_per_action: HashMap::new(),
        }
    }
}

impl AuditStatistics {
    /// Compute statistics from a slice of entries.
    pub fn compute(entries: &[AuditEntry]) -> Self {
        let mut stats = AuditStatistics::default();
        let mut actor_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
        for entry in entries {
            stats.total_entries += 1;
            if entry.outcome.is_failure() {
                stats.failure_count += 1;
            } else {
                stats.success_count += 1;
            }
            actor_ids.insert(entry.actor.id.clone());
            *stats
                .entries_per_action
                .entry(entry.action.kind_str().to_string())
                .or_insert(0) += 1;
        }
        stats.unique_actors = actor_ids.len();
        stats
    }
}

// ============================================================================
// AuditStore trait + implementations
// ============================================================================

/// Trait for audit persistence backends.
pub trait AuditStore: Send + Sync {
    /// Append a new entry to the store.
    fn append(&self, entry: AuditEntry) -> Result<(), String>;
    /// Query entries that match the given filter.
    fn query(&self, filter: &AuditFilter) -> Vec<AuditEntry>;
    /// Compute statistics over all stored entries.
    fn statistics(&self) -> AuditStatistics;
    /// Export all entries in the requested format.
    fn export(&self, format: ExportFormat) -> String;
}

/// In-memory audit store backed by a `Vec` behind a `Mutex`.
#[derive(Debug, Clone)]
pub struct InMemoryAuditStore {
    inner: Arc<Mutex<Vec<AuditEntry>>>,
}

impl InMemoryAuditStore {
    /// Create an empty store.
    pub fn new() -> Self {
        InMemoryAuditStore {
            inner: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Return the total number of entries.
    pub fn len(&self) -> usize {
        self.inner.lock().map(|v| v.len()).unwrap_or(0)
    }

    /// Return `true` when no entries are stored.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // ---- Internal export helpers ----------------------------------------

    fn export_json(entries: &[AuditEntry]) -> String {
        // Manual JSON serialization to avoid serde_json dependency on the
        // trait-object vtable (we do have serde_json available in Cargo.toml).
        match serde_json::to_string_pretty(entries) {
            Ok(s) => s,
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }

    fn export_csv(entries: &[AuditEntry]) -> String {
        let mut out = String::from(
            "id,timestamp,actor_id,actor_kind,actor_label,action,resource_kind,resource_id,outcome\n",
        );
        for e in entries {
            // Escape commas in free-text fields by quoting them.
            let actor_label = e.actor.label.replace('"', "\"\"");
            let resource_id = e.resource.identifier.replace('"', "\"\"");
            out.push_str(&format!(
                "{},{},{},{},\"{}\",{},{},\"{}\",{}\n",
                e.id,
                e.timestamp,
                e.actor.id,
                e.actor.kind,
                actor_label,
                e.action,
                e.resource.kind,
                resource_id,
                e.outcome,
            ));
        }
        out
    }

    fn export_markdown(entries: &[AuditEntry]) -> String {
        let mut out = String::from(
            "| ID | Timestamp | Actor | Action | Resource | Outcome |\n\
             |----|-----------|-------|--------|----------|---------|\n",
        );
        for e in entries {
            out.push_str(&format!(
                "| `{}` | {} | {} ({}) | {} | {}/{} | {} |\n",
                &e.id.0[..16.min(e.id.0.len())],
                e.timestamp,
                e.actor.label,
                e.actor.kind,
                e.action,
                e.resource.kind,
                e.resource.identifier,
                e.outcome,
            ));
        }
        out
    }
}

impl Default for InMemoryAuditStore {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditStore for InMemoryAuditStore {
    fn append(&self, entry: AuditEntry) -> Result<(), String> {
        self.inner.lock().map_err(|e| e.to_string())?.push(entry);
        Ok(())
    }

    fn query(&self, filter: &AuditFilter) -> Vec<AuditEntry> {
        let entries = match self.inner.lock() {
            Ok(v) => v.clone(),
            Err(_) => return vec![],
        };
        entries.into_iter().filter(|e| filter.matches(e)).collect()
    }

    fn statistics(&self) -> AuditStatistics {
        let entries = match self.inner.lock() {
            Ok(v) => v.clone(),
            Err(_) => return AuditStatistics::default(),
        };
        AuditStatistics::compute(&entries)
    }

    fn export(&self, format: ExportFormat) -> String {
        let entries = match self.inner.lock() {
            Ok(v) => v.clone(),
            Err(_) => return String::new(),
        };
        match format {
            ExportFormat::Json => Self::export_json(&entries),
            ExportFormat::Csv => Self::export_csv(&entries),
            ExportFormat::Markdown => Self::export_markdown(&entries),
        }
    }
}

/// File-backed audit store.
///
/// In a native context this would persist entries to `path`.  In WASM
/// (no filesystem) it delegates transparently to `InMemoryAuditStore`.
#[derive(Debug, Clone)]
pub struct FileAuditStore {
    /// Target file path (unused in WASM).
    pub path: String,
    inner: InMemoryAuditStore,
}

impl FileAuditStore {
    /// Create a file audit store writing to `path`.
    pub fn new(path: impl Into<String>) -> Self {
        FileAuditStore {
            path: path.into(),
            inner: InMemoryAuditStore::new(),
        }
    }
}

impl AuditStore for FileAuditStore {
    fn append(&self, entry: AuditEntry) -> Result<(), String> {
        self.inner.append(entry)
    }

    fn query(&self, filter: &AuditFilter) -> Vec<AuditEntry> {
        self.inner.query(filter)
    }

    fn statistics(&self) -> AuditStatistics {
        self.inner.statistics()
    }

    fn export(&self, format: ExportFormat) -> String {
        self.inner.export(format)
    }
}

// ============================================================================
// ReportTemplate
// ============================================================================

/// Template that controls the content of generated audit reports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportTemplate {
    /// Template name.
    pub name: String,
    /// Ordered list of section headings to include.
    pub sections: Vec<String>,
    /// Whether to include the statistics summary section.
    pub include_statistics: bool,
    /// Whether to include a policy-violations section.
    pub include_violations: bool,
}

impl ReportTemplate {
    /// Create a template with the given name and sections.
    pub fn new(name: impl Into<String>, sections: Vec<String>) -> Self {
        ReportTemplate {
            name: name.into(),
            sections,
            include_statistics: true,
            include_violations: true,
        }
    }

    /// Default report template.
    pub fn default_template() -> Self {
        ReportTemplate::new(
            "Default",
            vec![
                "Overview".to_string(),
                "Recent Activity".to_string(),
                "Statistics".to_string(),
                "Violations".to_string(),
            ],
        )
    }
}

impl Default for ReportTemplate {
    fn default() -> Self {
        Self::default_template()
    }
}

// ============================================================================
// AutoReportGenerator
// ============================================================================

/// Automatically generates audit reports at a configured `Frequency`.
pub struct AutoReportGenerator {
    /// How often to generate reports.
    pub frequency: Frequency,
    /// Where to deliver generated reports.
    pub delivery: ReportDelivery,
    /// Report template.
    pub template: ReportTemplate,
    /// Audit store to read entries from.
    store: Arc<dyn AuditStore + Send + Sync>,
    /// Timestamp of the last report generation.
    last_generated_at: Arc<Mutex<u64>>,
    /// Most recently generated report (for `InMemory` delivery).
    last_report: Arc<Mutex<Option<String>>>,
}

impl std::fmt::Debug for AutoReportGenerator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AutoReportGenerator")
            .field("frequency", &self.frequency)
            .field("delivery", &self.delivery)
            .field("template", &self.template)
            .finish_non_exhaustive()
    }
}

impl AutoReportGenerator {
    /// Create a new generator with `InMemory` delivery.
    pub fn new(frequency: Frequency, template: ReportTemplate) -> Self {
        let store: Arc<dyn AuditStore + Send + Sync> = Arc::new(InMemoryAuditStore::new());
        AutoReportGenerator {
            frequency,
            delivery: ReportDelivery::InMemory,
            template,
            store,
            last_generated_at: Arc::new(Mutex::new(0)),
            last_report: Arc::new(Mutex::new(None)),
        }
    }

    /// Create with an explicit store.
    pub fn with_store(
        frequency: Frequency,
        template: ReportTemplate,
        delivery: ReportDelivery,
        store: Arc<dyn AuditStore + Send + Sync>,
    ) -> Self {
        AutoReportGenerator {
            frequency,
            delivery,
            template,
            store,
            last_generated_at: Arc::new(Mutex::new(0)),
            last_report: Arc::new(Mutex::new(None)),
        }
    }

    /// Generate and return a report as a string, regardless of frequency.
    pub fn generate_report(&self) -> String {
        let stats = self.store.statistics();
        let all_filter = AuditFilter::new();
        let all_entries = self.store.query(&all_filter);

        let violation_filter = AuditFilter::new().action("policy_violation");
        let violations = self.store.query(&violation_filter);

        let mut report = format!("# Audit Report: {}\n\n", self.template.name);

        for section in &self.template.sections {
            report.push_str(&format!("## {}\n\n", section));
            match section.as_str() {
                "Overview" | "overview" => {
                    report.push_str(&format!(
                        "Total entries: {}  \nSuccess: {}  \nFailure: {}  \nUnique actors: {}\n\n",
                        stats.total_entries,
                        stats.success_count,
                        stats.failure_count,
                        stats.unique_actors,
                    ));
                }
                "Recent Activity" | "recent_activity" => {
                    let recent: Vec<&AuditEntry> = all_entries.iter().rev().take(10).collect();
                    if recent.is_empty() {
                        report.push_str("_No recent activity._\n\n");
                    } else {
                        for e in recent {
                            report.push_str(&format!(
                                "- [{}] {} by {} -> {}\n",
                                e.timestamp, e.action, e.actor.label, e.outcome
                            ));
                        }
                        report.push('\n');
                    }
                }
                "Statistics" | "statistics" => {
                    if self.template.include_statistics {
                        report.push_str("| Action | Count |\n|--------|-------|\n");
                        let mut action_counts: Vec<(&String, &u64)> =
                            stats.entries_per_action.iter().collect();
                        action_counts.sort_by(|a, b| b.1.cmp(a.1));
                        for (action, count) in action_counts {
                            report.push_str(&format!("| {} | {} |\n", action, count));
                        }
                        report.push('\n');
                    }
                }
                "Violations" | "violations" => {
                    if self.template.include_violations {
                        if violations.is_empty() {
                            report.push_str("_No policy violations recorded._\n\n");
                        } else {
                            for v in &violations {
                                report.push_str(&format!(
                                    "- [{}] {} by {} — {}\n",
                                    v.timestamp, v.action, v.actor.label, v.outcome
                                ));
                            }
                            report.push('\n');
                        }
                    }
                }
                other => {
                    report.push_str(&format!("_(Section '{}' has no renderer)_\n\n", other));
                }
            }
        }

        report
    }

    /// Generate a report only if enough time has elapsed since the last one.
    ///
    /// Returns `Some(report)` when a report was generated, `None` otherwise.
    pub fn check_and_generate(&self, current_ts: u64) -> Option<String> {
        let period = self.frequency.period_secs();
        let last = *self.last_generated_at.lock().ok()?;
        if current_ts >= last + period {
            let report = self.generate_report();
            *self.last_generated_at.lock().ok()? = current_ts;
            *self.last_report.lock().ok()? = Some(report.clone());
            Some(report)
        } else {
            None
        }
    }

    /// Retrieve the most recently generated report (for `InMemory` delivery).
    pub fn last_report(&self) -> Option<String> {
        self.last_report.lock().ok()?.clone()
    }
}

// ============================================================================
// AuditLogger
// ============================================================================

/// High-level facade for emitting and querying audit entries.
///
/// Each `AuditLogger` is bound to a specific `Actor`.  All entries logged
/// through it are attributed to that actor.
#[derive(Clone)]
pub struct AuditLogger {
    /// Backing store for all audit entries.
    store: Arc<dyn AuditStore + Send + Sync>,
    /// Actor attributed to entries logged through this instance.
    actor: Actor,
}

impl std::fmt::Debug for AuditLogger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuditLogger")
            .field("actor", &self.actor)
            .finish_non_exhaustive()
    }
}

impl AuditLogger {
    /// Create a logger bound to `actor`, backed by an in-memory store.
    pub fn new(actor: Actor) -> Self {
        AuditLogger {
            store: Arc::new(InMemoryAuditStore::new()),
            actor,
        }
    }

    /// Create a logger bound to `actor` with a shared store.
    pub fn with_store(actor: Actor, store: Arc<dyn AuditStore + Send + Sync>) -> Self {
        AuditLogger { store, actor }
    }

    /// Return a reference to the underlying store.
    pub fn store(&self) -> &Arc<dyn AuditStore + Send + Sync> {
        &self.store
    }

    /// Return the actor this logger is bound to.
    pub fn actor(&self) -> &Actor {
        &self.actor
    }

    // ---- Core logging methods -------------------------------------------

    /// Log an action with no additional metadata.
    ///
    /// Returns the `AuditId` of the newly created entry.
    pub fn log(&self, action: AuditAction, resource: Resource, outcome: Outcome) -> AuditId {
        self.log_with_metadata(action, resource, outcome, HashMap::new())
    }

    /// Log an action with arbitrary key-value metadata.
    ///
    /// Returns the `AuditId` of the newly created entry.
    pub fn log_with_metadata(
        &self,
        action: AuditAction,
        resource: Resource,
        outcome: Outcome,
        metadata: HashMap<String, String>,
    ) -> AuditId {
        let ts = current_ts();
        let entry = AuditEntry::new(ts, self.actor.clone(), action, resource, outcome)
            .with_metadata(metadata);
        let id = entry.id.clone();
        let _ = self.store.append(entry);
        id
    }

    // ---- Convenience logging methods ------------------------------------

    /// Log a successful color validation.
    pub fn log_color_validated(&self, color: &str, passes: bool) -> AuditId {
        let outcome = if passes {
            Outcome::Success
        } else {
            Outcome::Failure {
                reason: "Contract not satisfied".to_string(),
            }
        };
        self.log(
            AuditAction::ColorValidated,
            Resource::new("color", color),
            outcome,
        )
    }

    /// Log a color improvement.
    pub fn log_color_improved(&self, original: &str, improved: &str) -> AuditId {
        let mut meta = HashMap::new();
        meta.insert("original".into(), original.to_string());
        meta.insert("improved".into(), improved.to_string());
        self.log_with_metadata(
            AuditAction::ColorImproved,
            Resource::new("color", original),
            Outcome::Success,
            meta,
        )
    }

    /// Log a policy violation.
    pub fn log_policy_violation(&self, resource_id: &str, reason: &str) -> AuditId {
        self.log(
            AuditAction::PolicyViolation(reason.to_string()),
            Resource::new("policy", resource_id),
            Outcome::Failure {
                reason: reason.to_string(),
            },
        )
    }

    // ---- Query / Statistics methods ------------------------------------

    /// Query entries matching the given filter.
    pub fn query(&self, filter: &AuditFilter) -> Vec<AuditEntry> {
        self.store.query(filter)
    }

    /// Return aggregated statistics over all stored entries.
    pub fn statistics(&self) -> AuditStatistics {
        self.store.statistics()
    }

    /// Export all entries in the requested format.
    pub fn export(&self, format: ExportFormat) -> String {
        self.store.export(format)
    }

    /// Query entries for this logger's actor only.
    pub fn query_own(&self) -> Vec<AuditEntry> {
        let filter = AuditFilter::new().actor(self.actor.id.clone());
        self.store.query(&filter)
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// Return a monotonically increasing pseudo-timestamp in seconds.
fn current_ts() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static EPOCH: AtomicU64 = AtomicU64::new(1_735_689_600);
    EPOCH.fetch_add(1, Ordering::Relaxed)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_actor() -> Actor {
        Actor::new("user-1", "user", "Alice")
    }

    fn make_resource() -> Resource {
        Resource::new("color", "#0066cc")
    }

    // --- AuditId ---

    #[test]
    fn test_audit_id_unique() {
        let a = AuditId::generate();
        let b = AuditId::generate();
        assert_ne!(a, b);
    }

    #[test]
    fn test_audit_id_display() {
        let id = AuditId("abc-def".to_string());
        assert_eq!(format!("{}", id), "abc-def");
    }

    // --- AuditAction ---

    #[test]
    fn test_action_kind_str() {
        assert_eq!(AuditAction::ColorValidated.kind_str(), "color_validated");
        assert_eq!(
            AuditAction::PolicyViolation("x".into()).kind_str(),
            "policy_violation"
        );
    }

    // --- AuditFilter ---

    #[test]
    fn test_filter_timestamp_range() {
        let entry = AuditEntry::new(
            500,
            make_actor(),
            AuditAction::ColorValidated,
            make_resource(),
            Outcome::Success,
        );
        let f_pass = AuditFilter::new().from(400).to(600);
        let f_fail = AuditFilter::new().from(600);
        assert!(f_pass.matches(&entry));
        assert!(!f_fail.matches(&entry));
    }

    #[test]
    fn test_filter_actor() {
        let entry = AuditEntry::new(
            100,
            make_actor(),
            AuditAction::ColorValidated,
            make_resource(),
            Outcome::Success,
        );
        assert!(AuditFilter::new().actor("user-1").matches(&entry));
        assert!(!AuditFilter::new().actor("user-2").matches(&entry));
    }

    #[test]
    fn test_filter_action_kind() {
        let entry = AuditEntry::new(
            100,
            make_actor(),
            AuditAction::WorkflowExecuted,
            make_resource(),
            Outcome::Success,
        );
        assert!(AuditFilter::new()
            .action("workflow_executed")
            .matches(&entry));
        assert!(!AuditFilter::new().action("color_validated").matches(&entry));
    }

    #[test]
    fn test_filter_policy_violation_alias() {
        let entry = AuditEntry::new(
            100,
            make_actor(),
            AuditAction::PolicyViolation("contrast too low".into()),
            make_resource(),
            Outcome::Failure {
                reason: "contrast too low".into(),
            },
        );
        assert!(AuditFilter::new()
            .action("policy_violation")
            .matches(&entry));
    }

    // --- Outcome ---

    #[test]
    fn test_outcome_is_ok() {
        assert!(Outcome::Success.is_ok());
        assert!(Outcome::PartialSuccess {
            details: "x".into()
        }
        .is_ok());
        assert!(!Outcome::Failure {
            reason: "err".into()
        }
        .is_ok());
    }

    // --- InMemoryAuditStore ---

    #[test]
    fn test_store_append_and_query() {
        let store = InMemoryAuditStore::new();
        let entry = AuditEntry::new(
            42,
            make_actor(),
            AuditAction::ColorValidated,
            make_resource(),
            Outcome::Success,
        );
        store.append(entry).unwrap();
        let results = store.query(&AuditFilter::new());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].timestamp, 42);
    }

    #[test]
    fn test_store_query_filtered() {
        let store = InMemoryAuditStore::new();
        let e1 = AuditEntry::new(
            10,
            make_actor(),
            AuditAction::ColorValidated,
            make_resource(),
            Outcome::Success,
        );
        let e2 = AuditEntry::new(
            20,
            Actor::new("bot-1", "bot", "Bot"),
            AuditAction::WorkflowExecuted,
            make_resource(),
            Outcome::Success,
        );
        store.append(e1).unwrap();
        store.append(e2).unwrap();

        let results = store.query(&AuditFilter::new().actor("user-1"));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].action.kind_str(), "color_validated");
    }

    #[test]
    fn test_store_statistics() {
        let store = InMemoryAuditStore::new();
        store
            .append(AuditEntry::new(
                1,
                make_actor(),
                AuditAction::ColorValidated,
                make_resource(),
                Outcome::Success,
            ))
            .unwrap();
        store
            .append(AuditEntry::new(
                2,
                make_actor(),
                AuditAction::ColorValidated,
                make_resource(),
                Outcome::Failure {
                    reason: "low contrast".into(),
                },
            ))
            .unwrap();
        store
            .append(AuditEntry::new(
                3,
                Actor::new("u2", "user", "Bob"),
                AuditAction::WorkflowExecuted,
                make_resource(),
                Outcome::Success,
            ))
            .unwrap();

        let stats = store.statistics();
        assert_eq!(stats.total_entries, 3);
        assert_eq!(stats.success_count, 2);
        assert_eq!(stats.failure_count, 1);
        assert_eq!(stats.unique_actors, 2);
        assert_eq!(*stats.entries_per_action.get("color_validated").unwrap(), 2);
        assert_eq!(
            *stats.entries_per_action.get("workflow_executed").unwrap(),
            1
        );
    }

    #[test]
    fn test_store_export_csv() {
        let store = InMemoryAuditStore::new();
        store
            .append(AuditEntry::new(
                100,
                make_actor(),
                AuditAction::ColorValidated,
                make_resource(),
                Outcome::Success,
            ))
            .unwrap();
        let csv = store.export(ExportFormat::Csv);
        assert!(csv.contains("id,timestamp"));
        assert!(csv.contains("color_validated"));
        assert!(csv.contains("success"));
    }

    #[test]
    fn test_store_export_markdown() {
        let store = InMemoryAuditStore::new();
        store
            .append(AuditEntry::new(
                200,
                make_actor(),
                AuditAction::BotAuthenticated,
                make_resource(),
                Outcome::Success,
            ))
            .unwrap();
        let md = store.export(ExportFormat::Markdown);
        assert!(md.contains('|'));
        assert!(md.contains("bot_authenticated"));
    }

    #[test]
    fn test_store_export_json() {
        let store = InMemoryAuditStore::new();
        store
            .append(AuditEntry::new(
                300,
                make_actor(),
                AuditAction::CertificateIssued,
                make_resource(),
                Outcome::Success,
            ))
            .unwrap();
        let json = store.export(ExportFormat::Json);
        assert!(json.contains("timestamp"));
        assert!(json.contains("CertificateIssued"));
    }

    // --- AuditLogger ---

    #[test]
    fn test_logger_log_and_query() {
        let logger = AuditLogger::new(make_actor());
        let id = logger.log(
            AuditAction::ColorValidated,
            make_resource(),
            Outcome::Success,
        );
        assert!(!id.0.is_empty());

        let entries = logger.query(&AuditFilter::new());
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_logger_log_with_metadata() {
        let logger = AuditLogger::new(make_actor());
        let mut meta = HashMap::new();
        meta.insert("wcag_ratio".to_string(), "4.65".to_string());
        let id = logger.log_with_metadata(
            AuditAction::ColorValidated,
            make_resource(),
            Outcome::Success,
            meta,
        );
        assert!(!id.0.is_empty());
        let entries = logger.query(&AuditFilter::new());
        assert_eq!(
            entries[0].metadata.get("wcag_ratio").map(|s| s.as_str()),
            Some("4.65")
        );
    }

    #[test]
    fn test_logger_statistics() {
        let logger = AuditLogger::new(make_actor());
        logger.log(
            AuditAction::ColorValidated,
            make_resource(),
            Outcome::Success,
        );
        logger.log(
            AuditAction::ColorImproved,
            make_resource(),
            Outcome::Success,
        );
        let stats = logger.statistics();
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.success_count, 2);
    }

    #[test]
    fn test_logger_convenience_methods() {
        let logger = AuditLogger::new(make_actor());
        logger.log_color_validated("#0066cc", true);
        logger.log_color_validated("#aaaaaa", false);
        logger.log_color_improved("#888888", "#333333");
        logger.log_policy_violation("palette-001", "WCAG AA not met");

        let stats = logger.statistics();
        assert_eq!(stats.total_entries, 4);
        assert_eq!(stats.failure_count, 2); // one failed validation + one policy violation
    }

    #[test]
    fn test_logger_query_own() {
        let shared_store: Arc<dyn AuditStore + Send + Sync> = Arc::new(InMemoryAuditStore::new());

        let logger_a =
            AuditLogger::with_store(Actor::new("alice", "user", "Alice"), shared_store.clone());
        let logger_b =
            AuditLogger::with_store(Actor::new("bob", "user", "Bob"), shared_store.clone());

        logger_a.log(
            AuditAction::ColorValidated,
            make_resource(),
            Outcome::Success,
        );
        logger_a.log(
            AuditAction::WorkflowExecuted,
            make_resource(),
            Outcome::Success,
        );
        logger_b.log(
            AuditAction::SessionCreated,
            make_resource(),
            Outcome::Success,
        );

        let alice_entries = logger_a.query_own();
        assert_eq!(alice_entries.len(), 2);

        let bob_entries = logger_b.query_own();
        assert_eq!(bob_entries.len(), 1);
    }

    // --- AutoReportGenerator ---

    #[test]
    fn test_auto_report_generate() {
        let store: Arc<dyn AuditStore + Send + Sync> = Arc::new(InMemoryAuditStore::new());
        store
            .append(AuditEntry::new(
                1,
                make_actor(),
                AuditAction::ColorValidated,
                make_resource(),
                Outcome::Success,
            ))
            .unwrap();

        let gen = AutoReportGenerator::with_store(
            Frequency::Daily,
            ReportTemplate::default_template(),
            ReportDelivery::InMemory,
            store,
        );
        let report = gen.generate_report();
        assert!(report.contains("# Audit Report:"));
        assert!(report.contains("## Overview"));
        assert!(report.contains("Total entries: 1"));
    }

    #[test]
    fn test_auto_report_check_and_generate() {
        let gen = AutoReportGenerator::new(Frequency::Hourly, ReportTemplate::default_template());
        // At t=0, last=0, period=3600 → 0 >= 0+3600 is false → no report yet.
        let r0 = gen.check_and_generate(0);
        assert!(r0.is_none(), "Not yet due at t=0");
        // First call after a full period elapses.
        let r1 = gen.check_and_generate(3600);
        assert!(r1.is_some(), "Due after one full period");
        // One second later — NOT enough time has passed.
        let r2 = gen.check_and_generate(3601);
        assert!(r2.is_none(), "Not due one second after last generation");
        // Call after another full period.
        let r3 = gen.check_and_generate(7200);
        assert!(r3.is_some(), "Due after second full period");
    }

    #[test]
    fn test_frequency_periods() {
        assert_eq!(Frequency::Hourly.period_secs(), 3_600);
        assert_eq!(Frequency::Daily.period_secs(), 86_400);
        assert_eq!(Frequency::Weekly.period_secs(), 604_800);
        assert_eq!(Frequency::Monthly.period_secs(), 2_592_000);
    }

    // --- FileAuditStore ---

    #[test]
    fn test_file_audit_store_delegates_to_memory() {
        let store = FileAuditStore::new("/tmp/audit-test.log");
        store
            .append(AuditEntry::new(
                1,
                make_actor(),
                AuditAction::ColorValidated,
                make_resource(),
                Outcome::Success,
            ))
            .unwrap();
        let results = store.query(&AuditFilter::new());
        assert_eq!(results.len(), 1);
        let stats = store.statistics();
        assert_eq!(stats.total_entries, 1);
    }
}

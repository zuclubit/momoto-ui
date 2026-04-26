//! Bot Automation API — Phase 10
//!
//! Provides structured interfaces for bot/LLM automation workflows,
//! batch operations, report scheduling, and workflow composition.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Primitive types
// ============================================================================

/// The kind of operation being requested.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryType {
    /// Validate a color against constraints.
    Validate,
    /// Recommend an optimal foreground color.
    Recommend,
    /// Score a foreground/background pair.
    Score,
    /// Improve an existing foreground color.
    Improve,
    /// Full OKLCH / luminance analysis.
    Analyze,
    /// Convert a color to another space.
    Convert,
    /// Run a batch of operations.
    Batch,
    /// Generate a structured report.
    GenerateReport,
    /// Execute a named or inline workflow.
    ExecuteWorkflow,
    /// Retrieve workflow execution status.
    GetStatus,
}

// ============================================================================
// Workflow spec types (distinct from crate::query::WorkflowSpec)
// ============================================================================

/// A single step inside a `WorkflowSpec`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStepSpec {
    /// Unique identifier for this step (used to express dependencies).
    pub step_id: String,
    /// The operation this step performs.
    pub query_type: QueryType,
    /// Key/value parameters for the operation.
    pub params: HashMap<String, String>,
    /// Step IDs that must complete before this step can start.
    pub depends_on: Vec<String>,
}

impl WorkflowStepSpec {
    /// Construct a new step.
    pub fn new(step_id: impl Into<String>, query_type: QueryType) -> Self {
        Self {
            step_id: step_id.into(),
            query_type,
            params: HashMap::new(),
            depends_on: Vec::new(),
        }
    }

    /// Add a key/value parameter.
    pub fn with_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.insert(key.into(), value.into());
        self
    }

    /// Declare a dependency on another step.
    pub fn depends_on(mut self, step_id: impl Into<String>) -> Self {
        self.depends_on.push(step_id.into());
        self
    }
}

/// A named workflow containing ordered or parallel steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSpec {
    /// Human-readable workflow name.
    pub name: String,
    /// Steps in this workflow.
    pub steps: Vec<WorkflowStepSpec>,
    /// When `true`, eligible steps run concurrently.
    pub parallel: bool,
}

impl WorkflowSpec {
    /// Create a new sequential workflow.
    pub fn sequential(name: impl Into<String>, steps: Vec<WorkflowStepSpec>) -> Self {
        Self {
            name: name.into(),
            steps,
            parallel: false,
        }
    }

    /// Create a new parallel-eligible workflow.
    pub fn parallel(name: impl Into<String>, steps: Vec<WorkflowStepSpec>) -> Self {
        Self {
            name: name.into(),
            steps,
            parallel: true,
        }
    }
}

// ============================================================================
// Workflow execution configuration
// ============================================================================

/// Runtime settings for workflow execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    /// Maximum wall-clock seconds allowed for the entire workflow.
    pub timeout_secs: u64,
    /// How many times to retry a failed step.
    pub max_retries: u32,
    /// Send a completion notification when done.
    pub notify_on_completion: bool,
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 300,
            max_retries: 3,
            notify_on_completion: false,
        }
    }
}

// ============================================================================
// Workflow status
// ============================================================================

/// Lifecycle state of a running or completed workflow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowStatusType {
    /// Queued but not yet started.
    Pending,
    /// Currently executing.
    Running,
    /// Finished successfully.
    Completed,
    /// Terminated with an error.
    Failed,
    /// Stopped before completion.
    Cancelled,
}

/// Full status snapshot for a workflow execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStatus {
    /// Unique workflow execution ID.
    pub workflow_id: String,
    /// Current lifecycle state.
    pub status_type: WorkflowStatusType,
    /// Completion fraction (0.0 – 1.0).
    pub progress: f64,
    /// Unix seconds when execution started.
    pub started_at: Option<u64>,
    /// Unix seconds when execution finished.
    pub completed_at: Option<u64>,
    /// Error description if `status_type == Failed`.
    pub error: Option<String>,
}

/// Final execution report for a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowReport {
    /// The workflow execution ID.
    pub workflow_id: String,
    /// Total steps declared.
    pub total_steps: u32,
    /// Steps that completed successfully.
    pub completed_steps: u32,
    /// Steps that failed.
    pub failed_steps: u32,
    /// Serialised results, one entry per step.
    pub results: Vec<String>,
    /// Wall-clock duration in milliseconds.
    pub elapsed_ms: u64,
}

// ============================================================================
// Batch operations
// ============================================================================

/// A bundle of heterogeneous operations to run together.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOperation {
    /// Client-assigned ID for correlating the response.
    pub operation_id: String,
    /// Pairs of `(query_type_str, params_json)`.
    pub operations: Vec<(String, String)>,
}

impl BatchOperation {
    /// Create a new batch with the given ID.
    pub fn new(operation_id: impl Into<String>) -> Self {
        Self {
            operation_id: operation_id.into(),
            operations: Vec::new(),
        }
    }

    /// Append an operation.
    pub fn add(mut self, query_type: &str, params_json: &str) -> Self {
        self.operations
            .push((query_type.to_string(), params_json.to_string()));
        self
    }
}

/// Result of executing a `BatchOperation`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResponse {
    /// Mirrors `BatchOperation::operation_id`.
    pub operation_id: String,
    /// Total number of operations in the batch.
    pub total: u32,
    /// Operations that completed without error.
    pub succeeded: u32,
    /// Operations that returned an error.
    pub failed: u32,
    /// Serialised results in input order.
    pub results: Vec<String>,
}

// ============================================================================
// Report scheduling
// ============================================================================

/// The kind of report to schedule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportType {
    /// Summarise color palette quality.
    ColorSummary,
    /// Full WCAG / APCA accessibility audit.
    AccessibilityAudit,
    /// Throughput and latency metrics.
    PerformanceMetrics,
    /// Regulatory-style compliance overview.
    ComplianceReport,
}

/// Recurring report schedule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSchedule {
    /// How often to generate a report (in hours).
    pub frequency_hours: u32,
    /// What type of report to generate.
    pub report_type: ReportType,
    /// Email addresses / webhook URLs to notify.
    pub recipients: Vec<String>,
    /// Output format string (e.g. `"markdown"`, `"json"`).
    pub format: String,
}

// ============================================================================
// Bot query / response
// ============================================================================

/// A structured query submitted by a bot session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotQuery {
    /// Client-assigned query ID.
    pub query_id: String,
    /// Bot session identifier.
    pub bot_session: String,
    /// Operation type.
    pub query_type: QueryType,
    /// Operation parameters.
    pub params: HashMap<String, String>,
    /// Optional inline workflow to execute.
    pub workflow: Option<WorkflowSpec>,
}

impl BotQuery {
    /// Construct a simple query without a workflow.
    pub fn new(
        query_id: impl Into<String>,
        bot_session: impl Into<String>,
        query_type: QueryType,
    ) -> Self {
        Self {
            query_id: query_id.into(),
            bot_session: bot_session.into(),
            query_type,
            params: HashMap::new(),
            workflow: None,
        }
    }

    /// Attach a key/value parameter.
    pub fn with_param(mut self, k: impl Into<String>, v: impl Into<String>) -> Self {
        self.params.insert(k.into(), v.into());
        self
    }

    /// Attach an inline workflow.
    pub fn with_workflow(mut self, w: WorkflowSpec) -> Self {
        self.workflow = Some(w);
        self
    }
}

/// Result returned for a `BotQuery`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotResponse {
    /// Mirrors `BotQuery::query_id`.
    pub query_id: String,
    /// `true` when the operation completed without errors.
    pub success: bool,
    /// JSON or text result.
    pub result: Option<String>,
    /// Error message when `success == false`.
    pub error: Option<String>,
    /// Wall-clock duration in milliseconds.
    pub elapsed_ms: u64,
}

impl BotResponse {
    fn ok(query_id: impl Into<String>, result: String, elapsed_ms: u64) -> Self {
        Self {
            query_id: query_id.into(),
            success: true,
            result: Some(result),
            error: None,
            elapsed_ms,
        }
    }

    fn err(query_id: impl Into<String>, error: String, elapsed_ms: u64) -> Self {
        Self {
            query_id: query_id.into(),
            success: false,
            result: None,
            error: Some(error),
            elapsed_ms,
        }
    }
}

// ============================================================================
// Workflow templates
// ============================================================================

/// A named, reusable parameter for a `WorkflowTemplate`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateParameter {
    /// Parameter name used for substitution (`{{name}}`).
    pub name: String,
    /// Human description shown in tooling.
    pub description: String,
    /// Whether the caller must supply this parameter.
    pub required: bool,
    /// Value used when the parameter is not supplied (only valid if `!required`).
    pub default_value: Option<String>,
}

impl TemplateParameter {
    /// Required parameter with no default.
    pub fn required(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            required: true,
            default_value: None,
        }
    }

    /// Optional parameter with a default value.
    pub fn optional(
        name: impl Into<String>,
        description: impl Into<String>,
        default: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            required: false,
            default_value: Some(default.into()),
        }
    }
}

/// A named, parameterised workflow template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTemplate {
    /// Template name (used as key in catalogues).
    pub name: String,
    /// Human description.
    pub description: String,
    /// The base workflow specification (may contain `{{param}}` placeholders).
    pub spec: WorkflowSpec,
    /// Parameters this template exposes.
    pub parameters: Vec<TemplateParameter>,
}

impl WorkflowTemplate {
    /// Instantiate the template with the given parameter values.
    ///
    /// Parameter substitution replaces `{{name}}` tokens in all `params`
    /// values of every step.
    pub fn instantiate(&self, params: &HashMap<String, String>) -> WorkflowSpec {
        let mut spec = self.spec.clone();

        // Build effective parameter map (defaults + caller overrides)
        let mut effective: HashMap<String, String> = HashMap::new();
        for tp in &self.parameters {
            if let Some(default) = &tp.default_value {
                effective.insert(tp.name.clone(), default.clone());
            }
        }
        effective.extend(params.iter().map(|(k, v)| (k.clone(), v.clone())));

        // Substitute {{name}} in every step parameter
        for step in &mut spec.steps {
            let new_params: HashMap<String, String> = step
                .params
                .iter()
                .map(|(k, v)| {
                    let mut value = v.clone();
                    for (pname, pval) in &effective {
                        value = value.replace(&format!("{{{{{}}}}}", pname), pval);
                    }
                    (k.clone(), value)
                })
                .collect();
            step.params = new_params;
        }

        spec
    }
}

// ============================================================================
// Workflow composition
// ============================================================================

/// A directed edge between two workflow steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    /// Source step ID.
    pub from_step: String,
    /// Target step ID.
    pub to_step: String,
    /// Optional guard expression (evaluated as a string predicate).
    pub condition: Option<String>,
}

/// Fluent builder for constructing `WorkflowSpec` instances.
#[derive(Debug, Clone, Default)]
pub struct WorkflowComposer {
    /// Steps added so far.
    pub steps: Vec<WorkflowStepSpec>,
    /// Explicit connections between steps.
    pub connections: Vec<Connection>,
}

impl WorkflowComposer {
    /// Create an empty composer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a step to the composition.
    pub fn add_step(&mut self, step: WorkflowStepSpec) -> &mut Self {
        self.steps.push(step);
        self
    }

    /// Add an unconditional directed edge `from → to`.
    pub fn connect(&mut self, from: &str, to: &str) -> &mut Self {
        self.connections.push(Connection {
            from_step: from.to_string(),
            to_step: to.to_string(),
            condition: None,
        });
        // Also register the dependency so topological sort works
        if let Some(step) = self.steps.iter_mut().find(|s| s.step_id == to) {
            if !step.depends_on.contains(&from.to_string()) {
                step.depends_on.push(from.to_string());
            }
        }
        self
    }

    /// Add a conditional directed edge `from → to` guarded by `condition`.
    pub fn connect_if(&mut self, from: &str, to: &str, condition: &str) -> &mut Self {
        self.connections.push(Connection {
            from_step: from.to_string(),
            to_step: to.to_string(),
            condition: Some(condition.to_string()),
        });
        if let Some(step) = self.steps.iter_mut().find(|s| s.step_id == to) {
            if !step.depends_on.contains(&from.to_string()) {
                step.depends_on.push(from.to_string());
            }
        }
        self
    }

    /// Consume the composer and produce a `WorkflowSpec` with topologically sorted steps.
    pub fn build(&self, name: &str) -> WorkflowSpec {
        let sorted = topological_sort(&self.steps);
        WorkflowSpec {
            name: name.to_string(),
            steps: sorted,
            parallel: false,
        }
    }
}

/// Kahn's algorithm for topological ordering of workflow steps.
fn topological_sort(steps: &[WorkflowStepSpec]) -> Vec<WorkflowStepSpec> {
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();

    for step in steps {
        in_degree.entry(step.step_id.clone()).or_insert(0);
        for dep in &step.depends_on {
            *in_degree.entry(step.step_id.clone()).or_insert(0) += 1;
            adjacency
                .entry(dep.clone())
                .or_default()
                .push(step.step_id.clone());
        }
    }

    let mut queue: Vec<String> = in_degree
        .iter()
        .filter(|(_, &d)| d == 0)
        .map(|(id, _)| id.clone())
        .collect();
    queue.sort(); // deterministic ordering

    let mut order: Vec<String> = Vec::new();
    while let Some(id) = queue.first().cloned() {
        queue.remove(0);
        order.push(id.clone());
        if let Some(dependents) = adjacency.get(&id) {
            for dep in dependents {
                let entry = in_degree.entry(dep.clone()).or_insert(1);
                *entry = entry.saturating_sub(1);
                if *entry == 0 {
                    queue.push(dep.clone());
                    queue.sort();
                }
            }
        }
    }

    // Preserve original steps in topological order; append any remainder
    let mut sorted: Vec<WorkflowStepSpec> = Vec::new();
    for id in &order {
        if let Some(s) = steps.iter().find(|s| &s.step_id == id) {
            sorted.push(s.clone());
        }
    }
    // Append any steps not reached (cycle-breaker — they run last)
    for s in steps {
        if !sorted.iter().any(|x| x.step_id == s.step_id) {
            sorted.push(s.clone());
        }
    }
    sorted
}

// ============================================================================
// Built-in workflow templates catalogue
// ============================================================================

fn builtin_templates() -> Vec<WorkflowTemplate> {
    vec![
        WorkflowTemplate {
            name: "accessibility_audit".to_string(),
            description: "Validate a foreground/background pair for WCAG AA compliance."
                .to_string(),
            spec: WorkflowSpec::sequential(
                "accessibility_audit",
                vec![
                    WorkflowStepSpec {
                        step_id: "step_validate".to_string(),
                        query_type: QueryType::Validate,
                        params: {
                            let mut m = HashMap::new();
                            m.insert("foreground".to_string(), "{{foreground}}".to_string());
                            m.insert("background".to_string(), "{{background}}".to_string());
                            m
                        },
                        depends_on: vec![],
                    },
                    WorkflowStepSpec {
                        step_id: "step_score".to_string(),
                        query_type: QueryType::Score,
                        params: {
                            let mut m = HashMap::new();
                            m.insert("foreground".to_string(), "{{foreground}}".to_string());
                            m.insert("background".to_string(), "{{background}}".to_string());
                            m
                        },
                        depends_on: vec!["step_validate".to_string()],
                    },
                ],
            ),
            parameters: vec![
                TemplateParameter::required("foreground", "Foreground hex color"),
                TemplateParameter::required("background", "Background hex color"),
            ],
        },
        WorkflowTemplate {
            name: "palette_analysis".to_string(),
            description: "Analyze a seed color and recommend an accessible foreground.".to_string(),
            spec: WorkflowSpec::sequential(
                "palette_analysis",
                vec![
                    WorkflowStepSpec {
                        step_id: "step_analyze".to_string(),
                        query_type: QueryType::Analyze,
                        params: {
                            let mut m = HashMap::new();
                            m.insert("color".to_string(), "{{color}}".to_string());
                            m
                        },
                        depends_on: vec![],
                    },
                    WorkflowStepSpec {
                        step_id: "step_recommend".to_string(),
                        query_type: QueryType::Recommend,
                        params: {
                            let mut m = HashMap::new();
                            m.insert("background".to_string(), "{{color}}".to_string());
                            m
                        },
                        depends_on: vec!["step_analyze".to_string()],
                    },
                ],
            ),
            parameters: vec![TemplateParameter::required("color", "Seed hex color")],
        },
        WorkflowTemplate {
            name: "batch_convert".to_string(),
            description: "Convert a list of colors to the target color space.".to_string(),
            spec: WorkflowSpec::parallel(
                "batch_convert",
                vec![WorkflowStepSpec {
                    step_id: "step_convert".to_string(),
                    query_type: QueryType::Convert,
                    params: {
                        let mut m = HashMap::new();
                        m.insert("color".to_string(), "{{color}}".to_string());
                        m.insert("target_space".to_string(), "{{target_space}}".to_string());
                        m
                    },
                    depends_on: vec![],
                }],
            ),
            parameters: vec![
                TemplateParameter::required("color", "Input hex color"),
                TemplateParameter::optional("target_space", "Target color space", "oklch"),
            ],
        },
    ]
}

// ============================================================================
// In-memory workflow store
// ============================================================================

#[derive(Debug, Default)]
struct WorkflowStore {
    statuses: HashMap<String, WorkflowStatus>,
    reports: HashMap<String, WorkflowReport>,
    schedules: HashMap<String, ReportSchedule>,
}

impl WorkflowStore {
    fn new() -> Self {
        Self::default()
    }

    fn insert_status(&mut self, status: WorkflowStatus) {
        self.statuses.insert(status.workflow_id.clone(), status);
    }

    fn get_status(&self, id: &str) -> Option<&WorkflowStatus> {
        self.statuses.get(id)
    }

    fn insert_schedule(&mut self, id: String, schedule: ReportSchedule) {
        self.schedules.insert(id, schedule);
    }
}

// ============================================================================
// BotAPI
// ============================================================================

/// Top-level bot automation API.
#[derive(Debug)]
pub struct BotAPI {
    store: WorkflowStore,
    session_counter: u64,
}

impl Default for BotAPI {
    fn default() -> Self {
        Self::new()
    }
}

impl BotAPI {
    /// Create a new `BotAPI` with empty state.
    pub fn new() -> Self {
        Self {
            store: WorkflowStore::new(),
            session_counter: 0,
        }
    }

    /// Submit a `BotQuery` and return a `BotResponse`.
    pub fn submit_query(&mut self, query: BotQuery) -> BotResponse {
        let start = std::time::Instant::now();
        let qid = query.query_id.clone();

        let result = match &query.query_type {
            QueryType::Validate => self.handle_validate(&query),
            QueryType::Recommend => self.handle_recommend(&query),
            QueryType::Score => self.handle_score(&query),
            QueryType::Improve => self.handle_improve(&query),
            QueryType::Analyze => self.handle_analyze(&query),
            QueryType::Convert => self.handle_convert(&query),
            QueryType::Batch => Ok(format!(
                "{{\"message\":\"use submit_batch for batch ops\"}}"
            )),
            QueryType::GenerateReport => self.handle_generate_report(&query),
            QueryType::ExecuteWorkflow => self.handle_execute_workflow(&query),
            QueryType::GetStatus => {
                let wid = query.params.get("workflow_id").cloned().unwrap_or_default();
                let status = self.get_workflow_status(&wid);
                Ok(serde_json::to_string(&status).unwrap_or_default())
            }
        };

        let elapsed_ms = start.elapsed().as_millis() as u64;
        match result {
            Ok(r) => BotResponse::ok(qid, r, elapsed_ms),
            Err(e) => BotResponse::err(qid, e, elapsed_ms),
        }
    }

    /// Retrieve current status for a workflow by ID.
    pub fn get_workflow_status(&self, workflow_id: &str) -> WorkflowStatus {
        self.store
            .get_status(workflow_id)
            .cloned()
            .unwrap_or(WorkflowStatus {
                workflow_id: workflow_id.to_string(),
                status_type: WorkflowStatusType::Pending,
                progress: 0.0,
                started_at: None,
                completed_at: None,
                error: Some("Workflow not found".to_string()),
            })
    }

    /// Execute a batch of heterogeneous operations.
    pub fn submit_batch(&mut self, batch: BatchOperation) -> BatchResponse {
        let total = batch.operations.len() as u32;
        let mut succeeded = 0u32;
        let mut failed = 0u32;
        let mut results: Vec<String> = Vec::with_capacity(batch.operations.len());

        for (idx, (qt_str, params_json)) in batch.operations.iter().enumerate() {
            let query_type = parse_query_type(qt_str);
            let params = parse_params_json(params_json);
            let query = BotQuery {
                query_id: format!("{}-{}", batch.operation_id, idx),
                bot_session: "batch".to_string(),
                query_type,
                params,
                workflow: None,
            };
            let resp = self.submit_query(query);
            if resp.success {
                succeeded += 1;
                results.push(resp.result.unwrap_or_default());
            } else {
                failed += 1;
                results.push(format!("ERROR: {}", resp.error.unwrap_or_default()));
            }
        }

        BatchResponse {
            operation_id: batch.operation_id,
            total,
            succeeded,
            failed,
            results,
        }
    }

    /// Return the built-in workflow templates.
    pub fn list_templates(&self) -> Vec<WorkflowTemplate> {
        builtin_templates()
    }

    /// Schedule a recurring report and return its ID.
    pub fn schedule_report(&mut self, schedule: ReportSchedule) -> String {
        self.session_counter += 1;
        let id = format!("sched-{:08x}", self.session_counter);
        self.store.insert_schedule(id.clone(), schedule);
        id
    }

    // ------------------------------------------------------------------
    // Private handlers
    // ------------------------------------------------------------------

    fn handle_validate(&self, query: &BotQuery) -> Result<String, String> {
        let color = query
            .params
            .get("color")
            .or_else(|| query.params.get("foreground"))
            .cloned()
            .unwrap_or_else(|| "#000000".to_string());
        let bg = query
            .params
            .get("background")
            .cloned()
            .unwrap_or_else(|| "#ffffff".to_string());

        use momoto_core::{color::Color, perception::ContrastMetric};
        use momoto_metrics::wcag::{TextSize, WCAGLevel, WCAGMetric};

        let fg_c = Color::from_hex(&color).unwrap_or_else(|_| Color::from_srgb8(0, 0, 0));
        let bg_c = Color::from_hex(&bg).unwrap_or_else(|_| Color::from_srgb8(255, 255, 255));
        let ratio = WCAGMetric.evaluate(fg_c, bg_c).value;
        let passes_aa = WCAGMetric::passes(ratio, WCAGLevel::AA, TextSize::Normal);

        Ok(serde_json::json!({
            "valid": passes_aa,
            "ratio": ratio,
            "level": if WCAGMetric::passes(ratio, WCAGLevel::AAA, TextSize::Normal) { "AAA" }
                     else if passes_aa { "AA" } else { "FAIL" },
            "foreground": color,
            "background": bg,
        })
        .to_string())
    }

    fn handle_recommend(&self, query: &BotQuery) -> Result<String, String> {
        let background = query
            .params
            .get("background")
            .cloned()
            .unwrap_or_else(|| "#ffffff".to_string());
        use momoto_core::{color::Color, luminance::relative_luminance_srgb};
        let bg = Color::from_hex(&background).unwrap_or_else(|_| Color::from_srgb8(255, 255, 255));
        let lum = relative_luminance_srgb(&bg).value();
        let recommended = if lum > 0.5 { "#000000" } else { "#ffffff" };
        Ok(serde_json::json!({
            "recommended": recommended,
            "background": background,
            "reason": if lum > 0.5 { "Dark text on light background" } else { "Light text on dark background" }
        }).to_string())
    }

    fn handle_score(&self, query: &BotQuery) -> Result<String, String> {
        let fg = query
            .params
            .get("foreground")
            .cloned()
            .unwrap_or_else(|| "#000000".to_string());
        let bg = query
            .params
            .get("background")
            .cloned()
            .unwrap_or_else(|| "#ffffff".to_string());

        use momoto_core::{color::Color, perception::ContrastMetric};
        use momoto_metrics::wcag::{TextSize, WCAGLevel, WCAGMetric};

        let fg_c = Color::from_hex(&fg).unwrap_or_else(|_| Color::from_srgb8(0, 0, 0));
        let bg_c = Color::from_hex(&bg).unwrap_or_else(|_| Color::from_srgb8(255, 255, 255));
        let ratio = WCAGMetric.evaluate(fg_c, bg_c).value;
        let score = (ratio / 21.0).clamp(0.0, 1.0);
        let passes = WCAGMetric::passes(ratio, WCAGLevel::AA, TextSize::Normal);

        Ok(serde_json::json!({
            "score": score,
            "ratio": ratio,
            "passes": passes,
            "foreground": fg,
            "background": bg,
        })
        .to_string())
    }

    fn handle_improve(&self, query: &BotQuery) -> Result<String, String> {
        let fg = query
            .params
            .get("foreground")
            .cloned()
            .unwrap_or_else(|| "#888888".to_string());
        let bg = query
            .params
            .get("background")
            .cloned()
            .unwrap_or_else(|| "#ffffff".to_string());

        use momoto_core::perception::ContrastMetric;
        use momoto_core::{color::Color, luminance::relative_luminance_srgb, space::oklch::OKLCH};
        use momoto_metrics::wcag::{TextSize, WCAGLevel, WCAGMetric};

        let bg_c = Color::from_hex(&bg).unwrap_or_else(|_| Color::from_srgb8(255, 255, 255));
        let bg_lum = relative_luminance_srgb(&bg_c).value();

        let fg_c = Color::from_hex(&fg).unwrap_or_else(|_| Color::from_srgb8(128, 128, 128));
        let mut oklch = OKLCH::from_color(&fg_c);

        // Binary-search lightness to achieve 4.5:1 contrast
        let target_ratio = 4.5_f64;
        for _ in 0..20 {
            let candidate = oklch.to_color();
            let ratio = WCAGMetric.evaluate(candidate, bg_c).value;
            if ratio >= target_ratio {
                break;
            }
            if bg_lum > 0.5 {
                oklch = oklch.darken(0.05);
            } else {
                oklch = oklch.lighten(0.05);
            }
        }

        let improved = oklch.to_color();
        let new_ratio = WCAGMetric.evaluate(improved, bg_c).value;
        let improved_hex = color_to_hex(&improved);
        let passes = WCAGMetric::passes(new_ratio, WCAGLevel::AA, TextSize::Normal);

        Ok(serde_json::json!({
            "original": fg,
            "improved": improved_hex,
            "ratio": new_ratio,
            "passes": passes,
            "background": bg,
        })
        .to_string())
    }

    fn handle_analyze(&self, query: &BotQuery) -> Result<String, String> {
        let color = query
            .params
            .get("color")
            .cloned()
            .unwrap_or_else(|| "#000000".to_string());
        let report = crate::reporting::ReportGenerator::analyze_color(&color);
        serde_json::to_string(&report).map_err(|e| e.to_string())
    }

    fn handle_convert(&self, query: &BotQuery) -> Result<String, String> {
        let color = query
            .params
            .get("color")
            .cloned()
            .unwrap_or_else(|| "#000000".to_string());
        let target = query
            .params
            .get("target_space")
            .map(|s| s.as_str())
            .unwrap_or("oklch");

        use momoto_core::{color::Color, space::oklch::OKLCH};

        let c = Color::from_hex(&color).unwrap_or_else(|_| Color::from_srgb8(0, 0, 0));
        match target {
            "oklch" => {
                let ok = OKLCH::from_color(&c);
                Ok(serde_json::json!({
                    "space": "oklch",
                    "color": color,
                    "L": ok.l,
                    "C": ok.c,
                    "H": ok.h,
                })
                .to_string())
            }
            "srgb" => Ok(serde_json::json!({
                "space": "srgb",
                "color": color,
                "r": (c.srgb[0] * 255.0).round() as u8,
                "g": (c.srgb[1] * 255.0).round() as u8,
                "b": (c.srgb[2] * 255.0).round() as u8,
            })
            .to_string()),
            other => Err(format!("Unknown target space: {}", other)),
        }
    }

    fn handle_generate_report(&self, query: &BotQuery) -> Result<String, String> {
        let colors_raw = query.params.get("colors").cloned().unwrap_or_default();
        let colors: Vec<String> = if colors_raw.is_empty() {
            vec![]
        } else {
            colors_raw
                .split(',')
                .map(|s| s.trim().to_string())
                .collect()
        };
        let color_refs: Vec<&str> = colors.iter().map(|s| s.as_str()).collect();
        let report = crate::reporting::ReportGenerator::generate_comprehensive(&color_refs, &[]);
        let format = query
            .params
            .get("format")
            .map(|s| s.as_str())
            .unwrap_or("json");
        let config = crate::reporting::ReportConfig {
            format: match format {
                "markdown" | "md" => crate::reporting::ReportFormat::Markdown,
                "html" => crate::reporting::ReportFormat::Html,
                "csv" => crate::reporting::ReportFormat::Csv,
                _ => crate::reporting::ReportFormat::Json,
            },
            ..Default::default()
        };
        let gen = crate::reporting::ReportGenerator::new(config);
        Ok(gen.render(&report))
    }

    fn handle_execute_workflow(&mut self, query: &BotQuery) -> Result<String, String> {
        let wf = match &query.workflow {
            Some(w) => w.clone(),
            None => {
                let name = query
                    .params
                    .get("workflow_name")
                    .cloned()
                    .unwrap_or_default();
                let templates = builtin_templates();
                match templates.into_iter().find(|t| t.name == name) {
                    Some(t) => t.instantiate(&query.params),
                    None => return Err(format!("Workflow '{}' not found", name)),
                }
            }
        };

        let workflow_id = format!("wf-{:08x}", self.next_id());
        let now = current_unix_secs();

        // Mark as running
        self.store.insert_status(WorkflowStatus {
            workflow_id: workflow_id.clone(),
            status_type: WorkflowStatusType::Running,
            progress: 0.0,
            started_at: Some(now),
            completed_at: None,
            error: None,
        });

        let total = wf.steps.len() as u32;
        let mut completed = 0u32;
        let mut results: Vec<String> = Vec::new();

        for step in &wf.steps {
            let step_query = BotQuery {
                query_id: format!("{}-{}", workflow_id, step.step_id),
                bot_session: query.bot_session.clone(),
                query_type: step.query_type.clone(),
                params: step.params.clone(),
                workflow: None,
            };
            let resp = self.submit_query(step_query);
            if resp.success {
                completed += 1;
                results.push(resp.result.unwrap_or_default());
            } else {
                results.push(format!("STEP_ERROR: {}", resp.error.unwrap_or_default()));
            }
        }

        let finished = current_unix_secs();
        let progress = if total == 0 {
            1.0
        } else {
            completed as f64 / total as f64
        };

        // Mark as completed
        self.store.insert_status(WorkflowStatus {
            workflow_id: workflow_id.clone(),
            status_type: WorkflowStatusType::Completed,
            progress,
            started_at: Some(now),
            completed_at: Some(finished),
            error: None,
        });

        let report = WorkflowReport {
            workflow_id: workflow_id.clone(),
            total_steps: total,
            completed_steps: completed,
            failed_steps: total - completed,
            results,
            elapsed_ms: (finished - now) * 1000,
        };

        serde_json::to_string(&report).map_err(|e| e.to_string())
    }

    fn next_id(&mut self) -> u64 {
        self.session_counter += 1;
        self.session_counter
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn parse_query_type(s: &str) -> QueryType {
    match s.to_lowercase().as_str() {
        "validate" => QueryType::Validate,
        "recommend" => QueryType::Recommend,
        "score" => QueryType::Score,
        "improve" => QueryType::Improve,
        "analyze" | "analyse" => QueryType::Analyze,
        "convert" => QueryType::Convert,
        "batch" => QueryType::Batch,
        "generatereport" | "generate_report" => QueryType::GenerateReport,
        "executeworkflow" | "execute_workflow" => QueryType::ExecuteWorkflow,
        "getstatus" | "get_status" => QueryType::GetStatus,
        _ => QueryType::Analyze,
    }
}

fn parse_params_json(json: &str) -> HashMap<String, String> {
    serde_json::from_str::<HashMap<String, serde_json::Value>>(json)
        .unwrap_or_default()
        .into_iter()
        .map(|(k, v)| {
            let val = match v {
                serde_json::Value::String(s) => s,
                other => other.to_string(),
            };
            (k, val)
        })
        .collect()
}

fn color_to_hex(color: &momoto_core::color::Color) -> String {
    let r = (color.srgb[0] * 255.0).round().clamp(0.0, 255.0) as u8;
    let g = (color.srgb[1] * 255.0).round().clamp(0.0, 255.0) as u8;
    let b = (color.srgb[2] * 255.0).round().clamp(0.0, 255.0) as u8;
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

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
    fn test_bot_api_validate_passes() {
        let mut api = BotAPI::new();
        let q = BotQuery::new("q1", "s1", QueryType::Validate)
            .with_param("foreground", "#000000")
            .with_param("background", "#ffffff");
        let resp = api.submit_query(q);
        assert!(resp.success);
        let val: serde_json::Value = serde_json::from_str(&resp.result.unwrap()).unwrap();
        assert_eq!(val["valid"], true);
    }

    #[test]
    fn test_bot_api_validate_fails() {
        let mut api = BotAPI::new();
        let q = BotQuery::new("q2", "s1", QueryType::Validate)
            .with_param("foreground", "#cccccc")
            .with_param("background", "#ffffff");
        let resp = api.submit_query(q);
        assert!(resp.success);
        let val: serde_json::Value = serde_json::from_str(&resp.result.unwrap()).unwrap();
        assert_eq!(val["valid"], false);
    }

    #[test]
    fn test_bot_api_analyze() {
        let mut api = BotAPI::new();
        let q = BotQuery::new("q3", "s1", QueryType::Analyze).with_param("color", "#0066cc");
        let resp = api.submit_query(q);
        assert!(resp.success);
        let val: serde_json::Value = serde_json::from_str(&resp.result.unwrap()).unwrap();
        assert!(val.get("oklch").is_some());
    }

    #[test]
    fn test_bot_api_convert_oklch() {
        let mut api = BotAPI::new();
        let q = BotQuery::new("q4", "s1", QueryType::Convert)
            .with_param("color", "#ff0000")
            .with_param("target_space", "oklch");
        let resp = api.submit_query(q);
        assert!(resp.success);
        let val: serde_json::Value = serde_json::from_str(&resp.result.unwrap()).unwrap();
        assert_eq!(val["space"], "oklch");
    }

    #[test]
    fn test_bot_api_convert_srgb() {
        let mut api = BotAPI::new();
        let q = BotQuery::new("q5", "s1", QueryType::Convert)
            .with_param("color", "#ff0000")
            .with_param("target_space", "srgb");
        let resp = api.submit_query(q);
        assert!(resp.success);
        let val: serde_json::Value = serde_json::from_str(&resp.result.unwrap()).unwrap();
        assert_eq!(val["r"], 255);
    }

    #[test]
    fn test_bot_api_improve() {
        let mut api = BotAPI::new();
        let q = BotQuery::new("q6", "s1", QueryType::Improve)
            .with_param("foreground", "#888888")
            .with_param("background", "#ffffff");
        let resp = api.submit_query(q);
        assert!(resp.success);
        let val: serde_json::Value = serde_json::from_str(&resp.result.unwrap()).unwrap();
        // Should produce a passing foreground
        assert_eq!(val["passes"], true);
    }

    #[test]
    fn test_batch_operation() {
        let mut api = BotAPI::new();
        let batch = BatchOperation::new("batch-001")
            .add(
                "validate",
                r##"{"foreground":"#000000","background":"#ffffff"}"##,
            )
            .add("analyze", r##"{"color":"#ff6600"}"##);
        let resp = api.submit_batch(batch);
        assert_eq!(resp.total, 2);
        assert_eq!(resp.operation_id, "batch-001");
    }

    #[test]
    fn test_workflow_template_instantiate() {
        let templates = builtin_templates();
        let tmpl = templates
            .iter()
            .find(|t| t.name == "accessibility_audit")
            .unwrap();
        let mut params = HashMap::new();
        params.insert("foreground".to_string(), "#000000".to_string());
        params.insert("background".to_string(), "#ffffff".to_string());
        let spec = tmpl.instantiate(&params);
        assert_eq!(spec.name, "accessibility_audit");
        assert_eq!(spec.steps.len(), 2);
        assert_eq!(spec.steps[0].params["foreground"], "#000000");
    }

    #[test]
    fn test_workflow_composer_topological_sort() {
        let mut composer = WorkflowComposer::new();
        composer.add_step(WorkflowStepSpec::new("c", QueryType::Score));
        composer.add_step(WorkflowStepSpec::new("a", QueryType::Analyze));
        composer.add_step(WorkflowStepSpec::new("b", QueryType::Validate));
        composer.connect("a", "b");
        composer.connect("b", "c");
        let spec = composer.build("test-workflow");
        // a must come before b, b before c
        let ids: Vec<&str> = spec.steps.iter().map(|s| s.step_id.as_str()).collect();
        let pos_a = ids.iter().position(|&x| x == "a").unwrap();
        let pos_b = ids.iter().position(|&x| x == "b").unwrap();
        let pos_c = ids.iter().position(|&x| x == "c").unwrap();
        assert!(pos_a < pos_b);
        assert!(pos_b < pos_c);
    }

    #[test]
    fn test_list_templates() {
        let api = BotAPI::new();
        let templates = api.list_templates();
        assert!(!templates.is_empty());
        assert!(templates.iter().any(|t| t.name == "accessibility_audit"));
        assert!(templates.iter().any(|t| t.name == "palette_analysis"));
    }

    #[test]
    fn test_schedule_report() {
        let mut api = BotAPI::new();
        let schedule = ReportSchedule {
            frequency_hours: 24,
            report_type: ReportType::AccessibilityAudit,
            recipients: vec!["team@example.com".to_string()],
            format: "markdown".to_string(),
        };
        let id = api.schedule_report(schedule);
        assert!(id.starts_with("sched-"));
    }

    #[test]
    fn test_execute_workflow_via_query() {
        let mut api = BotAPI::new();
        let wf = WorkflowSpec::sequential(
            "simple",
            vec![WorkflowStepSpec::new("step1", QueryType::Analyze).with_param("color", "#0066cc")],
        );
        let q = BotQuery::new("q-wf", "s1", QueryType::ExecuteWorkflow).with_workflow(wf);
        let resp = api.submit_query(q);
        assert!(resp.success);
        let val: serde_json::Value = serde_json::from_str(&resp.result.unwrap()).unwrap();
        assert_eq!(val["total_steps"], 1);
        assert_eq!(val["completed_steps"], 1);
    }

    #[test]
    fn test_get_workflow_status_not_found() {
        let api = BotAPI::new();
        let status = api.get_workflow_status("nonexistent");
        assert_eq!(status.status_type, WorkflowStatusType::Pending);
        assert!(status.error.is_some());
    }

    #[test]
    fn test_workflow_step_spec_builder() {
        let step = WorkflowStepSpec::new("s1", QueryType::Validate)
            .with_param("foreground", "#000")
            .depends_on("s0");
        assert_eq!(step.step_id, "s1");
        assert_eq!(step.params["foreground"], "#000");
        assert!(step.depends_on.contains(&"s0".to_string()));
    }

    #[test]
    fn test_template_parameter_required() {
        let tp = TemplateParameter::required("color", "A hex color");
        assert!(tp.required);
        assert!(tp.default_value.is_none());
    }

    #[test]
    fn test_template_parameter_optional() {
        let tp = TemplateParameter::optional("space", "Target space", "oklch");
        assert!(!tp.required);
        assert_eq!(tp.default_value, Some("oklch".to_string()));
    }
}

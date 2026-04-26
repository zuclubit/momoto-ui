//! # Intelligent Orchestration
//!
//! Provides scheduling, resource tracking, parallelization advice, and
//! adaptive feedback-loop primitives for coordinating complex, multi-step
//! color-processing workflows.

use serde::{Deserialize, Serialize};

// ============================================================================
// SchedulerConfig
// ============================================================================

/// Configuration for the [`IntelligentScheduler`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    /// Maximum number of steps that may run concurrently.
    pub max_parallel: usize,
    /// Hard timeout budget for all scheduled work, in milliseconds.
    pub timeout_ms: u64,
    /// Steps with an estimated cost above this threshold receive a priority boost.
    pub priority_boost_threshold: f64,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_parallel: 4,
            timeout_ms: 30_000,
            priority_boost_threshold: 0.75,
        }
    }
}

// ============================================================================
// PrioritizedStep
// ============================================================================

/// A unit of work that can be scheduled and executed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrioritizedStep {
    /// Unique identifier for this step.
    pub id: String,
    /// Higher values run first.
    pub priority: u32,
    /// Relative cost estimate in the range `[0.0, 1.0]`.
    pub estimated_cost: f64,
    /// IDs of steps that must complete before this one can start.
    pub dependencies: Vec<String>,
}

impl PrioritizedStep {
    /// Create a new step with no dependencies.
    pub fn new(id: impl Into<String>, priority: u32, estimated_cost: f64) -> Self {
        Self {
            id: id.into(),
            priority,
            estimated_cost: estimated_cost.clamp(0.0, 1.0),
            dependencies: Vec::new(),
        }
    }

    /// Add a dependency by step ID.
    pub fn with_dependency(mut self, dep_id: impl Into<String>) -> Self {
        self.dependencies.push(dep_id.into());
        self
    }

    /// Returns `true` when all dependency IDs appear in the `completed` set.
    pub fn dependencies_met(&self, completed: &[String]) -> bool {
        self.dependencies.iter().all(|dep| completed.contains(dep))
    }
}

// ============================================================================
// DeferReason
// ============================================================================

/// Reason why a step was deferred rather than scheduled immediately.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeferReason {
    /// Available CPU / concurrent slots are exhausted.
    ResourceLimit,
    /// One or more dependency steps have not yet completed.
    DependencyPending,
    /// The step's priority is below the minimum threshold for immediate execution.
    PriorityTooLow,
    /// A rate-limit or cooldown interval is active.
    RateLimited,
}

impl DeferReason {
    /// Human-readable explanation string.
    pub fn description(&self) -> &'static str {
        match self {
            Self::ResourceLimit => "Insufficient concurrent slots or CPU budget",
            Self::DependencyPending => {
                "One or more required predecessor steps are not yet complete"
            }
            Self::PriorityTooLow => "Step priority is below the current execution threshold",
            Self::RateLimited => "Rate limit or cooldown interval is active",
        }
    }
}

// ============================================================================
// SchedulingDecision
// ============================================================================

/// Output from [`IntelligentScheduler::schedule`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingDecision {
    /// Steps cleared for immediate execution.
    pub execute_now: Vec<PrioritizedStep>,
    /// Steps that must wait, paired with the reason for deferral.
    pub defer: Vec<(PrioritizedStep, DeferReason)>,
    /// Estimated wall-clock milliseconds until all work is complete.
    pub estimated_completion_ms: u64,
}

impl SchedulingDecision {
    /// Returns `true` if there is at least one step ready to execute.
    pub fn has_work(&self) -> bool {
        !self.execute_now.is_empty()
    }

    /// Total number of steps (ready + deferred).
    pub fn total_steps(&self) -> usize {
        self.execute_now.len() + self.defer.len()
    }
}

// ============================================================================
// ResourceAvailability
// ============================================================================

/// Snapshot of currently available compute resources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAvailability {
    /// Normalised CPU capacity available `[0.0, 1.0]`.
    pub cpu_available: f64,
    /// Free memory in megabytes.
    pub memory_mb: usize,
    /// Number of concurrent execution slots that may be occupied.
    pub concurrent_slots: usize,
}

impl Default for ResourceAvailability {
    fn default() -> Self {
        Self {
            cpu_available: 1.0,
            memory_mb: 512,
            concurrent_slots: 4,
        }
    }
}

// ============================================================================
// ResourceConstraints
// ============================================================================

/// Upper bounds on resources that the scheduler must respect.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConstraints {
    /// Maximum CPU fraction that may be consumed `[0.0, 1.0]`.
    pub max_cpu: f64,
    /// Maximum memory in megabytes that may be used.
    pub max_memory_mb: usize,
    /// Maximum number of concurrently executing steps.
    pub max_concurrent: usize,
}

impl Default for ResourceConstraints {
    fn default() -> Self {
        Self {
            max_cpu: 0.90,
            max_memory_mb: 256,
            max_concurrent: 4,
        }
    }
}

// ============================================================================
// ResourceTracker
// ============================================================================

/// Tracks occupied resource slots and arbitrates acquisition requests.
#[derive(Debug, Clone)]
pub struct ResourceTracker {
    constraints: ResourceConstraints,
    occupied_slots: usize,
    cpu_used: f64,
}

impl ResourceTracker {
    /// Create a tracker governed by the supplied constraints.
    pub fn new(constraints: ResourceConstraints) -> Self {
        Self {
            constraints,
            occupied_slots: 0,
            cpu_used: 0.0,
        }
    }

    /// Create a tracker with default constraints.
    pub fn default() -> Self {
        Self::new(ResourceConstraints::default())
    }

    /// Return a snapshot of currently available resources.
    pub fn available(&self) -> ResourceAvailability {
        ResourceAvailability {
            cpu_available: (self.constraints.max_cpu - self.cpu_used).max(0.0),
            memory_mb: self.constraints.max_memory_mb,
            concurrent_slots: self
                .constraints
                .max_concurrent
                .saturating_sub(self.occupied_slots),
        }
    }

    /// Attempt to acquire one execution slot.
    ///
    /// Returns `true` on success, `false` if resource limits would be exceeded.
    pub fn acquire(&mut self) -> bool {
        if self.occupied_slots >= self.constraints.max_concurrent {
            return false;
        }
        let cost_per_slot = 1.0 / self.constraints.max_concurrent as f64;
        if self.cpu_used + cost_per_slot > self.constraints.max_cpu + 1e-9 {
            return false;
        }
        self.occupied_slots += 1;
        self.cpu_used += cost_per_slot;
        true
    }

    /// Release one previously acquired execution slot.
    pub fn release(&mut self) {
        if self.occupied_slots > 0 {
            self.occupied_slots -= 1;
            let cost_per_slot = 1.0 / self.constraints.max_concurrent as f64;
            self.cpu_used = (self.cpu_used - cost_per_slot).max(0.0);
        }
    }

    /// Number of currently occupied slots.
    pub fn occupied(&self) -> usize {
        self.occupied_slots
    }
}

// ============================================================================
// IntelligentScheduler
// ============================================================================

/// Decides which steps to execute immediately and which to defer, based on
/// priorities, resource availability, and dependency relationships.
#[derive(Debug, Clone)]
pub struct IntelligentScheduler {
    config: SchedulerConfig,
}

impl IntelligentScheduler {
    /// Create a scheduler with the given configuration.
    pub fn new(config: SchedulerConfig) -> Self {
        Self { config }
    }

    /// Create a scheduler with default configuration.
    pub fn default() -> Self {
        Self::new(SchedulerConfig::default())
    }

    /// Schedule a batch of steps given the current resource snapshot.
    ///
    /// Steps are sorted by descending priority.  Steps that exceed available
    /// slots or have unmet dependencies are deferred.  High-cost steps receive
    /// an automatic priority boost when their cost exceeds
    /// `config.priority_boost_threshold`.
    pub fn schedule(
        &self,
        steps: Vec<PrioritizedStep>,
        resources: &ResourceAvailability,
    ) -> SchedulingDecision {
        // Sort by (boosted) priority descending
        let mut sorted = steps;
        let boost_threshold = self.config.priority_boost_threshold;
        sorted.sort_by(|a, b| {
            let pa = Self::effective_priority(a, boost_threshold);
            let pb = Self::effective_priority(b, boost_threshold);
            pb.cmp(&pa)
        });

        let mut execute_now: Vec<PrioritizedStep> = Vec::new();
        let mut defer: Vec<(PrioritizedStep, DeferReason)> = Vec::new();
        let mut slots_used = 0usize;
        let completed_ids: Vec<String> = Vec::new(); // no prior knowledge here

        for step in sorted {
            // Check dependencies
            if !step.dependencies_met(&completed_ids) {
                defer.push((step, DeferReason::DependencyPending));
                continue;
            }
            // Check slot availability
            if slots_used >= resources.concurrent_slots.min(self.config.max_parallel) {
                defer.push((step, DeferReason::ResourceLimit));
                continue;
            }
            // Check if priority is acceptable (priority 0 = minimal)
            if step.priority == 0 && execute_now.len() >= 1 {
                defer.push((step, DeferReason::PriorityTooLow));
                continue;
            }
            slots_used += 1;
            execute_now.push(step);
        }

        // Estimate completion: assume each slot runs sequentially if parallel disabled
        let slot_count = execute_now.len().max(1);
        let avg_cost =
            execute_now.iter().map(|s| s.estimated_cost).sum::<f64>() / slot_count as f64;
        let estimated_completion_ms = (avg_cost * self.config.timeout_ms as f64) as u64;

        SchedulingDecision {
            execute_now,
            defer,
            estimated_completion_ms,
        }
    }

    fn effective_priority(step: &PrioritizedStep, boost_threshold: f64) -> u32 {
        if step.estimated_cost > boost_threshold {
            step.priority.saturating_add(10)
        } else {
            step.priority
        }
    }
}

// ============================================================================
// ExecutionStrategy
// ============================================================================

/// High-level execution strategy recommended by [`ParallelizationAdvisor`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStrategy {
    /// Run all steps one after another.
    Sequential,
    /// Run all independent steps concurrently.
    Parallel,
    /// Automatically determine the optimal mix based on dependency structure.
    Adaptive,
}

impl ExecutionStrategy {
    /// Human-readable description.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Sequential => "All steps execute in order, one at a time",
            Self::Parallel => "All independent steps execute concurrently",
            Self::Adaptive => "Mixed strategy derived from dependency graph analysis",
        }
    }
}

// ============================================================================
// ParallelGroup
// ============================================================================

/// A set of steps that may safely execute in parallel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelGroup {
    /// Steps in this group (no intra-group dependencies).
    pub steps: Vec<PrioritizedStep>,
    /// Limit on simultaneous execution within this group.
    pub max_parallel: usize,
}

impl ParallelGroup {
    /// Create a group with an explicit concurrency cap.
    pub fn new(steps: Vec<PrioritizedStep>, max_parallel: usize) -> Self {
        Self {
            steps,
            max_parallel,
        }
    }

    /// Estimated total cost of the group (sum of individual costs).
    pub fn total_cost(&self) -> f64 {
        self.steps.iter().map(|s| s.estimated_cost).sum()
    }
}

// ============================================================================
// ParallelizationAdvisor
// ============================================================================

/// Analyses a set of steps and recommends an execution strategy.
#[derive(Debug, Clone)]
pub struct ParallelizationAdvisor {
    /// Minimum number of independent steps needed to recommend [`ExecutionStrategy::Parallel`].
    pub min_parallel_threshold: usize,
}

impl ParallelizationAdvisor {
    /// Create an advisor with the default threshold (2 independent steps).
    pub fn new() -> Self {
        Self {
            min_parallel_threshold: 2,
        }
    }

    /// Recommend an [`ExecutionStrategy`] for the supplied steps.
    ///
    /// # Strategy selection
    /// - Empty or single step → `Sequential`
    /// - All independent, ≥ threshold → `Parallel`
    /// - Mix of independent and dependent → `Adaptive`
    /// - Too few independent to parallelise → `Sequential`
    pub fn recommend(&self, steps: &[PrioritizedStep]) -> ExecutionStrategy {
        if steps.len() <= 1 {
            return ExecutionStrategy::Sequential;
        }

        let has_deps = steps.iter().any(|s| !s.dependencies.is_empty());
        let independent_count = steps.iter().filter(|s| s.dependencies.is_empty()).count();

        if has_deps {
            // Any mix of independent + dependent steps → Adaptive
            if independent_count >= 1 {
                ExecutionStrategy::Adaptive
            } else {
                ExecutionStrategy::Sequential
            }
        } else if independent_count >= self.min_parallel_threshold {
            ExecutionStrategy::Parallel
        } else {
            ExecutionStrategy::Sequential
        }
    }

    /// Partition `steps` into [`ParallelGroup`]s respecting dependencies.
    ///
    /// A topological wave decomposition: each wave contains all steps whose
    /// dependencies are fully satisfied by previous waves.
    pub fn group_parallel(&self, steps: Vec<PrioritizedStep>) -> Vec<ParallelGroup> {
        let mut remaining: Vec<PrioritizedStep> = steps;
        let mut completed: Vec<String> = Vec::new();
        let mut groups: Vec<ParallelGroup> = Vec::new();

        while !remaining.is_empty() {
            // Collect all steps whose deps are met in this wave
            let (ready, not_ready): (Vec<_>, Vec<_>) = remaining
                .into_iter()
                .partition(|s| s.dependencies_met(&completed));

            if ready.is_empty() {
                // Circular dependency or unresolvable — push remainder as a
                // single sequential group to avoid an infinite loop.
                let max_p = not_ready.len();
                groups.push(ParallelGroup::new(not_ready, 1.max(max_p)));
                break;
            }

            // Mark ready steps as completed
            for s in &ready {
                completed.push(s.id.clone());
            }

            let max_p = ready.len();
            groups.push(ParallelGroup::new(ready, max_p));
            remaining = not_ready;
        }
        groups
    }
}

impl Default for ParallelizationAdvisor {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// ConvergenceStatus
// ============================================================================

/// Convergence state reported by a [`FeedbackLoop`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConvergenceStatus {
    /// The loop has converged within tolerance.
    Converged {
        /// Number of iterations required to converge.
        iterations: u32,
    },
    /// The loop failed to converge or diverged.
    Diverged {
        /// Human-readable explanation.
        reason: String,
    },
    /// Iteration is still in progress.
    InProgress {
        /// Current iteration index (0-based).
        iteration: u32,
    },
}

impl ConvergenceStatus {
    /// Returns `true` if the status is [`ConvergenceStatus::Converged`].
    pub fn is_converged(&self) -> bool {
        matches!(self, Self::Converged { .. })
    }
}

// ============================================================================
// FeedbackConfig
// ============================================================================

/// Configuration parameters for a [`FeedbackLoop`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackConfig {
    /// Maximum number of iterations before reporting divergence.
    pub max_iterations: u32,
    /// Absolute improvement below which the loop is considered converged.
    pub convergence_threshold: f64,
    /// Step size for gradient-descent-style updates `(0.0, 1.0]`.
    pub learning_rate: f64,
}

impl Default for FeedbackConfig {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            convergence_threshold: 1e-4,
            learning_rate: 0.1,
        }
    }
}

// ============================================================================
// IterationResult
// ============================================================================

/// Outcome of a single feedback-loop iteration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IterationResult {
    /// 0-based iteration index.
    pub iteration: u32,
    /// Absolute improvement achieved in this iteration.
    pub improvement: f64,
    /// `true` if the improvement fell below the convergence threshold.
    pub converged: bool,
}

// ============================================================================
// FeedbackLoop
// ============================================================================

/// Gradient-descent-style feedback loop for iterative refinement.
///
/// Tracks iteration history and detects convergence or divergence based on
/// the improvement signal passed to [`FeedbackLoop::iterate`].
#[derive(Debug, Clone)]
pub struct FeedbackLoop {
    config: FeedbackConfig,
    history: Vec<IterationResult>,
    current_value: f64,
    diverged: bool,
}

impl FeedbackLoop {
    /// Create a new feedback loop.
    pub fn new(config: FeedbackConfig) -> Self {
        Self {
            config,
            history: Vec::new(),
            current_value: f64::NAN,
            diverged: false,
        }
    }

    /// Perform one iteration step, moving `current` toward `target`.
    ///
    /// Returns the [`IterationResult`] for this step.
    pub fn iterate(&mut self, current: f64, target: f64) -> IterationResult {
        let iteration = self.history.len() as u32;

        // Gradient step
        let error = target - current;
        let next = current + self.config.learning_rate * error;
        let improvement = (error - (target - next)).abs();

        // Detect divergence: if next is further away than current
        let diverging = error.abs() > 0.0 && (target - next).abs() > error.abs() * 1.5;
        if diverging {
            self.diverged = true;
        }

        let converged = improvement.abs() < self.config.convergence_threshold;
        self.current_value = next;

        let result = IterationResult {
            iteration,
            improvement,
            converged,
        };
        self.history.push(result.clone());
        result
    }

    /// Return the current convergence status.
    pub fn status(&self) -> ConvergenceStatus {
        if self.diverged {
            return ConvergenceStatus::Diverged {
                reason: "Step produced a larger error than the previous estimate".to_string(),
            };
        }
        if let Some(last) = self.history.last() {
            if last.converged {
                return ConvergenceStatus::Converged {
                    iterations: self.history.len() as u32,
                };
            }
            if self.history.len() as u32 >= self.config.max_iterations {
                return ConvergenceStatus::Diverged {
                    reason: format!(
                        "Maximum iterations ({}) reached without convergence",
                        self.config.max_iterations
                    ),
                };
            }
            ConvergenceStatus::InProgress {
                iteration: last.iteration,
            }
        } else {
            ConvergenceStatus::InProgress { iteration: 0 }
        }
    }

    /// Number of iterations performed so far.
    pub fn iteration_count(&self) -> u32 {
        self.history.len() as u32
    }

    /// Full iteration history.
    pub fn history(&self) -> &[IterationResult] {
        &self.history
    }

    /// Reset the loop to initial state.
    pub fn reset(&mut self) {
        self.history.clear();
        self.current_value = f64::NAN;
        self.diverged = false;
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- SchedulerConfig ----

    #[test]
    fn test_scheduler_config_defaults() {
        let cfg = SchedulerConfig::default();
        assert!(cfg.max_parallel > 0);
        assert!(cfg.timeout_ms > 0);
        assert!(cfg.priority_boost_threshold > 0.0);
    }

    // ---- PrioritizedStep ----

    #[test]
    fn test_prioritized_step_deps_met() {
        let step = PrioritizedStep::new("b", 5, 0.3).with_dependency("a");
        assert!(!step.dependencies_met(&["x".to_string()]));
        assert!(step.dependencies_met(&["a".to_string(), "x".to_string()]));
    }

    #[test]
    fn test_prioritized_step_no_deps() {
        let step = PrioritizedStep::new("x", 10, 0.5);
        assert!(step.dependencies_met(&[]));
    }

    // ---- DeferReason ----

    #[test]
    fn test_defer_reason_descriptions_non_empty() {
        for reason in &[
            DeferReason::ResourceLimit,
            DeferReason::DependencyPending,
            DeferReason::PriorityTooLow,
            DeferReason::RateLimited,
        ] {
            assert!(!reason.description().is_empty());
        }
    }

    // ---- ResourceTracker ----

    #[test]
    fn test_resource_tracker_acquire_release() {
        let mut tracker = ResourceTracker::new(ResourceConstraints {
            max_concurrent: 2,
            max_cpu: 1.0,
            max_memory_mb: 256,
        });
        assert!(tracker.acquire());
        assert!(tracker.acquire());
        assert!(!tracker.acquire()); // third slot denied
        tracker.release();
        assert!(tracker.acquire()); // slot restored
        assert_eq!(tracker.occupied(), 2);
    }

    #[test]
    fn test_resource_tracker_available() {
        let mut tracker = ResourceTracker::default();
        let before = tracker.available();
        tracker.acquire();
        let after = tracker.available();
        assert!(after.concurrent_slots < before.concurrent_slots);
    }

    // ---- IntelligentScheduler ----

    #[test]
    fn test_scheduler_basic() {
        let scheduler = IntelligentScheduler::default();
        let resources = ResourceAvailability {
            concurrent_slots: 3,
            ..Default::default()
        };
        let steps = vec![
            PrioritizedStep::new("a", 10, 0.2),
            PrioritizedStep::new("b", 5, 0.4),
            PrioritizedStep::new("c", 1, 0.1),
        ];
        let decision = scheduler.schedule(steps, &resources);
        // Highest priority goes first
        assert!(!decision.execute_now.is_empty());
        assert_eq!(decision.execute_now[0].id, "a");
    }

    #[test]
    fn test_scheduler_respects_slot_limit() {
        let cfg = SchedulerConfig {
            max_parallel: 1,
            ..Default::default()
        };
        let scheduler = IntelligentScheduler::new(cfg);
        let resources = ResourceAvailability {
            concurrent_slots: 1,
            ..Default::default()
        };
        let steps = vec![
            PrioritizedStep::new("a", 10, 0.5),
            PrioritizedStep::new("b", 9, 0.5),
        ];
        let decision = scheduler.schedule(steps, &resources);
        assert_eq!(decision.execute_now.len(), 1);
        assert_eq!(decision.defer.len(), 1);
    }

    #[test]
    fn test_scheduler_defers_unmet_deps() {
        let scheduler = IntelligentScheduler::default();
        let resources = ResourceAvailability::default();
        let steps = vec![PrioritizedStep::new("b", 10, 0.5).with_dependency("a")];
        let decision = scheduler.schedule(steps, &resources);
        assert_eq!(decision.execute_now.len(), 0);
        assert_eq!(decision.defer.len(), 1);
        assert_eq!(decision.defer[0].1, DeferReason::DependencyPending);
    }

    // ---- ParallelizationAdvisor ----

    #[test]
    fn test_advisor_sequential_single() {
        let advisor = ParallelizationAdvisor::new();
        let steps = vec![PrioritizedStep::new("a", 1, 0.5)];
        assert_eq!(advisor.recommend(&steps), ExecutionStrategy::Sequential);
    }

    #[test]
    fn test_advisor_parallel_independent() {
        let advisor = ParallelizationAdvisor::new();
        let steps = vec![
            PrioritizedStep::new("a", 5, 0.3),
            PrioritizedStep::new("b", 3, 0.3),
        ];
        assert_eq!(advisor.recommend(&steps), ExecutionStrategy::Parallel);
    }

    #[test]
    fn test_advisor_adaptive_mixed() {
        let advisor = ParallelizationAdvisor::new();
        let steps = vec![
            PrioritizedStep::new("a", 5, 0.3),
            PrioritizedStep::new("b", 3, 0.3).with_dependency("a"),
        ];
        // One independent + one dependent → Adaptive
        assert_eq!(advisor.recommend(&steps), ExecutionStrategy::Adaptive);
    }

    #[test]
    fn test_advisor_group_parallel_waves() {
        let advisor = ParallelizationAdvisor::new();
        let steps = vec![
            PrioritizedStep::new("a", 10, 0.2),
            PrioritizedStep::new("b", 10, 0.2),
            PrioritizedStep::new("c", 5, 0.3).with_dependency("a"),
        ];
        let groups = advisor.group_parallel(steps);
        assert!(groups.len() >= 2, "Expected at least 2 waves");
        // Wave 1: a, b (no deps)
        let wave1_ids: Vec<&str> = groups[0].steps.iter().map(|s| s.id.as_str()).collect();
        assert!(wave1_ids.contains(&"a"));
        assert!(wave1_ids.contains(&"b"));
        // Wave 2: c (dep on a satisfied)
        let wave2_ids: Vec<&str> = groups[1].steps.iter().map(|s| s.id.as_str()).collect();
        assert!(wave2_ids.contains(&"c"));
    }

    // ---- FeedbackLoop ----

    #[test]
    fn test_feedback_loop_converges() {
        let mut fb = FeedbackLoop::new(FeedbackConfig {
            max_iterations: 200,
            convergence_threshold: 1e-3,
            learning_rate: 0.5,
        });

        let target = 1.0_f64;
        let mut current = 0.0_f64;

        for _ in 0..200 {
            let res = fb.iterate(current, target);
            current = current + 0.5 * (target - current);
            if res.converged {
                break;
            }
        }

        assert!(fb.status().is_converged(), "Expected convergence");
    }

    #[test]
    fn test_feedback_loop_max_iterations() {
        let mut fb = FeedbackLoop::new(FeedbackConfig {
            max_iterations: 3,
            convergence_threshold: 1e-10, // very tight — won't converge in 3 iters
            learning_rate: 0.01,
        });
        for _ in 0..3 {
            fb.iterate(0.0, 10.0);
        }
        // Should diverge (hit max iterations)
        let status = fb.status();
        assert!(matches!(status, ConvergenceStatus::Diverged { .. }));
    }

    #[test]
    fn test_feedback_loop_reset() {
        let mut fb = FeedbackLoop::new(FeedbackConfig::default());
        fb.iterate(0.0, 1.0);
        assert_eq!(fb.iteration_count(), 1);
        fb.reset();
        assert_eq!(fb.iteration_count(), 0);
        assert!(matches!(fb.status(), ConvergenceStatus::InProgress { .. }));
    }

    #[test]
    fn test_scheduling_decision_helpers() {
        let decision = SchedulingDecision {
            execute_now: vec![PrioritizedStep::new("x", 1, 0.5)],
            defer: vec![],
            estimated_completion_ms: 100,
        };
        assert!(decision.has_work());
        assert_eq!(decision.total_steps(), 1);
    }

    #[test]
    fn test_parallel_group_total_cost() {
        let group = ParallelGroup::new(
            vec![
                PrioritizedStep::new("a", 1, 0.3),
                PrioritizedStep::new("b", 1, 0.4),
            ],
            2,
        );
        assert!((group.total_cost() - 0.7).abs() < 1e-9);
    }
}

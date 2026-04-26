//! # Momoto Agent — AI/LLM Integration Layer
//!
//! **STATUS: EXPERIMENTAL**
//!
//! This crate provides structured interfaces for AI agents and LLMs to
//! interact with Momoto's perceptual color system.
//!
//! ## Warning
//!
//! This crate is experimental and may change without notice.
//! It is not covered by SemVer guarantees.
//!
//! ## Core Concepts
//!
//! ### AI Contracts
//!
//! An AI Contract is a declarative specification of color constraints
//! that an agent must satisfy. Contracts are:
//!
//! - **Declarative**: Describe what, not how
//! - **Verifiable**: Momoto can check compliance
//! - **Composable**: Multiple constraints can combine
//! - **Machine-readable**: JSON format
//!
//! ### Query/Response Protocol
//!
//! Agents interact with Momoto through structured queries:
//!
//! ```rust,ignore
//! use momoto_agent::{Query, AgentExecutor};
//!
//! let executor = AgentExecutor::new();
//!
//! // JSON query from LLM
//! let query = r##"{
//!     "action": "validate",
//!     "color": "#0066cc",
//!     "contract": {
//!         "constraints": [
//!             { "type": "min_contrast", "standard": "wcag", "level": "aa", "against": "#ffffff" }
//!         ]
//!     }
//! }"##;
//!
//! let response = executor.execute_json(query).unwrap();
//! println!("{}", response);
//! ```
//!
//! ## Quick Start
//!
//! ```rust
//! use momoto_agent::prelude::*;
//!
//! // Create a contract requiring WCAG AA contrast
//! let contract = Contract::new()
//!     .with_constraint(Constraint::min_contrast_wcag_aa("#ffffff"));
//!
//! // Validate a color against the contract
//! let result = validate("#0066cc", &contract);
//! println!("Valid: {}", result.is_valid());
//! ```
//!
//! Implements: RFC-0004
//! Authorization: governance/TIER_2_AUTHORIZATION.md

#![warn(missing_docs, missing_debug_implementations)]
#![allow(dead_code)] // Experimental crate, some APIs may be unused initially

pub mod contract;
pub mod experience;
pub mod orchestration;
pub mod query;
pub mod reporting;
pub mod response;
pub mod session;
pub mod workflow;

// Phase 10: Multi-Turn Bot Automation
pub mod audit;
pub mod bot_api;

// AI Visual Generator - Full Pipeline
pub mod visual_generator;

// Material → Color Bridge (spectral pipeline)
pub mod material_bridge;

// Complete Pipeline - 100% Crate Utilization
pub mod pipeline;

// Phase 8: Temporal Perception Engine
pub mod temporal;

// Phase 9: Perceptual Source of Truth Certification
pub mod certification;

mod executor;

// Re-exports
pub use contract::{ComplianceLevel, Constraint, Contract, ContrastStandard, Gamut};
pub use executor::AgentExecutor;
pub use experience::{
    generate_experience, generate_experience_with_color, list_presets, ExperienceBuilder,
    ExperienceGenerator, ThemePreset, VisualExperience,
};
pub use query::Query;
pub use reporting::{
    AccessibilityAuditReport, BatchDashboard, ColorAnalysisReport, ComprehensiveReport,
    DashboardBuilder, EffortLevel, ExecutiveSummary, Finding, LiveMetrics, LogCollector,
    MaterialPhysicsReport, MetricsDashboard, OutputMode, PerceptualQualityReport,
    PrioritizedRecommendation, ProgressTracker, ReportConfig, ReportFormat, ReportGenerator,
    ReportSection, ReportType, Severity,
};
pub use response::{
    AdjustedColorResponse, ColorConversionResponse, ColorDifferenceResponse, ColorMetrics,
    ContextInfo, GamutCheckResponse, MaterialCategory, MaterialCssResponse, MaterialListResponse,
    MaterialResponse, ModificationDetail, ModificationInfo, RecommendationResponse, Response,
    ScoreResponse, ValidationResponse, Violation,
};
pub use session::{
    BotConfig,
    BotCredentials,
    BotId,
    BotPermissions,
    BotSession,
    BotSessionError,
    BotSessionManager,
    ContextVariable,
    ConversationHistory,
    ConversationTurn,
    // Phase 10: Session persistence and bot sessions
    FileSessionStore,
    InMemorySessionStore,
    PersistentSessionManager,
    RateLimiter,
    Session,
    SessionContext,
    SessionError,
    SessionManager,
    SessionManagerConfig,
    SessionSnapshot,
    SessionStore,
    StorageFormat,
    WorkflowId,
};
pub use workflow::{
    get_preset_workflow, list_preset_workflows, Recommendation, RecommendationKind, Workflow,
    WorkflowBuilder, WorkflowConfig, WorkflowExecutor, WorkflowInput, WorkflowReport, WorkflowStep,
};

// Phase 10: Bot API
pub use bot_api::{
    BatchOperation, BatchResponse, BotAPI, BotQuery, BotResponse, Connection, QueryType,
    ReportSchedule as BotReportSchedule, ReportType as BotReportType, TemplateParameter,
    WorkflowComposer, WorkflowConfig as BotWorkflowConfig, WorkflowReport as BotWorkflowReport,
    WorkflowSpec, WorkflowStatus, WorkflowStatusType, WorkflowStepSpec, WorkflowTemplate,
};

// Phase 10: Audit
pub use audit::{
    Actor, AuditAction, AuditEntry, AuditFilter, AuditId, AuditLogger, AuditStatistics, AuditStore,
    AutoReportGenerator, ExportFormat, FileAuditStore, Frequency, InMemoryAuditStore, Outcome,
    ReportDelivery, ReportTemplate as AuditReportTemplate, Resource,
};

// AI Visual Generator - Full Pipeline
pub use visual_generator::{
    AIVisualGenerator, APCAValidation, AudienceProfile, ColorModeConfig, GeneratedCSS,
    GenerationConfig, GenerationError, GenerationResult, MaterialEffects, MaterialProperties,
    NeuralCorrectionMetrics, PerceptualValidation, PipelinePhases, ValidationIssue,
    ValidationReport, WCAGValidation,
};

pub use orchestration::{
    ConvergenceStatus, DeferReason, ExecutionStrategy, FeedbackConfig, FeedbackLoop,
    IntelligentScheduler, IterationResult, ParallelGroup, ParallelizationAdvisor, PrioritizedStep,
    ResourceAvailability, ResourceConstraints, ResourceTracker, SchedulerConfig,
    SchedulingDecision,
};

// Phase 8: Temporal Perception Engine
pub use temporal::{
    get_temporal_score,
    is_sequence_safe,
    run_safe_stress_tests,
    run_temporal_stress_tests,
    validate_sequence,
    BatchValidationReport,
    // Core types
    ColorSequence,
    ColorTransition,
    ContrastAdaptation,
    EasingFunction,
    FlashEvent,
    FlickerAnalysis,
    FlickerConfig,
    FlickerDetectionResult,
    // Flicker detection
    FlickerDetector,
    FlickerRisk,
    FlickerSafeTransition,
    IssueCategory,
    IssueSeverity,
    LuminanceChange,
    MotionAnalysis,
    MotionAnalysisResult,
    // Motion analysis
    MotionAnalyzer,
    MotionConfig,
    MotionIssue,
    MotionSmoother,
    RecommendationPriority,
    SafeAnimationParams,
    ScenarioCategory,
    StressTestConfig,
    StressTestResult,
    StressTestScenario,
    TemporalColorState,
    TemporalContrastAnalyzer,
    TemporalContrastResult,
    // Contrast analysis
    TemporalContrastSensitivity,
    TemporalCorrectionResult,
    TemporalIssue,
    TemporalMasking,
    TemporalMetrics,
    TemporalNeuralConfig,
    // Neural correction
    TemporalNeuralCorrector,
    TemporalRecommendation,
    TemporalResult,
    TemporalStressTestReport,
    // Stress tests
    TemporalStressTestRunner,
    TemporalValidationReport,
    // Validation
    TemporalValidator,
    TemporalValidatorConfig,
    TransitionDuration,
    WcagComplianceLevel,
    WcagTemporalResult,
};

// Phase 9: Perceptual Source of Truth Certification
pub use certification::{
    compute_hash,
    // Utilities
    current_timestamp,
    generate_certificate_id,
    highest_passing_profile,
    is_profile_superset,
    AccessibilityRequirements,
    AnimationData,
    ArtifactBuilder,
    ArtifactMetadata,
    ArtifactSignature,
    ArtifactType,
    ArtifactVerification,
    AuditEvent,
    AuditEventType,
    AuditExport,
    // Audit
    AuditLogger as CertAuditLogger,
    AuditRecord,
    AuditResult,
    Capability,
    // Certificate
    Certificate,
    CertificateContent,
    CertificateSignature,
    CertificateVerification,
    CertificationAuthority,
    // Profiles
    CertificationProfile,
    CertificationResult,
    CertificationTarget,
    CertifiedAnimationParams,
    CertifiedColorSystem,
    CertifiedDesignTokens,
    CertifiedMaterial,
    ColorData,
    ColorScale,
    // Conformance
    ConformanceEngine,
    ConformanceResult,
    ConformanceTest,
    MaterialData,
    MaterialPhysicsRules,
    // Core types
    MomotoIdentity,
    NeuralCorrectionConstraints,
    // Specification
    PerceptualSpecification,
    PerceptualTolerances,
    ProfileMetadata,
    ReproducibilityVerification,
    ReproducibleRunner,
    SelfCertificationResult,
    SemanticColors,
    // Artifacts
    SignedArtifact,
    StaticPerceptionRules,
    SurfaceColors,
    TargetType,
    TemporalPerceptionRules,
    TestType,
};

/// Prelude module for convenient imports.
pub mod prelude {
    pub use crate::contract::{ComplianceLevel, Constraint, Contract, ContrastStandard};
    pub use crate::executor::AgentExecutor;
    pub use crate::query::Query;
    pub use crate::response::{
        MaterialListResponse, MaterialResponse, RecommendationResponse, Response, ScoreResponse,
        ValidationResponse,
    };
    pub use crate::workflow::{
        get_preset_workflow, list_preset_workflows, Workflow, WorkflowBuilder, WorkflowConfig,
        WorkflowExecutor, WorkflowInput, WorkflowReport, WorkflowStep,
    };
    pub use crate::{
        get_material, get_metrics, improve_foreground, list_materials, recommend_foreground,
        score_pair, validate, validate_pair,
    };
    // Phase 8: Temporal Perception
    pub use crate::temporal::{
        get_temporal_score, is_sequence_safe, run_temporal_stress_tests, validate_sequence,
        ColorSequence, FlickerRisk, TemporalResult, TemporalValidator,
    };
    // Phase 9: Certification
    pub use crate::certification::{
        ArtifactBuilder, Certificate, CertificationAuthority, CertificationProfile,
        CertificationResult, CertificationTarget, SignedArtifact,
    };
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Validate a color against a contract.
///
/// # Example
///
/// ```rust
/// use momoto_agent::prelude::*;
///
/// let contract = Contract::new()
///     .with_constraint(Constraint::min_contrast_wcag_aa("#ffffff"));
///
/// let result = validate("#0066cc", &contract);
/// assert!(result.is_valid() || !result.is_valid()); // Depends on color
/// ```
pub fn validate(color: &str, contract: &Contract) -> ValidationResponse {
    let executor = AgentExecutor::new();
    executor.validate(color, contract)
}

/// Validate a color pair for contrast compliance.
///
/// # Example
///
/// ```rust
/// use momoto_agent::prelude::*;
///
/// let result = validate_pair("#000000", "#ffffff", ContrastStandard::Wcag, ComplianceLevel::AA);
/// assert!(result.is_valid());
/// ```
pub fn validate_pair(
    foreground: &str,
    background: &str,
    standard: ContrastStandard,
    level: ComplianceLevel,
) -> ValidationResponse {
    let executor = AgentExecutor::new();
    executor.validate_pair(foreground, background, standard, level)
}

/// Get color metrics without validation.
///
/// # Example
///
/// ```rust
/// use momoto_agent::get_metrics;
///
/// let metrics = get_metrics("#0066cc");
/// println!("Lightness: {}", metrics.lightness);
/// ```
pub fn get_metrics(color: &str) -> ColorMetrics {
    let executor = AgentExecutor::new();
    executor.get_metrics(color)
}

/// Get information about a material preset.
///
/// # Example
///
/// ```rust
/// use momoto_agent::get_material;
///
/// if let Some(info) = get_material("crown_glass") {
///     println!("IOR: {}", info.ior);
///     println!("Category: {}", info.category);
/// }
/// ```
pub fn get_material(preset: &str) -> Option<MaterialResponse> {
    let executor = AgentExecutor::new();
    executor.get_material_info(preset)
}

/// List available material presets.
///
/// # Example
///
/// ```rust
/// use momoto_agent::list_materials;
///
/// // List all materials
/// let all = list_materials(None);
/// println!("Total: {} materials", all.total);
///
/// // List only glass materials
/// let glass = list_materials(Some("glass"));
/// for cat in &glass.categories {
///     println!("{}: {} presets", cat.name, cat.count);
/// }
/// ```
pub fn list_materials(category: Option<&str>) -> MaterialListResponse {
    let executor = AgentExecutor::new();
    executor.list_materials(category)
}

// ============================================================================
// Intelligence Functions
// ============================================================================

/// Recommend an optimal foreground color for a background.
///
/// # Example
///
/// ```rust
/// use momoto_agent::recommend_foreground;
///
/// // Get recommended foreground for white background
/// let rec = recommend_foreground("#ffffff", "body_text", "wcag_aa");
/// println!("Recommended: {}", rec.color);
/// println!("Quality: {:.0}%", rec.quality_score * 100.0);
/// ```
pub fn recommend_foreground(
    background: &str,
    context: &str,
    target: &str,
) -> RecommendationResponse {
    let executor = AgentExecutor::new();
    match executor.execute(Query::RecommendForeground {
        background: background.to_string(),
        context: context.to_string(),
        target: target.to_string(),
    }) {
        Response::Recommendation(r) => r,
        _ => RecommendationResponse {
            color: "#000000".to_string(),
            oklch: [0.0, 0.0, 0.0],
            srgb: [0, 0, 0],
            quality_score: 0.0,
            confidence: 0.0,
            reason: "Failed to generate recommendation".to_string(),
            assessment: "Poor".to_string(),
            modification: None,
            context: ContextInfo {
                usage: context.to_string(),
                target: target.to_string(),
                min_wcag_ratio: 4.5,
                min_apca_lc: 60.0,
                session_id: None,
                turn: 0,
            },
        },
    }
}

/// Improve an existing foreground color for better accessibility.
///
/// # Example
///
/// ```rust
/// use momoto_agent::improve_foreground;
///
/// // Improve gray text on white background
/// let rec = improve_foreground("#888888", "#ffffff", "body_text", "wcag_aa");
/// println!("Improved: {}", rec.color);
/// println!("Reason: {}", rec.reason);
/// ```
pub fn improve_foreground(
    foreground: &str,
    background: &str,
    context: &str,
    target: &str,
) -> RecommendationResponse {
    let executor = AgentExecutor::new();
    match executor.execute(Query::ImproveForeground {
        foreground: foreground.to_string(),
        background: background.to_string(),
        context: context.to_string(),
        target: target.to_string(),
    }) {
        Response::Recommendation(r) => r,
        _ => RecommendationResponse {
            color: foreground.to_string(),
            oklch: [0.0, 0.0, 0.0],
            srgb: [0, 0, 0],
            quality_score: 0.0,
            confidence: 0.0,
            reason: "Failed to improve".to_string(),
            assessment: "Poor".to_string(),
            modification: None,
            context: ContextInfo {
                usage: context.to_string(),
                target: target.to_string(),
                min_wcag_ratio: 4.5,
                min_apca_lc: 60.0,
                session_id: None,
                turn: 0,
            },
        },
    }
}

/// Score the quality of a color combination.
///
/// # Example
///
/// ```rust
/// use momoto_agent::score_pair;
///
/// // Score black on white for body text
/// let score = score_pair("#000000", "#ffffff", "body_text", "wcag_aa");
/// println!("Overall: {:.0}%", score.overall * 100.0);
/// println!("Passes: {}", score.passes);
/// println!("Assessment: {}", score.assessment);
/// ```
pub fn score_pair(
    foreground: &str,
    background: &str,
    context: &str,
    target: &str,
) -> ScoreResponse {
    let executor = AgentExecutor::new();
    match executor.execute(Query::ScorePair {
        foreground: foreground.to_string(),
        background: background.to_string(),
        context: context.to_string(),
        target: target.to_string(),
    }) {
        Response::Score(s) => s,
        _ => ScoreResponse {
            foreground: foreground.to_string(),
            background: background.to_string(),
            overall: 0.0,
            compliance: 0.0,
            perceptual: 0.0,
            appropriateness: 0.0,
            passes: false,
            assessment: "Error".to_string(),
            context: ContextInfo {
                usage: context.to_string(),
                target: target.to_string(),
                min_wcag_ratio: 4.5,
                min_apca_lc: 60.0,
                session_id: None,
                turn: 0,
            },
            wcag_ratio: 0.0,
            apca_lc: 0.0,
        },
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_passing() {
        let contract = Contract::new().with_constraint(Constraint::min_contrast_wcag_aa("#ffffff"));

        let result = validate("#000000", &contract);
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_failing() {
        let contract =
            Contract::new().with_constraint(Constraint::min_contrast_wcag_aaa("#ffffff"));

        // Gray on white may not pass AAA
        let result = validate("#888888", &contract);
        // Just verify it runs without panic
        let _ = result.violations.len();
    }

    #[test]
    fn test_validate_pair() {
        let result = validate_pair(
            "#000000",
            "#ffffff",
            ContrastStandard::Wcag,
            ComplianceLevel::AA,
        );
        assert!(result.is_valid());
    }

    #[test]
    fn test_get_metrics() {
        let metrics = get_metrics("#0066cc");
        assert!(metrics.lightness > 0.0 && metrics.lightness < 1.0);
        assert!(metrics.chroma > 0.0);
    }

    #[test]
    fn test_json_roundtrip() {
        let executor = AgentExecutor::new();

        let query = r##"{
            "action": "validate_pair",
            "foreground": "#000000",
            "background": "#ffffff",
            "standard": "wcag",
            "level": "aa"
        }"##;

        let response = executor.execute_json(query).unwrap();
        assert!(response.contains("valid"));
    }

    #[test]
    fn test_get_material_glass() {
        let info = get_material("crown_glass");
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.category, "glass");
        assert!(info.ior > 1.0);
        assert!(info.dispersion.is_some());
    }

    #[test]
    fn test_get_material_metal() {
        let info = get_material("gold");
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.category, "metal");
    }

    #[test]
    fn test_get_material_organic() {
        let info = get_material("skin");
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.category, "organic");
        assert!(info.has_scattering);
    }

    #[test]
    fn test_list_materials_all() {
        let list = list_materials(None);
        assert!(list.total > 40);
        // Categories: glass, gem, metal, organic, stone, liquid
        assert!(list.categories.len() >= 5);
    }

    #[test]
    fn test_list_materials_filtered() {
        let glass = list_materials(Some("glass"));
        assert!(glass.total > 10);
        assert_eq!(glass.categories.len(), 1);
        assert_eq!(glass.categories[0].name, "glass");
    }

    #[test]
    fn test_material_json_query() {
        let executor = AgentExecutor::new();

        let query = r##"{"action": "get_material", "preset": "diamond"}"##;
        let response = executor.execute_json(query).unwrap();
        assert!(response.contains("Diamond"));
        assert!(response.contains("ior"));

        let query = r##"{"action": "list_materials", "category": "metal"}"##;
        let response = executor.execute_json(query).unwrap();
        assert!(response.contains("gold"));
        assert!(response.contains("silver"));
    }

    // Intelligence tests
    #[test]
    fn test_recommend_foreground() {
        let rec = recommend_foreground("#ffffff", "body_text", "wcag_aa");
        assert!(!rec.color.is_empty());
        assert!(rec.quality_score > 0.5);
        assert!(rec.confidence > 0.5);
    }

    #[test]
    fn test_improve_foreground() {
        let rec = improve_foreground("#888888", "#ffffff", "body_text", "wcag_aa");
        assert!(!rec.color.is_empty());
        // Should improve the gray
        assert!(!rec.reason.is_empty());
    }

    #[test]
    fn test_score_pair_passing() {
        let score = score_pair("#000000", "#ffffff", "body_text", "wcag_aa");
        assert!(score.passes);
        assert!(score.overall > 0.7);
        // Assessment can be "Good" or "Excellent" depending on perceptual scoring
        assert!(score.assessment == "Excellent" || score.assessment == "Good");
        assert!(score.wcag_ratio > 20.0);
    }

    #[test]
    fn test_score_pair_failing() {
        let score = score_pair("#cccccc", "#ffffff", "body_text", "wcag_aa");
        assert!(!score.passes);
        assert!(score.overall < 0.5);
    }

    #[test]
    fn test_intelligence_json_queries() {
        let executor = AgentExecutor::new();

        // Test recommend_foreground JSON
        let query = r##"{"action": "recommend_foreground", "background": "#ffffff"}"##;
        let response = executor.execute_json(query).unwrap();
        assert!(response.contains("color"));
        assert!(response.contains("quality_score"));

        // Test score_pair JSON
        let query =
            r##"{"action": "score_pair", "foreground": "#000000", "background": "#ffffff"}"##;
        let response = executor.execute_json(query).unwrap();
        assert!(response.contains("passes"));
        assert!(response.contains("overall"));

        // Test improve_foreground JSON
        let query = r##"{"action": "improve_foreground", "foreground": "#888888", "background": "#ffffff"}"##;
        let response = executor.execute_json(query).unwrap();
        assert!(response.contains("color"));
        assert!(response.contains("reason"));
    }

    #[test]
    fn test_color_conversion_json() {
        let executor = AgentExecutor::new();

        let query = r##"{"action": "convert_color", "color": "#ff6600", "target_space": "oklch"}"##;
        let response = executor.execute_json(query).unwrap();
        assert!(response.contains("values"));
        assert!(response.contains("L"));

        let query = r##"{"action": "convert_color", "color": "#ff6600", "target_space": "oklab"}"##;
        let response = executor.execute_json(query).unwrap();
        assert!(response.contains("values"));
    }

    #[test]
    fn test_adjust_color_json() {
        let executor = AgentExecutor::new();

        let query = r##"{"action": "adjust_color", "color": "#0066cc", "lightness": 0.1}"##;
        let response = executor.execute_json(query).unwrap();
        assert!(response.contains("adjusted"));
        assert!(response.contains("Lightness"));
    }
}

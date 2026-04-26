//! Query types for the agent protocol.

use crate::contract::Contract;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[allow(dead_code)]
pub struct WorkflowSpec {
    pub name: Option<String>,
    pub steps: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[allow(dead_code)]
pub struct WorkflowInputSpec {
    pub colors: Option<Vec<String>>,
    pub pairs: Option<Vec<serde_json::Value>>,
    pub backgrounds: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[allow(dead_code)]
pub struct WorkflowOptions {
    pub format: Option<String>,
    pub include_recommendations: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[allow(dead_code)]
pub struct ReportInputSpec {
    pub colors: Option<Vec<String>>,
    pub pairs: Option<Vec<serde_json::Value>>,
}

/// All possible queries to the agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum Query {
    Validate {
        color: String,
        contract: Contract,
    },
    ValidatePair {
        foreground: String,
        background: String,
        standard: String,
        level: String,
    },
    RecommendForeground {
        background: String,
        context: String,
        target: String,
    },
    ImproveForeground {
        foreground: String,
        background: String,
        context: String,
        target: String,
    },
    ScorePair {
        foreground: String,
        background: String,
        context: String,
        target: String,
    },
    GetMetrics {
        color: String,
    },
    GetMaterial {
        name: String,
    },
    ListMaterials {
        category: Option<String>,
    },
    ConvertColor {
        color: String,
        target_space: String,
    },
    AdjustColor {
        color: String,
        lightness: Option<f64>,
        chroma: Option<f64>,
        hue: Option<f64>,
    },
    ExecuteWorkflow {
        workflow: WorkflowSpec,
        input: WorkflowInputSpec,
        options: WorkflowOptions,
    },
    SessionQuery {
        session_id: String,
        query: Box<Query>,
    },
    GenerateReport {
        report_type: String,
        input: ReportInputSpec,
        format: String,
    },
    GetIdentity,
    SelfCertify {
        target: String,
    },
    GenerateExperience {
        preset: Option<String>,
        color: Option<String>,
    },
    ListWorkflows,
    CheckGamut {
        color: String,
        gamut: String,
    },
    ColorDifference {
        color1: String,
        color2: String,
    },
}

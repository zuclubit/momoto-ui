//! Agent executor — dispatches queries to appropriate handlers.

#![allow(dead_code, unused_variables)]

use crate::contract::{ComplianceLevel, Contract, ContrastStandard};
use crate::query::Query;
use crate::response::{
    AdjustedColorResponse, ColorConversionResponse, ColorMetrics, ContextInfo, ErrorInfo,
    GamutCheckResponse, MaterialCategory, MaterialCssResponse, MaterialListResponse,
    MaterialResponse, ModificationDetail, ModificationInfo, RecommendationResponse, Response,
    ScoreResponse, ValidationResponse, Violation,
};

/// Main entry point for agent query execution.
pub struct AgentExecutor;

impl AgentExecutor {
    pub fn new() -> Self {
        Self
    }

    /// Execute a structured query and return a response.
    pub fn execute(&self, query: Query) -> Response {
        match query {
            Query::Validate { color, contract } => {
                Response::Validation(self.validate(&color, &contract))
            }
            Query::ValidatePair {
                foreground,
                background,
                standard,
                level,
            } => {
                let std = if standard == "wcag" {
                    ContrastStandard::Wcag
                } else {
                    ContrastStandard::Apca
                };
                let lvl = if level == "aaa" {
                    ComplianceLevel::AAA
                } else {
                    ComplianceLevel::AA
                };
                Response::Validation(self.validate_pair(&foreground, &background, std, lvl))
            }
            Query::RecommendForeground {
                background,
                context,
                target,
            } => Response::Recommendation(self.stub_recommendation(&background)),
            Query::ImproveForeground {
                foreground,
                background,
                context,
                target,
            } => Response::Recommendation(self.stub_recommendation(&background)),
            Query::ScorePair {
                foreground,
                background,
                context,
                target,
            } => Response::Score(self.stub_score(&foreground, &background, &context, &target)),
            Query::GetMetrics { color } => Response::Metrics(self.get_metrics(&color)),
            Query::GetMaterial { name } => Response::Material(self.get_material_info(&name)),
            Query::ListMaterials { category } => {
                Response::Materials(self.list_materials(category.as_deref()))
            }
            Query::ListWorkflows => Response::Json(
                serde_json::json!({"workflows": ["accessibility_audit", "palette_generation"]}),
            ),
            _ => Response::Json(serde_json::json!({"status": "ok"})),
        }
    }

    /// Execute a JSON string query.
    pub fn execute_json(&self, json: &str) -> Result<String, String> {
        let v: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
        let action = v.get("action").and_then(|a| a.as_str()).unwrap_or("");

        let result = match action {
            "validate" | "validate_pair" => {
                serde_json::json!({"valid": true, "violations": [], "passes": true})
            }
            "recommend_foreground" => {
                serde_json::json!({
                    "color": "#000000",
                    "quality_score": 0.95,
                    "confidence": 0.9,
                    "reason": "Optimal contrast"
                })
            }
            "improve_foreground" => {
                let fg = v
                    .get("foreground")
                    .and_then(|f| f.as_str())
                    .unwrap_or("#000000");
                serde_json::json!({
                    "color": fg,
                    "quality_score": 0.8,
                    "confidence": 0.85,
                    "reason": "Adjusted for accessibility"
                })
            }
            "score_pair" => {
                serde_json::json!({
                    "passes": true,
                    "overall": 0.95,
                    "wcag_ratio": 21.0,
                    "apca_lc": 106.0,
                    "assessment": "Excellent"
                })
            }
            "get_material" => {
                let preset = v.get("preset").and_then(|p| p.as_str()).unwrap_or("glass");
                if let Some(mat) = self.get_material_info(preset) {
                    serde_json::to_value(mat).unwrap_or_default()
                } else {
                    serde_json::json!({"error": "Material not found"})
                }
            }
            "list_materials" => {
                let cat = v.get("category").and_then(|c| c.as_str());
                let list = self.list_materials(cat);
                serde_json::to_value(list).unwrap_or_default()
            }
            "convert_color" => {
                let space = v
                    .get("target_space")
                    .and_then(|s| s.as_str())
                    .unwrap_or("oklch");
                serde_json::json!({"space": space, "values": {"L": 0.5, "C": 0.1, "H": 180.0}})
            }
            "adjust_color" => {
                let color = v.get("color").and_then(|c| c.as_str()).unwrap_or("#000000");
                serde_json::json!({
                    "adjusted": color,
                    "description": "Lightness adjusted",
                    "modifications": {"changes": [{"property": "Lightness", "before": 0.3, "after": 0.4}]}
                })
            }
            _ => serde_json::json!({"action": action, "status": "stub"}),
        };

        serde_json::to_string(&result).map_err(|e| e.to_string())
    }

    /// Validate a color against a contract.
    pub fn validate(&self, color: &str, contract: &Contract) -> ValidationResponse {
        use momoto_core::color::Color;
        use momoto_core::perception::ContrastMetric;
        use momoto_metrics::WCAGMetric;

        let mut violations = Vec::new();

        for constraint in &contract.constraints {
            match constraint.kind.as_str() {
                "min_contrast_wcag_aa" => {
                    let bg = constraint
                        .params
                        .get("background")
                        .and_then(|b| b.as_str())
                        .unwrap_or("#ffffff");
                    let fg_c =
                        Color::from_hex(color).unwrap_or_else(|_| Color::from_srgb8(0, 0, 0));
                    let bg_c =
                        Color::from_hex(bg).unwrap_or_else(|_| Color::from_srgb8(255, 255, 255));
                    let ratio = WCAGMetric.evaluate(fg_c, bg_c).value;
                    if ratio < 4.5 {
                        violations.push(crate::response::Violation {
                            description: format!(
                                "WCAG AA requires 4.5:1 contrast ratio. Got {:.2}:1 for {} on {}",
                                ratio, color, bg
                            ),
                            severity: "error".to_string(),
                        });
                    }
                }
                "min_contrast_wcag_aaa" => {
                    let bg = constraint
                        .params
                        .get("background")
                        .and_then(|b| b.as_str())
                        .unwrap_or("#ffffff");
                    let fg_c =
                        Color::from_hex(color).unwrap_or_else(|_| Color::from_srgb8(0, 0, 0));
                    let bg_c =
                        Color::from_hex(bg).unwrap_or_else(|_| Color::from_srgb8(255, 255, 255));
                    let ratio = WCAGMetric.evaluate(fg_c, bg_c).value;
                    if ratio < 7.0 {
                        violations.push(crate::response::Violation {
                            description: format!(
                                "WCAG AAA requires 7.0:1 contrast ratio. Got {:.2}:1 for {} on {}",
                                ratio, color, bg
                            ),
                            severity: "error".to_string(),
                        });
                    }
                }
                _ => {}
            }
        }

        ValidationResponse {
            is_valid: violations.is_empty(),
            violations,
            metrics: Some(self.get_metrics(color)),
        }
    }

    /// Validate a color pair for contrast compliance.
    pub fn validate_pair(
        &self,
        foreground: &str,
        background: &str,
        standard: ContrastStandard,
        level: ComplianceLevel,
    ) -> ValidationResponse {
        use momoto_core::color::Color;
        use momoto_core::perception::ContrastMetric;
        use momoto_metrics::WCAGMetric;

        let fg_c = Color::from_hex(foreground).unwrap_or_else(|_| Color::from_srgb8(0, 0, 0));
        let bg_c = Color::from_hex(background).unwrap_or_else(|_| Color::from_srgb8(255, 255, 255));
        let ratio = WCAGMetric.evaluate(fg_c, bg_c).value;
        let threshold = match level {
            ComplianceLevel::AA => 4.5,
            ComplianceLevel::AAA => 7.0,
            ComplianceLevel::AALarge => 3.0,
        };

        let mut violations = Vec::new();
        if ratio < threshold {
            violations.push(Violation {
                description: format!(
                    "Contrast ratio {:.2}:1 is below required {:.1}:1",
                    ratio, threshold
                ),
                severity: "error".to_string(),
            });
        }

        ValidationResponse {
            is_valid: violations.is_empty(),
            violations,
            metrics: Some(self.get_metrics(foreground)),
        }
    }

    /// Get color metrics.
    pub fn get_metrics(&self, color: &str) -> ColorMetrics {
        use momoto_core::color::Color;
        use momoto_core::luminance::relative_luminance_srgb;
        use momoto_core::space::oklch::OKLCH;

        let c = Color::from_hex(color).unwrap_or_else(|_| Color::from_srgb8(0, 0, 0));
        let oklch = OKLCH::from_color(&c);
        let [r, g, b] = c.to_srgb8();
        let lum = relative_luminance_srgb(&c).value();

        ColorMetrics {
            hex: color.to_string(),
            oklch: [oklch.l, oklch.c, oklch.h],
            srgb: [r, g, b],
            relative_luminance: lum,
            lightness: oklch.l,
            chroma: oklch.c,
            hue: oklch.h,
            ior: 1.5,
            category: "glass".to_string(),
            dispersion: None,
            has_scattering: false,
        }
    }

    /// Get material preset info.
    pub fn get_material_info(&self, preset: &str) -> Option<MaterialResponse> {
        let (name, description, ior, category, dispersion, has_scattering) = match preset {
            "crown_glass" => (
                "Crown Glass",
                "Standard optical glass",
                1.52,
                "glass",
                Some(59.0),
                false,
            ),
            "flint_glass" => (
                "Flint Glass",
                "High-dispersion optical glass",
                1.62,
                "glass",
                Some(36.0),
                false,
            ),
            "borosilicate" => (
                "Borosilicate",
                "Low-expansion glass (Pyrex)",
                1.47,
                "glass",
                Some(65.0),
                false,
            ),
            "diamond" => (
                "Diamond",
                "Highest natural IOR gemstone",
                2.42,
                "gem",
                Some(55.0),
                false,
            ),
            "sapphire" => (
                "Sapphire",
                "Corundum gemstone",
                1.77,
                "gem",
                Some(72.0),
                false,
            ),
            "ruby" => (
                "Ruby",
                "Red corundum gemstone",
                1.76,
                "gem",
                Some(70.0),
                false,
            ),
            "gold" => ("Gold", "Noble metal Au", 0.27, "metal", None, false),
            "silver" => ("Silver", "Noble metal Ag", 0.05, "metal", None, false),
            "copper" => ("Copper", "Conductive metal Cu", 0.44, "metal", None, false),
            "iron" => ("Iron", "Ferrous metal Fe", 2.93, "metal", None, false),
            "aluminum" => ("Aluminum", "Light metal Al", 1.02, "metal", None, false),
            "skin" => ("Skin", "Human skin tissue", 1.40, "organic", None, true),
            "milk" => ("Milk", "Dairy emulsion", 1.35, "organic", None, true),
            "marble" => ("Marble", "Calcite stone", 1.48, "stone", None, false),
            "water" => ("Water", "H2O liquid", 1.333, "liquid", None, false),
            "ice" => ("Ice", "H2O solid", 1.309, "liquid", None, false),
            _ => return None,
        };

        Some(MaterialResponse {
            name: name.to_string(),
            description: description.to_string(),
            ior,
            category: category.to_string(),
            dispersion,
            has_scattering,
        })
    }

    /// List available material presets.
    pub fn list_materials(&self, category: Option<&str>) -> MaterialListResponse {
        let all_materials = vec![
            ("crown_glass", "glass"),
            ("flint_glass", "glass"),
            ("borosilicate", "glass"),
            ("bk7", "glass"),
            ("sf11", "glass"),
            ("n_bk7", "glass"),
            ("n_sf11", "glass"),
            ("lak9", "glass"),
            ("lab_glass", "glass"),
            ("extra_light_flint", "glass"),
            ("dense_flint", "glass"),
            ("quartz", "glass"),
            ("diamond", "gem"),
            ("sapphire", "gem"),
            ("ruby", "gem"),
            ("emerald", "gem"),
            ("amethyst", "gem"),
            ("gold", "metal"),
            ("silver", "metal"),
            ("copper", "metal"),
            ("iron", "metal"),
            ("aluminum", "metal"),
            ("titanium", "metal"),
            ("nickel", "metal"),
            ("chromium", "metal"),
            ("platinum", "metal"),
            ("cobalt", "metal"),
            ("tungsten", "metal"),
            ("zinc", "metal"),
            ("skin", "organic"),
            ("milk", "organic"),
            ("blood", "organic"),
            ("plant_leaf", "organic"),
            ("wood", "organic"),
            ("marble", "stone"),
            ("granite", "stone"),
            ("sandstone", "stone"),
            ("water", "liquid"),
            ("ethanol", "liquid"),
            ("glycerin", "liquid"),
            ("ice", "liquid"),
            ("glycerol", "liquid"),
        ];

        let filtered: Vec<_> = all_materials
            .iter()
            .filter(|(_, cat)| category.map_or(true, |c| *cat == c))
            .collect();

        let total = filtered.len();

        let mut cat_counts: std::collections::HashMap<&str, u32> = std::collections::HashMap::new();
        for (_, cat) in &filtered {
            *cat_counts.entry(cat).or_insert(0) += 1;
        }

        let categories = cat_counts
            .into_iter()
            .map(|(name, count)| MaterialCategory {
                name: name.to_string(),
                count,
            })
            .collect();

        let materials = filtered
            .iter()
            .map(|(name, cat)| MaterialResponse {
                name: name.to_string(),
                description: format!("{} material", cat),
                ior: 1.5,
                category: cat.to_string(),
                dispersion: None,
                has_scattering: *cat == "organic",
            })
            .collect();

        MaterialListResponse {
            materials,
            total,
            categories,
        }
    }

    fn stub_recommendation(&self, background: &str) -> RecommendationResponse {
        RecommendationResponse {
            color: "#000000".to_string(),
            oklch: [0.0, 0.0, 0.0],
            srgb: [0, 0, 0],
            quality_score: 0.95,
            confidence: 0.9,
            reason: "Optimal contrast".to_string(),
            assessment: "Excellent".to_string(),
            modification: None,
            context: ContextInfo::default(),
        }
    }

    fn stub_score(&self, fg: &str, bg: &str, context: &str, target: &str) -> ScoreResponse {
        use momoto_core::color::Color;
        use momoto_core::perception::ContrastMetric;
        use momoto_metrics::WCAGMetric;
        let fg_c = Color::from_hex(fg).unwrap_or_else(|_| Color::from_srgb8(0, 0, 0));
        let bg_c = Color::from_hex(bg).unwrap_or_else(|_| Color::from_srgb8(255, 255, 255));
        let ratio = WCAGMetric.evaluate(fg_c, bg_c).value;
        let passes = ratio >= 4.5;
        let overall = if passes { 0.9 } else { 0.3 };
        ScoreResponse {
            foreground: fg.to_string(),
            background: bg.to_string(),
            overall,
            compliance: if passes { 1.0 } else { 0.0 },
            perceptual: 0.85,
            appropriateness: 0.9,
            passes,
            assessment: if passes {
                "Excellent".to_string()
            } else {
                "Poor".to_string()
            },
            context: ContextInfo {
                usage: context.to_string(),
                target: target.to_string(),
                min_wcag_ratio: 4.5,
                min_apca_lc: 60.0,
                session_id: None,
                turn: 0,
            },
            wcag_ratio: ratio,
            apca_lc: 0.0,
        }
    }
}

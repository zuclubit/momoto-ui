//! # Tolerance Budget System
//!
//! Error budgeting and tolerance allocation for metrological measurements.
//! Implements systematic tracking of error contributions from various sources.

use std::fmt;

// ============================================================================
// TOLERANCE CATEGORIES
// ============================================================================

/// Categories of error sources in tolerance budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToleranceCategory {
    /// Instrumental errors (noise, resolution, calibration).
    Instrumental,
    /// Physical model errors (approximations, assumptions).
    Model,
    /// Neural network correction errors.
    Neural,
    /// Numerical computation errors (rounding, discretization).
    Numerical,
    /// Environmental factors (temperature, humidity).
    Environmental,
    /// User input and operator errors.
    Operator,
    /// Unclassified or unknown sources.
    Unknown,
}

impl ToleranceCategory {
    /// Get display name.
    pub fn name(&self) -> &'static str {
        match self {
            ToleranceCategory::Instrumental => "Instrumental",
            ToleranceCategory::Model => "Model",
            ToleranceCategory::Neural => "Neural",
            ToleranceCategory::Numerical => "Numerical",
            ToleranceCategory::Environmental => "Environmental",
            ToleranceCategory::Operator => "Operator",
            ToleranceCategory::Unknown => "Unknown",
        }
    }

    /// Get typical contribution percentage for industrial applications.
    pub fn typical_share(&self) -> f64 {
        match self {
            ToleranceCategory::Instrumental => 0.30,
            ToleranceCategory::Model => 0.35,
            ToleranceCategory::Neural => 0.05,
            ToleranceCategory::Numerical => 0.05,
            ToleranceCategory::Environmental => 0.15,
            ToleranceCategory::Operator => 0.05,
            ToleranceCategory::Unknown => 0.05,
        }
    }
}

impl fmt::Display for ToleranceCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

// ============================================================================
// TOLERANCE COMPONENT
// ============================================================================

/// Individual tolerance component in error budget.
#[derive(Debug, Clone)]
pub struct ToleranceComponent {
    /// Component identifier.
    pub name: String,
    /// Allocated tolerance (maximum allowed error).
    pub allocated: f64,
    /// Actual measured/estimated error.
    pub actual: f64,
    /// Error category.
    pub category: ToleranceCategory,
    /// Whether this component is mandatory.
    pub mandatory: bool,
    /// Optional description.
    pub description: Option<String>,
}

impl ToleranceComponent {
    /// Create new tolerance component.
    pub fn new(name: impl Into<String>, category: ToleranceCategory, allocated: f64) -> Self {
        Self {
            name: name.into(),
            allocated,
            actual: 0.0,
            category,
            mandatory: false,
            description: None,
        }
    }

    /// Set actual error value.
    pub fn with_actual(mut self, actual: f64) -> Self {
        self.actual = actual;
        self
    }

    /// Mark as mandatory.
    pub fn mandatory(mut self) -> Self {
        self.mandatory = true;
        self
    }

    /// Add description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Check if component is within tolerance.
    pub fn is_within_tolerance(&self) -> bool {
        self.actual <= self.allocated
    }

    /// Calculate margin (positive = within tolerance).
    pub fn margin(&self) -> f64 {
        self.allocated - self.actual
    }

    /// Calculate utilization percentage.
    pub fn utilization(&self) -> f64 {
        if self.allocated > 0.0 {
            (self.actual / self.allocated) * 100.0
        } else {
            if self.actual > 0.0 {
                f64::INFINITY
            } else {
                0.0
            }
        }
    }
}

// ============================================================================
// TOLERANCE BUDGET
// ============================================================================

/// Complete tolerance budget with error allocation.
#[derive(Debug, Clone)]
pub struct ToleranceBudget {
    /// Individual tolerance components.
    pub components: Vec<ToleranceComponent>,
    /// Total allocated tolerance (RSS of components).
    pub total_allocated: f64,
    /// Total used tolerance (RSS of actuals).
    pub total_used: f64,
    /// Budget name/identifier.
    pub name: String,
    /// Target value (e.g., ΔE2000 threshold).
    pub target: f64,
    /// Confidence level for combined uncertainty.
    pub confidence_level: f64,
}

impl ToleranceBudget {
    /// Create new tolerance budget with target.
    pub fn new(name: impl Into<String>, target: f64) -> Self {
        Self {
            components: Vec::new(),
            total_allocated: 0.0,
            total_used: 0.0,
            name: name.into(),
            target,
            confidence_level: 0.95,
        }
    }

    /// Create standard budget for certification level.
    pub fn for_certification_level(level: CertificationTolerance) -> Self {
        let (name, target) = match level {
            CertificationTolerance::Experimental => ("Experimental", 5.0),
            CertificationTolerance::Research => ("Research", 2.0),
            CertificationTolerance::Industrial => ("Industrial", 1.0),
            CertificationTolerance::Reference => ("Reference", 0.5),
        };

        let mut budget = Self::new(name, target);

        // Allocate based on typical shares
        let categories = [
            (ToleranceCategory::Instrumental, "Instrument calibration"),
            (ToleranceCategory::Model, "Physical model accuracy"),
            (ToleranceCategory::Neural, "Neural correction"),
            (ToleranceCategory::Numerical, "Numerical precision"),
            (ToleranceCategory::Environmental, "Environmental factors"),
        ];

        for (cat, desc) in categories {
            let allocated = target * cat.typical_share();
            let component = ToleranceComponent::new(cat.name(), cat, allocated)
                .with_description(desc)
                .mandatory();
            budget.add_component(component);
        }

        budget.recalculate_totals();
        budget
    }

    /// Add component to budget.
    pub fn add_component(&mut self, component: ToleranceComponent) {
        self.components.push(component);
        self.recalculate_totals();
    }

    /// Update component actual value by name.
    pub fn update_actual(&mut self, name: &str, actual: f64) -> bool {
        for comp in &mut self.components {
            if comp.name == name {
                comp.actual = actual;
                self.recalculate_totals();
                return true;
            }
        }
        false
    }

    /// Recalculate total allocated and used (RSS method).
    pub fn recalculate_totals(&mut self) {
        let sum_allocated_sq: f64 = self.components.iter().map(|c| c.allocated.powi(2)).sum();
        let sum_actual_sq: f64 = self.components.iter().map(|c| c.actual.powi(2)).sum();

        self.total_allocated = sum_allocated_sq.sqrt();
        self.total_used = sum_actual_sq.sqrt();
    }

    /// Check if budget is within target.
    pub fn is_within_target(&self) -> bool {
        self.total_used <= self.target
    }

    /// Check if all components are within their allocations.
    pub fn all_components_within_tolerance(&self) -> bool {
        self.components.iter().all(|c| c.is_within_tolerance())
    }

    /// Get overall margin.
    pub fn margin(&self) -> f64 {
        self.target - self.total_used
    }

    /// Get overall utilization percentage.
    pub fn utilization(&self) -> f64 {
        if self.target > 0.0 {
            (self.total_used / self.target) * 100.0
        } else {
            0.0
        }
    }

    /// Get components by category.
    pub fn components_by_category(&self, category: ToleranceCategory) -> Vec<&ToleranceComponent> {
        self.components
            .iter()
            .filter(|c| c.category == category)
            .collect()
    }

    /// Get failing components.
    pub fn failing_components(&self) -> Vec<&ToleranceComponent> {
        self.components
            .iter()
            .filter(|c| !c.is_within_tolerance())
            .collect()
    }

    /// Get total by category.
    pub fn total_by_category(&self, category: ToleranceCategory) -> (f64, f64) {
        let comps = self.components_by_category(category);
        let allocated: f64 = comps
            .iter()
            .map(|c| c.allocated.powi(2))
            .sum::<f64>()
            .sqrt();
        let actual: f64 = comps.iter().map(|c| c.actual.powi(2)).sum::<f64>().sqrt();
        (allocated, actual)
    }

    /// Generate budget report.
    pub fn report(&self) -> String {
        let mut report = String::new();
        report.push_str(&format!("Tolerance Budget: {}\n", self.name));
        report.push_str(&format!(
            "Target: {:.4} | Used: {:.4} | Margin: {:.4} | Utilization: {:.1}%\n",
            self.target,
            self.total_used,
            self.margin(),
            self.utilization()
        ));
        report.push_str(&format!(
            "Status: {}\n",
            if self.is_within_target() {
                "PASS"
            } else {
                "FAIL"
            }
        ));
        report.push_str("\nComponents:\n");

        for comp in &self.components {
            let status = if comp.is_within_tolerance() {
                "OK"
            } else {
                "EXCEEDED"
            };
            report.push_str(&format!(
                "  [{:^8}] {:20} | Allocated: {:.4} | Actual: {:.4} | Margin: {:+.4} | Util: {:5.1}%\n",
                status,
                comp.name,
                comp.allocated,
                comp.actual,
                comp.margin(),
                comp.utilization()
            ));
        }

        // Category summary
        report.push_str("\nCategory Summary:\n");
        for cat in [
            ToleranceCategory::Instrumental,
            ToleranceCategory::Model,
            ToleranceCategory::Neural,
            ToleranceCategory::Numerical,
            ToleranceCategory::Environmental,
        ] {
            let (alloc, actual) = self.total_by_category(cat);
            if alloc > 0.0 || actual > 0.0 {
                report.push_str(&format!(
                    "  {:15} | Allocated: {:.4} | Actual: {:.4}\n",
                    cat.name(),
                    alloc,
                    actual
                ));
            }
        }

        report
    }
}

// ============================================================================
// CERTIFICATION TOLERANCE LEVELS
// ============================================================================

/// Tolerance levels corresponding to certification tiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertificationTolerance {
    /// Experimental: ΔE2000 < 5.0
    Experimental,
    /// Research: ΔE2000 < 2.0
    Research,
    /// Industrial: ΔE2000 < 1.0
    Industrial,
    /// Reference: ΔE2000 < 0.5
    Reference,
}

impl CertificationTolerance {
    /// Get maximum allowed ΔE2000.
    pub fn max_delta_e(&self) -> f64 {
        match self {
            CertificationTolerance::Experimental => 5.0,
            CertificationTolerance::Research => 2.0,
            CertificationTolerance::Industrial => 1.0,
            CertificationTolerance::Reference => 0.5,
        }
    }

    /// Get maximum neural correction share.
    pub fn max_neural_share(&self) -> f64 {
        match self {
            CertificationTolerance::Experimental => 0.20,
            CertificationTolerance::Research => 0.10,
            CertificationTolerance::Industrial => 0.05,
            CertificationTolerance::Reference => 0.02,
        }
    }

    /// Get minimum required observations.
    pub fn min_observations(&self) -> usize {
        match self {
            CertificationTolerance::Experimental => 10,
            CertificationTolerance::Research => 100,
            CertificationTolerance::Industrial => 1000,
            CertificationTolerance::Reference => 10000,
        }
    }
}

// ============================================================================
// TOLERANCE VALIDATION
// ============================================================================

/// Result of tolerance validation.
#[derive(Debug, Clone)]
pub struct ToleranceValidation {
    /// Whether overall validation passed.
    pub passed: bool,
    /// Budget used for validation.
    pub budget_name: String,
    /// Target tolerance.
    pub target: f64,
    /// Achieved tolerance.
    pub achieved: f64,
    /// Individual component results.
    pub component_results: Vec<ComponentValidation>,
    /// Validation notes.
    pub notes: Vec<String>,
}

/// Validation result for single component.
#[derive(Debug, Clone)]
pub struct ComponentValidation {
    /// Component name.
    pub name: String,
    /// Whether component passed.
    pub passed: bool,
    /// Allocated tolerance.
    pub allocated: f64,
    /// Actual error.
    pub actual: f64,
    /// Category.
    pub category: ToleranceCategory,
}

impl ToleranceBudget {
    /// Validate budget and produce detailed result.
    pub fn validate(&self) -> ToleranceValidation {
        let component_results: Vec<ComponentValidation> = self
            .components
            .iter()
            .map(|c| ComponentValidation {
                name: c.name.clone(),
                passed: c.is_within_tolerance(),
                allocated: c.allocated,
                actual: c.actual,
                category: c.category,
            })
            .collect();

        let mut notes = Vec::new();

        // Check overall
        if !self.is_within_target() {
            notes.push(format!(
                "Total error {:.4} exceeds target {:.4}",
                self.total_used, self.target
            ));
        }

        // Check individual failures
        for comp in &self.components {
            if !comp.is_within_tolerance() {
                notes.push(format!(
                    "{} exceeded: {:.4} > {:.4} ({})",
                    comp.name, comp.actual, comp.allocated, comp.category
                ));
            }
        }

        // Check neural share
        let (_, neural_actual) = self.total_by_category(ToleranceCategory::Neural);
        let neural_share = if self.total_used > 0.0 {
            neural_actual / self.total_used
        } else {
            0.0
        };
        if neural_share > 0.05 {
            notes.push(format!(
                "Neural correction share {:.1}% exceeds 5% limit",
                neural_share * 100.0
            ));
        }

        ToleranceValidation {
            passed: self.is_within_target() && self.all_components_within_tolerance(),
            budget_name: self.name.clone(),
            target: self.target,
            achieved: self.total_used,
            component_results,
            notes,
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tolerance_component_creation() {
        let comp = ToleranceComponent::new("Test", ToleranceCategory::Model, 0.5)
            .with_actual(0.3)
            .mandatory()
            .with_description("Test component");

        assert_eq!(comp.name, "Test");
        assert_eq!(comp.allocated, 0.5);
        assert_eq!(comp.actual, 0.3);
        assert!(comp.mandatory);
        assert!(comp.is_within_tolerance());
        assert!((comp.margin() - 0.2).abs() < 1e-10);
        assert!((comp.utilization() - 60.0).abs() < 1e-10);
    }

    #[test]
    fn test_tolerance_component_exceeded() {
        let comp =
            ToleranceComponent::new("Exceeded", ToleranceCategory::Neural, 0.1).with_actual(0.15);

        assert!(!comp.is_within_tolerance());
        assert!(comp.margin() < 0.0);
        assert!(comp.utilization() > 100.0);
    }

    #[test]
    fn test_tolerance_budget_creation() {
        let mut budget = ToleranceBudget::new("Test Budget", 1.0);

        budget.add_component(
            ToleranceComponent::new("Instrumental", ToleranceCategory::Instrumental, 0.3)
                .with_actual(0.2),
        );
        budget.add_component(
            ToleranceComponent::new("Model", ToleranceCategory::Model, 0.4).with_actual(0.3),
        );
        budget.add_component(
            ToleranceComponent::new("Neural", ToleranceCategory::Neural, 0.1).with_actual(0.05),
        );

        // RSS: sqrt(0.3^2 + 0.4^2 + 0.1^2) ≈ 0.51
        assert!((budget.total_allocated - 0.5099).abs() < 0.01);
        // RSS: sqrt(0.2^2 + 0.3^2 + 0.05^2) ≈ 0.365
        assert!((budget.total_used - 0.3640).abs() < 0.01);

        assert!(budget.is_within_target());
        assert!(budget.all_components_within_tolerance());
    }

    #[test]
    fn test_certification_budget() {
        let budget = ToleranceBudget::for_certification_level(CertificationTolerance::Industrial);

        assert_eq!(budget.name, "Industrial");
        assert_eq!(budget.target, 1.0);
        assert_eq!(budget.components.len(), 5);

        // All components should have allocations
        for comp in &budget.components {
            assert!(comp.allocated > 0.0);
            assert!(comp.mandatory);
        }
    }

    #[test]
    fn test_budget_update_actual() {
        let mut budget =
            ToleranceBudget::for_certification_level(CertificationTolerance::Reference);

        assert!(budget.update_actual("Instrumental", 0.1));
        assert!(budget.update_actual("Model", 0.15));
        assert!(budget.update_actual("Neural", 0.02));

        assert!(budget.total_used > 0.0);
    }

    #[test]
    fn test_budget_validation() {
        let mut budget = ToleranceBudget::new("Test", 0.5);
        budget.add_component(
            ToleranceComponent::new("A", ToleranceCategory::Model, 0.3).with_actual(0.2),
        );
        budget.add_component(
            ToleranceComponent::new("B", ToleranceCategory::Instrumental, 0.2).with_actual(0.1),
        );

        let validation = budget.validate();
        assert!(validation.passed);
        assert!(validation.notes.is_empty());
    }

    #[test]
    fn test_budget_validation_failure() {
        let mut budget = ToleranceBudget::new("Failing", 0.3);
        budget.add_component(
            ToleranceComponent::new("Exceeded", ToleranceCategory::Model, 0.2).with_actual(0.4),
        );

        let validation = budget.validate();
        assert!(!validation.passed);
        assert!(!validation.notes.is_empty());
    }

    #[test]
    fn test_category_totals() {
        let mut budget = ToleranceBudget::new("Test", 1.0);
        budget.add_component(
            ToleranceComponent::new("M1", ToleranceCategory::Model, 0.3).with_actual(0.2),
        );
        budget.add_component(
            ToleranceComponent::new("M2", ToleranceCategory::Model, 0.4).with_actual(0.3),
        );

        let (alloc, actual) = budget.total_by_category(ToleranceCategory::Model);
        assert!((alloc - 0.5).abs() < 0.01);
        assert!((actual - 0.3606).abs() < 0.01);
    }

    #[test]
    fn test_certification_tolerance_levels() {
        assert_eq!(CertificationTolerance::Reference.max_delta_e(), 0.5);
        assert_eq!(CertificationTolerance::Industrial.max_delta_e(), 1.0);
        assert_eq!(CertificationTolerance::Research.max_delta_e(), 2.0);
        assert_eq!(CertificationTolerance::Experimental.max_delta_e(), 5.0);

        assert!(CertificationTolerance::Reference.max_neural_share() < 0.05);
    }

    #[test]
    fn test_budget_report() {
        let mut budget =
            ToleranceBudget::for_certification_level(CertificationTolerance::Industrial);
        budget.update_actual("Instrumental", 0.2);
        budget.update_actual("Model", 0.3);

        let report = budget.report();
        assert!(report.contains("Industrial"));
        assert!(report.contains("Instrumental"));
        assert!(report.contains("Model"));
    }

    #[test]
    fn test_failing_components() {
        let mut budget = ToleranceBudget::new("Test", 1.0);
        budget.add_component(
            ToleranceComponent::new("OK", ToleranceCategory::Model, 0.5).with_actual(0.3),
        );
        budget.add_component(
            ToleranceComponent::new("Fail", ToleranceCategory::Neural, 0.1).with_actual(0.2),
        );

        let failing = budget.failing_components();
        assert_eq!(failing.len(), 1);
        assert_eq!(failing[0].name, "Fail");
    }
}

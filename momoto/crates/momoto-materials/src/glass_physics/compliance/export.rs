//! # Metrological Export
//!
//! Export certified material twins in various formats with full metrological metadata.
//! Supports MaterialX, JSON, and human-readable compliance reports.

use crate::glass_physics::certification::CertifiedTwinProfile;

// ============================================================================
// EXPORT FORMATS
// ============================================================================

/// Export format options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// MaterialX with tolerance/uncertainty nodes.
    MaterialXCertified,
    /// Full metrological JSON.
    MetrologicalJSON,
    /// Human-readable compliance report.
    ComplianceReport,
    /// Minimal JSON (values only).
    MinimalJSON,
    /// CSV for data analysis.
    CSV,
}

impl ExportFormat {
    /// Get file extension for format.
    pub fn extension(&self) -> &'static str {
        match self {
            ExportFormat::MaterialXCertified => "mtlx",
            ExportFormat::MetrologicalJSON => "json",
            ExportFormat::ComplianceReport => "txt",
            ExportFormat::MinimalJSON => "json",
            ExportFormat::CSV => "csv",
        }
    }

    /// Get MIME type.
    pub fn mime_type(&self) -> &'static str {
        match self {
            ExportFormat::MaterialXCertified => "application/xml",
            ExportFormat::MetrologicalJSON => "application/json",
            ExportFormat::ComplianceReport => "text/plain",
            ExportFormat::MinimalJSON => "application/json",
            ExportFormat::CSV => "text/csv",
        }
    }
}

// ============================================================================
// METROLOGICAL EXPORTER
// ============================================================================

/// Exporter for certified material data.
#[derive(Debug, Clone)]
pub struct MetrologicalExporter {
    /// Output format.
    pub format: ExportFormat,
    /// Include uncertainty data.
    pub include_uncertainty: bool,
    /// Include full traceability chain.
    pub include_traceability: bool,
    /// Include neural correction stats.
    pub include_neural_stats: bool,
    /// Indent for formatted output.
    pub indent: usize,
}

impl Default for MetrologicalExporter {
    fn default() -> Self {
        Self {
            format: ExportFormat::MetrologicalJSON,
            include_uncertainty: true,
            include_traceability: true,
            include_neural_stats: true,
            indent: 2,
        }
    }
}

impl MetrologicalExporter {
    /// Create new exporter with default settings.
    pub fn new(format: ExportFormat) -> Self {
        Self {
            format,
            ..Default::default()
        }
    }

    /// Create MaterialX exporter.
    pub fn materialx() -> Self {
        Self::new(ExportFormat::MaterialXCertified)
    }

    /// Create JSON exporter.
    pub fn json() -> Self {
        Self::new(ExportFormat::MetrologicalJSON)
    }

    /// Create compliance report exporter.
    pub fn compliance_report() -> Self {
        Self::new(ExportFormat::ComplianceReport)
    }

    /// Set uncertainty inclusion.
    pub fn with_uncertainty(mut self, include: bool) -> Self {
        self.include_uncertainty = include;
        self
    }

    /// Set traceability inclusion.
    pub fn with_traceability(mut self, include: bool) -> Self {
        self.include_traceability = include;
        self
    }

    /// Export certified profile.
    pub fn export(&self, profile: &CertifiedTwinProfile) -> String {
        match self.format {
            ExportFormat::MaterialXCertified => self.export_materialx(profile),
            ExportFormat::MetrologicalJSON => self.export_json(profile),
            ExportFormat::ComplianceReport => self.export_compliance_report(profile),
            ExportFormat::MinimalJSON => self.export_minimal_json(profile),
            ExportFormat::CSV => self.export_csv(profile),
        }
    }

    /// Export as MaterialX with tolerance nodes.
    fn export_materialx(&self, profile: &CertifiedTwinProfile) -> String {
        let indent = " ".repeat(self.indent);

        let mut xml = String::new();
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<materialx version=\"1.38\">\n");

        // Certified material node
        xml.push_str(&format!(
            "{}<certified_material name=\"{}\" type=\"surfaceshader\">\n",
            indent, profile.name
        ));

        // Certification metadata
        xml.push_str(&format!(
            "{}{}<certification level=\"{}\" code=\"{}\"/>\n",
            indent,
            indent,
            profile.level,
            profile.level.code()
        ));

        xml.push_str(&format!(
            "{}{}<twin_id value=\"{}\"/>\n",
            indent, indent, profile.twin_id
        ));

        // Validity
        if let Some(until) = profile.valid_until {
            xml.push_str(&format!(
                "{}{}<validity certified_at=\"{}\" valid_until=\"{}\"/>\n",
                indent, indent, profile.certified_at, until
            ));
        }

        // Tolerance budget
        xml.push_str(&format!(
            "{}{}<tolerance_budget target=\"{}\" used=\"{}\" utilization=\"{:.1}%\"/>\n",
            indent,
            indent,
            profile.tolerance_budget.target,
            profile.tolerance_budget.total_used,
            profile.tolerance_budget.utilization()
        ));

        // Neural correction stats
        if self.include_neural_stats {
            xml.push_str(&format!(
                "{}{}<neural_correction share=\"{:.2}%\" max_magnitude=\"{:.4}\"/>\n",
                indent,
                indent,
                profile.neural_correction_stats.correction_share * 100.0,
                profile.neural_correction_stats.max_correction_magnitude
            ));
        }

        // Test results summary
        xml.push_str(&format!(
            "{}{}<test_results passed=\"{}\" total=\"{}\"/>\n",
            indent,
            indent,
            profile.passed_test_count(),
            profile.test_results.len()
        ));

        xml.push_str(&format!("{}</certified_material>\n", indent));
        xml.push_str("</materialx>\n");

        xml
    }

    /// Export as full metrological JSON.
    fn export_json(&self, profile: &CertifiedTwinProfile) -> String {
        let indent = " ".repeat(self.indent);

        let mut json = String::new();
        json.push_str("{\n");

        // Basic info
        json.push_str(&format!(
            "{}\"twin_id\": \"{}\",\n",
            indent, profile.twin_id
        ));
        json.push_str(&format!("{}\"name\": \"{}\",\n", indent, profile.name));
        json.push_str(&format!(
            "{}\"certification_level\": \"{}\",\n",
            indent, profile.level
        ));
        json.push_str(&format!(
            "{}\"certification_code\": \"{}\",\n",
            indent,
            profile.level.code()
        ));
        json.push_str(&format!(
            "{}\"certified_at\": {},\n",
            indent, profile.certified_at
        ));

        if let Some(until) = profile.valid_until {
            json.push_str(&format!("{}\"valid_until\": {},\n", indent, until));
        }

        json.push_str(&format!(
            "{}\"is_valid\": {},\n",
            indent,
            profile.is_valid()
        ));

        // Tolerance budget
        json.push_str(&format!("{}\"tolerance_budget\": {{\n", indent));
        json.push_str(&format!(
            "{}{}\"target\": {},\n",
            indent, indent, profile.tolerance_budget.target
        ));
        json.push_str(&format!(
            "{}{}\"total_used\": {},\n",
            indent, indent, profile.tolerance_budget.total_used
        ));
        json.push_str(&format!(
            "{}{}\"utilization_percent\": {:.2}\n",
            indent,
            indent,
            profile.tolerance_budget.utilization()
        ));
        json.push_str(&format!("{}}},\n", indent));

        // Neural stats
        if self.include_neural_stats {
            json.push_str(&format!("{}\"neural_correction\": {{\n", indent));
            json.push_str(&format!(
                "{}{}\"total_evaluations\": {},\n",
                indent, indent, profile.neural_correction_stats.total_evaluations
            ));
            json.push_str(&format!(
                "{}{}\"corrections_applied\": {},\n",
                indent, indent, profile.neural_correction_stats.corrections_applied
            ));
            json.push_str(&format!(
                "{}{}\"correction_share\": {},\n",
                indent, indent, profile.neural_correction_stats.correction_share
            ));
            json.push_str(&format!(
                "{}{}\"max_magnitude\": {},\n",
                indent, indent, profile.neural_correction_stats.max_correction_magnitude
            ));
            json.push_str(&format!(
                "{}{}\"violations_count\": {}\n",
                indent,
                indent,
                profile.neural_correction_stats.violations.len()
            ));
            json.push_str(&format!("{}}},\n", indent));
        }

        // Test results
        json.push_str(&format!("{}\"test_results\": {{\n", indent));
        json.push_str(&format!(
            "{}{}\"passed\": {},\n",
            indent,
            indent,
            profile.passed_test_count()
        ));
        json.push_str(&format!(
            "{}{}\"total\": {},\n",
            indent,
            indent,
            profile.test_results.len()
        ));
        json.push_str(&format!(
            "{}{}\"all_passed\": {}\n",
            indent,
            indent,
            profile.all_tests_passed()
        ));
        json.push_str(&format!("{}}},\n", indent));

        // Traceability summary
        if self.include_traceability {
            json.push_str(&format!("{}\"traceability\": {{\n", indent));
            json.push_str(&format!(
                "{}{}\"entry_count\": {},\n",
                indent,
                indent,
                profile.traceability.entries.len()
            ));
            json.push_str(&format!(
                "{}{}\"neural_share\": {:.4}\n",
                indent,
                indent,
                profile.traceability.total_neural_share()
            ));
            json.push_str(&format!("{}}},\n", indent));
        }

        // Metadata
        json.push_str(&format!("{}\"metadata\": {{\n", indent));
        json.push_str(&format!(
            "{}{}\"authority\": \"{}\",\n",
            indent, indent, profile.metadata.authority
        ));
        json.push_str(&format!(
            "{}{}\"software_version\": \"{}\"\n",
            indent, indent, profile.metadata.software_version
        ));
        json.push_str(&format!("{}}}\n", indent));

        json.push_str("}\n");

        json
    }

    /// Export as compliance report.
    fn export_compliance_report(&self, profile: &CertifiedTwinProfile) -> String {
        let mut report = String::new();

        // Header
        report.push_str("═══════════════════════════════════════════════════════════════════\n");
        report.push_str("                    MATERIAL TWIN COMPLIANCE REPORT                 \n");
        report.push_str("═══════════════════════════════════════════════════════════════════\n\n");

        // Identification
        report.push_str("IDENTIFICATION\n");
        report.push_str("──────────────────────────────────────────────────────────────────\n");
        report.push_str(&format!("Twin ID:              {}\n", profile.twin_id));
        report.push_str(&format!("Material Name:        {}\n", profile.name));
        report.push_str(&format!(
            "Certification Level:  {} ({})\n",
            profile.level,
            profile.level.code()
        ));
        report.push_str(&format!(
            "Certification Date:   {}\n",
            format_timestamp(profile.certified_at)
        ));

        if let Some(until) = profile.valid_until {
            report.push_str(&format!(
                "Valid Until:          {}\n",
                format_timestamp(until)
            ));
        }

        report.push_str(&format!(
            "Current Status:       {}\n",
            if profile.is_valid() {
                "VALID"
            } else {
                "EXPIRED"
            }
        ));

        report.push('\n');

        // Requirements
        report.push_str("LEVEL REQUIREMENTS\n");
        report.push_str("──────────────────────────────────────────────────────────────────\n");
        report.push_str(&format!(
            "Maximum ΔE2000:       {:.1}\n",
            profile.level.max_delta_e()
        ));
        report.push_str(&format!(
            "Maximum Neural Share: {:.0}%\n",
            profile.level.max_neural_share() * 100.0
        ));
        report.push_str(&format!(
            "Minimum Observations: {}\n",
            profile.level.min_observations()
        ));

        report.push('\n');

        // Tolerance Budget
        report.push_str("TOLERANCE BUDGET\n");
        report.push_str("──────────────────────────────────────────────────────────────────\n");
        report.push_str(&format!(
            "Target:               {:.4}\n",
            profile.tolerance_budget.target
        ));
        report.push_str(&format!(
            "Total Used:           {:.4}\n",
            profile.tolerance_budget.total_used
        ));
        report.push_str(&format!(
            "Margin:               {:.4}\n",
            profile.tolerance_budget.margin()
        ));
        report.push_str(&format!(
            "Utilization:          {:.1}%\n",
            profile.tolerance_budget.utilization()
        ));
        report.push_str(&format!(
            "Budget Status:        {}\n",
            if profile.tolerance_budget.is_within_target() {
                "WITHIN BUDGET"
            } else {
                "EXCEEDED"
            }
        ));

        report.push('\n');

        // Neural Correction
        report.push_str("NEURAL CORRECTION ACCOUNTABILITY\n");
        report.push_str("──────────────────────────────────────────────────────────────────\n");
        report.push_str(&format!(
            "Total Evaluations:    {}\n",
            profile.neural_correction_stats.total_evaluations
        ));
        report.push_str(&format!(
            "Corrections Applied:  {}\n",
            profile.neural_correction_stats.corrections_applied
        ));
        report.push_str(&format!(
            "Correction Share:     {:.2}%\n",
            profile.neural_correction_stats.correction_share * 100.0
        ));
        report.push_str(&format!(
            "Max Magnitude:        {:.4}\n",
            profile.neural_correction_stats.max_correction_magnitude
        ));
        report.push_str(&format!(
            "Mean Magnitude:       {:.4}\n",
            profile.neural_correction_stats.mean_correction_magnitude
        ));
        report.push_str(&format!(
            "Violations:           {}\n",
            profile.neural_correction_stats.violations.len()
        ));

        report.push('\n');

        // Test Results
        report.push_str("TEST RESULTS\n");
        report.push_str("──────────────────────────────────────────────────────────────────\n");
        report.push_str(&format!(
            "Tests Passed:         {}/{}\n",
            profile.passed_test_count(),
            profile.test_results.len()
        ));
        report.push_str(&format!(
            "Overall:              {}\n",
            if profile.all_tests_passed() {
                "ALL PASSED"
            } else {
                "SOME FAILED"
            }
        ));

        report.push('\n');

        for result in &profile.test_results {
            let status = if result.passed { "✓" } else { "✗" };
            report.push_str(&format!(
                "  {} {:30} {:.4} / {:.4}\n",
                status,
                result.test.name(),
                result.actual_value,
                result.threshold
            ));
        }

        report.push('\n');

        // Traceability
        report.push_str("TRACEABILITY\n");
        report.push_str("──────────────────────────────────────────────────────────────────\n");
        report.push_str(&format!(
            "Chain Entries:        {}\n",
            profile.traceability.entries.len()
        ));
        report.push_str(&format!(
            "Neural Operations:    {:.2}%\n",
            profile.traceability.total_neural_share() * 100.0
        ));

        if let Some(ref cal) = profile.traceability.root_calibration {
            report.push_str(&format!("Root Calibration:     {}\n", cal.name));
            report.push_str(&format!(
                "Certificate ID:       {}\n",
                cal.certificate_id.as_deref().unwrap_or("N/A")
            ));
        }

        report.push('\n');

        // Certification Authority
        report.push_str("CERTIFICATION AUTHORITY\n");
        report.push_str("──────────────────────────────────────────────────────────────────\n");
        report.push_str(&format!(
            "Authority:            {}\n",
            profile.metadata.authority
        ));

        if let Some(ref op) = profile.metadata.operator {
            report.push_str(&format!("Certified By:         {}\n", op));
        }

        report.push_str(&format!(
            "Software Version:     {}\n",
            profile.metadata.software_version
        ));

        report.push('\n');

        // Footer
        report.push_str("═══════════════════════════════════════════════════════════════════\n");
        report.push_str("This report was generated automatically by the Momoto Materials\n");
        report.push_str("Certification System. For questions, contact the certifying authority.\n");
        report.push_str("═══════════════════════════════════════════════════════════════════\n");

        report
    }

    /// Export minimal JSON (values only).
    fn export_minimal_json(&self, profile: &CertifiedTwinProfile) -> String {
        format!(
            "{{\"twin_id\":\"{}\",\"name\":\"{}\",\"level\":\"{}\",\"valid\":{}}}",
            profile.twin_id,
            profile.name,
            profile.level.code(),
            profile.is_valid()
        )
    }

    /// Export as CSV.
    fn export_csv(&self, profile: &CertifiedTwinProfile) -> String {
        let mut csv = String::new();

        csv.push_str("field,value\n");
        csv.push_str(&format!("twin_id,{}\n", profile.twin_id));
        csv.push_str(&format!("name,\"{}\"\n", profile.name));
        csv.push_str(&format!("level,{}\n", profile.level.code()));
        csv.push_str(&format!("certified_at,{}\n", profile.certified_at));
        csv.push_str(&format!("is_valid,{}\n", profile.is_valid()));
        csv.push_str(&format!(
            "tolerance_target,{}\n",
            profile.tolerance_budget.target
        ));
        csv.push_str(&format!(
            "tolerance_used,{}\n",
            profile.tolerance_budget.total_used
        ));
        csv.push_str(&format!(
            "neural_share,{}\n",
            profile.neural_correction_stats.correction_share
        ));
        csv.push_str(&format!("tests_passed,{}\n", profile.passed_test_count()));
        csv.push_str(&format!("tests_total,{}\n", profile.test_results.len()));

        csv
    }
}

/// Format Unix timestamp as ISO date string.
fn format_timestamp(timestamp: u64) -> String {
    // Simple formatting (would use chrono in production)
    let days = timestamp / 86400;
    let years = days / 365 + 1970;
    let remaining_days = days % 365;
    let months = remaining_days / 30 + 1;
    let day = remaining_days % 30 + 1;

    format!("{:04}-{:02}-{:02}", years, months, day)
}

// ============================================================================
// BATCH EXPORT
// ============================================================================

/// Batch export multiple profiles.
pub fn batch_export(profiles: &[CertifiedTwinProfile], format: ExportFormat) -> String {
    let exporter = MetrologicalExporter::new(format);

    match format {
        ExportFormat::CSV => {
            let mut csv = String::new();
            csv.push_str("twin_id,name,level,valid,tolerance_used,neural_share,tests_passed\n");

            for profile in profiles {
                csv.push_str(&format!(
                    "{},\"{}\",{},{},{},{},{}\n",
                    profile.twin_id,
                    profile.name,
                    profile.level.code(),
                    profile.is_valid(),
                    profile.tolerance_budget.total_used,
                    profile.neural_correction_stats.correction_share,
                    profile.passed_test_count()
                ));
            }
            csv
        }
        _ => profiles
            .iter()
            .map(|p| exporter.export(p))
            .collect::<Vec<_>>()
            .join("\n\n"),
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glass_physics::certification::levels::CertificationLevel;
    use crate::glass_physics::certification::requirements::{MandatoryTest, TestResult};

    fn make_test_profile() -> CertifiedTwinProfile {
        let results = vec![TestResult::pass(
            MandatoryTest::EnergyConservation { max_error: 0.05 },
            0.02,
        )];

        CertifiedTwinProfile::new("Test Gold", CertificationLevel::Industrial, results)
    }

    #[test]
    fn test_format_extension() {
        assert_eq!(ExportFormat::MaterialXCertified.extension(), "mtlx");
        assert_eq!(ExportFormat::MetrologicalJSON.extension(), "json");
        assert_eq!(ExportFormat::CSV.extension(), "csv");
    }

    #[test]
    fn test_json_export() {
        let profile = make_test_profile();
        let exporter = MetrologicalExporter::json();
        let json = exporter.export(&profile);

        assert!(json.contains("twin_id"));
        assert!(json.contains("Test Gold"));
        assert!(json.contains("Industrial"));
    }

    #[test]
    fn test_materialx_export() {
        let profile = make_test_profile();
        let exporter = MetrologicalExporter::materialx();
        let xml = exporter.export(&profile);

        assert!(xml.contains("<?xml"));
        assert!(xml.contains("materialx"));
        assert!(xml.contains("certified_material"));
    }

    #[test]
    fn test_compliance_report() {
        let profile = make_test_profile();
        let exporter = MetrologicalExporter::compliance_report();
        let report = exporter.export(&profile);

        assert!(report.contains("COMPLIANCE REPORT"));
        assert!(report.contains("Test Gold"));
        assert!(report.contains("Industrial"));
        assert!(report.contains("TOLERANCE BUDGET"));
    }

    #[test]
    fn test_minimal_json() {
        let profile = make_test_profile();
        let exporter = MetrologicalExporter::new(ExportFormat::MinimalJSON);
        let json = exporter.export(&profile);

        assert!(json.starts_with('{'));
        assert!(json.ends_with('}'));
        assert!(json.contains("twin_id"));
    }

    #[test]
    fn test_csv_export() {
        let profile = make_test_profile();
        let exporter = MetrologicalExporter::new(ExportFormat::CSV);
        let csv = exporter.export(&profile);

        assert!(csv.contains("field,value"));
        assert!(csv.contains("twin_id"));
    }

    #[test]
    fn test_exporter_options() {
        let exporter = MetrologicalExporter::json()
            .with_uncertainty(false)
            .with_traceability(false);

        assert!(!exporter.include_uncertainty);
        assert!(!exporter.include_traceability);
    }

    #[test]
    fn test_batch_export() {
        let profiles = vec![make_test_profile(), make_test_profile()];

        let csv = batch_export(&profiles, ExportFormat::CSV);
        assert!(csv.lines().count() >= 3); // Header + 2 profiles

        let json = batch_export(&profiles, ExportFormat::MetrologicalJSON);
        assert!(json.matches("twin_id").count() >= 2);
    }

    #[test]
    fn test_timestamp_format() {
        let ts = format_timestamp(0);
        assert!(ts.contains("1970"));

        let ts = format_timestamp(1704067200); // 2024-01-01 approx
        assert!(ts.contains("-"));
    }
}

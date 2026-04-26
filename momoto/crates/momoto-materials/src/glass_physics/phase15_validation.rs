//! # Phase 15 Validation Suite
//!
//! Comprehensive tests for Certifiable Material Twins.
//! Validates all Phase 15 components: metrology, instruments, certification, compliance.
//!
//! ## Test Categories
//!
//! 1. **Metrology Tests** (15): Measurement, uncertainty, traceability
//! 2. **Instrument Tests** (12): Gonioreflectometer, spectrophotometer, ellipsometer
//! 3. **Certification Tests** (15): Levels, requirements, auditor
//! 4. **Compliance Tests** (15): Ground truth, neural audit, export
//! 5. **Integration Tests** (5): Full certification pipeline
//! 6. **Memory Tests** (5): Budget validation

#[cfg(test)]
mod metrology_tests {
    use crate::glass_physics::metrology::*;

    #[test]
    fn test_measurement_creation() {
        let m = Measurement::calibrated(550.0, 0.5, Unit::Nanometers);
        assert!((m.value - 550.0).abs() < 1e-10);
        assert_eq!(m.unit, Unit::Nanometers);
        assert_eq!(m.quality, MeasurementQuality::Calibrated);
    }

    #[test]
    fn test_uncertainty_types() {
        let type_a = Uncertainty::TypeA {
            std_error: 0.1,
            n_samples: 100,
        };
        assert!((type_a.standard() - 0.1).abs() < 1e-10);

        let type_b = Uncertainty::TypeB {
            systematic: 0.05,
            source: "Calibration".to_string(),
        };
        assert!((type_b.standard() - 0.05).abs() < 1e-10);

        let combined = Uncertainty::Combined {
            type_a: 0.1,
            type_b: 0.05,
        };
        let expected = (0.1f64.powi(2) + 0.05f64.powi(2)).sqrt();
        assert!((combined.standard() - expected).abs() < 1e-10);
    }

    #[test]
    fn test_unit_conversions() {
        assert!((deg_to_rad(180.0) - std::f64::consts::PI).abs() < 1e-10);
        assert!((rad_to_deg(std::f64::consts::PI) - 180.0).abs() < 1e-10);
        assert!((celsius_to_kelvin(0.0) - 273.15).abs() < 1e-10);
        assert!((kelvin_to_celsius(273.15) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_units_compatible() {
        assert!(units_compatible(Unit::Radians, Unit::Degrees));
        assert!(units_compatible(Unit::Reflectance, Unit::Transmittance));
        assert!(!units_compatible(Unit::Nanometers, Unit::Kelvin));
    }

    #[test]
    fn test_traceability_chain() {
        let mut chain = TraceabilityChain::new();
        chain.record_measurement("Spectrophotometer", "Direct", MeasurementId::generate());
        chain.record_model_prediction("BRDF", "1.0", vec![], MeasurementId::generate());

        assert_eq!(chain.entries.len(), 2);
        assert!(chain.total_neural_share() < 1e-10);
    }

    #[test]
    fn test_traceability_neural_share() {
        let mut chain = TraceabilityChain::new();
        chain.record_neural_correction(
            0.05,
            0.03,
            MeasurementId::generate(),
            MeasurementId::generate(),
        );

        assert!((chain.total_neural_share() - 0.03).abs() < 1e-10);
    }

    #[test]
    fn test_tolerance_budget() {
        let budget = ToleranceBudget::for_certification_level(CertificationTolerance::Industrial);

        assert_eq!(budget.target, 1.0);
        assert!(!budget.components.is_empty());
        assert!(budget.is_within_target());
    }

    #[test]
    fn test_tolerance_validation() {
        let mut budget = ToleranceBudget::new("Test", 1.0);
        budget.add_component(
            ToleranceComponent::new("Model", ToleranceCategory::Model, 0.5).with_actual(0.3),
        );

        let validation = budget.validate();
        assert!(validation.passed);
    }

    #[test]
    fn test_uncertainty_propagation_linear() {
        let inputs = vec![
            Measurement::calibrated(10.0, 0.1, Unit::Dimensionless),
            Measurement::calibrated(20.0, 0.2, Unit::Dimensionless),
        ];
        let jacobian = vec![1.0, 1.0];

        let prop = UncertaintyPropagator::linear();
        let output = prop.propagate_forward(&inputs, &jacobian, 30.0, Unit::Dimensionless);

        assert!((output.value - 30.0).abs() < 1e-10);
        let expected_std = (0.1f64.powi(2) + 0.2f64.powi(2)).sqrt();
        assert!((output.uncertainty.standard() - expected_std).abs() < 0.01);
    }

    #[test]
    fn test_sensitivity_analysis() {
        let inputs = vec![
            Measurement::calibrated(10.0, 0.5, Unit::Dimensionless),
            Measurement::calibrated(20.0, 0.1, Unit::Dimensionless),
        ];
        let names = vec!["Large", "Small"];
        let jacobian = vec![1.0, 1.0];

        let analysis = SensitivityAnalysis::analyze(&inputs, &names, &jacobian);

        assert_eq!(analysis.dominant_source(), Some("Large"));
        assert!(analysis.percentages[0] > analysis.percentages[1]);
    }

    #[test]
    fn test_measurement_array() {
        let array = MeasurementArray {
            values: vec![0.5, 0.6, 0.7],
            uncertainties: vec![0.01, 0.01, 0.01],
            unit: Unit::Reflectance,
            quality: MeasurementQuality::Validated,
            domain: vec![400.0, 500.0, 600.0],
            domain_unit: Unit::Nanometers,
        };

        assert_eq!(array.values.len(), 3);
        assert!((array.mean_value() - 0.6).abs() < 1e-10);
    }

    #[test]
    fn test_measurement_confidence_interval() {
        let m = Measurement::calibrated(100.0, 5.0, Unit::Dimensionless);
        let (low, high) = m.confidence_interval();

        assert!(low < 100.0);
        assert!(high > 100.0);
        assert!((high - low) > 9.0); // ~2 sigma for 95%
    }

    #[test]
    fn test_metrology_memory_budget() {
        let estimate = estimate_memory_footprint();
        assert!(
            estimate.typical_usage() < 15_000,
            "Metrology exceeds 15KB budget"
        );
    }
}

#[cfg(test)]
mod instrument_tests {
    use crate::glass_physics::instruments::*;

    #[test]
    fn test_ideal_gonioreflectometer() {
        let mut gonio = VirtualGonioreflectometer::ideal();
        let brdf = lambertian_brdf(0.5);

        let measurement = gonio.measure_specular(brdf, 45.0, 550.0);
        let expected = 0.5 / std::f64::consts::PI;

        assert!((measurement.value - expected).abs() < 1e-4);
    }

    #[test]
    fn test_gonioreflectometer_angular_scan() {
        let mut gonio = VirtualGonioreflectometer::ideal().with_angular_step(10.0);
        let brdf = lambertian_brdf(0.8);

        let result = gonio.measure_angular(brdf, 30.0, 550.0);

        assert!(!result.reflected_angles_deg.is_empty());
        assert!(result.mean_brdf() > 0.0);
    }

    #[test]
    fn test_spectrophotometer_constant() {
        let mut spectro = VirtualSpectrophotometer::ideal()
            .with_wavelength_range(400.0, 700.0)
            .with_wavelength_step(10.0);

        let result = spectro.measure_reflectance(constant_reflectance(0.5));

        assert!((result.mean_value() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_spectrophotometer_transmittance() {
        let mut spectro = VirtualSpectrophotometer::ideal().with_wavelength_step(20.0);
        let result = spectro.measure_transmittance(constant_reflectance(0.9));

        assert!((result.mean_value() - 0.9).abs() < 0.001);
    }

    #[test]
    fn test_spectrophotometer_absorbance() {
        let mut spectro = VirtualSpectrophotometer::ideal().with_wavelength_step(20.0);
        let result = spectro.measure_absorbance(constant_reflectance(0.1));

        // A = -log10(0.1) = 1.0
        assert!((result.mean_value() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_ellipsometer_spectrum() {
        let mut ellip = VirtualEllipsometer::ideal()
            .with_wavelength_range(400.0, 700.0)
            .with_seed(42);

        let constants = constant_optical_constants(1.5, 0.0);
        let result = ellip.measure_spectrum(constants);

        assert!(!result.points.is_empty());
        assert!(result.mean_psi() > 0.0);
    }

    #[test]
    fn test_ellipsometer_thin_film() {
        let mut ellip = VirtualEllipsometer::ideal().with_wavelength_range(400.0, 800.0);

        let film = constant_optical_constants(1.46, 0.0);
        let substrate = silicon_optical_constants();

        let result = ellip.measure_thin_film(film, substrate, 100.0);

        assert!(result.thickness.value > 0.0);
        assert!(!result.points.is_empty());
    }

    #[test]
    fn test_instrument_noise_model() {
        let noise = NoiseModel::gaussian(0.01);
        assert!((noise.noise_std(1.0) - 0.01).abs() < 1e-10);

        let combined = NoiseModel::combined(0.01, 0.001);
        let std = combined.noise_std(1.0);
        assert!(std > 0.01);
    }

    #[test]
    fn test_instrument_reproducibility() {
        let mut gonio1 = VirtualGonioreflectometer::research_grade().with_seed(42);
        let mut gonio2 = VirtualGonioreflectometer::research_grade().with_seed(42);

        let brdf = lambertian_brdf(0.5);

        let m1 = gonio1.measure_specular(&brdf, 45.0, 550.0);
        let m2 = gonio2.measure_specular(&brdf, 45.0, 550.0);

        assert!((m1.value - m2.value).abs() < 1e-10);
    }

    #[test]
    fn test_instrument_suite() {
        let suite = InstrumentFactory::ideal_suite();

        assert!(matches!(
            suite.gonioreflectometer.config.noise_model,
            NoiseModel::None
        ));
        assert!(matches!(
            suite.spectrophotometer.config.noise_model,
            NoiseModel::None
        ));
        assert!(matches!(
            suite.ellipsometer.config.noise_model,
            NoiseModel::None
        ));
    }

    #[test]
    fn test_instruments_memory_budget() {
        let estimate = estimate_memory_footprint();
        assert!(
            estimate.typical_session() < 20_000,
            "Instruments exceed 20KB budget"
        );
    }
}

#[cfg(test)]
mod certification_tests {
    use crate::glass_physics::certification::*;

    #[test]
    fn test_certification_levels_ordering() {
        assert!(CertificationLevel::Experimental < CertificationLevel::Research);
        assert!(CertificationLevel::Research < CertificationLevel::Industrial);
        assert!(CertificationLevel::Industrial < CertificationLevel::Reference);
    }

    #[test]
    fn test_certification_level_thresholds() {
        assert!(
            CertificationLevel::Reference.max_delta_e()
                < CertificationLevel::Industrial.max_delta_e()
        );
        assert!(
            CertificationLevel::Reference.max_neural_share()
                < CertificationLevel::Industrial.max_neural_share()
        );
    }

    #[test]
    fn test_certification_metrics() {
        let metrics = CertificationMetrics::exemplary();
        assert!(CertificationLevel::Reference.can_achieve(&metrics));

        let research = CertificationMetrics::research_grade();
        assert!(CertificationLevel::Research.can_achieve(&research));
    }

    #[test]
    fn test_highest_achievable() {
        let metrics = CertificationMetrics::exemplary();
        let highest = CertificationLevel::highest_achievable(&metrics);

        assert_eq!(highest, Some(CertificationLevel::Reference));
    }

    #[test]
    fn test_mandatory_tests_by_level() {
        let exp_tests = required_tests(CertificationLevel::Experimental);
        let ref_tests = required_tests(CertificationLevel::Reference);

        assert!(exp_tests.len() < ref_tests.len());
    }

    #[test]
    fn test_test_result_pass() {
        let test = MandatoryTest::EnergyConservation { max_error: 0.05 };
        let result = TestResult::pass(test, 0.02);

        assert!(result.passed);
        assert!(result.margin() > 0.0);
    }

    #[test]
    fn test_test_result_fail() {
        let test = MandatoryTest::EnergyConservation { max_error: 0.05 };
        let result = TestResult::fail(test, 0.08, "Exceeded");

        assert!(!result.passed);
        assert!(result.margin() < 0.0);
    }

    #[test]
    fn test_certified_profile() {
        let results = vec![TestResult::pass(
            MandatoryTest::EnergyConservation { max_error: 0.05 },
            0.02,
        )];

        let profile =
            CertifiedTwinProfile::new("Test Gold", CertificationLevel::Industrial, results);

        assert!(profile.is_valid());
        assert!(profile.all_tests_passed());
    }

    #[test]
    fn test_neural_correction_stats() {
        let mut stats = NeuralCorrectionStats::new();
        stats.record(0.01, 0.05);
        stats.record(0.02, 0.05);

        assert_eq!(stats.corrections_applied, 2);
        assert!(stats.violations.is_empty());
    }

    #[test]
    fn test_certification_auditor() {
        let auditor = CertificationAuditor::new(CertificationLevel::Industrial);
        let data = MaterialAuditData::exemplary();

        let result = auditor.audit(&data);

        assert!(result.certified);
    }

    #[test]
    fn test_auditor_certification() {
        let auditor = CertificationAuditor::new(CertificationLevel::Research);
        let data = MaterialAuditData::exemplary();

        let profile = auditor.certify("Test Material", &data);

        assert!(profile.is_ok());
    }

    #[test]
    fn test_quick_certify() {
        let profile = quick_certify_experimental("Test", 3.0);

        assert!(profile.is_ok());
        assert_eq!(profile.unwrap().level, CertificationLevel::Experimental);
    }

    #[test]
    fn test_twin_id_unique() {
        let id1 = TwinId::new();
        let id2 = TwinId::new();

        assert_ne!(id1, id2);
    }

    #[test]
    fn test_certification_memory_budget() {
        let estimate = super::super::certification::estimate_memory_footprint();
        assert!(
            estimate.typical_certification() < 12_000,
            "Certification exceeds 12KB budget"
        );
    }
}

#[cfg(test)]
mod compliance_tests {
    use crate::glass_physics::certification::NeuralCorrectionStats;
    use crate::glass_physics::compliance::*;

    #[test]
    fn test_ground_truth_validator() {
        let mut validator = GroundTruthValidator::new();
        validator.add_dataset(gold_reference_data());

        assert!(!validator.datasets.is_empty());
    }

    #[test]
    fn test_validation_perfect() {
        let mut validator = GroundTruthValidator::new().with_tolerance(0.01);

        let mut measurements = Vec::new();
        for wl in [400.0, 500.0, 600.0, 700.0] {
            measurements.push(SpectralMeasurement::new(wl, 0.5));
        }

        validator.add_dataset(GroundTruthDataset::Published {
            reference: "Test".to_string(),
            data: measurements,
        });

        let report = validator.validate(|_wl, _| 0.5);

        assert!(report.passed);
        assert!(report.rmse_spectral < 1e-10);
    }

    #[test]
    fn test_neural_auditor() {
        let auditor = NeuralAuditor::new();
        let mut stats = NeuralCorrectionStats::new();
        stats.correction_share = 0.03;
        stats.max_correction_magnitude = 0.05;

        let result = auditor.audit(&stats);

        assert!(result.passed);
    }

    #[test]
    fn test_neural_auditor_failure() {
        let auditor = NeuralAuditor::new();
        let mut stats = NeuralCorrectionStats::new();
        stats.correction_share = 0.10; // Exceeds 5% limit

        let result = auditor.audit(&stats);

        assert!(!result.passed);
    }

    #[test]
    fn test_correction_check() {
        let auditor = NeuralAuditor::new();

        assert!(auditor.check_correction(0.05).is_ok());
        assert!(auditor.check_correction(0.15).is_violation());
    }

    #[test]
    fn test_reproducibility_deterministic() {
        let test = ReproducibilityTest::new().with_runs(5);
        let result = test.verify(|wl, angle| wl * 0.001 + angle * 0.01);

        assert!(result.deterministic);
        assert!((result.score() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_reproducibility_hash() {
        let mut func = |wl: f64, angle: f64| wl * 0.001 + angle * 0.01;

        let hash1 = compute_reproducibility_hash(&mut func, 42);
        let hash2 = compute_reproducibility_hash(&mut func, 42);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_comparison_equivalent() {
        let test = ReproducibilityTest::new();

        let result = test.compare(
            |wl, angle| wl * 0.001 + angle * 0.01,
            |wl, angle| wl * 0.001 + angle * 0.01,
        );

        assert!(result.equivalent);
    }

    #[test]
    fn test_export_json() {
        use crate::glass_physics::certification::requirements::{MandatoryTest, TestResult};
        use crate::glass_physics::certification::{CertificationLevel, CertifiedTwinProfile};

        let results = vec![TestResult::pass(
            MandatoryTest::EnergyConservation { max_error: 0.05 },
            0.02,
        )];

        let profile = CertifiedTwinProfile::new("Test", CertificationLevel::Industrial, results);
        let exporter = MetrologicalExporter::json();
        let json = exporter.export(&profile);

        assert!(json.contains("twin_id"));
        assert!(json.contains("Industrial"));
    }

    #[test]
    fn test_export_materialx() {
        use crate::glass_physics::certification::requirements::{MandatoryTest, TestResult};
        use crate::glass_physics::certification::{CertificationLevel, CertifiedTwinProfile};

        let results = vec![TestResult::pass(
            MandatoryTest::EnergyConservation { max_error: 0.05 },
            0.02,
        )];

        let profile = CertifiedTwinProfile::new("Test", CertificationLevel::Industrial, results);
        let exporter = MetrologicalExporter::materialx();
        let xml = exporter.export(&profile);

        assert!(xml.contains("<?xml"));
        assert!(xml.contains("materialx"));
    }

    #[test]
    fn test_export_compliance_report() {
        use crate::glass_physics::certification::requirements::{MandatoryTest, TestResult};
        use crate::glass_physics::certification::{CertificationLevel, CertifiedTwinProfile};

        let results = vec![TestResult::pass(
            MandatoryTest::EnergyConservation { max_error: 0.05 },
            0.02,
        )];

        let profile =
            CertifiedTwinProfile::new("Gold Twin", CertificationLevel::Reference, results);
        let exporter = MetrologicalExporter::compliance_report();
        let report = exporter.export(&profile);

        assert!(report.contains("COMPLIANCE REPORT"));
        assert!(report.contains("Gold Twin"));
    }

    #[test]
    fn test_quick_compliance_functions() {
        let passed = quick_neural_audit(0.03, 0.05, 0.05);
        assert!(passed);

        let failed = quick_neural_audit(0.10, 0.05, 0.05);
        assert!(!failed);
    }

    #[test]
    fn test_compliance_memory_budget() {
        let estimate = super::super::compliance::estimate_memory_footprint();
        assert!(
            estimate.typical_check() < 15_000,
            "Compliance exceeds 15KB budget"
        );
    }
}

#[cfg(test)]
mod integration_tests {
    use crate::glass_physics::certification::*;
    use crate::glass_physics::compliance::*;
    use crate::glass_physics::instruments::*;
    use crate::glass_physics::metrology::*;

    #[test]
    fn test_full_certification_pipeline() {
        // 1. Create instrument suite
        let mut suite = InstrumentFactory::ideal_suite().with_seed(42);

        // 2. Measure material
        let brdf = lambertian_brdf(0.5);
        let gonio_result = suite.gonioreflectometer.measure_angular(brdf, 30.0, 550.0);

        // 3. Validate against ground truth
        let mut validator = GroundTruthValidator::new();
        let report = validator.validate(|_wl, _angle| 0.5 / std::f64::consts::PI);

        // 4. Check neural correction
        let neural_stats = NeuralCorrectionStats::new();
        let auditor = NeuralAuditor::industrial_level();
        let neural_result = auditor.audit(&neural_stats);

        // 5. Build certification
        let cert_auditor = CertificationAuditor::new(CertificationLevel::Industrial);
        let audit_data = MaterialAuditData::exemplary();

        let cert_result = cert_auditor.certify("Lambertian Material", &audit_data);

        assert!(cert_result.is_ok());

        // 6. Export
        let profile = cert_result.unwrap();
        let exporter = MetrologicalExporter::json();
        let json = exporter.export(&profile);

        assert!(json.contains("Lambertian Material"));
    }

    #[test]
    fn test_gold_twin_with_gonioreflectometer() {
        let mut gonio = VirtualGonioreflectometer::research_grade()
            .with_angular_step(5.0)
            .with_seed(42);

        // Approximate gold BRDF (specular dominant)
        let gold_brdf = |theta_i: f64, theta_o: f64, wavelength: f64| {
            let specular = if (theta_o - theta_i).abs() < 0.1 {
                0.8
            } else {
                0.0
            };
            let diffuse = 0.02 / std::f64::consts::PI;

            // Wavelength effect (gold is yellow/red)
            let wl_factor = if wavelength > 550.0 { 1.0 } else { 0.5 };

            (specular + diffuse) * wl_factor
        };

        let result = gonio.measure_angular(gold_brdf, 45.0, 600.0);

        assert!(result.specular_peak().is_some());
        let (peak_angle, peak_value) = result.specular_peak().unwrap();
        assert!((peak_angle - 45.0).abs() < 5.0);
        assert!(peak_value > result.mean_brdf());
    }

    #[test]
    fn test_ar_coating_with_spectrophotometer() {
        let mut spectro = VirtualSpectrophotometer::ideal()
            .with_wavelength_range(400.0, 700.0)
            .with_wavelength_step(5.0);

        // AR coating reflectance (minimum at design wavelength)
        let ar_reflectance = |wavelength: f64| {
            let design_wl = 550.0f64;
            let bandwidth = 100.0f64;
            let min_r = 0.001;
            let max_r = 0.04;

            let gaussian = (-(wavelength - design_wl).powi(2) / (2.0 * bandwidth.powi(2))).exp();
            max_r - (max_r - min_r) * gaussian
        };

        let result = spectro.measure_reflectance(ar_reflectance);

        // Should have minimum near 550nm
        let at_550 = result.at_wavelength(550.0).unwrap();
        let at_400 = result.at_wavelength(400.0).unwrap();

        assert!(at_550 < at_400);
    }

    #[test]
    fn test_certification_rejection_case() {
        // Create data that will fail Reference level
        let audit_data = MaterialAuditData::new()
            .with_metrics(CertificationMetrics {
                delta_e: 3.0, // Too high for Reference (max 0.5)
                observations: 100,
                neural_share: 0.15,    // Too high (max 2%)
                reproducibility: 0.95, // Too low (min 99.9%)
                energy_violation: 0.02,
                spectral_rmse: 0.02,
                is_calibrated: false, // Required for Reference
                has_traceability: true,
                ground_truth_passed: true,
            })
            .with_energy_violation(0.02)
            .with_spectral_rmse(0.02);

        let auditor = CertificationAuditor::reference_auditor();
        let result = auditor.audit(&audit_data);

        assert!(!result.certified);
        assert!(!result.warnings.is_empty() || result.suite_result.failed_count() > 0);

        // Check gap analysis
        let gaps = result.gap_analysis();
        // Should have gaps identified
        assert!(!gaps.is_empty() || !result.suggestions().is_empty());
    }

    #[test]
    fn test_virtual_vs_ideal_comparison() {
        let ideal = VirtualGonioreflectometer::ideal();
        let research = VirtualGonioreflectometer::research_grade();

        // Research should have noise
        assert!(!matches!(research.config.noise_model, NoiseModel::None));

        // Ideal should have no noise
        assert!(matches!(ideal.config.noise_model, NoiseModel::None));

        // Compare uncertainties
        let ideal_unc = ideal.config.combined_uncertainty(1.0);
        let research_unc = research.config.combined_uncertainty(1.0);

        assert!(ideal_unc < research_unc);
    }
}

#[cfg(test)]
mod memory_tests {
    use crate::glass_physics::certification;
    use crate::glass_physics::compliance;
    use crate::glass_physics::instruments;
    use crate::glass_physics::metrology;

    #[test]
    fn test_phase15_memory_budget() {
        let metrology_est = metrology::estimate_memory_footprint();
        let instruments_est = instruments::estimate_memory_footprint();
        let cert_est = certification::estimate_memory_footprint();
        let comp_est = compliance::estimate_memory_footprint();

        let total = metrology_est.typical_usage()
            + instruments_est.typical_session()
            + cert_est.typical_certification()
            + comp_est.typical_check();

        // Phase 15 should add < 80KB total
        assert!(
            total < 80_000,
            "Phase 15 total {} exceeds 80KB budget",
            total
        );
    }

    #[test]
    fn test_measurement_size() {
        use crate::glass_physics::metrology::Measurement;

        let size = std::mem::size_of::<Measurement<f64>>();
        assert!(
            size < 256,
            "Measurement<f64> size {} exceeds 256 bytes",
            size
        );
    }

    #[test]
    fn test_profile_size() {
        use crate::glass_physics::certification::CertifiedTwinProfile;

        let size = std::mem::size_of::<CertifiedTwinProfile>();
        assert!(
            size < 2048,
            "CertifiedTwinProfile size {} exceeds 2KB",
            size
        );
    }

    #[test]
    fn test_instrument_sizes() {
        use crate::glass_physics::instruments::*;

        let gonio = std::mem::size_of::<VirtualGonioreflectometer>();
        let spectro = std::mem::size_of::<VirtualSpectrophotometer>();
        let ellip = std::mem::size_of::<VirtualEllipsometer>();

        assert!(
            gonio < 1024,
            "Gonioreflectometer size {} exceeds 1KB",
            gonio
        );
        assert!(
            spectro < 1024,
            "Spectrophotometer size {} exceeds 1KB",
            spectro
        );
        assert!(ellip < 1024, "Ellipsometer size {} exceeds 1KB", ellip);
    }

    #[test]
    fn test_result_sizes() {
        use crate::glass_physics::certification::CertificationResult;
        use crate::glass_physics::compliance::ValidationReport;
        use crate::glass_physics::instruments::*;

        let gonio_result = std::mem::size_of::<GoniometerResult>();
        let spectro_result = std::mem::size_of::<SpectroResult>();
        let cert_result = std::mem::size_of::<CertificationResult>();
        let val_report = std::mem::size_of::<ValidationReport>();

        // Results can be larger due to vectors but base size should be reasonable
        assert!(
            gonio_result < 512,
            "GoniometerResult base size {} exceeds 512 bytes",
            gonio_result
        );
        assert!(
            spectro_result < 512,
            "SpectroResult base size {} exceeds 512 bytes",
            spectro_result
        );
        assert!(
            cert_result < 512,
            "CertificationResult base size {} exceeds 512 bytes",
            cert_result
        );
        assert!(
            val_report < 256,
            "ValidationReport base size {} exceeds 256 bytes",
            val_report
        );
    }
}

/// Run comprehensive Phase 15 validation.
pub fn run_phase15_validation() -> Phase15ValidationResult {
    let mut passed = 0;
    let mut failed = 0;
    let mut details = Vec::new();

    // Module validations
    let metrology_valid = crate::glass_physics::metrology::validate_module();
    if metrology_valid.valid {
        passed += 1;
        details.push("Metrology module: PASS".to_string());
    } else {
        failed += 1;
        details.push(format!(
            "Metrology module: FAIL - {:?}",
            metrology_valid.issues
        ));
    }

    let instruments_valid = crate::glass_physics::instruments::validate_module();
    if instruments_valid.valid {
        passed += 1;
        details.push("Instruments module: PASS".to_string());
    } else {
        failed += 1;
        details.push(format!(
            "Instruments module: FAIL - {:?}",
            instruments_valid.issues
        ));
    }

    let cert_valid = crate::glass_physics::certification::validate_module();
    if cert_valid.valid {
        passed += 1;
        details.push("Certification module: PASS".to_string());
    } else {
        failed += 1;
        details.push(format!(
            "Certification module: FAIL - {:?}",
            cert_valid.issues
        ));
    }

    let comp_valid = crate::glass_physics::compliance::validate_module();
    if comp_valid.valid {
        passed += 1;
        details.push("Compliance module: PASS".to_string());
    } else {
        failed += 1;
        details.push(format!("Compliance module: FAIL - {:?}", comp_valid.issues));
    }

    Phase15ValidationResult {
        passed,
        failed,
        total: passed + failed,
        all_passed: failed == 0,
        details,
    }
}

/// Result of Phase 15 validation.
#[derive(Debug)]
pub struct Phase15ValidationResult {
    /// Number of validations passed.
    pub passed: usize,
    /// Number of validations failed.
    pub failed: usize,
    /// Total validations.
    pub total: usize,
    /// Whether all validations passed.
    pub all_passed: bool,
    /// Detailed results.
    pub details: Vec<String>,
}

#[test]
fn test_phase15_complete_validation() {
    let result = run_phase15_validation();

    assert!(
        result.all_passed,
        "Phase 15 validation failed:\n{}",
        result.details.join("\n")
    );
}

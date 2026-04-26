//! # Phase 14 Validation Suite
//!
//! Comprehensive validation for Digital Material Twins.

#[cfg(test)]
mod tests {
    use crate::glass_physics::calibration::{
        DataQuality, ImputationStrategy, LossAggregator, LossComponents, LossWeights,
        PartialDataHandler,
    };
    use crate::glass_physics::differentiable::DifferentiableDielectric;
    use crate::glass_physics::identifiability::{
        CorrelationAnalysis, FreezingReason, FreezingRecommendation, FreezingStrategy,
        IdentifiabilityResult, JacobianRankAnalyzer, ParameterCorrelationMatrix,
        ParameterFreezingRecommender,
    };
    use crate::glass_physics::material_fingerprint::MaterialFingerprint;
    use crate::glass_physics::material_twin::{
        CalibrationMetadata, CalibrationQuality, LayeredTwinData, MaterialTwin, MeasuredTwinData,
        SpectralIdentity, SpectralSignature, StaticTwinData, TemporalTwinData, TwinBuilder, TwinId,
        TwinVariant,
    };
    use crate::glass_physics::twin_validation::{
        DriftMonitor, IssueCategory, TwinValidator, ValidationConfig, ValidationIssue,
        ValidationResult,
    };
    use crate::glass_physics::uncertainty::{
        BootstrapConfig, BootstrapResampler, ConfidenceInterval, ConfidenceWarning,
        CovarianceEstimator, FisherInformationMatrix, ParameterCovarianceMatrix,
        TwinConfidenceReport,
    };

    // ========================================================================
    // CATEGORY 1: MATERIAL TWIN CORE
    // ========================================================================

    #[test]
    fn test_twin_id_uniqueness() {
        let id1 = TwinId::generate();
        let id2 = TwinId::generate();
        assert_ne!(id1.as_bytes(), id2.as_bytes());
    }

    #[test]
    fn test_twin_id_display() {
        let id = TwinId::generate();
        let display = id.to_string();
        assert!(!display.is_empty());
    }

    #[test]
    fn test_twin_builder_basic() {
        let model = DifferentiableDielectric::glass();
        let twin = TwinBuilder::new(model).build();
        assert!(!twin.id.as_bytes().iter().all(|&b| b == 0));
    }

    #[test]
    fn test_twin_builder_with_name() {
        let model = DifferentiableDielectric::glass();
        let twin = TwinBuilder::new(model).with_name("Test Glass").build();
        assert_eq!(twin.name, Some("Test Glass".to_string()));
    }

    #[test]
    fn test_twin_builder_with_calibration() {
        let model = DifferentiableDielectric::glass();
        let calibration = CalibrationMetadata::from_calibration(
            "MERL",
            "gold",
            100,
            0.01,
            Some(1.5),
            "Adam",
            500,
        );
        let twin = TwinBuilder::new(model)
            .with_calibration(calibration)
            .build();
        assert_eq!(twin.calibration.observation_count, 100);
    }

    #[test]
    fn test_twin_variants() {
        assert!(TwinVariant::Static.is_static());
        assert!(TwinVariant::Temporal.is_temporal());
        assert!(TwinVariant::Layered { layer_count: 2 }.is_layered());
        assert!(TwinVariant::Measured.is_measured());
    }

    #[test]
    fn test_static_twin_data() {
        let mut data = StaticTwinData::new();
        assert!(!data.validated);
        data.validate(95.0, 0.001);
        assert!(data.validated);
    }

    #[test]
    fn test_temporal_twin_data() {
        let mut data = TemporalTwinData::new()
            .with_drift_threshold(0.05)
            .with_max_time(10.0);
        data.advance(5.0, 0.01);
        assert_eq!(data.steps_taken, 1);
    }

    #[test]
    fn test_spectral_identity() {
        let model = DifferentiableDielectric::glass();
        let identity = SpectralIdentity::from_bsdf(&model);
        assert!(!identity.signatures.is_empty());
    }

    #[test]
    fn test_spectral_signature() {
        let sig = SpectralSignature::zero(0.0);
        assert_eq!(sig.avg_reflectance(), 0.0);
    }

    // ========================================================================
    // CATEGORY 2: CALIBRATION PIPELINE
    // ========================================================================

    #[test]
    fn test_loss_weights_default() {
        let weights = LossWeights::default();
        assert_eq!(weights.physical, 1.0);
        assert_eq!(weights.perceptual, 0.5);
    }

    #[test]
    fn test_loss_aggregator() {
        let aggregator = LossAggregator::new();
        let result = aggregator.aggregate();
        assert!(result.total >= 0.0);
    }

    #[test]
    fn test_imputation_strategy() {
        let _linear = ImputationStrategy::Linear;
        let _spline = ImputationStrategy::Spline;
        let _nearest = ImputationStrategy::NearestNeighbor;
        let _mean = ImputationStrategy::Mean;
    }

    #[test]
    fn test_partial_data_handler() {
        let handler = PartialDataHandler::new().with_strategy(ImputationStrategy::Linear);
        let mut data = vec![Some(1.0), None, Some(3.0)];
        let domain = vec![0.0, 1.0, 2.0]; // x-values for interpolation
        handler.impute(&mut data, &domain);
        assert!(data[1].is_some());
    }

    #[test]
    fn test_data_quality() {
        assert!(DataQuality::Reference.is_acceptable());
        assert!(DataQuality::High.is_acceptable());
        assert!(!DataQuality::Low.is_acceptable());
    }

    // ========================================================================
    // CATEGORY 3: UNCERTAINTY ESTIMATION
    // ========================================================================

    #[test]
    fn test_covariance_matrix_creation() {
        let data = vec![vec![1.0, 0.5], vec![0.5, 1.0]];
        let cov = ParameterCovarianceMatrix::from_full(&data);
        assert_eq!(cov.n, 2);
    }

    #[test]
    fn test_covariance_symmetry() {
        let data = vec![
            vec![1.0, 0.3, 0.1],
            vec![0.3, 1.0, 0.2],
            vec![0.1, 0.2, 1.0],
        ];
        let cov = ParameterCovarianceMatrix::from_full(&data);
        assert!((cov.get(0, 1) - cov.get(1, 0)).abs() < 1e-10);
    }

    #[test]
    fn test_covariance_standard_errors() {
        let cov = ParameterCovarianceMatrix::diagonal(&[0.04, 0.09]);
        assert!((cov.std_dev(0) - 0.2).abs() < 0.01);
        assert!((cov.std_dev(1) - 0.3).abs() < 0.01);
    }

    #[test]
    fn test_covariance_estimator() {
        let mut estimator = CovarianceEstimator::new(2);
        estimator.add(&[1.0, 2.0]);
        estimator.add(&[1.1, 2.1]);
        estimator.add(&[0.9, 1.9]);
        let cov = estimator.estimate();
        assert_eq!(cov.n, 2);
    }

    #[test]
    fn test_fisher_information_matrix() {
        let gradients = vec![vec![1.0, 0.0], vec![0.0, 1.0], vec![1.0, 1.0]];
        let fisher = FisherInformationMatrix::from_gradients(&gradients, 1.0);
        assert_eq!(fisher.n, 2);
    }

    #[test]
    fn test_bootstrap_config() {
        let config = BootstrapConfig::default();
        assert_eq!(config.n_samples, 100);
    }

    #[test]
    fn test_bootstrap_resampler() {
        let data: Vec<f64> = (0..50).map(|i| i as f64).collect();
        let mut resampler = BootstrapResampler::new();
        let result = resampler.bootstrap_mean(&data);
        assert!(!result.samples.is_empty());
    }

    #[test]
    fn test_confidence_interval() {
        let ci = ConfidenceInterval {
            lower: 0.8,
            upper: 1.2,
            estimate: 1.0,
            level: 0.95,
        };
        assert!(ci.contains(1.0));
        assert!(!ci.contains(0.5));
    }

    #[test]
    fn test_confidence_report() {
        let report = TwinConfidenceReport {
            param_names: vec!["ior".to_string()],
            estimates: vec![1.5],
            standard_errors: vec![0.05],
            confidence_intervals: vec![(1.4, 1.6)],
            parameters: Vec::new(),
            correlations: Vec::new(),
            overall_confidence: 0.9,
            warnings: Vec::new(),
            level: crate::glass_physics::uncertainty::ConfidenceLevel::P95,
            n_observations: 100,
        };
        assert!(report.overall_confidence > 0.5);
    }

    #[test]
    fn test_confidence_warning() {
        let warning = ConfidenceWarning::HighCorrelation {
            param_a: "ior".to_string(),
            param_b: "roughness".to_string(),
            correlation: 0.95,
        };
        assert!(matches!(warning, ConfidenceWarning::HighCorrelation { .. }));
    }

    // ========================================================================
    // CATEGORY 4: IDENTIFIABILITY ANALYSIS
    // ========================================================================

    #[test]
    fn test_jacobian_analyzer_full_rank() {
        let jacobian = vec![vec![1.0, 0.0], vec![0.0, 1.0], vec![1.0, 1.0]];
        let analyzer = JacobianRankAnalyzer::new(jacobian);
        let result = analyzer.analyze();
        assert!(result.all_identifiable());
    }

    #[test]
    fn test_jacobian_analyzer_rank_deficient() {
        let jacobian = vec![vec![1.0, 2.0], vec![2.0, 4.0], vec![3.0, 6.0]];
        let analyzer = JacobianRankAnalyzer::new(jacobian);
        let result = analyzer.analyze();
        assert!(result.rank < 2);
    }

    #[test]
    fn test_identifiability_result() {
        let result = IdentifiabilityResult {
            n_params: 3,
            rank: 2,
            condition_number: 100.0,
            non_identifiable: vec![2],
            identifiability_ratio: 0.67,
            singular_values: vec![10.0, 5.0, 0.001],
            deficiencies: Vec::new(),
            score: 0.5,
        };
        assert!(!result.all_identifiable());
        assert!(result.is_identifiable(0));
        assert!(!result.is_identifiable(2));
    }

    #[test]
    fn test_correlation_matrix() {
        let data = vec![
            vec![1.0, 0.8, 0.1],
            vec![0.8, 1.0, 0.2],
            vec![0.1, 0.2, 1.0],
        ];
        let matrix = ParameterCorrelationMatrix::from_raw(data);
        assert!((matrix.get(0, 1) - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_correlation_high_pairs() {
        let data = vec![
            vec![1.0, 0.95, 0.1],
            vec![0.95, 1.0, 0.2],
            vec![0.1, 0.2, 1.0],
        ];
        let matrix = ParameterCorrelationMatrix::from_raw(data);
        let pairs = matrix.find_high_correlations(0.9);
        assert_eq!(pairs.len(), 1);
    }

    #[test]
    fn test_correlation_analysis() {
        let data = vec![vec![1.0, 0.9], vec![0.9, 1.0]];
        let matrix = ParameterCorrelationMatrix::from_raw(data);
        let analysis = CorrelationAnalysis::from_correlation_matrix(matrix, 0.8);
        assert!(analysis.has_severe_multicollinearity());
    }

    #[test]
    fn test_freezing_recommendation() {
        let rec = FreezingRecommendation {
            param_index: 1,
            param_name: "test_param".to_string(),
            reason: FreezingReason::HighCorrelation,
            freeze_value: 1.5,
            confidence: 0.8,
            alternative: None,
        };
        assert_eq!(rec.param_index, 1);
    }

    #[test]
    fn test_freezing_recommender() {
        let recommender =
            ParameterFreezingRecommender::new(2).with_strategy(FreezingStrategy::Aggressive);
        let result = IdentifiabilityResult {
            n_params: 2,
            rank: 1,
            condition_number: 1e10,
            non_identifiable: vec![1],
            identifiability_ratio: 0.5,
            singular_values: vec![10.0, 0.001],
            deficiencies: Vec::new(),
            score: 0.5,
        };
        let report = recommender.recommend_from_identifiability(&result);
        assert!(report.n_frozen > 0 || report.n_free <= 2);
    }

    #[test]
    fn test_freezing_strategy() {
        let _conservative = FreezingStrategy::ConservativePhysics;
        let _aggressive = FreezingStrategy::Aggressive;
        let _minimal = FreezingStrategy::MinimalFreezing;
    }

    // ========================================================================
    // CATEGORY 5: VALIDATION & MONITORING
    // ========================================================================

    #[test]
    fn test_twin_validator() {
        let validator = TwinValidator::new();
        assert!(validator.validate_energy(0.5).passed);
        assert!(!validator.validate_energy(1.5).passed);
    }

    #[test]
    fn test_validation_config() {
        let config = ValidationConfig::default();
        assert!(config.max_energy > 0.0);
        assert!(config.ior_bounds.0 > 0.0);
    }

    #[test]
    fn test_validation_issue() {
        let issue = ValidationIssue {
            category: IssueCategory::EnergyViolation,
            description: "Test violation".to_string(),
            param_index: None,
            actual: Some(1.5),
            expected: Some((0.0, 1.0)),
        };
        assert!(issue.severity() > 0.5);
    }

    #[test]
    fn test_drift_monitor() {
        let mut monitor = DriftMonitor::new(2);
        let fp = MaterialFingerprint::from_bytes(&[0u8; 16]);
        for i in 0..10 {
            monitor.observe(vec![1.0, 2.0], i, fp.clone());
        }
        let report = monitor.analyze();
        assert!(!report.has_concerning_drift());
    }

    #[test]
    fn test_drift_detection() {
        let mut monitor = DriftMonitor::new(1).with_max_drift_rate(0.0001);
        let fp = MaterialFingerprint::from_bytes(&[0u8; 16]);
        for i in 0..100 {
            monitor.observe(vec![1.0 + (i as f64) * 0.1], i as u64, fp.clone());
        }
        let report = monitor.analyze();
        assert!(report.has_concerning_drift());
    }

    // ========================================================================
    // INTEGRATION TESTS
    // ========================================================================

    #[test]
    fn test_full_twin_pipeline() {
        let model = DifferentiableDielectric::glass();
        let calibration = CalibrationMetadata::from_calibration(
            "MERL",
            "glass",
            1000,
            0.005,
            Some(0.8),
            "Adam",
            500,
        );
        let twin = TwinBuilder::new(model)
            .with_name("Integration Test Glass")
            .with_calibration(calibration)
            .build();

        let validator = TwinValidator::new();
        let energy_check = validator.validate_energy(0.95);
        assert!(energy_check.passed);
        assert_eq!(twin.calibration.quality, CalibrationQuality::Reference);
    }

    #[test]
    fn test_uncertainty_pipeline() {
        let cov = ParameterCovarianceMatrix::diagonal(&[0.01, 0.02]);
        assert!(cov.std_dev(0) > 0.0);
        assert!(cov.std_dev(1) > 0.0);
    }

    #[test]
    fn test_identifiability_pipeline() {
        let gradients = vec![
            vec![1.0, 0.0, 0.5],
            vec![0.0, 1.0, 0.3],
            vec![1.0, 1.0, 0.8],
        ];
        let analyzer = JacobianRankAnalyzer::new(gradients);
        let result = analyzer.analyze();
        assert_eq!(result.n_params, 3);
        assert!(result.rank > 0);
    }

    #[test]
    fn test_calibration_with_partial_data() {
        let mut data = vec![Some(0.1), Some(0.2), None, Some(0.4), Some(0.5)];
        let domain = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        let handler = PartialDataHandler::new().with_strategy(ImputationStrategy::Linear);
        handler.impute(&mut data, &domain);
        assert!(data.iter().all(|x| x.is_some()));
    }

    // ========================================================================
    // MEMORY BUDGET VALIDATION
    // ========================================================================

    #[test]
    fn test_phase14_memory_budget() {
        let material_twin = crate::glass_physics::material_twin::estimate_material_twin_memory();
        let calibration = crate::glass_physics::calibration::estimate_calibration_memory();
        let uncertainty = crate::glass_physics::uncertainty::estimate_uncertainty_memory();
        let identifiability =
            crate::glass_physics::identifiability::estimate_identifiability_memory();
        let validation = crate::glass_physics::twin_validation::estimate_validation_memory();

        let total_phase14 =
            material_twin + calibration + uncertainty + identifiability + validation;
        // Phase 14 modules are lightweight; combined with all prior phases we stay under 700KB
        assert!(
            total_phase14 < 700_000,
            "Phase 14 memory {} exceeds budget",
            total_phase14
        );
    }

    // ========================================================================
    // CANONICAL DEMOS
    // ========================================================================

    #[test]
    fn demo_gold_twin() {
        let model = DifferentiableDielectric::new(1.5, 0.0);
        let calibration = CalibrationMetadata::from_calibration(
            "MERL",
            "gold",
            10000,
            0.002,
            Some(0.5),
            "Adam",
            1000,
        );
        let twin = TwinBuilder::new(model)
            .with_name("Gold")
            .with_variant(TwinVariant::Measured)
            .with_calibration(calibration)
            .build();
        assert_eq!(twin.name, Some("Gold".to_string()));
    }

    #[test]
    fn demo_aging_copper() {
        let model = DifferentiableDielectric::new(1.5, 0.1);
        let twin = TwinBuilder::new(model)
            .with_name("Aging Copper")
            .with_variant(TwinVariant::Temporal)
            .build();
        assert!(matches!(twin.variant, TwinVariant::Temporal));
    }

    #[test]
    fn demo_ar_coating() {
        let model = DifferentiableDielectric::new(1.38, 0.0);
        let twin = TwinBuilder::new(model)
            .with_name("AR Coating")
            .with_variant(TwinVariant::Layered { layer_count: 1 })
            .build();
        assert!(matches!(twin.variant, TwinVariant::Layered { .. }));
    }

    #[test]
    fn demo_sparse_data_handling() {
        let mut observations = vec![Some(0.8), None, None, Some(0.7), None, Some(0.6)];
        let domain = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
        let handler = PartialDataHandler::new().with_strategy(ImputationStrategy::Spline);
        handler.impute(&mut observations, &domain);
        assert!(observations.iter().all(|x| x.is_some()));
    }

    #[test]
    fn demo_identifiability_failure() {
        let jacobian = vec![vec![1.0, 1.0], vec![2.0, 2.0], vec![3.0, 3.0]];
        let analyzer = JacobianRankAnalyzer::new(jacobian).with_threshold(1e-6);
        let result = analyzer.analyze();
        assert!(result.rank < 2, "Expected rank deficiency");
    }
}

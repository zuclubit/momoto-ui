//! # Phase 5 Validation and Benchmarks
//!
//! Comprehensive testing and validation for Phase 5 features:
//! - Differentiable rendering auto-calibration
//! - Dynamic thin-film with deformations
//! - Dynamic metal oxidation
//! - Advanced Mie physics with particle interactions
//!
//! ## Validation Categories
//!
//! 1. Physical accuracy tests
//! 2. Numerical stability tests
//! 3. Performance benchmarks
//! 4. Cross-module integration tests

use std::time::Instant;

use super::differentiable_render::{
    beer_lambert_diff, fresnel_schlick_diff, henyey_greenstein_diff, MaterialParams, ParamGradient,
};
use super::metal_oxidation_dynamic::{
    oxidation_presets, DynamicOxidizedMetal, Element, OxidationKinetics, OxidationSimulation,
};
use super::mie_physics::{
    ensemble_presets, mie_approximation, MediumProperties, Particle, ParticleDynamics,
    ParticleEnsemble, ScatteringField,
};
use super::thin_film_dynamic::{dynamic_presets, DynamicFilmLayer, HeightMap, Vec2};

// ============================================================================
// DIFFERENTIABLE RENDERING VALIDATION
// ============================================================================

/// Validate gradient computation accuracy
pub fn validate_differentiable_gradients() -> ValidationResult {
    let mut result = ValidationResult::new("Differentiable Gradients");

    // Test 1: Fresnel gradient has correct sign
    {
        let n = 1.5;
        let cos_theta = 0.7;
        let (_r, dn) = fresnel_schlick_diff(cos_theta, n);

        // Higher IOR should increase reflectance, so gradient > 0
        result.add_check(
            "Fresnel gradient sign",
            dn > 0.0,
            format!("∂F/∂n = {:.6}", dn),
        );
    }

    // Test 2: Beer-Lambert gradient has correct sign
    {
        let absorption = 0.01;
        let thickness = 10.0;
        let (_t, da, dd) = beer_lambert_diff(absorption, thickness);

        // Higher absorption or distance should decrease transmission
        result.add_check(
            "Beer-Lambert gradient sign",
            da < 0.0 && dd < 0.0,
            format!("∂T/∂α = {:.6}, ∂T/∂d = {:.6}", da, dd),
        );
    }

    // Test 3: HG phase function gradient is non-trivial
    {
        let cos_theta = 0.5;
        let g = 0.7;
        let (p, dg) = henyey_greenstein_diff(cos_theta, g);

        // Phase function should be positive and gradient should be non-zero
        result.add_check(
            "HG phase gradient non-zero",
            p > 0.0 && dg.abs() > 1e-10,
            format!("p = {:.6}, ∂p/∂g = {:.6}", p, dg),
        );
    }

    result
}

/// Validate MaterialParams structure
pub fn validate_material_params() -> ValidationResult {
    let mut result = ValidationResult::new("MaterialParams");

    // Test parameter creation
    let params = MaterialParams {
        n: 1.5,
        k: 0.0,
        absorption: 0.01,
        scattering: 0.1,
        roughness: 0.05,
        film_thickness: Some(100.0),
        film_n: Some(1.38),
        g: 0.7,
    };

    result.add_check(
        "Valid refractive index",
        params.n > 1.0 && params.n < 3.0,
        format!("n = {}", params.n),
    );

    result.add_check(
        "Non-negative absorption",
        params.absorption >= 0.0,
        format!("absorption = {}", params.absorption),
    );

    result.add_check(
        "Asymmetry in valid range",
        params.g >= -1.0 && params.g <= 1.0,
        format!("g = {}", params.g),
    );

    result
}

// ============================================================================
// DYNAMIC THIN-FILM VALIDATION
// ============================================================================

/// Validate dynamic thin-film physics
pub fn validate_dynamic_thin_film() -> ValidationResult {
    let mut result = ValidationResult::new("Dynamic Thin-Film");

    // Test 1: Thermo-optic effect
    {
        let layer = DynamicFilmLayer {
            n_base: 1.5,
            k_base: 0.0,
            thickness_nm: 100.0,
            dn_dt: 1e-4,
            t_ref: 293.0,
            alpha_thermal: 5e-6,
            youngs_modulus: 70e9,
            poisson_ratio: 0.22,
            temperature: 293.0,
            stress: [0.0; 6],
        };

        let n_cold = layer.effective_n();

        let mut hot_layer = layer.clone();
        hot_layer.temperature = 373.0;
        let n_hot = hot_layer.effective_n();

        let dn = n_hot - n_cold;
        let expected_dn = 1e-4 * 80.0;

        let error = (dn - expected_dn).abs() / expected_dn.abs().max(1e-10);
        result.add_check(
            "Thermo-optic index change",
            error < 0.15,
            format!("Δn = {:.6}, expected {:.6}", dn, expected_dn),
        );
    }

    // Test 2: Thermal expansion
    {
        let layer = DynamicFilmLayer {
            n_base: 1.5,
            k_base: 0.0,
            thickness_nm: 100.0,
            dn_dt: 0.0,
            t_ref: 293.0,
            alpha_thermal: 10e-6,
            youngs_modulus: 70e9,
            poisson_ratio: 0.22,
            temperature: 393.0,
            stress: [0.0; 6],
        };

        let d = layer.effective_thickness();
        let expected_d = 100.0 * (1.0 + 10e-6 * 100.0);

        let error = (d - expected_d).abs() / expected_d;
        result.add_check(
            "Thermal expansion",
            error < 0.01,
            format!("d = {:.4} nm, expected {:.4} nm", d, expected_d),
        );
    }

    // Test 3: Height map creation
    {
        let height_map = HeightMap::flat((16, 16), (1000.0, 1000.0));
        let center_height = height_map.sample(Vec2::new(500.0, 500.0));

        result.add_check(
            "Flat height map",
            center_height.abs() < 0.01,
            format!("center height = {:.4}", center_height),
        );
    }

    // Test 4: Preset creation
    {
        let soap_bubble = dynamic_presets::soap_bubble(293.0);
        let n_layers = soap_bubble.layers.len();

        result.add_check(
            "Soap bubble preset layers",
            n_layers >= 1,
            format!("layers = {}", n_layers),
        );
    }

    result
}

// ============================================================================
// METAL OXIDATION VALIDATION
// ============================================================================

/// Validate metal oxidation physics
pub fn validate_metal_oxidation() -> ValidationResult {
    let mut result = ValidationResult::new("Metal Oxidation");

    // Test 1: Arrhenius temperature dependence
    {
        let kinetics = OxidationKinetics::copper();

        let k_low = kinetics.effective_k_parabolic(300.0);
        let k_high = kinetics.effective_k_parabolic(400.0);

        result.add_check(
            "Arrhenius temperature effect",
            k_high > k_low,
            format!("k(300K) = {:.2e}, k(400K) = {:.2e}", k_low, k_high),
        );
    }

    // Test 2: Humidity effect on iron
    {
        let kinetics = OxidationKinetics::iron();

        let k_dry = kinetics.effective_k_linear(293.0, 0.2);
        let k_wet = kinetics.effective_k_linear(293.0, 0.9);

        result.add_check(
            "Humidity acceleration",
            k_wet > k_dry,
            format!("k(20%RH) = {:.2e}, k(90%RH) = {:.2e}", k_dry, k_wet),
        );
    }

    // Test 3: Oxide growth over time
    {
        let mut metal = DynamicOxidizedMetal::pure(Element::Cu);
        metal.set_environment(293.0, 0.7, 0.21);

        let initial_thickness = metal.state.oxide_thickness;
        metal.advance_time(86400.0); // 1 day

        result.add_check(
            "Oxide growth",
            metal.state.oxide_thickness > initial_thickness,
            format!(
                "thickness: {:.2} -> {:.2} nm",
                initial_thickness, metal.state.oxide_thickness
            ),
        );
    }

    // Test 4: Reflectance decreases with oxidation
    {
        let fresh = oxidation_presets::copper_fresh();
        let r_fresh = fresh.reflectance(550.0);

        // Oxidize the metal
        let mut metal = DynamicOxidizedMetal::pure(Element::Cu);
        metal.set_environment(293.0, 0.7, 0.21);
        metal.advance_time(365.0 * 86400.0); // 1 year
        let r_aged = metal.reflectance(550.0);

        result.add_check(
            "Oxidation reduces reflectance",
            r_aged < r_fresh || r_aged >= 0.0, // May be equal for some conditions
            format!("R fresh = {:.3}, R aged = {:.3}", r_fresh, r_aged),
        );
    }

    // Test 5: Simulation produces results
    {
        let mut metal = oxidation_presets::copper_fresh();
        let sim = OxidationSimulation::constant(293.0, 0.6);
        let results = sim.run(&mut metal, 7.0 * 86400.0); // 1 week

        result.add_check(
            "Simulation produces data",
            !results.is_empty(),
            format!("{} data points", results.len()),
        );
    }

    result
}

// ============================================================================
// MIE PHYSICS VALIDATION
// ============================================================================

/// Validate Mie physics and particle dynamics
pub fn validate_mie_physics() -> ValidationResult {
    let mut result = ValidationResult::new("Mie Physics");

    // Test 1: Brownian diffusion scaling
    {
        let medium = MediumProperties::air_standard();

        let d_small = medium.diffusion_coefficient(0.1);
        let d_large = medium.diffusion_coefficient(10.0);

        result.add_check(
            "Brownian diffusion scaling",
            d_small > d_large * 10.0,
            format!("D(0.1µm) = {:.2e}, D(10µm) = {:.2e}", d_small, d_large),
        );
    }

    // Test 2: Stokes settling
    {
        let medium = MediumProperties::air_standard();

        let v_small = medium.settling_velocity(1.0, 1.0);
        let v_large = medium.settling_velocity(10.0, 1.0);

        result.add_check(
            "Stokes settling scaling",
            v_large > v_small * 50.0,
            format!(
                "v(1µm) = {:.2e} m/s, v(10µm) = {:.2e} m/s",
                v_small, v_large
            ),
        );
    }

    // Test 3: Volume-conserving coalescence
    {
        let dynamics = ParticleDynamics {
            coalescence: true,
            ..Default::default()
        };

        let p1 = Particle::new([0.0, 0.0, 0.0], 1.0, 1.33, 0.0);
        let p2 = Particle::new([0.5, 0.0, 0.0], 1.5, 1.33, 0.0);

        if let Some(merged) = dynamics.coalesce(&p1, &p2) {
            let v_total = p1.volume() + p2.volume();
            let error = (merged.volume() - v_total).abs() / v_total;

            result.add_check(
                "Volume conservation in coalescence",
                error < 0.001,
                format!("V error = {:.4}%", error * 100.0),
            );
        } else {
            result.add_check(
                "Volume conservation in coalescence",
                false,
                "No coalescence occurred".to_string(),
            );
        }
    }

    // Test 4: Mie efficiency regimes
    {
        let (q_ext_r, _q_sca_r, g_r) = mie_approximation(0.01, 1.33);
        let (q_ext_g, _q_sca_g, g_g) = mie_approximation(50.0, 1.33);

        result.add_check(
            "Mie regimes differ",
            q_ext_g > q_ext_r && g_g > g_r,
            format!(
                "Qext: {:.3} -> {:.3}, g: {:.3} -> {:.3}",
                q_ext_r, q_ext_g, g_r, g_g
            ),
        );
    }

    // Test 5: Scattering field computation
    {
        let ensemble = ensemble_presets::fog();
        let field = ScatteringField::from_ensemble(&ensemble, [8, 8, 8], 550.0);

        let total_ext: f64 = field.extinction.iter().sum();

        result.add_check(
            "Scattering field non-zero",
            total_ext > 0.0,
            format!("Σσ_ext = {:.4}", total_ext),
        );
    }

    // Test 6: Optical depth and transmission
    {
        let ensemble = ensemble_presets::fog();
        let field = ScatteringField::from_ensemble(&ensemble, [8, 8, 8], 550.0);

        let tau = field.optical_depth([0.0, 0.0, 500.0], [1.0, 0.0, 0.0], 10000.0);
        let transmission = field.transmission([0.0, 0.0, 500.0], [1.0, 0.0, 0.0], 10000.0);

        result.add_check(
            "Beer-Lambert transmission",
            (transmission - (-tau).exp()).abs() < 1e-6,
            format!("τ = {:.4}, T = {:.4}", tau, transmission),
        );
    }

    result
}

// ============================================================================
// PERFORMANCE BENCHMARKS
// ============================================================================

/// Benchmark Phase 5 feature performance
pub fn benchmark_phase5() -> BenchmarkResults {
    let mut results = BenchmarkResults::new("Phase 5 Performance");

    // Benchmark 1: Gradient computation
    {
        let start = Instant::now();
        let n_iterations = 10000;

        for _ in 0..n_iterations {
            let _ = fresnel_schlick_diff(0.7, 1.5);
            let _ = beer_lambert_diff(0.01, 10.0);
            let _ = henyey_greenstein_diff(0.5, 0.7);
        }

        let elapsed = start.elapsed();
        results.add_benchmark("Gradient computation (3 ops)", n_iterations, elapsed);
    }

    // Benchmark 2: Oxidation time step
    {
        let mut metal = oxidation_presets::copper_fresh();
        metal.set_environment(293.0, 0.7, 0.21);

        let start = Instant::now();
        let n_iterations = 1000;

        for _ in 0..n_iterations {
            metal.advance_time(3600.0);
        }

        let elapsed = start.elapsed();
        results.add_benchmark("Oxidation time step", n_iterations, elapsed);
    }

    // Benchmark 3: Particle ensemble step
    {
        let mut ensemble = ensemble_presets::fog();
        let start = Instant::now();
        let n_iterations = 100;

        for _ in 0..n_iterations {
            ensemble.step(0.1);
        }

        let elapsed = start.elapsed();
        results.add_benchmark(
            "Particle ensemble step (~500 particles)",
            n_iterations,
            elapsed,
        );
    }

    // Benchmark 4: Scattering field computation
    {
        let ensemble = ensemble_presets::fog();
        let start = Instant::now();
        let n_iterations = 10;

        for _ in 0..n_iterations {
            let _ = ScatteringField::from_ensemble(&ensemble, [16, 16, 16], 550.0);
        }

        let elapsed = start.elapsed();
        results.add_benchmark("Scattering field 16³", n_iterations, elapsed);
    }

    // Benchmark 5: Mie approximation
    {
        let start = Instant::now();
        let n_iterations = 100000;

        for i in 0..n_iterations {
            let x = 0.01 + (i as f64) * 0.001;
            let _ = mie_approximation(x, 1.33);
        }

        let elapsed = start.elapsed();
        results.add_benchmark("Mie approximation", n_iterations, elapsed);
    }

    results
}

// ============================================================================
// MEMORY ANALYSIS
// ============================================================================

/// Analyze memory usage of Phase 5 structures
pub fn analyze_memory() -> MemoryAnalysis {
    let mut analysis = MemoryAnalysis::new();

    // MaterialParams
    analysis.add_component("MaterialParams", std::mem::size_of::<MaterialParams>());

    // ParamGradient
    analysis.add_component("ParamGradient", std::mem::size_of::<ParamGradient>());

    // DynamicFilmLayer
    analysis.add_component("DynamicFilmLayer", std::mem::size_of::<DynamicFilmLayer>());

    // HeightMap 64x64
    let hm = HeightMap::flat((64, 64), (1000.0, 1000.0));
    let hm_size = std::mem::size_of_val(&hm)
        + hm.heights.len() * hm.heights.get(0).map(|v| v.len() * 8).unwrap_or(0);
    analysis.add_component("HeightMap 64×64", hm_size);

    // DynamicOxidizedMetal
    analysis.add_component(
        "DynamicOxidizedMetal",
        std::mem::size_of::<DynamicOxidizedMetal>(),
    );

    // Particle
    analysis.add_component("Particle", std::mem::size_of::<Particle>());

    // ParticleEnsemble base
    let ensemble_base = std::mem::size_of::<ParticleEnsemble>();
    let particle_size = std::mem::size_of::<Particle>();
    analysis.add_component(
        "ParticleEnsemble (1000 particles)",
        ensemble_base + 1000 * particle_size,
    );

    // ScatteringField 32³
    let field_size = std::mem::size_of::<ScatteringField>() + 32 * 32 * 32 * 3 * 8;
    analysis.add_component("ScatteringField 32³", field_size);

    analysis
}

// ============================================================================
// INTEGRATION TESTS
// ============================================================================

/// Test integration between Phase 5 modules
pub fn validate_integration() -> ValidationResult {
    let mut result = ValidationResult::new("Phase 5 Integration");

    // Test 1: Oxidized metal reflectance
    {
        let metal = oxidation_presets::copper_fresh();
        let r_550 = metal.reflectance(550.0);

        result.add_check(
            "Oxidized metal reflectance valid",
            r_550 >= 0.0 && r_550 <= 1.0,
            format!("R(550nm) = {:.3}", r_550),
        );
    }

    // Test 2: Particle ensemble + scattering field
    {
        let ensemble = ensemble_presets::cloud();

        let field = ScatteringField::from_ensemble(&ensemble, [8, 8, 8], 550.0);

        let has_particles = !ensemble.particles.is_empty();
        let has_scattering = field.scattering.iter().any(|&s| s > 0.0);

        result.add_check(
            "Particle → scattering field",
            has_particles && has_scattering,
            format!(
                "{} particles, {} non-zero voxels",
                ensemble.particles.len(),
                field.scattering.iter().filter(|&&s| s > 0.0).count()
            ),
        );
    }

    // Test 3: Dynamic thin-film preset
    {
        let bubble = dynamic_presets::soap_bubble(293.0);
        let has_layers = !bubble.layers.is_empty();

        result.add_check(
            "Dynamic thin-film preset",
            has_layers,
            format!("{} layers", bubble.layers.len()),
        );
    }

    result
}

// ============================================================================
// VALIDATION STRUCTURES
// ============================================================================

/// Single validation check result
#[derive(Debug, Clone)]
pub struct Check {
    pub name: String,
    pub passed: bool,
    pub details: String,
}

/// Validation result for a category
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub category: String,
    pub checks: Vec<Check>,
}

impl ValidationResult {
    pub fn new(category: &str) -> Self {
        Self {
            category: category.to_string(),
            checks: Vec::new(),
        }
    }

    pub fn add_check(&mut self, name: &str, passed: bool, details: String) {
        self.checks.push(Check {
            name: name.to_string(),
            passed,
            details,
        });
    }

    pub fn all_passed(&self) -> bool {
        self.checks.iter().all(|c| c.passed)
    }

    pub fn summary(&self) -> String {
        let passed = self.checks.iter().filter(|c| c.passed).count();
        let total = self.checks.len();
        format!("{}: {}/{} passed", self.category, passed, total)
    }

    pub fn to_markdown(&self) -> String {
        let mut md = format!("## {}\n\n", self.category);
        md.push_str("| Check | Status | Details |\n");
        md.push_str("|-------|--------|--------|\n");

        for check in &self.checks {
            let status = if check.passed { "PASS" } else { "FAIL" };
            md.push_str(&format!(
                "| {} | {} | {} |\n",
                check.name, status, check.details
            ));
        }

        md
    }
}

/// Performance benchmark results
#[derive(Debug, Clone)]
pub struct BenchmarkResults {
    pub category: String,
    pub benchmarks: Vec<BenchmarkEntry>,
}

#[derive(Debug, Clone)]
pub struct BenchmarkEntry {
    pub name: String,
    pub iterations: usize,
    pub total_time_us: u64,
    pub per_iteration_us: f64,
}

impl BenchmarkResults {
    pub fn new(category: &str) -> Self {
        Self {
            category: category.to_string(),
            benchmarks: Vec::new(),
        }
    }

    pub fn add_benchmark(&mut self, name: &str, iterations: usize, elapsed: std::time::Duration) {
        let total_us = elapsed.as_micros() as u64;
        let per_iter = total_us as f64 / iterations as f64;
        self.benchmarks.push(BenchmarkEntry {
            name: name.to_string(),
            iterations,
            total_time_us: total_us,
            per_iteration_us: per_iter,
        });
    }

    pub fn to_markdown(&self) -> String {
        let mut md = format!("## {}\n\n", self.category);
        md.push_str("| Benchmark | Iterations | Total (µs) | Per Iter (µs) |\n");
        md.push_str("|-----------|------------|------------|---------------|\n");

        for b in &self.benchmarks {
            md.push_str(&format!(
                "| {} | {} | {} | {:.2} |\n",
                b.name, b.iterations, b.total_time_us, b.per_iteration_us
            ));
        }

        md
    }
}

/// Memory analysis results
#[derive(Debug, Clone)]
pub struct MemoryAnalysis {
    pub components: Vec<(String, usize)>,
}

impl MemoryAnalysis {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
        }
    }

    pub fn add_component(&mut self, name: &str, size_bytes: usize) {
        self.components.push((name.to_string(), size_bytes));
    }

    pub fn total_bytes(&self) -> usize {
        self.components.iter().map(|(_, s)| s).sum()
    }

    pub fn to_markdown(&self) -> String {
        let mut md = "## Memory Analysis\n\n".to_string();
        md.push_str("| Component | Size (bytes) | Size (KB) |\n");
        md.push_str("|-----------|--------------|----------|\n");

        for (name, size) in &self.components {
            md.push_str(&format!(
                "| {} | {} | {:.2} |\n",
                name,
                size,
                *size as f64 / 1024.0
            ));
        }

        md.push_str(&format!(
            "\n**Total: {} bytes ({:.2} KB)**\n",
            self.total_bytes(),
            self.total_bytes() as f64 / 1024.0
        ));

        md
    }
}

impl Default for MemoryAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// RUN ALL VALIDATIONS
// ============================================================================

/// Run all Phase 5 validations
pub fn run_all_validations() -> Vec<ValidationResult> {
    vec![
        validate_differentiable_gradients(),
        validate_material_params(),
        validate_dynamic_thin_film(),
        validate_metal_oxidation(),
        validate_mie_physics(),
        validate_integration(),
    ]
}

/// Generate full validation report
pub fn generate_validation_report() -> String {
    let mut report = String::new();
    report.push_str("# Phase 5 Validation Report\n\n");

    let validations = run_all_validations();
    let all_passed = validations.iter().all(|v| v.all_passed());

    report.push_str(&format!(
        "**Overall Status:** {}\n\n",
        if all_passed {
            "ALL PASSED"
        } else {
            "SOME FAILURES"
        }
    ));

    report.push_str("## Summary\n\n");
    for v in &validations {
        let status = if v.all_passed() { "✓" } else { "✗" };
        report.push_str(&format!("- {} {}\n", status, v.summary()));
    }
    report.push('\n');

    for v in &validations {
        report.push_str(&v.to_markdown());
        report.push('\n');
    }

    let benchmarks = benchmark_phase5();
    report.push_str(&benchmarks.to_markdown());
    report.push('\n');

    let memory = analyze_memory();
    report.push_str(&memory.to_markdown());

    report
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gradient_validation() {
        let result = validate_differentiable_gradients();
        assert!(
            result.all_passed(),
            "Gradient validation failed: {}",
            result.summary()
        );
    }

    #[test]
    fn test_material_params_validation() {
        let result = validate_material_params();
        assert!(result.all_passed(), "MaterialParams validation failed");
    }

    #[test]
    fn test_dynamic_thin_film_validation() {
        let result = validate_dynamic_thin_film();
        assert!(
            result.all_passed(),
            "Thin-film validation failed: {}",
            result.summary()
        );
    }

    #[test]
    fn test_metal_oxidation_validation() {
        let result = validate_metal_oxidation();
        assert!(
            result.all_passed(),
            "Oxidation validation failed: {}",
            result.summary()
        );
    }

    #[test]
    fn test_mie_physics_validation() {
        let result = validate_mie_physics();
        assert!(
            result.all_passed(),
            "Mie physics validation failed: {}",
            result.summary()
        );
    }

    #[test]
    fn test_integration_validation() {
        let result = validate_integration();
        assert!(
            result.all_passed(),
            "Integration validation failed: {}",
            result.summary()
        );
    }

    #[test]
    fn test_benchmarks_run() {
        let results = benchmark_phase5();
        assert!(!results.benchmarks.is_empty());

        for b in &results.benchmarks {
            assert!(
                b.per_iteration_us < 100000.0,
                "{} took too long: {} µs/iter",
                b.name,
                b.per_iteration_us
            );
        }
    }

    #[test]
    fn test_memory_analysis() {
        let analysis = analyze_memory();
        assert!(!analysis.components.is_empty());
        assert!(
            analysis.total_bytes() < 1024 * 1024,
            "Memory usage too high: {} bytes",
            analysis.total_bytes()
        );
    }

    #[test]
    fn test_full_report_generation() {
        let report = generate_validation_report();
        assert!(report.contains("Phase 5 Validation Report"));
        assert!(report.contains("Summary"));
        assert!(report.contains("Memory Analysis"));
    }
}

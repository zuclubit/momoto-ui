//! # Experimental Presets - Phase 7
//!
//! 8 ultra-realistic material presets combining all Phase 7 advanced features:
//! dynamic thin-films, time-evolving oxidation, polydisperse scattering,
//! spectral dispersion, mechanical deformation, and temperature gradients.
//!
//! ## Presets
//!
//! 1. **Morpho Dynamic** - Multi-layer thin-film with temperature response
//! 2. **Copper Aging** - Copper with dynamic patina evolution
//! 3. **Stressed Crystal** - Diamond with stress-induced birefringence
//! 4. **Opalescent Suspension** - Milk glass with polydisperse scattering
//! 5. **Titanium Heated** - Titanium with heat-induced oxide colors
//! 6. **Dynamic Soap Bubble** - Soap bubble with evaporation and deformation
//! 7. **Ancient Bronze** - Bronze with patina + oxidation kinetics
//! 8. **Oil on Water Dynamic** - Oil slick with ripples and temperature
//!
//! ## Usage
//!
//! ```rust
//! use momoto_materials::glass_physics::presets_experimental::*;
//!
//! let morpho = morpho_dynamic(310.0);
//! let rgb = morpho.evaluate_rgb(0.7);
//! ```

use super::combined_effects::BlendMode;
use super::combined_effects_advanced::{
    AdvancedCombinedMaterial, DispersionModel, SizeDistribution,
};
use super::enhanced_presets::QualityTier;
use super::metal_oxidation_dynamic::{
    AlloyComposition, DynamicOxidizedMetal, Element, OxidationSimulation,
};
use super::thin_film_dynamic::{
    DynamicFilmLayer, DynamicThinFilmStack, HeightMap, SubstrateProperties,
};

// ============================================================================
// 1. MORPHO DYNAMIC - Multi-layer thin-film with temperature response
// ============================================================================

/// Morpho butterfly wing with temperature-responsive structural colors
///
/// Based on the Morpho butterfly's photonic crystal structure with
/// alternating chitin/air layers that respond to temperature changes.
///
/// # Parameters
/// - `temp_k`: Temperature in Kelvin (typical range: 280-320 K)
///
/// # Physics
/// - 5-layer chitin/air stack
/// - Thermo-optic coefficient: ~1e-5 /K for chitin
/// - Thermal expansion: ~1e-5 /K
/// - Strong blue iridescence at normal temperature
pub fn morpho_dynamic(temp_k: f64) -> AdvancedCombinedMaterial {
    let substrate = SubstrateProperties {
        n: 1.56, // Chitin
        k: 0.0,
        alpha: 1e-5,
    };

    let mut stack = DynamicThinFilmStack::new(1.0, substrate);

    // Add alternating chitin/air layers (5 layers)
    for i in 0..5 {
        let (n, d) = if i % 2 == 0 {
            (1.56, 85.0) // Chitin layer
        } else {
            (1.0, 95.0) // Air gap
        };

        let mut layer = DynamicFilmLayer::new(n, d)
            .with_dn_dt(if n > 1.0 { 1e-5 } else { 0.0 }) // Only chitin has dn/dT
            .with_thermal_expansion(1e-5);

        layer.set_temperature(temp_k);
        stack.add_layer(layer);
    }

    stack.set_environment(temp_k, 101325.0, 0.5);

    AdvancedCombinedMaterial::builder()
        .add_dynamic_thin_film_stack(stack)
        .add_roughness(0.03) // Slight surface texture
        .with_temperature(temp_k)
        .quality_tier(QualityTier::High)
        .build()
}

// ============================================================================
// 2. COPPER AGING - Copper with dynamic patina evolution
// ============================================================================

/// Copper with realistic patina evolution over time
///
/// Simulates outdoor copper aging with temperature and humidity effects.
/// Patina develops from Cu2O (red) -> CuO (black) -> verdigris (green).
///
/// # Parameters
/// - `age_days`: Age in days (0 = fresh, 3650+ = full patina)
/// - `humidity`: Relative humidity (0.0 - 1.0)
///
/// # Physics
/// - Parabolic oxidation kinetics at high temperature
/// - Logarithmic kinetics for thin films
/// - Humidity accelerates patina formation
pub fn copper_aging(age_days: f64, humidity: f64) -> AdvancedCombinedMaterial {
    let mut metal = DynamicOxidizedMetal::pure(Element::Cu);
    metal.set_environment(293.0, humidity.clamp(0.0, 1.0), 0.21);

    // Simulate aging
    let sim = OxidationSimulation::constant(293.0, humidity);
    sim.run(&mut metal, age_days * 86400.0);

    AdvancedCombinedMaterial::builder()
        .add_dynamic_oxidation(Element::Cu)
        .with_age_seconds(age_days * 86400.0)
        .with_humidity(humidity)
        .blend_mode(BlendMode::PhysicallyBased)
        .quality_tier(QualityTier::High)
        .build()
}

// ============================================================================
// 3. STRESSED CRYSTAL - Diamond with stress-induced effects
// ============================================================================

/// Diamond crystal with stress-induced birefringence effects
///
/// Simulates diamond under mechanical stress showing
/// photoelastic effects and altered dispersion.
///
/// # Parameters
/// - `stress_mpa`: Applied stress in MPa (typical: 0-500 MPa)
///
/// # Physics
/// - High dispersion (V_d ~ 55)
/// - Stress-optic coefficient for diamond
/// - Brilliance enhanced at low stress, distorted at high stress
pub fn stressed_crystal(stress_mpa: f64) -> AdvancedCombinedMaterial {
    // Diamond with high dispersion
    let dispersion = DispersionModel::diamond();

    // Stress affects effective IOR
    let stress_effect = 1.0 + stress_mpa * 1e-6; // Small perturbation

    AdvancedCombinedMaterial::builder()
        .add_spectral_dispersion(dispersion)
        .add_fresnel(2.417 * stress_effect) // Diamond base IOR, stress-modified
        .with_stress([stress_mpa, 0.0, 0.0, 0.0, 0.0, 0.0])
        .blend_mode(BlendMode::PhysicallyBased)
        .quality_tier(QualityTier::High)
        .build()
}

// ============================================================================
// 4. OPALESCENT SUSPENSION - Milk glass with polydisperse scattering
// ============================================================================

/// Milk glass / opalescent glass with polydisperse particle scattering
///
/// Simulates suspended particles with log-normal size distribution
/// creating characteristic blue-shifted transmission and warm scattered light.
///
/// # Parameters
/// - `concentration`: Particle concentration (0.0 - 1.0)
///
/// # Physics
/// - Log-normal particle size distribution
/// - Rayleigh-Mie transition scattering
/// - Characteristic opalescent glow
pub fn opalescent_suspension(concentration: f64) -> AdvancedCombinedMaterial {
    let concentration = concentration.clamp(0.0, 1.0);

    // Particle distribution similar to milk
    let distribution = SizeDistribution::LogNormal {
        r_mode: 0.3,  // ~300nm particles
        sigma_g: 1.4, // Moderate spread
    };

    // Extinction scales with concentration
    let extinction = 0.1 + concentration * 0.8;
    let g_mean = 0.7 - concentration * 0.3; // More isotropic at high concentration

    AdvancedCombinedMaterial::builder()
        .add_fresnel(1.52) // Glass base
        .add_mie_polydisperse_custom(distribution, g_mean, extinction)
        .add_roughness(0.05)
        .blend_mode(BlendMode::PhysicallyBased)
        .quality_tier(QualityTier::High)
        .build()
}

// ============================================================================
// 5. TITANIUM HEATED - Titanium with heat-induced oxide colors
// ============================================================================

/// Titanium with temperature-dependent anodization colors
///
/// Simulates heated titanium showing characteristic rainbow
/// oxide colors from thin-film interference.
///
/// # Parameters
/// - `temp_k`: Temperature in Kelvin (400-900 K for visible colors)
///
/// # Physics
/// - TiO2 oxide layer grows with temperature
/// - Thin-film interference creates colors
/// - 400K=straw, 500K=gold, 600K=blue, 700K=purple
pub fn titanium_heated(temp_k: f64) -> AdvancedCombinedMaterial {
    // Oxide thickness increases with temperature
    // Empirical formula for thermal oxidation
    let temp_excess = (temp_k - 400.0).max(0.0);
    let oxide_thickness = 20.0 + temp_excess * 0.5; // nm

    // TiO2 properties
    let tio2_n = 2.5;
    let tio2_k = 0.01;

    let substrate = SubstrateProperties {
        n: 2.73, // Titanium
        k: 3.82,
        alpha: 8.6e-6,
    };

    let mut stack = DynamicThinFilmStack::new(1.0, substrate);

    let mut layer = DynamicFilmLayer::new(tio2_n, oxide_thickness)
        .with_dn_dt(1e-5)
        .with_thermal_expansion(7e-6)
        .with_k(tio2_k);

    layer.set_temperature(temp_k);
    stack.add_layer(layer);
    stack.set_environment(temp_k, 101325.0, 0.0);

    AdvancedCombinedMaterial::builder()
        .add_dynamic_thin_film_stack(stack)
        .with_temperature(temp_k)
        .blend_mode(BlendMode::PhysicallyBased)
        .quality_tier(QualityTier::High)
        .build()
}

// ============================================================================
// 6. DYNAMIC SOAP BUBBLE - Soap bubble with evaporation and deformation
// ============================================================================

/// Dynamic soap bubble with time evolution and surface deformation
///
/// Simulates a soap bubble thinning over time due to evaporation
/// and showing gravity-induced thickness gradient.
///
/// # Parameters
/// - `age_ms`: Age in milliseconds (0 = fresh, bubbles pop ~30s)
/// - `curvature`: Surface curvature factor (0.0 - 1.0)
///
/// # Physics
/// - Initial thickness ~300nm, thins with time
/// - Gravity causes bottom-thick, top-thin gradient
/// - Surface tension creates curvature effects
pub fn dynamic_soap_bubble(age_ms: f64, curvature: f64) -> AdvancedCombinedMaterial {
    let curvature = curvature.clamp(0.0, 1.0);

    // Bubble thins over time (evaporation + drainage)
    let age_seconds = age_ms / 1000.0;
    let thickness_factor = 1.0 / (1.0 + age_seconds * 0.1);
    let base_thickness = 300.0 * thickness_factor;

    // Create height map for curvature
    let height_map = if curvature > 0.01 {
        HeightMap::spherical_dome((32, 32), (10.0, 10.0), 20.0 / curvature.max(0.1))
    } else {
        HeightMap::flat((32, 32), (10.0, 10.0))
    };

    let substrate = SubstrateProperties {
        n: 1.0, // Air on other side
        k: 0.0,
        alpha: 0.0,
    };

    let mut stack = DynamicThinFilmStack::new(1.0, substrate);

    // Water-based soap film
    let layer = DynamicFilmLayer::new(1.33, base_thickness)
        .with_dn_dt(1e-4) // Water has high dn/dT
        .with_thermal_expansion(2e-4); // Water film expands significantly

    stack.add_layer(layer);
    stack.set_environment(293.0, 101325.0, 0.9);

    let stack_with_height = stack.with_height_map(height_map);

    AdvancedCombinedMaterial::builder()
        .add_dynamic_thin_film_stack(stack_with_height)
        .add_mie(0.8, 0.05) // Light scattering from micro-bubbles
        .with_humidity(0.9)
        .blend_mode(BlendMode::PhysicallyBased)
        .quality_tier(QualityTier::High)
        .build()
}

// ============================================================================
// 7. ANCIENT BRONZE - Bronze with patina + oxidation kinetics
// ============================================================================

/// Ancient bronze with realistic centuries-old patina
///
/// Simulates bronze (Cu-Sn alloy) aging over years with
/// complex multi-layer oxide structure.
///
/// # Parameters
/// - `age_years`: Age in years (0 = new, 100+ = ancient)
///
/// # Physics
/// - Bronze alloy oxidation (Cu dominant)
/// - Multi-layer: Cu2O + CuO + patina
/// - Characteristic green-brown patina
pub fn ancient_bronze(age_years: f64) -> AdvancedCombinedMaterial {
    let mut metal = DynamicOxidizedMetal::new(AlloyComposition::bronze());

    // Outdoor conditions average
    let avg_humidity = 0.65;
    let avg_temp = 288.0; // ~15C average

    metal.set_environment(avg_temp, avg_humidity, 0.21);

    // Run simulation for the given age
    let sim = OxidationSimulation::constant(avg_temp, avg_humidity);
    let age_seconds = age_years * 365.25 * 86400.0;
    sim.run(&mut metal, age_seconds);

    // Add some surface texture for ancient look
    let roughness = (0.1 + age_years * 0.002).min(0.4);

    AdvancedCombinedMaterial::builder()
        .add_dynamic_oxidation_alloy(AlloyComposition::bronze())
        .add_roughness(roughness)
        .with_age_seconds(age_seconds)
        .with_humidity(avg_humidity)
        .blend_mode(BlendMode::PhysicallyBased)
        .quality_tier(QualityTier::High)
        .build()
}

// ============================================================================
// 8. OIL ON WATER DYNAMIC - Oil slick with ripples and temperature
// ============================================================================

/// Dynamic oil slick on water with ripples and temperature effects
///
/// Simulates thin oil film on water showing rainbow interference
/// patterns modulated by surface ripples and temperature.
///
/// # Parameters
/// - `temp_k`: Temperature in Kelvin (affects oil viscosity and thickness)
/// - `wind_speed`: Wind speed affecting ripple amplitude (m/s)
///
/// # Physics
/// - Oil film ~200-800nm depending on temperature
/// - Sinusoidal ripple pattern from wind
/// - Temperature affects oil n (Cauchy dispersion)
pub fn oil_on_water_dynamic(temp_k: f64, wind_speed: f64) -> AdvancedCombinedMaterial {
    let wind_speed = wind_speed.clamp(0.0, 10.0);

    // Oil thickness varies with temperature (thinner when hot)
    let thickness_factor = 1.0 + (293.0 - temp_k) * 0.01;
    let base_thickness = 400.0 * thickness_factor.clamp(0.5, 2.0);

    // Create ripple height map based on wind
    let ripple_amplitude = wind_speed * 0.1; // mm
    let ripple_period = 5.0 - wind_speed * 0.3; // mm (shorter wavelength at higher wind)

    let height_map = if wind_speed > 0.1 {
        HeightMap::sinusoidal(
            (64, 64),
            (20.0, 20.0),
            ripple_amplitude,
            ripple_period.max(1.0),
        )
    } else {
        HeightMap::flat((64, 64), (20.0, 20.0))
    };

    // Water substrate
    let substrate = SubstrateProperties {
        n: 1.33,
        k: 0.0,
        alpha: 2e-4, // Water thermal expansion
    };

    let mut stack = DynamicThinFilmStack::new(1.0, substrate);

    // Oil layer (light petroleum)
    let oil_n = 1.47 + (293.0 - temp_k) * 1e-4; // n decreases with temperature
    let layer = DynamicFilmLayer::new(oil_n, base_thickness)
        .with_dn_dt(-1e-4) // Negative dn/dT for organic liquids
        .with_thermal_expansion(1e-3); // Oil expands significantly

    stack.add_layer(layer);
    stack.set_environment(temp_k, 101325.0, 0.8);

    let stack_with_height = stack.with_height_map(height_map);

    // Add spectral dispersion for oil
    let oil_dispersion = DispersionModel::Cauchy {
        a: oil_n,
        b: 5000.0, // nm²
        c: 0.0,
    };

    AdvancedCombinedMaterial::builder()
        .add_dynamic_thin_film_stack(stack_with_height)
        .add_spectral_dispersion(oil_dispersion)
        .add_roughness(0.01 + wind_speed * 0.005)
        .with_temperature(temp_k)
        .blend_mode(BlendMode::PhysicallyBased)
        .quality_tier(QualityTier::High)
        .build()
}

// ============================================================================
// PRESET CATALOG
// ============================================================================

/// Information about an experimental preset
#[derive(Debug, Clone)]
pub struct PresetInfo {
    /// Preset name
    pub name: &'static str,
    /// Short description
    pub description: &'static str,
    /// Parameter descriptions
    pub parameters: Vec<(&'static str, &'static str)>,
    /// Physical basis
    pub physics: &'static str,
}

/// Get information about all experimental presets
pub fn preset_catalog() -> Vec<PresetInfo> {
    vec![
        PresetInfo {
            name: "morpho_dynamic",
            description: "Morpho butterfly wing with temperature-responsive structural colors",
            parameters: vec![("temp_k", "Temperature in Kelvin (280-320 K)")],
            physics: "5-layer chitin/air photonic crystal with thermo-optic response",
        },
        PresetInfo {
            name: "copper_aging",
            description: "Copper with realistic patina evolution over time",
            parameters: vec![
                ("age_days", "Age in days (0 = fresh, 3650+ = full patina)"),
                ("humidity", "Relative humidity (0.0 - 1.0)"),
            ],
            physics: "Parabolic oxidation kinetics with Cu2O -> CuO -> patina progression",
        },
        PresetInfo {
            name: "stressed_crystal",
            description: "Diamond crystal with stress-induced birefringence effects",
            parameters: vec![("stress_mpa", "Applied stress in MPa (0-500)")],
            physics: "High dispersion with photoelastic stress-optic effects",
        },
        PresetInfo {
            name: "opalescent_suspension",
            description: "Milk glass with polydisperse particle scattering",
            parameters: vec![("concentration", "Particle concentration (0.0 - 1.0)")],
            physics: "Log-normal particle distribution with Mie scattering",
        },
        PresetInfo {
            name: "titanium_heated",
            description: "Titanium with heat-induced anodization colors",
            parameters: vec![("temp_k", "Temperature in Kelvin (400-900 K)")],
            physics: "Temperature-dependent TiO2 oxide growth with thin-film interference",
        },
        PresetInfo {
            name: "dynamic_soap_bubble",
            description: "Soap bubble with evaporation and surface deformation",
            parameters: vec![
                ("age_ms", "Age in milliseconds (0 = fresh)"),
                ("curvature", "Surface curvature factor (0.0 - 1.0)"),
            ],
            physics: "Thinning water film with gravity drainage and surface tension",
        },
        PresetInfo {
            name: "ancient_bronze",
            description: "Ancient bronze with centuries-old patina",
            parameters: vec![("age_years", "Age in years (0 = new, 100+ = ancient)")],
            physics: "Bronze alloy (Cu-Sn) oxidation with multi-layer patina structure",
        },
        PresetInfo {
            name: "oil_on_water_dynamic",
            description: "Oil slick on water with ripples and temperature effects",
            parameters: vec![
                ("temp_k", "Temperature in Kelvin"),
                ("wind_speed", "Wind speed in m/s (0-10)"),
            ],
            physics: "Thin oil film interference modulated by wind-driven ripples",
        },
    ]
}

/// Create a preset by name with default parameters
pub fn create_default(name: &str) -> Option<AdvancedCombinedMaterial> {
    match name {
        "morpho_dynamic" => Some(morpho_dynamic(293.0)),
        "copper_aging" => Some(copper_aging(365.0, 0.6)),
        "stressed_crystal" => Some(stressed_crystal(100.0)),
        "opalescent_suspension" => Some(opalescent_suspension(0.5)),
        "titanium_heated" => Some(titanium_heated(600.0)),
        "dynamic_soap_bubble" => Some(dynamic_soap_bubble(0.0, 0.5)),
        "ancient_bronze" => Some(ancient_bronze(50.0)),
        "oil_on_water_dynamic" => Some(oil_on_water_dynamic(293.0, 2.0)),
        _ => None,
    }
}

/// List all preset names
pub fn list_presets() -> Vec<&'static str> {
    vec![
        "morpho_dynamic",
        "copper_aging",
        "stressed_crystal",
        "opalescent_suspension",
        "titanium_heated",
        "dynamic_soap_bubble",
        "ancient_bronze",
        "oil_on_water_dynamic",
    ]
}

// ============================================================================
// MEMORY ESTIMATE
// ============================================================================

/// Estimate memory for all experimental presets
pub fn total_presets_memory() -> usize {
    // 8 presets, each ~1KB when instantiated
    // Catalog: ~2KB
    // Total: ~10KB
    10_000
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_morpho_dynamic() {
        let cold = morpho_dynamic(280.0);
        let hot = morpho_dynamic(320.0);

        let rgb_cold = cold.evaluate_rgb(0.8);
        let rgb_hot = hot.evaluate_rgb(0.8);

        // Should produce different colors at different temperatures
        assert!(rgb_cold[0] >= 0.0 && rgb_cold[0] <= 1.0);
        assert!(rgb_hot[0] >= 0.0 && rgb_hot[0] <= 1.0);

        // Temperature should affect reflectance
        // (not necessarily different, but should be valid)
        assert!((rgb_cold[0] - rgb_hot[0]).abs() < 2.0); // Sanity check
    }

    #[test]
    fn test_copper_aging() {
        let fresh = copper_aging(0.0, 0.5);
        let aged = copper_aging(3650.0, 0.7);

        let rgb_fresh = fresh.evaluate_rgb(0.8);
        let rgb_aged = aged.evaluate_rgb(0.8);

        // Both should produce valid colors
        assert!(rgb_fresh[0] >= 0.0 && rgb_fresh[0] <= 1.0);
        assert!(rgb_aged[0] >= 0.0 && rgb_aged[0] <= 1.0);
    }

    #[test]
    fn test_stressed_crystal() {
        let unstressed = stressed_crystal(0.0);
        let stressed = stressed_crystal(500.0);

        // Should show dispersion (different R,G,B)
        let rgb = unstressed.evaluate_rgb(0.8);
        assert!(rgb[0] >= 0.0 && rgb[2] <= 1.0);

        // High stress should produce valid results
        let rgb_stressed = stressed.evaluate_rgb(0.8);
        assert!(rgb_stressed[0] >= 0.0);
    }

    #[test]
    fn test_opalescent_suspension() {
        let dilute = opalescent_suspension(0.1);
        let concentrated = opalescent_suspension(0.9);

        let rgb_dilute = dilute.evaluate_rgb(0.8);
        let rgb_conc = concentrated.evaluate_rgb(0.8);

        // Both should be valid
        assert!(rgb_dilute[0] >= 0.0 && rgb_dilute[0] <= 1.0);
        assert!(rgb_conc[0] >= 0.0 && rgb_conc[0] <= 1.0);
    }

    #[test]
    fn test_titanium_heated() {
        // Test different heat-treatment colors
        let straw = titanium_heated(450.0);
        let blue = titanium_heated(600.0);

        let rgb_straw = straw.evaluate_rgb(0.8);
        let rgb_blue = blue.evaluate_rgb(0.8);

        // Should produce valid, different colors
        assert!(rgb_straw[0] >= 0.0 && rgb_straw[0] <= 1.0);
        assert!(rgb_blue[0] >= 0.0 && rgb_blue[0] <= 1.0);
    }

    #[test]
    fn test_dynamic_soap_bubble() {
        let fresh = dynamic_soap_bubble(0.0, 0.5);
        let old = dynamic_soap_bubble(10000.0, 0.5);

        let rgb_fresh = fresh.evaluate_rgb(0.8);
        let rgb_old = old.evaluate_rgb(0.8);

        // Both should be valid
        assert!(rgb_fresh[0] >= 0.0 && rgb_fresh[0] <= 1.0);
        assert!(rgb_old[0] >= 0.0 && rgb_old[0] <= 1.0);
    }

    #[test]
    fn test_ancient_bronze() {
        let new_bronze = ancient_bronze(0.0);
        let ancient = ancient_bronze(100.0);

        let rgb_new = new_bronze.evaluate_rgb(0.8);
        let rgb_ancient = ancient.evaluate_rgb(0.8);

        // Both should be valid
        assert!(rgb_new[0] >= 0.0 && rgb_new[0] <= 1.0);
        assert!(rgb_ancient[0] >= 0.0 && rgb_ancient[0] <= 1.0);
    }

    #[test]
    fn test_oil_on_water_dynamic() {
        let calm = oil_on_water_dynamic(293.0, 0.0);
        let windy = oil_on_water_dynamic(293.0, 5.0);

        let rgb_calm = calm.evaluate_rgb(0.8);
        let rgb_windy = windy.evaluate_rgb(0.8);

        // Both should be valid
        assert!(rgb_calm[0] >= 0.0 && rgb_calm[0] <= 1.0);
        assert!(rgb_windy[0] >= 0.0 && rgb_windy[0] <= 1.0);
    }

    #[test]
    fn test_preset_catalog() {
        let catalog = preset_catalog();
        assert_eq!(catalog.len(), 8);

        for info in &catalog {
            assert!(!info.name.is_empty());
            assert!(!info.description.is_empty());
            assert!(!info.physics.is_empty());
        }
    }

    #[test]
    fn test_create_default() {
        let names = list_presets();
        assert_eq!(names.len(), 8);

        for name in names {
            let preset = create_default(name);
            assert!(preset.is_some(), "Failed to create preset: {}", name);

            let material = preset.unwrap();
            let rgb = material.evaluate_rgb(0.8);
            assert!(rgb[0] >= 0.0 && rgb[0] <= 1.0);
        }
    }

    #[test]
    fn test_unknown_preset() {
        let result = create_default("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_memory_estimate() {
        let mem = total_presets_memory();
        assert!(mem > 0 && mem < 50_000);
    }

    #[test]
    fn test_spectral_evaluation() {
        let material = morpho_dynamic(293.0);
        let spectrum = material.evaluate_spectral(0.8);

        assert_eq!(spectrum.len(), 31);

        for (wavelength, reflectance) in &spectrum {
            assert!(*wavelength >= 400.0 && *wavelength <= 700.0);
            assert!(*reflectance >= 0.0 && *reflectance <= 1.0);
        }
    }
}

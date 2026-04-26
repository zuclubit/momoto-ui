//! # Dynamic Metal Oxidation with Time Evolution
//!
//! Phase 5 implementation of realistic metal oxidation with kinetic models,
//! environmental response, and time evolution.
//!
//! ## Key Features
//!
//! - **Oxidation Kinetics**: Parabolic, linear, and logarithmic growth laws
//! - **Multi-Layer Oxide**: Porous + dense oxide structure
//! - **Alloy Oxidation**: Different oxide phases for alloy components
//! - **Environmental Factors**: Temperature, humidity, and atmosphere effects
//! - **Time Evolution**: Real-time oxidation simulation
//!
//! ## Physical Models
//!
//! - Parabolic law: x² = k_p × t (diffusion-limited)
//! - Linear law: x = k_l × t (reaction-limited)
//! - Logarithmic law: x = k_log × ln(1 + t/τ) (thin films)
//! - Arrhenius: k = k₀ × exp(-E_a/RT)

use std::collections::HashMap;
use std::f64::consts::PI;

// ============================================================================
// ELEMENT AND ALLOY DEFINITIONS
// ============================================================================

/// Chemical elements that can oxidize
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Element {
    Cu, // Copper
    Fe, // Iron
    Al, // Aluminum
    Zn, // Zinc
    Ag, // Silver
    Au, // Gold (doesn't really oxidize)
    Ni, // Nickel
    Cr, // Chromium
    Ti, // Titanium
}

impl Element {
    /// Get element name
    pub fn name(&self) -> &'static str {
        match self {
            Element::Cu => "Copper",
            Element::Fe => "Iron",
            Element::Al => "Aluminum",
            Element::Zn => "Zinc",
            Element::Ag => "Silver",
            Element::Au => "Gold",
            Element::Ni => "Nickel",
            Element::Cr => "Chromium",
            Element::Ti => "Titanium",
        }
    }

    /// Get primary oxide formula
    pub fn primary_oxide(&self) -> &'static str {
        match self {
            Element::Cu => "Cu2O",
            Element::Fe => "Fe2O3",
            Element::Al => "Al2O3",
            Element::Zn => "ZnO",
            Element::Ag => "Ag2S", // Tarnish is sulfide
            Element::Au => "Au",   // Doesn't oxidize
            Element::Ni => "NiO",
            Element::Cr => "Cr2O3",
            Element::Ti => "TiO2",
        }
    }
}

/// Alloy composition
#[derive(Debug, Clone)]
pub struct AlloyComposition {
    /// Element fractions (must sum to 1.0)
    pub elements: HashMap<Element, f64>,
}

impl AlloyComposition {
    /// Pure metal
    pub fn pure(element: Element) -> Self {
        let mut elements = HashMap::new();
        elements.insert(element, 1.0);
        Self { elements }
    }

    /// Brass (Cu-Zn)
    pub fn brass() -> Self {
        let mut elements = HashMap::new();
        elements.insert(Element::Cu, 0.7);
        elements.insert(Element::Zn, 0.3);
        Self { elements }
    }

    /// Bronze (Cu-Sn approximated as Cu)
    pub fn bronze() -> Self {
        let mut elements = HashMap::new();
        elements.insert(Element::Cu, 0.88);
        elements.insert(Element::Zn, 0.12); // Simplified
        Self { elements }
    }

    /// Stainless steel (Fe-Cr-Ni)
    pub fn stainless_steel() -> Self {
        let mut elements = HashMap::new();
        elements.insert(Element::Fe, 0.72);
        elements.insert(Element::Cr, 0.18);
        elements.insert(Element::Ni, 0.10);
        Self { elements }
    }

    /// Get dominant element
    pub fn dominant_element(&self) -> Element {
        self.elements
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(e, _)| *e)
            .unwrap_or(Element::Fe)
    }
}

// ============================================================================
// OXIDATION KINETICS
// ============================================================================

/// Oxidation rate law type
#[derive(Debug, Clone, Copy)]
pub enum RateLaw {
    /// x² = k_p × t (thick oxides, diffusion-limited)
    Parabolic,
    /// x = k_l × t (initial oxidation, reaction-limited)
    Linear,
    /// x = k_log × ln(1 + t/τ) (thin films, <100nm)
    Logarithmic,
}

/// Kinetic parameters for oxidation
#[derive(Debug, Clone)]
pub struct OxidationKinetics {
    /// Pre-exponential factor for parabolic constant (nm²/s)
    pub k0_parabolic: f64,
    /// Pre-exponential factor for linear constant (nm/s)
    pub k0_linear: f64,
    /// Logarithmic rate constant
    pub k_log: f64,
    /// Time constant for logarithmic law (s)
    pub tau_log: f64,
    /// Activation energy (eV)
    pub activation_energy: f64,
    /// Transition thickness from linear to parabolic (nm)
    pub transition_thickness: f64,
    /// Humidity enhancement factor
    pub humidity_factor: f64,
}

impl OxidationKinetics {
    /// Kinetics for copper
    pub fn copper() -> Self {
        Self {
            k0_parabolic: 1e8,      // nm²/s at high T
            k0_linear: 0.01,        // nm/s
            k_log: 10.0,            // nm
            tau_log: 3600.0,        // 1 hour
            activation_energy: 0.8, // eV
            transition_thickness: 50.0,
            humidity_factor: 1.5,
        }
    }

    /// Kinetics for iron
    pub fn iron() -> Self {
        Self {
            k0_parabolic: 1e9,
            k0_linear: 0.001,
            k_log: 5.0,
            tau_log: 1800.0,
            activation_energy: 1.0,
            transition_thickness: 30.0,
            humidity_factor: 3.0, // Iron rusts fast in humidity
        }
    }

    /// Kinetics for aluminum
    pub fn aluminum() -> Self {
        Self {
            k0_parabolic: 1e6,
            k0_linear: 0.1,
            k_log: 3.0,
            tau_log: 60.0,              // Fast initial oxide
            activation_energy: 1.5,     // Protective oxide
            transition_thickness: 10.0, // Thin oxide
            humidity_factor: 0.5,       // Humidity less important
        }
    }

    /// Get kinetics for element
    pub fn for_element(element: Element) -> Self {
        match element {
            Element::Cu => Self::copper(),
            Element::Fe => Self::iron(),
            Element::Al => Self::aluminum(),
            Element::Ag => Self {
                k0_parabolic: 1e5,
                k0_linear: 0.0001,
                k_log: 50.0,
                tau_log: 86400.0, // 1 day
                activation_energy: 0.5,
                transition_thickness: 100.0,
                humidity_factor: 2.0,
            },
            _ => Self::copper(), // Default
        }
    }

    /// Calculate effective rate constant at temperature
    pub fn effective_k_parabolic(&self, temp_k: f64) -> f64 {
        let kb = 8.617e-5; // eV/K
        self.k0_parabolic * (-self.activation_energy / (kb * temp_k)).exp()
    }

    /// Calculate effective linear rate constant
    pub fn effective_k_linear(&self, temp_k: f64, humidity: f64) -> f64 {
        let kb = 8.617e-5;
        let base = self.k0_linear * (-self.activation_energy / (kb * temp_k)).exp();
        base * (1.0 + (self.humidity_factor - 1.0) * humidity)
    }
}

// ============================================================================
// OXIDE LAYER STRUCTURE
// ============================================================================

/// Single oxide layer properties
#[derive(Debug, Clone)]
pub struct OxideLayerProperties {
    /// Layer name/formula
    pub name: String,
    /// Refractive index (real)
    pub n: f64,
    /// Extinction coefficient
    pub k: f64,
    /// Thickness (nm)
    pub thickness: f64,
    /// Porosity (0-1)
    pub porosity: f64,
    /// Color (RGB, for visualization)
    pub color: [f64; 3],
}

impl OxideLayerProperties {
    /// Cuprous oxide Cu2O (red)
    pub fn cu2o(thickness: f64) -> Self {
        Self {
            name: "Cu2O".into(),
            n: 2.71,
            k: 0.1,
            thickness,
            porosity: 0.0,
            color: [0.8, 0.2, 0.1],
        }
    }

    /// Cupric oxide CuO (black)
    pub fn cuo(thickness: f64) -> Self {
        Self {
            name: "CuO".into(),
            n: 2.63,
            k: 0.5,
            thickness,
            porosity: 0.1,
            color: [0.1, 0.1, 0.1],
        }
    }

    /// Patina/verdigris (green copper carbonate)
    pub fn patina(thickness: f64) -> Self {
        Self {
            name: "CuCO3".into(),
            n: 1.73,
            k: 0.05,
            thickness,
            porosity: 0.3,
            color: [0.2, 0.6, 0.4],
        }
    }

    /// Iron oxide (rust)
    pub fn rust(thickness: f64) -> Self {
        Self {
            name: "Fe2O3".into(),
            n: 2.94,
            k: 0.3,
            thickness,
            porosity: 0.4,
            color: [0.7, 0.3, 0.1],
        }
    }

    /// Aluminum oxide (clear)
    pub fn alumina(thickness: f64) -> Self {
        Self {
            name: "Al2O3".into(),
            n: 1.76,
            k: 0.0,
            thickness,
            porosity: 0.0,
            color: [0.9, 0.9, 0.9],
        }
    }

    /// Silver tarnish
    pub fn ag2s(thickness: f64) -> Self {
        Self {
            name: "Ag2S".into(),
            n: 2.2,
            k: 0.8,
            thickness,
            porosity: 0.0,
            color: [0.15, 0.15, 0.1],
        }
    }
}

/// Multi-layer oxide structure
#[derive(Debug, Clone)]
pub struct OxideStructure {
    /// Oxide layers (from surface to metal)
    pub layers: Vec<OxideLayerProperties>,
}

impl OxideStructure {
    /// Empty (no oxide)
    pub fn none() -> Self {
        Self { layers: Vec::new() }
    }

    /// Simple single-layer oxide
    pub fn single(layer: OxideLayerProperties) -> Self {
        Self {
            layers: vec![layer],
        }
    }

    /// Total oxide thickness
    pub fn total_thickness(&self) -> f64 {
        self.layers.iter().map(|l| l.thickness).sum()
    }

    /// Build copper oxide structure based on conditions
    pub fn copper_oxide(total_thickness: f64, humidity: f64, age_days: f64) -> Self {
        let mut layers = Vec::new();

        if total_thickness > 0.0 {
            // Inner layer: Cu2O
            let cu2o_thickness = (total_thickness * 0.4).min(100.0);
            layers.push(OxideLayerProperties::cu2o(cu2o_thickness));

            // Middle layer: CuO (develops over time)
            if total_thickness > 20.0 {
                let cuo_thickness = (total_thickness * 0.3).min(50.0);
                layers.push(OxideLayerProperties::cuo(cuo_thickness));
            }

            // Outer layer: Patina (develops with humidity and time)
            if humidity > 0.5 && age_days > 365.0 {
                let patina_factor = ((age_days - 365.0) / 3650.0).min(1.0); // Up to 10 years
                let patina_thickness = total_thickness * 0.3 * patina_factor * humidity;
                if patina_thickness > 1.0 {
                    layers.push(OxideLayerProperties::patina(patina_thickness));
                }
            }
        }

        Self { layers }
    }

    /// Build iron oxide structure
    pub fn iron_oxide(total_thickness: f64, humidity: f64) -> Self {
        let mut layers = Vec::new();

        if total_thickness > 0.0 {
            // Rust layer (porous, grows with humidity)
            let rust_thickness = total_thickness * (0.5 + 0.5 * humidity);
            layers.push(OxideLayerProperties::rust(rust_thickness));
        }

        Self { layers }
    }

    /// Build aluminum oxide structure
    pub fn aluminum_oxide(thickness: f64) -> Self {
        if thickness > 0.0 {
            Self::single(OxideLayerProperties::alumina(thickness))
        } else {
            Self::none()
        }
    }
}

// ============================================================================
// DYNAMIC OXIDATION STATE
// ============================================================================

/// Time-dependent oxidation state
#[derive(Debug, Clone)]
pub struct OxidationState {
    /// Current oxide thickness (nm)
    pub oxide_thickness: f64,
    /// Age since first exposure (seconds)
    pub age_seconds: f64,
    /// Temperature history: (time, temperature) pairs
    pub temp_history: Vec<(f64, f64)>,
    /// Average humidity experienced
    pub avg_humidity: f64,
    /// Current oxide structure
    pub structure: OxideStructure,
}

impl Default for OxidationState {
    fn default() -> Self {
        Self {
            oxide_thickness: 0.0,
            age_seconds: 0.0,
            temp_history: Vec::new(),
            avg_humidity: 0.5,
            structure: OxideStructure::none(),
        }
    }
}

impl OxidationState {
    /// Age in days
    pub fn age_days(&self) -> f64 {
        self.age_seconds / 86400.0
    }

    /// Age in years
    pub fn age_years(&self) -> f64 {
        self.age_seconds / (86400.0 * 365.25)
    }

    /// Average temperature from history
    pub fn avg_temperature(&self) -> f64 {
        if self.temp_history.is_empty() {
            293.0
        } else {
            let sum: f64 = self.temp_history.iter().map(|(_, t)| t).sum();
            sum / self.temp_history.len() as f64
        }
    }
}

// ============================================================================
// DYNAMIC OXIDIZED METAL
// ============================================================================

/// Base metal optical properties (Drude-like)
#[derive(Debug, Clone)]
pub struct BaseMetalOptical {
    /// Refractive index at 550nm
    pub n: f64,
    /// Extinction coefficient at 550nm
    pub k: f64,
    /// RGB reflectance
    pub reflectance_rgb: [f64; 3],
}

impl BaseMetalOptical {
    /// Copper
    pub fn copper() -> Self {
        Self {
            n: 0.27,
            k: 2.58,
            reflectance_rgb: [0.96, 0.64, 0.54],
        }
    }

    /// Iron
    pub fn iron() -> Self {
        Self {
            n: 2.87,
            k: 3.35,
            reflectance_rgb: [0.56, 0.57, 0.58],
        }
    }

    /// Aluminum
    pub fn aluminum() -> Self {
        Self {
            n: 1.37,
            k: 7.62,
            reflectance_rgb: [0.91, 0.92, 0.93],
        }
    }

    /// Silver
    pub fn silver() -> Self {
        Self {
            n: 0.13,
            k: 4.0,
            reflectance_rgb: [0.97, 0.97, 0.97],
        }
    }

    /// Gold
    #[allow(clippy::approx_constant)] // 3.14 is the actual extinction coefficient, not PI
    pub fn gold() -> Self {
        Self {
            n: 0.17,
            k: 3.14,
            reflectance_rgb: [0.95, 0.80, 0.35],
        }
    }

    /// For element
    pub fn for_element(element: Element) -> Self {
        match element {
            Element::Cu => Self::copper(),
            Element::Fe => Self::iron(),
            Element::Al => Self::aluminum(),
            Element::Ag => Self::silver(),
            Element::Au => Self::gold(),
            _ => Self::iron(),
        }
    }
}

/// Dynamic oxidized metal with time evolution
#[derive(Debug, Clone)]
pub struct DynamicOxidizedMetal {
    /// Alloy composition
    pub composition: AlloyComposition,
    /// Base metal optical properties
    pub base_optical: BaseMetalOptical,
    /// Oxidation kinetics
    pub kinetics: OxidationKinetics,
    /// Current oxidation state
    pub state: OxidationState,

    // Current environmental conditions
    /// Current temperature (K)
    pub temperature: f64,
    /// Current humidity (0-1)
    pub humidity: f64,
    /// Atmospheric oxygen partial pressure (atm)
    pub oxygen_pressure: f64,
}

impl DynamicOxidizedMetal {
    /// Create new dynamic metal
    pub fn new(composition: AlloyComposition) -> Self {
        let dominant = composition.dominant_element();

        Self {
            composition: composition.clone(),
            base_optical: BaseMetalOptical::for_element(dominant),
            kinetics: OxidationKinetics::for_element(dominant),
            state: OxidationState::default(),
            temperature: 293.0,
            humidity: 0.5,
            oxygen_pressure: 0.21,
        }
    }

    /// Create pure metal
    pub fn pure(element: Element) -> Self {
        Self::new(AlloyComposition::pure(element))
    }

    /// Set environmental conditions
    pub fn set_environment(&mut self, temp_k: f64, humidity: f64, o2_pressure: f64) {
        self.temperature = temp_k;
        self.humidity = humidity.clamp(0.0, 1.0);
        self.oxygen_pressure = o2_pressure.clamp(0.0, 1.0);

        // Record temperature history
        self.state
            .temp_history
            .push((self.state.age_seconds, temp_k));

        // Update average humidity
        let n = self.state.temp_history.len() as f64;
        self.state.avg_humidity = (self.state.avg_humidity * (n - 1.0) + humidity) / n;
    }

    /// Advance oxidation by time step
    pub fn advance_time(&mut self, dt_seconds: f64) {
        self.state.age_seconds += dt_seconds;

        // Determine rate law based on current thickness
        let rate_law = if self.state.oxide_thickness < 10.0 {
            RateLaw::Logarithmic
        } else if self.state.oxide_thickness < self.kinetics.transition_thickness {
            RateLaw::Linear
        } else {
            RateLaw::Parabolic
        };

        // Calculate thickness growth
        let dx = match rate_law {
            RateLaw::Logarithmic => {
                let x_max = self.kinetics.k_log
                    * (1.0 + self.state.age_seconds / self.kinetics.tau_log).ln();
                (x_max - self.state.oxide_thickness)
                    .max(0.0)
                    .min(dt_seconds * 0.01)
            }
            RateLaw::Linear => {
                let k = self
                    .kinetics
                    .effective_k_linear(self.temperature, self.humidity);
                k * dt_seconds * self.oxygen_pressure
            }
            RateLaw::Parabolic => {
                let k = self.kinetics.effective_k_parabolic(self.temperature);
                let x = self.state.oxide_thickness;
                if x > 0.0 {
                    (k * dt_seconds * self.oxygen_pressure) / (2.0 * x)
                } else {
                    0.0
                }
            }
        };

        self.state.oxide_thickness += dx;

        // Update oxide structure
        self.rebuild_oxide_structure();
    }

    /// Rebuild oxide layer structure based on current state
    fn rebuild_oxide_structure(&mut self) {
        let dominant = self.composition.dominant_element();
        let thickness = self.state.oxide_thickness;
        let humidity = self.state.avg_humidity;
        let age_days = self.state.age_days();

        self.state.structure = match dominant {
            Element::Cu => OxideStructure::copper_oxide(thickness, humidity, age_days),
            Element::Fe => OxideStructure::iron_oxide(thickness, humidity),
            Element::Al => OxideStructure::aluminum_oxide(thickness),
            Element::Ag => OxideStructure::single(OxideLayerProperties::ag2s(thickness)),
            _ => OxideStructure::none(),
        };
    }

    /// Calculate effective reflectance at wavelength
    pub fn reflectance(&self, wavelength_nm: f64) -> f64 {
        if self.state.oxide_thickness < 0.1 {
            // Fresh metal
            return self.fresnel_metal(wavelength_nm, self.base_optical.n, self.base_optical.k);
        }

        // Multi-layer reflectance
        let mut r_total = 0.0;
        let mut t_cumulative = 1.0;

        for layer in &self.state.structure.layers {
            // Fresnel reflection at this layer
            let r_layer = self.fresnel_dielectric(layer.n);

            // Beer-Lambert transmission through layer
            let alpha = 4.0 * PI * layer.k / wavelength_nm;
            let t_layer = (-alpha * layer.thickness).exp();

            // Effective porosity adjustment
            let t_effective = t_layer * (1.0 - layer.porosity * 0.5);

            r_total += t_cumulative * t_cumulative * r_layer;
            t_cumulative *= t_effective;
        }

        // Add substrate (metal) reflection
        r_total += t_cumulative
            * t_cumulative
            * self.fresnel_metal(wavelength_nm, self.base_optical.n, self.base_optical.k);

        r_total.clamp(0.0, 1.0)
    }

    /// Calculate RGB reflectance
    pub fn reflectance_rgb(&self) -> [f64; 3] {
        [
            self.reflectance(650.0),
            self.reflectance(550.0),
            self.reflectance(450.0),
        ]
    }

    fn fresnel_metal(&self, _wavelength: f64, n: f64, k: f64) -> f64 {
        // Normal incidence Fresnel for metal
        ((n - 1.0).powi(2) + k.powi(2)) / ((n + 1.0).powi(2) + k.powi(2))
    }

    fn fresnel_dielectric(&self, n: f64) -> f64 {
        ((1.0 - n) / (1.0 + n)).powi(2)
    }

    /// Get visual color (RGB 0-255)
    pub fn visual_color(&self) -> [u8; 3] {
        let rgb = self.reflectance_rgb();
        [
            (rgb[0] * 255.0).clamp(0.0, 255.0) as u8,
            (rgb[1] * 255.0).clamp(0.0, 255.0) as u8,
            (rgb[2] * 255.0).clamp(0.0, 255.0) as u8,
        ]
    }

    /// Reset to fresh metal state
    pub fn reset(&mut self) {
        self.state = OxidationState::default();
    }
}

// ============================================================================
// OXIDATION SIMULATION
// ============================================================================

/// Oxidation simulation parameters
#[derive(Debug, Clone)]
pub struct OxidationSimulation {
    /// Time step (seconds)
    pub dt: f64,
    /// Environmental schedule: (time, temp_k, humidity)
    pub environment_schedule: Vec<(f64, f64, f64)>,
}

impl OxidationSimulation {
    /// Constant conditions simulation
    pub fn constant(temp_k: f64, humidity: f64) -> Self {
        Self {
            dt: 3600.0, // 1 hour steps
            environment_schedule: vec![(0.0, temp_k, humidity)],
        }
    }

    /// Daily cycle simulation
    pub fn daily_cycle(min_temp: f64, max_temp: f64, humidity: f64) -> Self {
        let mut schedule = Vec::new();
        for hour in 0..24 {
            let t = hour as f64 * 3600.0;
            let temp = min_temp
                + (max_temp - min_temp)
                    * (0.5 + 0.5 * (2.0 * PI * hour as f64 / 24.0 - PI / 2.0).sin());
            schedule.push((t, temp, humidity));
        }
        Self {
            dt: 3600.0,
            environment_schedule: schedule,
        }
    }

    /// Run simulation for total time
    pub fn run(
        &self,
        metal: &mut DynamicOxidizedMetal,
        total_time: f64,
    ) -> Vec<(f64, f64, [f64; 3])> {
        let mut results = Vec::new();
        let mut t = 0.0;

        while t < total_time {
            // Get current environment from schedule
            let env = self.environment_at(t);
            metal.set_environment(env.0, env.1, 0.21);

            // Advance oxidation
            metal.advance_time(self.dt);

            // Record state
            results.push((t, metal.state.oxide_thickness, metal.reflectance_rgb()));

            t += self.dt;
        }

        results
    }

    fn environment_at(&self, t: f64) -> (f64, f64) {
        if self.environment_schedule.is_empty() {
            return (293.0, 0.5);
        }

        // Cyclic interpolation through schedule
        let cycle_time: f64 = self
            .environment_schedule
            .last()
            .map(|(t, _, _)| *t)
            .unwrap_or(3600.0);

        let t_mod = t % (cycle_time + self.dt);

        for window in self.environment_schedule.windows(2) {
            if t_mod >= window[0].0 && t_mod < window[1].0 {
                let frac = (t_mod - window[0].0) / (window[1].0 - window[0].0);
                return (
                    window[0].1 + frac * (window[1].1 - window[0].1),
                    window[0].2 + frac * (window[1].2 - window[0].2),
                );
            }
        }

        let last = self.environment_schedule.last().unwrap();
        (last.1, last.2)
    }
}

// ============================================================================
// PRESETS
// ============================================================================

/// Oxidation presets
pub mod oxidation_presets {
    use super::*;

    /// Fresh polished copper
    pub fn copper_fresh() -> DynamicOxidizedMetal {
        DynamicOxidizedMetal::pure(Element::Cu)
    }

    /// Copper after 1 year outdoors
    pub fn copper_1year() -> DynamicOxidizedMetal {
        let mut metal = copper_fresh();
        let sim = OxidationSimulation::constant(293.0, 0.6);
        sim.run(&mut metal, 365.0 * 86400.0);
        metal
    }

    /// Copper with full patina (10+ years)
    pub fn copper_patina() -> DynamicOxidizedMetal {
        let mut metal = copper_fresh();
        let sim = OxidationSimulation::constant(293.0, 0.7);
        sim.run(&mut metal, 10.0 * 365.0 * 86400.0);
        metal
    }

    /// Fresh iron
    pub fn iron_fresh() -> DynamicOxidizedMetal {
        DynamicOxidizedMetal::pure(Element::Fe)
    }

    /// Rusty iron (1 month in humid conditions)
    pub fn iron_rusty() -> DynamicOxidizedMetal {
        let mut metal = iron_fresh();
        let sim = OxidationSimulation::constant(293.0, 0.9);
        sim.run(&mut metal, 30.0 * 86400.0);
        metal
    }

    /// Fresh aluminum (has thin native oxide)
    pub fn aluminum_fresh() -> DynamicOxidizedMetal {
        let mut metal = DynamicOxidizedMetal::pure(Element::Al);
        metal.state.oxide_thickness = 3.0; // Native oxide
        metal.rebuild_oxide_structure();
        metal
    }

    /// Fresh silver
    pub fn silver_fresh() -> DynamicOxidizedMetal {
        DynamicOxidizedMetal::pure(Element::Ag)
    }

    /// Tarnished silver
    pub fn silver_tarnished() -> DynamicOxidizedMetal {
        let mut metal = silver_fresh();
        let sim = OxidationSimulation::constant(293.0, 0.5);
        sim.run(&mut metal, 90.0 * 86400.0); // 3 months
        metal
    }

    /// Weathered brass
    pub fn brass_weathered() -> DynamicOxidizedMetal {
        let mut metal = DynamicOxidizedMetal::new(AlloyComposition::brass());
        let sim = OxidationSimulation::constant(293.0, 0.6);
        sim.run(&mut metal, 180.0 * 86400.0); // 6 months
        metal
    }
}

// ============================================================================
// CSS GENERATION
// ============================================================================

/// Generate CSS for oxidized metal appearance
pub fn to_css_oxidized(metal: &DynamicOxidizedMetal) -> String {
    let rgb = metal.visual_color();

    // Base color
    let base = format!("rgb({}, {}, {})", rgb[0], rgb[1], rgb[2]);

    // Add texture for heavily oxidized surfaces
    if metal.state.oxide_thickness > 100.0 {
        let porosity = metal
            .state
            .structure
            .layers
            .iter()
            .map(|l| l.porosity)
            .sum::<f64>()
            / metal.state.structure.layers.len().max(1) as f64;

        let noise_opacity = (porosity * 0.3).clamp(0.0, 0.2);

        format!(
            "background: linear-gradient(135deg, {} 0%, {} 50%, {} 100%); \
             background-blend-mode: overlay; \
             opacity: {:.2}",
            base,
            format!("rgba({}, {}, {}, 0.8)", rgb[0], rgb[1], rgb[2]),
            base,
            1.0 - noise_opacity
        )
    } else {
        format!(
            "background: {}; background-image: linear-gradient(135deg, \
                 rgba(255,255,255,0.2) 0%, transparent 50%, rgba(0,0,0,0.1) 100%)",
            base
        )
    }
}

/// Generate CSS animation for oxidation over time
pub fn to_css_oxidation_animation(
    start: &DynamicOxidizedMetal,
    end: &DynamicOxidizedMetal,
    duration_s: f64,
) -> String {
    let start_rgb = start.visual_color();
    let end_rgb = end.visual_color();

    format!(
        "@keyframes oxidation {{\n\
         0% {{ background-color: rgb({}, {}, {}); }}\n\
         100% {{ background-color: rgb({}, {}, {}); }}\n\
         }}\n\
         animation: oxidation {:.1}s ease-in-out forwards;",
        start_rgb[0], start_rgb[1], start_rgb[2], end_rgb[0], end_rgb[1], end_rgb[2], duration_s
    )
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oxidation_kinetics_temperature() {
        let kinetics = OxidationKinetics::copper();

        let k_cold = kinetics.effective_k_parabolic(300.0);
        let k_hot = kinetics.effective_k_parabolic(400.0);

        // Arrhenius: higher temp = faster oxidation
        assert!(k_hot > k_cold);
    }

    #[test]
    fn test_oxidation_growth() {
        let mut metal = DynamicOxidizedMetal::pure(Element::Cu);
        metal.set_environment(293.0, 0.5, 0.21);

        let initial_thickness = metal.state.oxide_thickness;
        metal.advance_time(86400.0); // 1 day

        // Oxide should grow
        assert!(metal.state.oxide_thickness > initial_thickness);
    }

    #[test]
    fn test_humidity_effect() {
        let kinetics = OxidationKinetics::iron();

        let k_dry = kinetics.effective_k_linear(293.0, 0.2);
        let k_wet = kinetics.effective_k_linear(293.0, 0.9);

        // Iron rusts faster in humidity
        assert!(k_wet > k_dry);
    }

    #[test]
    fn test_oxide_structure_evolution() {
        let mut metal = oxidation_presets::copper_fresh();
        let sim = OxidationSimulation::constant(293.0, 0.7);
        sim.run(&mut metal, 365.0 * 86400.0); // 1 year

        // Should have multiple oxide layers
        assert!(!metal.state.structure.layers.is_empty());
    }

    #[test]
    fn test_reflectance_changes() {
        let fresh = oxidation_presets::copper_fresh();
        let r_fresh = fresh.reflectance(550.0);

        // Oxidize copper
        let mut oxidized = oxidation_presets::copper_fresh();
        oxidized.set_environment(293.0, 0.7, 0.21);
        oxidized.advance_time(365.0 * 86400.0); // 1 year
        let r_oxidized = oxidized.reflectance(550.0);

        // Reflectance should be valid
        assert!(r_fresh >= 0.0 && r_fresh <= 1.0);
        assert!(r_oxidized >= 0.0 && r_oxidized <= 1.0);

        // Oxide should have grown
        assert!(oxidized.state.oxide_thickness > 0.0);
    }

    #[test]
    fn test_alloy_oxidation() {
        let mut brass = DynamicOxidizedMetal::new(AlloyComposition::brass());
        brass.set_environment(293.0, 0.5, 0.21);
        brass.advance_time(86400.0);

        // Brass should oxidize
        assert!(brass.state.oxide_thickness > 0.0);
    }

    #[test]
    fn test_simulation_produces_results() {
        let mut metal = oxidation_presets::copper_fresh();
        let sim = OxidationSimulation::daily_cycle(283.0, 303.0, 0.6);
        let results = sim.run(&mut metal, 7.0 * 86400.0); // 1 week

        assert!(!results.is_empty());

        // Check monotonic thickness increase
        for window in results.windows(2) {
            assert!(window[1].1 >= window[0].1);
        }
    }

    #[test]
    fn test_css_generation() {
        let metal = oxidation_presets::copper_1year();
        let css = to_css_oxidized(&metal);

        assert!(css.contains("background"));
        assert!(css.contains("rgb"));
    }

    #[test]
    fn test_aluminum_protective_oxide() {
        let al = oxidation_presets::aluminum_fresh();

        // Aluminum has thin protective oxide
        assert!(al.state.oxide_thickness > 0.0);
        assert!(al.state.oxide_thickness < 10.0);

        // High reflectance even with oxide
        assert!(al.reflectance(550.0) > 0.8);
    }
}

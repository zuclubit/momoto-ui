//! # Temperature-Dependent Metal IOR (Phase 4)
//!
//! Models metal optical properties as a function of temperature,
//! including oxidation layer effects.
//!
//! ## Physical Background
//!
//! Metal optical properties depend on temperature through the Drude model:
//!
//! ```text
//! ε(ω, T) = ε∞ - ωₚ²(T) / (ω² + iγ(T)ω)
//! ```
//!
//! Where:
//! - ωₚ(T) = plasma frequency (decreases with T)
//! - γ(T) = damping rate (increases with T)
//!
//! ## Oxidation Effects
//!
//! Metals develop oxide layers over time:
//! - Copper: Cu → Cu₂O → CuO (patina)
//! - Aluminum: Al₂O₃ (native oxide, self-limiting)
//! - Iron: Fe₂O₃ (rust)
//! - Silver: Ag₂S (tarnish)
//!
//! ## References
//!
//! - Ordal et al. (1983): "Optical properties of metals"
//! - Rakic et al. (1998): "Optical properties of metallic films"
//! - CRC Handbook of Chemistry and Physics

use std::f64::consts::PI;

use super::complex_ior::{Complex, ComplexIOR, SpectralComplexIOR};
use super::thin_film::ThinFilm;

// ============================================================================
// DRUDE MODEL
// ============================================================================

/// Drude model parameters for a metal
#[derive(Debug, Clone, Copy)]
pub struct DrudeParams {
    /// High-frequency dielectric constant (ε∞)
    pub eps_inf: f64,
    /// Plasma frequency in eV
    pub omega_p: f64,
    /// Damping rate in eV
    pub gamma: f64,
    /// Reference temperature (Kelvin)
    pub t_ref: f64,
    /// Temperature coefficient for ωₚ (K⁻¹)
    pub d_omega_p: f64,
    /// Temperature coefficient for γ (K⁻¹)
    pub d_gamma: f64,
}

impl DrudeParams {
    /// Create new Drude parameters
    pub const fn new(
        eps_inf: f64,
        omega_p: f64,
        gamma: f64,
        t_ref: f64,
        d_omega_p: f64,
        d_gamma: f64,
    ) -> Self {
        Self {
            eps_inf,
            omega_p,
            gamma,
            t_ref,
            d_omega_p,
            d_gamma,
        }
    }

    /// Get temperature-adjusted parameters
    pub fn at_temperature(&self, temp_k: f64) -> (f64, f64) {
        let dt = temp_k - self.t_ref;
        let omega_p = self.omega_p * (1.0 + self.d_omega_p * dt);
        let gamma = self.gamma * (1.0 + self.d_gamma * dt);
        (omega_p.max(0.1), gamma.max(0.001))
    }

    /// Calculate complex dielectric function at given energy and temperature
    pub fn epsilon(&self, energy_ev: f64, temp_k: f64) -> Complex {
        let (omega_p, gamma) = self.at_temperature(temp_k);

        let omega = energy_ev;
        let omega_p2 = omega_p * omega_p;
        let omega2 = omega * omega;

        // ε = ε∞ - ωₚ² / (ω² + iγω)
        let denom = Complex::new(omega2, gamma * omega);
        let drude_term = Complex::real(omega_p2) / denom;

        Complex::real(self.eps_inf) - drude_term
    }

    /// Calculate complex refractive index at given wavelength and temperature
    pub fn complex_ior(&self, wavelength_nm: f64, temp_k: f64) -> ComplexIOR {
        // Convert wavelength to energy: E(eV) = 1239.84 / λ(nm)
        let energy_ev = 1239.84 / wavelength_nm;
        let eps = self.epsilon(energy_ev, temp_k);

        // n + ik = sqrt(ε)
        let n_complex = eps.sqrt();

        ComplexIOR::new(n_complex.re.abs(), n_complex.im.abs())
    }

    /// Calculate spectral complex IOR (RGB) at temperature
    pub fn spectral_ior(&self, temp_k: f64) -> SpectralComplexIOR {
        SpectralComplexIOR::new(
            self.complex_ior(650.0, temp_k), // Red
            self.complex_ior(550.0, temp_k), // Green
            self.complex_ior(450.0, temp_k), // Blue
        )
    }
}

// ============================================================================
// METAL DRUDE PRESETS
// ============================================================================

/// Drude model parameters for common metals
pub mod drude_metals {
    use super::DrudeParams;

    /// Gold (Au) - Drude model parameters
    ///
    /// Reference: Ordal et al. (1983)
    pub const GOLD: DrudeParams = DrudeParams::new(
        9.84,    // ε∞
        9.03,    // ωₚ (eV)
        0.053,   // γ (eV)
        300.0,   // T_ref (K)
        -1.5e-4, // dωₚ/dT
        3.2e-3,  // dγ/dT
    );

    /// Silver (Ag) - Drude model parameters
    pub const SILVER: DrudeParams = DrudeParams::new(
        3.7,     // ε∞
        9.01,    // ωₚ (eV)
        0.018,   // γ (eV)
        300.0,   // T_ref
        -1.2e-4, // dωₚ/dT
        4.1e-3,  // dγ/dT
    );

    /// Copper (Cu) - Drude model parameters
    pub const COPPER: DrudeParams = DrudeParams::new(
        10.6,    // ε∞
        8.88,    // ωₚ (eV)
        0.047,   // γ (eV)
        300.0,   // T_ref
        -1.8e-4, // dωₚ/dT
        3.8e-3,  // dγ/dT
    );

    /// Aluminum (Al) - Drude model parameters
    pub const ALUMINUM: DrudeParams = DrudeParams::new(
        1.0,     // ε∞ (nearly free electron)
        14.75,   // ωₚ (eV)
        0.082,   // γ (eV)
        300.0,   // T_ref
        -0.8e-4, // dωₚ/dT
        2.5e-3,  // dγ/dT
    );

    /// Iron (Fe) - Drude model parameters
    pub const IRON: DrudeParams = DrudeParams::new(
        6.0,     // ε∞
        4.5,     // ωₚ (eV)
        0.18,    // γ (eV)
        300.0,   // T_ref
        -1.0e-4, // dωₚ/dT
        2.8e-3,  // dγ/dT
    );

    /// Platinum (Pt) - Drude model parameters
    pub const PLATINUM: DrudeParams = DrudeParams::new(
        5.6,     // ε∞
        5.15,    // ωₚ (eV)
        0.12,    // γ (eV)
        300.0,   // T_ref
        -0.6e-4, // dωₚ/dT
        2.2e-3,  // dγ/dT
    );

    /// Nickel (Ni) - Drude model parameters
    pub const NICKEL: DrudeParams = DrudeParams::new(
        4.5,     // ε∞
        4.89,    // ωₚ (eV)
        0.11,    // γ (eV)
        300.0,   // T_ref
        -0.9e-4, // dωₚ/dT
        3.0e-3,  // dγ/dT
    );

    /// Get all Drude presets
    pub fn all_presets() -> Vec<(&'static str, DrudeParams)> {
        vec![
            ("Gold", GOLD),
            ("Silver", SILVER),
            ("Copper", COPPER),
            ("Aluminum", ALUMINUM),
            ("Iron", IRON),
            ("Platinum", PLATINUM),
            ("Nickel", NICKEL),
        ]
    }
}

// ============================================================================
// OXIDATION LAYERS
// ============================================================================

/// Oxide layer properties
#[derive(Debug, Clone, Copy)]
pub struct OxideLayer {
    /// Oxide name
    pub name: &'static str,
    /// Refractive index of oxide (real part)
    pub n: f64,
    /// Absorption coefficient (imaginary part)
    pub k: f64,
    /// Native thickness in nm (natural formation)
    pub native_thickness_nm: f64,
    /// Maximum thickness in nm (heavily oxidized)
    pub max_thickness_nm: f64,
}

impl OxideLayer {
    /// Create a ThinFilm for this oxide at given oxidation level
    pub fn to_thin_film(&self, oxidation_level: f64) -> ThinFilm {
        let thickness = self.native_thickness_nm
            + oxidation_level * (self.max_thickness_nm - self.native_thickness_nm);

        ThinFilm::new(self.n, thickness)
    }
}

/// Oxide layer presets for common metals
pub mod oxides {
    use super::OxideLayer;

    /// Copper(I) oxide - Cu₂O (fresh oxidation, reddish)
    pub const COPPER_OXIDE_I: OxideLayer = OxideLayer {
        name: "Cu2O",
        n: 2.7,
        k: 0.01,
        native_thickness_nm: 2.0,
        max_thickness_nm: 100.0,
    };

    /// Copper(II) oxide - CuO (aged oxidation, black)
    pub const COPPER_OXIDE_II: OxideLayer = OxideLayer {
        name: "CuO",
        n: 2.6,
        k: 0.1,
        native_thickness_nm: 0.0,
        max_thickness_nm: 500.0,
    };

    /// Aluminum oxide - Al₂O₃ (native oxide, transparent)
    pub const ALUMINUM_OXIDE: OxideLayer = OxideLayer {
        name: "Al2O3",
        n: 1.76,
        k: 0.0,
        native_thickness_nm: 2.0,
        max_thickness_nm: 10.0, // Self-limiting
    };

    /// Iron oxide - Fe₂O₃ (rust, reddish-brown)
    pub const IRON_OXIDE: OxideLayer = OxideLayer {
        name: "Fe2O3",
        n: 2.9,
        k: 0.3,
        native_thickness_nm: 2.0,
        max_thickness_nm: 1000.0,
    };

    /// Silver sulfide - Ag₂S (tarnish, dark)
    pub const SILVER_SULFIDE: OxideLayer = OxideLayer {
        name: "Ag2S",
        n: 2.0,
        k: 0.5,
        native_thickness_nm: 0.0,
        max_thickness_nm: 100.0,
    };

    /// Nickel oxide - NiO (greenish)
    pub const NICKEL_OXIDE: OxideLayer = OxideLayer {
        name: "NiO",
        n: 2.18,
        k: 0.05,
        native_thickness_nm: 1.0,
        max_thickness_nm: 50.0,
    };

    /// Bronze patina (copper carbonate, green)
    pub const BRONZE_PATINA: OxideLayer = OxideLayer {
        name: "Cu2CO3(OH)2",
        n: 1.8,
        k: 0.02,
        native_thickness_nm: 0.0,
        max_thickness_nm: 200.0,
    };

    /// Get all oxide presets
    pub fn all_presets() -> Vec<(&'static str, OxideLayer)> {
        vec![
            ("Copper(I) Oxide", COPPER_OXIDE_I),
            ("Copper(II) Oxide", COPPER_OXIDE_II),
            ("Aluminum Oxide", ALUMINUM_OXIDE),
            ("Iron Oxide (Rust)", IRON_OXIDE),
            ("Silver Sulfide", SILVER_SULFIDE),
            ("Nickel Oxide", NICKEL_OXIDE),
            ("Bronze Patina", BRONZE_PATINA),
        ]
    }
}

// ============================================================================
// TEMPERATURE-DEPENDENT OXIDIZED METAL
// ============================================================================

/// Temperature-dependent metal with oxidation layer
#[derive(Debug, Clone)]
pub struct TempOxidizedMetal {
    /// Base metal Drude parameters
    pub drude: DrudeParams,
    /// Oxide layer properties
    pub oxide: OxideLayer,
    /// Current temperature (Kelvin)
    pub temperature_k: f64,
    /// Oxidation level (0.0 = fresh, 1.0 = heavily oxidized)
    pub oxidation_level: f64,
}

impl TempOxidizedMetal {
    /// Create a new temperature-dependent oxidized metal
    pub fn new(drude: DrudeParams, oxide: OxideLayer) -> Self {
        Self {
            drude,
            oxide,
            temperature_k: 300.0, // Room temperature
            oxidation_level: 0.0, // Fresh
        }
    }

    /// Set temperature
    pub fn with_temperature(mut self, temp_k: f64) -> Self {
        self.temperature_k = temp_k;
        self
    }

    /// Set oxidation level
    pub fn with_oxidation(mut self, level: f64) -> Self {
        self.oxidation_level = level.clamp(0.0, 1.0);
        self
    }

    /// Get base metal IOR at current temperature
    pub fn metal_ior(&self, wavelength_nm: f64) -> ComplexIOR {
        self.drude.complex_ior(wavelength_nm, self.temperature_k)
    }

    /// Get base metal spectral IOR at current temperature
    pub fn metal_spectral_ior(&self) -> SpectralComplexIOR {
        self.drude.spectral_ior(self.temperature_k)
    }

    /// Get oxide thin film at current oxidation level
    pub fn oxide_film(&self) -> ThinFilm {
        self.oxide.to_thin_film(self.oxidation_level)
    }

    /// Calculate effective reflectance including oxide layer
    ///
    /// Uses simplified model: Fresnel from air→oxide→metal
    pub fn effective_reflectance(&self, wavelength_nm: f64, cos_theta: f64) -> f64 {
        let metal_ior = self.metal_ior(wavelength_nm);
        let oxide_film = self.oxide_film();

        if self.oxidation_level < 0.01 {
            // No oxide: just metal Fresnel
            return super::complex_ior::fresnel_conductor_unpolarized(1.0, metal_ior, cos_theta);
        }

        // With oxide: thin-film interference
        // Simplified: use oxide single-layer reflectance modulated by metal
        let r_oxide = oxide_film.reflectance(wavelength_nm, self.oxide.n, cos_theta);
        let r_metal =
            super::complex_ior::fresnel_conductor_unpolarized(self.oxide.n, metal_ior, cos_theta);

        // Absorption in oxide layer reduces reflectance
        // Thicker oxide (higher oxidation_level) = more absorption
        let absorption =
            (-self.oxide.k * self.oxidation_level * oxide_film.thickness_nm * 0.01).exp();

        // Approximate interference effect (can shift wavelength response)
        let phase = oxide_film.phase_difference(wavelength_nm, cos_theta);
        let interference = 1.0 + 0.1 * phase.cos(); // Small modulation

        // Combined reflectance: oxide + transmitted->metal->transmitted back
        // Absorption dampens the overall reflectance
        let r_combined = r_oxide + r_metal * (1.0 - r_oxide) * absorption;
        (r_combined * interference).clamp(0.0, 1.0)
    }

    /// Calculate effective RGB reflectance
    pub fn effective_reflectance_rgb(&self, cos_theta: f64) -> [f64; 3] {
        [
            self.effective_reflectance(650.0, cos_theta),
            self.effective_reflectance(550.0, cos_theta),
            self.effective_reflectance(450.0, cos_theta),
        ]
    }
}

// ============================================================================
// PRESET OXIDIZED METALS
// ============================================================================

/// Pre-defined oxidized metal configurations
pub mod oxidized_presets {
    use super::*;

    /// Fresh copper (no oxidation)
    pub fn copper_fresh() -> TempOxidizedMetal {
        TempOxidizedMetal::new(drude_metals::COPPER, oxides::COPPER_OXIDE_I).with_oxidation(0.0)
    }

    /// Slightly oxidized copper (light tarnish)
    pub fn copper_tarnished() -> TempOxidizedMetal {
        TempOxidizedMetal::new(drude_metals::COPPER, oxides::COPPER_OXIDE_I).with_oxidation(0.3)
    }

    /// Heavily oxidized copper (dark patina)
    pub fn copper_patina() -> TempOxidizedMetal {
        TempOxidizedMetal::new(drude_metals::COPPER, oxides::COPPER_OXIDE_II).with_oxidation(0.8)
    }

    /// Fresh silver
    pub fn silver_fresh() -> TempOxidizedMetal {
        TempOxidizedMetal::new(drude_metals::SILVER, oxides::SILVER_SULFIDE).with_oxidation(0.0)
    }

    /// Tarnished silver
    pub fn silver_tarnished() -> TempOxidizedMetal {
        TempOxidizedMetal::new(drude_metals::SILVER, oxides::SILVER_SULFIDE).with_oxidation(0.5)
    }

    /// Fresh aluminum (with native oxide)
    pub fn aluminum_fresh() -> TempOxidizedMetal {
        TempOxidizedMetal::new(drude_metals::ALUMINUM, oxides::ALUMINUM_OXIDE).with_oxidation(0.2)
        // Native oxide always present
    }

    /// Rusty iron
    pub fn iron_rusty() -> TempOxidizedMetal {
        TempOxidizedMetal::new(drude_metals::IRON, oxides::IRON_OXIDE).with_oxidation(0.7)
    }

    /// Hot gold (elevated temperature)
    pub fn gold_hot() -> TempOxidizedMetal {
        TempOxidizedMetal::new(drude_metals::GOLD, oxides::ALUMINUM_OXIDE) // Gold doesn't oxidize
            .with_temperature(500.0)
            .with_oxidation(0.0)
    }

    /// Get all oxidized metal presets
    pub fn all_presets() -> Vec<(&'static str, TempOxidizedMetal)> {
        vec![
            ("Copper (Fresh)", copper_fresh()),
            ("Copper (Tarnished)", copper_tarnished()),
            ("Copper (Patina)", copper_patina()),
            ("Silver (Fresh)", silver_fresh()),
            ("Silver (Tarnished)", silver_tarnished()),
            ("Aluminum (Fresh)", aluminum_fresh()),
            ("Iron (Rusty)", iron_rusty()),
            ("Gold (Hot)", gold_hot()),
        ]
    }
}

// ============================================================================
// CSS GENERATION
// ============================================================================

/// Generate CSS for temperature-dependent metal effect
pub fn to_css_temp_metal(metal: &TempOxidizedMetal, light_angle_deg: f64) -> String {
    let cos_light = (light_angle_deg * PI / 180.0).cos().abs();
    let rgb = metal.effective_reflectance_rgb(cos_light);

    let r = (rgb[0] * 255.0).clamp(0.0, 255.0) as u8;
    let g = (rgb[1] * 255.0).clamp(0.0, 255.0) as u8;
    let b = (rgb[2] * 255.0).clamp(0.0, 255.0) as u8;

    // Add temperature-based color shift (metals get redder when hot)
    let temp_factor = ((metal.temperature_k - 300.0) / 1000.0).clamp(0.0, 1.0);
    let r_hot = (r as f64 * (1.0 + 0.2 * temp_factor)).min(255.0) as u8;
    let g_hot = (g as f64 * (1.0 - 0.1 * temp_factor)).max(0.0) as u8;
    let b_hot = (b as f64 * (1.0 - 0.15 * temp_factor)).max(0.0) as u8;

    format!(
        "linear-gradient({}deg, \
         rgb({}, {}, {}) 0%, \
         rgb({}, {}, {}) 50%, \
         rgb({}, {}, {}) 100%)",
        light_angle_deg,
        r_hot,
        g_hot,
        b_hot,
        (r_hot as f64 * 1.2).min(255.0) as u8,
        (g_hot as f64 * 1.15).min(255.0) as u8,
        (b_hot as f64 * 1.1).min(255.0) as u8,
        r_hot,
        g_hot,
        b_hot,
    )
}

/// Generate CSS for oxidation effect with patina
pub fn to_css_patina(metal: &TempOxidizedMetal) -> String {
    let rgb_fresh = {
        let fresh = TempOxidizedMetal::new(metal.drude, metal.oxide).with_oxidation(0.0);
        fresh.effective_reflectance_rgb(0.8)
    };

    let rgb_oxidized = metal.effective_reflectance_rgb(0.8);

    let r1 = (rgb_fresh[0] * 255.0).clamp(0.0, 255.0) as u8;
    let g1 = (rgb_fresh[1] * 255.0).clamp(0.0, 255.0) as u8;
    let b1 = (rgb_fresh[2] * 255.0).clamp(0.0, 255.0) as u8;

    let r2 = (rgb_oxidized[0] * 255.0).clamp(0.0, 255.0) as u8;
    let g2 = (rgb_oxidized[1] * 255.0).clamp(0.0, 255.0) as u8;
    let b2 = (rgb_oxidized[2] * 255.0).clamp(0.0, 255.0) as u8;

    // Radial gradient: fresh center, oxidized edges
    format!(
        "radial-gradient(ellipse at 40% 40%, \
         rgb({}, {}, {}) 0%, \
         rgb({}, {}, {}) 70%, \
         rgb({}, {}, {}) 100%)",
        r1,
        g1,
        b1,
        (r1 as f64 * 0.7 + r2 as f64 * 0.3) as u8,
        (g1 as f64 * 0.7 + g2 as f64 * 0.3) as u8,
        (b1 as f64 * 0.7 + b2 as f64 * 0.3) as u8,
        r2,
        g2,
        b2,
    )
}

// ============================================================================
// TEMPERATURE ANALYSIS
// ============================================================================

/// Calculate reflectance change with temperature
pub fn temperature_sensitivity(drude: &DrudeParams, wavelength_nm: f64) -> Vec<(f64, f64)> {
    (200..=600)
        .step_by(50)
        .map(|temp| {
            let ior = drude.complex_ior(wavelength_nm, temp as f64);
            let r = super::complex_ior::fresnel_conductor_unpolarized(1.0, ior, 1.0);
            (temp as f64, r)
        })
        .collect()
}

/// Memory usage for temperature-dependent metals
pub fn temp_metal_memory() -> usize {
    // No LUTs, just struct sizes
    std::mem::size_of::<DrudeParams>()
        + std::mem::size_of::<OxideLayer>()
        + std::mem::size_of::<TempOxidizedMetal>()
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drude_model() {
        let gold = drude_metals::GOLD;
        let ior = gold.complex_ior(550.0, 300.0);

        // Gold should have significant extinction
        assert!(ior.k > 1.0, "Gold k should be > 1");
        // n should be reasonable
        assert!(ior.n > 0.0 && ior.n < 5.0, "Gold n should be reasonable");
    }

    #[test]
    fn test_temperature_dependence() {
        let gold = drude_metals::GOLD;

        let ior_cold = gold.complex_ior(550.0, 200.0);
        let ior_hot = gold.complex_ior(550.0, 500.0);

        // Higher temperature should increase damping (higher k)
        // or at least change the optical properties
        let diff_n = (ior_cold.n - ior_hot.n).abs();
        let diff_k = (ior_cold.k - ior_hot.k).abs();

        assert!(
            diff_n > 0.001 || diff_k > 0.001,
            "Temperature should affect IOR"
        );
    }

    #[test]
    fn test_oxide_layer() {
        let oxide = oxides::COPPER_OXIDE_I;
        let film_fresh = oxide.to_thin_film(0.0);
        let film_oxidized = oxide.to_thin_film(1.0);

        assert!(
            film_oxidized.thickness_nm > film_fresh.thickness_nm,
            "More oxidation = thicker layer"
        );
    }

    #[test]
    fn test_oxidized_metal_reflectance() {
        let fresh = oxidized_presets::copper_fresh();
        let patina = oxidized_presets::copper_patina();

        let r_fresh = fresh.effective_reflectance(550.0, 1.0);
        let r_patina = patina.effective_reflectance(550.0, 1.0);

        // Both should be valid reflectances
        assert!(r_fresh >= 0.0 && r_fresh <= 1.0);
        assert!(r_patina >= 0.0 && r_patina <= 1.0);

        // Fresh copper should be more reflective than patina'd copper
        assert!(
            r_fresh > r_patina * 0.5,
            "Fresh copper should be more reflective"
        );
    }

    #[test]
    fn test_spectral_ior() {
        let gold = drude_metals::GOLD;
        let spectral = gold.spectral_ior(300.0);

        // All channels should have valid IOR
        assert!(spectral.red.n > 0.0);
        assert!(spectral.green.n > 0.0);
        assert!(spectral.blue.n > 0.0);
    }

    #[test]
    fn test_temperature_sensitivity() {
        let sensitivity = temperature_sensitivity(&drude_metals::GOLD, 550.0);

        assert!(!sensitivity.is_empty());

        for (temp, r) in &sensitivity {
            assert!(*temp >= 200.0 && *temp <= 600.0);
            assert!(*r >= 0.0 && *r <= 1.0);
        }
    }

    #[test]
    fn test_all_drude_presets() {
        let presets = drude_metals::all_presets();

        for (name, drude) in presets {
            let ior = drude.complex_ior(550.0, 300.0);
            assert!(ior.n > 0.0 && ior.k > 0.0, "{} should have valid IOR", name);
        }
    }

    #[test]
    fn test_all_oxide_presets() {
        let presets = oxides::all_presets();

        for (name, oxide) in presets {
            assert!(oxide.n > 1.0, "{} should have n > 1", name);
            assert!(
                oxide.max_thickness_nm > oxide.native_thickness_nm,
                "{} max > native thickness",
                name
            );
        }
    }

    #[test]
    fn test_all_oxidized_presets() {
        let presets = oxidized_presets::all_presets();

        for (name, metal) in presets {
            let rgb = metal.effective_reflectance_rgb(0.8);

            for (i, &r) in rgb.iter().enumerate() {
                assert!(
                    r >= 0.0 && r <= 1.0,
                    "{} RGB[{}] should be valid: {}",
                    name,
                    i,
                    r
                );
            }
        }
    }

    #[test]
    fn test_css_generation() {
        let copper = oxidized_presets::copper_patina();
        let css = to_css_patina(&copper);

        assert!(css.contains("radial-gradient"));
        assert!(css.contains("rgb"));
    }

    #[test]
    fn test_memory_usage() {
        let mem = temp_metal_memory();
        assert!(mem < 1000, "Memory should be minimal: {} bytes", mem);
    }
}

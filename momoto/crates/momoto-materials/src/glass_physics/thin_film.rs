//! # Thin-Film Interference (Phase 3)
//!
//! Models iridescent effects from thin transparent films.
//!
//! ## Physical Background
//!
//! Thin-film interference occurs when light reflects from both surfaces
//! of a thin transparent layer. The path difference creates constructive
//! or destructive interference depending on wavelength and angle.
//!
//! ```text
//!     incident light
//!          │
//!          ▼
//!     ────────────── air (n = 1.0)
//!          │
//!          ▼  reflected rays interfere
//!     ────────────── film (n_film, thickness d)
//!          │
//!          ▼
//!     ────────────── substrate (n_substrate)
//! ```
//!
//! ## Applications
//!
//! - **Soap bubbles**: Variable thickness creates rainbow patterns
//! - **Oil slicks**: Thin oil on water
//! - **Anti-reflective coatings**: Destructive interference at specific wavelength
//! - **Decorative effects**: Holographic, iridescent materials
//!
//! ## References
//!
//! - Born & Wolf: "Principles of Optics", Chapter 7
//! - Hecht: "Optics", 5th Ed., Chapter 9

use std::f64::consts::PI;

// ============================================================================
// THIN FILM PARAMETERS
// ============================================================================

/// Thin-film coating parameters
#[derive(Debug, Clone, Copy)]
pub struct ThinFilm {
    /// Film refractive index
    pub n_film: f64,
    /// Film thickness in nanometers
    pub thickness_nm: f64,
}

impl ThinFilm {
    /// Create new thin film coating
    pub const fn new(n_film: f64, thickness_nm: f64) -> Self {
        Self {
            n_film,
            thickness_nm,
        }
    }

    /// Calculate optical path difference
    ///
    /// OPD = 2 * n_film * d * cos(theta_film)
    ///
    /// Where theta_film is the angle inside the film (from Snell's law)
    #[inline]
    pub fn optical_path_difference(&self, cos_theta_air: f64) -> f64 {
        // Snell's law: sin(theta_air) = n_film * sin(theta_film)
        let sin_air = (1.0 - cos_theta_air * cos_theta_air).sqrt();
        let sin_film = sin_air / self.n_film;
        let cos_film = (1.0 - sin_film * sin_film).sqrt();

        2.0 * self.n_film * self.thickness_nm * cos_film
    }

    /// Calculate phase difference for a given wavelength
    ///
    /// delta = 2 * pi * OPD / lambda
    #[inline]
    pub fn phase_difference(&self, wavelength_nm: f64, cos_theta: f64) -> f64 {
        let opd = self.optical_path_difference(cos_theta);
        2.0 * PI * opd / wavelength_nm
    }

    /// Calculate reflectance at a single wavelength
    ///
    /// Uses the Airy formula for thin-film interference:
    ///
    /// R = (r1² + r2² + 2*r1*r2*cos(delta)) / (1 + r1²*r2² + 2*r1*r2*cos(delta))
    ///
    /// Where r1, r2 are Fresnel reflection coefficients at each interface.
    pub fn reflectance(&self, wavelength_nm: f64, n_substrate: f64, cos_theta: f64) -> f64 {
        let cos_t = cos_theta.abs().clamp(0.0, 1.0);

        // Fresnel coefficients at each interface (using amplitude, not intensity)
        let r1 = fresnel_amplitude(1.0, self.n_film, cos_t);
        let r2 = fresnel_amplitude_transmitted(1.0, self.n_film, n_substrate, cos_t);

        // Phase difference
        let delta = self.phase_difference(wavelength_nm, cos_t);
        let cos_delta = delta.cos();

        // Airy formula for reflectance
        let r1_sq = r1 * r1;
        let r2_sq = r2 * r2;
        let two_r1_r2 = 2.0 * r1 * r2;

        let num = r1_sq + r2_sq + two_r1_r2 * cos_delta;
        let den = 1.0 + r1_sq * r2_sq + two_r1_r2 * cos_delta;

        (num / den).clamp(0.0, 1.0)
    }

    /// Calculate RGB reflectance including interference
    pub fn reflectance_rgb(&self, n_substrate: f64, cos_theta: f64) -> [f64; 3] {
        [
            self.reflectance(650.0, n_substrate, cos_theta), // Red
            self.reflectance(550.0, n_substrate, cos_theta), // Green
            self.reflectance(450.0, n_substrate, cos_theta), // Blue
        ]
    }

    /// Calculate full spectrum reflectance (8 wavelengths)
    ///
    /// Returns (wavelengths, reflectances) for spectral rendering
    pub fn reflectance_spectrum(&self, n_substrate: f64, cos_theta: f64) -> ([f64; 8], [f64; 8]) {
        let wavelengths = [400.0, 450.0, 500.0, 550.0, 600.0, 650.0, 700.0, 750.0];
        let mut reflectances = [0.0; 8];

        for (i, &lambda) in wavelengths.iter().enumerate() {
            reflectances[i] = self.reflectance(lambda, n_substrate, cos_theta);
        }

        (wavelengths, reflectances)
    }

    /// Find wavelength of maximum constructive interference
    ///
    /// For first-order maximum: OPD = lambda
    pub fn max_wavelength(&self, cos_theta: f64) -> f64 {
        self.optical_path_difference(cos_theta)
    }

    /// Find wavelength of maximum destructive interference
    ///
    /// For first-order minimum: OPD = lambda/2
    pub fn min_wavelength(&self, cos_theta: f64) -> f64 {
        2.0 * self.optical_path_difference(cos_theta)
    }
}

impl Default for ThinFilm {
    fn default() -> Self {
        // Default: soap bubble (~100nm soap film)
        Self::new(1.33, 100.0)
    }
}

// ============================================================================
// FRESNEL HELPERS (AMPLITUDE)
// ============================================================================

/// Fresnel reflection amplitude coefficient (not intensity)
///
/// r = (n1 * cos_t1 - n2 * cos_t2) / (n1 * cos_t1 + n2 * cos_t2)
///
/// Uses s-polarization for simplicity (average would be more accurate)
fn fresnel_amplitude(n1: f64, n2: f64, cos_theta1: f64) -> f64 {
    // Snell's law for cos_theta2
    let sin_theta1 = (1.0 - cos_theta1 * cos_theta1).sqrt();
    let sin_theta2 = n1 * sin_theta1 / n2;

    // Total internal reflection check
    if sin_theta2 >= 1.0 {
        return 1.0;
    }

    let cos_theta2 = (1.0 - sin_theta2 * sin_theta2).sqrt();

    // s-polarization
    (n1 * cos_theta1 - n2 * cos_theta2) / (n1 * cos_theta1 + n2 * cos_theta2)
}

/// Fresnel reflection at second interface (film -> substrate)
///
/// Accounts for refraction into the film first
fn fresnel_amplitude_transmitted(
    n_air: f64,
    n_film: f64,
    n_substrate: f64,
    cos_theta_air: f64,
) -> f64 {
    // Angle in film from Snell's law
    let sin_air = (1.0 - cos_theta_air * cos_theta_air).sqrt();
    let sin_film = n_air * sin_air / n_film;

    if sin_film >= 1.0 {
        return 1.0;
    }

    let cos_film = (1.0 - sin_film * sin_film).sqrt();

    // Reflection at film-substrate interface
    fresnel_amplitude(n_film, n_substrate, cos_film)
}

// ============================================================================
// THIN FILM PRESETS
// ============================================================================

/// Pre-defined thin film configurations
pub mod presets {
    use super::ThinFilm;

    /// Soap bubble (thin water film)
    ///
    /// Variable thickness creates rainbow colors
    pub const SOAP_BUBBLE_THIN: ThinFilm = ThinFilm::new(1.33, 100.0);
    pub const SOAP_BUBBLE_MEDIUM: ThinFilm = ThinFilm::new(1.33, 200.0);
    pub const SOAP_BUBBLE_THICK: ThinFilm = ThinFilm::new(1.33, 400.0);

    /// Oil slick on water
    ///
    /// n_oil ≈ 1.5, on water (n ≈ 1.33)
    pub const OIL_THIN: ThinFilm = ThinFilm::new(1.5, 150.0);
    pub const OIL_MEDIUM: ThinFilm = ThinFilm::new(1.5, 300.0);
    pub const OIL_THICK: ThinFilm = ThinFilm::new(1.5, 500.0);

    /// Anti-reflective coating (MgF2 on glass)
    ///
    /// Quarter-wave thickness at 550nm: d = 550/(4*1.38) ≈ 100nm
    pub const AR_COATING: ThinFilm = ThinFilm::new(1.38, 100.0);

    /// Oxide layer (SiO2 on silicon)
    ///
    /// Creates characteristic colors on chips
    pub const OXIDE_THIN: ThinFilm = ThinFilm::new(1.46, 50.0);
    pub const OXIDE_MEDIUM: ThinFilm = ThinFilm::new(1.46, 150.0);
    pub const OXIDE_THICK: ThinFilm = ThinFilm::new(1.46, 300.0);

    /// Beetle shell coating
    ///
    /// Chitin-like material creates iridescence
    pub const BEETLE_SHELL: ThinFilm = ThinFilm::new(1.56, 250.0);

    /// Pearl nacre
    ///
    /// Multiple thin aragonite layers
    pub const NACRE: ThinFilm = ThinFilm::new(1.68, 350.0);

    /// Get all presets with names
    pub fn all_presets() -> Vec<(&'static str, ThinFilm, f64)> {
        // Returns (name, film, suggested_substrate_ior)
        vec![
            ("Soap Bubble (thin)", SOAP_BUBBLE_THIN, 1.0),
            ("Soap Bubble (medium)", SOAP_BUBBLE_MEDIUM, 1.0),
            ("Soap Bubble (thick)", SOAP_BUBBLE_THICK, 1.0),
            ("Oil Slick (thin)", OIL_THIN, 1.33),
            ("Oil Slick (medium)", OIL_MEDIUM, 1.33),
            ("Oil Slick (thick)", OIL_THICK, 1.33),
            ("AR Coating", AR_COATING, 1.52),
            ("Oxide (thin)", OXIDE_THIN, 4.0),
            ("Oxide (medium)", OXIDE_MEDIUM, 4.0),
            ("Oxide (thick)", OXIDE_THICK, 4.0),
            ("Beetle Shell", BEETLE_SHELL, 1.5),
            ("Pearl Nacre", NACRE, 1.5),
        ]
    }
}

// ============================================================================
// CSS GENERATION
// ============================================================================

/// Convert thin-film reflectance to RGB color
///
/// Maps spectral reflectance to a visible color
pub fn thin_film_to_rgb(film: &ThinFilm, n_substrate: f64, cos_theta: f64) -> (u8, u8, u8) {
    let rgb = film.reflectance_rgb(n_substrate, cos_theta);

    // Scale to 0-255
    let r = (rgb[0] * 255.0).clamp(0.0, 255.0) as u8;
    let g = (rgb[1] * 255.0).clamp(0.0, 255.0) as u8;
    let b = (rgb[2] * 255.0).clamp(0.0, 255.0) as u8;

    (r, g, b)
}

/// Generate CSS gradient for iridescent effect
///
/// Creates angle-dependent color shift
pub fn to_css_iridescent_gradient(film: &ThinFilm, n_substrate: f64, base_color: &str) -> String {
    // Sample at different angles
    let angles = [0.0, 15.0, 30.0, 45.0, 60.0, 75.0];
    let mut stops = Vec::new();

    for (i, &angle_deg) in angles.iter().enumerate() {
        let cos_theta = (angle_deg * PI / 180.0).cos();
        let (r, g, b) = thin_film_to_rgb(film, n_substrate, cos_theta);
        let position = (i as f64 / (angles.len() - 1) as f64) * 100.0;

        stops.push(format!("rgba({}, {}, {}, 0.5) {:.0}%", r, g, b, position));
    }

    format!(
        "linear-gradient(135deg, {} 0%, {} 100%), {}",
        stops.join(", "),
        stops.last().unwrap_or(&"transparent".to_string()),
        base_color
    )
}

/// Generate CSS for soap bubble effect
pub fn to_css_soap_bubble(film: &ThinFilm, _size_percent: f64) -> String {
    let rgb_center = film.reflectance_rgb(1.0, 1.0); // Normal incidence
    let rgb_edge = film.reflectance_rgb(1.0, 0.3); // Grazing

    let r_c = (rgb_center[0] * 255.0) as u8;
    let g_c = (rgb_center[1] * 255.0) as u8;
    let b_c = (rgb_center[2] * 255.0) as u8;

    let r_e = (rgb_edge[0] * 255.0) as u8;
    let g_e = (rgb_edge[1] * 255.0) as u8;
    let b_e = (rgb_edge[2] * 255.0) as u8;

    format!(
        "radial-gradient(circle at 30% 30%, \
         rgba(255, 255, 255, 0.8) 0%, \
         rgba({}, {}, {}, 0.4) 20%, \
         rgba({}, {}, {}, 0.3) 50%, \
         rgba({}, {}, {}, 0.5) 80%, \
         rgba({}, {}, {}, 0.7) 100%)",
        r_c, g_c, b_c, r_c, g_c, b_c, r_e, g_e, b_e, r_e, g_e, b_e,
    )
}

/// Generate CSS for oil slick effect
pub fn to_css_oil_slick(film: &ThinFilm) -> String {
    // Oil on water
    let n_water = 1.33;

    // Multiple angle samples for rainbow effect
    let (r1, g1, b1) = thin_film_to_rgb(film, n_water, 0.95);
    let (r2, g2, b2) = thin_film_to_rgb(film, n_water, 0.7);
    let (r3, g3, b3) = thin_film_to_rgb(film, n_water, 0.5);
    let (r4, g4, b4) = thin_film_to_rgb(film, n_water, 0.3);

    format!(
        "linear-gradient(45deg, \
         rgba({}, {}, {}, 0.6) 0%, \
         rgba({}, {}, {}, 0.5) 25%, \
         rgba({}, {}, {}, 0.5) 50%, \
         rgba({}, {}, {}, 0.6) 75%, \
         rgba({}, {}, {}, 0.6) 100%)",
        r1, g1, b1, r2, g2, b2, r3, g3, b3, r4, g4, b4, r1, g1, b1,
    )
}

// ============================================================================
// MULTI-LAYER THIN FILMS
// ============================================================================

/// Multi-layer thin film stack (for advanced effects)
#[derive(Debug, Clone)]
pub struct ThinFilmStack {
    /// Stack of films from air side to substrate
    pub layers: Vec<ThinFilm>,
    /// Substrate refractive index
    pub n_substrate: f64,
}

impl ThinFilmStack {
    /// Create new thin film stack
    pub fn new(layers: Vec<ThinFilm>, n_substrate: f64) -> Self {
        Self {
            layers,
            n_substrate,
        }
    }

    /// Calculate reflectance using transfer matrix method (simplified)
    ///
    /// For accurate multi-layer calculation, we approximate by summing
    /// contributions from each interface.
    pub fn reflectance(&self, wavelength_nm: f64, cos_theta: f64) -> f64 {
        if self.layers.is_empty() {
            // No film: just air-substrate interface
            let r = fresnel_amplitude(1.0, self.n_substrate, cos_theta);
            return r * r;
        }

        // Single layer: use exact formula
        if self.layers.len() == 1 {
            return self.layers[0].reflectance(wavelength_nm, self.n_substrate, cos_theta);
        }

        // Multiple layers: simplified approximation
        // Sum contributions with phase offsets
        let mut total_r: f64 = 0.0;
        let mut n_prev: f64 = 1.0;
        let mut accumulated_phase: f64 = 0.0;

        for (i, layer) in self.layers.iter().enumerate() {
            let n_next = if i + 1 < self.layers.len() {
                self.layers[i + 1].n_film
            } else {
                self.n_substrate
            };

            // Reflection at this interface
            let r = fresnel_amplitude(n_prev, layer.n_film, cos_theta);
            let phase = layer.phase_difference(wavelength_nm, cos_theta) + accumulated_phase;

            // Add with interference
            total_r += r * r + 2.0 * r * total_r.sqrt() * phase.cos();
            accumulated_phase += phase;
            n_prev = layer.n_film;
            let _ = n_next; // Suppress unused warning
        }

        total_r.clamp(0.0, 1.0)
    }

    /// Calculate RGB reflectance for stack
    pub fn reflectance_rgb(&self, cos_theta: f64) -> [f64; 3] {
        [
            self.reflectance(650.0, cos_theta),
            self.reflectance(550.0, cos_theta),
            self.reflectance(450.0, cos_theta),
        ]
    }
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Calculate optimal AR coating thickness for a given wavelength
///
/// For quarter-wave AR coating: d = lambda / (4 * n_film)
pub fn ar_coating_thickness(wavelength_nm: f64, n_film: f64) -> f64 {
    wavelength_nm / (4.0 * n_film)
}

/// Calculate dominant color from thin-film interference
///
/// Returns the wavelength with maximum reflectance
pub fn dominant_wavelength(film: &ThinFilm, n_substrate: f64, cos_theta: f64) -> f64 {
    let mut max_r = 0.0;
    let mut max_lambda = 550.0;

    // Scan visible range
    for lambda in (400..=700).step_by(10) {
        let r = film.reflectance(lambda as f64, n_substrate, cos_theta);
        if r > max_r {
            max_r = r;
            max_lambda = lambda as f64;
        }
    }

    max_lambda
}

/// Total memory used by thin-film module (negligible)
pub fn total_thin_film_memory() -> usize {
    // No LUTs, just struct sizes
    std::mem::size_of::<ThinFilm>() + std::mem::size_of::<ThinFilmStack>()
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optical_path_difference() {
        let film = ThinFilm::new(1.5, 100.0);

        // Normal incidence: OPD = 2 * n * d = 2 * 1.5 * 100 = 300nm
        let opd_normal = film.optical_path_difference(1.0);
        assert!(
            (opd_normal - 300.0).abs() < 1.0,
            "OPD at normal should be ~300nm"
        );

        // OPD decreases at grazing angles
        let opd_grazing = film.optical_path_difference(0.5);
        assert!(opd_grazing < opd_normal, "OPD should decrease at angle");
    }

    #[test]
    fn test_reflectance_bounds() {
        let film = ThinFilm::new(1.4, 200.0);

        for lambda in [450.0, 550.0, 650.0] {
            for cos_t in [0.3, 0.5, 0.7, 1.0] {
                let r = film.reflectance(lambda, 1.5, cos_t);
                assert!(r >= 0.0, "Reflectance should be >= 0");
                assert!(r <= 1.0, "Reflectance should be <= 1");
            }
        }
    }

    #[test]
    fn test_wavelength_dependence() {
        // Thin film should show wavelength-dependent reflectance
        let film = ThinFilm::new(1.4, 200.0);
        let rgb = film.reflectance_rgb(1.5, 0.8);

        // Colors should differ (interference pattern)
        let variation = (rgb[0] - rgb[1]).abs() + (rgb[1] - rgb[2]).abs();
        assert!(variation > 0.01, "Should show wavelength dependence");
    }

    #[test]
    fn test_ar_coating() {
        // Quarter-wave AR coating should minimize reflection at design wavelength
        let n_film = 1.38;
        let design_lambda = 550.0;
        let thickness = ar_coating_thickness(design_lambda, n_film);

        let ar = ThinFilm::new(n_film, thickness);

        // Check reflectance is lower than bare glass
        let r_coated = ar.reflectance(design_lambda, 1.52, 1.0);
        let r_bare = fresnel_amplitude(1.0, 1.52, 1.0).powi(2);

        assert!(
            r_coated < r_bare,
            "AR coating should reduce reflection: {} < {}",
            r_coated,
            r_bare
        );
    }

    #[test]
    fn test_soap_bubble_presets() {
        // Different thicknesses should give different colors
        let thin = presets::SOAP_BUBBLE_THIN;
        let thick = presets::SOAP_BUBBLE_THICK;

        let rgb_thin = thin.reflectance_rgb(1.0, 0.8);
        let rgb_thick = thick.reflectance_rgb(1.0, 0.8);

        // Should be different colors
        let diff = (rgb_thin[0] - rgb_thick[0]).abs()
            + (rgb_thin[1] - rgb_thick[1]).abs()
            + (rgb_thin[2] - rgb_thick[2]).abs();

        assert!(
            diff > 0.1,
            "Different thicknesses should give different colors"
        );
    }

    #[test]
    fn test_angle_dependence() {
        let film = presets::OIL_MEDIUM;

        let rgb_normal = film.reflectance_rgb(1.33, 1.0);
        let rgb_angled = film.reflectance_rgb(1.33, 0.5);

        // Colors should shift with angle (iridescence)
        let diff = (rgb_normal[0] - rgb_angled[0]).abs()
            + (rgb_normal[1] - rgb_angled[1]).abs()
            + (rgb_normal[2] - rgb_angled[2]).abs();

        assert!(diff > 0.01, "Color should shift with viewing angle");
    }

    #[test]
    fn test_thin_film_stack() {
        // Create a multi-layer coating
        let stack = ThinFilmStack::new(
            vec![ThinFilm::new(1.38, 100.0), ThinFilm::new(1.70, 50.0)],
            1.52,
        );

        let rgb = stack.reflectance_rgb(0.8);

        for &r in &rgb {
            assert!(r >= 0.0 && r <= 1.0, "Stack reflectance should be valid");
        }
    }

    #[test]
    fn test_all_presets() {
        let presets = presets::all_presets();
        assert!(!presets.is_empty());

        for (name, film, n_sub) in presets {
            let rgb = film.reflectance_rgb(n_sub, 0.8);

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
        let film = presets::SOAP_BUBBLE_MEDIUM;
        let css = to_css_soap_bubble(&film, 100.0);

        assert!(css.contains("radial-gradient"));
        assert!(css.contains("rgba"));
    }

    #[test]
    fn test_dominant_wavelength() {
        let film = ThinFilm::new(1.4, 200.0);
        let lambda = dominant_wavelength(&film, 1.5, 0.9);

        assert!(
            lambda >= 400.0 && lambda <= 700.0,
            "Should be in visible range"
        );
    }
}

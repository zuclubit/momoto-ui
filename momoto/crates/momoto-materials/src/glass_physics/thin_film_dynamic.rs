//! # Dynamic Thin-Film with Physical Deformations
//!
//! Phase 5 implementation of thin-film stacks that respond to physical conditions:
//! temperature, stress, curvature, and time evolution.
//!
//! ## Key Features
//!
//! - **Thermo-Optic Effects**: Temperature-dependent refractive index
//! - **Thermal Expansion**: Temperature-dependent thickness
//! - **Stress-Induced Deformation**: Mechanical stress effects
//! - **Curvature Mapping**: Spatially-varying thickness on curved surfaces
//! - **Iridescence Mapping**: Full color maps for deformed films
//!
//! ## Physical Models
//!
//! - `dn/dT`: Thermo-optic coefficient (typically 10⁻⁵ to 10⁻⁴ /K)
//! - `α`: Thermal expansion (typically 10⁻⁵ to 10⁻⁴ /K)
//! - `E`: Young's modulus for stress effects
//! - `κ`: Local curvature for thickness variation

use std::f64::consts::PI;

// ============================================================================
// DYNAMIC FILM LAYER
// ============================================================================

/// Properties of a dynamic thin-film layer
#[derive(Debug, Clone)]
pub struct DynamicFilmLayer {
    // Base properties
    /// Base refractive index at reference temperature
    pub n_base: f64,
    /// Base extinction coefficient
    pub k_base: f64,
    /// Base thickness in nanometers
    pub thickness_nm: f64,

    // Thermo-optic properties
    /// Thermo-optic coefficient dn/dT (1/K)
    pub dn_dt: f64,
    /// Reference temperature (K)
    pub t_ref: f64,

    // Mechanical properties
    /// Thermal expansion coefficient (1/K)
    pub alpha_thermal: f64,
    /// Young's modulus (GPa)
    pub youngs_modulus: f64,
    /// Poisson's ratio
    pub poisson_ratio: f64,

    // Current state
    /// Current temperature (K)
    pub temperature: f64,
    /// Applied stress (MPa) - [σxx, σyy, σzz, σxy, σyz, σzx] Voigt notation
    pub stress: [f64; 6],
}

impl DynamicFilmLayer {
    /// Create new dynamic layer
    pub fn new(n: f64, thickness_nm: f64) -> Self {
        Self {
            n_base: n,
            k_base: 0.0,
            thickness_nm,
            dn_dt: 1e-5,          // Typical value for glass
            t_ref: 293.0,         // 20°C
            alpha_thermal: 5e-6,  // Typical for SiO2
            youngs_modulus: 70.0, // GPa, typical for glass
            poisson_ratio: 0.17,
            temperature: 293.0,
            stress: [0.0; 6],
        }
    }

    /// Set thermo-optic coefficient
    pub fn with_dn_dt(mut self, dn_dt: f64) -> Self {
        self.dn_dt = dn_dt;
        self
    }

    /// Set thermal expansion
    pub fn with_thermal_expansion(mut self, alpha: f64) -> Self {
        self.alpha_thermal = alpha;
        self
    }

    /// Set mechanical properties
    pub fn with_mechanical(mut self, youngs: f64, poisson: f64) -> Self {
        self.youngs_modulus = youngs;
        self.poisson_ratio = poisson;
        self
    }

    /// Set extinction coefficient
    pub fn with_k(mut self, k: f64) -> Self {
        self.k_base = k;
        self
    }

    /// Update temperature
    pub fn set_temperature(&mut self, temp_k: f64) {
        self.temperature = temp_k;
    }

    /// Update stress state
    pub fn set_stress(&mut self, stress: [f64; 6]) {
        self.stress = stress;
    }

    /// Calculate thermal strain
    fn thermal_strain(&self) -> f64 {
        self.alpha_thermal * (self.temperature - self.t_ref)
    }

    /// Calculate stress-induced strain (simplified uniaxial)
    fn stress_strain(&self) -> f64 {
        let sigma_mean = (self.stress[0] + self.stress[1] + self.stress[2]) / 3.0;
        // Convert MPa to strain: ε = σ/E (with E in GPa = 1000 MPa)
        sigma_mean / (self.youngs_modulus * 1000.0)
    }

    /// Get effective refractive index at current temperature
    pub fn effective_n(&self) -> f64 {
        self.n_base + self.dn_dt * (self.temperature - self.t_ref)
    }

    /// Get effective thickness at current conditions
    pub fn effective_thickness(&self) -> f64 {
        self.thickness_nm * (1.0 + self.thermal_strain() + self.stress_strain())
    }

    /// Get all effective properties
    pub fn effective_properties(&self) -> (f64, f64, f64) {
        (self.effective_n(), self.k_base, self.effective_thickness())
    }
}

// ============================================================================
// CURVATURE / HEIGHT MAP
// ============================================================================

/// 2D position
#[derive(Debug, Clone, Copy)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Height map for curved surfaces
#[derive(Debug, Clone)]
pub struct HeightMap {
    /// Height values in a grid
    pub heights: Vec<Vec<f64>>,
    /// Grid resolution
    pub resolution: (usize, usize),
    /// Physical size (mm)
    pub size: (f64, f64),
}

impl HeightMap {
    /// Create flat height map
    pub fn flat(resolution: (usize, usize), size: (f64, f64)) -> Self {
        Self {
            heights: vec![vec![0.0; resolution.0]; resolution.1],
            resolution,
            size,
        }
    }

    /// Create spherical dome
    pub fn spherical_dome(resolution: (usize, usize), size: (f64, f64), radius: f64) -> Self {
        let mut heights = vec![vec![0.0; resolution.0]; resolution.1];

        for y in 0..resolution.1 {
            for x in 0..resolution.0 {
                let px = (x as f64 / resolution.0 as f64 - 0.5) * size.0;
                let py = (y as f64 / resolution.1 as f64 - 0.5) * size.1;
                let r2 = px * px + py * py;

                if r2 < radius * radius {
                    heights[y][x] = (radius * radius - r2).sqrt() - radius;
                }
            }
        }

        Self {
            heights,
            resolution,
            size,
        }
    }

    /// Create sinusoidal ripple
    pub fn sinusoidal(
        resolution: (usize, usize),
        size: (f64, f64),
        amplitude: f64,
        period: f64,
    ) -> Self {
        let mut heights = vec![vec![0.0; resolution.0]; resolution.1];

        for y in 0..resolution.1 {
            for x in 0..resolution.0 {
                let px = x as f64 / resolution.0 as f64 * size.0;
                heights[y][x] = amplitude * (2.0 * PI * px / period).sin();
            }
        }

        Self {
            heights,
            resolution,
            size,
        }
    }

    /// Sample height at position (0-1 normalized coordinates)
    pub fn sample(&self, pos: Vec2) -> f64 {
        let x = (pos.x * self.resolution.0 as f64).clamp(0.0, (self.resolution.0 - 1) as f64);
        let y = (pos.y * self.resolution.1 as f64).clamp(0.0, (self.resolution.1 - 1) as f64);

        let x0 = x.floor() as usize;
        let y0 = y.floor() as usize;
        let x1 = (x0 + 1).min(self.resolution.0 - 1);
        let y1 = (y0 + 1).min(self.resolution.1 - 1);

        let tx = x - x0 as f64;
        let ty = y - y0 as f64;

        // Bilinear interpolation
        let h00 = self.heights[y0][x0];
        let h10 = self.heights[y0][x1];
        let h01 = self.heights[y1][x0];
        let h11 = self.heights[y1][x1];

        let h0 = h00 + (h10 - h00) * tx;
        let h1 = h01 + (h11 - h01) * tx;
        h0 + (h1 - h0) * ty
    }

    /// Calculate local normal at position
    pub fn normal(&self, pos: Vec2) -> [f64; 3] {
        let eps = 0.01;
        let h_x0 = self.sample(Vec2::new((pos.x - eps).max(0.0), pos.y));
        let h_x1 = self.sample(Vec2::new((pos.x + eps).min(1.0), pos.y));
        let h_y0 = self.sample(Vec2::new(pos.x, (pos.y - eps).max(0.0)));
        let h_y1 = self.sample(Vec2::new(pos.x, (pos.y + eps).min(1.0)));

        let dx = (h_x1 - h_x0) / (2.0 * eps * self.size.0);
        let dy = (h_y1 - h_y0) / (2.0 * eps * self.size.1);

        let len = (1.0 + dx * dx + dy * dy).sqrt();
        [-dx / len, -dy / len, 1.0 / len]
    }

    /// Calculate local curvature at position
    pub fn curvature(&self, pos: Vec2) -> f64 {
        let eps = 0.01;
        let h_c = self.sample(pos);
        let h_x0 = self.sample(Vec2::new((pos.x - eps).max(0.0), pos.y));
        let h_x1 = self.sample(Vec2::new((pos.x + eps).min(1.0), pos.y));
        let h_y0 = self.sample(Vec2::new(pos.x, (pos.y - eps).max(0.0)));
        let h_y1 = self.sample(Vec2::new(pos.x, (pos.y + eps).min(1.0)));

        // Second derivative approximation
        let d2x = (h_x1 - 2.0 * h_c + h_x0) / (eps * eps * self.size.0 * self.size.0);
        let d2y = (h_y1 - 2.0 * h_c + h_y0) / (eps * eps * self.size.1 * self.size.1);

        (d2x + d2y) / 2.0
    }
}

// ============================================================================
// DYNAMIC THIN-FILM STACK
// ============================================================================

/// Substrate properties
#[derive(Debug, Clone)]
pub struct SubstrateProperties {
    /// Refractive index
    pub n: f64,
    /// Extinction coefficient
    pub k: f64,
    /// Thermal expansion coefficient
    pub alpha: f64,
}

impl Default for SubstrateProperties {
    fn default() -> Self {
        Self {
            n: 1.52, // BK7 glass
            k: 0.0,
            alpha: 7e-6, // BK7 thermal expansion
        }
    }
}

/// Dynamic thin-film stack with environmental response
#[derive(Debug, Clone)]
pub struct DynamicThinFilmStack {
    /// Film layers (from ambient to substrate)
    pub layers: Vec<DynamicFilmLayer>,
    /// Substrate properties
    pub substrate: SubstrateProperties,
    /// Ambient refractive index
    pub n_ambient: f64,

    // Environmental conditions
    /// Ambient temperature (K)
    pub ambient_temp: f64,
    /// Ambient pressure (Pa)
    pub ambient_pressure: f64,
    /// Relative humidity (0-1)
    pub humidity: f64,

    // Geometry
    /// Height map for curved surfaces
    pub height_map: Option<HeightMap>,
}

impl DynamicThinFilmStack {
    /// Create new stack
    pub fn new(n_ambient: f64, substrate: SubstrateProperties) -> Self {
        Self {
            layers: Vec::new(),
            substrate,
            n_ambient,
            ambient_temp: 293.0,
            ambient_pressure: 101325.0,
            humidity: 0.5,
            height_map: None,
        }
    }

    /// Add a dynamic layer
    pub fn add_layer(&mut self, layer: DynamicFilmLayer) {
        self.layers.push(layer);
    }

    /// Set height map for curved surface
    pub fn with_height_map(mut self, height_map: HeightMap) -> Self {
        self.height_map = Some(height_map);
        self
    }

    /// Set environmental conditions
    pub fn set_environment(&mut self, temp: f64, pressure: f64, humidity: f64) {
        self.ambient_temp = temp;
        self.ambient_pressure = pressure;
        self.humidity = humidity;

        // Update all layers
        for layer in &mut self.layers {
            layer.set_temperature(temp);
        }
    }

    /// Apply uniform stress to all layers
    pub fn apply_stress(&mut self, stress: [f64; 6]) {
        for layer in &mut self.layers {
            layer.set_stress(stress);
        }
    }

    /// Calculate local incidence angle at position
    fn local_incidence_angle(&self, pos: Vec2, global_angle: f64) -> f64 {
        if let Some(ref hm) = self.height_map {
            let normal = hm.normal(pos);
            // Simple approximation: adjust angle based on normal tilt
            let tilt = normal[2].acos();
            (global_angle + tilt).abs()
        } else {
            global_angle
        }
    }

    /// Calculate thickness factor at position (due to curvature)
    fn curvature_thickness_factor(&self, pos: Vec2) -> f64 {
        if let Some(ref hm) = self.height_map {
            let normal = hm.normal(pos);
            // Thickness increases on inclined surfaces
            1.0 / normal[2].max(0.1)
        } else {
            1.0
        }
    }

    /// Get effective layer properties at position
    fn effective_layers_at(&self, pos: Vec2) -> Vec<(f64, f64, f64)> {
        let curvature_factor = self.curvature_thickness_factor(pos);

        self.layers
            .iter()
            .map(|layer| {
                let (n, k, d) = layer.effective_properties();
                (n, k, d * curvature_factor)
            })
            .collect()
    }

    /// Calculate reflectance at a position using transfer matrix method
    pub fn reflectance_at(&self, pos: Vec2, wavelength: f64, angle_deg: f64) -> f64 {
        let angle_rad = angle_deg.to_radians();
        let local_angle = self.local_incidence_angle(pos, angle_rad);
        let cos_theta = local_angle.cos();

        let layers = self.effective_layers_at(pos);

        if layers.is_empty() {
            // Bare substrate
            let r = (self.n_ambient - self.substrate.n) / (self.n_ambient + self.substrate.n);
            return r * r;
        }

        // Simplified thin-film reflectance (single layer approximation)
        if layers.len() == 1 {
            let (n_film, _, d) = layers[0];
            return self.single_layer_reflectance(wavelength, n_film, d, cos_theta);
        }

        // Multi-layer: use matrix method
        self.transfer_matrix_reflectance(&layers, wavelength, cos_theta)
    }

    /// Single layer reflectance (Airy formula)
    fn single_layer_reflectance(
        &self,
        wavelength: f64,
        n_film: f64,
        d: f64,
        cos_theta: f64,
    ) -> f64 {
        let r01 = (self.n_ambient - n_film) / (self.n_ambient + n_film);
        let r12 = (n_film - self.substrate.n) / (n_film + self.substrate.n);

        let cos_theta_film = (1.0 - (self.n_ambient / n_film).powi(2) * (1.0 - cos_theta.powi(2)))
            .max(0.0)
            .sqrt();
        let delta = 4.0 * PI * n_film * d * cos_theta_film / wavelength;

        let cos_delta = delta.cos();
        let r01_sq = r01 * r01;
        let r12_sq = r12 * r12;

        (r01_sq + r12_sq + 2.0 * r01 * r12 * cos_delta)
            / (1.0 + r01_sq * r12_sq + 2.0 * r01 * r12 * cos_delta)
    }

    /// Multi-layer transfer matrix reflectance
    fn transfer_matrix_reflectance(
        &self,
        layers: &[(f64, f64, f64)],
        wavelength: f64,
        cos_theta: f64,
    ) -> f64 {
        // Build interface matrices
        let mut m = [[1.0, 0.0], [0.0, 1.0]]; // Identity

        let mut n_prev = self.n_ambient;
        let mut cos_prev = cos_theta;

        for &(n, _k, d) in layers {
            // Snell's law for angle in this layer
            let sin_theta = (self.n_ambient / n) * (1.0 - cos_theta.powi(2)).sqrt();
            let cos_curr = (1.0 - sin_theta.powi(2)).max(0.0).sqrt();

            // Interface matrix (s-polarization)
            let r = (n_prev * cos_prev - n * cos_curr) / (n_prev * cos_prev + n * cos_curr);
            let t = 2.0 * n_prev * cos_prev / (n_prev * cos_prev + n * cos_curr);

            let mi = [[1.0 / t, r / t], [r / t, 1.0 / t]];

            // Propagation matrix
            let delta = 2.0 * PI * n * d * cos_curr / wavelength;
            let mp = [[delta.cos(), delta.sin()], [-delta.sin(), delta.cos()]];

            // Multiply matrices
            m = self.mat_mul(&self.mat_mul(&m, &mi), &mp);

            n_prev = n;
            cos_prev = cos_curr;
        }

        // Final interface to substrate
        let sin_sub = (self.n_ambient / self.substrate.n) * (1.0 - cos_theta.powi(2)).sqrt();
        let cos_sub = (1.0 - sin_sub.powi(2)).max(0.0).sqrt();
        let r_sub = (n_prev * cos_prev - self.substrate.n * cos_sub)
            / (n_prev * cos_prev + self.substrate.n * cos_sub);
        let t_sub = 2.0 * n_prev * cos_prev / (n_prev * cos_prev + self.substrate.n * cos_sub);

        let m_final = [[1.0 / t_sub, r_sub / t_sub], [r_sub / t_sub, 1.0 / t_sub]];
        let m_total = self.mat_mul(&m, &m_final);

        let r = m_total[1][0] / m_total[0][0];
        r * r
    }

    fn mat_mul(&self, a: &[[f64; 2]; 2], b: &[[f64; 2]; 2]) -> [[f64; 2]; 2] {
        [
            [
                a[0][0] * b[0][0] + a[0][1] * b[1][0],
                a[0][0] * b[0][1] + a[0][1] * b[1][1],
            ],
            [
                a[1][0] * b[0][0] + a[1][1] * b[1][0],
                a[1][0] * b[0][1] + a[1][1] * b[1][1],
            ],
        ]
    }

    /// Calculate RGB reflectance at position
    pub fn reflectance_rgb_at(&self, pos: Vec2, angle_deg: f64) -> [f64; 3] {
        [
            self.reflectance_at(pos, 650.0, angle_deg),
            self.reflectance_at(pos, 550.0, angle_deg),
            self.reflectance_at(pos, 450.0, angle_deg),
        ]
    }
}

// ============================================================================
// IRIDESCENCE MAP
// ============================================================================

/// Map of structural colors across a surface
#[derive(Debug, Clone)]
pub struct IridescenceMap {
    /// RGB values at each point
    pub colors: Vec<Vec<[f64; 3]>>,
    /// Resolution
    pub resolution: (usize, usize),
}

impl IridescenceMap {
    /// Create empty map
    pub fn new(resolution: (usize, usize)) -> Self {
        Self {
            colors: vec![vec![[0.0; 3]; resolution.0]; resolution.1],
            resolution,
        }
    }

    /// Set color at position
    pub fn set(&mut self, x: usize, y: usize, rgb: [f64; 3]) {
        if x < self.resolution.0 && y < self.resolution.1 {
            self.colors[y][x] = rgb;
        }
    }

    /// Get color at position
    pub fn get(&self, x: usize, y: usize) -> [f64; 3] {
        if x < self.resolution.0 && y < self.resolution.1 {
            self.colors[y][x]
        } else {
            [0.0; 3]
        }
    }

    /// Convert to image data (u8 RGB)
    pub fn to_image_data(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(self.resolution.0 * self.resolution.1 * 3);
        for row in &self.colors {
            for &rgb in row {
                data.push((rgb[0] * 255.0).clamp(0.0, 255.0) as u8);
                data.push((rgb[1] * 255.0).clamp(0.0, 255.0) as u8);
                data.push((rgb[2] * 255.0).clamp(0.0, 255.0) as u8);
            }
        }
        data
    }
}

/// Compute full iridescence map for a dynamic stack
pub fn compute_iridescence_map(
    stack: &DynamicThinFilmStack,
    resolution: (usize, usize),
    view_angle: f64,
) -> IridescenceMap {
    let mut map = IridescenceMap::new(resolution);

    for y in 0..resolution.1 {
        for x in 0..resolution.0 {
            let pos = Vec2::new(
                x as f64 / resolution.0 as f64,
                y as f64 / resolution.1 as f64,
            );

            let rgb = stack.reflectance_rgb_at(pos, view_angle);
            map.set(x, y, rgb);
        }
    }

    map
}

// ============================================================================
// PRESETS
// ============================================================================

/// Dynamic thin-film presets
pub mod dynamic_presets {
    use super::*;

    /// Soap bubble with temperature response
    pub fn soap_bubble(temp_k: f64) -> DynamicThinFilmStack {
        let mut stack = DynamicThinFilmStack::new(
            1.0,
            SubstrateProperties {
                n: 1.0, // Air on other side
                k: 0.0,
                alpha: 0.0,
            },
        );

        let layer = DynamicFilmLayer::new(1.33, 300.0)
            .with_dn_dt(1e-4) // Water has high dn/dT
            .with_thermal_expansion(2e-4); // Water film expands significantly

        stack.add_layer(layer);
        stack.set_environment(temp_k, 101325.0, 0.8);
        stack
    }

    /// Morpho butterfly wing with curvature
    pub fn morpho_curved(height_map: HeightMap) -> DynamicThinFilmStack {
        let mut stack = DynamicThinFilmStack::new(
            1.0,
            SubstrateProperties {
                n: 1.56,
                k: 0.0,
                alpha: 1e-5,
            },
        )
        .with_height_map(height_map);

        // Chitin/air alternating layers
        for i in 0..5 {
            let (n, d) = if i % 2 == 0 {
                (1.56, 85.0) // Chitin
            } else {
                (1.0, 95.0) // Air
            };
            stack.add_layer(DynamicFilmLayer::new(n, d));
        }

        stack
    }

    /// AR coating with stress response
    pub fn ar_coating_stressed(stress_mpa: f64) -> DynamicThinFilmStack {
        let mut stack = DynamicThinFilmStack::new(1.0, SubstrateProperties::default());

        let mut layer = DynamicFilmLayer::new(1.38, 100.0) // MgF2
            .with_dn_dt(1e-6)
            .with_thermal_expansion(1e-5)
            .with_mechanical(50.0, 0.25);

        layer.set_stress([stress_mpa, stress_mpa, 0.0, 0.0, 0.0, 0.0]);
        stack.add_layer(layer);
        stack
    }

    /// Oil slick on water with ripples
    pub fn oil_slick_rippled() -> DynamicThinFilmStack {
        let height_map = HeightMap::sinusoidal((64, 64), (10.0, 10.0), 0.5, 2.0);

        let mut stack = DynamicThinFilmStack::new(
            1.0,
            SubstrateProperties {
                n: 1.33, // Water
                k: 0.0,
                alpha: 2e-4,
            },
        )
        .with_height_map(height_map);

        stack.add_layer(DynamicFilmLayer::new(1.5, 500.0)); // Oil layer
        stack
    }

    /// Heated glass with thermal gradient
    pub fn heated_glass(center_temp: f64, edge_temp: f64) -> DynamicThinFilmStack {
        // Note: This creates a uniform-temperature stack
        // For actual gradients, would need spatial temperature field
        let avg_temp = (center_temp + edge_temp) / 2.0;

        let mut stack = DynamicThinFilmStack::new(1.0, SubstrateProperties::default());

        let layer = DynamicFilmLayer::new(1.38, 100.0)
            .with_dn_dt(1e-5)
            .with_thermal_expansion(5e-6);

        stack.add_layer(layer);
        stack.set_environment(avg_temp, 101325.0, 0.5);
        stack
    }
}

// ============================================================================
// CSS GENERATION
// ============================================================================

/// Generate CSS gradient from iridescence map
pub fn to_css_iridescence(map: &IridescenceMap, angle_deg: f64) -> String {
    let mut colors = Vec::new();

    // Sample along a diagonal
    let steps = 10;
    for i in 0..steps {
        let t = i as f64 / (steps - 1) as f64;
        let x = (t * map.resolution.0 as f64) as usize;
        let y = (t * map.resolution.1 as f64) as usize;

        let rgb = map.get(x.min(map.resolution.0 - 1), y.min(map.resolution.1 - 1));
        let r = (rgb[0] * 255.0).clamp(0.0, 255.0) as u8;
        let g = (rgb[1] * 255.0).clamp(0.0, 255.0) as u8;
        let b = (rgb[2] * 255.0).clamp(0.0, 255.0) as u8;

        colors.push(format!("rgb({}, {}, {}) {}%", r, g, b, (t * 100.0) as u8));
    }

    format!("linear-gradient({}deg, {})", angle_deg, colors.join(", "))
}

/// Generate animated CSS for temperature change
pub fn to_css_temperature_animation(
    stack: &DynamicThinFilmStack,
    temp_range: (f64, f64),
) -> String {
    let mut keyframes = String::from("@keyframes temperature-shift {\n");

    let steps = 5;
    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let temp = temp_range.0 + t * (temp_range.1 - temp_range.0);

        let mut temp_stack = stack.clone();
        temp_stack.set_environment(temp, 101325.0, 0.5);

        let rgb = temp_stack.reflectance_rgb_at(Vec2::new(0.5, 0.5), 0.0);
        let r = (rgb[0] * 255.0).clamp(0.0, 255.0) as u8;
        let g = (rgb[1] * 255.0).clamp(0.0, 255.0) as u8;
        let b = (rgb[2] * 255.0).clamp(0.0, 255.0) as u8;

        keyframes.push_str(&format!(
            "  {}% {{ background-color: rgb({}, {}, {}); }}\n",
            (t * 100.0) as u8,
            r,
            g,
            b
        ));
    }

    keyframes.push_str("}\n");
    keyframes
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_layer_temperature() {
        let mut layer = DynamicFilmLayer::new(1.5, 100.0).with_dn_dt(1e-4);

        layer.set_temperature(293.0);
        let n_cold = layer.effective_n();

        layer.set_temperature(373.0); // +80K
        let n_hot = layer.effective_n();

        // n should increase with temperature (positive dn/dT)
        assert!(n_hot > n_cold);
        assert!((n_hot - n_cold - 0.008).abs() < 0.001); // 80K × 1e-4 = 0.008
    }

    #[test]
    fn test_dynamic_layer_thermal_expansion() {
        let mut layer = DynamicFilmLayer::new(1.5, 100.0).with_thermal_expansion(1e-4);

        layer.set_temperature(293.0);
        let d_cold = layer.effective_thickness();

        layer.set_temperature(393.0); // +100K
        let d_hot = layer.effective_thickness();

        // Thickness should increase
        assert!(d_hot > d_cold);
    }

    #[test]
    fn test_dynamic_layer_stress() {
        let mut layer = DynamicFilmLayer::new(1.5, 100.0).with_mechanical(70.0, 0.17);

        let d_unstressed = layer.effective_thickness();

        layer.set_stress([100.0, 100.0, 100.0, 0.0, 0.0, 0.0]); // 100 MPa hydrostatic
        let d_stressed = layer.effective_thickness();

        // Thickness should change under stress
        assert!((d_stressed - d_unstressed).abs() > 0.0);
    }

    #[test]
    fn test_height_map_spherical() {
        let hm = HeightMap::spherical_dome((32, 32), (10.0, 10.0), 20.0);

        // Center should be highest (dome bulges up at center)
        // Formula: h = sqrt(r² - d²) - r, so h=0 at center, h=-r at edge
        let h_center = hm.sample(Vec2::new(0.5, 0.5));
        let h_edge = hm.sample(Vec2::new(0.0, 0.5));

        assert!(
            h_center >= h_edge,
            "Center {} should be >= edge {}",
            h_center,
            h_edge
        );
    }

    #[test]
    fn test_height_map_normal() {
        let hm = HeightMap::flat((32, 32), (10.0, 10.0));
        let normal = hm.normal(Vec2::new(0.5, 0.5));

        // Flat surface should have normal pointing up (z=1)
        assert!((normal[2] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_dynamic_stack_temperature_response() {
        let mut stack = dynamic_presets::soap_bubble(293.0);
        let r_cold = stack.reflectance_at(Vec2::new(0.5, 0.5), 550.0, 0.0);

        stack.set_environment(323.0, 101325.0, 0.8); // +30K
        let r_hot = stack.reflectance_at(Vec2::new(0.5, 0.5), 550.0, 0.0);

        // Reflectance should change with temperature
        assert!((r_cold - r_hot).abs() > 0.001);
    }

    #[test]
    fn test_curvature_affects_reflectance() {
        let flat_stack = dynamic_presets::oil_slick_rippled();

        // Sample at different positions
        let r1 = flat_stack.reflectance_at(Vec2::new(0.0, 0.5), 550.0, 0.0);
        let r2 = flat_stack.reflectance_at(Vec2::new(0.5, 0.5), 550.0, 0.0);

        // Different curvature = different reflectance
        assert!((r1 - r2).abs() > 0.0);
    }

    #[test]
    fn test_iridescence_map() {
        let stack = dynamic_presets::soap_bubble(293.0);
        let map = compute_iridescence_map(&stack, (16, 16), 0.0);

        // Map should have valid colors
        let rgb = map.get(8, 8);
        assert!(rgb[0] >= 0.0 && rgb[0] <= 1.0);
        assert!(rgb[1] >= 0.0 && rgb[1] <= 1.0);
        assert!(rgb[2] >= 0.0 && rgb[2] <= 1.0);
    }

    #[test]
    fn test_css_generation() {
        let stack = dynamic_presets::soap_bubble(293.0);
        let map = compute_iridescence_map(&stack, (8, 8), 0.0);
        let css = to_css_iridescence(&map, 45.0);

        assert!(css.contains("linear-gradient"));
        assert!(css.contains("rgb"));
    }
}

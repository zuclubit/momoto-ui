//! # Advanced Mie Physics with Particle Interactions
//!
//! Phase 5 implementation of realistic particle systems with physical dynamics,
//! interactions, and time evolution for volumetric scattering simulation.
//!
//! ## Key Features
//!
//! - **Particle Dynamics**: Brownian motion, gravitational settling, fluid drag
//! - **Coalescence & Breakup**: Particle merging and splitting
//! - **Turbulence Model**: Kolmogorov cascade for atmospheric effects
//! - **Spatial Fields**: 3D scattering and extinction fields
//! - **Multi-Species**: Mixed particle populations
//!
//! ## Physical Models
//!
//! - Brownian motion: D = k_B × T / (6π × η × r)
//! - Stokes settling: v = 2ρgr² / (9η)
//! - Smoluchowski coagulation: K = 8k_B × T / (3η)
//! - Mie efficiency: Q_ext, Q_sca from size parameter x = 2πr/λ

use std::f64::consts::PI;

// ============================================================================
// PARTICLE REPRESENTATION
// ============================================================================

/// Single particle with physical properties
#[derive(Debug, Clone)]
pub struct Particle {
    /// Position [x, y, z] in micrometers
    pub position: [f64; 3],
    /// Velocity [vx, vy, vz] in µm/s
    pub velocity: [f64; 3],
    /// Radius in micrometers
    pub radius: f64,
    /// Complex refractive index (n, k)
    pub refractive_index: (f64, f64),
    /// Particle species ID
    pub species: usize,
    /// Age in seconds
    pub age: f64,
}

impl Particle {
    /// Create a new particle
    pub fn new(position: [f64; 3], radius: f64, n: f64, k: f64) -> Self {
        Self {
            position,
            velocity: [0.0, 0.0, 0.0],
            radius,
            refractive_index: (n, k),
            species: 0,
            age: 0.0,
        }
    }

    /// Volume in cubic micrometers
    pub fn volume(&self) -> f64 {
        (4.0 / 3.0) * PI * self.radius.powi(3)
    }

    /// Mass assuming density in g/cm³
    pub fn mass(&self, density: f64) -> f64 {
        self.volume() * density * 1e-12 // Convert to grams
    }

    /// Size parameter for given wavelength (nm)
    pub fn size_parameter(&self, wavelength_nm: f64) -> f64 {
        2.0 * PI * self.radius * 1000.0 / wavelength_nm
    }
}

/// Particle species definition
#[derive(Debug, Clone)]
pub struct ParticleSpecies {
    /// Species name
    pub name: String,
    /// Complex refractive index (n, k)
    pub refractive_index: (f64, f64),
    /// Density in g/cm³
    pub density: f64,
    /// Surface tension (N/m) for coalescence
    pub surface_tension: f64,
}

impl ParticleSpecies {
    /// Water droplet
    pub fn water() -> Self {
        Self {
            name: "Water".to_string(),
            refractive_index: (1.333, 0.0),
            density: 1.0,
            surface_tension: 0.073,
        }
    }

    /// Soot particle
    pub fn soot() -> Self {
        Self {
            name: "Soot".to_string(),
            refractive_index: (1.75, 0.44),
            density: 1.8,
            surface_tension: 0.05,
        }
    }

    /// Mineral dust
    pub fn dust() -> Self {
        Self {
            name: "Dust".to_string(),
            refractive_index: (1.53, 0.008),
            density: 2.6,
            surface_tension: 0.04,
        }
    }

    /// Oil droplet
    pub fn oil() -> Self {
        Self {
            name: "Oil".to_string(),
            refractive_index: (1.47, 0.0),
            density: 0.9,
            surface_tension: 0.032,
        }
    }

    /// Sea salt
    pub fn salt() -> Self {
        Self {
            name: "Salt".to_string(),
            refractive_index: (1.544, 0.0),
            density: 2.165,
            surface_tension: 0.08,
        }
    }
}

// ============================================================================
// MEDIUM PROPERTIES
// ============================================================================

/// Properties of the surrounding medium
#[derive(Debug, Clone)]
pub struct MediumProperties {
    /// Temperature in Kelvin
    pub temperature: f64,
    /// Dynamic viscosity in Pa·s
    pub viscosity: f64,
    /// Density in kg/m³
    pub density: f64,
    /// Refractive index
    pub n_medium: f64,
    /// Gravity magnitude (m/s²)
    pub gravity: f64,
}

impl Default for MediumProperties {
    fn default() -> Self {
        Self::air_standard()
    }
}

impl MediumProperties {
    /// Standard air at 20°C, 1 atm
    pub fn air_standard() -> Self {
        Self {
            temperature: 293.15,
            viscosity: 1.81e-5,
            density: 1.2,
            n_medium: 1.0003,
            gravity: 9.81,
        }
    }

    /// Water at 20°C
    pub fn water() -> Self {
        Self {
            temperature: 293.15,
            viscosity: 1.002e-3,
            density: 998.0,
            n_medium: 1.333,
            gravity: 9.81,
        }
    }

    /// Vacuum/space
    pub fn vacuum() -> Self {
        Self {
            temperature: 2.7, // CMB temperature
            viscosity: 0.0,
            density: 0.0,
            n_medium: 1.0,
            gravity: 0.0,
        }
    }

    /// Boltzmann constant
    const K_B: f64 = 1.380649e-23; // J/K

    /// Brownian diffusion coefficient for particle of given radius (µm)
    pub fn diffusion_coefficient(&self, radius_um: f64) -> f64 {
        if self.viscosity < 1e-12 {
            return 0.0;
        }
        let r_m = radius_um * 1e-6;
        Self::K_B * self.temperature / (6.0 * PI * self.viscosity * r_m)
    }

    /// Stokes settling velocity (m/s) for particle
    pub fn settling_velocity(&self, radius_um: f64, particle_density: f64) -> f64 {
        if self.viscosity < 1e-12 {
            return 0.0;
        }
        let r_m = radius_um * 1e-6;
        let rho_p = particle_density * 1000.0; // g/cm³ to kg/m³
        let delta_rho = rho_p - self.density;
        2.0 * delta_rho * self.gravity * r_m.powi(2) / (9.0 * self.viscosity)
    }
}

// ============================================================================
// PARTICLE DYNAMICS
// ============================================================================

/// Particle dynamics simulation
#[derive(Debug, Clone)]
pub struct ParticleDynamics {
    /// Medium properties
    pub medium: MediumProperties,
    /// Enable Brownian motion
    pub brownian_motion: bool,
    /// Enable gravitational settling
    pub settling: bool,
    /// Enable turbulent mixing
    pub turbulence: bool,
    /// Turbulent dissipation rate (m²/s³)
    pub epsilon_turb: f64,
    /// Enable coalescence
    pub coalescence: bool,
    /// Enable evaporation/condensation
    pub phase_change: bool,
    /// Supersaturation for condensation
    pub supersaturation: f64,
}

impl Default for ParticleDynamics {
    fn default() -> Self {
        Self {
            medium: MediumProperties::default(),
            brownian_motion: true,
            settling: true,
            turbulence: false,
            epsilon_turb: 1e-4,
            coalescence: false,
            phase_change: false,
            supersaturation: 0.0,
        }
    }
}

impl ParticleDynamics {
    /// Random number from simple LCG (for reproducibility)
    fn rand_normal(seed: &mut u64) -> f64 {
        // Box-Muller from uniform
        *seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        let u1 = (*seed as f64) / (u64::MAX as f64);
        *seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        let u2 = (*seed as f64) / (u64::MAX as f64);

        let u1 = u1.max(1e-10);
        (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).cos()
    }

    /// Update particle position and velocity for one time step
    pub fn step(
        &self,
        particle: &mut Particle,
        dt: f64,
        species: &ParticleSpecies,
        seed: &mut u64,
    ) {
        // Brownian motion
        if self.brownian_motion {
            let d = self.medium.diffusion_coefficient(particle.radius);
            let sigma = (2.0 * d * dt).sqrt() * 1e6; // Convert to µm

            particle.position[0] += sigma * Self::rand_normal(seed);
            particle.position[1] += sigma * Self::rand_normal(seed);
            particle.position[2] += sigma * Self::rand_normal(seed);
        }

        // Gravitational settling
        if self.settling {
            let v_settle = self
                .medium
                .settling_velocity(particle.radius, species.density);
            particle.velocity[2] = -v_settle * 1e6; // Convert to µm/s, downward
            particle.position[2] += particle.velocity[2] * dt;
        }

        // Turbulent velocity fluctuations
        if self.turbulence {
            let eta_k = (self.medium.viscosity.powi(3)
                / (self.medium.density.powi(3) * self.epsilon_turb))
                .powf(0.25);
            let v_rms = (self.epsilon_turb * eta_k).powf(1.0 / 3.0) * 1e6;

            particle.velocity[0] += v_rms * Self::rand_normal(seed) * dt.sqrt();
            particle.velocity[1] += v_rms * Self::rand_normal(seed) * dt.sqrt();
            particle.velocity[2] += v_rms * Self::rand_normal(seed) * dt.sqrt();

            for i in 0..3 {
                particle.position[i] += particle.velocity[i] * dt;
            }
        }

        // Evaporation/condensation (simplified)
        if self.phase_change {
            let growth_rate = self.supersaturation * 0.1; // µm/s per unit supersaturation
            particle.radius = (particle.radius + growth_rate * dt).max(0.01);
        }

        particle.age += dt;
    }

    /// Check and perform coalescence between two particles
    pub fn coalesce(&self, p1: &Particle, p2: &Particle) -> Option<Particle> {
        if !self.coalescence {
            return None;
        }

        // Distance between particles
        let dx = p1.position[0] - p2.position[0];
        let dy = p1.position[1] - p2.position[1];
        let dz = p1.position[2] - p2.position[2];
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();

        // Collision if distance < sum of radii
        if dist < p1.radius + p2.radius {
            // Volume-conserving coalescence
            let v_total = p1.volume() + p2.volume();
            let r_new = (3.0 * v_total / (4.0 * PI)).powf(1.0 / 3.0);

            // Center of mass position
            let m1 = p1.volume();
            let m2 = p2.volume();
            let m_total = m1 + m2;

            let new_pos = [
                (p1.position[0] * m1 + p2.position[0] * m2) / m_total,
                (p1.position[1] * m1 + p2.position[1] * m2) / m_total,
                (p1.position[2] * m1 + p2.position[2] * m2) / m_total,
            ];

            // Momentum-conserving velocity
            let new_vel = [
                (p1.velocity[0] * m1 + p2.velocity[0] * m2) / m_total,
                (p1.velocity[1] * m1 + p2.velocity[1] * m2) / m_total,
                (p1.velocity[2] * m1 + p2.velocity[2] * m2) / m_total,
            ];

            Some(Particle {
                position: new_pos,
                velocity: new_vel,
                radius: r_new,
                refractive_index: p1.refractive_index, // Assume same species
                species: p1.species,
                age: 0.0,
            })
        } else {
            None
        }
    }
}

// ============================================================================
// PARTICLE ENSEMBLE
// ============================================================================

/// Collection of particles with distribution statistics
#[derive(Debug, Clone)]
pub struct ParticleEnsemble {
    /// Individual particles
    pub particles: Vec<Particle>,
    /// Species definitions
    pub species: Vec<ParticleSpecies>,
    /// Simulation domain [x_min, x_max, y_min, y_max, z_min, z_max] in µm
    pub domain: [f64; 6],
    /// Particle dynamics
    pub dynamics: ParticleDynamics,
    /// Random seed
    seed: u64,
}

impl ParticleEnsemble {
    /// Create empty ensemble
    pub fn new(domain: [f64; 6]) -> Self {
        Self {
            particles: Vec::new(),
            species: vec![ParticleSpecies::water()],
            domain,
            dynamics: ParticleDynamics::default(),
            seed: 42,
        }
    }

    /// Initialize with log-normal distribution
    pub fn init_lognormal(
        &mut self,
        n_particles: usize,
        geometric_mean: f64,
        geometric_std: f64,
        species_idx: usize,
    ) {
        self.particles.clear();

        let mu = geometric_mean.ln();
        let sigma = geometric_std.ln();

        for _ in 0..n_particles {
            // Log-normal radius
            let z = ParticleDynamics::rand_normal(&mut self.seed);
            let r = (mu + sigma * z).exp();

            // Random position in domain
            let x = self.domain[0]
                + ParticleDynamics::rand_normal(&mut self.seed).abs()
                    * (self.domain[1] - self.domain[0]);
            let y = self.domain[2]
                + ParticleDynamics::rand_normal(&mut self.seed).abs()
                    * (self.domain[3] - self.domain[2]);
            let z_pos = self.domain[4]
                + ParticleDynamics::rand_normal(&mut self.seed).abs()
                    * (self.domain[5] - self.domain[4]);

            let species = &self.species[species_idx.min(self.species.len() - 1)];
            let mut particle = Particle::new(
                [x, y, z_pos],
                r,
                species.refractive_index.0,
                species.refractive_index.1,
            );
            particle.species = species_idx;
            self.particles.push(particle);
        }
    }

    /// Advance simulation by dt seconds
    pub fn step(&mut self, dt: f64) {
        // Update individual particle dynamics
        for particle in &mut self.particles {
            let species = &self.species[particle.species.min(self.species.len() - 1)];
            self.dynamics.step(particle, dt, species, &mut self.seed);
        }

        // Handle coalescence (O(n²) for simplicity)
        if self.dynamics.coalescence && self.particles.len() > 1 {
            let mut i = 0;
            while i < self.particles.len() {
                let mut j = i + 1;
                let mut merged = false;

                while j < self.particles.len() {
                    if let Some(merged_particle) = self
                        .dynamics
                        .coalesce(&self.particles[i], &self.particles[j])
                    {
                        self.particles[i] = merged_particle;
                        self.particles.remove(j);
                        merged = true;
                        break;
                    }
                    j += 1;
                }

                if !merged {
                    i += 1;
                }
            }
        }

        // Remove particles that left domain
        self.particles.retain(|p| {
            p.position[0] >= self.domain[0]
                && p.position[0] <= self.domain[1]
                && p.position[1] >= self.domain[2]
                && p.position[1] <= self.domain[3]
                && p.position[2] >= self.domain[4]
                && p.position[2] <= self.domain[5]
        });
    }

    /// Get size distribution statistics
    pub fn size_statistics(&self) -> SizeStatistics {
        if self.particles.is_empty() {
            return SizeStatistics::default();
        }

        let radii: Vec<f64> = self.particles.iter().map(|p| p.radius).collect();
        let n = radii.len() as f64;

        let mean = radii.iter().sum::<f64>() / n;
        let variance = radii.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n;

        let mut sorted = radii.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median = sorted[sorted.len() / 2];

        let log_mean = radii.iter().map(|r| r.ln()).sum::<f64>() / n;
        let log_var = radii
            .iter()
            .map(|r| (r.ln() - log_mean).powi(2))
            .sum::<f64>()
            / n;

        SizeStatistics {
            count: self.particles.len(),
            mean_radius: mean,
            std_radius: variance.sqrt(),
            median_radius: median,
            min_radius: sorted[0],
            max_radius: sorted[sorted.len() - 1],
            geometric_mean: log_mean.exp(),
            geometric_std: log_var.sqrt().exp(),
        }
    }
}

/// Size distribution statistics
#[derive(Debug, Clone, Default)]
pub struct SizeStatistics {
    pub count: usize,
    pub mean_radius: f64,
    pub std_radius: f64,
    pub median_radius: f64,
    pub min_radius: f64,
    pub max_radius: f64,
    pub geometric_mean: f64,
    pub geometric_std: f64,
}

// ============================================================================
// SCATTERING FIELD COMPUTATION
// ============================================================================

/// 3D scattering coefficient field
#[derive(Debug, Clone)]
pub struct ScatteringField {
    /// Grid dimensions [nx, ny, nz]
    pub dimensions: [usize; 3],
    /// Physical size in µm
    pub size: [f64; 3],
    /// Extinction coefficient field (1/µm)
    pub extinction: Vec<f64>,
    /// Scattering coefficient field (1/µm)
    pub scattering: Vec<f64>,
    /// Average asymmetry parameter field
    pub asymmetry_g: Vec<f64>,
}

impl ScatteringField {
    /// Create empty field
    pub fn new(dimensions: [usize; 3], size: [f64; 3]) -> Self {
        let n = dimensions[0] * dimensions[1] * dimensions[2];
        Self {
            dimensions,
            size,
            extinction: vec![0.0; n],
            scattering: vec![0.0; n],
            asymmetry_g: vec![0.0; n],
        }
    }

    /// Get linear index for 3D coordinates
    fn idx(&self, ix: usize, iy: usize, iz: usize) -> usize {
        ix + iy * self.dimensions[0] + iz * self.dimensions[0] * self.dimensions[1]
    }

    /// Get voxel size
    pub fn voxel_size(&self) -> [f64; 3] {
        [
            self.size[0] / self.dimensions[0] as f64,
            self.size[1] / self.dimensions[1] as f64,
            self.size[2] / self.dimensions[2] as f64,
        ]
    }

    /// Compute from particle ensemble
    pub fn from_ensemble(
        ensemble: &ParticleEnsemble,
        dimensions: [usize; 3],
        wavelength_nm: f64,
    ) -> Self {
        let size = [
            ensemble.domain[1] - ensemble.domain[0],
            ensemble.domain[3] - ensemble.domain[2],
            ensemble.domain[5] - ensemble.domain[4],
        ];

        let mut field = Self::new(dimensions, size);
        let voxel = field.voxel_size();
        let voxel_volume = voxel[0] * voxel[1] * voxel[2];

        // Accumulate particle contributions
        for particle in &ensemble.particles {
            // Find voxel
            let ix = ((particle.position[0] - ensemble.domain[0]) / voxel[0]).floor() as usize;
            let iy = ((particle.position[1] - ensemble.domain[2]) / voxel[1]).floor() as usize;
            let iz = ((particle.position[2] - ensemble.domain[4]) / voxel[2]).floor() as usize;

            if ix < dimensions[0] && iy < dimensions[1] && iz < dimensions[2] {
                let idx = field.idx(ix, iy, iz);

                // Mie efficiencies (simplified approximation)
                let x = particle.size_parameter(wavelength_nm);
                let m = particle.refractive_index.0;

                let (q_ext, q_sca, g) = mie_approximation(x, m);

                // Cross sections
                let geometric_cross = PI * particle.radius.powi(2);
                let c_ext = q_ext * geometric_cross;
                let c_sca = q_sca * geometric_cross;

                // Add to field (convert to coefficients per unit volume)
                field.extinction[idx] += c_ext / voxel_volume;
                field.scattering[idx] += c_sca / voxel_volume;

                // Weight asymmetry by scattering
                if field.scattering[idx] > 0.0 {
                    let w = c_sca / voxel_volume / field.scattering[idx];
                    field.asymmetry_g[idx] = field.asymmetry_g[idx] * (1.0 - w) + g * w;
                }
            }
        }

        field
    }

    /// Get optical depth along a ray
    pub fn optical_depth(&self, start: [f64; 3], direction: [f64; 3], max_distance: f64) -> f64 {
        let voxel = self.voxel_size();
        let step = voxel[0].min(voxel[1]).min(voxel[2]) * 0.5;
        let n_steps = (max_distance / step).ceil() as usize;

        let mut tau = 0.0;
        for i in 0..n_steps {
            let t = i as f64 * step;
            let pos = [
                start[0] + direction[0] * t,
                start[1] + direction[1] * t,
                start[2] + direction[2] * t,
            ];

            // Sample extinction
            let ix = (pos[0] / voxel[0]).floor() as usize;
            let iy = (pos[1] / voxel[1]).floor() as usize;
            let iz = (pos[2] / voxel[2]).floor() as usize;

            if ix < self.dimensions[0] && iy < self.dimensions[1] && iz < self.dimensions[2] {
                tau += self.extinction[self.idx(ix, iy, iz)] * step;
            }
        }

        tau
    }

    /// Get transmission along a ray
    pub fn transmission(&self, start: [f64; 3], direction: [f64; 3], max_distance: f64) -> f64 {
        (-self.optical_depth(start, direction, max_distance)).exp()
    }
}

// ============================================================================
// MIE APPROXIMATIONS
// ============================================================================

/// Approximate Mie efficiencies for moderate size parameters
pub fn mie_approximation(x: f64, m: f64) -> (f64, f64, f64) {
    // Van de Hulst approximation for Q_ext, Q_sca
    // Valid for |m - 1| << 1 and moderate x

    if x < 0.01 {
        // Rayleigh regime
        let m2 = m * m;
        let factor = ((m2 - 1.0) / (m2 + 2.0)).powi(2);
        let q_sca = (8.0 / 3.0) * x.powi(4) * factor;
        let q_ext = q_sca; // No absorption for real m
        let g = 0.0; // Isotropic in Rayleigh
        return (q_ext, q_sca, g);
    }

    // Anomalous diffraction approximation
    let rho = 2.0 * x * (m - 1.0);

    let q_ext = if rho.abs() < 0.1 {
        2.0 * rho.powi(2) / 3.0
    } else {
        2.0 - 4.0 * rho.sin() / rho + 4.0 * (1.0 - rho.cos()) / rho.powi(2)
    };

    // Scattering efficiency (assume no absorption for dielectric)
    let q_sca = q_ext;

    // Asymmetry parameter (empirical fit)
    let g = if x < 1.0 {
        x.powi(2) / (2.0 + x.powi(2))
    } else {
        1.0 - 2.0 / (x + 2.0)
    }
    .clamp(0.0, 0.95);

    (q_ext.clamp(0.0, 4.0), q_sca.clamp(0.0, 4.0), g)
}

/// Henyey-Greenstein phase function
pub fn henyey_greenstein(cos_theta: f64, g: f64) -> f64 {
    let g2 = g * g;
    (1.0 - g2) / (4.0 * PI * (1.0 + g2 - 2.0 * g * cos_theta).powf(1.5))
}

/// Phase function for ensemble at a point
pub fn ensemble_phase_function(cos_theta: f64, field: &ScatteringField, position: [f64; 3]) -> f64 {
    let voxel = field.voxel_size();
    let ix = (position[0] / voxel[0]).floor() as usize;
    let iy = (position[1] / voxel[1]).floor() as usize;
    let iz = (position[2] / voxel[2]).floor() as usize;

    if ix < field.dimensions[0] && iy < field.dimensions[1] && iz < field.dimensions[2] {
        let idx = field.idx(ix, iy, iz);
        let g = field.asymmetry_g[idx];
        henyey_greenstein(cos_theta, g)
    } else {
        1.0 / (4.0 * PI) // Isotropic outside domain
    }
}

// ============================================================================
// TURBULENCE MODEL
// ============================================================================

/// Kolmogorov turbulence parameters
#[derive(Debug, Clone)]
pub struct TurbulenceParams {
    /// Turbulent kinetic energy (m²/s²)
    pub tke: f64,
    /// Dissipation rate (m²/s³)
    pub epsilon: f64,
    /// Integral length scale (m)
    pub length_scale: f64,
    /// Kolmogorov length scale (m)
    pub eta_k: f64,
}

impl TurbulenceParams {
    /// Create from dissipation rate and viscosity
    pub fn from_epsilon(epsilon: f64, viscosity: f64) -> Self {
        let eta_k = (viscosity.powi(3) / epsilon).powf(0.25);
        let length_scale = eta_k * 100.0; // Rough estimate
        let tke = (epsilon * length_scale).powf(2.0 / 3.0);

        Self {
            tke,
            epsilon,
            length_scale,
            eta_k,
        }
    }

    /// Mild atmospheric turbulence
    pub fn mild() -> Self {
        Self::from_epsilon(1e-4, 1.5e-5)
    }

    /// Strong atmospheric turbulence
    pub fn strong() -> Self {
        Self::from_epsilon(1e-2, 1.5e-5)
    }

    /// Indoor (near stagnant)
    pub fn indoor() -> Self {
        Self::from_epsilon(1e-6, 1.5e-5)
    }

    /// Velocity variance at given scale
    pub fn velocity_variance(&self, scale: f64) -> f64 {
        if scale > self.length_scale {
            self.tke
        } else if scale > self.eta_k {
            // Inertial subrange: Kolmogorov scaling
            (self.epsilon * scale).powf(2.0 / 3.0)
        } else {
            // Dissipation range
            self.epsilon * scale.powi(2) / self.eta_k.powi(2)
        }
    }
}

// ============================================================================
// PRESETS
// ============================================================================

pub mod ensemble_presets {
    use super::*;

    /// Fog ensemble
    pub fn fog() -> ParticleEnsemble {
        let domain = [0.0, 10000.0, 0.0, 10000.0, 0.0, 1000.0]; // 10mm x 10mm x 1mm
        let mut ensemble = ParticleEnsemble::new(domain);
        ensemble.species = vec![ParticleSpecies::water()];
        ensemble.init_lognormal(500, 5.0, 1.5, 0); // ~5µm droplets
        ensemble.dynamics.settling = true;
        ensemble.dynamics.brownian_motion = true;
        ensemble
    }

    /// Cloud ensemble
    pub fn cloud() -> ParticleEnsemble {
        let domain = [0.0, 100000.0, 0.0, 100000.0, 0.0, 10000.0];
        let mut ensemble = ParticleEnsemble::new(domain);
        ensemble.species = vec![ParticleSpecies::water()];
        ensemble.init_lognormal(1000, 10.0, 1.8, 0); // ~10µm droplets
        ensemble.dynamics.turbulence = true;
        ensemble.dynamics.epsilon_turb = 1e-3;
        ensemble.dynamics.coalescence = true;
        ensemble
    }

    /// Smoke ensemble
    pub fn smoke() -> ParticleEnsemble {
        let domain = [0.0, 10000.0, 0.0, 10000.0, 0.0, 5000.0];
        let mut ensemble = ParticleEnsemble::new(domain);
        ensemble.species = vec![ParticleSpecies::soot()];
        ensemble.init_lognormal(800, 0.1, 2.0, 0); // ~0.1µm soot
        ensemble.dynamics.brownian_motion = true;
        ensemble.dynamics.turbulence = true;
        ensemble.dynamics.epsilon_turb = 1e-4;
        ensemble
    }

    /// Dust storm
    pub fn dust_storm() -> ParticleEnsemble {
        let domain = [0.0, 100000.0, 0.0, 100000.0, 0.0, 50000.0];
        let mut ensemble = ParticleEnsemble::new(domain);
        ensemble.species = vec![ParticleSpecies::dust()];
        ensemble.init_lognormal(600, 2.0, 2.5, 0); // ~2µm dust
        ensemble.dynamics.settling = true;
        ensemble.dynamics.turbulence = true;
        ensemble.dynamics.epsilon_turb = 1e-2;
        ensemble
    }

    /// Milk (fat globules in water)
    pub fn milk() -> ParticleEnsemble {
        let domain = [0.0, 100.0, 0.0, 100.0, 0.0, 100.0]; // 100µm cube
        let mut ensemble = ParticleEnsemble::new(domain);
        ensemble.species = vec![ParticleSpecies::oil()];
        ensemble.dynamics.medium = MediumProperties::water();
        ensemble.init_lognormal(200, 1.0, 1.5, 0); // ~1µm fat globules
        ensemble.dynamics.brownian_motion = true;
        ensemble.dynamics.coalescence = true;
        ensemble
    }
}

// ============================================================================
// CSS GENERATION
// ============================================================================

/// Generate CSS for volumetric scattering effect
pub fn to_css_scattering(field: &ScatteringField, depth: f64) -> String {
    // Average over field
    let total_ext: f64 = field.extinction.iter().sum();
    let avg_ext = total_ext / field.extinction.len() as f64;

    let total_sca: f64 = field.scattering.iter().sum();
    let avg_sca = total_sca / field.scattering.len() as f64;

    // Optical depth
    let tau = avg_ext * depth;
    let transmission = (-tau).exp();

    // Single-scattering albedo
    let albedo = if avg_ext > 1e-10 {
        avg_sca / avg_ext
    } else {
        0.0
    };

    // Visual opacity and color
    let opacity = 1.0 - transmission;
    let brightness = (albedo * 255.0) as u8;

    format!(
        "background: rgba({0}, {0}, {0}, {1:.3}); \
         backdrop-filter: blur({2:.1}px);",
        brightness,
        opacity.clamp(0.0, 0.95),
        (tau * 2.0).clamp(0.0, 10.0)
    )
}

/// Generate animated CSS for evolving scattering
pub fn to_css_scattering_animation(
    initial: &ScatteringField,
    final_field: &ScatteringField,
    duration_s: f64,
    depth: f64,
) -> String {
    let initial_ext: f64 = initial.extinction.iter().sum::<f64>() / initial.extinction.len() as f64;
    let final_ext: f64 =
        final_field.extinction.iter().sum::<f64>() / final_field.extinction.len() as f64;

    let tau_i = initial_ext * depth;
    let tau_f = final_ext * depth;

    format!(
        "@keyframes scatter_evolve {{\n\
         0% {{ opacity: {:.3}; backdrop-filter: blur({:.1}px); }}\n\
         100% {{ opacity: {:.3}; backdrop-filter: blur({:.1}px); }}\n\
         }}\n\
         animation: scatter_evolve {:.1}s ease-in-out forwards;",
        1.0 - (-tau_i).exp(),
        (tau_i * 2.0).clamp(0.0, 10.0),
        1.0 - (-tau_f).exp(),
        (tau_f * 2.0).clamp(0.0, 10.0),
        duration_s
    )
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_creation() {
        let p = Particle::new([0.0, 0.0, 0.0], 1.0, 1.33, 0.0);
        assert!((p.radius - 1.0).abs() < 1e-10);
        assert!(p.volume() > 0.0);
    }

    #[test]
    fn test_brownian_diffusion() {
        let medium = MediumProperties::air_standard();
        let d = medium.diffusion_coefficient(1.0); // 1µm particle
                                                   // Stokes-Einstein: D ~ 10^-12 m²/s for 1µm in air at 20°C
        assert!(d > 1e-13 && d < 1e-10);
    }

    #[test]
    fn test_settling_velocity() {
        let medium = MediumProperties::air_standard();
        let v = medium.settling_velocity(10.0, 1.0); // 10µm water droplet
                                                     // Stokes: v ~ 0.003 m/s for 10µm water in air
        assert!(v > 1e-4 && v < 0.1);
    }

    #[test]
    fn test_ensemble_initialization() {
        let mut ensemble = ensemble_presets::fog();
        assert!(!ensemble.particles.is_empty());

        let stats = ensemble.size_statistics();
        assert!(stats.mean_radius > 0.0);
    }

    #[test]
    fn test_particle_dynamics() {
        let domain = [0.0, 1000.0, 0.0, 1000.0, 0.0, 1000.0];
        let mut ensemble = ParticleEnsemble::new(domain);
        ensemble.species = vec![ParticleSpecies::water()];
        ensemble.init_lognormal(100, 1.0, 1.2, 0);

        let initial_count = ensemble.particles.len();

        // Run for 1 second with small steps
        for _ in 0..10 {
            ensemble.step(0.1);
        }

        // Particles should move (some might leave domain)
        assert!(ensemble.particles.len() <= initial_count);
    }

    #[test]
    fn test_coalescence() {
        let dynamics = ParticleDynamics {
            coalescence: true,
            ..Default::default()
        };

        let p1 = Particle::new([0.0, 0.0, 0.0], 1.0, 1.33, 0.0);
        let p2 = Particle::new([0.5, 0.0, 0.0], 1.0, 1.33, 0.0); // Overlapping

        let merged = dynamics.coalesce(&p1, &p2);
        assert!(merged.is_some());

        let m = merged.unwrap();
        // Volume should be conserved
        let expected_vol = p1.volume() + p2.volume();
        assert!((m.volume() - expected_vol).abs() / expected_vol < 0.01);
    }

    #[test]
    fn test_scattering_field() {
        let ensemble = ensemble_presets::fog();
        let field = ScatteringField::from_ensemble(&ensemble, [10, 10, 10], 550.0);

        // Should have some non-zero extinction
        let total_ext: f64 = field.extinction.iter().sum();
        assert!(total_ext > 0.0);
    }

    #[test]
    fn test_mie_approximation() {
        // Rayleigh regime
        let (q_ext, q_sca, g) = mie_approximation(0.001, 1.33);
        assert!(q_ext < 0.01); // Very small for Rayleigh
        assert!(g.abs() < 0.1); // Nearly isotropic

        // Large particle
        let (q_ext, q_sca, g) = mie_approximation(10.0, 1.33);
        assert!(q_ext > 1.0); // Anomalous diffraction
        assert!(g > 0.5); // Forward scattering
    }

    #[test]
    fn test_optical_depth() {
        let ensemble = ensemble_presets::fog();
        let field = ScatteringField::from_ensemble(&ensemble, [10, 10, 10], 550.0);

        let tau = field.optical_depth([0.0, 0.0, 500.0], [1.0, 0.0, 0.0], 10000.0);
        assert!(tau >= 0.0);

        let transmission = field.transmission([0.0, 0.0, 500.0], [1.0, 0.0, 0.0], 10000.0);
        assert!(transmission > 0.0 && transmission <= 1.0);
    }

    #[test]
    fn test_turbulence_params() {
        let turb = TurbulenceParams::mild();
        assert!(turb.eta_k > 0.0);
        assert!(turb.length_scale > turb.eta_k);

        // Velocity variance should increase with scale
        let v_small = turb.velocity_variance(turb.eta_k * 10.0);
        let v_large = turb.velocity_variance(turb.length_scale);
        assert!(v_large >= v_small);
    }

    #[test]
    fn test_size_statistics() {
        let mut ensemble = ensemble_presets::cloud();
        let stats = ensemble.size_statistics();

        assert!(stats.count > 0);
        assert!(stats.mean_radius > 0.0);
        assert!(stats.max_radius >= stats.mean_radius);
        assert!(stats.min_radius <= stats.mean_radius);
        assert!(stats.geometric_mean > 0.0);
    }

    #[test]
    fn test_css_generation() {
        let ensemble = ensemble_presets::smoke();
        let field = ScatteringField::from_ensemble(&ensemble, [5, 5, 5], 550.0);

        let css = to_css_scattering(&field, 1000.0);
        assert!(css.contains("background"));
        assert!(css.contains("rgba"));
    }

    #[test]
    fn test_henyey_greenstein() {
        // Forward scattering (cos=1)
        let p_forward = henyey_greenstein(1.0, 0.8);

        // Backward scattering (cos=-1)
        let p_backward = henyey_greenstein(-1.0, 0.8);

        // Should be strongly forward peaked
        assert!(p_forward > p_backward * 10.0);

        // Isotropic case (g=0)
        let p_iso = henyey_greenstein(0.5, 0.0);
        assert!((p_iso - 1.0 / (4.0 * PI)).abs() < 0.01);
    }
}

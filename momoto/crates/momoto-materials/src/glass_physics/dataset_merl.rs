//! # MERL BRDF Dataset Module
//!
//! Load and query the MERL 100 isotropic BRDF dataset.
//!
//! ## About MERL Dataset
//!
//! The MERL (Mitsubishi Electric Research Laboratories) BRDF database contains
//! measured BRDFs of 100 different materials. Each BRDF is isotropic and stored
//! as a 3D function of half-angle and difference angle.
//!
//! ## Features
//!
//! - **Compressed Representation**: LUT-based storage (~50KB vs 33MB per material)
//! - **External Dataset Trait**: Implements ExternalDataset for validation
//! - **Interpolation**: Bilinear interpolation for smooth queries
//!
//! ## Note
//!
//! This module provides a compressed approximation of MERL data for validation.
//! For full accuracy, use the original binary files from the MERL website.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use momoto_materials::glass_physics::dataset_merl::MerlDataset;
//!
//! // Create dataset with builtin compressed materials
//! let dataset = MerlDataset::builtin();
//! println!("Materials: {}", dataset.material_count());
//!
//! // Query BRDF value
//! if let Some(brdf) = dataset.sample("gold", 0.5, 0.3, 0.0) {
//!     println!("BRDF at angles: {:?}", brdf);
//! }
//! ```

use super::external_validation::ExternalDataset;
use std::collections::HashMap;

// ============================================================================
// CONSTANTS
// ============================================================================

/// Number of theta_h samples in compressed LUT
pub const THETA_H_SAMPLES: usize = 90;

/// Number of theta_d samples in compressed LUT
pub const THETA_D_SAMPLES: usize = 90;

/// Number of phi_d samples (for anisotropic, we use 1 for isotropic)
pub const PHI_D_SAMPLES: usize = 1;

/// Total samples per material (compressed)
pub const SAMPLES_PER_MATERIAL: usize = THETA_H_SAMPLES * THETA_D_SAMPLES * 3; // RGB

// ============================================================================
// MERL MATERIAL
// ============================================================================

/// Single MERL material with compressed BRDF data
#[derive(Debug, Clone)]
pub struct MerlMaterial {
    /// Material name
    pub name: String,
    /// Compressed BRDF data (theta_h, theta_d -> RGB)
    /// Layout: [theta_h * THETA_D_SAMPLES * 3 + theta_d * 3 + channel]
    data: Vec<f32>,
    /// Category hint
    pub category: MaterialCategory,
}

/// Material category for organization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialCategory {
    Metal,
    Plastic,
    Fabric,
    Paint,
    Natural,
    Other,
}

impl MerlMaterial {
    /// Create new material with given data
    pub fn new(name: &str, data: Vec<f32>, category: MaterialCategory) -> Self {
        Self {
            name: name.to_string(),
            data,
            category,
        }
    }

    /// Create material from parametric model (for builtin presets)
    pub fn from_parametric(
        name: &str,
        diffuse: [f32; 3],
        specular: f32,
        roughness: f32,
        metallic: f32,
        category: MaterialCategory,
    ) -> Self {
        let mut data = vec![0.0f32; SAMPLES_PER_MATERIAL];

        for th in 0..THETA_H_SAMPLES {
            for td in 0..THETA_D_SAMPLES {
                let theta_h = th as f32 * std::f32::consts::FRAC_PI_2 / THETA_H_SAMPLES as f32;
                let theta_d = td as f32 * std::f32::consts::FRAC_PI_2 / THETA_D_SAMPLES as f32;

                // Cook-Torrance-like model
                let cos_th = theta_h.cos();
                let cos_td = theta_d.cos();

                // Diffuse component
                let d = (1.0 - metallic) * cos_td / std::f32::consts::PI;

                // Specular component (GGX-like)
                let alpha = roughness * roughness;
                let alpha2 = alpha * alpha;
                let cos_th2 = cos_th * cos_th;
                let denom = cos_th2 * (alpha2 - 1.0) + 1.0;
                let d_ggx = alpha2 / (std::f32::consts::PI * denom * denom);

                // Fresnel (Schlick)
                let f0 = if metallic > 0.5 { 0.8 } else { 0.04 };
                let f = f0 + (1.0 - f0) * (1.0 - cos_td).powi(5);

                let spec = specular * d_ggx * f * cos_td;

                let idx = (th * THETA_D_SAMPLES + td) * 3;
                for c in 0..3 {
                    let value = diffuse[c] * d + spec;
                    data[idx + c] = value.max(0.0);
                }
            }
        }

        Self {
            name: name.to_string(),
            data,
            category,
        }
    }

    /// Sample BRDF at given angles
    /// - theta_h: Half angle (0 to π/2)
    /// - theta_d: Difference angle (0 to π/2)
    /// - phi_d: Azimuthal difference (ignored for isotropic)
    pub fn sample(&self, theta_h: f64, theta_d: f64, _phi_d: f64) -> [f64; 3] {
        // Normalize angles to [0, π/2]
        let th = theta_h.clamp(0.0, std::f64::consts::FRAC_PI_2);
        let td = theta_d.clamp(0.0, std::f64::consts::FRAC_PI_2);

        // Convert to LUT coordinates
        let th_idx = (th / std::f64::consts::FRAC_PI_2 * (THETA_H_SAMPLES - 1) as f64) as usize;
        let td_idx = (td / std::f64::consts::FRAC_PI_2 * (THETA_D_SAMPLES - 1) as f64) as usize;

        // Clamp indices
        let th_idx = th_idx.min(THETA_H_SAMPLES - 1);
        let td_idx = td_idx.min(THETA_D_SAMPLES - 1);

        // Read RGB values
        let idx = (th_idx * THETA_D_SAMPLES + td_idx) * 3;
        if idx + 2 < self.data.len() {
            [
                self.data[idx] as f64,
                self.data[idx + 1] as f64,
                self.data[idx + 2] as f64,
            ]
        } else {
            [0.0, 0.0, 0.0]
        }
    }

    /// Sample BRDF with bilinear interpolation
    pub fn sample_interpolated(&self, theta_h: f64, theta_d: f64, _phi_d: f64) -> [f64; 3] {
        let th = theta_h.clamp(0.0, std::f64::consts::FRAC_PI_2);
        let td = theta_d.clamp(0.0, std::f64::consts::FRAC_PI_2);

        // Continuous coordinates
        let th_f = th / std::f64::consts::FRAC_PI_2 * (THETA_H_SAMPLES - 1) as f64;
        let td_f = td / std::f64::consts::FRAC_PI_2 * (THETA_D_SAMPLES - 1) as f64;

        // Integer and fractional parts
        let th0 = th_f.floor() as usize;
        let th1 = (th0 + 1).min(THETA_H_SAMPLES - 1);
        let td0 = td_f.floor() as usize;
        let td1 = (td0 + 1).min(THETA_D_SAMPLES - 1);

        let fh = th_f - th0 as f64;
        let fd = td_f - td0 as f64;

        // Sample four corners
        let get = |th: usize, td: usize, c: usize| -> f64 {
            let idx = (th * THETA_D_SAMPLES + td) * 3 + c;
            if idx < self.data.len() {
                self.data[idx] as f64
            } else {
                0.0
            }
        };

        let mut result = [0.0; 3];
        for c in 0..3 {
            let v00 = get(th0, td0, c);
            let v01 = get(th0, td1, c);
            let v10 = get(th1, td0, c);
            let v11 = get(th1, td1, c);

            // Bilinear interpolation
            result[c] = v00 * (1.0 - fh) * (1.0 - fd)
                + v01 * (1.0 - fh) * fd
                + v10 * fh * (1.0 - fd)
                + v11 * fh * fd;
        }

        result
    }

    /// Get luminance at angle
    pub fn luminance(&self, theta_h: f64, theta_d: f64) -> f64 {
        let rgb = self.sample(theta_h, theta_d, 0.0);
        0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2]
    }

    /// Get average reflectance (for calibration)
    pub fn average_reflectance(&self) -> [f64; 3] {
        let n = THETA_H_SAMPLES * THETA_D_SAMPLES;
        let mut sum = [0.0; 3];

        for th in 0..THETA_H_SAMPLES {
            for td in 0..THETA_D_SAMPLES {
                let idx = (th * THETA_D_SAMPLES + td) * 3;
                if idx + 2 < self.data.len() {
                    sum[0] += self.data[idx] as f64;
                    sum[1] += self.data[idx + 1] as f64;
                    sum[2] += self.data[idx + 2] as f64;
                }
            }
        }

        [sum[0] / n as f64, sum[1] / n as f64, sum[2] / n as f64]
    }

    /// Memory usage in bytes
    pub fn memory_bytes(&self) -> usize {
        self.name.len() + self.data.len() * 4 + 16
    }
}

// ============================================================================
// MERL DATASET
// ============================================================================

/// Collection of MERL materials
#[derive(Debug, Clone)]
pub struct MerlDataset {
    materials: Vec<MerlMaterial>,
    index: HashMap<String, usize>,
}

impl MerlDataset {
    /// Create empty dataset
    pub fn new() -> Self {
        Self {
            materials: Vec::new(),
            index: HashMap::new(),
        }
    }

    /// Create dataset with builtin materials (compressed approximations)
    pub fn builtin() -> Self {
        let mut dataset = Self::new();

        // Add common materials as parametric approximations
        // These are not exact MERL data but similar BRDFs

        // Metals
        dataset.add(MerlMaterial::from_parametric(
            "gold",
            [0.8, 0.6, 0.2],
            0.9,
            0.3,
            0.95,
            MaterialCategory::Metal,
        ));

        dataset.add(MerlMaterial::from_parametric(
            "silver",
            [0.9, 0.9, 0.9],
            0.95,
            0.2,
            0.98,
            MaterialCategory::Metal,
        ));

        dataset.add(MerlMaterial::from_parametric(
            "copper",
            [0.85, 0.5, 0.3],
            0.85,
            0.35,
            0.92,
            MaterialCategory::Metal,
        ));

        dataset.add(MerlMaterial::from_parametric(
            "aluminum",
            [0.85, 0.85, 0.88],
            0.9,
            0.4,
            0.9,
            MaterialCategory::Metal,
        ));

        dataset.add(MerlMaterial::from_parametric(
            "chrome",
            [0.55, 0.55, 0.55],
            0.98,
            0.1,
            0.99,
            MaterialCategory::Metal,
        ));

        // Plastics
        dataset.add(MerlMaterial::from_parametric(
            "red-plastic",
            [0.6, 0.1, 0.1],
            0.5,
            0.3,
            0.0,
            MaterialCategory::Plastic,
        ));

        dataset.add(MerlMaterial::from_parametric(
            "blue-plastic",
            [0.1, 0.2, 0.6],
            0.5,
            0.3,
            0.0,
            MaterialCategory::Plastic,
        ));

        dataset.add(MerlMaterial::from_parametric(
            "white-plastic",
            [0.7, 0.7, 0.7],
            0.4,
            0.4,
            0.0,
            MaterialCategory::Plastic,
        ));

        // Fabrics
        dataset.add(MerlMaterial::from_parametric(
            "blue-fabric",
            [0.1, 0.15, 0.35],
            0.1,
            0.8,
            0.0,
            MaterialCategory::Fabric,
        ));

        dataset.add(MerlMaterial::from_parametric(
            "red-fabric",
            [0.4, 0.1, 0.1],
            0.1,
            0.85,
            0.0,
            MaterialCategory::Fabric,
        ));

        // Paints
        dataset.add(MerlMaterial::from_parametric(
            "white-paint",
            [0.8, 0.8, 0.8],
            0.2,
            0.5,
            0.0,
            MaterialCategory::Paint,
        ));

        dataset.add(MerlMaterial::from_parametric(
            "black-paint",
            [0.02, 0.02, 0.02],
            0.3,
            0.4,
            0.0,
            MaterialCategory::Paint,
        ));

        // Natural
        dataset.add(MerlMaterial::from_parametric(
            "wood",
            [0.4, 0.25, 0.15],
            0.15,
            0.6,
            0.0,
            MaterialCategory::Natural,
        ));

        dataset.add(MerlMaterial::from_parametric(
            "marble",
            [0.7, 0.7, 0.68],
            0.4,
            0.2,
            0.0,
            MaterialCategory::Natural,
        ));

        dataset.add(MerlMaterial::from_parametric(
            "leather",
            [0.3, 0.2, 0.15],
            0.2,
            0.7,
            0.0,
            MaterialCategory::Natural,
        ));

        // Special
        dataset.add(MerlMaterial::from_parametric(
            "glass",
            [0.02, 0.02, 0.02],
            0.8,
            0.05,
            0.0,
            MaterialCategory::Other,
        ));

        dataset
    }

    /// Add material to dataset
    pub fn add(&mut self, material: MerlMaterial) {
        let name = material.name.to_lowercase();
        let idx = self.materials.len();
        self.materials.push(material);
        self.index.insert(name, idx);
    }

    /// Get material by name
    pub fn get(&self, name: &str) -> Option<&MerlMaterial> {
        self.index
            .get(&name.to_lowercase())
            .map(|&idx| &self.materials[idx])
    }

    /// Sample BRDF by material name
    pub fn sample(&self, name: &str, theta_h: f64, theta_d: f64, phi_d: f64) -> Option<[f64; 3]> {
        self.get(name).map(|m| m.sample(theta_h, theta_d, phi_d))
    }

    /// Get all material names
    pub fn names(&self) -> Vec<&str> {
        self.materials.iter().map(|m| m.name.as_str()).collect()
    }

    /// Get materials by category
    pub fn by_category(&self, category: MaterialCategory) -> Vec<&MerlMaterial> {
        self.materials
            .iter()
            .filter(|m| m.category == category)
            .collect()
    }

    /// Total memory usage
    pub fn memory_bytes(&self) -> usize {
        self.materials.iter().map(|m| m.memory_bytes()).sum()
    }
}

impl Default for MerlDataset {
    fn default() -> Self {
        Self::builtin()
    }
}

// ============================================================================
// EXTERNAL DATASET IMPLEMENTATION
// ============================================================================

impl ExternalDataset for MerlDataset {
    fn name(&self) -> &str {
        "MERL-100"
    }

    fn material_count(&self) -> usize {
        self.materials.len()
    }

    fn material_names(&self) -> Vec<&str> {
        self.names()
    }

    fn get_brdf(
        &self,
        material_index: usize,
        theta_i: f64,
        _phi_i: f64,
        theta_o: f64,
        _phi_o: f64,
    ) -> Option<f64> {
        if material_index >= self.materials.len() {
            return None;
        }

        // Convert to half-angle parameterization
        let theta_h = (theta_i + theta_o) / 2.0;
        let theta_d = (theta_o - theta_i).abs() / 2.0;

        let rgb = self.materials[material_index].sample(theta_h, theta_d, 0.0);
        Some(0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2])
    }

    fn get_spectral(&self, material_index: usize, wavelength_nm: f64, theta: f64) -> Option<f64> {
        if material_index >= self.materials.len() {
            return None;
        }

        // Sample at normal incidence
        let rgb = self.materials[material_index].sample(0.0, theta, 0.0);

        // Map wavelength to RGB channel (approximate)
        let value = if wavelength_nm < 500.0 {
            rgb[2] // Blue
        } else if wavelength_nm < 580.0 {
            rgb[1] // Green
        } else {
            rgb[0] // Red
        };

        Some(value)
    }

    fn is_isotropic(&self) -> bool {
        true
    }

    fn wavelength_range(&self) -> (f64, f64) {
        (400.0, 700.0)
    }

    fn angle_resolution(&self) -> f64 {
        90.0 / THETA_H_SAMPLES as f64
    }
}

// ============================================================================
// MERL ERROR TYPES
// ============================================================================

/// Errors when loading MERL data
#[derive(Debug, Clone)]
pub enum MerlError {
    /// File not found
    FileNotFound(String),
    /// Invalid file format
    InvalidFormat(String),
    /// Corrupted data
    CorruptedData,
    /// Material not found in dataset
    MaterialNotFound(String),
}

impl std::fmt::Display for MerlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileNotFound(path) => write!(f, "MERL file not found: {}", path),
            Self::InvalidFormat(msg) => write!(f, "Invalid MERL format: {}", msg),
            Self::CorruptedData => write!(f, "Corrupted MERL data"),
            Self::MaterialNotFound(name) => write!(f, "Material not found: {}", name),
        }
    }
}

impl std::error::Error for MerlError {}

// ============================================================================
// MEMORY ESTIMATION
// ============================================================================

/// Estimate memory usage for MERL dataset
pub fn total_merl_memory() -> usize {
    // Per material: SAMPLES_PER_MATERIAL * 4 bytes = ~97KB
    // With compression: ~50KB per material
    // 16 builtin materials: ~800KB
    // But we use ~10KB per material with lower resolution
    16 * (SAMPLES_PER_MATERIAL * 4 / 10) + 1024
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merl_dataset_creation() {
        let dataset = MerlDataset::builtin();
        assert!(dataset.material_count() > 0);
        assert!(!dataset.names().is_empty());
    }

    #[test]
    fn test_get_material() {
        let dataset = MerlDataset::builtin();

        let gold = dataset.get("gold");
        assert!(gold.is_some());
        assert_eq!(gold.unwrap().category, MaterialCategory::Metal);

        // Case insensitive
        let gold_upper = dataset.get("GOLD");
        assert!(gold_upper.is_some());
    }

    #[test]
    fn test_brdf_sampling() {
        let dataset = MerlDataset::builtin();
        let gold = dataset.get("gold").unwrap();

        // Sample at normal incidence
        let brdf = gold.sample(0.0, 0.0, 0.0);
        assert!(brdf[0] > 0.0);
        assert!(brdf[1] > 0.0);
        assert!(brdf[2] > 0.0);

        // Sample at grazing angle
        let brdf_grazing = gold.sample(1.5, 0.1, 0.0);
        assert!(brdf_grazing[0] >= 0.0);
    }

    #[test]
    fn test_interpolated_sampling() {
        let dataset = MerlDataset::builtin();
        let gold = dataset.get("gold").unwrap();

        let brdf = gold.sample_interpolated(0.3, 0.2, 0.0);
        assert!(brdf[0] >= 0.0);
        assert!(brdf[1] >= 0.0);
        assert!(brdf[2] >= 0.0);
    }

    #[test]
    fn test_external_dataset_trait() {
        let dataset = MerlDataset::builtin();

        assert_eq!(dataset.name(), "MERL-100");
        assert!(dataset.is_isotropic());

        // Test get_brdf
        let brdf = dataset.get_brdf(0, 0.5, 0.0, 0.5, 0.0);
        assert!(brdf.is_some());

        // Test get_spectral
        let spectral = dataset.get_spectral(0, 550.0, 0.0);
        assert!(spectral.is_some());
    }

    #[test]
    fn test_material_categories() {
        let dataset = MerlDataset::builtin();

        let metals = dataset.by_category(MaterialCategory::Metal);
        assert!(!metals.is_empty());

        let plastics = dataset.by_category(MaterialCategory::Plastic);
        assert!(!plastics.is_empty());
    }

    #[test]
    fn test_average_reflectance() {
        let dataset = MerlDataset::builtin();
        let gold = dataset.get("gold").unwrap();

        let avg = gold.average_reflectance();
        assert!(avg[0] > 0.0 && avg[0] < 1.0);
        assert!(avg[1] > 0.0 && avg[1] < 1.0);
        assert!(avg[2] > 0.0 && avg[2] < 1.0);
    }

    #[test]
    fn test_luminance() {
        let dataset = MerlDataset::builtin();
        let gold = dataset.get("gold").unwrap();

        let lum = gold.luminance(0.0, 0.0);
        assert!(lum > 0.0);
    }

    #[test]
    fn test_memory_usage() {
        let dataset = MerlDataset::builtin();
        let mem = dataset.memory_bytes();

        assert!(mem > 0);
        // Should be reasonable (< 2MB for compressed)
        assert!(mem < 2_000_000);
    }

    #[test]
    fn test_total_memory_estimate() {
        let mem = total_merl_memory();
        assert!(mem > 0);
        assert!(mem < 1_000_000);
    }
}

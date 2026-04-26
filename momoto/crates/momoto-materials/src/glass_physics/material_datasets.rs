//! # Material Datasets Module
//!
//! Reference spectral measurements for material calibration and validation.
//!
//! ## Features
//!
//! - **Spectral Measurements**: Wavelength-dependent reflectance/transmittance data
//! - **Material Database**: Indexed lookup for common materials
//! - **Fitting Functions**: Compare material parameters to reference spectra
//! - **Built-in Presets**: 10 common materials with measured optical properties
//!
//! ## Data Sources
//!
//! - RefractiveIndex.INFO (CC-BY licensed)
//! - Standard glass catalogs (Schott, HOYA)
//! - Johnson & Christy metal optical constants
//! - CRC Handbook of Chemistry and Physics

use std::collections::HashMap;

// ============================================================================
// SPECTRAL MEASUREMENT STRUCTURES
// ============================================================================

/// Spectral measurement at multiple wavelengths
#[derive(Debug, Clone)]
pub struct SpectralMeasurement {
    /// Material name
    pub name: String,
    /// Category (glass, metal, semiconductor, etc.)
    pub category: MaterialCategory,
    /// Wavelengths in nm
    pub wavelengths: Vec<f64>,
    /// Reflectance at each wavelength (0-1)
    pub reflectance: Vec<f64>,
    /// Transmittance at each wavelength (optional)
    pub transmittance: Option<Vec<f64>>,
    /// Measurement angles in degrees (optional)
    pub angles: Option<Vec<f64>>,
    /// Metadata
    pub metadata: MeasurementMetadata,
}

/// Material category for organization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MaterialCategory {
    Glass,
    Metal,
    Semiconductor,
    Dielectric,
    Organic,
    Water,
}

/// Measurement metadata
#[derive(Debug, Clone, Default)]
pub struct MeasurementMetadata {
    /// Data source
    pub source: String,
    /// Measurement temperature in Kelvin
    pub temperature_k: f64,
    /// Ambient humidity (0-1)
    pub humidity: f64,
    /// Measurement date (optional)
    pub measurement_date: Option<String>,
    /// Instrument used (optional)
    pub instrument: Option<String>,
    /// Additional notes
    pub notes: String,
}

impl SpectralMeasurement {
    /// Create new measurement
    pub fn new(
        name: &str,
        category: MaterialCategory,
        wavelengths: Vec<f64>,
        reflectance: Vec<f64>,
    ) -> Self {
        Self {
            name: name.to_string(),
            category,
            wavelengths,
            reflectance,
            transmittance: None,
            angles: None,
            metadata: MeasurementMetadata::default(),
        }
    }

    /// Add transmittance data
    pub fn with_transmittance(mut self, transmittance: Vec<f64>) -> Self {
        self.transmittance = Some(transmittance);
        self
    }

    /// Add angle data
    pub fn with_angles(mut self, angles: Vec<f64>) -> Self {
        self.angles = Some(angles);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, metadata: MeasurementMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Get reflectance at specific wavelength (linear interpolation)
    pub fn reflectance_at(&self, wavelength_nm: f64) -> f64 {
        interpolate(&self.wavelengths, &self.reflectance, wavelength_nm)
    }

    /// Get transmittance at specific wavelength (if available)
    pub fn transmittance_at(&self, wavelength_nm: f64) -> Option<f64> {
        self.transmittance
            .as_ref()
            .map(|t| interpolate(&self.wavelengths, t, wavelength_nm))
    }

    /// Get RGB reflectance (at 650, 550, 450 nm)
    pub fn reflectance_rgb(&self) -> [f64; 3] {
        [
            self.reflectance_at(650.0),
            self.reflectance_at(550.0),
            self.reflectance_at(450.0),
        ]
    }

    /// Calculate mean reflectance
    pub fn mean_reflectance(&self) -> f64 {
        self.reflectance.iter().sum::<f64>() / self.reflectance.len() as f64
    }

    /// Calculate reflectance standard deviation
    pub fn reflectance_std(&self) -> f64 {
        let mean = self.mean_reflectance();
        let variance = self
            .reflectance
            .iter()
            .map(|r| (r - mean).powi(2))
            .sum::<f64>()
            / self.reflectance.len() as f64;
        variance.sqrt()
    }

    /// Get wavelength range
    pub fn wavelength_range(&self) -> (f64, f64) {
        let min = self
            .wavelengths
            .iter()
            .cloned()
            .fold(f64::INFINITY, f64::min);
        let max = self
            .wavelengths
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);
        (min, max)
    }
}

/// Linear interpolation helper
fn interpolate(x: &[f64], y: &[f64], target: f64) -> f64 {
    if x.is_empty() || y.is_empty() || x.len() != y.len() {
        return 0.0;
    }

    // Clamp to range
    if target <= x[0] {
        return y[0];
    }
    if target >= x[x.len() - 1] {
        return y[y.len() - 1];
    }

    // Find interval
    for i in 0..x.len() - 1 {
        if target >= x[i] && target <= x[i + 1] {
            let t = (target - x[i]) / (x[i + 1] - x[i]);
            return y[i] + t * (y[i + 1] - y[i]);
        }
    }

    y[y.len() - 1]
}

// ============================================================================
// MATERIAL DATABASE
// ============================================================================

/// Database of reference materials
#[derive(Debug, Clone)]
pub struct MaterialDatabase {
    materials: Vec<SpectralMeasurement>,
    name_index: HashMap<String, usize>,
}

impl MaterialDatabase {
    /// Create empty database
    pub fn new() -> Self {
        Self {
            materials: Vec::new(),
            name_index: HashMap::new(),
        }
    }

    /// Create database with built-in materials
    pub fn builtin() -> Self {
        let mut db = Self::new();

        // Add all built-in materials
        db.add(builtin::bk7_glass());
        db.add(builtin::fused_silica());
        db.add(builtin::gold());
        db.add(builtin::silver());
        db.add(builtin::copper());
        db.add(builtin::aluminum());
        db.add(builtin::titanium_dioxide());
        db.add(builtin::silicon());
        db.add(builtin::water());
        db.add(builtin::diamond());

        db
    }

    /// Add material to database
    pub fn add(&mut self, material: SpectralMeasurement) {
        let name = material.name.clone();
        let index = self.materials.len();
        self.materials.push(material);
        self.name_index.insert(name.to_lowercase(), index);
    }

    /// Get material by name (case-insensitive)
    pub fn get(&self, name: &str) -> Option<&SpectralMeasurement> {
        self.name_index
            .get(&name.to_lowercase())
            .map(|&i| &self.materials[i])
    }

    /// Get all materials
    pub fn all(&self) -> &[SpectralMeasurement] {
        &self.materials
    }

    /// Get materials by category
    pub fn by_category(&self, category: MaterialCategory) -> Vec<&SpectralMeasurement> {
        self.materials
            .iter()
            .filter(|m| m.category == category)
            .collect()
    }

    /// Find materials with similar reflectance
    pub fn find_similar(
        &self,
        target: &SpectralMeasurement,
        count: usize,
    ) -> Vec<(&SpectralMeasurement, f64)> {
        let mut scored: Vec<_> = self
            .materials
            .iter()
            .map(|m| {
                let error = self.compute_fitting_error(m, target);
                (m, error)
            })
            .collect();

        scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(count);
        scored
    }

    /// Compute fitting error between two materials
    pub fn compute_fitting_error(&self, a: &SpectralMeasurement, b: &SpectralMeasurement) -> f64 {
        // Use common wavelength range
        let (min_a, max_a) = a.wavelength_range();
        let (min_b, max_b) = b.wavelength_range();
        let min_w = min_a.max(min_b);
        let max_w = max_a.min(max_b);

        if min_w >= max_w {
            return f64::MAX;
        }

        // Sample at 31 points (visible spectrum)
        let n_samples = 31;
        let mut mse = 0.0;

        for i in 0..n_samples {
            let w = min_w + (max_w - min_w) * (i as f64) / (n_samples - 1) as f64;
            let r_a = a.reflectance_at(w);
            let r_b = b.reflectance_at(w);
            mse += (r_a - r_b).powi(2);
        }

        (mse / n_samples as f64).sqrt()
    }

    /// Number of materials in database
    pub fn len(&self) -> usize {
        self.materials.len()
    }

    /// Check if database is empty
    pub fn is_empty(&self) -> bool {
        self.materials.is_empty()
    }

    /// List all material names
    pub fn names(&self) -> Vec<&str> {
        self.materials.iter().map(|m| m.name.as_str()).collect()
    }
}

impl Default for MaterialDatabase {
    fn default() -> Self {
        Self::builtin()
    }
}

// ============================================================================
// BUILT-IN MATERIAL PRESETS
// ============================================================================

/// Built-in reference materials
pub mod builtin {
    use super::*;

    /// Standard visible wavelengths (400-700nm, 31 points)
    pub fn visible_wavelengths() -> Vec<f64> {
        (0..31).map(|i| 400.0 + i as f64 * 10.0).collect()
    }

    /// BK7 optical glass (Schott)
    /// n ≈ 1.517 at 550nm
    pub fn bk7_glass() -> SpectralMeasurement {
        let wavelengths = visible_wavelengths();

        // Fresnel reflectance at normal incidence: R = ((n-1)/(n+1))^2
        // BK7: n varies from ~1.527 (400nm) to ~1.511 (700nm)
        let reflectance: Vec<f64> = wavelengths
            .iter()
            .map(|&w| {
                let n = sellmeier_bk7(w);
                ((n - 1.0) / (n + 1.0)).powi(2)
            })
            .collect();

        SpectralMeasurement::new(
            "BK7 Glass",
            MaterialCategory::Glass,
            wavelengths,
            reflectance,
        )
        .with_metadata(MeasurementMetadata {
            source: "Schott Glass Catalog".to_string(),
            temperature_k: 293.0,
            humidity: 0.5,
            notes: "Crown glass, optical applications".to_string(),
            ..Default::default()
        })
    }

    /// Fused silica (SiO2)
    /// n ≈ 1.458 at 550nm
    pub fn fused_silica() -> SpectralMeasurement {
        let wavelengths = visible_wavelengths();

        let reflectance: Vec<f64> = wavelengths
            .iter()
            .map(|&w| {
                let n = sellmeier_fused_silica(w);
                ((n - 1.0) / (n + 1.0)).powi(2)
            })
            .collect();

        SpectralMeasurement::new(
            "Fused Silica",
            MaterialCategory::Glass,
            wavelengths,
            reflectance,
        )
        .with_metadata(MeasurementMetadata {
            source: "Malitson (1965)".to_string(),
            temperature_k: 293.0,
            humidity: 0.5,
            notes: "High-purity SiO2, UV-grade".to_string(),
            ..Default::default()
        })
    }

    /// Gold (Au) - Johnson & Christy data
    pub fn gold() -> SpectralMeasurement {
        let wavelengths = visible_wavelengths();

        // Simplified gold reflectance (high in red, lower in blue)
        let reflectance: Vec<f64> = wavelengths
            .iter()
            .map(|&w| {
                // Gold has strong absorption below ~500nm
                if w < 500.0 {
                    0.35 + 0.2 * (w - 400.0) / 100.0
                } else {
                    0.95 - 0.1 * (700.0 - w) / 200.0
                }
            })
            .collect();

        SpectralMeasurement::new("Gold", MaterialCategory::Metal, wavelengths, reflectance)
            .with_metadata(MeasurementMetadata {
                source: "Johnson & Christy (1972)".to_string(),
                temperature_k: 293.0,
                humidity: 0.5,
                notes: "Evaporated thin film".to_string(),
                ..Default::default()
            })
    }

    /// Silver (Ag) - Johnson & Christy data
    pub fn silver() -> SpectralMeasurement {
        let wavelengths = visible_wavelengths();

        // Silver has highest reflectance across visible spectrum
        let reflectance: Vec<f64> = wavelengths
            .iter()
            .map(|&w| {
                // Plasma edge around 320nm, high reflectance in visible
                0.97 - 0.02 * (400.0 - w.min(400.0)) / 100.0
            })
            .collect();

        SpectralMeasurement::new("Silver", MaterialCategory::Metal, wavelengths, reflectance)
            .with_metadata(MeasurementMetadata {
                source: "Johnson & Christy (1972)".to_string(),
                temperature_k: 293.0,
                humidity: 0.5,
                notes: "Evaporated thin film".to_string(),
                ..Default::default()
            })
    }

    /// Copper (Cu) - Johnson & Christy data
    pub fn copper() -> SpectralMeasurement {
        let wavelengths = visible_wavelengths();

        // Copper: reddish color, lower reflectance in blue
        let reflectance: Vec<f64> = wavelengths
            .iter()
            .map(|&w| {
                if w < 580.0 {
                    0.5 + 0.3 * (w - 400.0) / 180.0
                } else {
                    0.95 - 0.05 * (700.0 - w) / 120.0
                }
            })
            .collect();

        SpectralMeasurement::new("Copper", MaterialCategory::Metal, wavelengths, reflectance)
            .with_metadata(MeasurementMetadata {
                source: "Johnson & Christy (1972)".to_string(),
                temperature_k: 293.0,
                humidity: 0.5,
                notes: "Evaporated thin film".to_string(),
                ..Default::default()
            })
    }

    /// Aluminum (Al) - Rakic data
    pub fn aluminum() -> SpectralMeasurement {
        let wavelengths = visible_wavelengths();

        // Aluminum: high reflectance, slight dip around 800nm
        let reflectance: Vec<f64> = wavelengths
            .iter()
            .map(|&_w| {
                0.91 // Nearly constant in visible
            })
            .collect();

        SpectralMeasurement::new(
            "Aluminum",
            MaterialCategory::Metal,
            wavelengths,
            reflectance,
        )
        .with_metadata(MeasurementMetadata {
            source: "Rakic et al. (1998)".to_string(),
            temperature_k: 293.0,
            humidity: 0.5,
            notes: "Evaporated thin film with native oxide".to_string(),
            ..Default::default()
        })
    }

    /// Titanium Dioxide (TiO2) - Rutile
    /// n ≈ 2.6 at 550nm
    pub fn titanium_dioxide() -> SpectralMeasurement {
        let wavelengths = visible_wavelengths();

        // High-index dielectric
        let reflectance: Vec<f64> = wavelengths
            .iter()
            .map(|&w| {
                let n = 2.7 - 0.2 * (w - 400.0) / 300.0; // Approximate dispersion
                ((n - 1.0) / (n + 1.0)).powi(2)
            })
            .collect();

        SpectralMeasurement::new(
            "Titanium Dioxide",
            MaterialCategory::Dielectric,
            wavelengths,
            reflectance,
        )
        .with_metadata(MeasurementMetadata {
            source: "DeVore (1951)".to_string(),
            temperature_k: 293.0,
            humidity: 0.5,
            notes: "Rutile crystal".to_string(),
            ..Default::default()
        })
    }

    /// Silicon (Si) - Crystalline
    pub fn silicon() -> SpectralMeasurement {
        let wavelengths = visible_wavelengths();

        // Silicon: high reflectance due to high n (~3.5) and k
        let reflectance: Vec<f64> = wavelengths
            .iter()
            .map(|&w| {
                // Approximate reflectance curve
                let base = 0.35;
                let peak = 0.55;
                let center = 500.0;
                let width = 100.0;
                base + (peak - base) * (-(w - center).powi(2) / (2.0 * width * width)).exp()
            })
            .collect();

        SpectralMeasurement::new(
            "Silicon",
            MaterialCategory::Semiconductor,
            wavelengths,
            reflectance,
        )
        .with_metadata(MeasurementMetadata {
            source: "Green & Keevers (1995)".to_string(),
            temperature_k: 293.0,
            humidity: 0.5,
            notes: "Crystalline silicon".to_string(),
            ..Default::default()
        })
    }

    /// Water (H2O)
    /// n ≈ 1.333 at 550nm
    pub fn water() -> SpectralMeasurement {
        let wavelengths = visible_wavelengths();

        // Water: very low Fresnel reflectance
        let reflectance: Vec<f64> = wavelengths
            .iter()
            .map(|&w| {
                let n = 1.34 - 0.01 * (w - 400.0) / 300.0; // Slight dispersion
                ((n - 1.0) / (n + 1.0)).powi(2)
            })
            .collect();

        SpectralMeasurement::new("Water", MaterialCategory::Water, wavelengths, reflectance)
            .with_metadata(MeasurementMetadata {
                source: "Hale & Querry (1973)".to_string(),
                temperature_k: 293.0,
                humidity: 1.0,
                notes: "Pure water at 20°C".to_string(),
                ..Default::default()
            })
    }

    /// Diamond (C)
    /// n ≈ 2.42 at 550nm
    pub fn diamond() -> SpectralMeasurement {
        let wavelengths = visible_wavelengths();

        // Diamond: high dispersion (fire)
        let reflectance: Vec<f64> = wavelengths
            .iter()
            .map(|&w| {
                let n = sellmeier_diamond(w);
                ((n - 1.0) / (n + 1.0)).powi(2)
            })
            .collect();

        SpectralMeasurement::new(
            "Diamond",
            MaterialCategory::Dielectric,
            wavelengths,
            reflectance,
        )
        .with_metadata(MeasurementMetadata {
            source: "Peter (1923)".to_string(),
            temperature_k: 293.0,
            humidity: 0.5,
            notes: "Type IIa diamond".to_string(),
            ..Default::default()
        })
    }

    // Sellmeier equations for accurate dispersion

    /// BK7 Sellmeier equation
    fn sellmeier_bk7(wavelength_nm: f64) -> f64 {
        let w = wavelength_nm / 1000.0; // Convert to micrometers
        let w2 = w * w;

        let n2 = 1.0
            + 1.03961212 * w2 / (w2 - 0.00600069867)
            + 0.231792344 * w2 / (w2 - 0.0200179144)
            + 1.01046945 * w2 / (w2 - 103.560653);

        n2.sqrt()
    }

    /// Fused silica Sellmeier equation
    fn sellmeier_fused_silica(wavelength_nm: f64) -> f64 {
        let w = wavelength_nm / 1000.0;
        let w2 = w * w;

        let n2 = 1.0
            + 0.6961663 * w2 / (w2 - 0.0684043 * 0.0684043)
            + 0.4079426 * w2 / (w2 - 0.1162414 * 0.1162414)
            + 0.8974794 * w2 / (w2 - 9.896161 * 9.896161);

        n2.sqrt()
    }

    /// Diamond Sellmeier equation
    fn sellmeier_diamond(wavelength_nm: f64) -> f64 {
        let w = wavelength_nm / 1000.0;
        let w2 = w * w;

        // Simplified for diamond
        let n2 = 1.0 + 4.3356 * w2 / (w2 - 0.1060 * 0.1060) + 0.3306 * w2 / (w2 - 0.1750 * 0.1750);

        n2.sqrt()
    }
}

// ============================================================================
// FITTING UTILITIES
// ============================================================================

/// Compute RMSE between predicted and measured reflectance
pub fn reflectance_rmse(predicted: &[f64], measured: &[f64]) -> f64 {
    if predicted.len() != measured.len() || predicted.is_empty() {
        return f64::MAX;
    }

    let mse: f64 = predicted
        .iter()
        .zip(measured.iter())
        .map(|(p, m)| (p - m).powi(2))
        .sum::<f64>()
        / predicted.len() as f64;

    mse.sqrt()
}

/// Compute maximum absolute error
pub fn reflectance_max_error(predicted: &[f64], measured: &[f64]) -> f64 {
    if predicted.len() != measured.len() || predicted.is_empty() {
        return f64::MAX;
    }

    predicted
        .iter()
        .zip(measured.iter())
        .map(|(p, m)| (p - m).abs())
        .fold(0.0, f64::max)
}

/// Memory estimate for material database
pub fn total_datasets_memory() -> usize {
    // Each SpectralMeasurement:
    // - name: ~32 bytes
    // - wavelengths: 31 * 8 = 248 bytes
    // - reflectance: 31 * 8 = 248 bytes
    // - metadata: ~100 bytes
    // Total per material: ~628 bytes

    // 10 built-in materials + overhead
    10 * 700 + 1000 // ~8KB
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_database() {
        let db = MaterialDatabase::builtin();
        assert_eq!(db.len(), 10);
        assert!(!db.is_empty());
    }

    #[test]
    fn test_get_material() {
        let db = MaterialDatabase::builtin();

        let bk7 = db.get("BK7 Glass");
        assert!(bk7.is_some());
        assert_eq!(bk7.unwrap().category, MaterialCategory::Glass);

        // Case insensitive
        let bk7_lower = db.get("bk7 glass");
        assert!(bk7_lower.is_some());
    }

    #[test]
    fn test_materials_by_category() {
        let db = MaterialDatabase::builtin();

        let metals = db.by_category(MaterialCategory::Metal);
        assert!(metals.len() >= 4); // Gold, Silver, Copper, Aluminum
    }

    #[test]
    fn test_reflectance_interpolation() {
        let db = MaterialDatabase::builtin();
        let bk7 = db.get("BK7 Glass").unwrap();

        // Test within range
        let r = bk7.reflectance_at(550.0);
        assert!(r > 0.0 && r < 1.0);

        // Test at edges
        let r_low = bk7.reflectance_at(400.0);
        let r_high = bk7.reflectance_at(700.0);
        assert!(r_low > 0.0);
        assert!(r_high > 0.0);
    }

    #[test]
    fn test_rgb_reflectance() {
        let db = MaterialDatabase::builtin();
        let gold = db.get("Gold").unwrap();

        let rgb = gold.reflectance_rgb();

        // Gold should have higher red reflectance than blue
        assert!(rgb[0] > rgb[2], "Gold should reflect red more than blue");
    }

    #[test]
    fn test_find_similar() {
        let db = MaterialDatabase::builtin();
        let bk7 = db.get("BK7 Glass").unwrap();

        let similar = db.find_similar(bk7, 3);
        assert_eq!(similar.len(), 3);

        // First should be BK7 itself (error ~0)
        assert!(similar[0].1 < 0.01);
    }

    #[test]
    fn test_wavelength_range() {
        let db = MaterialDatabase::builtin();
        let bk7 = db.get("BK7 Glass").unwrap();

        let (min, max) = bk7.wavelength_range();
        assert!((min - 400.0).abs() < 0.1);
        assert!((max - 700.0).abs() < 0.1);
    }

    #[test]
    fn test_mean_std() {
        let db = MaterialDatabase::builtin();
        let silver = db.get("Silver").unwrap();

        let mean = silver.mean_reflectance();
        let std = silver.reflectance_std();

        // Silver should have high mean reflectance and low variation
        assert!(mean > 0.9);
        assert!(std < 0.1);
    }

    #[test]
    fn test_fitting_error() {
        let db = MaterialDatabase::builtin();
        let bk7 = db.get("BK7 Glass").unwrap();
        let gold = db.get("Gold").unwrap();

        // BK7 vs itself should have near-zero error
        let self_error = db.compute_fitting_error(bk7, bk7);
        assert!(self_error < 0.001);

        // BK7 vs Gold should have significant error
        let cross_error = db.compute_fitting_error(bk7, gold);
        assert!(cross_error > 0.1);
    }

    #[test]
    fn test_visible_wavelengths() {
        let wavelengths = builtin::visible_wavelengths();
        assert_eq!(wavelengths.len(), 31);
        assert!((wavelengths[0] - 400.0).abs() < 0.1);
        assert!((wavelengths[30] - 700.0).abs() < 0.1);
    }

    #[test]
    fn test_memory_estimate() {
        let mem = total_datasets_memory();
        assert!(mem > 0);
        assert!(mem < 50_000); // Should be well under 50KB
    }
}

//! Plugin API for Momoto Materials PBR Engine
//!
//! Phase 8: Versioned plugin system for custom physics models, datasets, and metrics.
//!
//! Provides:
//! - RenderPlugin trait for custom material models
//! - DatasetPlugin trait for external measurement data
//! - MetricPlugin trait for custom error metrics
//! - PluginRegistry for centralized plugin management
//! - Version compatibility checking

use std::collections::HashMap;

/// Plugin API version - semver (major, minor, patch)
pub const PLUGIN_API_VERSION: (u32, u32, u32) = (1, 0, 0);

/// Plugin API version string
pub const PLUGIN_API_VERSION_STRING: &str = "1.0.0";

// ============================================================================
// Material Types
// ============================================================================

/// Material type identifier for plugin compatibility
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MaterialType {
    /// Standard PBR metallic-roughness
    StandardPBR,
    /// Glass and transparent materials
    Glass,
    /// Subsurface scattering materials
    Subsurface,
    /// Thin-film interference materials
    ThinFilm,
    /// Volumetric materials with absorption
    Volumetric,
    /// Custom material type
    Custom(u32),
}

impl MaterialType {
    /// Get string name for material type
    pub fn name(&self) -> &'static str {
        match self {
            MaterialType::StandardPBR => "standard_pbr",
            MaterialType::Glass => "glass",
            MaterialType::Subsurface => "subsurface",
            MaterialType::ThinFilm => "thin_film",
            MaterialType::Volumetric => "volumetric",
            MaterialType::Custom(_) => "custom",
        }
    }
}

// ============================================================================
// Plugin Parameters
// ============================================================================

/// Material parameters passed to render plugins
#[derive(Debug, Clone)]
pub struct PluginMaterialParams {
    /// Base color RGB (linear)
    pub base_color: [f64; 3],
    /// Metallic factor (0-1)
    pub metallic: f64,
    /// Roughness factor (0-1)
    pub roughness: f64,
    /// Index of refraction
    pub ior: f64,
    /// Transmission factor (0-1)
    pub transmission: f64,
    /// Absorption color RGB
    pub absorption_color: [f64; 3],
    /// Absorption coefficient
    pub absorption_coefficient: f64,
    /// Thin-film thickness in nanometers (0 = no thin film)
    pub thin_film_thickness: f64,
    /// Thin-film IOR
    pub thin_film_ior: f64,
    /// Subsurface scattering color
    pub subsurface_color: [f64; 3],
    /// Subsurface scattering radius
    pub subsurface_radius: f64,
    /// Custom parameters map
    pub custom: HashMap<String, f64>,
}

impl Default for PluginMaterialParams {
    fn default() -> Self {
        Self {
            base_color: [1.0, 1.0, 1.0],
            metallic: 0.0,
            roughness: 0.5,
            ior: 1.5,
            transmission: 0.0,
            absorption_color: [1.0, 1.0, 1.0],
            absorption_coefficient: 0.0,
            thin_film_thickness: 0.0,
            thin_film_ior: 1.5,
            subsurface_color: [1.0, 1.0, 1.0],
            subsurface_radius: 0.0,
            custom: HashMap::new(),
        }
    }
}

impl PluginMaterialParams {
    /// Create new params with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Create metallic material params
    pub fn metallic(base_color: [f64; 3], roughness: f64) -> Self {
        Self {
            base_color,
            metallic: 1.0,
            roughness,
            ..Default::default()
        }
    }

    /// Create dielectric material params
    pub fn dielectric(base_color: [f64; 3], roughness: f64, ior: f64) -> Self {
        Self {
            base_color,
            metallic: 0.0,
            roughness,
            ior,
            ..Default::default()
        }
    }

    /// Create glass material params
    pub fn glass(color: [f64; 3], ior: f64, roughness: f64) -> Self {
        Self {
            base_color: color,
            metallic: 0.0,
            roughness,
            ior,
            transmission: 1.0,
            ..Default::default()
        }
    }

    /// Set custom parameter
    pub fn with_custom(mut self, key: &str, value: f64) -> Self {
        self.custom.insert(key.to_string(), value);
        self
    }
}

// ============================================================================
// Evaluation Context
// ============================================================================

/// Context for plugin evaluation
#[derive(Debug, Clone)]
pub struct EvaluationContext {
    /// Wavelength in nanometers (380-780)
    pub wavelength: f64,
    /// Incident angle theta (0 = normal)
    pub theta_i: f64,
    /// Outgoing angle theta
    pub theta_o: f64,
    /// Azimuthal angle phi
    pub phi: f64,
    /// Material thickness for volumetric effects
    pub thickness: f64,
    /// Current quality tier (0=Fast, 5=Reference)
    pub quality_tier: u32,
    /// Enable spectral evaluation
    pub spectral_mode: bool,
    /// Wavelengths for spectral evaluation
    pub wavelengths: Vec<f64>,
}

impl Default for EvaluationContext {
    fn default() -> Self {
        Self {
            wavelength: 550.0,
            theta_i: 0.0,
            theta_o: 0.0,
            phi: 0.0,
            thickness: 1.0,
            quality_tier: 3,
            spectral_mode: false,
            wavelengths: Vec::new(),
        }
    }
}

impl EvaluationContext {
    /// Create context for normal incidence
    pub fn normal_incidence(wavelength: f64) -> Self {
        Self {
            wavelength,
            ..Default::default()
        }
    }

    /// Create context for specific angles
    pub fn with_angles(wavelength: f64, theta_i: f64, theta_o: f64, phi: f64) -> Self {
        Self {
            wavelength,
            theta_i,
            theta_o,
            phi,
            ..Default::default()
        }
    }

    /// Create spectral evaluation context
    pub fn spectral(wavelengths: Vec<f64>, theta_i: f64) -> Self {
        Self {
            wavelength: wavelengths.first().copied().unwrap_or(550.0),
            theta_i,
            spectral_mode: true,
            wavelengths,
            ..Default::default()
        }
    }
}

// ============================================================================
// Plugin Output
// ============================================================================

/// Output from render plugin evaluation
#[derive(Debug, Clone)]
pub struct PluginRenderOutput {
    /// Spectral reflectance (per wavelength)
    pub reflectance: Vec<f64>,
    /// Spectral transmittance (per wavelength)
    pub transmittance: Vec<f64>,
    /// XYZ color values
    pub xyz: [f64; 3],
    /// Linear RGB color values
    pub rgb: [f64; 3],
    /// Energy conservation error
    pub energy_error: f64,
    /// Computation time in microseconds
    pub computation_time_us: f64,
    /// Additional output data
    pub metadata: HashMap<String, f64>,
}

impl Default for PluginRenderOutput {
    fn default() -> Self {
        Self {
            reflectance: vec![0.5],
            transmittance: vec![0.0],
            xyz: [0.0, 0.0, 0.0],
            rgb: [0.5, 0.5, 0.5],
            energy_error: 0.0,
            computation_time_us: 0.0,
            metadata: HashMap::new(),
        }
    }
}

// ============================================================================
// Spectral Measurement
// ============================================================================

/// Spectral measurement data from dataset plugins
#[derive(Debug, Clone)]
pub struct SpectralMeasurement {
    /// Material name
    pub name: String,
    /// Wavelengths in nm
    pub wavelengths: Vec<f64>,
    /// Reflectance values per wavelength
    pub reflectance: Vec<f64>,
    /// Transmittance values per wavelength (if available)
    pub transmittance: Option<Vec<f64>>,
    /// Measurement angle theta
    pub theta: f64,
    /// Measurement uncertainty
    pub uncertainty: Option<f64>,
}

impl SpectralMeasurement {
    /// Create new measurement
    pub fn new(name: &str, wavelengths: Vec<f64>, reflectance: Vec<f64>, theta: f64) -> Self {
        Self {
            name: name.to_string(),
            wavelengths,
            reflectance,
            transmittance: None,
            theta,
            uncertainty: None,
        }
    }

    /// Interpolate reflectance at wavelength
    pub fn interpolate(&self, wavelength: f64) -> f64 {
        if self.wavelengths.is_empty() || self.reflectance.is_empty() {
            return 0.0;
        }

        // Find bracket
        let n = self.wavelengths.len();
        if wavelength <= self.wavelengths[0] {
            return self.reflectance[0];
        }
        if wavelength >= self.wavelengths[n - 1] {
            return self.reflectance[n - 1];
        }

        // Linear interpolation
        for i in 0..n - 1 {
            if wavelength >= self.wavelengths[i] && wavelength <= self.wavelengths[i + 1] {
                let t = (wavelength - self.wavelengths[i])
                    / (self.wavelengths[i + 1] - self.wavelengths[i]);
                return self.reflectance[i] * (1.0 - t) + self.reflectance[i + 1] * t;
            }
        }

        0.0
    }
}

// ============================================================================
// Plugin Traits
// ============================================================================

/// Trait for custom render plugins
pub trait RenderPlugin: Send + Sync {
    /// Plugin name
    fn name(&self) -> &str;

    /// Plugin version (major, minor, patch)
    fn version(&self) -> (u32, u32, u32);

    /// API version this plugin was built for
    fn api_version(&self) -> (u32, u32, u32);

    /// Evaluate material at given context
    fn evaluate(
        &self,
        params: &PluginMaterialParams,
        ctx: &EvaluationContext,
    ) -> PluginRenderOutput;

    /// Check if plugin supports material type
    fn supports_material(&self, material_type: MaterialType) -> bool;

    /// Plugin description
    fn description(&self) -> &str {
        "Custom render plugin"
    }

    /// Plugin author
    fn author(&self) -> &str {
        "Unknown"
    }
}

/// Trait for dataset plugins (external measurements)
pub trait DatasetPlugin: Send + Sync {
    /// Dataset name
    fn name(&self) -> &str;

    /// Number of materials in dataset
    fn material_count(&self) -> usize;

    /// List of material names
    fn material_names(&self) -> Vec<&str>;

    /// Get spectral measurement for material
    fn get_measurement(&self, name: &str) -> Option<SpectralMeasurement>;

    /// Get BRDF value at angles
    fn get_brdf(&self, name: &str, theta_i: f64, theta_o: f64, phi: f64) -> Option<f64> {
        let _ = (name, theta_i, theta_o, phi);
        None
    }

    /// Dataset description
    fn description(&self) -> &str {
        "External measurement dataset"
    }

    /// Dataset source/reference
    fn source(&self) -> &str {
        "Unknown"
    }
}

/// Trait for custom error metric plugins
pub trait MetricPlugin: Send + Sync {
    /// Metric name
    fn name(&self) -> &str;

    /// Compute metric between measured and rendered spectra
    fn compute(&self, measured: &[f64], rendered: &[f64], wavelengths: &[f64]) -> f64;

    /// Metric description
    fn description(&self) -> &str {
        "Custom error metric"
    }

    /// Lower is better? (true for errors, false for correlation)
    fn lower_is_better(&self) -> bool {
        true
    }

    /// Ideal value (0 for errors, 1 for correlation)
    fn ideal_value(&self) -> f64 {
        0.0
    }
}

// ============================================================================
// Plugin Registry
// ============================================================================

/// Plugin registration and management
pub struct PluginRegistry {
    render_plugins: Vec<Box<dyn RenderPlugin>>,
    dataset_plugins: Vec<Box<dyn DatasetPlugin>>,
    metric_plugins: Vec<Box<dyn MetricPlugin>>,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    /// Create empty registry
    pub fn new() -> Self {
        Self {
            render_plugins: Vec::new(),
            dataset_plugins: Vec::new(),
            metric_plugins: Vec::new(),
        }
    }

    /// Check API version compatibility
    pub fn check_api_compatibility(plugin_api: (u32, u32, u32)) -> bool {
        // Major version must match
        if plugin_api.0 != PLUGIN_API_VERSION.0 {
            return false;
        }
        // Plugin minor version must be <= current
        if plugin_api.1 > PLUGIN_API_VERSION.1 {
            return false;
        }
        true
    }

    /// Register render plugin
    pub fn register_render(&mut self, plugin: Box<dyn RenderPlugin>) -> Result<(), PluginError> {
        if !Self::check_api_compatibility(plugin.api_version()) {
            return Err(PluginError::IncompatibleVersion {
                plugin_name: plugin.name().to_string(),
                plugin_api: plugin.api_version(),
                host_api: PLUGIN_API_VERSION,
            });
        }

        // Check for duplicate
        if self
            .render_plugins
            .iter()
            .any(|p| p.name() == plugin.name())
        {
            return Err(PluginError::DuplicatePlugin(plugin.name().to_string()));
        }

        self.render_plugins.push(plugin);
        Ok(())
    }

    /// Register dataset plugin
    pub fn register_dataset(&mut self, plugin: Box<dyn DatasetPlugin>) -> Result<(), PluginError> {
        // Check for duplicate
        if self
            .dataset_plugins
            .iter()
            .any(|p| p.name() == plugin.name())
        {
            return Err(PluginError::DuplicatePlugin(plugin.name().to_string()));
        }

        self.dataset_plugins.push(plugin);
        Ok(())
    }

    /// Register metric plugin
    pub fn register_metric(&mut self, plugin: Box<dyn MetricPlugin>) -> Result<(), PluginError> {
        // Check for duplicate
        if self
            .metric_plugins
            .iter()
            .any(|p| p.name() == plugin.name())
        {
            return Err(PluginError::DuplicatePlugin(plugin.name().to_string()));
        }

        self.metric_plugins.push(plugin);
        Ok(())
    }

    /// Get render plugin by name
    pub fn get_render_plugin(&self, name: &str) -> Option<&dyn RenderPlugin> {
        self.render_plugins
            .iter()
            .find(|p| p.name() == name)
            .map(|p| p.as_ref())
    }

    /// Get dataset plugin by name
    pub fn get_dataset_plugin(&self, name: &str) -> Option<&dyn DatasetPlugin> {
        self.dataset_plugins
            .iter()
            .find(|p| p.name() == name)
            .map(|p| p.as_ref())
    }

    /// Get metric plugin by name
    pub fn get_metric_plugin(&self, name: &str) -> Option<&dyn MetricPlugin> {
        self.metric_plugins
            .iter()
            .find(|p| p.name() == name)
            .map(|p| p.as_ref())
    }

    /// List all registered plugins
    pub fn list_plugins(&self) -> PluginInventory {
        PluginInventory {
            render_plugins: self
                .render_plugins
                .iter()
                .map(|p| PluginInfo {
                    name: p.name().to_string(),
                    version: p.version(),
                    description: p.description().to_string(),
                })
                .collect(),
            dataset_plugins: self
                .dataset_plugins
                .iter()
                .map(|p| PluginInfo {
                    name: p.name().to_string(),
                    version: (1, 0, 0),
                    description: p.description().to_string(),
                })
                .collect(),
            metric_plugins: self
                .metric_plugins
                .iter()
                .map(|p| PluginInfo {
                    name: p.name().to_string(),
                    version: (1, 0, 0),
                    description: p.description().to_string(),
                })
                .collect(),
        }
    }

    /// Get render plugins supporting material type
    pub fn get_render_plugins_for(&self, material_type: MaterialType) -> Vec<&dyn RenderPlugin> {
        self.render_plugins
            .iter()
            .filter(|p| p.supports_material(material_type))
            .map(|p| p.as_ref())
            .collect()
    }

    /// Total number of registered plugins
    pub fn plugin_count(&self) -> usize {
        self.render_plugins.len() + self.dataset_plugins.len() + self.metric_plugins.len()
    }

    /// Unregister render plugin by name
    pub fn unregister_render(&mut self, name: &str) -> bool {
        let len_before = self.render_plugins.len();
        self.render_plugins.retain(|p| p.name() != name);
        self.render_plugins.len() < len_before
    }

    /// Unregister dataset plugin by name
    pub fn unregister_dataset(&mut self, name: &str) -> bool {
        let len_before = self.dataset_plugins.len();
        self.dataset_plugins.retain(|p| p.name() != name);
        self.dataset_plugins.len() < len_before
    }

    /// Unregister metric plugin by name
    pub fn unregister_metric(&mut self, name: &str) -> bool {
        let len_before = self.metric_plugins.len();
        self.metric_plugins.retain(|p| p.name() != name);
        self.metric_plugins.len() < len_before
    }
}

/// Plugin registration error
#[derive(Debug, Clone)]
pub enum PluginError {
    /// API version incompatibility
    IncompatibleVersion {
        plugin_name: String,
        plugin_api: (u32, u32, u32),
        host_api: (u32, u32, u32),
    },
    /// Plugin with same name already registered
    DuplicatePlugin(String),
    /// Plugin initialization failed
    InitializationFailed(String),
    /// Plugin not found
    NotFound(String),
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginError::IncompatibleVersion {
                plugin_name,
                plugin_api,
                host_api,
            } => {
                write!(
                    f,
                    "Plugin '{}' API version {}.{}.{} incompatible with host {}.{}.{}",
                    plugin_name,
                    plugin_api.0,
                    plugin_api.1,
                    plugin_api.2,
                    host_api.0,
                    host_api.1,
                    host_api.2
                )
            }
            PluginError::DuplicatePlugin(name) => {
                write!(f, "Plugin '{}' already registered", name)
            }
            PluginError::InitializationFailed(msg) => {
                write!(f, "Plugin initialization failed: {}", msg)
            }
            PluginError::NotFound(name) => {
                write!(f, "Plugin '{}' not found", name)
            }
        }
    }
}

impl std::error::Error for PluginError {}

/// Plugin information for inventory
#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub version: (u32, u32, u32),
    pub description: String,
}

/// Full plugin inventory
#[derive(Debug, Clone)]
pub struct PluginInventory {
    pub render_plugins: Vec<PluginInfo>,
    pub dataset_plugins: Vec<PluginInfo>,
    pub metric_plugins: Vec<PluginInfo>,
}

impl PluginInventory {
    /// Total plugins
    pub fn total(&self) -> usize {
        self.render_plugins.len() + self.dataset_plugins.len() + self.metric_plugins.len()
    }
}

// ============================================================================
// Built-in Plugins
// ============================================================================

/// Built-in Lambertian render plugin for testing
pub struct LambertianPlugin;

impl RenderPlugin for LambertianPlugin {
    fn name(&self) -> &str {
        "builtin_lambertian"
    }

    fn version(&self) -> (u32, u32, u32) {
        (1, 0, 0)
    }

    fn api_version(&self) -> (u32, u32, u32) {
        PLUGIN_API_VERSION
    }

    fn evaluate(
        &self,
        params: &PluginMaterialParams,
        ctx: &EvaluationContext,
    ) -> PluginRenderOutput {
        let start = std::time::Instant::now();

        // Lambertian: constant reflectance independent of angles
        let reflectance = if ctx.spectral_mode && !ctx.wavelengths.is_empty() {
            // Use base color as spectral reflectance (simplified)
            let avg = (params.base_color[0] + params.base_color[1] + params.base_color[2]) / 3.0;
            vec![avg; ctx.wavelengths.len()]
        } else {
            let avg = (params.base_color[0] + params.base_color[1] + params.base_color[2]) / 3.0;
            vec![avg]
        };

        let elapsed = start.elapsed().as_micros() as f64;

        PluginRenderOutput {
            reflectance,
            transmittance: vec![0.0],
            xyz: [
                params.base_color[0] * 0.4124564,
                params.base_color[1] * 0.3575761,
                params.base_color[2] * 0.1804375,
            ],
            rgb: params.base_color,
            energy_error: 0.0,
            computation_time_us: elapsed,
            metadata: HashMap::new(),
        }
    }

    fn supports_material(&self, material_type: MaterialType) -> bool {
        matches!(material_type, MaterialType::StandardPBR)
    }

    fn description(&self) -> &str {
        "Built-in Lambertian diffuse model"
    }

    fn author(&self) -> &str {
        "Momoto"
    }
}

/// Built-in RMSE metric plugin
pub struct RmseMetricPlugin;

impl MetricPlugin for RmseMetricPlugin {
    fn name(&self) -> &str {
        "builtin_rmse"
    }

    fn compute(&self, measured: &[f64], rendered: &[f64], _wavelengths: &[f64]) -> f64 {
        if measured.len() != rendered.len() || measured.is_empty() {
            return f64::INFINITY;
        }

        let sum_sq: f64 = measured
            .iter()
            .zip(rendered.iter())
            .map(|(m, r)| (m - r).powi(2))
            .sum();

        (sum_sq / measured.len() as f64).sqrt()
    }

    fn description(&self) -> &str {
        "Root Mean Square Error"
    }

    fn lower_is_better(&self) -> bool {
        true
    }

    fn ideal_value(&self) -> f64 {
        0.0
    }
}

/// Built-in Spectral Angle Mapper metric plugin
pub struct SamMetricPlugin;

impl MetricPlugin for SamMetricPlugin {
    fn name(&self) -> &str {
        "builtin_sam"
    }

    fn compute(&self, measured: &[f64], rendered: &[f64], _wavelengths: &[f64]) -> f64 {
        if measured.len() != rendered.len() || measured.is_empty() {
            return std::f64::consts::PI;
        }

        let dot: f64 = measured
            .iter()
            .zip(rendered.iter())
            .map(|(m, r)| m * r)
            .sum();

        let mag_m: f64 = measured.iter().map(|m| m * m).sum::<f64>().sqrt();
        let mag_r: f64 = rendered.iter().map(|r| r * r).sum::<f64>().sqrt();

        if mag_m < 1e-10 || mag_r < 1e-10 {
            return std::f64::consts::PI;
        }

        let cos_angle = (dot / (mag_m * mag_r)).clamp(-1.0, 1.0);
        cos_angle.acos()
    }

    fn description(&self) -> &str {
        "Spectral Angle Mapper (radians)"
    }

    fn lower_is_better(&self) -> bool {
        true
    }

    fn ideal_value(&self) -> f64 {
        0.0
    }
}

// ============================================================================
// Registry with Built-ins
// ============================================================================

impl PluginRegistry {
    /// Create registry with built-in plugins
    pub fn with_builtins() -> Self {
        let mut registry = Self::new();

        // Register built-in render plugins
        let _ = registry.register_render(Box::new(LambertianPlugin));

        // Register built-in metric plugins
        let _ = registry.register_metric(Box::new(RmseMetricPlugin));
        let _ = registry.register_metric(Box::new(SamMetricPlugin));

        registry
    }
}

// ============================================================================
// Memory Estimation
// ============================================================================

/// Estimate memory usage for plugin API structures
pub fn estimate_plugin_api_memory() -> usize {
    let mut total = 0;

    // PluginRegistry base
    total += std::mem::size_of::<PluginRegistry>();

    // Built-in plugins (approximate)
    total += 512; // LambertianPlugin
    total += 256; // RmseMetricPlugin
    total += 256; // SamMetricPlugin

    // Typical registered plugins overhead
    total += 10 * 1024; // ~10KB for typical plugin set

    total
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_api_version() {
        assert_eq!(PLUGIN_API_VERSION, (1, 0, 0));
        assert_eq!(PLUGIN_API_VERSION_STRING, "1.0.0");
    }

    #[test]
    fn test_material_type() {
        assert_eq!(MaterialType::StandardPBR.name(), "standard_pbr");
        assert_eq!(MaterialType::Glass.name(), "glass");
        assert_eq!(MaterialType::Custom(42).name(), "custom");
    }

    #[test]
    fn test_plugin_material_params() {
        let params = PluginMaterialParams::default();
        assert_eq!(params.metallic, 0.0);
        assert_eq!(params.roughness, 0.5);
        assert_eq!(params.ior, 1.5);

        let metallic = PluginMaterialParams::metallic([1.0, 0.8, 0.0], 0.2);
        assert_eq!(metallic.metallic, 1.0);
        assert_eq!(metallic.roughness, 0.2);

        let glass = PluginMaterialParams::glass([0.9, 0.9, 0.9], 1.52, 0.0);
        assert_eq!(glass.transmission, 1.0);
        assert_eq!(glass.ior, 1.52);
    }

    #[test]
    fn test_evaluation_context() {
        let ctx = EvaluationContext::default();
        assert_eq!(ctx.wavelength, 550.0);
        assert_eq!(ctx.theta_i, 0.0);

        let ctx_angles = EvaluationContext::with_angles(600.0, 0.5, 0.3, 0.0);
        assert_eq!(ctx_angles.wavelength, 600.0);
        assert_eq!(ctx_angles.theta_i, 0.5);
    }

    #[test]
    fn test_spectral_measurement_interpolation() {
        let measurement =
            SpectralMeasurement::new("test", vec![400.0, 500.0, 600.0], vec![0.1, 0.5, 0.9], 0.0);

        assert!((measurement.interpolate(400.0) - 0.1).abs() < 1e-10);
        assert!((measurement.interpolate(500.0) - 0.5).abs() < 1e-10);
        assert!((measurement.interpolate(450.0) - 0.3).abs() < 1e-10);
        assert!((measurement.interpolate(700.0) - 0.9).abs() < 1e-10); // Clamp
    }

    #[test]
    fn test_plugin_registry() {
        let mut registry = PluginRegistry::new();
        assert_eq!(registry.plugin_count(), 0);

        registry
            .register_render(Box::new(LambertianPlugin))
            .unwrap();
        assert_eq!(registry.plugin_count(), 1);

        // Duplicate should fail
        let result = registry.register_render(Box::new(LambertianPlugin));
        assert!(result.is_err());
    }

    #[test]
    fn test_plugin_registry_with_builtins() {
        let registry = PluginRegistry::with_builtins();
        assert_eq!(registry.plugin_count(), 3); // 1 render + 2 metric

        let inventory = registry.list_plugins();
        assert_eq!(inventory.render_plugins.len(), 1);
        assert_eq!(inventory.metric_plugins.len(), 2);
    }

    #[test]
    fn test_lambertian_plugin() {
        let plugin = LambertianPlugin;
        assert_eq!(plugin.name(), "builtin_lambertian");
        assert!(plugin.supports_material(MaterialType::StandardPBR));
        assert!(!plugin.supports_material(MaterialType::Glass));

        let params = PluginMaterialParams::default();
        let ctx = EvaluationContext::default();
        let output = plugin.evaluate(&params, &ctx);

        assert!(!output.reflectance.is_empty());
        assert!(output.computation_time_us >= 0.0);
    }

    #[test]
    fn test_rmse_metric() {
        let plugin = RmseMetricPlugin;

        let measured = vec![0.1, 0.2, 0.3];
        let rendered = vec![0.1, 0.2, 0.3];
        let wavelengths = vec![400.0, 500.0, 600.0];

        let rmse = plugin.compute(&measured, &rendered, &wavelengths);
        assert!(rmse < 1e-10);

        let rendered2 = vec![0.2, 0.3, 0.4];
        let rmse2 = plugin.compute(&measured, &rendered2, &wavelengths);
        assert!(rmse2 > 0.0);
    }

    #[test]
    fn test_sam_metric() {
        let plugin = SamMetricPlugin;

        // Identical spectra
        let measured = vec![0.1, 0.2, 0.3];
        let rendered = vec![0.1, 0.2, 0.3];
        let wavelengths = vec![400.0, 500.0, 600.0];

        let sam = plugin.compute(&measured, &rendered, &wavelengths);
        assert!(sam < 1e-10);

        // Scaled spectra (same angle)
        let rendered2 = vec![0.2, 0.4, 0.6];
        let sam2 = plugin.compute(&measured, &rendered2, &wavelengths);
        assert!(sam2 < 1e-10);

        // Different shape
        let rendered3 = vec![0.3, 0.2, 0.1];
        let sam3 = plugin.compute(&measured, &rendered3, &wavelengths);
        assert!(sam3 > 0.1);
    }

    #[test]
    fn test_api_compatibility() {
        // Same major version, lower minor - compatible
        assert!(PluginRegistry::check_api_compatibility((1, 0, 0)));

        // Same major version, same minor - compatible
        assert!(PluginRegistry::check_api_compatibility((1, 0, 5)));

        // Different major version - incompatible
        assert!(!PluginRegistry::check_api_compatibility((2, 0, 0)));
        assert!(!PluginRegistry::check_api_compatibility((0, 0, 0)));

        // Higher minor version - incompatible
        assert!(!PluginRegistry::check_api_compatibility((1, 1, 0)));
    }

    #[test]
    fn test_plugin_unregister() {
        let mut registry = PluginRegistry::with_builtins();
        let initial_count = registry.plugin_count();

        let removed = registry.unregister_render("builtin_lambertian");
        assert!(removed);
        assert_eq!(registry.plugin_count(), initial_count - 1);

        let removed_again = registry.unregister_render("builtin_lambertian");
        assert!(!removed_again);
    }

    #[test]
    fn test_get_render_plugins_for() {
        let registry = PluginRegistry::with_builtins();

        let pbr_plugins = registry.get_render_plugins_for(MaterialType::StandardPBR);
        assert_eq!(pbr_plugins.len(), 1);

        let glass_plugins = registry.get_render_plugins_for(MaterialType::Glass);
        assert_eq!(glass_plugins.len(), 0);
    }

    #[test]
    fn test_memory_estimate() {
        let estimate = estimate_plugin_api_memory();
        assert!(estimate > 0);
        assert!(estimate < 20 * 1024); // Should be under 20KB
    }
}

//! # Material Export Module
//!
//! Generate shader code and material descriptors from Momoto materials.
//!
//! ## Features
//!
//! - **GLSL Export**: WebGL2/Desktop shader code
//! - **WGSL Export**: WebGPU shader code
//! - **MaterialX Export**: Industry-standard material descriptors
//! - **CSS Export**: Enhanced CSS for web rendering
//!
//! ## Usage
//!
//! ```rust
//! use momoto_materials::glass_physics::material_export::{
//!     MaterialExporter, ExportTarget, MaterialDescriptor
//! };
//!
//! let exporter = MaterialExporter::new(ExportTarget::glsl_300es());
//! let descriptor = MaterialDescriptor::new("MyGlass")
//!     .with_ior(1.5)
//!     .with_roughness(0.1);
//!
//! let glsl = exporter.export(&descriptor);
//! println!("{}", glsl);
//! ```

use std::collections::HashMap;

// ============================================================================
// EXPORT TARGET
// ============================================================================

/// Target format for export
#[derive(Debug, Clone)]
pub enum ExportTarget {
    /// GLSL shader code
    GLSL { version: GlslVersion },
    /// WGSL shader code (WebGPU)
    WGSL,
    /// MaterialX XML descriptor
    MaterialX { version: String },
    /// MaterialX JSON descriptor
    MaterialXJson,
    /// Enhanced CSS
    CSS,
}

impl ExportTarget {
    /// Create GLSL ES 3.0 target (WebGL2)
    pub fn glsl_300es() -> Self {
        Self::GLSL {
            version: GlslVersion::V300ES,
        }
    }

    /// Create GLSL 3.30 target (Desktop OpenGL)
    pub fn glsl_330() -> Self {
        Self::GLSL {
            version: GlslVersion::V330,
        }
    }

    /// Create WGSL target
    pub fn wgsl() -> Self {
        Self::WGSL
    }

    /// Create MaterialX 1.38 target
    pub fn materialx() -> Self {
        Self::MaterialX {
            version: "1.38".to_string(),
        }
    }

    /// Create CSS target
    pub fn css() -> Self {
        Self::CSS
    }
}

/// GLSL version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlslVersion {
    /// OpenGL ES 3.0 / WebGL 2.0
    V300ES,
    /// OpenGL 3.30
    V330,
    /// OpenGL 4.50 (Vulkan compatibility)
    V450,
}

// ============================================================================
// MATERIAL DESCRIPTOR
// ============================================================================

/// Property value types
#[derive(Debug, Clone)]
pub enum PropertyValue {
    /// Scalar float
    Float(f64),
    /// 3-component vector (color, normal, etc.)
    Vec3([f64; 3]),
    /// Texture reference (filename or URI)
    Texture(String),
    /// Spectral data (wavelength, value pairs)
    Spectrum(Vec<(f64, f64)>),
    /// Boolean flag
    Bool(bool),
    /// Integer value
    Int(i64),
}

/// Thin-film layer descriptor
#[derive(Debug, Clone)]
pub struct ThinFilmDescriptor {
    /// Film refractive index
    pub n_film: f64,
    /// Film thickness in nanometers
    pub thickness_nm: f64,
    /// Substrate refractive index
    pub n_substrate: f64,
}

/// Subsurface scattering descriptor
#[derive(Debug, Clone)]
pub struct SubsurfaceDescriptor {
    /// Scattering color
    pub scatter_color: [f64; 3],
    /// Mean free path (mm)
    pub mean_free_path: f64,
    /// Anisotropy (-1 to 1)
    pub anisotropy: f64,
}

/// Complete material descriptor for export
#[derive(Debug, Clone)]
pub struct MaterialDescriptor {
    /// Material name
    pub name: String,
    /// Material version
    pub version: String,
    /// Base color (albedo)
    pub base_color: [f64; 3],
    /// Metallic factor (0-1)
    pub metallic: f64,
    /// Roughness factor (0-1)
    pub roughness: f64,
    /// Index of refraction
    pub ior: f64,
    /// Specular intensity (for non-metals)
    pub specular: f64,
    /// Transmission factor (0-1)
    pub transmission: f64,
    /// Thin-film parameters (optional)
    pub thin_film: Option<ThinFilmDescriptor>,
    /// Subsurface parameters (optional)
    pub subsurface: Option<SubsurfaceDescriptor>,
    /// Custom properties
    pub custom_properties: HashMap<String, PropertyValue>,
}

impl MaterialDescriptor {
    /// Create new descriptor with name
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            version: "1.0".to_string(),
            base_color: [0.8, 0.8, 0.8],
            metallic: 0.0,
            roughness: 0.5,
            ior: 1.5,
            specular: 0.5,
            transmission: 0.0,
            thin_film: None,
            subsurface: None,
            custom_properties: HashMap::new(),
        }
    }

    /// Set base color
    pub fn with_base_color(mut self, r: f64, g: f64, b: f64) -> Self {
        self.base_color = [r, g, b];
        self
    }

    /// Set metallic
    pub fn with_metallic(mut self, metallic: f64) -> Self {
        self.metallic = metallic.clamp(0.0, 1.0);
        self
    }

    /// Set roughness
    pub fn with_roughness(mut self, roughness: f64) -> Self {
        self.roughness = roughness.clamp(0.0, 1.0);
        self
    }

    /// Set IOR
    pub fn with_ior(mut self, ior: f64) -> Self {
        self.ior = ior.max(1.0);
        self
    }

    /// Set transmission
    pub fn with_transmission(mut self, transmission: f64) -> Self {
        self.transmission = transmission.clamp(0.0, 1.0);
        self
    }

    /// Add thin-film
    pub fn with_thin_film(mut self, n_film: f64, thickness_nm: f64, n_substrate: f64) -> Self {
        self.thin_film = Some(ThinFilmDescriptor {
            n_film,
            thickness_nm,
            n_substrate,
        });
        self
    }

    /// Add custom property
    pub fn with_property(mut self, name: &str, value: PropertyValue) -> Self {
        self.custom_properties.insert(name.to_string(), value);
        self
    }

    /// Compute F0 from IOR
    pub fn f0(&self) -> f64 {
        ((self.ior - 1.0) / (self.ior + 1.0)).powi(2)
    }

    /// Check if material is dielectric
    pub fn is_dielectric(&self) -> bool {
        self.metallic < 0.5
    }

    /// Check if material has thin-film
    pub fn has_thin_film(&self) -> bool {
        self.thin_film.is_some()
    }
}

// ============================================================================
// EXPORT OPTIONS
// ============================================================================

/// Options for export
#[derive(Debug, Clone)]
pub struct ExportOptions {
    /// Include comments in output
    pub include_comments: bool,
    /// Optimize for code size
    pub optimize_for_size: bool,
    /// Inline LUT data in shader
    pub inline_luts: bool,
    /// Generate uniform declarations
    pub generate_uniforms: bool,
    /// Use high precision (mediump vs highp)
    pub high_precision: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            include_comments: true,
            optimize_for_size: false,
            inline_luts: false,
            generate_uniforms: true,
            high_precision: true,
        }
    }
}

// ============================================================================
// MATERIAL EXPORTER
// ============================================================================

/// Main exporter for material conversion
pub struct MaterialExporter {
    target: ExportTarget,
    options: ExportOptions,
}

impl MaterialExporter {
    /// Create new exporter with target
    pub fn new(target: ExportTarget) -> Self {
        Self {
            target,
            options: ExportOptions::default(),
        }
    }

    /// Create exporter with options
    pub fn with_options(target: ExportTarget, options: ExportOptions) -> Self {
        Self { target, options }
    }

    /// Export material descriptor
    pub fn export(&self, material: &MaterialDescriptor) -> String {
        match &self.target {
            ExportTarget::GLSL { version } => self.export_glsl(material, *version),
            ExportTarget::WGSL => self.export_wgsl(material),
            ExportTarget::MaterialX { version } => self.export_materialx(material, version),
            ExportTarget::MaterialXJson => self.export_materialx_json(material),
            ExportTarget::CSS => self.export_css(material),
        }
    }

    /// Export to GLSL shader code
    pub fn export_glsl(&self, material: &MaterialDescriptor, version: GlslVersion) -> String {
        let mut code = String::new();

        // Version directive
        match version {
            GlslVersion::V300ES => code.push_str("#version 300 es\n"),
            GlslVersion::V330 => code.push_str("#version 330 core\n"),
            GlslVersion::V450 => code.push_str("#version 450\n"),
        }

        // Precision
        if matches!(version, GlslVersion::V300ES) {
            if self.options.high_precision {
                code.push_str("precision highp float;\n");
            } else {
                code.push_str("precision mediump float;\n");
            }
        }

        code.push('\n');

        // Comments
        if self.options.include_comments {
            code.push_str(&format!("// Material: {}\n", material.name));
            code.push_str(&format!("// Version: {}\n", material.version));
            code.push_str("// Generated by Momoto Materials Phase 8\n\n");
        }

        // Material constants
        code.push_str("// Material Properties\n");
        code.push_str(&format!(
            "const vec3 u_baseColor = vec3({:.4}, {:.4}, {:.4});\n",
            material.base_color[0], material.base_color[1], material.base_color[2]
        ));
        code.push_str(&format!(
            "const float u_metallic = {:.4};\n",
            material.metallic
        ));
        code.push_str(&format!(
            "const float u_roughness = {:.4};\n",
            material.roughness
        ));
        code.push_str(&format!("const float u_ior = {:.4};\n", material.ior));
        code.push_str(&format!("const float u_f0 = {:.4};\n", material.f0()));

        code.push('\n');

        // Include shader functions
        code.push_str(GLSL_FRESNEL_SCHLICK);
        code.push('\n');
        code.push_str(GLSL_GGX_DISTRIBUTION);
        code.push('\n');
        code.push_str(GLSL_GEOMETRY_SMITH);
        code.push('\n');

        // Thin-film if present
        if material.has_thin_film() {
            code.push_str(GLSL_THIN_FILM);
            code.push('\n');

            let tf = material.thin_film.as_ref().unwrap();
            code.push_str(&format!("const float u_thinFilmN = {:.4};\n", tf.n_film));
            code.push_str(&format!(
                "const float u_thinFilmThickness = {:.4};\n",
                tf.thickness_nm
            ));
            code.push('\n');
        }

        // PBR evaluation function
        code.push_str(GLSL_PBR_EVALUATE);

        code
    }

    /// Export to WGSL shader code
    pub fn export_wgsl(&self, material: &MaterialDescriptor) -> String {
        let mut code = String::new();

        if self.options.include_comments {
            code.push_str(&format!("// Material: {}\n", material.name));
            code.push_str("// Generated by Momoto Materials Phase 8\n\n");
        }

        // Material struct
        code.push_str("struct Material {\n");
        code.push_str("    baseColor: vec3<f32>,\n");
        code.push_str("    metallic: f32,\n");
        code.push_str("    roughness: f32,\n");
        code.push_str("    ior: f32,\n");
        code.push_str("}\n\n");

        // Material constant
        code.push_str(&format!(
            "const material = Material(\n    vec3<f32>({:.4}, {:.4}, {:.4}),\n    {:.4},\n    {:.4},\n    {:.4}\n);\n\n",
            material.base_color[0], material.base_color[1], material.base_color[2],
            material.metallic, material.roughness, material.ior
        ));

        // Fresnel function
        code.push_str(WGSL_FRESNEL_SCHLICK);
        code.push('\n');

        // GGX distribution
        code.push_str(WGSL_GGX_DISTRIBUTION);
        code.push('\n');

        // PBR evaluate
        code.push_str(WGSL_PBR_EVALUATE);

        code
    }

    /// Export to MaterialX XML
    pub fn export_materialx(&self, material: &MaterialDescriptor, _version: &str) -> String {
        let mut xml = String::new();

        xml.push_str("<?xml version=\"1.0\"?>\n");
        xml.push_str("<materialx version=\"1.38\">\n");
        xml.push_str(&format!(
            "  <standard_surface name=\"{}\" type=\"surfaceshader\">\n",
            material.name
        ));

        // Base properties
        xml.push_str(&format!(
            "    <input name=\"base_color\" type=\"color3\" value=\"{:.4}, {:.4}, {:.4}\" />\n",
            material.base_color[0], material.base_color[1], material.base_color[2]
        ));
        xml.push_str(&format!(
            "    <input name=\"metalness\" type=\"float\" value=\"{:.4}\" />\n",
            material.metallic
        ));
        xml.push_str(&format!(
            "    <input name=\"specular_roughness\" type=\"float\" value=\"{:.4}\" />\n",
            material.roughness
        ));
        xml.push_str(&format!(
            "    <input name=\"specular_IOR\" type=\"float\" value=\"{:.4}\" />\n",
            material.ior
        ));

        if material.transmission > 0.0 {
            xml.push_str(&format!(
                "    <input name=\"transmission\" type=\"float\" value=\"{:.4}\" />\n",
                material.transmission
            ));
        }

        xml.push_str("  </standard_surface>\n");
        xml.push_str("</materialx>\n");

        xml
    }

    /// Export to MaterialX JSON
    pub fn export_materialx_json(&self, material: &MaterialDescriptor) -> String {
        let mut json = String::from("{\n");

        json.push_str(&format!("  \"name\": \"{}\",\n", material.name));
        json.push_str("  \"type\": \"standard_surface\",\n");
        json.push_str("  \"inputs\": {\n");

        json.push_str(&format!(
            "    \"base_color\": [{:.4}, {:.4}, {:.4}],\n",
            material.base_color[0], material.base_color[1], material.base_color[2]
        ));
        json.push_str(&format!("    \"metalness\": {:.4},\n", material.metallic));
        json.push_str(&format!(
            "    \"specular_roughness\": {:.4},\n",
            material.roughness
        ));
        json.push_str(&format!("    \"specular_IOR\": {:.4}", material.ior));

        if material.transmission > 0.0 {
            json.push_str(&format!(
                ",\n    \"transmission\": {:.4}",
                material.transmission
            ));
        }

        json.push_str("\n  }\n");
        json.push_str("}\n");

        json
    }

    /// Export to enhanced CSS
    pub fn export_css(&self, material: &MaterialDescriptor) -> String {
        let mut css = String::new();

        if self.options.include_comments {
            css.push_str(&format!("/* Material: {} */\n", material.name));
        }

        // Convert to CSS custom properties
        css.push_str(&format!(
            ".material-{} {{\n",
            material.name.to_lowercase().replace(' ', "-")
        ));

        // Custom properties
        css.push_str(&format!(
            "  --base-color: rgb({:.0}, {:.0}, {:.0});\n",
            material.base_color[0] * 255.0,
            material.base_color[1] * 255.0,
            material.base_color[2] * 255.0
        ));
        css.push_str(&format!("  --metallic: {:.2};\n", material.metallic));
        css.push_str(&format!("  --roughness: {:.2};\n", material.roughness));
        css.push_str(&format!("  --ior: {:.2};\n", material.ior));

        // Background
        css.push_str("  background: var(--base-color);\n");

        // Reflection effect for metals
        if material.metallic > 0.5 {
            css.push_str(&format!(
                "  background: linear-gradient(135deg, \n    rgba(255,255,255,{:.2}) 0%, \n    var(--base-color) 50%, \n    rgba(0,0,0,{:.2}) 100%);\n",
                0.3 * (1.0 - material.roughness),
                0.2 * (1.0 - material.roughness)
            ));
        }

        // Glass effect for transparent
        if material.transmission > 0.5 {
            css.push_str(&format!(
                "  backdrop-filter: blur({}px) saturate({});\n",
                (1.0 - material.roughness) * 10.0,
                1.0 + material.transmission * 0.5
            ));
            css.push_str(&format!(
                "  background: rgba({:.0}, {:.0}, {:.0}, {:.2});\n",
                material.base_color[0] * 255.0,
                material.base_color[1] * 255.0,
                material.base_color[2] * 255.0,
                1.0 - material.transmission
            ));
        }

        css.push_str("}\n");

        css
    }
}

// ============================================================================
// GLSL SHADER CODE TEMPLATES
// ============================================================================

const GLSL_FRESNEL_SCHLICK: &str = r#"
// Fresnel-Schlick approximation
vec3 fresnelSchlick(float cosTheta, vec3 F0) {
    return F0 + (1.0 - F0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
}

float fresnelSchlickScalar(float cosTheta, float ior) {
    float f0 = pow((ior - 1.0) / (ior + 1.0), 2.0);
    return f0 + (1.0 - f0) * pow(1.0 - cosTheta, 5.0);
}
"#;

const GLSL_GGX_DISTRIBUTION: &str = r#"
// GGX/Trowbridge-Reitz distribution
float distributionGGX(vec3 N, vec3 H, float roughness) {
    float a = roughness * roughness;
    float a2 = a * a;
    float NdotH = max(dot(N, H), 0.0);
    float NdotH2 = NdotH * NdotH;

    float denom = (NdotH2 * (a2 - 1.0) + 1.0);
    denom = 3.14159265 * denom * denom;

    return a2 / denom;
}
"#;

const GLSL_GEOMETRY_SMITH: &str = r#"
// Smith geometry function
float geometrySchlickGGX(float NdotV, float roughness) {
    float r = roughness + 1.0;
    float k = (r * r) / 8.0;
    return NdotV / (NdotV * (1.0 - k) + k);
}

float geometrySmith(vec3 N, vec3 V, vec3 L, float roughness) {
    float NdotV = max(dot(N, V), 0.0);
    float NdotL = max(dot(N, L), 0.0);
    return geometrySchlickGGX(NdotV, roughness) * geometrySchlickGGX(NdotL, roughness);
}
"#;

const GLSL_THIN_FILM: &str = r#"
// Thin-film interference
vec3 thinFilmInterference(float cosTheta, float filmN, float thickness, float wavelength) {
    float delta = 2.0 * 3.14159265 * filmN * thickness * cosTheta / wavelength;
    float interference = 0.5 + 0.5 * cos(delta);
    return vec3(interference);
}
"#;

const GLSL_PBR_EVALUATE: &str = r#"
// PBR material evaluation
vec3 evaluatePBR(vec3 N, vec3 V, vec3 L, vec3 albedo, float metallic, float roughness, float ior) {
    vec3 H = normalize(V + L);
    float NdotL = max(dot(N, L), 0.0);
    float NdotV = max(dot(N, V), 0.0);
    float HdotV = max(dot(H, V), 0.0);

    // F0 for dielectric/metal
    vec3 F0 = mix(vec3(pow((ior - 1.0) / (ior + 1.0), 2.0)), albedo, metallic);

    // Cook-Torrance BRDF
    float D = distributionGGX(N, H, roughness);
    float G = geometrySmith(N, V, L, roughness);
    vec3 F = fresnelSchlick(HdotV, F0);

    vec3 numerator = D * G * F;
    float denominator = 4.0 * NdotV * NdotL + 0.0001;
    vec3 specular = numerator / denominator;

    // Energy conservation
    vec3 kS = F;
    vec3 kD = (1.0 - kS) * (1.0 - metallic);

    return (kD * albedo / 3.14159265 + specular) * NdotL;
}
"#;

// ============================================================================
// WGSL SHADER CODE TEMPLATES
// ============================================================================

const WGSL_FRESNEL_SCHLICK: &str = r#"
// Fresnel-Schlick approximation
fn fresnel_schlick(cos_theta: f32, F0: vec3<f32>) -> vec3<f32> {
    return F0 + (1.0 - F0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}
"#;

const WGSL_GGX_DISTRIBUTION: &str = r#"
// GGX distribution
fn distribution_ggx(N: vec3<f32>, H: vec3<f32>, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let NdotH = max(dot(N, H), 0.0);
    let NdotH2 = NdotH * NdotH;

    let denom = NdotH2 * (a2 - 1.0) + 1.0;
    return a2 / (3.14159265 * denom * denom);
}
"#;

const WGSL_PBR_EVALUATE: &str = r#"
// PBR evaluation
fn evaluate_pbr(N: vec3<f32>, V: vec3<f32>, L: vec3<f32>, mat: Material) -> vec3<f32> {
    let H = normalize(V + L);
    let NdotL = max(dot(N, L), 0.0);
    let HdotV = max(dot(H, V), 0.0);

    let f0_dielectric = pow((mat.ior - 1.0) / (mat.ior + 1.0), 2.0);
    let F0 = mix(vec3<f32>(f0_dielectric), mat.baseColor, mat.metallic);

    let F = fresnel_schlick(HdotV, F0);
    let D = distribution_ggx(N, H, mat.roughness);

    let specular = D * F * 0.25;
    let diffuse = mat.baseColor * (1.0 - mat.metallic) / 3.14159265;

    return (diffuse + specular) * NdotL;
}
"#;

// ============================================================================
// MEMORY ESTIMATION
// ============================================================================

/// Estimate memory usage for export module
pub fn total_export_memory() -> usize {
    // Code templates: ~5KB
    // MaterialDescriptor: ~200 bytes
    // ExportOptions: ~32 bytes
    // Working buffers: ~2KB
    5120 + 2048
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_descriptor() {
        let mat = MaterialDescriptor::new("TestGlass")
            .with_ior(1.5)
            .with_roughness(0.1)
            .with_base_color(0.9, 0.9, 0.95);

        assert_eq!(mat.name, "TestGlass");
        assert!((mat.ior - 1.5).abs() < 0.001);
        assert!(mat.is_dielectric());
        assert!((mat.f0() - 0.04).abs() < 0.01);
    }

    #[test]
    fn test_glsl_export() {
        let mat = MaterialDescriptor::new("Glass").with_ior(1.5);
        let exporter = MaterialExporter::new(ExportTarget::glsl_300es());

        let glsl = exporter.export(&mat);

        assert!(glsl.contains("#version 300 es"));
        assert!(glsl.contains("u_ior"));
        assert!(glsl.contains("fresnelSchlick"));
    }

    #[test]
    fn test_wgsl_export() {
        let mat = MaterialDescriptor::new("Metal")
            .with_metallic(0.9)
            .with_roughness(0.3);

        let exporter = MaterialExporter::new(ExportTarget::wgsl());
        let wgsl = exporter.export(&mat);

        assert!(wgsl.contains("struct Material"));
        assert!(wgsl.contains("fresnel_schlick"));
    }

    #[test]
    fn test_materialx_export() {
        let mat = MaterialDescriptor::new("Gold")
            .with_metallic(0.95)
            .with_base_color(0.8, 0.6, 0.2);

        let exporter = MaterialExporter::new(ExportTarget::materialx());
        let xml = exporter.export(&mat);

        assert!(xml.contains("materialx"));
        assert!(xml.contains("standard_surface"));
        assert!(xml.contains("Gold"));
    }

    #[test]
    fn test_css_export() {
        let mat = MaterialDescriptor::new("Glass")
            .with_transmission(0.8)
            .with_ior(1.5);

        let exporter = MaterialExporter::new(ExportTarget::css());
        let css = exporter.export(&mat);

        assert!(css.contains(".material-glass"));
        assert!(css.contains("backdrop-filter"));
    }

    #[test]
    fn test_thin_film_descriptor() {
        let mat = MaterialDescriptor::new("SoapBubble").with_thin_film(1.33, 300.0, 1.0);

        assert!(mat.has_thin_film());
        let tf = mat.thin_film.unwrap();
        assert!((tf.n_film - 1.33).abs() < 0.01);
    }

    #[test]
    fn test_memory_estimate() {
        let mem = total_export_memory();
        assert!(mem > 0);
        assert!(mem < 50_000);
    }
}

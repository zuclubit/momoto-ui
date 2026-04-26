//! # Material Import Module
//!
//! Import material parameters from other engines and formats.
//!
//! ## Supported Formats
//!
//! - **MaterialX**: Standard material exchange format
//! - **glTF**: PBR Metallic-Roughness materials
//! - **Custom**: Extensible via ImportAdapter trait
//!
//! ## Usage
//!
//! ```rust
//! use momoto_materials::glass_physics::material_import::{
//!     MaterialImporter, ImportSource
//! };
//!
//! let importer = MaterialImporter::new();
//!
//! // Import from MaterialX string
//! let mtlx = r#"<materialx>...</materialx>"#;
//! if let Ok(descriptor) = importer.import(ImportSource::MaterialXString(mtlx.to_string())) {
//!     println!("Imported: {}", descriptor.name);
//! }
//! ```

use super::material_export::MaterialDescriptor;
use std::path::PathBuf;

// ============================================================================
// IMPORT SOURCE
// ============================================================================

/// Source for material import
#[derive(Debug, Clone)]
pub enum ImportSource {
    /// MaterialX XML file
    MaterialXFile(PathBuf),
    /// MaterialX XML string
    MaterialXString(String),
    /// glTF material file
    GltfFile(PathBuf),
    /// glTF material JSON string
    GltfString(String),
    /// Generic JSON parameters
    JsonString(String),
    /// Custom adapter
    Custom { adapter_name: String, data: Vec<u8> },
}

// ============================================================================
// IMPORT ADAPTER TRAIT
// ============================================================================

/// Trait for custom import adapters
pub trait ImportAdapter: Send + Sync {
    /// Adapter name
    fn name(&self) -> &str;

    /// Supported file extensions
    fn supported_extensions(&self) -> &[&str];

    /// Import material from raw data
    fn import(&self, data: &[u8]) -> Result<MaterialDescriptor, ImportError>;

    /// Check if adapter can handle this data
    fn can_handle(&self, data: &[u8]) -> bool {
        !data.is_empty()
    }
}

// ============================================================================
// IMPORT ERROR
// ============================================================================

/// Errors during material import
#[derive(Debug, Clone)]
pub enum ImportError {
    /// File not found
    FileNotFound(String),
    /// Unsupported format
    UnsupportedFormat(String),
    /// Parse error
    ParseError(String),
    /// Missing required property
    MissingProperty(String),
    /// Invalid property value
    InvalidValue {
        property: String,
        value: String,
        expected: String,
    },
    /// IO error
    IoError(String),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileNotFound(path) => write!(f, "File not found: {}", path),
            Self::UnsupportedFormat(fmt) => write!(f, "Unsupported format: {}", fmt),
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
            Self::MissingProperty(prop) => write!(f, "Missing property: {}", prop),
            Self::InvalidValue {
                property,
                value,
                expected,
            } => write!(
                f,
                "Invalid value for '{}': '{}', expected {}",
                property, value, expected
            ),
            Self::IoError(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

impl std::error::Error for ImportError {}

// ============================================================================
// MATERIAL IMPORTER
// ============================================================================

/// Main importer for material conversion
pub struct MaterialImporter {
    /// Custom adapters
    adapters: Vec<Box<dyn ImportAdapter>>,
}

impl MaterialImporter {
    /// Create new importer
    pub fn new() -> Self {
        Self {
            adapters: Vec::new(),
        }
    }

    /// Register custom adapter
    pub fn register_adapter(&mut self, adapter: Box<dyn ImportAdapter>) {
        self.adapters.push(adapter);
    }

    /// Import material from source
    pub fn import(&self, source: ImportSource) -> Result<MaterialDescriptor, ImportError> {
        match source {
            ImportSource::MaterialXString(xml) => self.parse_materialx(&xml),
            ImportSource::MaterialXFile(path) => {
                // In a real implementation, read file here
                Err(ImportError::FileNotFound(path.display().to_string()))
            }
            ImportSource::GltfString(json) => self.parse_gltf(&json),
            ImportSource::GltfFile(path) => {
                Err(ImportError::FileNotFound(path.display().to_string()))
            }
            ImportSource::JsonString(json) => self.parse_json(&json),
            ImportSource::Custom { adapter_name, data } => {
                if let Some(adapter) = self.adapters.iter().find(|a| a.name() == adapter_name) {
                    adapter.import(&data)
                } else {
                    Err(ImportError::UnsupportedFormat(adapter_name))
                }
            }
        }
    }

    /// Detect format from data
    pub fn detect_format(data: &[u8]) -> Option<String> {
        let text = String::from_utf8_lossy(data);
        let trimmed = text.trim();

        if trimmed.starts_with("<?xml") || trimmed.starts_with("<materialx") {
            Some("materialx".to_string())
        } else if trimmed.starts_with('{') && trimmed.contains("\"pbrMetallicRoughness\"") {
            Some("gltf".to_string())
        } else if trimmed.starts_with('{') {
            Some("json".to_string())
        } else {
            None
        }
    }

    /// Parse MaterialX XML
    fn parse_materialx(&self, xml: &str) -> Result<MaterialDescriptor, ImportError> {
        // Simple XML parsing (production would use a proper XML parser)
        let mut descriptor = MaterialDescriptor::new("imported");

        // Extract name
        if let Some(name) = extract_xml_attribute(xml, "name") {
            descriptor.name = name;
        }

        // Parse base_color
        if let Some(value) = extract_xml_input(xml, "base_color") {
            if let Ok(color) = parse_color3(&value) {
                descriptor.base_color = color;
            }
        }

        // Parse metalness
        if let Some(value) = extract_xml_input(xml, "metalness") {
            if let Ok(v) = value.parse::<f64>() {
                descriptor.metallic = v.clamp(0.0, 1.0);
            }
        }

        // Parse specular_roughness
        if let Some(value) = extract_xml_input(xml, "specular_roughness") {
            if let Ok(v) = value.parse::<f64>() {
                descriptor.roughness = v.clamp(0.0, 1.0);
            }
        }

        // Parse specular_IOR
        if let Some(value) = extract_xml_input(xml, "specular_IOR") {
            if let Ok(v) = value.parse::<f64>() {
                descriptor.ior = v.max(1.0);
            }
        }

        // Parse transmission
        if let Some(value) = extract_xml_input(xml, "transmission") {
            if let Ok(v) = value.parse::<f64>() {
                descriptor.transmission = v.clamp(0.0, 1.0);
            }
        }

        Ok(descriptor)
    }

    /// Parse glTF PBR material
    fn parse_gltf(&self, json: &str) -> Result<MaterialDescriptor, ImportError> {
        let mut descriptor = MaterialDescriptor::new("imported");

        // Extract name
        if let Some(name) = extract_json_string(json, "name") {
            descriptor.name = name;
        }

        // Parse pbrMetallicRoughness
        if let Some(pbr_section) = extract_json_object(json, "pbrMetallicRoughness") {
            // Base color
            if let Some(bc) = extract_json_array(&pbr_section, "baseColorFactor") {
                let parts: Vec<f64> = bc
                    .split(',')
                    .filter_map(|s| s.trim().parse().ok())
                    .collect();
                if parts.len() >= 3 {
                    descriptor.base_color = [parts[0], parts[1], parts[2]];
                }
            }

            // Metallic
            if let Some(value) = extract_json_number(&pbr_section, "metallicFactor") {
                descriptor.metallic = value.clamp(0.0, 1.0);
            }

            // Roughness
            if let Some(value) = extract_json_number(&pbr_section, "roughnessFactor") {
                descriptor.roughness = value.clamp(0.0, 1.0);
            }
        }

        // Default IOR for glTF (1.5 is typical)
        descriptor.ior = 1.5;

        Ok(descriptor)
    }

    /// Parse generic JSON parameters
    fn parse_json(&self, json: &str) -> Result<MaterialDescriptor, ImportError> {
        let mut descriptor = MaterialDescriptor::new("imported");

        // Name
        if let Some(name) = extract_json_string(json, "name") {
            descriptor.name = name;
        }

        // Direct properties
        if let Some(value) = extract_json_number(json, "ior") {
            descriptor.ior = value.max(1.0);
        }

        if let Some(value) = extract_json_number(json, "roughness") {
            descriptor.roughness = value.clamp(0.0, 1.0);
        }

        if let Some(value) = extract_json_number(json, "metallic") {
            descriptor.metallic = value.clamp(0.0, 1.0);
        }

        if let Some(value) = extract_json_number(json, "transmission") {
            descriptor.transmission = value.clamp(0.0, 1.0);
        }

        // Base color as array
        if let Some(bc) = extract_json_array(json, "baseColor") {
            let parts: Vec<f64> = bc
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            if parts.len() >= 3 {
                descriptor.base_color = [parts[0], parts[1], parts[2]];
            }
        }

        Ok(descriptor)
    }
}

impl Default for MaterialImporter {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// PARSING HELPERS
// ============================================================================

/// Extract XML attribute value (simple regex-free implementation)
fn extract_xml_attribute(xml: &str, attr: &str) -> Option<String> {
    let pattern = format!("{}=\"", attr);
    if let Some(start) = xml.find(&pattern) {
        let value_start = start + pattern.len();
        if let Some(end) = xml[value_start..].find('"') {
            return Some(xml[value_start..value_start + end].to_string());
        }
    }
    None
}

/// Extract XML input value
fn extract_xml_input(xml: &str, name: &str) -> Option<String> {
    let pattern = format!("name=\"{}\"", name);
    if let Some(pos) = xml.find(&pattern) {
        // Find value attribute in same tag
        let tag_end = xml[pos..].find("/>").unwrap_or(xml.len() - pos);
        let tag = &xml[pos..pos + tag_end];

        if let Some(value_start) = tag.find("value=\"") {
            let start = value_start + 7;
            if let Some(end) = tag[start..].find('"') {
                return Some(tag[start..start + end].to_string());
            }
        }
    }
    None
}

/// Parse color3 from string (e.g., "0.8, 0.6, 0.2")
fn parse_color3(s: &str) -> Result<[f64; 3], ImportError> {
    let parts: Vec<f64> = s
        .split(',')
        .map(|p| p.trim())
        .filter_map(|p| p.parse().ok())
        .collect();

    if parts.len() >= 3 {
        Ok([parts[0], parts[1], parts[2]])
    } else {
        Err(ImportError::ParseError(format!("Invalid color3: '{}'", s)))
    }
}

/// Extract JSON string value
fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\":", key);
    if let Some(pos) = json.find(&pattern) {
        let after_key = &json[pos + pattern.len()..];
        let trimmed = after_key.trim_start();

        if trimmed.starts_with('"') {
            let start = 1;
            if let Some(end) = trimmed[start..].find('"') {
                return Some(trimmed[start..start + end].to_string());
            }
        }
    }
    None
}

/// Extract JSON number value
fn extract_json_number(json: &str, key: &str) -> Option<f64> {
    let pattern = format!("\"{}\":", key);
    if let Some(pos) = json.find(&pattern) {
        let after_key = &json[pos + pattern.len()..];
        let trimmed = after_key.trim_start();

        // Find end of number
        let end = trimmed
            .find(|c: char| !c.is_numeric() && c != '.' && c != '-')
            .unwrap_or(trimmed.len());

        if end > 0 {
            return trimmed[..end].parse().ok();
        }
    }
    None
}

/// Extract JSON array as string
fn extract_json_array(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\":", key);
    if let Some(pos) = json.find(&pattern) {
        let after_key = &json[pos + pattern.len()..];
        if let Some(start) = after_key.find('[') {
            if let Some(end) = after_key[start..].find(']') {
                return Some(after_key[start + 1..start + end].to_string());
            }
        }
    }
    None
}

/// Extract JSON object as string
fn extract_json_object(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\":", key);
    if let Some(pos) = json.find(&pattern) {
        let after_key = &json[pos + pattern.len()..];
        if let Some(start) = after_key.find('{') {
            let mut depth = 1;
            for (i, c) in after_key[start + 1..].chars().enumerate() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            return Some(after_key[start..start + i + 2].to_string());
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    None
}

// ============================================================================
// MEMORY ESTIMATION
// ============================================================================

/// Estimate memory usage for import module
pub fn total_import_memory() -> usize {
    // Parser buffers: ~4KB
    // MaterialDescriptor: ~200 bytes
    // Adapter list: ~1KB
    4096 + 1024
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_importer_creation() {
        let importer = MaterialImporter::new();
        assert!(importer.adapters.is_empty());
    }

    #[test]
    fn test_parse_materialx() {
        let xml = r#"
            <materialx version="1.38">
                <standard_surface name="TestMaterial" type="surfaceshader">
                    <input name="base_color" type="color3" value="0.8, 0.6, 0.2" />
                    <input name="metalness" type="float" value="0.9" />
                    <input name="specular_roughness" type="float" value="0.3" />
                    <input name="specular_IOR" type="float" value="1.5" />
                </standard_surface>
            </materialx>
        "#;

        let importer = MaterialImporter::new();
        let result = importer.import(ImportSource::MaterialXString(xml.to_string()));

        assert!(result.is_ok());
        let mat = result.unwrap();
        assert!((mat.metallic - 0.9).abs() < 0.01);
        assert!((mat.roughness - 0.3).abs() < 0.01);
    }

    #[test]
    fn test_parse_gltf() {
        let json = r#"{
            "name": "GoldMetal",
            "pbrMetallicRoughness": {
                "baseColorFactor": [0.8, 0.6, 0.2, 1.0],
                "metallicFactor": 0.95,
                "roughnessFactor": 0.2
            }
        }"#;

        let importer = MaterialImporter::new();
        let result = importer.import(ImportSource::GltfString(json.to_string()));

        assert!(result.is_ok());
        let mat = result.unwrap();
        assert_eq!(mat.name, "GoldMetal");
        assert!((mat.metallic - 0.95).abs() < 0.01);
    }

    #[test]
    fn test_parse_json() {
        let json = r#"{
            "name": "SimpleGlass",
            "ior": 1.52,
            "roughness": 0.05,
            "transmission": 0.9
        }"#;

        let importer = MaterialImporter::new();
        let result = importer.import(ImportSource::JsonString(json.to_string()));

        assert!(result.is_ok());
        let mat = result.unwrap();
        assert_eq!(mat.name, "SimpleGlass");
        assert!((mat.ior - 1.52).abs() < 0.01);
    }

    #[test]
    fn test_detect_format() {
        let materialx = b"<?xml version=\"1.0\"?><materialx>";
        assert_eq!(
            MaterialImporter::detect_format(materialx),
            Some("materialx".to_string())
        );

        let gltf = b"{\"pbrMetallicRoughness\": {}}";
        assert_eq!(
            MaterialImporter::detect_format(gltf),
            Some("gltf".to_string())
        );

        let json = b"{\"ior\": 1.5}";
        assert_eq!(
            MaterialImporter::detect_format(json),
            Some("json".to_string())
        );
    }

    #[test]
    fn test_parse_color3() {
        let result = parse_color3("0.8, 0.6, 0.2");
        assert!(result.is_ok());
        let color = result.unwrap();
        assert!((color[0] - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_memory_estimate() {
        let mem = total_import_memory();
        assert!(mem > 0);
        assert!(mem < 20_000);
    }
}

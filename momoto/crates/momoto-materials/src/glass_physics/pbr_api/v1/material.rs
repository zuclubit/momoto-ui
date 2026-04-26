//! # Material Types
//!
//! High-level material abstractions for the stable API.

use super::bsdf::{BSDFResponse, ConductorBSDF, DielectricBSDF, ThinFilmBSDF, BSDF};
use super::context::EvaluationContext;

/// High-level material wrapper.
///
/// Provides a unified interface for different material types.
#[derive(Debug, Clone)]
pub struct Material {
    /// Material name.
    pub name: String,
    /// Material layers.
    pub layers: Vec<Layer>,
    /// Base color (sRGB, 0-1).
    pub base_color: [f64; 3],
    /// Overall opacity (0-1).
    pub opacity: f64,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            layers: vec![Layer::Dielectric {
                ior: 1.5,
                roughness: 0.0,
            }],
            base_color: [1.0, 1.0, 1.0],
            opacity: 1.0,
        }
    }
}

impl Material {
    /// Create a new material with a name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Create from a preset.
    pub fn from_preset(preset: MaterialPreset) -> Self {
        match preset {
            MaterialPreset::Glass => Self {
                name: "Glass".to_string(),
                layers: vec![Layer::Dielectric {
                    ior: 1.5,
                    roughness: 0.0,
                }],
                base_color: [1.0, 1.0, 1.0],
                opacity: 1.0,
            },
            MaterialPreset::FrostedGlass => Self {
                name: "Frosted Glass".to_string(),
                layers: vec![Layer::Dielectric {
                    ior: 1.5,
                    roughness: 0.3,
                }],
                base_color: [1.0, 1.0, 1.0],
                opacity: 1.0,
            },
            MaterialPreset::Water => Self {
                name: "Water".to_string(),
                layers: vec![Layer::Dielectric {
                    ior: 1.33,
                    roughness: 0.0,
                }],
                base_color: [0.9, 0.95, 1.0],
                opacity: 1.0,
            },
            MaterialPreset::Diamond => Self {
                name: "Diamond".to_string(),
                layers: vec![Layer::Dielectric {
                    ior: 2.42,
                    roughness: 0.0,
                }],
                base_color: [1.0, 1.0, 1.0],
                opacity: 1.0,
            },
            MaterialPreset::Gold => Self {
                name: "Gold".to_string(),
                layers: vec![Layer::Conductor {
                    n: 0.18,
                    k: 3.0,
                    roughness: 0.1,
                }],
                base_color: [1.0, 0.84, 0.0],
                opacity: 1.0,
            },
            MaterialPreset::Silver => Self {
                name: "Silver".to_string(),
                layers: vec![Layer::Conductor {
                    n: 0.14,
                    k: 4.0,
                    roughness: 0.1,
                }],
                base_color: [0.97, 0.96, 0.91],
                opacity: 1.0,
            },
            MaterialPreset::Copper => Self {
                name: "Copper".to_string(),
                layers: vec![Layer::Conductor {
                    n: 0.27,
                    k: 3.4,
                    roughness: 0.15,
                }],
                base_color: [0.95, 0.64, 0.54],
                opacity: 1.0,
            },
            MaterialPreset::SoapBubble => Self {
                name: "Soap Bubble".to_string(),
                layers: vec![Layer::ThinFilm {
                    film_ior: 1.33,
                    substrate_ior: 1.0,
                    thickness_nm: 300.0,
                }],
                base_color: [1.0, 1.0, 1.0],
                opacity: 0.3,
            },
            MaterialPreset::OilSlick => Self {
                name: "Oil Slick".to_string(),
                layers: vec![Layer::ThinFilm {
                    film_ior: 1.5,
                    substrate_ior: 1.33,
                    thickness_nm: 400.0,
                }],
                base_color: [0.1, 0.1, 0.1],
                opacity: 1.0,
            },
        }
    }

    /// Add a layer.
    pub fn with_layer(mut self, layer: Layer) -> Self {
        self.layers.push(layer);
        self
    }

    /// Set base color.
    pub fn with_color(mut self, r: f64, g: f64, b: f64) -> Self {
        self.base_color = [r, g, b];
        self
    }

    /// Set opacity.
    pub fn with_opacity(mut self, opacity: f64) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Evaluate material at a given context.
    pub fn evaluate(&self, ctx: &EvaluationContext) -> BSDFResponse {
        // Evaluate first layer (simplified - full implementation would combine layers)
        if let Some(layer) = self.layers.first() {
            layer.evaluate(ctx)
        } else {
            BSDFResponse::default()
        }
    }
}

/// Material layer type.
#[derive(Debug, Clone)]
pub enum Layer {
    /// Dielectric (glass, water, crystal).
    Dielectric {
        /// Index of refraction.
        ior: f64,
        /// Surface roughness (0-1).
        roughness: f64,
    },
    /// Conductor (metal).
    Conductor {
        /// Real part of complex IOR.
        n: f64,
        /// Imaginary part (extinction coefficient).
        k: f64,
        /// Surface roughness (0-1).
        roughness: f64,
    },
    /// Thin-film interference.
    ThinFilm {
        /// Film IOR.
        film_ior: f64,
        /// Substrate IOR.
        substrate_ior: f64,
        /// Film thickness in nanometers.
        thickness_nm: f64,
    },
    /// Subsurface scattering.
    Subsurface {
        /// Scattering coefficient.
        sigma_s: f64,
        /// Absorption coefficient.
        sigma_a: f64,
        /// Scattering asymmetry.
        g: f64,
    },
    /// Lambertian (matte diffuse).
    Lambertian {
        /// Albedo (0-1).
        albedo: f64,
    },
}

impl Layer {
    /// Evaluate layer at a given context.
    pub fn evaluate(&self, ctx: &EvaluationContext) -> BSDFResponse {
        let bsdf_ctx = ctx.to_bsdf_context();

        match self {
            Layer::Dielectric { ior, roughness } => {
                let bsdf = DielectricBSDF::new(*ior, *roughness);
                bsdf.evaluate(&bsdf_ctx)
            }
            Layer::Conductor { n, k, roughness } => {
                let bsdf = ConductorBSDF::new(*n, *k, *roughness);
                bsdf.evaluate(&bsdf_ctx)
            }
            Layer::ThinFilm {
                film_ior,
                substrate_ior,
                thickness_nm,
            } => {
                let bsdf = ThinFilmBSDF::new(*film_ior, *substrate_ior, *thickness_nm);
                bsdf.evaluate(&bsdf_ctx)
            }
            Layer::Subsurface { .. } => {
                // Simplified - returns diffuse approximation
                BSDFResponse::new(0.5, 0.0, 0.5)
            }
            Layer::Lambertian { albedo } => {
                // Lambertian: all reflection, no transmission
                BSDFResponse::new(*albedo, 0.0, 1.0 - *albedo)
            }
        }
    }
}

/// Material presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialPreset {
    /// Clear glass.
    Glass,
    /// Frosted glass.
    FrostedGlass,
    /// Water.
    Water,
    /// Diamond.
    Diamond,
    /// Gold metal.
    Gold,
    /// Silver metal.
    Silver,
    /// Copper metal.
    Copper,
    /// Soap bubble (thin-film).
    SoapBubble,
    /// Oil slick (thin-film).
    OilSlick,
}

impl MaterialPreset {
    /// Get all available presets.
    pub fn all() -> &'static [MaterialPreset] {
        &[
            MaterialPreset::Glass,
            MaterialPreset::FrostedGlass,
            MaterialPreset::Water,
            MaterialPreset::Diamond,
            MaterialPreset::Gold,
            MaterialPreset::Silver,
            MaterialPreset::Copper,
            MaterialPreset::SoapBubble,
            MaterialPreset::OilSlick,
        ]
    }
}

/// Builder for creating materials.
#[derive(Debug, Clone, Default)]
pub struct MaterialBuilder {
    name: Option<String>,
    layers: Vec<Layer>,
    base_color: [f64; 3],
    opacity: f64,
}

impl MaterialBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            name: None,
            layers: Vec::new(),
            base_color: [1.0, 1.0, 1.0],
            opacity: 1.0,
        }
    }

    /// Set name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Add a dielectric layer.
    pub fn dielectric(mut self, ior: f64, roughness: f64) -> Self {
        self.layers.push(Layer::Dielectric { ior, roughness });
        self
    }

    /// Add a conductor layer.
    pub fn conductor(mut self, n: f64, k: f64, roughness: f64) -> Self {
        self.layers.push(Layer::Conductor { n, k, roughness });
        self
    }

    /// Add a thin-film layer.
    pub fn thin_film(mut self, film_ior: f64, substrate_ior: f64, thickness_nm: f64) -> Self {
        self.layers.push(Layer::ThinFilm {
            film_ior,
            substrate_ior,
            thickness_nm,
        });
        self
    }

    /// Set base color.
    pub fn color(mut self, r: f64, g: f64, b: f64) -> Self {
        self.base_color = [r, g, b];
        self
    }

    /// Set opacity.
    pub fn opacity(mut self, opacity: f64) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Build the material.
    pub fn build(self) -> Material {
        Material {
            name: self.name.unwrap_or_else(|| "Custom".to_string()),
            layers: if self.layers.is_empty() {
                vec![Layer::Dielectric {
                    ior: 1.5,
                    roughness: 0.0,
                }]
            } else {
                self.layers
            },
            base_color: self.base_color,
            opacity: self.opacity,
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_material() {
        let mat = Material::default();
        assert_eq!(mat.name, "Default");
        assert!(!mat.layers.is_empty());
    }

    #[test]
    fn test_preset_glass() {
        let mat = Material::from_preset(MaterialPreset::Glass);
        assert_eq!(mat.name, "Glass");
        assert!(matches!(mat.layers[0], Layer::Dielectric { ior, .. } if (ior - 1.5).abs() < 0.01));
    }

    #[test]
    fn test_preset_gold() {
        let mat = Material::from_preset(MaterialPreset::Gold);
        assert_eq!(mat.name, "Gold");
        assert!(matches!(mat.layers[0], Layer::Conductor { .. }));
    }

    #[test]
    fn test_builder() {
        let mat = MaterialBuilder::new()
            .name("Custom Glass")
            .dielectric(1.52, 0.1)
            .color(0.9, 0.95, 1.0)
            .build();

        assert_eq!(mat.name, "Custom Glass");
        assert!((mat.base_color[0] - 0.9).abs() < 0.01);
    }

    #[test]
    fn test_all_presets() {
        let presets = MaterialPreset::all();
        assert!(!presets.is_empty());

        for preset in presets {
            let mat = Material::from_preset(*preset);
            assert!(!mat.name.is_empty());
        }
    }

    #[test]
    fn test_preset_water() {
        let mat = Material::from_preset(MaterialPreset::Water);
        assert_eq!(mat.name, "Water");
        assert!(
            matches!(mat.layers[0], Layer::Dielectric { ior, .. } if (ior - 1.33).abs() < 0.01)
        );
    }

    #[test]
    fn test_preset_diamond() {
        let mat = Material::from_preset(MaterialPreset::Diamond);
        assert_eq!(mat.name, "Diamond");
        assert!(
            matches!(mat.layers[0], Layer::Dielectric { ior, .. } if (ior - 2.42).abs() < 0.01)
        );
    }

    #[test]
    fn test_preset_silver() {
        let mat = Material::from_preset(MaterialPreset::Silver);
        assert_eq!(mat.name, "Silver");
        assert!(matches!(mat.layers[0], Layer::Conductor { .. }));
    }

    #[test]
    fn test_preset_copper() {
        let mat = Material::from_preset(MaterialPreset::Copper);
        assert_eq!(mat.name, "Copper");
        assert!(matches!(mat.layers[0], Layer::Conductor { .. }));
    }

    #[test]
    fn test_preset_soap_bubble() {
        let mat = Material::from_preset(MaterialPreset::SoapBubble);
        assert_eq!(mat.name, "Soap Bubble");
        assert!(matches!(mat.layers[0], Layer::ThinFilm { .. }));
    }

    #[test]
    fn test_preset_oil_slick() {
        let mat = Material::from_preset(MaterialPreset::OilSlick);
        assert_eq!(mat.name, "Oil Slick");
        assert!(matches!(mat.layers[0], Layer::ThinFilm { .. }));
    }

    #[test]
    fn test_builder_thin_film() {
        let mat = MaterialBuilder::new()
            .name("Rainbow Film")
            .thin_film(1.45, 1.0, 500.0)
            .build();

        assert_eq!(mat.name, "Rainbow Film");
        assert!(matches!(mat.layers[0], Layer::ThinFilm { .. }));
    }

    #[test]
    fn test_builder_conductor() {
        let mat = MaterialBuilder::new()
            .conductor(0.2, 3.5, 0.05)
            .color(0.9, 0.8, 0.7)
            .build();

        assert!(matches!(mat.layers[0], Layer::Conductor { .. }));
        assert!((mat.base_color[0] - 0.9).abs() < 0.01);
    }

    #[test]
    fn test_material_with_layer() {
        let mat = Material::new("Custom").with_layer(Layer::Lambertian { albedo: 0.8 });

        assert_eq!(mat.layers.len(), 2); // Default + added
    }

    #[test]
    fn test_material_with_opacity() {
        let mat = Material::new("Transparent").with_opacity(0.5);

        assert!((mat.opacity - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_material_opacity_clamping() {
        let mat = Material::new("Over").with_opacity(1.5);
        assert!((mat.opacity - 1.0).abs() < 0.01);

        let mat2 = Material::new("Under").with_opacity(-0.5);
        assert!(mat2.opacity >= 0.0);
    }
}

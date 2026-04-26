//! # Evaluation Context
//!
//! Context types for BSDF evaluation.

use super::bsdf::BSDFContext;

/// 3D direction vector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector3 {
    /// X component.
    pub x: f64,
    /// Y component.
    pub y: f64,
    /// Z component.
    pub z: f64,
}

impl Vector3 {
    /// Create a new vector.
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Zero vector.
    pub fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }

    /// Unit Z vector (up).
    pub fn unit_z() -> Self {
        Self::new(0.0, 0.0, 1.0)
    }

    /// Dot product.
    pub fn dot(&self, other: &Vector3) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Vector length.
    pub fn length(&self) -> f64 {
        self.dot(self).sqrt()
    }

    /// Normalize vector.
    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len > 1e-10 {
            Self::new(self.x / len, self.y / len, self.z / len)
        } else {
            Self::unit_z()
        }
    }

    /// Reflect around normal.
    pub fn reflect(&self, normal: &Vector3) -> Self {
        let d = 2.0 * self.dot(normal);
        Self::new(
            self.x - d * normal.x,
            self.y - d * normal.y,
            self.z - d * normal.z,
        )
    }
}

impl Default for Vector3 {
    fn default() -> Self {
        Self::unit_z()
    }
}

impl From<super::super::super::unified_bsdf::Vector3> for Vector3 {
    fn from(v: super::super::super::unified_bsdf::Vector3) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}

impl From<Vector3> for super::super::super::unified_bsdf::Vector3 {
    fn from(v: Vector3) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}

/// Evaluation context for BSDF.
///
/// Contains all geometric information needed for BSDF evaluation.
#[derive(Debug, Clone)]
pub struct EvaluationContext {
    /// Incident direction (towards surface).
    pub wi: Vector3,
    /// Outgoing direction (away from surface).
    pub wo: Vector3,
    /// Surface normal.
    pub normal: Vector3,
    /// Surface tangent (for anisotropic).
    pub tangent: Vector3,
    /// Wavelength in nm (for spectral).
    pub wavelength: Option<f64>,
}

impl Default for EvaluationContext {
    fn default() -> Self {
        Self {
            wi: Vector3::unit_z(),
            wo: Vector3::unit_z(),
            normal: Vector3::unit_z(),
            tangent: Vector3::new(1.0, 0.0, 0.0),
            wavelength: None,
        }
    }
}

impl EvaluationContext {
    /// Create a new evaluation context.
    pub fn new(wi: Vector3, wo: Vector3, normal: Vector3) -> Self {
        Self {
            wi: wi.normalize(),
            wo: wo.normalize(),
            normal: normal.normalize(),
            tangent: Vector3::new(1.0, 0.0, 0.0),
            wavelength: None,
        }
    }

    /// Create for normal incidence (looking straight at surface).
    pub fn normal_incidence() -> Self {
        Self::default()
    }

    /// Create for a specific incident angle.
    pub fn at_angle(theta_degrees: f64) -> Self {
        let theta = theta_degrees.to_radians();
        let cos_theta = theta.cos();
        let sin_theta = theta.sin();

        Self {
            wi: Vector3::new(sin_theta, 0.0, cos_theta),
            wo: Vector3::new(-sin_theta, 0.0, cos_theta), // Reflection
            normal: Vector3::unit_z(),
            tangent: Vector3::new(1.0, 0.0, 0.0),
            wavelength: None,
        }
    }

    /// Set wavelength for spectral evaluation.
    pub fn with_wavelength(mut self, wavelength_nm: f64) -> Self {
        self.wavelength = Some(wavelength_nm);
        self
    }

    /// Set tangent for anisotropic evaluation.
    pub fn with_tangent(mut self, tangent: Vector3) -> Self {
        self.tangent = tangent.normalize();
        self
    }

    /// Cosine of incident angle.
    pub fn cos_theta_i(&self) -> f64 {
        self.wi.dot(&self.normal).abs()
    }

    /// Cosine of outgoing angle.
    pub fn cos_theta_o(&self) -> f64 {
        self.wo.dot(&self.normal).abs()
    }

    /// Convert to internal BSDFContext.
    pub fn to_bsdf_context(&self) -> BSDFContext {
        use super::super::super::unified_bsdf::Vector3 as BSDFVector3;

        let n: BSDFVector3 = self.normal.into();
        let t: BSDFVector3 = self.tangent.into();

        // Compute bitangent via cross product: b = n × t
        let bitangent = BSDFVector3::new(
            n.y * t.z - n.z * t.y,
            n.z * t.x - n.x * t.z,
            n.x * t.y - n.y * t.x,
        )
        .normalize();

        BSDFContext {
            wi: self.wi.into(),
            wo: self.wo.into(),
            normal: n,
            tangent: t,
            bitangent,
            wavelength: self.wavelength.unwrap_or(550.0),
            wavelengths: None,
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
    fn test_vector3_creation() {
        let v = Vector3::new(1.0, 2.0, 3.0);
        assert!((v.x - 1.0).abs() < 0.01);
        assert!((v.y - 2.0).abs() < 0.01);
        assert!((v.z - 3.0).abs() < 0.01);
    }

    #[test]
    fn test_vector3_normalize() {
        let v = Vector3::new(3.0, 4.0, 0.0);
        let n = v.normalize();
        assert!((n.length() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_vector3_dot() {
        let a = Vector3::new(1.0, 0.0, 0.0);
        let b = Vector3::new(0.0, 1.0, 0.0);
        assert!(a.dot(&b).abs() < 0.001);

        let c = Vector3::new(1.0, 0.0, 0.0);
        assert!((a.dot(&c) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_context_normal_incidence() {
        let ctx = EvaluationContext::normal_incidence();
        assert!((ctx.cos_theta_i() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_context_at_angle() {
        let ctx = EvaluationContext::at_angle(45.0);
        let expected_cos = 45.0_f64.to_radians().cos();
        assert!((ctx.cos_theta_i() - expected_cos).abs() < 0.01);
    }
}

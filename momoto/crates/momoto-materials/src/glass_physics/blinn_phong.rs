//! # Blinn-Phong Specular Lighting Model
//!
//! Dynamic specular highlight calculation for realistic glass reflections.
//!
//! ## Why Blinn-Phong?
//!
//! Most UI glass effects use **static CSS gradients** for highlights—radial gradients
//! positioned arbitrarily at "nice" spots like `30% 20%`. This looks artificial because:
//! - Highlights don't respond to light sources
//! - They don't change based on material properties
//! - They're the same regardless of viewing angle
//!
//! **Blinn-Phong** calculates highlights based on:
//! - **Light position**: Where light comes from
//! - **View direction**: Where you're looking from
//! - **Surface normal**: How the surface is oriented
//! - **Shininess**: Material property (smooth = tight highlight, rough = soft)
//!
//! ## Blinn-Phong vs. Phong
//!
//! | Property | Phong | Blinn-Phong |
//! |----------|-------|-------------|
//! | Speed | Slower | **Faster** |
//! | Highlight shape | Spherical | **Elliptical** (more realistic) |
//! | Edge artifacts | Yes (>90° angles) | **None** |
//! | Physical accuracy | Approximate | **Better approximation** |
//!
//! Blinn-Phong uses the **halfway vector** between light and view, avoiding
//! the reflection vector calculation and edge cases.
//!
//! ## Usage
//!
//! ```rust
//! use momoto_materials::glass_physics::blinn_phong::{
//!     blinn_phong_specular, roughness_to_shininess
//! };
//! use momoto_materials::glass_physics::light_model::Vec3;
//!
//! let normal = Vec3::new(0.0, 0.0, 1.0);  // Flat surface facing up
//! let light = Vec3::new(-0.5, -0.3, 0.8).normalize();  // Light from upper-left
//! let view = Vec3::new(0.0, 0.0, 1.0);    // Looking straight down
//!
//! let shininess = roughness_to_shininess(0.15); // Regular glass
//! let intensity = blinn_phong_specular(normal, light, view, shininess);
//!
//! println!("Specular intensity: {:.1}%", intensity * 100.0);
//! ```

use super::light_model::Vec3;

/// Calculate Blinn-Phong specular intensity
///
/// This is the **core specular calculation** used for all glass highlights.
///
/// # Formula
///
/// ```text
/// I_spec = (N · H)^shininess
///
/// where H = normalize(L + V)
/// ```
///
/// - **N**: Surface normal (what direction the surface faces)
/// - **L**: Light direction (from surface to light)
/// - **V**: View direction (from surface to viewer)
/// - **H**: Halfway vector (bisector of L and V)
///
/// # Arguments
///
/// * `normal` - Surface normal vector (**must be normalized**)
/// * `light_dir` - Direction from surface to light source (**must be normalized**)
/// * `view_dir` - Direction from surface to viewer (**must be normalized**)
/// * `shininess` - Specular shininess/tightness (1-256)
///   - **1-8**: Very rough (broad, soft highlight)
///   - **16-64**: Regular materials (moderate highlight)
///   - **128-256**: Very smooth (tight, sharp highlight)
///
/// # Returns
///
/// Specular intensity from 0.0 (no highlight) to 1.0 (full highlight)
///
/// # Example
///
/// ```rust
/// use momoto_materials::glass_physics::blinn_phong::blinn_phong_specular;
/// use momoto_materials::glass_physics::light_model::Vec3;
///
/// let normal = Vec3::new(0.0, 0.0, 1.0);
/// let light = Vec3::new(0.0, 1.0, 1.0).normalize();
/// let view = Vec3::new(0.0, 0.0, 1.0);
///
/// // Smooth glass (high shininess)
/// let smooth = blinn_phong_specular(normal, light, view, 128.0);
///
/// // Rough glass (low shininess)
/// let rough = blinn_phong_specular(normal, light, view, 16.0);
///
/// assert!(smooth < rough); // Smooth glass has tighter highlight
/// ```
#[inline]
pub fn blinn_phong_specular(normal: Vec3, light_dir: Vec3, view_dir: Vec3, shininess: f64) -> f64 {
    // Calculate halfway vector (bisector of light and view)
    let halfway = (light_dir + view_dir).normalize();

    // Calculate dot product of normal and halfway vector
    let n_dot_h = normal.dot(&halfway).max(0.0);

    // Apply shininess power
    if n_dot_h > 0.0 {
        n_dot_h.powf(shininess)
    } else {
        0.0
    }
}

/// Calculate multiple specular highlight layers for layered glass effect
///
/// Real glass often shows multiple reflections:
/// - **Main highlight**: Direct reflection of primary light
/// - **Secondary highlights**: Reflections from environment
/// - **Edge highlights**: Rim lighting from grazing light
///
/// This generates 3-4 highlight layers at different positions and intensities.
///
/// # Arguments
///
/// * `normal` - Surface normal vector
/// * `light_dir` - Primary light direction
/// * `view_dir` - View direction
/// * `base_shininess` - Shininess for main highlight
///
/// # Returns
///
/// Vector of `(position_x, position_y, intensity, size)` tuples
/// - **position_x, position_y**: Position in normalized coordinates (0.0-1.0)
/// - **intensity**: Brightness (0.0-1.0)
/// - **size**: Size multiplier (0.5-2.0, where 1.0 is base size)
pub fn calculate_specular_layers(
    normal: Vec3,
    light_dir: Vec3,
    view_dir: Vec3,
    base_shininess: f64,
) -> Vec<(f64, f64, f64, f64)> {
    let mut layers = Vec::new();

    // Layer 1: Main specular highlight (strongest)
    let main_intensity = blinn_phong_specular(normal, light_dir, view_dir, base_shininess);

    // Position calculation: project light direction onto surface plane
    let light_x = 0.5 + light_dir.x * 0.3;
    let light_y = 0.5 - light_dir.y * 0.3;

    layers.push((light_x, light_y, main_intensity, 1.4));

    // Layer 2: Secondary reflection (weaker, offset)
    let secondary_light =
        Vec3::new(light_dir.x + 0.4, light_dir.y + 0.2, light_dir.z * 0.8).normalize();

    let secondary_intensity =
        blinn_phong_specular(normal, secondary_light, view_dir, base_shininess * 0.7);

    let sec_x = 0.5 + secondary_light.x * 0.35;
    let sec_y = 0.5 - secondary_light.y * 0.35;

    layers.push((sec_x, sec_y, secondary_intensity * 0.4, 1.0));

    // Layer 3: Top edge highlight (rim lighting)
    let edge_normal = Vec3::new(0.0, -0.3, 0.95).normalize();
    let edge_shininess = base_shininess * 2.0;

    let edge_intensity = blinn_phong_specular(edge_normal, light_dir, view_dir, edge_shininess);

    layers.push((0.5, 0.0, edge_intensity * 0.25, 2.0));

    // Layer 4: Left edge highlight
    let left_normal = Vec3::new(-0.3, 0.0, 0.95).normalize();
    let left_intensity = blinn_phong_specular(left_normal, light_dir, view_dir, edge_shininess);

    layers.push((0.0, 0.5, left_intensity * 0.2, 1.8));

    layers
}

/// Map PBR-style roughness (0-1) to Blinn-Phong shininess (1-256)
///
/// Modern PBR workflows use **roughness** as the material property:
/// - 0.0 = perfectly smooth (mirror)
/// - 1.0 = completely rough (diffuse)
///
/// But Blinn-Phong uses **shininess**:
/// - 1 = rough
/// - 256+ = smooth
///
/// This function converts between them using a perceptually-linear mapping.
///
/// # Formula
///
/// ```text
/// shininess = (1 / (roughness² + 0.01)) × 2.56
/// ```
///
/// # Arguments
///
/// * `roughness` - PBR roughness value (0.0-1.0)
///   - **0.0**: Mirror-smooth
///   - **0.05**: Clear glass
///   - **0.15**: Regular glass
///   - **0.25**: Thick glass
///   - **0.6**: Frosted glass
///   - **1.0**: Completely rough
///
/// # Returns
///
/// Shininess value suitable for [`blinn_phong_specular`]
///
/// # Example
///
/// ```rust
/// use momoto_materials::glass_physics::blinn_phong::roughness_to_shininess;
///
/// // Smooth glass (low roughness)
/// let smooth_shininess = roughness_to_shininess(0.05);
/// assert!(smooth_shininess > 100.0);
///
/// // Rough glass (high roughness)
/// let rough_shininess = roughness_to_shininess(0.6);
/// assert!(rough_shininess < 20.0);
/// ```
#[inline]
pub fn roughness_to_shininess(roughness: f64) -> f64 {
    let clamped = roughness.clamp(0.0, 1.0);
    (1.0 / (clamped * clamped + 0.01)) * 2.56
}

/// Calculate specular highlight position for CSS gradient
///
/// Converts 3D light direction into 2D screen position for CSS rendering.
#[inline]
pub fn calculate_highlight_position(light_dir: Vec3) -> (f64, f64) {
    let x = (light_dir.x * 0.4 + 0.5).clamp(0.0, 1.0);
    let y = (-light_dir.y * 0.4 + 0.5).clamp(0.0, 1.0);
    (x, y)
}

// ============================================================================
// CSS Generation
// ============================================================================

/// Generate CSS radial gradient for primary specular highlight
///
/// Creates a positioned light spot based on Blinn-Phong model.
/// The highlight is elliptical (wider than tall) for realism.
///
/// ## Apple Liquid Glass Quality
///
/// This implementation produces a **visible specular spot** that:
/// - Has a bright, concentrated center
/// - Fades smoothly to the edges
/// - Uses enhanced opacity for clear visibility
/// - Mimics real light reflections on glass
///
/// # Arguments
///
/// * `intensity` - Highlight intensity (0.0-1.0)
/// * `size` - Highlight size as percentage (20-60)
/// * `pos_x` - Horizontal position percentage (0-100)
/// * `pos_y` - Vertical position percentage (0-100)
///
/// # Returns
///
/// CSS radial-gradient string
///
/// # Example
///
/// ```rust
/// use momoto_materials::glass_physics::blinn_phong::to_css_specular_highlight;
///
/// let css = to_css_specular_highlight(0.5, 40.0, 28.0, 18.0);
/// assert!(css.contains("radial-gradient"));
/// assert!(css.contains("at 28% 18%"));
/// ```
pub fn to_css_specular_highlight(intensity: f64, size: f64, pos_x: f64, pos_y: f64) -> String {
    let intensity = intensity.clamp(0.0, 1.0);
    let size = size.clamp(10.0, 100.0);
    let pos_x = pos_x.clamp(0.0, 100.0);
    let pos_y = pos_y.clamp(0.0, 100.0);

    // Ellipse aspect ratio (wider than tall for natural look)
    let height = size * 0.65;

    // ENHANCED: Boosted opacity values for visible specular
    // Apple-quality specular is clearly visible, not subtle
    let boosted = (intensity * 1.5).min(1.0);

    format!(
        "radial-gradient(ellipse {:.0}% {:.0}% at {:.0}% {:.0}%, \
         rgba(255, 255, 255, {:.3}) 0%, \
         rgba(255, 255, 255, {:.3}) 15%, \
         rgba(255, 255, 255, {:.3}) 40%, \
         rgba(255, 255, 255, {:.3}) 60%, \
         transparent 85%)",
        size,
        height,
        pos_x,
        pos_y,
        boosted,        // Bright center
        boosted * 0.75, // Still visible
        boosted * 0.4,  // Fading
        boosted * 0.15, // Nearly gone
    )
}

/// Generate CSS radial gradient for secondary specular (fill light)
///
/// Creates a weaker highlight at the opposite corner to simulate
/// ambient/fill lighting in the environment.
///
/// # Arguments
///
/// * `intensity` - Highlight intensity (0.0-1.0)
/// * `size` - Highlight size as percentage (15-40)
///
/// # Returns
///
/// CSS radial-gradient string positioned at bottom-right
pub fn to_css_secondary_specular(intensity: f64, size: f64) -> String {
    let intensity = intensity.clamp(0.0, 1.0);
    let size = size.clamp(15.0, 40.0);

    format!(
        "radial-gradient(circle {:.0}% at 70% 80%, \
         rgba(255, 255, 255, {:.3}) 0%, \
         transparent 60%)",
        size, intensity
    )
}

/// Generate CSS linear gradient for inner top highlight
///
/// Simulates light hitting the top edge of glass,
/// creating a bright line that fades downward.
///
/// ## Apple Liquid Glass Quality
///
/// This creates a **visible top edge shine** that:
/// - Has a bright concentrated line at the very top
/// - Fades gradually into the glass body
/// - Creates the illusion of thickness and depth
///
/// # Arguments
///
/// * `intensity` - Highlight intensity (0.0-1.0)
/// * `light_mode` - Whether to use light mode colors
///
/// # Returns
///
/// CSS linear-gradient string for top-to-bottom fade
pub fn to_css_inner_highlight(intensity: f64, light_mode: bool) -> String {
    let intensity = intensity.clamp(0.0, 1.0);

    // ENHANCED: Stronger top highlight for visible depth effect
    let boosted = if light_mode {
        (intensity * 1.6).min(1.0)
    } else {
        (intensity * 1.3).min(1.0)
    };

    let color = "255, 255, 255";

    format!(
        "linear-gradient(180deg, \
         rgba({}, {:.3}) 0%, \
         rgba({}, {:.3}) 2%, \
         rgba({}, {:.3}) 8%, \
         rgba({}, {:.3}) 20%, \
         transparent 35%)",
        color,
        boosted, // Bright top edge
        color,
        boosted * 0.7, // Still strong
        color,
        boosted * 0.35, // Fading
        color,
        boosted * 0.12, // Nearly gone
    )
}

/// Generate CSS radial gradient for inner glow effect
///
/// Creates a soft inner glow that gives the glass a sense of
/// internal luminosity and depth - a signature of Apple's Liquid Glass.
///
/// # Arguments
///
/// * `intensity` - Glow intensity (0.0-1.0)
/// * `light_mode` - Whether to use light mode colors
///
/// # Returns
///
/// CSS radial-gradient string for inner glow
pub fn to_css_inner_glow(intensity: f64, light_mode: bool) -> String {
    let intensity = intensity.clamp(0.0, 1.0);

    let opacity = if light_mode {
        intensity * 0.35
    } else {
        intensity * 0.25
    };

    let color = "255, 255, 255";

    // Radial gradient from center outward, creating inner luminosity
    format!(
        "radial-gradient(ellipse 80% 70% at 50% 40%, \
         rgba({}, {:.3}) 0%, \
         rgba({}, {:.3}) 30%, \
         rgba({}, {:.3}) 60%, \
         transparent 100%)",
        color,
        opacity * 0.8, // Soft center glow
        color,
        opacity * 0.5, // Mid-fade
        color,
        opacity * 0.2, // Outer fade
    )
}

/// Generate all specular layers as CSS gradients
///
/// Returns a vector of CSS gradient strings that can be combined
/// in the background property.
///
/// # Arguments
///
/// * `config` - Tuple of (intensity, size, pos_x, pos_y)
///
/// # Returns
///
/// Vector of CSS gradient strings
pub fn generate_all_specular_css(
    primary_intensity: f64,
    primary_size: f64,
    primary_pos: (f64, f64),
    include_secondary: bool,
    include_inner: bool,
    light_mode: bool,
) -> Vec<String> {
    let mut gradients = Vec::new();

    // Primary specular
    gradients.push(to_css_specular_highlight(
        primary_intensity,
        primary_size,
        primary_pos.0,
        primary_pos.1,
    ));

    // Secondary specular (fill light)
    if include_secondary && primary_intensity > 0.2 {
        gradients.push(to_css_secondary_specular(primary_intensity * 0.4, 25.0));
    }

    // Inner top highlight
    if include_inner {
        let inner_intensity = if light_mode { 0.45 } else { 0.25 };
        gradients.push(to_css_inner_highlight(inner_intensity, light_mode));
    }

    gradients
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blinn_phong_aligned() {
        let normal = Vec3::new(0.0, 0.0, 1.0);
        let light = Vec3::new(0.0, 0.0, 1.0);
        let view = Vec3::new(0.0, 0.0, 1.0);

        let intensity = blinn_phong_specular(normal, light, view, 32.0);
        assert!(intensity > 0.95);
    }

    #[test]
    fn test_roughness_conversion() {
        let shininess = roughness_to_shininess(0.05);
        assert!(shininess > 100.0);

        let shininess = roughness_to_shininess(0.6);
        assert!(shininess < 20.0);
    }

    #[test]
    fn test_calculate_specular_layers() {
        let normal = Vec3::new(0.0, 0.0, 1.0);
        let light = Vec3::new(-0.5, -0.3, 0.8).normalize();
        let view = Vec3::new(0.0, 0.0, 1.0);

        let layers = calculate_specular_layers(normal, light, view, 64.0);

        assert!(layers.len() >= 3 && layers.len() <= 4);

        for (x, y, intensity, size) in &layers {
            assert!(*x >= 0.0 && *x <= 1.0);
            assert!(*y >= 0.0 && *y <= 1.0);
            assert!(*intensity >= 0.0 && *intensity <= 1.0);
            assert!(*size > 0.0 && *size <= 3.0);
        }
    }
}

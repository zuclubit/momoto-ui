//! Tests for newly exposed public APIs in v6.0.0
//!
//! These tests validate that the newly public modules and functions
//! are accessible and work correctly.

use momoto_core::color::Color;
use momoto_core::gamma::{linear_to_srgb, srgb_to_linear};
use momoto_core::gamut::GAMUT_COEFFICIENTS;
use momoto_core::luminance::{
    relative_luminance_apca, relative_luminance_srgb, soft_clamp, RelativeLuminance,
};
use momoto_core::space::oklch::OKLCH;

// ============================================================================
// Gamma Module Tests
// ============================================================================

#[test]
fn test_srgb_to_linear_black() {
    assert_eq!(srgb_to_linear(0.0), 0.0);
}

#[test]
fn test_srgb_to_linear_white() {
    assert!((srgb_to_linear(1.0) - 1.0).abs() < 0.0001);
}

#[test]
fn test_srgb_to_linear_mid_gray() {
    // sRGB 0.5 (mid gray) → linear ~0.214
    let linear = srgb_to_linear(0.5);
    assert!(
        (linear - 0.214).abs() < 0.01,
        "Mid gray should be ~0.214 in linear, got {}",
        linear
    );
}

#[test]
fn test_linear_to_srgb_black() {
    assert_eq!(linear_to_srgb(0.0), 0.0);
}

#[test]
fn test_linear_to_srgb_white() {
    assert!((linear_to_srgb(1.0) - 1.0).abs() < 0.0001);
}

#[test]
fn test_linear_to_srgb_mid_gray() {
    // linear 0.214 → sRGB ~0.5
    let srgb = linear_to_srgb(0.214);
    assert!(
        (srgb - 0.5).abs() < 0.01,
        "Linear 0.214 should be ~0.5 in sRGB, got {}",
        srgb
    );
}

#[test]
fn test_gamma_roundtrip() {
    // Verify roundtrip: sRGB → linear → sRGB
    let test_values = [0.0, 0.25, 0.5, 0.75, 1.0];
    for &srgb in &test_values {
        let linear = srgb_to_linear(srgb);
        let back = linear_to_srgb(linear);
        assert!(
            (back - srgb).abs() < 0.0001,
            "Roundtrip failed for {}: got {}",
            srgb,
            back
        );
    }
}

#[test]
fn test_gamma_linear_segment() {
    // Below threshold (0.04045), gamma should be linear
    let small = 0.01;
    let linear = srgb_to_linear(small);
    let expected = small / 12.92;
    assert!(
        (linear - expected).abs() < 0.0001,
        "Linear segment: expected {}, got {}",
        expected,
        linear
    );
}

// ============================================================================
// Gamut Coefficients Tests
// ============================================================================

#[test]
fn test_gamut_coefficients_count() {
    // Should have 12 hue anchor points
    assert_eq!(GAMUT_COEFFICIENTS.len(), 12);
}

#[test]
fn test_gamut_coefficients_hue_range() {
    // All hue values should be in 0-360 range
    for &(hue, _) in &GAMUT_COEFFICIENTS {
        assert!(hue < 360, "Hue {} should be less than 360", hue);
    }
}

#[test]
fn test_gamut_coefficients_positive() {
    // All coefficients should be positive
    for &(hue, (a, b)) in &GAMUT_COEFFICIENTS {
        assert!(a > 0.0, "Coefficient 'a' at hue {} should be positive", hue);
        assert!(
            b >= 0.0,
            "Coefficient 'b' at hue {} should be non-negative",
            hue
        );
    }
}

#[test]
fn test_gamut_red_hue() {
    // Red (hue 0) should have specific coefficients
    let red_coef = GAMUT_COEFFICIENTS
        .iter()
        .find(|(h, _)| *h == 0)
        .map(|(_, c)| *c);
    assert_eq!(red_coef, Some((0.28, 0.02)));
}

#[test]
fn test_gamut_estimation_uses_coefficients() {
    // Verify OKLCH.estimate_max_chroma uses these coefficients
    // Red at L=0.5: max_c ≈ 0.28 * 0.5 * 0.5 + 0.02 = 0.09
    let red = OKLCH::new(0.5, 0.05, 0.0);
    let max_c = red.estimate_max_chroma();
    assert!(
        (max_c - 0.09).abs() < 0.02,
        "Red max chroma at L=0.5 should be ~0.09, got {}",
        max_c
    );
}

// ============================================================================
// Luminance Module Tests
// ============================================================================

#[test]
fn test_relative_luminance_srgb_black() {
    let black = Color::from_srgb8(0, 0, 0);
    let y = relative_luminance_srgb(&black).value();
    assert!(y < 0.001, "Black luminance should be ~0, got {}", y);
}

#[test]
fn test_relative_luminance_srgb_white() {
    let white = Color::from_srgb8(255, 255, 255);
    let y = relative_luminance_srgb(&white).value();
    assert!(
        (y - 1.0).abs() < 0.001,
        "White luminance should be ~1, got {}",
        y
    );
}

#[test]
fn test_relative_luminance_srgb_red() {
    // Pure red has luminance ~0.2126 (red coefficient)
    let red = Color::from_srgb8(255, 0, 0);
    let y = relative_luminance_srgb(&red).value();
    assert!(
        (y - 0.2126).abs() < 0.01,
        "Red luminance should be ~0.2126, got {}",
        y
    );
}

#[test]
fn test_relative_luminance_srgb_green() {
    // Pure green has luminance ~0.7152 (green coefficient)
    let green = Color::from_srgb8(0, 255, 0);
    let y = relative_luminance_srgb(&green).value();
    assert!(
        (y - 0.7152).abs() < 0.01,
        "Green luminance should be ~0.7152, got {}",
        y
    );
}

#[test]
fn test_relative_luminance_apca_vs_srgb() {
    // APCA uses slightly different coefficients
    let gray = Color::from_srgb8(128, 128, 128);
    let y_srgb = relative_luminance_srgb(&gray).value();
    let y_apca = relative_luminance_apca(&gray).value();

    // Both should be similar for gray
    assert!(
        (y_srgb - y_apca).abs() < 0.01,
        "Gray luminance difference too large: sRGB={}, APCA={}",
        y_srgb,
        y_apca
    );
}

#[test]
fn test_soft_clamp_above_threshold() {
    // Above threshold, value passes through unchanged
    let high = RelativeLuminance::new(0.1);
    let threshold = 0.022;
    let exponent = 1.414;
    let clamped = soft_clamp(high, threshold, exponent);
    assert!(
        (clamped.value() - high.value()).abs() < 0.001,
        "Above threshold should pass through: {:?} → {:?}",
        high,
        clamped
    );
}

#[test]
fn test_soft_clamp_below_threshold() {
    // Below threshold (0.022), soft clamp applies
    let low = RelativeLuminance::new(0.01);
    let threshold = 0.022;
    let exponent = 1.414;
    let clamped = soft_clamp(low, threshold, exponent);
    // Clamped value should be greater than input
    assert!(
        clamped.value() >= low.value(),
        "Soft clamp should increase value: {:?} → {:?}",
        low,
        clamped
    );
}

// ============================================================================
// Integration Tests - Using All New APIs Together
// ============================================================================

#[test]
fn test_integration_color_workflow() {
    // Create a color - using a more conservative gray to ensure gamut safety
    let gray = Color::from_srgb8(128, 128, 128);

    // Convert to OKLCH
    let oklch = OKLCH::from_color(&gray);

    // Gray should definitely be in gamut (low chroma)
    assert!(oklch.c < 0.01, "Gray should have near-zero chroma");
    assert!(oklch.is_in_gamut(), "Gray should be in gamut");

    // Get max chroma for this hue at this lightness
    let max_c = oklch.estimate_max_chroma();
    assert!(max_c > 0.0, "Max chroma should be positive");

    // Calculate luminance
    let y_srgb = relative_luminance_srgb(&gray).value();
    let y_apca = relative_luminance_apca(&gray).value();

    // Both should be moderate for mid-gray (~0.21)
    assert!(
        y_srgb > 0.1 && y_srgb < 0.4,
        "Gray sRGB luminance should be moderate, got {}",
        y_srgb
    );
    assert!(
        y_apca > 0.1 && y_apca < 0.4,
        "Gray APCA luminance should be moderate, got {}",
        y_apca
    );
}

#[test]
fn test_integration_saturated_color_workflow() {
    // Test with a saturated color (may or may not be in gamut)
    let blue = Color::from_srgb8(59, 130, 246);

    // Convert to OKLCH
    let oklch = OKLCH::from_color(&blue);

    // Blue-500 has moderate chroma
    assert!(oklch.c > 0.05, "Blue-500 should have noticeable chroma");
    assert!(
        oklch.l > 0.5 && oklch.l < 0.8,
        "Blue-500 should have moderate lightness"
    );

    // Map to gamut and verify it's valid
    let gamut_safe = oklch.map_to_gamut();
    assert!(gamut_safe.l == oklch.l, "Lightness should be preserved");
    assert!(gamut_safe.h == oklch.h, "Hue should be preserved");
    assert!(gamut_safe.c <= oklch.c, "Chroma should not increase");

    // Calculate luminance
    let y_srgb = relative_luminance_srgb(&blue).value();
    assert!(
        y_srgb > 0.1 && y_srgb < 0.5,
        "Blue-500 sRGB luminance should be moderate, got {}",
        y_srgb
    );
}

#[test]
fn test_integration_gamma_in_color_creation() {
    // Verify gamma is used correctly in Color creation
    let mid_gray = Color::from_srgb8(128, 128, 128);

    // The linear value should NOT be 0.5
    // sRGB 128/255 ≈ 0.502 → linear ≈ 0.216
    let expected_linear = srgb_to_linear(128.0 / 255.0);
    assert!(
        (mid_gray.linear[0] - expected_linear).abs() < 0.001,
        "Color.linear should use gamma: expected {}, got {}",
        expected_linear,
        mid_gray.linear[0]
    );
}

// ============================================================================
// Feature-Gated Tests (internals)
// ============================================================================

#[cfg(feature = "internals")]
mod internals_tests {
    use momoto_core::matrices::{LAB_TO_LMS, LMS_TO_LAB, LMS_TO_RGB, RGB_TO_LMS};

    #[test]
    fn test_rgb_to_lms_matrix() {
        // Matrix should sum to approximately 1 for each row
        // (preserves energy for white)
        for row in &RGB_TO_LMS {
            let sum: f64 = row.iter().sum();
            assert!(
                (sum - 1.0).abs() < 0.01,
                "RGB_TO_LMS row should sum to ~1: {}",
                sum
            );
        }
    }

    #[test]
    fn test_lms_to_lab_matrix() {
        // Verify matrix is accessible
        assert_eq!(LMS_TO_LAB.len(), 3);
        assert_eq!(LMS_TO_LAB[0].len(), 3);
    }

    #[test]
    fn test_lab_to_lms_matrix() {
        // First row should have 1.0 as first element
        assert!((LAB_TO_LMS[0][0] - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_lms_to_rgb_matrix() {
        // Verify matrix is accessible
        assert_eq!(LMS_TO_RGB.len(), 3);
        assert_eq!(LMS_TO_RGB[0].len(), 3);
    }
}

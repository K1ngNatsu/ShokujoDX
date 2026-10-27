//! Deterministic math functions.
//!
//! Platform math libraries can give different results in the last bit for
//! transcendental functions. These wrappers use the `libm` crate, which gives
//! the same result on all platforms. Use these functions in all engine crates.
//! `sqrt` and basic arithmetic are exact under IEEE 754, so `std` is safe for them.

pub fn sin(x: f64) -> f64 {
    libm::sin(x)
}

pub fn cos(x: f64) -> f64 {
    libm::cos(x)
}

pub fn tan(x: f64) -> f64 {
    libm::tan(x)
}

pub fn atan2(y: f64, x: f64) -> f64 {
    libm::atan2(y, x)
}

pub fn hypot(x: f64, y: f64) -> f64 {
    libm::hypot(x, y)
}

pub fn pow(x: f64, y: f64) -> f64 {
    libm::pow(x, y)
}

pub fn cbrt(x: f64) -> f64 {
    libm::cbrt(x)
}

pub fn exp(x: f64) -> f64 {
    libm::exp(x)
}

/// The natural logarithm.
pub fn ln(x: f64) -> f64 {
    libm::log(x)
}

/// Converts degrees to radians.
pub fn to_radians(deg: f64) -> f64 {
    deg * (core::f64::consts::PI / 180.0)
}

/// Converts radians to degrees.
pub fn to_degrees(rad: f64) -> f64 {
    rad * (180.0 / core::f64::consts::PI)
}

/// Returns the sine and the cosine of an angle in degrees.
///
/// Multiples of 90° give exact values (0, 1 or -1), so quarter turns do not
/// add rounding errors.
pub fn sin_cos_deg(deg: f64) -> (f64, f64) {
    let r = deg.rem_euclid(360.0);
    if r == 0.0 {
        (0.0, 1.0)
    } else if r == 90.0 {
        (1.0, 0.0)
    } else if r == 180.0 {
        (0.0, -1.0)
    } else if r == 270.0 {
        (-1.0, 0.0)
    } else {
        let rad = to_radians(r);
        (sin(rad), cos(rad))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quarter_turns_are_exact() {
        assert_eq!(sin_cos_deg(90.0), (1.0, 0.0));
        assert_eq!(sin_cos_deg(-90.0), (-1.0, 0.0));
        assert_eq!(sin_cos_deg(450.0), (1.0, 0.0));
    }

    #[test]
    fn general_angle_is_close() {
        let (s, c) = sin_cos_deg(30.0);
        assert!((s - 0.5).abs() < 1e-12);
        assert!((c - 0.75_f64.sqrt()).abs() < 1e-12);
    }
}

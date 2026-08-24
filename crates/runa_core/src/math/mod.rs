//! Interpolation, easing, and smoothing utilities.
//!
//! Modeled after Unity's `Mathf` — provides `lerp`, `smooth_damp`,
//! `smooth_step`, easing curves, and extension methods for `f32`, `Vec2`, etc.

pub mod collision2d;
pub mod collision3d;

use glam::{Vec2, Vec3, Vec4};

// ── scalar lerp ──────────────────────────────────────────────────────

/// Clamped linear interpolation: `a + (b - a) * t.clamp(0, 1)`.
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

/// Unclamped linear interpolation.
pub fn lerp_unclamped(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Returns `t` such that `lerp(a, b, t) == value`.
/// Clamped to `[0, 1]` when `value` is within `[a, b]`.
pub fn inverse_lerp(a: f32, b: f32, value: f32) -> f32 {
    if (a - b).abs() < f32::EPSILON {
        return 0.0;
    }
    ((value - a) / (b - a)).clamp(0.0, 1.0)
}

/// Maps `value` from the source range `[from_a, from_b]` to the
/// destination range `[to_a, to_b]`.
pub fn remap(value: f32, from_a: f32, from_b: f32, to_a: f32, to_b: f32) -> f32 {
    let t = ((value - from_a) / (from_b - from_a)).clamp(0.0, 1.0);
    to_a + (to_b - to_a) * t
}

// ── angle lerp ──────────────────────────────────────────────────────

/// Lerp between two angles in degrees, taking the shortest path.
pub fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
    let mut delta = (b - a) % 360.0;
    if delta > 180.0 {
        delta -= 360.0;
    } else if delta < -180.0 {
        delta += 360.0;
    }
    a + delta * t.clamp(0.0, 1.0)
}

// ── smooth step ─────────────────────────────────────────────────────

/// Hermite interpolation (3rd-order smooth step).
pub fn smooth_step(a: f32, b: f32, t: f32) -> f32 {
    let t = ((t - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// 5th-order smoother step.
pub fn smoother_step(a: f32, b: f32, t: f32) -> f32 {
    let t = ((t - a) / (b - a)).clamp(0.0, 1.0);
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

// ── easing curves (normalised time 0..1) ────────────────────────────

pub fn ease_in_quad(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t
}

pub fn ease_out_quad(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    -t * (t - 2.0)
}

pub fn ease_in_out_quad(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        2.0 * t * t
    } else {
        -1.0 + (4.0 - 2.0 * t) * t
    }
}

pub fn ease_in_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * t
}

pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let t = t - 1.0;
    t * t * t + 1.0
}

pub fn ease_in_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        let t = 2.0 * t - 2.0;
        0.5 * t * t * t + 1.0
    }
}

pub fn ease_in_expo(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t <= 0.0 {
        0.0
    } else {
        2.0_f32.powf(10.0 * (t - 1.0))
    }
}

pub fn ease_out_expo(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t >= 1.0 {
        1.0
    } else {
        1.0 - 2.0_f32.powf(-10.0 * t)
    }
}

pub fn ease_in_out_expo(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t <= 0.0 || t >= 1.0 {
        return t;
    }
    if t < 0.5 {
        0.5 * 2.0_f32.powf(20.0 * t - 10.0)
    } else {
        -0.5 * 2.0_f32.powf(-20.0 * t + 10.0) + 1.0
    }
}

pub fn ease_in_elastic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t <= 0.0 || t >= 1.0 {
        return t;
    }
    let p = 0.3;
    let _s = p / 4.0;
    -(2.0_f32.powf(10.0 * (t - 1.0)) * ((t - 1.0 - _s) * std::f32::consts::TAU / p).sin())
}

pub fn ease_out_elastic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t <= 0.0 || t >= 1.0 {
        return t;
    }
    let p = 0.3;
    let _s = p / 4.0;
    2.0_f32.powf(-10.0 * t) * ((t - _s) * std::f32::consts::TAU / p).sin() + 1.0
}

pub fn ease_in_out_elastic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t <= 0.0 || t >= 1.0 {
        return t;
    }
    let p = 0.45;
    if t < 0.5 {
        -0.5 * 2.0_f32.powf(20.0 * t - 10.0)
            * ((20.0 * t - 11.125) * (2.0 * std::f32::consts::PI / p)).sin()
    } else {
        0.5 * 2.0_f32.powf(-20.0 * t + 10.0)
            * ((20.0 * t - 11.125) * (2.0 * std::f32::consts::PI / p)).sin()
            + 1.0
    }
}

pub fn ease_in_bounce(t: f32) -> f32 {
    1.0 - ease_out_bounce(1.0 - t.clamp(0.0, 1.0))
}

pub fn ease_out_bounce(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 1.0 / 2.75 {
        7.5625 * t * t
    } else if t < 2.0 / 2.75 {
        let t = t - 1.5 / 2.75;
        7.5625 * t * t + 0.75
    } else if t < 2.5 / 2.75 {
        let t = t - 2.25 / 2.75;
        7.5625 * t * t + 0.9375
    } else {
        let t = t - 2.625 / 2.75;
        7.5625 * t * t + 0.984375
    }
}

// ── smooth damp (Unity-style spring physics) ────────────────────────

/// Smoothly moves `current` toward `target` with spring-physics damping.
///
/// Parameters:
/// - `current` — the current value
/// - `target` — target value
/// - `current_velocity` — in/out velocity (modified each call)
/// - `smooth_time` — approximate time to reach target (seconds)
/// - `max_speed` — optional speed clamp
/// - `delta_time` — frame time
///
/// Returns the new smoothed value.
pub fn smooth_damp(
    current: f32,
    target: f32,
    current_velocity: &mut f32,
    smooth_time: f32,
    max_speed: f32,
    delta_time: f32,
) -> f32 {
    let omega = 2.0 / smooth_time.max(0.0001);
    let x = omega * delta_time;
    let exp = 1.0 / (1.0 + x + 0.48 * x * x + 0.235 * x * x * x);
    let mut change = current - target;
    let max_change = max_speed * smooth_time;
    change = change.clamp(-max_change, max_change);
    let target = current - change;
    let temp = (*current_velocity + omega * change) * delta_time;
    *current_velocity = (*current_velocity - omega * temp) * exp;
    target + (change + temp) * exp
}

/// `smooth_damp` with no speed limit.
pub fn smooth_damp_unlimited(
    current: f32,
    target: f32,
    current_velocity: &mut f32,
    smooth_time: f32,
    delta_time: f32,
) -> f32 {
    smooth_damp(
        current,
        target,
        current_velocity,
        smooth_time,
        f32::INFINITY,
        delta_time,
    )
}

// ── move towards ────────────────────────────────────────────────────

/// Moves `current` toward `target` by `max_delta` (clamped step).
pub fn move_towards(current: f32, target: f32, max_delta: f32) -> f32 {
    if (target - current).abs() <= max_delta {
        target
    } else {
        current + (target - current).signum() * max_delta
    }
}

/// Moves an angle in degrees toward `target` by `max_delta`.
pub fn move_towards_angle(current: f32, target: f32, max_delta: f32) -> f32 {
    let delta = (target - current) % 360.0;
    let delta = if delta > 180.0 {
        delta - 360.0
    } else if delta < -180.0 {
        delta + 360.0
    } else {
        delta
    };
    if delta.abs() <= max_delta {
        target
    } else {
        current + delta.signum() * max_delta
    }
}

// ── vector extensions ───────────────────────────────────────────────

/// Extension trait adding `lerp`, `smooth_damp`, etc. to `f32`.
pub trait LerpExt {
    fn lerp(self, other: Self, t: f32) -> Self;
    fn lerp_unclamped(self, other: Self, t: f32) -> Self;
    fn smooth_step(self, other: Self, t: f32) -> Self;
    fn move_towards(self, other: Self, max_delta: f32) -> Self;
}

impl LerpExt for f32 {
    fn lerp(self, other: Self, t: f32) -> Self {
        self + (other - self) * t.clamp(0.0, 1.0)
    }

    fn lerp_unclamped(self, other: Self, t: f32) -> Self {
        self + (other - self) * t
    }

    fn smooth_step(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        self + (other - self) * (t * t * (3.0 - 2.0 * t))
    }

    fn move_towards(self, other: Self, max_delta: f32) -> Self {
        if (other - self).abs() <= max_delta {
            other
        } else {
            self + (other - self).signum() * max_delta
        }
    }
}

macro_rules! impl_lerp_ext_for_vec {
    ($ty:ident) => {
        impl LerpExt for $ty {
            fn lerp(self, other: Self, t: f32) -> Self {
                Self::lerp(self, other, t.clamp(0.0, 1.0))
            }

            fn lerp_unclamped(self, other: Self, t: f32) -> Self {
                Self::lerp(self, other, t)
            }

            fn smooth_step(self, other: Self, t: f32) -> Self {
                let t = t.clamp(0.0, 1.0);
                let s = t * t * (3.0 - 2.0 * t);
                Self::lerp(self, other, s)
            }

            fn move_towards(self, other: Self, max_delta: f32) -> Self {
                let diff = other - self;
                let dist = diff.length();
                if dist <= max_delta {
                    other
                } else {
                    self + diff / dist * max_delta
                }
            }
        }
    };
}

impl_lerp_ext_for_vec!(Vec2);
impl_lerp_ext_for_vec!(Vec3);
impl_lerp_ext_for_vec!(Vec4);

/// `smooth_damp` for `Vec3` — dampens each component independently.
pub fn smooth_damp_vec3(
    current: Vec3,
    target: Vec3,
    current_velocity: &mut Vec3,
    smooth_time: f32,
    max_speed: f32,
    delta_time: f32,
) -> Vec3 {
    Vec3::new(
        smooth_damp(
            current.x,
            target.x,
            &mut current_velocity.x,
            smooth_time,
            max_speed,
            delta_time,
        ),
        smooth_damp(
            current.y,
            target.y,
            &mut current_velocity.y,
            smooth_time,
            max_speed,
            delta_time,
        ),
        smooth_damp(
            current.z,
            target.z,
            &mut current_velocity.z,
            smooth_time,
            max_speed,
            delta_time,
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lerp_clamps_t() {
        assert_eq!(lerp(0.0, 10.0, -0.5), 0.0);
        assert_eq!(lerp(0.0, 10.0, 1.5), 10.0);
    }

    #[test]
    fn lerp_midpoint() {
        assert!((lerp(0.0, 10.0, 0.5) - 5.0).abs() < 1e-6);
    }

    #[test]
    fn inverse_lerp_works() {
        assert!((inverse_lerp(0.0, 10.0, 5.0) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn smooth_step_midpoint() {
        let v = smooth_step(0.0, 1.0, 0.5);
        assert!((v - 0.5).abs() < 1e-6);
    }

    #[test]
    fn ease_in_out_quad_symmetric() {
        let mid = ease_in_out_quad(0.5);
        assert!((mid - 0.5).abs() < 1e-6);
        let left = ease_in_out_quad(0.25);
        let right = ease_in_out_quad(0.75);
        assert!((left + right - 1.0).abs() < 1e-6);
    }

    #[test]
    fn smooth_damp_converges() {
        let mut vel = 0.0;
        let mut val = 0.0;
        for _ in 0..60 {
            val = smooth_damp(val, 10.0, &mut vel, 0.5, f32::INFINITY, 1.0 / 60.0);
        }
        assert!((val - 10.0).abs() < 1.0);
    }

    #[test]
    fn move_towards_clamps() {
        assert_eq!(move_towards(0.0, 10.0, 3.0), 3.0);
        assert_eq!(move_towards(0.0, 10.0, 100.0), 10.0);
    }

    #[test]
    fn f32_lerp_ext() {
        let v = 0.0_f32.lerp(10.0, 0.5);
        assert!((v - 5.0).abs() < 1e-6);
    }

    #[test]
    fn vec3_lerp_ext() {
        let v = Vec3::ZERO.lerp(Vec3::ONE, 0.5);
        assert!((v.x - 0.5).abs() < 1e-6);
    }
}

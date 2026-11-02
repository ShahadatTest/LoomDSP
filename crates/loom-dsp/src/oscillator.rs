//! Wave shapes from a phase.
//!
//! Each shape takes a phase instead of a frequency, and works out the phase
//! step itself. The band-limited shapes use polynomial corrections. Background:
//! V. Välimäki and A. Huovilainen, "Antialiasing Oscillators in Subtractive
//! Synthesis", IEEE Signal Processing Magazine, 2007.

use std::f64::consts::TAU;

/// Polynomial band-limited step. Smooths the jump at the wrap point, which
/// removes most of the aliasing a naive saw produces.
#[inline]
pub fn poly_blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let x = t / dt;
        2.0 * x - x * x - 1.0
    } else if t > 1.0 - dt {
        let x = (t - 1.0) / dt;
        x * x + 2.0 * x + 1.0
    } else {
        0.0
    }
}

/// Polynomial band-limited ramp: the integral of a two-sample polynomial step.
/// Smooths a unit slope change at `t = 0`, rounding the sharp corners of a
/// triangle wave without a leaky integrator.
#[inline]
pub fn poly_blamp(t: f32, dt: f32) -> f32 {
    if t < dt {
        let u = 1.0 - t / dt;
        (1.0 / 6.0) * u * u * u
    } else if t > 1.0 - dt {
        let u = (t - 1.0) / dt + 1.0;
        (1.0 / 6.0) * u * u * u
    } else {
        0.0
    }
}

/// A band-limited saw.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Saw {
    previous: f32,
}

impl Saw {
    /// Creates a saw at phase 0.
    pub fn new() -> Self {
        Self::default()
    }

    /// Back to phase 0.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// The wave at `phase`, which wraps to 0 to 1.
    #[inline]
    pub fn tick(&mut self, phase: f32) -> f32 {
        let t = phase - phase.floor();
        // The phase step tells us the frequency, so the shaper does not
        // need a separate frequency input.
        let mut dt = t - self.previous;
        dt -= dt.floor();
        self.previous = t;
        let mut y = 2.0 * t - 1.0;
        if dt > 0.0 && dt < 0.5 {
            y -= poly_blep(t, dt);
        }
        y
    }
}

/// A band-limited pulse.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pulse {
    previous: f32,
    previous_fall: f32,
}

impl Default for Pulse {
    fn default() -> Self {
        Self::new()
    }
}

impl Pulse {
    /// Creates a pulse at phase 0, as if the width had been 0.5.
    pub fn new() -> Self {
        Self {
            previous: 0.0,
            previous_fall: 0.5,
        }
    }

    /// Back to phase 0 at `width`.
    pub fn reset(&mut self, width: f32) {
        self.previous = 0.0;
        self.previous_fall = 1.0 - width;
    }

    /// The wave at `phase`, which wraps to 0 to 1. `width` must be above 0 and
    /// below 1.
    #[inline]
    pub fn tick(&mut self, phase: f32, width: f32) -> f32 {
        let t = phase - phase.floor();
        let mut dt = t - self.previous;
        dt -= dt.floor();
        self.previous = t;

        let mut t_fall = t - width;
        t_fall -= t_fall.floor();
        let mut dt_fall = t_fall - self.previous_fall;
        dt_fall -= dt_fall.floor();
        self.previous_fall = t_fall;

        let mut y = if t < width { 1.0 } else { -1.0 };
        let step = dt.min(1.0 - dt);
        let step_fall = dt_fall.min(1.0 - dt_fall);
        // When the pulse or gap is narrower than `step + step_fall`, the
        // two BLEP windows overlap; use a shared step so their slopes cancel.
        let (step, step_fall) = if width < step + step_fall || 1.0 - width < step + step_fall {
            let shared = step.max(step_fall);
            (shared, shared)
        } else {
            (step, step_fall)
        };
        if step > 0.0 && step < 0.5 {
            // Jump at the start of the cycle (+2 ascending, -2 descending).
            y += poly_blep(t, step);
        }
        // Jump when `t` crosses `width` (-2 ascending, +2 descending).
        if step_fall > 0.0 && step_fall < 0.5 {
            y -= poly_blep(t_fall, step_fall);
        }
        y.clamp(-1.0, 1.0)
    }
}

/// A band-limited triangle, aligned with a sine: 0 at phase 0, +1 at 0.25, 0
/// at 0.5, and -1 at 0.75.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Triangle {
    previous: f32,
}

impl Triangle {
    /// Creates a triangle at phase 0.
    pub fn new() -> Self {
        Self::default()
    }

    /// Back to phase 0.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// The wave at `phase`, which wraps to 0 to 1.
    #[inline]
    pub fn tick(&mut self, phase: f32) -> f32 {
        let t = phase - phase.floor();
        let mut dt = t - self.previous;
        dt -= dt.floor();
        self.previous = t;
        let mut t_top = t - 0.25;
        t_top -= t_top.floor();
        let mut y = 4.0 * (t_top - 0.5).abs() - 1.0;
        let step = dt.min(1.0 - dt);
        if step > 0.0 && step < 0.5 {
            let mut t_bottom = t - 0.75;
            t_bottom -= t_bottom.floor();
            // Slope changes by -8*step at the top peak and +8*step at the bottom peak.
            y += 8.0 * step * (poly_blamp(t_bottom, step) - poly_blamp(t_top, step));
        }
        y
    }
}

/// A wave with sharp corners and no band limiting, for slow modulation. Every
/// shape crosses 0 going up at phase 0, so a sine and a triangle line up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlowWave {
    /// A sine.
    Sine,
    /// A triangle.
    Triangle,
    /// A ramp that rises.
    RampUp,
    /// A ramp that falls.
    RampDown,
    /// A square.
    Square,
}

impl SlowWave {
    /// The wave at phase `p` in 0 to 1.
    #[inline]
    pub fn value(self, p: f64) -> f32 {
        let fract = |x: f64| x - x.floor();
        let value = match self {
            Self::Sine => (TAU * p).sin(),
            Self::Triangle => 1.0 - 4.0 * (fract(p + 0.25) - 0.5).abs(),
            Self::RampUp => 2.0 * fract(p + 0.5) - 1.0,
            Self::RampDown => -(2.0 * fract(p + 0.5) - 1.0),
            Self::Square => {
                if p < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
        };
        value as f32
    }
}

#[cfg(test)]
mod tests;

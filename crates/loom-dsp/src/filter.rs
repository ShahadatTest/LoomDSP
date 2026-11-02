//! Filters.

pub mod dc_blocker;
pub mod ladder;
pub mod one_pole;
pub mod svf;

use std::f32::consts::PI;

/// The integrator gain for a cutoff under the bilinear transform. Warping the
/// cutoff with `tan` puts it exactly where it should be at any sample rate.
///
/// `cutoff_hz` must be above 0 and below half of `sample_rate`.
#[inline]
pub fn prewarp(cutoff_hz: f32, sample_rate: f32) -> f32 {
    (PI * cutoff_hz / sample_rate).tan()
}

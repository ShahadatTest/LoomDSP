use super::*;
use crate::testing::alias_share;

/// A high note that aliases badly when it is naive.
const NOTE: u32 = 3_517;

/// The correction removes at least 90% of a saw's alias power.
#[test]
fn saw_has_far_less_aliasing_than_a_naive_saw() {
    let naive = alias_share(NOTE, |t| 2.0 * t - 1.0);
    let mut saw = Saw::new();
    let corrected = alias_share(NOTE, |t| saw.tick(t));
    assert!(corrected < naive * 0.1, "{corrected} against {naive}");
}

/// The correction removes at least 90% of a pulse's alias power.
#[test]
fn pulse_has_far_less_aliasing_than_a_naive_pulse() {
    let naive = alias_share(NOTE, |t| if t < 0.5 { 1.0 } else { -1.0 });
    let mut pulse = Pulse::new();
    let corrected = alias_share(NOTE, |t| pulse.tick(t, 0.5));
    assert!(corrected < naive * 0.1, "{corrected} against {naive}");
}

/// The correction removes at least 90% of a triangle's alias power.
#[test]
fn triangle_has_far_less_aliasing_than_a_naive_triangle() {
    let naive = alias_share(NOTE, |t| 1.0 - 4.0 * ((t + 0.25).fract() - 0.5).abs());
    let mut triangle = Triangle::new();
    let corrected = alias_share(NOTE, |t| triangle.tick(t));
    assert!(corrected < naive * 0.1, "{corrected} against {naive}");
}

/// The corrections only act near a jump.
#[test]
fn corrections_are_zero_away_from_the_edge() {
    assert_eq!(poly_blep(0.5, 0.01), 0.0);
    assert_eq!(poly_blamp(0.5, 0.01), 0.0);
}

/// A pulse stays in range when its width moves fast, even when the two edges
/// overlap.
#[test]
fn pulse_stays_in_range_with_moving_width() {
    let mut pulse = Pulse::new();
    for i in 0..48_000 {
        let phase = (i as f32 * 0.21).fract();
        let width = 0.5 + 0.49 * (i as f32 * 0.37).sin();
        let y = pulse.tick(phase, width);
        assert!((-1.0..=1.0).contains(&y), "frame {i}: {y}");
    }
}

/// A descending phase works as well as a rising one: a saw comes out reversed.
#[test]
fn descending_phase_runs_the_saw_backward() {
    let mut saw = Saw::new();
    saw.tick(0.5);
    assert!((saw.tick(0.4) - -0.2).abs() < 1e-5);
}

/// Every slow wave crosses 0 going up at phase 0, except the square, which
/// starts high.
#[test]
fn slow_waves_line_up() {
    let waves = [SlowWave::Sine, SlowWave::Triangle, SlowWave::RampUp];
    for wave in waves {
        assert!(wave.value(0.0).abs() < 1e-6, "{wave:?}");
        assert!(wave.value(0.01) > 0.0, "{wave:?}");
    }
    assert!(SlowWave::RampDown.value(0.0).abs() < 1e-6);
    assert!(SlowWave::RampDown.value(0.01) < 0.0);
    assert_eq!(SlowWave::Square.value(0.25), 1.0);
    assert_eq!(SlowWave::Square.value(0.75), -1.0);
}

/// `reset` returns each shape to phase 0, so the next sample is as new.
#[test]
fn reset_returns_to_the_start() {
    let mut saw = Saw::new();
    saw.tick(0.3);
    saw.reset();
    assert_eq!(saw, Saw::new());
    let mut triangle = Triangle::new();
    triangle.tick(0.3);
    triangle.reset();
    assert_eq!(triangle, Triangle::new());
    let mut pulse = Pulse::new();
    pulse.tick(0.3, 0.5);
    pulse.reset(0.5);
    assert_eq!(pulse, Pulse::new());
}

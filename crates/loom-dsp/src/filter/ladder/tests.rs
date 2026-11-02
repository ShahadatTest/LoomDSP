use super::*;
use crate::filter::prewarp;
use crate::testing::{RATE, tone_gain, warped};

/// At a small level the saturation does nothing, so each slope matches the
/// closed-loop transfer `1 / ((1 + s)^N + k)` under the bilinear transform.
#[test]
fn small_signal_response_matches_the_theory() {
    let cutoff = 1_000.0;
    let g = prewarp(cutoff as f32, RATE);
    for (slope, stages) in [(0, 2), (1, 3), (2, 4)] {
        for k in [0.0, 1.0] {
            for ratio in [0.2, 0.5, 1.0, 2.0, 4.0] {
                let hz = cutoff * ratio;
                let w = warped(hz, cutoff);
                let (mut real, mut imaginary) = (1.0f64, 0.0f64);
                for _ in 0..stages {
                    (real, imaginary) = (real - imaginary * w, imaginary + real * w);
                }
                let expected = 1.0 / f64::hypot(real + k, imaginary);
                let mut filter = Ladder::new(RATE);
                filter.reset(slope);
                let measured = tone_gain(hz, 1e-5, |x| filter.tick(x, g, k as f32, 1.0));
                assert!(
                    (measured - expected).abs() < 0.01 * expected.max(0.01),
                    "{stages} stages, k {k}, {hz} Hz: {measured} against {expected}"
                );
            }
        }
    }
}

/// During a slope change the feedback gain never passes the limit of a
/// tap that is heard, never jumps, and ends at the new slope's limit.
#[test]
fn slope_change_keeps_the_gain_under_every_heard_limit() {
    for (from, to) in [(0, 2), (2, 0), (1, 2), (2, 1), (0, 1), (1, 0)] {
        let mut ladder = Ladder::new(RATE);
        ladder.reset(from);
        ladder.change_slope(to);
        let mut last = ladder.limit;
        for _ in 0..2_000 {
            let k = ladder.step_slope();
            for (tap, &w) in ladder.weights.iter().enumerate() {
                assert!(w == 0.0 || k <= K_MAX[tap], "{from} to {to}: k {k} on tap {tap}");
            }
            assert!((k - last).abs() <= ladder.limit_step * 1.001, "{from} to {to}: jump");
            last = k;
        }
        assert_eq!(ladder.weights[to], 1.0, "{from} to {to}");
        assert_eq!(last, K_MAX[to], "{from} to {to}");
    }
}

/// At full resonance the saturation keeps a ringing loop bounded.
#[test]
fn full_resonance_stays_bounded() {
    for (slope, k) in K_MAX.into_iter().enumerate() {
        let g = prewarp(2_000.0, RATE);
        let mut ladder = Ladder::new(RATE);
        ladder.reset(slope);
        ladder.tick(1.0, g, k, 1.0);
        for _ in 0..200_000 {
            let output = ladder.tick(0.0, g, k, 1.0);
            assert!(output.is_finite() && output.abs() < 16.0, "slope {slope}: {output}");
        }
    }
}

/// `clear_loop` empties the stages and leaves the slope alone. `reset` goes
/// straight to a slope.
#[test]
fn clearing_and_resetting() {
    let mut ladder = Ladder::new(RATE);
    ladder.reset(1);
    ladder.tick(1.0, 0.5, 2.0, 1.0);
    ladder.clear_loop();
    assert_eq!(ladder.target(), 1);
    assert_eq!(ladder.tick(0.0, 0.5, 2.0, 1.0), 0.0);
    ladder.change_slope(2);
    ladder.reset(0);
    assert_eq!(ladder.target(), 0);
    assert_eq!(ladder.step_slope(), K_MAX[0]);
}

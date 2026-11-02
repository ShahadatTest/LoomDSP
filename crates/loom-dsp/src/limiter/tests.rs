use super::*;
use crate::testing::RATE;

const CEILING: f32 = 0.9;
const LOOKAHEAD: usize = 96;

/// A limiter with a 100 ms release.
fn limiter() -> Limiter {
    Limiter::new(CEILING, LOOKAHEAD, 0.1, RATE)
}

/// No output ever passes the ceiling, in either mode, on any signal.
#[test]
fn output_never_passes_the_ceiling() {
    for lookahead in [false, true] {
        let mut limiter = limiter();
        for i in 0..50_000 {
            let t = i as f32;
            let x = 8.0 * (t * 0.03).sin() * (t * 0.0007).sin();
            let (l, r) = limiter.tick(x, -0.5 * x, lookahead);
            assert!(l.abs() <= CEILING && r.abs() <= CEILING, "frame {i}");
        }
    }
}

/// A signal under the ceiling passes unchanged, and the lookahead mode delays
/// it by exactly its lookahead.
#[test]
fn quiet_signals_pass_unchanged() {
    let input: Vec<f32> = (0..1_000).map(|i| 0.8 * (i as f32 * 0.1).sin()).collect();
    let mut direct = limiter();
    let mut ahead = limiter();
    assert_eq!(ahead.lookahead(), LOOKAHEAD);
    for (i, &x) in input.iter().enumerate() {
        assert_eq!(direct.tick(x, x, false), (x, x));
        let expected = if i >= LOOKAHEAD { input[i - LOOKAHEAD] } else { 0.0 };
        assert_eq!(ahead.tick(x, x, true), (expected, expected));
    }
}

/// One gain acts on both channels, so their balance does not move.
#[test]
fn channels_are_linked() {
    let mut limiter = limiter();
    let (l, r) = limiter.tick(2.0, 0.5, false);
    assert_eq!(l, CEILING);
    assert!((r - CEILING / 4.0).abs() < 1e-6);
}

/// After a peak the gain recovers by one time constant in the release time.
#[test]
fn release_follows_its_time_constant() {
    let mut limiter = limiter();
    limiter.tick(2.0 * CEILING, 0.0, false);
    let gain_drop = 0.5f32;
    for _ in 0..(0.1 * RATE) as usize - 1 {
        limiter.tick(0.0, 0.0, false);
    }
    let (output, _) = limiter.tick(0.1, 0.0, false);
    let expected = 0.1 * (1.0 - gain_drop * (-1.0f32).exp());
    assert!((output - expected).abs() < 1e-3, "{output} against {expected}");
}

/// With lookahead the gain falls in straight steps before the peak, with no
/// jump bigger than the ramp.
#[test]
fn lookahead_ramps_before_a_peak() {
    let mut limiter = limiter();
    let needed = CEILING / 2.0;
    let step = (1.0 - needed) / (LOOKAHEAD + 1) as f32;
    let mut gains = vec![];
    for i in 0..1_000 {
        let x = if i < 500 { 0.5 } else { 2.0 };
        let (out, _) = limiter.tick(x, x, true);
        if i > LOOKAHEAD {
            let delayed = if i - LOOKAHEAD < 500 { 0.5 } else { 2.0 };
            gains.push(out / delayed);
        }
    }
    for pair in gains.windows(2) {
        assert!(pair[1] - pair[0] < 1e-3 && pair[0] - pair[1] < step * 1.1, "{pair:?}");
    }
    assert!(gains.iter().copied().fold(1.0, f32::min) < needed + 1e-3);
}

/// `reset` returns to unity gain and a silent delay line, with no allocation
/// needed: capacity stays.
#[test]
fn reset_clears_the_state() {
    let mut limiter = limiter();
    limiter.tick(4.0, 4.0, true);
    limiter.reset();
    assert_eq!(limiter.tick(0.5, 0.5, true), (0.0, 0.0));
    assert_eq!(limiter.gain, 1.0);
}

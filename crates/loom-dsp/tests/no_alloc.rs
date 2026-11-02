//! A tick never allocates or frees, nor does `reset` or `copy_from` (ADR-0035).
//!
//! This test binary installs an allocator that aborts if any allocation or
//! deallocation happens inside `assert_no_alloc`. It is active in debug
//! builds, which is how `cargo test` runs. `assert_no_alloc` has no
//! `AllocDisabler` in release builds, so this file is empty there.

#![cfg(debug_assertions)]

use assert_no_alloc::{AllocDisabler, assert_no_alloc};
use loom_dsp::delay::DelayLine;
use loom_dsp::envelope::{Envelope, Settings, power};
use loom_dsp::filter::dc_blocker::DcBlocker;
use loom_dsp::filter::ladder::Ladder;
use loom_dsp::filter::one_pole::OnePole;
use loom_dsp::filter::{prewarp, svf};
use loom_dsp::limiter::Limiter;
use loom_dsp::noise::{Colour, Noise};
use loom_dsp::oscillator::{Pulse, Saw, SlowWave, Triangle};
use loom_dsp::slew::Slew;

#[global_allocator]
static ALLOCATOR: AllocDisabler = AllocDisabler;

const RATE: f32 = 48_000.0;
/// Enough frames to fill every queue and wrap every buffer in this file.
const FRAMES: usize = 1_000;

/// A test signal with falling peaks well over 1.
fn falling_peaks(i: usize) -> f32 {
    (100.0 - i as f32 * 0.05) * if i.is_multiple_of(2) { 1.0 } else { -1.0 }
}

/// The filters, including a slope change on the ladder.
#[test]
fn filters_never_allocate() {
    let mut state_variable = svf::Svf::new();
    let mut one_pole = OnePole::new();
    let mut dc = DcBlocker::new(5.0, RATE);
    let mut ladder = Ladder::new(RATE);
    assert_no_alloc(|| {
        for i in 0..FRAMES {
            let x = falling_peaks(i);
            state_variable.tick(x, svf::Coefficients::new(1_000.0, 0.5, RATE));
            one_pole.tick(x, prewarp(1_000.0, RATE));
            dc.tick(x);
            if i == 10 {
                ladder.change_slope(0);
            }
            let limit = ladder.step_slope();
            ladder.tick(x, prewarp(1_000.0, RATE), limit, 1.0);
        }
        state_variable.reset();
        one_pole.reset();
        dc.reset();
        ladder.reset(1);
        ladder.clear_loop();
    });
}

/// The oscillator shapes and the slow waves.
#[test]
fn shapes_never_allocate() {
    let (mut saw, mut pulse, mut triangle) = (Saw::new(), Pulse::new(), Triangle::new());
    assert_no_alloc(|| {
        for i in 0..FRAMES {
            let phase = (i as f32 * 0.013).fract();
            saw.tick(phase);
            pulse.tick(phase, 0.3);
            triangle.tick(phase);
            for wave in [
                SlowWave::Sine,
                SlowWave::Triangle,
                SlowWave::RampUp,
                SlowWave::RampDown,
                SlowWave::Square,
            ] {
                wave.value(f64::from(phase));
            }
        }
        saw.reset();
        pulse.reset(0.3);
        triangle.reset();
    });
}

/// Noise of each colour, with its restarts and state copy.
#[test]
fn noise_never_allocates() {
    let mut noise = Noise::new(RATE);
    let other = Noise::new(RATE);
    assert_no_alloc(|| {
        for _ in 0..FRAMES {
            for colour in [Colour::White, Colour::Pink, Colour::Brown] {
                noise.tick(colour);
            }
        }
        noise.restart(7);
        noise.restart_keeping_brown(8);
        noise.copy_brown_from(&other);
    });
}

/// The envelope through all its stages, and the slew limiter.
#[test]
fn envelope_and_slew_never_allocate() {
    let settings = Settings {
        attack: 0.001,
        decay: 0.001,
        sustain: 0.5,
        release: 0.001,
        ad: false,
        power: power(0.5),
    };
    let mut envelope = Envelope::new(RATE);
    let mut slew = Slew::new(RATE);
    assert_no_alloc(|| {
        for i in 0..FRAMES {
            envelope.tick(i < FRAMES / 2, settings);
            slew.tick(falling_peaks(i), 0.01, 0.01);
        }
        envelope.reset();
        slew.reset();
    });
}

/// The delay line and the all-pass, past the point where they wrap.
#[test]
fn delays_never_allocate() {
    let mut delay = DelayLine::new(0.002, RATE);
    let mut allpass = DelayLine::new(0.002, RATE);
    let other = DelayLine::new(0.002, RATE);
    assert_no_alloc(|| {
        for i in 0..FRAMES {
            delay.step_delay(falling_peaks(i), 20.5);
            allpass.step_allpass(falling_peaks(i), 20.5, 0.5);
        }
        assert!(delay.copy_from(&other));
        delay.clear();
    });
}

/// The limiter in each mode. Falling peaks keep every sample in the sliding
/// minimum, which fills its queue to the most it can hold.
#[test]
fn the_limiter_never_allocates() {
    let lookahead = 96;
    let mut limiter = Limiter::new(0.9, lookahead, 0.1, RATE);
    assert_no_alloc(|| {
        for i in 0..FRAMES {
            limiter.tick(falling_peaks(i), falling_peaks(i), true);
        }
        limiter.reset();
        for i in 0..FRAMES {
            limiter.tick(falling_peaks(i), falling_peaks(i), false);
        }
        limiter.reset();
    });
}

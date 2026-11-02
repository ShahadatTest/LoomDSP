//! Pure DSP algorithms, with no ports, graph, or real-time host (ADR-0035).
//!
//! An algorithm is a plain struct that holds state. Usually `tick` processes
//! one sample and `reset` clears it. Some differ because of what they do:
//! `DelayLine` has `step_delay` and `step_allpass` for its two uses of one
//! memory, and `clear`, and `Noise` has `restart`, because it needs a seed.
//! Coefficients are a separate small type, so the caller decides how often to
//! compute them.
//!
//! Each algorithm states the input it accepts. It does not guard against
//! values outside that range, and it does not smooth parameters: both belong
//! to the caller. There are two exceptions, because a caller cannot do them: a
//! guard inside a feedback path, and a check that keeps a read inside memory.
//! Processing a sample never allocates, locks, or does I/O.
//!
//! ```
//! use loom_dsp::filter::svf::{Coefficients, Svf};
//!
//! let coefficients = Coefficients::new(1_000.0, 1.0, 48_000.0);
//! let mut filter = Svf::new();
//! let outputs = filter.tick(0.5, coefficients);
//! assert!(outputs.low.is_finite());
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs, missing_debug_implementations)]
#![deny(
    clippy::missing_inline_in_public_items,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::dbg_macro,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented
)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod delay;
pub mod envelope;
pub mod filter;
pub mod limiter;
pub mod noise;
pub mod oscillator;
pub mod slew;

mod static_checks;
#[cfg(test)]
mod testing;

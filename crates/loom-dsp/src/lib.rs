//! Pure DSP algorithms, with no ports, graph, or real-time host (ADR-0035).
//!
//! An algorithm is a plain struct: it holds state, `tick` processes one
//! sample, and `reset` clears it. Coefficients are a separate small type, so
//! the caller decides how often to compute them.
//!
//! Each algorithm states the input it accepts. It does not guard against
//! values outside that range, and it does not smooth parameters: both belong
//! to the caller. The exception is a guard inside a feedback path, where a
//! caller cannot reach it. A `tick` never allocates, locks, or does I/O.
//!
//! ```
//! use loom_dsp::filter::svf::{Coefficients, Svf};
//!
//! let coefficients = Coefficients::new(1_000.0, 1.0, 48_000.0);
//! let mut filter = Svf::new();
//! let outputs = filter.tick(0.5, coefficients);
//! assert!(outputs.low.is_finite());
//! ```

pub mod delay;
pub mod envelope;
pub mod filter;
pub mod limiter;
pub mod noise;
pub mod oscillator;
pub mod slew;

#[cfg(test)]
mod testing;

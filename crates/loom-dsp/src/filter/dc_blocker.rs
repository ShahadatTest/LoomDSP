//! A first-order high-pass filter that removes a constant offset:
//! `y[n] = a * (y[n-1] + x[n] - x[n-1])`.

use std::f32::consts::TAU;

/// The state and setting of one DC blocker.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DcBlocker {
    coefficient: f32,
    previous_in: f32,
    previous_out: f32,
}

impl DcBlocker {
    /// Creates a blocker with its corner near `cutoff_hz`. The cutoff must be
    /// far below half of `sample_rate`.
    pub fn new(cutoff_hz: f32, sample_rate: f32) -> Self {
        Self {
            coefficient: (-TAU * cutoff_hz / sample_rate).exp(),
            previous_in: 0.0,
            previous_out: 0.0,
        }
    }

    /// Clears the memory.
    pub fn reset(&mut self) {
        self.previous_in = 0.0;
        self.previous_out = 0.0;
    }

    /// Processes one sample. `input` must be finite.
    #[inline]
    pub fn tick(&mut self, input: f32) -> f32 {
        // The difference first: added to a large held input, a small output
        // would round away and stop decaying.
        let mut output = self.coefficient * (self.previous_out + (input - self.previous_in));
        // A held offset decays toward 0 for ever. Stop it before it becomes
        // a denormal, which is slow on some CPUs.
        if output.abs() < 1e-20 {
            output = 0.0;
        }
        self.previous_in = input;
        self.previous_out = output;
        output
    }
}

#[cfg(test)]
mod tests;

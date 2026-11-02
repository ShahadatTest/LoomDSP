//! First-order filter with trapezoidal integration.
//!
//! Source: V. Zavalishin, "The Art of VA Filter Design".

/// Both responses for one sample.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Outputs {
    /// Low-pass.
    pub low: f32,
    /// High-pass: the input minus the low-pass.
    pub high: f32,
}

/// The running state of one filter.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct OnePole {
    state: f32,
}

impl OnePole {
    /// Creates a filter with cleared state.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears the filter's memory.
    #[inline]
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Processes one sample. `g` comes from [`super::prewarp`]. `input` must
    /// be finite: NaN or infinity would stay in the state until
    /// [`OnePole::reset`].
    #[inline]
    pub fn tick(&mut self, input: f32, g: f32) -> Outputs {
        debug_assert!(input.is_finite(), "input {input}");
        let v = (input - self.state) * g / (1.0 + g);
        let low = v + self.state;
        self.state = low + v;
        Outputs { low, high: input - low }
    }
}

#[cfg(test)]
mod tests;

//! A resonant low-pass made of one-pole stages in a loop, with saturation in
//! the feedback. Two, three, or four stages are heard.
//!
//! Sources:
//!
//! - T. Stilson and J. Smith, "Analyzing the Moog VCF with Considerations for
//!   Digital Implementation", Proc. ICMC 1996: the loop of identical one-pole
//!   stages with inverting feedback, and the gain where it starts to oscillate.
//! - V. Zavalishin, "The Art of VA Filter Design": the trapezoidal one-pole
//!   stage, the prewarped cutoff, and solving the feedback loop with no unit
//!   delay in it (zero-delay feedback).

/// The number of one-pole stages the ladder always runs.
const STAGES: usize = 4;

/// The feedback gain at full resonance for each slope (2, 3, and 4 stages).
///
/// For N identical stages `1 / (1 + s)` with feedback `-k`, the loop rings
/// forever when `(1 + s)^N = -k` has a root on the imaginary axis: `k = 8`
/// for N = 3 (at `√3` times the cutoff) and `k = 4` for N = 4 (at the
/// cutoff), as Stilson and Smith show. Two stages never reach it. The 18 and
/// 24 dB values are 5% above the limit: at the limit itself the loop only
/// keeps an oscillation that is already there, and the tanh in the feedback
/// pulls it below, so full resonance would not sing.
pub const K_MAX: [f32; 3] = [8.0, 8.4, 4.2];

/// How fast the feedback limit moves during a slope change, per second:
/// from 8.4 to 4.2 in 10 ms.
const LIMIT_RAMP: f32 = (8.4 - 4.2) / 0.01;

/// The running state of one ladder. A slope is an index: 0, 1, or 2 for 2, 3,
/// or 4 stages.
#[derive(Debug, Clone, PartialEq)]
pub struct Ladder {
    /// The slope the output is at or fading to.
    target: usize,
    /// How much each tap (after stage 2, 3, and 4) adds to the output and
    /// to the feedback. One of them is 1, except during a slope change.
    weights: [f32; 3],
    /// The weights when the current slope change began.
    from: [f32; 3],
    /// Progress of the slope crossfade, from 0 to 1. 1 means done.
    fade: f32,
    /// Progress per frame of the 10 ms crossfade.
    fade_step: f32,
    /// The feedback gain at full resonance now. During a slope change it
    /// moves toward the limit of the taps in use at [`LIMIT_RAMP`], so the
    /// gain never jumps.
    limit: f32,
    /// How far `limit` may move in one frame.
    limit_step: f32,
    /// The trapezoidal integrator state of each stage. All four always
    /// run, so a change of slope finds the stages already moving.
    state: [f32; STAGES],
    /// `tanh(y) / y` for the last output `y`: the slope of the line from
    /// the origin to the point on the tanh curve. See [`Ladder::tick`].
    saturation: f32,
}

impl Ladder {
    /// Creates a cleared ladder at four stages.
    pub fn new(sample_rate: f32) -> Self {
        Self {
            target: 2,
            weights: [0.0, 0.0, 1.0],
            from: [0.0, 0.0, 1.0],
            fade: 1.0,
            fade_step: 1.0 / (0.01 * sample_rate).max(1.0),
            limit: K_MAX[2],
            limit_step: LIMIT_RAMP / sample_rate,
            state: [0.0; STAGES],
            saturation: 1.0,
        }
    }

    /// Clears the loop and goes straight to `slope`, with no crossfade.
    pub fn reset(&mut self, slope: usize) {
        self.target = slope;
        self.weights = [0.0; 3];
        self.weights[slope] = 1.0;
        self.from = self.weights;
        self.fade = 1.0;
        self.limit = K_MAX[slope];
        self.clear_loop();
    }

    /// Clears only the stages and the saturation, so a bad value cannot stay
    /// in the loop. A slope change in progress goes on.
    pub fn clear_loop(&mut self) {
        self.state = [0.0; STAGES];
        self.saturation = 1.0;
    }

    /// The slope the output is at or fading to.
    #[inline]
    pub fn target(&self) -> usize {
        self.target
    }

    /// Starts a 10 ms crossfade from the current tap weights to `slope`.
    /// A change during a crossfade starts from where that one had got to.
    pub fn change_slope(&mut self, slope: usize) {
        self.target = slope;
        self.from = self.weights;
        self.fade = 0.0;
    }

    /// Moves a slope change on by one frame and returns the feedback gain
    /// at full resonance for this frame. Call it once per frame.
    ///
    /// The gain may never pass the limit of any tap that is heard or about
    /// to be heard. So when the new slope's limit is lower, the gain first
    /// moves down to it, with only the old taps heard, and the crossfade
    /// waits. When it is higher, the crossfade runs first, at the old gain,
    /// and the gain then moves up. The gain never jumps, so the output
    /// never clicks.
    pub fn step_slope(&mut self) -> f32 {
        let wanted = self
            .weights
            .iter()
            .zip(K_MAX)
            .enumerate()
            .filter(|&(tap, (&w, _))| tap == self.target || w > 0.0)
            .map(|(_, (_, k))| k)
            .fold(f32::INFINITY, f32::min);
        self.limit = if self.limit > wanted {
            (self.limit - self.limit_step).max(wanted)
        } else {
            (self.limit + self.limit_step).min(wanted)
        };
        if self.fade < 1.0 && self.limit <= K_MAX[self.target] {
            self.fade = (self.fade + self.fade_step).min(1.0);
            for (tap, weight) in self.weights.iter_mut().enumerate() {
                let to = if tap == self.target { 1.0 } else { 0.0 };
                *weight = self.from[tap] + (to - self.from[tap]) * self.fade;
            }
            if self.fade >= 1.0 {
                self.weights = [0.0; 3];
                self.weights[self.target] = 1.0;
            }
        }
        self.limit
    }

    /// One frame of the filter: input `x`, prewarped gain `g` from
    /// [`super::prewarp`], feedback gain `k`, input gain `gain`. Returns the
    /// output. `x` must be finite.
    ///
    /// Each stage is Zavalishin's trapezoidal one-pole: with
    /// `G = g / (1 + g)`, its output is `y = G·x + s / (1 + g)`, where `s`
    /// is its state. So every tap is a straight line in the loop input `u`:
    /// `y = a·u + b`. The loop is `u = gain·x - k·tanh(y)`. The tanh is
    /// replaced by its secant through the origin at the last output,
    /// `tanh(y) ≈ y · tanh(y_last) / y_last`, so `u` comes from one
    /// division, with no unit delay in the loop (zero-delay feedback). The
    /// secant slope is at most 1, so it only ever lowers the loop gain.
    #[inline]
    pub fn tick(&mut self, x: f32, g: f32, k: f32, gain: f32) -> f32 {
        let big_g = g / (1.0 + g);
        let carry = 1.0 / (1.0 + g);
        // The taps after stages 2, 3, and 4 as `a·u + b`.
        let (mut a, mut b) = (1.0, 0.0);
        let mut taps = [(0.0, 0.0); 3];
        for (stage, &s) in self.state.iter().enumerate() {
            a *= big_g;
            b = big_g * b + carry * s;
            if stage >= 1 {
                taps[stage - 1] = (a, b);
            }
        }
        let (mut fa, mut fb) = (0.0, 0.0);
        for (&(a, b), &w) in taps.iter().zip(&self.weights) {
            fa += w * a;
            fb += w * b;
        }
        let loop_gain = k * self.saturation;
        let u = (gain * x - loop_gain * fb) / (1.0 + loop_gain * fa);

        // Run the four stages on `u`.
        let mut input = u;
        let mut outs = [0.0; STAGES];
        for (s, out) in self.state.iter_mut().zip(&mut outs) {
            let v = (input - *s) * big_g;
            let y = v + *s;
            *s = y + v;
            *out = y;
            input = y;
        }
        let y: f32 = outs[1..].iter().zip(&self.weights).map(|(y, w)| y * w).sum();
        self.saturation = if y.abs() < 1e-4 { 1.0 } else { y.tanh() / y };
        y
    }
}

#[cfg(test)]
mod tests;

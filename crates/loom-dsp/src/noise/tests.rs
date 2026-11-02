use super::*;
use crate::testing::RATE;

const COLOURS: [Colour; 3] = [Colour::White, Colour::Pink, Colour::Brown];

/// The RMS of `frames` samples of a colour.
fn rms(colour: Colour, frames: usize) -> f32 {
    let mut noise = Noise::new(RATE);
    noise.restart(7);
    let sum: f32 = (0..frames).map(|_| noise.tick(colour).powi(2)).sum();
    (sum / frames as f32).sqrt()
}

/// One seed gives the same bits, and a restart repeats them (hard rule 3).
#[test]
fn a_seed_repeats_exactly() {
    for colour in COLOURS {
        let mut noise = Noise::new(RATE);
        noise.restart(99);
        let first: Vec<u32> = (0..1_000).map(|_| noise.tick(colour).to_bits()).collect();
        noise.restart(99);
        let second: Vec<u32> = (0..1_000).map(|_| noise.tick(colour).to_bits()).collect();
        assert_eq!(first, second, "{colour:?}");
    }
}

/// Two seeds give different numbers.
#[test]
fn seeds_differ() {
    let mut a = Noise::new(RATE);
    let mut b = Noise::new(RATE);
    a.restart(1);
    b.restart(2);
    assert_ne!(a.tick(Colour::White), b.tick(Colour::White));
}

/// White noise from -1 to 1 has an RMS of `1 / √3`, and the others match it.
#[test]
fn every_colour_has_the_same_rms() {
    let white = 1.0 / 3.0f32.sqrt();
    for colour in COLOURS {
        let measured = rms(colour, RATE as usize * 10);
        assert!((measured - white).abs() < 0.1, "{colour:?}: {measured}");
    }
}

/// A white stream is the same whichever colour is asked for.
#[test]
fn colours_share_the_white_stream() {
    let mut noise = Noise::new(RATE);
    noise.tick(Colour::Pink);
    noise.tick(Colour::Brown);
    let mut plain = Noise::new(RATE);
    plain.tick(Colour::White);
    plain.tick(Colour::White);
    assert_eq!(noise.state, plain.state);
}

/// The brown memory survives when asked to, so a brown signal does not jump,
/// and a full restart clears it.
#[test]
fn restarts_keep_or_clear_the_brown_memory() {
    let mut noise = Noise::new(RATE);
    for _ in 0..1_000 {
        noise.tick(Colour::Brown);
    }
    let brown = noise.brown;
    assert_ne!(brown, 0.0);
    noise.restart_keeping_brown(5);
    assert_eq!(noise.brown, brown);
    noise.restart(5);
    assert_eq!(noise.brown, 0.0);
}

/// A copy takes the brown memory and not the random stream.
#[test]
fn copy_brown_leaves_the_streams_alone() {
    let mut source = Noise::new(RATE);
    for _ in 0..100 {
        source.tick(Colour::Brown);
    }
    let mut copy = Noise::new(RATE);
    let streams = (copy.state, copy.aux);
    copy.copy_brown_from(&source);
    assert_eq!(copy.brown, source.brown);
    assert_eq!((copy.state, copy.aux), streams);
}

/// The value from one step stays from -1 to 1, and a stream never sticks at 0.
#[test]
fn white_stays_in_range() {
    let mut state = 1;
    for _ in 0..100_000 {
        let value = next_white(&mut state);
        assert!((-1.0..=1.0).contains(&value));
        assert_ne!(state, 0);
    }
}

/// No random stream repeats another at a shifted offset, within or across two
/// seeds. The two seeds are the ones `loom.osc.noise` gives its stereo channels.
#[test]
fn streams_from_two_seeds_do_not_overlap() {
    let (mut left, mut right) = (Noise::new(RATE), Noise::new(RATE));
    left.restart(42);
    right.restart(42 ^ 0x85EB_CA6B);
    let states = [left.state, left.aux, right.state, right.aux].map(|mut state| {
        let mut values: Vec<u32> = (0..4_096)
            .map(|_| {
                next_white(&mut state);
                state
            })
            .collect();
        values.sort_unstable();
        values
    });
    for (i, a) in states.iter().enumerate() {
        for (j, b) in states.iter().enumerate().skip(i + 1) {
            assert!(
                !a.iter().any(|x| b.binary_search(x).is_ok()),
                "streams {i} and {j} overlap"
            );
        }
    }
}

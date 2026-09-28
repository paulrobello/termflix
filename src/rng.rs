//! Seedable RNG source for animations (thread-local).
//!
//! Unseeded behavior matches `rand::rng()` (OS entropy). After [`set_seed`],
//! every [`new_rng`] call on this thread returns a distinct but reproducible
//! `StdRng` stream, so seeded runs produce byte-identical frames run to run.
use rand::{SeedableRng, rngs::StdRng};
use std::cell::Cell;

thread_local! {
    /// `Some((seed, next_stream))` when seeded on this thread.
    static STATE: Cell<Option<(u64, u64)>> = const { Cell::new(None) };
}

/// Seed this thread's RNG source. Call at startup and before each gallery capture / test case.
pub fn set_seed(seed: u64) {
    STATE.with(|s| s.set(Some((seed, 0))));
}

/// A new RNG: a deterministic stream per call when seeded, OS entropy otherwise.
pub fn new_rng() -> StdRng {
    STATE.with(|s| match s.get() {
        Some((seed, n)) => {
            s.set(Some((seed, n + 1)));
            StdRng::seed_from_u64(seed ^ n.wrapping_mul(0x9E37_79B9_7F4A_7C15))
        }
        None => rand::make_rng(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngExt;

    #[test]
    fn same_seed_same_sequence() {
        set_seed(7);
        let a: Vec<u64> = (0..4).map(|_| new_rng().random()).collect();
        set_seed(7);
        let b: Vec<u64> = (0..4).map(|_| new_rng().random()).collect();
        assert_eq!(a, b);
        STATE.with(|s| s.set(None));
    }

    #[test]
    fn different_seeds_differ() {
        set_seed(1);
        let a: u64 = new_rng().random();
        set_seed(2);
        let b: u64 = new_rng().random();
        assert_ne!(a, b);
        STATE.with(|s| s.set(None));
    }

    #[test]
    fn streams_within_a_seed_differ() {
        set_seed(9);
        let a: u64 = new_rng().random();
        let b: u64 = new_rng().random();
        assert_ne!(a, b);
        STATE.with(|s| s.set(None));
    }
}

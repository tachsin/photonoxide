//! A small seeded random number generator: xoshiro256** seeded through SplitMix64 (D. Blackman,
//! S. Vigna, ACM Trans. Math. Softw. 47, 36 (2021), doi:10.1145/3460772). The same seed gives
//! the same numbers on every system, which is all a sampler with an explicit seed needs.

use crate::portable;
use std::f64::consts::TAU;

/// One step of SplitMix64: the seed's next state and its output.
fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// xoshiro256**.
#[derive(Clone, Debug)]
pub(crate) struct Rng {
    s: [u64; 4],
}

impl Rng {
    /// A generator from a seed; streams for different `stream` values are independent in
    /// practice (each seeds SplitMix64 from the seed and the stream together).
    pub(crate) fn new(seed: u64, stream: u64) -> Rng {
        let mut state = seed ^ stream.wrapping_mul(0xd1b5_4a32_d192_ed03);
        let s = [
            splitmix(&mut state),
            splitmix(&mut state),
            splitmix(&mut state),
            splitmix(&mut state),
        ];
        Rng { s }
    }

    pub(crate) fn next_u64(&mut self) -> u64 {
        let result = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        result
    }

    /// Uniform on [0, 1), from the top 53 bits.
    pub(crate) fn uniform(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Uniform on 0..n, for n ≥ 1.
    pub(crate) fn below(&mut self, n: usize) -> usize {
        ((self.uniform() * n as f64) as usize).min(n - 1)
    }

    /// A standard normal number, by the Box–Muller transform.
    pub(crate) fn normal(&mut self) -> f64 {
        let u = 1.0 - self.uniform(); // (0, 1]
        let v = self.uniform();
        (-2.0 * portable::ln(u)).sqrt() * portable::cos(TAU * v)
    }

    /// An index drawn with probability proportional to `weights` (all ≥ 0, not all 0).
    pub(crate) fn pick(&mut self, weights: &[f64]) -> usize {
        let total: f64 = weights.iter().sum();
        let mut x = self.uniform() * total;
        for (i, &w) in weights.iter().enumerate() {
            if x < w {
                return i;
            }
            x -= w;
        }
        // round-off at the top end: the last index with a weight
        weights.iter().rposition(|&w| w > 0.0).unwrap_or(0)
    }
}

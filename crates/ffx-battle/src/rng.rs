//! The game's random number generators (`docs/rewrite-plan.md`, M1).
//!
//! - The global generator (`rnd`, `0x007989a0`): one 32-bit seed at
//!   `0x00c42200`.
//! - 68 indexed generators (`brnd`, `0x007988f0`): states at `0x01135ee0`,
//!   each with its own multiplier (`0x00c42208`, 68 × `u32`) and XOR
//!   constant (`0x00c42318`, 68 × `u16`).
//! - `MsRandMize` (`0x00798890`) seeds both from the clock seed.
//!
//! Every step multiplies, then recombines the halves of the product as
//! `(x >> 16) + (x << 16)` with an arithmetic shift: an add, not a rotate,
//! so a negative product carries into the high half. Results are the low
//! 31 bits.
//!
//! The multiplier and XOR tables are the game's data: [`bridge`] reads them
//! from game memory, and a host without `FFX.exe` running reads them from
//! the player's `FFX.exe`.

/// Generators in the indexed set.
pub const GENERATORS: u32 = 68;

fn mix(x: u32) -> u32 {
    (x.cast_signed() >> 16)
        .cast_unsigned()
        .wrapping_add(x << 16)
}

/// One step of the global generator: the new seed.
#[must_use]
pub fn step_global(seed: u32) -> u32 {
    mix(seed.wrapping_mul(0x5d58_8b65).wrapping_add(0x3c35))
}

/// One step of an indexed generator: its new state.
#[must_use]
pub fn step_indexed(state: u32, multiplier: u32, xor: u16) -> u32 {
    mix(multiplier.wrapping_mul(state) ^ u32::from(xor))
}

/// What a generator returns for a state.
#[must_use]
pub const fn output(state: u32) -> u32 {
    state & 0x7fff_ffff
}

/// The two seeds `MsRandMize(n)` derives from the clock seed.
#[must_use]
pub fn seeds(clock: u32, n: u32) -> (u32, u32) {
    let s = clock.wrapping_mul(n.wrapping_add(1));
    (
        s.wrapping_mul(0x420c_56d7).wrapping_add(0x2e0a),
        s.wrapping_mul(0x599e_67e6).wrapping_add(0x301d),
    )
}

/// The generators where the game keeps them, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use super::{GENERATORS, output, seeds, step_global, step_indexed};

    /// The global generator's seed.
    pub const SEED: Global<u32> = Global::new(0x00c4_2200);
    /// The second seed `MsRandMize` sets (read elsewhere).
    pub const SEED2: Global<u32> = Global::new(0x00c4_2204);
    /// The indexed generators' multipliers.
    pub const MULTIPLIERS: Global<u32> = Global::new(0x00c4_2208);
    /// The indexed generators' XOR constants.
    pub const XORS: Global<u16> = Global::new(0x00c4_2318);
    /// The indexed generators' states.
    pub const STATES: Global<u32> = Global::new(0x0113_5ee0);

    /// `rnd` (`0x007989a0`): steps the global generator.
    pub fn rnd(mem: &mut impl Mem) -> u32 {
        let seed = step_global(SEED.get(mem));
        SEED.set(mem, seed);
        output(seed)
    }

    /// `brnd(index)` (`0x007988f0`): steps generator `index`. Like the
    /// original, the index is not checked.
    pub fn brnd(mem: &mut impl Mem, index: u32) -> u32 {
        let state = STATES.ptr(mem).at(index);
        let next = step_indexed(
            state.get(mem),
            MULTIPLIERS.ptr(mem).at(index).get(mem),
            XORS.ptr(mem).at(index).get(mem),
        );
        state.set(mem, next);
        output(next)
    }

    /// `MsRandMize(n)` (`0x00798890`) given the clock seed (`Rng_ClockSeed`,
    /// which the caller asks the platform for).
    pub fn ms_rand_mize(mem: &mut impl Mem, clock: u32, n: u32) {
        let (a, b) = seeds(clock, n);
        SEED.set(mem, a);
        SEED2.set(mem, b);
        rnd(mem);
        for i in 0..GENERATORS {
            let v = rnd(mem);
            STATES.ptr(mem).at(i).set(mem, v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mix_adds_the_sign() {
        // 0x8000_0000 >> 16 (arithmetic) = 0xffff_8000; + (x << 16) = 0
        assert_eq!(mix(0x8000_0000), 0xffff_8000);
        assert_eq!(mix(0x0001_0002), 0x0002_0001);
    }
}

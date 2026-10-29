//! M5 stage 1, lists `small` and `main`, round 4: leftovers of the debug
//! tools (the movie camera's default path).

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // registers and indices
pub mod bridge {
    use ffx_graphic::host::Host;

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    /// The camera debug lists: the current list `0x025d5f90`, and per
    /// list the point flags `0x025d5f44[]`, the point count
    /// `0x025d5f4c[]` and the points `0x025d5f54[]`.
    pub const CUR_LIST: u32 = 0x025d_5f90;
    pub const LIST_FLAGS: u32 = 0x025d_5f44;
    pub const LIST_COUNT: u32 = 0x025d_5f4c;
    pub const LIST_POINTS: u32 = 0x025d_5f54;
    /// The `f32` constants 10, -10, -20, -30.
    pub const TEN: u32 = 0x00b9_2318;
    pub const MINUS_TEN: u32 = 0x00b3_04fc;
    pub const MINUS_TWENTY: u32 = 0x00b5_4d08;
    pub const MINUS_THIRTY: u32 = 0x00b9_235c;

    /// `FUN_00941980()` (cdecl): an empty current list gets five default
    /// points (16 bytes each: `x` 10, -10, -20, -30 for the first, third,
    /// fourth and fifth, 0 elsewhere) whose flag words (12 bytes apart)
    /// are set to 1.
    pub fn default_points(h: &mut impl Host) {
        let k: u32 = h.read(h.rebase(CUR_LIST));
        let count_at = at(h.rebase(LIST_COUNT), k.wrapping_mul(4));
        if h.read::<u32>(count_at) != 0 {
            return;
        }
        let flags: u32 = h.read(at(h.rebase(LIST_FLAGS), k.wrapping_mul(4)));
        let points: u32 = h.read(at(h.rebase(LIST_POINTS), k.wrapping_mul(4)));
        h.write(count_at, 5u32);
        let firsts = [
            Some(TEN),
            None,
            Some(MINUS_TEN),
            Some(MINUS_TWENTY),
            Some(MINUS_THIRTY),
        ];
        for (i, first) in (0u32..).zip(firsts) {
            h.write(at(flags, 12 * i), 1u32);
            // fld/fstp dword of a normal constant: its bits
            let x: u32 = match first {
                Some(c) => h.read(h.rebase(c)),
                None => 0,
            };
            let p = at(points, 16 * i);
            h.write(p, x);
            for j in 1..4 {
                h.write(at(p, 4 * j), 0u32);
            }
        }
    }
}

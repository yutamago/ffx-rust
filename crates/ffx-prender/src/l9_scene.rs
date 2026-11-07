//! M5 stage 1, round 9 (`tail9`, list `phyre-scene`):
//! `PContextSwitchLights_buildCombinations_004d4e90`, vtable slot 1 of
//! `Phyre_PRendering_PContextSwitchLights_vftable`: every combination of
//! up to `+0x38` (light, shadow caster) pairs the switch can be in, each
//! as a packed key (4 bits of pair count, then 10 bits a pair: caster id
//! `<< 5 | light id`, the pairs sorted), the distinct ones stored as word
//! arrays in the output `PArray` of 8-byte array headers, an empty one
//! last.
//!
//! The light list is copied into an alloca and sorted there by the
//! original's sort (its address goes to it), so the bridge is split at
//! the alloca ([`bridge::lights_size`]); the pair keys, the bit array
//! and its writer stay at the original's `ebp` offsets ([`bridge::FRAME`]).
//! Callees stay calls to the originals.

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // the original's registers
pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    /// The sorts (`std::sort` internals, cdecl): `(first, last, n, less)`
    /// and the insertion pass `(first, last, n)`; the light order
    /// `Phyre_PRendering_PContextSwitchLights_lessC_004d3bd0`.
    pub const SORT: u32 = 0x004d_20d0;
    pub const SORT_KEYS: u32 = 0x004d_1ff0;
    pub const LESS: u32 = 0x004d_3bd0;
    /// `Phyre_PRendering_PShadowCaster_CheckLightShadowCasterCompatibility
    /// (light, caster)` (cdecl, `al`); `PBitArray_setBits_004d55d0(bits;
    /// value, bit, count)` (thiscall, `ret 0xc`).
    pub const COMPATIBLE: u32 = 0x004c_ae20;
    pub const SET_BITS: u32 = 0x004d_55d0;
    /// `PhyreMalloc(size, align)`, `PhyreFree(p)`,
    /// `PhyreFreeHeaders_004d3720(headers, n)`, the CRT's `memcpy` and
    /// `memset` thunks (cdecl); `PArray_truncateHeaders_004d58b0(this; n)`
    /// (thiscall, `ret 4`).
    pub const MALLOC: u32 = 0x0042_fb80;
    pub const FREE: u32 = 0x0042_fb20;
    pub const FREE_HEADERS: u32 = 0x004d_3720;
    pub const MEMCPY: u32 = 0x0094_9256;
    pub const MEMSET: u32 = 0x0094_964c;
    pub const TRUNCATE: u32 = 0x004d_58b0;

    /// The bytes of the original's frame below `ebp` the bridge uses: the
    /// pair keys (`ebp - 0xe4`, 32 words), the bit writer (`ebp - 0xf4`:
    /// a pointer to the words), the counters (`ebp - 0xa4`, 32 words), the
    /// key words (`ebp - 0x24`, 8 words).
    pub const FRAME: u32 = 0xf4;

    const MASK: u32 = 0x7fff_ffff;
    const KEYS: u32 = 0xe4;
    const WRITER: u32 = 0xf4;
    const COUNTERS: u32 = 0xa4;
    const WORDS: u32 = 0x24;

    /// The light list's alloca: `4 * (+0x14 & 0x7fffffff)` bytes.
    pub fn lights_size(h: &impl Host, this: u32) -> u32 {
        (h.read::<u32>(this.wrapping_add(0x14)) & MASK).wrapping_mul(4)
    }

    /// An 8-byte array header at `hdr` given `count` elements of `size`
    /// bytes (the original's inline `PArray` resize): fresh storage
    /// unless the count is already `count` (0 for none; `Err(0xd)` when
    /// `PhyreMalloc` gives none), the old storage freed when owned, set
    /// and not the new one.
    fn resize(h: &mut impl Host, hdr: u32, count: u32, size: u32) -> Result<(), u32> {
        if h.read::<u32>(hdr) == count {
            return Ok(());
        }
        let mut new = 0;
        if count != 0 {
            new = h.call_sym(MALLOC, Conv::Cdecl, &[count.wrapping_mul(size), 4]);
            if new == 0 {
                return Err(0xd);
            }
        }
        let old: u32 = h.read(hdr.wrapping_add(4));
        if old != new && h.read::<u32>(hdr) & !MASK == 0 && old != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[old]);
        }
        h.write(hdr.wrapping_add(4), new);
        h.write(hdr, count);
        Ok(())
    }

    /// `buildCombinations(this, out)` (`0x004d4e90`, two stack words,
    /// `ret 8`) -> 0, or 0xd when `PhyreMalloc` gives no storage.
    /// `lights`: the alloca of [`lights_size`] bytes; `ebp`: the frame
    /// ([`FRAME`] bytes below it). `this`: `+0x14`/`+0x18` the lights
    /// (count, words), `+0x1c`/`+0x20` the casters, `+0x38` the most
    /// pairs. `out`: a `PArray` of 8-byte headers (`+0` count with the
    /// not-owned bit, `+4` storage), sized to `1 + sum(P^j, j = 1..K)`
    /// (`P` = lights x (casters + 1)) when it differs (the old headers
    /// freed when owned), then filled level by level (`j` pairs): every
    /// combination of lights and casters (caster 0 is none) whose pairs
    /// are all compatible (or casterless) packed, and stored unless an
    /// equal key was stored earlier in the level; then one empty key and
    /// the array truncated to the keys stored.
    #[allow(clippy::too_many_lines)]
    pub fn build(h: &mut impl Host, ebp: u32, lights: u32, this: u32, out: u32) -> u32 {
        let fr = |o: u32| ebp.wrapping_sub(o);
        let n = h.read::<u32>(this.wrapping_add(0x14)) & MASK;
        let src: u32 = h.read(this.wrapping_add(0x18));
        for i in 0..n {
            let v: u32 = h.read(src.wrapping_add(4 * i));
            h.write(lights.wrapping_add(4 * i), v);
        }
        let less = h.rebase(LESS);
        h.call_sym(
            SORT,
            Conv::Cdecl,
            &[lights, lights.wrapping_add(n.wrapping_mul(4)), n, less],
        );
        let m = h.read::<u32>(this.wrapping_add(0x1c)) & MASK;
        let csrc: u32 = h.read(this.wrapping_add(0x20));
        let mut casters = vec![0_u32];
        for i in 0..m {
            casters.push(h.read(csrc.wrapping_add(4 * i)));
        }
        let m1 = m.wrapping_add(1);
        let k_max: u32 = h.read(this.wrapping_add(0x38));
        let p = m1.wrapping_mul(n);
        let mut total = 1_u32;
        let mut j = 1_u32;
        while j <= k_max {
            let mut term = 1_u32;
            for _ in 0..j {
                term = term.wrapping_mul(p);
            }
            total = total.wrapping_add(term);
            j += 1;
        }
        if total != h.read::<u32>(out) {
            let mut new = 0;
            if total != 0 {
                new = h.call_sym(MALLOC, Conv::Cdecl, &[total.wrapping_mul(8), 4]);
                if new == 0 {
                    return 0xd;
                }
                for i in 0..total {
                    let at = new.wrapping_add(i.wrapping_mul(8));
                    if at != 0 {
                        h.write(at, 0_u32);
                        h.write(at.wrapping_add(4), 0_u32);
                    }
                }
            }
            let old: u32 = h.read(out.wrapping_add(4));
            let word: u32 = h.read(out);
            if old != new && word & !MASK == 0 {
                h.call_sym(FREE_HEADERS, Conv::Cdecl, &[old, word & MASK]);
            }
            h.write(out.wrapping_add(4), new);
            h.write(out, total);
        }
        let mut stored = 0_u32;
        let writer = fr(WRITER);
        let counter = |l: u32| fr(COUNTERS).wrapping_add(4 * l);
        let mut level = 1_u32;
        while level <= k_max {
            let words = (0x2d + 10 * (level - 1)) >> 5;
            let start = stored;
            h.call_sym(MEMSET, Conv::Cdecl, &[fr(COUNTERS - 4), 0, 0x7c]);
            h.write(fr(COUNTERS), 0_u32);
            while h.read::<u32>(counter(0)) < n {
                // the pairs of this combination: light C[k], caster C[L + k]
                let mut ok = true;
                for k in 0..level {
                    let c: u32 = h.read(counter(level + k));
                    let l: u32 = h.read(counter(k));
                    let light: u32 = h.read(lights.wrapping_add(l.wrapping_mul(4)));
                    let caster = casters.get(c as usize).copied().unwrap_or(0);
                    if ok {
                        let [r, ..] = h
                            .call_sym(COMPATIBLE, Conv::Cdecl, &[light, caster])
                            .to_le_bytes();
                        ok = r != 0 || caster == 0;
                    }
                    let ck = if caster == 0 {
                        0
                    } else {
                        h.read(caster.wrapping_add(0x10))
                    };
                    let lk = if light == 0 {
                        0
                    } else {
                        h.read::<u32>(light.wrapping_add(0x10))
                    };
                    h.write(fr(KEYS).wrapping_add(4 * k), (ck << 5) | lk);
                }
                if ok {
                    let keys = fr(KEYS);
                    h.call_sym(
                        SORT_KEYS,
                        Conv::Cdecl,
                        &[keys, keys.wrapping_add(4 * level), level],
                    );
                    for w in 0..8 {
                        h.write(fr(WORDS).wrapping_add(4 * w), 0_u32);
                    }
                    h.write(writer, fr(WORDS));
                    h.call_sym(SET_BITS, Conv::Thiscall(writer), &[level, 0, 4]);
                    let mut bit = 4_u32;
                    for k in 0..level {
                        let key: u32 = h.read(keys.wrapping_add(4 * k));
                        h.call_sym(SET_BITS, Conv::Thiscall(writer), &[key, bit, 10]);
                        bit += 10;
                    }
                    // stored before in this level? (an empty header matches)
                    let entries: u32 = h.read(out.wrapping_add(4));
                    let mut found = false;
                    let mut i = start;
                    while i < stored {
                        let hdr = entries.wrapping_add(i.wrapping_mul(8));
                        let cnt = h.read::<u32>(hdr) & MASK;
                        let data: u32 = h.read(hdr.wrapping_add(4));
                        found = true;
                        for c in 0..cnt {
                            let a: u32 = h.read(data.wrapping_add(4 * c));
                            let b: u32 = h.read(fr(WORDS).wrapping_add(4 * c));
                            found = found && a == b;
                        }
                        i += 1;
                        if found {
                            break;
                        }
                    }
                    if !found {
                        let hdr = entries.wrapping_add(stored.wrapping_mul(8));
                        if let Err(e) = resize(h, hdr, words, 4) {
                            return e;
                        }
                        let len = (h.read::<u32>(hdr) & MASK).wrapping_mul(4);
                        let data: u32 = h.read(hdr.wrapping_add(4));
                        h.call_sym(MEMCPY, Conv::Cdecl, &[data, fr(WORDS), len]);
                        stored += 1;
                    }
                }
                // the next combination: the last counter up, carried
                let last = counter(2 * level - 1);
                let v: u32 = h.read(last);
                h.write(last, v.wrapping_add(1));
                let mut i = 2 * level - 1;
                while i > 0 {
                    let limit = if i < level { n } else { m1 };
                    if h.read::<u32>(counter(i)) < limit {
                        break;
                    }
                    let up: u32 = h.read(counter(i - 1));
                    h.write(counter(i - 1), up.wrapping_add(1));
                    h.write(counter(i), 0_u32);
                    i -= 1;
                }
            }
            level += 1;
        }
        let entries: u32 = h.read(out.wrapping_add(4));
        let hdr = entries.wrapping_add(stored.wrapping_mul(8));
        if let Err(e) = resize(h, hdr, 1, 4) {
            return e;
        }
        let data: u32 = h.read(hdr.wrapping_add(4));
        h.write(data, 0_u32);
        h.call_sym(TRUNCATE, Conv::Thiscall(out), &[stored.wrapping_add(1)]);
        0
    }
}

//! M5 stage 1, list `vfx`, round 5 batch 15: the fake environment
//! model's render data (`FUN_00715af0`, `initVFXRenderDataBuffer_FakeEnvModel`'s
//! callee): one data buffer per group of the model, named by the mode
//! (a fixed name, the group's loaded bp texture, or the default texture).
//!
//! The counts, names and texture outputs the original keeps in its frame
//! come in through its `ebp` (the shim passes a `StackFrame`).

/// The frame: `ebp` minus its lowest local.
pub const FRAME: u32 = 0x2094;
/// Mode 1's 12-byte name, the default texture name (31 bytes), the bp
/// texture name format and the missing-texture message.
pub const MODE1_NAME: u32 = 0x00b4_5b3c;
pub const DEFAULT_NAME: u32 = 0x00b4_3768;
pub const BP_NAME_FORMAT: u32 = 0x00b5_0114;
pub const TEXTURE_MISSING_FORMAT: u32 = 0x00b4_ff58;
/// `MSVCR110!sprintf` (the slot).
pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;

// ---- callees ----

/// `crt_memset_thunk(p, v, n)`, `graphicVFXIsTextureLoaded(name, out, 4
/// outputs)`, `dbgPrintf(fmt, ...)`, `graphicVFXCreate(lo, hi, flags)`,
/// `graphicVFXCreateDataBuffer(lo, hi, n, counts, counts, names, flags)`
/// (cdecl).
pub const MEMSET: u32 = 0x0094_964c;
pub const IS_TEXTURE_LOADED: u32 = 0x0064_5400;
pub const DBG_PRINTF: u32 = 0x0062_f500;
pub const VFX_CREATE: u32 = 0x0064_4de0;
pub const VFX_CREATE_DATA_BUFFER: u32 = 0x0064_5040;

pub mod bridge {
    use super::{
        BP_NAME_FORMAT, DBG_PRINTF, DEFAULT_NAME, IS_TEXTURE_LOADED, MEMSET, MODE1_NAME,
        SPRINTF_SLOT, TEXTURE_MISSING_FORMAT, VFX_CREATE, VFX_CREATE_DATA_BUFFER,
    };
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn copy(h: &mut impl Host, to: u32, from: u32, n: u32) {
        for k in 0..n {
            let b: u8 = h.read(from + k);
            h.write(to + k, b);
        }
    }

    /// `FakeEnvModel_createRenderData_00715af0(model, _ x7, mode)` -> 1, or
    /// 0 when a group's texture isn't loaded (reported): the effect keyed
    /// `(mode, model)` (the model plus the mode's sign in the high word)
    /// gets one data buffer per group of the model (`+4` from it, `+0x10`
    /// indices in all; `3 k` each, `k` the group's `+2`, groups `0x10 +
    /// 0x28 k` long). Groups of kind 3 (`+1`) are named by the mode: 1 a
    /// fixed name, 0 the texture `graphicVFXIsTextureLoaded` finds for the
    /// bp word `+8`, else the default texture's. A group of another kind
    /// adds an unnamed entry and stays on the pointer the last step left
    /// (the mode itself for the first, as the original's register holds).
    /// Flags 0x208 for mode 1, else 0xa.
    pub fn create_render_data(h: &mut impl Host, ebp: u32, model: u32, mode: u32) -> u32 {
        let (counts, counts2, extra) = (ebp - 0x1ff4, ebp - 0x1f7c, ebp - 0x206c);
        let names = ebp - 0x1f04;
        let found = ebp - 0x104;
        h.write(counts, 0_u32);
        cdecl(h, MEMSET, &[counts + 4, 0, 0x74]);
        h.write(counts2, 0_u32);
        cdecl(h, MEMSET, &[counts2 + 4, 0, 0x74]);
        h.write(extra, 1_u32);
        cdecl(h, MEMSET, &[extra + 4, 0, 0x74]);
        let lo = mode;
        let hi = model.wrapping_add((mode.cast_signed() >> 31).cast_unsigned());
        let mut left = i32::from(h.read::<i16>(model + 0x10));
        let mut group = model.wrapping_add(h.read(model + 4));
        // the register the original's next-group pointer lives in
        let mut next = lo;
        let mut n = 0_u32;
        while left > 0 {
            let name = names.wrapping_add(n.wrapping_mul(0x100));
            if h.read::<u8>(group + 1) == 3 {
                let k = i32::from(h.read::<i16>(group + 2)).cast_unsigned();
                h.write(counts.wrapping_add(n.wrapping_mul(4)), k.wrapping_mul(3));
                h.write(counts2.wrapping_add(n.wrapping_mul(4)), k.wrapping_mul(3));
                if mode == 1 {
                    let src = h.rebase(MODE1_NAME);
                    copy(h, name, src, 12);
                } else {
                    if mode == 0 {
                        let v = u64::from(h.read::<u32>(group + 8))
                            | (u64::from(h.read::<u32>(group + 0xc)) << 32);
                        let field = |s: u32, m: u64| ((v >> s) & m) as u32;
                        let sprintf: u32 = h.read(h.rebase(SPRINTF_SLOT));
                        let fmt = h.rebase(BP_NAME_FORMAT);
                        h.call(
                            sprintf,
                            Conv::Cdecl,
                            &[
                                name,
                                fmt,
                                field(0, 0x3fff),
                                field(20, 0x3f),
                                1 << field(26, 0xf),
                                1 << field(30, 0xf),
                            ],
                        );
                    } else {
                        let src = h.rebase(DEFAULT_NAME);
                        copy(h, name, src, 31);
                    }
                    let loaded = cdecl(
                        h,
                        IS_TEXTURE_LOADED,
                        &[
                            name,
                            found,
                            ebp - 0x2090,
                            ebp - 0x2094,
                            ebp - 0x2088,
                            ebp - 0x208c,
                        ],
                    );
                    if loaded == 0 {
                        let fmt = h.rebase(TEXTURE_MISSING_FORMAT);
                        cdecl(h, DBG_PRINTF, &[fmt, name]);
                        return 0;
                    }
                    let mut i = 0_u32;
                    loop {
                        let c: u8 = h.read(found + i);
                        h.write(name + i, c);
                        if c == 0 {
                            break;
                        }
                        i += 1;
                    }
                }
                let k = i32::from(h.read::<i16>(group + 2)).cast_unsigned();
                next = group.wrapping_add(k.wrapping_mul(5).wrapping_add(2).wrapping_mul(8));
            }
            left = left.wrapping_sub(i32::from(h.read::<i16>(group + 2)));
            n += 1;
            group = next;
        }
        let flags = if mode == 1 { 0x208 } else { 0xa };
        cdecl(h, VFX_CREATE, &[lo, hi, flags]);
        cdecl(
            h,
            VFX_CREATE_DATA_BUFFER,
            &[lo, hi, n, counts, counts2, names, extra],
        );
        1
    }
}

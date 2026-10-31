//! M5 stage 1, list `vfx`, round 5 batch 14: the `KaGra001` particle
//! program's render data (`FUN_00715dc0`): one data buffer per textured
//! group of its model, each named after its loaded texture.
//!
//! The counts, names and texture outputs the original keeps in its frame
//! come in through its `ebp` (the shim passes a `StackFrame`).

/// The frame: `ebp` minus its lowest local.
pub const FRAME: u32 = 0x2364;
/// The bp texture name format, the missing-texture message.
pub const BP_NAME_FORMAT: u32 = 0x00b5_0114;
pub const TEXTURE_MISSING_FORMAT: u32 = 0x00b4_ff58;
/// `MSVCR110!sprintf` (the slot).
pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;

// ---- callees ----

/// `crt_memset_thunk(p, v, n)`, `get_ptr32_func(p)`,
/// `graphicVFXIsTextureLoaded(name, out, 4 outputs)`, `dbgPrintf(fmt,
/// ...)`, `graphicVFXCreate(lo, hi, flags)`, `graphicVFXCreateDataBuffer(lo,
/// hi, n, counts, counts, names, flags)` (cdecl).
pub const MEMSET: u32 = 0x0094_964c;
pub const GET_PTR32: u32 = 0x00a4_4650;
pub const IS_TEXTURE_LOADED: u32 = 0x0064_5400;
pub const DBG_PRINTF: u32 = 0x0062_f500;
pub const VFX_CREATE: u32 = 0x0064_4de0;
pub const VFX_CREATE_DATA_BUFFER: u32 = 0x0064_5040;

pub mod bridge {
    use super::{
        BP_NAME_FORMAT, DBG_PRINTF, GET_PTR32, IS_TEXTURE_LOADED, MEMSET, SPRINTF_SLOT,
        TEXTURE_MISSING_FORMAT, VFX_CREATE, VFX_CREATE_DATA_BUFFER,
    };
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// `KaGra001_createRenderData_00715dc0(lo, hi, obj, _, flags)` -> 1, or
    /// 0 past 29 groups, on a group not of kind 3 (`+1`) or one whose
    /// texture isn't loaded (reported): the model (`get_ptr32_func(obj
    /// +0x1c)`, its groups from `+4`, `+0x10` vertices in all) gets one
    /// data buffer per group of `3 k` indices (`k` its `+2`), named after
    /// the group's bp texture (`+8`, 64 bits) as `graphicVFXIsTextureLoaded`
    /// names it; the effect is created with flag 8.
    pub fn create_render_data(
        h: &mut impl Host,
        ebp: u32,
        lo: u32,
        hi: u32,
        obj: u32,
        flags: u32,
    ) -> u32 {
        let (counts, counts2, extra) = (ebp - 0x206c, ebp - 0x20e4, ebp - 0x215c);
        let names = ebp - 0x1ff4;
        let found = ebp - 0x1f4;
        h.write(counts, 0_u32);
        cdecl(h, MEMSET, &[counts + 4, 0, 0x74]);
        h.write(counts2, 0_u32);
        cdecl(h, MEMSET, &[counts2 + 4, 0, 0x74]);
        h.write(extra, 1_u32);
        cdecl(h, MEMSET, &[extra + 4, 0, 0x74]);
        let p: u32 = h.read(obj + 0x1c);
        let model = cdecl(h, GET_PTR32, &[p]);
        let mut left = i32::from(h.read::<i16>(model + 0x10));
        let mut group = model.wrapping_add(h.read(model + 4));
        let mut n = 0_u32;
        while left > 0 {
            n += 1;
            let name = names + 0x100 * (n - 1);
            if n >= 0x1e || h.read::<u8>(group + 1) != 3 {
                return 0;
            }
            let k = i32::from(h.read::<i16>(group + 2));
            let mut next = group + 0x10;
            let mut count = 0_u32;
            if k > 0 {
                count = k.cast_unsigned().wrapping_mul(3);
                next = next.wrapping_add(k.cast_unsigned() << 5);
            }
            h.write(counts2 + 4 * (n - 1), count);
            h.write(counts + 4 * (n - 1), count);
            let v =
                u64::from(h.read::<u32>(group + 8)) | (u64::from(h.read::<u32>(group + 0xc)) << 32);
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
            let loaded = cdecl(
                h,
                IS_TEXTURE_LOADED,
                &[
                    name,
                    found,
                    ebp - 0x2358,
                    ebp - 0x2360,
                    ebp - 0x2354,
                    ebp - 0x235c,
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
            left = left.wrapping_sub(i32::from(h.read::<i16>(group + 2)));
            h.write(ebp - 0x22c4 + 4 * (n - 1), 0_u32);
            h.write(ebp - 0x224c + 4 * (n - 1), 0_u32);
            group = next;
        }
        cdecl(h, VFX_CREATE, &[lo, hi, flags | 8]);
        cdecl(
            h,
            VFX_CREATE_DATA_BUFFER,
            &[lo, hi, n, counts, counts2, names, extra],
        );
        1
    }
}

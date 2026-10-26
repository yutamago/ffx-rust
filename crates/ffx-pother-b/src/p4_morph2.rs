//! The morph manager's per-frame update (`MorphManager::update`, h2p4
//! round).
//!
//! Layout (`g_MorphManager`, 0x28 bytes): four `PArray`s of (instance,
//! value) entries, count word (bit 31: storage not owned) then data: `+0`
//! position morphs (8-byte entries `{instance, weight}`), `+8` vertex
//! colour morphs (`{instance, weight}`), `+0x10` UV morphs (`{instance,
//! weight}`), `+0x18` UV scrolls (16-byte entries `{instance, x, y, _}`);
//! `+0x20` the time.

use ffx_graphic::host::{Conv, Host};

use crate::methods::quiet;

/// `MorphManager_setMorphWeight_006ac900` (stdcall `(morph, w)`).
pub const SET_MORPH_WEIGHT: u32 = 0x006a_c900;
/// `ShaderParams_FindByName` (thiscall `(shader, name)` -> parameter).
pub const FIND_PARAM: u32 = 0x0056_cbf0;
/// `memcpy` (cdecl `(dst, src, n)`).
pub const MEMCPY: u32 = 0x0094_9256;
/// The parameter names: `"VertexColorMorphWeight"`, `"UVMorphWeight"`,
/// `"UvOffsetX"`, `"UvOffsetY"`.
pub const VERTEX_COLOR_WEIGHT: u32 = 0x00b4_8398;
pub const UV_WEIGHT: u32 = 0x00b4_83b0;
pub const UV_OFFSET_X: u32 = 0x00b4_83c0;
pub const UV_OFFSET_Y: u32 = 0x00b4_83cc;

const COUNT: u32 = 0x7fff_ffff;

pub mod bridge {
    //! The original over game memory.

    use super::{
        COUNT, Conv, FIND_PARAM, Host, MEMCPY, SET_MORPH_WEIGHT, UV_OFFSET_X, UV_OFFSET_Y,
        UV_WEIGHT, VERTEX_COLOR_WEIGHT, quiet,
    };

    /// A `PArray`'s elements as the original's inline code finds them:
    /// more than one at `[arr + 4]`, one inline at `arr + 4`, none 0.
    fn elements(h: &mut impl Host, arr: u32) -> u32 {
        match h.read::<u32>(arr) & COUNT {
            0 => 0,
            1 => arr.wrapping_add(4),
            _ => h.read(arr.wrapping_add(4)),
        }
    }

    /// The material list's count (`[[e + 0x10]]`, re-read by every test).
    fn materials(h: &mut impl Host, e: u32) -> u32 {
        let list: u32 = h.read(e.wrapping_add(0x10));
        h.read::<u32>(list) & COUNT
    }

    /// The shader parameter `name` of material element `el` (`{shader,
    /// buffer}`) set from the `size` bytes at `src`, when it fits the
    /// buffer (its first word the size) after the header word.
    fn set(h: &mut impl Host, p: u32, buf: u32, src: u32) {
        let off = u32::from(h.read::<u16>(p.wrapping_add(8)));
        let size = u32::from(h.read::<u16>(p.wrapping_add(0xa)) & 0x1fff);
        if off + size <= h.read::<u32>(buf) && off >= 4 {
            h.call_sym(MEMCPY, Conv::Cdecl, &[off.wrapping_add(buf), src, size]);
        }
    }

    /// `MorphManager::update(mm, dt)` (`0x006acd20`, `ret 4`): the time
    /// `+0x20 += dt`; each position morph with a target (`[instance +
    /// 0x18]`) gets its weight (`setMorphWeight`); each material of every
    /// vertex-colour and UV morph instance (the list at `+0x10`) its
    /// `"VertexColorMorphWeight"` / `"UVMorphWeight"`, and of every UV
    /// scroll its `"UvOffsetX"` and `"UvOffsetY"` (both must exist). The
    /// values are copied from the original's frame (`ebp - 0x10`, `-0x1c`,
    /// `-0x20`, `-0x24`); the port writes every frame word the original
    /// does, at its offset from `ebp`, so a parameter longer than 4 bytes
    /// copies the same neighbours (junk read: `[ebp-0x1c]` when no UV
    /// morph had a material, never written; 0, brief 2026-10-24). Returns
    /// the word the original leaves in its argument slot (the UV morphs'
    /// count, or a UV scroll's material list's elements).
    #[allow(clippy::too_many_lines)]
    pub fn update(h: &mut impl Host, ebp: u32, mm: u32, dt: u32) -> u32 {
        let t: u32 = h.read(mm + 0x20);
        h.write(
            mm + 0x20,
            ffx_bridge::x87::store(ffx_bridge::x87::add(f(dt), f(t))),
        );
        h.write(ebp - 4, mm);
        // position morphs
        let n: u32 = h.read::<u32>(mm) & COUNT;
        for i in 0..n {
            let data: u32 = h.read(mm + 4);
            let e: u32 = h.read(data.wrapping_add(8 * i));
            if e == 0 {
                continue;
            }
            let morph: u32 = h.read(e.wrapping_add(0x18));
            if morph == 0 {
                continue;
            }
            let w = quiet(h.read(data.wrapping_add(8 * i + 4)));
            h.call_sym(SET_MORPH_WEIGHT, Conv::Stdcall, &[morph, w]);
        }
        // vertex colour morphs
        let n: u32 = h.read::<u32>(mm + 8) & COUNT;
        h.write(ebp - 0x18, n);
        h.write(ebp - 8, 0u32);
        for j in 0..n {
            let table: u32 = h.read(mm + 0xc);
            let e: u32 = h.read(table.wrapping_add(8 * j));
            h.write(ebp - 0x14, e);
            if e != 0 {
                let w = quiet(h.read(table.wrapping_add(8 * j + 4)));
                h.write(ebp - 0xc, w);
                let list: u32 = h.read(e.wrapping_add(0x10));
                let data = elements(h, list);
                h.write(ebp + 8, data);
                let name = h.rebase(VERTEX_COLOR_WEIGHT);
                let mut k = 0;
                while k < materials(h, e) {
                    let w: u32 = h.read(ebp - 0xc);
                    h.write(ebp - 0x10, quiet(w));
                    let el: u32 = h.read(data.wrapping_add(4 * k));
                    let buf: u32 = h.read(el.wrapping_add(4));
                    let shader: u32 = h.read(el);
                    let p = h.call_sym(FIND_PARAM, Conv::Thiscall(shader), &[name]);
                    if p != 0 {
                        set(h, p, buf, ebp - 0x10);
                    }
                    k += 1;
                }
            }
            h.write(ebp - 8, j + 1);
        }
        // UV morphs
        let n: u32 = h.read::<u32>(mm + 0x10) & COUNT;
        h.write(ebp - 0x14, n);
        let mut slot = 0;
        h.write(ebp + 8, 0u32);
        for j in 0..n {
            let table: u32 = h.read(mm + 0x14);
            let e: u32 = h.read(table.wrapping_add(8 * j));
            h.write(ebp - 0x18, e);
            if e != 0 {
                let list: u32 = h.read(e.wrapping_add(0x10));
                let data = elements(h, list);
                h.write(ebp - 8, data);
                let name = h.rebase(UV_WEIGHT);
                let mut k = 0;
                while k < materials(h, e) {
                    let el: u32 = h.read(data.wrapping_add(4 * k));
                    if el != 0 {
                        let table: u32 = h.read(mm + 0x14);
                        let w = quiet(h.read(table.wrapping_add(8 * j + 4)));
                        h.write(ebp - 0x1c, w);
                        let buf: u32 = h.read(el.wrapping_add(4));
                        let shader: u32 = h.read(el);
                        let p = h.call_sym(FIND_PARAM, Conv::Thiscall(shader), &[name]);
                        if p != 0 {
                            set(h, p, buf, ebp - 0x1c);
                        }
                    }
                    k += 1;
                }
            }
            slot = j + 1;
            h.write(ebp + 8, slot);
        }
        // UV scrolls
        let n: u32 = h.read::<u32>(mm + 0x18) & COUNT;
        h.write(ebp - 0xc, n);
        if n == 0 {
            return slot;
        }
        h.write(ebp - 8, 0u32);
        for j in 0..n {
            let table: u32 = h.read(mm + 0x1c);
            let e: u32 = h.read(table.wrapping_add(0x10 * j));
            h.write(ebp - 0x14, e);
            if e != 0 {
                let list: u32 = h.read(e.wrapping_add(0x10));
                let data = elements(h, list);
                slot = data;
                h.write(ebp + 8, data);
                let (nx, ny) = (h.rebase(UV_OFFSET_X), h.rebase(UV_OFFSET_Y));
                let mut k = 0;
                while k < materials(h, e) {
                    let el: u32 = h.read(data.wrapping_add(4 * k));
                    if el != 0 {
                        let table: u32 = h.read(mm + 0x1c);
                        let buf: u32 = h.read(el.wrapping_add(4));
                        let x = quiet(h.read(table.wrapping_add(0x10 * j + 4)));
                        h.write(ebp - 0x20, x);
                        let y = quiet(h.read(table.wrapping_add(0x10 * j + 8)));
                        h.write(ebp - 0x24, y);
                        let shader: u32 = h.read(el);
                        let px = h.call_sym(FIND_PARAM, Conv::Thiscall(shader), &[nx]);
                        h.write(ebp - 0x18, px);
                        let py = h.call_sym(FIND_PARAM, Conv::Thiscall(shader), &[ny]);
                        if px != 0 && py != 0 {
                            let off = u32::from(h.read::<u16>(px + 8));
                            h.write(ebp - 0x18, off);
                            set(h, px, buf, ebp - 0x20);
                            set(h, py, buf, ebp - 0x24);
                        }
                    }
                    k += 1;
                }
            }
            h.write(ebp - 8, 0x10 * (j + 1));
            h.write(ebp - 0xc, n - j - 1);
        }
        slot
    }

    fn f(bits: u32) -> f64 {
        ffx_bridge::x87::load(bits)
    }
}

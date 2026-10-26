//! The render-side glue, thirty-fifth batch (M4 port round 8, list
//! `h3-glue-render`): a character instance registered with the texture
//! animations. The exception frame is left out, as in [`crate::glue10`].

/// `Phyre_PhyrePrintf(level, format, ...)` (cdecl), its format
/// `"register(%s, %u)\n"`, `PhyreMalloc(n)` and the aligned
/// `PhyreMalloc(n, align)`, `PhyreFree(p)`, `operator new` (cdecl) and
/// the slots' constructor `0x006af420(this; n)` (`ret 4`).
pub const PHYRE_PRINTF: u32 = 0x0043_52f0;
pub const REGISTER_FORMAT: u32 = 0x00b4_8604;
pub const PHYRE_MALLOC: u32 = 0x0042_fb40;
pub const PHYRE_MALLOC_ALIGNED: u32 = 0x0042_fb80;
pub const PHYRE_FREE: u32 = 0x0042_fb20;
pub const OPERATOR_NEW: u32 = 0x0063_0310;
pub const SLOTS_CTOR: u32 = 0x006a_f420;
/// A Phyre array's count bits (the top bit marks borrowed storage).
pub const COUNT: u32 = 0x7fff_ffff;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    /// The C strings at `a` and `b` compare equal.
    fn str_eq(h: &impl Host, a: u32, b: u32) -> bool {
        let mut k = 0u32;
        loop {
            let x: u8 = h.read(a.wrapping_add(k));
            if x != h.read::<u8>(b.wrapping_add(k)) {
                return false;
            }
            if x == 0 {
                return true;
            }
            k = k.wrapping_add(1);
        }
    }

    /// `TextureAnimationManager_registerCharacterInstance(this, name, id;
    /// slots)` (thiscall, `ret 8`; frame: `this` at `ebp - 0x10`, the
    /// free entry at `- 0x18`, the animation's index at `- 0x14`, the new
    /// array at `- 0x1c`): unless `id` is registered already (the
    /// instances `+0x6c`/`+0x70`, 12 bytes each: index, id, slot list),
    /// the animated character named `name` (`+0x64`/`+0x68`, 32 bytes
    /// each, the tagged name pointer at `+0x18`, the materials at
    /// `+0x10`/`+0x14`) gets a slot list: per material its id and a new
    /// slot object of 0x20 slots. The record goes into the first free
    /// instance (id 0) or is appended (the array regrown to 3n + 3).
    /// `name`'s slot takes the list, then each new slot object, then the
    /// count when the array grows.
    pub fn register_character_instance(
        h: &mut impl Host,
        ebp: u32,
        slots: u32,
        this: u32,
        name: u32,
        id: u32,
    ) {
        h.write(ebp - 0x10, this);
        let format = h.rebase(REGISTER_FORMAT);
        h.call_sym(PHYRE_PRINTF, Conv::Cdecl, &[2, format, name, id]);
        let count = h.read::<u32>(this + 0x6c) & COUNT;
        let mut free = count;
        h.write(ebp - 0x18, free);
        let records: u32 = h.read(this + 0x70);
        for k in 0..count {
            let held: u32 = h.read(records.wrapping_add(12 * k + 4));
            if held == id {
                return;
            }
            if held == 0 && free == count {
                free = k;
                h.write(ebp - 0x18, free);
            }
        }
        h.write(ebp - 0x28, u32::MAX);
        h.write(ebp - 0x20, 0u32);
        h.write(ebp - 0x14, 0u32);
        let n = h.read::<u32>(this + 0x64) & COUNT;
        if n == 0 {
            return;
        }
        let entries: u32 = h.read(this + 0x68);
        let mut index = 0u32;
        loop {
            let tagged: u32 = h.read(entries.wrapping_add(32 * index + 0x18));
            let own = tagged.wrapping_sub(u32::from(
                h.read::<u8>(entries.wrapping_add(32 * index + 0x18)) & 1,
            ));
            if own != 0 && str_eq(h, own, name) {
                break;
            }
            index = index.wrapping_add(1);
            h.write(ebp - 0x14, index);
            if index >= n {
                return;
            }
        }
        if index == u32::MAX {
            return;
        }
        let list = slot_list(h, slots, this, index);
        let count = h.read::<u32>(this + 0x6c) & COUNT;
        if free != count {
            let records: u32 = h.read(this + 0x70);
            let at = records.wrapping_add(free.wrapping_mul(12));
            h.write(at, index);
            h.write(at + 4, id);
            h.write(at + 8, list);
            return;
        }
        let grown = h.call_sym(
            PHYRE_MALLOC_ALIGNED,
            Conv::Cdecl,
            &[count.wrapping_mul(3).wrapping_add(3).wrapping_mul(4), 4],
        );
        h.write(ebp - 0x1c, grown);
        if grown == 0 {
            return;
        }
        let count = h.read::<u32>(this + 0x6c) & COUNT;
        let mut from: u32 = h.read(this + 0x70);
        h.write(slots, count);
        let mut to = grown;
        for _ in 0..count {
            if to != 0 {
                for w in 0..3 {
                    let v: u32 = h.read(from + 4 * w);
                    h.write(to + 4 * w, v);
                }
                from = from.wrapping_add(12);
            }
            to = to.wrapping_add(12);
        }
        let at = grown.wrapping_add(count.wrapping_mul(12));
        if at != 0 {
            h.write(at, index);
            h.write(at + 4, id);
            h.write(at + 8, list);
        }
        let old: u32 = h.read(this + 0x70);
        let borrowed = h.read::<u32>(this + 0x6c) & !COUNT != 0;
        if old != grown && !borrowed && old != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
        }
        h.write(this + 0x70, grown);
        h.write(this + 0x6c, count.wrapping_add(1));
    }

    /// The slot list `{n, entries}` (8 bytes, `PhyreMalloc`) for the
    /// animated character `index`: per material its id and a new slot
    /// object (0 when `operator new` fails). `slots` (`name`'s) takes the
    /// list, then each object.
    fn slot_list(h: &mut impl Host, slots: u32, this: u32, index: u32) -> u32 {
        let list = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[8]);
        h.write(slots, list);
        if list != 0 {
            let entries: u32 = h.read(this + 0x68);
            let n = h.read::<u32>(entries.wrapping_add(32 * index + 0x10)) & COUNT;
            h.write(list, 0u32);
            let data = if n == 0 {
                0
            } else {
                h.call_sym(PHYRE_MALLOC_ALIGNED, Conv::Cdecl, &[n.wrapping_mul(8), 4])
            };
            h.write(list + 4, data);
            h.write(list, if data == 0 { 0 } else { n });
        }
        let mut k = 0u32;
        loop {
            let entries: u32 = h.read(this + 0x68);
            let entry = entries.wrapping_add(32 * index);
            if k >= h.read::<u32>(entry + 0x10) & COUNT {
                break;
            }
            let ids: u32 = h.read(entry + 0x14);
            let material: u32 = h.read(ids.wrapping_add(4 * k));
            let data: u32 = h.read(list.wrapping_add(4));
            h.write(data.wrapping_add(8 * k), material);
            let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x14]);
            h.write(slots, p);
            let object = if p == 0 {
                0
            } else {
                h.call_sym(SLOTS_CTOR, Conv::Thiscall(p), &[0x20])
            };
            let data: u32 = h.read(list.wrapping_add(4));
            h.write(data.wrapping_add(8 * k + 4), object);
            k = k.wrapping_add(1);
        }
        list
    }
}

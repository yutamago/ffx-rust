//! The render-side glue, twenty-ninth batch (M4 port round 7, list
//! `h3-glue-render`): the VFX render data's geometry segments and the
//! water distortion's material set-up.

/// `g_VFXRenderDataTable`, `ClassVFXRenderDataTable_GetVFXDataByInstance
/// (this, a, b)` and `GetVFXDynamicGeometrySegmentFromPool(this, &segment,
/// &buffer)` (thiscall, `ret 8`), `operator_new_006304c0(n)` (cdecl),
/// `g_DynGeoFillBuffer`, `DynGeoMemManager_getCurrentBufferId(this)` and
/// `strncpy`'s import slot (cdecl).
pub const VFX_TABLE: u32 = 0x00cc_ca6c;
pub const GET_VFX_DATA: u32 = 0x0069_f6e0;
pub const POOL_SEGMENT: u32 = 0x0069_f7f0;
pub const OPERATOR_NEW_ARRAY: u32 = 0x0063_04c0;
pub const DYN_GEO_FILL: u32 = 0x00cd_edf0;
pub const CURRENT_BUFFER_ID: u32 = 0x006d_e9f0;
pub const STRNCPY: u32 = 0x00b0_c368;

/// The water distortion: the material class `0x00ca5268`,
/// `PhyreSceneCursor_Seek4` / `Next4(this)`, `ShaderParams_FindByName
/// (shader, name)`, `"ColorBuffer"`, `"DepthStencilBuffer"`, the two
/// sampler states, the texture check `0x0059e520(this, texture)` (`ret 4`;
/// `TrophyManager_Install`'s folded body, 0), `memcpy` (cdecl),
/// `Phyre_PParameterBuffer_setTextureChecked_0056e670(this, desc, tex)`
/// (`ret 8`), `Phyre_PhyrePrintf(level, format, ...)`, the message,
/// `operator new` / `operator delete` (cdecl), `g_RenderDelegationHelper`,
/// `RenderDelegationHelper_registerDelegation(this, data, size, flags,
/// callback)` (`ret 0x10`) and `WaterDistortionEffectDelegations_captureFrame`.
pub const MATERIAL_CLASS: u32 = 0x00ca_5268;
pub const CURSOR_SEEK4: u32 = 0x0065_ecb0;
pub const CURSOR_NEXT4: u32 = 0x0065_ee40;
pub const FIND_PARAMETER: u32 = crate::h3_glue21::FIND_PARAMETER;
pub const COLOR_BUFFER: u32 = crate::h3_glue21::COLOR_BUFFER;
pub const DEPTH_STENCIL_BUFFER: u32 = 0x00b4_bcbc;
pub const COLOUR_SAMPLER: u32 = 0x00ce_8208;
pub const DEPTH_SAMPLER: u32 = 0x00ce_822c;
pub const TEXTURE_CHECK: u32 = 0x0059_e520;
pub const MEMCPY: u32 = crate::glue5::MEMCPY;
pub const SET_TEXTURE_CHECKED: u32 = 0x0056_e670;
pub const PHYRE_PRINTF: u32 = crate::glue10::PHYRE_PRINTF;
pub const HITS_MESSAGE: u32 = 0x00b4_bc98;
pub const OPERATOR_NEW: u32 = 0x0063_0310;
pub const OPERATOR_DELETE: u32 = crate::glue10::OPERATOR_DELETE;
pub const RENDER_DELEGATION_HELPER: u32 = 0x00cc_ca98;
pub const REGISTER_DELEGATION: u32 = 0x006a_de20;
pub const CAPTURE_FRAME: u32 = 0x006f_5420;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    /// `graphicVFXCreateDataBuffer(key_a, key_b, n, a, b, names, c; slots)`
    /// (cdecl; frame: `entry` at `ebp - 4`, the pool's segment and buffer
    /// at `ebp - 8` / `ebp - 0xc`): the VFX render data of instance
    /// `(key_a, key_b)` gets `n` geometry segments (0x6c bytes; `+0x90`
    /// the count, `+0x94` the array): one from the table's pool when `n`
    /// is 1 and the pool has one (its name buffer kept), else a new
    /// array. Segment `k` takes `a[k]` (`+4`, `+0x64`), `b[k]` (`+8`,
    /// `+0x68`), `c[k]` (`+0`, when `c` is given), a name (`names + 0x100
    /// k`, copied into a new buffer, or the pool's), the current dynamic
    /// buffer's slot (`+0x2c` / `+0x30` of 0x1c-byte halves: `a[k]` and
    /// `b[k]` there, 0 in the other), zeros at `+0x34` and `+0x50`, and
    /// `+0x24 = 1`. The slots: `key_b`'s top byte is the pool flag, `a`'s
    /// `a - b`, `b` and `names` advance, `n` counts down.
    #[allow(clippy::too_many_arguments)] // the original's seven and the slots
    pub fn vfx_create_data_buffer(
        h: &mut impl Host,
        ebp: u32,
        slots: u32,
        key_a: u32,
        key_b: u32,
        n: u32,
        a: u32,
        b: u32,
        names: u32,
        c: u32,
    ) {
        if key_a | key_b == 0 {
            return;
        }
        let table = h.read::<u32>(h.rebase(VFX_TABLE));
        let entry = h.call_sym(GET_VFX_DATA, Conv::Thiscall(table), &[key_a, key_b]);
        h.write(ebp - 4, entry);
        if entry == 0 {
            return;
        }
        h.write(entry + 0x90, n);
        let flag = slots + 7;
        h.write(flag, 0u8);
        let mut pooled = false;
        if n == 1 {
            let table = h.read::<u32>(h.rebase(VFX_TABLE));
            let found = h.call_sym(POOL_SEGMENT, Conv::Thiscall(table), &[ebp - 8, ebp - 0xc]);
            if found != 0 {
                let segment: u32 = h.read(ebp - 8);
                let buffer: u32 = h.read(ebp - 0xc);
                h.write(entry + 0x94, segment);
                h.write(segment.wrapping_add(0x20), buffer);
                h.write(flag, 1u8);
                pooled = true;
            }
        }
        if !pooled {
            let size = u64::from(n) * 0x6c;
            let size = if size > u64::from(u32::MAX) {
                u32::MAX
            } else {
                size as u32
            };
            let array = h.call_sym(OPERATOR_NEW_ARRAY, Conv::Cdecl, &[size]);
            h.write(entry + 0x94, array);
        }
        if (n as i32) <= 0 {
            return;
        }
        h.write(slots + 0xc, a.wrapping_sub(b));
        h.write(ebp - 0xc, c.wrapping_sub(b));
        let mut left = n;
        let mut b_at = b;
        let mut name = names;
        let mut at = 0u32;
        loop {
            let a_k: u32 = h.read(a.wrapping_sub(b).wrapping_add(b_at));
            let b_k: u32 = h.read(b_at);
            for (off, value) in [(4, a_k), (8, b_k), (0x64, a_k), (0x68, b_k)] {
                let segments: u32 = h.read(entry + 0x94);
                h.write(segments.wrapping_add(at + off), value);
            }
            let len = strlen(h, name);
            if !pooled {
                let p = h.call_sym(OPERATOR_NEW_ARRAY, Conv::Cdecl, &[len + 1]);
                let segments: u32 = h.read(entry + 0x94);
                h.write(segments.wrapping_add(at + 0x20), p);
            }
            if c != 0 {
                let c_k: u32 = h.read(c.wrapping_sub(b).wrapping_add(b_at));
                let segments: u32 = h.read(entry + 0x94);
                h.write(segments.wrapping_add(at), c_k);
            }
            set_buffer_halves(h, entry, at);
            let segments: u32 = h.read(entry + 0x94);
            let dst: u32 = h.read(segments.wrapping_add(at + 0x20));
            if pooled {
                let mut k = 0;
                loop {
                    let byte: u8 = h.read(name.wrapping_add(k));
                    h.write(dst.wrapping_add(k), byte);
                    if byte == 0 {
                        break;
                    }
                    k += 1;
                }
            } else {
                let strncpy = h.read::<u32>(h.rebase(STRNCPY));
                let len = strlen(h, name);
                h.call(strncpy, Conv::Cdecl, &[dst, name, len + 1]);
            }
            let segments: u32 = h.read(entry + 0x94);
            h.write(segments.wrapping_add(at + 0x24), 1u32);
            b_at = b_at.wrapping_add(4);
            name = name.wrapping_add(0x100);
            at += 0x6c;
            left = left.wrapping_sub(1);
            h.write(slots + 8, left);
            h.write(slots + 0x10, b_at);
            h.write(slots + 0x14, name);
            if left == 0 {
                return;
            }
        }
    }

    /// The segment at `at` of `entry`'s array: the current dynamic
    /// buffer's 0x1c-byte half (`+0x2c`) takes the segment's `+4` and `+8`,
    /// the other half 0 there, and `+0x34` / `+0x50` are cleared.
    fn set_buffer_halves(h: &mut impl Host, entry: u32, at: u32) {
        let fill = h.read::<u32>(h.rebase(DYN_GEO_FILL));
        let id = h.call_sym(CURRENT_BUFFER_ID, Conv::Thiscall(fill), &[]);
        let half = id.wrapping_mul(0x1c);
        let other = (id ^ 1).wrapping_mul(0x1c);
        for off in [0x2c, 0x30] {
            let record = h.read::<u32>(entry + 0x94).wrapping_add(at);
            let value: u32 = h.read(record + off - 0x28);
            h.write(record.wrapping_add(half).wrapping_add(off), value);
            let segments: u32 = h.read(entry + 0x94);
            h.write(
                segments
                    .wrapping_add(at)
                    .wrapping_add(other)
                    .wrapping_add(off),
                0u32,
            );
        }
        for off in [0x34, 0x50] {
            let segments: u32 = h.read(entry + 0x94);
            h.write(segments.wrapping_add(at + off), 0u32);
        }
    }

    /// The length of the C string at `s`.
    fn strlen(h: &impl Host, s: u32) -> u32 {
        let mut n = 0u32;
        while h.read::<u8>(s.wrapping_add(n)) != 0 {
            n += 1;
        }
        n
    }
    /// `WaterDistortionEffectDelegations_processWaterDistortion(task;
    /// slots)` (cdecl, a delegation callback; frame: the cursor at `ebp -
    /// 0x64`, the depth record at `- 0x44`, the colour record at `- 0x34`,
    /// the task's words at `- 0x24` .. `- 0x1c`, the material's place at
    /// `- 0x18` / `- 0x14`, the second texture at `- 0x10`, the buffer at
    /// `- 0xc`, the hit count at `- 8`, the material at `- 4`): over the
    /// task's scene (`[task]`) materials of class `0x00ca5268`, a
    /// `"ColorBuffer"` of a texture kind gets the first target's texture
    /// (`{0x42, 0, 0x00ce8208, texture}`, copied when it fits), and a
    /// `"DepthStencilBuffer"` the second's (bound through
    /// `setTextureChecked`, counted). The count is printed; with hits a
    /// copy of the task's three words is registered for
    /// `WaterDistortionEffectDelegations_captureFrame`. The first target's
    /// texture goes over `task`'s slot.
    pub fn process_water_distortion(h: &mut impl Host, ebp: u32, slots: u32, task: u32) {
        if task == 0 {
            return;
        }
        let scene: u32 = h.read(task);
        let words: [u32; 3] = [h.read(task + 4), h.read(task + 8), h.read(task + 0xc)];
        h.write(ebp - 0x1c, words[0]);
        h.write(ebp - 0x20, words[1]);
        h.write(ebp - 0x24, words[2]);
        let first = target_texture(h, words[2]);
        h.write(slots, first);
        let second_target: u32 = h.read(task + 0x10);
        let second = target_texture(h, second_target);
        h.write(ebp - 0x10, second);
        let cursor = ebp - 0x64;
        crate::h3_glue14::bridge::cursor_over(h, cursor, scene, MATERIAL_CLASS);
        h.call_sym(CURSOR_SEEK4, Conv::Thiscall(cursor), &[]);
        h.write(ebp - 8, 0u32);
        let mut hits = 0u32;
        crate::h3_glue14::bridge::walk(h, cursor, CURSOR_NEXT4, |h, base| {
            let offset: u32 = h.read(cursor + 0x10);
            let material = offset.wrapping_add(base);
            h.write(ebp - 4, material);
            let shader: u32 = h.read(material);
            let name = h.rebase(COLOR_BUFFER);
            let parameter = h.call_sym(FIND_PARAMETER, Conv::Thiscall(shader), &[name]);
            h.write(ebp - 0x14, parameter);
            if parameter == 0 {
                return;
            }
            bind_colour(h, ebp, slots, material, parameter);
            let material: u32 = h.read(ebp - 4);
            let shader: u32 = h.read(material);
            let name = h.rebase(DEPTH_STENCIL_BUFFER);
            let parameter = h.call_sym(FIND_PARAMETER, Conv::Thiscall(shader), &[name]);
            if parameter == 0 {
                return;
            }
            let second: u32 = h.read(ebp - 0x10);
            h.write(ebp - 0x38, second);
            let record = ebp - 0x44;
            h.write(record, 0x42u32);
            let buffer: u32 = h.read(material + 4);
            let sampler = h.rebase(DEPTH_SAMPLER);
            h.write(record + 8, sampler);
            h.write(record + 4, 0u32);
            h.call_sym(
                SET_TEXTURE_CHECKED,
                Conv::Thiscall(buffer),
                &[parameter, record],
            );
            hits += 1;
            h.write(ebp - 8, hits);
        });
        let text = h.rebase(HITS_MESSAGE);
        h.call_sym(PHYRE_PRINTF, Conv::Cdecl, &[2, text, hits]);
        if (hits as i32) <= 0 {
            return;
        }
        let copy = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0xc]);
        if copy != 0 {
            for k in 0..3 {
                h.write(copy + 4 * k, 0u32);
            }
        }
        for (k, &word) in words.iter().enumerate() {
            h.write(copy.wrapping_add(4 * k as u32), word);
        }
        let helper = h.read::<u32>(h.rebase(RENDER_DELEGATION_HELPER));
        let callback = h.rebase(CAPTURE_FRAME);
        h.call_sym(
            REGISTER_DELEGATION,
            Conv::Thiscall(helper),
            &[copy, 0xc, 0x40, callback],
        );
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[copy]);
    }

    /// The water material's `"ColorBuffer"` (`parameter`): the first
    /// target's texture record at `ebp - 0x34`, copied in for a texture
    /// kind when the texture passes the check and the record fits.
    fn bind_colour(h: &mut impl Host, ebp: u32, slots: u32, material: u32, parameter: u32) {
        let texture: u32 = h.read(slots);
        h.write(ebp - 0x28, texture);
        let buffer: u32 = h.read(material + 4);
        h.write(ebp - 0xc, buffer);
        let kind = h.read::<u8>(parameter + 2);
        let record = ebp - 0x34;
        h.write(record, 0x42u32);
        let sampler = h.rebase(COLOUR_SAMPLER);
        h.write(record + 8, sampler);
        h.write(record + 4, 0u32);
        if matches!(kind, 0x42 | 0xd9 | 0xda | 0xdb | 0xdc | 0x12) {
            let usable =
                texture == 0 || h.call_sym(TEXTURE_CHECK, Conv::Thiscall(sampler), &[texture]) == 0;
            if usable {
                let size = u32::from(h.read::<u16>(parameter + 0xa) & 0x1fff);
                let offset = u32::from(h.read::<u16>(parameter + 8));
                h.write(ebp - 0x14, size);
                h.write(ebp - 0x18, offset);
                if offset + size <= h.read::<u32>(buffer) && offset >= 4 {
                    h.call_sym(
                        MEMCPY,
                        Conv::Cdecl,
                        &[buffer.wrapping_add(offset), record, size],
                    );
                }
            }
        }
    }

    /// A render target's texture: `*target` unless its byte `+4` is set.
    fn target_texture(h: &impl Host, target: u32) -> u32 {
        if h.read::<u8>(target + 4) == 0 {
            h.read(target)
        } else {
            0
        }
    }
}

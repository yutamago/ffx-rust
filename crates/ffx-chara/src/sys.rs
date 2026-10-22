//! The character module's own settings: drawing switches, the shadow
//! colour and direction, the camera and screen matrices it draws with,
//! cloth editing values, mouth and talk settings, the effect packet
//! writer. All globals of `FFX.exe`.

/// `Ch_GuiSetMouthTh`'s threshold: clamped to 0..=200.
#[must_use]
pub const fn mouth_threshold(v: i32) -> i32 {
    if v < 0 {
        0
    } else if v > 200 {
        200
    } else {
        v
    }
}

/// The originals, on the globals.
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use super::mouth_threshold;

    /// `i8` cloth parameter editing (`Ch_ClothSetParaEdit`).
    pub const CLOTH_PARA_EDIT: Global<i8> = Global::new(0x012f_bc54);
    /// `f32` cloth y (`Ch_ClothSetY`).
    pub const CLOTH_Y: Global<f32> = Global::new(0x012f_bc58);
    /// `f32` cloth xz (`Ch_ClothSetXZ`).
    pub const CLOTH_XZ: Global<f32> = Global::new(0x012f_bc5c);
    /// `u8` set by `Ch_MemMoveSetTrigger`.
    pub const MEM_MOVE_TRIGGER: Global<u8> = Global::new(0x012f_bda0);
    /// `i8` drawing switch (`Ch_DrawSetParaEdit`).
    pub const DRAW_PARA_EDIT: Global<i8> = Global::new(0x0130_07d9);
    /// `i8` forced specular (`Ch_DrawSetFourceSpec`).
    pub const DRAW_FOURCE_SPEC: Global<i8> = Global::new(0x0130_07da);
    /// `i8` specular off (`Ch_DrawSetDisableSpec`).
    pub const DRAW_DISABLE_SPEC: Global<i8> = Global::new(0x0130_07db);
    /// `i8` (`Ch_EnableGetTexture`).
    pub const ENABLE_TEXTURE: Global<i8> = Global::new(0x0130_07e8);
    /// The camera matrix (64 bytes, `Ch_SysGetCameraMatrixPtr`).
    pub const CAMERA_MATRIX: Global<[u32; 16]> = Global::new(0x0130_08f8);
    /// The screen matrix (64 bytes, `Ch_SysGetScreenMatrixPtr`).
    pub const SCREEN_MATRIX: Global<[u32; 16]> = Global::new(0x0130_0938);
    /// `u8` shadow mode (`Ch_DrawSetShadowMode`).
    pub const SHADOW_MODE: Global<u8> = Global::new(0x0130_08bd);
    /// `u8` × 3 shadow colour (`Ch_DrawSetShadowColor`).
    pub const SHADOW_COLOR: Global<[u8; 3]> = Global::new(0x0130_08bf);
    /// `u32` × 2 shadow direction (`Ch_DrawSetShadowDir`).
    pub const SHADOW_DIR: Global<[u32; 2]> = Global::new(0x0130_08c4);
    /// `u8` × 2 talk settings (`Ch_GuiSetTalk`).
    pub const TALK: Global<[u8; 2]> = Global::new(0x0130_08d9);
    /// `i8` motion event dump (`Ch_SetDumpMotEv`).
    pub const DUMP_MOT_EV: Global<i8> = Global::new(0x0130_08db);
    /// `u32` × n by index (`Ch_SetFuka`).
    pub const FUKA: Global<u32> = Global::new(0x0130_09b8);
    /// `u32`, 0 or 1 (`Ch_GuiSetCalcNormAll`).
    pub const GUI_CALC_NORMAL: Global<u32> = Global::new(0x0130_09e0);
    /// `u32` (`Ch_MotionSetHalf`).
    pub const MOTION_HALF: Global<u32> = Global::new(0x0130_17f8);
    /// `i16` mouth threshold (`Ch_GuiSetMouthTh`).
    pub const MOUTH_TH: Global<i16> = Global::new(0x00c4_9718);
    /// Where the effect packet writer is, or 0 when it only measures
    /// (`Ch_EffectPacketStoreEND`).
    pub const PACKET_PTR: Global<u32> = Global::new(0x0130_3658);
    /// Bytes the effect packet would take (`Ch_EffectPacketGetSize`).
    pub const PACKET_SIZE: Global<u32> = Global::new(0x0130_365c);
    /// `u16` packet material (`Ch_EffectPacketStoreMAT`).
    pub const PACKET_MAT: Global<u16> = Global::new(0x0130_3674);
    /// `u16` packet program (`Ch_EffectPacketStorePRG`).
    pub const PACKET_PRG: Global<u16> = Global::new(0x0130_367c);

    /// `u8` set by `Sg_SetMovieGomiAvoid`, read by `Ch_DrawGetShadowSemi`.
    pub const MOVIE_GOMI_AVOID: Global<u8> = Global::new(0x0130_08be);

    /// `Ch_DrawGetShadowMode()` (`0x0082ec60`) and `Ch_DrawGetShadowSemi()`
    /// (`0x0082ec70`): a byte setting, zero-extended.
    pub fn get_u8(mem: &impl Mem, g: Global<u8>) -> u32 {
        u32::from(g.get(mem))
    }

    /// Entries of `g_CdLib` (`0x02310c40`), the CD library's function
    /// table, by offset: `+8` (`Ch_ReadSync`), `+0x10` (`Chr_TableIndex`,
    /// `Ch_RomDataReadInit`) and `+0x80` (`Ch_RomReadEventMGRPsync`).
    pub const CD_LIB_08: Global<u32> = Global::new(0x0231_0c48);
    pub const CD_LIB_10: Global<u32> = Global::new(0x0231_0c50);
    pub const CD_LIB_80: Global<u32> = Global::new(0x0231_0cc0);
    /// Per kind (8): a table of `i16` count and `i16` ids.
    pub const KIND_TABLES: Global<u32> = Global::new(0x012f_fab8);
    /// Per kind: the `i16` `Chr_TableIndex` passes to [`CD_LIB_10`].
    pub const KIND_ARGS: Global<i16> = Global::new(0x00c4_9660);
    /// The `i8` index step per table entry.
    pub const KIND_STRIDE: Global<i8> = Global::new(0x00c4_967f);

    /// `Ch_ReadSync()` (`0x0082a5e0`): a tail jump through
    /// [`CD_LIB_08`]; its result.
    pub fn ch_read_sync<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = CD_LIB_08.get(mem);
        call(mem, f, &[])
    }

    /// `Ch_RomReadEventMGRPsync(p)` (`0x0082a7f0`): [`CD_LIB_80`]`(12,
    /// p)`; its result.
    pub fn ch_rom_read_event_mgrp_sync<M: Mem>(
        mem: &mut M,
        p: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = CD_LIB_80.get(mem);
        call(mem, f, &[12, p])
    }

    /// `Chr_TableIndex(id)` (`0x0082a640`): the position of `id & 0xfff`
    /// in its kind's table (kind `id >> 12 & 0xf`, 0..=7), counted in
    /// [`KIND_STRIDE`] steps, after calling [`CD_LIB_10`] with the kind's
    /// [`KIND_ARGS`] entry; -1 when absent.
    pub fn chr_table_index<M: Mem>(
        mem: &mut M,
        id: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> i32 {
        let kind = (id.cast_signed() >> 12) & 0xf;
        if kind > 7 {
            return -1;
        }
        let k = kind.cast_unsigned();
        let table: u32 = mem.read(KIND_TABLES.ptr(mem).addr.wrapping_add(4 * k));
        let want = (id & 0xfff).cast_signed();
        let mut index = 0i32;
        let mut i = 0i32;
        while i < i32::from(mem.read::<i16>(table)) {
            let at = table.wrapping_add(2 + 2 * i.cast_unsigned());
            if i32::from(mem.read::<i16>(at)) == want {
                let arg = i32::from(mem.read::<i16>(KIND_ARGS.ptr(mem).addr.wrapping_add(2 * k)));
                let f = CD_LIB_10.get(mem);
                call(mem, f, &[arg.cast_unsigned()]);
                return index;
            }
            index = index.wrapping_add(i32::from(KIND_STRIDE.get(mem)));
            i += 1;
        }
        -1
    }

    /// The effect packet being written (`Ch_EffectPacketSetPtr`).
    pub const PACKET_BASE: Global<u32> = Global::new(0x0130_3650);
    /// The open block's value after each of the four packet states
    /// (`+4` after `0x0130366c`, MAT, PRG and the fourth, 8 bytes apart).
    pub const PACKET_STATE_VALUES: [u32; 4] = [0x0130_3670, 0x0130_3678, 0x0130_3680, 0x0130_3688];

    /// `Ch_EffectPacketSetPtr(p)` (`0x00845710`): starts an effect packet
    /// at `p`, or with 0 starts measuring one (size 0x20, nothing
    /// counted). A packet starts with a 0x20-byte header: 0, the header
    /// size, the vertex and normal buffers' offsets from `p`
    /// (`sg_getvtex`/`sg_getnorm`, passed in), the measured triangles and
    /// one more than each measured maximum (`u16`), 0. The four state
    /// values are reset to `0xffff`. Returns what the original leaves in
    /// EAX: the second maximum plus one, or -1 without a packet.
    pub fn ch_effect_packet_set_ptr<M: Mem>(
        mem: &mut M,
        p: u32,
        vtex: impl FnOnce(&mut M) -> u32,
        norm: impl FnOnce(&mut M) -> u32,
    ) -> u32 {
        PACKET_PTR.set(mem, p);
        let image = PACKET_PTR.ptr(mem).addr.wrapping_sub(0x0130_3658);
        for v in PACKET_STATE_VALUES {
            mem.write(image.wrapping_add(v), 0xffffu16);
        }
        if p == 0 {
            PACKET_SIZE.set(mem, 0x20);
            PACKET_POLYS.set(mem, 0);
            PACKET_MAX.set(mem, [0, 0]);
            return u32::MAX;
        }
        PACKET_PTR.set(mem, p.wrapping_add(0x20));
        PACKET_BASE.set(mem, p);
        mem.write(p, 0u32);
        mem.write(p.wrapping_add(4), 0x20u32);
        let v = vtex(mem);
        let base = PACKET_BASE.get(mem);
        mem.write(base.wrapping_add(8), v.wrapping_sub(base));
        let n = norm(mem);
        let base = PACKET_BASE.get(mem);
        mem.write(base.wrapping_add(0xc), n.wrapping_sub(base));
        #[allow(clippy::cast_possible_truncation)] // the low halves
        {
            let polys = PACKET_POLYS.get(mem) as u16;
            let [mx, my] = PACKET_MAX.get(mem).map(i32::cast_unsigned);
            mem.write(base.wrapping_add(0x10), polys);
            mem.write(base.wrapping_add(0x12), mx.wrapping_add(1) as u16);
            mem.write(base.wrapping_add(0x14), my.wrapping_add(1) as u16);
            mem.write(base.wrapping_add(0x16), 0u16);
            my.wrapping_add(1)
        }
    }

    /// `g_FFXIsDebug`: debug build features on.
    pub const IS_DEBUG: Global<u8> = Global::new(0x0133_c910);
    /// `Gui_ButtonSetTexture` (`0x0084c9d0`, the PC UI's).
    pub const GUI_BUTTON_SET_TEXTURE: u32 = 0x0084_c9d0;

    /// `Ch_EnableSetTexture(v)` (`0x00830330`): the texture switch
    /// ([`ENABLE_TEXTURE`]) = bit 0 of `v`; when `v` differs from the old
    /// value in a debug build, `Gui_ButtonSetTexture()`. Returns the old
    /// value (sign-extended).
    pub fn ch_enable_set_texture<M: Mem>(
        mem: &mut M,
        v: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> i32 {
        let old = i32::from(ENABLE_TEXTURE.get(mem));
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // bit 0
        ENABLE_TEXTURE.set(mem, (v & 1) as i8);
        if v.cast_signed() != old && IS_DEBUG.get(mem) != 0 {
            call(mem, mem.rebase(GUI_BUTTON_SET_TEXTURE), &[]);
        }
        old
    }

    /// `g_CdLib` itself (`+0`, read) and `+0x14` (a table's size), as
    /// `Ch_RomDataReadInit` uses them.
    pub const CD_LIB: Global<u32> = Global::new(0x0231_0c40);
    pub const CD_LIB_14: Global<u32> = Global::new(0x0231_0c54);

    /// `Ch_RomDataReadInit()` (`0x0082a5f0`): the kind tables:
    /// `CD_LIB_10(0x14)`, then for kinds 0..7 a buffer of the table's size
    /// (`CD_LIB_14(kind)`, `user_malloc`) named `"tenuki.gomen."`
    /// (`Sg_MemSetStr(p, name, kind)`), stored in [`KIND_TABLES`] and read
    /// into (`CD_LIB(kind, p, 0, 0)`). Returns the last read's result.
    pub fn ch_rom_data_read_init<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = CD_LIB_10.get(mem);
        call(mem, f, &[0x14]);
        let mut r = 0;
        for kind in 0..7u32 {
            let f = CD_LIB_14.get(mem);
            let size = call(mem, f, &[kind]);
            let p = call(mem, mem.rebase(crate::handle::bridge::USER_MALLOC), &[size]);
            let slot = KIND_TABLES.ptr(mem).addr + 4 * kind;
            mem.write(slot, p);
            let name = mem.rebase(0x00b5_9650);
            call(
                mem,
                mem.rebase(crate::handle::bridge::MEM_SET_STR),
                &[p, name, kind],
            );
            let p: u32 = mem.read(slot);
            let f = CD_LIB.get(mem);
            r = call(mem, f, &[kind, p, 0, 0]);
        }
        r
    }

    /// `u8` (`Ch_SysDrawSetSituation`).
    pub const DRAW_SITUATION: Global<u8> = Global::new(0x0130_08c2);
    /// Non-zero while the renderer is not to be told (`0x012fb7d8`).
    pub const DRAW_DETACHED: Global<u32> = Global::new(0x012f_b7d8);
    /// `graphicCharacterInsertDrawSituation(v)` (`0x00639ff0`).
    pub const INSERT_DRAW_SITUATION: u32 = 0x0063_9ff0;

    /// `Ch_SysDrawSetSituation(v)` (`0x00830cd0`): byte
    /// [`DRAW_SITUATION`] = `v`, passed on to the renderer
    /// (`graphicCharacterInsertDrawSituation(v)`, its result) unless
    /// [`DRAW_DETACHED`]; else returns `v`.
    pub fn ch_sys_draw_set_situation<M: Mem>(
        mem: &mut M,
        v: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // the low byte
        DRAW_SITUATION.set(mem, v as u8);
        if DRAW_DETACHED.get(mem) != 0 {
            return v;
        }
        call(mem, mem.rebase(INSERT_DRAW_SITUATION), &[v])
    }

    /// `g_CdLib + 0x98`: called with (file, buffer) before a read.
    pub const CD_LIB_98: Global<u32> = Global::new(0x0231_0c98);
    /// `Cache_Entry` and `Cache_Entry_NotYet` (`0x00840560`, `0x008405f0`).
    pub const CACHE_ENTRY: u32 = 0x0084_0560;
    pub const CACHE_ENTRY_NOT_YET: u32 = 0x0084_05f0;
    /// `Sg_MemSetType(p, type)` (`0x0083c590`).
    pub const MEM_SET_TYPE: u32 = 0x0083_c590;
    /// `Ch_RomReadChrCB(request)` (`0x0082a6c0`).
    pub const ROM_READ_CHR_CB: u32 = 0x0082_a6c0;
    /// Bytes of scratch `ch_mgrp_read` needs: the read request (0x1c) and
    /// the block's name (from `+0x20`).
    pub const MGRP_READ_SCRATCH: u32 = 0x60;

    /// `Ch_MgrpRead(id, n)` (`0x0082a4c0`): queues the read of character
    /// `id`'s resident motion group `n` unless the cache has it (state 0
    /// or 1 for key `n << 16 | id`, kind 2): its file is the character's
    /// table index + 1 + `n`; with a size (`g_CdLib_14(file)`, not 0 nor
    /// 0x10) a buffer (`user_malloc((size + 0x8f & !0x7f) - 0x10)`)
    /// named `"[<chrname>:resident<n>.mgrp]"` (`Sg_MemSetStr`, type 3),
    /// `g_CdLib_98(file, buffer)`, the cache entry (`Cache_Entry(2, key,
    /// buffer)`, not yet read), and the read (`g_CdLib(file, buffer,
    /// Ch_RomReadChrCB, &request)`, the request's `+0xc`.. `+0x18` = key,
    /// buffer, 2 and a word the original leaves uninitialised, 0 here).
    /// The request and the name go in `scratch` ([`MGRP_READ_SCRATCH`]
    /// bytes; the original uses its stack: the read copies the request).
    /// Returns -1 for an unknown character or no size, else 0.
    pub fn ch_mgrp_read<M: Mem>(
        mem: &mut M,
        id: u32,
        n: u32,
        scratch: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let h = crate::handle::bridge::MEM_SET_STR;
        let key = (n << 16) | id;
        let state = call(
            mem,
            mem.rebase(crate::motion::bridge::CACHE_CHECK_YET),
            &[2, key],
        );
        if state <= 1 {
            return 0;
        }
        let index = call(mem, mem.rebase(0x0082_a640), &[id]);
        if index.cast_signed() < 0 {
            return u32::MAX;
        }
        let file = index.wrapping_add(1).wrapping_add(n);
        let f = CD_LIB_14.get(mem);
        let size = call(mem, f, &[file]);
        if size == 0 || size == 0x10 {
            return u32::MAX;
        }
        let len = (size.wrapping_add(0x8f) & 0xffff_ff80).wrapping_sub(0x10);
        let buf = call(mem, mem.rebase(crate::handle::bridge::USER_MALLOC), &[len]);
        let chrname = call(mem, mem.rebase(0x0083_8060), &[id, n]);
        let mut name = b"[".to_vec();
        name.extend(crate::handle::bridge::c_string(mem, chrname));
        name.extend(format!(":resident{}.mgrp]", n.cast_signed()).bytes());
        let name_at = scratch + 0x20;
        crate::handle::bridge::put_c_string(mem, name_at, &name);
        call(mem, mem.rebase(h), &[buf, name_at, u32::MAX]);
        call(mem, mem.rebase(MEM_SET_TYPE), &[buf, 3]);
        let f = CD_LIB_98.get(mem);
        call(mem, f, &[file, buf]);
        mem.write(scratch + 0xc, [key, buf, 2, 0]);
        call(mem, mem.rebase(CACHE_ENTRY), &[2, key, buf]);
        call(mem, mem.rebase(CACHE_ENTRY_NOT_YET), &[2, key, 1]);
        let f = CD_LIB.get(mem);
        call(mem, f, &[file, buf, mem.rebase(ROM_READ_CHR_CB), scratch]);
        0
    }

    /// `isEternalCalmMgrp(n)` (`0x0067ab30`), `getEternalCalmMgrpSize(n)`
    /// (`0x0067aa70`, platform), `Ch_RomReadEventMGRPcb` (`0x0082a7d0`)
    /// and the `"SG:RomRead:%s %x\n"` format (`0x00b596b8`).
    pub const IS_ETERNAL_CALM_MGRP: u32 = 0x0067_ab30;
    pub const ETERNAL_CALM_MGRP_SIZE: u32 = 0x0067_aa70;
    pub const ROM_READ_EVENT_MGRP_CB: u32 = 0x0082_a7d0;
    pub const ROM_READ_FORMAT: u32 = 0x00b5_96b8;
    /// Bytes of scratch `ch_rom_read_event_mgrp` needs: the read request
    /// (`[ebp-0x60]`) and the block's name (`[ebp-0x44]`).
    pub const ROM_READ_EVENT_MGRP_SCRATCH: u32 = 0x60;

    /// `Ch_RomReadEventMGRP(file, x)` (`0x0082a6e0`; caller
    /// `0x00837650`): queues the read of event motion group `file`. After
    /// `g_CdLib_10(12)` its size is `g_CdLib_14(file)`; a size of 0 falls
    /// back to the Eternal Calm group `file / 18` (`isEternalCalmMgrp`,
    /// `getEternalCalmMgrpSize`). No size, or 0x10, returns 0. Else a
    /// buffer (`Sg_MemSetAlign(0x40)`, `user_malloc(size)`) named
    /// `"[EventMotionGroup <x>]"` (`Sg_MemSetStr`, type 3),
    /// `g_CdLib_98(file, buffer)` (logged with `dbgPrintf`), and the read
    /// (`g_CdLib(file, buffer, Ch_RomReadEventMGRPcb, &request)`, the
    /// request's `+0xc` = `x`, `+0x10` = the buffer; the original leaves
    /// the rest of it uninitialised). The request and the name go in
    /// `scratch` ([`ROM_READ_EVENT_MGRP_SCRATCH`] bytes; the original's
    /// stack). Returns the buffer.
    pub fn ch_rom_read_event_mgrp<M: Mem>(
        mem: &mut M,
        file: u32,
        x: u32,
        scratch: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = CD_LIB_10.get(mem);
        call(mem, f, &[12]);
        let f = CD_LIB_14.get(mem);
        let mut size = call(mem, f, &[file]);
        if size == 0 {
            let n = (file.cast_signed() / 18).cast_unsigned();
            if call(mem, mem.rebase(IS_ETERNAL_CALM_MGRP), &[n]) == 0 {
                return 0;
            }
            size = call(mem, mem.rebase(ETERNAL_CALM_MGRP_SIZE), &[n]);
            if size == 0 {
                return 0;
            }
        }
        if size == 0x10 {
            return 0;
        }
        call(
            mem,
            mem.rebase(crate::texture::bridge::MEM_SET_ALIGN),
            &[0x40],
        );
        let buf = call(mem, mem.rebase(crate::handle::bridge::USER_MALLOC), &[size]);
        let name = format!("[EventMotionGroup {}]", x.cast_signed());
        let name_at = scratch + 0x1c;
        crate::handle::bridge::put_c_string(mem, name_at, name.as_bytes());
        let set_str = mem.rebase(crate::handle::bridge::MEM_SET_STR);
        call(mem, set_str, &[buf, name_at, u32::MAX]);
        call(mem, mem.rebase(MEM_SET_TYPE), &[buf, 3]);
        let f = CD_LIB_98.get(mem);
        let r = call(mem, f, &[file, buf]);
        let fmt = mem.rebase(ROM_READ_FORMAT);
        call(
            mem,
            mem.rebase(crate::handle::bridge::DBG_PRINTF),
            &[fmt, r, buf],
        );
        mem.write(scratch + 0xc, [x, buf]);
        let f = CD_LIB.get(mem);
        let cb = mem.rebase(ROM_READ_EVENT_MGRP_CB);
        call(mem, f, &[file, buf, cb, scratch]);
        buf
    }

    /// The effect packet builders `Ch_EffectPacketCreateMain` calls, all
    /// through the game: `sg_setvtex_and_normal(vtex, norm)`
    /// (`0x00845ca0`), `Ch_EffectPacketStoreMAT`/`PRG` (`0x00845800`,
    /// `0x00845810`), `Sg_SetChrPacketState`/`Word` (`0x00845c30`,
    /// `0x00845c40`), `get_ptr32_func` (`0x00a44650`),
    /// `Ch_EffectPacketStoreStrip` (`0x00844370`), `Ch_EffectPacketStorePoly`
    /// (`0x00845820`) and `Ch_EffectPacketStoreEND` (`0x008457d0`).
    pub const SET_VTEX_AND_NORMAL: u32 = 0x0084_5ca0;
    pub const STORE_MAT: u32 = 0x0084_5800;
    pub const STORE_PRG: u32 = 0x0084_5810;
    pub const SET_PACKET_STATE: u32 = 0x0084_5c30;
    pub const SET_PACKET_WORD: u32 = 0x0084_5c40;
    pub const GET_PTR32: u32 = 0x00a4_4650;
    pub const STORE_STRIP: u32 = 0x0084_4370;
    pub const STORE_POLY: u32 = 0x0084_5820;
    pub const STORE_END: u32 = 0x0084_57d0;
    /// Bytes of scratch [`ch_effect_packet_create_main`] needs: one
    /// triangle's three vertices of four `u16` (`[ebp-0x1c]`).
    pub const PACKET_CREATE_SCRATCH: u32 = 0x18;

    /// A polygon index: `w` itself, or with bit 15 a reference back from
    /// `base` (`(w & 0x7fff) * 4 / 3` halfwords, and one more for `odd`).
    fn poly_index(mem: &impl Mem, base: u32, w: u16, odd: bool) -> u16 {
        if w & 0x8000 == 0 {
            return w;
        }
        let back = (u32::from(w & 0x7fff) << 2) / 3 * 2;
        mem.read(
            base.wrapping_sub(back)
                .wrapping_sub(if odd { 2 } else { 0 }),
        )
    }

    /// `Ch_EffectPacketCreateMain(p)` (`0x00842c80`; callers
    /// `Ch_ModelCalcSizeEffectPacket`, `Ch_ModelCreateEffectPacket`): the
    /// effect packet of one model. `p` holds the model data (its groups:
    /// count `u16` `+6`, 12-byte entries from `+0x10`), the normals and
    /// the vertices (`sg_setvtex_and_normal(p[2], p[1])`). Per group:
    /// - a new MAT byte (`+1`) or state byte (`+2`) is stored
    ///   (`Ch_EffectPacketStoreMAT`, `Sg_SetChrPacketState`) and forgets
    ///   the PRG; a new word byte (`+0`, `Sg_SetChrPacketWord`) makes its
    ///   bit 2 the PRG, stored when it differs (all start as `0xff`, the
    ///   PRG as -1 against 0);
    /// - with bit 0 of the word byte, strips: `u16` `+4` blocks from
    ///   `get_ptr32_func(+8)`, each with byte counts: a block of `[0]` 16
    ///   bytes, `[2]` strips whose lengths are the bytes from `+4`, their
    ///   8-byte points from `[1]` bytes in (`Ch_EffectPacketStoreStrip(0)`
    ///   opens each strip);
    /// - else `+4 / 3` triangles of 24 bytes: indices 0, 1, 4, 5, 8, 9
    ///   through [`poly_index`] (from the triangle, its `+8`, its `+0x10`),
    ///   2, 3, 6, 7, 10, 11 as they are; `Ch_EffectPacketStorePoly` gets
    ///   the three vertices, built in `scratch`.
    ///
    /// Then the end tag (`Ch_EffectPacketStoreEND`). Counts are read again
    /// at each step and compared signed. No caller reads `eax`.
    #[allow(clippy::too_many_lines)]
    pub fn ch_effect_packet_create_main<M: Mem>(
        mem: &mut M,
        p: u32,
        scratch: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let g = |mem: &M, f: u32| mem.rebase(f);
        let hdr: u32 = mem.read(p);
        let groups: u32 = mem.read(hdr.wrapping_add(0x10));
        let (norm, vtex): (u32, u32) = (mem.read(p.wrapping_add(4)), mem.read(p.wrapping_add(8)));
        call(mem, g(mem, SET_VTEX_AND_NORMAL), &[vtex, norm]);
        let (mut mat, mut state, mut word) = (0xffu8, 0xffu8, 0xffu8);
        let mut prg = u32::MAX;
        let mut prg_word = 0u32;
        let mut i = 0i32;
        while i < i32::from(mem.read::<u16>(hdr.wrapping_add(6))) {
            let e = groups.wrapping_add(i.cast_unsigned() * 0xc);
            let b: u8 = mem.read(e.wrapping_add(1));
            if b != mat {
                mat = b;
                call(mem, g(mem, STORE_MAT), &[u32::from(b)]);
                prg = u32::MAX;
            }
            let b: u8 = mem.read(e.wrapping_add(2));
            if b != state {
                state = b;
                call(mem, g(mem, SET_PACKET_STATE), &[u32::from(b)]);
                prg = u32::MAX;
            }
            let b: u8 = mem.read(e);
            if b != word {
                word = b;
                call(mem, g(mem, SET_PACKET_WORD), &[u32::from(b)]);
                prg_word = u32::from(mem.read::<u8>(e) >> 2) & 1;
            }
            if prg != prg_word {
                call(mem, g(mem, STORE_PRG), &[prg_word]);
                prg = prg_word;
            }
            let strips = mem.read::<u8>(e) & 1 != 0;
            let data: u32 = mem.read(e.wrapping_add(8));
            let count = |mem: &M| i32::from(mem.read::<u16>(e.wrapping_add(4)));
            let mut q = call(mem, g(mem, GET_PTR32), &[data]);
            if strips {
                let mut k = 0i32;
                while k < count(mem) {
                    let mut point = q.wrapping_add(u32::from(mem.read::<u8>(q.wrapping_add(1))));
                    let lens = q.wrapping_add(4);
                    let mut j = 0u32;
                    while j < u32::from(mem.read::<u8>(q.wrapping_add(2))) {
                        call(mem, g(mem, STORE_STRIP), &[0]);
                        let mut s = 0u32;
                        while s < u32::from(mem.read::<u8>(lens.wrapping_add(j))) {
                            call(mem, g(mem, STORE_STRIP), &[point]);
                            point = point.wrapping_add(8);
                            s += 1;
                        }
                        j += 1;
                    }
                    q = q.wrapping_add(u32::from(mem.read::<u8>(q)) << 4);
                    k += 1;
                }
            } else {
                let mut t = 0i32;
                while t < count(mem) / 3 {
                    let w = |mem: &M, k: u32| mem.read::<u16>(q.wrapping_add(2 * k));
                    let mut v = [0u16; 12];
                    for (k, slot) in (0u32..).zip(v.iter_mut()) {
                        let base = q.wrapping_add((k / 4) * 8);
                        *slot = match k % 4 {
                            0 => poly_index(mem, base, w(mem, k), false),
                            1 => poly_index(mem, base, w(mem, k), true),
                            _ => w(mem, k),
                        };
                    }
                    mem.write(scratch, v);
                    let store = g(mem, STORE_POLY);
                    call(mem, store, &[scratch, scratch + 8, scratch + 0x10]);
                    q = q.wrapping_add(0x18);
                    t += 1;
                }
            }
            i += 1;
        }
        call(mem, g(mem, STORE_END), &[]);
    }

    /// `Ch_*Get*` of an `i8` setting, sign-extended.
    pub fn get_i8(mem: &impl Mem, g: Global<i8>) -> i32 {
        i32::from(g.get(mem))
    }

    /// `Ch_*Set*` of a byte setting: the low byte of `v`.
    pub fn set_i8(mem: &mut impl Mem, g: Global<i8>, v: u32) {
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // the low byte
        g.set(mem, v as u8 as i8);
    }

    /// `Ch_ClothSetXZ(v)` (`0x00824a40`) and `Ch_ClothSetY(v)`
    /// (`0x00824a50`).
    pub fn set_f32(mem: &mut impl Mem, g: Global<f32>, v: f32) {
        g.set(mem, crate::x87::load(v));
    }

    /// `Ch_MemMoveSetTrigger()` (`0x00827250`).
    pub fn ch_mem_move_set_trigger(mem: &mut impl Mem) {
        MEM_MOVE_TRIGGER.set(mem, 1);
    }

    /// `Ch_DrawGetShadowColor(&r, &g, &b)` (`0x0082ebf0`): null outputs
    /// are skipped.
    pub fn ch_draw_get_shadow_color(mem: &mut impl Mem, out: [u32; 3]) {
        let c = SHADOW_COLOR.get(mem);
        for (o, v) in out.into_iter().zip(c) {
            if o != 0 {
                mem.write(o, u32::from(v));
            }
        }
    }

    /// `Ch_DrawSetShadowColor(r, g, b)` (`0x00830100`).
    pub fn ch_draw_set_shadow_color(mem: &mut impl Mem, [r, g, b]: [u32; 3]) {
        #[allow(clippy::cast_possible_truncation)] // low bytes
        SHADOW_COLOR.set(mem, [r as u8, g as u8, b as u8]);
    }

    /// `Ch_DrawGetShadowDir(&a, &b)` (`0x0082ec30`): null outputs are
    /// skipped.
    pub fn ch_draw_get_shadow_dir(mem: &mut impl Mem, out: [u32; 2]) {
        let d = SHADOW_DIR.get(mem);
        for (o, v) in out.into_iter().zip(d) {
            if o != 0 {
                mem.write(o, v);
            }
        }
    }

    /// `Ch_DrawSetShadowDir(a, b)` (`0x00830120`).
    pub fn ch_draw_set_shadow_dir(mem: &mut impl Mem, d: [u32; 2]) {
        SHADOW_DIR.set(mem, d);
    }

    /// `Ch_DrawSetShadowMode(mode)` (`0x00830140`).
    pub fn ch_draw_set_shadow_mode(mem: &mut impl Mem, mode: u32) {
        #[allow(clippy::cast_possible_truncation)] // the low byte
        SHADOW_MODE.set(mem, mode as u8);
    }

    /// `Ch_GuiGetCalcNormal()` (`0x00834060`).
    pub fn ch_gui_get_calc_normal(mem: &impl Mem) -> u32 {
        GUI_CALC_NORMAL.get(mem)
    }

    /// `Ch_GuiSetCalcNormAll(on)` (`0x00834090`).
    pub fn ch_gui_set_calc_norm_all(mem: &mut impl Mem, on: u32) {
        GUI_CALC_NORMAL.set(mem, on & 1);
    }

    /// `Ch_GuiSetMouthTh(v)` (`0x008340a0`): clamped to 0..=200; returns
    /// that (left in `eax`).
    pub fn ch_gui_set_mouth_th(mem: &mut impl Mem, v: i32) -> i32 {
        let t = mouth_threshold(v);
        #[allow(clippy::cast_possible_truncation)] // 0..=200
        MOUTH_TH.set(mem, t as i16);
        t
    }

    /// `Ch_GuiSetTalk(a, b)` (`0x008340d0`).
    pub fn ch_gui_set_talk(mem: &mut impl Mem, a: u32, b: u32) {
        #[allow(clippy::cast_possible_truncation)] // low bytes
        TALK.set(mem, [a as u8, b as u8]);
    }

    /// `Ch_GetFuka(i)` (`0x00833f60`); the index is not checked.
    pub fn ch_get_fuka(mem: &impl Mem, i: u32) -> u32 {
        FUKA.ptr(mem).at(i).get(mem)
    }

    /// `Ch_SetFuka(i, v)` (`0x008357d0`).
    pub fn ch_set_fuka(mem: &mut impl Mem, i: u32, v: u32) {
        let p = FUKA.ptr(mem).at(i);
        p.set(mem, v);
    }

    /// `Ch_SysGetScreenMatrixPtr()` (`0x00835b70`).
    pub fn ch_sys_get_screen_matrix_ptr(mem: &impl Mem) -> u32 {
        SCREEN_MATRIX.ptr(mem).addr
    }

    /// `Ch_SysGetCameraMatrixPtr()` (`0x00835ba0`).
    pub fn ch_sys_get_camera_matrix_ptr(mem: &impl Mem) -> u32 {
        CAMERA_MATRIX.ptr(mem).addr
    }

    /// `Ch_SysSetScreenMatrix(m)` (`0x00835e90`) and
    /// `Ch_SysSetCameraMatrix(m)` (`0x00835eb0`): 64 bytes from `m`.
    pub fn set_matrix(mem: &mut impl Mem, g: Global<[u32; 16]>, m: u32) {
        let v: [u32; 16] = mem.read(m);
        g.set(mem, v);
    }

    /// `Ch_MotionGetHalf()` (`0x00839870`).
    pub fn ch_motion_get_half(mem: &impl Mem) -> u32 {
        MOTION_HALF.get(mem)
    }

    /// `Ch_MotionSetHalf(v)` (`0x00839950`).
    pub fn ch_motion_set_half(mem: &mut impl Mem, v: u32) {
        MOTION_HALF.set(mem, v);
    }

    /// `Ch_EffectPacketGetSize()` (`0x00845700`).
    pub fn ch_effect_packet_get_size(mem: &impl Mem) -> u32 {
        PACKET_SIZE.get(mem)
    }

    /// `Ch_EffectPacketStoreEND()` (`0x008457d0`): the 16-byte end tag
    /// (`0xff00`, words 2 and 3 zero) when writing, else only its size.
    pub fn ch_effect_packet_store_end(mem: &mut impl Mem) {
        let p = PACKET_PTR.get(mem);
        if p == 0 {
            let s = PACKET_SIZE.get(mem);
            PACKET_SIZE.set(mem, s.wrapping_add(16));
            return;
        }
        mem.write(p, 0xff00u32);
        mem.write(p + 8, 0u32);
        mem.write(p + 0xc, 0u32);
        PACKET_PTR.set(mem, p.wrapping_add(16));
    }

    /// `Ch_EffectPacketStoreMAT(v)` (`0x00845800`) and
    /// `Ch_EffectPacketStorePRG(v)` (`0x00845810`).
    pub fn set_u16(mem: &mut impl Mem, g: Global<u16>, v: u32) {
        #[allow(clippy::cast_possible_truncation)] // the low half
        g.set(mem, v as u16);
    }

    /// `u8` icon drawing (`Ch_SetDrawIcon`).
    pub const DRAW_ICON: Global<u8> = Global::new(0x0130_07e9);
    /// `u8` second icon drawing (`Ch_SetDrawIcon2`).
    pub const DRAW_ICON2: Global<u8> = Global::new(0x0130_07ea);
    /// `u32` set by `Ch_SetAllocateErr`.
    pub const ALLOCATE_ERR: Global<u32> = Global::new(0x012f_bd98);
    /// `i8` switch set by `Ch_SysSetSwitch7d8` (bit 0).
    pub const SWITCH_7D8: Global<i8> = Global::new(0x0130_07d8);
    /// `u32` set by `Ch_SysSet7e4`.
    pub const SYS_7E4: Global<u32> = Global::new(0x0130_07e4);
    /// `u32` set by `Ch_SysSet6e0`.
    pub const SYS_6E0: Global<u32> = Global::new(0x00c4_96e0);
    /// `Ch_DrawReserveCalcNorm`'s round-robin cursor into the character
    /// array (0: start over); cleared by `Ch_FunctionHistoryClear`.
    pub const CALC_NORM_CURSOR: Global<u32> = Global::new(0x0130_09e8);

    /// `Ch_SetDrawIcon(on)` (`0x00830c70`) and `Ch_SetDrawIcon2(on)`
    /// (`0x00830c80`): bit 0.
    pub fn set_bit0(mem: &mut impl Mem, g: Global<u8>, on: u32) {
        #[allow(clippy::cast_possible_truncation)] // bit 0
        g.set(mem, (on & 1) as u8);
    }

    /// `Ch_SetAllocateErr()` (`0x008285d0`).
    pub fn ch_set_allocate_err(mem: &mut impl Mem) {
        ALLOCATE_ERR.set(mem, 1);
    }

    /// `Ch_SysSetSwitch7d8(on)` (`0x008317b0`): bit 0; returns the old
    /// value, sign-extended.
    pub fn ch_sys_set_switch_7d8(mem: &mut impl Mem, on: u32) -> i32 {
        let old = i32::from(SWITCH_7D8.get(mem));
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // bit 0
        SWITCH_7D8.set(mem, (on & 1) as i8);
        old
    }

    /// The tag of the current effect packet block (`Ch_EffectPacketStorePoly`).
    pub const PACKET_HEAD: Global<u32> = Global::new(0x0130_3654);
    /// Polygons measured while only measuring.
    pub const PACKET_POLYS: Global<u32> = Global::new(0x0130_3660);
    /// The largest x and y measured (`i32`).
    pub const PACKET_MAX: Global<[i32; 2]> = Global::new(0x0130_3664);
    /// `u16` texture index, MAT, PRG and one more state, each followed by
    /// the value the current block was opened with (`0x0130366c`..`8a`,
    /// 4 bytes apart).
    pub const PACKET_STATE: Global<u16> = Global::new(0x0130_366c);
    /// The texture of the current block (a texture list entry, or 0).
    pub const PACKET_TEX: Global<u32> = Global::new(0x0130_368c);

    /// `u16 * 4096 / (size * 16)` truncated, as `fistp` in chop mode; the
    /// low half stored.
    fn uv(v: u16, size: u16) -> u16 {
        let q = f64::from(v) * 4096.0 / (f64::from(size) * 16.0);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // in range
        let r = if q.is_finite() {
            q.trunc() as i64 as u32
        } else {
            0x8000_0000
        };
        #[allow(clippy::cast_possible_truncation)] // the low half
        let r = r as u16;
        r
    }

    /// `Ch_EffectPacketStorePoly(a, b, c)` (`0x00845820`): one triangle
    /// (vertices of `u16` x, y, u, v). A changed texture/MAT/PRG state
    /// opens a new 32-byte block (tag `0x1000`, the texture's registers,
    /// or `0x8044` without one). A triangle takes 28 bytes: x's at `+0`,
    /// y's at `+0x14`, with a texture u, v in 4.12 of its size at `+8`.
    /// While only measuring, the sizes, a polygon count and the largest x
    /// and y are kept instead.
    pub fn ch_effect_packet_store_poly(mem: &mut impl Mem, a: u32, b: u32, c: u32) {
        let st = PACKET_STATE.ptr(mem).addr;
        let changed =
            (0..4).any(|k| mem.read::<u16>(st + 8 * k) != mem.read::<u16>(st + 8 * k + 4));
        let mut size_add = 0u32;
        let mut p = PACKET_PTR.get(mem);
        if changed {
            for k in 0..4 {
                let v: u16 = mem.read(st + 8 * k);
                mem.write(st + 8 * k + 4, v);
            }
            if p == 0 {
                size_add = 0x20;
            } else {
                let list = crate::texanim::bridge::TEX0_LIST.get(mem);
                let tex = if list == 0 {
                    0
                } else {
                    let i = i32::from(mem.read::<i16>(st));
                    list.wrapping_add(i.cast_unsigned() << 5)
                };
                PACKET_TEX.set(mem, tex);
                PACKET_HEAD.set(mem, p);
                mem.write(p, [0x1000u32, 0x7fc0_c0c0]);
                if tex == 0 {
                    mem.write(p + 0xc, 0u32);
                    mem.write(p + 8, 0u32);
                    mem.write(p + 0x10, 0x8044u16);
                } else {
                    let r: [u32; 2] = mem.read(tex + 0x14);
                    mem.write(p + 8, r);
                    let v: [u8; 2] = mem.read(tex + 0x11);
                    mem.write(p + 0x10, v);
                }
                p += 0x20;
                PACKET_PTR.set(mem, p);
            }
        }
        if p == 0 {
            let s = PACKET_SIZE.get(mem);
            PACKET_SIZE.set(mem, s.wrapping_add(size_add).wrapping_add(0x1c));
            let mut max = PACKET_MAX.get(mem);
            for (k, m) in (0u32..).zip(max.iter_mut()) {
                let v = [a, b, c].map(|q| i32::from(mem.read::<u16>(q + 2 * k)));
                let top = v[0].max(v[1]).max(v[2]);
                if top > *m {
                    *m = top;
                }
            }
            PACKET_MAX.set(mem, max);
            let n = PACKET_POLYS.get(mem);
            PACKET_POLYS.set(mem, n.wrapping_add(1));
            return;
        }
        let tex = PACKET_TEX.get(mem);
        mem.write(p + 6, 0u16);
        let v = [a, b, c].map(|q| mem.read::<[u16; 4]>(q));
        mem.write(p, [v[0][0], v[1][0], v[2][0]]);
        mem.write(p + 0x14, [v[0][1], v[1][1], v[2][1]]);
        if tex != 0 {
            let (w, h): (u16, u16) = (mem.read(tex + 8), mem.read(tex + 0xa));
            for (k, q) in (0u32..).zip(v) {
                mem.write(p + 8 + 4 * k, [uv(q[2], w), uv(q[3], h)]);
            }
        }
        let head = PACKET_HEAD.get(mem);
        let n: u16 = mem.read(head + 2);
        mem.write(head + 2, n.wrapping_add(1));
        PACKET_PTR.set(mem, p + 0x1c);
    }

    /// The effect packets' colour scale (`f32` × 3, `Ch_EffectSetColor`).
    pub const EFFECT_COLOR: Global<[f32; 3]> = Global::new(0x0130_33ac);
    /// The effect packets' alpha (`i32`, `Ch_EffectSetAlpha`).
    pub const EFFECT_ALPHA: Global<u32> = Global::new(0x0130_33b8);
    /// `128.0` (`double`).
    pub const K_128: Global<f64> = Global::new(0x00b4_c170);

    /// `Ch_EffectSetColor(r, g, b)` (`0x00842c40`): through the x87.
    pub fn ch_effect_set_color(mem: &mut impl Mem, rgb: [f32; 3]) {
        EFFECT_COLOR.set(mem, rgb.map(crate::x87::load));
    }

    /// `Ch_EffectSetAlpha(a)` (`0x00842c60`): `_ftol2_sse(a × 128)`.
    pub fn ch_effect_set_alpha(mem: &mut impl Mem, a: f32) {
        let x = crate::x87::Reg::load(a)
            .mul_st(crate::x87::Reg::constant(K_128.get(mem)))
            .get();
        let v = ffx_bridge::x87::ftol(mem, x);
        EFFECT_ALPHA.set(mem, v);
    }

    /// `Chr_TableIndex(id)` (`0x0082a640`) and the `atoi` import slot.
    pub const TABLE_INDEX: u32 = 0x0082_a640;
    pub const ATOI_SLOT: u32 = 0x00b0_c40c;
    /// Bytes of scratch `ch_get_char_id` needs (its 4-byte digit buffer).
    pub const CHAR_ID_SCRATCH: u32 = 4;

    /// `Ch_GetCharID(id)` (`0x0082a080`): the file name of `id`'s table
    /// entry (`g_CdLib_98(Chr_TableIndex(id))`; none: -1) gives its number,
    /// `atoi` of the 7th to 5th characters from the end. The original keeps
    /// those three in a 4-byte stack slot holding the caller's ECX, so its
    /// 4th byte (and any of the three a short name does not reach) is
    /// undefined; here they are NUL, which matches the original whenever
    /// that byte is not a digit.
    pub fn ch_get_char_id<M: Mem>(
        mem: &mut M,
        id: u32,
        scratch: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let k = call(mem, mem.rebase(TABLE_INDEX), &[id]);
        let f = CD_LIB_98.get(mem);
        let s = call(mem, f, &[k]);
        if s == 0 {
            return u32::MAX;
        }
        let mut n = 0u32;
        while mem.read::<u8>(s.wrapping_add(n)) != 0 {
            n += 1;
        }
        mem.write(scratch, 0u32);
        for (j, back) in [7u32, 6, 5].into_iter().enumerate() {
            if n >= back {
                let b: u8 = mem.read(s.wrapping_add(n - back));
                #[allow(clippy::cast_possible_truncation)] // 0..3
                mem.write(scratch + j as u32, b);
            }
        }
        let atoi: u32 = mem.read(mem.rebase(ATOI_SLOT));
        call(mem, atoi, &[scratch])
    }

    /// `ch_create_name(id)` (`0x00838100`), `ch_create_section(id, name)`
    /// (`0x00838150`), `Cache_Entry_RenderData(kind, id, &a, &b, &c, &d)`
    /// (`0x00840680`), `Ch_LoadHdRenderData(file, c, d, id)` (`0x0082a1c0`),
    /// the `sprintf` import slot and the block name format
    /// `"[%s:%s]CHRDATA"` (`0x00b596a8`).
    pub const CREATE_NAME: u32 = 0x0083_8100;
    pub const CREATE_SECTION: u32 = 0x0083_8150;
    pub const CACHE_ENTRY_RENDER_DATA: u32 = 0x0084_0680;
    pub const LOAD_HD_RENDER_DATA: u32 = 0x0082_a1c0;
    pub const SPRINTF_IMPORT: u32 = 0x00b0_c3dc;
    pub const CHRDATA_FORMAT: u32 = 0x00b5_96a8;
    /// Bytes of scratch `ch_data_read` needs: the original's frame (offset
    /// `0x74 - n` stands for `[ebp - n]`; the read request at `[ebp-0x60]`).
    pub const DATA_READ_SCRATCH: u32 = 0x74;

    /// `Ch_DataRead(id)` (`0x00829ef0`): queues the read of a character's
    /// data unless it is cached or loading (`Cache_CheckYet(1, id)` 1 or 0:
    /// 0). Its table entry (`Chr_TableIndex`; none, or no size from
    /// `g_CdLib_14`: -1) gets a block of the size rounded as the heap
    /// rounds, named `"[<section>:<name>]CHRDATA"`, entered in the cache
    /// (`Cache_Entry`, `Cache_Entry_NotYet`), and read into
    /// (`g_CdLib(index, block, Ch_RomReadChrCB, request)`, the request's
    /// `+0xc` = id, `+0x10` = block, `+0x14` = 1). The HD render data is
    /// loaded too (`Ch_LoadHdRenderData(file, ...)` into the cache's render
    /// slot, its second slot cleared). Returns 0. (`dbgPrintf` is left out.)
    pub fn ch_data_read<M: Mem>(
        mem: &mut M,
        id: u32,
        scratch: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let at = |n: u32| scratch + 0x74 - n;
        let state = call(
            mem,
            mem.rebase(crate::motion::bridge::CACHE_CHECK_YET),
            &[1, id],
        );
        if state <= 1 {
            return 0;
        }
        let index = call(mem, mem.rebase(0x0082_a640), &[id]);
        mem.write(at(0x70), index);
        if index.cast_signed() < 0 {
            return u32::MAX;
        }
        let f = CD_LIB_14.get(mem);
        let size = call(mem, f, &[index]);
        if size == 0 {
            return u32::MAX;
        }
        let len = (size.wrapping_add(0x8f) & 0xffff_ff80).wrapping_sub(0x10);
        let block = call(mem, mem.rebase(crate::handle::bridge::USER_MALLOC), &[len]);
        let name = call(mem, mem.rebase(CREATE_NAME), &[id]);
        let section = call(mem, mem.rebase(CREATE_SECTION), &[id, name]);
        let sprintf: u32 = mem.read(mem.rebase(SPRINTF_IMPORT));
        // the original leaves `name` pushed: it is the format's second %s
        call(
            mem,
            sprintf,
            &[at(0x44), mem.rebase(CHRDATA_FORMAT), section, name],
        );
        call(
            mem,
            mem.rebase(crate::handle::bridge::MEM_SET_STR),
            &[block, at(0x44), u32::MAX],
        );
        let f = CD_LIB_98.get(mem);
        let file = call(mem, f, &[index]);
        mem.write(at(0x54), id);
        mem.write(at(0x50), block);
        mem.write(at(0x4c), 1u32);
        call(mem, mem.rebase(CACHE_ENTRY), &[1, id, block]);
        call(mem, mem.rebase(CACHE_ENTRY_NOT_YET), &[1, id, 1]);
        let f = CD_LIB.get(mem);
        let index: u32 = mem.read(at(0x70));
        call(
            mem,
            f,
            &[index, block, mem.rebase(ROM_READ_CHR_CB), at(0x60)],
        );
        call(
            mem,
            mem.rebase(CACHE_ENTRY_RENDER_DATA),
            &[1, id, at(0x64), at(0x68), at(0x74), at(0x6c)],
        );
        let (c, d) = (mem.read::<u32>(at(0x74)), mem.read::<u32>(at(0x6c)));
        let r = call(mem, mem.rebase(LOAD_HD_RENDER_DATA), &[file, c, d, id]);
        let a: u32 = mem.read(at(0x64));
        mem.write(a, r);
        let b: u32 = mem.read(at(0x68));
        mem.write(b, 0u32);
        0
    }

    /// `Ch_RomReadEventMGRPcb(req)` (`0x0082a7d0`): the event group read
    /// for `req` is added (`ChN_AddEventMgrp(+0x10, +0xc)`).
    pub fn ch_rom_read_event_mgrp_cb<M: Mem>(
        mem: &mut M,
        req: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let (data, x) = (
            mem.read::<u32>(req.wrapping_add(0x10)),
            mem.read::<u32>(req.wrapping_add(0xc)),
        );
        call(mem, mem.rebase(0x0083_7630), &[data, x])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thresholds_clamp() {
        assert_eq!(mouth_threshold(-1), 0);
        assert_eq!(mouth_threshold(201), 200);
        assert_eq!(mouth_threshold(77), 77);
    }
}

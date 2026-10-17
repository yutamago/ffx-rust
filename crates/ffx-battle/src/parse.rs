//! A command taking effect (`docs/rewrite-plan.md`, M2): the command cue's
//! first entry chosen and started (`MsCommandCueProcess`, `0x00792210`),
//! its command parsed into the actor's motions, effect and cameras
//! (`MsParseCommand`), and its magic effect file read (`MsEffectRead`).
//!
//! The effect files are system's and vfx's (`MagicFile_*`, the overlay
//! effects, the CD library): those go to the game's code through `s.call`.

/// The game's memory, through [`ffx_bridge::Mem`].
#[allow(clippy::many_single_char_names)] // indices, actors, the script calls
pub mod bridge {
    use ffx_bridge::{Global, IMAGE_BASE, Mem};

    use crate::actor::bridge::ms_get_chr;
    use crate::atel::bridge::Script;

    fn byte(mem: &impl Mem, p: u32, o: u32) -> u8 {
        mem.read(p.wrapping_add(o))
    }

    fn code(mem: &impl Mem, sym: u32) -> u32 {
        sym.wrapping_sub(IMAGE_BASE).wrapping_add(mem.image_base())
    }

    /// The magic effect being read (`MsEffectRead`; its pair is
    /// `misc::LAST_MAGIC2`).
    pub const LAST_MAGIC: Global<u32> = Global::new(0x0112_c8ec);
    /// Set: no magic file is read (`MsEffectRead` reads file 0).
    pub const NO_MAGIC_FILE: Global<u8> = Global::new(0x0112_a902);
    /// Cleared when a read starts.
    pub const EFFECT_READ_STAT: Global<u32> = Global::new(0x0113_3368);
    /// The actor whose effect file is being read (`MsEffectReadComplete`).
    pub const EFFECT_READ_CHR: Global<u32> = Global::new(0x0113_337c);
    /// Set while the overlay effect stands in for a file still loading.
    pub const EFFECT_OVERLAY_BUSY: Global<u8> = Global::new(0x0112_c9d8);
    /// The overlay effect table an actor plays while its file loads
    /// (`+0xf8c`).
    pub const EFFECT_OVERLAY_DEFAULT: Global<u8> = Global::new(0x0134_0870);
    /// Set for effect 0x84 by actor model 0x108f in scene 0x1e70000.
    pub const EFFECT_SPECIAL_SCENE: Global<u32> = Global::new(0x0194_0abc);
    /// Cleared after a read starts.
    pub const EFFECT_READ_CLEAR: Global<u32> = Global::new(0x00c4_21e8);
    /// Set once effect file 0x15e was taken preloaded.
    pub const EFFECT_PRELOADED: Global<u32> = Global::new(0x012f_b8b8);

    /// The game's functions the effect read calls.
    pub const MS_BATTLE_ERROR: u32 = 0x0079_f210;
    pub const GET_CD: u32 = 0x0076_d0d0;
    pub const MAGIC_FILE_SET_MAGIC_ID: u32 = 0x009d_a7a0;
    pub const MAGIC_FILE_LOAD: u32 = 0x009d_a430;
    pub const MAGIC_FILE_IS_PRX_PRELOADED: u32 = 0x009d_ae60;
    pub const MAGIC_FILE_START: u32 = 0x009d_a7f0;
    pub const MAGIC_FILE_GET_EFFECT_OVERLAY_TABLE: u32 = 0x009d_a370;
    pub const MAGIC_FILE_CLEANUP: u32 = 0x009d_a2d0;
    pub const IS_LOAD_TEXTURE: u32 = 0x009d_a280;
    pub const GRAPHIC_IS_MAGIC_FILE_ASYNC_LOAD_AVAILABLE: u32 = 0x0064_1c60;
    pub const OP_OVL_EFFECT_INIT: u32 = 0x007f_cdd0;
    pub const CH_GET_ID: u32 = 0x0082_6b80;
    /// `MsBattleError`'s messages (`dbgPrintf`, then it hangs).
    pub const EFFECT_READ_ERROR: u32 = 0x00b5_49f8;

    /// `MsEffectRead(i, id)` (`0x00787f20`): actor `i & 0xff` starts
    /// reading magic effect `id` (`+0xdfb` its read state). Reading while
    /// an event effect plays or the actor reads already is an error
    /// (`MsBattleError`, which hangs): -1. With a stone effect playing only
    /// the state goes to 0xe. Otherwise the battle camera is reset
    /// (`MsAtelCamInit`), the CD library told 0x11 (its `+0x10`), state 1,
    /// and the effect's file (file 0 when [`NO_MAGIC_FILE`]) set and loaded
    /// (`MagicFile_SetMagicId`, `MagicFile_Load`); effect 0x84 by model
    /// 0x108f in scene 0x1e70000 marks [`EFFECT_SPECIAL_SCENE`]. A load
    /// to wait for (positive, without `AEON_PLAIN2`) has the actor play
    /// (`+0xdfd`) either the asynchronous file (state 0x10) or the overlay
    /// effect meanwhile ([`EFFECT_OVERLAY_DEFAULT`], `op_ovl_effect_init`);
    /// a preloaded effect file 0x15e completes at once
    /// ([`ms_effect_read_complete`]). Else nothing plays. 0.
    pub fn ms_effect_read<M: Mem>(mem: &mut M, s: &mut Script<'_, M>, i: u32, id: u32) -> i32 {
        let i = i & 0xff;
        let a = ms_get_chr(mem, i);
        if crate::state::bridge::EVENT_EFFECT_STAT.get(mem) != 0 || byte(mem, a, 0xdfb) != 0 {
            let (f, msg) = (code(mem, MS_BATTLE_ERROR), code(mem, EFFECT_READ_ERROR));
            (s.call)(mem, f, &[msg]);
            return -1;
        }
        LAST_MAGIC.set(mem, id);
        crate::misc::bridge::LAST_MAGIC2.set(mem, id);
        if crate::state::bridge::ms_stone_effect_check(mem) != 0 {
            mem.write(a.wrapping_add(0xdfb), 0xeu8);
            return 0;
        }
        let file = if NO_MAGIC_FILE.get(mem) == 0 { id } else { 0 };
        crate::minor::bridge::ms_atel_cam_init(mem);
        mem.write(a.wrapping_add(0xdfe), 0u8);
        mem.write(a.wrapping_add(0xe0d), 0u8);
        EFFECT_READ_STAT.set(mem, 0);
        let f = code(mem, GET_CD);
        let cd = (s.call)(mem, f, &[]);
        let f: u32 = mem.read(cd.wrapping_add(0x10));
        (s.call)(mem, f, &[0x11]);
        mem.write(a.wrapping_add(0xdfb), 1u8);
        let n = crate::exec::bridge::ms_effect_file_number(mem, file.cast_signed(), -1);
        let f = code(mem, MAGIC_FILE_SET_MAGIC_ID);
        (s.call)(mem, f, &[file]);
        let f = code(mem, MAGIC_FILE_LOAD);
        let load = (s.call)(mem, f, &[]).cast_signed();
        if file == 0x84 && crate::state::bridge::ms_get_battle_scene(mem) == 0x01e7_0000 {
            let f = code(mem, CH_GET_ID);
            let model: u32 = mem.read(a);
            if (s.call)(mem, f, &[model]) == 0x108f {
                EFFECT_SPECIAL_SCENE.set(mem, 1);
            }
        }
        if crate::action::bridge::AEON_PLAIN2.get(mem) != 0 || load <= 0 {
            mem.write(a.wrapping_add(0xdfd), 0u8);
            mem.write(a.wrapping_add(0xf8c), 0u32);
            return 0;
        }
        let async_load = code(mem, GRAPHIC_IS_MAGIC_FILE_ASYNC_LOAD_AVAILABLE);
        let available = (s.call)(mem, async_load, &[]);
        EFFECT_READ_CHR.set(mem, i);
        if available != 0 {
            mem.write(a.wrapping_add(0xdfd), 1u8);
            mem.write(a.wrapping_add(0xdfb), 0x10u8);
            mem.write(a.wrapping_add(0xf8c), 0u32);
        } else {
            EFFECT_OVERLAY_BUSY.set(mem, 1);
            mem.write(a.wrapping_add(0xdfd), 1u8);
            let table = EFFECT_OVERLAY_DEFAULT.ptr(mem).addr;
            mem.write(a.wrapping_add(0xf8c), table);
            let f = code(mem, OP_OVL_EFFECT_INIT);
            (s.call)(mem, f, &[]);
        }
        EFFECT_READ_CLEAR.set(mem, 0);
        if n != 0x15e {
            return 0;
        }
        let f = code(mem, MAGIC_FILE_IS_PRX_PRELOADED);
        if (s.call)(mem, f, &[0x15e]) == 0
            || (s.call)(mem, async_load, &[]) != 3
            || EFFECT_PRELOADED.get(mem) != 0
        {
            return 0;
        }
        let f = code(mem, IS_LOAD_TEXTURE);
        if (s.call)(mem, f, &[]) == 0 {
            return 0;
        }
        ms_effect_read_complete(mem, s);
        EFFECT_PRELOADED.set(mem, 1);
        0
    }

    /// `MsEffectReadComplete()` (`0x00788110`): the reading actor
    /// ([`EFFECT_READ_CHR`]) plays the overlay effect while the file can
    /// load asynchronously (state 1); a started file (`MagicFile_Start`)
    /// gives its overlay table (`+0xf8c`), the read ends ([`EFFECT_READ_CHR`]
    /// 0); a failed one is cleaned up (`MagicFile_Cleanup`). Returns the
    /// table or the clean-up's result.
    pub fn ms_effect_read_complete<M: Mem>(mem: &mut M, s: &mut Script<'_, M>) -> u32 {
        let a = ms_get_chr(mem, EFFECT_READ_CHR.get(mem) & 0xff);
        let f = code(mem, GRAPHIC_IS_MAGIC_FILE_ASYNC_LOAD_AVAILABLE);
        if (s.call)(mem, f, &[]) != 0 {
            EFFECT_OVERLAY_BUSY.set(mem, 1);
            mem.write(a.wrapping_add(0xdfd), 1u8);
            mem.write(a.wrapping_add(0xdfb), 1u8);
            let table = EFFECT_OVERLAY_DEFAULT.ptr(mem).addr;
            mem.write(a.wrapping_add(0xf8c), table);
            let f = code(mem, OP_OVL_EFFECT_INIT);
            (s.call)(mem, f, &[]);
        }
        let f = code(mem, MAGIC_FILE_START);
        if (s.call)(mem, f, &[]) != 0 {
            let f = code(mem, MAGIC_FILE_GET_EFFECT_OVERLAY_TABLE);
            let table = (s.call)(mem, f, &[0]);
            mem.write(a.wrapping_add(0xf8c), table);
            mem.write(a.wrapping_add(0xdfd), 0u8);
            EFFECT_OVERLAY_BUSY.set(mem, 0);
            EFFECT_READ_CHR.set(mem, 0);
            return table;
        }
        EFFECT_OVERLAY_BUSY.set(mem, 0);
        mem.write(a.wrapping_add(0xdfd), 0u8);
        mem.write(a.wrapping_add(0xf8c), 0u32);
        let f = code(mem, MAGIC_FILE_CLEANUP);
        (s.call)(mem, f, &[])
    }
}

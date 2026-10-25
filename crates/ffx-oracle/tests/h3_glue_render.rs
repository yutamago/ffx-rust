//! `ffx_prender::glue` (M4 port round 1, list `h3-glue-render`) against
//! the original: on the image alone with synthetic objects, and on the
//! snapshots (`FFX_SNAPSHOT`) with the game's own scene and managers.
//! Results and every byte written must agree. Needs `FFX_GAME_DIR`.
//!
//! Callees that stay original are replaced in both oracles alike by
//! recording stubs: each call appends its tag, `ecx` (for methods) and its
//! arguments to a log in memory, so the diff checks that both made the
//! same calls in the same order with the same `this`. On the image the
//! singleton getters are stubs too (returning a synthetic object, or 0);
//! on a snapshot they run as they are, over the game's instances.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_prender::glue::{self, bridge as g};

/// `graphicFontDraw`'s body: no test here calls it, so the recording
/// stubs are written over it.
const CAVE: u32 = 0x0064_0660;
const CAVE_LEN: u32 = 1760;
/// After the stubs: `mov eax, POISON; ret`, which [`G`] calls before each
/// original so that a callee leaving `eax` alone shows the poison, not what
/// the previous call returned.
const POISON_AT: u32 = CAVE + CAVE_LEN;
const POISON: u32 = 0xdead_beef;

/// Deterministic xorshift stream.
struct Rng(u32);

impl Rng {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
    fn below(&mut self, n: u32) -> u32 {
        self.next() % n
    }
    fn chance(&mut self, n: u32) -> bool {
        self.below(n) == 0
    }
    /// Bytes, about half of them 0 (flags read as bytes see both).
    fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n)
            .map(|_| if self.chance(2) { 0 } else { self.next() as u8 })
            .collect()
    }
    /// A word: small values, 0, 1, -1 and anything.
    fn word(&mut self) -> u32 {
        match self.below(5) {
            0 => 0,
            1 => 1,
            2 => self.below(8),
            3 => 0xffff_ffff,
            _ => self.next(),
        }
    }
    /// A float that is not a signalling NaN (the emulator keeps those).
    fn float(&mut self) -> f32 {
        loop {
            let bits = match self.below(4) {
                0 => self.next(),
                1 => (self.below(4000) as f32 - 2000.0).to_bits(),
                2 => (self.next() as f32 / 1000.0).to_bits(),
                _ => [
                    0,
                    0x8000_0000,
                    0x7f80_0000,
                    0xff80_0000,
                    0x7fc0_0000,
                    1,
                    0x7f7f_ffff,
                ][self.below(7) as usize],
            };
            let nan = bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0;
            if !(nan && bits & 0x0040_0000 == 0) {
                return f32::from_bits(bits);
            }
        }
    }
}

/// The bridge's view of an oracle: its memory, and its emulator for the
/// original callees.
struct G<'a>(&'a mut Oracle, u32);

impl Mem for G<'_> {
    fn read_bytes(&self, addr: u32, out: &mut [u8]) {
        self.0.read_bytes(addr, out);
    }
    fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        Mem::write_bytes(self.0, addr, data);
    }
    fn image_base(&self) -> u32 {
        self.0.image_base()
    }
}

fn conv(c: host::Conv) -> Conv {
    match c {
        host::Conv::Cdecl => Conv::Cdecl,
        host::Conv::Stdcall => Conv::Stdcall,
        host::Conv::Thiscall(t) => Conv::Thiscall(t),
        host::Conv::Fastcall => Conv::Fastcall,
    }
}

impl Host for G<'_> {
    fn call(&mut self, addr: u32, c: host::Conv, args: &[u32]) -> u32 {
        ffx_oracle::check_width("Host::call", addr, args.len(), host::max_words(c, false));
        self.0.call(self.1, Conv::Cdecl, &[]).unwrap();
        self.0.call(addr, conv(c), args).unwrap().eax
    }
    fn scratch(&mut self) -> u32 {
        self.0.stack_range().start + 0x100
    }
}

/// What a function returns, to compare.
#[derive(Debug, Clone, Copy)]
enum Out {
    /// Nothing the callers read.
    None,
    /// `eax`.
    U32(u32),
}

struct Twins {
    a: Oracle,
    b: Oracle,
    /// The call log: a count, then entries of 8 words (tag, `this`, six
    /// arguments).
    log: u32,
    cave: u32,
    cave_next: u32,
    snapshot: bool,
    /// Bytes the stubs replaced, put back by [`Twins::reset`].
    patched: Vec<(u32, Vec<u8>)>,
}

fn game_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("FFX_GAME_DIR is not set; skipping");
    }
    dir
}

impl Twins {
    fn new(mut a: Oracle, mut b: Oracle, snapshot: bool) -> Self {
        a.set_fpu_control(0x027f).unwrap();
        b.set_fpu_control(0x027f).unwrap();
        let cave = a.rebase(CAVE);
        let mut t = Self {
            a,
            b,
            log: 0,
            cave,
            cave_next: cave,
            snapshot,
            patched: Vec::new(),
        };
        t.log = t.zeros(0x8000);
        let at = t.rebase(POISON_AT);
        let mut code = vec![0xb8];
        code.extend(POISON.to_le_bytes());
        code.push(0xc3);
        t.write(at, &code);
        t
    }

    fn image() -> Option<Self> {
        let exe = game_dir()?.join("FFX.exe");
        Some(Self::new(
            Oracle::load(&exe).unwrap(),
            Oracle::load(&exe).unwrap(),
            false,
        ))
    }

    fn snapshot() -> Option<Self> {
        let dir = game_dir()?;
        let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
            eprintln!("FFX_SNAPSHOT is not set; skipping");
            return None;
        };
        let exe = dir.join("FFX.exe");
        let pages = Pages::read(PathBuf::from(snap)).unwrap();
        Some(Self::new(
            Oracle::from_snapshot(&exe, &pages).unwrap(),
            Oracle::from_snapshot(&exe, &pages).unwrap(),
            true,
        ))
    }

    /// Both kinds: the image, then the snapshot when one is set.
    fn each(mut f: impl FnMut(&mut Self)) {
        if let Some(mut t) = Self::image() {
            f(&mut t);
        }
        if let Some(mut t) = Self::snapshot() {
            f(&mut t);
        }
    }

    fn rebase(&self, sym: u32) -> u32 {
        self.a.rebase(sym)
    }

    fn alloc(&mut self, bytes: &[u8]) -> u32 {
        let x = self.a.alloc_bytes(bytes).unwrap();
        assert_eq!(self.b.alloc_bytes(bytes).unwrap(), x);
        x
    }

    fn zeros(&mut self, n: usize) -> u32 {
        self.alloc(&vec![0; n])
    }

    fn write(&mut self, at: u32, bytes: &[u8]) {
        self.a.write(at, bytes).unwrap();
        self.b.write(at, bytes).unwrap();
    }

    fn w32(&mut self, at: u32, v: u32) {
        self.write(at, &v.to_le_bytes());
    }

    fn r32(&self, at: u32) -> u32 {
        Mem::read(&self.b, at)
    }

    /// Sets the global at `sym`.
    fn global(&mut self, sym: u32, v: u32) {
        let at = self.rebase(sym);
        self.w32(at, v);
    }

    /// Writes `bytes` at `at`, to be put back by [`Twins::reset`].
    fn patch(&mut self, at: u32, bytes: &[u8]) {
        let mut old = vec![0; bytes.len()];
        Mem::read_bytes(&self.b, at, &mut old);
        self.patched.push((at, old));
        self.write(at, bytes);
    }

    fn reset(&mut self) {
        for (at, old) in std::mem::take(&mut self.patched).into_iter().rev() {
            self.write(at, &old);
        }
        self.cave_next = self.cave;
        let log = self.log;
        self.w32(log, 0);
    }

    /// Writes a recording stub: logs `tag`, `ecx` when `this`, and `n`
    /// stack arguments; pops `pops` words; returns `ret` (`None`: leaves
    /// `eax` as it was). Placed at `at` when given (a `jmp` there), else
    /// only in the cave. Returns its address.
    fn stub(
        &mut self,
        at: Option<u32>,
        tag: u32,
        this: bool,
        n: u32,
        pops: u32,
        ret: Option<u32>,
    ) -> u32 {
        assert!(n <= 6);
        let log = self.log;
        // the arguments sit one word higher after `push eax`
        let keep = u8::from(ret.is_none()) * 4;
        let mut c = vec![];
        if keep != 0 {
            c.push(0x50); // push eax
        }
        c.push(0xa1);
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(tag.to_le_bytes()); // mov [eax], tag
        if this {
            c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        }
        for i in 0..n {
            c.extend([0x8b, 0x54, 0x24, (4 + 4 * i) as u8 + keep]); // mov edx, [esp+4+4i]
            c.extend([0x89, 0x50, (8 + 4 * i) as u8]); // mov [eax+8+4i], edx
        }
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        match ret {
            Some(ret) => {
                c.push(0xb8);
                c.extend(ret.to_le_bytes()); // mov eax, ret
            }
            None => c.push(0x58), // pop eax
        }
        if pops == 0 {
            c.push(0xc3);
        } else {
            c.push(0xc2);
            c.extend(((pops * 4) as u16).to_le_bytes());
        }
        let addr = self.cave_next;
        assert!(addr + c.len() as u32 <= self.cave + CAVE_LEN, "cave full");
        self.write(addr, &c);
        self.cave_next = addr + c.len() as u32;
        if let Some(at) = at {
            let mut j = vec![0xe9];
            j.extend(addr.wrapping_sub(at + 5).to_le_bytes());
            self.patch(at, &j);
        }
        addr
    }

    fn diff(&mut self, what: &str) {
        assert_eq!(self.a.diff(&mut self.b).unwrap(), [], "{what}: memory");
    }

    /// Runs the original at `sym` in `a` and `bridge` on `b`; compares the
    /// result as `bridge` says and every byte written.
    fn check(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        bridge: impl FnOnce(&mut G) -> Out,
    ) {
        let addr = self.rebase(sym);
        let ra = self
            .a
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        let poison = self.rebase(POISON_AT);
        let out = bridge(&mut G(&mut self.b, poison));
        assert!(
            ra.st0_raw.is_none(),
            "{what}: the original leaves ST0 set ({:?}); the port returns {out:?}",
            ra.st0
        );
        match out {
            Out::None => {}
            Out::U32(v) => assert_eq!(ra.eax, v, "{what}: eax"),
        }
        self.diff(what);
    }
}

// ---- table-driven one-liners ----

/// How a callee is replaced.
#[derive(Clone, Copy)]
enum K {
    /// A method: logs `ecx` and `n` stack words, pops them.
    This(u32),
    /// A method that leaves `eax` alone (a byte store): logs as
    /// [`K::This`].
    Keep(u32),
    /// cdecl: logs `n` stack words, pops none.
    Cdecl(u32),
    /// A singleton getter (cdecl, no arguments): on the image a stub
    /// returning the case's instance (the object, or 0); on a snapshot
    /// left original.
    Getter,
    /// A `stdcall` import with `n` words, through its slot (the stub's
    /// address is written into the slot).
    Import(u32),
}

/// `this` of a case.
#[derive(Clone, Copy)]
enum This {
    /// A cdecl original: none.
    None,
    /// A thiscall original on the object in this global (on a snapshot,
    /// the game's).
    Global(u32),
    /// A thiscall original on the synthetic object.
    Object,
}

/// A port: the oracle, `this` (0 for cdecl originals) and the stack
/// arguments.
type Port = fn(&mut G, u32, &[u32]) -> Out;

struct Case {
    name: &'static str,
    sym: u32,
    /// How the original takes `this`.
    this: This,
    args: usize,
    /// Float arguments (as bits), which [`Rng::float`] fills.
    float_args: bool,
    stubs: &'static [(u32, K)],
    port: Port,
}

const fn cdecl(
    name: &'static str,
    sym: u32,
    args: usize,
    stubs: &'static [(u32, K)],
    port: Port,
) -> Case {
    Case {
        name,
        sym,
        this: This::None,
        args,
        float_args: false,
        stubs,
        port,
    }
}

const fn method(
    name: &'static str,
    sym: u32,
    this: This,
    args: usize,
    stubs: &'static [(u32, K)],
    port: Port,
) -> Case {
    Case {
        name,
        sym,
        this,
        args,
        float_args: false,
        stubs,
        port,
    }
}

fn u(v: u32) -> Out {
    Out::U32(v)
}

/// The globals the one-liners load their object from.
const GLOBALS: [u32; 4] = [
    glue::SCENE.sym,
    glue::DYN_GEO_FILL.sym,
    glue::CHR_SHADER_RENDER.sym,
    glue::MENU_TEXTURE_MANAGER.sym,
];

const SCENE: u32 = glue::SCENE.sym;
const DYN_GEO: u32 = glue::DYN_GEO_FILL.sym;

#[rustfmt::skip]
const CASES: &[Case] = &[
    // g_PhyreScene one-liners
    cdecl("activate_loading_screen", 0x0063_9130, 0, &[(glue::SCENE_START_LOADING_SCREEN, K::This(0))], |m, _, _| u(g::activate_loading_screen(m))),
    cdecl("deactivate_loading_screen", 0x0063_de40, 0, &[(glue::SCENE_STOP_LOADING_SCREEN, K::This(0))], |m, _, _| u(g::deactivate_loading_screen(m))),
    cdecl("scene_clear_list", 0x0063_dac0, 0, &[(glue::SCENE_CLEAR_LIST, K::This(0))], |m, _, _| u(g::scene_clear_list(m))),
    cdecl("draw_main_menu_water_effect", 0x0063_e920, 0, &[(glue::SCENE_DRAW_MAIN_MENU_WAVE, K::This(0))], |m, _, _| u(g::draw_main_menu_water_effect(m))),
    cdecl("field_map_shutdown", 0x0064_03b0, 0, &[(glue::SCENE_FIELD_MAP_SHUTDOWN, K::This(0))], |m, _, _| u(g::field_map_shutdown(m))),
    cdecl("frame_start", 0x0064_0df0, 0, &[(glue::SCENE_FRAME_START, K::This(0))], |m, _, _| { g::frame_start(m); Out::None }),
    cdecl("level_get_name", 0x0064_1d90, 0, &[(glue::SCENE_LEVEL_NAME, K::This(0))], |m, _, _| u(g::level_get_name(m))),
    cdecl("render_loading_screen", 0x0064_2c00, 0, &[(glue::SCENE_RENDER_LOADING_SCREEN, K::This(0))], |m, _, _| u(g::render_loading_screen(m))),
    cdecl("show_auto_save_prompt", 0x0064_3de0, 0, &[(glue::SCENE_SHOW_AUTO_SAVE_PROMPT, K::This(0))], |m, _, _| { g::show_auto_save_prompt(m); Out::None }),
    cdecl("texture_video_update", 0x0064_42c0, 0, &[(glue::SCENE_UPDATE_TEXTURE_VIDEO, K::This(0))], |m, _, _| u(g::texture_video_update(m))),
    cdecl("field_map_initialize", 0x0064_0200, 0, &[(glue::SCENE_FIELD_MAP_INITIALIZE, K::This(1))], |m, _, _| u(g::field_map_initialize(m))),
    cdecl("is_split_screen_enabled", 0x0064_1cc0, 0, &[(glue::SCENE_SPLIT_SCREEN_STAT, K::This(0))], |m, _, _| u(g::is_split_screen_enabled(m))),
    // g_PhyreScene with one argument
    cdecl("atel_load_texture", 0x0063_9310, 1, &[(glue::SCENE_LOAD_ATEL_VFX_TEXTURE, K::This(1))], |m, _, a| u(g::atel_load_texture(m, a[0]))),
    cdecl("atel_unload_texture", 0x0063_9330, 1, &[(glue::SCENE_UNLOAD_ATEL_VFX_TEXTURE, K::This(1))], |m, _, a| u(g::atel_unload_texture(m, a[0]))),
    cdecl("character_instance_add_to_pipeline", 0x0063_a010, 1, &[(glue::SCENE_ADD_TO_PIPELINE, K::This(1))], |m, _, a| { g::character_instance_add_to_pipeline(m, a[0]); Out::None }),
    cdecl("get_cha_pos", 0x0064_1080, 1, &[(glue::SCENE_GET_CHA_POS, K::This(1))], |m, _, a| u(g::get_cha_pos(m, a[0]))),
    cdecl("magic_set_path", 0x0064_2050, 1, &[(glue::SCENE_SET_MAGIC_PATH, K::This(1))], |m, _, a| u(g::magic_set_path(m, a[0]))),
    cdecl("magic_set_to_be_delete_path", 0x0064_2070, 1, &[(glue::SCENE_SET_TO_BE_DELETE_PATH, K::This(1))], |m, _, a| u(g::magic_set_to_be_delete_path(m, a[0]))),
    cdecl("magic_vfx_load_texture", 0x0064_20b0, 1, &[(glue::SCENE_LOAD_MAGIC_VFX_TEXTURE, K::This(1))], |m, _, a| u(g::magic_vfx_load_texture(m, a[0]))),
    cdecl("magic_vfx_unload_texture", 0x0064_20f0, 1, &[(glue::SCENE_UNLOAD_MAGIC_VFX_TEXTURE, K::This(1))], |m, _, a| u(g::magic_vfx_unload_texture(m, a[0]))),
    cdecl("mesh_instance_group_id_set_visible", 0x0064_23d0, 1, &[(glue::SCENE_SET_GROUP_VISIBLE, K::This(1))], |m, _, a| u(g::mesh_instance_group_id_set_visible(m, a[0]))),
    cdecl("menu_unload_special_texture", 0x0064_2390, 1, &[(glue::MENU_UNLOAD_SPECIAL, K::This(1))], |m, _, a| u(g::menu_unload_special_texture(m, a[0]))),
    // dynamic geometry, the character shader renderer
    cdecl("dyn_geo_clear_buffer", 0x0063_fd80, 0, &[(glue::DYN_GEO_CLEAR, K::This(0))], |m, _, _| { g::dyn_geo_clear_buffer(m); Out::None }),
    cdecl("dyn_geo_current_buffer_id", 0x0063_ff50, 0, &[(glue::DYN_GEO_CURRENT_ID, K::This(0))], |m, _, _| u(g::dyn_geo_current_buffer_id(m))),
    cdecl("dyn_geo_lock", 0x0063_ff60, 0, &[(glue::DYN_GEO_LOCK, K::This(0))], |m, _, _| u(g::dyn_geo_lock(m))),
    cdecl("dyn_geo_unlock", 0x0064_0050, 0, &[(glue::DYN_GEO_UNLOCK, K::This(0))], |m, _, _| u(g::dyn_geo_unlock(m))),
    method("lock_for_record_num", 0x006d_eaf0, This::Global(DYN_GEO), 0, &[(glue::WAIT_FOR_SINGLE_OBJECT, K::Import(2))], |m, this, _| u(g::lock_for_record_num(m, this))),
    method("unlock_for_record_num", 0x006d_ed50, This::Global(DYN_GEO), 0, &[(glue::RELEASE_MUTEX, K::Import(1))], |m, this, _| u(g::unlock_for_record_num(m, this))),
    method("swap_dyn_geo_memory", 0x006d_ed40, This::Object, 0, &[(glue::DYN_GEO_SWAP, K::This(0))], |m, this, _| { g::swap_dyn_geo_memory(m, this); Out::None }),
    cdecl("shader_get_vertex_info", 0x0064_3db0, 0, &[(glue::CHR_GET_VERTEX_INFO, K::This(0))], |m, _, _| u(g::shader_get_vertex_info(m))),
    cdecl("shadow_start", 0x0064_3dc0, 0, &[(glue::CHR_SHADOW_START, K::This(0))], |m, _, _| u(g::shadow_start(m))),
    cdecl("shadow_end", 0x0064_3dd0, 0, &[(glue::CHR_SHADOW_END, K::This(0))], |m, _, _| { g::shadow_end(m); Out::None }),
    // manager singletons
    cdecl("broken_screen_set_flag5", 0x0063_9360, 0, &[(glue::BROKEN_SCREEN_INSTANCE, K::Getter), (glue::BROKEN_SET_5, K::Keep(0))], |m, _, _| u(g::broken_screen_set_flag5(m))),
    cdecl("broken_screen_set_flag4", 0x0063_9370, 0, &[(glue::BROKEN_SCREEN_INSTANCE, K::Getter), (glue::BROKEN_SET_4, K::Keep(0))], |m, _, _| u(g::broken_screen_set_flag4(m))),
    cdecl("broken_screen_set_flag6", 0x0063_9390, 0, &[(glue::BROKEN_SCREEN_INSTANCE, K::Getter), (glue::BROKEN_SET_6, K::Keep(0))], |m, _, _| u(g::broken_screen_set_flag6(m))),
    cdecl("broken_screen_reset", 0x0063_9490, 0, &[(glue::BROKEN_SCREEN_INSTANCE, K::Getter), (glue::BROKEN_RESET, K::This(0))], |m, _, _| { g::broken_screen_reset(m); Out::None }),
    cdecl("broken_screen_start_keep_frame", 0x0063_94a0, 0, &[(glue::BROKEN_SCREEN_INSTANCE, K::Getter), (glue::BROKEN_KEEP_FRAME, K::Keep(0))], |m, _, _| u(g::broken_screen_start_keep_frame(m))),
    cdecl("get_current_frame", 0x0064_10d0, 0, &[(glue::FMV_INSTANCE, K::Getter), (glue::FMV_CURRENT_FRAME, K::This(0))], |m, _, _| u(g::get_current_frame(m))),
    cdecl("video_pause", 0x0064_5de0, 0, &[(glue::FMV_INSTANCE, K::Getter), (glue::FMV_PAUSE, K::This(0))], |m, _, _| { g::video_pause(m); Out::None }),
    cdecl("destroy_fmv", 0x0063_df30, 0, &[(glue::FMV_INSTANCE, K::Getter), (glue::FMV_DESTROY, K::This(0))], |m, _, _| u(g::destroy_fmv(m))),
    cdecl("is_video_busy", 0x0064_1ce0, 0, &[(glue::FMV_INSTANCE, K::Getter), (glue::FMV_STATE_CHECK, K::This(0))], |m, _, _| u(g::is_video_busy(m))),
    cdecl("is_video_playing", 0x0064_1cf0, 0, &[(glue::FMV_INSTANCE, K::Getter)], |m, _, _| u(g::is_video_playing(m))),
    cdecl("scene_is_playing_video", 0x0064_5db0, 0, &[(glue::FMV_INSTANCE, K::Getter)], |m, _, _| u(g::scene_is_playing_video(m))),
    cdecl("magic_vfx_unload_special_shader_table", 0x0064_20e0, 0, &[(glue::VFX_SHADER_INSTANCE, K::Getter), (glue::VFX_SHADER_UNLOAD_MAGIC, K::This(0))], |m, _, _| u(g::magic_vfx_unload_special_shader_table(m))),
    cdecl("update_shadow_visibility", 0x0064_4ab0, 0, &[(glue::SHADOW_INSTANCE, K::Getter), (glue::SHADOW_CLEAR_CASCADES, K::This(0))], |m, _, _| u(g::update_shadow_visibility(m))),
    cdecl("get_file_read_sema_id", 0x0064_1150, 0, &[(glue::ASYNC_LOAD_INSTANCE, K::Getter)], |m, _, _| u(g::get_file_read_sema_id(m))),
    cdecl("get_old_file_read_complete_sema_id", 0x0064_1360, 0, &[(glue::ASYNC_LOAD_INSTANCE, K::Getter)], |m, _, _| u(g::get_old_file_read_complete_sema_id(m))),
    // the application
    cdecl("start_loading_screen", 0x0066_fb30, 0, &[(glue::GET_APPLICATION, K::Getter)], |m, _, _| u(g::start_loading_screen(m))),
    cdecl("stop_loading_screen", 0x0066_fb50, 0, &[(glue::GET_APPLICATION, K::Getter)], |m, _, _| u(g::stop_loading_screen(m))),
    cdecl("is_loading_screen_active", 0x0064_1c40, 0, &[(glue::GET_APPLICATION, K::Getter)], |m, _, _| u(g::is_loading_screen_active(m))),
    cdecl("set_flip_vsync", 0x0064_30e0, 1, &[(glue::GET_APPLICATION, K::Getter), (glue::SET_FLIP_VSYNC_INTERVAL, K::Cdecl(1))], |m, _, a| u(g::set_flip_vsync(m, a[0]))),
    // scene and geometry methods
    method("scene_set_magic_path", 0x0066_cfb0, This::Global(SCENE), 1, &[(glue::PSTRING_ASSIGN, K::This(1))], |m, this, a| u(g::scene_set_magic_path(m, this, a[0]))),
    method("scene_set_to_be_delete_path", 0x0066_e830, This::Global(SCENE), 1, &[(glue::PSTRING_ASSIGN, K::This(1))], |m, this, a| u(g::scene_set_to_be_delete_path(m, this, a[0]))),
    method("scene_load_atel_vfx_texture", 0x0066_3f00, This::Global(SCENE), 1, &[(glue::SCENE_LOAD_VFX_TEXTURE_ASYNC, K::This(2))], |m, this, a| u(g::scene_load_atel_vfx_texture(m, this, a[0]))),
    method("scene_unload_atel_vfx_texture", 0x0067_0b00, This::Global(SCENE), 1, &[(glue::SCENE_UNLOAD_VFX_TEXTURE, K::This(2))], |m, this, a| u(g::scene_unload_atel_vfx_texture(m, this, a[0]))),
    method("scene_release_vfx_meshes", 0x0065_9090, This::Global(SCENE), 0, &[(glue::RELEASE_MESHES, K::This(0))], |m, this, _| u(g::scene_release_vfx_meshes(m, this))),
    Case { float_args: true, ..method("set_z_correction", 0x006a_3b20, This::Object, 1, &[], |m, this, a| { g::set_z_correction(m, this, f32::from_bits(a[0])); Out::None }) },
    method("get_ready_a8", 0x006a_2010, This::Object, 0, &[], |m, this, _| u(g::get_ready_a8(m, this))),
];

/// Runs every case `n` times on `t` (fresh stubs and objects each time).
fn run_cases(t: &mut Twins, cases: &[Case], n: u32, rng: &mut Rng) {
    const OBJ_LEN: usize = 0x2000;
    let obj = t.zeros(OBJ_LEN);
    // one object per global, so a port reading the wrong one shows
    let owned: Vec<u32> = GLOBALS.iter().map(|_| t.zeros(0x100)).collect();
    let snapshot = t.snapshot;
    for case in cases {
        for i in 0..n {
            t.reset();
            // a fresh synthetic object: random bytes, about half of them 0,
            // with a valid pointer at +0xa8 (a record) and a first word that
            // is 0 every other time
            let bytes = rng.bytes(OBJ_LEN);
            t.write(obj, &bytes);
            t.w32(obj + 0xa8, obj + 0x400);
            if i % 2 == 0 {
                t.w32(obj, 0);
            } else {
                t.w32(obj, obj + 0x800);
            }
            if !snapshot {
                for (&gl, &o) in GLOBALS.iter().zip(&owned) {
                    let b = rng.bytes(0x100);
                    t.write(o, &b);
                    t.global(gl, o);
                }
            }
            // a getter gives the object, or 0 now and then where the case
            // only hands it to a method (fields of a null one would fault)
            let only_methods = case
                .stubs
                .iter()
                .any(|&(_, k)| matches!(k, K::This(_) | K::Keep(_)));
            let instance = if i % 4 == 3 && only_methods { 0 } else { obj };
            for &(sym, kind) in case.stubs {
                let at = t.rebase(sym);
                let ret = if rng.chance(2) {
                    rng.word()
                } else {
                    rng.word() ^ (sym & 0xff00)
                };
                match kind {
                    K::This(n) => {
                        t.stub(Some(at), sym, true, n, n, Some(ret));
                    }
                    K::Keep(n) => {
                        t.stub(Some(at), sym, true, n, n, None);
                    }
                    K::Cdecl(n) => {
                        t.stub(Some(at), sym, false, n, 0, Some(ret));
                    }
                    K::Getter => {
                        if !snapshot {
                            t.stub(Some(at), sym, false, 0, 0, Some(instance));
                        }
                    }
                    K::Import(n) => {
                        let s = t.stub(None, sym, false, n, n, Some(ret));
                        t.patch(at, &s.to_le_bytes());
                    }
                }
            }
            let args: Vec<u32> = (0..case.args)
                .map(|_| {
                    if case.float_args {
                        rng.float().to_bits()
                    } else {
                        rng.word()
                    }
                })
                .collect();
            let (this, conv) = match case.this {
                This::None => (0, Conv::Cdecl),
                This::Global(gl) => {
                    let this = t.r32(t.rebase(gl));
                    (this, Conv::Thiscall(this))
                }
                This::Object => (obj, Conv::Thiscall(obj)),
            };
            let what = format!("{} #{i}", case.name);
            let port = case.port;
            t.check(&what, case.sym, conv, &args, |m| port(m, this, &args));
        }
    }
}

#[test]
fn batch1_one_liners_match_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 8 } else { 32 };
        run_cases(t, CASES, n, &mut Rng(0x1234_5678));
    });
}

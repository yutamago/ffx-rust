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

use std::cell::Cell;
use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_prender::glue::{self, bridge as g};
use ffx_prender::glue2::{self, bridge as g2};
use ffx_prender::glue3::{self, bridge as g3};
use ffx_prender::glue4::{self, bridge as g4};
use ffx_prender::glue5::{self, bridge as g5};
use ffx_prender::glue6::{self, bridge as g6};

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
    /// `al` (the rest of `eax` is the caller's or garbage).
    U8(u8),
}

struct Twins {
    a: Oracle,
    b: Oracle,
    /// The call log: a count, then entries of 16 words (tag, `this`, up
    /// to 14 arguments).
    log: u32,
    cave: u32,
    cave_next: u32,
    snapshot: bool,
    /// [`Twins::stub`] logs the word at `ecx` instead of `ecx`.
    deref_this: bool,
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
            deref_this: false,
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
        assert!(n <= 14);
        let log = self.log;
        // the arguments sit one word higher after `push eax`
        let keep = u8::from(ret.is_none()) * 4;
        let mut c = vec![];
        if keep != 0 {
            c.push(0x50); // push eax
        }
        c.push(0xa1);
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x06]); // shl eax, 6
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(tag.to_le_bytes()); // mov [eax], tag
        if this && self.deref_this {
            c.extend([0x8b, 0x11, 0x89, 0x50, 0x04]); // mov edx, [ecx]; mov [eax+4], edx
        } else if this {
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
            Out::U8(v) => assert_eq!(ra.eax & 0xff, u32::from(v), "{what}: al"),
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
    /// As [`K::Keep`], logging the word at `ecx` (what the callee finds in
    /// the object's first field) instead of `ecx`.
    KeepDeref(u32),
    /// A method without arguments returning an address inside the
    /// synthetic object (`+0x200`), for originals that write through it.
    Entry,
    /// cdecl: logs `n` stack words, pops none.
    Cdecl(u32),
    /// A singleton getter (cdecl, no arguments): on the image a stub
    /// returning the case's instance (the object, or 0); on a snapshot
    /// left original.
    Getter,
    /// A `stdcall` import with `n` words, through its slot (the stub's
    /// address is written into the slot).
    Import(u32),
    /// A cdecl import with `n` words, through its slot.
    ImportC(u32),
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
    /// A stdcall original: no `this`.
    Stdcall,
}

/// A port: the oracle, `this` (0 for cdecl originals) and the stack
/// arguments.
type Port = fn(&mut G, u32, &[u32]) -> Out;

#[derive(Clone, Copy)]
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
    /// Extra set-up after the stubs: may change the object, `this` and the
    /// arguments.
    prep: Option<Prep>,
    /// Whether the getters may return 0 (the original only hands the
    /// instance to a method or tests it).
    null_instance: bool,
    /// A port with a frame at the original's address: the bytes below
    /// `ebp` it uses and the bytes of stack arguments (see
    /// [`FRAME_EBP`]).
    frame: Option<(u32, u32)>,
}

/// A case's extra set-up: the twins, the synthetic object, `this`, the
/// arguments.
type Prep = fn(&mut Twins, u32, &mut u32, &mut [u32], &mut Rng);

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
        prep: None,
        null_instance: false,
        frame: None,
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
        prep: None,
        null_instance: false,
        frame: None,
    }
}

fn u(v: u32) -> Out {
    Out::U32(v)
}

/// The globals the one-liners load their object from (one synthetic
/// object each on the image).
const GLOBALS: [u32; 6] = [
    glue::SCENE.sym,
    glue::DYN_GEO_FILL.sym,
    glue::CHR_SHADER_RENDER.sym,
    glue::MENU_TEXTURE_MANAGER.sym,
    glue2::RENDER_DELEGATION_HELPER.sym,
    glue2::IGGY_MENU_MANAGER.sym,
];

/// Object globals the originals test for 0: an object, or 0 now and then.
const NULLABLE: [u32; 2] = [glue2::GRAPHIC_ABMAP.sym, glue2::GRAPHIC_UI.sym];

/// Plain word globals: random words.
const WORDS: [u32; 3] = [
    glue2::GRAPHIC_SKIP_FRAME.sym,
    glue2::SHADOW_SIZE.sym,
    glue2::MAGIC_FILE_ASYNC_LOAD.sym,
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
    Case { null_instance: true, ..cdecl("destroy_fmv", 0x0063_df30, 0, &[(glue::FMV_INSTANCE, K::Getter), (glue::FMV_DESTROY, K::This(0))], |m, _, _| u(g::destroy_fmv(m))) },
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
#[allow(clippy::too_many_lines)] // the stub kinds
fn run_cases(t: &mut Twins, cases: &[Case], n: u32, rng: &mut Rng) {
    const OBJ_LEN: usize = 0x2000;
    let obj = t.zeros(OBJ_LEN);
    // one object per global, so a port reading the wrong one shows
    let owned: Vec<u32> = GLOBALS.iter().map(|_| t.zeros(0x100)).collect();
    let nullable: Vec<u32> = NULLABLE.iter().map(|_| t.zeros(0x100)).collect();
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
                for (&gl, &o) in NULLABLE.iter().zip(&nullable) {
                    let b = rng.bytes(0x100);
                    t.write(o, &b);
                    let v = if rng.chance(4) { 0 } else { o };
                    t.global(gl, v);
                }
                for &gl in &WORDS {
                    let v = rng.word();
                    t.global(gl, v);
                }
            }
            // a getter gives the object, or 0 now and then where the case
            // only hands it to a method (fields of a null one would fault)
            let instance = if i % 4 == 3 && case.null_instance {
                0
            } else {
                obj
            };
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
                    K::KeepDeref(n) => {
                        t.deref_this = true;
                        t.stub(Some(at), sym, true, n, n, None);
                        t.deref_this = false;
                    }
                    K::Keep(n) => {
                        t.stub(Some(at), sym, true, n, n, None);
                    }
                    K::Cdecl(n) => {
                        t.stub(Some(at), sym, false, n, 0, Some(ret));
                    }
                    K::Entry => {
                        t.stub(Some(at), sym, true, 0, 0, Some(obj + 0x200));
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
                    K::ImportC(n) => {
                        let s = t.stub(None, sym, false, n, 0, Some(ret));
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
            let mut args = args;
            let mut this = match case.this {
                This::None | This::Stdcall => 0,
                This::Global(gl) => t.r32(t.rebase(gl)),
                This::Object => obj,
            };
            if let Some(prep) = case.prep {
                prep(t, obj, &mut this, &mut args, rng);
            }
            let conv = match case.this {
                This::None => Conv::Cdecl,
                This::Stdcall => Conv::Stdcall,
                _ => Conv::Thiscall(this),
            };
            let what = format!("{} #{i}", case.name);
            let port = case.port;
            match case.frame {
                None => t.check(&what, case.sym, conv, &args, |m| port(m, this, &args)),
                Some((low, high)) => {
                    // the original's ebp: its stack arguments, the return
                    // address and the saved ebp below where the call pushes
                    let ebp = t.a.stack_range().end - 0x100 - 4 * args.len() as u32 - 8;
                    let lo = ebp - low;
                    let fill = rng.bytes((low + 8 + high) as usize);
                    t.write(lo, &fill);
                    FRAME_EBP.with(|f| f.set(ebp));
                    t.b.set_call_top(Some(lo - 0x100));
                    t.check(&what, case.sym, conv, &args, |m| port(m, this, &args));
                    t.b.set_call_top(None);
                    // what the callees saw through the frame's pointers
                    for (from, n) in [(lo, low), (ebp + 8, high)] {
                        let mut x = vec![0; n as usize];
                        let mut y = vec![0; n as usize];
                        Mem::read_bytes(&t.a, from, &mut x);
                        Mem::read_bytes(&t.b, from, &mut y);
                        assert_eq!(x, y, "{what}: frame at {from:#x}");
                    }
                }
            }
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

// ---- batch 2 (ffx_prender::glue2) ----

/// `args[0]` = the object.
fn arg_object(_: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], _: &mut Rng) {
    args[0] = obj;
}

/// `args[0]` = the object, or 0 now and then.
fn arg_object_or_null(_: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    args[0] = if rng.chance(4) { 0 } else { obj };
}

/// The task callback: `task = (f, arg)` with `f` a recording stub.
fn prep_task_callback(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let ret = rng.word();
    let f = t.stub(None, 0xf00d, false, 1, 0, Some(ret));
    t.w32(obj, f);
    args[0] = obj;
}

/// `onCutbufferShaderLoaded(shader, task)`: the task's scene placed so
/// that its `+0xfaa4` lands in the object.
fn prep_cutbuffer(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], _: &mut Rng) {
    let scene = (obj + 0x200).wrapping_sub(glue2::SCENE_CUTBUFFER_SHADER);
    t.w32(obj, scene);
    args[1] = obj;
}

/// `deleteIggyMenu(this)`: `this` placed so that its `+0x10dc8` lands in
/// the object.
fn prep_iggy(_: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], _: &mut Rng) {
    *this = (obj + 0x300).wrapping_sub(glue2::SCENE_IGGY_MENU);
}

/// The scene's "magic async off" byte: 1 half of the time (image only).
fn prep_magic_async(t: &mut Twins, _: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    // the PRX check: idle (0) half of the time
    let busy = if rng.chance(2) { 0 } else { rng.word() };
    let at = t.rebase(glue2::LOAD_PRX_IN_PROGRESS);
    t.stub(
        Some(at),
        glue2::LOAD_PRX_IN_PROGRESS,
        false,
        0,
        0,
        Some(busy),
    );
    if !t.snapshot {
        let scene = t.r32(t.rebase(SCENE));
        let v = if rng.chance(3) { 1 } else { rng.below(3) as u8 };
        t.write(scene + glue2::SCENE_MAGIC_ASYNC_OFF, &[v]);
    }
}

/// The sphere grid's state around 2 (image only, when it exists).
fn prep_abmap_state(t: &mut Twins, _: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let abmap = t.r32(t.rebase(glue2::GRAPHIC_ABMAP.sym));
    if !t.snapshot && abmap != 0 {
        let v = [0, 1, 2, 3, 0xffff_ffff, 0x8000_0000][rng.below(6) as usize];
        t.w32(abmap + 4, v);
    }
}

#[rustfmt::skip]
const CASES2: &[Case] = &[
    // post effects
    cdecl("activate_blur_effect_ex", 0x0063_9110, 1, &[(glue2::BLUR_EX_INSTANCE, K::Getter), (glue2::BLUR_SET_TARGET, K::This(1))], |m, _, a| u(g2::activate_blur_effect_ex(m, a[0]))),
    cdecl("blur_set_field84", 0x0064_3810, 1, &[(glue2::BLUR_EX_INSTANCE, K::Getter), (glue2::BLUR_SET_FIELD84, K::This(1))], |m, _, a| u(g2::blur_set_field84(m, a[0]))),
    Case { float_args: true, ..cdecl("blur_set_floats", 0x0064_3890, 2, &[(glue2::BLUR_EX_INSTANCE, K::Getter), (glue2::BLUR_SET_FLOATS6C, K::Keep(2))], |m, _, a| u(g2::blur_set_floats(m, a[0], a[1]))) },
    cdecl("magic_vfx_load_special_shader_table", 0x0064_2090, 1, &[(glue::VFX_SHADER_INSTANCE, K::Getter), (glue2::VFX_SHADER_LOAD_MAGIC, K::Keep(1))], |m, _, a| u(g2::magic_vfx_load_special_shader_table(m, a[0]))),
    cdecl("set_distortion_skipped", 0x0064_2f80, 1, &[(glue2::DISTORTION_INSTANCE, K::Getter)], |m, _, a| u(g2::set_distortion_skipped(m, a[0]))),
    cdecl("set_distortion_enable", 0x0064_2f50, 1, &[(glue2::DISTORTION_INSTANCE, K::Getter)], |m, _, a| u(g2::set_distortion_enable(m, a[0]))),
    Case { float_args: true, ..cdecl("set_distortion_blur", 0x0064_2f30, 1, &[(glue2::DISTORTION_INSTANCE, K::Getter), (glue2::DISTORTION_UPDATE_BLUR, K::This(1))], |m, _, a| u(g2::set_distortion_blur(m, a[0]))) },
    Case { float_args: true, ..cdecl("set_fog_limit", 0x0064_31c0, 1, &[(glue2::FOG_INSTANCE, K::Getter), (glue2::FOG_SET_LIMIT, K::This(1))], |m, _, a| u(g2::set_fog_limit(m, a[0]))) },
    Case { float_args: true, ..cdecl("set_motion_blur_weight", 0x0064_3750, 1, &[(glue2::MOTION_BLUR_INSTANCE, K::Getter), (glue2::MOTION_BLUR_SET_WEIGHT, K::This(1))], |m, _, a| { g2::set_motion_blur_weight(m, a[0]); Out::None }) },
    cdecl("set_motion_blur_enable", 0x0064_3730, 1, &[(glue2::MOTION_BLUR_INSTANCE, K::Getter)], |m, _, a| u(g2::set_motion_blur_enable(m, a[0]))),
    cdecl("cross_fade_capture_start", 0x0063_9150, 0, &[(glue2::POST_PROCESS_INSTANCE, K::Getter), (glue2::POST_SET_CROSS_FADE_CAPTURE, K::Keep(1)), (glue2::POST_SET_BYTE_123BB, K::Keep(1))], |m, _, _| u(g2::cross_fade_capture_start(m))),
    Case { prep: Some(arg_object_or_null), ..cdecl("water_distortion_capture_frame", 0x006f_5420, 1, &[(glue2::POST_PROCESS_INSTANCE, K::Getter), (glue2::POST_COPY_BUFFER, K::This(6))], |m, _, a| u(g2::water_distortion_capture_frame(m, a[0]))) },
    // the broken screen
    Case { float_args: true, ..cdecl("broken_screen_update_opacity", 0x0063_94e0, 1, &[(glue::BROKEN_SCREEN_INSTANCE, K::Getter), (glue2::BROKEN_SET_OPACITY, K::Keep(1))], |m, _, a| u(g2::broken_screen_update_opacity(m, a[0]))) },
    cdecl("broken_screen_init_block_data", 0x0063_9470, 3, &[(glue::BROKEN_SCREEN_INSTANCE, K::Getter), (glue2::BROKEN_INIT_BLOCK_DATA, K::This(3))], |m, _, a| u(g2::broken_screen_init_block_data(m, a[0], a[1], a[2]))),
    cdecl("broken_screen_update_light_parameters", 0x0063_94b0, 7, &[(glue::BROKEN_SCREEN_INSTANCE, K::Getter), (glue2::BROKEN_UPDATE_LIGHT, K::This(7))], |m, _, a| u(g2::broken_screen_update_light_parameters(m, [a[0], a[1], a[2], a[3], a[4], a[5], a[6]]))),
    cdecl("broken_screen_update_polygon_matrix", 0x0063_9540, 3, &[(glue::BROKEN_SCREEN_INSTANCE, K::Getter), (glue2::BROKEN_POLYGON, K::This(2)), (glue2::BROKEN_POLYGON_MATRIX, K::This(1))], |m, _, a| u(g2::broken_screen_update_polygon_matrix(m, a[0], a[1], a[2]))),
    cdecl("broken_screen_block_bytes", 0x0063_93a0, 1, &[(glue::BROKEN_SCREEN_INSTANCE, K::Getter)], |m, _, a| u(g2::broken_screen_block_bytes(m, a[0]))),
    // scene
    Case { float_args: true, ..cdecl("set_chadist", 0x0064_2ef0, 1, &[(glue2::SCENE_SET_CHADIST, K::This(1))], |m, _, a| { g2::set_chadist(m, a[0]); Out::None }) },
    cdecl("character_insert_draw_situation", 0x0063_9ff0, 1, &[(glue2::SCENE_ADD_CHR_DRAW_SITUATION, K::This(1))], |m, _, a| { g2::character_insert_draw_situation(m, a[0]); Out::None }),
    cdecl("field_map_unload", 0x0064_03c0, 0, &[(glue2::CLEAR_DEPTH_BIAS, K::Cdecl(0)), (glue2::SCENE_TEXTURE_VIDEO_UNLOAD, K::This(0)), (glue2::SCENE_FIELD_MAP_UNLOAD, K::This(0))], |m, _, _| u(g2::field_map_unload(m))),
    cdecl("render", 0x0064_2bb0, 0, &[(glue2::SCENE_RENDER, K::This(0)), (glue2::WAIT_PPP_FILL, K::Cdecl(0)), (glue2::SCENE_DO_DELETE_MESH_INSTANCES, K::This(0))], |m, _, _| u(g2::render(m))),
    cdecl("texture_animation_set_material_visibility", 0x0064_4120, 2, &[(glue2::TEX_ANIM_INSTANCE, K::Getter), (glue2::TEX_ANIM_MATERIAL_BY_LAYER, K::This(1)), (glue2::SCENE_SET_MATERIAL_VISIBILITY, K::Keep(2))], |m, _, a| u(g2::texture_animation_set_material_visibility(m, a[0], a[1]))),
    Case { prep: Some(prep_magic_async), ..cdecl("set_magic_file_async_load_enable", 0x0064_35e0, 1, &[(glue2::LOAD_PRX_IN_PROGRESS, K::Cdecl(0))], |m, _, a| u(g2::set_magic_file_async_load_enable(m, a[0]))) },
    method("scene_force_unload_menu_texture", 0x0065_d780, This::Global(SCENE), 1, &[(glue2::CLUSTER_INSTANCE, K::Getter), (glue2::CLUSTER_FORCE_RELEASE, K::This(1))], |m, _, a| u(g2::scene_force_unload_menu_texture(m, a[0]))),
    method("scene_unload_menu_texture", 0x0067_1120, This::Global(SCENE), 1, &[(glue2::CLUSTER_INSTANCE, K::Getter), (glue2::CLUSTER_RELEASE, K::This(1))], |m, _, a| u(g2::scene_unload_menu_texture(m, a[0]))),
    method("scene_load_menu_texture", 0x0066_51d0, This::Global(SCENE), 1, &[(glue::ASYNC_LOAD_INSTANCE, K::Getter), (glue2::ASYNC_LOAD_CLUSTER, K::Cdecl(5))], |m, _, a| u(g2::scene_load_menu_texture(m, a[0]))),
    Case { prep: Some(prep_cutbuffer), ..cdecl("scene_on_cutbuffer_shader_loaded", 0x0066_6610, 2, &[(glue::ASYNC_LOAD_INSTANCE, K::Getter)], |m, _, a| u(g2::scene_on_cutbuffer_shader_loaded(m, a[0], a[1]))) },
    Case { prep: Some(prep_iggy), ..method("scene_delete_iggy_menu", 0x0065_b750, This::Object, 0, &[(glue2::IGGY_MENU_DTOR, K::This(0)), (glue2::OPERATOR_DELETE, K::Cdecl(1))], |m, this, _| { g2::scene_delete_iggy_menu(m, this); Out::None }) },
    // async tasks
    Case { prep: Some(prep_task_callback), ..cdecl("async_task_callback", 0x0063_7860, 1, &[(glue::ASYNC_LOAD_INSTANCE, K::Getter)], |m, _, a| u(g2::async_task_callback(m, a[0]))) },
    cdecl("regist_async_task", 0x0064_2950, 2, &[(glue::ASYNC_LOAD_INSTANCE, K::Getter), (glue2::ASYNC_REGIST_TASK, K::Cdecl(7))], |m, _, a| u(g2::regist_async_task(m, a[0], a[1]))),
    cdecl("try_async_load_if_available", 0x0064_42f0, 2, &[(glue::ASYNC_LOAD_INSTANCE, K::Getter), (glue2::ASYNC_REGIST_TASK, K::Cdecl(7))], |m, _, a| u(g2::regist_async_task(m, a[0], a[1]))),
    Case { prep: Some(arg_object), ..cdecl("async_load_character_task", 0x0065_7850, 1, &[(glue::ASYNC_LOAD_INSTANCE, K::Getter), (glue2::SCENE_LOAD_CHARACTER, K::This(2)), (glue2::OPERATOR_DELETE_TASK, K::Cdecl(1))], |m, _, a| u(g2::async_load_character_task(m, a[0]))) },
    Case { prep: Some(arg_object), ..cdecl("async_load_magic_file_vfx_texture_task", 0x0065_7880, 1, &[(glue::ASYNC_LOAD_INSTANCE, K::Getter), (glue2::SCENE_LOAD_MAGIC_FILE_VFX_TEXTURE, K::This(3)), (glue2::OPERATOR_DELETE_TASK, K::Cdecl(1))], |m, _, a| u(g2::async_load_magic_file_vfx_texture_task(m, a[0]))) },
    Case { prep: Some(arg_object), ..cdecl("async_load_vfx_texture_task", 0x0065_78b0, 1, &[(glue::ASYNC_LOAD_INSTANCE, K::Getter), (glue2::SCENE_LOAD_VFX_TEXTURE, K::This(3)), (glue2::OPERATOR_DELETE_TASK, K::Cdecl(1))], |m, _, a| u(g2::async_load_vfx_texture_task(m, a[0]))) },
    // menus, the sphere grid, the UI
    cdecl("menu_load_special_texture", 0x0064_2300, 3, &[(glue2::MENU_LOAD_SPECIAL, K::This(3))], |m, _, a| u(g2::menu_load_special_texture(m, a[0], a[1], a[2]))),
    cdecl("abmap_get_vertex_info", 0x0063_8fd0, 2, &[(glue2::ABMAP_GET_VERTEX_INFO, K::This(2))], |m, _, a| u(g2::abmap_get_vertex_info(m, a[0], a[1]))),
    cdecl("abmap_reset_layers", 0x0063_90d0, 1, &[(glue2::ABMAP_RESET_LAYERS, K::This(1))], |m, _, a| { g2::abmap_reset_layers(m, a[0]); Out::None }),
    cdecl("abmap_create", 0x0063_8f90, 1, &[(glue::GET_APPLICATION, K::Getter), (glue2::RENDER_THREAD_INSTANCE, K::Getter), (glue2::RENDER_THREAD_SYNC, K::This(0)), (glue2::ABMAP_CREATE, K::Cdecl(1))], |m, _, a| u(g2::abmap_create(m, a[0]))),
    cdecl("abmap_destroy", 0x0063_8fb0, 0, &[(glue::GET_APPLICATION, K::Getter), (glue2::RENDER_THREAD_INSTANCE, K::Getter), (glue2::RENDER_THREAD_SYNC, K::This(0)), (glue2::ABMAP_TO_DESTROY, K::This(0))], |m, _, _| { g2::abmap_destroy(m); Out::None }),
    cdecl("map_part_lang_override", 0x0064_12b0, 2, &[(glue2::UI_LANG_OVERRIDE, K::This(2))], |m, _, a| u(g2::map_part_lang_override(m, a[0], a[1]))),
    cdecl("set_ui_draw_type", 0x0064_3af0, 1, &[(glue2::UI_SET_DRAW_TYPE, K::This(1))], |m, _, a| { g2::set_ui_draw_type(m, a[0]); Out::None }),
    Case { prep: Some(prep_abmap_state), ..cdecl("menu_reset", 0x0064_2320, 0, &[(glue2::UI_MENU_RESET, K::This(0))], |m, _, _| u(g2::menu_reset(m))) },
    Case { prep: Some(prep_abmap_state), ..cdecl("menu_update", 0x0064_23b0, 0, &[(glue2::UI_MENU_UPDATE, K::This(0))], |m, _, _| u(g2::menu_update(m))) },
    cdecl("update_map_yiq", 0x0064_4990, 0, &[(glue2::INPUT_INSTANCE, K::Getter), (glue2::INPUT_UPDATE, K::This(0)), (glue2::IGGY_GLOBAL, K::Getter), (glue2::IGGY_GLOBAL_UPDATE, K::This(0)), (glue2::IGGY_MENU_UPDATE, K::This(0)), (glue2::INPUT_CLEAR, K::Cdecl(0))], |m, _, _| u(g2::update_map_yiq(m))),
    // other managers
    cdecl("get_loc_text_with_length", 0x0064_12f0, 3, &[(glue2::LOCALIZATION_INSTANCE, K::Getter), (glue2::LOCALIZATION_GET_TEXT, K::This(3))], |m, _, a| u(g2::get_loc_text_with_length(m, a[0], a[1], a[2]))),
    cdecl("set_material_uv", 0x0064_36e0, 3, &[(glue2::MORPH_INSTANCE, K::Getter), (glue2::MORPH_SET_MATERIAL_UV, K::This(3))], |m, _, a| u(g2::set_material_uv(m, a[0], a[1], a[2]))),
    cdecl("texture_animation_set_paused", 0x0064_3a00, 2, &[(glue2::TEX_ANIM_INSTANCE, K::Getter), (glue2::TEX_ANIM_SET_PAUSED, K::This(2))], |m, _, a| u(g2::texture_animation_set_paused(m, a[0], a[1]))),
    cdecl("play_video", 0x0064_27c0, 1, &[(glue::FMV_INSTANCE, K::Getter), (glue2::FMV_PLAY_VIDEO, K::This(1))], |m, _, a| u(g2::play_video(m, a[0]))),
    cdecl("init_fmv_player", 0x0064_1690, 2, &[(glue::FMV_INSTANCE, K::Getter), (glue2::FMV_INITIALIZE, K::This(3))], |m, _, a| u(g2::init_fmv_player(m, a[0], a[1]))),
    cdecl("video_update", 0x0064_5df0, 0, &[(glue::FMV_INSTANCE, K::Getter), (glue::GET_APPLICATION, K::Getter), (glue2::FMV_HANDLE_INPUT, K::This(1)), (glue2::FMV_UPDATE_TEXTURE, K::This(0))], |m, _, _| u(g2::video_update(m))),
    cdecl("register_reset_screen_fade_delegation", 0x0064_5e60, 1, &[(glue2::REGISTER_DELEGATION, K::This(4))], |m, _, a| u(g2::register_reset_screen_fade_delegation(m, a[0]))),
    method("shadow_manager_initialize", 0x006c_e100, This::Object, 0, &[(glue2::SHADOW_LOAD_SHADERS, K::This(0)), (glue2::SHADOW_CREATE_BUFFER, K::This(2)), (glue2::SHADOW_SETUP, K::This(0))], |m, this, _| u(g2::shadow_manager_initialize(m, this))),
    cdecl("model_buffer_clear_all", 0x0072_ac30, 0, &[(glue2::MEMSET, K::Cdecl(3))], |m, _, _| u(g2::model_buffer_clear_all(m))),
    // the application
    cdecl("set_force_no_vsync", 0x0064_3200, 1, &[(glue::GET_APPLICATION, K::Getter)], |m, _, a| u(g2::set_force_no_vsync(m, a[0]))),
    method("application_mode_or_2", 0x0042_f750, This::Object, 0, &[], |m, this, _| u(g2::application_mode_or_2(m, this))),
];

#[test]
fn batch2_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 8 } else { 32 };
        run_cases(t, CASES2, n, &mut Rng(0x9e37_79b9));
    });
}

// ---- batch 3 (ffx_prender::glue3) ----

/// The low scene fields (`+1`, `+0x10cc`, `+0x1195`) on the object
/// itself, with `+0x10cc` pointing at a post-process object in it.
fn prep_scene_low(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let post = (obj + 0x1800).wrapping_sub(0x0001_23bb);
    t.w32(obj + 0x10cc, post);
    let v = if rng.chance(2) { 0 } else { rng.next() as u8 };
    t.write(obj + 0x1800, &[v]);
    let b = [0, 1, 4, 5, 0xff][rng.below(5) as usize];
    t.write(obj + 1, &[b]);
}

/// `obj +4` set or 0, and a recording callback or none.
fn prep_regist_helper(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    args[0] = obj + 0x1000;
    args[1] = obj;
    if rng.chance(3) {
        t.w32(obj + 4, 0);
    }
    args[3] = if rng.chance(3) {
        0
    } else {
        let r = rng.word();
        t.stub(None, 0xcb, false, 1, 0, Some(r))
    };
}

/// `key` = the object.
fn prep_key(_: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], _: &mut Rng) {
    args[0] = obj + 0x40;
}

/// The magic id: 0x2b9 now and then, and the two counters around 0.
fn prep_glow(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let base = obj.wrapping_sub(0xf000);
    *this = base;
    let id = if rng.chance(3) { 0x2b9 } else { rng.word() };
    let at = t.rebase(glue3::MAGIC_ID);
    t.stub(Some(at), glue3::MAGIC_ID, true, 0, 0, Some(id));
    for off in [0xfb60, 0xfb74] {
        let v = [0, 1, 0xffff_ffff, 0x8000_0000, 7][rng.below(5) as usize];
        t.w32(base + off, v);
    }
}

/// `this` = a fresh object large enough for `+0x12404` (the diff then
/// sees every store).
fn prep_12400(t: &mut Twins, _: u32, this: &mut u32, _: &mut [u32], _: &mut Rng) {
    *this = t.zeros(0x12408);
}

/// The pause-game global: set or 0.
fn prep_pause(t: &mut Twins, _: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let v = if rng.chance(2) { 0 } else { rng.next() | 1 };
    t.global(glue3::PAUSE_GAME.sym, v);
}

/// `operator new` gives the object, or 0 now and then.
fn prep_new(t: &mut Twins, obj: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let p = if rng.chance(4) { 0 } else { obj + 0x100 };
    let at = t.rebase(glue3::OPERATOR_NEW);
    t.stub(Some(at), glue3::OPERATOR_NEW, false, 1, 0, Some(p));
}

#[rustfmt::skip]
const CASES3: &[Case] = &[
    Case { float_args: true, ..cdecl("set_material_colour_scale2", 0x0064_3670, 4, &[(glue3::SCENE_SET_MATERIAL_COLOUR_SCALE2, K::This(4))], |m, _, a| { g3::set_material_colour_scale2(m, [a[0], a[1], a[2], a[3]]); Out::None }) },
    Case { float_args: true, ..cdecl("blur_set_positive_params", 0x0064_3830, 4, &[(glue2::BLUR_EX_INSTANCE, K::Getter), (glue3::BLUR_SET_POSITIVE_PARAMS, K::Keep(4))], |m, _, a| u(g3::blur_set_positive_params(m, [a[0], a[1], a[2], a[3]]))) },
    Case { float_args: true, ..cdecl("blur_set_floats74", 0x0064_3860, 4, &[(glue2::BLUR_EX_INSTANCE, K::Getter), (glue3::BLUR_SET_FLOATS74, K::Keep(4))], |m, _, a| u(g3::blur_set_floats74(m, [a[0], a[1], a[2], a[3]]))) },
    Case { float_args: true, ..cdecl("add_screen_fade_task", 0x0063_9240, 5, &[(glue3::SCREEN_FADE_REGISTER, K::Cdecl(6))], |m, _, a| u(g3::add_screen_fade_task(m, [a[0], a[1], a[2], a[3]], a[4], 0x1001))) },
    Case { float_args: true, ..cdecl("add_screen_fade_task_after_render", 0x0063_9280, 5, &[(glue3::SCREEN_FADE_REGISTER, K::Cdecl(6))], |m, _, a| u(g3::add_screen_fade_task(m, [a[0], a[1], a[2], a[3]], a[4], 0x801))) },
    Case { prep: Some(arg_object), ..cdecl("async_field_map_load_task", 0x0065_7820, 1, &[(glue::ASYNC_LOAD_INSTANCE, K::Getter), (glue3::SCENE_FIELD_MAP_LOAD, K::This(2)), (glue3::OPERATOR_DELETE_TASK, K::Cdecl(1))], |m, _, a| u(g3::async_field_map_load_task(m, a[0]))) },
    Case { prep: Some(prep_12400), ..method("set_fields_12400", 0x0069_3be0, This::Object, 4, &[(glue3::FIELDS_12400_UPDATE, K::This(0))], |m, this, a| u(g3::set_fields_12400(m, this, a[0], a[1], a[2]))) },
    Case { prep: Some(prep_key), ..method("cluster_new_entry", 0x0069_9300, This::Object, 1, &[(glue3::CLUSTER_ALLOC_ENTRY, K::Entry)], |m, this, a| u(g3::cluster_new_entry(m, this, a[0]))) },
    Case { prep: Some(prep_scene_low), ..method("scene_is_motion_blur_on", 0x0066_01e0, This::Object, 0, &[(glue2::MOTION_BLUR_INSTANCE, K::Getter)], |m, this, _| Out::U8(g3::scene_is_motion_blur_on(m, this))) },
    Case { prep: Some(prep_scene_low), ..method("scene_check_1195", 0x0066_62f0, This::Object, 0, &[(glue3::CHECK_01326B28, K::Cdecl(0))], |m, this, _| Out::U8(g3::scene_check_1195(m, this))) },
    Case { prep: Some(prep_glow), ..method("scene_is_glow_after_vfx", 0x0065_efa0, This::Object, 0, &[], |m, this, _| Out::U8(g3::scene_is_glow_after_vfx(m, this))) },
    Case { prep: Some(prep_pause), ..cdecl("create_pause_game", 0x006b_cc40, 0, &[(glue3::OPERATOR_NEW, K::Cdecl(1)), (glue3::PAUSE_GAME_CTOR, K::This(0))], |m, _, _| { g3::create_pause_game(m); Out::None }) },
    Case { prep: Some(prep_new), ..cdecl("tree_buy_head_node", 0x0065_4ed0, 0, &[(glue3::XBAD_ALLOC, K::Import(0))], |m, _, _| u(g3::tree_buy_head_node(m))) },
    method("scene_resize_target", 0x0069_8060, This::Object, 2, &[(glue3::TEXT_RESIZE, K::This(2))], |m, this, a| u(g3::scene_resize_target(m, this, a[0], a[1]))),
    Case { prep: Some(prep_regist_helper), ..cdecl("character_regist_helper", 0x0063_7880, 4, &[(glue3::CHARACTER_LOOKUP, K::Cdecl(2))], |m, _, a| u(g3::character_regist_helper(m, a[0], a[1], a[2], a[3]))) },
];

#[test]
fn batch3_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 8 } else { 32 };
        run_cases(t, CASES3, n, &mut Rng(0x0bad_cafe));
    });
}

// ---- batch 4 (ffx_prender::glue4, round 2) ----

/// One of `vs`.
fn pick(rng: &mut Rng, vs: &[u32]) -> u32 {
    vs[rng.below(vs.len() as u32) as usize]
}

/// A `PArray` of pointers at `obj + 0x40` (count `+0`, elements `+4` at
/// `obj + 0x100`), or none now and then; `args[0]` = it.
fn prep_parray(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let n = rng.below(6);
    let top = if rng.chance(3) { 0x8000_0000 } else { 0 };
    t.w32(obj + 0x40, n | top);
    t.w32(obj + 0x44, obj + 0x100);
    for i in 0..n {
        t.w32(obj + 0x100 + 4 * i, obj + 0x400 + 0x40 * i);
    }
    args[0] = if rng.chance(5) { 0 } else { obj + 0x40 };
}

/// `prep_parray`, and the scene's visibility test returning `al` 0 most
/// of the time.
fn prep_visible_list(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    prep_parray(t, obj, this, args, rng);
    let r = if rng.chance(3) {
        rng.next() | 1
    } else {
        rng.next() & !0xff
    };
    let at = t.rebase(glue4::SCENE_IS_MESH_VISIBLE);
    t.stub(Some(at), glue4::SCENE_IS_MESH_VISIBLE, true, 1, 1, Some(r));
}

/// The abmap's state: 4 (delete) half of the time (image only).
fn prep_abmap4(t: &mut Twins, _: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let abmap = t.r32(t.rebase(glue2::GRAPHIC_ABMAP.sym));
    if !t.snapshot && abmap != 0 {
        let v = pick(rng, &[4, 4, 0, 3, 5, 0xffff_ffff]);
        t.w32(abmap + 4, v);
    }
}

/// The loader task `(manager, i)` at `obj + 0x40`, `i` the last (13)
/// half of the time.
fn prep_shader_bin(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let i = if rng.chance(2) { 13 } else { rng.below(14) };
    t.w32(obj + 0x40, obj + 0x100);
    t.w32(obj + 0x44, i);
    args[1] = obj + 0x40;
}

/// The delete pool: count `+8`, keys `+0xc` (8 bytes, about half 0),
/// mesh instances `+0x1c`.
fn prep_delete_pool(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let n = rng.below(7);
    let top = if rng.chance(3) { 0x8000_0000 } else { 0 };
    t.w32(obj + 8, n | top);
    t.w32(obj + 0xc, obj + 0x100);
    t.w32(obj + 0x1c, obj + 0x200);
    for i in 0..n {
        let (lo, hi) = match rng.below(4) {
            0 => (0, 0),
            1 => (rng.next(), 0),
            2 => (0, rng.next()),
            _ => (rng.next(), rng.next()),
        };
        t.w32(obj + 0x100 + 8 * i, lo);
        t.w32(obj + 0x104 + 8 * i, hi);
        t.w32(obj + 0x200 + 4 * i, obj + 0x400 + 0x20 * i);
        t.w32(obj + 0x40c + 0x20 * i, 0x1111_1111);
    }
}

/// A scene large enough for `+0x10d8c`, an entry with or without a mesh.
fn prep_mesh_visible(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x10d90);
    *this = scene;
    let flag = if rng.chance(3) { rng.word() | 1 } else { 0 };
    t.w32(scene + 0x0001_0d8c, flag);
    let camera = rng.next();
    t.w32(scene + 0xdc, camera);
    let mesh = if rng.chance(3) { 0 } else { rng.next() | 1 };
    t.w32(obj + 0x5c, mesh);
    args[0] = if rng.chance(5) { 0 } else { obj + 0x40 };
}

/// The instance 0 now and then; the flag global random.
fn prep_shader_by_instance(t: &mut Twins, _: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    if rng.chance(4) {
        args[0] = 0;
        args[1] = 0;
    }
    let v = rng.word();
    t.global(glue4::USE_OPTIMIZED_VFX_SHADER_ARG.sym, v);
}

/// The scene's cluster fields: `+0xfb60` around 0, byte `+0xfb44`.
fn prep_move_cluster(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj.wrapping_sub(0xf000);
    let v = pick(rng, &[0, 0, 1, 0xffff_ffff, 0x8000_0000]);
    t.w32(obj + 0xb60, v);
    let b = if rng.chance(2) {
        0
    } else {
        rng.next() as u8 | 1
    };
    t.write(obj + 0xb44, &[b]);
}

/// The scene's list count `+0x10d84`: owned (top bit clear) or not.
fn prep_clear_list(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj.wrapping_sub(0x0001_0000);
    let top = if rng.chance(2) { 0x8000_0000 } else { 0 };
    let v = rng.next() & 0x7fff_ffff | top;
    t.w32(obj + 0xd84, v);
}

/// The data lookup returning 0 (the whole loop) half of the time; keys
/// near the 32-bit carry.
fn prep_atel_shp(t: &mut Twins, _: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    if rng.chance(2) {
        let at = t.rebase(glue4::VFX_GET_DATA_BY_INSTANCE);
        t.stub(
            Some(at),
            glue4::VFX_GET_DATA_BY_INSTANCE,
            true,
            2,
            2,
            Some(0),
        );
    }
    match rng.below(4) {
        0 => {
            args[0] = 0;
            args[1] = 0;
        }
        1 => args[0] = 0u32.wrapping_sub(rng.below(60_000)),
        _ => {}
    }
}

/// The magic id: one of the special ones half of the time.
fn prep_special_magic(t: &mut Twins, _: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let id = if rng.chance(2) {
        pick(
            rng,
            &[0x6a, 0x119, 0x203, 0x241, 0x20a, 0x29b, 0x1bd, 0x10e],
        )
    } else {
        pick(rng, &[0, 0x6b, 0x2b9, 0x1_006a, 0x10e | 0xff00])
    };
    let high = if rng.chance(2) {
        rng.next() & 0xffff_0000
    } else {
        0
    };
    let at = t.rebase(glue4::MAGIC_ID);
    t.stub(Some(at), glue4::MAGIC_ID, true, 0, 0, Some(id | high));
}

/// A material at `m` whose remapping (`+8` from, `+0xc` to) is one of the
/// passes or none.
fn material_at(t: &mut Twins, m: u32, rng: &mut Rng) {
    let passes = [
        glue4::PASS_SCENE,
        glue4::PASS_OPTIMIZED_TRANS,
        glue4::PASS_FORCE_TRANSPARENT,
        glue4::PASS_KEEP,
    ];
    let from = if rng.chance(3) {
        rng.next()
    } else {
        t.rebase(glue4::PASS_SCENE)
    };
    let to = if rng.chance(5) {
        rng.next()
    } else {
        let p = pick(rng, &passes);
        t.rebase(p)
    };
    t.w32(m + 8, from);
    t.w32(m + 0xc, to);
}

/// A material at `obj + 0x40`; the flag word with its low byte 0 half of
/// the time.
fn prep_material(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    material_at(t, obj + 0x40, rng);
    args[0] = obj + 0x40;
    args[1] = if rng.chance(2) {
        rng.next() & !0xff
    } else {
        rng.next() | 1
    };
}

/// The post-process manager at `+0x10cc`, mode 0, 1 or other.
fn prep_post_aa(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    t.w32(obj + 0x10cc, obj + 0x1800);
    args[0] = pick(rng, &[0, 1, 0, 1, 2, 0xffff_ffff]);
}

/// A geometry at `obj + 0x40` (or none) with a material array at
/// `obj + 0xc0`, `PArray_Data` giving `obj + 0x200`, materials at
/// `obj + 0x400`.
fn prep_vfx_geometry(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    t.w32(obj + 0x4c, obj + 0x80);
    t.w32(obj + 0x90, obj + 0xc0);
    let n = rng.below(5);
    let top = if rng.chance(3) { 0x8000_0000 } else { 0 };
    t.w32(obj + 0xc0, n | top);
    for i in 0..n {
        let m = obj + 0x400 + 0x20 * i;
        t.w32(obj + 0x200 + 4 * i, m);
        material_at(t, m, rng);
    }
    let at = t.rebase(glue4::PARRAY_DATA);
    t.stub(Some(at), glue4::PARRAY_DATA, true, 0, 0, Some(obj + 0x200));
    args[0] = if rng.chance(5) { 0 } else { obj + 0x40 };
    let any = rng.next();
    args[1] = pick(rng, &[0, 1, 0x100, any]);
}

/// Two word arrays (`+0` count, `+8` data): `this` at `obj + 0x40`,
/// `other` at `obj + 0x80` (or `this`).
fn prep_assign(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj + 0x40;
    let n = pick(rng, &[0, 1, 2, 5, 0xffff_ffff]);
    t.w32(obj + 0x80, n);
    t.w32(obj + 0x88, obj + 0x200);
    // a null destination only where its first slot is the only one
    let dst = if n == 1 && rng.chance(2) {
        0
    } else {
        obj + 0x300
    };
    t.w32(obj + 0x48, dst);
    args[0] = if rng.chance(6) { *this } else { obj + 0x80 };
}

/// The scene's prompt (`+0x10ddc`) set or 0, the Iggy manager 0 now and
/// then (image only).
fn prep_auto_save(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj.wrapping_sub(0x0001_0000);
    let old = if rng.chance(2) { 0 } else { rng.next() | 1 };
    t.w32(obj + 0xddc, old);
    if !t.snapshot && rng.chance(2) {
        t.global(glue2::IGGY_MENU_MANAGER.sym, 0);
    }
}

/// `isVFXTextureLoaded` giving 0..3, the cluster lookup 0 or not.
fn prep_bp_loaded(t: &mut Twins, _: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let r = pick(rng, &[0, 1, 1, 2, 3, 0x101]);
    let at = t.rebase(glue4::SCENE_IS_VFX_TEXTURE_LOADED);
    t.stub(
        Some(at),
        glue4::SCENE_IS_VFX_TEXTURE_LOADED,
        true,
        7,
        7,
        Some(r),
    );
    let c = if rng.chance(2) { 0 } else { rng.next() | 1 };
    let at = t.rebase(glue4::CLUSTER_BY_NAME);
    t.stub(Some(at), glue4::CLUSTER_BY_NAME, true, 1, 1, Some(c));
}

/// An array (count, capacity, data) at `a` of `size`-byte entries, its
/// data at `data` (or 0 when empty); now and then a negative count and
/// capacity, with the data placed so that the new slot falls on 0 (the
/// signed compare and the signed `n / 8`).
fn array_at(t: &mut Twins, a: u32, data: u32, size: u32, rng: &mut Rng) {
    if rng.chance(6) {
        // count -10: n = -9, slot = data + (n - 1) * size
        t.w32(a, 0xffff_fff6);
        let cap = pick(rng, &[0xffff_ff9c, 0xffff_ffff, 3]);
        t.w32(a + 4, cap);
        t.w32(a + 8, 10 * size);
        return;
    }
    let n = rng.below(8);
    let cap = match rng.below(3) {
        0 => n,
        1 => n + 1 + rng.below(3),
        _ => n.saturating_sub(1),
    };
    t.w32(a, n);
    t.w32(a + 4, cap);
    let d = if n == 0 && rng.chance(2) { 0 } else { data };
    t.w32(a + 8, d);
}

/// The scene's pipeline array at `+0xfb48` (count, capacity, data), an
/// entry or none.
fn prep_pipeline(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj.wrapping_sub(0xf000);
    array_at(t, obj + 0xb48, obj + 0x1000, 4, rng);
    args[0] = if rng.chance(5) { 0 } else { obj + 0x40 };
}

/// The draw-event array at `obj + 0x40`, the event at `obj + 0x80`.
fn prep_event_push(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj + 0x40;
    array_at(t, obj + 0x40, obj + 0x400, 16, rng);
    args[0] = obj + 0x80;
}

/// The scene's event fields (`+0xf8cc`, layers `+0xf910`/`+0xf954`,
/// counters `+0xfb60`) and an event of type -1..7.
fn prep_add_event(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj.wrapping_sub(0xf000);
    let kind = pick(rng, &[0, 1, 2, 3, 4, 5, 6, 7, 0xffff_ffff, 0x8000_0000]);
    t.w32(obj + 0x80, kind);
    let layer = pick(rng, &[0, 0, 1, 3, 8, 0xffff_ffff]);
    t.w32(obj + 0x954, layer);
    args[0] = obj + 0x80;
}

/// A tree header at `obj + 0x100` whose root `+0xc` is the header, 0, or a
/// chain of left links back to the header.
fn prep_tree(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj + 0x40;
    let tree = obj + 0x100;
    let root = match rng.below(4) {
        0 => tree,
        1 => 0,
        _ => {
            let n = 1 + rng.below(4);
            for k in 0..n {
                let node = obj + 0x200 + 0x20 * k;
                let left = if k + 1 == n { tree } else { node + 0x20 };
                t.w32(node, left);
            }
            obj + 0x200
        }
    };
    t.w32(tree + 0xc, root);
    args[0] = tree;
}

/// The fill's mode `+0` (0..3), count `+8` and floats `[+0x48]`.
fn prep_fill(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let mode = pick(rng, &[1, 2, 1, 2, 0, 3]);
    t.w32(obj, mode);
    // 0x4000_0000 * 4 wraps to 0: no stores
    let n = if rng.chance(6) {
        0x4000_0000
    } else {
        rng.below(7)
    };
    t.w32(obj + 8, n);
    t.w32(obj + 0x48, obj + 0x400);
}

/// An object at `obj` whose virtual slot 0 is a recording stub; `p` =
/// `obj + 4`.
fn prep_descriptor_destroy(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let r = rng.next();
    let f = t.stub(None, 0xd7, true, 1, 1, Some(r));
    t.w32(obj, obj + 0x800);
    t.w32(obj + 0x800, f);
    args[0] = obj + 4;
}

/// 64 slots (`+8`, 8 bytes) with `data` in one of them, or none; the
/// compare-exchange giving 1 or not.
fn prep_free_list(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let data = rng.next() | 1;
    for i in 0..64 {
        t.w32(obj + 8 + 8 * i, data ^ (i + 1));
    }
    if !rng.chance(4) {
        let i = rng.below(64);
        t.w32(obj + 8 + 8 * i, data);
    }
    args[0] = data;
    let r = if rng.chance(2) { 1 } else { rng.word() };
    let s = t.stub(
        None,
        glue4::INTERLOCKED_COMPARE_EXCHANGE,
        false,
        3,
        3,
        Some(r),
    );
    let at = t.rebase(glue4::INTERLOCKED_COMPARE_EXCHANGE);
    t.patch(at, &s.to_le_bytes());
}

/// `this` such that the scene's YIQ fields (`+0xfc88`..) land in the
/// object.
fn this_for_yiq(_: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], _: &mut Rng) {
    *this = obj.wrapping_sub(0xf000);
}

fn arr<const N: usize>(a: &[u32]) -> [u32; N] {
    a[..N].try_into().unwrap()
}

#[rustfmt::skip]
const CASES4: &[Case] = &[
    cdecl("graphic_reset_yiq", 0x0064_2cc0, 0, &[(glue4::SCENE_RESET_YIQ, K::This(0))], |m, _, _| u(g4::graphic_reset_yiq(m))),
    Case { prep: Some(this_for_yiq), ..method("scene_reset_yiq", 0x0066_bf40, This::Object, 0, &[(glue4::SCENE_UPDATE_MAP_YIQ, K::This(0))], |m, this, _| u(g4::scene_reset_yiq(m, this))) },
    Case { prep: Some(prep_parray), ..cdecl("update_skeleton_bounds", 0x0063_c500, 1, &[(glue4::MESH_UPDATE_BOUNDS_FROM_SKELETON, K::This(0))], |m, _, a| { g4::update_skeleton_bounds(m, a[0]); Out::None }) },
    Case { float_args: true, ..cdecl("set_abmap", 0x0064_2da0, 5, &[(glue4::SCENE_SET_ABMAP_CAMERA, K::This(5))], |m, _, a| u(g4::set_abmap(m, arr(a)))) },
    Case { prep: Some(prep_abmap4), ..cdecl("abmap_update", 0x0063_90f0, 0, &[(glue4::ABMAP_DELETE, K::Cdecl(0)), (glue4::ABMAP_UPDATE, K::This(0))], |m, _, _| { g4::abmap_update(m); Out::None }) },
    Case { prep: Some(prep_shader_bin), ..cdecl("post_on_shader_bin_loaded", 0x0069_4c20, 2, &[(glue::ASYNC_LOAD_INSTANCE, K::Getter), (glue4::POST_SHADER_BIN_STEP, K::This(1)), (glue4::POST_ON_ALL_SHADER_BINS, K::This(0))], |m, _, a| u(g4::post_on_shader_bin_loaded(m, a[0], a[1]))) },
    Case { null_instance: true, ..cdecl("broken_screen_update_polygon_data", 0x0063_9500, 11, &[(glue::BROKEN_SCREEN_INSTANCE, K::Getter), (glue4::BROKEN_POLYGON, K::This(2)), (glue4::BROKEN_POLYGON_UPDATE, K::This(9))], |m, _, a| u(g4::broken_screen_update_polygon_data(m, &arr(a)))) },
    Case { prep: Some(prep_delete_pool), ..method("vfx_remove_deleted_pool_mesh_instances", 0x0069_fc70, This::Object, 0, &[], |m, this, _| u(g4::vfx_remove_deleted_pool_mesh_instances(m, this))) },
    Case { prep: Some(prep_mesh_visible), ..method("scene_is_mesh_visible", 0x0066_03c0, This::Object, 1, &[(glue4::SCENE_MESH_VISIBLE_TEST, K::This(1))], |m, this, a| u(g4::scene_is_mesh_visible(m, this, a[0]))) },
    Case { prep: Some(prep_shader_by_instance), ..cdecl("use_optimized_vfx_shader_by_instance", 0x0064_4b10, 2, &[(glue4::VFX_GET_DYN_GEO_BY_INSTANCE, K::This(2)), (glue4::USE_OPTIMIZED_VFX_SHADER, K::Cdecl(2))], |m, _, a| u(g4::use_optimized_vfx_shader_by_instance(m, a[0], a[1]))) },
    Case { prep: Some(prep_move_cluster), ..method("scene_move_cluster", 0x0065_8f00, This::Object, 0, &[(glue4::WORLD_REMOVE_CLUSTER, K::This(1)), (glue4::WORLD_ADD_CLUSTER, K::This(1))], |m, this, _| { g4::scene_move_cluster(m, this); Out::None }) },
    Case { prep: Some(prep_clear_list), ..method("scene_clear_list", 0x0065_8fc0, This::Object, 0, &[(glue4::PHYRE_FREE, K::Cdecl(1))], |m, this, _| u(g4::scene_clear_list(m, this))) },
    Case { prep: Some(prep_atel_shp), ..cdecl("vfx_is_atel_shp_instanced", 0x0064_5d40, 2, &[(glue4::VFX_GET_DATA_BY_INSTANCE, K::This(2))], |m, _, a| u(g4::vfx_is_atel_shp_instanced(m, a[0], a[1]))) },
    Case { prep: Some(prep_special_magic), ..method("scene_is_special_magic", 0x0065_f1f0, This::Object, 0, &[], |m, this, _| u(g4::scene_is_special_magic(m, this))) },
    Case { prep: Some(prep_material), ..cdecl("use_vfx_optimized_trans", 0x0065_4b20, 2, &[(glue4::MATERIAL_SET_PASS_REMAPPING, K::This(2))], |m, _, a| u(g4::use_vfx_optimized_trans(m, a[0], a[1]))) },
    Case { prep: Some(prep_material), ..cdecl("force_transparent", 0x0065_1400, 2, &[(glue4::MATERIAL_SET_PASS_REMAPPING, K::This(2))], |m, _, a| u(g4::force_transparent(m, a[0], a[1]))) },
    Case { float_args: true, ..cdecl("update_distortion_vertex", 0x0064_4940, 8, &[(glue4::DISTORTION_INSTANCE, K::Getter), (glue4::DISTORTION_UPDATE_VERTEX, K::This(8))], |m, _, a| u(g4::update_distortion_vertex(m, a[0], a[1], arr(&a[2..])))) },
    Case { prep: Some(prep_post_aa), ..method("scene_render_post_aa", 0x0065_35b0, This::Object, 1, &[(glue4::POST_RENDER_FXAA, K::This(2)), (glue4::POST_RENDER_SMAA, K::This(1))], |m, this, a| u(g4::scene_render_post_aa(m, this, a[0]))) },
    Case { prep: Some(prep_vfx_geometry), ..cdecl("use_optimized_vfx_shader", 0x0064_4ac0, 2, &[(glue4::MATERIAL_SET_PASS_REMAPPING, K::This(2))], |m, _, a| u(g4::use_optimized_vfx_shader(m, a[0], a[1]))) },
    Case { prep: Some(prep_assign), ..method("parray_assign_0064ead0", 0x0064_ead0, This::Object, 1, &[(glue4::ARRAY_RESERVE_667F00, K::KeepDeref(1))], |m, this, a| u(g4::parray_assign(m, glue4::ARRAY_RESERVE_667F00, this, a[0]))) },
    Case { prep: Some(prep_assign), ..method("parray_assign_0064eb20", 0x0064_eb20, This::Object, 1, &[(glue4::ARRAY_RESERVE_668080, K::KeepDeref(1))], |m, this, a| u(g4::parray_assign(m, glue4::ARRAY_RESERVE_668080, this, a[0]))) },
    Case { prep: Some(prep_auto_save), ..method("scene_show_auto_saving_prompt", 0x0066_f480, This::Object, 0, &[(glue4::IGGY_MENU_DTOR, K::This(0)), (glue4::OPERATOR_DELETE, K::Cdecl(1)), (glue4::IGGY_PLAYER_OPEN_SCALED, K::Cdecl(5))], |m, this, _| { g4::scene_show_auto_saving_prompt(m, this); Out::None }) },
    Case { prep: Some(prep_bp_loaded), ..cdecl("vfx_is_texture_with_bp_loaded", 0x0064_5450, 7, &[(glue4::CLUSTER_INSTANCE, K::Getter)], |m, _, a| u(g4::vfx_is_texture_with_bp_loaded(m, &arr(a)))) },
    Case { prep: Some(prep_visible_list), ..cdecl("character_instance_is_visible", 0x0063_b230, 1, &[], |m, _, a| u(g4::character_instance_is_visible(m, a[0]))) },
    Case { prep: Some(prep_pipeline), ..method("scene_add_to_pipeline", 0x0065_6590, This::Object, 1, &[(glue4::ARRAY_RESERVE_667F80, K::Keep(1))], |m, this, a| { g4::scene_add_to_pipeline(m, this, a[0]); Out::None }) },
    Case { prep: Some(prep_event_push), ..method("vfx_draw_event_push", 0x0065_6350, This::Object, 1, &[(glue4::ARRAY_RESERVE_668280, K::Keep(1))], |m, this, a| u(g4::vfx_draw_event_push(m, this, a[0]))) },
    Case { prep: Some(prep_add_event), ..method("scene_add_vfx_draw_event", 0x0065_6940, This::Object, 1, &[(glue4::VFX_DRAW_EVENT_PUSH, K::This(1))], |m, this, a| u(g4::scene_add_vfx_draw_event(m, this, a[0]))) },
    Case { prep: Some(prep_tree), ..method("tree_iterator_begin", 0x0064_dae0, This::Object, 1, &[(glue4::TREE_SUCCESSOR, K::This(1))], |m, this, a| u(g4::tree_iterator_begin(m, this, a[0]))) },
    Case { prep: Some(prep_fill), ..method("broken_fill_floats", 0x006d_4150, This::Object, 0, &[], |m, this, _| u(g4::broken_fill_floats(m, this))) },
    Case { prep: Some(prep_descriptor_destroy), ..method("shadow_dynamic_mesh_destroy", 0x0067_89c0, This::Stdcall, 1, &[], |m, _, a| u(g4::shadow_dynamic_mesh_destroy(m, a[0]))) },
    Case { prep: Some(prep_free_list), ..method("set_data_to_free_list", 0x006a_e1a0, This::Object, 1, &[(glue4::IOB_FUNC, K::ImportC(0)), (glue4::FPRINTF, K::ImportC(3))], |m, this, a| u(g4::set_data_to_free_list(m, this, a[0]))) },
];

#[test]
fn batch4_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 8 } else { 40 };
        run_cases(t, CASES4, n, &mut Rng(0x4b47_1e04));
    });
}

// ---- batch 5 (ffx_prender::glue5, round 2) ----

thread_local! {
    /// The original's `ebp` for a case with a frame ([`Case::frame`]),
    /// which the port's frame takes.
    static FRAME_EBP: Cell<u32> = const { Cell::new(0) };
}

fn frame_ebp() -> u32 {
    FRAME_EBP.with(Cell::get)
}

impl Twins {
    /// Places `code` in the cave; its address.
    fn code(&mut self, code: &[u8]) -> u32 {
        let addr = self.cave_next;
        assert!(
            addr + code.len() as u32 <= self.cave + CAVE_LEN,
            "cave full"
        );
        self.write(addr, code);
        self.cave_next = addr + code.len() as u32;
        addr
    }

    /// A recording stub at `sym` (as [`Twins::stub`]) that first stores
    /// `value` at `[arg0 + off]` (an output pointer).
    fn stub_out(&mut self, sym: u32, n: u32, pops: u32, ret: u32, off: u8, value: u32) {
        let log = self.stub(None, sym, true, n, pops, Some(ret));
        let mut c = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp+4]
        c.extend([0xc7, 0x40, off]); // mov dword [eax+off], value
        c.extend(value.to_le_bytes());
        let at = self.cave_next;
        c.push(0xe9); // jmp log
        c.extend(log.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
        let code = self.code(&c);
        let site = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(site + 5).to_le_bytes());
        self.patch(site, &j);
    }
}

/// A scene cursor at `obj + 0x40` (end `+0x14`, node `+0x18`, target
/// `+0x1c`) over a chain of nodes at `obj + 0x200`, each valid (its
/// `+0x2c` the target and `+0x40` set) or not.
fn prep_cursor(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let cursor = obj + 0x40;
    *this = cursor;
    let end = obj + 0x1f00;
    let target = obj + 0x1800;
    t.w32(cursor + 0x14, end);
    t.w32(cursor + 0x1c, target);
    let n = rng.below(5);
    for k in 0..n {
        let node = obj + 0x200 + 0x80 * k;
        let next = if k + 1 == n { end } else { node + 0x80 };
        t.w32(node, next);
        let owner = match rng.below(3) {
            0 => 0,
            1 => target,
            _ => obj + 0x1900,
        };
        t.w32(node + 0x2c, owner);
        let set = u32::from(!rng.chance(3));
        t.w32(node + 0x40, set);
    }
    t.w32(obj + 0x1940, 0);
    let first = if n == 0 || rng.chance(5) {
        0
    } else {
        obj + 0x200
    };
    t.w32(cursor + 0x18, first);
}

/// The 20-byte array at `obj + 0x40`, the entry at `obj + 0x80`.
fn prep_push20(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj + 0x40;
    array_at(t, obj + 0x40, obj + 0x400, 20, rng);
    args[0] = obj + 0x80;
}

/// The scene's object pool (`+0xfbb8` base, `+0xfbc4` used bytes,
/// `+0xfbc8` lowest free), an object with a recording destructor, and
/// the pool cluster or another.
fn prep_delete_object(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = obj.wrapping_sub(0xf000);
    *this = scene;
    let object = obj + 0x300;
    let i = pick(rng, &[0, 5, 0xbb7, 0xbb8, 0xffff_ffff, 100]);
    let low = if rng.chance(3) { rng.below(16) } else { 0 };
    t.w32(
        obj + 0xbb8,
        object.wrapping_sub(16u32.wrapping_mul(i)).wrapping_sub(low),
    );
    t.w32(obj + 0xbc4, obj + 0x1000);
    let free = pick(rng, &[0, 3, 6, 0xbb8, 0xffff_ffff]);
    t.w32(obj + 0xbc8, free);
    let r = rng.next();
    let f = t.stub(None, 0xd7, true, 1, 1, Some(r));
    t.w32(object, obj + 0x800);
    t.w32(obj + 0x800, f);
    args[0] = match rng.below(6) {
        0 => 0,
        1 => 0xffff_fffc,
        _ => object,
    };
    args[1] = if rng.chance(3) {
        rng.next()
    } else {
        scene + 0x12d4
    };
}

/// A word vector at `obj + 0x40` (begin, end, capacity end) and a value
/// inside it, at its end, or outside.
#[allow(clippy::many_single_char_names)] // the vector, count and code
fn prep_vector(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let v = obj + 0x40;
    *this = v;
    if rng.chance(6) {
        for k in 0..3 {
            t.w32(v + 4 * k, 0);
        }
        args[0] = obj + 0x80;
        return;
    }
    let begin = obj + 0x400;
    let n = rng.below(6);
    let end = begin + 4 * n;
    let cap = if rng.chance(2) { end } else { end + 8 };
    t.w32(v, begin);
    t.w32(v + 4, end);
    t.w32(v + 8, cap);
    if cap == end {
        // the growth moves the elements to obj + 0x600 (left as they are
        // there: the old and new copies differ)
        let log = t.stub(None, glue5::VECTOR_GROW, true, 1, 1, None);
        let moved = obj + 0x600;
        let mut c = vec![0xc7, 0x01]; // mov dword [ecx], moved
        c.extend(moved.to_le_bytes());
        c.extend([0xc7, 0x41, 0x04]); // mov dword [ecx+4], moved end
        c.extend((moved + 4 * n).to_le_bytes());
        c.extend([0xc7, 0x41, 0x08]); // mov dword [ecx+8], capacity end
        c.extend((moved + 4 * n + 8).to_le_bytes());
        let at = t.cave_next;
        c.push(0xe9);
        c.extend(log.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
        let code = t.code(&c);
        let site = t.rebase(glue5::VECTOR_GROW);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(site + 5).to_le_bytes());
        t.patch(site, &j);
    }
    args[0] = match rng.below(4) {
        0 => obj + 0x80,
        1 => end,
        _ => begin + 4 * rng.below(n.max(1)),
    };
}

/// The map number (0x17, 0x9f or others), the game-over flag, and the
/// pending delayed release 0 or not.
fn prep_delay_release(t: &mut Twins, _: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let map = pick(rng, &[0x17, 0x9f, 5, 0, 0x17, 0x9f]);
    t.global(glue5::CURRENT_MAP_NO.sym, map);
    let over = u32::from(rng.chance(3));
    t.global(glue5::GAME_OVER_NOTIFY.sym, over);
    let pending = if rng.chance(2) { 0 } else { rng.next() | 1 };
    let at = t.rebase(glue5::CHR_IS_DELAYED_RELEASE);
    t.stub(
        Some(at),
        glue5::CHR_IS_DELAYED_RELEASE,
        true,
        0,
        0,
        Some(pending),
    );
}

/// The dynamic geometry at `obj + 0x40` (or none) whose virtual slots 1
/// and 2 are recording stubs; the instance 0 now and then.
fn prep_vfx_2d(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let geo = obj + 0x40;
    let r1 = if rng.chance(2) { 0 } else { rng.next() | 1 };
    let s1 = t.stub(None, 0xa1, true, 2, 2, Some(r1));
    let r2 = rng.next();
    let s2 = t.stub(None, 0xa2, true, 1, 1, Some(r2));
    t.w32(geo, obj + 0x900);
    t.w32(obj + 0x904, s1);
    t.w32(obj + 0x908, s2);
    let g = if rng.chance(4) { 0 } else { geo };
    let at = t.rebase(glue5::VFX_GET_DYN_GEO_BY_INSTANCE);
    t.stub(
        Some(at),
        glue5::VFX_GET_DYN_GEO_BY_INSTANCE,
        true,
        2,
        2,
        Some(g),
    );
    if rng.chance(4) {
        args[0] = 0;
        args[1] = 0;
    }
}

/// The dream effect on or off, its phase around 5.
fn prep_dream(t: &mut Twins, _: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let on = u32::from(!rng.chance(2));
    t.global(glue5::DREAM_ON.sym, on);
    let phase = pick(rng, &[0, 4, 5, 6, 0xffff_ffff, 0x8000_0000]);
    t.global(glue5::DREAM_PHASE.sym, phase);
}

/// A cluster or none; the task `(manager, id)` at `obj + 0x40`.
fn prep_shader_loaded(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    if rng.chance(4) {
        args[0] = 0;
    }
    let manager = rng.next();
    t.w32(obj + 0x40, manager);
    let id = rng.next();
    t.w32(obj + 0x44, id);
    args[1] = obj + 0x40;
}

/// A material at `obj + 0x40` (or none), its parameter found or not.
fn prep_visibility(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let m = obj + 0x40;
    let params = rng.next();
    t.w32(m, params);
    let buffer = rng.next();
    t.w32(m + 4, buffer);
    args[0] = if rng.chance(5) { 0 } else { m };
    let p = if rng.chance(4) { 0 } else { rng.next() | 1 };
    let at = t.rebase(glue5::SHADER_PARAMS_FIND);
    t.stub(Some(at), glue5::SHADER_PARAMS_FIND, true, 1, 1, Some(p));
    let any = rng.next();
    args[1] = pick(rng, &[0, 1, 0xffff_ffff, 0x0100_0001, 0x8000_0000, any]);
}

/// `PeekMessageW` writing a message (`WM_QUIT` now and then) and giving
/// one or none.
fn prep_peek(t: &mut Twins, _: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let got = u32::from(!rng.chance(3));
    let message = pick(rng, &[0x12, 0x12, 0x0f, 0x112]);
    let log = t.stub(None, glue5::PEEK_MESSAGE, false, 5, 5, Some(got));
    let mut c = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp+4]
    c.extend([0xc7, 0x40, 0x04]); // mov dword [eax+4], message
    c.extend(message.to_le_bytes());
    let at = t.cave_next;
    c.push(0xe9);
    c.extend(log.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
    let code = t.code(&c);
    let slot = t.rebase(glue5::PEEK_MESSAGE);
    t.patch(slot, &code.to_le_bytes());
}

/// The constant entry (`+0xa0`/`+0xa4`) or none, the buffer
/// (`[[+0x1c] + 4]`) of a small size, offsets and lengths around it.
fn prep_constant(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj;
    for field in [0xa0, 0xa4] {
        let e = if rng.chance(5) { 0 } else { obj + 0x100 };
        t.w32(obj + field, e);
    }
    t.w32(obj + 0x1c, obj + 0x180);
    t.w32(obj + 0x184, obj + 0x200);
    let size = pick(rng, &[0, 8, 16, 20, 32, 64]);
    t.w32(obj + 0x200, size);
    let off = pick(rng, &[0, 3, 4, 8, 12, 16, 48]);
    let high = if rng.chance(2) { 0xe000 } else { 0 };
    let len = pick(rng, &[0, 4, 8, 16, 12]) | high;
    t.write(obj + 0x108, &(off as u16).to_le_bytes());
    t.write(obj + 0x10a, &(len as u16).to_le_bytes());
}

/// The map lookup storing the end (`[this]`, not found) or a node at
/// `obj + 0x300` into the iterator; the cached key matching or not.
fn prep_vfx_find(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let end = obj + 0x1f00;
    t.w32(obj, end);
    let node = if rng.chance(2) { end } else { obj + 0x300 };
    let r = rng.next();
    t.stub_out(glue5::VFX_MAP_FIND, 2, 2, r, 0, node);
    if rng.chance(2) {
        t.w32(obj + 0x20, args[0]);
        let hi = if rng.chance(3) { args[1] ^ 1 } else { args[1] };
        t.w32(obj + 0x24, hi);
    }
    let at = t.rebase(glue5::VFX_MAP_NEW_NODE);
    t.stub(
        Some(at),
        glue5::VFX_MAP_NEW_NODE,
        true,
        1,
        1,
        Some(obj + 0x500),
    );
}

/// A mesh instance at `obj + 0x40` with a world matrix (`+0xc`) or none,
/// the matrix argument at `obj + 0x400`.
fn prep_mesh_matrix(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj + 0x40;
    let world = if rng.chance(2) { 0 } else { rng.next() | 1 };
    t.w32(obj + 0x4c, world);
    args[0] = obj + 0x400;
}

const fn framed(case: Case, low: u32, high: u32) -> Case {
    Case {
        frame: Some((low, high)),
        ..case
    }
}

#[rustfmt::skip]
const CASES5: &[Case] = &[
    Case { prep: Some(prep_cursor), ..method("cursor_next_6377a0", 0x0063_77a0, This::Object, 0, &[(glue5::CURSOR_SET_NODE, K::This(1))], |m, this, _| u(g5::cursor_next(m, this, glue5::CURSOR_IS_VALID_637F80))) },
    Case { prep: Some(prep_cursor), ..method("cursor_next_6b22e0", 0x006b_22e0, This::Object, 0, &[(glue5::CURSOR_SET_NODE, K::This(1))], |m, this, _| u(g5::cursor_next(m, this, glue5::CURSOR_IS_VALID_6B28F0))) },
    Case { prep: Some(prep_cursor), ..method("cursor_begin_637730", 0x0063_7730, This::Object, 0, &[(glue5::CURSOR_SET_NODE, K::This(1))], |m, this, _| u(g5::cursor_begin(m, this, glue5::CURSOR_IS_VALID_637F80))) },
    Case { prep: Some(prep_cursor), ..method("cursor_begin_65eb60", 0x0065_eb60, This::Object, 0, &[(glue5::CURSOR_SET_NODE, K::This(1))], |m, this, _| u(g5::cursor_begin(m, this, glue5::CURSOR_IS_VALID_660270))) },
    Case { prep: Some(prep_cursor), ..method("cursor_begin_6b2230", 0x006b_2230, This::Object, 0, &[(glue5::CURSOR_SET_NODE, K::This(1))], |m, this, _| u(g5::cursor_begin(m, this, glue5::CURSOR_IS_VALID_6B28F0))) },
    Case { prep: Some(prep_push20), ..method("parray_push_back20", 0x0065_62f0, This::Object, 1, &[(glue5::ARRAY_RESERVE_668200, K::Keep(1))], |m, this, a| u(g5::parray_push_back20(m, this, a[0]))) },
    Case { prep: Some(prep_delete_object), ..method("scene_delete_object_in_cluster", 0x0065_a250, This::Object, 2, &[(glue5::CLUSTER_DELETE_OBJECT, K::This(1))], |m, this, a| u(g5::scene_delete_object_in_cluster(m, this, a[0], a[1]))) },
    Case { prep: Some(prep_vector), ..method("vector_push_back", 0x0066_7620, This::Object, 1, &[(glue5::VECTOR_GROW, K::Keep(1))], |m, this, a| { g5::vector_push_back(m, this, a[0]); Out::None }) },
    Case { prep: Some(prep_delay_release), ..cdecl("enable_chr_cluster_delay_release", 0x0064_0090, 0, &[(glue5::SCENE_LEVEL_NAME_HASH, K::This(0)), (glue5::CLUSTER_INSTANCE, K::Getter), (glue5::CHR_ENABLE_DELAY_RELEASE, K::This(0)), (glue5::CHR_APPLY_DELAYED_RELEASE, K::This(0)), (glue5::IOB_FUNC, K::ImportC(0)), (glue5::FPRINTF, K::ImportC(2))], |m, _, _| u(g5::enable_chr_cluster_delay_release(m))) },
    Case { prep: Some(prep_vfx_2d), ..cdecl("vfx_2d_setup", 0x0064_4b50, 2, &[(glue5::PHYRE_PRINTF, K::Cdecl(2))], |m, _, a| u(g5::vfx_2d_setup(m, a[0], a[1]))) },
    framed(Case { prep: Some(prep_dream), ..cdecl("scene_on_menu_texture_loaded", 0x0066_6740, 1, &[(glue5::GET_APPLICATION, K::Cdecl(0)), (glue5::FIXUP_CLUSTERS, K::Cdecl(2))], |m, _, a| u(g5::scene_on_menu_texture_loaded(m, frame_ebp(), a[0]))) }, 0, 4),
    framed(Case { prep: Some(prep_shader_loaded), ..cdecl("vfx_shader_on_loaded", 0x006c_13e0, 2, &[(glue5::ASYNC_LOAD_INSTANCE, K::Getter), (glue5::VFX_SHADER_REGISTER, K::This(2)), (glue5::GET_APPLICATION, K::Cdecl(0)), (glue5::FIXUP_CLUSTERS, K::Cdecl(2))], |m, _, a| u(g5::vfx_shader_on_loaded(m, frame_ebp(), a[0], a[1]))) }, 0, 8),
    framed(Case { prep: Some(prep_visibility), ..method("scene_set_material_visibility", 0x0066_d5c0, This::Stdcall, 2, &[(glue5::SHADER_PARAM_SET_VALUE, K::This(2))], |m, _, a| { g5::scene_set_material_visibility(m, frame_ebp(), a[0], a[1]); Out::None }) }, 0x10, 0),
    framed(Case { prep: Some(prep_peek), ..cdecl("process_windows_message", 0x0064_2860, 0, &[(glue5::TRANSLATE_MESSAGE, K::Import(1)), (glue5::DISPATCH_MESSAGE, K::Import(1))], |m, _, _| u(g5::process_windows_message(m, frame_ebp()))) }, 0x1c, 0),
    framed(Case { prep: Some(prep_constant), ..method("broken_set_constant_a0", 0x006d_46a0, This::Object, 4, &[(glue5::MEMCPY, K::Cdecl(3))], |m, this, a| u(g5::broken_set_constant(m, frame_ebp(), this, arr(a), 0xa0))) }, 0, 16),
    framed(Case { prep: Some(prep_constant), ..method("broken_set_constant_a4", 0x006d_4710, This::Object, 4, &[(glue5::MEMCPY, K::Cdecl(3))], |m, this, a| u(g5::broken_set_constant(m, frame_ebp(), this, arr(a), 0xa4))) }, 0, 16),
    framed(Case { prep: Some(prep_vfx_find), ..method("vfx_table_remove", 0x0069_fc10, This::Object, 2, &[(glue5::VFX_INSERT_TO_DELETE_POOL, K::This(4)), (glue5::VFX_MAP_ERASE, K::This(2))], |m, this, a| u(g5::vfx_table_remove(m, frame_ebp(), this, a[0], a[1]))) }, 4, 8),
    framed(Case { prep: Some(prep_vfx_find), ..method("vfx_table_insert", 0x0069_f890, This::Object, 3, &[(glue5::VFX_MAP_INSERT, K::This(4))], |m, this, a| u(g5::vfx_table_insert(m, frame_ebp(), this, a[0], a[1], a[2]))) }, 0x10, 12),
    framed(Case { prep: Some(prep_mesh_matrix), ..method("mesh_instance_is_visible", 0x0066_3cc0, This::Object, 1, &[(glue5::MATRIX4_FROM_3X4, K::This(1)), (glue5::MATRIX4_MULTIPLY, K::Cdecl(3)), (glue5::BOUNDS_IS_VISIBLE, K::This(1))], |m, this, a| u(g5::mesh_instance_is_visible(m, frame_ebp(), this, a[0]))) }, 0x80, 0),
];

#[test]
fn batch5_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 8 } else { 40 };
        run_cases(t, CASES5, n, &mut Rng(0x5eed_b005));
    });
}

// ---- batch 6 (ffx_prender::glue6, round 2) ----

impl Twins {
    /// A recording stub at `sym` (logging `ecx`) that first stores the
    /// words `writes` at `[ecx + off]` (an object the callee fills).
    fn stub_this_out(
        &mut self,
        sym: u32,
        n: u32,
        pops: u32,
        ret: Option<u32>,
        writes: &[(u8, u32)],
    ) {
        let log = self.stub(None, sym, true, n, pops, ret);
        let mut c = vec![];
        for &(off, v) in writes {
            c.extend([0xc7, 0x41, off]); // mov dword [ecx+off], v
            c.extend(v.to_le_bytes());
        }
        let at = self.cave_next;
        c.push(0xe9);
        c.extend(log.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
        let code = self.code(&c);
        let site = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(site + 5).to_le_bytes());
        self.patch(site, &j);
    }

    /// A recording stub at `sym` returning `ret`.
    fn stub_ret(&mut self, sym: u32, this: bool, n: u32, pops: u32, ret: u32) {
        let at = self.rebase(sym);
        self.stub(Some(at), sym, this, n, pops, Some(ret));
    }
}

const SINGLETONS: [glue6::Singleton; 7] = [
    glue6::VFX_RENDER_DATA_TABLE_SINGLETON,
    glue6::BROKEN_SCREEN_SINGLETON,
    glue6::FREE_CAMERA_SINGLETON,
    glue6::CLUSTER_MANAGER_SINGLETON,
    glue6::VFX_SHADER_MANAGER_SINGLETON,
    glue6::DISTORTION_SINGLETON,
    glue6::SHADOW_MANAGER_SINGLETON,
];

/// Every singleton global 0 or set; `operator new` giving memory or 0.
fn prep_singletons(t: &mut Twins, obj: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    for s in SINGLETONS {
        let v = if rng.chance(2) { 0 } else { rng.next() | 1 };
        t.global(s.global.sym, v);
    }
    let v = if rng.chance(2) { 0 } else { rng.next() | 1 };
    t.global(glue6::VFX_RENDER_MAP.sym, v);
    let p = if rng.chance(4) { 0 } else { obj + 0x100 };
    t.stub_ret(glue6::OPERATOR_NEW, false, 1, 0, p);
}

/// The task `(scene, a, b, list, reader, c, flag)` at `obj + 0x40`, the
/// list's count `[list]` equal to the reader's or not.
fn prep_asset_loaded(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let task = obj + 0x40;
    for k in [0, 1, 2, 4, 5] {
        let w = rng.next();
        t.w32(task + 4 * k, w);
    }
    t.w32(task + 0xc, obj + 0x100);
    let flag = if rng.chance(2) { 0 } else { rng.next() };
    t.w32(task + 0x18, flag);
    let count = rng.below(5);
    t.w32(obj + 0x100, count);
    let r = if rng.chance(2) { count } else { count + 1 };
    t.stub_ret(glue6::ASSET_LIST_COUNT, true, 0, 0, r);
    args[1] = task;
}

/// A scene at `obj + 0x1800` with its arrays `+0x60`, `+0x3c`, `+0x54`;
/// the task `(id, a, b, c)` at `obj + 0x40`.
fn prep_vfx_texture_loaded(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = obj + 0x1800;
    t.global(SCENE, scene);
    t.w32(scene + 0x60, obj + 0x1000);
    t.w32(scene + 0x3c, obj + 0x1100);
    t.w32(scene + 0x54, obj + 0x1200);
    t.w32(obj + 0x40, rng.below(16));
    for k in 1..4 {
        let w = rng.next();
        t.w32(obj + 0x40 + 4 * k, w);
    }
    args[1] = obj + 0x40;
}

/// The PS2 loader's list (count 0..4, data at `obj + 0x100`).
fn prep_ps2_list(t: &mut Twins, obj: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let m = rng.next();
    t.global(glue6::PS2_LOAD_MUTEX.sym, m);
    let list = t.rebase(glue6::PS2_LOAD_LIST);
    t.w32(list, rng.below(5));
    let cap = rng.next();
    t.w32(list + 4, cap);
    t.w32(list + 8, obj + 0x100);
}

/// A scene with the renderer (`+0xd0`) and the mesh list (`+0x3854`,
/// `+0x385c`); `beginScene` failing now and then.
fn prep_dtt(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x3860);
    *this = scene;
    t.w32(scene + 0xd0, obj + 0x100);
    let n = rng.below(5);
    t.w32(scene + 0x3854, n);
    t.w32(scene + 0x385c, obj + 0x300);
    for k in 0..n {
        let mesh = if rng.chance(4) {
            0
        } else {
            obj + 0x400 + 0x40 * k
        };
        t.w32(obj + 0x300 + 4 * k, mesh);
    }
    let r = if rng.chance(4) { rng.next() | 1 } else { 0 };
    t.stub_ret(glue6::RENDERER_BEGIN_SCENE, true, 1, 1, r);
}

/// A pool of `size`-byte slots in the scene (`base` field, used bytes,
/// lowest free) and an object with a recording destructor in it, or not.
fn pool_at(
    t: &mut Twins,
    obj: u32,
    this: &mut u32,
    args: &mut [u32],
    rng: &mut Rng,
    fields: [u32; 3],
    size: u32,
) {
    let scene = obj.wrapping_sub(0xf000);
    *this = scene;
    let object = obj + 0x300;
    let i = pick(rng, &[0, 5, 0xbb7, 0xbb8, 0xffff_ffff, 100]);
    let low = if rng.chance(3) { rng.below(size) } else { 0 };
    let base = if rng.chance(5) {
        // just above the object: the division rounds toward 0 (index 0)
        object + 1 + rng.below(size - 1)
    } else {
        object.wrapping_sub(size.wrapping_mul(i)).wrapping_sub(low)
    };
    t.w32(fields[0] - 0xf000 + obj, base);
    t.w32(fields[1] - 0xf000 + obj, obj + 0x1000);
    let free = pick(rng, &[0, 3, 6, 0xbb8, 0xffff_ffff]);
    t.w32(fields[2] - 0xf000 + obj, free);
    let r = rng.next();
    let f = t.stub(None, 0xd7, true, 1, 1, Some(r));
    t.w32(object, obj + 0x800);
    t.w32(obj + 0x800, f);
    args[0] = match rng.below(6) {
        0 => 0,
        1 => 0xffff_fffc,
        _ => object,
    };
    args[1] = if rng.chance(3) {
        rng.next()
    } else {
        scene + 0x12d4
    };
}

fn prep_pool56(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    pool_at(t, obj, this, args, rng, [0xfb90, 0xfb9c, 0xfba0], 56);
}

fn prep_pool28(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    pool_at(t, obj, this, args, rng, [0xfba4, 0xfbb0, 0xfbb4], 28);
}

/// The lookup's entries (`+4 + 4i`), each list empty or not, and the
/// cursor search finding an object or not.
fn prep_post_lookup(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let i = rng.below(4);
    args[0] = i;
    let entry = if rng.chance(5) { 0 } else { obj + 0x100 };
    t.w32(obj + 4 + 4 * i, entry);
    let list = obj + 0x11c;
    let first = if rng.chance(2) { list } else { obj + 0x400 };
    t.w32(list, first);
    let found = if rng.chance(3) { 0 } else { rng.next() | 1 };
    let object = rng.next();
    let base = rng.next();
    // a miss leaves the object fields as the port wrote them
    let writes: &[(u8, u32)] = if found == 0 && rng.chance(2) {
        &[(0xc, 0)]
    } else if found == 0 {
        &[(0xc, 0), (0x10, object | 1)]
    } else {
        &[(0xc, found), (0x10, object), (0, base)]
    };
    t.stub_this_out(glue6::CURSOR_SEEK3, 0, 0, None, writes);
}

/// A scene large enough for the mutex `+0xfb3c`, its array at `+0x1188`.
fn prep_regist_clusters(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0xfb40);
    *this = scene;
    let m = rng.next();
    t.w32(scene + 0xfb3c, m);
    array_at(t, scene + 0x1188, obj + 0x400, 8, rng);
}

/// The Iggy player opened or not; the application at `obj + 0x400`.
fn prep_opening(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj.wrapping_sub(0x0001_0000);
    let player = if rng.chance(3) { 0 } else { obj + 0x200 };
    t.stub_ret(glue6::IGGY_PLAYER_OPEN, false, 5, 0, player);
    t.stub_ret(glue6::GET_APPLICATION, false, 0, 0, obj + 0x400);
    let b = rng.next() as u8;
    let at = t.rebase(glue6::OPENING_PLAYING.sym);
    t.write(at, &[b]);
}

/// The broken screen's mode and a colour at `obj + 0x80`.
fn prep_colour(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let mode = pick(rng, &[1, 2, 1, 2, 0, 3]);
    t.w32(obj, mode);
    for k in 0..4 {
        let f = if k == 3 && rng.chance(2) {
            // alphas around 1, where the clamp works
            [0.0f32, 0.5, 1.0, 1.5, 2.0, 0.999, 3.0][rng.below(7) as usize]
        } else {
            rng.float()
        };
        t.write(obj + 0x80 + 4 * k, &f.to_le_bytes());
    }
    args[0] = obj + 0x80;
}

/// A free list (`+0x18`) with a node or empty (refilled or not), the key
/// and value at `obj + 0x80`, the insert refusing now and then.
fn prep_tree_insert(
    t: &mut Twins,
    obj: u32,
    this: &mut u32,
    args: &mut [u32],
    rng: &mut Rng,
    insert: u32,
) {
    let tree = obj + 0x40;
    *this = tree;
    let node = obj + 0x200;
    t.w32(node, obj + 0x240);
    let head = if rng.chance(3) { 0 } else { node };
    t.w32(tree + 0x18, head);
    let refill = if rng.chance(2) { 0 } else { node };
    t.stub_this_out(glue6::FREE_LIST_ALLOCATE_BLOCK, 1, 1, None, &[(0, refill)]);
    let r = if rng.chance(2) { 0 } else { rng.next() | 1 };
    t.stub_ret(insert, true, 1, 1, r);
    args[0] = obj + 0x80;
    args[1] = obj + 0x84;
}

fn prep_tree_insert_64c020(
    t: &mut Twins,
    obj: u32,
    this: &mut u32,
    args: &mut [u32],
    rng: &mut Rng,
) {
    prep_tree_insert(t, obj, this, args, rng, glue6::TREE_INSERT_64C020);
}

fn prep_tree_insert_6be960(
    t: &mut Twins,
    obj: u32,
    this: &mut u32,
    args: &mut [u32],
    rng: &mut Rng,
) {
    prep_tree_insert(t, obj, this, args, rng, glue6::TREE_INSERT_6BE960);
}

/// The application at `obj + 0x400` (slot 10 a stub), its renderer at
/// `+0x124` (slot 2 a stub); `flip`, slot 2 and the window messages
/// failing or quitting now and then; the Iggy menus or none.
fn prep_render_opening(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let app = obj + 0x400;
    t.stub_ret(glue6::GET_APPLICATION, false, 0, 0, app);
    let s10 = t.stub(None, 0xa10, true, 0, 0, Some(1));
    t.w32(app, obj + 0x900);
    t.w32(obj + 0x900 + 40, s10);
    let renderer = app + 0x124;
    let r2 = if rng.chance(4) { rng.next() | 1 } else { 0 };
    let s2 = t.stub(None, 0xa02, true, 0, 0, Some(r2));
    t.w32(renderer, obj + 0xa00);
    t.w32(obj + 0xa08, s2);
    let flip = if rng.chance(4) { rng.next() | 1 } else { 0 };
    t.stub_ret(glue6::RENDERER_FLIP, true, 0, 0, flip);
    let quit = u32::from(rng.chance(3));
    t.stub_ret(glue6::PROCESS_WINDOWS_MESSAGE, false, 0, 0, quit);
    let menus = if rng.chance(3) { 0 } else { rng.next() | 1 };
    t.global(glue6::IGGY_MENU_MANAGER.sym, menus);
}

/// The two sampler descriptions: random bytes, the flag bit 7 set or not.
fn prep_samplers(t: &mut Twins, _: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    for desc in [glue6::SAMPLER_DESC_A, glue6::SAMPLER_DESC_B] {
        let at = t.rebase(desc);
        let b = rng.bytes(0x24);
        t.write(at, &b);
        let flags = rng.next() & !0x80 | if rng.chance(2) { 0x80 } else { 0 };
        t.w32(at + 0x1c, flags);
    }
}

#[rustfmt::skip]
const CASES6: &[Case] = &[
    Case { prep: Some(prep_singletons), ..cdecl("create_vfx_render_data_table", 0x0069_f4c0, 0, &[(0x0069_e140, K::This(0))], |m, _, _| { g6::create_singleton(m, glue6::VFX_RENDER_DATA_TABLE_SINGLETON); Out::None }) },
    Case { prep: Some(prep_singletons), ..cdecl("create_broken_screen", 0x006d_4d30, 0, &[(0x006d_4ba0, K::This(0))], |m, _, _| { g6::create_singleton(m, glue6::BROKEN_SCREEN_SINGLETON); Out::None }) },
    Case { prep: Some(prep_singletons), ..cdecl("create_free_camera", 0x0042_f260, 0, &[(0x0067_fe70, K::This(0))], |m, _, _| u(g6::free_camera_instance(m))) },
    Case { prep: Some(prep_singletons), ..cdecl("create_cluster_manager", 0x0069_a060, 0, &[(0x0069_9ab0, K::This(0))], |m, _, _| { g6::create_singleton(m, glue6::CLUSTER_MANAGER_SINGLETON); Out::None }) },
    Case { prep: Some(prep_singletons), ..cdecl("create_vfx_shader_manager", 0x006c_1420, 0, &[(0x006b_f4a0, K::This(0))], |m, _, _| { g6::create_singleton(m, glue6::VFX_SHADER_MANAGER_SINGLETON); Out::None }) },
    Case { prep: Some(prep_singletons), ..cdecl("create_distortion", 0x006c_2190, 0, &[(0x006c_2010, K::This(0))], |m, _, _| { g6::create_singleton(m, glue6::DISTORTION_SINGLETON); Out::None }) },
    Case { prep: Some(prep_singletons), ..cdecl("create_shadow_manager", 0x006c_9500, 0, &[(0x006c_7d80, K::This(0))], |m, _, _| { g6::create_singleton(m, glue6::SHADOW_MANAGER_SINGLETON); Out::None }) },
    Case { prep: Some(prep_singletons), ..cdecl("create_vfx_render_map", 0x0069_f440, 0, &[(glue6::MAP_BUY_HEAD_NODE, K::This(0))], |m, _, _| { g6::create_vfx_render_map(m); Out::None }) },
    Case { prep: Some(prep_asset_loaded), ..cdecl("scene_on_field_map_asset_loaded", 0x0066_6630, 2, &[(glue6::ASYNC_LOAD_INSTANCE, K::Getter), (glue6::SCENE_DO_ON_FIELD_MAP_ASSET_LOADED, K::This(7)), (glue6::SCENE_DO_ON_ALL_FIELD_MAP_ASSETS_LOADED, K::This(4))], |m, _, a| u(g6::scene_on_field_map_asset_loaded(m, a[0], a[1]))) },
    framed(Case { prep: Some(prep_vfx_texture_loaded), ..cdecl("scene_on_vfx_texture_loaded", 0x0066_6880, 2, &[(glue6::ASYNC_LOAD_INSTANCE, K::Getter), (glue6::GET_APPLICATION, K::Cdecl(0)), (glue6::FIXUP_CLUSTERS, K::Cdecl(2)), (glue6::SCENE_REGIST_VFX_TEXTURE, K::This(3))], |m, _, a| u(g6::scene_on_vfx_texture_loaded(m, frame_ebp(), a[0], a[1]))) }, 0, 4),
    Case { prep: Some(prep_ps2_list), ..cdecl("process_ps2_data_loading_callback", 0x0064_27f0, 0, &[(glue6::WAIT_FOR_SINGLE_OBJECT, K::Import(2)), (glue6::RELEASE_MUTEX, K::Import(1)), (glue6::SA_READ_CALLBACK, K::Cdecl(1)), (glue6::PS2_LOAD_LIST_RESET, K::This(1))], |m, _, _| u(g6::process_ps2_data_loading_callback(m))) },
    Case { prep: Some(prep_dtt), ..method("scene_render_dtt_vfx", 0x0065_31b0, This::Object, 0, &[(glue6::RENDER_MESH_INSTANCE, K::This(4)), (glue6::RENDERER_END_SCENE, K::This(1))], |m, this, _| u(g6::scene_render_dtt_vfx(m, this))) },
    method("sampler_holder_ctor", 0x006f_5ae0, This::Object, 1, &[(glue6::SAMPLER_STATE_CTOR, K::This(0))], |m, this, a| u(g6::sampler_holder_ctor(m, this, a[0]))),
    Case { prep: Some(prep_pool56), ..method("scene_delete_pool_object56", 0x0065_a430, This::Object, 2, &[(glue6::POOL56_DTOR, K::This(0)), (glue6::CLUSTER_DELETE_OBJECT, K::This(1))], |m, this, a| u(g6::scene_delete_pool_object56(m, this, a[0], a[1]))) },
    Case { prep: Some(prep_pool28), ..method("scene_delete_pool_object28", 0x0065_a1d0, This::Object, 2, &[(glue6::CLUSTER_DELETE_OBJECT, K::This(1))], |m, this, a| u(g6::scene_delete_pool_object28(m, this, a[0], a[1]))) },
    cdecl("level_set_name", 0x0064_1db0, 1, &[(glue6::HASH_U, K::Cdecl(1)), (glue6::IOB_FUNC, K::ImportC(0)), (glue6::FPRINTF, K::ImportC(3)), (glue6::POST_PROCESS_INSTANCE, K::Getter), (glue6::POST_SET_HDAO_SETTING, K::This(1)), (glue6::SCENE_SET_LEVEL_NAME_HASH, K::This(1)), (glue6::SCENE_SET_LEVEL_NAME, K::This(1))], |m, _, a| u(g6::level_set_name(m, a[0]))),
    framed(Case { prep: Some(prep_post_lookup), ..method("post_lookup", 0x0069_35f0, This::Object, 1, &[], |m, this, a| u(g6::post_lookup(m, frame_ebp(), this, a[0]))) }, 0x20, 0),
    Case { prep: Some(prep_regist_clusters), ..method("scene_regist_clusters_to_add", 0x0066_8550, This::Object, 2, &[(glue6::WAIT_FOR_SINGLE_OBJECT, K::Import(2)), (glue6::RELEASE_MUTEX, K::Import(1)), (glue6::ARRAY_RESERVE_668300, K::Keep(1))], |m, this, a| u(g6::scene_regist_clusters_to_add(m, this, a[0], a[1]))) },
    Case { prep: Some(prep_opening), ..method("scene_begin_render_opening_screen", 0x0065_79d0, This::Object, 0, &[(glue6::IGGY_SET_CALLBACK, K::This(2)), (glue6::RENDERER_SET_CLEAR_COLOR, K::This(4))], |m, this, _| u(g6::scene_begin_render_opening_screen(m, this))) },
    Case { prep: Some(prep_colour), ..method("broken_set_colour", 0x006d_4620, This::Object, 1, &[], |m, this, a| { g6::broken_set_colour(m, this, a[0]); Out::None }) },
    Case { prep: Some(prep_tree_insert_64c020), ..method("tree_insert_new_0065fd50", 0x0065_fd50, This::Object, 2, &[(glue6::FREE_LIST_DEALLOCATE, K::This(1)), (glue6::TREE_POST_INSERT, K::This(1))], |m, this, a| u(g6::tree_insert_new(m, glue6::TREE_INSERT_64C020, this, a[0], a[1]))) },
    Case { prep: Some(prep_tree_insert_6be960), ..method("tree_insert_new_006c0810", 0x006c_0810, This::Object, 2, &[(glue6::FREE_LIST_DEALLOCATE, K::This(1)), (glue6::TREE_POST_INSERT, K::This(1))], |m, this, a| u(g6::tree_insert_new(m, glue6::TREE_INSERT_6BE960, this, a[0], a[1]))) },
    Case { prep: Some(prep_render_opening), ..method("scene_render_opening_screen", 0x0066_b2e0, This::Object, 0, &[(glue6::RENDERER_BEGIN_SCENE, K::This(1)), (glue6::RENDERER_END_SCENE, K::This(1)), (glue6::IGGY_MENU_RENDER, K::This(0)), (glue6::EXIT_PROCESS, K::Import(1)), (glue6::SLEEP_MS, K::Cdecl(1)), (glue6::SCENE_DO_DELETE_MESH_INSTANCES, K::This(0))], |m, this, _| u(g6::scene_render_opening_screen(m, this))) },
    Case { prep: Some(prep_samplers), ..cdecl("sampler_states_init", 0x006f_5470, 0, &[(glue6::SAMPLER_CREATE, K::This(0))], |m, _, _| u(g6::sampler_states_init(m))) },
];

#[test]
fn batch6_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 8 } else { 40 };
        run_cases(t, CASES6, n, &mut Rng(0x6006_b006));
    });
}

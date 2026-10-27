//! The render-side glue, sixty-fifth batch (M4 port round 19, list
//! `h3-glue-render`): the scene's texture video load. The exception frame
//! and cookie are left out, as in [`crate::glue10`].

use ffx_bridge::Global;

pub use crate::h3_glue13::{ARRAY_DELETE, ARRAY_NEW};
pub use crate::h3_glue16::{FMV_INSTANCE, GET_SCENARIO_FLAG, PHYRE_PRINTF, TEXTURE_IS_BOUND};
pub use crate::h3_glue18::{SET_DIMENSIONS, UPDATE_MAX_LEVEL};
pub use crate::h3_glue35::PHYRE_MALLOC;
pub use crate::h3_glue47::{FIOS_CLOSE, FIOS_OPEN, FIOS_READ};
pub use crate::h3_glue59::FIOS_UNIFY;

/// `TkGetNowEventJumpId()` (cdecl).
pub const EVENT_JUMP_ID: u32 = 0x0088_d660;
/// `PhyFMVPlayerManager_getVideoName(manager; id, name)` (thiscall, ret
/// 8): `al`, whether video `id` has a name (written to `name`).
pub const GET_VIDEO_NAME: u32 = 0x006d_93e0;
/// `PhyFMVPlayerDelegation_onTextureVideoPlay(manager)` (thiscall).
pub const ON_TEXTURE_VIDEO_PLAY: u32 = 0x006d_a480;
/// `PVideoPlaybackWin32_open(player; path, loop)` (thiscall, ret 8).
pub const VIDEO_OPEN: u32 = 0x00a2_7f20;
/// `strstr`'s import slot (cdecl).
pub const STRSTR_IAT: u32 = crate::glue8::STRSTR;
/// `memcpy` (cdecl).
pub const MEMCPY: u32 = crate::basic::MEMCPY;
/// `PTexture2DD3D11`'s constructor (thiscall) and
/// `FUN_00599b70(texture; count)` (thiscall, ret 4): sets the buffer
/// count, 0 when the texture's buffers are to be cleared (`+8`).
pub const TEXTURE_CTOR: u32 = 0x0058_4240;
pub const SET_BUFFER_COUNT: u32 = 0x0059_9b70;
/// The texture format the video textures get (`0x00ca2fcc`).
pub const VIDEO_FORMAT: u32 = 0x00ca_2fcc;
/// `graphicVFXSetCurrentVFXType(kind)`, `graphicVFXCreate(lo, hi,
/// flags)`, `graphicVFXCreateDataBuffer(lo, hi, n, a, b, names, c)`,
/// `graphicVFXGetRenderDataByInstance(lo, hi)`, `graphicVFXSetup(lo,
/// hi)` and `graphicVFXSetVisible(lo, hi, on)` (all cdecl).
pub const VFX_SET_TYPE: u32 = 0x0064_5670;
pub const VFX_CREATE: u32 = 0x0064_4de0;
pub const VFX_CREATE_DATA_BUFFER: u32 = 0x0064_5040;
pub const VFX_RENDER_DATA: u32 = 0x0064_53d0;
pub const VFX_SETUP: u32 = 0x0064_5780;
pub const VFX_SET_VISIBLE: u32 = 0x0064_5710;

/// The format strings: `"ScenarioFlag=%d\n"`, `"Texture Video not found
/// for video %d\n"`, `"Load Texture Video %d=%s, loop=%d\n"` and the
/// buffer-count warning.
pub const SCENARIO_FORMAT: u32 = 0x00b4_236c;
pub const NOT_FOUND_FORMAT: u32 = 0x00b4_2380;
pub const LOAD_FORMAT: u32 = 0x00b4_23a8;
pub const ALTERING_BUFFER_COUNT: u32 = 0x00b4_2278;
/// The map names looked for in the path (`"lchb01"`, `"lchb04"`,
/// `"luca08"`) and the texture video files chosen
/// (`.../texVideoLchb.bin`, `.../texVideoLchb04.bin`,
/// `.../texVideoLuca.bin`).
pub const LCHB01: u32 = 0x00b4_23cc;
pub const LCHB04: u32 = 0x00b4_2410;
pub const LUCA08: u32 = 0x00b4_2454;
pub const FILE_LCHB: u32 = 0x00b4_23d4;
pub const FILE_LCHB04: u32 = 0x00b4_2418;
pub const FILE_LUCA: u32 = 0x00b4_245c;
/// `"VideoTexture"` (13 bytes copied into the frame).
pub const VIDEO_TEXTURE: u32 = 0x00b4_24a8;

/// The open error's message: ATL's default string manager, the resource
/// lookup, `LoadString`, `CA2W`, `Replace`, `free`, `ShowCursor`,
/// `MessageBoxW`, `ResetToLauncher`, `InterlockedExchangeAdd` (the
/// original's `lock xadd`), `L"$name$"`, `L"error"` and the string id
/// (as in `PhyFMVPlayerManager_fmvPlayer_play`).
pub const STRING_MANAGER: u32 = 0x00c6_4bf0;
pub const FIND_RESOURCE: u32 = 0x0064_f970;
pub const LOAD_STRING: u32 = 0x0065_2310;
pub const A2W: u32 = 0x0065_1800;
pub const STRING_REPLACE: u32 = 0x0065_4070;
pub const FREE_IAT: u32 = 0x00b0_c370;
pub const SHOW_CURSOR_IAT: u32 = 0x00b0_c688;
pub const MESSAGE_BOX_IAT: u32 = 0x00b0_c684;
pub const RESET_TO_LAUNCHER: u32 = 0x0042_f4d0;
pub const INTERLOCKED_ADD_IAT: u32 = 0x00b0_c128;
pub const NAME_TOKEN: u32 = 0x00b4_2498;
pub const ERROR_TITLE: u32 = 0x00b3_f9fc;
pub const MOVIE_ERROR_ID: u32 = 0x7833;

/// The quad template's constants (-0.54, -0.325, 0.425, 0.5, -1), the
/// render data's scale (10), and its bounds (-10000, 10000).
pub const QUAD_A: Global<u32> = Global::new(0x00b4_3dec);
pub const QUAD_B: Global<u32> = Global::new(0x00b4_3de8);
pub const QUAD_C: Global<u32> = Global::new(0x00b4_3d08);
pub const QUAD_HALF: Global<u32> = Global::new(0x00b9_2194);
pub const MINUS_ONE: Global<u32> = Global::new(0x00b9_2340);
pub const TEN: Global<u32> = Global::new(0x00b9_2318);
pub const LOW: Global<u32> = Global::new(0x00b4_1db0);
pub const HIGH: Global<u32> = Global::new(0x00b1_0f4c);

/// The quad template the original builds in its frame for every effect
/// and never reads: 66 words from `ebp - 0xa24` up, coded `1` (1.0),
/// `0`, `N` (-1), `A`, `B`, `C`, `H` (the constants above).
pub const QUAD_TEMPLATE: &[u8; 66] =
    b"1111111111111111111111110N00N00N00N00N00N0AB0CB0AH0CH0110110001110";

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87::round_trip;
    use ffx_graphic::host::{Conv, Host};

    /// The frame (`ebp -` these): the scratch words, the effect name, the
    /// read buffer, the video name and the unified path.
    const SCRATCH_60: u32 = 0xb60;
    const SCRATCH_5C: u32 = 0xb5c;
    const ID_LO: u32 = 0xb58;
    const MIN_Y: u32 = 0xb54;
    const ID: u32 = 0xb50;
    const INDICES: u32 = 0xb4c;
    const ID_HI: u32 = 0xb48;
    const SCENE: u32 = 0xb44;
    const MIN_Z: u32 = 0xb40;
    const COUNTS: u32 = 0xb3c;
    const LEFT: u32 = 0xb38;
    const MAX_X: u32 = 0xb34;
    const RECORD: u32 = 0xb30;
    const DATA: u32 = 0xb2c;
    const EFFECT_NAME: u32 = 0xb24;
    const QUAD: u32 = 0xa24;
    const READ_BUFFER: u32 = 0x91c;
    const NAME: u32 = 0x51c;
    const PATH: u32 = 0x11c;
    const INDEX_WORDS: u32 = 0x1c;

    /// The three video textures.
    const TEXTURES: [u32; 3] = [0xdd8, 0xddc, 0xde0];

    fn printf(h: &mut impl Host, args: &[u32]) {
        h.call_sym(PHYRE_PRINTF, Conv::Cdecl, args);
    }

    /// `n * size` as the original's `mul`: all ones when it overflows.
    fn words(n: u32, size: u32) -> u32 {
        n.saturating_mul(size)
    }

    /// `PhyreScene_textureVideoLoad(scene; path)` (thiscall, ret 4; frame
    /// from `ebp - 0xb60`; Ghidra: void).
    ///
    /// `+0xdd1` cleared. Scenario flag 0x194 picks video 0x6b (a byte
    /// loop flag `+0x10c8`), event jump 0x15d video 200 (the flag set as
    /// a word); the flag logged. With a video (`+0x10c4` not -1) not yet
    /// loaded (`+0xdd0`): its name from the FMV manager (none: logged,
    /// the video reset to -1). Then the texture video file for the map
    /// in `path` (lchb01, lchb04, else Luca's) read in 0x3ff-byte chunks
    /// into a 0x14000-byte buffer (`+0xdd4`); the three 1920x1080
    /// textures made ([`textures`]); the FMV delegation told and the
    /// movie opened on `+0xde8` with the loop flag in the low byte of a
    /// frame word (its upper bytes the last texture allocation's). An
    /// open failing with -3 shows [`open_error`]. Then `+0x10ca`
    /// cleared, `+0xdd0` and `+0xdc8` set, and the file's effects made
    /// ([`effect`], their count at `+0x10c0`); the buffer freed.
    pub fn texture_video_load(h: &mut impl Host, ebp: u32, scene: u32, path: u32) {
        h.write(ebp - SCENE, scene);
        h.write(scene + 0xdd1, 0_u8);
        let flag = h.call_sym(GET_SCENARIO_FLAG, Conv::Cdecl, &[]) & 0xffff;
        if flag == 0x194 {
            h.write(scene + 0x10c4, 0x6b_u32);
            h.write(scene + 0x10c8, 1_u8);
        }
        if h.call_sym(EVENT_JUMP_ID, Conv::Cdecl, &[]) == 0x15d {
            h.write(scene + 0x10c4, 200_u32);
            h.write(scene + 0x10c8, 1_u16);
        }
        let format = h.rebase(SCENARIO_FORMAT);
        printf(h, &[2, format, flag]);
        let video: u32 = h.read(scene + 0x10c4);
        if video == u32::MAX || h.read::<u8>(scene + 0xdd0) != 0 {
            return;
        }
        let name = ebp - NAME;
        let manager = h.call_sym(FMV_INSTANCE, Conv::Cdecl, &[]);
        let found = h.call_sym(GET_VIDEO_NAME, Conv::Thiscall(manager), &[video, name]);
        if found as u8 == 0 {
            let format = h.rebase(NOT_FOUND_FORMAT);
            let video = h.read(scene + 0x10c4);
            printf(h, &[2, format, video]);
            h.write(scene + 0x10c4, u32::MAX);
            return;
        }
        let looped = u32::from(h.read::<u8>(scene + 0x10c8));
        let video = h.read(scene + 0x10c4);
        let format = h.rebase(LOAD_FORMAT);
        printf(h, &[2, format, video, name, looped]);
        let file = video_file(h, path);
        read_file(h, ebp, scene, file);
        textures(h, ebp, scene);
        let manager = h.call_sym(FMV_INSTANCE, Conv::Cdecl, &[]);
        h.call_sym(ON_TEXTURE_VIDEO_PLAY, Conv::Thiscall(manager), &[]);
        let looped: u8 = h.read(scene + 0x10c8);
        h.write(ebp - LEFT, looped);
        let word = h.read(ebp - LEFT);
        let opened = h.call_sym(VIDEO_OPEN, Conv::Thiscall(scene + 0xde8), &[name, word]);
        if opened == 0xffff_fffd {
            open_error(h, ebp - DATA, name);
        }
        effects(h, ebp, scene);
        let data = h.read(scene + 0xdd4);
        h.call_sym(ARRAY_DELETE, Conv::Cdecl, &[data]);
        h.write(scene + 0xdd4, 0_u32);
    }

    /// The texture video file for the map in `path` (a `strstr` for
    /// lchb01, then lchb04, then luca08, whose answer is not used).
    fn video_file(h: &mut impl Host, path: u32) -> u32 {
        let strstr: u32 = h.read(h.rebase(STRSTR_IAT));
        for (map, file) in [(LCHB01, FILE_LCHB), (LCHB04, FILE_LCHB04)] {
            let map = h.rebase(map);
            if h.call(strstr, Conv::Cdecl, &[path, map]) != 0 {
                return h.rebase(file);
            }
        }
        let map = h.rebase(LUCA08);
        h.call(strstr, Conv::Cdecl, &[path, map]);
        h.rebase(FILE_LUCA)
    }

    /// The file unified into `ebp - 0x11c`, opened (handle at `ebp -
    /// 0xb2c`) and read in chunks of up to 0x3ff bytes through `ebp -
    /// 0x91c` into a new 0x14000-byte buffer at `+0xdd4`, until a read
    /// answers 0 or less; then closed.
    fn read_file(h: &mut impl Host, ebp: u32, scene: u32, file: u32) {
        let unified = ebp - PATH;
        h.call_sym(FIOS_UNIFY, Conv::Cdecl, &[file, unified, 0x100]);
        let handle = h.call_sym(FIOS_OPEN, Conv::Cdecl, &[unified, 1]);
        h.write(ebp - DATA, handle);
        let buffer = h.call_sym(ARRAY_NEW, Conv::Cdecl, &[0x14000]);
        h.write(scene + 0xdd4, buffer);
        let chunk = ebp - READ_BUFFER;
        let mut at = 0_u32;
        loop {
            let n = h.call_sym(FIOS_READ, Conv::Cdecl, &[handle, chunk, 0x3ff]);
            if n as i32 <= 0 {
                break;
            }
            let buffer: u32 = h.read(scene + 0xdd4);
            h.call_sym(MEMCPY, Conv::Cdecl, &[buffer.wrapping_add(at), chunk, n]);
            at = at.wrapping_add(n);
        }
        h.call_sym(FIOS_CLOSE, Conv::Cdecl, &[handle]);
    }

    /// The three textures (`+0xdd8`, `+0xddc`, `+0xde0`): each 0x74 bytes
    /// from `PhyreMalloc` (kept at `ebp - 0xb38`), constructed (0 when
    /// none). Then each sized 1920x1080 in the video format, given 3
    /// buffers unless it has 3 (`+0x24`, its top bit aside; warned when
    /// bound, `+8` cleared when that answers 0) and flagged 0x200
    /// (`+0x14`); then each cleared of flag 2 and its levels updated.
    fn textures(h: &mut impl Host, ebp: u32, scene: u32) {
        for off in TEXTURES {
            let p = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[0x74]);
            h.write(ebp - LEFT, p);
            let texture = if p == 0 {
                0
            } else {
                h.call_sym(TEXTURE_CTOR, Conv::Thiscall(p), &[]);
                p
            };
            h.write(scene + off, texture);
        }
        let format = h.rebase(VIDEO_FORMAT);
        for off in TEXTURES {
            let texture: u32 = h.read(scene + off);
            h.call_sym(
                SET_DIMENSIONS,
                Conv::Thiscall(texture),
                &[0x780, 0x438, format, 0],
            );
            let texture: u32 = h.read(scene + off);
            if h.read::<u32>(texture.wrapping_add(0x24)) & 0x7fff_ffff != 3 {
                if h.call_sym(TEXTURE_IS_BOUND, Conv::Thiscall(texture), &[]) as u8 != 0 {
                    let warning = h.rebase(ALTERING_BUFFER_COUNT);
                    printf(h, &[1, warning]);
                }
                if h.call_sym(SET_BUFFER_COUNT, Conv::Thiscall(texture), &[3]) == 0 {
                    h.write(texture.wrapping_add(8), 0_u32);
                }
            }
            let texture: u32 = h.read(scene + off);
            let flags: u32 = h.read(texture.wrapping_add(0x14));
            h.write(texture.wrapping_add(0x14), flags | 0x200);
        }
        for off in TEXTURES {
            let texture: u32 = h.read(scene + off);
            let flags: u32 = h.read(texture.wrapping_add(0x14));
            h.write(texture.wrapping_add(0x14), flags & !2);
            h.call_sym(UPDATE_MAX_LEVEL, Conv::Thiscall(texture), &[]);
        }
    }

    /// The open error's message box (string 0x7833 with `$name$`
    /// replaced by the video name in UTF-16; the string at `at`, the
    /// conversion's pointer at `at + 4` and its inline buffer at `at +
    /// 8`), then `ResetToLauncher`, and the string released.
    fn open_error(h: &mut impl Host, at: u32, name: u32) {
        let conv_at = at + 4;
        let inline = at + 8;
        let strings = h.rebase(STRING_MANAGER);
        let nil = h.call_virtual(strings, 3, &[]);
        h.write(at, nil.wrapping_add(0x10));
        let module = h.call_sym(FIND_RESOURCE, Conv::Cdecl, &[MOVIE_ERROR_ID, 0]);
        if module != 0 {
            h.call_sym(LOAD_STRING, Conv::Thiscall(at), &[module, MOVIE_ERROR_ID]);
        }
        h.write(conv_at, inline);
        h.call_sym(A2W, Conv::Thiscall(conv_at), &[name, 0xfde9]);
        let converted: u32 = h.read(conv_at);
        let token = h.rebase(NAME_TOKEN);
        h.call_sym(STRING_REPLACE, Conv::Thiscall(at), &[token, converted]);
        let converted: u32 = h.read(conv_at);
        if converted != inline {
            let free: u32 = h.read(h.rebase(FREE_IAT));
            h.call(free, Conv::Cdecl, &[converted]);
        }
        let show: u32 = h.read(h.rebase(SHOW_CURSOR_IAT));
        h.call(show, Conv::Stdcall, &[1]);
        let text: u32 = h.read(at);
        let message_box: u32 = h.read(h.rebase(MESSAGE_BOX_IAT));
        let title = h.rebase(ERROR_TITLE);
        h.call(message_box, Conv::Stdcall, &[0, text, title, 0x10]);
        h.call_sym(RESET_TO_LAUNCHER, Conv::Cdecl, &[]);
        let data = text.wrapping_sub(0x10);
        let add: u32 = h.read(h.rebase(INTERLOCKED_ADD_IAT));
        let old = h.call(add, Conv::Stdcall, &[data.wrapping_add(0xc), u32::MAX]);
        if old.wrapping_sub(1) as i32 <= 0 {
            let owner: u32 = h.read(data);
            h.call_virtual(owner, 1, &[data]);
        }
    }

    /// The effects: the buffer's word 0 (signed) of them, the effect
    /// name `"VideoTexture"` copied to `ebp - 0xb24`, then for each its
    /// record (`+0xd68 + 0x18 k`) and [`effect`].
    fn effects(h: &mut impl Host, ebp: u32, scene: u32) {
        let data: u32 = h.read(scene + 0xdd4);
        h.write(scene + 0x10ca, 0_u8);
        h.write(scene + 0xdd0, 1_u8);
        h.write(scene + 0xdc8, 1_u32);
        let count: u32 = h.read(data);
        h.write(scene + 0x10c0, count);
        let text = h.rebase(VIDEO_TEXTURE);
        for k in 0..3 {
            let w: u32 = h.read(text + 4 * k);
            h.write(ebp - EFFECT_NAME + 4 * k, w);
        }
        let mut p = data.wrapping_add(4);
        h.write(ebp - DATA, p);
        h.write(ebp - LEFT, count);
        let last: u8 = h.read(text + 12);
        h.write(ebp - EFFECT_NAME + 12, last);
        if count as i32 <= 0 {
            return;
        }
        let mut id = 0x78_u32;
        h.write(ebp - ID, id);
        let mut record = scene + 0xd6c;
        h.write(ebp - RECORD, record);
        loop {
            let next = effect(h, ebp, scene, p, id, record);
            id = id.wrapping_add(1);
            record = record.wrapping_add(0x18);
            let left = h.read::<u32>(ebp - LEFT).wrapping_sub(1);
            h.write(ebp - LEFT, left);
            p = next;
            h.write(ebp - RECORD, record);
            h.write(ebp - DATA, p);
            h.write(ebp - ID, id);
            if left == 0 {
                break;
            }
        }
    }

    /// One effect from the data at `p` (`[n, m, n positions, m vec3s, m
    /// vec4s, m vec2s, m halves]`): VFX instance `0x5bc:id` created
    /// (kind 1, flags 0x200000 for video 200, else 0x40000), its data
    /// buffer (kind 4) named, and its render data's `+0x94` given copies
    /// of the five arrays (`+0xc` .. `+0x1c`, sizes all ones when the
    /// `mul` overflows; also kept in the record `+0 .. +0x10`; `+0x24` =
    /// 2), the record's `-4` cleared, its matrix scaled by 10 and its
    /// bounds +-10000; then set up and made visible. The original also
    /// builds a quad template and the positions' bounds in its frame,
    /// which nothing reads (the y maximum's compare goes to the x
    /// maximum's word, as in the original). The next effect's data.
    fn effect(h: &mut impl Host, ebp: u32, scene: u32, p: u32, id: u32, record: u32) -> u32 {
        let counts = p.wrapping_add(4);
        h.write(ebp - SCRATCH_5C, 4_u32);
        h.write(ebp - SCRATCH_60, 6_u32);
        h.write(ebp - COUNTS, counts);
        h.call_sym(VFX_SET_TYPE, Conv::Cdecl, &[1]);
        let hi = (((id as i32) >> 31) as u32).wrapping_add(0x5bc);
        let flags = if h.read::<u32>(scene + 0x10c4) == 200 {
            0x0020_0000
        } else {
            0x0004_0000
        };
        h.write(ebp - ID_HI, hi);
        h.write(ebp - ID_LO, id);
        h.call_sym(VFX_CREATE, Conv::Cdecl, &[id, hi, flags]);
        h.call_sym(VFX_SET_TYPE, Conv::Cdecl, &[4]);
        let name = ebp - EFFECT_NAME;
        h.call_sym(
            VFX_CREATE_DATA_BUFFER,
            Conv::Cdecl,
            &[id, hi, 1, p, counts, name, 0],
        );
        let render = h.call_sym(VFX_RENDER_DATA, Conv::Cdecl, &[id, hi]);
        h.write(ebp - MAX_X, render);
        quad_template(h, ebp);
        let n: u32 = h.read(p);
        let normals = p.wrapping_add(8).wrapping_add(n.wrapping_mul(12));
        let m: u32 = h.read(counts);
        let colors = normals.wrapping_add(m.wrapping_mul(12));
        let uvs = colors.wrapping_add(m << 4);
        let indices = uvs.wrapping_add(m.wrapping_mul(8));
        h.write(ebp - INDICES, indices);
        h.write(record.wrapping_sub(4), 0_u32);
        let arrays = [
            (0xc, p.wrapping_add(8)),
            (0x10, normals),
            (0x14, colors),
            (0x18, uvs),
            (0x1c, indices),
        ];
        for (k, (off, src)) in (0_u32..).zip(arrays) {
            let size = if k == 0 {
                words(h.read::<u32>(p).wrapping_mul(3), 4)
            } else {
                let m: u32 = h.read(counts);
                match k {
                    1 => words(m.wrapping_mul(3), 4),
                    2 => words(m << 2, 4),
                    3 => words(m.wrapping_add(m), 4),
                    _ => words(m, 2),
                }
            };
            let array = h.call_sym(ARRAY_NEW, Conv::Cdecl, &[size]);
            let sub: u32 = h.read(render.wrapping_add(0x94));
            h.write(sub.wrapping_add(off), array);
            let bytes = if k == 0 {
                h.read::<u32>(p).wrapping_mul(12)
            } else {
                let m: u32 = h.read(counts);
                match k {
                    1 => m.wrapping_mul(12),
                    2 => m << 4,
                    3 => m << 3,
                    _ => m.wrapping_add(m),
                }
            };
            let sub: u32 = h.read(render.wrapping_add(0x94));
            let dst: u32 = h.read(sub.wrapping_add(off));
            h.call_sym(MEMCPY, Conv::Cdecl, &[dst, src, bytes]);
            let sub: u32 = h.read(render.wrapping_add(0x94));
            let kept: u32 = h.read(sub.wrapping_add(off));
            h.write(record.wrapping_add(4 * k), kept);
        }
        let sub: u32 = h.read(render.wrapping_add(0x94));
        h.write(sub.wrapping_add(0x24), 2_u32);
        let ten = TEN.get(h);
        h.write(render, ten);
        for off in [
            4, 8, 0xc, 0x10, 0x18, 0x1c, 0x20, 0x24, 0x2c, 0x30, 0x34, 0x38,
        ] {
            h.write(render.wrapping_add(off), 0_u32);
        }
        h.write(render.wrapping_add(0x14), ten);
        h.write(render.wrapping_add(0x28), ten);
        h.write(render.wrapping_add(0x3c), 1f32);
        position_bounds(h, ebp, p);
        let low = LOW.get(h);
        for k in 0..3 {
            h.write(render.wrapping_add(0x98 + 4 * k), low);
        }
        let high = HIGH.get(h);
        for k in 0..3 {
            h.write(render.wrapping_add(0xa4 + 4 * k), high);
        }
        let lo: u32 = h.read(ebp - ID_LO);
        let hi: u32 = h.read(ebp - ID_HI);
        h.call_sym(VFX_SETUP, Conv::Cdecl, &[lo, hi]);
        h.call_sym(VFX_SET_VISIBLE, Conv::Cdecl, &[lo, hi, 1]);
        let m: u32 = h.read(counts);
        let indices: u32 = h.read(ebp - INDICES);
        indices.wrapping_add(m.wrapping_mul(2))
    }

    /// The quad template (see [`QUAD_TEMPLATE`]) and the index words at
    /// `ebp - 0x1c` (0x10000, 0x10002, 0x30002).
    fn quad_template(h: &mut impl Host, ebp: u32) {
        let a = QUAD_A.get(h);
        let b = QUAD_B.get(h);
        let c = QUAD_C.get(h);
        let half = QUAD_HALF.get(h);
        let minus_one = MINUS_ONE.get(h);
        for (k, code) in QUAD_TEMPLATE.iter().enumerate() {
            let v = match code {
                b'1' => 1f32.to_bits(),
                b'N' => minus_one,
                b'A' => a,
                b'B' => b,
                b'C' => c,
                b'H' => half,
                _ => 0,
            };
            h.write(ebp - QUAD + 4 * k as u32, v);
        }
        for (k, v) in [0x1_0000_u32, 0x1_0002, 0x3_0002].into_iter().enumerate() {
            h.write(ebp - INDEX_WORDS + 4 * k as u32, v);
        }
    }

    /// The f32 at `at`.
    fn f(h: &impl Host, at: u32) -> f32 {
        f32::from_bits(h.read::<u32>(at))
    }

    /// The positions' bounds the original keeps in its frame (`fld`/
    /// `fstp` round trips): x minimum at `ebp - 0xb30`, x maximum (and
    /// the y maximum's stores) at `- 0xb34`, y minimum at `- 0xb54`, z
    /// minimum at `- 0xb40`, z maximum at `- 0xb2c`; the y maximum is
    /// compared against the first y only.
    fn position_bounds(h: &mut impl Host, ebp: u32, p: u32) {
        let x0 = round_trip(f(h, p.wrapping_add(8)));
        h.write(ebp - MAX_X, x0);
        h.write(ebp - RECORD, x0);
        let y0 = round_trip(f(h, p.wrapping_add(0xc)));
        h.write(ebp - MIN_Z, y0);
        h.write(ebp - MIN_Y, y0);
        let n: u32 = h.read(p);
        let z0 = round_trip(f(h, p.wrapping_add(0x10)));
        h.write(ebp - DATA, z0);
        h.write(ebp - MIN_Z, z0);
        let mut max_z = z0;
        let mut at = p.wrapping_add(0x10);
        if n as i32 <= 0 {
            return;
        }
        for _ in 0..n {
            let x = f(h, at.wrapping_sub(8));
            if f(h, ebp - RECORD) > x {
                h.write(ebp - RECORD, round_trip(x));
            }
            if f(h, ebp - MAX_X) < x {
                h.write(ebp - MAX_X, round_trip(x));
            }
            let y = f(h, at.wrapping_sub(4));
            if f(h, ebp - MIN_Y) > y {
                h.write(ebp - MIN_Y, round_trip(y));
            }
            if y > y0 {
                h.write(ebp - MAX_X, round_trip(y));
            }
            let z = f(h, at);
            if f(h, ebp - MIN_Z) > z {
                h.write(ebp - MIN_Z, round_trip(z));
            }
            if z > max_z {
                max_z = round_trip(z);
                h.write(ebp - DATA, max_z);
            }
            at = at.wrapping_add(0xc);
        }
    }
}

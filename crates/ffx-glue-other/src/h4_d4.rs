//! Continuation round 2 of list `h4-glue-other`, batch 4: the escape
//! menu's eight-value Flash call, the texture-animation frame by name,
//! the soft particles' specular-map reset, and the FMOD data loaders
//! (sync manager buffers, voice lists and projects, music projects, the
//! shout stream).
//!
//! Bridges that hand callees a local take `frame`, the block laid out as
//! the original's frame from its lowest local up.

use ffx_bridge::Global;
use ffx_graphic::host::{Conv, Host};

use crate::frames::ERRCHECK;

/// `crt_memset_thunk(p, c, n)` (cdecl).
pub const MEMSET: u32 = 0x0094_964c;
/// `printToOutput(prefix, fmt, ...)` (cdecl).
pub const PRINT_TO_OUTPUT: u32 = 0x0070_7b20;
/// Its FMOD prefix (`0x00b4c30c`).
pub const PRINT_PREFIX: u32 = 0x00b4_c30c;
/// `sprintf` (import slot `0x00b0c3dc`; cdecl).
pub const SPRINTF: Global<u32> = Global::new(0x00b0_c3dc);

// ---- the eight-value Flash call

/// `IggyMenu_callIggyAS3External(menu; target, name, n, values, unused)`
/// (thiscall, ret 0x14).
pub const CALL_AS3: u32 = 0x00a2_8ba0;
/// Bytes of [`call_flash_8`]'s frame: eight 16-byte Iggy values, the
/// cookie word.
pub const FLASH_8_FRAME: u32 = 0x84;

/// `IggyMenu_callFlashFunction_8_006ffaa0(menu, name, v0..v7)` (cdecl,
/// the eight 16-byte values by value; `frame` its copy of them, at `ebp -
/// 0x84`): `IggyMenu_callIggyAS3External(menu; 0, name, 8, &values, 1)`.
/// Returns its `eax`.
pub fn call_flash_8(
    h: &mut impl Host,
    frame: u32,
    menu: u32,
    name: u32,
    values: &[u32; 32],
) -> u32 {
    for (k, &v) in values.iter().enumerate() {
        h.write(frame + 4 * k as u32, v);
    }
    h.call_sym(CALL_AS3, Conv::Thiscall(menu), &[0, name, 8, frame, 1])
}

// ---- the texture-animation frame

/// The frame name format `"%05d_%02d_%03d_%03d"` (`0x00b670a8`).
pub const FRAME_NAME_FORMAT: u32 = 0x00b6_70a8;
/// `graphicSetTextureAnimationFrameByName(frame, name)` (cdecl).
pub const SET_FRAME_BY_NAME: u32 = 0x0064_3a40;
/// Bytes of [`set_texture_anim_frame`]'s frame: the record's four words
/// (`+0x30`, `+0x34`, `+0x20`, `+0x24`), the name (0x100), the cookie.
pub const ANIM_FRAME_FRAME: u32 = 0x114;

/// `yiRender_setTextureAnimFrame_0090bdb0(base, a, b)` (cdecl; `frame` its
/// locals from `ebp - 0x114`): the animation record `*(base + 0x720 a +
/// 0x1c b + 0x9d9f0)` names its frame `"%05d_%02d_%03d_%03d"` from the
/// low half and top byte of `+0x24` and the low half and bits 16..26 of
/// `+0x34`; `graphicSetTextureAnimationFrameByName(b, name)`. Returns its
/// `eax`.
pub fn set_texture_anim_frame(h: &mut impl Host, frame: u32, base: u32, a: u32, b: u32) -> u32 {
    let at = base
        .wrapping_add(a.wrapping_mul(0x720))
        .wrapping_add(b.wrapping_mul(0x1c))
        .wrapping_add(0x9_d9f0);
    let rec: u32 = h.read(at);
    let w20: u32 = h.read(rec.wrapping_add(0x20));
    let w24: u32 = h.read(rec.wrapping_add(0x24));
    let w30: u32 = h.read(rec.wrapping_add(0x30));
    let w34: u32 = h.read(rec.wrapping_add(0x34));
    h.write(frame + 8, w20);
    h.write(frame, w30);
    h.write(frame + 4, w34);
    h.write(frame + 0xc, w24);
    let name = frame + 0x10;
    let f = SPRINTF.get(h);
    let fmt = h.rebase(FRAME_NAME_FORMAT);
    h.call(
        f,
        Conv::Cdecl,
        &[
            name,
            fmt,
            w24 & 0xffff,
            w24 >> 24,
            w34 & 0xffff,
            (w34 >> 16) & 0x7ff,
        ],
    );
    h.call_sym(SET_FRAME_BY_NAME, Conv::Cdecl, &[b, name])
}

// ---- the soft particles' specular maps

/// `PArray_Data_004d45a0(array)` (thiscall): its elements.
pub const ARRAY_DATA: u32 = 0x004d_45a0;
/// `ShaderParams_FindByName(shader; name)` (thiscall, ret 4).
pub const FIND_PARAM: u32 = 0x0056_cbf0;
/// `"SpecularMapSampler"` (`0x00b495e8`).
pub const SPECULAR_MAP: u32 = 0x00b4_95e8;
/// `memcpy` (`0x00949256`, the import thunk; cdecl).
pub const MEMCPY: u32 = 0x0094_9256;
/// Bytes of [`reset_specular_maps`]'s frame: the value (0x10), the
/// walk's words.
pub const SPECULAR_FRAME: u32 = 0x28;

/// `SoftParticle_resetSpecularMaps_006c5850(this)` (thiscall; `frame` its
/// locals from `ebp - 0x28`: the value `{0x42, 0, 0, 0}`, then the
/// walk's state at `+0x10` (the meshes' list), `+0x14` (its elements),
/// `+0x18` (the slot counter), `+0x1c` (`this`), `+0x20` (mesh index), `+0x24` (the
/// mesh's materials)): for each of the four slots with a mesh list, every
/// material (`PArray_Data`) whose shader (`+0`) has a texture-typed
/// `SpecularMapSampler` (type byte `+2`: 0x42, 0xd9..0xdc, 0x12) gets
/// the null texture value copied into its parameter block (`+4`, at the
/// parameter's offset word `+8` (at least 4), length `+0xa & 0x1fff`,
/// within the block's size word); the slot and its byte `+0x5b0` are
/// cleared. Returns `this`.
pub fn reset_specular_maps(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    h.write(frame + 0x1c, this);
    h.write(frame + 0x18, 0u32);
    for slot in 0..4u32 {
        let meshes: u32 = h.read(this + 4 * slot);
        h.write(frame + 0x10, meshes);
        if meshes == 0 {
            h.write(frame + 0x18, slot + 1);
            continue;
        }
        let elements: u32 = h.read(meshes.wrapping_add(4));
        h.write(frame + 0x14, elements);
        h.write(frame + 0x20, 0u32);
        let mut i = 0u32;
        while i < h.read::<u32>(meshes) & 0x7fff_ffff {
            let mesh: u32 = h.read(elements.wrapping_add(4 * i));
            let materials: u32 = h.read(mesh.wrapping_add(0x10));
            h.write(frame + 0x24, materials);
            let mut k = 0u32;
            while k < h.read::<u32>(materials) & 0x7fff_ffff {
                let data = h.call_sym(ARRAY_DATA, Conv::Thiscall(materials), &[]);
                let material: u32 = h.read(data.wrapping_add(4 * k));
                let shader: u32 = h.read(material);
                let name = h.rebase(SPECULAR_MAP);
                let param = h.call_sym(FIND_PARAM, Conv::Thiscall(shader), &[name]);
                if param != 0 {
                    let kind: u8 = h.read(param.wrapping_add(2));
                    let block: u32 = h.read(material.wrapping_add(4));
                    h.write(frame, 0x42u32);
                    h.write(frame + 0xc, 0u32);
                    h.write(frame + 8, 0u32);
                    h.write(frame + 4, 0u32);
                    if matches!(kind, 0x42 | 0xd9..=0xdc | 0x12) {
                        let offset = u32::from(h.read::<u16>(param.wrapping_add(8)));
                        let len = u32::from(h.read::<u16>(param.wrapping_add(0xa)) & 0x1fff);
                        if offset + len <= h.read::<u32>(block) && offset >= 4 {
                            h.call_sym(
                                MEMCPY,
                                Conv::Cdecl,
                                &[block.wrapping_add(offset), frame, len],
                            );
                        }
                    }
                }
                k += 1;
            }
            i += 1;
            h.write(frame + 0x20, i);
        }
        h.write(this + 4 * slot, 0u32);
        h.write(this + 0x5b0 + slot, 0u8);
        h.write(frame + 0x18, slot + 1);
    }
    this
}

// ---- the sync manager

/// `user_malloc(size)` (cdecl).
pub const USER_MALLOC: u32 = 0x0068_7020;
/// `"PS3"` (`0x00b4a20c`).
pub const SYNC_PLATFORM: u32 = 0x00b4_a20c;

/// `SyncManager_initData(this)` (thiscall): zeroed buffers (`user_malloc`,
/// `memset`): 0x100 bytes at `+0`, 0x200 at `+4` and `+0x24..+0x3c`, 0x190
/// at `+0x40`; `"PS3"` printed into the first (`sprintf`); `+8..+0x1c`
/// cleared, `+0x20` 1. Returns 0.
pub fn sync_init_data(h: &mut impl Host, this: u32) -> u32 {
    for (at, n) in [
        (0, 0x100),
        (4, 0x200),
        (0x24, 0x200),
        (0x28, 0x200),
        (0x2c, 0x200),
        (0x30, 0x200),
        (0x34, 0x200),
        (0x38, 0x200),
        (0x3c, 0x200),
        (0x40, 0x190),
    ] {
        let p = h.call_sym(USER_MALLOC, Conv::Cdecl, &[n]);
        h.write(this + at, p);
        h.call_sym(MEMSET, Conv::Cdecl, &[p, 0, n]);
    }
    let f = SPRINTF.get(h);
    let buf: u32 = h.read(this);
    let text = h.rebase(SYNC_PLATFORM);
    h.call(f, Conv::Cdecl, &[buf, text]);
    for at in [8, 0xc, 0x10, 0x14, 0x18, 0x1c] {
        h.write(this + at, 0u32);
    }
    h.write(this + 0x20, 1u32);
    0
}

// ---- the voice lists

/// `getFullFilename(out, name)` (cdecl).
pub const FULL_FILENAME: u32 = 0x0070_7aa0;
/// `fiosOpen(path, mode)` (cdecl).
pub const FIOS_OPEN: u32 = 0x0067_9730;
/// `fiosFileSize(file)` (cdecl).
pub const FIOS_SIZE: u32 = 0x0067_9570;
/// `user_aligned_malloc(bytes, align)` (cdecl).
pub const ALIGNED_MALLOC: u32 = 0x0068_6ff0;
/// `fiosRead(file, buf, n)` (cdecl).
pub const FIOS_READ: u32 = 0x0067_9800;
/// `fiosClose(file)` (cdecl).
pub const FIOS_CLOSE: u32 = 0x0067_9510;
/// Bytes of [`voice_init_list`]'s frame: two 0x100-byte paths, the
/// cookie.
pub const VOICE_LIST_FRAME: u32 = 0x204;

/// A cleared 0x100-byte path at `buf` (byte 0, then `memset` of the
/// rest) made the full name of `name` (`getFullFilename`).
fn full_path(h: &mut impl Host, buf: u32, name: u32) {
    h.write(buf, 0u8);
    h.call_sym(MEMSET, Conv::Cdecl, &[buf + 1, 0, 0xff]);
    h.call_sym(FULL_FILENAME, Conv::Cdecl, &[buf, name]);
}

/// The whole file at `path` read into a 16-aligned block stored at `to`
/// (`fiosFileSize`, `user_aligned_malloc`, `fiosRead`, `fiosClose`), or a
/// `printToOutput(PREFIX, missing, path)`. Returns the last call's `eax`.
fn read_whole(h: &mut impl Host, path: u32, to: u32, missing: u32) -> u32 {
    let file = h.call_sym(FIOS_OPEN, Conv::Cdecl, &[path, 1]);
    if file == 0 {
        let prefix = h.rebase(PRINT_PREFIX);
        let missing = h.rebase(missing);
        return h.call_sym(PRINT_TO_OUTPUT, Conv::Cdecl, &[prefix, missing, path]);
    }
    let size = h.call_sym(FIOS_SIZE, Conv::Cdecl, &[file]);
    let block = h.call_sym(ALIGNED_MALLOC, Conv::Cdecl, &[size, 0x10]);
    h.write(to, block);
    h.call_sym(FIOS_READ, Conv::Cdecl, &[file, block, size]);
    h.call_sym(FIOS_CLOSE, Conv::Cdecl, &[file])
}

/// `FmodVoice_initList(this)` (thiscall; `frame` its two paths from
/// `ebp - 0x204`): the voice list file (`0x00b4f994`) read into `+0x24`, then
/// the sub-list (`0x00b4f9f8` with the flag `+4`, else `0x00b4fa14`) into
/// `+0x28` (missing files printed). Returns the last call's `eax`.
pub fn voice_init_list(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    full_path(h, frame, h.rebase(0x00b4_f994));
    read_whole(h, frame, this + 0x24, 0x00b4_f9b0);
    let path = frame + 0x100;
    h.write(path, 0u8);
    h.call_sym(MEMSET, Conv::Cdecl, &[path + 1, 0, 0xff]);
    let name = if h.read::<u8>(this + 4) != 0 {
        0x00b4_f9f8
    } else {
        0x00b4_fa14
    };
    h.call_sym(FULL_FILENAME, Conv::Cdecl, &[path, h.rebase(name)]);
    read_whole(h, path, this + 0x28, 0x00b4_fa30)
}

// ---- event projects

/// `FMOD::EventSystem::load(system, path, info, &project)` (stdcall
/// thunk).
pub const EVENT_SYSTEM_LOAD: u32 = 0x0094_8c94;
/// Bytes of [`load_event_project`]'s frame: the kept argument, the path
/// (0x100), the cookie.
pub const PROJECT_FRAME: u32 = 0x108;

/// What a project load reports with: its `ERRCHECK` file and lines
/// (release, free, load, group), the missing-file line and the group's
/// name.
pub struct ProjectSite {
    /// `ERRCHECK`'s file name.
    pub file: u32,
    /// Lines of the project's release (vtable `+0x28`), free (`+0`), the
    /// load and the group lookup (`+8`).
    pub lines: [u32; 4],
    /// `printToOutput`'s missing-file format.
    pub missing: u32,
    /// The group's name.
    pub group: u32,
}

/// The music's project site (`FmodMusic.cpp`).
pub const MUSIC_PROJECT: ProjectSite = ProjectSite {
    file: 0x00b4_f4a0,
    lines: [0x6d, 0x6e, 0x76, 0x80],
    missing: 0x00b4_f4b8,
    group: 0x00b4_f4f4,
};
/// The voices' project site (`FmodVoice.cpp`).
pub const VOICE_PROJECT: ProjectSite = ProjectSite {
    file: 0x00b4_f8c0,
    lines: [0x137, 0x138, 0x148, 0x152],
    missing: 0x00b4_f948,
    group: 0x00b4_f98c,
};

/// The project at `*project` released and freed (its virtual `+0x28` and
/// `+0`, each `ERRCHECK`ed) with `*project` and `*group` cleared; `name`
/// loaded into `*project` (`EventSystem::load(system, path, 0,
/// project)`; the path at `path`); without a project a missing-file line
/// and `*group` cleared, else the group looked up (`+8: (project, name,
/// 0, group)`). Returns the last call's `eax`.
fn load_project(
    h: &mut impl Host,
    path: u32,
    system: u32,
    name: u32,
    project: u32,
    group: u32,
    site: &ProjectSite,
) -> u32 {
    let file = h.rebase(site.file);
    let old: u32 = h.read(project);
    if old != 0 {
        let vtable: u32 = h.read(old);
        let f: u32 = h.read(vtable.wrapping_add(0x28));
        let r = h.call(f, Conv::Stdcall, &[old]);
        h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, site.lines[0]]);
        let old: u32 = h.read(project);
        let vtable: u32 = h.read(old);
        let f: u32 = h.read(vtable);
        let r = h.call(f, Conv::Stdcall, &[old]);
        h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, site.lines[1]]);
        h.write(project, 0u32);
        h.write(group, 0u32);
    }
    full_path(h, path, name);
    let r = h.call_sym(
        EVENT_SYSTEM_LOAD,
        Conv::Stdcall,
        &[system, path, 0, project],
    );
    h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, site.lines[2]]);
    let loaded: u32 = h.read(project);
    if loaded == 0 {
        let prefix = h.rebase(PRINT_PREFIX);
        let missing = h.rebase(site.missing);
        let eax = h.call_sym(PRINT_TO_OUTPUT, Conv::Cdecl, &[prefix, missing, path]);
        h.write(group, 0u32);
        return eax;
    }
    let vtable: u32 = h.read(loaded);
    let f: u32 = h.read(vtable.wrapping_add(8));
    let group_name = h.rebase(site.group);
    let r = h.call(f, Conv::Stdcall, &[loaded, group_name, 0, group]);
    h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, site.lines[3]])
}

/// `FmodMusic_loadEventProject(system, name, &project, &group)` (stdcall,
/// ret 0x10; `frame` its locals from `ebp - 0x108`: `name`, the path):
/// [`load_project`] with the music's site. Returns the last call's `eax`.
pub fn load_event_project(
    h: &mut impl Host,
    frame: u32,
    system: u32,
    name: u32,
    project: u32,
    group: u32,
) -> u32 {
    h.write(frame, name);
    load_project(h, frame + 4, system, name, project, group, &MUSIC_PROJECT)
}

/// `FmodShout_initData(this)` (thiscall).
pub const SHOUT_INIT_DATA: u32 = 0x0070_e2d0;

/// `FmodVoice_initData(this; event_system, system)` (thiscall, ret 8;
/// `frame` its locals from `ebp - 0x108`: `system`, the path): without an
/// event system a line (`0x00b4f8d8`); else [`load_project`] of the voice
/// project (`0x00b4f904` with the flag `+4`, else `0x00b4f924`) into
/// `+0x20` / `+0x1c`; then the shout's data (`FmodShout_initData(+0x84;
/// system, flag)`). Returns its `eax`.
pub fn voice_init_data(
    h: &mut impl Host,
    frame: u32,
    this: u32,
    event_system: u32,
    system: u32,
) -> u32 {
    h.write(frame, system);
    if event_system == 0 {
        let prefix = h.rebase(PRINT_PREFIX);
        let text = h.rebase(0x00b4_f8d8);
        h.call_sym(PRINT_TO_OUTPUT, Conv::Cdecl, &[prefix, text]);
    } else {
        let name = if h.read::<u8>(this + 4) != 0 {
            0x00b4_f904
        } else {
            0x00b4_f924
        };
        let name = h.rebase(name);
        load_project(
            h,
            frame + 4,
            event_system,
            name,
            this + 0x20,
            this + 0x1c,
            &VOICE_PROJECT,
        );
    }
    let shout: u32 = h.read(this + 0x84);
    let flag = u32::from(h.read::<u8>(this + 4));
    h.call_sym(SHOUT_INIT_DATA, Conv::Thiscall(shout), &[system, flag])
}

// ---- the shout stream

/// `FMOD::Sound::release(sound)` (stdcall thunk).
pub const SOUND_RELEASE: u32 = 0x0094_8cfa;
/// `FMOD::System::createStream(system, path, mode, info, &sound)`
/// (stdcall thunk).
pub const CREATE_STREAM: u32 = 0x0094_8cee;
/// `FMOD::Sound::getNumSubSounds(sound, &n)` (stdcall thunk).
pub const SUB_SOUNDS: u32 = 0x0094_8d0c;
/// `FmodShout.cpp`'s name for `ERRCHECK` (`0x00b4fd34`).
pub const SHOUT_FILE: u32 = 0x00b4_fd34;
/// Bytes of [`shout_init_data`]'s frame: the path, the cookie.
pub const SHOUT_FRAME: u32 = 0x104;

/// `FmodShout_initData(this; system, flag)` (thiscall, ret 8; `frame` its
/// path): without a system a line (`0x00b4fd08`) and 0; else the old
/// stream (`+0`) released (line 0x3b, `+0xc` cleared), the stream
/// (`0x00b4fd4c` with `flag`, else `0x00b4fd78`) created (`createStream(
/// system, path, 0x200, 0, this)`, line 0x4a); without it a
/// missing-file line and `+0xc` cleared, else its sub-sound count into
/// `+0xc` (line 0x54). Returns 0.
pub fn shout_init_data(h: &mut impl Host, frame: u32, this: u32, system: u32, flag: u32) -> u32 {
    let prefix = h.rebase(PRINT_PREFIX);
    if system == 0 {
        let text = h.rebase(0x00b4_fd08);
        h.call_sym(PRINT_TO_OUTPUT, Conv::Cdecl, &[prefix, text]);
        return 0;
    }
    let file = h.rebase(SHOUT_FILE);
    let old: u32 = h.read(this);
    if old != 0 {
        let r = h.call_sym(SOUND_RELEASE, Conv::Stdcall, &[old]);
        h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, 0x3b]);
        h.write(this + 0xc, 0u32);
    }
    let name = if flag as u8 != 0 {
        0x00b4_fd4c
    } else {
        0x00b4_fd78
    };
    full_path(h, frame, h.rebase(name));
    let r = h.call_sym(
        CREATE_STREAM,
        Conv::Stdcall,
        &[system, frame, 0x200, 0, this],
    );
    h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, 0x4a]);
    let sound: u32 = h.read(this);
    if sound == 0 {
        let missing = h.rebase(0x00b4_fda4);
        h.call_sym(PRINT_TO_OUTPUT, Conv::Cdecl, &[prefix, missing, frame]);
        h.write(this + 0xc, 0u32);
    } else {
        let r = h.call_sym(SUB_SOUNDS, Conv::Stdcall, &[sound, this + 0xc]);
        h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, 0x54]);
    }
    0
}

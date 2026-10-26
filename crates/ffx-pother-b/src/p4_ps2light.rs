//! `Phyre::PS2LightingSetup`: up to eight meshes lit with the PS2
//! lighting effect, each with its matrices, colours and a key.
//!
//! Layout: the meshes at `0x00ce63e4` (8 words; 0 is free), their count
//! at `0x00ce6404`, then one 0x12c-byte record per slot at `0x00ce6408`:
//! `+0` and `+0x40` two 4x4 matrices, `+0x80` a float, `+0x84` and
//! `+0xc4` two more matrices, `+0x104` a float, `+0x108` and `+0x118`
//! two 4-word vectors, `+0x128` the key.

use ffx_graphic::host::{Conv, Host};

/// The registered meshes (8 words).
pub const MESHES: u32 = 0x00ce_63e4;
/// How many are registered.
pub const COUNT: u32 = 0x00ce_6404;
/// The records, 0x12c bytes each.
pub const RECORDS: u32 = 0x00ce_6408;
/// A record's size.
pub const RECORD: u32 = 0x12c;
/// `ShaderParam_Write_006426e0` (cdecl `(mesh, name, src)`).
pub const SHADER_PARAM_WRITE: u32 = 0x0064_26e0;
/// The parameter names [`bridge::apply`] writes (`.rdata`) and their
/// offsets in the record.
pub const PARAMS: [(u32, u32); 8] = [
    (0x00b4_1c00, 0),     // "LvMatrix"
    (0x00b4_1c0c, 0x40),  // "LcMatrix"
    (0x00b4_1c18, 0x80),  // "charlight"
    (0x00b4_1c24, 0x84),  // "ffxLvMatrix"
    (0x00b4_1c30, 0xc4),  // "ffxLcMatrix"
    (0x00b4_1c3c, 0x104), // "ffxcharlight"
    (0x00b4_1c4c, 0x108), // "ffxshade"
    (0x00b4_1c58, 0x118), // "ambientlt"
];
/// `Phyre_PFramework_PApplication_GetApplication` (cdecl).
pub const GET_APPLICATION: u32 = 0x0062_3b90;
/// The effect name the application's `+0x16c` gets (`0x00c16af4`, the
/// `"PS2Lighting"` entry).
pub const PS2_LIGHTING: u32 = 0x00c1_6af4;
/// `Phyre_PRendering_PRendererBase_renderMeshInstance` (thiscall
/// `(renderer, inst, mesh, 0, context)`, `ret 0x10`).
pub const RENDER_MESH_INSTANCE: u32 = 0x005b_9a90;
/// `g_PhyreScene`.
pub const PHYRE_SCENE: u32 = 0x00cc_b9d8;
/// `PhyreScene_addVFXDrawEvent` (thiscall `(scene, &event)`, `ret 4`).
pub const ADD_VFX_DRAW_EVENT: u32 = 0x0065_6940;
/// `graphicCharacterInstanceSetVisible` (cdecl `(inst, on)`).
pub const SET_VISIBLE: u32 = 0x0063_c390;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        ADD_VFX_DRAW_EVENT, COUNT, Conv, GET_APPLICATION, Host, MESHES, PARAMS, PHYRE_SCENE,
        PS2_LIGHTING, RECORD, RECORDS, RENDER_MESH_INSTANCE, SET_VISIBLE, SHADER_PARAM_WRITE,
    };
    use crate::methods::quiet;

    /// `0x006e5a40(mesh)` (cdecl) -> 1 when `mesh` is registered, else 0:
    /// its record's eight parameters into the mesh's materials
    /// (`ShaderParam_Write`); with a key of 0 the application's effect
    /// becomes [`PS2_LIGHTING`] and each of the mesh's instances is
    /// rendered (`renderMeshInstance(app + 0x124; inst, inst +0x10, 0,
    /// app +0x260)`, the application fetched again for each); then the
    /// slot is freed. Returns, with it, the slot index the original
    /// leaves in its argument's slot (8 when not found; `mesh` for null).
    pub fn apply(h: &mut impl Host, mesh: u32) -> (u32, u32) {
        if mesh == 0 {
            return (0, mesh);
        }
        let meshes = h.rebase(MESHES);
        let Some(i) = (0..8).find(|&i| {
            let e: u32 = h.read(meshes + 4 * i);
            e != 0 && e == mesh
        }) else {
            return (0, 8);
        };
        let rec = h.rebase(RECORDS) + RECORD * i;
        for (name, off) in PARAMS {
            let name = h.rebase(name);
            h.call_sym(SHADER_PARAM_WRITE, Conv::Cdecl, &[mesh, name, rec + off]);
        }
        if h.read::<u32>(rec + 0x128) == 0 {
            let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
            h.write(app + 0x16c, h.rebase(PS2_LIGHTING));
            let mut k = 0u32;
            while k < h.read::<u32>(mesh) & 0x7fff_ffff {
                let list: u32 = h.read(mesh + 4);
                let at = list.wrapping_add(4 * k);
                let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
                let context: u32 = h.read(app + 0x260);
                let inst: u32 = h.read(at);
                let m: u32 = h.read(inst + 0x10);
                h.call_sym(
                    RENDER_MESH_INSTANCE,
                    Conv::Thiscall(app + 0x124),
                    &[inst, m, 0, context],
                );
                k += 1;
            }
        }
        let count = h.rebase(COUNT);
        let n: u32 = h.read(count);
        h.write(count, n.wrapping_sub(1));
        h.write(meshes + 4 * i, 0u32);
        (1, i)
    }

    /// The words of a record, as [`register`] takes them: the matrices'
    /// and vectors' addresses, the floats' bits.
    #[derive(Debug, Clone, Copy)]
    pub struct Setup {
        pub m1: u32,
        pub m2: u32,
        pub f1: u32,
        pub m3: u32,
        pub m4: u32,
        pub f2: u32,
        pub v1: u32,
        pub v2: u32,
    }

    fn copy(h: &mut impl Host, dst: u32, src: u32, words: u32) {
        for k in 0..words {
            let w: u32 = h.read(src + 4 * k);
            h.write(dst + 4 * k, w);
        }
    }

    /// `PS2LightingSetup::registerWithPointLightEffect(mesh, m1, m2, f1,
    /// m3, m4, f2, v1, v2, key)` (`0x006e5c90`, cdecl): unless `mesh` is
    /// registered with that key already, or all eight slots are taken,
    /// the first free slot gets `mesh` and a record of the arguments; a
    /// VFX draw event `{3, _, mesh, 0}` (at `ebp - 0x14`; the scene fills
    /// the second word) is queued and the mesh's instance made invisible
    /// (`graphicCharacterInstanceSetVisible(mesh, 0)`, whose `eax` it
    /// returns; on the early returns the first search's cursor: the
    /// matching mesh word, or the end of the table).
    pub fn register(h: &mut impl Host, ebp: u32, mesh: u32, s: &Setup, key: u32) -> u32 {
        let meshes = h.rebase(MESHES);
        let recs = h.rebase(RECORDS);
        for i in 0..8 {
            if h.read::<u32>(meshes + 4 * i) == mesh
                && h.read::<u32>(recs + RECORD * i + 0x128) == key
            {
                // eax: the search's cursor
                return meshes + 4 * i;
            }
        }
        let Some(i) = (0..8).find(|&i| h.read::<u32>(meshes + 4 * i) == 0) else {
            return meshes + 0x20;
        };
        let count = h.rebase(COUNT);
        let n: u32 = h.read(count);
        h.write(count, n.wrapping_add(1));
        h.write(meshes + 4 * i, mesh);
        let rec = recs + RECORD * i;
        copy(h, rec, s.m1, 16);
        copy(h, rec + 0x40, s.m2, 16);
        h.write(rec + 0x80, quiet(s.f1));
        copy(h, rec + 0x84, s.m3, 16);
        copy(h, rec + 0xc4, s.m4, 16);
        h.write(rec + 0x104, quiet(s.f2));
        copy(h, rec + 0x108, s.v1, 4);
        copy(h, rec + 0x118, s.v2, 4);
        h.write(rec + 0x128, key);
        let event = ebp - 0x14;
        h.write(event, 3u32);
        h.write(event + 8, mesh);
        // junk read: [ebp-0x8] never written; 0 (brief 2026-10-24): the
        // event queue copies four words
        h.write(event + 0xc, 0u32);
        let scene: u32 = h.read(h.rebase(PHYRE_SCENE));
        h.call_sym(ADD_VFX_DRAW_EVENT, Conv::Thiscall(scene), &[event]);
        h.call_sym(SET_VISIBLE, Conv::Cdecl, &[mesh, 0])
    }
}

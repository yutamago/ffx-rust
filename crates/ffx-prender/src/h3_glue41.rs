//! The render-side glue, forty-first batch (M4 port round 10, list
//! `h3-glue-render`): the characters' depth pre-pass, the motion-blur
//! frame of the MF152 effect.

use ffx_bridge::Global;

/// `ClassLoadedCharacterTable_GetInstance()`, its map of characters
/// `0x006a2220(table)` (the table itself), the character's material
/// list count `0x006a29a0(character)` (`(+0xc - +8) / 4`, signed) and
/// `renderMeshInstance(renderer; mesh, instance, 0, pass)` (`ret 0x10`).
pub const LOADED_CHARACTER_TABLE: u32 = crate::h3_glue14::LOADED_CHARACTER_TABLE;
pub const CHARACTER_MAP: u32 = 0x006a_2220;
pub const LIST_COUNT: u32 = 0x006a_29a0;
pub const RENDER_MESH_INSTANCE: u32 = crate::h3_glue21::RENDER_MESH_INSTANCE;

/// The passes: `"ZPrePass"` (stored in the renderer's `+0x48`),
/// `"Transparent"` and `"FakeTransparent"` (compared as pointers); the
/// material parameters `"ADDITIONAL_ZPREPASS"` and `"BUILT_IN_ZPREPASS"`
/// and the empty string they are compared with; `g_CurrentMapNo` (map
/// 0x180 draws no pre-pass).
pub const ZPREPASS_PASS: u32 = 0x00c1_6a74;
pub const TRANSPARENT_PASS: u32 = crate::h3_glue21::TRANSPARENT_PASS;
pub const FAKE_TRANSPARENT_PASS: u32 = 0x00c1_6b14;
pub const ADDITIONAL_ZPREPASS: u32 = 0x00b4_25f0;
pub const BUILT_IN_ZPREPASS: u32 = 0x00b4_2604;
pub const EMPTY: u32 = 0x00b8_fbeb;
pub const CURRENT_MAP_NO: Global<u32> = crate::glue5::CURRENT_MAP_NO;

/// The MF152 frame: the render surface's constructor and destructor,
/// `setColorTarget(renderer; surface, 0)`, `setDepthStencilTarget
/// (renderer; surface)`, `setCamera`, `setViewport`, `setClearColor
/// (renderer; 4 floats)`, `beginScene` / `endScene(renderer; 1)`,
/// `GetApplication` and `renderWorld(app; world, camera, pass)`, the
/// world's `removeCluster` / `addCluster(world; cluster)`,
/// `PhyreScene_attachChrClusterTo3DWorld(this)`, `MotionBlur_GetInstance`,
/// its copy `0x006c5570(blur; source, target)`, `setMotionBlurWeight
/// (blur; w)` and `renderMotionBlur(blur; renderer, target)`; the passes
/// `"Opaque"` and `"Transparent"`; the `f64` steps 0.05, 0.45 and 0.01.
pub const SURFACE_CTOR: u32 = crate::h3_glue18::SURFACE_CTOR;
pub const SURFACE_DTOR: u32 = crate::h3_glue18::SURFACE_DTOR;
pub const SET_COLOR_TARGET: u32 = crate::h3_glue18::SET_COLOR_TARGET;
pub const SET_DEPTH_STENCIL_TARGET: u32 = crate::h3_glue37::SET_DEPTH_STENCIL_TARGET;
pub const SET_CAMERA: u32 = crate::h3_glue21::SET_CAMERA;
pub const SET_VIEWPORT: u32 = crate::h3_glue21::SET_VIEWPORT;
pub const SET_CLEAR_COLOR: u32 = crate::h3_glue37::SET_CLEAR_COLOR;
pub const BEGIN_SCENE: u32 = crate::h3_glue18::BEGIN_SCENE;
pub const END_SCENE: u32 = crate::h3_glue18::END_SCENE;
pub const GET_APPLICATION: u32 = crate::h3_glue12::GET_APPLICATION;
pub const RENDER_WORLD: u32 = crate::h3_glue12::RENDER_WORLD;
pub const WORLD_REMOVE_CLUSTER: u32 = crate::glue4::WORLD_REMOVE_CLUSTER;
pub const WORLD_ADD_CLUSTER: u32 = crate::glue4::WORLD_ADD_CLUSTER;
pub const ATTACH_CHR_CLUSTER: u32 = crate::glue9::SCENE_ATTACH_CHR_CLUSTER;
pub const MOTION_BLUR_INSTANCE: u32 = crate::glue2::MOTION_BLUR_INSTANCE;
pub const MOTION_BLUR_COPY: u32 = 0x006c_5570;
pub const MOTION_BLUR_SET_WEIGHT: u32 = crate::glue2::MOTION_BLUR_SET_WEIGHT;
pub const MOTION_BLUR_RENDER: u32 = 0x006c_5410;
pub const OPAQUE_PASS: u32 = crate::h3_glue21::OPAQUE_PASS;
pub const FADE_STEP: u32 = 0x00b4_12e8;
pub const FADE_LOW: u32 = 0x00b4_3d50;
pub const FADE_RISE: u32 = 0x00b1_f2b8;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use crate::glue10::bridge::successor;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// The C string at `a` equals the one at `b`.
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

    /// Whether the tagged string `tagged` (low bit a flag) is the image's
    /// string at `sym`: the same pointer, or equal bytes; a null one never.
    fn is(h: &impl Host, tagged: u32, sym: u32) -> bool {
        let s = tagged.wrapping_sub(tagged & 1);
        let image = h.rebase(sym);
        s == image || (s != 0 && str_eq(h, s, image))
    }

    /// A `PArray`'s elements: inline (`+4`) for one, the pointer `+4` for
    /// more, none for 0 (count `+0`, the top bit a flag).
    fn elements(h: &impl Host, array: u32, count: u32) -> u32 {
        if count > 1 {
            h.read(array + 4)
        } else if count == 0 {
            0
        } else {
            array + 4
        }
    }

    /// The pre-pass flags of a mesh's materials so far: `additional`
    /// (an `ADDITIONAL_ZPREPASS` parameter not `""`) and `built_in`
    /// (likewise `BUILT_IN_ZPREPASS`), both kept from one material to
    /// the next.
    struct Flags {
        additional: bool,
        built_in: bool,
    }

    /// Scans a material's parameters (`[q] + 4` count, `+8` the
    /// name/value pairs) into `flags`.
    fn scan_parameters(h: &impl Host, material: u32, flags: &mut Flags) {
        let block: u32 = h.read(material);
        let count = h.read::<u32>(block + 4) & 0x7fff_ffff;
        let mut entry: u32 = h.read(block + 8);
        for _ in 0..count {
            let name: u32 = h.read(entry);
            let value: u32 = h.read(entry + 4);
            if is(h, name, ADDITIONAL_ZPREPASS) && !is(h, value, EMPTY) {
                flags.additional = true;
            }
            if is(h, name, BUILT_IN_ZPREPASS) && !is(h, value, EMPTY) {
                flags.built_in = true;
            }
            entry = entry.wrapping_add(8);
        }
    }

    /// Whether the mesh `mesh` of list `list` gets the pre-pass: one of
    /// its instance's (`+0x10`) materials, scanned in order, ends with an
    /// additional pre-pass on a fake-transparent material (`+8`, or `+0xc`
    /// when `+8` is "Transparent") of a list with its `+0x14` flag, no
    /// built-in one so far, and the map not 0x180.
    fn wants_prepass(h: &impl Host, list: u32, mesh: u32) -> bool {
        let instance: u32 = h.read(mesh + 0x10);
        let count = h.read::<u32>(instance) & 0x7fff_ffff;
        let mut flags = Flags {
            additional: false,
            built_in: false,
        };
        for p in 0..count {
            let materials = elements(h, instance, count);
            let material: u32 = h.read(materials.wrapping_add(4 * p));
            scan_parameters(h, material, &mut flags);
            if flags.additional {
                let transparent = h.rebase(TRANSPARENT_PASS);
                let fake = h.rebase(FAKE_TRANSPARENT_PASS);
                let first: u32 = h.read(material + 8);
                let pass = if first == transparent {
                    h.read(material + 0xc)
                } else {
                    transparent
                };
                flags.additional = h.read::<u8>(list + 0x14) != 0 && pass == fake;
            }
            if CURRENT_MAP_NO.get(h) != 0x180 && flags.additional && !flags.built_in {
                return true;
            }
        }
        false
    }

    /// `PhyreScene_DrawZPrePass(this)` (`ecx` only): pass `"ZPrePass"`;
    /// for each loaded character (the table's map, in order) with lists
    /// (`+0x28`: `0x006a29a0` > 0, the first list not empty), each list
    /// (`[+8]`), each mesh in it: drawn once when
    /// [`wants_prepass`]. Returns 0.
    pub fn draw_z_prepass(h: &mut impl Host, this: u32) -> u32 {
        let renderer: u32 = h.read(this + 0xd0);
        let pass = h.rebase(ZPREPASS_PASS);
        h.write(renderer + 0x48, pass);
        let table = h.call_sym(LOADED_CHARACTER_TABLE, Conv::Cdecl, &[]);
        let map = h.call_sym(CHARACTER_MAP, Conv::Thiscall(table), &[]);
        let head: u32 = h.read(map);
        let mut node: u32 = h.read(head);
        while node != h.read::<u32>(map) {
            let character: u32 = h.read(node + 0x28);
            if (h.call_sym(LIST_COUNT, Conv::Thiscall(character), &[]) as i32) > 0 {
                let lists: u32 = h.read(character + 8);
                let first: u32 = h.read(lists);
                if h.read::<u32>(first) & 0x7fff_ffff != 0 {
                    let mut j = 0u32;
                    while (j as i32)
                        < (h.call_sym(LIST_COUNT, Conv::Thiscall(character), &[]) as i32)
                    {
                        let lists: u32 = h.read(character + 8);
                        let list: u32 = h.read(lists.wrapping_add(4 * j));
                        let mut k = 0u32;
                        while k < h.read::<u32>(list) & 0x7fff_ffff {
                            let meshes: u32 = h.read(list + 4);
                            let mesh: u32 = h.read(meshes.wrapping_add(4 * k));
                            if wants_prepass(h, list, mesh) {
                                let renderer: u32 = h.read(this + 0xd0);
                                let pass: u32 = h.read(renderer + 0x13c);
                                let instance: u32 = h.read(mesh + 0x10);
                                h.call_sym(
                                    RENDER_MESH_INSTANCE,
                                    Conv::Thiscall(renderer),
                                    &[mesh, instance, 0, pass],
                                );
                            }
                            k += 1;
                        }
                        j += 1;
                    }
                }
            }
            node = successor(h, node);
        }
        0
    }

    /// The `f64` at `sym`.
    fn constant(h: &impl Host, sym: u32) -> f64 {
        let at = h.rebase(sym);
        f64::from_bits(h.read::<u64>(at))
    }

    /// A post-process target's texture: `*target`, or 0 with its byte
    /// `+4` set.
    fn texture(h: &impl Host, target: u32) -> u32 {
        if h.read::<u8>(target + 4) == 0 {
            h.read(target)
        } else {
            0
        }
    }

    /// The effect's blur weight `+0x4f0` (`f32`): down by 0.05 until it
    /// drops below 0.45 (`+0x4f4` set), then up by 0.01, at most 1.
    fn step_weight(h: &mut impl Host, this: u32) {
        if h.read::<u8>(this + 0x4f4) == 0 {
            let v = x87::sub(x87::load(h.read(this + 0x4f0)), constant(h, FADE_STEP));
            h.write(this + 0x4f0, x87::store(v));
        }
        if x87::load(h.read(this + 0x4f0)) < constant(h, FADE_LOW) {
            h.write(this + 0x4f4, 1u8);
        }
        if h.read::<u8>(this + 0x4f4) != 0 {
            let v = x87::add(x87::load(h.read(this + 0x4f0)), constant(h, FADE_RISE));
            h.write(this + 0x4f0, x87::store(v));
        }
        if 1.0 < x87::load(h.read(this + 0x4f0)) {
            h.write(this + 0x4f0, 1.0f32.to_bits());
        }
    }

    /// The camera `+0x394`'s `+4` (0 without one).
    fn camera(h: &impl Host, this: u32) -> u32 {
        let c: u32 = h.read(this + 0x394);
        if c == 0 { 0 } else { c.wrapping_add(4) }
    }

    /// The two surfaces destroyed (depth, then colour); `error`.
    fn release(h: &mut impl Host, ebp: u32, error: u32) -> u32 {
        h.call_sym(SURFACE_DTOR, Conv::Thiscall(ebp - 0x20), &[]);
        h.call_sym(SURFACE_DTOR, Conv::Thiscall(ebp - 0x30), &[]);
        error
    }

    /// `PhyreScene_RenderMF152Related(this)` (`ecx` only; frame: the
    /// colour surface at `ebp - 0x30`, the depth one at `- 0x20`, the
    /// weight's scratch at `- 0x10`). Off (`+0xfaa1` clear): `+0xfaa2`
    /// and `+0x4f4` cleared, 0. Else `+0xfaa2` set, the weight stepped
    /// (`step_weight`), the cluster `+0x11e4` moved out of world `+0x7c`
    /// into world `+0x9c` when `+0xfb44` asks (then cleared); world
    /// `+0x9c` rendered opaque then transparent with camera `+0x394` into
    /// the post-process targets `+0xa8` (colour) and `+0x90` (depth),
    /// cleared to 0; the characters attached; the motion blur copies
    /// `+0xa8` to `+0x8c`, blends with weight `1 - w` into `+0x8c` and
    /// copies `+0x1232c` to `+0x8c`. Returns the scene's error, or 0.
    pub fn render_mf152(h: &mut impl Host, ebp: u32, this: u32) -> u32 {
        if h.read::<u8>(this + 0xfaa1) == 0 {
            h.write(this + 0xfaa2, 0u8);
            h.write(this + 0x4f4, 0u8);
            return 0;
        }
        if h.read::<u8>(this + 0xfaa2) == 0 {
            h.write(this + 0xfaa2, 1u8);
        }
        step_weight(h, this);
        if h.read::<u8>(this + 0xfb44) != 0 {
            let cluster = this + 0x11e4;
            h.call_sym(
                WORLD_REMOVE_CLUSTER,
                Conv::Thiscall(this + 0x7c),
                &[cluster],
            );
            h.call_sym(WORLD_ADD_CLUSTER, Conv::Thiscall(this + 0x9c), &[cluster]);
            h.write(this + 0xfb44, 0u8);
        }
        for (surface, target) in [(ebp - 0x30, 0xa8), (ebp - 0x20, 0x90)] {
            let post: u32 = h.read(this + 0x10cc);
            let t: u32 = h.read(post + target);
            h.call_sym(SURFACE_CTOR, Conv::Thiscall(surface), &[t, 0]);
        }
        let renderer: u32 = h.read(this + 0xd0);
        h.call_sym(SET_COLOR_TARGET, Conv::Thiscall(renderer), &[ebp - 0x30, 0]);
        let renderer: u32 = h.read(this + 0xd0);
        h.call_sym(
            SET_DEPTH_STENCIL_TARGET,
            Conv::Thiscall(renderer),
            &[ebp - 0x20],
        );
        let c = camera(h, this);
        let renderer: u32 = h.read(this + 0xd0);
        h.call_sym(SET_CAMERA, Conv::Thiscall(renderer), &[c]);
        let post: u32 = h.read(this + 0x10cc);
        let colour = texture(h, h.read(post + 0xa8));
        let w: u32 = h.read(colour.wrapping_add(0x1c));
        let ht: u32 = h.read(colour.wrapping_add(0x20));
        let renderer: u32 = h.read(this + 0xd0);
        h.call_sym(SET_VIEWPORT, Conv::Thiscall(renderer), &[0, 0, w, ht]);
        let renderer: u32 = h.read(this + 0xd0);
        h.call_sym(SET_CLEAR_COLOR, Conv::Thiscall(renderer), &[0; 4]);
        let renderer: u32 = h.read(this + 0xd0);
        let error = h.call_sym(BEGIN_SCENE, Conv::Thiscall(renderer), &[1]);
        if error != 0 {
            return release(h, ebp, error);
        }
        for (pass, word) in [(OPAQUE_PASS, 1), (TRANSPARENT_PASS, u32::MAX)] {
            let renderer: u32 = h.read(this + 0xd0);
            let pass = h.rebase(pass);
            h.write(renderer + 0x48, pass);
            let c = camera(h, this);
            let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
            h.call_sym(RENDER_WORLD, Conv::Thiscall(app), &[this + 0x9c, c, word]);
        }
        let renderer: u32 = h.read(this + 0xd0);
        let error = h.call_sym(END_SCENE, Conv::Thiscall(renderer), &[1]);
        if error != 0 {
            return release(h, ebp, error);
        }
        h.call_sym(ATTACH_CHR_CLUSTER, Conv::Thiscall(this), &[]);
        let post: u32 = h.read(this + 0x10cc);
        let target = texture(h, h.read(post + 0x8c));
        let source = texture(h, h.read(post + 0xa8));
        let blur = h.call_sym(MOTION_BLUR_INSTANCE, Conv::Cdecl, &[]);
        h.call_sym(MOTION_BLUR_COPY, Conv::Thiscall(blur), &[source, target]);
        let blur = h.call_sym(MOTION_BLUR_INSTANCE, Conv::Cdecl, &[]);
        h.write(blur + 0x58, 1u16);
        let weight = x87::store(x87::sub(1.0, x87::load(h.read(this + 0x4f0))));
        h.write(ebp - 0x10, weight);
        let blur = h.call_sym(MOTION_BLUR_INSTANCE, Conv::Cdecl, &[]);
        h.call_sym(MOTION_BLUR_SET_WEIGHT, Conv::Thiscall(blur), &[weight]);
        let post: u32 = h.read(this + 0x10cc);
        let target: u32 = h.read(post + 0x8c);
        let renderer: u32 = h.read(this + 0xd0);
        let blur = h.call_sym(MOTION_BLUR_INSTANCE, Conv::Cdecl, &[]);
        h.call_sym(
            MOTION_BLUR_RENDER,
            Conv::Thiscall(blur),
            &[renderer, target],
        );
        let blur = h.call_sym(MOTION_BLUR_INSTANCE, Conv::Cdecl, &[]);
        h.write(blur + 0x58, 0u8);
        let post: u32 = h.read(this + 0x10cc);
        let target = texture(h, h.read(post + 0x8c));
        let source = texture(h, h.read(post + 0x1_232c));
        let blur = h.call_sym(MOTION_BLUR_INSTANCE, Conv::Cdecl, &[]);
        h.call_sym(MOTION_BLUR_COPY, Conv::Thiscall(blur), &[source, target]);
        release(h, ebp, 0)
    }
}

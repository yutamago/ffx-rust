//! The render-side glue, thirty-first batch (M4 port round 8, list
//! `h3-glue-render`): the shadow manager's target clears, a mesh instance
//! from the scene's pool, the scene's shadow update and the shadow-map
//! scale and bias. The exception frames and cookies are left out, as in
//! [`crate::glue10`].

use ffx_bridge::Global;

/// The render surface's constructor and destructor, `setColorTarget(this;
/// surface, slot)` (`ret 8`), `setDepthStencilTarget(this; target)`,
/// `beginScene(this; flags)`, `clearScene(this; flags)`, `endScene(this;
/// flags)` (`ret 4`), `setClearDepth(this; depth)` (`ret 4`) and
/// `setClearColor(this; r, g, b, a)` (`ret 0x10`).
pub const SURFACE_CTOR: u32 = crate::h3_glue18::SURFACE_CTOR;
pub const SURFACE_DTOR: u32 = crate::h3_glue18::SURFACE_DTOR;
pub const SET_COLOR_TARGET: u32 = crate::h3_glue18::SET_COLOR_TARGET;
pub const SET_DEPTH_STENCIL_TARGET: u32 = 0x005b_b5b0;
pub const BEGIN_SCENE: u32 = crate::h3_glue18::BEGIN_SCENE;
pub const CLEAR_SCENE: u32 = 0x005b_6330;
pub const END_SCENE: u32 = crate::h3_glue18::END_SCENE;
pub const SET_CLEAR_DEPTH: u32 = 0x005b_af10;
pub const SET_CLEAR_COLOR: u32 = 0x005b_ada0;
/// The shadow manager's targets cleared, with their surfaces' frame
/// offsets (below `ebp`).
pub const SHADOW_TARGETS: [(u32, u32); 5] = [
    (0x60, 0x5c),
    (0x64, 0x4c),
    (0x6c, 0x3c),
    (0x70, 0x2c),
    (0x68, 0x1c),
];

/// `allocateInstanceList(cluster; 1, type, 0)`, `PCluster::create(cluster;
/// 1, type, 0)` (`ret 0xc`), the instance and bounds types, the
/// `PMeshInstance` constructor and the bounds link (`ret 4`), the matrix
/// type, `PMatrix4_Identity_00451bb0(out)` (cdecl, returns `out`) and
/// `PMatrix4_toWorldMatrix3x4(this; m)` (`ret 4`).
pub const ALLOCATE_INSTANCE_LIST: u32 = crate::h3_glue30::ALLOCATE_INSTANCE_LIST;
pub const CLUSTER_CREATE: u32 = crate::h3_glue30::CLUSTER_CREATE;
pub const INSTANCE_TYPE: u32 = crate::h3_glue30::INSTANCE_TYPE;
pub const BOUNDS_TYPE: u32 = crate::h3_glue30::BOUNDS_TYPE;
pub const INSTANCE_CTOR: u32 = crate::h3_glue30::INSTANCE_CTOR;
pub const BOUNDS_LINK: u32 = crate::h3_glue30::BOUNDS_LINK;
pub const MATRIX_TYPE: u32 = crate::h3_glue30::MATRIX_TYPE;
pub const IDENTITY: u32 = 0x0045_1bb0;
pub const TO_WORLD: u32 = 0x0045_1190;
/// The scene's mesh pool: 3000 slots.
pub const POOL_SLOTS: u32 = 3000;

/// `ShadowManager_getInstance`, `MsBattleCheck`'s thunk (cdecl, no
/// arguments), the caster visibility (`ret 4`), the cascades (`ret 0x10`),
/// the two shadow passes `0x006c9e50` and `0x006caee0` (`ret 0xc`), the
/// cascade matrix (`ret 8`), the caster average (`ST0`), the dummy map,
/// a cascade (`ret 4`), the scene's linear sampler (`ret 4`), the
/// parameter copies (`ret 8`) and the checked texture set (`ret 8`).
pub const SHADOW_INSTANCE: u32 = crate::h3_glue18::SHADOW_INSTANCE;
pub const BATTLE_CHECK: u32 = 0x0063_8e60;
pub const UPDATE_CASTER_VISIBILITY: u32 = 0x006c_e6f0;
pub const UPDATE_CASCADES: u32 = 0x006c_e860;
pub const SHADOW_PASS_9E50: u32 = 0x006c_9e50;
pub const SHADOW_PASS_AEE0: u32 = 0x006c_aee0;
pub const CASCADE_MATRIX: u32 = 0x006d_23a0;
pub const AVERAGE_CASTER_VALUE: u32 = 0x006c_cad0;
pub const DUMMY_SHADOW_MAP: u32 = 0x006d_2380;
pub const CASCADE: u32 = 0x006d_22e0;
pub const LINEAR_SAMPLER: u32 = 0x0065_e050;
pub const COPY_TO_BUFFER: u32 = 0x0063_8720;
pub const SET_VALUE2: u32 = 0x0063_86c0;
pub const SET_TEXTURE_CHECKED: u32 = 0x0056_e670;

/// The scene cursor's set-up over the map objects (`0x0064ce60(this;
/// owner)`, `ret 4`) and its step (`0x0064f1a0(this; out, 0)`, `ret 8`),
/// `ShaderParams_FindByName(shader; name)` (`ret 4`),
/// `ShadowManager_GetShadowSize` (cdecl), `ShaderParamBuffer_SetValue`
/// (`ret 8`), the two parameter names and the doubles 64 and 32.
pub const CURSOR_CTOR: u32 = 0x0064_ce60;
pub const CURSOR_STEP: u32 = 0x0064_f1a0;
pub const FIND_PARAMETER: u32 = 0x0056_cbf0;
pub const SHADOW_SIZE: u32 = 0x006c_cd00;
pub const SET_VALUE: u32 = 0x0063_8780;
pub const SCALE_BIAS_NAMES: [u32; 2] = [0x00b4_2520, 0x00b4_2530];
pub const SIXTY_FOUR: Global<f64> = Global::new(0x00b4_1308);
pub const THIRTY_TWO: Global<f64> = Global::new(0x00b4_12f8);

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// `0x006ce130(this, renderer)` (thiscall, `ret 4`; frame: five render
    /// surfaces at `ebp - 0x5c` .. `- 0x1c`): clear depth 1 and colour 0;
    /// each of the shadow targets `+0x60`, `+0x64`, `+0x6c`, `+0x70`,
    /// `+0x68` in turn gets a surface, is bound as the colour target with
    /// no depth target, and has a scene begun (0), cleared and ended (1);
    /// then the colour target is unbound. The first error stops it; the
    /// surfaces made are destroyed in reverse. `eax`: the error, or 0.
    pub fn shadow_clear_targets(h: &mut impl Host, ebp: u32, this: u32, renderer: u32) -> u32 {
        let r = Conv::Thiscall(renderer);
        h.call_sym(SET_CLEAR_DEPTH, r, &[1f32.to_bits()]);
        h.call_sym(SET_CLEAR_COLOR, r, &[0; 4]);
        let mut made = vec![];
        let mut error = 0;
        for (target, at) in SHADOW_TARGETS {
            let texture: u32 = h.read(this + target);
            let surface = ebp - at;
            h.call_sym(SURFACE_CTOR, Conv::Thiscall(surface), &[texture, 0]);
            made.push(surface);
            error = h.call_sym(SET_COLOR_TARGET, r, &[surface, 0]);
            if error != 0 {
                break;
            }
            error = h.call_sym(SET_DEPTH_STENCIL_TARGET, r, &[0]);
            if error != 0 {
                break;
            }
            error = h.call_sym(BEGIN_SCENE, r, &[0]);
            if error != 0 {
                break;
            }
            h.call_sym(CLEAR_SCENE, r, &[1]);
            error = h.call_sym(END_SCENE, r, &[1]);
            if error != 0 {
                break;
            }
        }
        if error == 0 {
            error = h.call_sym(SET_COLOR_TARGET, r, &[0, 0]);
        }
        for surface in made.into_iter().rev() {
            h.call_sym(SURFACE_DTOR, Conv::Thiscall(surface), &[]);
        }
        error
    }

    /// The bounds of `instance`: linked into the bounds object `bounds`;
    /// the bounds object, or 0.
    fn link_bounds(h: &mut impl Host, bounds: u32, instance: u32) -> u32 {
        h.call_sym(BOUNDS_LINK, Conv::Thiscall(bounds), &[instance])
    }

    /// `0x006599c0(this, mesh, cluster; slots)` (thiscall, `ret 8`; frame:
    /// an identity matrix at `ebp - 0x50`): a mesh instance of `mesh`
    /// with bounds and a matrix. For the scene's own cluster (`+0x12d4`)
    /// while the pool (`+0xfbe4`, signed) has a free slot, the pool's
    /// instance (`+0xfbcc`, 0x84 bytes), bounds (`+0xfbd0`, 0x24) and
    /// matrix (`+0xfbd4`, 0x30) are used, the slot is marked in `+0xfbe0`
    /// and the index moves to the next free one (3000: none); else all
    /// three come from `cluster`. Returns the instance (0: none); the
    /// pool path writes over `mesh`'s and `cluster`'s slots.
    pub fn scene_new_mesh_instance(
        h: &mut impl Host,
        ebp: u32,
        slots: u32,
        this: u32,
        mesh: u32,
        cluster: u32,
    ) -> u32 {
        let index: u32 = h.read(this + 0xfbe4);
        if cluster != this + 0x12d4 || index as i32 >= POOL_SLOTS as i32 {
            return from_cluster(h, ebp, slots, mesh, cluster);
        }
        let slot = index.wrapping_mul(0x84).wrapping_add(h.read(this + 0xfbcc));
        h.write(slots + 4, slot);
        let instance = if slot == 0 {
            0
        } else {
            pooled(h, ebp, slots, this, mesh, slot)
        };
        let index: u32 = h.read(this + 0xfbe4);
        let used: u32 = h.read(this + 0xfbe0);
        h.write(used.wrapping_add(index), 1u8);
        let mut next = h.read::<u32>(this + 0xfbe4).wrapping_add(1);
        if (next as i32) < POOL_SLOTS as i32 {
            let used: u32 = h.read(this + 0xfbe0);
            while (next as i32) < POOL_SLOTS as i32 {
                if h.read::<u8>(used.wrapping_add(next)) == 0 {
                    h.write(this + 0xfbe4, next);
                    break;
                }
                next = next.wrapping_add(1);
            }
        }
        if next == POOL_SLOTS {
            h.write(this + 0xfbe4, POOL_SLOTS);
        }
        instance
    }

    /// The pool path's instance in `slot`, its bounds and matrix.
    fn pooled(h: &mut impl Host, ebp: u32, slots: u32, this: u32, mesh: u32, slot: u32) -> u32 {
        h.write(ebp - 0x10, slot);
        let instance = h.call_sym(INSTANCE_CTOR, Conv::Thiscall(slot), &[mesh]);
        if instance == 0 {
            return 0;
        }
        let index: u32 = h.read(this + 0xfbe4);
        let bounds = index.wrapping_mul(0x24).wrapping_add(h.read(this + 0xfbd0));
        h.write(slots, bounds);
        if bounds == 0 {
            return instance;
        }
        h.write(slots + 4, bounds);
        let linked = link_bounds(h, bounds, instance);
        h.write(slots, linked);
        if linked == 0 {
            return instance;
        }
        let index: u32 = h.read(this + 0xfbe4);
        let matrix = index.wrapping_mul(0x30).wrapping_add(h.read(this + 0xfbd4));
        let m = h.call_sym(IDENTITY, Conv::Cdecl, &[ebp - 0x50]);
        h.call_sym(TO_WORLD, Conv::Thiscall(matrix), &[m]);
        h.write(instance + 4, matrix);
        h.write(linked + 0xc, matrix);
        instance
    }

    /// The other path: instance, bounds and matrix from `cluster`.
    fn from_cluster(h: &mut impl Host, ebp: u32, slots: u32, mesh: u32, cluster: u32) -> u32 {
        let c = Conv::Thiscall(cluster);
        let ty = h.rebase(INSTANCE_TYPE);
        let list = h.call_sym(ALLOCATE_INSTANCE_LIST, c, &[1, ty, 0]);
        if list == 0 {
            return 0;
        }
        let at: u32 = h.read(list + 0x34);
        h.write(slots + 4, at);
        if at == 0 {
            return 0;
        }
        h.write(ebp - 0x10, at);
        let instance = h.call_sym(INSTANCE_CTOR, Conv::Thiscall(at), &[mesh]);
        if instance == 0 {
            return 0;
        }
        let ty = h.rebase(BOUNDS_TYPE);
        let list = h.call_sym(ALLOCATE_INSTANCE_LIST, c, &[1, ty, 0]);
        if list == 0 {
            return instance;
        }
        let bounds: u32 = h.read(list + 0x34);
        h.write(slots, bounds);
        if bounds == 0 {
            return instance;
        }
        h.write(slots + 4, bounds);
        let linked = link_bounds(h, bounds, instance);
        if linked == 0 {
            return instance;
        }
        let ty = h.rebase(MATRIX_TYPE);
        let matrix = h.call_sym(CLUSTER_CREATE, c, &[1, ty, 0]);
        for k in 0..16u32 {
            let v = if k % 5 == 0 { 1f32 } else { 0f32 };
            h.write(ebp - 0x50 + 4 * k, v.to_bits());
        }
        h.call_sym(TO_WORLD, Conv::Thiscall(matrix), &[ebp - 0x50]);
        h.write(instance + 4, matrix);
        h.write(linked + 0xc, matrix);
        instance
    }

    /// The scene's camera for the shadow manager: `+0xdc`'s `+4`, or 0.
    fn camera(h: &impl Host, this: u32) -> u32 {
        let c: u32 = h.read(this + 0xdc);
        if c == 0 { 0 } else { c.wrapping_add(4) }
    }

    /// `MsBattleCheck() != 0`.
    fn in_battle(h: &mut impl Host) -> u32 {
        u32::from(h.call_sym(BATTLE_CHECK, Conv::Cdecl, &[]) != 0)
    }

    /// The shadow manager.
    fn shadows(h: &mut impl Host) -> u32 {
        h.call_sym(SHADOW_INSTANCE, Conv::Cdecl, &[])
    }

    /// `0x00653600(this)` (thiscall; frame: the cascade matrix at `ebp -
    /// 0x64`, two texture records at `- 0x24` and `- 0x14`, the caster
    /// value at `- 4`): by the scene's shadow mode (`+0xf9cc`): 2, 3 and
    /// 4 update the casters' visibility from the camera, the cascades and
    /// the pass `0x006c9e50`, then (map loaded, `+0x10d4`, and not loading,
    /// `+0x1195`) fill each shadow material's parameters (`+0x38c4`
    /// entries: the cascade matrices `+0x38a8`/`+0x38b0`, the caster value
    /// `+0x38b8`, and for mode 4 the shadow texture `+0x38c0`: the dummy
    /// map in battle, else cascade 1, with the linear sampler); 0 and 1
    /// only the casters' visibility. Every mode but 4 then runs the pass
    /// `0x006caee0`; other modes do nothing. Returns 0.
    pub fn scene_update_shadows(h: &mut impl Host, ebp: u32, this: u32) -> u32 {
        let mode: u32 = h.read(this + 0xf9cc);
        match mode {
            2..=4 => {
                let cam = camera(h, this);
                let sh = shadows(h);
                h.call_sym(UPDATE_CASTER_VISIBILITY, Conv::Thiscall(sh), &[cam]);
                let cam = camera(h, this);
                let map: u32 = h.read(this + 0xfaf4);
                let mode: u32 = h.read(this + 0xf9cc);
                let battle = in_battle(h);
                let sh = shadows(h);
                h.call_sym(
                    UPDATE_CASCADES,
                    Conv::Thiscall(sh),
                    &[battle, mode, cam, map],
                );
                let battle = in_battle(h);
                let mode: u32 = h.read(this + 0xf9cc);
                let renderer: u32 = h.read(this + 0xd0);
                let sh = shadows(h);
                h.call_sym(
                    SHADOW_PASS_9E50,
                    Conv::Thiscall(sh),
                    &[renderer, mode, battle],
                );
                if h.read::<u8>(this + 0x1195) == 0 && h.read::<u32>(this + 0x10d4) != 0 {
                    materials(h, ebp, this);
                }
                if h.read::<u32>(this + 0xf9cc) == 4 {
                    return 0;
                }
            }
            0 | 1 => {
                let cam = camera(h, this);
                let sh = shadows(h);
                h.call_sym(UPDATE_CASTER_VISIBILITY, Conv::Thiscall(sh), &[cam]);
            }
            _ => return 0,
        }
        let battle = in_battle(h);
        let mode: u32 = h.read(this + 0xf9cc);
        let renderer: u32 = h.read(this + 0xd0);
        let sh = shadows(h);
        h.call_sym(
            SHADOW_PASS_AEE0,
            Conv::Thiscall(sh),
            &[renderer, mode, battle],
        );
        0
    }

    /// The `i`th word of the array at `this + list`.
    fn entry(h: &impl Host, this: u32, list: u32, i: u32) -> u32 {
        let base: u32 = h.read(this + list);
        h.read(base.wrapping_add(i.wrapping_mul(4)))
    }

    /// The `i`th material's parameter buffer (`+0x38c8`'s entry, `+4`).
    fn buffer(h: &impl Host, this: u32, i: u32) -> u32 {
        let material = entry(h, this, 0x38c8, i);
        h.read(material.wrapping_add(4))
    }

    /// The shadow materials' parameters (see [`scene_update_shadows`]).
    fn materials(h: &mut impl Host, ebp: u32, this: u32) {
        let mut i = 0u32;
        while i < h.read::<u32>(this + 0x38c4) & 0x7fff_ffff {
            for (list, cascade) in [(0x38a8, 0), (0x38b0, 1)] {
                if entry(h, this, list, i) != 0 {
                    let sh = shadows(h);
                    h.call_sym(CASCADE_MATRIX, Conv::Thiscall(sh), &[ebp - 0x64, cascade]);
                    let param = entry(h, this, list, i);
                    let buf = buffer(h, this, i);
                    h.call_sym(COPY_TO_BUFFER, Conv::Thiscall(buf), &[param, ebp - 0x64]);
                }
            }
            if entry(h, this, 0x38b8, i) != 0 {
                let sh = shadows(h);
                let v = h.call_sym_st0_f64(AVERAGE_CASTER_VALUE, Conv::Thiscall(sh), &[]);
                h.write(ebp - 4, x87::store(v));
                let param = entry(h, this, 0x38b8, i);
                let buf = buffer(h, this, i);
                h.call_sym(SET_VALUE2, Conv::Thiscall(buf), &[param, ebp - 4]);
            }
            if h.read::<u32>(this + 0xf9cc) == 4 && entry(h, this, 0x38c0, i) != 0 {
                let record = if in_battle(h) != 0 {
                    h.write(ebp - 0x14, 0x42u32);
                    let sh = shadows(h);
                    let map = h.call_sym(DUMMY_SHADOW_MAP, Conv::Thiscall(sh), &[]);
                    texture_record(h, this, ebp - 0x14, map);
                    ebp - 0x14
                } else {
                    h.write(ebp - 0x24, 0x42u32);
                    let sh = shadows(h);
                    let map = h.call_sym(CASCADE, Conv::Thiscall(sh), &[1]);
                    texture_record(h, this, ebp - 0x24, map);
                    ebp - 0x24
                };
                let param = entry(h, this, 0x38c0, i);
                let buf = buffer(h, this, i);
                h.call_sym(SET_TEXTURE_CHECKED, Conv::Thiscall(buf), &[param, record]);
            }
            i = i.wrapping_add(1);
        }
    }

    /// A texture record `{0x42, 0, sampler, texture}` at `record` (its type
    /// word already stored): the shadow map `map`'s texture (`+0`, none
    /// when its `+4` byte is set) and the scene's linear sampler 2.
    fn texture_record(h: &mut impl Host, this: u32, record: u32, map: u32) {
        let texture = if h.read::<u8>(map.wrapping_add(4)) == 0 {
            h.read(map)
        } else {
            0
        };
        h.write(record + 0xc, texture);
        let sampler = h.call_sym(LINEAR_SAMPLER, Conv::Thiscall(this), &[2]);
        h.write(record + 8, sampler);
        h.write(record + 4, 0u32);
    }

    /// The shadow size as `fild` / `fstp dword` leave it: `f32` bits,
    /// with the integer itself stored first at `at`.
    fn size_at(h: &mut impl Host, at: u32) -> f64 {
        let size = h.call_sym(SHADOW_SIZE, Conv::Cdecl, &[]);
        h.write(at, size);
        #[allow(clippy::cast_possible_wrap, clippy::cast_precision_loss)] // fild
        let f = (size as i32) as f32;
        h.write(at, f.to_bits());
        f64::from(f)
    }

    /// One `MySMScaleBias` record `{(s - 64) / s, (s - 64) / s, 32 / s,
    /// 32 / s}` at `record` (six shadow-size calls, each value through
    /// the scratch word `int`, the two differences through the qwords
    /// `diffs`, the three middle results at `outs` before they are
    /// copied; the last size goes through `last`), handed to `SetValue`.
    fn scale_bias(
        h: &mut impl Host,
        buffer: u32,
        param: u32,
        record: u32,
        [int, last]: [u32; 2],
        diffs: [u32; 2],
        outs: [u32; 3],
    ) {
        let sixty_four = SIXTY_FOUR.get(h);
        let thirty_two = THIRTY_TWO.get(h);
        let mut parts = [0u32; 3];
        for (k, diff) in diffs.into_iter().enumerate() {
            let s = size_at(h, int);
            let d = x87::sub(s, sixty_four);
            h.write(diff, d.to_bits());
            let s = size_at(h, int);
            let q = x87::store(x87::div(h.read::<f64>(diff), s));
            h.write(outs[k], q);
            parts[k] = q;
        }
        let s = size_at(h, int);
        let w = x87::store(x87::div(thirty_two, s));
        h.write(outs[2], w);
        parts[2] = w;
        for (k, v) in parts.into_iter().enumerate() {
            let v = x87::store(x87::load(v));
            h.write(record + 4 * k as u32, v);
        }
        let s = size_at(h, last);
        let w = x87::store(x87::div(thirty_two, s));
        h.write(record + 0xc, w);
        h.call_sym(SET_VALUE, Conv::Thiscall(buffer), &[param, record]);
    }

    /// `0x006542b0(this)` (thiscall; frame: the cursor at `ebp - 0x58`,
    /// its step's copy at `- 0x78`, the records at `- 0x38` and `- 0x28`
    /// and their scratch above): with shadows on (`+0x10d1`), a map
    /// (`+0x10d4`), no load running (`+0x1195`) and the shadow manager's
    /// update flag (`+0x4064`) set, each map object's shader gets its
    /// `MySMScaleBias` and `MySMScaleBias2` parameters from the shadow
    /// size; the flag is then cleared.
    pub fn scene_set_shadow_scale_bias(h: &mut impl Host, ebp: u32, this: u32) {
        if h.read::<u8>(this + 0x10d1) == 0
            || h.read::<u32>(this + 0x10d4) == 0
            || h.read::<u8>(this + 0x1195) != 0
        {
            return;
        }
        let sh = shadows(h);
        if h.read::<u8>(sh.wrapping_add(0x4064)) == 0 {
            return;
        }
        let map: u32 = h.read(this + 0x10d4);
        let cursor = ebp - 0x58;
        h.call_sym(CURSOR_CTOR, Conv::Thiscall(cursor), &[map]);
        while h.read::<u32>(cursor + 0xc) != 0 {
            let offset: u32 = h.read(cursor + 0x10);
            let base: u32 = h.read(cursor);
            let object = base.wrapping_add(offset);
            let [first, second] = SCALE_BIAS_NAMES.map(|n| h.rebase(n));
            let shader: u32 = h.read(object);
            let param = h.call_sym(FIND_PARAMETER, Conv::Thiscall(shader), &[first]);
            if param != 0 {
                let buffer: u32 = h.read(object.wrapping_add(4));
                scale_bias(
                    h,
                    buffer,
                    param,
                    ebp - 0x28,
                    [ebp - 4, ebp - 0x14],
                    [ebp - 0x18, ebp - 0x18],
                    [ebp - 0xc, ebp - 0x14, ebp - 4],
                );
            }
            let shader: u32 = h.read(object);
            let param = h.call_sym(FIND_PARAMETER, Conv::Thiscall(shader), &[second]);
            if param != 0 {
                let buffer: u32 = h.read(object.wrapping_add(4));
                scale_bias(
                    h,
                    buffer,
                    param,
                    ebp - 0x38,
                    [ebp - 0x14, ebp - 0x14],
                    [ebp - 0x10, ebp - 8],
                    [ebp - 0xc, ebp - 4, ebp - 0x14],
                );
            }
            h.call_sym(CURSOR_STEP, Conv::Thiscall(cursor), &[ebp - 0x78, 0]);
        }
        let sh = shadows(h);
        h.write(sh.wrapping_add(0x4064), 0u8);
    }
}

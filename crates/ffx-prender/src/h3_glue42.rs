//! The render-side glue, forty-second batch (M4 port round 10, list
//! `h3-glue-render`): the post-process manager's screen-fade mesh, the
//! shadow manager's caster lists. The exception frame and cookie are
//! left out, as in [`crate::glue10`].

/// `PostProcessManager_lookup_006935f0(this; i)` (`ret 4`), the
/// cluster's `Phyre_PCluster_create(cluster; n, class, 0)` and
/// `allocateInstanceList(cluster; n, class, 0)` (`ret 0xc`), the heap's
/// `PhyreMalloc(size, align)` and `PhyreFree(p)` (cdecl), the segment
/// entries' construct / destroy `(data, n)` (cdecl), the vertex
/// streams' constructor `0x0055dc40(this)`, `PArray40_setBuffer(array;
/// data, n)` (`ret 8`), the vertex buffer's set-up `0x004835d0(vb; 0xc,
/// 8, 1)` (`ret 0xc`), `isBound(vb)`, `release(vb)`, the map
/// `0x00434c50(vb; &out)` (`ret 4`) and unmap `0x00564cb0(vb; n, &out)`
/// (`ret 8`), `PMeshInstance_ctor(instance; array)` (`ret 4`),
/// `createMaterial(variant; cluster, 0, 0)` (`ret 0xc`),
/// `ShaderParams_FindByName(shader; name)` (`ret 4`) and `memcpy`.
pub const POST_LOOKUP: u32 = 0x0069_35f0;
pub const CLUSTER_CREATE: u32 = 0x0044_1010;
pub const ALLOCATE_INSTANCE_LIST: u32 = 0x0044_0b70;
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
pub const PHYRE_FREE: u32 = 0x0042_fb20;
pub const ENTRIES_CONSTRUCT: u32 = 0x0043_4430;
pub const ENTRIES_DESTROY: u32 = 0x0043_4210;
pub const STREAMS_CTOR: u32 = 0x0055_dc40;
pub const SET_BUFFER40: u32 = 0x0043_4e40;
pub const VB_SETUP: u32 = 0x0048_35d0;
pub const VB_IS_BOUND: u32 = 0x0056_2da0;
pub const VB_RELEASE: u32 = 0x0056_46d0;
pub const VB_MAP: u32 = 0x0043_4c50;
pub const VB_UNMAP: u32 = 0x0056_4cb0;
pub const MESH_INSTANCE_CTOR: u32 = 0x004f_27e0;
pub const CREATE_MATERIAL: u32 = 0x004e_6a40;
pub const FIND_PARAMETER: u32 = crate::h3_glue37::FIND_PARAMETER;
pub const MEMCPY: u32 = crate::glue5::MEMCPY;

/// The classes created (mesh segments, mesh instances, materials), the
/// vertex format word and the parameter's name `"ScreenFadeColor"`; the
/// quad's `f32` constants -0.75, -1, -0.9999 and 0.75.
pub const SEGMENT_CLASS: u32 = 0x00c9_9c80;
pub const INSTANCE_CLASS: u32 = 0x00ca_7228;
pub const MATERIAL_CLASS: u32 = 0x00c9_98e8;
pub const VERTEX_FORMAT: u32 = 0x00c9_85b4;
pub const FADE_COLOUR_NAME: u32 = 0x00b4_7a38;
pub const QUAD_A: u32 = 0x00b4_7c9c;
pub const QUAD_B: u32 = 0x00b9_2340;
pub const QUAD_Z: u32 = 0x00b4_7ca0;
pub const QUAD_D: u32 = 0x00b4_3d18;

/// The caster lists: `PArray_PushInto_006d26d0(array; storage, &item)`
/// (`ret 8`), the box tests `BoundingBox_IsVisible_00663a30` and
/// `BoundingBox_IsInside_006d27e0(box; matrix)` (`ret 4`, `al`), the
/// matrices' `PMatrix4_FromMatrix3x4_00451080(out; m)` (`ret 4`) and
/// `PMatrix4_Multiply_004ca360(out, a, b)` (cdecl), and the size limit
/// (`f32`) above which a caster is tested whole.
pub const PUSH_INTO: u32 = 0x006d_26d0;
pub const BOX_IS_VISIBLE: u32 = crate::h3_glue40::BOX_IS_VISIBLE;
pub const BOX_IS_INSIDE: u32 = 0x006d_27e0;
pub const MATRIX_FROM_3X4: u32 = crate::h3_glue21::MATRIX_FROM_3X4;
pub const MATRIX_MULTIPLY: u32 = crate::h3_glue21::MATRIX_MULTIPLY;
pub const CASTER_SIZE_LIMIT: u32 = 0x00c3_4404;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// A `PArray`'s elements: the pointer `+4` for more than one, inline
    /// (`+4`) for one, none for 0 (count `+0`, the top bit "borrowed").
    fn elements(h: &impl Host, array: u32, count: u32) -> u32 {
        if count > 1 {
            h.read(array + 4)
        } else if count == 0 {
            0
        } else {
            array + 4
        }
    }

    /// `PCluster_create(this + 0x3c; 1, class, 0)`.
    fn create(h: &mut impl Host, this: u32, class: u32) -> u32 {
        let class = h.rebase(class);
        h.call_sym(CLUSTER_CREATE, Conv::Thiscall(this + 0x3c), &[1, class, 0])
    }

    /// The segment array `segments` gets one fresh 0x6c-byte entry when
    /// it does not hold exactly one: the old storage destroyed unless
    /// borrowed (top bit) or the same.
    fn one_segment(h: &mut impl Host, segments: u32) {
        if h.read::<u32>(segments) == 1 {
            return;
        }
        let entry = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[0x6c, 4]);
        if entry == 0 {
            return;
        }
        h.call_sym(ENTRIES_CONSTRUCT, Conv::Cdecl, &[entry, 1]);
        let word: u32 = h.read(segments);
        let old: u32 = h.read(segments + 4);
        if old != entry && word >> 31 == 0 {
            h.call_sym(ENTRIES_DESTROY, Conv::Cdecl, &[old, word & 0x7fff_ffff]);
        }
        h.write(segments + 4, entry);
        h.write(segments, 1u32);
    }

    /// The quad's 8 vertices (x, y, z), as the original's x87 stack
    /// stores them.
    fn quad(h: &impl Host) -> [u32; 24] {
        let c = |sym: u32| x87::store(x87::load(h.read(h.rebase(sym))));
        let (a, b, z, d) = (c(QUAD_A), c(QUAD_B), c(QUAD_Z), c(QUAD_D));
        let one = 1.0f32.to_bits();
        [
            a, b, z, a, one, z, d, b, z, d, one, z, a, a, z, d, a, z, a, d, z, d, d, z,
        ]
    }

    /// The vertex buffer `vb`: 12-byte vertices, 8 of them, the quad
    /// written through a mapping (`ebp - 0x30` gets its pointer) and
    /// unmapped at the next frame index.
    fn fill_quad(h: &mut impl Host, ebp: u32, vb: u32) {
        h.call_sym(VB_SETUP, Conv::Thiscall(vb), &[0xc, 8, 1]);
        let size = h.read::<u32>(vb).wrapping_mul(h.read(vb + 4));
        if h.read::<u32>(vb + 0x38) != size {
            if h.call_sym(VB_IS_BOUND, Conv::Thiscall(vb), &[]) as u8 != 0 {
                h.call_sym(VB_RELEASE, Conv::Thiscall(vb), &[]);
            }
            h.write(vb + 0x38, size);
        }
        let format: u32 = h.read(vb + 0xc);
        h.write(format + 8, 2u8);
        let name = h.rebase(VERTEX_FORMAT);
        h.write(format + 4, name);
        h.write(format, 0u32);
        h.call_sym(VB_MAP, Conv::Thiscall(vb), &[ebp - 0x30]);
        let at: u32 = h.read(ebp - 0x30);
        for (k, v) in quad(h).into_iter().enumerate() {
            h.write(at.wrapping_add(4 * k as u32), v);
        }
        let next = h.read::<u32>(vb + 0x10).wrapping_add(1);
        let index = if next == h.read::<u32>(vb + 0x18) & 0x7fff_ffff {
            0
        } else {
            next
        };
        if h.call_sym(VB_UNMAP, Conv::Thiscall(vb), &[index, ebp - 0x30]) == 0 {
            h.write(vb + 0x10, index);
            h.write(vb + 0x14, 0u8);
        }
    }

    /// `0x006931a0(this)` (`ecx` only; frame: the fade colour record at
    /// `ebp - 0x40`, the mapped vertices' pointer at `- 0x30`, the
    /// entries' and instance's words at `- 0x28` .. `- 0x14`). The
    /// variant `lookup(5)` (none: 0x13); a mesh with one segment (one
    /// stream of 0x40 bytes, the quad in its vertex buffer `+0x2c`), its
    /// instance into `+0x123ac` (0 when none is allocated), the variant's
    /// material as the instance's only material (`+0x10`); the material's
    /// `"ScreenFadeColor"` set to (0, 1, 0, 1). Returns 0, 3 when the
    /// parameter does not fit its buffer, 0x13 without the variant, the
    /// material or the parameter.
    pub fn post_screen_fade_mesh(h: &mut impl Host, ebp: u32, this: u32) -> u32 {
        let variant = h.call_sym(POST_LOOKUP, Conv::Thiscall(this), &[5]);
        h.write(ebp - 0x20, variant);
        if variant == 0 {
            return 0x13;
        }
        let segments = create(h, this, SEGMENT_CLASS);
        one_segment(h, segments);
        let segment: u32 = h.read(segments + 4);
        h.write(ebp - 0x1c, segment);
        h.write(ebp - 0x18, segment + 0x28);
        h.write(segment + 0x10, 1u32);
        if h.read::<u32>(segment + 0x28) != 1 {
            let streams = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[0x40, 4]);
            h.write(ebp - 0x24, streams);
            if streams != 0 {
                h.write(ebp - 0x14, 0u32);
                h.write(ebp - 0x28, streams);
                h.call_sym(STREAMS_CTOR, Conv::Thiscall(streams), &[]);
                h.write(ebp - 0x14, 1u32);
                h.call_sym(SET_BUFFER40, Conv::Thiscall(segment + 0x28), &[streams, 1]);
            }
        }
        let vb: u32 = h.read(segment + 0x2c);
        fill_quad(h, ebp, vb);
        let class = h.rebase(INSTANCE_CLASS);
        let list = h.call_sym(
            ALLOCATE_INSTANCE_LIST,
            Conv::Thiscall(this + 0x3c),
            &[1, class, 0],
        );
        let mut instance = 0;
        if list != 0 {
            let at: u32 = h.read(list + 0x34);
            h.write(ebp - 0x28, at);
            if at != 0 {
                h.write(ebp - 0x24, at);
                instance = h.call_sym(MESH_INSTANCE_CTOR, Conv::Thiscall(at), &[segments]);
            }
        }
        h.write(this + 0x1_23ac, instance);
        let material = h.call_sym(
            CREATE_MATERIAL,
            Conv::Thiscall(variant),
            &[this + 0x3c, 0, 0],
        );
        let materials = create(h, this, MATERIAL_CLASS);
        let word: u32 = h.read(materials);
        if word != 1 {
            let old = elements(h, materials, word & 0x7fff_ffff);
            let inline = materials.wrapping_add(4);
            if inline != 0 {
                h.write(inline, 0u32);
                if old != inline && word >> 31 == 0 {
                    h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
                }
                h.write(materials, 1u32);
            }
        }
        let count = h.read::<u32>(materials) & 0x7fff_ffff;
        if count != 0 {
            let first = elements(h, materials, count);
            h.write(first, material);
        }
        let instance: u32 = h.read(this + 0x1_23ac);
        h.write(instance + 0x10, materials);
        if material == 0 {
            return 0x13;
        }
        let shader: u32 = h.read(material);
        let name = h.rebase(FADE_COLOUR_NAME);
        let parameter = h.call_sym(FIND_PARAMETER, Conv::Thiscall(shader), &[name]);
        if parameter == 0 {
            return 0x13;
        }
        let offset = u32::from(h.read::<u16>(parameter + 8));
        let size = u32::from(h.read::<u16>(parameter + 0xa) & 0x1fff);
        let zero = 0.0f32.to_bits();
        let one = 1.0f32.to_bits();
        for (k, v) in [zero, one, zero, one].into_iter().enumerate() {
            h.write(ebp - 0x40 + 4 * k as u32, v);
        }
        let buffer: u32 = h.read(material + 4);
        if offset + size > h.read::<u32>(buffer) || offset < 4 {
            return 3;
        }
        h.call_sym(MEMCPY, Conv::Cdecl, &[buffer + offset, ebp - 0x40, size]);
        0
    }

    /// A `PArray` emptied: its storage freed unless borrowed (top bit)
    /// or none.
    fn clear(h: &mut impl Host, array: u32) {
        let word: u32 = h.read(array);
        if word == 0 {
            return;
        }
        let data: u32 = h.read(array + 4);
        if data != 0 && word >> 31 == 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
        }
        h.write(array + 4, 0u32);
        h.write(array, 0u32);
    }

    /// The caster `j` of `list`: its box (`[+0x1c]`, kept at `ebp - 8`)
    /// tested by `test` with `camera`, times the box's 3x4 (`+0xc`) when
    /// it has one (into `ebp - local`, the product at `- 0xf4`), else a
    /// copy of `camera` at `ebp - copy`. `al`.
    #[allow(clippy::too_many_arguments)] // the frame's places
    fn caster_test(
        h: &mut impl Host,
        ebp: u32,
        list: u32,
        j: u32,
        camera: u32,
        local: u32,
        copy: u32,
        test: u32,
    ) -> u8 {
        let items: u32 = h.read(list + 4);
        let item: u32 = h.read(items.wrapping_add(4 * j));
        let bounds: u32 = h.read(item + 0x1c);
        h.write(ebp - 8, bounds);
        let m: u32 = h.read(bounds + 0xc);
        let matrix = if m == 0 {
            for k in 0..16 {
                let v: u32 = h.read(camera.wrapping_add(4 * k));
                h.write(ebp - copy + 4 * k, v);
            }
            ebp - copy
        } else {
            h.call_sym(MATRIX_FROM_3X4, Conv::Thiscall(ebp - local), &[m]);
            h.call_sym(
                MATRIX_MULTIPLY,
                Conv::Cdecl,
                &[ebp - 0xf4, camera, ebp - local],
            )
        };
        h.call_sym(test, Conv::Thiscall(bounds), &[matrix]) as u8
    }

    /// `list` appended to `array` (a new buffer one longer, through
    /// `ebp - 8`).
    fn push(h: &mut impl Host, ebp: u32, array: u32, list: u32) {
        h.write(ebp - 8, list);
        let n = (h.read::<u32>(array) & 0x7fff_ffff).wrapping_add(1);
        let storage = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[n.wrapping_mul(4), 4]);
        h.call_sym(PUSH_INTO, Conv::Thiscall(array), &[storage, ebp - 8]);
    }

    /// `0x006c8090(this, kind, out, camera, single)` (thiscall, `ret
    /// 0x10`; frame: the product at `ebp - 0xf4`, matrices at `- 0xb4`
    /// and `- 0x74`, the loop's words `- 0x34` .. `- 8`, the two flags
    /// `- 2`, `- 1`). The two caster arrays `out` (`+0`) and `out + 8`
    /// are emptied; for each of the 64 slots (8 blocks of 8) the flags
    /// `+0x2b28` / `+0x2b68` are cleared, and a slot with a list
    /// (`+0xde8`), in the shadow map (`+0xfe8`) and visible (`+0x10e8`),
    /// accepted by the filter `+0x4048(+0x2ea8)` when `+0x4030` is set,
    /// is tested: for kind 4 without `single`, each caster against
    /// `camera` (flag 1) and the second matrix `camera + 0x40` (flag 2),
    /// any one passing; otherwise flag 1 holds while every caster passes
    /// (whole-box visibility when the slot's size `+0x6b8` or `+0x7b8`
    /// exceeds the limit, else the inside test). The flags are stored
    /// and the list pushed onto `out` (flag 1) and `out + 8` (flag 2).
    /// `eax`: the loop's base word, `-0x276e - this`.
    pub fn collect_casters(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        kind: u32,
        out: u32,
        camera: u32,
        single: u32,
    ) -> u32 {
        h.write(ebp - 0x20, this);
        clear(h, out);
        h.write(ebp - 0x30, out + 8);
        clear(h, out + 8);
        let base = 0xffff_d892u32.wrapping_sub(this);
        h.write(ebp - 0x18, base);
        h.write(ebp - 0x2c, 0xffff_d8d2u32.wrapping_sub(this));
        h.write(ebp - 0x34, 0xffff_e042u32.wrapping_sub(this));
        for block in 0..8u32 {
            h.write(ebp - 0x28, 8 - block);
            h.write(ebp - 0xc, this + 0x2b68 + 8 * block);
            for c in 0..8u32 {
                let slot = 8 * block + c;
                h.write(ebp - 0x10, c);
                h.write(ebp - 0x14, this + 0xde8 + 4 * slot);
                h.write(this + 0x2b28 + slot, 0u8);
                h.write(this + 0x2b68 + slot, 0u8);
                let list: u32 = h.read(this + 0xde8 + 4 * slot);
                if list == 0
                    || h.read::<u32>(this + 0xfe8 + 4 * slot) != 1
                    || h.read::<u32>(this + 0x10e8 + 4 * slot) != 1
                {
                    continue;
                }
                if h.read::<u8>(this + 0x4030) != 0 {
                    let filter: u32 = h.read(this + 0x4048);
                    let arg: u32 = h.read(this + 0x2ea8 + 4 * slot);
                    if h.call(filter, Conv::Cdecl, &[arg]) as u8 == 0 {
                        continue;
                    }
                }
                h.write(ebp - 0x1c, list);
                let mut first = 0u8;
                let mut second = 0u8;
                h.write(ebp - 1, first);
                h.write(ebp - 2, second);
                if h.read::<u32>(list) & 0x7fff_ffff != 0 {
                    if kind == 4 && single as u8 == 0 {
                        h.write(ebp - 0x24, camera.wrapping_add(0x40));
                        let mut j = 0;
                        while j < h.read::<u32>(list) & 0x7fff_ffff {
                            if caster_test(h, ebp, list, j, camera, 0x74, 0xb4, BOX_IS_VISIBLE) != 0
                            {
                                first = 1;
                                h.write(ebp - 1, first);
                            }
                            let far = camera.wrapping_add(0x40);
                            if caster_test(h, ebp, list, j, far, 0xb4, 0x74, BOX_IS_VISIBLE) != 0 {
                                second = 1;
                                h.write(ebp - 2, second);
                            }
                            j += 1;
                        }
                    } else {
                        first = 1;
                        h.write(ebp - 1, first);
                        let mut j = 0;
                        while j < h.read::<u32>(list) & 0x7fff_ffff {
                            let limit = x87::load(h.read(h.rebase(CASTER_SIZE_LIMIT)));
                            let a = x87::load(h.read(this + 0x6b8 + 4 * slot));
                            let b = x87::load(h.read(this + 0x7b8 + 4 * slot));
                            let test = if limit < a || b > limit {
                                BOX_IS_VISIBLE
                            } else {
                                BOX_IS_INSIDE
                            };
                            if caster_test(h, ebp, list, j, camera, 0xb4, 0x74, test) == 0 {
                                first = 0;
                                h.write(ebp - 1, first);
                                break;
                            }
                            j += 1;
                        }
                    }
                }
                h.write(this + 0x2b28 + slot, first);
                h.write(this + 0x2b68 + slot, second);
                if first != 0 {
                    push(h, ebp, out, list);
                }
                if second != 0 {
                    push(h, ebp, out + 8, list);
                }
            }
            h.write(ebp - 0x10, 8u32);
            h.write(ebp - 0x14, this + 0xde8 + 4 * (8 * block + 8));
        }
        h.write(ebp - 0x28, 0u32);
        h.write(ebp - 0xc, this + 0x2ba8);
        base
    }
}

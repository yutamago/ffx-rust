//! The render-side glue, twenty-fifth batch (M4 port round 7, list
//! `h3-glue-render`): a texture-animation entry's release, the scene's
//! material colour scale and the VFX table's delete pool.

/// `Phyre_PhyreFree(p)` and `operator delete(p)` (both cdecl).
pub const PHYRE_FREE: u32 = crate::glue4::PHYRE_FREE;
pub const OPERATOR_DELETE: u32 = crate::glue10::OPERATOR_DELETE;

/// The colour scale's walk: `PRedBlackTree::successor(this, node)`
/// (thiscall on the tree head, `ret 4`), `PArray::data(this)`,
/// `ShaderParams_FindByName(shader, name)` (thiscall, `ret 4`),
/// `memcpy(dst, src, n)` (cdecl), `"clrScaleFactor"` and the 0.0001
/// (`f64`) the light colour's squared length must pass.
pub const TREE_SUCCESSOR: u32 = crate::h3_glue20::TREE_SUCCESSOR;
pub const ARRAY_DATA: u32 = crate::h3_glue20::ARRAY_DATA;
pub const FIND_PARAMETER: u32 = crate::h3_glue20::FIND_PARAMETER;
pub const MEMCPY: u32 = crate::glue5::MEMCPY;
pub const CLR_SCALE_FACTOR: u32 = 0x00b4_3c30;
pub const MIN_LENGTH2: u32 = 0x00b3_eb50;

/// The delete pool's releases: `operator_delete_00630540(p)` (cdecl),
/// `ModelBuffer_ReleaseBufferByIndex(index)` (cdecl, a `u16`) and
/// `ClassDynamicGeometry_clear(this)` (thiscall, plain `ret`).
pub const OPERATOR_DELETE_ARRAY: u32 = 0x0063_0540;
pub const RELEASE_BUFFER_BY_INDEX: u32 = 0x0072_c1b0;
pub const GEOMETRY_CLEAR: u32 = 0x006a_1040;

/// The shadow mesh: `PCluster::create(cluster; 1, type, 0)` and
/// `allocateInstanceList(cluster; 1, type, 0)` (thiscall, `ret 0xc`),
/// `PhyreMalloc(n, align)`, the segment array's construct and destroy
/// (cdecl), the stream constructor (thiscall), the stream array's
/// `setBuffer(this; data, n)` (`ret 8`), the format size (cdecl),
/// `isBound` and the index buffer's release (thiscall),
/// `LayoutDesc_setup(desc; a, b)` (`ret 8`), the
/// `ShadowDynamicMeshInstance` and `PMeshInstance` constructors
/// (`ret 4`), and the types: mesh, dynamic mesh, shadow instance, mesh
/// instance.
pub const CLUSTER_CREATE: u32 = crate::p6_quads::CLUSTER_CREATE;
pub const ALLOCATE_INSTANCE_LIST: u32 = crate::p6_quads::ALLOCATE_INSTANCE_LIST;
pub const MALLOC_ALIGNED: u32 = crate::p6_quads::MALLOC_ALIGNED;
pub const SEGMENTS_CONSTRUCT: u32 = crate::p6_quads::SEGMENTS_CONSTRUCT;
pub const SEGMENTS_DESTROY: u32 = crate::p6_quads::SEGMENTS_DESTROY;
pub const STREAM_CTOR: u32 = crate::p6_quads::STREAM_CTOR;
pub const STREAMS_SET: u32 = crate::p6_quads::STREAMS_SET;
pub const FORMAT_SIZE: u32 = 0x0055_bc40;
pub const INDEX_IS_BOUND: u32 = 0x0056_2db0;
pub const INDEX_RELEASE: u32 = 0x0056_48c0;
pub const LAYOUT_SETUP: u32 = 0x0067_8bd0;
pub const SHADOW_INSTANCE_CTOR: u32 = 0x006f_9490;
pub const INSTANCE_CTOR: u32 = crate::p6_quads::INSTANCE_CTOR;
pub const MESH_TYPE: u32 = crate::p6_quads::MESH_TYPE;
pub const DYNAMIC_MESH_TYPE: u32 = 0x00cc_c158;
pub const SHADOW_INSTANCE_TYPE: u32 = 0x00ce_82f8;
pub const INSTANCE_TYPE: u32 = crate::p6_quads::INSTANCE_TYPE;

/// The broken-screen pass: the three parameter names (`"ColorBuffer"`,
/// `"UvScaleBias"`, `"OutputScale"`), `0x006d4780(this; a, b)` (`ret 8`),
/// `DynamicMesh_setStreams_00677ce0(mesh; owner, kind, flags)` (`ret
/// 0xc`), `BrokenScreenPolygonDynamicMeshInstance_ctor(this; mesh)` and
/// the broken-screen mesh and instance types.
pub const COLOR_BUFFER: u32 = 0x00b4_25d8;
pub const UV_SCALE_BIAS: u32 = 0x00b4_25e4;
pub const OUTPUT_SCALE: u32 = 0x00b4_948c;
pub const PASS_SETUP_6D4780: u32 = 0x006d_4780;
pub const SET_STREAMS: u32 = 0x0067_7ce0;
pub const POLYGON_INSTANCE_CTOR: u32 = 0x0067_6b70;
pub const BROKEN_MESH_TYPE: u32 = 0x00cc_c058;
pub const POLYGON_INSTANCE_TYPE: u32 = 0x00cc_bf88;

/// The count word's flag bit (the array does not own its storage).
const UNOWNED: u32 = 0x8000_0000;
const COUNT_MASK: u32 = 0x7fff_ffff;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn free(h: &mut impl Host, p: u32) {
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
    }

    /// A small `{count, data}` array record behind a pointer at `slot`
    /// (`+0x38` and `+0x3c` of a 0x40-byte item): its data freed when
    /// owned (and, with `need_data`, set), the record cleared and freed,
    /// the pointer cleared.
    fn release_record(h: &mut impl Host, slot: u32, need_data: bool) {
        let record: u32 = h.read(slot);
        if record == 0 {
            return;
        }
        let count: u32 = h.read(record);
        if count & UNOWNED == 0 {
            let data: u32 = h.read(record + 4);
            if !need_data || data != 0 {
                free(h, data);
            }
        }
        h.write(record + 4, 0u32);
        h.write(record, 0u32);
        free(h, record);
        h.write(slot, 0u32);
    }

    /// `0x006b3fe0(this)` (thiscall): a texture-animation entry's release
    /// (`TextureAnimationManager_unregisterByName`'s per-entry step).
    /// The items (`+0x14`, count `+0x10`) get their first word cleared and
    /// are deleted, then their array freed; each 0x40-byte animation
    /// (`+0xc`, count `+8`) loses its two records (`+0x38`, `+0x3c`), then
    /// all of them are destroyed (virtual 0 with 0) and freed; the word
    /// array (`+4`, count `+0`) is cleared and freed; the two tagged
    /// pointers (`+0x18`, `+0x1c`) are freed unless their low bit is set.
    pub fn texture_entry_release(h: &mut impl Host, this: u32) {
        let mut i = 0u32;
        while i < h.read::<u32>(this + 0x10) & COUNT_MASK {
            let items: u32 = h.read(this + 0x14);
            let item: u32 = h.read(items.wrapping_add(4 * i));
            h.write(item, 0u32);
            let items: u32 = h.read(this + 0x14);
            let item: u32 = h.read(items.wrapping_add(4 * i));
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[item]);
            i += 1;
        }
        release_array(h, this + 0x10);
        let mut i = 0u32;
        while i < h.read::<u32>(this + 8) & COUNT_MASK {
            let base: u32 = h.read(this + 0xc);
            let item = base.wrapping_add(0x40 * i);
            release_record(h, item.wrapping_add(0x38), false);
            release_record(h, item.wrapping_add(0x3c), true);
            i += 1;
        }
        let count: u32 = h.read(this + 8);
        if count != 0 {
            let data: u32 = h.read(this + 0xc);
            if data != 0 && count & UNOWNED == 0 {
                for k in 0..count & COUNT_MASK {
                    h.call_virtual(data.wrapping_add(0x40 * k), 0, &[0]);
                }
                free(h, data);
            }
            h.write(this + 0xc, 0u32);
            h.write(this + 8, 0u32);
        }
        let mut i = 0u32;
        while i < h.read::<u32>(this) & COUNT_MASK {
            let words: u32 = h.read(this + 4);
            h.write(words.wrapping_add(4 * i), 0u32);
            i += 1;
        }
        release_array(h, this);
        for slot in [this + 0x18, this + 0x1c] {
            let p: u32 = h.read(slot);
            if p & 1 == 0 {
                free(h, p);
            }
            h.write(slot, 0u32);
        }
    }

    /// A `{count, data}` array at `at`: with a count, the data freed when
    /// set and owned, then both cleared (data first).
    fn release_array(h: &mut impl Host, at: u32) {
        let count: u32 = h.read(at);
        if count == 0 {
            return;
        }
        let data: u32 = h.read(at + 4);
        if data != 0 && count & UNOWNED == 0 {
            free(h, data);
        }
        h.write(at + 4, 0u32);
        h.write(at, 0u32);
    }

    /// `PhyreScene_setMaterialColourScale2(this, r, g, b, unused; slots)`
    /// (thiscall, `ret 0x10`; float bits): with a map cluster (`+0x10d4`)
    /// the colour `{r, g, b, 1}` goes to `+0xfc78`; when the light colour
    /// (`+0x10d30`) has a squared length (summed in `r`'s slot, rounded to
    /// `f32` at each step) above 0.0001, the light (`+0xcc`) gets it
    /// scaled by the colour (`+0x30..`); then every material of the map
    /// tree (`+0x1198`, as `PhyreScene_updateMapYIQ` walks it) with a
    /// `"clrScaleFactor"` that fits its buffer (offset 4 on) takes the
    /// colour. The slots end as the walk's next node (`r`), the last
    /// material array (`g`) and the list index (`b`).
    #[allow(clippy::similar_names)] // the light's three channels
    pub fn scene_set_material_colour_scale2(
        h: &mut impl Host,
        slots: u32,
        this: u32,
        r: u32,
        g: u32,
        b: u32,
    ) {
        if h.read::<u32>(this + 0x10d4) == 0 {
            return;
        }
        let colour = this + 0xfc78;
        let (r, g, b) = (x87::load(r), x87::load(g), x87::load(b));
        h.write(colour, x87::store(r));
        h.write(colour + 4, x87::store(g));
        h.write(colour + 8, x87::store(b));
        h.write(colour + 0xc, 1.0f32.to_bits());
        let c30 = x87::load(h.read(this + 0x1_0d30));
        let c34 = x87::load(h.read(this + 0x1_0d34));
        let c38 = x87::load(h.read(this + 0x1_0d38));
        let sum = x87::store(x87::mul(c30, c30));
        let sum = x87::store(x87::add(x87::load(sum), x87::mul(c34, c34)));
        let sum = x87::store(x87::add(x87::load(sum), x87::mul(c38, c38)));
        let at = h.rebase(MIN_LENGTH2);
        let limit = f64::from_bits(h.read::<u64>(at));
        if x87::load(sum) > limit {
            let light: u32 = h.read(this + 0xcc);
            let lr = x87::store(x87::mul(r, c30));
            let lg = x87::store(x87::mul(g, c34));
            let lb = x87::store(x87::mul(b, c38));
            h.write(light.wrapping_add(0x30), lr);
            h.write(light.wrapping_add(0x34), lg);
            h.write(light.wrapping_add(0x38), lb);
        }
        let head = this + 0x1198;
        let mut node = head + 0xc;
        let mut at: u32 = h.read(head + 0xc);
        if at == head || at == 0 {
            node = at;
        } else {
            while at != head {
                node = at;
                at = h.read(at);
            }
        }
        h.write(slots, node);
        if node == head {
            return;
        }
        let mut next = h.call_sym(TREE_SUCCESSOR, Conv::Thiscall(head), &[node]);
        h.write(slots, next);
        loop {
            let list: u32 = h.read(node.wrapping_add(0x10));
            h.write(slots + 8, 0u32);
            let mut k = 0u32;
            while k < h.read::<u32>(list) {
                let items: u32 = h.read(list + 8);
                let item: u32 = h.read(items.wrapping_add(4 * k));
                let array: u32 = h.read(item + 0x10);
                h.write(slots + 4, array);
                let mut j = 0u32;
                while j < h.read::<u32>(array) & COUNT_MASK {
                    let data = h.call_sym(ARRAY_DATA, Conv::Thiscall(array), &[]);
                    let material: u32 = h.read(data.wrapping_add(4 * j));
                    let shader: u32 = h.read(material);
                    let name = h.rebase(CLR_SCALE_FACTOR);
                    let parameter = h.call_sym(FIND_PARAMETER, Conv::Thiscall(shader), &[name]);
                    if parameter != 0 && colour != 0 {
                        let buffer: u32 = h.read(material + 4);
                        let offset = u32::from(h.read::<u16>(parameter + 8));
                        let size = u32::from(h.read::<u16>(parameter + 0xa) & 0x1fff);
                        if offset + size <= h.read::<u32>(buffer) && offset >= 4 {
                            h.call_sym(
                                MEMCPY,
                                Conv::Cdecl,
                                &[buffer.wrapping_add(offset), colour, size],
                            );
                        }
                    }
                    j += 1;
                }
                k += 1;
                h.write(slots + 8, k);
            }
            node = next;
            if next != head {
                next = h.call_sym(TREE_SUCCESSOR, Conv::Thiscall(head), &[next]);
            }
            h.write(slots, next);
            if node == head {
                return;
            }
        }
    }

    /// `ClassVFXRenderDataTable_UpdateDeletePool(this)` (thiscall): each
    /// pool entry with a key (`+0xc`, 8 bytes) counts down (`+0x14`); one
    /// that goes below 0 releases its geometry (`+0x1c`'s object, its
    /// `+0x24`): segments (`+0x94`, 0x6c bytes, count `+0x90`) given back
    /// to the segment pool (`+0x3c`, flags `+0x44`, low mark `+0x48`; 4000
    /// of them) or deleted, the model buffer released, the geometry given
    /// back to its pool (`+0x2c`, flags `+0x34`, low mark `+0x38`; 3000)
    /// and cleared, or deleted with its object (virtual 0 with 1); then
    /// the entry is cleared. Ends with `+0x20` and `+0x24` at 0.
    pub fn vfx_table_update_delete_pool(h: &mut impl Host, this: u32) {
        let mut i = 0u32;
        while i < h.read::<u32>(this + 8) & COUNT_MASK {
            let keys: u32 = h.read(this + 0xc);
            let key = keys.wrapping_add(8 * i);
            if h.read::<u32>(key) | h.read::<u32>(key + 4) != 0 {
                let counters: u32 = h.read(this + 0x14);
                let counter = counters.wrapping_add(4 * i);
                let left = h.read::<u32>(counter).wrapping_sub(1);
                h.write(counter, left);
                let counters: u32 = h.read(this + 0x14);
                if (h.read::<u32>(counters.wrapping_add(4 * i)) as i32) < 0 {
                    release_entry(h, this, i);
                }
            }
            i += 1;
        }
        h.write(this + 0x20, 0u32);
        h.write(this + 0x24, 0u32);
    }

    /// Gives `slot` back to a pool when `at` lies inside it: `(at - base)
    /// / size` (signed) below `limit`, its flag (`flags`) cleared and the
    /// low mark (`low`) lowered. Whether it did.
    #[allow(clippy::too_many_arguments)] // the pool's fields
    fn pool_give_back(
        h: &mut impl Host,
        at: u32,
        base: u32,
        size: i32,
        limit: u32,
        flags: u32,
        low: u32,
    ) -> bool {
        let base: u32 = h.read(base);
        let slot = (at.wrapping_sub(base) as i32 / size) as u32;
        if slot >= limit {
            return false;
        }
        let flags: u32 = h.read(flags);
        h.write(slot.wrapping_add(flags), 0u8);
        if (slot as i32) < h.read::<u32>(low) as i32 {
            h.write(low, slot);
        }
        true
    }

    /// The expired entry `i` of the delete pool.
    fn release_entry(h: &mut impl Host, this: u32, i: u32) {
        let objects: u32 = h.read(this + 0x1c);
        let object: u32 = h.read(objects.wrapping_add(4 * i));
        let geometry: u32 = h.read(object.wrapping_add(0x24));
        let count: u32 = h.read(geometry.wrapping_add(0x90));
        let pooled_segments = count == 1 && {
            let segments: u32 = h.read(geometry.wrapping_add(0x94));
            pool_give_back(
                h,
                segments,
                this + 0x3c,
                0x6c,
                4000,
                this + 0x44,
                this + 0x48,
            )
        };
        let pooled = pool_give_back(
            h,
            geometry,
            this + 0x2c,
            0xd0,
            3000,
            this + 0x34,
            this + 0x38,
        );
        if (count as i32) > 0 {
            let mut left = count;
            let mut at = 0u32;
            loop {
                if !pooled_segments {
                    let segments: u32 = h.read(geometry.wrapping_add(0x94));
                    let p: u32 = h.read(segments.wrapping_add(at + 0x20));
                    h.call_sym(OPERATOR_DELETE_ARRAY, Conv::Cdecl, &[p]);
                }
                left = left.wrapping_sub(1);
                for off in [0x20, 0xc, 0x14, 0x10, 0x18, 0x1c] {
                    let segments: u32 = h.read(geometry.wrapping_add(0x94));
                    h.write(segments.wrapping_add(at + off), 0u32);
                }
                at += 0x6c;
                if left == 0 {
                    break;
                }
            }
        }
        if !pooled_segments {
            let segments: u32 = h.read(geometry.wrapping_add(0x94));
            h.call_sym(OPERATOR_DELETE_ARRAY, Conv::Cdecl, &[segments]);
        }
        let index = u32::from(h.read::<u16>(geometry.wrapping_add(0xc2)));
        h.write(geometry.wrapping_add(0x94), 0u32);
        h.call_sym(RELEASE_BUFFER_BY_INDEX, Conv::Cdecl, &[index]);
        if !pooled {
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[geometry]);
        }
        let objects: u32 = h.read(this + 0x1c);
        let object: u32 = h.read(objects.wrapping_add(4 * i));
        h.write(object.wrapping_add(0x24), 0u32);
        let objects: u32 = h.read(this + 0x1c);
        let object: u32 = h.read(objects.wrapping_add(4 * i));
        if pooled {
            h.call_sym(GEOMETRY_CLEAR, Conv::Thiscall(object), &[]);
        } else if object != 0 {
            h.call_virtual(object, 0, &[1]);
        }
        let objects: u32 = h.read(this + 0x1c);
        h.write(objects.wrapping_add(4 * i), 0u32);
        let counters: u32 = h.read(this + 0x14);
        h.write(counters.wrapping_add(4 * i), 0xffff_ffffu32);
        let keys: u32 = h.read(this + 0xc);
        h.write(keys.wrapping_add(8 * i), 0u32);
        h.write(keys.wrapping_add(8 * i + 4), 0u32);
    }

    /// `PCluster::create` or `allocateInstanceList` on `cluster` with the
    /// type at `ty`.
    fn cluster_call(h: &mut impl Host, f: u32, cluster: u32, ty: u32) -> u32 {
        let ty = h.rebase(ty);
        h.call_sym(f, Conv::Thiscall(cluster), &[1, ty, 0])
    }

    /// `0x006c9580(this, out; slots)` (thiscall, `ret 4`): a shadow mesh
    /// on the cluster at `this + 4`. The mesh (`PCluster::create`) gets
    /// one segment (a constructed `PhyreMalloc(0x6c, 4)` block replacing
    /// owned old data) of triangle lists (`+0x10 = 2`) with four streams
    /// (unless it has four), the index block `{0, 7, 12, format 8}`
    /// (`+0x30..+0x3c`) sized `12 * FormatSize(8)` (`+0x64`; a bound
    /// buffer released on a change); a dynamic mesh goes to `*out` and is
    /// laid out `(8, 12)`; its `ShadowDynamicMeshInstance` goes into the
    /// mesh's `PMeshInstance` (`+0x18`), which is returned. `out`'s slot
    /// takes each instance found.
    pub fn shadow_mesh_create(h: &mut impl Host, slots: u32, this: u32, out: u32) -> u32 {
        let cluster = this + 4;
        let mesh = cluster_call(h, CLUSTER_CREATE, cluster, MESH_TYPE);
        if h.read::<u32>(mesh) != 1 {
            let data = h.call_sym(MALLOC_ALIGNED, Conv::Cdecl, &[0x6c, 4]);
            if data != 0 {
                h.call_sym(SEGMENTS_CONSTRUCT, Conv::Cdecl, &[data, 1]);
                let count: u32 = h.read(mesh);
                let old: u32 = h.read(mesh + 4);
                if old != data && count & UNOWNED == 0 {
                    h.call_sym(SEGMENTS_DESTROY, Conv::Cdecl, &[old, count & COUNT_MASK]);
                }
                h.write(mesh + 4, data);
                h.write(mesh, 1u32);
            }
        }
        let segment: u32 = h.read(mesh + 4);
        h.write(segment, 0u32);
        h.write(segment.wrapping_add(0x10), 2u32);
        let streams = segment.wrapping_add(0x28);
        if h.read::<u32>(streams) != 4 {
            let data = h.call_sym(MALLOC_ALIGNED, Conv::Cdecl, &[0x100, 4]);
            if data != 0 {
                for k in 0..4 {
                    let stream = data.wrapping_add(0x40 * k);
                    if stream != 0 {
                        h.call_sym(STREAM_CTOR, Conv::Thiscall(stream), &[]);
                    }
                }
                h.call_sym(STREAMS_SET, Conv::Thiscall(streams), &[data, 4]);
            }
        }
        h.write(segment.wrapping_add(0x38), 0xcu32);
        h.write(segment.wrapping_add(0x3c), 8u8);
        h.write(segment.wrapping_add(0x30), 0u32);
        h.write(segment.wrapping_add(0x34), 7u32);
        let size = h.call_sym(FORMAT_SIZE, Conv::Cdecl, &[8]).wrapping_mul(12);
        if h.read::<u32>(segment.wrapping_add(0x64)) != size {
            let index = segment.wrapping_add(0x30);
            if h.call_sym(INDEX_IS_BOUND, Conv::Thiscall(index), &[]) as u8 != 0 {
                h.call_sym(INDEX_RELEASE, Conv::Thiscall(index), &[]);
            }
            h.write(segment.wrapping_add(0x64), size);
        }
        let dynamic = cluster_call(h, CLUSTER_CREATE, cluster, DYNAMIC_MESH_TYPE);
        let dynamic = if dynamic == 0 { 0 } else { dynamic - 4 };
        h.write(out, dynamic);
        h.call_sym(LAYOUT_SETUP, Conv::Thiscall(dynamic), &[8, 0xc]);
        let mut shadow = 0;
        let list = cluster_call(h, ALLOCATE_INSTANCE_LIST, cluster, SHADOW_INSTANCE_TYPE);
        if list != 0 {
            let at: u32 = h.read(list + 0x34);
            h.write(slots, at);
            if at != 0 {
                let dynamic: u32 = h.read(out);
                shadow = h.call_sym(SHADOW_INSTANCE_CTOR, Conv::Thiscall(at), &[dynamic]);
            }
        }
        let mut instance = 0;
        let list = cluster_call(h, ALLOCATE_INSTANCE_LIST, cluster, INSTANCE_TYPE);
        if list != 0 {
            let at: u32 = h.read(list + 0x34);
            h.write(slots, at);
            if at != 0 {
                instance = h.call_sym(INSTANCE_CTOR, Conv::Thiscall(at), &[mesh]);
            }
        }
        h.write(instance.wrapping_add(0x18), shadow);
        instance
    }

    /// `0x006d3fb0(this, shader, streams, a, cluster, b, c; slots)`
    /// (thiscall, `ret 0x18`): a broken-screen pass's set-up. `a`,
    /// `cluster` and `shader` go to `+0xc`, `+0x14`, `+0x18`, the streams'
    /// first word to `+0x1c`, `{1, 1, 1, 1}` to `+0x30`; the shader's
    /// three parameters to `+0xa8`, `+0xa4`, `+0xa0`; a mesh to `+0x20`;
    /// `0x006d4780(this; b, c)` is the result (kept in `a`'s slot); a
    /// dynamic mesh (`+0x28`) gets its streams for the pass kind (`+8`:
    /// 6 gives 0x18, 8 gives 0x24, others 0); a polygon instance
    /// (`+0x2c`) and a mesh instance (`+0x24`, which takes the streams at
    /// `+0x10` and the polygon instance at `+0x18`). The slots of
    /// `shader` and `cluster` take each instance found.
    #[allow(clippy::too_many_arguments)] // the original's
    pub fn broken_pass_setup(
        h: &mut impl Host,
        slots: u32,
        this: u32,
        shader: u32,
        streams: u32,
        a: u32,
        cluster: u32,
        b: u32,
        c: u32,
    ) -> u32 {
        h.write(this + 0xc, a);
        h.write(this + 0x14, cluster);
        h.write(this + 0x18, shader);
        let count = h.read::<u32>(streams) & COUNT_MASK;
        let first = if count > 1 {
            h.read(streams + 4)
        } else if count != 0 {
            streams + 4
        } else {
            0
        };
        let word: u32 = h.read(first);
        h.write(this + 0x1c, word);
        for k in 0..4 {
            h.write(this + 0x30 + 4 * k, 1.0f32.to_bits());
        }
        for (name, at) in [
            (COLOR_BUFFER, 0xa8),
            (UV_SCALE_BIAS, 0xa4),
            (OUTPUT_SCALE, 0xa0),
        ] {
            let name = h.rebase(name);
            let p = h.call_sym(FIND_PARAMETER, Conv::Thiscall(shader), &[name]);
            h.write(this + at, p);
        }
        let cluster: u32 = h.read(this + 0x14);
        let mesh = cluster_call(h, CLUSTER_CREATE, cluster, MESH_TYPE);
        h.write(this + 0x20, mesh);
        let result = h.call_sym(PASS_SETUP_6D4780, Conv::Thiscall(this), &[b, c]);
        h.write(slots + 8, result);
        let cluster: u32 = h.read(this + 0x14);
        let dynamic = cluster_call(h, CLUSTER_CREATE, cluster, BROKEN_MESH_TYPE);
        let dynamic = if dynamic == 0 { 0 } else { dynamic - 4 };
        let kind: u32 = h.read(this + 8);
        h.write(this + 0x28, dynamic);
        let flags = match kind {
            6 => 0x18,
            8 => 0x24,
            _ => 0,
        };
        h.call_sym(SET_STREAMS, Conv::Thiscall(dynamic), &[this, kind, flags]);
        let cluster: u32 = h.read(this + 0x14);
        let mut polygon = 0;
        let list = cluster_call(h, ALLOCATE_INSTANCE_LIST, cluster, POLYGON_INSTANCE_TYPE);
        if list != 0 {
            let at: u32 = h.read(list + 0x34);
            h.write(slots + 0xc, at);
            if at != 0 {
                h.write(slots, at);
                let dynamic: u32 = h.read(this + 0x28);
                polygon = h.call_sym(POLYGON_INSTANCE_CTOR, Conv::Thiscall(at), &[dynamic]);
            }
        }
        let cluster: u32 = h.read(this + 0x14);
        let mesh: u32 = h.read(this + 0x20);
        h.write(this + 0x2c, polygon);
        let mut instance = 0;
        let list = cluster_call(h, ALLOCATE_INSTANCE_LIST, cluster, INSTANCE_TYPE);
        if list != 0 {
            let at: u32 = h.read(list + 0x34);
            h.write(slots + 0xc, at);
            if at != 0 {
                h.write(slots, at);
                instance = h.call_sym(INSTANCE_CTOR, Conv::Thiscall(at), &[mesh]);
            }
        }
        h.write(this + 0x24, instance);
        h.write(instance.wrapping_add(0x10), streams);
        let instance: u32 = h.read(this + 0x24);
        let polygon: u32 = h.read(this + 0x2c);
        h.write(instance.wrapping_add(0x18), polygon);
        h.read(slots + 8)
    }
}

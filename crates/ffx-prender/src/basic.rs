//! Small `PhyreEngine` methods the render code reaches (M4 port round 1,
//! h1-render-a): constructors that only store defaults, forwarders,
//! comparators and lookups. Each function's doc names its original.

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

/// `PGeometry::PDynamicMesh`'s vtable.
pub const DYNAMIC_MESH_VTABLE: u32 = 0x00b1_6868;
/// `PNode_RegisterClass` (cdecl, no arguments).
pub const NODE_REGISTER: u32 = 0x0050_5450;
/// `PUtilitySprite::initialize`'s seven registrations (cdecl, no
/// arguments), in call order.
pub const SPRITE_REGISTER: [u32; 7] = [
    0x0056_91b0,
    0x0056_8dc0,
    0x0056_a1c0,
    0x0056_a390,
    0x0056_86f0,
    0x0056_f990,
    0x0056_8ab0,
];
/// `PSamplerStateD3D11_ctor` (thiscall).
pub const SAMPLER_D3D11_CTOR: u32 = 0x0058_3b20;
/// The sub-object constructor `0x004835d0` calls on `this + 8` (thiscall,
/// one stack word).
pub const STREAM_CTOR: u32 = 0x0048_3710;
/// `PShaderPass`'s destructor (thiscall).
pub const SHADER_PASS_DTOR: u32 = 0x0058_53c0;
/// `PhyreFree` (cdecl).
pub const FREE: u32 = 0x0042_fb20;
/// `PEventFlagWin32::PEventFlagWin32` (thiscall).
pub const EVENT_FLAG_CTOR: u32 = 0x0042_fc80;

/// `PhyreMalloc` (cdecl `(size, alignment)`).
pub const MALLOC: u32 = 0x0042_fb80;
/// The byte a new `PMaterial`'s one-byte block starts with.
pub const MATERIAL_BYTE: u32 = 0x00b8_fbeb;
/// The global object whose virtual slot 1 gives the default buffer
/// (`0x0048c6e0`).
pub const DEFAULTS_OBJECT: u32 = 0x00ca_5268;
/// `PClassDescriptor::applyDefaultBufferWithValidation` (thiscall, two
/// words).
pub const APPLY_DEFAULTS: u32 = 0x0043_c160;
/// The member's own apply (`0x0048c650`, thiscall, two words).
pub const APPLY_MEMBER: u32 = 0x0048_c650;
/// The iterator's first step (`0x00464f30`, thiscall).
pub const ITERATOR_FIRST: u32 = 0x0046_4f30;
/// The iterator's type tag (`PTexture2D`'s descriptor).
pub const ITERATOR_TAG: u32 = 0x00ca_37d0;
/// `PUtility`'s list of utilities (sorted by `+0xc`).
pub const UTILITIES: u32 = 0x00c9_01a8;
/// `PUtility`'s vtable, its `+8` table, `PUtilitySprite`'s vtable.
pub const UTILITY_VTABLES: [u32; 3] = [0x00b0_e078, 0x00b0_e14c, 0x00b0_e140];
/// The sprite utility's member constructor (`0x00433700`, thiscall).
pub const SPRITE_MEMBER_CTOR: u32 = 0x0043_3700;

/// The 0x40-byte element destructor of `0x00433900`'s array (thiscall).
pub const ELEMENT40_DTOR: u32 = 0x0055_dfa0;
/// The CRT's `memcpy` (cdecl).
pub const MEMCPY: u32 = 0x0094_9256;

/// `PRenderer` draw wrappers `0x00498500` calls (thiscall on the
/// context): indexed instanced (four words), indexed (three words).
pub const DRAW_INSTANCED: u32 = 0x0059_2c60;
/// See [`DRAW_INSTANCED`].
pub const DRAW_INDEXED: u32 = 0x0059_2b60;
/// The renderer context, its guard word, constructor and `atexit`
/// destructor (`0x004649d0`).
pub const CONTEXT: u32 = 0x00c9_4f00;
/// See [`CONTEXT`].
pub const CONTEXT_GUARD: u32 = 0x00c9_628c;
/// See [`CONTEXT`].
pub const CONTEXT_CTOR: u32 = 0x004f_ee70;
/// See [`CONTEXT`].
pub const CONTEXT_DTOR: u32 = 0x00af_b710;
/// The CRT's `atexit` (cdecl).
pub const ATEXIT: u32 = 0x0094_9207;

/// The renderer id `0x00464710` returns: `"11XD"` read as a little-endian
/// word.
pub const RENDERER_ID: u32 = 0x4458_3131;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        APPLY_DEFAULTS, APPLY_MEMBER, ATEXIT, CONTEXT, CONTEXT_CTOR, CONTEXT_DTOR, CONTEXT_GUARD,
        Conv, DEFAULTS_OBJECT, DRAW_INDEXED, DRAW_INSTANCED, DYNAMIC_MESH_VTABLE, ELEMENT40_DTOR,
        EVENT_FLAG_CTOR, FREE, Host, ITERATOR_FIRST, ITERATOR_TAG, MALLOC, MATERIAL_BYTE, MEMCPY,
        Mem, NODE_REGISTER, SAMPLER_D3D11_CTOR, SHADER_PASS_DTOR, SPRITE_MEMBER_CTOR,
        SPRITE_REGISTER, STREAM_CTOR, UTILITIES, UTILITY_VTABLES,
    };
    use ffx_bridge::x87;

    /// `0x0042fcc0(this)` -> `this`: one word 0.
    pub fn clear_word(m: &mut impl Mem, this: u32) -> u32 {
        m.write(this, 0u32);
        this
    }

    /// `0x0049f990(this)` -> `this`: two words and a byte 0.
    pub fn clear_9(m: &mut impl Mem, this: u32) -> u32 {
        m.write(this, 0u32);
        m.write(this + 4, 0u32);
        m.write(this + 8, 0u8);
        this
    }

    /// `PWorld::PWorld(this)` -> `this`: an empty list head (both links
    /// to itself) and two words 0.
    pub fn world_ctor(m: &mut impl Mem, this: u32) -> u32 {
        m.write(this, this);
        m.write(this + 4, this);
        m.write(this + 8, 0u32);
        m.write(this + 0xc, 0u32);
        this
    }

    /// `0x004a1660(this)` -> `this`: `0x34460000` then three words 0.
    pub fn ctor_4a1660(m: &mut impl Mem, this: u32) -> u32 {
        m.write(this, 0x3446_0000u32);
        m.write(this + 4, 0u32);
        m.write(this + 8, 0u32);
        m.write(this + 0xc, 0u32);
        this
    }

    /// `0x004b5050(this)` -> `this`: six words 0.
    pub fn clear_24(m: &mut impl Mem, this: u32) -> u32 {
        for i in 0..6 {
            m.write(this + 4 * i, 0u32);
        }
        this
    }

    /// `PDynamicMesh::PDynamicMesh(this)` -> `this`: its vtable, six words
    /// 0.
    pub fn dynamic_mesh_ctor(m: &mut impl Mem, this: u32) -> u32 {
        let vtable = m.rebase(DYNAMIC_MESH_VTABLE);
        m.write(this, vtable);
        for i in 1..7 {
            m.write(this + 4 * i, 0u32);
        }
        this
    }

    /// `0x0047fbe0(this, a, b)` -> `this`: `a`, 0, the byte `b`.
    pub fn ctor_47fbe0(m: &mut impl Mem, this: u32, a: u32, b: u8) -> u32 {
        m.write(this, a);
        m.write(this + 4, 0u32);
        m.write(this + 8, b);
        this
    }

    /// `PScene::PUtilityScene::initialize` -> 0: registers `PNode`.
    pub fn scene_initialize(h: &mut impl Host) -> u32 {
        h.call_sym(NODE_REGISTER, Conv::Cdecl, &[]);
        0
    }

    /// `PSprite::PUtilitySprite::initialize` -> 0: seven registrations.
    pub fn sprite_initialize(h: &mut impl Host) -> u32 {
        for f in SPRITE_REGISTER {
            h.call_sym(f, Conv::Cdecl, &[]);
        }
        0
    }

    /// `0x004be860(this)` -> `this`: `PSamplerStateD3D11_ctor` on it.
    pub fn sampler_d3d11_new(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(SAMPLER_D3D11_CTOR, Conv::Thiscall(this), &[]);
        this
    }

    /// `0x004835d0(this, a, b, c)`: the sub-object at `this + 8` made
    /// from `c`, then `a`, `b` at `+0`, `+4`; the sub-constructor's `eax`.
    pub fn ctor_4835d0(h: &mut impl Host, this: u32, a: u32, b: u32, c: u32) -> u32 {
        let r = h.call_sym(STREAM_CTOR, Conv::Thiscall(this + 8), &[c]);
        h.write(this, a);
        h.write(this + 4, b);
        r
    }

    /// `0x004d3700(a, b)`: whether `a`'s second word is below `b`'s
    /// (unsigned), 1 or 0 (a sort's comparator).
    pub fn less_by_second(m: &impl Mem, a: u32, b: u32) -> u32 {
        u32::from(m.read::<u32>(a + 4) < m.read::<u32>(b + 4))
    }

    /// `0x0047fac0(this, a, b, c, d, _)`: `this`'s virtual slot 1 with
    /// `a`..`d` (a fifth word is popped, not passed); its `eax`.
    pub fn forward_visit(h: &mut impl Host, this: u32, args: [u32; 4]) -> u32 {
        h.call_virtual(this, 1, &args)
    }

    /// `0x00482d70(data, count)`: `count` 12-byte elements from `data`
    /// get `0, 0, 0xff34` (a `u16`), each unless its address is 0.
    pub fn init_elements(m: &mut impl Mem, data: u32, count: u32) {
        let mut e = data;
        for _ in 0..count {
            if e != 0 {
                m.write(e, 0u32);
                m.write(e + 4, 0u32);
                m.write(e + 8, 0xff34u16);
            }
            e = e.wrapping_add(12);
        }
    }

    /// `0x004bd770(this, key)`: the first 16-byte entry of the array at
    /// `this + 0x14` (count, data) whose first word is `key`, or 0 (also
    /// for key 0).
    pub fn find_entry(m: &impl Mem, this: u32, key: u32) -> u32 {
        if key == 0 {
            return 0;
        }
        let data: u32 = m.read(this + 0x18);
        let end = (m.read::<u32>(this + 0x14) & 0x7fff_ffff)
            .wrapping_shl(4)
            .wrapping_add(data);
        let mut e = data;
        while e < end {
            if m.read::<u32>(e) == key {
                return e;
            }
            e = e.wrapping_add(16);
        }
        0
    }

    /// `0x004bc850(data, count)`: unless `data` is 0, each 0x254-byte
    /// `PShaderPass` destroyed, then `PhyreFree(data)`.
    pub fn release_shader_passes(h: &mut impl Host, data: u32, count: u32) {
        if data == 0 {
            return;
        }
        let mut e = data;
        for _ in 0..count {
            h.call_sym(SHADER_PASS_DTOR, Conv::Thiscall(e), &[]);
            e = e.wrapping_add(0x254);
        }
        h.call_sym(FREE, Conv::Cdecl, &[data]);
    }

    /// `PWorld_classEntry(world, descriptor)`: the world's entry for the
    /// class (`world +0xc` + id * 20, id at `descriptor +0x80`), or 0 when
    /// the id is not below the count (`world +8`, without bit 31).
    pub fn class_entry(m: &impl Mem, world: u32, descriptor: u32) -> u32 {
        let id: u32 = m.read(descriptor + 0x80);
        if id >= m.read::<u32>(world + 8) & 0x7fff_ffff {
            return 0;
        }
        m.read::<u32>(world + 0xc).wrapping_add(id.wrapping_mul(20))
    }

    /// `0x00443150(data, count)`: `count` 20-byte list heads (both links
    /// to the element, two words 0, -1), each unless its address is 0.
    pub fn init_list_heads(m: &mut impl Mem, data: u32, count: u32) {
        let mut e = data;
        for _ in 0..count {
            if e != 0 {
                m.write(e, e);
                m.write(e + 4, e);
                m.write(e + 8, 0u32);
                m.write(e + 0xc, 0u32);
                m.write(e + 0x10, u32::MAX);
            }
            e = e.wrapping_add(20);
        }
    }

    /// `0x00497db0(data, count)`: `count` 16-byte elements zeroed, each
    /// unless its address is 0.
    pub fn zero_elements16(m: &mut impl Mem, data: u32, count: u32) {
        let mut e = data;
        for _ in 0..count {
            if e != 0 {
                for i in 0..4 {
                    m.write(e + 4 * i, 0u32);
                }
            }
            e = e.wrapping_add(16);
        }
    }

    /// `0x0049f450(dst, src, count)`: `count` 16-byte elements copied; a
    /// destination at address 0 is skipped and the source stays.
    pub fn copy_elements16(m: &mut impl Mem, dst: u32, src: u32, count: u32) {
        let (mut d, mut s) = (dst, src);
        for _ in 0..count {
            if d != 0 {
                for i in 0..4 {
                    let w: u32 = m.read(s + 4 * i);
                    m.write(d + 4 * i, w);
                }
                s = s.wrapping_add(16);
            }
            d = d.wrapping_add(16);
        }
    }

    /// `0x00480bb0(dst, src, count)`: `count` 12-byte elements copied as
    /// two words and two bytes; a destination at address 0 is skipped and
    /// the source stays.
    pub fn copy_elements10(m: &mut impl Mem, dst: u32, src: u32, count: u32) {
        let (mut d, mut s) = (dst, src);
        for _ in 0..count {
            if d != 0 {
                let a: u32 = m.read(s);
                m.write(d, a);
                let b: u32 = m.read(s + 4);
                m.write(d + 4, b);
                let c: u8 = m.read(s + 8);
                m.write(d + 8, c);
                let e: u8 = m.read(s + 9);
                m.write(d + 9, e);
                s = s.wrapping_add(12);
            }
            d = d.wrapping_add(12);
        }
    }

    /// `0x00434360(data, count)`: unless `data` is 0, each word's block
    /// freed when its bit 0 is clear, then `data`.
    pub fn release_tagged_words(h: &mut impl Host, data: u32, count: u32) {
        if data == 0 {
            return;
        }
        for i in 0..count {
            let block: u32 = h.read(data.wrapping_add(4 * i));
            if block & 1 == 0 {
                h.call_sym(FREE, Conv::Cdecl, &[block]);
            }
        }
        h.call_sym(FREE, Conv::Cdecl, &[data]);
    }

    /// A type's create callback `(out, name)`: `find(name)` (cdecl) into
    /// `out`; 0 when found, else 0x13.
    pub fn create_result(h: &mut impl Host, find: u32, out: u32, name: u32) -> u32 {
        let found = h.call_sym(find, Conv::Cdecl, &[name]);
        h.write(out, found);
        if found == 0 { 0x13 } else { 0 }
    }

    /// Registers `obj` (`+0` its name, `+4` the link) at the head of the
    /// list at the global `head`: 0x11 without a name, 0x12 when
    /// `find(name)` already knows it, else 0. With a `table` global, `obj`
    /// is first stored there at index `obj +8`.
    pub fn register_named(
        h: &mut impl Host,
        obj: u32,
        find: u32,
        head: u32,
        table: Option<u32>,
    ) -> u32 {
        if let Some(table) = table {
            let index: u32 = h.read(obj + 8);
            let at = h.rebase(table).wrapping_add(index.wrapping_mul(4));
            h.write(at, obj);
        }
        let name: u32 = h.read(obj);
        if name == 0 {
            return 0x11;
        }
        if h.call_sym(find, Conv::Cdecl, &[name]) != 0 {
            return 0x12;
        }
        let head = h.rebase(head);
        let first: u32 = h.read(head);
        h.write(obj + 4, first);
        h.write(head, obj);
        0
    }

    /// A camera's deleting destructor `(this, flags)` -> `this`: its
    /// vtable, the block at `+0xd0` freed unless tagged (bit 0), `this`
    /// freed when flags bit 0 is set.
    pub fn camera_delete(h: &mut impl Host, this: u32, flags: u32, vtable: u32) -> u32 {
        let vtable = h.rebase(vtable);
        h.write(this, vtable);
        let block: u32 = h.read(this + 0xd0);
        if block & 1 == 0 {
            h.call_sym(FREE, Conv::Cdecl, &[block]);
        }
        if flags & 1 != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[this]);
        }
        this
    }

    /// `0x004315e0(this)` -> `this`: byte 0, a `PEventFlagWin32` at `+4`,
    /// byte `+8` 0, three words 0.
    pub fn event_owner_ctor(h: &mut impl Host, this: u32) -> u32 {
        h.write(this, 0u8);
        h.call_sym(EVENT_FLAG_CTOR, Conv::Thiscall(this + 4), &[]);
        h.write(this + 8, 0u8);
        h.write(this + 0xc, 0u32);
        h.write(this + 0x10, 0u32);
        h.write(this + 0x14, 0u32);
        this
    }

    /// `0x00463d10(this)` -> `this`: eight words 0.
    pub fn clear_32(m: &mut impl Mem, this: u32) -> u32 {
        for i in 0..8 {
            m.write(this + 4 * i, 0u32);
        }
        this
    }

    /// Whether the C strings at `a` and `b` are equal.
    fn same_str(m: &impl Mem, a: u32, b: u32) -> bool {
        let mut i = 0u32;
        loop {
            let x: u8 = m.read(a.wrapping_add(i));
            if x != m.read::<u8>(b.wrapping_add(i)) {
                return false;
            }
            if x == 0 {
                return true;
            }
            i = i.wrapping_add(1);
        }
    }

    /// The first entry of the list at the global `head` (links at
    /// `next_off`) whose name (the string at `name_off`) equals `name`, or
    /// 0 (also for a 0 `name`).
    pub fn find_named(m: &impl Mem, head: u32, name_off: u32, next_off: u32, name: u32) -> u32 {
        if name == 0 {
            return 0;
        }
        let mut e: u32 = m.read(m.rebase(head));
        while e != 0 {
            if same_str(m, m.read(e + name_off), name) {
                return e;
            }
            e = m.read(e + next_off);
        }
        0
    }

    /// `0x004d37e0(name)`: as [`find_named`] (name `+4`, link `+8`) with
    /// the comparison the import at `compare` makes (cdecl, 0 = equal).
    pub fn find_named_by(h: &mut impl Host, head: u32, compare: u32, name: u32) -> u32 {
        if name == 0 {
            return 0;
        }
        let head = h.rebase(head);
        let mut e: u32 = h.read(head);
        if e == 0 {
            return 0;
        }
        let slot = h.rebase(compare);
        let f: u32 = h.read(slot);
        while e != 0 {
            let entry_name: u32 = h.read(e + 4);
            if h.call(f, Conv::Cdecl, &[entry_name, name]) == 0 {
                return e;
            }
            e = h.read(e + 8);
        }
        0
    }

    /// A `PSharray` of 20-byte elements at `this + 0x14` (count, data):
    /// the address of element `i`'s `+0x10` (the single element lives in the
    /// header, at `this + 0x18`; with none, from address 0).
    pub fn sharray_element(m: &impl Mem, this: u32, i: u32) -> u32 {
        sharray_element_at(m, this + 0x14, i)
    }

    /// [`sharray_element`] of the `PSharray` header at `array` (count,
    /// data).
    pub fn sharray_element_at(m: &impl Mem, array: u32, i: u32) -> u32 {
        let n = m.read::<u32>(array) & 0x7fff_ffff;
        let base = match n {
            0 => 0,
            1 => array + 4,
            _ => m.read(array + 4),
        };
        base.wrapping_add(i.wrapping_mul(20)).wrapping_add(16)
    }

    /// `0x00442290(dst, src, count)`: as [`copy_elements16`] with 20-byte
    /// elements.
    pub fn copy_elements20(m: &mut impl Mem, dst: u32, src: u32, count: u32) {
        let (mut d, mut s) = (dst, src);
        for _ in 0..count {
            if d != 0 {
                for i in 0..5 {
                    let w: u32 = m.read(s + 4 * i);
                    m.write(d + 4 * i, w);
                }
                s = s.wrapping_add(20);
            }
            d = d.wrapping_add(20);
        }
    }

    /// `PDynamicMesh::getDataBlockCount(mesh, kind)`: of the kind's run of
    /// 12-byte blocks (`mesh +0x14` + 16 * kind: first, count; blocks at
    /// `mesh +0xc`, `mesh +8` of them), how many have a 0 first word. A
    /// first index out of range reads from address 0, as the original.
    pub fn data_block_count(m: &impl Mem, mesh: u32, kind: u32) -> u32 {
        let run = m
            .read::<u32>(mesh + 0x14)
            .wrapping_add(kind.wrapping_mul(16));
        let first: u32 = m.read(run);
        let n: u32 = m.read(run + 4);
        let mut e = if first < m.read::<u32>(mesh + 8) & 0x7fff_ffff {
            m.read::<u32>(mesh + 0xc)
                .wrapping_add(first.wrapping_mul(12))
        } else {
            0
        };
        let mut count = 0;
        for _ in 0..n {
            if m.read::<u32>(e) == 0 {
                count += 1;
            }
            e = e.wrapping_add(12);
        }
        count
    }

    /// `PCamera::ConvertProjectionMatrixDepthRangeToZeroOne(camera, near,
    /// far)` (cdecl, `f32` bits): with a 0 at `camera +0x3c` (a
    /// perspective projection), `+0x28` = far / (near - far) and `+0x38` =
    /// far * near / (near - far); else `+0x28` = 1 / (near - far) and
    /// `+0x38` = near / (near - far); on the x87 at 53 bits.
    pub fn depth_range_zero_one(m: &mut impl Mem, camera: u32, near: u32, far: u32) {
        let (n, f) = (x87::load(near), x87::load(far));
        let d = x87::sub(n, f);
        let (a, b) = if x87::load(m.read(camera + 0x3c)) == 0.0 {
            (x87::div(f, d), x87::div(x87::mul(f, n), d))
        } else {
            (x87::div(1.0, d), x87::div(n, d))
        };
        m.write(camera + 0x28, x87::store(a));
        m.write(camera + 0x38, x87::store(b));
    }

    /// `PMaterial::PMaterial(this, a, b)` -> `this`: `a`, `b`, two words
    /// 0, a one-byte block (`PhyreMalloc(1, 2)`, its byte from the image)
    /// and 0.
    pub fn material_ctor(h: &mut impl Host, this: u32, a: u32, b: u32) -> u32 {
        h.write(this, a);
        h.write(this + 4, b);
        h.write(this + 8, 0u32);
        h.write(this + 0xc, 0u32);
        let block = h.call_sym(MALLOC, Conv::Cdecl, &[1, 2]);
        if block != 0 {
            let at = h.rebase(MATERIAL_BYTE);
            let byte: u8 = h.read(at);
            h.write(block, byte);
        }
        h.write(this + 0x10, block);
        h.write(this + 0x14, 0u32);
        this
    }

    /// `0x0048c540(array, i, v)`: a `PSharray` of words' element `i` set
    /// to `v`: 0, or 3 when `i` is not below the count.
    pub fn sharray_set(m: &mut impl Mem, array: u32, i: u32, v: u32) -> u32 {
        let n = m.read::<u32>(array) & 0x7fff_ffff;
        if i >= n {
            return 3;
        }
        let base = if n > 1 { m.read(array + 4) } else { array + 4 };
        m.write(base.wrapping_add(i.wrapping_mul(4)), v);
        0
    }

    /// `0x0048c6e0(this, buffer, flag)` -> 0: with the default buffer of
    /// the global defaults object (its virtual slot 1), `buffer == this +
    /// 4` and `flag == 1`, `applyDefaultBufferWithValidation(default,
    /// buffer, 1)`; else the member's own apply `0x0048c650(this, buffer,
    /// flag)`.
    pub fn apply_defaults(h: &mut impl Host, this: u32, buffer: u32, flag: u32) -> u32 {
        let object = h.rebase(DEFAULTS_OBJECT);
        let default = h.call_virtual(object, 1, &[]);
        if buffer == this + 4 && flag == 1 && default != 0 {
            h.call_sym(APPLY_DEFAULTS, Conv::Thiscall(default), &[buffer, flag]);
        } else {
            h.call_sym(APPLY_MEMBER, Conv::Thiscall(this), &[buffer, flag]);
        }
        0
    }

    /// `0x00463bf0(this, list)` -> `this`: an iterator over the list head
    /// at `list + 0x1c`: five words 0, the head, its first entry (0 when
    /// the list is empty), the `PTexture2D` tag; then its first step.
    pub fn iterator_ctor(h: &mut impl Host, this: u32, list: u32) -> u32 {
        let head = list.wrapping_add(0x1c);
        for i in 0..5 {
            h.write(this + 4 * i, 0u32);
        }
        h.write(this + 0x14, head);
        let first: u32 = h.read(head);
        h.write(this + 0x18, if first == head { 0 } else { first });
        let tag = h.rebase(ITERATOR_TAG);
        h.write(this + 0x1c, tag);
        h.call_sym(ITERATOR_FIRST, Conv::Thiscall(this), &[]);
        this
    }

    /// `PUtilitySprite::PUtilitySprite(this)` -> `this`: as a `PUtility`
    /// of priority 0xff linked into the utilities list before the first
    /// one of priority 0xff or more, then its own vtable and member.
    pub fn utility_sprite_ctor(h: &mut impl Host, this: u32) -> u32 {
        let utility = h.rebase(UTILITY_VTABLES[0]);
        let table = h.rebase(UTILITY_VTABLES[1]);
        let sprite = h.rebase(UTILITY_VTABLES[2]);
        h.write(this, utility);
        h.write(this + 8, table);
        h.write(this + 0xc, 0xffu32);
        let mut link = h.rebase(UTILITIES);
        if h.read::<u32>(link) != 0 {
            loop {
                let e: u32 = h.read(link);
                if h.read::<u32>(e + 0xc) >= 0xff {
                    break;
                }
                link = e + 4;
                if h.read::<u32>(e + 4) == 0 {
                    break;
                }
            }
        }
        let next: u32 = h.read(link);
        h.write(this + 4, next);
        h.write(link, this);
        h.write(this, sprite);
        h.call_sym(SPRITE_MEMBER_CTOR, Conv::Thiscall(this + 0x10), &[]);
        this
    }

    /// `0x00483810(this, buffer, count)`: the array at `this + 8` set to
    /// the external `buffer` (an owned old block other than it freed): 0;
    /// 5 for a negative count.
    pub fn set_external_at8(h: &mut impl Host, this: u32, buffer: u32, count: u32) -> u32 {
        if (count as i32) < 0 {
            return 5;
        }
        let old: u32 = h.read(this + 8);
        let data: u32 = h.read(this + 0xc);
        if data != buffer && old & 0x8000_0000 == 0 && data != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[data]);
        }
        h.write(this + 0xc, buffer);
        h.write(this + 8, count | 0x8000_0000);
        0
    }

    /// `PWorld`'s destructor body (`0x00442a80(this)`): each node of the
    /// list at `this` (links at node `+0x48`, the node 0x48 before them)
    /// gets `+0x40` = 0; then the class entries' block freed when owned,
    /// and the array emptied.
    pub fn world_release(h: &mut impl Host, this: u32) {
        let mut link: u32 = h.read(this);
        while link != this && link != 0 {
            let node = link.wrapping_sub(0x48);
            if node == 0 {
                break;
            }
            h.write(node + 0x40, 0u32);
            link = h.read(node + 0x48);
        }
        let count: u32 = h.read(this + 8);
        if count & 0x8000_0000 == 0 {
            let data: u32 = h.read(this + 0xc);
            if data != 0 {
                h.call_sym(FREE, Conv::Cdecl, &[data]);
            }
        }
        h.write(this + 0xc, 0u32);
        h.write(this + 8, 0u32);
    }

    /// `PConstantBuffer`'s destructor (`this`): `release(this)` when
    /// `check(this)` sets `al`, then `dtor(this)`, whose `eax` it returns.
    pub fn dtor_checked(h: &mut impl Host, [check, release, dtor]: [u32; 3], this: u32) -> u32 {
        if h.call_sym(check, Conv::Thiscall(this), &[]) & 0xff != 0 {
            h.call_sym(release, Conv::Thiscall(this), &[]);
        }
        h.call_sym(dtor, Conv::Thiscall(this), &[])
    }

    /// `0x00433900(this)`: an array of 0x40-byte elements at `this`
    /// (count, data): when owned and allocated, each element's destructor
    /// (`0x0055dfa0`, thiscall) and `PhyreFree(data)`; then emptied.
    pub fn release_array40(h: &mut impl Host, this: u32) {
        let count: u32 = h.read(this);
        if count & 0x8000_0000 == 0 {
            let data: u32 = h.read(this + 4);
            if data != 0 {
                let mut e = data;
                for _ in 0..count & 0x7fff_ffff {
                    h.call_sym(ELEMENT40_DTOR, Conv::Thiscall(e), &[]);
                    e = e.wrapping_add(0x40);
                }
                h.call_sym(FREE, Conv::Cdecl, &[data]);
            }
        }
        h.write(this + 4, 0u32);
        h.write(this, 0u32);
    }

    /// `0x004d57d0(this, parameter, src)`: copies the parameter's bytes
    /// (`u16` offset `+8`, size the low 13 bits of `+0xa`) from `src` into
    /// the buffer at `this` (`+0` its size) with the CRT's `memcpy`: 0;
    /// 3 when they don't fit or the offset is below 4; 1 without `src`.
    pub fn write_parameter(h: &mut impl Host, this: u32, parameter: u32, src: u32) -> u32 {
        if src == 0 {
            return 1;
        }
        let off = u32::from(h.read::<u16>(parameter + 8));
        let size = u32::from(h.read::<u16>(parameter + 0xa) & 0x1fff);
        if off + size > h.read::<u32>(this) || off < 4 {
            return 3;
        }
        h.call_sym(MEMCPY, Conv::Cdecl, &[this.wrapping_add(off), src, size]);
        0
    }

    /// `0x004bbe40(this)`: the array at `this` (count, data) of 0x254-byte
    /// `PShaderPass`es: when owned and allocated, each one's destructor
    /// and `PhyreFree(data)`; then emptied.
    pub fn release_shader_pass_array(h: &mut impl Host, this: u32) {
        let count: u32 = h.read(this);
        if count & 0x8000_0000 == 0 {
            let data: u32 = h.read(this + 4);
            if data != 0 {
                let mut e = data;
                for _ in 0..count & 0x7fff_ffff {
                    h.call_sym(SHADER_PASS_DTOR, Conv::Thiscall(e), &[]);
                    e = e.wrapping_add(0x254);
                }
                h.call_sym(FREE, Conv::Cdecl, &[data]);
            }
        }
        h.write(this + 4, 0u32);
        h.write(this, 0u32);
    }

    /// `0x00433700(this)` -> `this`: fourteen words 0.
    pub fn clear_56(m: &mut impl Mem, this: u32) -> u32 {
        for i in 0..14 {
            m.write(this + 4 * i, 0u32);
        }
        this
    }

    /// `PRenderTargetBase::getViewportSize(this, width, height)`: by the
    /// kind byte `+4` of the target reference (`+0` the object): 0 and 1
    /// the object's `+0x1c`, `+0x20`; 2 (a cube map) its `+0x1c` for both;
    /// 0x15 without an object; other kinds 0 without writing.
    pub fn viewport_size(m: &mut impl Mem, this: u32, width: u32, height: u32) -> u32 {
        let kind: u8 = m.read(this + 4);
        if kind > 2 {
            return 0;
        }
        let object: u32 = m.read(this);
        if object == 0 {
            return 0x15;
        }
        if kind == 2 {
            let size: u32 = m.read(object + 0x1c);
            m.write(height, size);
            m.write(width, size);
        } else {
            let w: u32 = m.read(object + 0x1c);
            m.write(width, w);
            let h: u32 = m.read(object + 0x20);
            m.write(height, h);
        }
        0
    }

    /// `0x00498500(ctx, _, segment, blocks, indirect, extra)` (stdcall):
    /// with an `indirect` draw, `DRAW_INDEXED_INSTANCED(ctx; segment +0x10,
    /// indirect +8, indirect, extra)`; else `DRAW_INDEXED(ctx; segment
    /// +0x10, 0, first)` where `first` is the first nonzero `+4` word of
    /// the 0x40-byte entries of the `blocks` array (or the last read, 0).
    /// That call's `eax`.
    pub fn draw_segment(
        h: &mut impl Host,
        ctx: u32,
        segment: u32,
        blocks: u32,
        indirect: u32,
        extra: u32,
    ) -> u32 {
        let mut first = 0;
        if indirect == 0 {
            let n = h.read::<u32>(blocks) & 0x7fff_ffff;
            if n != 0 {
                let mut e = h.read::<u32>(blocks + 4).wrapping_add(4);
                for _ in 0..n {
                    first = h.read(e);
                    if first != 0 {
                        break;
                    }
                    e = e.wrapping_add(0x40);
                }
            }
        } else {
            first = h.read(indirect + 8);
        }
        let stream: u32 = h.read(segment + 0x10);
        if indirect != 0 {
            h.call_sym(
                DRAW_INSTANCED,
                Conv::Thiscall(ctx),
                &[stream, first, indirect, extra],
            )
        } else {
            h.call_sym(DRAW_INDEXED, Conv::Thiscall(ctx), &[stream, 0, first])
        }
    }

    /// `PWorld::addInstanceList(world, list)`: for each class of the list
    /// (descriptors from `list +0x2c`, linked at `+0x40`) that has an
    /// entry in the world (id `+0x80` below the count), the entry's size
    /// grows by `list +0x3c` and its count by one; the list's node `+0x20`
    /// is linked after the first such entry's head.
    pub fn add_instance_list(m: &mut impl Mem, world: u32, list: u32) {
        let mut linked = false;
        let mut d: u32 = m.read(list + 0x2c);
        while d != 0 {
            let id: u32 = m.read(d + 0x80);
            if id < m.read::<u32>(world + 8) & 0x7fff_ffff {
                let e = m.read::<u32>(world + 0xc).wrapping_add(id.wrapping_mul(20));
                if e != 0 {
                    if !linked {
                        let first: u32 = m.read(e);
                        let node = list + 0x20;
                        m.write(node, first);
                        m.write(node + 4, e);
                        m.write(e, node);
                        m.write(first + 4, node);
                        linked = true;
                    }
                    let size = m.read::<u32>(e + 8).wrapping_add(m.read(list + 0x3c));
                    m.write(e + 8, size);
                    let count = m.read::<u32>(e + 0xc).wrapping_add(1);
                    m.write(e + 0xc, count);
                }
            }
            d = m.read(d + 0x40);
        }
    }

    /// `PWorld::removeInstanceList(world, list)` -> 0: the reverse of
    /// [`add_instance_list`]: the list's node unlinked (and pointed at
    /// itself) at the first entry, each entry's size and count reduced.
    pub fn remove_instance_list(m: &mut impl Mem, world: u32, list: u32) -> u32 {
        let mut unlinked = false;
        let mut d: u32 = m.read(list + 0x2c);
        while d != 0 {
            let id: u32 = m.read(d + 0x80);
            if id < m.read::<u32>(world + 8) & 0x7fff_ffff {
                let e = m.read::<u32>(world + 0xc).wrapping_add(id.wrapping_mul(20));
                if e != 0 {
                    if !unlinked {
                        let node = list + 0x20;
                        let next: u32 = m.read(node);
                        let prev: u32 = m.read(node + 4);
                        m.write(next + 4, prev);
                        m.write(prev, next);
                        m.write(node, node);
                        m.write(node + 4, node);
                        unlinked = true;
                    }
                    let size = m.read::<u32>(e + 8).wrapping_sub(m.read(list + 0x3c));
                    m.write(e + 8, size);
                    let count = m.read::<u32>(e + 0xc).wrapping_sub(1);
                    m.write(e + 0xc, count);
                }
            }
            d = m.read(d + 0x40);
        }
        0
    }

    /// The instance lists of a cluster (its list head at `+0x1c`): the
    /// first, or 0 when the list is empty.
    fn first_list(m: &impl Mem, head: u32) -> u32 {
        let first: u32 = m.read(head);
        if first == head { 0 } else { first }
    }

    /// `PWorld::addCluster(world, cluster)`: out of its old world first
    /// (`cluster +0x40`), then linked into this world's list (node
    /// `cluster +0x48`), and each of its instance lists added.
    pub fn add_cluster(m: &mut impl Mem, world: u32, cluster: u32) {
        let old: u32 = m.read(cluster + 0x40);
        if old != 0 {
            remove_cluster(m, old, cluster);
        }
        let first: u32 = m.read(world);
        let node = cluster + 0x48;
        m.write(node, first);
        m.write(node + 4, world);
        m.write(world, node);
        m.write(first + 4, node);
        m.write(cluster + 0x40, world);
        let head = cluster + 0x1c;
        let mut list = first_list(m, head);
        while list != 0 {
            add_instance_list(m, world, list);
            let next: u32 = m.read(list);
            list = if next == head { 0 } else { next };
        }
    }

    /// `PWorld::removeCluster(world, cluster)`: when the cluster is in
    /// this world, unlinked (its node pointed at itself, `+0x40` 0) and
    /// each of its instance lists removed.
    pub fn remove_cluster(m: &mut impl Mem, world: u32, cluster: u32) {
        if m.read::<u32>(cluster + 0x40) != world {
            return;
        }
        let node = cluster + 0x48;
        let next: u32 = m.read(node);
        let prev: u32 = m.read(node + 4);
        m.write(next + 4, prev);
        m.write(prev, next);
        m.write(node, node);
        m.write(node + 4, node);
        m.write(cluster + 0x40, 0u32);
        let head = cluster + 0x1c;
        let mut list = first_list(m, head);
        while list != 0 {
            remove_instance_list(m, world, list);
            let next: u32 = m.read(list);
            list = if next == head { 0 } else { next };
        }
    }

    /// `0x004649d0()`: the renderer context (`0x00c94f00`), constructed
    /// (`0x004fee70`, thiscall) and registered with `atexit` on the first
    /// call (bit 0 of `0x00c9628c`).
    pub fn renderer_context(h: &mut impl Host) -> u32 {
        let guard = h.rebase(CONTEXT_GUARD);
        let obj = h.rebase(CONTEXT);
        let flags: u32 = h.read(guard);
        if flags & 1 == 0 {
            h.write(guard, flags | 1);
            h.call_sym(CONTEXT_CTOR, Conv::Thiscall(obj), &[]);
            let dtor = h.rebase(CONTEXT_DTOR);
            h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
        }
        obj
    }

    /// `0x004d7560(this, a, b)`: the cdecl function stored at `this +0x94`
    /// called with `(a, b)`; its `eax`.
    pub fn call_stored(h: &mut impl Host, this: u32, a: u32, b: u32) -> u32 {
        let f: u32 = h.read(this + 0x94);
        h.call(f, Conv::Cdecl, &[a, b])
    }
}

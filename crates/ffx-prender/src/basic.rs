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

/// The renderer id `0x00464710` returns: `"11XD"` read as a little-endian
/// word.
pub const RENDERER_ID: u32 = 0x4458_3131;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        Conv, DYNAMIC_MESH_VTABLE, EVENT_FLAG_CTOR, FREE, Host, Mem, NODE_REGISTER,
        SAMPLER_D3D11_CTOR, SHADER_PASS_DTOR, SPRITE_REGISTER, STREAM_CTOR,
    };

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
        let n = m.read::<u32>(this + 0x14) & 0x7fff_ffff;
        let base = match n {
            0 => 0,
            1 => this + 0x18,
            _ => m.read(this + 0x18),
        };
        base.wrapping_add(i.wrapping_mul(20)).wrapping_add(16)
    }
}

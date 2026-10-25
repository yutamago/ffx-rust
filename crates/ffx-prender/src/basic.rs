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
/// The renderer id `0x00464710` returns: `"11XD"` read as a little-endian
/// word.
pub const RENDERER_ID: u32 = 0x4458_3131;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        Conv, DYNAMIC_MESH_VTABLE, FREE, Host, Mem, NODE_REGISTER, SAMPLER_D3D11_CTOR,
        SHADER_PASS_DTOR, SPRITE_REGISTER, STREAM_CTOR,
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
}

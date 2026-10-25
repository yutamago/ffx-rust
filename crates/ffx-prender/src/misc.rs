//! Small render-side `PhyreEngine` methods: field getters and setters of
//! the render interface, materials, mesh-instance bounds and shader
//! passes, the count helpers of the shader and stream tables, a few
//! constructors and destructors, and constant vtable slots.
//!
//! Each follows its original's machine code (its stores in order, its
//! `eax`): a setter that leaves the last argument it loaded in `eax`
//! returns it, one that does not touch `eax` returns nothing.

use ffx_graphic::host::{Conv, Host};

/// `Phyre::PhyreFree(p)` (cdecl).
pub const PHYRE_FREE: u32 = crate::descriptor::PHYRE_FREE;

/// `PDynamicMeshInstance`'s vtable.
pub const DYNAMIC_MESH_INSTANCE_VTABLE: u32 = 0x00b2_3700;

/// The list of scene render pass types (`0x00ca54f8`: first; `+4` next,
/// `+0xc` the lookup index).
pub const RENDER_PASS_TYPES: u32 = 0x00ca_54f8;

pub mod bridge {
    //! The originals over game memory.

    use super::{Conv, DYNAMIC_MESH_INSTANCE_VTABLE, Host, PHYRE_FREE, RENDER_PASS_TYPES};
    use ffx_bridge::x87;

    /// `fld dword [from]; fstp dword [to]`.
    fn copy_float(m: &mut impl Host, to: u32, from: u32) {
        let bits: u32 = m.read(from);
        m.write(to, x87::store(x87::load(bits)));
    }

    /// `PMaterial::setEffectVariant(material, a, b)` and
    /// `setSceneRenderPassRemapping` (`at` 0 and 8): two words; returns
    /// `b` (left in `eax`).
    pub fn set_pair(m: &mut impl Host, this: u32, at: u32, a: u32, b: u32) -> u32 {
        m.write(this + at, a);
        m.write(this + at + 4, b);
        b
    }

    /// `PSceneRenderPassType::InvalidateLookupIndices()`: every type's
    /// lookup index is -1; returns the list's end (0).
    pub fn invalidate_lookup_indices(m: &mut impl Host) -> u32 {
        let mut t: u32 = m.read(m.rebase(RENDER_PASS_TYPES));
        while t != 0 {
            m.write(t + 0xc, u32::MAX);
            t = m.read(t + 4);
        }
        0
    }

    /// `0x004df9c0(out, key)`: `*out = find(key)` (cdecl); 0 when found,
    /// `0x13` when not.
    pub fn lookup_into(h: &mut impl Host, out: u32, key: u32, find: u32) -> u32 {
        let f = h.rebase(find);
        let r = h.call(f, Conv::Cdecl, &[key]);
        h.write(out, r);
        if r == 0 { 0x13 } else { 0 }
    }

    /// `0x004f30d0(v, out, s)`: `out[i] = v[i] * s` for four floats (x87:
    /// the product exact, rounded at each store); returns `out`.
    pub fn scale4(m: &mut impl Host, v: u32, out: u32, s: u32) -> u32 {
        let s = x87::load(s);
        for i in 0..4 {
            let x = x87::load(m.read(v + 4 * i));
            m.write(out + 4 * i, x87::store(x87::mul(x, s)));
        }
        out
    }

    /// A count word masked to 31 bits, 1 for an empty table
    /// (`PConstantBufferNVN::getBufferCount` and its neighbours).
    pub fn count_or_one(m: &mut impl Host, this: u32, at: u32) -> u32 {
        match m.read::<u32>(this + at) & 0x7fff_ffff {
            0 => 1,
            n => n,
        }
    }

    /// `PDynamicMeshInstance::PDynamicMeshInstance(this, owner)`: the
    /// vtable, `+4 = owner`, `+8 = 0`; returns `this`.
    pub fn dynamic_mesh_instance(m: &mut impl Host, this: u32, owner: u32) -> u32 {
        let v = m.rebase(DYNAMIC_MESH_INSTANCE_VTABLE);
        m.write(this, v);
        m.write(this + 4, owner);
        m.write(this + 8, 0u32);
        this
    }

    /// `PDynamicMeshInstance`'s destructor body: the vtable.
    pub fn dynamic_mesh_instance_dtor(m: &mut impl Host, this: u32) {
        let v = m.rebase(DYNAMIC_MESH_INSTANCE_VTABLE);
        m.write(this, v);
    }

    /// The scalar deleting destructors `(this, flags)`: `dtor(this)`
    /// (thiscall; or, with no `dtor`, `PDynamicMeshInstance`'s body), then
    /// `Phyre_PhyreFree(this)` when `flags & 1`; returns `this`.
    pub fn scalar_deleting_dtor(
        h: &mut impl Host,
        this: u32,
        flags: u32,
        dtor: Option<u32>,
    ) -> u32 {
        match dtor {
            Some(d) => {
                let f = h.rebase(d);
                h.call(f, Conv::Thiscall(this), &[]);
            }
            None => dynamic_mesh_instance_dtor(h, this),
        }
        if flags & 1 != 0 {
            let f = h.rebase(PHYRE_FREE);
            h.call(f, Conv::Cdecl, &[this]);
        }
        this
    }

    /// Zeroes `words` words at `this` and, when `byte` is given, the byte
    /// at that offset; returns `this` (the small constructors).
    pub fn zero_init(m: &mut impl Host, this: u32, words: u32, byte: Option<u32>) -> u32 {
        for i in 0..words {
            m.write(this + 4 * i, 0u32);
        }
        if let Some(b) = byte {
            m.write(this + b, 0u8);
        }
        this
    }

    /// `0x00583420(this)`: `Phyre_ZeroInit8_005031e0(this)` (thiscall),
    /// then `+8..+0x10 = 0`, byte `+0x14 = 0`; returns `this`.
    pub fn zero_init21(h: &mut impl Host, this: u32) -> u32 {
        let f = h.rebase(0x0050_31e0);
        h.call(f, Conv::Thiscall(this), &[]);
        for off in [8, 0xc, 0x10] {
            h.write(this + off, 0u32);
        }
        h.write(this + 0x14, 0u8);
        this
    }

    /// A constructor that runs `base(this)` (thiscall), then zeroes the
    /// words at `zero`; returns `this`.
    pub fn ctor_then_zero(h: &mut impl Host, this: u32, base: u32, zero: &[u32]) -> u32 {
        let f = h.rebase(base);
        h.call(f, Conv::Thiscall(this), &[]);
        for &off in zero {
            h.write(this + off, 0u32);
        }
        this
    }

    /// `0x005844a0(this, a..h)`: `base(this; a, b, c)` (thiscall), then
    /// `+0x14..+0x24 = d..h`; returns `this`.
    pub fn ctor8(h: &mut impl Host, this: u32, args: [u32; 8], base: u32) -> u32 {
        let f = h.rebase(base);
        h.call(f, Conv::Thiscall(this), &args[..3]);
        for (i, &v) in args[3..].iter().enumerate() {
            h.write(this + 0x14 + 4 * i as u32, v);
        }
        this
    }

    /// `PMeshInstanceBounds::setMinAndSize(bounds, min, size)`: three
    /// floats of each (`+0` and `+0x10`); returns `size` (left in `eax`).
    pub fn set_min_and_size(m: &mut impl Host, this: u32, min: u32, size: u32) -> u32 {
        for i in 0..3 {
            copy_float(m, this + 4 * i, min + 4 * i);
        }
        for i in 0..3 {
            copy_float(m, this + 0x10 + 4 * i, size + 4 * i);
        }
        size
    }

    /// `PRenderInterfaceNVN::getViewportSize(this, w, h, n)`: `*w =
    /// +0x78`, `*h = +0x7c`, `*n = 3 * +0x78` (read again after the
    /// stores); returns 0.
    pub fn viewport_size(m: &mut impl Host, this: u32, w: u32, hh: u32, n: u32) -> u32 {
        let v: u32 = m.read(this + 0x78);
        m.write(w, v);
        let v: u32 = m.read(this + 0x7c);
        m.write(hh, v);
        let v: u32 = m.read(this + 0x78);
        m.write(n, v.wrapping_mul(3));
        0
    }

    /// `0x005980c0(this, a)`: `+0x31c = a`, four words cleared; returns 0.
    pub fn set_target(m: &mut impl Host, this: u32, a: u32) -> u32 {
        m.write(this + 0x31c, a);
        for off in [0x330, 0x334, 0x358, 0x35c] {
            m.write(this + off, 0u32);
        }
        0
    }

    /// `PShaderPassNVN::releaseShaderPass(this, pass)`: `pass +0x10..+0x1c`
    /// and `+0xc` cleared; returns 0.
    pub fn release_shader_pass(m: &mut impl Host, pass: u32) -> u32 {
        for off in [0x10, 0x14, 0x18, 0x1c, 0xc] {
            m.write(pass + off, 0u32);
        }
        0
    }

    /// `0x00599d50(this, r, g, b, a)`: four floats at `+0x37c`.
    pub fn set_clear_color(m: &mut impl Host, this: u32, c: [u32; 4]) {
        for (i, &v) in c.iter().enumerate() {
            m.write(this + 0x37c + 4 * i as u32, x87::store(x87::load(v)));
        }
    }

    /// `PRenderInterfaceNVN::setClearDepth(this, d)`: `+0x38c` (a float).
    pub fn set_clear_depth(m: &mut impl Host, this: u32, d: u32) {
        m.write(this + 0x38c, x87::store(x87::load(d)));
    }

    /// `0x0058ab10(m)`: the mode's code (the jump table at `0x0058ab40`).
    #[must_use]
    pub fn map_mode(mode: u32) -> u32 {
        match mode {
            1 => 1,
            3 => 4,
            4 => 2,
            _ => 3,
        }
    }

    /// `0x0058af90(items, n)`: zeroes `n` 16-byte items (each whose address
    /// is not null).
    pub fn zero_array16(m: &mut impl Host, items: u32, n: u32) {
        let mut e = items;
        for _ in 0..n {
            if e != 0 {
                for i in 0..4 {
                    m.write(e + 4 * i, 0u32);
                }
            }
            e = e.wrapping_add(0x10);
        }
    }

    /// `0x0058e5b0(this, a)`: `f(this + 0x320; [this + 0x70], a)`
    /// (thiscall), its `eax`.
    pub fn forward_593930(h: &mut impl Host, this: u32, a: u32, f: u32) -> u32 {
        let first: u32 = h.read(this + 0x70);
        let f = h.rebase(f);
        h.call(f, Conv::Thiscall(this + 0x320), &[first, a])
    }

    /// `0x00593db0`/`0x00594060(this, i)`: for `i` below the table's count
    /// (`+0x24`, 31 bits), the word at `field` of `get(this + 0x24; i)`
    /// (thiscall); else 0.
    pub fn element_field(h: &mut impl Host, this: u32, i: u32, get: u32, field: u32) -> u32 {
        let n = h.read::<u32>(this + 0x24) & 0x7fff_ffff;
        if i >= n {
            return 0;
        }
        let f = h.rebase(get);
        let e = h.call(f, Conv::Thiscall(this + 0x24), &[i]);
        h.read(e.wrapping_add(field))
    }

    /// `0x0059a120(this, b, a)`: `+0x59c = b as u8`, `+0x5a4 = map(a)`
    /// (cdecl); returns 0.
    pub fn set_mode(h: &mut impl Host, this: u32, b: u32, a: u32, map: u32) -> u32 {
        h.write(this + 0x59c, b & 0xff);
        let f = h.rebase(map);
        let v = h.call(f, Conv::Cdecl, &[a]);
        h.write(this + 0x5a4, v);
        0
    }

    /// `0x0059aff0(obj, a)`: when `obj + 0x10` is set, `f(that; obj, a)`
    /// (thiscall); returns 0.
    pub fn forward_if_bound(h: &mut impl Host, obj: u32, a: u32, f: u32) -> u32 {
        let t: u32 = h.read(obj + 0x10);
        if t != 0 {
            let f = h.rebase(f);
            h.call(f, Conv::Thiscall(t), &[obj, a]);
        }
        0
    }

    /// `0x0059bbf0(this, x, y, w, h)`: the four at `+0x1324`, then
    /// `f(this; x, y, w, h)` (thiscall), its `eax`.
    pub fn set_viewport(h: &mut impl Host, this: u32, r: [u32; 4], f: u32) -> u32 {
        for (i, &v) in r.iter().enumerate() {
            h.write(this + 0x1324 + 4 * i as u32, v);
        }
        let f = h.rebase(f);
        h.call(f, Conv::Thiscall(this), &r)
    }

    /// The word at `this + at` (`PRenderInterface::getTextureQuality`).
    pub fn field(m: &mut impl Host, this: u32, at: u32) -> u32 {
        m.read(this + at)
    }

    /// Bit 0 of the word at `this + at` (the NVN data blocks' `isBound`).
    pub fn field_bit0(m: &mut impl Host, this: u32, at: u32) -> u32 {
        m.read::<u32>(this + at) & 1
    }

    /// 1 when the word at `this + at` is set (`isBound` of the D3D11
    /// buffers and textures: `xor eax, eax; cmp; setne al`).
    pub fn field_nonzero(m: &mut impl Host, this: u32, at: u32) -> u32 {
        u32::from(m.read::<u32>(this + at) != 0)
    }

    /// `this + at = v`; returns `v` (left in `eax`).
    pub fn set_field(m: &mut impl Host, this: u32, at: u32, v: u32) -> u32 {
        m.write(this + at, v);
        v
    }

    /// The byte at `this + at = v`; returns 0 (`Renderer_SetVSync`).
    pub fn set_byte_ret0(m: &mut impl Host, this: u32, at: u32, v: u32) -> u32 {
        m.write(this + at, v as u8);
        0
    }

    /// A thiscall `f(this)` that returns `this` (`0x004fee70`).
    pub fn call_return_this(h: &mut impl Host, this: u32, f: u32) -> u32 {
        let f = h.rebase(f);
        h.call(f, Conv::Thiscall(this), &[]);
        this
    }
}

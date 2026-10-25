//! The class descriptors' little members (`DeclareClassDescriptor` tables:
//! construct in place, destroy, forward) and other short helpers the
//! containers use.
//!
//! The "construct" members clear an object when given one; "destroy"
//! members free a `PArray`'s buffer unless its count word's top bit says
//! the buffer is not owned (`PArray`: `+0` count | `0x80000000` for
//! borrowed storage, `+4` the elements). All of them end with `mov al, 1`.

/// `0x0044af00` (thiscall `(object)`): a destructor `0x0044c2f0` and
/// `0x0044c310` forward to.
pub const DESTROY_44AF00: u32 = 0x0044_af00;
/// `0x00499dc0` (thiscall `(object)`), `0x0049bba0`'s.
pub const DESTROY_499DC0: u32 = 0x0049_9dc0;
/// `0x004ea2d0` (thiscall `(object)`), `0x004eec90`'s.
pub const DESTROY_4EA2D0: u32 = 0x004e_a2d0;
/// `0x004e9f70` (thiscall `(object)`), `0x004eeb30`'s.
pub const DESTROY_4E9F70: u32 = 0x004e_9f70;
/// `0x004ec310` (cdecl `(elements, count)`), `0x004eec50`'s.
pub const DESTROY_ARRAY_4EC310: u32 = 0x004e_c310;
/// A float constant `0x004cc240` stores (`0x00b92318`).
pub const CONST_B92318: u32 = 0x00b9_2318;
/// `0x00a3baf0` (thiscall `(object, a, b)`), `0x00a34ae0`'s.
pub const CALL_A3BAF0: u32 = 0x00a3_baf0;
/// The ring buffer's block allocator (thiscall `(ring, size)`).
pub const RING_ALLOCATE_BLOCK: u32 = 0x005c_d110;
/// The `0x00a34b10` state's pointer.
pub const A34_STATE: u32 = 0x01a8_58b4;
/// Its allocator (cdecl `(size, kind)`).
pub const A35A70: u32 = 0x00a3_5a70;

pub mod bridge {
    //! The originals over game memory.

    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    use super::RING_ALLOCATE_BLOCK;
    use super::{A34_STATE, A35A70, CALL_A3BAF0, CONST_B92318, DESTROY_ARRAY_4EC310};
    use crate::phyre_free;

    /// `eax` after `mov eax, p ... mov al, 1`.
    const fn al1(p: u32) -> u32 {
        (p & !0xff) | 1
    }

    /// Construct members that clear `words` words at `p` when `p` is set
    /// (`0x0044c2b0`, `0x0044c2d0`, `0x0049bb00`, `0x004cc1d0`,
    /// `0x004eea90`-`0x004eeb10`, `0x00a34270`: 2; `0x00a34290`: 3;
    /// `0x004459e0`: 1). `eax`: `p` with `al = 1`.
    pub fn clear_words(h: &mut impl Host, p: u32, words: u32) -> u32 {
        if p != 0 {
            for i in 0..words {
                h.write(p + 4 * i, 0u32);
            }
        }
        al1(p)
    }

    /// `0x0049bb20(p)`: six words clear and the half word `+0x18` =
    /// `0x3434`.
    pub fn clear_49bb20(h: &mut impl Host, p: u32) -> u32 {
        if p != 0 {
            for i in 0..6 {
                h.write(p + 4 * i, 0u32);
            }
            h.write(p + 0x18, 0x3434u16);
        }
        al1(p)
    }

    /// `0x004d3eb0(p)`: `+0x10`, then `+0 .. +0xc` clear.
    pub fn clear_4d3eb0(h: &mut impl Host, p: u32) -> u32 {
        if p != 0 {
            h.write(p + 0x10, 0u32);
            for i in 0..4 {
                h.write(p + 4 * i, 0u32);
            }
        }
        al1(p)
    }

    /// `0x004cc240(p)`: `+0xc0 .. +0xdc`: 0, c, 0.0, c, then four zero
    /// words (`c` the float at `0x00b92318`).
    pub fn init_4cc240(h: &mut impl Host, p: u32) -> u32 {
        if p != 0 {
            let c = x87::load(h.read(h.rebase(CONST_B92318)));
            h.write(p + 0xc0, 0u32);
            h.write(p + 0xc4, x87::store(c));
            h.write(p + 0xc8, x87::store(0.0));
            h.write(p + 0xcc, x87::store(c));
            for off in [0xd0, 0xd4, 0xd8, 0xdc] {
                h.write(p + off, 0u32);
            }
        }
        al1(p)
    }

    /// Destroy members of a `PArray` (`0x0049bb60`, `0x004cc2a0` when
    /// `checked`: only a non-null buffer; `0x004eeb50`-`0x004eec10`,
    /// `0x00a342c0`, `0x00a34300`): `PhyreFree` the owned buffer, then
    /// clear the array. `al = 1`.
    pub fn free_array(h: &mut impl Host, p: u32, checked: bool) -> u8 {
        let count: u32 = h.read(p);
        if count >> 31 == 0 {
            let data: u32 = h.read(p + 4);
            if !checked || data != 0 {
                phyre_free(h, data);
            }
        }
        h.write(p + 4, 0u32);
        h.write(p, 0u32);
        1
    }

    /// `0x004eec50(p)`: an owned array's elements destroyed by
    /// `0x004ec310(elements, count)`, then the array cleared. `al = 1`.
    pub fn destroy_array(h: &mut impl Host, p: u32) -> u8 {
        let count: u32 = h.read(p);
        if count >> 31 == 0 {
            let data: u32 = h.read(p + 4);
            h.call_sym(
                DESTROY_ARRAY_4EC310,
                Conv::Cdecl,
                &[data, count & 0x7fff_ffff],
            );
        }
        h.write(p + 4, 0u32);
        h.write(p, 0u32);
        1
    }

    /// Destroy members that forward to a destructor (thiscall on `p`):
    /// `0x0044c2f0`, `0x0044c310` (`0x0044af00`), `0x0049bba0`
    /// (`0x00499dc0`), `0x004eec90` (`0x004ea2d0`); `0x004eeb30`
    /// (`0x004e9f70`) only when `p` is set (`checked`). `al = 1`.
    pub fn forward(h: &mut impl Host, sym: u32, p: u32, checked: bool) -> u8 {
        if !checked || p != 0 {
            h.call_sym(sym, Conv::Thiscall(p), &[]);
        }
        1
    }

    /// `0x00445a00(p)`: `PhyreFree(*p)` unless bit 0 of the pointer is
    /// set (a `PString`'s borrowed text). `al = 1`.
    pub fn free_string(h: &mut impl Host, p: u32) -> u8 {
        let text: u32 = h.read(p);
        if text & 1 == 0 {
            phyre_free(h, text);
        }
        1
    }

    /// `0x00445d60` / `0x004f9150` / `0x004f9180(this, object, a, b)`:
    /// the callback at `this +0x3c` (thiscall `(object, a, b)`), or `0x15`.
    pub fn callback_3c(h: &mut impl Host, this: u32, object: u32, a: u32, b: u32) -> u32 {
        let f: u32 = h.read(this + 0x3c);
        if f == 0 {
            return 0x15;
        }
        h.call(f, Conv::Thiscall(object), &[a, b])
    }

    /// `0x004eee60` / `0x004eeec0(this, i)`: the callback at `this +0x38`
    /// (thiscall `(i)`), else the word at `i` in the table
    /// `this +0x24` relocated from `[this +0xc] +0x88`.
    pub fn callback_38(h: &mut impl Host, this: u32, i: u32) -> u32 {
        let f: u32 = h.read(this + 0x38);
        if f != 0 {
            return h.call(f, Conv::Thiscall(i), &[]);
        }
        let owner: u32 = h.read(this + 0xc);
        let base = h
            .read::<u32>(this + 0x24)
            .wrapping_sub(h.read(owner + 0x88));
        h.read(base.wrapping_add(i))
    }

    /// `0x00a34a30(this)`: two words clear; `this`.
    pub fn clear_pair(h: &mut impl Host, this: u32) -> u32 {
        h.write(this, 0u32);
        h.write(this + 4, 0u32);
        this
    }

    /// `0x00a34a60(this)`: `(0x2000000, 10, 0x200000, 1)`; `this`.
    pub fn init_a34a60(h: &mut impl Host, this: u32) -> u32 {
        h.write(this, 0x0200_0000u32);
        h.write(this + 4, 10u32);
        h.write(this + 8, 0x0020_0000u32);
        h.write(this + 0xc, 1u32);
        this
    }

    /// `0x00a34910(slot)`: releases the COM object in `*slot`
    /// (`IUnknown::Release`, vtable slot 2) and clears the slot.
    pub fn release(h: &mut impl Host, slot: u32) {
        let obj: u32 = h.read(slot);
        if obj != 0 {
            let vt: u32 = h.read(obj);
            let f: u32 = h.read(vt + 8);
            h.call(f, Conv::Stdcall, &[obj]);
            h.write(slot, 0u32);
        }
    }

    /// `0x00a34ae0(p)`: `q = p[1]`; `q[2] = 0x00a3baf0(q[0]; p[0], q[1])`;
    /// returns it.
    pub fn call_a34ae0(h: &mut impl Host, p: u32) -> u32 {
        let q: u32 = h.read(p + 4);
        let obj: u32 = h.read(q);
        let a: u32 = h.read(p);
        let b: u32 = h.read(q + 4);
        let r = h.call_sym(CALL_A3BAF0, Conv::Thiscall(obj), &[a, b]);
        h.write(q + 8, r);
        r
    }

    /// `0x00a34b10(n)`: the state's two buffers (`0x00a35a70(n, 1)` and
    /// `(3n / 4, 2)`, each with its size and two clear words) and the
    /// limit `min(n / 16, 0x2000) & !3`. `eax`: the state pointer with
    /// `al` 1 when both buffers exist, else 0.
    pub fn init_a34b10(h: &mut impl Host, n: u32) -> u32 {
        let quarter = n.wrapping_mul(3) >> 2;
        let s: u32 = h.read(h.rebase(A34_STATE));
        let a = h.call_sym(A35A70, Conv::Cdecl, &[n, 1]);
        h.write(s + 0x998, a);
        h.write(s + 0x99c, n);
        h.write(s + 0x9a0, 0u32);
        h.write(s + 0x9a4, 0u32);
        let s: u32 = h.read(h.rebase(A34_STATE));
        let b = h.call_sym(A35A70, Conv::Cdecl, &[quarter, 2]);
        h.write(s + 0x9a8, b);
        h.write(s + 0x9ac, quarter);
        h.write(s + 0x9b0, 0u32);
        h.write(s + 0x9b4, 0u32);
        let limit = (n >> 4).min(0x2000) & !3;
        let s: u32 = h.read(h.rebase(A34_STATE));
        h.write(s + 0x9c0, limit);
        let ok = h.read::<u32>(s + 0x998) != 0 && h.read::<u32>(s + 0x9a8) != 0;
        (s & !0xff) | u32::from(ok)
    }

    /// `0x005cd340(ring, n)`: whether `n` more bytes fit the ring's
    /// current run (`+0x24` used, `+0x28` the read end, `+0x1c` the end,
    /// `+0x18` the start). `eax`: `+0x28` with `al` the answer.
    pub fn ring_fits(h: &mut impl Host, ring: u32, n: u32) -> u32 {
        let read: u32 = h.read(ring + 0x28);
        let used: u32 = h.read(ring + 0x24);
        let end = n.wrapping_add(used);
        let fits = if used < read && end >= read {
            true
        } else {
            end >= h.read::<u32>(ring + 0x1c) && read == h.read::<u32>(ring + 0x18)
        };
        (read & !0xff) | u32::from(fits)
    }

    /// `PArray_Data` `0x004d45a0(array)`: the elements: the pointer at
    /// `+4` for more than one, the word `+4` itself for one, 0 for none.
    pub fn array_data(h: &mut impl Host, array: u32) -> u32 {
        let n = h.read::<u32>(array) & 0x7fff_ffff;
        if n > 1 {
            h.read(array + 4)
        } else if n != 0 {
            array + 4
        } else {
            0
        }
    }

    /// `PPreprocessRingBuffer::allocate(ring, size)`: a block
    /// (`0x005cd110`) marked in use (`+4 = 1`); the block or 0.
    pub fn ring_allocate(h: &mut impl Host, ring: u32, size: u32) -> u32 {
        let block = h.call_sym(RING_ALLOCATE_BLOCK, Conv::Thiscall(ring), &[size]);
        if block != 0 {
            h.write(block + 4, 1u32);
        }
        block
    }
}

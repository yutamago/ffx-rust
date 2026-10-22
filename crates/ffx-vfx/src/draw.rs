//! The ppp draw programs' set-up (`Con`) and tear-down (`Des`) records and
//! draws: the per-instance work areas they initialise and release.

/// The functions over game memory.
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    pub(crate) fn g(mem: &impl Mem, a: u32) -> u32 {
        Global::<u32>::new(a).ptr(mem).addr
    }

    pub(crate) const GET_PTR32: u32 = 0x00a4_4650;
    const SET_PTR32: u32 = 0x00a4_4700;
    const UNIT_MATRIX: u32 = 0x006e_d9a0;
    const VFX_DESTROY: u32 = 0x0064_5280;
    const MEM_FREE: u32 = 0x0071_6a90;
    pub(crate) const NAME_HASH: u32 = 0x0064_1da0;
    /// `g_pppEnvSlot`: where the current ppp environment pointer is kept.
    pub const ENV_SLOT: u32 = 0x00c3_a4cc;
    /// `g_pppCurrentProgram`.
    pub const CURRENT_PROGRAM: u32 = 0x0230_fd20;

    /// The levels (`graphicLevelGetNameHash`) whose shape fields
    /// `pppShapeFieldRenderOn_00743030` turns off.
    pub const SHAPE_FIELD_PLAIN_LEVELS: [u32; 9] = [
        0x76f, 0x18e7, 0x5c8, 0x1f7, 0x4c4, 0x1b8a, 0x324, 0x1e8a, 0x1139,
    ];

    /// `pppKeShpDttDraw(obj, key, ctrl)` (`0x00755040`): one distortion
    /// sprite. Work areas `a` (0), `b` (1), `m` (2, a matrix), `c` (3) at
    /// `obj +0xa0 + [[ctrl +0xc] + 4k]`. Unless `g_pppStopStatus`: `a +4` +=
    /// `a +8`, then `a +0` += that (`f32` each), and on the key's frame
    /// (`obj +0xc` = `[key]`) the key's `+4`, `+8`, `+0xc` added to `a +0`,
    /// `+4`, `+8`. Unless `g_GraphicSkipFrame`, the sprite record at `ebp -
    /// 0x80` (`ebp` the original's 16-aligned frame pointer) for
    /// `KeDttSpr_MkGifPk_Pre` and `KeDttSpr_MkGifPk`: `+0` the colour (`m
    /// +0x30`; with key byte `+0x19` bit 1 `c +0, +4`, 512, 1), `+0x10` the
    /// four `u16` part `+0x70.. * i16 b +0..` products as `f32` times
    /// 2^-19, `+0x20`/`+0x28` `set_ptr32(g_pppScreenMatrix)` (the second
    /// call first), `+0x24` `set_ptr32(g_PppPacketPos + 0x10)`, `+0x30` =
    /// `m` times `(0, -a +0, 0)` (`FFXVu0ApplyMatrix`) with `+0x3c` = key
    /// byte `+0x19`, `+0x40` the frame buffer's 64-bit TEX0 word from the
    /// draw buffer (`Sg_GetDBuffDC`, page `(key byte +0x18 + its index) &
    /// 1`, after `graphicDttPageFlag`), `+0x48` its `+0x84` sign-extended,
    /// `+0x50` its `+0x78`, `+0x7c`, `+0x58` the key's `+0x10`, `+0x14`
    /// times the part's `+0x60`, `+0x64`, `+0x60` `set_ptr32(obj)`; `+0x2c`
    /// is left as the stack has it. `Sg_GetDBuffDC`'s index word at `ebp -
    /// 0x98`. Callees through `call`.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn ppp_ke_shp_dtt_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        ctrl: u32,
        ebp: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{add, load, mul, store};
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = g(mem, at);
            call(mem, f, args)
        };
        let offs: u32 = mem.read(ctrl.wrapping_add(0xc));
        let area = |mem: &M, k: u32| -> u32 {
            let o: u32 = mem.read(offs.wrapping_add(4 * k));
            obj.wrapping_add(0xa0).wrapping_add(o)
        };
        let part: u32 = mem.read(g(mem, 0x0230_ffe4));
        let a = area(mem, 0);
        let m = area(mem, 2);
        let rf = |mem: &M, at: u32| load(mem.read(at));
        if crate::ppp::bridge::STOP.get(mem) == 0 {
            let y = store(add(rf(mem, a.wrapping_add(8)), rf(mem, a.wrapping_add(4))));
            mem.write(a.wrapping_add(4), y);
            let x = store(add(rf(mem, a), load(y)));
            mem.write(a, x);
            if mem.read::<u32>(obj.wrapping_add(0xc)) == mem.read::<u32>(key) {
                let x = store(add(load(x), rf(mem, key.wrapping_add(4))));
                mem.write(a, x);
                let y = store(add(load(y), rf(mem, key.wrapping_add(8))));
                mem.write(a.wrapping_add(4), y);
                let z = store(add(
                    rf(mem, key.wrapping_add(0xc)),
                    rf(mem, a.wrapping_add(8)),
                ));
                mem.write(a.wrapping_add(8), z);
            }
        }
        if mem.read::<u32>(g(mem, 0x012f_b7d0)) != 0 {
            return;
        }
        let rec = ebp.wrapping_sub(0x80);
        let r = |k: u32| rec.wrapping_add(k);
        let b = area(mem, 1);
        let k: f64 = mem.read(g(mem, 0x00b5_1650));
        for i in 0..4 {
            let scale = u32::from(mem.read::<u16>(part.wrapping_add(0x70 + 2 * i)));
            let n = i32::from(mem.read::<i16>(b.wrapping_add(2 * i))).cast_unsigned();
            // `imul`, `fild`, stored as f32, then times 2^-19
            let v = store(f64::from(scale.wrapping_mul(n).cast_signed()));
            mem.write(r(0x10 + 4 * i), store(mul(load(v), k)));
        }
        if mem.read::<u8>(key.wrapping_add(0x19)) & 2 != 0 {
            let c = area(mem, 3);
            mem.write(r(0), store(rf(mem, c)));
            mem.write(r(4), store(rf(mem, c.wrapping_add(4))));
            let alpha: u32 = mem.read(g(mem, 0x00b4_1d9c));
            mem.write(r(8), store(load(alpha)));
            mem.write(r(0xc), 1.0f32);
        } else {
            let colour: [u32; 4] = mem.read(m.wrapping_add(0x30));
            mem.write(rec, colour);
        }
        let screen = g(mem, 0x0230_fe60);
        let s = go(mem, 0x00a4_4700, &[screen]);
        mem.write(r(0x28), s);
        let s = go(mem, 0x00a4_4700, &[screen]);
        mem.write(r(0x20), s);
        for (at, pk) in [(0x58, 0x60), (0x5c, 0x64)] {
            let v = store(mul(
                rf(mem, key.wrapping_add(at - 0x48)),
                rf(mem, part.wrapping_add(pk)),
            ));
            mem.write(r(at), v);
        }
        let pos: u32 = mem.read(g(mem, 0x0230_fd2c));
        let s = go(mem, 0x00a4_4700, &[pos.wrapping_add(0x10)]);
        mem.write(r(0x24), s);
        let index_at = ebp.wrapping_sub(0x98);
        let dc = go(mem, 0x0082_0530, &[index_at]);
        let page_byte = mem.read::<u8>(key.wrapping_add(0x18));
        let index: u32 = mem.read(index_at);
        let page = u32::from(page_byte).wrapping_add(index) & 1;
        let off = page * 0x130;
        // `mov cl, [key +0x18]` over the key pointer in ecx
        go(
            mem,
            0x0063_fc10,
            &[(key & 0xffff_ff00) | u32::from(page_byte)],
        );
        let buf = dc.wrapping_add(off);
        let fb = mem.read::<u32>(buf.wrapping_add(0x60)).cast_signed();
        let base = u64::from(((fb >> 24) & 0x3f).cast_unsigned() | 0x2a80);
        let width = u64::from(((fb >> 16) & 0x3f).cast_unsigned());
        let low = u64::from((fb.cast_unsigned() & 0x1ff) << 5);
        mem.write(r(0x40), (((base << 6) | width) << 14) | low);
        let h = mem.read::<u32>(buf.wrapping_add(0x84)).cast_signed();
        mem.write(r(0x48), i64::from(h).cast_unsigned());
        let w: [u32; 2] = mem.read(buf.wrapping_add(0x78));
        mem.write(r(0x50), w);
        let neg = -load(mem.read(a));
        mem.write(r(0x38), 0.0f32);
        mem.write(r(0x30), 0.0f32);
        mem.write(r(0x3c), 0u32);
        mem.write(r(0x34), store(neg));
        go(mem, 0x006e_d2e0, &[r(0x30), m, r(0x30)]);
        mem.write(r(0x3c), u32::from(mem.read::<u8>(key.wrapping_add(0x19))));
        let s = go(mem, 0x00a4_4700, &[obj]);
        mem.write(r(0x60), s);
        go(mem, 0x0076_8d30, &[rec]);
        go(mem, 0x0076_7790, &[rec]);
    }

    /// `pppKeMdlDttDraw(obj, key, ctrl)` (`0x0072e790`): one model
    /// distortion; nothing on map 0x13e (`g_CurrentMapNo`). Work areas `a`
    /// (0), `b` (1), `m` (2) at `obj +0xa0 + [[ctrl +0xc] + 4k]`; from key
    /// byte `+0x14` 4 up `a +8` holds a pointer (`get_ptr32_func` non-zero:
    /// the step's z is 0). Unless `g_pppStopStatus`: `a +4` += z, `a +0` +=
    /// that, and on the key's frame (`obj +0xc` = `[key]`) the key's `+8`,
    /// `+0xc` (and below 4 `+0x10` into `a +8`) added. Unless
    /// `g_GraphicSkipFrame`, the key's effect `e` (`+4`) is 0xffff, or the
    /// `u16` part `+0x76` times `i16 b +6` is 0: the record at `ebp - 0x90`
    /// (`ebp` the original's 16-aligned frame pointer) for
    /// `KeDttMdl_MkGifPk`: `+0`/`+4` the draw buffer's `+0x78`, `+0x7c`
    /// (`Sg_GetDBuffDC`, page `(key byte +0x14 + its index) & 1`), `+8` the
    /// 64-bit TEX0 word (below byte 2 the frame buffer's, else `(u16 +0x62
    /// & 0x3f) << 14 | 0x2_a820_1a00`), `+0x10` `g_pppScreenMatrix`, `+0x14`
    /// `m`, `+0x18` `g_PppPacketPos` (put back after the call), `+0x1c` the
    /// effect's model (`get_ptr32_func` of the environment's `+0x20` record
    /// `e`'s `+0x1c`), `+0x20` the part's `+0x60` times `a +0`, `+0x24` key
    /// byte `+0x15`, `+0x28` the buffer's `+0x84` sign-extended, `+0x30` and
    /// `+0x34` the address of `+0x7c`, `+0x38..+0x40` the pointed-at word,
    /// bit 3 of the byte and 1 (else `+0x40` 0), `+0x44`
    /// `get_ptr32_func([obj +0x90])`, `+0x48` `obj`, `+0x7c..+0x88` the four
    /// `u16` part `+0x70.. * i16 b +0..` products as `f32` times 2^-26; the
    /// original's spills in between (`+0x5c` the buffer index, `+0x60` obj,
    /// `+0x64` the offset of `a`, `+0x68` the page byte over the key's
    /// address, `+0x6c` the page offset, `+0x70` key, `+0x74` the draw
    /// buffer, `+0x78` the last product's `f32`) are written as it leaves
    /// them. After the call `[a +8]` gets `+0x38` back (from 4 up). Callees
    /// through `call`.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn ppp_ke_mdl_dtt_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        ctrl: u32,
        ebp: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{add, load, mul, store};
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = g(mem, at);
            call(mem, f, args)
        };
        let offs: u32 = mem.read(ctrl.wrapping_add(0xc));
        let area_off = |mem: &M, k: u32| -> u32 { mem.read(offs.wrapping_add(4 * k)) };
        let part: u32 = mem.read(g(mem, 0x0230_ffe4));
        let m = obj.wrapping_add(0xa0).wrapping_add(area_off(mem, 2));
        if mem.read::<u32>(g(mem, 0x012f_bc40)) == 0x13e {
            return;
        }
        let a_off = area_off(mem, 0);
        let a = obj.wrapping_add(0xa0).wrapping_add(a_off);
        let rf = |mem: &M, at: u32| load(mem.read(at));
        let kind = |mem: &M| mem.read::<u8>(key.wrapping_add(0x14));
        let z = if kind(mem) >= 4 && go(mem, 0x00a4_4650, &[mem.read(a.wrapping_add(8))]) != 0 {
            0.0f32.to_bits()
        } else {
            store(rf(mem, a.wrapping_add(8)))
        };
        if crate::ppp::bridge::STOP.get(mem) == 0 {
            let y = store(add(load(z), rf(mem, a.wrapping_add(4))));
            mem.write(a.wrapping_add(4), y);
            let x = store(add(rf(mem, a), load(y)));
            mem.write(a, x);
            if mem.read::<u32>(obj.wrapping_add(0xc)) == mem.read::<u32>(key) {
                mem.write(a, store(add(load(x), rf(mem, key.wrapping_add(8)))));
                mem.write(
                    a.wrapping_add(4),
                    store(add(load(y), rf(mem, key.wrapping_add(0xc)))),
                );
                if kind(mem) < 4 {
                    let v = store(add(
                        rf(mem, key.wrapping_add(0x10)),
                        rf(mem, a.wrapping_add(8)),
                    ));
                    mem.write(a.wrapping_add(8), v);
                }
            }
        }
        if mem.read::<u32>(g(mem, 0x012f_b7d0)) != 0 {
            return;
        }
        let e: u32 = mem.read(key.wrapping_add(4));
        if e == 0xffff {
            return;
        }
        let b = obj.wrapping_add(0xa0).wrapping_add(area_off(mem, 1));
        let product = |mem: &M, i: u32| {
            let scale = u32::from(mem.read::<u16>(part.wrapping_add(0x70 + 2 * i)));
            let n = i32::from(mem.read::<i16>(b.wrapping_add(2 * i))).cast_unsigned();
            scale.wrapping_mul(n)
        };
        let n3 = product(mem, 3);
        if n3 == 0 {
            return;
        }
        let rec = ebp.wrapping_sub(0x90);
        let r = |k: u32| rec.wrapping_add(k);
        let k: f64 = mem.read(g(mem, 0x00b5_1530));
        // `fild`, stored as f32, then times 2^-26
        let int = |n: u32| store(f64::from(n.cast_signed()));
        mem.write(r(0x88), store(mul(load(int(n3)), k)));
        let mut last = 0;
        for i in 0..3 {
            last = int(product(mem, i));
            mem.write(r(0x7c + 4 * i), store(mul(load(last), k)));
        }
        mem.write(r(0x78), last);
        mem.write(r(0x14), m);
        mem.write(r(0x30), r(0x7c));
        mem.write(r(0x34), r(0x7c));
        let pos: u32 = mem.read(g(mem, 0x0230_fd2c));
        mem.write(r(0x18), pos);
        mem.write(r(0x10), g(mem, 0x0230_fe60));
        let slot: u32 = mem.read(g(mem, ENV_SLOT));
        let env: u32 = mem.read(mem.read::<u32>(slot).wrapping_add(0x20));
        let model: u32 = mem.read(env.wrapping_add(e << 5).wrapping_add(0x1c));
        let p = go(mem, 0x00a4_4650, &[model]);
        mem.write(r(0x1c), p);
        let scaled = store(mul(rf(mem, part.wrapping_add(0x60)), rf(mem, a)));
        mem.write(r(0x24), u32::from(mem.read::<u8>(key.wrapping_add(0x15))));
        mem.write(r(0x20), scaled);
        let w: u32 = mem.read(obj.wrapping_add(0x90));
        let p = go(mem, 0x00a4_4650, &[w]);
        mem.write(r(0x44), p);
        let byte = kind(mem);
        let target: u32 = mem.read(a.wrapping_add(8));
        if byte >= 4 && target != 0 {
            let v: u32 = mem.read(target);
            mem.write(r(0x38), v);
            mem.write(r(0x40), 1u32);
            mem.write(r(0x3c), u32::from(byte & 8));
        } else {
            mem.write(r(0x40), 0u32);
        }
        let index_at = r(0x5c);
        let dc = go(mem, 0x0082_0530, &[index_at]);
        let page_byte = kind(mem);
        let page = u32::from(page_byte).wrapping_add(mem.read(index_at)) & 1;
        let off = page * 0x130;
        let page_word = (key & 0xffff_ff00) | u32::from(page_byte);
        mem.write(r(0x74), dc);
        mem.write(r(0x68), page_word);
        mem.write(r(0x6c), off);
        let buf = dc.wrapping_add(off);
        let tex0 = if page_byte < 2 {
            let fb = mem.read::<u32>(buf.wrapping_add(0x60)).cast_signed();
            let base = u64::from(((fb >> 24) & 0x3f).cast_unsigned() | 0x2a80);
            let width = u64::from(((fb >> 16) & 0x3f).cast_unsigned());
            let low = u64::from((fb.cast_unsigned() & 0x1ff) << 5);
            (((base << 6) | width) << 14) | low
        } else {
            let v = u64::from(mem.read::<u16>(buf.wrapping_add(0x62)) & 0x3f);
            (v << 14) | 0x2_a820_1a00
        };
        mem.write(r(8), tex0);
        let h = mem.read::<u32>(buf.wrapping_add(0x84)).cast_signed();
        mem.write(r(0x28), i64::from(h).cast_unsigned());
        let src: [u32; 2] = mem.read(buf.wrapping_add(0x78));
        mem.write(rec, src);
        go(mem, 0x0063_fc10, &[page_word]);
        mem.write(r(0x48), obj);
        mem.write(r(0x60), obj);
        mem.write(r(0x64), a_off);
        mem.write(r(0x70), key);
        go(mem, 0x0072_ded0, &[rec]);
        let pos: u32 = mem.read(r(0x18));
        mem.write(g(mem, 0x0230_fd2c), pos);
        if kind(mem) >= 4 {
            let target: u32 = mem.read(a.wrapping_add(8));
            if target != 0 {
                let v: u32 = mem.read(r(0x38));
                mem.write(target, v);
            }
        }
    }

    /// `pppShapeFieldRenderOn_00743030()` (`0x00743030`): 0 on the nine
    /// [`SHAPE_FIELD_PLAIN_LEVELS`], else 1 (`graphicLevelGetNameHash`
    /// through `call`).
    pub fn shape_field_render_on<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let h = call(mem, g(mem, NAME_HASH), &[]);
        u32::from(!SHAPE_FIELD_PLAIN_LEVELS.contains(&h))
    }

    /// The locals `pppCreateFrameBufferData_00715a40` and
    /// `pppCreateD3D11FrameBufferData_00716050` hand to
    /// `graphicVFXCreateDataBuffer`: `+0` the buffer count (1), `+4` and
    /// `+8` the two sizes, `+0xc` the name (up to 0x100 bytes).
    pub const FRAME_BUFFER_FRAME: u32 = 0x10c;

    /// `graphicVFXCreate(lo, hi, flags)`, then one data buffer
    /// (`graphicVFXCreateDataBuffer(lo, hi, 1, &a, &b, name, &count)` with
    /// the frame filled as [`FRAME_BUFFER_FRAME`] says); whether the
    /// instance now has render data (`graphicVFXGetRenderDataByInstance`).
    fn create_frame_buffer<M: Mem>(
        mem: &mut M,
        lo: u32,
        hi: u32,
        flags: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        call(mem, g(mem, 0x0064_4de0), &[lo, hi, flags]);
        let args = [lo, hi, 1, frame + 4, frame + 8, frame + 0xc, frame];
        call(mem, g(mem, 0x0064_5040), &args);
        u32::from(call(mem, g(mem, GET_RENDER_DATA), &[lo, hi]) != 0)
    }

    /// `pppCreateFrameBufferData_00715a40(lo, hi)` (`0x00715a40`): a VFX
    /// instance (flags 0x10000) with one 0x441 x 0x1800 data buffer named
    /// `FrameBuffer` (12 bytes from `0x00b45b3c`), `frame` standing for
    /// the original's locals ([`FRAME_BUFFER_FRAME`]). Returns whether it
    /// has render data. Callees through `call`.
    pub fn create_frame_buffer_data<M: Mem>(
        mem: &mut M,
        lo: u32,
        hi: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let name: [u32; 3] = mem.read(g(mem, 0x00b4_5b3c));
        mem.write(frame, [1u32, 0x441, 0x1800]);
        mem.write(frame + 0xc, name);
        create_frame_buffer(mem, lo, hi, 0x1_0000, frame, call)
    }

    /// `pppCreateD3D11FrameBufferData_00716050(lo, hi, n)` (`0x00716050`):
    /// a VFX instance (flags 0x20000000) with one data buffer of `8 n + 4`
    /// by `12 n + 6` named `D3D11FrameBuffer` (copied from `0x00b47f18` to
    /// its NUL, as the original's `strcpy`), `frame` standing for its
    /// locals. Returns whether it has render data. Callees through `call`.
    pub fn create_d3d11_frame_buffer_data<M: Mem>(
        mem: &mut M,
        lo: u32,
        hi: u32,
        n: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        mem.write(frame, [1u32, 0, 0]);
        let src = g(mem, 0x00b4_7f18);
        for i in 0..0x100 {
            let c: u8 = mem.read(src + i);
            mem.write(frame + 0xc + i, c);
            if c == 0 {
                break;
            }
        }
        let sizes = [
            n.wrapping_mul(8).wrapping_add(4),
            n.wrapping_mul(12).wrapping_add(6),
        ];
        mem.write(frame + 4, sizes);
        create_frame_buffer(mem, lo, hi, 0x2000_0000, frame, call)
    }

    /// `graphicVFXGetRenderDataByInstance(lo, hi)`, called through `call`.
    const GET_RENDER_DATA: u32 = 0x0064_53d0;
    /// `graphicDynGeoMemManagerGetCurrentBufferId()`, called through `call`.
    const CURRENT_BUFFER_ID: u32 = 0x0063_ff50;

    /// A render data's parts: `i32 +0x90` of them, 0x6c bytes each from
    /// `[+0x94]` (reread for every access, as the original does). A part
    /// keeps its index count at `+4` and vertex count at `+8`, and one
    /// 0x1c-byte record per dynamic-geometry buffer from `+0x2c` whose first
    /// two words take those counts.
    fn rd_part(mem: &impl Mem, rd: u32, i: u32) -> u32 {
        mem.read::<u32>(rd.wrapping_add(0x94))
            .wrapping_add(i.wrapping_mul(0x6c))
    }

    /// `caculateVFXRenderDataShapeFieldBufferSize(lo, hi, shape, n)`
    /// (`0x00712bd0`): with the render data of instance `(lo, hi)`, the
    /// shape's entry `n` (`get_ptr32(*shape)`, its `i16 +0x10 + 8n` the
    /// entry's offset) adds 6 x its `i16 +2` to the first part's vertex
    /// count (`+8`) and 4 x it to its index count (`+4`). Returns 1, or 0
    /// without render data.
    pub fn caculate_vfx_render_data_shape_field_buffer_size<M: Mem>(
        mem: &mut M,
        args: [u32; 4],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let [lo, hi, shape, n] = args;
        let f = g(mem, GET_RENDER_DATA);
        let rd = call(mem, f, &[lo, hi]);
        if rd == 0 {
            return 0;
        }
        let f = g(mem, GET_PTR32);
        let arg: u32 = mem.read(shape);
        let base = call(mem, f, &[arg]);
        let n = n & 0xffff;
        let off = i32::from(mem.read::<i16>(base.wrapping_add(0x10 + 8 * n)));
        let entry = base.wrapping_add(off.cast_unsigned());
        let count = |mem: &M| i32::from(mem.read::<i16>(entry.wrapping_add(2))).cast_unsigned();
        let at = rd_part(mem, rd, 0).wrapping_add(8);
        let v = mem.read::<u32>(at).wrapping_add(count(mem).wrapping_mul(6));
        mem.write(at, v);
        let at = rd_part(mem, rd, 0).wrapping_add(4);
        let v = mem.read::<u32>(at).wrapping_add(count(mem).wrapping_mul(4));
        mem.write(at, v);
        1
    }

    /// The part loop of [`clear_vfx_render_data_shp`]: for each part, its
    /// counts cleared and copied into the current buffer's record
    /// (`+0x2c + 0x1c id`: index count, vertex count); `zero` first runs
    /// on the part (the memset of the `WithMemset` form).
    fn clear_parts<M: Mem>(mem: &mut M, rd: u32, id: u32, mut zero: impl FnMut(&mut M, u32)) {
        let rec = id.wrapping_mul(0x1c);
        let mut i = 0u32;
        while i.cast_signed() < mem.read::<i32>(rd.wrapping_add(0x90)) {
            zero(mem, rd_part(mem, rd, i));
            mem.write(rd_part(mem, rd, i).wrapping_add(8), 0u32);
            mem.write(rd_part(mem, rd, i).wrapping_add(4), 0u32);
            let p = rd_part(mem, rd, i);
            let v: u32 = mem.read(p.wrapping_add(8));
            mem.write(p.wrapping_add(0x30).wrapping_add(rec), v);
            let p = rd_part(mem, rd, i);
            let v: u32 = mem.read(p.wrapping_add(4));
            mem.write(p.wrapping_add(0x2c).wrapping_add(rec), v);
            i = i.wrapping_add(1);
        }
    }

    /// `clearVFXRenderDataShp(lo, hi)` (`0x00713290`): every part of the
    /// render data of instance `(lo, hi)` gets index and vertex counts 0,
    /// also in its record for the current dynamic-geometry buffer
    /// (`graphicDynGeoMemManagerGetCurrentBufferId`). Returns 1, or 0
    /// without render data.
    pub fn clear_vfx_render_data_shp<M: Mem>(
        mem: &mut M,
        lo: u32,
        hi: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = g(mem, GET_RENDER_DATA);
        let rd = call(mem, f, &[lo, hi]);
        if rd == 0 {
            return 0;
        }
        let f = g(mem, CURRENT_BUFFER_ID);
        let id = call(mem, f, &[]);
        clear_parts(mem, rd, id, |_, _| {});
        1
    }

    /// `clearVFXRenderDataShpWithMemset(lo, hi)` (`0x00713320`):
    /// [`clear_vfx_render_data_shp`], each part's index buffer (`[+0x1c]`,
    /// `2 x [+0x68]` bytes) zeroed first (the CRT `memset`).
    pub fn clear_vfx_render_data_shp_with_memset<M: Mem>(
        mem: &mut M,
        lo: u32,
        hi: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = g(mem, GET_RENDER_DATA);
        let rd = call(mem, f, &[lo, hi]);
        if rd == 0 {
            return 0;
        }
        let f = g(mem, CURRENT_BUFFER_ID);
        let id = call(mem, f, &[]);
        clear_parts(mem, rd, id, |mem, p| {
            let buf: u32 = mem.read(p.wrapping_add(0x1c));
            let n = mem.read::<u32>(p.wrapping_add(0x68)).wrapping_mul(2);
            for k in 0..n {
                mem.write(buf.wrapping_add(k), 0u8);
            }
        });
        1
    }

    /// The 64-bit renderer instance of object `obj` in the current program:
    /// `(program, obj + sign(program))`.
    pub(crate) fn instance(mem: &impl Mem, obj: u32) -> [u32; 2] {
        let prog: u32 = mem.read(g(mem, CURRENT_PROGRAM));
        [
            prog,
            obj.wrapping_add(prog.cast_signed().wrapping_shr(31).cast_unsigned()),
        ]
    }

    /// A word of the set-up: a null pointer (`set_ptr32(0)`) or 0.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Init {
        /// `set_ptr32(0)`.
        Null,
        /// 0.
        Zero,
    }

    /// One `pppDrawShape*Con` layout.
    #[derive(Debug, Clone, Copy)]
    pub struct ConLayout {
        /// The words from `+0x40` of the work area.
        pub words: &'static [Init],
        /// The shape record's (`[[obj +8]]`) end and start fields of the
        /// frame count.
        pub span: (u32, u32),
    }

    /// `pppDrawShapeFieldCon` (`0x00742b50`).
    pub const FIELD_CON: ConLayout = ConLayout {
        words: &[Init::Null, Init::Null, Init::Zero, Init::Zero, Init::Null],
        span: (0x1c, 0x18),
    };
    /// `pppDrawShapeFieldRevCon` (`0x00743f00`).
    pub const FIELD_REV_CON: ConLayout = ConLayout {
        words: &[Init::Null, Init::Null, Init::Zero, Init::Zero, Init::Null],
        span: (0x14, 0x10),
    };
    /// `pppDrawShapeFieldSpdCon` (`0x007450a0`).
    pub const FIELD_SPD_CON: ConLayout = ConLayout {
        words: &[
            Init::Null,
            Init::Null,
            Init::Zero,
            Init::Zero,
            Init::Null,
            Init::Null,
        ],
        span: (0x14, 0x10),
    };
    /// `pppDrawShapeFieldGlobalCon` (`0x00746200`).
    pub const FIELD_GLOBAL_CON: ConLayout = ConLayout {
        words: &[Init::Null, Init::Zero, Init::Zero, Init::Null],
        span: (0x1c, 0x18),
    };
    /// `pppDrawShapeCameraCon` (`0x00747410`) and
    /// `pppDrawShapeCameraDisPosCon` (`0x00748660`).
    pub const CAMERA_CON: ConLayout = ConLayout {
        words: &[Init::Null, Init::Zero, Init::Zero, Init::Null],
        span: (0x14, 0x10),
    };

    /// A `pppDrawShape*Con(obj, prog)`: the work area `obj + 0xa0 +
    /// [[prog +0xc]]` gets a unit matrix, then `layout`'s words from `+0x40`,
    /// then the frame count `(end - start) / 4096 + 1` of the shape record
    /// `[[obj +8]]` (through `get_ptr32_func`) and a 0. Returns the count.
    pub fn shape_con<M: Mem>(
        mem: &mut M,
        obj: u32,
        prog: u32,
        layout: ConLayout,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let t: u32 = mem.read(prog.wrapping_add(0xc));
        let off: u32 = mem.read(t);
        let w = obj.wrapping_add(0xa0).wrapping_add(off);
        let (get, set) = (g(mem, GET_PTR32), g(mem, SET_PTR32));
        let s8: u32 = mem.read(obj.wrapping_add(8));
        let p = call(mem, get, &[s8]);
        let pp: u32 = mem.read(p);
        let shape = call(mem, get, &[pp]);
        call(mem, g(mem, UNIT_MATRIX), &[w]);
        let mut at = w.wrapping_add(0x40);
        for &k in layout.words {
            let x = match k {
                Init::Null => call(mem, set, &[0]),
                Init::Zero => 0,
            };
            mem.write(at, x);
            at = at.wrapping_add(4);
        }
        let end: i32 = mem.read(shape.wrapping_add(layout.span.0));
        let start: i32 = mem.read(shape.wrapping_add(layout.span.1));
        let n = (end.wrapping_sub(start) / 0x1000).wrapping_add(1);
        mem.write(at, n);
        mem.write(at.wrapping_add(4), 0u32);
        n.cast_unsigned()
    }

    /// How many bytes one buffer of a [`shape_alloc`] takes.
    #[derive(Debug, Clone, Copy)]
    pub enum Size {
        /// The key's count (`[key +8]`) records of this many bytes.
        PerKey(u32),
        /// Eight bytes per the count at this work-area offset.
        PerCount(u32),
        /// A fixed size.
        Fixed(u32),
    }

    /// A buffer of a [`shape_alloc`]: where its `set_ptr32` word goes, its
    /// size, and whether it is cleared.
    #[derive(Debug, Clone, Copy)]
    pub struct Buffer {
        /// Its offset in the work area.
        pub off: u32,
        /// Its size.
        pub size: Size,
        /// Cleared after the allocation.
        pub clear: bool,
    }

    /// `pppDrawShapeFieldAlloc_00742f20` (`0x00742f20`) and
    /// `pppDrawShapeFieldRevAlloc_00744260` (`0x00744260`), byte-identical.
    pub const FIELD_ALLOC: &[Buffer] = &[
        Buffer {
            off: 0x40,
            size: Size::PerKey(0xa0),
            clear: true,
        },
        Buffer {
            off: 0x44,
            size: Size::Fixed(0xc),
            clear: false,
        },
        Buffer {
            off: 0x50,
            size: Size::PerCount(0x54),
            clear: true,
        },
    ];
    /// `pppDrawShapeFieldGlobalAlloc_00746540` (`0x00746540`).
    pub const FIELD_GLOBAL_ALLOC: &[Buffer] = &[
        Buffer {
            off: 0x40,
            size: Size::PerKey(0xb0),
            clear: true,
        },
        Buffer {
            off: 0x4c,
            size: Size::PerCount(0x50),
            clear: true,
        },
    ];
    /// `pppDrawShapeCameraAlloc_00747760` (`0x00747760`) and
    /// `pppDrawShapeCameraDisPosAlloc_007489b0` (`0x007489b0`),
    /// byte-identical; [`FIELD_GLOBAL_ALLOC`] with 0xa0-byte records.
    pub const CAMERA_ALLOC: &[Buffer] = &[
        Buffer {
            off: 0x40,
            size: Size::PerKey(0xa0),
            clear: true,
        },
        Buffer {
            off: 0x4c,
            size: Size::PerCount(0x50),
            clear: true,
        },
    ];

    /// A `pppDrawShape*Alloc(obj, key, prog)`, the buffers a shape draw
    /// makes on its first frame: each of `buffers` in turn comes from
    /// `pppMemAlloc(env, env + 0x10, size)` (`env` = `[[g_pppEnvSlot]]`,
    /// read again each time), its `set_ptr32` word is stored in the work
    /// area `obj + 0xa0 + [[prog +0xc]]`, and when `is_ptr32_null` says it
    /// failed the result is 0; else it is cleared (`memset` through
    /// `get_ptr32_func`) when asked. Returns 1 when all were made. Callees
    /// through `call`.
    pub fn shape_alloc<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        prog: u32,
        buffers: &[Buffer],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let offsets: u32 = mem.read(prog.wrapping_add(0xc));
        let area = obj.wrapping_add(0xa0).wrapping_add(mem.read(offsets));
        for b in buffers {
            let size = |mem: &M| match b.size {
                Size::PerKey(n) => mem.read::<u32>(key.wrapping_add(8)).wrapping_mul(n),
                Size::PerCount(at) => mem.read::<u32>(area.wrapping_add(at)) << 3,
                Size::Fixed(n) => n,
            };
            let env: u32 = mem.read(mem.read::<u32>(g(mem, ENV_SLOT)));
            let n = size(mem);
            let p = call(mem, g(mem, 0x0071_6a30), &[env, env.wrapping_add(0x10), n]);
            let word = call(mem, g(mem, SET_PTR32), &[p]);
            mem.write(area.wrapping_add(b.off), word);
            if call(mem, g(mem, 0x00a4_46a0), &[word]) != 0 {
                return 0;
            }
            if b.clear {
                let n = size(mem);
                let word: u32 = mem.read(area.wrapping_add(b.off));
                let at = call(mem, g(mem, GET_PTR32), &[word]);
                call(mem, g(mem, 0x0094_964c), &[at, 0, n]);
            }
        }
        1
    }

    /// A pointer the tear-down releases.
    #[derive(Debug, Clone, Copy)]
    pub struct Free {
        /// Its offset in the work area.
        pub off: u32,
        /// Freed only when not null (else always).
        pub check: bool,
        /// Then set to `set_ptr32(0)`.
        pub reset: bool,
    }

    /// How a `Des` destroys the renderer instance.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Destroy {
        /// Not at all.
        Never,
        /// First.
        First,
        /// First, unless the level's name hash is one of these.
        FirstUnless(&'static [u32]),
        /// First and again at the end (its result returned).
        Twice,
    }

    /// One `ppp*Des` layout.
    #[derive(Debug, Clone, Copy)]
    pub struct DesLayout {
        /// The index in `[prog +0xc]` of the work area's offset.
        pub slot: u32,
        /// The renderer instance's destruction.
        pub destroy: Destroy,
        /// The pointers released, in order.
        pub frees: &'static [Free],
    }

    const fn fr(off: u32, check: bool, reset: bool) -> Free {
        Free { off, check, reset }
    }

    /// `pppDrawShapeFieldDes` (`0x00742bd0`).
    pub const FIELD_DES: DesLayout = DesLayout {
        slot: 0,
        destroy: Destroy::FirstUnless(&[
            0x76f, 0x18e7, 0x5c8, 0x1f7, 0x4c4, 0x1b8a, 0x324, 0x1e8a, 0x1139,
        ]),
        frees: &[
            fr(0xe0, true, false),
            fr(0xe4, true, false),
            fr(0xf0, true, false),
        ],
    };
    /// `pppDrawShapeFieldRevDes` (`0x00743f80`).
    pub const FIELD_REV_DES: DesLayout = DesLayout {
        slot: 0,
        destroy: Destroy::First,
        frees: &[
            fr(0xe0, true, true),
            fr(0xe4, false, true),
            fr(0xf0, true, true),
        ],
    };
    /// `pppDrawShapeFieldSpdDes` (`0x00745130`).
    pub const FIELD_SPD_DES: DesLayout = DesLayout {
        slot: 0,
        destroy: Destroy::First,
        frees: &[
            fr(0xe0, true, true),
            fr(0xe4, false, false),
            fr(0xf0, true, true),
            fr(0xf4, true, true),
        ],
    };
    /// `pppDrawShapeFieldGlobalDes` (`0x00746280`).
    pub const FIELD_GLOBAL_DES: DesLayout = DesLayout {
        slot: 0,
        destroy: Destroy::First,
        frees: &[fr(0xe0, true, true), fr(0xec, true, true)],
    };
    /// `pppDrawShapeCameraDes` (`0x00747490`) and
    /// `pppDrawShapeCameraDisPosDes` (`0x007486e0`).
    pub const CAMERA_DES: DesLayout = DesLayout {
        slot: 0,
        destroy: Destroy::Twice,
        frees: &[fr(0xe0, true, true), fr(0xec, true, true)],
    };
    /// `pppDrawRainDes` (`0x00756270`).
    pub const RAIN_DES: DesLayout = DesLayout {
        slot: 1,
        destroy: Destroy::Never,
        frees: &[fr(0xa4, true, true)],
    };

    /// A `ppp*Des(obj, prog)`: the renderer instance destroyed
    /// (`graphicVFXDestroy`) as `layout` says, and the work area's
    /// (`obj + [[prog +0xc] + 4 slot]`) pointers released (`pppMemFree` in
    /// the current environment), as `layout` says. Returns `eax` as the
    /// original leaves it.
    pub fn ppp_des<M: Mem>(
        mem: &mut M,
        obj: u32,
        prog: u32,
        layout: DesLayout,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let t: u32 = mem.read(prog.wrapping_add(0xc));
        let off: u32 = mem.read(t.wrapping_add(4 * layout.slot));
        let w = obj.wrapping_add(off);
        let destroy = g(mem, VFX_DESTROY);
        let mut eax = match layout.destroy {
            Destroy::Never => 0,
            Destroy::First | Destroy::Twice => {
                let id = instance(mem, obj);
                call(mem, destroy, &id)
            }
            Destroy::FirstUnless(skip) => {
                let h = call(mem, g(mem, NAME_HASH), &[]);
                if skip.contains(&h) {
                    h
                } else {
                    let id = instance(mem, obj);
                    call(mem, destroy, &id)
                }
            }
        };
        let (get, set, free) = (g(mem, GET_PTR32), g(mem, SET_PTR32), g(mem, MEM_FREE));
        for f in layout.frees {
            let raw: u32 = mem.read(w.wrapping_add(f.off));
            let p = call(mem, get, &[raw]);
            eax = p;
            if p != 0 || !f.check {
                let env_at: u32 = mem.read(g(mem, ENV_SLOT));
                let env: u32 = mem.read(env_at);
                eax = call(mem, free, &[env, env.wrapping_add(0x10), p]);
            }
            if f.reset {
                eax = call(mem, set, &[0]);
                mem.write(w.wrapping_add(f.off), eax);
            }
        }
        if layout.destroy == Destroy::Twice {
            let id = instance(mem, obj);
            eax = call(mem, destroy, &id);
        }
        eax
    }

    const SPR_ALLOC: u32 = 0x007e_6a00;
    const SPR_FREE: u32 = 0x007e_6a60;
    const FLUSH_CACHE: u32 = 0x0062_f4c0;
    const SYNC_PATH: u32 = 0x0062_f880;
    const SET_DEF_LOAD_IMAGE: u32 = 0x0062_f820;
    const EXEC_LOAD_IMAGE: u32 = 0x0062_f7a0;
    const SET_DEF_STORE_IMAGE: u32 = 0x0062_f830;
    const EXEC_STORE_IMAGE: u32 = 0x0062_f7b0;
    const SUB_VECTOR: u32 = 0x006e_d940;
    const DIV_VECTOR: u32 = 0x006e_d3a0;
    const ADD_VECTOR: u32 = 0x006e_d290;
    const FTOI4_VECTOR: u32 = 0x0062_fcd0;
    const GET_DBUFF_DC: u32 = 0x0082_0530;
    /// The CLUT's 32-entry step table (signed entry deltas, CSM1 order).
    const CLUT_STEP: u32 = 0x00c3_e6a0;
    /// The CLUT's 32-entry position table (entry `i` within its block).
    const CLUT_POS: u32 = 0x00c3_e6c0;
    /// `g_pppStop`.
    pub(crate) const STOP: u32 = 0x0230_fd34;
    /// The current lens-flare view (`u16`).
    const LENS_VIEW: u32 = 0x0230_fffe;

    pub(crate) fn f(mem: &impl Mem, at: u32) -> f64 {
        ffx_bridge::x87::load(mem.read(at))
    }

    fn sbyte(mem: &impl Mem, at: u32) -> u32 {
        i32::from(mem.read::<u8>(at).cast_signed()).cast_unsigned()
    }

    /// `pppKeTkFadeDraw(obj, key, prog)` (`0x0072ee50`): the fade's work
    /// area `[pos, speed, accel]` at `obj + 0xa0 + [[prog + 0xc]]` steps
    /// (`speed += accel`, `pos += speed`, the key's `+8..+0x10` added on
    /// its frame) unless stopped; then, unless the key's texture (`+4`) is
    /// `0xffff`, a 256-entry CLUT in a scratch buffer: `n = ftol(pos)`
    /// clamped to `[-0x100, 0xff]`; for `n >= 0` the first `n` entries hold
    /// colour `+0x18` and a ramp from it to colour `+0x14` fills the rest,
    /// for `n < 0` the ramp covers the first `n + 0x100` and `+0x14` the
    /// rest (entries in CSM1 order, through the tables at `0x00c3e6a0`).
    /// The upload (`sceGsSetDefLoadImage`, `sceGsExecLoadImage` to the
    /// texture's CLUT at `+0x10`) is the renderer seam, a call through
    /// `call` (no-op stubs in `FFX.exe`); the locals lie below `frame`.
    /// Returns what the original leaves in eax.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn ppp_ke_tk_fade_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        prog: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{add, ftol, ftol2, load, store};
        let off: u32 = mem.read(mem.read::<u32>(prog.wrapping_add(0xc)));
        let w = obj.wrapping_add(off).wrapping_add(0xa0);
        let mut eax = mem.read::<u32>(prog.wrapping_add(0xc));
        if mem.read::<u32>(g(mem, STOP)) == 0 {
            let speed = store(add(f(mem, w.wrapping_add(8)), f(mem, w.wrapping_add(4))));
            mem.write(w.wrapping_add(4), speed);
            let pos = store(add(f(mem, w), load(speed)));
            mem.write(w, pos);
            eax = mem.read(obj.wrapping_add(0xc));
            if eax == mem.read::<u32>(key) {
                mem.write(w, store(add(load(pos), f(mem, key.wrapping_add(8)))));
                mem.write(
                    w.wrapping_add(4),
                    store(add(load(speed), f(mem, key.wrapping_add(0xc)))),
                );
                let accel = store(add(
                    f(mem, key.wrapping_add(0x10)),
                    f(mem, w.wrapping_add(8)),
                ));
                mem.write(w.wrapping_add(8), accel);
            }
        }
        let tex_id: u32 = mem.read(key.wrapping_add(4));
        if tex_id == 0xffff {
            return eax;
        }
        let buf = call(mem, g(mem, SPR_ALLOC), &[0x420]);
        for i in 0..8 {
            let v: u32 = mem.read(g(mem, CLUT_STEP).wrapping_add(4 * i));
            mem.write(buf.wrapping_add(0x400 + 4 * i), v);
        }
        let env: u32 = mem.read(mem.read::<u32>(g(mem, ENV_SLOT)));
        let rec = mem
            .read::<u32>(env.wrapping_add(0x20))
            .wrapping_add(tex_id << 5);
        let tex: u32 = mem.read(rec.wrapping_add(0x10));
        let tex = tex | (tex.cast_signed() >> 31).cast_unsigned();
        let step =
            |mem: &M, i: u32| sbyte(mem, buf.wrapping_add(0x400 + (i & 0x1f))).wrapping_mul(4);
        let n = ftol(mem, f(mem, w)).cast_signed();
        let (end_c, start_c) = (
            mem.read::<u32>(key.wrapping_add(0x14)),
            mem.read::<u32>(key.wrapping_add(0x18)),
        );
        let (mut at, from, to, div);
        if n < 0 {
            // the tail [n, 0x100) is the end colour, the ramp [0, n) before it
            let n = n.max(-0x100) + 0x100;
            let pos = sbyte(mem, g(mem, CLUT_POS).wrapping_add(n.cast_unsigned() & 0x1f))
                .wrapping_add(n.cast_unsigned() & 0xe0);
            let mut p = buf.wrapping_add(pos.wrapping_mul(4));
            for i in n..0x100 {
                mem.write(p, end_c);
                p = p.wrapping_add(step(mem, i.cast_unsigned()));
            }
            #[allow(clippy::cast_precision_loss)] // fild of a byte count
            let d = n as f32;
            (at, from, to, div) = (buf, 0, n, d);
        } else {
            // the head [0, n) is the start colour, the ramp [n, 0x100) after it
            let n = n.min(0xff);
            let mut p = buf;
            for i in 0..n {
                mem.write(p, start_c);
                p = p.wrapping_add(step(mem, i.cast_unsigned()));
            }
            #[allow(clippy::cast_precision_loss)] // 256 - n, exact
            let d = (0x100 - n) as f32;
            (at, from, to, div) = (p, n, 0x100, d);
        }
        // the ramp: start + i (end - start) / div, per channel, on the stack
        let (s, e) = (frame.wrapping_sub(0x14), frame.wrapping_sub(0x24));
        for (k, b) in (0u32..).zip(start_c.to_le_bytes()) {
            mem.write(s.wrapping_add(4 * k), f32::from(b).to_bits());
        }
        for (k, b) in (0u32..).zip(end_c.to_le_bytes()) {
            mem.write(e.wrapping_add(4 * k), f32::from(b).to_bits());
        }
        call(mem, g(mem, SUB_VECTOR), &[e, e, s]);
        call(mem, g(mem, DIV_VECTOR), &[e, e, div.to_bits()]);
        for i in from..to {
            let mut c = 0u32;
            for k in (0..4).rev() {
                c = (c << 8) | ftol2(f(mem, s.wrapping_add(4 * k)));
            }
            mem.write(at, c);
            call(mem, g(mem, ADD_VECTOR), &[s, s, e]);
            at = at.wrapping_add(step(mem, i.cast_unsigned()));
        }
        // the renderer seam: the CLUT upload
        let lp = frame.wrapping_sub(0x80);
        call(
            mem,
            g(mem, SET_DEF_LOAD_IMAGE),
            &[
                lp,
                (tex >> 7) & 0x3fff,
                4,
                (tex >> 21) & 0xf,
                0,
                0,
                0x10,
                0x10,
            ],
        );
        call(mem, g(mem, FLUSH_CACHE), &[0]);
        call(mem, g(mem, EXEC_LOAD_IMAGE), &[lp, buf]);
        call(mem, g(mem, SYNC_PATH), &[0, 0]);
        call(mem, g(mem, SPR_FREE), &[0x420])
    }

    /// `pppKeLnsLpTOtDraw(obj, _, frame_no)` (`0x0075b230`): the lens
    /// flare's occlusion test. Off its view (`u16` `+0xcc` against
    /// `0x0230fffe`) the visibility `+0x7c` is 0; at 0 it stays; off its
    /// frame (`+0x8c` and `frame_no` modulo `+0x8d + 1` differ) it decays
    /// by `+0xac`. Otherwise the `+0xd0 x +0xd1` half-extent box (in 1/8
    /// pixels) round the projected point (`FFXVu0FTOI4Vector(+0xc0)`),
    /// clamped to the draw buffer's scissor, is read back from the depth
    /// buffer into a scratch buffer (`sceGsSetDefStoreImage`,
    /// `sceGsExecStoreImage`: the renderer seam, calls through `call`;
    /// no-op stubs in `FFX.exe`), the texels whose 24-bit z lies below the
    /// point's are counted, and with `v = count / (+0xd0 * +0xd1)` the
    /// visibility becomes `v * +0x7c` and `+0xac` becomes `v` (both 0 for
    /// an empty box). The locals lie below `frame`.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn ppp_ke_lns_lp_t_ot_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        frame_no: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{div, load, mul, store};
        let at = |o: u32| obj.wrapping_add(o);
        if mem.read::<u16>(at(0xcc)) != mem.read::<u16>(g(mem, LENS_VIEW)) {
            mem.write(at(0x7c), 0u32);
            return;
        }
        // fucompp: equal to 0.0 (not unordered)
        if f(mem, at(0x7c)) == 0.0 {
            return;
        }
        let m = i32::from(mem.read::<u8>(at(0x8d))) + 1;
        let a = i32::from(mem.read::<u8>(at(0x8c)));
        if a % m != frame_no.cast_signed().wrapping_rem(m) {
            let v = store(mul(f(mem, at(0xac)), f(mem, at(0x7c))));
            mem.write(at(0x7c), v);
            return;
        }
        let slot = frame.wrapping_sub(0x98);
        mem.write(slot, m.cast_unsigned());
        let dc = call(mem, g(mem, GET_DBUFF_DC), &[slot]);
        let gs = if mem.read::<u8>(slot) & 1 != 0 {
            dc.wrapping_add(0x190)
        } else {
            dc.wrapping_add(0x60)
        };
        let ox = u32::from(mem.read::<u16>(gs.wrapping_add(0x18)));
        let oy = u32::from(mem.read::<u16>(gs.wrapping_add(0x1c)));
        let sx: u32 = mem.read(gs.wrapping_add(0x24));
        let sy: u32 = mem.read(gs.wrapping_add(0x28));
        let x0 = ((sx & 0x7ff) << 4).wrapping_add(ox).cast_signed();
        let x1 = ((((sx >> 16) & 0x7ff) + 1) << 4)
            .wrapping_add(ox)
            .cast_signed();
        let y0 = ((sy & 0x7ff) << 4).wrapping_add(oy).cast_signed();
        let y1 = ((((sy >> 16) & 0x7ff) + 1) << 4)
            .wrapping_add(oy)
            .cast_signed();
        let v = frame.wrapping_sub(0x14);
        call(mem, g(mem, FTOI4_VECTOR), &[v, at(0xc0)]);
        let px = mem.read::<u32>(v).cast_signed();
        let py = mem.read::<u32>(v.wrapping_add(4)).cast_signed();
        let hx = i32::from(mem.read::<u8>(at(0xd0)));
        let hy = i32::from(mem.read::<u8>(at(0xd1)));
        // clamped low end first, as the original
        let clamp = |x: i32, lo: i32, hi: i32| {
            if x < lo {
                lo
            } else if x > hi {
                hi
            } else {
                x
            }
        };
        let left = clamp(px.wrapping_sub(hx * 8), x0, x1);
        let top = clamp(py.wrapping_sub(hy * 8), y0, y1);
        let right = clamp(px.wrapping_add(hx * 8), x0, x1);
        let bottom = clamp(py.wrapping_add(hy * 8), y0, y1);
        let w = right.wrapping_sub(left).wrapping_add(0xf) >> 4;
        let h = bottom.wrapping_sub(top).wrapping_add(0xf) >> 4;
        if w <= 0 || h <= 0 {
            mem.write(at(0x7c), 0u32);
            mem.write(at(0xac), 0u32);
            return;
        }
        let wa = (w.wrapping_add(7)) & !7;
        let buf = call(mem, g(mem, SPR_ALLOC), &[0x3060]);
        let ha = (h + 3) & !3;
        let tx = left.wrapping_sub(ox.cast_signed()) >> 4;
        let ty = top.wrapping_sub(oy.cast_signed()) >> 4;
        let psm = u32::from(mem.read::<u16>(gs.wrapping_add(2)) & 0x3f);
        let fbp = u32::from((mem.read::<u16>(gs.wrapping_add(0xc)) & 0x1ff) << 5);
        // the renderer seam: the depth readback
        let lp = frame.wrapping_sub(0x80);
        call(
            mem,
            g(mem, SET_DEF_STORE_IMAGE),
            &[
                lp,
                fbp,
                psm,
                0x31,
                tx.cast_unsigned(),
                ty.cast_unsigned(),
                wa.cast_unsigned(),
                ha.cast_unsigned(),
            ],
        );
        call(mem, g(mem, FLUSH_CACHE), &[0]);
        let mut p = buf.wrapping_add(0x60);
        call(mem, g(mem, EXEC_STORE_IMAGE), &[lp, p]);
        call(mem, g(mem, SYNC_PATH), &[0, 0]);
        let z = (mem.read::<u32>(v.wrapping_add(8)).cast_signed() >> 4).cast_unsigned();
        let pad = (wa - w).wrapping_mul(3).cast_unsigned();
        let mut count = 0.0f32;
        for _ in 0..h {
            for _ in 0..w {
                let t = u32::from_le_bytes([
                    mem.read(p),
                    mem.read(p.wrapping_add(1)),
                    mem.read(p.wrapping_add(2)),
                    0,
                ]);
                if t < z {
                    count += 1.0;
                }
                p = p.wrapping_add(3);
            }
            p = p.wrapping_add(pad);
        }
        call(mem, g(mem, SPR_FREE), &[0x3060]);
        #[allow(clippy::cast_precision_loss)] // fild, then fstp m32
        let area = hy.wrapping_mul(hx) as f32;
        let vis = store(div(f64::from(count), load(area.to_bits())));
        mem.write(at(0x7c), store(mul(load(vis), f(mem, at(0x7c)))));
        mem.write(at(0xac), vis);
    }

    /// Where a shape particle step finds its key's fields: the lifetime
    /// (`u32`), the velocity step and the spin step (three `f32` each).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ShapeStep {
        /// `0x007472b0` (`pppDrawShapeCameraDraw`) and `0x00748500`
        /// (`pppDrawShapeCameraDisPosDraw`): `+0x50`, `+0x50`, `+0x90`.
        Camera,
        /// `0x00743da0` (`pppDrawShapeFieldRevDraw`): `+0x54`, `+0x60`,
        /// `+0xa0`.
        FieldRev,
    }

    /// The shape draws' particle step (`0x007472b0`, `0x00748500`,
    /// `0x00743da0`; [`ShapeStep`] picks the key layout) over `obj +8`
    /// (`i32`) 0xa0-byte particles at `ptr32(key +0x40)`, with the shape
    /// `ptr32(env +0x24 [obj +4])`. Each live one (bit 0 of `+0`): its age
    /// `+8` counts up (at the key's lifetime or past, unsigned, bit 0
    /// clears); its velocity `+0x20..+0x28` gets the key's velocity step
    /// (`obj` value first) and moves the position `+0x10`
    /// (`FFXVu0AddVector`, four lanes); its spin speed `+0x40..+0x48` gets
    /// the spin step and turns `+0x30..+0x38`. The frame clock `u16 +4`
    /// adds `u16 obj +0xc`; at the frame's length (`i16 shape +0x12 + 8 f`
    /// for frame `f = u16 +6`, compared as `i32` from a `u16` clock) it
    /// drops by it and the frame steps on; past the last (`i16 shape +6`)
    /// a looping frame (bit 7 of `shape +0x14 + 8 f`) clears clock and
    /// frame (one dword), else the clock clears and the frame stays.
    pub fn ppp_shape_step<M: Mem>(mem: &mut M, kind: ShapeStep, obj: u32, key: u32) {
        use crate::x87::add;
        use ffx_system::stub::ptr32;
        let (life, vel, spin) = match kind {
            ShapeStep::Camera => (0x50, 0x50, 0x90),
            ShapeStep::FieldRev => (0x54, 0x60, 0xa0),
        };
        let count: i32 = mem.read(obj.wrapping_add(8));
        let parts = ptr32(mem.read(key.wrapping_add(0x40)));
        let env: u32 = mem.read(mem.read::<u32>(g(mem, ENV_SLOT)));
        let shapes: u32 = mem.read(env.wrapping_add(0x24));
        let no: u32 = mem.read(obj.wrapping_add(4));
        let shape = ptr32(mem.read(shapes.wrapping_add(no.wrapping_mul(4))));
        let f = |mem: &M, a: u32| mem.read::<f32>(a);
        let mut r = parts;
        for _ in 0..count.max(0) {
            let flags: u32 = mem.read(r);
            if flags & 1 != 0 {
                let frame = u32::from(mem.read::<u16>(r.wrapping_add(6)));
                let age = mem.read::<u32>(r.wrapping_add(8)).wrapping_add(1);
                mem.write(r.wrapping_add(8), age);
                if age >= mem.read::<u32>(key.wrapping_add(life)) {
                    mem.write(r, flags & !1);
                }
                for (o, at) in [(vel, 0x20), (spin, 0x40)] {
                    for i in 0..3 {
                        let (src, dst) = (obj.wrapping_add(o + 4 * i), r.wrapping_add(at + 4 * i));
                        let v = add(f(mem, src), f(mem, dst));
                        mem.write(dst, v);
                    }
                    if at == 0x20 {
                        let pos = r.wrapping_add(0x10);
                        ffx_ps2::vu0::bridge::add_vector(mem, pos, pos, r.wrapping_add(0x20));
                    }
                }
                for i in 0..3 {
                    let (src, dst) = (r.wrapping_add(0x40 + 4 * i), r.wrapping_add(0x30 + 4 * i));
                    let v = add(f(mem, src), f(mem, dst));
                    mem.write(dst, v);
                }
                let step: u16 = mem.read(obj.wrapping_add(0xc));
                let clock = step.wrapping_add(mem.read(r.wrapping_add(4)));
                mem.write(r.wrapping_add(4), clock);
                let entry = shape.wrapping_add(8 * frame);
                let len: u16 = mem.read(entry.wrapping_add(0x12));
                if i32::from(clock) >= i32::from(len.cast_signed()) {
                    mem.write(r.wrapping_add(4), clock.wrapping_sub(len));
                    let next = mem.read::<u16>(r.wrapping_add(6)).wrapping_add(1);
                    mem.write(r.wrapping_add(6), next);
                    let last = i32::from(mem.read::<i16>(shape.wrapping_add(6)));
                    if i32::from(next) >= last {
                        if mem.read::<u8>(entry.wrapping_add(0x14)) & 0x80 != 0 {
                            mem.write(r.wrapping_add(4), 0u32);
                        } else {
                            mem.write(r.wrapping_add(4), 0u16);
                            mem.write(r.wrapping_add(6), next.wrapping_sub(1));
                        }
                    }
                }
            }
            r = r.wrapping_add(0xa0);
        }
    }

    /// `g_Vu0_Vf00` (`0x00c0a004`): the emulated VU0 `vf00`.
    const VF00: u32 = 0x00c0_a004;
    /// `g_Vu0_VfMove` (`0x00c8f980`): the emulated VU0 register copies
    /// go through.
    const VF_MOVE: u32 = 0x00c8_f980;
    /// `g_pppCameraMatrix` (`0x0230ff60`).
    const CAMERA_MATRIX: u32 = 0x0230_ff60;

    /// The camera shape draws' emitter (`0x00747090`, and the copy
    /// `0x007482e0`; `pppDrawShapeCameraDraw`/`DisPosDraw`), on area 0
    /// `w` of `obj`: while the countdown `w +0xe4` runs it only counts
    /// down; at 0 it births `u8 key +0xb0` particles and reloads it with
    /// `u8 key +0xb1` - 1. Each particle (0xa0 bytes, number `w +0xe8` in
    /// `ptr32(w +0xe0)`, which then steps on and wraps at `key +8`): bit 0
    /// set, clock, frame and age 0; velocity `+0x20` = key `+0x30` plus a
    /// random `key +0x40` (`KeVecAddRandom`), spin speed `+0x30` = key
    /// `+0x60` scaled at random by `key +0x70` (`KeVecScaleRandom`), `+0x40`
    /// = key `+0x80` (`w` 0 for all three), spin `+0x50` = `vf00` plus a
    /// random `key +0xa0`, a unit matrix `+0x60` turned about z by
    /// `f32(spin.z * 4096 / 360)` (`op_rot_m_z`), and the position `+0x10`
    /// = the camera's (row 3 of `FFXVu0InversMatrix(g_pppCameraMatrix)`,
    /// computed once into `frame`, 16 words, and copied through
    /// `g_Vu0_VfMove`) plus a random `key +0x10` scaled by `u8 key
    /// +0x20`. Callees through `call`.
    pub fn ppp_shape_camera_emit<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        ctrl: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use crate::x87::load;
        let offsets: u32 = mem.read(ctrl.wrapping_add(0xc));
        let w = obj.wrapping_add(mem.read(offsets));
        let countdown: u32 = mem.read(w.wrapping_add(0xe4));
        if countdown != 0 {
            mem.write(w.wrapping_add(0xe4), countdown.wrapping_sub(1));
            return;
        }
        let camera = g(mem, CAMERA_MATRIX);
        call(mem, g(mem, 0x006e_d450), &[frame, camera]);
        let (add_random, scale_random) = (g(mem, 0x0076_6f10), g(mem, 0x0076_6f70));
        let mut i = 0i32;
        while i < i32::from(mem.read::<u8>(key.wrapping_add(0xb0))) {
            let parts = ffx_system::stub::ptr32(mem.read(w.wrapping_add(0xe0)));
            let no: u32 = mem.read(w.wrapping_add(0xe8));
            let p = parts.wrapping_add(no.wrapping_mul(0xa0));
            let at = |o: u32| p.wrapping_add(o);
            let flags: u32 = mem.read(p);
            mem.write(p, flags | 1);
            mem.write(at(4), 0u32);
            mem.write(at(8), 0u32);
            // (the original first copies vf00 to +0x10 too: a dead store,
            // the position below overwrites it before any read)
            // fld / fstp copies of the key's vectors, w 0
            for (dst, src) in [(0x20, 0x30), (0x30, 0x60), (0x40, 0x80)] {
                for k in 0..3 {
                    let v: u32 = mem.read(key.wrapping_add(src + 4 * k));
                    mem.write(at(dst + 4 * k), load(v));
                }
                mem.write(at(dst + 0xc), 0.0f32);
                match dst {
                    0x20 => {
                        call(mem, add_random, &[at(0x20), key.wrapping_add(0x40), 0]);
                    }
                    0x30 => {
                        call(mem, scale_random, &[at(0x30), key.wrapping_add(0x70), 0]);
                    }
                    _ => {}
                }
            }
            let vf00: [u32; 4] = mem.read(g(mem, VF00));
            mem.write(at(0x50), vf00);
            call(mem, add_random, &[at(0x50), key.wrapping_add(0xa0), 0]);
            call(mem, g(mem, 0x006e_d9a0), &[at(0x60)]);
            let spin = crate::x87::Reg::load_f32(mem.read(at(0x58)))
                .constant(crate::x87::MUL, 4096.0)
                .constant(|a, b| a / b, 360.0)
                .store();
            // (fld / fstp of a stored value: already quiet)
            call(
                mem,
                g(mem, 0x007e_9760),
                &[at(0x60), at(0x60), spin.to_bits()],
            );
            let mv = g(mem, VF_MOVE);
            let row3: [u32; 4] = mem.read(frame.wrapping_add(0x30));
            for k in [1u32, 2, 3, 0] {
                mem.write(mv.wrapping_add(4 * k), row3[k as usize]);
            }
            let pos: [u32; 4] = mem.read(mv);
            mem.write(at(0x10), pos);
            let scale = u32::from(mem.read::<u8>(key.wrapping_add(0x20)));
            call(mem, add_random, &[at(0x10), key.wrapping_add(0x10), scale]);
            let next = mem.read::<u32>(w.wrapping_add(0xe8)).wrapping_add(1);
            mem.write(w.wrapping_add(0xe8), next);
            if next >= mem.read::<u32>(key.wrapping_add(8)) {
                mem.write(w.wrapping_add(0xe8), 0u32);
            }
            i += 1;
        }
        let reload = u32::from(mem.read::<u8>(key.wrapping_add(0xb1)));
        mem.write(w.wrapping_add(0xe4), reload.wrapping_sub(1));
    }

    /// `0x00745610(p, src)` (`pppDrawShapeFieldSpdInit`, ours; called by
    /// `0x00745f70`): a particle `p` from the source `src`: bit 0 of `+0`
    /// set, `+4`, `+8` = 0; `+0x10` = `vf00`; `+0x20` = `src +0x40` (xyz, w
    /// 0) plus `KeVecAddRandom(.., src +0x50, 0)`; `+0x30` = `src +0x60`
    /// (w 0); `+0x40` = `src +0x70` (w 0) through
    /// `KeVecScaleRandom(.., src +0x80, 0)`; `+0x50` = `src +0x90` (w 0);
    /// `+0x60` = `vf00` plus `KeVecAddRandom(.., src +0xb0, 0)`; `+0x70` =
    /// the unit matrix turned about z by `f32 +0x68 x 4096 / 360`
    /// (`[0x00b515c8]`, `[0x00b48380]`; `op_rot_m_z`). The `KeVec*` callees
    /// through `call`. Floats through the FPU.
    pub fn ppp_shape_field_spd_init<M: Mem>(
        mem: &mut M,
        p: u32,
        src: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{div, load, mul, store};
        let at = |o: u32| p.wrapping_add(o);
        let copy3 = |mem: &mut M, to: u32, from: u32| {
            for k in 0..3 {
                let v = load(mem.read(src.wrapping_add(from + 4 * k)));
                mem.write(at(to + 4 * k), store(v));
            }
            mem.write(at(to + 0xc), 0f32);
        };
        let w: u32 = mem.read(p);
        mem.write(p, w | 1);
        mem.write(at(4), 0u32);
        mem.write(at(8), 0u32);
        let vf00: [u32; 4] = mem.read(g(mem, 0x00c0_a004));
        mem.write(at(0x10), vf00);
        copy3(mem, 0x20, 0x40);
        let add_random = g(mem, 0x0076_6f10);
        call(mem, add_random, &[at(0x20), src.wrapping_add(0x50), 0]);
        copy3(mem, 0x30, 0x60);
        copy3(mem, 0x40, 0x70);
        call(
            mem,
            g(mem, 0x0076_6f70),
            &[at(0x40), src.wrapping_add(0x80), 0],
        );
        copy3(mem, 0x50, 0x90);
        let vf00: [u32; 4] = mem.read(g(mem, 0x00c0_a004));
        mem.write(at(0x60), vf00);
        call(mem, add_random, &[at(0x60), src.wrapping_add(0xb0), 0]);
        let m = at(0x70);
        ffx_ps2::vu0::bridge::matrix_identity(mem, m);
        let (k1, k2): (f64, f64) = (mem.read(g(mem, 0x00b5_15c8)), mem.read(g(mem, 0x00b4_8380)));
        let a = store(div(mul(load(mem.read(at(0x68))), k1), k2));
        crate::op::bridge::op_rt_m(mem, m, m, f32::from_bits(load_bits(a)), crate::mat::Axis::Z);
    }

    /// `f32` bits through `fld m32; fstp m32` (a signalling NaN quiet).
    fn load_bits(bits: u32) -> u32 {
        ffx_bridge::x87::store(ffx_bridge::x87::load(bits))
    }

    // ---- round 35 b ----

    /// `pppEiZCrctDisPosDraw(obj, key, ctrl)` (`0x00735ee0`): the matrix
    /// `m` (in the original's locals at `frame - 0x44`, `frame` its `ebp`)
    /// gets the object's rows `+0x10`, `+0x20`, `+0x30` scaled by the key's
    /// `f32 +0x20`, `+0x24`, `+0x28` (`FFXVu0ScaleVector`) and the position
    /// `obj +0x40 - g_pppWorldMatrix +0x30` (each `f32`) with 1.0. By the
    /// key's byte `+0x30`: 0 adds the key's `+0x10..+0x18` to the
    /// position, then `m = g_pppViewMatrix x m` (`FFXVu0MulMatrix(m,
    /// view, m)`); 1 multiplies first, then adds to the product's position;
    /// 2 `m = g_pppWorldMatrix x m`, adds the current part's scale `+0x60`
    /// times the key's offsets, then `m = g_pppCameraMatrix x m`; others
    /// leave it. `m` is copied into work area 0 (`FFXVu0CopyMatrix`).
    /// Callees through `call`.
    pub fn ppp_ei_z_crct_dis_pos_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        ctrl: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{add, load, mul, store, sub};
        let part: u32 = mem.read(g(mem, 0x0230_ffe4));
        let offsets: u32 = mem.read(ctrl.wrapping_add(0xc));
        let area = obj
            .wrapping_add(0xa0)
            .wrapping_add(mem.read::<u32>(offsets));
        let m = frame.wrapping_sub(0x44);
        let scale = g(mem, 0x006e_d8f0);
        for k in 0..3 {
            let s = store(load(mem.read(key.wrapping_add(0x20 + 4 * k))));
            call(
                mem,
                scale,
                &[m + 0x10 * k, obj.wrapping_add(0x10 + 0x10 * k), s],
            );
        }
        let pos = m.wrapping_add(0x30);
        let world = g(mem, 0x0230_ffa0);
        let f = |mem: &M, at: u32| load(mem.read(at));
        for k in 0..3 {
            let v = sub(
                f(mem, obj.wrapping_add(0x40 + 4 * k)),
                f(mem, world.wrapping_add(0x30 + 4 * k)),
            );
            mem.write(pos + 4 * k, store(v));
        }
        mem.write(pos + 0xc, 1f32);
        let mul_matrix = g(mem, 0x006e_d5c0);
        let add_key = |mem: &mut M, scaled: bool| {
            for k in 0..3 {
                let o = f(mem, key.wrapping_add(0x10 + 4 * k));
                let o = if scaled {
                    mul(f(mem, part.wrapping_add(0x60 + 4 * k)), o)
                } else {
                    o
                };
                let v = add(o, f(mem, pos + 4 * k));
                mem.write(pos + 4 * k, store(v));
            }
        };
        let view = g(mem, 0x0230_ff20);
        match mem.read::<u8>(key.wrapping_add(0x30)) {
            0 => {
                add_key(mem, false);
                call(mem, mul_matrix, &[m, view, m]);
            }
            1 => {
                call(mem, mul_matrix, &[m, view, m]);
                add_key(mem, false);
            }
            2 => {
                call(mem, mul_matrix, &[m, world, m]);
                add_key(mem, true);
                call(mem, mul_matrix, &[m, g(mem, 0x0230_ff60), m]);
            }
            _ => {}
        }
        call(mem, g(mem, 0x006e_d360), &[area, m]);
    }

    /// `0x00745f70` (`pppDrawShapeFieldGlobalEmit`, ours; `(obj, def, ctrl)`,
    /// called by `pppDrawShapeFieldGlobalDraw`): in work area 0 `w = obj +
    /// 0xa0 + [[ctrl +0xc]]` the countdown `+0x44` steps down; at 0 it
    /// emits `u8 def +0xc0` particles (the count read again each time) and
    /// restarts at `u8 def +0xc1 - 1`. A particle is the 0xb0-byte record
    /// `+0x48` (a ring of `def +8`) of the array `get_ptr32_func(w +0x40)`:
    /// `pppDrawShapeFieldSpdInit(rec, def, w)`, the emulated `VfMove`
    /// register and `rec +0x10` take `g_pppWorldMatrix +0x30` (four words),
    /// `KeVecAddRandom(rec +0x10, def +0x10, u8 def +0x20)`, and `rec
    /// +0x20` and `+0x30` go through a copy of `g_pppWorldMatrix` without
    /// its translation (`FFXVu0ApplyMatrix`; the copy in the original's
    /// locals at `frame - 0x44`, `frame` its `ebp`, its `x, y, z`
    /// translation zeroed). Returns the new countdown (left in eax).
    /// Callees through `call`.
    pub fn ppp_draw_shape_field_global_emit<M: Mem>(
        mem: &mut M,
        obj: u32,
        def: u32,
        ctrl: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let offsets: u32 = mem.read(ctrl.wrapping_add(0xc));
        let w = obj
            .wrapping_add(0xa0)
            .wrapping_add(mem.read::<u32>(offsets));
        let left: u32 = mem.read(w + 0x44);
        if left != 0 {
            let n = left.wrapping_sub(1);
            mem.write(w + 0x44, n);
            return n;
        }
        let m = frame.wrapping_sub(0x44);
        let world = g(mem, 0x0230_ffa0);
        call(mem, g(mem, 0x006e_d360), &[m, world]);
        // the original's zeroed locals at ebp -0x14..-0xc are the copy's
        // translation
        for at in [0x38, 0x34, 0x30] {
            mem.write(m + at, 0f32);
        }
        let mut i = 0u32;
        while i < u32::from(mem.read::<u8>(def.wrapping_add(0xc0))) {
            let base = ffx_system::stub::ptr32(mem.read(w + 0x40));
            let rec = mem
                .read::<u32>(w + 0x48)
                .wrapping_mul(0xb0)
                .wrapping_add(base);
            call(mem, g(mem, 0x0074_5610), &[rec, def, w]);
            let at: [u32; 4] = mem.read(world + 0x30);
            mem.write(g(mem, 0x00c8_f980), at);
            mem.write(rec.wrapping_add(0x10), at);
            let spread = u32::from(mem.read::<u8>(def.wrapping_add(0x20)));
            let args = [rec.wrapping_add(0x10), def.wrapping_add(0x10), spread];
            call(mem, g(mem, 0x0076_6f10), &args);
            for off in [0x20, 0x30] {
                let v = rec.wrapping_add(off);
                call(mem, g(mem, 0x006e_d2e0), &[v, m, v]);
            }
            let next = mem.read::<u32>(w + 0x48).wrapping_add(1);
            mem.write(w + 0x48, next);
            if next >= mem.read::<u32>(def.wrapping_add(8)) {
                mem.write(w + 0x48, 0u32);
            }
            i += 1;
        }
        let n = u32::from(mem.read::<u8>(def.wrapping_add(0xc1))).wrapping_sub(1);
        mem.write(w + 0x44, n);
        n
    }

    /// `0x007454a0` (`pppDrawShapeFieldSpdAlloc`, ours; `(obj, def, ctrl)`,
    /// called by `pppDrawShapeFieldSpdDraw`): in work area 0 `w = obj +
    /// 0xa0 + [[ctrl +0xc]]`, four buffers from the current environment's
    /// heap (`pppMemAlloc(env, env + 0x10, size)`, `env = [[0x00c3a4cc]]`),
    /// each kept as `set_ptr32`: `+0x40` the particles (`def +8 x 0xa0`
    /// bytes, zeroed), `+0x44` 0xc bytes, `+0x50` (`w +0x58 x 8`, zeroed)
    /// and `+0x54` (`w +0x58 x 16`, zeroed); the first null one returns 0,
    /// else 1. Callees through `call` (`memset` through its thunk
    /// `0x0094964c`).
    pub fn ppp_draw_shape_field_spd_alloc<M: Mem>(
        mem: &mut M,
        obj: u32,
        def: u32,
        ctrl: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let offsets: u32 = mem.read(ctrl.wrapping_add(0xc));
        let w = obj
            .wrapping_add(0xa0)
            .wrapping_add(mem.read::<u32>(offsets));
        let count: u32 = mem.read(def.wrapping_add(8));
        // (field, size, zeroed); the sizes read when they come up
        for (at, zeroed) in [(0x40, true), (0x44, false), (0x50, true), (0x54, true)] {
            let size = match at {
                0x40 => count.wrapping_mul(0xa0),
                0x44 => 0xc,
                0x50 => mem.read::<u32>(w + 0x58) << 3,
                _ => mem.read::<u32>(w + 0x58) << 4,
            };
            let env: u32 = mem.read(crate::heap::bridge::ENV_SLOT.get(mem));
            let block = call(
                mem,
                g(mem, 0x0071_6a30),
                &[env, env.wrapping_add(0x10), size],
            );
            let kept = ffx_system::stub::ptr32(block);
            mem.write(w + at, kept);
            if ffx_system::stub::is_ptr32_null(kept) != 0 {
                return 0;
            }
            if zeroed {
                let to = ffx_system::stub::ptr32(mem.read(w + at));
                call(mem, g(mem, 0x0094_964c), &[to, 0, size]);
            }
        }
        1
    }

    // ---- round 35 d ----

    /// The animation of the shape field steps: the record's `u16 +4`
    /// (time) goes on by `step`; reaching `i16 anim[8 frame +0x12]` (the
    /// frame's length, `frame` the record's `u16 +6` read before) it drops
    /// by it and the frame goes on; past the last frame (`i16 anim +6`) a
    /// frame flagged `0x80` (byte `anim[8 frame +0x14]`) clears time and
    /// frame (one word), else the time is 0 and the frame stays.
    fn shape_field_anim(mem: &mut impl Mem, rec: u32, step: u16, anim: u32, frame: u32) {
        let entry = anim.wrapping_add(frame << 3);
        let time = step.wrapping_add(mem.read::<u16>(rec + 4));
        mem.write(rec + 4, time);
        let length = mem.read::<u16>(entry.wrapping_add(0x12));
        if i32::from(time) < i32::from(length.cast_signed()) {
            return;
        }
        let next = mem.read::<u16>(rec + 6).wrapping_add(1);
        mem.write(rec + 4, time.wrapping_sub(length));
        mem.write(rec + 6, next);
        if i32::from(next) < i32::from(mem.read::<i16>(anim.wrapping_add(6))) {
            return;
        }
        if mem.read::<u8>(entry.wrapping_add(0x14)) & 0x80 != 0 {
            mem.write(rec + 4, 0u32);
        } else {
            mem.write(rec + 4, 0u16);
            mem.write(rec + 6, next.wrapping_sub(1));
        }
    }

    /// The particle array (`get_ptr32_func(w +0x40)`), the animation
    /// (`get_ptr32_func` of word `def +4` of the current environment's
    /// table `+0x24`) and the count (`i32 def +8`) of a shape field step.
    fn shape_field_parts(mem: &impl Mem, def: u32, w: u32) -> (u32, u32, i32) {
        use ffx_system::stub::ptr32;
        let parts = ptr32(mem.read(w.wrapping_add(0x40)));
        let env: u32 = mem.read(crate::heap::bridge::ENV_SLOT.get(mem));
        let table: u32 = mem.read(env.wrapping_add(0x24));
        let i: u32 = mem.read(def.wrapping_add(4));
        let anim = ptr32(mem.read(table.wrapping_add(i << 2)));
        (parts, anim, mem.read(def.wrapping_add(8)))
    }

    /// `f32 [to + 4k] = [a + 4k] + [b + 4k]` for `k` 0..3 (`fld a; fadd b`).
    fn add3(mem: &mut impl Mem, to: u32, a: u32, b: u32) {
        use ffx_bridge::x87::{add, load, store};
        for k in 0..3 {
            let v = add(load(mem.read(a + 4 * k)), load(mem.read(b + 4 * k)));
            mem.write(to + 4 * k, store(v));
        }
    }

    /// `0x00744f60` (`pppDrawShapeFieldSpdStep`, ours; `(def, w)`, called
    /// by `pppDrawShapeFieldSpdDraw`): each live (bit 0) 0xa0-byte
    /// particle `p` of `def +8`: `p +0x10 += vel[i32 p +8]`
    /// (`FFXVu0AddVector`, `vel` the 16-byte vectors at `get_ptr32_func(w
    /// +0x54)`), `p +0x40 += def +0x90` and `p +0x30 += p +0x40` (`x, y,
    /// z`, `f32`), the animation steps by `u16 def +0xc`, then `p +8` counts
    /// up and the particle dies (bit 0 cleared) at `w +0x58`. Callees
    /// through `call`.
    pub fn ppp_draw_shape_field_spd_step<M: Mem>(
        mem: &mut M,
        def: u32,
        w: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let (parts, anim, n) = shape_field_parts(mem, def, w);
        let mut p = parts;
        for _ in 0..n.max(0) {
            if mem.read::<u32>(p) & 1 != 0 {
                let frame = u32::from(mem.read::<u16>(p.wrapping_add(6)));
                let vels = ffx_system::stub::ptr32(mem.read(w.wrapping_add(0x54)));
                let vel = vels.wrapping_add(mem.read::<u32>(p.wrapping_add(8)) << 4);
                let at = p.wrapping_add(0x10);
                call(mem, g(mem, 0x006e_d290), &[at, at, vel]);
                add3(
                    mem,
                    p.wrapping_add(0x40),
                    def.wrapping_add(0x90),
                    p.wrapping_add(0x40),
                );
                add3(
                    mem,
                    p.wrapping_add(0x30),
                    p.wrapping_add(0x40),
                    p.wrapping_add(0x30),
                );
                let step: u16 = mem.read(def.wrapping_add(0xc));
                shape_field_anim(mem, p, step, anim, frame);
                let age = mem.read::<u32>(p.wrapping_add(8)).wrapping_add(1);
                mem.write(p.wrapping_add(8), age);
                if age >= mem.read::<u32>(w.wrapping_add(0x58)) {
                    let flags = mem.read::<u32>(p) & !1;
                    mem.write(p, flags);
                }
            }
            p = p.wrapping_add(0xa0);
        }
    }

    /// `0x007460b0` (`pppDrawShapeFieldGlobalStep`, ours; `(def, w)`,
    /// called by `pppDrawShapeFieldGlobalDraw`): each live (bit 0)
    /// 0xb0-byte particle `p` of `def +8`: `p +8` counts up and the
    /// particle dies (bit 0 cleared, from the flags read before) at `w
    /// +0x50`, but this step still runs: `p +0x20 += p +0x30`, `p +0x10 +=
    /// p +0x20` (`FFXVu0AddVector`), `p +0x50 += def +0xa0`, `p +0x40 += p
    /// +0x50` (`x, y, z`, `f32`), and the animation steps by `u16 def
    /// +0xc`. Callees through `call`.
    pub fn ppp_draw_shape_field_global_step<M: Mem>(
        mem: &mut M,
        def: u32,
        w: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let (parts, anim, n) = shape_field_parts(mem, def, w);
        let mut p = parts;
        for _ in 0..n.max(0) {
            let flags: u32 = mem.read(p);
            if flags & 1 != 0 {
                let frame = u32::from(mem.read::<u16>(p.wrapping_add(6)));
                let age = mem.read::<u32>(p.wrapping_add(8)).wrapping_add(1);
                mem.write(p.wrapping_add(8), age);
                if age >= mem.read::<u32>(w.wrapping_add(0x50)) {
                    mem.write(p, flags & !1);
                }
                let add = g(mem, 0x006e_d290);
                let (pos, vel, acc) = (
                    p.wrapping_add(0x10),
                    p.wrapping_add(0x20),
                    p.wrapping_add(0x30),
                );
                call(mem, add, &[vel, vel, acc]);
                call(mem, add, &[pos, pos, vel]);
                add3(
                    mem,
                    p.wrapping_add(0x50),
                    def.wrapping_add(0xa0),
                    p.wrapping_add(0x50),
                );
                add3(
                    mem,
                    p.wrapping_add(0x40),
                    p.wrapping_add(0x40),
                    p.wrapping_add(0x50),
                );
                let step: u16 = mem.read(def.wrapping_add(0xc));
                shape_field_anim(mem, p, step, anim, frame);
            }
            p = p.wrapping_add(0xb0);
        }
    }

    // ---- round 36 b ----

    /// `0x00742980` (`pppDrawShapeFieldStep`, ours; `(base, def, w)`,
    /// called by `pppDrawShapeFieldDraw`): each live (bit 0) 0xa0-byte
    /// particle `p` (index `i`): `p +8` counts up and at `w +0x54` the
    /// particle dies (bit 0 cleared, from the flags read before) and, on the
    /// [`SHAPE_FIELD_PLAIN_LEVELS`] (`graphicLevelGetNameHash()`), gets
    /// `graphicVFXDestroy(id)` with the 64-bit `id = (base << 32) + 0x32 i +
    /// g_pppCurrentProgram` (both sign-extended); the step still runs: `p
    /// +0x20 += def +0x60`, `p +0x10 += p +0x20` (`FFXVu0AddVector`), `p
    /// +0x40 += def +0xa0`, `p +0x30 += p +0x40` (`f32`), and the animation
    /// steps by `u16 def +0xc`. Callees through `call`.
    pub fn ppp_draw_shape_field_step<M: Mem>(
        mem: &mut M,
        base: u32,
        def: u32,
        w: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let (parts, anim, n) = shape_field_parts(mem, def, w);
        let mut p = parts;
        let mut id = 0i32;
        for _ in 0..n.max(0) {
            let flags: u32 = mem.read(p);
            if flags & 1 != 0 {
                let frame = u32::from(mem.read::<u16>(p.wrapping_add(6)));
                let age = mem.read::<u32>(p.wrapping_add(8)).wrapping_add(1);
                mem.write(p.wrapping_add(8), age);
                if age >= mem.read::<u32>(w.wrapping_add(0x54)) {
                    mem.write(p, flags & !1);
                    let hash = call(mem, g(mem, 0x0064_1da0), &[]);
                    if SHAPE_FIELD_PLAIN_LEVELS.contains(&hash) {
                        let prog = mem.read::<i32>(g(mem, CURRENT_PROGRAM));
                        let v = (i64::from(base) << 32)
                            .wrapping_add(i64::from(id))
                            .wrapping_add(i64::from(prog))
                            .cast_unsigned();
                        #[allow(clippy::cast_possible_truncation)] // the halves
                        let (lo, hi) = (v as u32, (v >> 32) as u32);
                        call(mem, g(mem, 0x0064_5280), &[lo, hi]);
                    }
                }
                add3(
                    mem,
                    p.wrapping_add(0x20),
                    def.wrapping_add(0x60),
                    p.wrapping_add(0x20),
                );
                let at = p.wrapping_add(0x10);
                call(mem, g(mem, 0x006e_d290), &[at, at, p.wrapping_add(0x20)]);
                add3(
                    mem,
                    p.wrapping_add(0x40),
                    def.wrapping_add(0xa0),
                    p.wrapping_add(0x40),
                );
                add3(
                    mem,
                    p.wrapping_add(0x30),
                    p.wrapping_add(0x30),
                    p.wrapping_add(0x40),
                );
                let step: u16 = mem.read(def.wrapping_add(0xc));
                shape_field_anim(mem, p, step, anim, frame);
            }
            p = p.wrapping_add(0xa0);
            id = id.wrapping_add(0x32);
        }
    }

    /// `0x00744d80` (`pppDrawShapeFieldSpdEmit`, ours; `(obj, def, ctrl)`,
    /// called by `pppDrawShapeFieldSpdDraw`): the work area `a = obj +
    /// *ctrl[+0xc]` keeps a countdown at `+0xe8`; when it is 0, `u8 def
    /// +0xb0` particles are born at the ring index `a +0xec` (wrapping at
    /// `def +8`) of the array `get_ptr32_func(a +0xe0)`, 0xa0 bytes each:
    /// bit 0 set, `+4`, `+8` = 0, `+0x20` = `def +0x40` randomised by `def
    /// +0x50` (`KeVecAddRandom`), `+0x30` = `def +0x60` scaled by `def
    /// +0x70` (`KeVecScaleRandom`), `+0x40` = `def +0x80`, `+0x50` =
    /// `vf00` plus `def +0xa0`, `+0x60` the unit matrix turned about z by
    /// `f32 (+0x58 x 4096 / 360)` (`op_rot_m_z`), `+0x10` = `vf00` plus
    /// `def +0x10` (mode `u8 def +0x20`), and the countdown starts again
    /// at `u8 def +0xb1`. Then it counts down by one; returns the new value
    /// (left in `eax`). Callees through `call`.
    pub fn ppp_draw_shape_field_spd_emit<M: Mem>(
        mem: &mut M,
        obj: u32,
        def: u32,
        ctrl: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        shape_field_emit(mem, obj, def, ctrl, Emit::<NoFloat<M>>::Spd, call)
    }

    /// `0x00742750` (`pppDrawShapeFieldEmit`, ours; `(obj, def, ctrl)`,
    /// called by `pppDrawShapeFieldDraw`): as
    /// [`ppp_draw_shape_field_spd_emit`] with the count at `u8 def +0xc0`
    /// and the restart at `+0xc1`, `+0x30` = `def +0x70` scaled by `def
    /// +0x80`, `+0x40` = `def +0x90`, `+0x50` randomised by `def +0xb0`,
    /// and `+0x60` turned by `pppGetRotMatrixZ(ftol(t x 4096 x 16 / 360))`,
    /// `t` the angle `f32 +0x58` brought into 0..360 by one turn (+360
    /// below 0, -360 above 360, rounded to `f32`; NaN as it is).
    pub fn ppp_draw_shape_field_emit<M: Mem>(
        mem: &mut M,
        obj: u32,
        def: u32,
        ctrl: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        shape_field_emit(mem, obj, def, ctrl, Emit::<NoFloat<M>>::Plain, call)
    }

    /// `0x00743b60` (`pppDrawShapeFieldRevEmit`, ours; `(obj, def, ctrl)`,
    /// called by `pppDrawShapeFieldRevDraw`): [`ppp_draw_shape_field_emit`]'s
    /// layout with the `Spd` angle (`op_rot_m_z`), and after it, for bit 2
    /// then bit 1 of `def +0xf1`, when `op_random(1.0)` is above 0 the `x`
    /// (then `y`) of `+0x30` and `+0x40` change sign.
    pub fn ppp_draw_shape_field_rev_emit<M: Mem>(
        mem: &mut M,
        obj: u32,
        def: u32,
        ctrl: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        shape_field_emit(mem, obj, def, ctrl, Emit::Rev(fcall), call)
    }

    /// The float callback type of the emits that call none.
    type NoFloat<M> = fn(&mut M, u32, &[u32]) -> f64;

    /// Which shape field emit: its layout, angle and flips.
    enum Emit<'a, F> {
        Spd,
        Plain,
        /// With `op_random`'s `ST0` through the callback.
        Rev(&'a mut F),
    }

    /// The three shape field emits.
    #[allow(clippy::many_single_char_names)]
    fn shape_field_emit<M: Mem, F: FnMut(&mut M, u32, &[u32]) -> f64>(
        mem: &mut M,
        obj: u32,
        def: u32,
        ctrl: u32,
        mut kind: Emit<'_, F>,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{add, div, ftol, load, mul, store, sub};
        let plain = !matches!(kind, Emit::Spd);
        // count, restart, velocity, its scale, acceleration, angle range
        let (count, restart, vel, vel_scale, acc, spin) = if plain {
            (0xc0, 0xc1, 0x70, 0x80, 0x90, 0xb0)
        } else {
            (0xb0, 0xb1, 0x60, 0x70, 0x80, 0xa0)
        };
        let off: u32 = mem.read(mem.read::<u32>(ctrl.wrapping_add(0xc)));
        let a = obj.wrapping_add(off);
        let mut timer: u32 = mem.read(a.wrapping_add(0xe8));
        if timer == 0 {
            let vf00 = crate::prog::bridge::VF00.ptr(mem).addr;
            let copy = |mem: &mut M, to: u32, from: u32| {
                for k in 0..3 {
                    let v = store(load(mem.read(from.wrapping_add(4 * k))));
                    mem.write(to.wrapping_add(4 * k), v);
                }
                mem.write(to.wrapping_add(0xc), 0u32);
            };
            let set_vf00 = |mem: &mut M, to: u32| {
                for k in 0..4 {
                    let v: u32 = mem.read(vf00.wrapping_add(4 * k));
                    mem.write(to.wrapping_add(4 * k), v);
                }
            };
            let mut i = 0i32;
            while i < i32::from(mem.read::<u8>(def.wrapping_add(count))) {
                let parts = ffx_system::stub::ptr32(mem.read(a.wrapping_add(0xe0)));
                let idx: u32 = mem.read(a.wrapping_add(0xec));
                let p = parts.wrapping_add(idx.wrapping_mul(0xa0));
                let flags: u32 = mem.read(p);
                mem.write(p, flags | 1);
                mem.write(p.wrapping_add(4), [0u32; 2]);
                set_vf00(mem, p.wrapping_add(0x10));
                let add_random = g(mem, 0x0076_6f10);
                let scale_random = g(mem, 0x0076_6f70);
                copy(mem, p.wrapping_add(0x20), def.wrapping_add(0x40));
                let args = [p.wrapping_add(0x20), def.wrapping_add(0x50), 0];
                call(mem, add_random, &args);
                copy(mem, p.wrapping_add(0x30), def.wrapping_add(vel));
                let args = [p.wrapping_add(0x30), def.wrapping_add(vel_scale), 0];
                call(mem, scale_random, &args);
                copy(mem, p.wrapping_add(0x40), def.wrapping_add(acc));
                set_vf00(mem, p.wrapping_add(0x50));
                let args = [p.wrapping_add(0x50), def.wrapping_add(spin), 0];
                call(mem, add_random, &args);
                let m = p.wrapping_add(0x60);
                call(mem, g(mem, 0x006e_d9a0), &[m]);
                let k4096: f64 = mem.read(g(mem, 0x00b5_15c8));
                let k360: f64 = mem.read(g(mem, 0x00b4_8380));
                let angle = load(mem.read(p.wrapping_add(0x58)));
                if matches!(kind, Emit::Plain) {
                    let k16: f64 = mem.read(g(mem, 0x00b3_13a0));
                    let top = load(mem.read(g(mem, 0x00b5_15d0)));
                    let t = if angle < 0.0 {
                        load(store(add(angle, k360)))
                    } else if top < angle {
                        load(store(sub(angle, k360)))
                    } else {
                        angle
                    };
                    let turn = ftol(mem, div(mul(mul(t, k4096), k16), k360));
                    call(mem, g(mem, 0x0072_cb10), &[m, turn]);
                } else {
                    let turn = div(mul(angle, k4096), k360);
                    call(mem, g(mem, 0x007e_9760), &[m, m, store(turn)]);
                }
                if let Emit::Rev(fcall) = &mut kind {
                    let flips: u8 = mem.read(def.wrapping_add(0xf1));
                    for (bit, o) in [(2u8, 0u32), (1, 4)] {
                        if flips & bit == 0 {
                            continue;
                        }
                        let r = fcall(mem, g(mem, 0x007e_6700), &[1f32.to_bits()]);
                        if 0.0 < r {
                            for at in [0x30 + o, 0x40 + o] {
                                let v = load(mem.read(p.wrapping_add(at)));
                                mem.write(p.wrapping_add(at), store(-v));
                            }
                        }
                    }
                }
                set_vf00(mem, p.wrapping_add(0x10));
                let mode = u32::from(mem.read::<u8>(def.wrapping_add(0x20)));
                let args = [p.wrapping_add(0x10), def.wrapping_add(0x10), mode];
                call(mem, add_random, &args);
                let next = mem.read::<u32>(a.wrapping_add(0xec)).wrapping_add(1);
                mem.write(a.wrapping_add(0xec), next);
                if next >= mem.read::<u32>(def.wrapping_add(8)) {
                    mem.write(a.wrapping_add(0xec), 0u32);
                }
                i += 1;
            }
            timer = u32::from(mem.read::<u8>(def.wrapping_add(restart)));
        }
        let left = timer.wrapping_sub(1);
        mem.write(a.wrapping_add(0xe8), left);
        left
    }

    /// `0x00756320` (`pppDrawRainAlloc`, ours; `(def, work)`, called by
    /// `pppDrawRainDraw`): `work +4` = `pppMemAlloc(env, env +0x10, size)`
    /// for `n = def +8` drops of 0x30 bytes (0 and nothing else when that fails), each
    /// drop `q`: `+0..+8` = `op_random(f32 def +4)` plus the camera position
    /// (row 3 of `FFXVu0InversMatrix(g_pppCameraMatrix)`, in the original's
    /// locals at `frame - 0x44`, `frame` its `ebp`; also to `g_Vu0_VfMove`),
    /// `+0xc` = 1.0, `+0x10` = `def +0x10..+0x18` (w 0) plus
    /// `KeVecAddRandom(def +0x20)` (also to `g_Vu0_VfMove`), `+0x20` that
    /// normalised (`FFXVu0Normalize`) and scaled by minus `op_random(def
    /// +0x34) + def +0x30` (`rcuiScaleVector_copy_00756600`, `junk` the three
    /// words it reads past its arguments: the caller's saved `edi`, `esi`,
    /// `ebx`); then `work +0` = `def`. Returns 1, or 0 on the failure.
    /// Callees through `call`, `op_random`'s `ST0` through `fcall`.
    #[allow(clippy::too_many_arguments, clippy::many_single_char_names)]
    pub fn ppp_draw_rain_alloc<M: Mem>(
        mem: &mut M,
        def: u32,
        work: u32,
        frame: u32,
        junk: [u32; 3],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        use ffx_bridge::x87::{add, load, store};
        use ffx_system::stub::{is_ptr32_null, ptr32};
        let inverse = frame.wrapping_sub(0x44);
        call(mem, g(mem, 0x006e_d450), &[inverse, g(mem, 0x0230_ff60)]);
        let vf_move = g(mem, 0x00c8_f980);
        let copy4 = |mem: &mut M, to: u32, from: u32| {
            for k in 0..4 {
                let v: u32 = mem.read(from.wrapping_add(4 * k));
                mem.write(to.wrapping_add(4 * k), v);
            }
        };
        let eye = inverse.wrapping_add(0x30);
        copy4(mem, vf_move, eye);
        let env: u32 = mem.read(crate::heap::bridge::ENV_SLOT.get(mem));
        let n: u32 = mem.read(def.wrapping_add(8));
        let size = n.wrapping_mul(0x30);
        let buf = call(
            mem,
            g(mem, 0x0071_6a30),
            &[env, env.wrapping_add(0x10), size],
        );
        let p = ptr32(buf);
        mem.write(work.wrapping_add(4), p);
        if is_ptr32_null(p) != 0 {
            return 0;
        }
        let random = g(mem, 0x007e_6700);
        let mut i = 0u32;
        while i < mem.read::<u32>(def.wrapping_add(8)) {
            let q = buf.wrapping_add(0x30 * i);
            i += 1;
            for k in 0..3 {
                let spread: u32 = mem.read(def.wrapping_add(4));
                let r = fcall(mem, random, &[store(load(spread))]);
                let e = load(mem.read(eye.wrapping_add(4 * k)));
                mem.write(q.wrapping_add(4 * k), store(add(r, e)));
            }
            mem.write(q.wrapping_add(0xc), 1f32);
            let dir = q.wrapping_add(0x10);
            for k in 0..3 {
                let v = store(load(mem.read(def.wrapping_add(0x10 + 4 * k))));
                mem.write(dir.wrapping_add(4 * k), v);
            }
            mem.write(dir.wrapping_add(0xc), 0u32);
            call(mem, g(mem, 0x0076_6f10), &[dir, def.wrapping_add(0x20), 0]);
            copy4(mem, vf_move, dir);
            let vel = q.wrapping_add(0x20);
            copy4(mem, vel, dir);
            call(mem, g(mem, 0x006e_d6f0), &[vel, vel]);
            let spread: u32 = mem.read(def.wrapping_add(0x34));
            let r = fcall(mem, random, &[store(load(spread))]);
            let speed = store(-add(r, load(mem.read(def.wrapping_add(0x30)))));
            let args = [vel, vel, store(load(speed)), junk[0], junk[1], junk[2]];
            call(mem, g(mem, 0x0075_6600), &args);
        }
        mem.write(work, ptr32(def));
        1
    }

    /// `0x00755a00` (`pppDrawRainStep`, ours; `(obj, def, ctrl)`, called by
    /// `pppDrawRainDraw`): moves the drops of [`ppp_draw_rain_alloc`]
    /// (`ptr32(obj + [[ctrl +0xc] +4] + 0xa4)`, `def +8` drops of 0x30
    /// bytes) and re-spawns the ones leaving the box of half size `d = def
    /// +4` around the camera (row 3 of `FFXVu0InversMatrix
    /// (g_pppCameraMatrix)`, in the original's locals at `frame - 0x44`,
    /// `frame` its `ebp`; also to `g_Vu0_VfMove`). Per drop: position
    /// (`+0`) += direction (`+0x10`), all four lanes, through
    /// `g_Vu0_VfMove` (`f32` each; the direction also to the register at
    /// `0x00c8f970`); the look-ahead `n` = position + velocity (`+0x20`,
    /// the same way, left in `g_Vu0_VfMove`). Then for x, y, z in turn: a
    /// drop whose position and `n` are both below `cam - d` (strictly, so
    /// not for NaNs) re-spawns on the far side (`d + cam`), one with both
    /// above `cam + d` at `cam - d`: as in [`ppp_draw_rain_alloc`] the
    /// position `op_random(d) + cam` (w 1) but that axis, the direction
    /// `def +0x10..+0x18` (w 0) plus `KeVecAddRandom(def +0x20)` (also to
    /// `g_Vu0_VfMove`), the velocity that normalised and scaled by minus
    /// `op_random(def +0x34) + def +0x30` (`rcuiScaleVector_copy_00756600`,
    /// `junk` the three words it reads past its arguments: the caller's
    /// saved `ebx`, `edi`, `esi`). Later axes see the re-spawned position
    /// but the old `n`. Callees through `call`, `op_random`'s `ST0` through
    /// `fcall`.
    #[allow(clippy::too_many_arguments, clippy::many_single_char_names)]
    pub fn ppp_draw_rain_step<M: Mem>(
        mem: &mut M,
        obj: u32,
        def: u32,
        ctrl: u32,
        frame: u32,
        junk: [u32; 3],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) {
        use ffx_bridge::x87::{add, load, store, sub};
        use ffx_system::stub::ptr32;
        let c: u32 = mem.read(ctrl.wrapping_add(0xc));
        let off: u32 = mem.read(c.wrapping_add(4));
        let drops = ptr32(mem.read(obj.wrapping_add(off).wrapping_add(0xa4)));
        let inverse = frame.wrapping_sub(0x44);
        call(mem, g(mem, 0x006e_d450), &[inverse, g(mem, 0x0230_ff60)]);
        let vf_move = g(mem, 0x00c8_f980);
        let vf_dir = g(mem, 0x00c8_f970);
        let copy4 = |mem: &mut M, to: u32, from: u32| {
            for k in 0..4 {
                let v: u32 = mem.read(from.wrapping_add(4 * k));
                mem.write(to.wrapping_add(4 * k), v);
            }
        };
        let eye = inverse.wrapping_add(0x30);
        copy4(mem, vf_move, eye);
        let cam: [u32; 3] = mem.read(eye);
        let half: u32 = mem.read(def.wrapping_add(4));
        let lo = cam.map(|e| store(sub(load(e), load(half))));
        let hi = cam.map(|e| store(add(load(half), load(e))));
        // position += step (`g_Vu0_VfMove` = step + position, lane by lane)
        let step = |mem: &mut M, pos: u32, by: u32| {
            copy4(mem, vf_move, pos);
            copy4(mem, vf_dir, by);
            for k in 0..4 {
                let b: u32 = mem.read(by.wrapping_add(4 * k));
                let p: u32 = mem.read(vf_move + 4 * k);
                mem.write(vf_move + 4 * k, store(add(load(b), load(p))));
            }
        };
        let random = g(mem, 0x007e_6700);
        let mut i = 0u32;
        while i < mem.read::<u32>(def.wrapping_add(8)) {
            let pos = drops.wrapping_add(0x30 * i);
            let (dir, vel) = (pos.wrapping_add(0x10), pos.wrapping_add(0x20));
            step(mem, pos, dir);
            copy4(mem, pos, vf_move);
            step(mem, pos, vel);
            let n: [f32; 3] = mem.read(vf_move);
            for (axis, far) in [
                (0u32, false),
                (1, false),
                (2, false),
                (0, true),
                (1, true),
                (2, true),
            ] {
                let k = axis as usize;
                let at = pos.wrapping_add(4 * axis);
                let p: f32 = mem.read(at);
                let out = if far {
                    let h = f32::from_bits(hi[k]);
                    h < p && h < n[k]
                } else {
                    let l = f32::from_bits(lo[k]);
                    p < l && n[k] < l
                };
                if !out {
                    continue;
                }
                for e in 0..3u32 {
                    let spread: u32 = mem.read(def.wrapping_add(4));
                    let r = fcall(mem, random, &[store(load(spread))]);
                    let ce: u32 = mem.read(eye.wrapping_add(4 * e));
                    mem.write(pos.wrapping_add(4 * e), store(add(r, load(ce))));
                }
                mem.write(pos.wrapping_add(0xc), 1f32);
                let spread: u32 = mem.read(def.wrapping_add(4));
                let ce: u32 = mem.read(eye.wrapping_add(4 * axis));
                let v = if far {
                    store(sub(load(ce), load(spread)))
                } else {
                    store(add(load(spread), load(ce)))
                };
                mem.write(at, v);
                for e in 0..3 {
                    let v = store(load(mem.read(def.wrapping_add(0x10 + 4 * e))));
                    mem.write(dir.wrapping_add(4 * e), v);
                }
                mem.write(dir.wrapping_add(0xc), 0u32);
                call(mem, g(mem, 0x0076_6f10), &[dir, def.wrapping_add(0x20), 0]);
                copy4(mem, vf_move, dir);
                copy4(mem, vel, dir);
                call(mem, g(mem, 0x006e_d6f0), &[vel, vel]);
                let spread: u32 = mem.read(def.wrapping_add(0x34));
                let r = fcall(mem, random, &[store(load(spread))]);
                let speed = store(-add(r, load(mem.read(def.wrapping_add(0x30)))));
                let args = [vel, vel, store(load(speed)), junk[0], junk[1], junk[2]];
                call(mem, g(mem, 0x0075_6600), &args);
            }
            i += 1;
        }
    }

    /// `0x00755430` (`pppDrawRainLines`, ours; `(obj, def, ctrl)`, called by
    /// `pppDrawRainDraw`), `frame` the original's `ebp`: the drops of
    /// [`ppp_draw_rain_alloc`] (`ptr32(obj + [[ctrl +0xc] +4] + 0xa4)`, `def
    /// +8` of them) as line render data. The transform at `frame - 0x64` is
    /// `ScreenProjection` times `g_pppCameraMatrix` (`FFXVu0MulMatrix`); the
    /// line record at `frame - 0xbc` holds the lines (`user_malloc(n
    /// 0x38)`), their count and an identity matrix. The colour: the current
    /// part's `u16 +0x70..+0x76` times the work area's `i16 +0xa0..+0xa6`
    /// (`[[ctrl +0xc]]`), `>> 19`. Each drop: its position (`frame - 0x24`;
    /// also to `g_Vu0_VfMove`) and position + velocity (`+0x20`, lane by
    /// lane through `g_Vu0_VfMove`, the velocity also to `0x00c8f970`; at
    /// `frame - 0x14`) through `sceGsSwapDBuff(m, a, 2)` and
    /// `Ch_SysGetScreenMatrixPtr` of each; when all three give 0 a line
    /// (0x38 bytes): `a.xyz`, the colour `f32(ftol(c))` at `+0xc..+0x18`,
    /// `b.xyz` at `+0x1c`, the colour's first three again at `+0x28` and 0.
    /// Then `InitVFXRenderDataLine(inst, record, 0)` without render data,
    /// else `updateVFXRenderDataLine(inst, record)` and `graphicVFXUpdate`;
    /// last `user_free(cursor - n 0x38)` (the original's pointer: not the
    /// allocation when drops were left out). Callees through `call`;
    /// returns what `user_free` leaves in `eax`.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn ppp_draw_rain_lines<M: Mem>(
        mem: &mut M,
        obj: u32,
        def: u32,
        ctrl: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{add, ftol, load, round_trip, store};
        use ffx_system::stub::ptr32;
        let rd = |mem: &M, a: u32| mem.read::<u32>(a);
        let offs = rd(mem, ctrl.wrapping_add(0xc));
        let area = rd(mem, offs);
        let drops = ptr32(rd(
            mem,
            obj.wrapping_add(rd(mem, offs.wrapping_add(4)))
                .wrapping_add(0xa4),
        ));
        let at = |o: u32| frame.wrapping_sub(o);
        let ident = at(0x120);
        let mut unit = [0f32; 16];
        for k in 0..4 {
            unit[5 * k] = 1.0;
        }
        mem.write(ident, unit);
        let m = at(0x64);
        call(mem, g(mem, 0x0076_6ae0), &[m]);
        call(mem, g(mem, 0x006e_d5c0), &[m, m, g(mem, CAMERA_MATRIX)]);
        let rec = at(0xbc);
        mem.write(rec.wrapping_add(8), unit);
        let n = rd(mem, def.wrapping_add(8));
        let buf = call(mem, g(mem, 0x0068_7020), &[n.wrapping_mul(0x38)]);
        mem.write(rec, buf);
        mem.write(rec.wrapping_add(4), 0u32);
        let part = rd(mem, g(mem, 0x0230_ffe4));
        let colour: [u32; 4] = std::array::from_fn(|k| {
            let k = u32::try_from(k).unwrap_or(0);
            let p = u32::from(mem.read::<u16>(part.wrapping_add(0x70 + 2 * k)));
            let w = i32::from(mem.read::<i16>(obj.wrapping_add(area).wrapping_add(0xa0 + 2 * k)));
            let v = p.cast_signed().wrapping_mul(w) >> 19;
            store(f64::from(v))
        });
        let vf_move = g(mem, 0x00c8_f980);
        let vf_vel = g(mem, 0x00c8_f970);
        let (a, b) = (at(0x24), at(0x14));
        let copy4 = |mem: &mut M, to: u32, from: u32| {
            for k in 0..4 {
                let v: u32 = mem.read(from.wrapping_add(4 * k));
                mem.write(to.wrapping_add(4 * k), v);
            }
        };
        let mut cursor = buf;
        let mut i = 0u32;
        while i < rd(mem, def.wrapping_add(8)) {
            let drop = drops.wrapping_add(0x30 * i);
            copy4(mem, vf_move, drop);
            copy4(mem, a, drop);
            copy4(mem, vf_move, drop);
            copy4(mem, vf_vel, drop.wrapping_add(0x20));
            for k in 0..4 {
                let v: u32 = mem.read(drop.wrapping_add(0x20 + 4 * k));
                let p: u32 = mem.read(vf_move + 4 * k);
                mem.write(vf_move + 4 * k, store(add(load(v), load(p))));
            }
            copy4(mem, b, vf_move);
            let keep = call(mem, g(mem, 0x0076_6b80), &[m, a, 2]) == 0
                && call(mem, g(mem, 0x0062_fc90), &[a]) == 0
                && call(mem, g(mem, 0x0062_fc90), &[b]) == 0;
            if keep {
                let c = colour.map(|x| {
                    let n = ftol(mem, f64::from(f32::from_bits(x)));
                    store(f64::from(n.cast_signed()))
                });
                for k in 0..4u32 {
                    mem.write(cursor.wrapping_add(0xc + 4 * k), c[k as usize]);
                }
                for k in 0..3u32 {
                    let x: f32 = mem.read(a.wrapping_add(4 * k));
                    mem.write(cursor.wrapping_add(4 * k), round_trip(x));
                }
                for k in 0..3u32 {
                    mem.write(cursor.wrapping_add(0x28 + 4 * k), c[k as usize]);
                }
                mem.write(cursor.wrapping_add(0x34), 0u32);
                for k in 0..3u32 {
                    let x: f32 = mem.read(b.wrapping_add(4 * k));
                    mem.write(cursor.wrapping_add(0x1c + 4 * k), round_trip(x));
                }
                let count = rd(mem, rec.wrapping_add(4));
                mem.write(rec.wrapping_add(4), count.wrapping_add(1));
                cursor = cursor.wrapping_add(0x38);
            }
            i += 1;
        }
        let [lo, hi] = instance(mem, obj);
        if call(mem, g(mem, GET_RENDER_DATA), &[lo, hi]) == 0 {
            call(mem, g(mem, 0x0071_12d0), &[lo, hi, rec, 0]);
        } else {
            call(mem, g(mem, 0x0071_a0c0), &[lo, hi, rec]);
            call(mem, g(mem, VFX_UPDATE), &[lo, hi]);
        }
        let n = rd(mem, def.wrapping_add(8));
        call(
            mem,
            g(mem, 0x0068_7010),
            &[cursor.wrapping_sub(n.wrapping_mul(0x38))],
        )
    }

    /// Where a shape render keeps its locals: offsets below the
    /// original's aligned frame pointer (`ebp`).
    #[derive(Debug, Clone, Copy)]
    pub struct RenderFrame {
        /// The render request (0x20 bytes; `+0` and `+0xc` never written).
        pub req: u32,
        /// The inverse camera (times the inverse normalised world).
        pub c: u32,
        /// The scale and translation matrix (the normalised world before).
        pub s: u32,
        /// The particle's translation.
        pub u: u32,
        /// The camera times the world (the camera copy without the world).
        pub v2: u32,
        /// The camera copy.
        pub v: u32,
        /// The particle's scaled rotation.
        pub p: u32,
        /// The request's matrix (`+0x10`).
        pub r: u32,
        /// The request's second matrix (`+0x1c`, with the key's flag).
        pub q: u32,
    }

    /// A shape render's differences: its key layout, colour table,
    /// particle records and frame.
    #[derive(Debug, Clone, Copy)]
    pub struct ShapeRender {
        /// The key's options: flag bytes `+2..+5`, translation `+0x10`,
        /// scale `+0x20`, mode byte `+0x30`.
        pub key: u32,
        /// The work-area word holding the colour pair table.
        pub colours: u32,
        /// The particle record size.
        pub stride: u32,
        /// The record's scale vector (its rotation matrix `+0x30` after).
        pub scale: u32,
        /// Whether the inverse camera takes the inverse world in.
        pub world: bool,
        /// Whether `graphicUseOptimizedVFXShaderByVFXInstance` ends it.
        pub shader: bool,
        /// Whether the particles sit relative to the world position (the
        /// third work area's `+0xa0` less it, once) instead of at the
        /// area's `+0xa0`.
        pub dis_pos: bool,
        /// The locals.
        pub frame: RenderFrame,
    }

    const FIELD_FRAME: RenderFrame = RenderFrame {
        req: 0x280,
        c: 0xc8,
        s: 0x48,
        u: 0x108,
        v2: 0x1c8,
        v: 0x208,
        p: 0x148,
        r: 0x188,
        q: 0x88,
    };

    /// `0x00743280`, `pppDrawShapeFieldRevDraw`'s render.
    pub const FIELD_REV_RENDER: ShapeRender = ShapeRender {
        key: 0xc0,
        colours: 0x50,
        stride: 0xa0,
        scale: 0x30,
        world: true,
        shader: true,
        dis_pos: false,
        frame: FIELD_FRAME,
    };
    /// `0x007444a0`, `pppDrawShapeFieldSpdDraw`'s render.
    pub const FIELD_SPD_RENDER: ShapeRender = ShapeRender {
        key: 0xb0,
        ..FIELD_REV_RENDER
    };
    /// `0x007467b0`, `pppDrawShapeCameraDraw`'s render (its 0xa0-byte
    /// particles stepped like the field draws').
    pub const CAMERA_RENDER: ShapeRender = ShapeRender {
        key: 0xb0,
        colours: 0x4c,
        ..FIELD_REV_RENDER
    };
    /// `0x00745740`, `pppDrawShapeFieldGlobalDraw`'s render (0xb0-byte
    /// particles).
    pub const FIELD_GLOBAL_RENDER: ShapeRender = ShapeRender {
        key: 0xc0,
        colours: 0x4c,
        stride: 0xb0,
        scale: 0x40,
        world: false,
        shader: false,
        dis_pos: false,
        frame: RenderFrame {
            req: 0x240,
            c: 0x108,
            s: 0x88,
            u: 0x148,
            v2: 0x1c8,
            v: 0x1c8,
            p: 0xc8,
            r: 0x188,
            q: 0x48,
        },
    };
    /// `0x007479d0`, `pppDrawShapeCameraDisPosDraw`'s render ([`CAMERA_RENDER`]
    /// with positions relative to the world).
    pub const CAMERA_DIS_POS_RENDER: ShapeRender = ShapeRender {
        key: 0xb0,
        colours: 0x4c,
        dis_pos: true,
        ..FIELD_REV_RENDER
    };

    const COPY_MATRIX: u32 = 0x006e_d360;
    const INVERSE_MATRIX: u32 = 0x006e_d450;
    const NORMALIZE: u32 = 0x006e_d6f0;
    const MUL_MATRIX: u32 = 0x006e_d5c0;
    const CLEAR_SHP: u32 = 0x0071_3290;
    const SHAPE_FIELD_SIZE: u32 = 0x0071_2bd0;
    const DYN_GEO_BUFFER: u32 = 0x0063_fdb0;
    const UPDATE_SHAPE_FIELD: u32 = 0x0071_a4e0;
    const VFX_UPDATE: u32 = 0x0064_58a0;
    const USE_OPTIMIZED_SHADER: u32 = 0x0064_4b10;
    const WORLD_MATRIX: u32 = 0x0230_ffa0;
    const PACKET_POS: u32 = 0x0230_fd2c;
    const COLOR_SHIFT: u32 = 0x00c8_b500;

    /// `f(args)` at the image address `f`, through `call`.
    fn fun<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        f: u32,
        args: &[u32],
    ) -> u32 {
        call(mem, g(mem, f), args)
    }

    /// The shape renders of the shape field and camera draws (`0x00743280`,
    /// `0x007444a0`, `0x00745740`, `0x007467b0`, `0x007479d0`; cdecl
    /// `(obj, key, prog, area)`, `frame` the original's aligned `ebp`):
    /// unless the key's shape (`+4`) is `0xffff`, a render request on the
    /// stack (flags `+2`: 0x800 with key byte `+2`, `| 0x400` with `+3`;
    /// `+8` = 0; `+0x10` the particle matrix `R`; byte `+0x18` = 0; `+0x1c`
    /// the second matrix `Q` with key byte `+4`, else 0) and the view
    /// matrices: `C` = the inverse camera without translation, times
    /// (`world`) the inverse of the world with normalised rows and no
    /// translation; `S` = the key's scale (1, 1, 1 when all three are 0)
    /// and translation. With render data, it is cleared, sized for every
    /// live particle (record bit 0;
    /// `caculateVFXRenderDataShapeFieldBufferSize` with its frame `u16
    /// +6`), and the call ends when no dynamic-geometry buffer comes back.
    /// Then each live particle: its colour (pair `[+8]` of the colour table,
    /// through the EE: [`crate::mdl::bridge::ee_colour_at`]) at `+4`,
    /// `R = C x P x U x V2` (`P` = the record's scale on the diagonal
    /// times its rotation, `U` = a translation by its position `+0x10`
    /// plus the third work area's `+0xa0`), `Q` by the key's mode byte,
    /// `g_PppPacketPos` at `+0x14`, and `updateVFXRenderDataShapeField`
    /// with the frame's data. Last `graphicVFXUpdate` with render data and
    /// (`shader`) the optimised shader.
    #[allow(
        clippy::too_many_lines,
        clippy::too_many_arguments,
        clippy::many_single_char_names
    )]
    pub fn ppp_draw_shape_render<M: Mem>(
        mem: &mut M,
        v: &ShapeRender,
        obj: u32,
        key: u32,
        prog: u32,
        area: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{add, load, round_trip, store, sub};
        use ffx_system::stub::ptr32;
        let rd = |mem: &M, a: u32| mem.read::<u32>(a);
        let kb = |k: u32| key.wrapping_add(v.key + k);
        let shape = rd(mem, key.wrapping_add(4));
        if shape == 0xffff {
            return;
        }
        let f = v.frame;
        let at = |o: u32| frame.wrapping_sub(o);
        let (req, c, s, u, v2, cam, p, r, q) = (
            at(f.req),
            at(f.c),
            at(f.s),
            at(f.u),
            at(f.v2),
            at(f.v),
            at(f.p),
            at(f.r),
            at(f.q),
        );
        let env = rd(mem, rd(mem, g(mem, ENV_SLOT)));
        let slot = rd(mem, env.wrapping_add(0x24)).wrapping_add(shape.wrapping_mul(4));
        let mut flags = 0u16;
        if mem.read::<u8>(kb(2)) != 0 {
            flags = 0x800;
        }
        if mem.read::<u8>(kb(3)) != 0 {
            flags |= 0x400;
        }
        let second = mem.read::<u8>(kb(4)) != 0;
        mem.write(req.wrapping_add(2), flags);
        mem.write(req.wrapping_add(0x1c), if second { q } else { 0 });
        mem.write(req.wrapping_add(8), 0u32);
        mem.write(req.wrapping_add(0x18), 0u8);
        mem.write(req.wrapping_add(0x10), r);
        let set = ptr32(rd(mem, slot));
        let (camera, world) = (g(mem, CAMERA_MATRIX), g(mem, WORLD_MATRIX));
        let zero_translation = |mem: &mut M, m: u32| {
            for k in 0..3 {
                mem.write(m.wrapping_add(0x30 + 4 * k), 0u32);
            }
        };
        fun(mem, call, COPY_MATRIX, &[c, camera]);
        zero_translation(mem, c);
        fun(mem, call, INVERSE_MATRIX, &[c, c]);
        if v.world {
            // the inverse world in the scale matrix's place
            fun(mem, call, COPY_MATRIX, &[s, world]);
            for row in [s, s.wrapping_add(0x10), s.wrapping_add(0x20)] {
                fun(mem, call, NORMALIZE, &[row, row]);
            }
            zero_translation(mem, s);
            fun(mem, call, INVERSE_MATRIX, &[s, s]);
            fun(mem, call, MUL_MATRIX, &[c, s, c]);
        }
        fun(mem, call, UNIT_MATRIX, &[s]);
        let scale = [0x20u32, 0x24, 0x28].map(|o| f32::from_bits(rd(mem, kb(o))));
        let unit = scale.iter().all(|&x| x == 0.0);
        for (k, x) in (0u32..).zip(scale) {
            mem.write(
                s.wrapping_add(0x14 * k),
                if unit { 1.0 } else { round_trip(x) },
            );
        }
        for k in 0..3 {
            let t = round_trip(f32::from_bits(rd(mem, kb(0x10 + 4 * k))));
            mem.write(s.wrapping_add(0x30 + 4 * k), t);
        }
        fun(mem, call, UNIT_MATRIX, &[u]);
        fun(mem, call, COPY_MATRIX, &[cam, camera]);
        if v.world {
            fun(mem, call, MUL_MATRIX, &[v2, cam, world]);
        }
        let third = rd(mem, rd(mem, prog.wrapping_add(0xc)).wrapping_add(8));
        let base = obj.wrapping_add(third).wrapping_add(0xa0);
        let count = rd(mem, key.wrapping_add(8));
        let mut rec = ptr32(rd(mem, area.wrapping_add(0x40)));
        // the camera-relative draw: the base less the world position, once
        let offset = v.dis_pos.then(|| {
            [0u32, 4, 8].map(|k| {
                let w = load(rd(mem, world.wrapping_add(0x30 + k)));
                store(sub(load(rd(mem, base.wrapping_add(k))), w))
            })
        });
        let render_data = |mem: &mut M, call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32| {
            let [lo, hi] = instance(mem, obj);
            call(mem, g(mem, GET_RENDER_DATA), &[lo, hi])
        };
        let positive = count.cast_signed() > 0;
        if render_data(mem, call) != 0 {
            let [lo, hi] = instance(mem, obj);
            fun(mem, call, CLEAR_SHP, &[lo, hi]);
            if positive {
                for _ in 0..count {
                    if mem.read::<u8>(rec) & 1 != 0 && render_data(mem, call) != 0 {
                        let n = u32::from(mem.read::<u16>(rec.wrapping_add(6)));
                        let [lo, hi] = instance(mem, obj);
                        fun(mem, call, SHAPE_FIELD_SIZE, &[lo, hi, slot, n]);
                    }
                    rec = rec.wrapping_add(v.stride);
                }
            }
            let data = render_data(mem, call);
            let parts = rd(mem, data.wrapping_add(0x94));
            let (vertices, indices) = (
                rd(mem, parts.wrapping_add(4)),
                rd(mem, parts.wrapping_add(8)),
            );
            if fun(mem, call, DYN_GEO_BUFFER, &[data, 0, vertices, indices]) == 0 {
                return;
            }
            rec = ptr32(rd(mem, area.wrapping_add(0x40)));
            let [lo, hi] = instance(mem, obj);
            fun(mem, call, CLEAR_SHP, &[lo, hi]);
        }
        if positive {
            for _ in 0..count {
                if mem.read::<u8>(rec) & 1 != 0 {
                    let frame_no = u32::from(mem.read::<u16>(rec.wrapping_add(6)));
                    let entry = set.wrapping_add(0x10 + 8 * frame_no);
                    let data = set.wrapping_add(i32::from(mem.read::<i16>(entry)).cast_unsigned());
                    let table = ptr32(rd(mem, area.wrapping_add(v.colours)));
                    let pair = table.wrapping_add(rd(mem, rec.wrapping_add(8)).wrapping_mul(8));
                    let colour =
                        crate::mdl::bridge::ee_colour_at(mem, pair.wrapping_sub(0xa0), false, call);
                    let shift = g(mem, COLOR_SHIFT);
                    let sh = rd(mem, shift);
                    mem.write(req.wrapping_add(4).wrapping_add(sh), colour);
                    mem.write(shift, 0u32);
                    fun(mem, call, UNIT_MATRIX, &[p]);
                    for k in 0..3 {
                        let x =
                            round_trip(f32::from_bits(rd(mem, rec.wrapping_add(v.scale + 4 * k))));
                        mem.write(p.wrapping_add(0x14 * k), x);
                    }
                    fun(
                        mem,
                        call,
                        MUL_MATRIX,
                        &[p, rec.wrapping_add(v.scale + 0x30), p],
                    );
                    for k in 0..3 {
                        let pos = load(rd(mem, rec.wrapping_add(0x10 + 4 * k)));
                        let at = match offset {
                            Some(o) => o[k as usize],
                            None => rd(mem, base.wrapping_add(4 * k)),
                        };
                        mem.write(u.wrapping_add(0x30 + 4 * k), store(add(pos, load(at))));
                    }
                    fun(mem, call, MUL_MATRIX, &[r, c, p]);
                    fun(mem, call, MUL_MATRIX, &[r, u, r]);
                    fun(mem, call, MUL_MATRIX, &[r, v2, r]);
                    if second {
                        fun(mem, call, MUL_MATRIX, &[q, c, p]);
                        let steps: &[u32] = match mem.read::<u8>(kb(0x30)) {
                            0 => &[s, u, v2],
                            1 => &[u, world, camera, s],
                            2 => &[u, world, s, cam],
                            _ => &[],
                        };
                        for &m in steps {
                            fun(mem, call, MUL_MATRIX, &[q, m, q]);
                        }
                    }
                    let packet = rd(mem, g(mem, PACKET_POS));
                    mem.write(req.wrapping_add(0x14), packet);
                    if render_data(mem, call) != 0 {
                        let [lo, hi] = instance(mem, obj);
                        fun(mem, call, UPDATE_SHAPE_FIELD, &[lo, hi, data, req]);
                    }
                }
                rec = rec.wrapping_add(v.stride);
            }
        }
        if render_data(mem, call) != 0 {
            let [lo, hi] = instance(mem, obj);
            fun(mem, call, VFX_UPDATE, &[lo, hi]);
        }
        if v.shader {
            let [lo, hi] = instance(mem, obj);
            fun(mem, call, USE_OPTIMIZED_SHADER, &[lo, hi]);
        }
    }

    /// [`ppp_draw_shape_field_render`]'s locals (the screen check's matrix
    /// at `0xc8`).
    const FIELD_RENDER_FRAME: RenderFrame = RenderFrame {
        req: 0x2c0,
        c: 0x108,
        s: 0x48,
        u: 0x148,
        v2: 0x208,
        v: 0x248,
        p: 0x188,
        r: 0x1c8,
        q: 0x88,
    };

    /// `pppDrawShapeFieldRender_00741b90(obj, key, prog, area)`
    /// (`0x00741b90`, ours; `pppDrawShapeFieldDraw`'s render; `frame` the
    /// original's aligned `ebp`): [`ppp_draw_shape_render`] with
    /// [`FIELD_REV_RENDER`]'s layout (key options at `+0xc0`, colours
    /// `+0x50`, 0xa0-byte particles) and these differences:
    /// - whether there is render data is asked once, at the start;
    /// - the request's second matrix is 0 on map 0xdb with YIQ mode 0x12
    ///   and on map 0x16 with mode 2 (it is still computed);
    /// - on level 0x1ca (`MagicFile_GetMagicId` is called and ignored) the
    ///   draw ends unless the camera × world's position through
    ///   `g_pppScreenMatrix` (`frame - 0xc8`), divided by its `w`, has `x`
    ///   in (1604.7, 2491.3), `y` in (1756.8, 2339.2) and `z` in (0,
    ///   4194304) (NaNs pass);
    /// - the sizing pass needs `pppShapeFieldRenderOn_00743030` too, and
    ///   does not ask for render data per particle;
    /// - on level 0xc04, map 0x77, YIQ mode 4 or 5, the particle's `z` gets
    ///   +6 (`f64`);
    /// - on the [`SHAPE_FIELD_PLAIN_LEVELS`] each particle `k` has its own render
    ///   data, instance `(i64) 0x32 k + (i64) g_pppCurrentProgram + (obj <<
    ///   32)`, unless `g_GraphicSkipFrame`: none yet gives
    ///   `InitVFXRenderDataShp(inst, shape, request, frame,
    ///   calculateVFXSP_AlphaFunction(data byte [+4] + 4))`, else
    ///   `updateVFXRenderDataShp(inst, shape, request, frame)` and
    ///   `graphicVFXUpdate(inst)` (the `Path3` variants for request flag 4,
    ///   which is never set); on those levels the end does nothing;
    /// - elsewhere the end's `graphicVFXUpdate` needs the render data, the
    ///   optimised shader does not.
    #[allow(
        clippy::too_many_lines,
        clippy::too_many_arguments,
        clippy::many_single_char_names
    )]
    pub fn ppp_draw_shape_field_render<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        prog: u32,
        area: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{add, div, load, round_trip, store};
        use ffx_system::stub::ptr32;
        let v = FIELD_REV_RENDER;
        let rd = |mem: &M, a: u32| mem.read::<u32>(a);
        let kb = |k: u32| key.wrapping_add(v.key + k);
        let level = |mem: &mut M, call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32| {
            call(mem, g(mem, NAME_HASH), &[])
        };
        let [lo, hi] = instance(mem, obj);
        let has_data = fun(mem, call, GET_RENDER_DATA, &[lo, hi]) != 0;
        let shape = rd(mem, key.wrapping_add(4));
        if shape == 0xffff {
            return;
        }
        let f = FIELD_RENDER_FRAME;
        let at = |o: u32| frame.wrapping_sub(o);
        let (req, c, s, u, v2, cam, p, r, q) = (
            at(f.req),
            at(f.c),
            at(f.s),
            at(f.u),
            at(f.v2),
            at(f.v),
            at(f.p),
            at(f.r),
            at(f.q),
        );
        let env = rd(mem, rd(mem, g(mem, ENV_SLOT)));
        let slot = rd(mem, env.wrapping_add(0x24)).wrapping_add(shape.wrapping_mul(4));
        let mut flags = 0u16;
        if mem.read::<u8>(kb(2)) != 0 {
            flags = 0x800;
        }
        if mem.read::<u8>(kb(3)) != 0 {
            flags |= 0x400;
        }
        mem.write(req.wrapping_add(2), flags);
        let second = mem.read::<u8>(kb(4)) != 0;
        let map = rd(mem, g(mem, 0x012f_bc40));
        let yiq = rd(mem, g(mem, 0x00c3_a504));
        let no_second = (map == 0xdb && yiq == 0x12) || (map == 0x16 && yiq == 2);
        mem.write(
            req.wrapping_add(0x1c),
            if second && !no_second { q } else { 0 },
        );
        mem.write(req.wrapping_add(8), 0u32);
        mem.write(req.wrapping_add(0x18), 0u8);
        mem.write(req.wrapping_add(0x10), r);
        let set = ptr32(rd(mem, slot));
        let (camera, world) = (g(mem, CAMERA_MATRIX), g(mem, WORLD_MATRIX));
        let zero_translation = |mem: &mut M, m: u32| {
            for k in 0..3 {
                mem.write(m.wrapping_add(0x30 + 4 * k), 0u32);
            }
        };
        fun(mem, call, COPY_MATRIX, &[c, camera]);
        zero_translation(mem, c);
        fun(mem, call, INVERSE_MATRIX, &[c, c]);
        fun(mem, call, COPY_MATRIX, &[s, world]);
        for row in [s, s.wrapping_add(0x10), s.wrapping_add(0x20)] {
            fun(mem, call, NORMALIZE, &[row, row]);
        }
        zero_translation(mem, s);
        fun(mem, call, INVERSE_MATRIX, &[s, s]);
        fun(mem, call, MUL_MATRIX, &[c, s, c]);
        fun(mem, call, UNIT_MATRIX, &[s]);
        let scale = [0x20u32, 0x24, 0x28].map(|o| f32::from_bits(rd(mem, kb(o))));
        let unit = scale.iter().all(|&x| x == 0.0);
        for (k, x) in (0u32..).zip(scale) {
            mem.write(
                s.wrapping_add(0x14 * k),
                if unit { 1.0 } else { round_trip(x) },
            );
        }
        for k in 0..3 {
            let t = round_trip(f32::from_bits(rd(mem, kb(0x10 + 4 * k))));
            mem.write(s.wrapping_add(0x30 + 4 * k), t);
        }
        fun(mem, call, UNIT_MATRIX, &[u]);
        fun(mem, call, COPY_MATRIX, &[cam, camera]);
        fun(mem, call, MUL_MATRIX, &[v2, cam, world]);
        let hash = level(mem, call);
        fun(mem, call, 0x009d_a3b0, &[]);
        if hash == 0x1ca {
            let screen = at(0xc8);
            fun(mem, call, MUL_MATRIX, &[screen, g(mem, 0x0230_fe60), v2]);
            let w = load(rd(mem, screen.wrapping_add(0x3c)));
            let mut xyz = [0f64; 3];
            for (k, e) in (0u32..).zip(xyz.iter_mut()) {
                let at = screen.wrapping_add(0x30 + 4 * k);
                let x = store(div(load(rd(mem, at)), w));
                mem.write(at, x);
                *e = f64::from(f32::from_bits(x));
            }
            // strict bounds that a NaN passes (not `<=`: the x87 flags)
            let inside = |x: f64, lo: f64, hi: f64| !(x >= hi || x <= lo);
            let (x, y, z) = (xyz[0], xyz[1], xyz[2]);
            if !(inside(x, 1_604.699_951_171_875, f64::from(2_491.3f32))
                && inside(y, 1_756.800_048_828_125, f64::from(2_339.2f32))
                && inside(z, 0.0, 4_194_304.0))
            {
                return;
            }
        }
        let third = rd(mem, rd(mem, prog.wrapping_add(0xc)).wrapping_add(8));
        let base = obj.wrapping_add(third).wrapping_add(0xa0);
        let count = rd(mem, key.wrapping_add(8));
        let positive = count.cast_signed() > 0;
        let mut rec = ptr32(rd(mem, area.wrapping_add(0x40)));
        let on = fun(mem, call, 0x0074_3030, &[]) != 0;
        if on && has_data {
            let [lo, hi] = instance(mem, obj);
            fun(mem, call, CLEAR_SHP, &[lo, hi]);
            if positive {
                for _ in 0..count {
                    if mem.read::<u8>(rec) & 1 != 0 {
                        let n = u32::from(mem.read::<u16>(rec.wrapping_add(6)));
                        let [lo, hi] = instance(mem, obj);
                        fun(mem, call, SHAPE_FIELD_SIZE, &[lo, hi, slot, n]);
                    }
                    rec = rec.wrapping_add(v.stride);
                }
            }
            let [lo, hi] = instance(mem, obj);
            let data = fun(mem, call, GET_RENDER_DATA, &[lo, hi]);
            let parts = rd(mem, data.wrapping_add(0x94));
            let (vertices, indices) = (
                rd(mem, parts.wrapping_add(4)),
                rd(mem, parts.wrapping_add(8)),
            );
            if fun(mem, call, DYN_GEO_BUFFER, &[data, 0, vertices, indices]) == 0 {
                return;
            }
            rec = ptr32(rd(mem, area.wrapping_add(0x40)));
            let [lo, hi] = instance(mem, obj);
            fun(mem, call, CLEAR_SHP, &[lo, hi]);
        }
        if positive {
            let mut nth = 0u32;
            for _ in 0..count {
                if mem.read::<u8>(rec) & 1 != 0 {
                    let frame_no = u32::from(mem.read::<u16>(rec.wrapping_add(6)));
                    let entry = set.wrapping_add(0x10 + 8 * frame_no);
                    let data = set.wrapping_add(i32::from(mem.read::<i16>(entry)).cast_unsigned());
                    let table = ptr32(rd(mem, area.wrapping_add(v.colours)));
                    let pair = table.wrapping_add(rd(mem, rec.wrapping_add(8)).wrapping_mul(8));
                    let colour =
                        crate::mdl::bridge::ee_colour_at(mem, pair.wrapping_sub(0xa0), false, call);
                    let shift = g(mem, COLOR_SHIFT);
                    let sh = rd(mem, shift);
                    mem.write(req.wrapping_add(4).wrapping_add(sh), colour);
                    mem.write(shift, 0u32);
                    fun(mem, call, UNIT_MATRIX, &[p]);
                    for k in 0..3 {
                        let x =
                            round_trip(f32::from_bits(rd(mem, rec.wrapping_add(v.scale + 4 * k))));
                        mem.write(p.wrapping_add(0x14 * k), x);
                    }
                    fun(
                        mem,
                        call,
                        MUL_MATRIX,
                        &[p, rec.wrapping_add(v.scale + 0x30), p],
                    );
                    for k in 0..3 {
                        let pos = load(rd(mem, rec.wrapping_add(0x10 + 4 * k)));
                        let at = rd(mem, base.wrapping_add(4 * k));
                        mem.write(u.wrapping_add(0x30 + 4 * k), store(add(pos, load(at))));
                    }
                    if level(mem, call) == 0xc04
                        && rd(mem, g(mem, 0x012f_bc40)) == 0x77
                        && matches!(rd(mem, g(mem, 0x00c3_a504)), 4 | 5)
                    {
                        let z = load(rd(mem, u.wrapping_add(0x38)));
                        mem.write(u.wrapping_add(0x38), store(add(z, 6.0)));
                    }
                    fun(mem, call, MUL_MATRIX, &[r, c, p]);
                    fun(mem, call, MUL_MATRIX, &[r, u, r]);
                    fun(mem, call, MUL_MATRIX, &[r, v2, r]);
                    if second {
                        fun(mem, call, MUL_MATRIX, &[q, c, p]);
                        let steps: &[u32] = match mem.read::<u8>(kb(0x30)) {
                            0 => &[s, u, v2],
                            1 => &[u, world, camera, s],
                            2 => &[u, world, s, cam],
                            _ => &[],
                        };
                        for &m in steps {
                            fun(mem, call, MUL_MATRIX, &[q, m, q]);
                        }
                    }
                    let packet = rd(mem, g(mem, PACKET_POS));
                    mem.write(req.wrapping_add(0x14), packet);
                    if SHAPE_FIELD_PLAIN_LEVELS.contains(&level(mem, call)) {
                        if rd(mem, g(mem, 0x012f_b7d0)) == 0 {
                            let cur = rd(mem, g(mem, CURRENT_PROGRAM));
                            let sum = i64::from(nth.cast_signed()) + i64::from(cur.cast_signed());
                            let inst = sum.cast_unsigned().wrapping_add(u64::from(obj) << 32);
                            #[allow(clippy::cast_possible_truncation)] // the halves
                            let (ilo, ihi) = (inst as u32, (inst >> 32) as u32);
                            let path3 = mem.read::<u16>(req.wrapping_add(2)) & 4 != 0;
                            if fun(mem, call, GET_RENDER_DATA, &[ilo, ihi]) == 0 {
                                let rel = rd(mem, data.wrapping_add(4));
                                let a = u32::from(
                                    mem.read::<u8>(data.wrapping_add(rel).wrapping_add(4)),
                                );
                                let alpha = fun(mem, call, 0x0071_2c30, &[a]);
                                let init_fn = if path3 { 0x0071_1030 } else { 0x0071_1460 };
                                fun(mem, call, init_fn, &[ilo, ihi, slot, req, frame_no, alpha]);
                            } else {
                                fun(mem, call, 0x0071_b8a0, &[ilo, ihi, slot, req, frame_no]);
                                let update = if path3 { 0x0064_5600 } else { VFX_UPDATE };
                                fun(mem, call, update, &[ilo, ihi]);
                            }
                        }
                    } else if has_data {
                        let [lo, hi] = instance(mem, obj);
                        fun(mem, call, UPDATE_SHAPE_FIELD, &[lo, hi, data, req]);
                    }
                }
                rec = rec.wrapping_add(v.stride);
                nth = nth.wrapping_add(0x32);
            }
        }
        if SHAPE_FIELD_PLAIN_LEVELS.contains(&level(mem, call)) {
            return;
        }
        let [lo, hi] = instance(mem, obj);
        if has_data {
            fun(mem, call, VFX_UPDATE, &[lo, hi]);
        }
        fun(mem, call, USE_OPTIMIZED_SHADER, &[lo, hi]);
    }

    /// `pppDrawFilterPacket_00756740(obj, tex, prog)` (`0x00756740`, name
    /// ours; called by `pppDrawFilterDraw`), `frame` the original's `ebp`:
    /// a full-screen textured sprite as a packet at `g_PppPacketPos` (the
    /// packet record `{start, cursor, 0x7fffffff}` at `frame - 0x58`, the
    /// texture's log2 sizes at `frame - 0x68`, `- 0x60`, `- 0x5c`): its
    /// colour from the work area `[[prog+0xc]]` through the EE
    /// ([`crate::mdl::bridge::ee_colour_at`]), `rcbgTexGetLog2Size(tex
    /// +0xc, tex +0x10, ...)`, `TEX0` from `tex +4`/`+8` and those sizes,
    /// the blend bytes `tex +0x24`/`+0x25`, the UVs from the second work
    /// area's `i32 +0xa4`/`+0xa8` (/ 256) and `tex +0x14`/`+0x18` over the
    /// size (`u32 +0xc`/`+0x10`), the instance, and corners -1/1 (-1.1/1.1
    /// in x when `g_pppFpWide`). `prcPacketDataClose` ends it; the
    /// position moves past it, and `op_func_final_set(0x00756d20, start,
    /// 0, 1)`.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn ppp_draw_filter_packet<M: Mem>(
        mem: &mut M,
        obj: u32,
        tex: u32,
        prog: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{add, div, load, mul, store};
        let rd = |mem: &M, a: u32| mem.read::<u32>(a);
        let offs = rd(mem, prog.wrapping_add(0xc));
        let (first, second) = (rd(mem, offs), rd(mem, offs.wrapping_add(4)));
        let packet = g(mem, PACKET_POS);
        let start = rd(mem, packet);
        let rec = frame.wrapping_sub(0x58);
        let q = start.wrapping_add(0x10);
        mem.write(rec, [start, q, 0x7fff_ffff]);
        let colour = crate::mdl::bridge::ee_colour_at(mem, obj.wrapping_add(first), false, call);
        let shift = g(mem, COLOR_SHIFT);
        mem.write(
            frame.wrapping_sub(0x70).wrapping_add(rd(mem, shift)),
            colour,
        );
        mem.write(shift, 0u32);
        let logs = [0x68u32, 0x60, 0x5c].map(|o| frame.wrapping_sub(o));
        let (w, h) = (
            rd(mem, tex.wrapping_add(0xc)),
            rd(mem, tex.wrapping_add(0x10)),
        );
        fun(mem, call, 0x0092_0050, &[w, h, logs[0], logs[1], logs[2]]);
        let w4 = |mem: &mut M, at: u32, words: &[u32]| {
            for (k, &v) in (0u32..).zip(words) {
                mem.write(q.wrapping_add(at + 4 * k), v);
            }
        };
        w4(mem, 0, &[6, 0x1000_0000, 0xe, 0]);
        // TEX0: tbp, tbw, psm, tw, th, tcc, tfx, from the texture and its sizes
        let sext = |mem: &M, a: u32| i64::from(rd(mem, a).cast_signed()).cast_unsigned();
        let mut tex0 = u64::from(rd(mem, tex.wrapping_add(8)));
        tex0 = tex0 << 7 | sext(mem, logs[1]);
        tex0 = tex0 << 4 | sext(mem, logs[0]);
        tex0 = tex0 << 12 | sext(mem, logs[2]);
        tex0 = (tex0 | 0x8000_0010_04c0) << 14 | u64::from(rd(mem, tex.wrapping_add(4)));
        mem.write(q.wrapping_add(0x10), tex0);
        let blend = [0x24u32, 0x25].map(|o| u32::from(mem.read::<u8>(tex.wrapping_add(o))));
        let colour = rd(mem, frame.wrapping_sub(0x70));
        w4(mem, 0x18, &[6, 0, blend[0], blend[1], 0x42, 0, colour, 0]);
        w4(
            mem,
            0x38,
            &[
                1,
                0,
                0,
                0,
                8,
                0,
                0x3_000f,
                0,
                0x47,
                0,
                0x3100_0108,
                1,
                0x4e,
                0,
                1,
                0x40ab_4000,
                0x5353,
                0,
            ],
        );
        let k = 0.003_906_25;
        let int = |mem: &M, o: u32| {
            let v = rd(mem, obj.wrapping_add(second).wrapping_add(o)).cast_signed();
            load(store(f64::from(v)))
        };
        let size = |mem: &M, o: u32| load(store(f64::from(rd(mem, tex.wrapping_add(o)))));
        let u0 = store(div(mul(int(mem, 0xa4), k), size(mem, 0xc)));
        let v0 = store(div(mul(int(mem, 0xa8), k), size(mem, 0x10)));
        let [lo, hi] = instance(mem, obj);
        let wide = crate::fp::bridge::WIDE.get(mem) != 0;
        let left = if wide { 0xbf8c_cccd } else { 0xbf80_0000 };
        w4(mem, 0x80, &[u0, v0, lo, hi, left, 0xbf80_0000, 0, 0]);
        let u1 = store(div(
            add(mul(int(mem, 0xa4), k), size(mem, 0x14)),
            size(mem, 0xc),
        ));
        let v1 = store(div(
            add(mul(k, int(mem, 0xa8)), size(mem, 0x18)),
            size(mem, 0x10),
        ));
        w4(mem, 0xa0, &[u1, v1]);
        let right = if wide { 0x3f8c_cccd } else { 0x3f80_0000 };
        w4(
            mem,
            0xb0,
            &[
                right,
                0x3f80_0000,
                0,
                0,
                1,
                0x1000_0000,
                0xe,
                0,
                0x3100_0108,
                0,
                0x4e,
                0,
            ],
        );
        mem.write(rec.wrapping_add(4), q.wrapping_add(0xe0));
        fun(mem, call, 0x0076_6b90, &[rec]);
        let end = rd(mem, rec.wrapping_add(4)).wrapping_add(0x10);
        mem.write(rec.wrapping_add(4), end);
        mem.write(packet, end);
        let start = rd(mem, rec);
        let draw = g(mem, 0x0075_6d20);
        fun(mem, call, 0x007e_b410, &[draw, start, 0, 1]);
    }

    /// `pppKeMdlBmpDraw_Body_00748b00(obj)` (`0x00748b00`; `pppKeMdlBmpDraw`):
    /// the model's triangles as GS packets in a 0x1a0-byte scratch block
    /// (`op_spr_alloc`; fixed tags and registers, then per triangle the
    /// three positions, their texture `st` from the second set, the colour
    /// bytes as floats, `u16` texture words, three floats) each queued with
    /// `op_ot_add([obj+8], 0x20, block[+0x118] / 0x3000, 0)`, `[obj+8]`
    /// moving 0x200 a triangle. The model (`[obj+0xc]`) and texture
    /// (`[obj+0x10]`) records come in pairs: kind 3 with kind 2 (else it
    /// stops without saving the position), `i16 +2` triangles of 0x28 and
    /// 0x20 bytes after a 0x10-byte head (`TEX0` from `+8`, sign-extended;
    /// the block's `+0x110`/`+0x114` = `1 <<` its width and height bits),
    /// until a model record of kind 0xff. Triangle `[obj+0x24]` (counted
    /// down from the count) prints its corners unless it was printed last
    /// time (`0x00d4b468`, then set).
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn ppp_ke_mdl_bmp_draw_body<M: Mem>(
        mem: &mut M,
        obj: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::round_trip;
        let rd = |mem: &M, a: u32| mem.read::<u32>(a);
        let model = rd(mem, obj.wrapping_add(0xc));
        let mut a = rd(mem, model.wrapping_add(4)).wrapping_add(model);
        let texs = rd(mem, obj.wrapping_add(0x10));
        let mut b = rd(mem, texs.wrapping_add(4)).wrapping_add(texs);
        let verts = rd(mem, model.wrapping_add(8)).wrapping_add(model);
        let sts = rd(mem, model.wrapping_add(0xc)).wrapping_add(model);
        let mut ot = rd(mem, obj.wrapping_add(8));
        let q = fun(mem, call, 0x007e_6a00, &[0x1a0]);
        let w = |mem: &mut M, o: u32, v: u32| mem.write(q.wrapping_add(o), v);
        for (o, v) in [
            (0u32, 1u32),
            (4, 0xe02d_c000),
            (8, 0x412e_eeee),
            (0xc, 0x0041_2412),
            (0x18, 6),
            (0x1c, 0),
            (0x20, 0x5_3001),
            (0x24, 0),
            (0x28, 0x47),
            (0x2c, 0),
            (0x30, 0x64),
            (0x34, 0x80),
            (0x38, 0x42),
            (0x3c, 0),
            (0x40, 0x61),
            (0x44, 0),
            (0x48, 0x14),
            (0x4c, 0),
            (0x50, 0),
            (0x54, 0),
            (0x58, 8),
            (0x5c, 0),
            (0x60, 0x68),
            (0x64, 0x80),
            (0x68, 0x42),
            (0x6c, 0),
            (0x78, 6),
            (0x7c, 0),
            (0x80, 1),
            (0x84, 0xb029_c000),
            (0x88, 0x4124_12ee),
            (0x8c, 0x412),
            (0x90, 0x8001),
            (0x94, 0xb029_c000),
            (0x98, 0x4124_12ee),
            (0x9c, 0x412),
            (0xa0, 0x62),
            (0xa4, 0x80),
            (0xa8, 0x42),
            (0xac, 0),
            (0x190, 0x8001),
            (0x194, 0x4_c000),
            (0x198, 0x4141_4141),
            (0x19c, 0x4141_4141),
        ] {
            w(mem, o, v);
        }
        let printed = g(mem, 0x00d4_b468);
        let wanted = |mem: &M| rd(mem, obj.wrapping_add(0x24));
        loop {
            let count = mem.read::<i16>(a.wrapping_add(2));
            if mem.read::<u8>(a.wrapping_add(1)) != 3 || mem.read::<u8>(b.wrapping_add(1)) != 2 {
                return fun(mem, call, 0x007e_6a60, &[0x1a0]);
            }
            let sext = |v: u32| v.cast_signed().wrapping_shr(31).cast_unsigned();
            let t0 = rd(mem, a.wrapping_add(8));
            w(mem, 0x70, t0);
            w(mem, 0x74, sext(t0));
            let t1 = rd(mem, b.wrapping_add(8));
            w(mem, 0x10, t1);
            w(mem, 0x14, sext(t1));
            let n = i32::from(count);
            let next_a = a
                .wrapping_add(0x10)
                .wrapping_add(n.cast_unsigned().wrapping_mul(0x28));
            let next_b = b.wrapping_add(0x10).wrapping_add(n.cast_unsigned() << 5);
            let tw = 1u32 << (rd(mem, q.wrapping_add(0x70)) >> 26 & 0xf);
            w(mem, 0x110, tw);
            let th = 1u32 << (rd(mem, q.wrapping_add(0x74)) & 0xf);
            w(mem, 0x114, th);
            let (mut tri, mut st, mut k, mut left) =
                (a.wrapping_add(0x10), b.wrapping_add(0x10), n, count);
            while left > 0 {
                let idx = |mem: &M, o: u32| u32::from(mem.read::<u16>(tri.wrapping_add(o)));
                let corner = |mem: &mut M, base: u32, i: u32, to: u32| {
                    for c in 0..3u32 {
                        let v = i32::from(mem.read::<i16>(base.wrapping_add(i * 6 + 2 * c)));
                        mem.write(q.wrapping_add(to + 4 * c), v);
                    }
                };
                for (o, to) in [(0xcu32, 0xb0u32), (0xe, 0xc0), (0x10, 0xd0)] {
                    let i = idx(mem, o);
                    corner(mem, verts, i, to);
                }
                let want = wanted(mem);
                if k == want.cast_signed() && rd(mem, printed) != want {
                    fun(mem, call, 0x0062_f500, &[g(mem, 0x00b5_15e0)]);
                    for (fmt, at) in [
                        (0x00b5_15e8u32, 0xb0u32),
                        (0x00b5_15f8, 0xc0),
                        (0x00b5_1608, 0xd0),
                    ] {
                        let p: [u32; 3] = mem.read(q.wrapping_add(at));
                        fun(mem, call, 0x0062_f500, &[g(mem, fmt), p[0], p[1], p[2]]);
                    }
                }
                for (o, to) in [(0x20u32, 0x160u32), (0x22, 0x170), (0x24, 0x180)] {
                    let i = idx(mem, o);
                    corner(mem, sts, i, to);
                }
                for c in 0..12u32 {
                    let byte = mem.read::<u8>(tri.wrapping_add(c));
                    mem.write(q.wrapping_add(0xe0 + 4 * c), f32::from(byte));
                }
                for (o, to) in [
                    (0x14u32, 0x120u32),
                    (0x16, 0x124),
                    (0x18, 0x130),
                    (0x1a, 0x134),
                    (0x1c, 0x140),
                    (0x1e, 0x144),
                ] {
                    let v = idx(mem, o);
                    w(mem, to, v);
                }
                for c in 0..3u32 {
                    let f = round_trip(mem.read::<f32>(st.wrapping_add(4 * c)));
                    mem.write(q.wrapping_add(0x150 + 4 * c), f);
                }
                let depth = rd(mem, q.wrapping_add(0x118)).cast_signed() / 0x3000;
                fun(
                    mem,
                    call,
                    0x007e_c420,
                    &[ot, 0x20, depth.cast_unsigned(), 0],
                );
                ot = ot.wrapping_add(0x200);
                st = st.wrapping_add(0x20);
                k = k.wrapping_sub(1);
                left = left.wrapping_sub(1);
                tri = tri.wrapping_add(0x28);
            }
            a = next_a;
            b = next_b;
            if mem.read::<u8>(a.wrapping_add(1)) == 0xff {
                break;
            }
        }
        let want = wanted(mem);
        if rd(mem, printed) != want {
            mem.write(printed, want);
        }
        mem.write(obj.wrapping_add(8), ot);
        fun(mem, call, 0x007e_6a60, &[0x1a0])
    }

    /// `pppDrawRainDraw(obj, def, ctrl)` (`0x007562c0`, the `pppDrawRain`
    /// program's draw): with its work area `w = obj + [[ctrl +0xc] +4] +
    /// 0xa0` (which the original also leaves in its `ctrl` argument slot,
    /// written to `slot` here) still null (`is_ptr32_null(w[0])`),
    /// `pppDrawRainAlloc(def, w)`, returning 0 when that fails; then
    /// `pppDrawRainStep(obj, def, ctrl)` and `pppDrawRainLines(obj, def,
    /// ctrl)`, whose `eax` is returned. Callees through `call`.
    pub fn ppp_draw_rain_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        def: u32,
        ctrl: u32,
        slot: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let k: u32 = mem.read(ctrl.wrapping_add(0xc));
        let w = mem
            .read::<u32>(k.wrapping_add(4))
            .wrapping_add(0xa0)
            .wrapping_add(obj);
        mem.write(slot, w);
        let head: u32 = mem.read(w);
        if fun(mem, call, 0x00a4_46a0, &[head]) != 0 && fun(mem, call, 0x0075_6320, &[def, w]) == 0
        {
            return 0;
        }
        fun(mem, call, 0x0075_5a00, &[obj, def, ctrl]);
        fun(mem, call, 0x0075_5430, &[obj, def, ctrl])
    }

    /// `pppDrawFilterDraw(obj, rec, prog)` (`0x00757280`, the
    /// `pppDrawFilter` program's draw): its work area `w = obj + [[prog
    /// +0xc] +4] + 0xa0` gets `set_ptr32(rec)` and two zero scroll words
    /// while `is_ptr32_null(w[0])`; the scrolls `w[1]`, `w[2]` step by `rec
    /// +0x1c`, `+0x20` and wrap into `[0, 256 rec +0xc)` and `[0, 256 rec
    /// +0x10)` (one add below 0, one subtract at or past the end, unsigned
    /// compares), then `pppDrawFilterPacket(obj, rec, prog)`. Callees
    /// through `call`; returns the packet's `eax`.
    pub fn ppp_draw_filter_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        rec: u32,
        prog: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let work = |mem: &M| {
            let k: u32 = mem.read(prog.wrapping_add(0xc));
            obj.wrapping_add(0xa0)
                .wrapping_add(mem.read::<u32>(k.wrapping_add(4)))
        };
        let w = work(mem);
        let head: u32 = mem.read(w);
        if fun(mem, call, 0x00a4_46a0, &[head]) != 0 {
            let p = fun(mem, call, 0x00a4_4700, &[rec]);
            mem.write(w, p);
            mem.write(w.wrapping_add(8), 0u32);
            mem.write(w.wrapping_add(4), 0u32);
        }
        let w = work(mem);
        for (at, step) in [(4u32, 0x1cu32), (8, 0x20)] {
            let s = mem
                .read::<u32>(w.wrapping_add(at))
                .wrapping_add(mem.read(rec.wrapping_add(step)));
            mem.write(w.wrapping_add(at), s);
        }
        let ends = [0xcu32, 0x10].map(|o| mem.read::<u32>(rec.wrapping_add(o)) << 8);
        for (at, end) in [4u32, 8].into_iter().zip(ends) {
            let s: u32 = mem.read(w.wrapping_add(at));
            if s.cast_signed() < 0 {
                mem.write(w.wrapping_add(at), s.wrapping_add(end));
            }
            let s: u32 = mem.read(w.wrapping_add(at));
            if s >= end {
                mem.write(w.wrapping_add(at), s.wrapping_sub(end));
            }
        }
        fun(mem, call, 0x0075_6740, &[obj, rec, prog])
    }

    /// A shape field or camera draw's differences: its callees, work-area
    /// layout and particle records.
    #[derive(Debug, Clone, Copy)]
    pub struct ShapeDraw {
        /// The buffer set-up `(obj, key, prog)`.
        pub alloc: u32,
        /// The emitter `(obj, key, prog)`.
        pub emit: u32,
        /// The render `(obj, key, prog, area)`.
        pub render: u32,
        /// The particle step `(key, area)` (`(obj, key, area)` with
        /// `field`).
        pub step: u32,
        /// The work-area word set once the frame table is filled.
        pub flag: u32,
        /// The work-area word holding the frame table's length.
        pub limit: u32,
        /// The work-area word holding the frame table (four `u16`s a
        /// frame).
        pub table: u32,
        /// The work-area word holding a second table of the key's vector
        /// `+0x40..+0x48` (and 0.0), 16 bytes a frame.
        pub vectors: Option<u32>,
        /// The particle record size.
        pub record: u32,
        /// `pppDrawShapeFieldDraw`: the render data only while
        /// `pppShapeFieldRenderOn_00743030()`, its alpha function with
        /// 0x800 on map 0xdb in YIQ mode 0x12, no render on a skipped
        /// frame, the step taking `obj` too.
        pub field: bool,
        /// The camera draws: nothing after the frame table while the ppp
        /// steps are stopped, the step unconditional otherwise.
        pub camera: bool,
    }

    /// `pppDrawShapeFieldDraw` (`0x00742cc0`).
    pub const FIELD_DRAW: ShapeDraw = ShapeDraw {
        alloc: 0x0074_2f20,
        emit: 0x0074_2750,
        render: 0x0074_1b90,
        step: 0x0074_2980,
        flag: 0x58,
        limit: 0x54,
        table: 0x50,
        vectors: None,
        record: 0xa0,
        field: true,
        camera: false,
    };
    /// `pppDrawShapeFieldRevDraw` (`0x00744040`).
    pub const FIELD_REV_DRAW: ShapeDraw = ShapeDraw {
        alloc: 0x0074_4260,
        emit: 0x0074_3b60,
        render: 0x0074_3280,
        step: 0x0074_3da0,
        field: false,
        ..FIELD_DRAW
    };
    /// `pppDrawShapeFieldSpdDraw` (`0x00745210`).
    pub const FIELD_SPD_DRAW: ShapeDraw = ShapeDraw {
        alloc: 0x0074_54a0,
        emit: 0x0074_4d80,
        render: 0x0074_44a0,
        step: 0x0074_4f60,
        flag: 0x5c,
        limit: 0x58,
        vectors: Some(0x54),
        ..FIELD_REV_DRAW
    };
    /// `pppDrawShapeFieldGlobalDraw` (`0x00746320`).
    pub const FIELD_GLOBAL_DRAW: ShapeDraw = ShapeDraw {
        alloc: 0x0074_6540,
        emit: 0x0074_5f70,
        render: 0x0074_5740,
        step: 0x0074_60b0,
        flag: 0x54,
        limit: 0x50,
        table: 0x4c,
        record: 0xb0,
        ..FIELD_REV_DRAW
    };
    /// `pppDrawShapeCameraDraw` (`0x00747540`).
    pub const CAMERA_DRAW: ShapeDraw = ShapeDraw {
        alloc: 0x0074_7760,
        emit: 0x0074_7090,
        render: 0x0074_67b0,
        step: 0x0074_72b0,
        record: 0xa0,
        camera: true,
        ..FIELD_GLOBAL_DRAW
    };
    /// `pppDrawShapeCameraDisPosDraw` (`0x00748790`).
    pub const CAMERA_DIS_POS_DRAW: ShapeDraw = ShapeDraw {
        alloc: 0x0074_89b0,
        emit: 0x0074_82e0,
        render: 0x0074_79d0,
        step: 0x0074_8500,
        ..CAMERA_DRAW
    };

    /// The shape field and camera draws (`pppDrawShapeFieldDraw`,
    /// `...FieldRevDraw`, `...FieldSpdDraw`, `...FieldGlobalDraw`,
    /// `...CameraDraw`, `...CameraDisPosDraw`; cdecl `(obj, key, prog)`,
    /// the ppp program records' draws). The work area `area = obj + 0xa0 +
    /// [[prog +0xc]]` (which the originals also leave in an argument slot;
    /// their only caller, `pppRunPartStd`, pushes its words afresh), the
    /// shape `[[obj +8]]`. Nothing for the key's texture `+4` 0xffff or a
    /// null environment entry (`[[g_pppEnvSlot] +0x24]` at it). Without
    /// render data for the instance (`g_pppCurrentProgram`, `obj` plus its
    /// sign), `InitVFXRenderDataShapeField(instance, &entry, [key +8],
    /// calculateVFXSP_AlphaFunction(byte texture tag +4))`. A null
    /// particle buffer (`area +0x40`) gets `alloc` (nothing more when that
    /// fails). While the table is not yet filled and frame `t` = `[obj
    /// +0xc] / 4096` is below its length, the four `u16`s at `obj + 0xa0 +
    /// [[prog +0xc] +4]` go to frame `t` of the table (and the key's vector
    /// to the second one). From the shape's last frame (`[shape +0x14]` -
    /// 0x1000) on, a live particle (bit 0 of a record among `[key +8]`)
    /// holds the frame back by 0x1000, and the table counts as filled.
    /// Then `emit` (ppp steps running, the part's `+0x1d` clear), the
    /// render and the step (steps running). Callees through `call`.
    #[allow(clippy::too_many_lines)]
    pub fn ppp_draw_shape_draw<M: Mem>(
        mem: &mut M,
        v: &ShapeDraw,
        obj: u32,
        key: u32,
        prog: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{load, store};
        use ffx_system::stub::ptr32;
        let rd = |mem: &M, a: u32| mem.read::<u32>(a);
        let offsets = rd(mem, prog.wrapping_add(0xc));
        let area = obj.wrapping_add(0xa0).wrapping_add(rd(mem, offsets));
        let shape = ptr32(rd(mem, ptr32(rd(mem, obj.wrapping_add(8)))));
        let tex_no = rd(mem, key.wrapping_add(4));
        if tex_no == 0xffff {
            return;
        }
        let entries = |mem: &M| rd(mem, rd(mem, rd(mem, g(mem, ENV_SLOT))).wrapping_add(0x24));
        let tex = ptr32(rd(mem, entries(mem).wrapping_add(tex_no.wrapping_mul(4))));
        if tex == 0 {
            return;
        }
        let instance = |mem: &M| {
            let lo = rd(mem, g(mem, CURRENT_PROGRAM));
            // `cdq; add eax, 0; adc edx, obj`
            (
                lo,
                obj.wrapping_add((lo.cast_signed() >> 31).cast_unsigned()),
            )
        };
        if !v.field || fun(mem, call, 0x0074_3030, &[]) != 0 {
            let (lo, hi) = instance(mem);
            if fun(mem, call, 0x0064_53d0, &[lo, hi]) == 0 {
                let tag = tex.wrapping_add(
                    i32::from(mem.read::<i16>(tex.wrapping_add(0x10))).cast_unsigned(),
                );
                let a = mem.read::<u8>(
                    tag.wrapping_add(rd(mem, tag.wrapping_add(4)))
                        .wrapping_add(4),
                );
                let mut alpha = fun(mem, call, 0x0071_2c30, &[u32::from(a)]);
                if v.field
                    && rd(mem, g(mem, 0x012f_bc40)) == 0xdb
                    && rd(mem, g(mem, 0x00c3_a504)) == 0x12
                {
                    alpha |= 0x800;
                }
                let no = rd(mem, key.wrapping_add(4));
                let entry = entries(mem).wrapping_add(no.wrapping_mul(4));
                let (lo, hi) = instance(mem);
                let count = rd(mem, key.wrapping_add(8));
                fun(mem, call, 0x0071_13d0, &[lo, hi, entry, count, alpha]);
            }
        }
        let parts = rd(mem, area.wrapping_add(0x40));
        if fun(mem, call, 0x00a4_46a0, &[parts]) != 0
            && fun(mem, call, v.alloc, &[obj, key, prog]) == 0
        {
            return;
        }
        // `cdq; and edx, 0xfff; add; sar 12`: rounded toward zero
        let t = (rd(mem, obj.wrapping_add(0xc)).cast_signed() / 4096).cast_unsigned();
        if rd(mem, area.wrapping_add(v.flag)) == 0 && t < rd(mem, area.wrapping_add(v.limit)) {
            let offsets = rd(mem, prog.wrapping_add(0xc));
            let src = obj
                .wrapping_add(rd(mem, offsets.wrapping_add(4)))
                .wrapping_add(0xa0);
            for k in 0..4u32 {
                let h: u16 = mem.read(src.wrapping_add(2 * k));
                let table = ptr32(rd(mem, area.wrapping_add(v.table)));
                mem.write(table.wrapping_add(t.wrapping_mul(8)).wrapping_add(2 * k), h);
            }
            if let Some(at) = v.vectors {
                let row = t.wrapping_mul(16);
                for k in 0..3u32 {
                    let x = store(load(rd(mem, key.wrapping_add(0x40 + 4 * k))));
                    let table = ptr32(rd(mem, area.wrapping_add(at)));
                    mem.write(table.wrapping_add(row).wrapping_add(4 * k), x);
                }
                let table = ptr32(rd(mem, area.wrapping_add(at)));
                mem.write(table.wrapping_add(row).wrapping_add(0xc), 0u32);
            }
        }
        let last = rd(mem, shape.wrapping_add(0x14)).wrapping_sub(0x1000);
        if rd(mem, obj.wrapping_add(0xc)).cast_signed() >= last.cast_signed() {
            let n = rd(mem, key.wrapping_add(8)).cast_signed();
            let first = ptr32(rd(mem, area.wrapping_add(0x40)));
            let live = (0..n).any(|i| {
                let at = first.wrapping_add(i.cast_unsigned().wrapping_mul(v.record));
                mem.read::<u8>(at) & 1 != 0
            });
            if live {
                let f = rd(mem, obj.wrapping_add(0xc));
                mem.write(obj.wrapping_add(0xc), f.wrapping_sub(0x1000));
            }
            mem.write(area.wrapping_add(v.flag), 1u32);
        }
        let running = |mem: &M| rd(mem, g(mem, STOP)) == 0;
        if v.camera && !running(mem) {
            return;
        }
        if running(mem) {
            let part = crate::prog::bridge::CURRENT_PART.get(mem);
            if mem.read::<u8>(part.wrapping_add(0x1d)) == 0 {
                fun(mem, call, v.emit, &[obj, key, prog]);
            }
        }
        if !v.field || rd(mem, g(mem, 0x012f_b7d0)) == 0 {
            fun(mem, call, v.render, &[obj, key, prog, area]);
        }
        // the camera draws step without a second look at the stop flag
        if v.camera || running(mem) {
            if v.field {
                fun(mem, call, v.step, &[obj, key, area]);
            } else {
                fun(mem, call, v.step, &[key, area]);
            }
        }
    }
}

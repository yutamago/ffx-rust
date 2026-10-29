//! M5 stage 1, list `phyre-render-b`, batch 7: methods that hand the
//! addresses of their own locals to callees (a colour or a parameter
//! vector to `memcpy` and to the D3D11 context, a matrix to the engine's
//! maths, an out word). Each bridge takes `ebp`, where the original's
//! frame pointer is, and keeps those locals at the original's offsets
//! below it; the shim passes the top of a `StackFrame`, the tests the
//! original's own `ebp` (`docs/porting.md`, "Frames callees see").

use ffx_bridge::x87;

/// `PArray_Data_004d45a0(array)`, `ShaderParams_FindByName(shader; name)`
/// (thiscall), `crt_memcpy_thunk` (cdecl).
pub const ARRAY_DATA: u32 = 0x004d_45a0;
pub const FIND_PARAM: u32 = 0x0056_cbf0;
pub const MEMCPY: u32 = 0x0094_9256;
/// The fog parameter's name (`"fgEff"`) and the tree successor the fog
/// effect walks with (`0x005751d0(tree; node)`, thiscall).
pub const FOG_PARAM_NAME: u32 = 0x00b4_ad78;
pub const TREE_NEXT: u32 = 0x0057_51d0;
/// `0x00593f90(array; i)` (thiscall): a UAV of the buffer's array.
pub const UAV_OF: u32 = 0x0059_3f90;
/// `Phyre_PPostProcessing_makeRenderTarget_00a06a30` (cdecl, seven
/// words), `Phyre_PRendering_PTexture2DBase_updateMaxLevel` (thiscall),
/// `Phyre_PPostProcessing_PMotionBlur_setTarget_00a17da0` (thiscall), the
/// two texture classes.
pub const MAKE_TARGET: u32 = 0x00a0_6a30;
pub const UPDATE_MAX_LEVEL: u32 = 0x004c_4250;
pub const BLUR_SET_TARGET: u32 = 0x00a1_7da0;
pub const TEXTURE_2D_CLASS: u32 = 0x00ca_3364;
pub const VELOCITY_CLASS: u32 = 0x00ca_310c;
/// `PMatrix4_FromMatrix3x4_00451080(out; m)`, `PMatrix4_Rotate3_004ca140(m;
/// out, v)` (thiscall), the light kind, and the constants of
/// `0x006a84d0`: a `double` -1.0 (times 0.0: -0.0) and `g_ChrClothFlip`
/// (the `float` -1.0).
pub const MATRIX_FROM_3X4: u32 = 0x0045_1080;
pub const MATRIX_ROTATE3: u32 = 0x004c_a140;
pub const LIGHT_KIND_DIRECTIONAL: u32 = 0x00c1_0828;
pub const MINUS_ONE_DOUBLE: u32 = 0x00b9_2350;
pub const MINUS_ONE_FLOAT: u32 = 0x00b9_2340;
/// The all-ones UAV counts `0x00593380` hands the context.
pub const UAV_COUNTS: u32 = 0x00b3_3a24;

/// A `float` through `fld dword; fstp dword` (a signalling NaN quieted).
fn quiet(bits: u32) -> u32 {
    x87::round_trip(f32::from_bits(bits)).to_bits()
}

pub mod bridge {
    //! The originals over game memory; `ebp` is where the original's
    //! frame pointer is.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    fn this_call(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    /// For each material of `arr` (`PArray_Data`), the named parameter's
    /// bytes from `src` when it fits (offset at least 4, within the
    /// buffer's size word); `before` runs once a parameter is found.
    fn copy_parameter<H: Host>(
        h: &mut H,
        arr: u32,
        name: u32,
        src: u32,
        mut before: impl FnMut(&mut H),
    ) {
        let count = h.read::<u32>(arr) & 0x7fff_ffff;
        for i in 0..count {
            let list = this_call(h, ARRAY_DATA, arr, &[]);
            let e: u32 = h.read(list.wrapping_add(4 * i));
            let buffer: u32 = h.read(e + 4);
            let material: u32 = h.read(e);
            let p = this_call(h, FIND_PARAM, material, &[name]);
            if p == 0 {
                continue;
            }
            before(h);
            let off = u32::from(h.read::<u16>(p + 8));
            let size = u32::from(h.read::<u16>(p + 0xa) & 0x1fff);
            if off.wrapping_add(size) > h.read::<u32>(buffer) || off < 4 {
                continue;
            }
            cdecl(h, MEMCPY, &[buffer.wrapping_add(off), src, size]);
        }
    }

    /// `Phyre_CharacterShaderManager_setShaderParameterVector_006dfa30(inst,
    /// name, x*, y*, z*, w*)` (cdecl): the four floats (through the x87)
    /// into the local vector at `ebp - 0x14` whenever a material has the
    /// parameter, and copied into its buffer as
    /// `Phyre_CharacterShaderManager_setShaderParameter` does.
    pub fn set_vector(h: &mut impl Host, ebp: u32, inst: u32, name: u32, v: [u32; 4]) {
        let arr: u32 = h.read(inst + 0x10);
        h.write(ebp - 4, arr);
        let local = ebp - 0x14;
        copy_parameter(h, arr, name, local, |h| {
            for (k, &p) in v.iter().enumerate() {
                let bits = quiet(h.read(p));
                h.write(local + 4 * k as u32, bits);
            }
        });
    }

    /// `Phyre_FogEffectManager_setChrFogEffect(fog; v)`: `v` (three times,
    /// through the x87) at `ebp - 0x1c` copied into the `"fgEff"`
    /// parameter of every material of every instance in the fog's tree
    /// (`+0xa8`), in the tree's order from the last node of its `+0xc`
    /// chain.
    pub fn set_chr_fog_effect(h: &mut impl Host, ebp: u32, fog: u32, v: u32) {
        let q = quiet(v);
        let local = ebp - 0x1c;
        h.write(local, [q, q, q]);
        let tree: u32 = h.read(fog + 0xa8);
        h.write(ebp - 0xc, tree);
        let first: u32 = h.read(tree + 0xc);
        let mut node = if first == tree || first == 0 {
            first
        } else {
            let mut cur = first;
            loop {
                let next: u32 = h.read(cur);
                if next == tree {
                    break cur;
                }
                cur = next;
            }
        };
        if node == tree {
            return;
        }
        let mut next = this_call(h, TREE_NEXT, tree, &[node]);
        let name = h.rebase(FOG_PARAM_NAME);
        while node != tree {
            let arr: u32 = h.read(node + 0xc);
            h.write(ebp - 8, arr);
            copy_parameter(h, arr, name, local, |_| {});
            node = next;
            if next != tree {
                next = this_call(h, TREE_NEXT, tree, &[next]);
            }
        }
    }

    /// `0x0058e510(state; buffer, value, i)`: 5 without a context, for `i`
    /// past the buffer's views (at least one) or without view `i`; else
    /// the context clears view `i` with `value` four times (method
    /// `+0xcc`, the values at `ebp - 0x14`); 0.
    pub fn clear_uav(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        buffer: u32,
        value: u32,
        i: u32,
    ) -> u32 {
        let ctx: u32 = h.read(this + 0x70);
        h.write(ebp - 0x18, ctx);
        if ctx == 0 {
            return 5;
        }
        let mut count = h.read::<u32>(buffer + 0x14) & 0x7fff_ffff;
        if count == 0 {
            count = 1;
        }
        if i >= count || this_call(h, UAV_OF, buffer, &[i]) == 0 {
            return 5;
        }
        let local = ebp - 0x14;
        h.write(local, [value; 4]);
        let vt: u32 = h.read(ctx);
        let uav = this_call(h, UAV_OF, buffer, &[i]);
        let f: u32 = h.read(vt + 0xcc);
        h.call(f, Conv::Stdcall, &[ctx, uav, local]);
        0
    }

    /// `0x00a17e00(blur; cluster, w, h)`: 0x16 when the blur has a target;
    /// a 2D render target made (its result word at `ebp - 4`, returned
    /// when the target fails or the result is not 0), its texture's
    /// `+0x14` bit 1 cleared and levels updated, a velocity target made
    /// (`+0xcd8`), then `setTarget(cluster, target)`'s result.
    pub fn blur_setup(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        cluster: u32,
        w: u32,
        hgt: u32,
    ) -> u32 {
        // the prologue's `push ecx` reserves the local with `this` in it
        h.write(ebp - 4, this);
        if h.read::<u32>(this + 0xd4c) != 0 {
            return 0x16;
        }
        let out = ebp - 4;
        h.write(out, 0u32);
        let class = h.rebase(TEXTURE_2D_CLASS);
        let target = cdecl(h, MAKE_TARGET, &[cluster, w, hgt, class, 0, 0, out]); // call width: 7
        let r: u32 = h.read(out);
        if target == 0 || r != 0 {
            return r;
        }
        let tex = if h.read::<u8>(target + 4) == 0 {
            h.read(target)
        } else {
            0
        };
        let flags: u32 = h.read(tex + 0x14);
        h.write(tex + 0x14, flags & !2);
        this_call(h, UPDATE_MAX_LEVEL, tex, &[]);
        let class = h.rebase(VELOCITY_CLASS);
        let velocity = cdecl(h, MAKE_TARGET, &[cluster, w, hgt, class, 0, 0, 0]); // call width: 7
        h.write(this + 0xcd8, velocity);
        this_call(h, BLUR_SET_TARGET, this, &[cluster, target])
    }

    /// `0x006a84d0(lighting; out, i)`: light 3 is the ambient direction
    /// (-0, -1, -0); a directional light `i` of a set-up manager gives
    /// (-0, -0, -1) rotated by its node's matrix (the 4x4 at `ebp -
    /// 0x50`, the vector at `ebp - 0x10`); anything else (0, 0, 1); `out`.
    pub fn light_direction(h: &mut impl Host, ebp: u32, this: u32, out: u32, i: u32) -> u32 {
        let c: f64 = h.read(h.rebase(MINUS_ONE_DOUBLE));
        let z = x87::store(x87::mul(0.0, c));
        let one: u32 = h.read(h.rebase(MINUS_ONE_FLOAT));
        let minus_one = quiet(one);
        if i == 3 {
            h.write(out, [z, minus_one, z]);
            return out;
        }
        if h.read::<u8>(this + 0x1cc) != 0 {
            let lights: u32 = h.read(this + 0xf0);
            let list: u32 = h.read(lights + 4);
            let e: u32 = h.read(list.wrapping_add(i.wrapping_mul(4)));
            let kind = h.rebase(LIGHT_KIND_DIRECTIONAL);
            if h.read::<u32>(e + 0x18) == kind {
                let node: u32 = h.read(e + 0x10);
                let m = ebp - 0x50;
                this_call(h, MATRIX_FROM_3X4, m, &[node]);
                let v = ebp - 0x10;
                h.write(v, [z, z, minus_one]);
                this_call(h, MATRIX_ROTATE3, m, &[out, v]);
                return out;
            }
        }
        h.write(out, [0, 0, 1f32.to_bits()]);
        out
    }

    /// `0x00593380(tables; renderer*, _)`: the shader resource tables
    /// bound on the context of `*renderer`: the UAV (`+0`, with `+8` and
    /// its count) to the compute stage, the stage's eight (one below
    /// feature level 0xb000) UAV slots cleared (zeros at `ebp - 0x20`),
    /// the UAV again into slot `[+0x18]` of the list `+0x27c` when that is
    /// not -1, then the shader `+0x4e0` (method `+0x5c`), the eight
    /// buffers at `+0x27c` (`+0x58`), the 0x80 views at `+0x1c` (`+0x7c`)
    /// and the 0x10 samplers at `+0x21c` (`+0x80`); `al` = 1.
    pub fn bind_tables(h: &mut impl Host, ebp: u32, this: u32, renderer: u32) -> u8 {
        let r: u32 = h.read(renderer);
        let uav: u32 = h.read(this);
        if uav != 0 {
            set_uav(h, r, this, uav);
        }
        let zeros = ebp - 0x20;
        h.write(zeros, [0u32; 8]);
        let n = if (h.read::<u32>(r + 0xb4) as i32) >= 0xb000 {
            8
        } else {
            1
        };
        let counts = h.rebase(UAV_COUNTS);
        com(h, r, 0x110, &[0, n, zeros, counts]);
        let slot: u32 = h.read(this + 0x18);
        if slot != 0xffff_ffff {
            let uav: u32 = h.read(this);
            if uav != 0 {
                set_uav(h, r, this, uav);
            }
            let slot: u32 = h.read(this + 0x18);
            let uav: u32 = h.read(this);
            h.write(
                this.wrapping_add(slot.wrapping_mul(4)).wrapping_add(0x27c),
                uav,
            );
        }
        let shader: u32 = h.read(this + 0x4e0);
        com(h, r, 0x5c, &[shader, 0, 0]);
        com(h, r, 0x58, &[0, 8, this + 0x27c]);
        com(h, r, 0x7c, &[0, 0x80, this + 0x1c]);
        com(h, r, 0x80, &[0, 0x10, this + 0x21c]);
        1
    }

    /// Method `+0xc0` of the renderer's context with the UAV.
    fn set_uav(h: &mut impl Host, r: u32, this: u32, uav: u32) {
        let count = h.read::<u32>(this + 4) & 0x7fff_ffff;
        let srv: u32 = h.read(this + 8);
        com(h, r, 0xc0, &[uav, 0, 0, srv, count, count]);
    }

    /// Context (`[r + 0x70]`) method `off` with `args` after the context.
    fn com(h: &mut impl Host, r: u32, off: u32, args: &[u32]) {
        let ctx: u32 = h.read(r + 0x70);
        let vt: u32 = h.read(ctx);
        let f: u32 = h.read(vt + off);
        let mut words = [0u32; 8];
        words[0] = ctx;
        words[1..=args.len()].copy_from_slice(args);
        h.call(f, Conv::Stdcall, &words[..=args.len()]); // call width: 7
    }
}

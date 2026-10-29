//! M5 stage 1, list `phyre-render-b`: the small methods of the renderer,
//! its render-interface state setters and thread-pool jobs, the
//! character-lighting and fog managers' teardown, and the bitmap font's
//! glyph helpers. Each port follows its original's machine code
//! (`objdump -d FFX.exe`); `eax` is what the original leaves where its
//! callers (often through a vtable) may read it.

use ffx_bridge::x87;

/// `Phyre_PhyreFree(p)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `operator_delete` (`0x00630390`) and `operator_delete_00630540`
/// (`delete[]`), cdecl.
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
pub const OPERATOR_DELETE_ARRAY: u32 = 0x0063_0540;
/// The empty function `0x006142f0` tail-jumps to.
pub const EMPTY: u32 = 0x0058_52f0;
/// `KERNEL32!CloseHandle` of a handle at word 0 (`0x00430250`,
/// `thiscall`, eax 0).
pub const CLOSE_HANDLE_AT: u32 = 0x0043_0250;
/// The IAT slot of `iggy_w32!_IggyShutdown@0` (stdcall, no words).
pub const IGGY_SHUTDOWN: u32 = 0x00b0_c808;
/// `Phyre_PTTYCallback_dispatch_00435390(level, message)` (cdecl).
pub const TTY_DISPATCH: u32 = 0x0043_5390;
/// The CRT's `eh vector destructor iterator(p, size, count, dtor)`
/// (stdcall).
pub const EH_VECTOR_DTOR: u32 = 0x0094_96d2;
/// `0x006142d0`'s element destructor and the array's shape.
pub const RING_ELEMENT_DTOR: u32 = 0x0061_4200;
pub const RING_ELEMENT_SIZE: u32 = 0x200c;
/// `PShaderPassStateD3D11_apply_0059ad20(state; key)` (thiscall).
pub const PASS_STATE_APPLY: u32 = 0x0059_ad20;
/// The render-interface state setter `0x00599820(state; a, b)` that
/// `0x00599810` forwards to with `this + 0x570`.
pub const SET_STATE_570: u32 = 0x0059_9820;
pub const STATE_570: u32 = 0x570;
/// The three-word setters `0x00615cd0`, `0x00615da0` and the four-float
/// `0x00615b50` forward to (thiscall, `0x17`).
pub const FWD_3_A: u32 = 0x0059_a1a0;
pub const FWD_3_B: u32 = 0x0059_a910;
pub const FWD_4: u32 = 0x0059_9800;
/// The thread-pool jobs' destructors (`thiscall`): the renderer job's
/// `0x005b4aa0` and the ring-buffered job's `0x005cd3b0`.
pub const RENDERER_JOB_DTOR: u32 = 0x005b_4aa0;
pub const RING_JOB_DTOR: u32 = 0x005c_d3b0;
/// The vtables the small destructors store.
pub const BUFFER_ACCEPTANCE_VTABLE: u32 = 0x00b3_f168;
pub const BUFFER_ALLOCATOR_VTABLE: u32 = 0x00b3_f174;
pub const AVOID_OVERWRITE_VTABLE: u32 = 0x00b3_f17c;
/// `PAvoidOutputOverwritingInput`'s parent destructor body (folded with
/// `SpriteFrameRegion_SpriteFrameRegion`, `thiscall`).
pub const AVOID_OVERWRITE_PARENT: u32 = 0x0061_6140;
/// `TextUtil_terminate` (thiscall).
pub const TEXT_UTIL_TERMINATE: u32 = 0x006f_6950;
/// `g_CharacterLighting`, `g_CharacterLightingTweakUtil`,
/// `g_FogEffectManager` and what `graphicFinalize` runs on them.
pub const CHARACTER_LIGHTING: u32 = 0x00cc_ca84;
pub const CHARACTER_LIGHTING_TWEAK: u32 = 0x00cc_ca88;
pub const FOG_EFFECT_MANAGER: u32 = 0x00ce_5170;
pub const CHARACTER_LIGHTING_DTOR: u32 = 0x006a_69b0;
pub const TEXT_UTIL_ENTRY_DTOR: u32 = 0x0069_2950;
/// `Phyre_PRendering_PSceneWideParameters_getBuffer(params; a, b, c)`.
pub const SCENE_WIDE_GET_BUFFER: u32 = 0x004f_dec0;
/// `Phyre_PRendering_PRenderThread_GetInstance` (cdecl) and
/// `Phyre_PRendering_PRenderThread_sync` (thiscall).
pub const RENDER_THREAD_INSTANCE: u32 = 0x0061_4410;
pub const RENDER_THREAD_SYNC: u32 = 0x0061_5ea0;
/// `Phyre_PSharray_element28_00585c90(array; i)` (thiscall).
pub const SHARRAY_ELEMENT: u32 = 0x0058_5c90;
/// `Phyre_PRendering_PRendererBase_renderMeshInstance(renderer;
/// instance, contexts, x, y)` (thiscall).
pub const RENDER_MESH_INSTANCE: u32 = 0x005b_9a90;
/// The render jobs' element size in `0x005b5330`'s array.
pub const JOB_SIZE: u32 = 0xe4;
/// A bitmap-font glyph's size.
pub const GLYPH_SIZE: u32 = 0x30;
/// What `Phyre_PRendering_PRenderInterface` setters return.
pub const E_OK: u32 = 0;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    fn this_call(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    /// The import in IAT slot `slot` (stdcall).
    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Stdcall, args)
    }

    /// A `float` loaded and stored by the x87 (`fld dword; fstp dword`):
    /// the same bits, a signalling NaN quieted.
    fn through_x87(bits: u32) -> u32 {
        x87::round_trip(f32::from_bits(bits)).to_bits()
    }

    // ---- the render interface and the renderer ----

    /// `0x00599810(this; a, b)`: the setter `0x00599820` on `this + 0x570`
    /// (a tail jump); its `eax`.
    pub fn set_state_570(h: &mut impl Host, this: u32, a: u32, b: u32) -> u32 {
        this_call(h, SET_STATE_570, this.wrapping_add(STATE_570), &[a, b])
    }

    /// `0x0059a1c0(this; flag)`: `[this + 0x5a0]` = the flag byte is not
    /// 0; 0.
    pub fn set_5a0(h: &mut impl Host, this: u32, flag: u32) -> u32 {
        h.write(this + 0x5a0, u32::from(flag & 0xff != 0));
        E_OK
    }

    /// `0x0059a8d0(this; mode)`: `[this + 0x574]` = 2 for mode 1, else 3;
    /// 0.
    pub fn set_574(h: &mut impl Host, this: u32, mode: u32) -> u32 {
        h.write(this + 0x574, if mode == 1 { 2u32 } else { 3 });
        E_OK
    }

    /// `0x00599ed0(this; r, g, b, a)`: the colour write mask, one bit per
    /// non-zero byte (red bit 0 .. alpha bit 3), at byte `+0x5f4`; 0.
    pub fn set_write_mask(h: &mut impl Host, this: u32, rgba: [u32; 4]) -> u32 {
        let mut mask = 0u8;
        for (bit, &v) in rgba.iter().enumerate() {
            if v & 0xff != 0 {
                mask |= 1 << bit;
            }
        }
        h.write(this + 0x5f4, mask);
        E_OK
    }

    /// `0x00615cd0`, `0x00615da0`: `(this; a, x, y)` forwarded to `f` with
    /// the floats passed through the x87; its `eax`.
    pub fn forward_3(h: &mut impl Host, f: u32, this: u32, a: u32, x: u32, y: u32) -> u32 {
        let (x, y) = (through_x87(x), through_x87(y));
        this_call(h, f, this, &[a, x, y])
    }

    /// `0x00615b50(this; x, y, z, w)`: forwarded to [`FWD_4`] with the
    /// floats passed through the x87; its `eax`.
    pub fn forward_4(h: &mut impl Host, this: u32, v: [u32; 4]) -> u32 {
        let v = v.map(through_x87);
        this_call(h, FWD_4, this, &v) // call width: 4
    }

    /// `0x0061c870(this)`: the pass state `[[this] + 0x14]` applied with
    /// `this + 4` when both are set; `eax` as the original leaves it.
    pub fn apply_pass_state(h: &mut impl Host, this: u32) -> u32 {
        let owner: u32 = h.read(this);
        if owner == 0 {
            return 0;
        }
        let state: u32 = h.read(owner + 0x14);
        if state == 0 {
            return owner;
        }
        this_call(h, PASS_STATE_APPLY, state, &[this + 4])
    }

    /// `Renderer_PresentFlip(this)`: the swap chain at `+0x74` presents
    /// (`IDXGISwapChain::Present(sync, 0)`, sync = byte `+0x131c` set); 1,
    /// or 0 without a swap chain.
    pub fn present_flip(h: &mut impl Host, this: u32) -> u8 {
        let chain: u32 = h.read(this + 0x74);
        if chain == 0 {
            return 0;
        }
        let sync = u32::from(h.read::<u8>(this + 0x131c) != 0);
        let vt: u32 = h.read(chain);
        let present: u32 = h.read(vt + 0x20);
        h.call(present, Conv::Stdcall, &[chain, sync, 0]);
        1
    }

    /// `0x00592b00(this; a, b, c)`: method `+0xa4` of the COM object at
    /// `+0x70` with the three words; 0.
    pub fn com_a4(h: &mut impl Host, this: u32, a: u32, b: u32, c: u32) -> u32 {
        let obj: u32 = h.read(this + 0x70);
        let vt: u32 = h.read(obj);
        let f: u32 = h.read(vt + 0xa4);
        h.call(f, Conv::Stdcall, &[obj, a, b, c]);
        E_OK
    }

    /// `0x0059c330(this)`: the two pairs of COM objects at `+0`/`+8` and
    /// `+4`/`+0xc` released (`Release`, method `+8`) and cleared, in the
    /// original's order (`+0`, `+8`, `+4`, `+0xc`). Its callers read no
    /// `eax`.
    pub fn release_pairs(h: &mut impl Host, this: u32) {
        for i in 0..2u32 {
            for at in [this + 4 * i, this + 4 * i + 8] {
                let obj: u32 = h.read(at);
                if obj != 0 {
                    let vt: u32 = h.read(obj);
                    let f: u32 = h.read(vt + 8);
                    h.call(f, Conv::Stdcall, &[obj]);
                    h.write(at, 0u32);
                }
            }
        }
    }

    /// `0x005bc440(this)`: the critical section at `+0x44` closed and
    /// its owner byte cleared, `+0x44` and `+0x54` zeroed. `eax`: the
    /// close's 0, or `None` (the caller's `eax`) without a section.
    pub fn close_section(h: &mut impl Host, this: u32) -> Option<u32> {
        let cs: u32 = h.read(this + 0x44);
        let eax = if cs == 0 {
            None
        } else {
            let r = this_call(h, CLOSE_HANDLE_AT, cs, &[]);
            h.write(cs + 4, 0u8);
            h.write(this + 0x44, 0u32);
            Some(r)
        };
        h.write(this + 0x54, 0u32);
        eax
    }

    /// `0x005bc400(this)`: the render thread synced, then
    /// [`close_section`]; `eax` the close's, else the sync's.
    pub fn sync_and_close(h: &mut impl Host, this: u32) -> u32 {
        let thread = cdecl(h, RENDER_THREAD_INSTANCE, &[]);
        let synced = this_call(h, RENDER_THREAD_SYNC, thread, &[]);
        close_section(h, this).unwrap_or(synced)
    }

    /// `Phyre_PRendering_PRendererBase_generateSceneWideParameterCaptureBuffer(this)`:
    /// `[this + 0x58]`, made from the scene-wide parameters (`+0x54`) and
    /// the camera (`+0x5c`) when missing; 0 without either.
    pub fn scene_wide_capture(h: &mut impl Host, this: u32) -> u32 {
        let have: u32 = h.read(this + 0x58);
        if have == 0 {
            let params: u32 = h.read(this + 0x54);
            if params == 0 {
                return 0;
            }
            let camera: u32 = h.read(this + 0x5c);
            if camera == 0 {
                return 0;
            }
            let ctx: u32 = h.read(this + 0x18);
            let b = this_call(h, SCENE_WIDE_GET_BUFFER, params, &[camera, ctx, this]);
            h.write(this + 0x58, b);
        }
        h.read(this + 0x58)
    }

    /// `0x00593fe0(this; i)`: word `+0xc` of element `i` of the
    /// `PSharray` at `+0x24`, 0 past its end.
    pub fn sharray_word_c(h: &mut impl Host, this: u32, i: u32) -> u32 {
        let count = h.read::<u32>(this + 0x24) & 0x7fff_ffff;
        if i >= count {
            return 0;
        }
        let e = this_call(h, SHARRAY_ELEMENT, this + 0x24, &[i]);
        h.read(e.wrapping_add(0xc))
    }

    /// `0x005b5330(jobs, n)`: each of the `n` 0xe4-byte jobs destroyed
    /// (virtual slot 0, flags 0), then the array freed; nothing for a
    /// null array.
    pub fn free_jobs(h: &mut impl Host, jobs: u32, n: u32) {
        if jobs == 0 {
            return;
        }
        for i in 0..n {
            let job = jobs.wrapping_add(i.wrapping_mul(JOB_SIZE));
            h.call_virtual(job, 0, &[0]);
        }
        cdecl(h, PHYRE_FREE, &[jobs]);
    }

    /// The deleting destructors of the thread-pool jobs (`0x005b5080`,
    /// `0x005b50b0`): `dtor(this)`, `operator delete(this)` when
    /// `flags & 1`; `this`.
    pub fn job_delete(h: &mut impl Host, this: u32, flags: u32, dtor: u32) -> u32 {
        this_call(h, dtor, this, &[]);
        if flags & 1 != 0 {
            cdecl(h, OPERATOR_DELETE, &[this]);
        }
        this
    }

    /// `0x006142d0(this)`: the two 0x200c-byte ring elements at `this + 8`
    /// destroyed through the CRT's vector destructor; its `eax`.
    pub fn ring_elements_dtor(h: &mut impl Host, this: u32) -> u32 {
        let dtor = h.rebase(RING_ELEMENT_DTOR);
        h.call_sym(
            EH_VECTOR_DTOR,
            Conv::Stdcall,
            &[this + 8, RING_ELEMENT_SIZE, 2, dtor],
        )
    }

    /// `PDeferredKick::canKick(this)`: word 0 is not 0.
    pub fn can_kick(h: &mut impl Host, this: u32) -> u32 {
        u32::from(h.read::<u32>(this) != 0)
    }

    /// `0x00613fb0(this)`: word 0 = 0; `this`.
    pub fn clear_word0(h: &mut impl Host, this: u32) -> u32 {
        h.write(this, 0u32);
        this
    }

    /// `0x00613f30(this; a, b)`: `{b | 0x80000000, a}`; `this`.
    pub fn init_external(h: &mut impl Host, this: u32, a: u32, b: u32) -> u32 {
        h.write(this + 4, a);
        h.write(this, b | 0x8000_0000);
        this
    }

    /// `0x005b4930(this)`: word `+4` of the object at word 0 cleared when
    /// there is one; `eax` = word 0.
    pub fn clear_owner_word4(h: &mut impl Host, this: u32) -> u32 {
        let p: u32 = h.read(this);
        if p != 0 {
            h.write(p + 4, 0u32);
        }
        p
    }

    /// `0x006e2e20(p)`: word `+4` of `p` cleared when not null; `p`.
    pub fn clear_word4(h: &mut impl Host, p: u32) -> u32 {
        if p != 0 {
            h.write(p + 4, 0u32);
        }
        p
    }

    // ---- small destructors ----

    /// `0x00616170(this; flags)`: `PBufferAcceptance`'s deleting
    /// destructor: its vtable, `operator delete` when `flags & 1`; `this`.
    pub fn acceptance_delete(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let vt = h.rebase(BUFFER_ACCEPTANCE_VTABLE);
        h.write(this, vt);
        if flags & 1 != 0 {
            cdecl(h, OPERATOR_DELETE, &[this]);
        }
        this
    }

    /// `0x006161a0(this; flags)`: `PNetworkOptimizeBufferAllocator`'s
    /// deleting destructor: its vtable, its buffer (`+4`) freed when set,
    /// `operator delete` when `flags & 1`; `this`.
    pub fn allocator_delete(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let buffer: u32 = h.read(this + 4);
        let vt = h.rebase(BUFFER_ALLOCATOR_VTABLE);
        h.write(this, vt);
        if buffer != 0 {
            cdecl(h, PHYRE_FREE, &[buffer]);
        }
        if flags & 1 != 0 {
            cdecl(h, OPERATOR_DELETE, &[this]);
        }
        this
    }

    /// `0x006165c0(this; flags)`: `PAvoidOutputOverwritingInput`'s
    /// deleting destructor: its vtable, the parent's body, `operator
    /// delete` when `flags & 1`; `this`.
    pub fn avoid_overwrite_delete(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let vt = h.rebase(AVOID_OVERWRITE_VTABLE);
        h.write(this, vt);
        this_call(h, AVOID_OVERWRITE_PARENT, this, &[]);
        if flags & 1 != 0 {
            cdecl(h, OPERATOR_DELETE, &[this]);
        }
        this
    }

    /// `0x006164a0(this; a, b)`: `{vtable, a, b}`; `this`.
    pub fn avoid_overwrite_ctor(h: &mut impl Host, this: u32, a: u32, b: u32) -> u32 {
        h.write(this + 4, a);
        h.write(this + 8, b);
        let vt = h.rebase(AVOID_OVERWRITE_VTABLE);
        h.write(this, vt);
        this
    }

    // ---- the graphic managers' teardown ----

    /// `0x006a6dd0()`: `g_CharacterLighting` destroyed, deleted and
    /// cleared when set.
    pub fn character_lighting_finalize(h: &mut impl Host) {
        let at = h.rebase(CHARACTER_LIGHTING);
        let p: u32 = h.read(at);
        if p != 0 {
            this_call(h, CHARACTER_LIGHTING_DTOR, p, &[]);
            cdecl(h, OPERATOR_DELETE, &[p]);
            h.write(at, 0u32);
        }
    }

    /// `0x006a6e00()`: `g_CharacterLightingTweakUtil`'s entry (`+4`)
    /// destroyed, the object deleted and the global cleared when set.
    pub fn tweak_util_finalize(h: &mut impl Host) {
        let at = h.rebase(CHARACTER_LIGHTING_TWEAK);
        let p: u32 = h.read(at);
        if p != 0 {
            this_call(h, TEXT_UTIL_ENTRY_DTOR, p + 4, &[]);
            cdecl(h, OPERATOR_DELETE, &[p]);
            h.write(at, 0u32);
        }
    }

    /// `0x006e32a0()`: `g_FogEffectManager` deleted when set (the global
    /// stays); `eax` 0 or `operator delete`'s.
    pub fn fog_manager_delete(h: &mut impl Host) -> u32 {
        let at = h.rebase(FOG_EFFECT_MANAGER);
        let p: u32 = h.read(at);
        if p == 0 {
            return 0;
        }
        cdecl(h, OPERATOR_DELETE, &[p])
    }

    /// `0x006ab5c0(this)`: the text utility at `+4` terminated, the
    /// buffer at `+0xf8` deleted (`delete[]`) and cleared; `delete[]`'s
    /// `eax`.
    pub fn text_util_release(h: &mut impl Host, this: u32) -> u32 {
        this_call(h, TEXT_UTIL_TERMINATE, this + 4, &[]);
        let buffer: u32 = h.read(this + 0xf8);
        let r = cdecl(h, OPERATOR_DELETE_ARRAY, &[buffer]);
        h.write(this + 0xf8, 0u32);
        r
    }

    /// `Phyre_FogEffectManager_setChrFogFadeType(this; type)`: `+0xb4` =
    /// the type, 2 for any type of 3 or more (signed); `eax` the type.
    pub fn set_fog_fade_type(h: &mut impl Host, this: u32, kind: u32) -> u32 {
        let v = if (kind as i32) < 3 { kind } else { 2 };
        h.write(this + 0xb4, v);
        kind
    }

    // ---- Iggy ----

    /// `PUtilityIggy::terminate` (`0x00a3e720`): `IggyShutdown()`; 0.
    pub fn iggy_terminate(h: &mut impl Host) -> u32 {
        import(h, IGGY_SHUTDOWN, &[]);
        0
    }

    /// `0x00a3e5e0(a, b, message, d)`: Iggy's warning callback, the
    /// message printed at level 2; the print's `eax`.
    pub fn iggy_warning(h: &mut impl Host, message: u32) -> u32 {
        cdecl(h, TTY_DISPATCH, &[2, message])
    }

    // ---- the bitmap font ----

    /// `Phyre_PText_PBitmapFontGlyph_ctor_00a2bab0(this)`: the glyph
    /// zeroed (the floats as `+0.0`, byte `+0x2c`); `this`.
    pub fn glyph_ctor(h: &mut impl Host, this: u32) -> u32 {
        h.write(this, [0u32; 11]);
        h.write(this + 0x2c, 0u8);
        this
    }

    /// `Phyre_PText_PBitmapFontGlyphArray_find_00a29320(this; key, first)`:
    /// the index of the first glyph from `first` on whose word 0 is
    /// `*key`, or -1.
    pub fn glyph_find(h: &mut impl Host, this: u32, key: u32, first: u32) -> u32 {
        let count = h.read::<u32>(this) & 0x7fff_ffff;
        if first >= count {
            return u32::MAX;
        }
        let want: u32 = h.read(key);
        let data: u32 = h.read(this + 4);
        for i in first..count {
            let g = data.wrapping_add(i.wrapping_mul(GLYPH_SIZE));
            if h.read::<u32>(g) == want {
                return i;
            }
        }
        u32::MAX
    }

    /// `0x00a2a400(this; inst)`: the text's mesh instance drawn: `+0x164`
    /// = `inst`, `inst + 0x48` = `[[this + 0x158] + 0x34]`, then
    /// `renderMeshInstance(inst; this + 0xa4, [this + 0xb4], 0, [inst +
    /// 0x13c])`; its `eax`.
    pub fn text_render(h: &mut impl Host, this: u32, inst: u32) -> u32 {
        let font: u32 = h.read(this + 0x158);
        h.write(this + 0x164, inst);
        let v: u32 = h.read(font + 0x34);
        let y: u32 = h.read(inst + 0x13c);
        h.write(inst + 0x48, v);
        let list = this + 0xa4;
        let n: u32 = h.read(list + 0x10);
        this_call(h, RENDER_MESH_INSTANCE, inst, &[list, n, 0, y])
    }

    /// `0x005c8580(pp, out, len)`: `*out = *pp`, `*len` = its length
    /// with the terminator; 0.
    pub fn string_with_length(h: &mut impl Host, pp: u32, out: u32, len: u32) -> u32 {
        let s: u32 = h.read(pp);
        let mut n = 0u32;
        while h.read::<u8>(s.wrapping_add(n)) != 0 {
            n = n.wrapping_add(1);
        }
        h.write(len, n.wrapping_add(1));
        h.write(out, s);
        0
    }
}

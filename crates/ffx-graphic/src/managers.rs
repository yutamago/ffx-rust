//! The render layer's managers (singletons the remaster keeps beside the
//! scene: post-processing, shadows, texture animation, dynamic geometry
//! memory, distortion and blur) and their one-line accessors.
//!
//! Each manager is a C++ object; these are the accessors whose logic is
//! the game's (fields, counters, singletons). Their virtual methods and
//! the `PhyreEngine` work behind them stay original.

/// The originals, over game memory.
pub mod bridge {
    use ffx_bridge::{Global, Mem, x87};

    pub use crate::scene::bridge::{embedded, field, set_byte_field, set_field, set_if_changed};

    /// `g_PostProcessManager` (`PostProcessManager_GetInstance`).
    pub const POST_PROCESS: Global<u32> = Global::new(0x00cc_c8a0);
    /// `g_PostProcessTweakUtil` (`PostProcessTweakUtil_GetInstance`).
    pub const POST_PROCESS_TWEAK: Global<u32> = Global::new(0x00cc_c89c);
    /// `g_TextureAnimationManager` (`TextureAnimationManager_GetInstance`).
    pub const TEXTURE_ANIMATION: Global<u32> = Global::new(0x00cc_caa8);
    /// `g_ShadowManager` (`ShadowManager_getInstance`).
    pub const SHADOW: Global<u32> = Global::new(0x00cd_ec28);
    /// `g_ShadowSize` (`ShadowManager_GetShadowSize`).
    pub const SHADOW_SIZE: Global<u32> = Global::new(0x00c3_4408);
    /// `g_SpecularMapParamsManager` (`SpecularMapParamsManager_GetInstance`).
    pub const SPECULAR_MAP_PARAMS: Global<u32> = Global::new(0x00cd_ec1c);
    /// `g_DebugMenuManager` (`DebugMenuManager_GetInstance`).
    pub const DEBUG_MENU: Global<u32> = Global::new(0x00cc_cab0);
    /// `g_GraphicPauseGame` (`graphicCheckPauseGame`), an object pointer.
    pub const PAUSE_GAME: Global<u32> = Global::new(0x00cc_cb00);
    /// `g_DynGeoFillBuffer`: the dynamic geometry fill buffer's state.
    pub const DYN_GEO_FILL_BUFFER: Global<u32> = Global::new(0x00cd_edf0);

    /// A singleton getter: the global's value.
    pub fn instance(m: &impl Mem, g: Global<u32>) -> u32 {
        g.get(m)
    }

    /// `PostProcessTweakUtil_Destroy` (`FUN_00692bc0`) and the other
    /// singleton deleters (`DebugMenuManager`, the pause object): the
    /// virtual destructor `(1)` (deleting), then the global is cleared.
    /// Returns `eax` when an object was deleted (its destructor's), else
    /// `None` (`eax` untouched).
    pub fn destroy_singleton(h: &mut impl crate::host::Host, g: Global<u32>) -> Option<u32> {
        let obj = g.get(h);
        if obj == 0 {
            return None;
        }
        let r = h.call_virtual(obj, 0, &[1]);
        g.set(h, 0);
        Some(r)
    }

    /// `graphicDynGeoFillBufferIsValid()`: the fill buffer's first word is
    /// set.
    pub fn dyn_geo_fill_buffer_is_valid(m: &impl Mem) -> u32 {
        let p = DYN_GEO_FILL_BUFFER.get(m);
        u32::from(m.read::<u32>(p) != 0)
    }

    // ---- DynGeoMemManager: +0 the current buffer set ----
    // A buffer set: +0 the current buffer, +4 its index, +8 the count,
    // +0x10 the buffers' table, +0x14 the mutex; a buffer: +4 its start,
    // +8 its fill position.

    /// `DynGeoMemManager_getCurrentBufferId(this)`: the set's index, -1
    /// without one.
    pub fn dyn_geo_current_buffer_id(m: &impl Mem, this: u32) -> u32 {
        match m.read::<u32>(this) {
            0 => u32::MAX,
            set => m.read(set + 4),
        }
    }

    /// `DynGeoMemManager_clearDynGeoMemory(this)`: the current buffer's
    /// fill position back to its start. Returns `eax` when it did
    /// (`None`: untouched).
    pub fn dyn_geo_clear(m: &mut impl Mem, this: u32) -> Option<u32> {
        let set: u32 = m.read(this);
        if set == 0 {
            return None;
        }
        let index: u32 = m.read(set + 4);
        let slot = m
            .read::<u32>(set + 0x10)
            .wrapping_add(index.wrapping_mul(4));
        let buffer: u32 = m.read(slot);
        let start: u32 = m.read(buffer + 4);
        m.write(set, start);
        let buffer: u32 = m.read(slot);
        let start: u32 = m.read(buffer + 4);
        m.write(buffer + 8, start);
        Some(start)
    }

    /// `DynGeoMemManager_selectBuffer(this, i)` (`FUN_006dec90`): the
    /// current word becomes word `i + 1`; returns it.
    pub fn dyn_geo_select(m: &mut impl Mem, this: u32, i: u32) -> u32 {
        let v: u32 = m.read(this.wrapping_add(i.wrapping_mul(4)).wrapping_add(4));
        m.write(this, v);
        v
    }

    // ---- TextureAnimationManager ----

    /// `TextureAnimationManager_getMaterialByLayNum(this, lay)`: material
    /// `+8 + +0xc + lay` of the table at `+0x78` (its count at `+0x74`,
    /// bit 31 a flag), 0 past the end.
    pub fn material_by_lay_num(m: &impl Mem, this: u32, lay: u32) -> u32 {
        let count = m.read::<u32>(this + 0x74) & 0x7fff_ffff;
        let i = m
            .read::<u32>(this + 0xc)
            .wrapping_add(m.read(this + 8))
            .wrapping_add(lay);
        if i < count {
            m.read(m.read::<u32>(this + 0x78).wrapping_add(i.wrapping_mul(4)))
        } else {
            0
        }
    }

    /// `TextureAnimation_next(this)` (`FUN_006b22c0`, with `step` 1) and
    /// `TextureAnimation_prev` (`FUN_006b2380`, `step` -1): the frame
    /// (`+0x18`) moves by `step` modulo the frame count (`+8`, bit 31 a
    /// flag); returns the quotient (`eax`; `edx` is the new frame). With a
    /// count of 0 the original faults (`div`); this leaves the frame and
    /// returns 0.
    pub fn step_frame(m: &mut impl Mem, this: u32, step: u32) -> u32 {
        let count = m.read::<u32>(this + 8) & 0x7fff_ffff;
        let v = m
            .read::<u32>(this + 0x18)
            .wrapping_add(step)
            .wrapping_add(count);
        let (Some(q), Some(r)) = (v.checked_div(count), v.checked_rem(count)) else {
            return 0;
        };
        m.write(this + 0x18, r);
        q
    }

    /// `TextureAnimationInfoChar_active(this)` (0) and `_deactive` (1):
    /// the byte `+0x34`.
    pub fn set_inactive(m: &mut impl Mem, this: u32, inactive: bool) {
        m.write(this + 0x34, u8::from(inactive));
    }

    /// The `thiscall` getters of two floats (`FUN_006b1cc0`: `+0x24`,
    /// `FUN_006b1ce0`: `+0x2c`) into `out`, through the x87. Returns `out`.
    pub fn get_float_pair(m: &mut impl Mem, this: u32, offset: u32, out: u32) -> u32 {
        for i in 0..2 {
            let v: u32 = m.read(this + offset + 4 * i);
            m.write(out + 4 * i, x87::round_trip(f32::from_bits(v)).to_bits());
        }
        out
    }

    /// The `thiscall` setters of one float (`fld`/`fstp`).
    pub fn set_float(m: &mut impl Mem, this: u32, offset: u32, v: f32) {
        m.write(this + offset, x87::round_trip(v).to_bits());
    }

    /// `Distortion_updateDistortionBlur(this, v)`: the float `+0x78`;
    /// returns 0.
    pub fn distortion_update_blur(m: &mut impl Mem, this: u32, v: f32) -> u32 {
        set_float(m, this, 0x78, v);
        0
    }

    /// `BlurEffect_activate(this)`: active (`+0xc`), weight 0 (`+0x84`),
    /// state 2 (`+0x10`).
    pub fn blur_activate(m: &mut impl Mem, this: u32) {
        m.write(this + 0xc, 1u8);
        m.write(this + 0x84, 0u32);
        m.write(this + 0x10, 2u32);
    }

    /// `BlurEffect_setTarget(this, v)` (`FUN_006c4a90`): `+0x2c` and the
    /// changed flag `+0x28`. Returns `v`.
    pub fn set_with_flag(m: &mut impl Mem, this: u32, v: u32) -> u32 {
        m.write(this + 0x2c, v);
        m.write(this + 0x28, 1u8);
        v
    }

    /// `ShadowManager_clearCascades(this)` (`FUN_006d1780`): 64 words from
    /// `+0xfe8` to 0. Returns 0 (`eax`).
    pub fn clear_words(m: &mut impl Mem, this: u32) -> u32 {
        m.write_bytes(this + 0xfe8, &[0; 0x100]);
        0
    }

    /// `ShadowManager_getCascade(this, i)` (`FUN_006d22e0`): word `i` from
    /// `+0x60`.
    pub fn word_at(m: &impl Mem, this: u32, base: u32, i: u32) -> u32 {
        m.read(this.wrapping_add(base).wrapping_add(i.wrapping_mul(4)))
    }

    /// `PostProcessManager_isEffectAllowed(id)` (`FUN_00698730`): false for
    /// the effects 0x17 and 0x15c; in `al`.
    #[must_use]
    pub fn effect_allowed(id: u32) -> u8 {
        u8::from(id != 0x17 && id != 0x15c)
    }
}

//! Render switches the system layer keeps: the extended alpha scale, the
//! point-light shader for one spell, the depth-bias table, the effect
//! fill-buffer requests.

/// The magic ID whose effect uses the point-light shader
/// (`useShaderPointLight`).
pub const POINT_LIGHT_MAGIC: u32 = 0x207;

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use ffx_bridge::x87::{self, Reg};

    /// The alpha scale `useExtendedAlpha` sets (`f32`).
    pub const ALPHA_SCALE: Global<f32> = Global::new(0x00c3_a4d0);
    /// The scale with extended alpha (the constant 512.0 in `.rdata`).
    pub const ALPHA_EXTENDED: Global<f32> = Global::new(0x00b4_1d9c);
    /// The scale without (the constant 256.0 in `.rdata`).
    pub const ALPHA_NORMAL: Global<f32> = Global::new(0x00b5_01c0);
    /// The depth-bias table has entries to clear.
    pub const DEPTH_BIAS_DIRTY: Global<u8> = Global::new(0x00ce_5176);
    /// Entries in use in the depth-bias table.
    pub const DEPTH_BIAS_COUNT: Global<u32> = Global::new(0x0194_2998);
    /// The depth-bias table: 256 entries of (`u32`, `f32` scale, `f32`).
    pub const DEPTH_BIAS: Global<[u32; 3]> = Global::new(0x0194_299c);
    /// Entries in [`DEPTH_BIAS`].
    pub const DEPTH_BIAS_ENTRIES: u32 = 0x100;
    /// The effect fill-buffer requests (12 words).
    pub const FILL_REQUESTS: Global<[u32; 12]> = Global::new(0x00ce_c1c0);

    /// `useExtendedAlpha(on)` (`0x0071e460`).
    pub fn use_extended_alpha(mem: &mut impl Mem, on: u32) {
        let src = if on != 0 {
            ALPHA_EXTENDED
        } else {
            ALPHA_NORMAL
        };
        let v = x87::round_trip(src.get(mem));
        ALPHA_SCALE.set(mem, v);
    }

    /// `useShaderPointLight()` (`0x0071e6f0`): the current magic is
    /// [`super::POINT_LIGHT_MAGIC`].
    pub fn use_shader_point_light(mem: &impl Mem) -> u32 {
        u32::from(crate::instance::bridge::magic_id(mem) == super::POINT_LIGHT_MAGIC)
    }

    /// `DepthBiasHooks::clearDepthBias()` (`0x006e40b0`): when dirty, every
    /// entry back to (0, 1.0, 0.0) and the count to 0.
    pub fn clear_depth_bias(mem: &mut impl Mem) {
        if DEPTH_BIAS_DIRTY.get(mem) == 0 {
            return;
        }
        DEPTH_BIAS_DIRTY.set(mem, 0);
        let t = DEPTH_BIAS.ptr(mem);
        for i in 0..DEPTH_BIAS_ENTRIES {
            let e = t.at(i).addr;
            mem.write(e, 0u32);
            mem.write(e + 4, Reg::ONE.store());
            mem.write(e + 8, Reg::ZERO.store());
        }
        DEPTH_BIAS_COUNT.set(mem, 0);
    }

    /// `JobSchedule::pppPart::clearFillBufferRequest()` (`0x0071ecd0`).
    pub fn clear_fill_buffer_request(mem: &mut impl Mem) {
        FILL_REQUESTS.set(mem, [0; 12]);
    }
}

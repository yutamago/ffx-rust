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

    /// The six fill-buffer queues' bases (`0x00cec1a8`), request counts
    /// (`0x00cec1c0`) and loads (`0x00cec1d8`).
    pub const FILL_BUFFERS: Global<[u32; 6]> = Global::new(0x00ce_c1a8);
    /// See [`FILL_BUFFERS`].
    pub const FILL_COUNTS: Global<[i32; 6]> = Global::new(0x00ce_c1c0);
    /// See [`FILL_BUFFERS`].
    pub const FILL_LOADS: Global<[i32; 6]> = Global::new(0x00ce_c1d8);

    /// `JobSchedule_pppPart_getFillBufferRequest(size)` (`0x00728660`): of
    /// the queues with fewer than 0x156 requests, the least loaded (first on
    /// a tie, and only below 100000) gets the request: its count and load
    /// grow and the request's 0x214-byte slot is returned; 0 when none.
    pub fn get_fill_buffer_request(mem: &mut impl Mem, size: i32) -> u32 {
        let counts = FILL_COUNTS.ptr(mem).addr;
        let loads = FILL_LOADS.ptr(mem).addr;
        let mut best = 100_000i32;
        let mut pick = None;
        for k in 0..6u32 {
            let count: i32 = mem.read(counts + 4 * k);
            let load: i32 = mem.read(loads + 4 * k);
            if count < 0x156 && load < best {
                best = load;
                pick = Some(k);
            }
        }
        let Some(k) = pick else {
            return 0;
        };
        let count = mem.read::<i32>(counts + 4 * k).wrapping_add(1);
        mem.write(counts + 4 * k, count);
        let load = mem.read::<i32>(loads + 4 * k).wrapping_add(size);
        mem.write(loads + 4 * k, load);
        let base: u32 = mem.read(FILL_BUFFERS.ptr(mem).addr + 4 * k);
        base.wrapping_sub(0x214)
            .wrapping_add(count.cast_unsigned().wrapping_mul(0x214))
    }

    /// The UI element record `BeginScissor_XY`/`graphicDrawUIElement` draw
    /// (`0x025d1700`): 0x20-byte vertices, `x, y` at `+0`, `u, v` at `+8`,
    /// the colour words at `+0x10`.
    pub const UI_VERTICES: u32 = 0x025d_1700;

    /// `0x009066d0(x, y, k)`: vertex `k`'s position (`f32`s through the
    /// x87, signalling NaNs quieted). Returns `k × 0x20` (the original's
    /// `eax`).
    pub fn set_ui_vertex_xy(mem: &mut impl Mem, x: f32, y: f32, k: u32) -> u32 {
        set_ui_vertex_pair(mem, 0, x, y, k)
    }

    /// `0x009066b0(u, v, k)`: vertex `k`'s texture coordinates, as
    /// [`set_ui_vertex_xy`].
    pub fn set_ui_vertex_uv(mem: &mut impl Mem, u: f32, v: f32, k: u32) -> u32 {
        set_ui_vertex_pair(mem, 8, u, v, k)
    }

    fn set_ui_vertex_pair(mem: &mut impl Mem, off: u32, a: f32, b: f32, k: u32) -> u32 {
        use ffx_bridge::x87::round_trip;
        let at = k.wrapping_shl(5);
        let vertex = mem.rebase(UI_VERTICES).wrapping_add(at).wrapping_add(off);
        mem.write(vertex, round_trip(a));
        mem.write(vertex.wrapping_add(4), round_trip(b));
        at
    }
}

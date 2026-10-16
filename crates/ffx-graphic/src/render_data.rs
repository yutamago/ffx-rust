//! Filling an effect's render data from its particle part
//! (`updateVFXRenderData`): the per-part render data flags, the alpha
//! scale of a few magic files, the colour, the point light, the YIQ
//! switch, the job-system fill request and, per segment of the part's
//! shape, the dynamic-geometry buffers (`docs/formats/render.md`).

/// The shape segment types: `(vertices, indices)` per primitive count `n`
/// as `graphicDynGeoMemManagerGetBufferForSegment` is asked for them, and
/// the bytes each primitive takes in the shape data.
///
/// `None` for the types the original does not handle (the cursor stays).
#[must_use]
pub fn segment_type(kind: i32) -> Option<SegmentType> {
    let (quads, stride, per_part) = match kind {
        0 => (false, 0x14, true),
        1 => (false, 0x1c, true),
        2 => (false, 0x20, true),
        3 => (false, 0x28, false),
        4 => (true, 0x18, true),
        5 => (true, 0x20, false),
        6 => (true, 0x28, true),
        7 => (true, 0x30, false),
        _ => return None,
    };
    Some(SegmentType {
        quads,
        stride,
        per_part,
    })
}

/// How a shape segment's buffers are made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentType {
    /// Quads (`4n` vertices, `6n` indices) rather than triangles (`3n`,
    /// `3n`).
    pub quads: bool,
    /// Shape-data bytes per primitive.
    pub stride: u32,
    /// Whether every part of a multi-part effect gets its own buffer
    /// (else only the first part's render data).
    pub per_part: bool,
}

impl SegmentType {
    /// The vertex and index counts for `n` primitives.
    #[must_use]
    pub fn counts(self, n: i32) -> (u32, u32) {
        #[allow(clippy::cast_sign_loss)] // the original's word arithmetic
        if self.quads {
            ((n.wrapping_mul(4)) as u32, (n.wrapping_mul(6)) as u32)
        } else {
            let v = n.wrapping_mul(3) as u32;
            (v, v)
        }
    }
}

/// The originals, over game memory.
pub mod bridge {
    use super::segment_type;
    use crate::host::{Conv, Host};
    use crate::managers::bridge::dyn_geo_fill_buffer_is_valid;
    use crate::vfx::bridge::{
        SKIP_FRAME, enable_yiq, get_dynamic_geometry_by_instance, set_parameters,
    };
    use crate::vfx::part_key;
    use ffx_bridge::{Global, Mem, x87};

    /// `g_MagicFile_MagicId` (what `MagicFile_GetMagicId` returns).
    pub const MAGIC_ID: Global<u32> = Global::new(0x00c6_4ca0);
    /// `g_AlphaScale` (`f32`).
    pub const ALPHA_SCALE: Global<u32> = Global::new(0x00c3_a4d0);
    /// `g_AlphaScaleInv`: `0.0078125 / g_AlphaScale` (`f32`).
    pub const ALPHA_SCALE_INV: Global<u32> = Global::new(0x00c3_a494);
    /// `g_VfxColorScale`: four floats the colour is multiplied by.
    pub const COLOR_SCALE: Global<u32> = Global::new(0x00c3_a488);
    /// `g_VfxYiqMode`: -1 off.
    pub const YIQ_MODE: Global<i32> = Global::new(0x00c3_a504);
    /// `g_CurrentMapNo`.
    pub const CURRENT_MAP: Global<u32> = Global::new(0x012f_bc40);
    /// `g_pppCurrentProgram`.
    pub const PPP_CURRENT_PROGRAM: Global<u32> = Global::new(0x0230_fd20);
    /// `g_VfxPhase` (`0x1f` doubles the alpha of two magic files).
    pub const VFX_PHASE: Global<u32> = Global::new(0x00cc_b9d4);
    /// `g_VfxPass`: copied into every fill request (`+0x108`).
    pub const VFX_PASS: Global<u32> = Global::new(0x0113_335c);
    /// `g_pppFillMatrix`: 16 words copied into every fill request.
    pub const FILL_MATRIX: Global<u32> = Global::new(0x0230_fca0);

    /// `ModelBuffer_GetBufferByIndex(index)`.
    pub const MODEL_BUFFER_GET: u32 = 0x0072_c190;
    /// `ModelBuffer_ReleaseBufferByIndex(index)`.
    pub const MODEL_BUFFER_RELEASE: u32 = 0x0072_c1b0;
    /// `ModelBuffer_AllocateBuffer(shape)`.
    pub const MODEL_BUFFER_ALLOCATE: u32 = 0x0072_ac20;
    /// `sceVu0InversMatrix4x4(dst, m)`.
    pub const INVERSE_MATRIX: u32 = 0x006e_d460;
    /// `FFXVu0ApplyMatrix(dst, m, v)`.
    pub const APPLY_MATRIX: u32 = 0x006e_d2e0;
    /// `JobSchedule_pppPart_getFillBufferRequest(vertices)`.
    pub const FILL_REQUEST: u32 = 0x0072_8660;
    /// `graphicDynGeoMemManagerGetBufferForSegment(data, segment, vertices,
    /// indices)`: 0 when the memory is exhausted.
    pub const BUFFER_FOR_SEGMENT: u32 = 0x0063_fdb0;
    /// MSVCR110's `memset`, through the image's thunk.
    pub const MEMSET: u32 = 0x0094_964c;

    /// Render-data segments are this long; the original gives up at the
    /// 29th (`cmp esi, 0xca8`).
    const SEGMENT: u32 = 0x6c;

    /// `updateVFXRenderData(lo, hi, part, obj)`: 1 when done (or the frame
    /// is skipped), 0 when the instance has no render data or a buffer
    /// could not be had.
    ///
    /// The original keeps the parts in two four-entry arrays of its
    /// frame; a kind-2 effect of more than four parts (never made:
    /// `createVFXRenderDataBuffer` splits at most four) would overwrite its
    /// other locals there, and one of fewer than four leaves the fill
    /// request's other part words uninitialised. Both are zero here.
    #[allow(clippy::too_many_lines, clippy::similar_names, clippy::many_single_char_names)] // one original, in its order
    pub fn update(h: &mut impl Host, lo: u32, hi: u32, part: u32, obj: u32) -> u32 {
        // get_ptr32_func is the identity
        let shape: u32 = h.read(part + 0x1c);
        if SKIP_FRAME.get(h) != 0 {
            return 1;
        }
        let key = u64::from(hi) << 32 | u64::from(lo);
        let geo0 = get_dynamic_geometry_by_instance(h, lo, hi);
        let rd = if geo0 == 0 {
            0
        } else {
            h.read::<u32>(geo0 + 0x24)
        };
        if rd == 0 {
            return 0;
        }
        h.write(rd + 0xc1, 1u8);
        let mut header = shape.wrapping_add(h.read(shape + 4));
        let mut total = i32::from(h.read::<i16>(shape + 0x10));
        let kind = i32::from(h.read::<i8>(rd + 0xc0));
        let mut geos: Vec<u32> = Vec::new();
        let mut rds: Vec<u32> = Vec::new();
        if kind == 1 || kind == 2 {
            let mut p = 0;
            while p < if kind == 1 {
                4
            } else {
                i32::from(h.read::<u16>(rd + 0xc4))
            } {
                let (plo, phi) = halves(part_key(key, p));
                let g = get_dynamic_geometry_by_instance(h, plo, phi);
                geos.push(g);
                let r = if g == 0 { 0 } else { h.read::<u32>(g + 0x24) };
                rds.push(r);
                if r == 0 {
                    return 0;
                }
                h.write(r + 0xc1, 1u8);
                let b: u8 = h.read(obj + 0x44);
                h.write(r + 0xb6, b);
                p += 1;
            }
        } else {
            let b: u8 = h.read(obj + 0x44);
            h.write(rd + 0xb6, b);
        }
        let mut cursor = header.wrapping_add(0x10);
        let texture = h.read::<u32>(header + 8) & 0x3fff;

        // ---- the alpha scale of a few magic files ----
        if MAGIC_ID.get(h) == 0x176 && matches!(texture, 0x3780 | 0x3800 | 0x3880) {
            h.write(obj + 0x19, 0x3030u16);
            h.write(obj + 0x18, 0x30u8);
        }
        let doubled = (MAGIC_ID.get(h) == 0x66 || MAGIC_ID.get(h) == 0x67) && texture == 0x3400;
        ALPHA_SCALE.set(h, alpha(doubled));
        for id in [0x203, 0x29b] {
            if MAGIC_ID.get(h) == id
                && texture == 0
                && h.read::<u8>(obj + 8) == 0x48
                && PPP_CURRENT_PROGRAM.get(h) == 0xb
            {
                let doubled = VFX_PHASE.get(h) == 0x1f;
                ALPHA_SCALE.set(h, alpha(doubled));
            }
        }
        let inv = x87::div(0.007_812_5, x87::load(ALPHA_SCALE.get(h)));
        ALPHA_SCALE_INV.set(h, x87::store(inv));

        // ---- the colour ----
        let mut color = [1.0f32; 4];
        let (mut uv, mut rgba) = (0u32, 0u32);
        if obj != 0 {
            for (i, c) in (0u32..).zip(color.iter_mut()) {
                let b = f64::from(h.read::<u8>(obj + 0x18 + i));
                *c = f32::from_bits(x87::store(x87::mul(b, f64::from(*c))));
            }
            // (the original compares the first segment's texture name with
            // "RWTT" and on a match only reloads three of the colours)
            let scale = COLOR_SCALE.ptr(h);
            for (i, c) in (0u32..).zip(color.iter_mut()) {
                let s = f32::from_bits(scale.field::<u32>(4 * i).get(h));
                *c = x87::Reg::load(s).mul_m32(*c).store();
            }
            uv = h.read(obj + 0x10);
            rgba = h.read(obj + 0x14);
        }
        let saved_38: u32 = h.read(obj + 0x38);
        let saved_3c: u32 = h.read(obj + 0x3c);
        let v38 = if saved_38 == 0 {
            shape.wrapping_add(h.read(shape + 8))
        } else {
            saved_38
        };
        let v3c = if saved_3c == 0 {
            shape.wrapping_add(h.read(shape + 0xc))
        } else {
            saved_3c
        };
        h.write(obj + 0x38, v38);
        h.write(obj + 0x3c, v3c);

        // ---- the model buffer ----
        let index = u32::from(h.read::<u16>(rd + 0xc2));
        let buffer = h.call_sym(MODEL_BUFFER_GET, Conv::Cdecl, &[index]);
        if buffer != 0 && h.read::<u32>(buffer + 8) != shape {
            let index = u32::from(h.read::<u16>(rd + 0xc2));
            h.call_sym(MODEL_BUFFER_RELEASE, Conv::Cdecl, &[index]);
            let new = h.call_sym(MODEL_BUFFER_ALLOCATE, Conv::Cdecl, &[shape]);
            #[allow(clippy::cast_possible_truncation)] // `ax`
            h.write(rd + 0xc2, new as u16);
        }

        // ---- the point light ----
        if h.read::<u8>(obj) & 0x40 != 0 && MAGIC_ID.get(h) == 0x207 {
            point_light(h, key, kind, obj);
        }

        // ---- YIQ ----
        if YIQ_MODE.get(h) != -1 && CURRENT_MAP.get(h) == 0x188 {
            if kind == 0 {
                enable_yiq(h, lo, hi);
            } else {
                for p in 0..4 {
                    let (plo, phi) = halves(part_key(key, p));
                    enable_yiq(h, plo, phi);
                }
            }
        }

        // ---- the fill request ----
        if dyn_geo_fill_buffer_is_valid(h) != 0 {
            #[allow(clippy::cast_sign_loss)] // pushed as a word
            let req = h.call_sym(FILL_REQUEST, Conv::Cdecl, &[total as u32]);
            h.write(req, 0u8);
            let pass = VFX_PASS.get(h);
            h.write(req + 0x108, pass);
            let flags: u32 = h.read(obj);
            h.write(req + 0x10, flags);
            let v: u32 = h.read(obj + 0x38);
            h.write(req + 0x14, v);
            let has_44 = h.read::<u32>(obj + 0x44) != 0;
            h.write(req + 1, u8::from(has_44));
            let v: u32 = h.read(part + 0x1c);
            h.write(req + 0x18, v);
            #[allow(clippy::cast_possible_truncation)] // `ax`
            h.write(req + 0x1c, texture as u16);
            let from: u32 = h.read(obj + 0x20);
            copy_words(h, req + 0x20, from, 16);
            let from: u32 = h.read(obj + 0x44);
            if from != 0 {
                copy_words(h, req + 0x60, from, 16);
            }
            h.write(req + 0xa0, uv);
            h.write(req + 0xa4, rgba);
            let b: u8 = h.read(obj + 8);
            h.write(req + 0x1e, b);
            for (i, c) in (0u32..).zip(color) {
                h.write(req + 0xb0 + 4 * i, c.to_bits());
            }
            let matrix = FILL_MATRIX.ptr(h).addr;
            copy_words(h, req + 0x1d4, matrix, 16);
            if kind == 0 {
                h.write(req + 0xc0, rd);
                h.write(req + 0xf0, geo0);
            } else {
                let g = |i: usize| geos.get(i).copied().unwrap_or(0);
                let r = |i: usize| rds.get(i).copied().unwrap_or(0);
                for (i, at) in (0..4).zip((0u32..).step_by(4)) {
                    h.write(req + 0xf0 + at, g(i));
                }
                for (i, at) in (0..4).zip((0u32..).step_by(4)) {
                    h.write(req + 0xc0 + at, r(i));
                }
                for base in [0xd0, 0xe0] {
                    for (i, at) in (0..4).zip((0u32..).step_by(4)) {
                        h.write(req + base + at, g(i));
                    }
                }
            }
            if h.read::<u8>(obj) & 0x40 != 0 {
                let v: u32 = h.read(obj + 0x3c);
                h.write(req + 0x100, v);
                for (to, field) in [(0x190, 0x24), (0x110, 0x2c), (0x150, 0x28)] {
                    let from: u32 = h.read(obj + field);
                    copy_words(h, req + to, from, 16);
                }
                let b: u8 = h.read(obj + 0x4d);
                h.write(req + 0x1d0, b);
            }
            h.write(req + 0x104, 0u32);
        }
        h.write(obj + 0x38, saved_38);
        h.write(obj + 0x3c, saved_3c);

        // ---- the segments' buffers ----
        let mut segment = 0u32;
        while total > 0 {
            if (segment + 1) * SEGMENT >= 0xca8 {
                return 0;
            }
            let n = i32::from(h.read::<i16>(header + 2));
            if let Some(t) = segment_type(i32::from(h.read::<i8>(header + 1))) {
                let (vertices, indices) = t.counts(n);
                let ok = if t.per_part && kind == 1 {
                    (0..4).all(|p| {
                        let r = rds.get(p).copied().unwrap_or(0);
                        let ok = h.call_sym(
                            BUFFER_FOR_SEGMENT,
                            Conv::Cdecl,
                            &[r, segment, vertices, indices],
                        ) != 0;
                        if ok {
                            let segs: u32 = h.read(r + 0x94);
                            h.write(segs + segment * SEGMENT + 4, 0u32);
                            let segs: u32 = h.read(r + 0x94);
                            h.write(segs + segment * SEGMENT + 8, 0u32);
                        }
                        ok
                    })
                } else if t.per_part && kind == 2 {
                    let mut p = 0usize;
                    let mut ok = true;
                    #[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)] // the counter
                    while ok && (p as i32) < i32::from(h.read::<u16>(rd + 0xc4)) {
                        let r = rds.get(p).copied().unwrap_or(0);
                        let segs: u32 = h.read(r + 0x94);
                        let at = segs + segment * SEGMENT;
                        let (a, b): (u32, u32) = (h.read(at + 4), h.read(at + 8));
                        ok = h.call_sym(BUFFER_FOR_SEGMENT, Conv::Cdecl, &[r, segment, a, b]) != 0;
                        if ok {
                            let segs: u32 = h.read(r + 0x94);
                            let at = segs + segment * SEGMENT;
                            let len = h.read::<u32>(at + 8).wrapping_mul(2);
                            let dst: u32 = h.read(at + 0x1c);
                            h.call_sym(MEMSET, Conv::Cdecl, &[dst, 0, len]);
                        }
                        p += 1;
                    }
                    ok
                } else {
                    h.call_sym(
                        BUFFER_FOR_SEGMENT,
                        Conv::Cdecl,
                        &[rd, segment, vertices, indices],
                    ) != 0
                };
                if !ok {
                    return 0;
                }
                #[allow(clippy::cast_sign_loss)] // the original's word arithmetic
                let advance = (n as u32).wrapping_mul(t.stride);
                cursor = cursor.wrapping_add(advance);
            }
            total -= i32::from(h.read::<i16>(header + 2));
            header = cursor;
            cursor = header.wrapping_add(0x10);
            segment += 1;
        }
        1
    }

    /// `g_pppFillMatrix2`, `g_pppFillMatrix3`: two more 16-word blocks a
    /// shape's fill request takes.
    pub const FILL_MATRIX_2: Global<u32> = Global::new(0x0230_fc20);
    /// See [`FILL_MATRIX_2`].
    pub const FILL_MATRIX_3: Global<u32> = Global::new(0x0230_fbe0);

    /// The part a shape primitive goes to, by its kind byte (`0x48` 1,
    /// `0x42` 2, `0x46` 3, `0x41` 4, `0x88` 5, else 0).
    #[must_use]
    pub fn shape_part(kind: u8) -> u32 {
        match kind {
            0x48 => 1,
            0x42 => 2,
            0x46 => 3,
            0x41 => 4,
            0x88 => 5,
            _ => 0,
        }
    }

    /// `updateVFXRenderDataShp(lo, hi, part, obj, segment)`: the render data
    /// of a shape effect. A few magic files force the object's colour
    /// (`obj + 4..7`) to `0x80` grey; on level 0x76f the shape's depth is
    /// pushed back; the shape's primitives are counted into their parts'
    /// first segments (6 indices and 4 vertices each), the job system gets
    /// a fill request, and every part the mask names gets its buffers.
    /// Returns 1, or 0 when a buffer could not be had.
    ///
    /// The original keeps the parts in a 17-word array of its frame (six
    /// geometries, the kind, six render data, ...); a part count above 6
    /// would read its other locals. Those parts are 0 here.
    #[allow(clippy::too_many_lines, clippy::similar_names, clippy::many_single_char_names)] // one original, in its order
    pub fn update_shp(
        h: &mut impl Host,
        lo: u32,
        hi: u32,
        part: u32,
        obj: u32,
        segment: u32,
    ) -> u32 {
        let magic = MAGIC_ID.get(h);
        if magic == 0x12e {
            if first_texture(h, part) == 0x3960 {
                grey(h, obj, true);
            }
        } else {
            if matches!(magic, 0x274 | 0x99 | 0x9a) && first_texture(h, part) == 0x3400 {
                grey(h, obj, false);
            }
            if matches!(magic, 0x259 | 0x98 | 0x215) && first_texture(h, part) == 0x39a0 {
                grey(h, obj, false);
            }
            if matches!(magic, 0x123 | 0x154) && first_texture(h, part) == 0x3800 {
                grey(h, obj, true);
            }
        }
        if SKIP_FRAME.get(h) != 0 {
            return 1;
        }
        if crate::scene::bridge::level_get_name_hash(h) == 0x76f && matches!(YIQ_MODE.get(h), 3..=5)
        {
            let p: u32 = h.read(obj + 0x10);
            let depth = f32::from_bits(h.read(p + 0x38));
            // `fcomp`: only a depth below 300 (not NaN)
            if 300.0 > depth {
                h.write(p + 0x38, (-10_000.0f32).to_bits());
            }
        }
        let key = u64::from(hi) << 32 | u64::from(lo);
        let geo0 = get_dynamic_geometry_by_instance(h, lo, hi);
        let rd0: u32 = if geo0 == 0 { 0 } else { h.read(geo0 + 0x24) };
        // the original reads it unchecked
        let kind = i32::from(h.read::<i8>(rd0 + 0xc0));
        // the frame's part arrays: geometries and render data
        let mut geos = [0u32; 6];
        let mut rds = [0u32; 6];
        geos[0] = geo0;
        rds[0] = rd0;
        let shape: u32 = h.read(part);
        let seg = shape.wrapping_add_signed(i32::from(
            h.read::<i16>(
                shape
                    .wrapping_add((segment & 0xffff) * 8)
                    .wrapping_add(0x10),
            ),
        ));
        let mut i = 0;
        while i < h.read::<i32>(rd0 + 0x90) {
            #[allow(clippy::cast_sign_loss)] // i is not negative
            let at = 0x6c * i as u32;
            let segs: u32 = h.read(rd0 + 0x94);
            h.write(segs + at + 8, 0u32);
            let segs: u32 = h.read(rd0 + 0x94);
            h.write(segs + at + 4, 0u32);
            i += 1;
        }
        if h.read::<u32>(obj + 0x1c) != 0 {
            h.write(rd0 + 0xb6, 1u8);
        }
        let entries = seg.wrapping_add(h.read(seg + 4)).wrapping_add(4);
        // after the parts loop the original keeps part 1 in two more words
        let (mut geo1, mut rd1) = (0u32, 0u32);
        if kind == 0 {
            let n = i32::from(h.read::<i16>(seg + 2));
            let segs: u32 = h.read(rd0 + 0x94);
            let v: u32 = h.read(segs + 8);
            #[allow(clippy::cast_sign_loss)] // the original's word arithmetic
            h.write(segs + 8, v.wrapping_add(n.wrapping_mul(6) as u32));
            let n = i32::from(h.read::<i16>(seg + 2));
            let segs: u32 = h.read(rd0 + 0x94);
            let v: u32 = h.read(segs + 4);
            #[allow(clippy::cast_sign_loss)]
            h.write(segs + 4, v.wrapping_add(n.wrapping_mul(4) as u32));
        } else {
            h.write(rd0 + 0xc8, 0u32);
            if h.read::<i16>(seg + 2) > 0 {
                let mut entry = entries;
                let mut done = 0;
                loop {
                    let p = shape_part(h.read(entry));
                    let pi = p as usize;
                    if rds[pi] == 0 {
                        #[allow(clippy::cast_possible_wrap)] // p is at most 5
                        let (plo, phi) = halves(part_key(key, p as i32));
                        let g = get_dynamic_geometry_by_instance(h, plo, phi);
                        geos[pi] = g;
                        let r: u32 = if g == 0 { 0 } else { h.read(g + 0x24) };
                        let segs: u32 = h.read(r + 0x94);
                        rds[pi] = r;
                        h.write(segs + 8, 0u32);
                        let segs: u32 = h.read(r + 0x94);
                        h.write(segs + 4, 0u32);
                    }
                    let r = rds[pi];
                    let segs: u32 = h.read(r + 0x94);
                    let v: u32 = h.read(segs + 8);
                    h.write(segs + 8, v.wrapping_add(6));
                    let segs: u32 = h.read(r + 0x94);
                    let v: u32 = h.read(segs + 4);
                    h.write(segs + 4, v.wrapping_add(4));
                    let mask: u32 = h.read(rd0 + 0xc8);
                    h.write(rd0 + 0xc8, mask | 1 << p);
                    done += 1;
                    entry += 8;
                    if done >= i32::from(h.read::<i16>(seg + 2)) {
                        break;
                    }
                }
                geo1 = geos[1];
                rd1 = rds[1];
            }
        }
        if dyn_geo_fill_buffer_is_valid(h) != 0 {
            let n = i32::from(h.read::<i16>(seg + 2));
            #[allow(clippy::cast_sign_loss)] // pushed as a word
            let req = h.call_sym(FILL_REQUEST, Conv::Cdecl, &[n as u32]);
            h.write(req, 1u8);
            h.write(req + 0x10, seg);
            let has_1c = h.read::<u32>(obj + 0x1c) != 0;
            h.write(req + 1, u8::from(has_1c));
            let has_1c = h.read::<u8>(req + 1) != 0;
            let colour = u32::from(h.read::<u16>(obj + 2));
            h.write(req + 0x14, geo0);
            h.write(req + 0x18, geo1);
            for (i, at) in (2..6).zip((0x1c..).step_by(4)) {
                h.write(req + at, geos[i]);
            }
            h.write(req + 0x2c, rd0);
            h.write(req + 0x30, rd1);
            for (i, at) in (2..5).zip((0x34..).step_by(4)) {
                h.write(req + at, rds[i]);
            }
            h.write(req + 0x44, colour);
            h.write(req + 0x40, rds[5]);
            let from: u32 = h.read(obj + 0x10);
            copy_words(h, req + 0x48, from, 16);
            if has_1c {
                let from: u32 = h.read(obj + 0x1c);
                copy_words(h, req + 0x88, from, 16);
            }
            let v: u32 = h.read(obj + 4);
            h.write(req + 0xc8, v);
            h.write(rd0 + 0xc1, 1u8);
            #[allow(clippy::cast_sign_loss)] // stored as a word
            h.write(req + 0xcc, kind as u32);
            for (to, g) in [
                (0xd0, FILL_MATRIX),
                (0x110, FILL_MATRIX_2),
                (0x150, FILL_MATRIX_3),
            ] {
                let from = g.ptr(h).addr;
                copy_words(h, req + to, from, 16);
            }
        }
        let mut p = 0u32;
        while p < u32::from(h.read::<u16>(rd0 + 0xc4)) {
            if h.read::<u32>(rd0 + 0xc8) & 1u32.wrapping_shl(p) != 0 {
                let r = rds.get(p as usize).copied().unwrap_or(0);
                let mut s = 0u32;
                #[allow(clippy::cast_possible_wrap)] // the counter
                while (s as i32) < h.read::<i32>(r + 0x90) {
                    let segs: u32 = h.read(r + 0x94);
                    let at = segs + 0x6c * s;
                    let (a, b): (u32, u32) = (h.read(at + 4), h.read(at + 8));
                    if h.call_sym(BUFFER_FOR_SEGMENT, Conv::Cdecl, &[r, s, a, b]) == 0 {
                        return 0;
                    }
                    s += 1;
                }
            }
            p += 1;
        }
        1
    }

    /// The texture word of a shape's first segment (`& 0x3fff`).
    fn first_texture(h: &impl Mem, part: u32) -> u32 {
        let shape: u32 = h.read(part);
        let first = shape.wrapping_add_signed(i32::from(h.read::<i16>(shape + 0x10)));
        h.read::<u32>(first + 0x10) & 0x3fff
    }

    /// The object's colour to `0x80` grey (`obj + 4..7`), in either of the
    /// original's store orders.
    fn grey(h: &mut impl Mem, obj: u32, byte_first: bool) {
        if byte_first {
            h.write(obj + 6, 0x80u8);
            h.write(obj + 4, 0x8080u16);
        } else {
            h.write(obj + 4, 0x8080u16);
            h.write(obj + 6, 0x80u8);
        }
    }

    /// `g_AlphaScale`'s two values (`f32` bits).
    fn alpha(doubled: bool) -> u32 {
        if doubled { 512.0f32 } else { 256.0f32 }.to_bits()
    }

    fn halves(key: u64) -> (u32, u32) {
        #[allow(clippy::cast_possible_truncation)] // the halves
        (key as u32, (key >> 32) as u32)
    }

    /// `rep movsd`: `n` words from `from` to `to`, in order.
    fn copy_words(h: &mut impl Mem, to: u32, from: u32, n: u32) {
        for i in 0..n {
            let v: u32 = h.read(from + 4 * i);
            h.write(to + 4 * i, v);
        }
    }

    /// The point light of a magic file 0x207 part (`obj[0] & 0x40`): the
    /// light's matrix (`*(obj + 0x28)`) taken into the camera's frame
    /// (the inverse of `*(obj + 0x2c)`), handed with the colour
    /// (`obj + 0x24`) and the power (`obj[0x4d] / 255`) to every part.
    fn point_light(h: &mut impl Host, key: u64, kind: i32, obj: u32) {
        let m = h.scratch();
        let inv = m + 0x40;
        let src: u32 = h.read(obj + 0x28);
        copy_words(h, m, src, 16);
        let camera: u32 = h.read(obj + 0x2c);
        h.call_sym(INVERSE_MATRIX, Conv::Cdecl, &[inv, camera]);
        for row in 0..4 {
            let r = m + 0x10 * row;
            h.write(r + 0xc, 1.0f32.to_bits());
            h.call_sym(APPLY_MATRIX, Conv::Cdecl, &[r, inv, r]);
            let src: u32 = h.read(obj + 0x28);
            let w = x87::round_trip(f32::from_bits(h.read(src + 0x10 * row + 0xc)));
            h.write(r + 0xc, w.to_bits());
        }
        let parts = if kind == 0 { 1 } else { 4 };
        for p in 0..parts {
            let b = f64::from(h.read::<u8>(obj + 0x4d));
            let power = f32::from_bits(x87::store(x87::div(b, 255.0)));
            let color: u32 = h.read(obj + 0x24);
            let (plo, phi) = halves(part_key(key, p));
            set_parameters(
                h,
                crate::vfx::bridge::SET_POINT_LIGHT,
                plo,
                phi,
                m,
                color,
                power,
            );
        }
    }
}

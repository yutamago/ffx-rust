//! `rcbg` scene objects: models and their primitive packets, layers,
//! texture transfers (`trx`), palettes, animations and particle
//! simulations (`docs/formats/field.md`).

use ffx_bridge::Global;

/// Bytes per primitive record of each packet type (`g_GuidePacketStrides`,
/// 8 bytes, in `.data`).
pub const PACKET_STRIDES: Global<u8> = Global::new(0x00c5_e790);

/// A model (`rcbgModel`).
pub mod obj {
    /// `+0x14`: its OMD blob (`rcbgModelCreateFromOMD`).
    pub const OMD: u32 = 0x14;
    /// `+0xc0`: the vertex-to-world matrix (`Guide_MeshObject`'s users).
    pub const MATRIX: u32 = 0xc0;
}

/// A model's OMD blob: primitive packets and vertices.
pub mod omd {
    /// `+0x4`: offset of the first packet (`rcbgModelGetPrimHeader`).
    pub const PACKETS: u32 = 0x4;
    /// `+0x8`: offset of the vertices (three `i16` each).
    pub const VERTICES: u32 = 0x8;
    /// `+0x10`: primitives in all packets (`i16`, `rcbgModelSetAlpha`).
    pub const PRIMS: u32 = 0x10;
    /// `+0x12`: vertex count (`u16`).
    pub const VERTEX_COUNT: u32 = 0x12;
    /// `+0x20`: where `rcbgModelSetAlpha` starts walking the packets (it
    /// does not read [`PACKETS`]).
    pub const FIRST_PACKET: u32 = 0x20;
}

/// A primitive packet: a 16-byte header, then `count` records of the
/// type's stride ([`super::PACKET_STRIDES`]).
pub mod packet {
    /// `+0x1`: the type (`i8`, 0..=7). Types 0..=3 have three vertices, 4..=7
    /// four (`rcbgModelSetAlpha`); type 0 records keep their three vertex
    /// indices at `+0xc` (the minimap).
    pub const TYPE: u32 = 0x1;
    /// `+0x2`: records in the packet (`i16`).
    pub const COUNT: u32 = 0x2;
    /// Header size.
    pub const HEADER: u32 = 0x10;
    /// A record starts with one RGBA colour per vertex; the alpha bytes.
    pub const ALPHAS: [u32; 4] = [0x3, 0x7, 0xb, 0xf];
}

/// Bytes a packet takes, header included: `0x10 + stride * count`, with
/// the original's signed arithmetic.
#[must_use]
pub fn packet_size(stride: u8, count: i16) -> u32 {
    #[allow(clippy::cast_sign_loss)] // two's complement, as the original adds it
    let records = (i32::from(stride) * i32::from(count)) as u32;
    packet::HEADER.wrapping_add(records)
}

/// A layer (`rcbgLayerLoad`).
pub mod layer {
    /// `+0x4`: width (`i32`).
    pub const WIDTH: u32 = 0x4;
    /// `+0x8`: height (`i32`).
    pub const HEIGHT: u32 = 0x8;
    /// Where `rcbgLayerLoadCalcCenter` stores half the width.
    pub const CENTER_X: [u32; 3] = [0x68, 0x64, 0x4c];
    /// Where it stores half the height.
    pub const CENTER_Y: [u32; 3] = [0x70, 0x6c, 0x50];
}

/// Half of a layer dimension, as the original computes it: the `i32`
/// rounded to `f32`, then halved.
#[must_use]
#[allow(clippy::cast_precision_loss)] // the original rounds the same way
pub fn half(n: i32) -> u32 {
    crate::x87::store(f64::from(n as f32) * 0.5)
}

/// A texture transfer object (`rcbgTrxInit`, `0x40` bytes).
pub mod trx {
    /// `+0x0`: the object type.
    pub const KIND: u32 = 0x0;
    /// The type `rcbgTrxInit` gives it.
    pub const KIND_TRX: u32 = 0x0002_0002;
    /// `+0x4`: the delete function (`rcbgTrxDelete`, `0x0092d450`).
    pub const DELETE: u32 = 0x4;
    /// `+0xc`: the draw function (`rcbgTrxDraw`, `0x0092d470`).
    pub const DRAW: u32 = 0xc;
    /// `+0x30`: entries (`rcbgTrxGetCnt`).
    pub const COUNT: u32 = 0x30;
    /// `+0x34`: bytes per entry.
    pub const ENTRY_SIZE: u32 = 0x34;
    /// `+0x38`: the entries (`rcbgTrxGetEntry`); an entry's data follows
    /// its 16-byte header (`rcbgTrxGetEntryData`).
    pub const ENTRIES: u32 = 0x38;
    /// `+0x3c`: the transfer to run on the next draw; -1 for none
    /// (`rcbgTrxSetNo`, cleared by `rcbgTrxDraw`).
    pub const NO: u32 = 0x3c;
    /// The original's delete function.
    pub const DELETE_FN: u32 = 0x0092_d450;
    /// The original's draw function.
    pub const DRAW_FN: u32 = 0x0092_d470;
    /// Words `rcbgTrxInit` clears (`+0x14` it leaves).
    pub const CLEARED: [u32; 11] = [
        0x8, 0x10, 0x18, 0x1c, 0x20, 0x24, 0x28, 0x2c, 0x30, 0x34, 0x38,
    ];
}

/// A palette (`rcbgPalInit`, 16 bytes).
pub mod pal {
    /// `+0x8`: the palette number (`u16`, `rcbgPalSetNo`).
    pub const NO: u32 = 0x8;
}

/// An animation (`rcbgAnimeInit`, 32 bytes).
pub mod anime {
    /// `+0x6`: the current frame (`i16`).
    pub const CURRENT: u32 = 0x6;
    /// `+0x8`: frames (`i32`).
    pub const COUNT: u32 = 0x8;
    /// `+0xc`: the frame entries, 4 bytes each.
    pub const FRAMES: u32 = 0xc;
    /// `+0x14`: set to `0x100` by `rcbgAnimeInit` (the rest is cleared).
    pub const RATE: u32 = 0x14;
}

/// A particle simulation (`rcbgPSimInit`, `0xb0` bytes).
pub mod psim {
    /// Size.
    pub const SIZE: u32 = 0xb0;
    /// The object type `rcbgPSimInit` gives it.
    pub const KIND_PSIM: u32 = 0x0002_0004;
    /// Its delete, process and draw functions (`+4`, `+8`, `+0xc`; the
    /// second is `rcbgPSimProc`).
    pub const FUNCTIONS: [u32; 3] = [0x0091_fd40, 0x0091_fe70, 0x0091_fd60];
    /// Where its model's blob keeps the input particles (`rcbgPSimGetOmdIn`).
    pub const OMD_IN: u32 = 0x20;
    /// `+0x14`: its model.
    pub const MODEL: u32 = 0x14;
    /// `+0xa4`: the initial count (`rcbgPSimSetInitCnt`).
    pub const INIT_CNT: u32 = 0xa4;
    /// `+0x4c`: a position (`rcbgPSimGetPos` 0).
    pub const POS: u32 = 0x4c;
    /// `+0x8c`: the other position (`rcbgPSimGetPos` 1).
    pub const POS2: u32 = 0x8c;
}

/// `rcFree` (`0x009258e0`): a jump to `user_free`; the `rcbg*Fini`
/// functions free through it.
pub const RC_FREE: u32 = 0x0092_58e0;
/// The stream word `rcefSetStream` sets (`0x025d5c14`).
pub const REF_STREAM: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x025d_5c14);
/// Set to 3 with a stream (`0x025d5be0`).
pub const REF_STREAM_MODE: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x025d_5be0);
/// 0.03 (`f64`, `.rdata`): `rcfxParticleInit`'s `+8`.
pub const PARTICLE_RATE: u32 = 0x00b5_4cc0;
/// 0.001 and 0.1 (`f32`s, `.rdata`): `rcfxParticleInit`'s `+0x18` and
/// `+0x20`.
pub const PARTICLE_DEFAULTS: [u32; 2] = [0x00b3_68a4, 0x00b1_0f38];
/// The GIF tag word `rcefSetupDraw` copies to `+0x14`.
pub const REF_GIF_TAG: u32 = 0x00c4_98dc;
/// The two buffers `rcefSetupDraw` points `+0x34` and `+0x38` at.
pub const REF_BUFFERS: [u32; 2] = [0x0230_fd60, 0x0230_fda0];
/// `rcfxNail`'s callbacks (`rcfxNailInit`: `+4`, `+0x10`, `+0x14`).
pub const NAIL_CALLBACKS: [u32; 3] = [0x0092_9070, 0x0092_9190, 0x0092_8f40];
/// `rcfxWind`'s callbacks (`rcfxWindInit`: `+4`, `+0x14`).
pub const WIND_CALLBACKS: [u32; 2] = [0x0092_9840, 0x0092_96d0];
/// `rcfxGravity`'s update callback (`0x00929560`).
pub const GRAVITY_UPDATE: u32 = 0x0092_9560;
/// `rcfxGravity`'s draw callback (`0x00929500`).
pub const GRAVITY_DRAW: u32 = 0x0092_9500;
/// The chunk tags `rcbgChkId` looks up: a null-terminated table of
/// pointers to 4-character ids.
pub const CHUNK_IDS: u32 = 0x00c5_e480;
/// The GS pixel format per TIM2 image type (`rcdtTm2GetPsm`).
pub const TM2_PSM: u32 = 0x00c5_e754;
/// The draw function `rcbgSceneDrawLayer` queues.
pub const DRAW_LAYER_FUNC: u32 = 0x0092_0850;

/// The per-map scene parts of `rcbgFP.c` (`rcbgFPm*Init`) and `rcfxDamp`.
pub mod fp {
    /// The parts' object type.
    pub const KIND: u32 = 0x0002_0008;
    /// `rcbgFPm097Init`'s functions.
    pub const M097: [u32; 2] = [0x0092_2540, 0x0092_25d0];
    /// `rcbgFPm173Init`'s functions.
    pub const M173: [u32; 2] = [0x0092_2b40, 0x0092_2be0];
    /// `rcbgFPm565Init`'s functions.
    pub const M565: [u32; 2] = [0x0092_2f30, 0x0092_2fb0];
    /// `rcfxDampInit`'s functions (`+4`, `+0x14`).
    pub const DAMP_FUNCTIONS: [u32; 2] = [0x0092_93c0, 0x0092_9360];
    /// Its factor, an `f32` in `.rdata`.
    pub const DAMP_FACTOR: u32 = 0x00b9_2154;
}

/// A model's scale (`rcbgModelSetScale` stores ten times it on the
/// matrix diagonal; `rcbgModelGetScale` divides by 10).
pub const SCALE_FACTOR: f64 = 10.0;

/// `rcPad16(n)`: bytes from `n` to the next multiple of 16, with the C
/// remainder (a negative `n` gives 17..=31).
#[must_use]
pub const fn pad16(n: i32) -> i32 {
    let r = n % 16;
    if r != 0 { 16 - r } else { 0 }
}

/// `rcLog2Ceil(n)`: the smallest `k` in 1..=16 with `n <= 1 << k`
/// (16 when none is).
#[must_use]
pub fn log2_ceil(n: i32) -> u32 {
    let mut k = 1u32;
    let mut p = 2u32;
    loop {
        if n <= p.cast_signed() {
            return k;
        }
        p = p.rotate_left(1);
        k += 1;
        if k >= 16 {
            return k;
        }
    }
}

/// The scene objects where the game keeps them, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::Mem;
    use ffx_bridge::x87::Reg;
    use ffx_system::stub::ptr32;

    use super::{
        PACKET_STRIDES, anime, half, layer, obj, omd, packet, packet_size, pal, psim, trx,
    };

    fn stride(mem: &impl Mem, kind: i8) -> u8 {
        #[allow(clippy::cast_sign_loss)]
        // a negative type indexes before the table, as the original
        let at = PACKET_STRIDES
            .ptr(mem)
            .addr
            .wrapping_add(i32::from(kind) as u32);
        mem.read(at)
    }

    /// `rcbgModelCreateFromOMD(model, omd)` (`0x00927d70`).
    pub fn rcbg_model_create_from_omd(mem: &mut impl Mem, m: u32, blob: u32) {
        mem.write(m + obj::OMD, blob);
    }

    /// `rcbgModelGetPrimHeader(model, n)` (`0x009282c0`): the header of
    /// packet `n` (0 or negative: the first).
    pub fn rcbg_model_get_prim_header(mem: &impl Mem, m: u32, n: i32) -> u32 {
        let blob = mem.read::<u32>(m + obj::OMD);
        let mut p = blob.wrapping_add(mem.read::<u32>(blob + omd::PACKETS));
        for _ in 0..n.max(0) {
            let s = stride(mem, mem.read(p + packet::TYPE));
            p = p.wrapping_add(packet_size(s, mem.read(p + packet::COUNT)));
        }
        p
    }

    /// `rcbgModelGetPrim(model, n)` (`0x009282a0`): packet `n`'s records.
    pub fn rcbg_model_get_prim(mem: &impl Mem, m: u32, n: i32) -> u32 {
        rcbg_model_get_prim_header(mem, m, n).wrapping_add(packet::HEADER)
    }

    /// `rcbgModelSetAlpha(model, alpha)` (`0x00928460`): every vertex
    /// colour's alpha, packet by packet until the blob's primitive count is
    /// used up.
    pub fn rcbg_model_set_alpha(mem: &mut impl Mem, m: u32, alpha: u8) {
        let blob = mem.read::<u32>(m + obj::OMD);
        let mut left = i32::from(mem.read::<i16>(blob + omd::PRIMS));
        let mut p = blob + omd::FIRST_PACKET;
        while left > 0 {
            let kind = mem.read::<i8>(p + packet::TYPE);
            let count = i32::from(mem.read::<i16>(p + packet::COUNT));
            let mut r = p.wrapping_add(packet::HEADER);
            for _ in 0..count.max(0) {
                let corners = if kind >= 4 { 4 } else { 3 };
                for &o in &packet::ALPHAS[..corners] {
                    mem.write(r.wrapping_add(o), alpha);
                }
                r = r.wrapping_add(u32::from(stride(mem, kind)));
            }
            left -= count;
            p = r;
        }
    }

    /// `rcbgLayerLoadCalcCenter(layer)` (`0x0092b270`): half the width and
    /// height, each stored three times.
    pub fn rcbg_layer_load_calc_center(mem: &mut impl Mem, l: u32) {
        let x = half(mem.read(l + layer::WIDTH));
        let y = half(mem.read(l + layer::HEIGHT));
        for o in layer::CENTER_X {
            mem.write(l + o, x);
        }
        for o in layer::CENTER_Y {
            mem.write(l + o, y);
        }
    }

    /// `rcbgTrxInit(trx)` (`0x0092d510`).
    pub fn rcbg_trx_init(mem: &mut impl Mem, t: u32) {
        mem.write(t + trx::KIND, trx::KIND_TRX);
        let delete = mem.rebase(trx::DELETE_FN);
        let draw = mem.rebase(trx::DRAW_FN);
        mem.write(t + trx::DELETE, delete);
        mem.write(t + trx::DRAW, draw);
        for o in trx::CLEARED {
            mem.write(t + o, 0u32);
        }
        mem.write(t + trx::NO, u32::MAX);
    }

    /// `rcbgTrxSetNo(trx, no)` (`0x0092d670`).
    pub fn rcbg_trx_set_no(mem: &mut impl Mem, t: u32, no: u32) {
        mem.write(t + trx::NO, no);
    }

    /// `rcbgPalInit(pal)` (`0x009299b0`).
    pub fn rcbg_pal_init(mem: &mut impl Mem, p: u32) {
        mem.write(p, [0u32; 4]);
    }

    /// `rcbgPalSetNo(pal, no)` (`0x00929ab0`).
    pub fn rcbg_pal_set_no(mem: &mut impl Mem, p: u32, no: u16) {
        mem.write(p + pal::NO, no);
    }

    /// `rcbgAnimeInit(anime)` (`0x009308a0`).
    pub fn rcbg_anime_init(mem: &mut impl Mem, a: u32) {
        mem.write(a, [0u32; 8]);
        mem.write(a + anime::RATE, 0x100u32);
    }

    /// `rcbgPSimSetInitCnt(psim, n)` (`0x0091fef0`).
    pub fn rcbg_psim_set_init_cnt(mem: &mut impl Mem, p: u32, n: u32) {
        mem.write(p + psim::INIT_CNT, n);
    }

    /// `rcbgModelGetScale(model)` (`0x00928300`): the matrix's first
    /// element over 10.
    pub fn rcbg_model_get_scale(mem: &impl Mem, m: u32) -> f32 {
        let x = crate::x87::load(mem.read(m + obj::MATRIX));
        f32::from_bits(crate::x87::store(x / super::SCALE_FACTOR))
    }

    /// `rcbgTrxGetCnt(trx)` (`0x0092d4c0`).
    pub fn rcbg_trx_get_cnt(mem: &impl Mem, t: u32) -> u32 {
        mem.read(t + trx::COUNT)
    }

    /// `rcbgTrxGetEntry(trx, n)` (`0x0092d4d0`).
    pub fn rcbg_trx_get_entry(mem: &impl Mem, t: u32, n: u32) -> u32 {
        let size = mem.read::<u32>(t + trx::ENTRY_SIZE);
        size.wrapping_mul(n)
            .wrapping_add(mem.read(t + trx::ENTRIES))
    }

    /// `rcbgTrxGetEntryData(trx, n)` (`0x0092d4f0`).
    pub fn rcbg_trx_get_entry_data(mem: &impl Mem, t: u32, n: u32) -> u32 {
        rcbg_trx_get_entry(mem, t, n).wrapping_add(0x10)
    }

    /// `rcbgPSimInit(psim)` (`0x0091fe00`).
    pub fn rcbg_psim_init(mem: &mut impl Mem, p: u32) {
        mem.write_bytes(p, &[0; psim::SIZE as usize]);
        mem.write(p, psim::KIND_PSIM);
        for (i, f) in (1u32..).zip(psim::FUNCTIONS) {
            let at = mem.rebase(f);
            mem.write(p + 4 * i, at);
        }
    }

    /// `rcbgPSimGetOmdIn(psim)` (`0x0091fdc0`).
    pub fn rcbg_psim_get_omd_in(mem: &impl Mem, p: u32) -> u32 {
        mem.read::<u32>(p + psim::MODEL).wrapping_add(psim::OMD_IN)
    }

    /// `rcbgFPm173End(fp)` (`0x00922b50`): `+0x24` = 1.
    pub fn rcbg_fpm173_end(mem: &mut impl Mem, f: u32) {
        mem.write(f + 0x24, 1u32);
    }

    /// `rcbgAnimeGetFrame(anime, n)` (`0x00930860`): frame `n`'s entry (4
    /// bytes from `+0xc`), or null when `n` is not below the count at `+8`.
    pub fn rcbg_anime_get_frame(mem: &impl Mem, a: u32, n: i32) -> u32 {
        if n >= mem.read::<i32>(a + anime::COUNT) {
            return 0;
        }
        #[allow(clippy::cast_sign_loss)] // negative indexes back, as the original
        mem.read::<u32>(a + anime::FRAMES)
            .wrapping_add((n as u32).wrapping_mul(4))
    }

    /// `rcbgAnimeGetCurrent(anime)` (`0x00930880`): the entry of the frame
    /// at `+6` (`i16`).
    pub fn rcbg_anime_get_current(mem: &impl Mem, a: u32) -> u32 {
        let n = i32::from(mem.read::<i16>(a + anime::CURRENT));
        #[allow(clippy::cast_sign_loss)]
        mem.read::<u32>(a + anime::FRAMES)
            .wrapping_add((n as u32).wrapping_mul(4))
    }

    /// `rcbgPSimClearOMD(psim)` (`0x0091f9b0`): detaches its model's blob.
    pub fn rcbg_psim_clear_omd(mem: &mut impl Mem, p: u32) {
        let m = mem.read::<u32>(p + psim::MODEL);
        if m != 0 {
            mem.write(m + obj::OMD, 0u32);
        }
    }

    // ---- round 4: particle positions, scene palettes, FP parts, TIM2 ----

    /// `rcbgPSimGetPos(psim, which, out)` (`0x0091fdd0`): the position at
    /// `+0x4c` (`which` 0) or `+0x8c`; returns `FFXVu0CopyVector`'s `eax`.
    pub fn rcbg_psim_get_pos(mem: &mut impl Mem, p: u32, which: u32, out: u32) -> u32 {
        let at = p.wrapping_add(if which == 0 { psim::POS } else { psim::POS2 });
        ffx_ps2::vu0::bridge::copy_vector(mem, out, at)
    }

    /// `rcbgSceneSetPal(scene)` (`0x009210a0`): every layer's palette back
    /// to number 0 (`rcbgPalSetNo` on `+0xb0` of each `0xc0`-byte layer).
    pub fn rcbg_scene_set_pal(mem: &mut impl Mem, s: u32) {
        let layers = mem.read::<u32>(s.wrapping_add(8));
        let mut i = 0i32;
        while i < mem.read::<i32>(s.wrapping_add(4)) {
            #[allow(clippy::cast_sign_loss)] // i >= 0
            let l = layers.wrapping_add(i as u32 * 0xc0);
            rcbg_pal_set_no(mem, l.wrapping_add(0xb0), 0);
            i += 1;
        }
    }

    /// `rcbgChkId(id)` (`0x00921940`): the index of the chunk tag that
    /// `strncmp(id, tag, 4)` matches in the null-terminated table at
    /// `0x00c5e480`, or -1.
    pub fn rcbg_chk_id(mem: &impl Mem, id: u32) -> i32 {
        let table = mem.rebase(super::CHUNK_IDS);
        let mut i = 0u32;
        loop {
            let tag = mem.read::<u32>(table + 4 * i);
            if tag == 0 {
                return -1;
            }
            if strncmp4(mem, id, tag) {
                #[allow(clippy::cast_possible_wrap)]
                return i as i32;
            }
            i += 1;
        }
    }

    /// Whether `strncmp(a, b, 4)` is 0.
    fn strncmp4(mem: &impl Mem, a: u32, b: u32) -> bool {
        for k in 0..4 {
            let (x, y) = (mem.read::<u8>(a + k), mem.read::<u8>(b + k));
            if x != y {
                return false;
            }
            if x == 0 {
                return true;
            }
        }
        true
    }

    /// Writes an FP part's header: type `0x20008`, its functions (rebased),
    /// and 0.
    fn fp_header(mem: &mut impl Mem, p: u32, functions: [u32; 2]) {
        mem.write(p, super::fp::KIND);
        let [a, b] = functions.map(|f| mem.rebase(f));
        mem.write(p.wrapping_add(4), a);
        mem.write(p.wrapping_add(8), b);
        mem.write(p.wrapping_add(0xc), 0u32);
    }

    /// `rcbgFPm097Init(part)` (`0x00922560`, map 0x61's part): cleared (64
    /// bytes, `memset`), then the header; returns the part.
    pub fn rcbg_fpm097_init(mem: &mut impl Mem, p: u32) -> u32 {
        mem.write(p, [0u32; 16]);
        fp_header(mem, p, super::fp::M097);
        p
    }

    /// `rcbgFPm173Init(part)` (`0x00922b70`, map 0xad's part): `+0x10..+0x28`
    /// cleared, the header; returns the part.
    pub fn rcbg_fpm173_init(mem: &mut impl Mem, p: u32) -> u32 {
        mem.write(p.wrapping_add(0x10), [0u32; 6]);
        fp_header(mem, p, super::fp::M173);
        p
    }

    /// `rcbgFPm565Init(part)` (`0x00922f50`, maps 0x235 and 0x236's part):
    /// the header; returns the part.
    pub fn rcbg_fpm565_init(mem: &mut impl Mem, p: u32) -> u32 {
        fp_header(mem, p, super::fp::M565);
        p
    }

    /// `rcfxDampInit(damp)` (`0x009293d0`, `rcfxDamp.c`): type `0x30003`,
    /// its two functions, the factor at `+0x1c` (an `f32` in `.rdata`,
    /// 0.0001); returns the object.
    pub fn rcfx_damp_init(mem: &mut impl Mem, p: u32) -> u32 {
        let k = mem.read::<u32>(mem.rebase(super::fp::DAMP_FACTOR));
        mem.write(p.wrapping_add(0x1c), crate::x87::copy(k));
        mem.write(p, 0x0003_0003u32);
        let a = mem.rebase(super::fp::DAMP_FUNCTIONS[0]);
        mem.write(p.wrapping_add(4), a);
        mem.write(p.wrapping_add(8), [0u32; 3]);
        let b = mem.rebase(super::fp::DAMP_FUNCTIONS[1]);
        mem.write(p.wrapping_add(0x14), b);
        mem.write(p.wrapping_add(0x18), 0u32);
        p
    }

    /// `rcbgFpGetWork(id, part, index)` (`0x00923100`): through the VFX
    /// run record of `id` (`pppFpGetMngPtr`, `run_of`), its data's part
    /// table: `index`'s work offset plus `0xa0` from the data; 0 when a
    /// pointer on the way is null.
    pub fn rcbg_fp_get_work<M: Mem>(
        mem: &mut M,
        id: u32,
        part: u32,
        index: u32,
        run_of: impl FnOnce(&mut M, u32) -> u32,
    ) -> u32 {
        let run = run_of(mem, id);
        if run == 0 {
            return 0;
        }
        let data = ptr32(mem.read::<u32>(run.wrapping_add(0x30)));
        if data == 0 {
            return 0;
        }
        let parts = ptr32(mem.read::<u32>(data.wrapping_add(8)));
        if parts == 0 {
            return 0;
        }
        let table = ptr32(mem.read::<u32>(parts));
        if table == 0 {
            return 0;
        }
        let entry = (part << 4).wrapping_add(0x28).wrapping_add(table);
        if entry == 0 {
            return 0;
        }
        let works = ptr32(mem.read::<u32>(entry.wrapping_add(0xc)));
        mem.read::<u32>(works.wrapping_add(index.wrapping_mul(4)))
            .wrapping_add(0xa0)
            .wrapping_add(data)
    }

    /// `rcbgSceneDrawLayer(layer)` (`0x00920830`):
    /// `op_func_after_ot3d_set(0x00920850, layer, 0, 1)` (`after_set` with
    /// the rebased function and the layer); returns its result.
    pub fn rcbg_scene_draw_layer<M: Mem>(
        mem: &mut M,
        layer: u32,
        after_set: impl FnOnce(&mut M, u32, u32) -> u32,
    ) -> u32 {
        let f = mem.rebase(super::DRAW_LAYER_FUNC);
        after_set(mem, f, layer)
    }

    /// `rcbgSceneWaveDCStop(scene)` (`0x00920650`): the wave
    /// `Yn_SceneSetDC` starts ends (`+0xac` 0.0, `+0xdc` 0).
    pub fn rcbg_scene_wave_dc_stop(mem: &mut impl Mem, s: u32) {
        mem.write(s.wrapping_add(0xac), 0f32);
        mem.write(s.wrapping_add(0xdc), 0u32);
    }

    /// `rcbgSceneWaveC0Stop(scene)` (`0x00921200`): the wave
    /// `Yn_SceneSetC0` starts ends (`+0xbc`, `+0xb8` 0.0, `+0xc0` 0).
    pub fn rcbg_scene_wave_c0_stop(mem: &mut impl Mem, s: u32) {
        mem.write(s.wrapping_add(0xbc), 0f32);
        mem.write(s.wrapping_add(0xb8), 0f32);
        mem.write(s.wrapping_add(0xc0), 0u32);
    }

    /// `rcfxParticleClearForces(p)` (`0x009289b0`, from
    /// `rcfxParticleResetForce`): each of the `+0x1c` particles' force
    /// (16 bytes in the array at `+0x2c`) zeroed.
    pub fn rcfx_particle_clear_forces(mem: &mut impl Mem, p: u32) {
        let mut i = 0i32;
        while i < mem.read::<i32>(p.wrapping_add(0x1c)) {
            #[allow(clippy::cast_sign_loss)] // i >= 0
            let off = i as u32 * 0x10;
            for k in [0xc, 8, 4, 0] {
                let a = mem.read::<u32>(p.wrapping_add(0x2c));
                mem.write(a.wrapping_add(off + k), 0f32);
            }
            i += 1;
        }
    }

    /// `rcdtTm2GetPsm(picture)` (`0x00923280`): the GS pixel format of a
    /// TIM2 picture header's image type (`+0x13`) from the table at
    /// `0x00c5e754` (1 → PSMCT16, 2 → PSMCT24, 3 → PSMCT32, 4 → PSMT4,
    /// 5 → PSMT8); 0 for type 0.
    pub fn rcdt_tm2_get_psm(mem: &impl Mem, pic: u32) -> u32 {
        match mem.read::<u8>(pic.wrapping_add(0x13)) {
            0 => 0,
            t => mem.read(mem.rebase(super::TM2_PSM) + 4 * u32::from(t)),
        }
    }

    /// `rcdtTm2ClutAlphaToPs2(clut)` (`0x00923240`): 256 RGBA entries'
    /// alpha from 0..=255 to the GS's 0..=128 (`a * 128 / 255`, rounded
    /// down); an entry with alpha 0 becomes all 0.
    pub fn rcdt_tm2_clut_alpha_to_ps2(mem: &mut impl Mem, clut: u32) {
        for i in 0..256 {
            let at = clut.wrapping_add(4 * i);
            let a = u32::from(mem.read::<u8>(at.wrapping_add(3)));
            if a == 0 {
                mem.write(at, 0u32);
            } else {
                mem.write(at.wrapping_add(3), 0u8);
                let w = mem.read::<u32>(at);
                mem.write(at, w | ((a * 128 / 255) << 24));
            }
        }
    }

    /// `rcdtRsd2Init(rsd)` (`0x00924390`, `rcdtRsd2.c`): eight words
    /// cleared.
    pub fn rcdt_rsd2_init(mem: &mut impl Mem, p: u32) {
        mem.write(p, [0u32; 8]);
    }

    /// `rcbgTrxSetRegs(t, a..g)` (`0x0092d360`, from `rcbgPalAlloc`'s
    /// set-up): the low words of two GS register values built from
    /// sign-extended `i16`s, `+0x1c` = `((c << 8 | b) << 16) | a`, `+0x24`
    /// = `e << 16 | d`, `+0x28` = `f`, `+0x2c` = `g | f >> 31`, `+0x18` and
    /// `+0x20` 0. Returns `f`.
    #[allow(clippy::too_many_arguments, clippy::cast_sign_loss)]
    pub fn rcbg_trx_set_regs(
        mem: &mut impl Mem,
        t: u32,
        a: i16,
        b: i16,
        c: i16,
        d: i16,
        e: i16,
        f: i16,
        g: i16,
    ) -> u32 {
        let x = |v: i16| i32::from(v) as u32;
        let w1c = ((x(c) << 8 | x(b)) << 16) | x(a);
        mem.write(t.wrapping_add(0x1c), w1c);
        mem.write(t.wrapping_add(0x24), (x(e) << 16) | x(d));
        mem.write(t.wrapping_add(0x2c), x(g) | (i32::from(f) >> 31) as u32);
        mem.write(t.wrapping_add(0x18), 0u32);
        mem.write(t.wrapping_add(0x20), 0u32);
        mem.write(t.wrapping_add(0x28), x(f));
        x(f)
    }

    /// `rcbgClipCodeAnd(out, a, b, c, d)` (`0x0092aed0`, from
    /// `rcbgClipVertex`): `out = (a & c, b & d)`; returns `out`.
    pub fn rcbg_clip_code_and(mem: &mut impl Mem, out: u32, a: u32, b: u32, c: u32, d: u32) -> u32 {
        mem.write(out, [a & c, b & d]);
        out
    }

    /// `rcfxParticleResetForce(p)` (`0x00928970`): each of its `+0x34`
    /// forces (array at `+0x38`) with a reset function (`+0x10`) gets
    /// `call(mem, function, force, p)`, then [`rcfx_particle_clear_forces`].
    pub fn rcfx_particle_reset_force<M: Mem>(
        mem: &mut M,
        p: u32,
        mut call: impl FnMut(&mut M, u32, u32, u32),
    ) {
        let mut i = 0i32;
        while i < mem.read::<i32>(p.wrapping_add(0x34)) {
            #[allow(clippy::cast_sign_loss)] // i >= 0
            let at = mem
                .read::<u32>(p.wrapping_add(0x38))
                .wrapping_add(i as u32 * 4);
            let force = mem.read::<u32>(at);
            let f = mem.read::<u32>(force.wrapping_add(0x10));
            if f != 0 {
                call(mem, f, force, p);
            }
            i += 1;
        }
        rcfx_particle_clear_forces(mem, p);
    }

    /// `rcbgSceneDraw(scene)` (`0x00920740`): each of its `+0xc` objects
    /// (array at `+0x10`) with a draw function (`+0xc`) gets
    /// `call(mem, function, object)`, then the layers are queued
    /// ([`rcbg_scene_draw_layer`]); returns that result.
    pub fn rcbg_scene_draw<M: Mem>(
        mem: &mut M,
        s: u32,
        mut call: impl FnMut(&mut M, u32, u32),
        after_set: impl FnOnce(&mut M, u32, u32) -> u32,
    ) -> u32 {
        let mut i = 0i32;
        while i < mem.read::<i32>(s.wrapping_add(0xc)) {
            #[allow(clippy::cast_sign_loss)] // i >= 0
            let at = mem
                .read::<u32>(s.wrapping_add(0x10))
                .wrapping_add(i as u32 * 4);
            let o = mem.read::<u32>(at);
            let f = mem.read::<u32>(o.wrapping_add(0xc));
            if f != 0 {
                call(mem, f, o);
            }
            i += 1;
        }
        rcbg_scene_draw_layer(mem, s, after_set)
    }

    /// `Yn_PSimGetHandle(n)` (`0x009199f0`): the word at
    /// [`rcbg_fp_get_work`]`(n + 0x5b, 6, 2)` (FP work of map part 6).
    pub fn yn_psim_get_handle<M: Mem>(
        mem: &mut M,
        n: u32,
        run_of: impl FnOnce(&mut M, u32) -> u32,
    ) -> u32 {
        let w = rcbg_fp_get_work(mem, n.wrapping_add(0x5b), 6, 2, run_of);
        ptr32(mem.read::<u32>(w))
    }

    /// `rcbgPSimSetReady(psim)` (`0x0091ffb0`): its particle system's
    /// forces reset ([`rcfx_particle_reset_force`] on `+0x18`), then flag
    /// bit 0 (`+0x10`).
    pub fn rcbg_psim_set_ready<M: Mem>(
        mem: &mut M,
        p: u32,
        call: impl FnMut(&mut M, u32, u32, u32),
    ) {
        let sys = mem.read::<u32>(p.wrapping_add(0x18));
        rcfx_particle_reset_force(mem, sys, call);
        let f = mem.read::<u32>(p.wrapping_add(0x10));
        mem.write(p.wrapping_add(0x10), f | 1);
    }

    /// `rcbgPSimSetPos(psim, which, v)` (`0x0091ff10`): position `which`
    /// (0: `+0x4c`, else `+0x8c`) from `v`'s x, y, z; the first time each
    /// is set (`+0x9c`, `+0xa0`) the initial count (`+0xa4`) drops, and
    /// when it reaches 0 the simulation is made ready
    /// ([`rcbg_psim_set_ready`]).
    pub fn rcbg_psim_set_pos<M: Mem>(
        mem: &mut M,
        p: u32,
        which: u32,
        v: u32,
        call: impl FnMut(&mut M, u32, u32, u32),
    ) {
        let (pos, set) = if which == 0 {
            (psim::POS, 0x9c)
        } else {
            (psim::POS2, 0xa0)
        };
        let xyz = mem.read::<[u32; 3]>(v).map(crate::x87::copy);
        mem.write(p.wrapping_add(pos), xyz);
        if mem.read::<u32>(p.wrapping_add(set)) != 0 {
            return;
        }
        let mut ready = false;
        let n = mem.read::<u32>(p.wrapping_add(psim::INIT_CNT));
        if n != 0 {
            mem.write(p.wrapping_add(psim::INIT_CNT), n.wrapping_sub(1));
            ready = n == 1;
        }
        mem.write(p.wrapping_add(set), 1u32);
        if ready {
            rcbg_psim_set_ready(mem, p, call);
        }
    }

    /// `Yn_SeaGetWork()` (`0x00919a10`): [`rcbg_fp_get_work`]`(0x5a, 6,
    /// 2)`, the sea's FP work (`Yn_SeaSetAng`, `Yn_SeaSetPos`,
    /// `Yn_SeaSetUvSpd`).
    pub fn yn_sea_get_work<M: Mem>(mem: &mut M, run_of: impl FnOnce(&mut M, u32) -> u32) -> u32 {
        rcbg_fp_get_work(mem, 0x5a, 6, 2, run_of)
    }

    // ---- round 5: palettes, layers, scene objects, FP parts, characters ----

    /// `rcbgPalGetEntryData(pal, n)` (`0x00929990`): entry `n`'s data of
    /// the palette's transfer (`+0xc`, [`rcbg_trx_get_entry_data`]).
    pub fn rcbg_pal_get_entry_data(mem: &impl Mem, pal: u32, n: u32) -> u32 {
        let t = mem.read::<u32>(pal.wrapping_add(0xc));
        rcbg_trx_get_entry_data(mem, t, n)
    }

    /// `rcbgPalGetLastEntryData(pal)` (`0x00929930`): the last entry's
    /// (the transfer's count at `+0x30`, less 1).
    pub fn rcbg_pal_get_last_entry_data(mem: &impl Mem, pal: u32) -> u32 {
        let t = mem.read::<u32>(pal.wrapping_add(0xc));
        let n = mem.read::<u32>(t.wrapping_add(0x30)).wrapping_sub(1);
        rcbg_trx_get_entry_data(mem, t, n)
    }

    /// `rcbgLayerGetTexData(layer)` (`0x0092acf0`): entry 0's data of the
    /// layer's texture transfer (`+0xbc`), 0 without one.
    pub fn rcbg_layer_get_tex_data(mem: &impl Mem, layer: u32) -> u32 {
        match mem.read::<u32>(layer.wrapping_add(0xbc)) {
            0 => 0,
            t => rcbg_trx_get_entry_data(mem, t, 0),
        }
    }

    /// `rcbgTexGetLog2Size(w, h, &lw, &lh, &tbw)` (`0x00920050`): the
    /// GS log2 sizes (`rcLog2Ceil`) and the buffer width in 64-pixel units,
    /// rounded up to even; each output optional.
    pub fn rcbg_tex_get_log2_size(mem: &mut impl Mem, w: i32, h: i32, lw: u32, lh: u32, tbw: u32) {
        if lw != 0 {
            mem.write(lw, super::log2_ceil(w));
        }
        if lh != 0 {
            mem.write(lh, super::log2_ceil(h));
        }
        if tbw != 0 {
            let n = w.wrapping_add(0x3f);
            let mut v = n.wrapping_add((n >> 31) & 0x3f) >> 6;
            mem.write(tbw, v);
            if v & 1 != 0 {
                v = v.wrapping_add(1);
                mem.write(tbw, v);
            }
        }
    }

    /// The scene, when Yn's field state, its map and the scene exist.
    fn yn_scene(mem: &impl Mem) -> Option<u32> {
        let s = crate::guide::YN_MAP_SYSTEM.get(mem);
        if s == 0 {
            return None;
        }
        let m = mem.read::<u32>(s);
        if m == 0 {
            return None;
        }
        Some(mem.read::<u32>(m.wrapping_add(8))).filter(|&c| c != 0)
    }

    /// `Yn_SceneGetObject(kind, n)` (`0x00921be0`): the scene's
    /// `rcbgSceneGetObject`, 0 without a scene.
    pub fn yn_scene_get_object(mem: &impl Mem, kind: u32, n: u32) -> u32 {
        yn_scene(mem).map_or(0, |s| {
            crate::scene::bridge::rcbg_scene_get_object(mem, s, kind, n)
        })
    }

    /// `Yn_SceneCountObjects(kind)` (`0x00921c20`): the scene's
    /// `rcbgSceneCountObjects`, 0 without a scene.
    pub fn yn_scene_count_objects(mem: &impl Mem, kind: u32) -> u32 {
        yn_scene(mem).map_or(0, |s| {
            crate::scene::bridge::rcbg_scene_count_objects(mem, s, kind)
        })
    }

    /// `Yn_BattleCaptureNum()` (`0x00921910`): while Yn's battle number
    /// (`+0xc`) is 0, takes `MsBattleGetNum` when non-zero and clears
    /// `+8`.
    pub fn yn_battle_capture_num(mem: &mut impl Mem) {
        let s = crate::guide::YN_MAP_SYSTEM.get(mem);
        if mem.read::<u32>(s.wrapping_add(0xc)) != 0 {
            return;
        }
        let n = ffx_battle::state::bridge::ms_battle_get_num(mem);
        if n != 0 {
            let s = crate::guide::YN_MAP_SYSTEM.get(mem);
            mem.write(s.wrapping_add(0xc), n);
            let s = crate::guide::YN_MAP_SYSTEM.get(mem);
            mem.write(s.wrapping_add(8), 0u32);
        }
    }

    /// `rcbgFpFadeInit(fade, id, step, frames)` (`0x00922930`, from
    /// `rcbgFPm097`'s process): hides FP `id` (`pppFpIDdispOff`,
    /// `hide`) and sets up its fade: 0, the id, the step (a float), the
    /// frames, 0.
    pub fn rcbg_fp_fade_init<M: Mem>(
        mem: &mut M,
        fade: u32,
        id: u32,
        step: u32,
        frames: u32,
        hide: impl FnOnce(&mut M, u32),
    ) {
        hide(mem, id);
        mem.write(fade.wrapping_add(8), crate::x87::copy(step));
        mem.write(fade.wrapping_add(4), id);
        mem.write(fade, 0u32);
        mem.write(fade.wrapping_add(0xc), frames);
        mem.write(fade.wrapping_add(0x10), 0u32);
    }

    /// `rcbgChrFindById(id)` (`0x0092e1e0`): the first character whose
    /// `Ch_GetId` is `id`, through the character handle cursor; 0 for none.
    pub fn rcbg_chr_find_by_id(mem: &mut impl Mem, id: u32) -> u32 {
        use ffx_chara::handle::bridge as h;
        h::ch_handle_rewind(mem);
        let mut c = h::ch_handle_get_next(mem);
        while c != 0 {
            if h::ch_get_id(mem, c) == id {
                return c;
            }
            c = h::ch_handle_get_next(mem);
        }
        0
    }

    /// `rcbgChrHandleIndex(handle)` (`0x00924f30`): the battle slot
    /// (`MsGetChrHandle`, 0..=14 then 20..=27) holding `handle`, or -1.
    pub fn rcbg_chr_handle_index(mem: &impl Mem, handle: u32) -> u32 {
        (0..=14u32)
            .chain(20..=27)
            .find(|&i| ffx_battle::actor::bridge::ms_get_chr_handle(mem, i) == handle)
            .unwrap_or(u32::MAX)
    }

    // ---- round 6 ----

    /// `rcbgTrxLoad(trx, parse)` (`0x0092b710`): three 8-byte GS register
    /// values (`+0x18`, `+0x20`, `+0x28`), the entry count and size
    /// (`+0x30`, `+0x34`) from the parser, the entries at its position
    /// (`+0x38`), then past them (`rcbgTrxSetTag` is empty on PC).
    pub fn rcbg_trx_load(mem: &mut impl Mem, t: u32, p: u32) {
        use crate::rcdt::bridge::{rcdt_parse_advance, rcdt_parse_get_bin};
        rcdt_parse_get_bin(mem, p, t.wrapping_add(0x18), 8);
        rcdt_parse_get_bin(mem, p, t.wrapping_add(0x20), 8);
        rcdt_parse_get_bin(mem, p, t.wrapping_add(0x28), 8);
        rcdt_parse_get_bin(mem, p, t.wrapping_add(trx::COUNT), 4);
        rcdt_parse_get_bin(mem, p, t.wrapping_add(trx::ENTRY_SIZE), 4);
        let at = mem.read::<u32>(p.wrapping_add(4));
        mem.write(t.wrapping_add(trx::ENTRIES), at);
        let n = mem.read::<u32>(t.wrapping_add(trx::COUNT));
        let size = mem.read::<u32>(t.wrapping_add(trx::ENTRY_SIZE));
        rcdt_parse_advance(mem, p, n.wrapping_mul(size));
    }

    /// `Yn_SeaSetAng(v)` (`0x0091ae40`): the sea's angle (`+0x50` of its
    /// FP work, `FFXVu0CopyVectorXYZ`); returns that `eax`, 0 without the
    /// work.
    pub fn yn_sea_set_ang<M: Mem>(
        mem: &mut M,
        v: u32,
        run_of: impl FnOnce(&mut M, u32) -> u32,
    ) -> u32 {
        match yn_sea_get_work(mem, run_of) {
            0 => 0,
            w => ffx_ps2::vu0::bridge::copy_vector_xyz(mem, w.wrapping_add(0x50), v),
        }
    }

    /// `Yn_SeaSetPos(v)` (`0x0091ae80`): its position (`+0x40`).
    pub fn yn_sea_set_pos<M: Mem>(
        mem: &mut M,
        v: u32,
        run_of: impl FnOnce(&mut M, u32) -> u32,
    ) -> u32 {
        match yn_sea_get_work(mem, run_of) {
            0 => 0,
            w => ffx_ps2::vu0::bridge::copy_vector_xyz(mem, w.wrapping_add(0x40), v),
        }
    }

    /// `Yn_SeaSetUvSpd(u, v)` (`0x0091aea0`): the texture scroll speeds
    /// (`+0x60`, `+0x64`) and their flag (`+0xe4`).
    pub fn yn_sea_set_uv_spd<M: Mem>(
        mem: &mut M,
        u: u32,
        v: u32,
        run_of: impl FnOnce(&mut M, u32) -> u32,
    ) {
        let w = yn_sea_get_work(mem, run_of);
        if w != 0 {
            mem.write(w.wrapping_add(0xe4), 1u8);
            mem.write(w.wrapping_add(0x60), crate::x87::copy(u));
            mem.write(w.wrapping_add(0x64), crate::x87::copy(v));
        }
    }

    /// `Yn_PSimSetPos(psim, which, v)` (`0x00919f10`):
    /// [`rcbg_psim_set_pos`] for a simulation and `which` below 2.
    pub fn yn_psim_set_pos<M: Mem>(
        mem: &mut M,
        p: u32,
        which: i32,
        v: u32,
        call: impl FnMut(&mut M, u32, u32, u32),
    ) {
        if p != 0 && which < 2 {
            rcbg_psim_set_pos(mem, p, which.cast_unsigned(), v, call);
        }
    }

    /// `Yn_SeaGetAngPtr()` (`0x0091adf0`): the sea's angle (`+0x50` of its
    /// FP work), 0 without the work.
    pub fn yn_sea_get_ang_ptr<M: Mem>(mem: &mut M, run_of: impl FnOnce(&mut M, u32) -> u32) -> u32 {
        match yn_sea_get_work(mem, run_of) {
            0 => 0,
            w => w.wrapping_add(0x50),
        }
    }

    /// `Yn_SeaGetPosPtr()` (`0x0091ae10`): its position (`+0x40`).
    pub fn yn_sea_get_pos_ptr<M: Mem>(mem: &mut M, run_of: impl FnOnce(&mut M, u32) -> u32) -> u32 {
        match yn_sea_get_work(mem, run_of) {
            0 => 0,
            w => w.wrapping_add(0x40),
        }
    }

    /// `rcbgPalCopyToLast(pal)` (`0x009299d0`): the palette's current
    /// entry (`i16` at `+8`) copied into the last (256 words), which is
    /// then made the transfer to run (`rcbgTrxSetNo`).
    pub fn rcbg_pal_copy_to_last(mem: &mut impl Mem, pal: u32) {
        let no = i32::from(mem.read::<i16>(pal.wrapping_add(8))).cast_unsigned();
        let src = rcbg_pal_get_entry_data(mem, pal, no);
        let dst = rcbg_pal_get_last_entry_data(mem, pal);
        for i in 0..256u32 {
            let v = mem.read::<u32>(src.wrapping_add(4 * i));
            mem.write(dst.wrapping_add(4 * i), v);
        }
        let t = mem.read::<u32>(pal.wrapping_add(0xc));
        let n = mem.read::<u32>(t.wrapping_add(trx::COUNT)).wrapping_sub(1);
        rcbg_trx_set_no(mem, t, n);
    }

    /// `rcdtTm2CopyRect(tm2, dst, x, y, bytes, rows)` (`0x00923ae0`): rows
    /// of `bytes` from the image data (`+0x40`, row stride `u16` at
    /// `+0x24`) at byte `x` of row `y`, each to the next 8-byte boundary
    /// of `dst` (`memcpy`).
    pub fn rcdt_tm2_copy_rect(
        mem: &mut impl Mem,
        tm2: u32,
        dst: u32,
        x: u32,
        y: u32,
        bytes: u32,
        rows: i32,
    ) {
        let data = ptr32(mem.read::<u32>(tm2.wrapping_add(0x40)));
        let stride = u32::from(mem.read::<u16>(tm2.wrapping_add(0x24)));
        let mut src = stride.wrapping_mul(y).wrapping_add(data.wrapping_add(x));
        let step = bytes.wrapping_add(7) & !7;
        let mut d = dst;
        let mut buf = vec![0u8; bytes as usize];
        for _ in 0..rows.max(0) {
            mem.read_bytes(src, &mut buf);
            mem.write_bytes(d, &buf);
            d = d.wrapping_add(step);
            let stride = u32::from(mem.read::<u16>(tm2.wrapping_add(0x24)));
            src = src.wrapping_add(stride);
        }
    }

    /// `rcVec4fToFixed(out, v)` (`0x00925950`): `v`'s four floats times 16
    /// (`FFXVu0RotTransPers`'s `f64`) through `_ftol2_sse`, as 12.4 fixed
    /// point; returns the last.
    pub fn rc_vec4f_to_fixed(mem: &mut impl Mem, out: u32, v: u32) -> u32 {
        let k = Reg::constant(ffx_ps2::vu0::bridge::FIXED_SCALE.get(mem));
        let src = mem.read::<[f32; 4]>(v);
        let mut r = 0;
        for (i, x) in (0u32..).zip(src) {
            r = ffx_bridge::x87::ftol(mem, (Reg::load(x) * k).value());
            mem.write(out.wrapping_add(4 * i), r);
        }
        r
    }

    /// `rcMeshPastIndices(obj)` (`0x00928270`): the mesh's (`+0x14`) data
    /// (`+0xc`) past its index triples (`+0x14`, an `i16` count, 6 bytes
    /// each), rounded up to 16 bytes.
    pub fn rc_mesh_past_indices(mem: &impl Mem, obj: u32) -> u32 {
        let mesh = mem.read::<u32>(obj.wrapping_add(0x14));
        let mut n = i32::from(mem.read::<i16>(mesh.wrapping_add(0x14)))
            .wrapping_mul(6)
            .cast_unsigned();
        if n & 0xf != 0 {
            n = (n & !0xf).wrapping_add(0x10);
        }
        mem.read::<u32>(mesh.wrapping_add(0xc)).wrapping_add(n)
    }

    /// `rcfxGravityInit(fx)` (`0x00929570`): kind `0x30002`, its update
    /// (`0x00929560`) and draw (`0x00929500`) callbacks, the rest 0 (the
    /// four floats from `+0x1c` as 0.0). Returns `fx`.
    pub fn rcfx_gravity_init(mem: &mut impl Mem, fx: u32) -> u32 {
        mem.write(fx, 0x0003_0002u32);
        let up = mem.rebase(super::GRAVITY_UPDATE);
        mem.write(fx.wrapping_add(4), up);
        for off in [8u32, 0xc, 0x10] {
            mem.write(fx.wrapping_add(off), 0u32);
        }
        let draw = mem.rebase(super::GRAVITY_DRAW);
        mem.write(fx.wrapping_add(0x14), draw);
        for off in [0x18u32, 0x28, 0x24, 0x20, 0x1c] {
            mem.write(fx.wrapping_add(off), 0u32);
        }
        fx
    }

    /// `rcbgIdentity(x)` (`0x009202a0`): `x` (called on a saved register
    /// by `rcbgRestoreReg`).
    #[must_use]
    pub const fn rcbg_identity(x: u32) -> u32 {
        x
    }

    /// `rcefAnimeStep(anim)` (`0x0092f400`): adds the speed (`+0x44`) to
    /// the frame time (`+0x40`, `i16`); past the current key's length
    /// (`+0x12` of the 8-byte key `+0x42` in the table `+4`) moves to the
    /// next key, and past the last (`+6` of the table) loops (key flag
    /// 0x80: `+0x40` as a word cleared) or stops (bit 0 of `+0` cleared,
    /// time 0, the key kept). Returns the `eax` it ends with.
    pub fn rcef_anime_step(mem: &mut impl Mem, anim: u32) -> u32 {
        let table = mem.read::<u32>(anim.wrapping_add(4));
        let key = mem.read::<u16>(anim.wrapping_add(0x42));
        let k = table.wrapping_add(i32::from(key.cast_signed()).cast_unsigned().wrapping_mul(8));
        let speed = mem.read::<u16>(anim.wrapping_add(0x44));
        let t = mem.read::<u16>(anim.wrapping_add(0x40)).wrapping_add(speed);
        mem.write(anim.wrapping_add(0x40), t);
        let len = mem.read::<u16>(k.wrapping_add(0x12));
        if t.cast_signed() < len.cast_signed() {
            return u32::from(t);
        }
        let t = t.wrapping_sub(len);
        mem.write(anim.wrapping_add(0x40), t);
        let next = key.wrapping_add(1);
        mem.write(anim.wrapping_add(0x42), next);
        let eax = u32::from(key) + 1;
        if next.cast_signed() < mem.read::<i16>(table.wrapping_add(6)) {
            return eax;
        }
        if mem.read::<u8>(k.wrapping_add(0x14)) & 0x80 != 0 {
            mem.write(anim.wrapping_add(0x40), 0u32);
            return 0;
        }
        let f = mem.read::<u32>(anim);
        mem.write(anim, f & !1);
        mem.write(anim.wrapping_add(0x40), 0u16);
        mem.write(anim.wrapping_add(0x42), key);
        u32::from(key)
    }

    /// `rcfxParticleInit(fx)` (`0x009288e0`): the defaults: `+8` 0.03
    /// (`f64`), `+0x18` 0.001 and `+0x20` 0.1 (`f32`s from `.rdata`), the
    /// rest 0. Returns `fx`.
    pub fn rcfx_particle_init(mem: &mut impl Mem, fx: u32) -> u32 {
        mem.write(fx, 0u64);
        mem.write(fx.wrapping_add(0x1c), 0u32);
        let d = mem.rebase(super::PARTICLE_RATE);
        let d = mem.read::<u64>(d);
        mem.write(fx.wrapping_add(8), d);
        for off in [0x24u32, 0x28, 0x2c] {
            mem.write(fx.wrapping_add(off), 0u32);
        }
        mem.write(fx.wrapping_add(0x10), 0u64);
        mem.write(fx.wrapping_add(0x30), 0u32);
        let a = mem.rebase(super::PARTICLE_DEFAULTS[0]);
        let a = mem.read::<u32>(a);
        mem.write(fx.wrapping_add(0x34), 0u32);
        mem.write(fx.wrapping_add(0x18), crate::x87::copy(a));
        mem.write(fx.wrapping_add(0x38), 0u32);
        let b = mem.rebase(super::PARTICLE_DEFAULTS[1]);
        let b = mem.read::<u32>(b);
        mem.write(fx.wrapping_add(0x3c), 0u32);
        mem.write(fx.wrapping_add(0x20), crate::x87::copy(b));
        fx
    }

    /// `rcefSetupDraw(draw, tex, a, b)` (`0x0092f6b0`): fills a draw
    /// record: 1, `tex`, a GIF tag (`+0x10` 0x1c, `+0x14` from
    /// `0x00c498dc`, `+0x18` 0x44), `+0x30` its own `+0x88`, `+0x34`/`+0x38`
    /// two fixed buffers (`0x0230fd60`, `0x0230fda0`), `a` at `+0x80` and
    /// `+0x3c`, `b` at `+0x84`. Returns `draw + 0x88`.
    pub fn rcef_setup_draw(mem: &mut impl Mem, draw: u32, tex: u32, a: u32, b: u32) -> u32 {
        mem.write(draw.wrapping_add(4), tex);
        mem.write(draw.wrapping_add(0x84), b);
        mem.write(draw, 1u32);
        mem.write(draw.wrapping_add(0x80), a);
        mem.write(draw.wrapping_add(0x10), 0x1cu32);
        let at = mem.rebase(super::REF_GIF_TAG);
        let g = mem.read::<u32>(at);
        mem.write(draw.wrapping_add(0x14), g);
        mem.write(draw.wrapping_add(0x18), 0x44u8);
        mem.write(draw.wrapping_add(0x30), draw.wrapping_add(0x88));
        let b0 = mem.rebase(super::REF_BUFFERS[0]);
        let b1 = mem.rebase(super::REF_BUFFERS[1]);
        mem.write(draw.wrapping_add(0x34), b0);
        mem.write(draw.wrapping_add(0x38), b1);
        mem.write(draw.wrapping_add(0x3c), a);
        draw.wrapping_add(0x88)
    }

    /// `rcfxNailInit(fx)` (`0x00929100`): kind `0x30004`, its callbacks
    /// (`+4` `0x00929070`, `+0x10` `0x00929190`, `+0x14` `0x00928f40`),
    /// `+0x18` 2, `+0x30` 0.0 (`f64`), the rest 0. Returns `fx`.
    pub fn rcfx_nail_init(mem: &mut impl Mem, fx: u32) -> u32 {
        mem.write(fx.wrapping_add(0x30), 0u64);
        mem.write(fx, 0x0003_0004u32);
        let [a, b, c] = super::NAIL_CALLBACKS.map(|f| mem.rebase(f));
        mem.write(fx.wrapping_add(4), a);
        mem.write(fx.wrapping_add(8), 0u32);
        mem.write(fx.wrapping_add(0xc), 0u32);
        mem.write(fx.wrapping_add(0x10), b);
        mem.write(fx.wrapping_add(0x14), c);
        mem.write(fx.wrapping_add(0x18), 2u32);
        for off in [0x1cu32, 0x20, 0x24, 0x28, 0x2c, 0x38] {
            mem.write(fx.wrapping_add(off), 0u32);
        }
        fx
    }

    /// `rcfxWindInit(fx)` (`0x00929850`): kind `0x30006`, its callbacks
    /// (`+4` `0x00929840`, `+0x14` `0x009296d0`), `+0x1c` 1.0, `+0x34` 0.1
    /// (from `.rdata`), `+0x40` 1.0 (`f64`), the rest 0. Returns `fx`.
    pub fn rcfx_wind_init(mem: &mut impl Mem, fx: u32) -> u32 {
        mem.write(fx, 0x0003_0006u32);
        let [a, b] = super::WIND_CALLBACKS.map(|f| mem.rebase(f));
        mem.write(fx.wrapping_add(4), a);
        for off in [8u32, 0xc, 0x10] {
            mem.write(fx.wrapping_add(off), 0u32);
        }
        mem.write(fx.wrapping_add(0x14), b);
        mem.write(fx.wrapping_add(0x18), 0u32);
        mem.write(fx.wrapping_add(0x1c), 1.0f32.to_bits());
        for off in [0x20u32, 0x24, 0x28, 0x2c, 0x30] {
            mem.write(fx.wrapping_add(off), 0u32);
        }
        let k = mem.rebase(super::PARTICLE_DEFAULTS[1]);
        let k = mem.read::<u32>(k);
        mem.write(fx.wrapping_add(0x34), crate::x87::copy(k));
        mem.write(fx.wrapping_add(0x38), 0u32);
        mem.write(fx.wrapping_add(0x40), 1.0f64.to_bits());
        mem.write(fx.wrapping_add(0x50), 0u64);
        mem.write(fx.wrapping_add(0x48), 0u64);
        mem.write(fx.wrapping_add(0x5c), 0u32);
        mem.write(fx.wrapping_add(0x58), 0u32);
        fx
    }

    /// `rcFree(p)` (`0x009258e0`, a jump to `user_free`) through the
    /// caller's `call(mem, function, args)`: returns its `eax`.
    pub fn rc_free<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
        p: u32,
    ) -> u32 {
        let f = mem.rebase(super::RC_FREE);
        call(mem, f, &[p])
    }

    /// Frees the block a word at `at` points to (when set) and clears the
    /// word: the pattern of the `*Fini` functions. Returns `rcFree`'s
    /// `eax`, or 0 with nothing to free.
    pub fn free_field<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
        at: u32,
    ) -> u32 {
        let p = mem.read::<u32>(at);
        if p == 0 {
            return 0;
        }
        let r = rc_free(mem, call, p);
        mem.write(at, 0u32);
        r
    }

    /// `rcbgModelFini(model)` (`0x00928240`): frees its animation data
    /// (`+0x14`).
    pub fn rcbg_model_fini<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        model: u32,
        call: &mut C,
    ) -> u32 {
        free_field(mem, call, model.wrapping_add(0x14))
    }

    /// `rcbgModelDelete(model)` (`0x00928000`): [`rcbg_model_fini`], then
    /// the model itself freed.
    pub fn rcbg_model_delete<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        model: u32,
        call: &mut C,
    ) -> u32 {
        rcbg_model_fini(mem, model, call);
        rc_free(mem, call, model)
    }

    /// `rcbgTrxFini(trx)` (`0x0092d490`): frees the transfer's own data
    /// (`+0x38`) when flag bit 0 (`+0x10`) says it owns it.
    pub fn rcbg_trx_fini<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        trx: u32,
        call: &mut C,
    ) {
        if mem.read::<u8>(trx.wrapping_add(0x10)) & 1 != 0 {
            free_field(mem, call, trx.wrapping_add(0x38));
        }
    }

    /// `rcbgTrxDelete(trx)` (`0x0092d450`): [`rcbg_trx_fini`], then the
    /// transfer freed.
    pub fn rcbg_trx_delete<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        trx: u32,
        call: &mut C,
    ) -> u32 {
        rcbg_trx_fini(mem, trx, call);
        rc_free(mem, call, trx)
    }

    /// `rcbgLayerAnimeFini(anime)` (`0x00930830`): frees `+0xc` (a
    /// layer's `+0x90` block).
    pub fn rcbg_layer_anime_fini<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        a: u32,
        call: &mut C,
    ) -> u32 {
        free_field(mem, call, a.wrapping_add(0xc))
    }

    /// `rcbgEmbeddedTrxGetCnt(obj)` (`0x00922140`): the entry count of the
    /// transfer embedded at `+0x20` ([`rcbg_trx_get_cnt`]); reached
    /// through `Yn_EmbeddedTrxGetCnt` (`0x00919bc0`).
    pub fn rcbg_embedded_trx_get_cnt(mem: &impl Mem, obj: u32) -> u32 {
        rcbg_trx_get_cnt(mem, obj.wrapping_add(0x20))
    }

    /// `rcefSetStream(stream)` (`0x00925710`, a table entry at
    /// `0x00c65524`): the stream word (`0x025d5c14`), and with one, mode 3
    /// at `0x025d5be0`. Returns `stream`.
    pub fn rcef_set_stream(mem: &mut impl Mem, s: u32) -> u32 {
        super::REF_STREAM.set(mem, s);
        if s != 0 {
            super::REF_STREAM_MODE.set(mem, 3);
        }
        s
    }

    /// `rcbgRunRecordHook(obj)` (`0x0091fd60`, set by `0x0091fe2e`): calls
    /// the hook at `+0xc` of the record `+0x14` on that record, when set.
    /// Returns its `eax`, else the record.
    pub fn rcbg_run_record_hook<M: Mem>(
        mem: &mut M,
        obj: u32,
        mut call: impl FnMut(&mut M, u32, u32) -> u32,
    ) -> u32 {
        let r = mem.read::<u32>(obj.wrapping_add(0x14));
        let f = mem.read::<u32>(r.wrapping_add(0xc));
        if f == 0 {
            return r;
        }
        call(mem, f, r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packet_sizes_are_signed() {
        assert_eq!(packet_size(0x14, 3), 0x10 + 0x3c);
        assert_eq!(packet_size(0x14, -1), 0x10u32.wrapping_sub(0x14));
    }

    #[test]
    fn halves_round_like_the_original() {
        assert_eq!(f32::from_bits(half(640)), 320.0);
        assert_eq!(f32::from_bits(half(16_777_217)), 8_388_608.0);
    }
}

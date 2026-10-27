//! The render-side glue, fifty-sixth batch (M4 port round 16, list
//! `h3-glue-render`): the scene's boot-time set-up.

/// `SpdCtrl_Init()` (cdecl).
pub const SPD_CTRL_INIT: u32 = 0x006f_72f0;
/// `Phyre_PWorld_initialize(world)` (thiscall) and `addCluster(world;
/// cluster)` (`ret 4`).
pub const WORLD_INITIALIZE: u32 = 0x0044_3a40;
pub const WORLD_ADD_CLUSTER: u32 = 0x0044_3250;
/// `Phyre_PFramework_PApplication_GetApplication()` (cdecl).
pub const GET_APPLICATION: u32 = 0x0062_3b90;
/// `CharacterLighting_GetInstance()` (cdecl) and `initialize(lighting;
/// device)` (`ret 4`).
pub const LIGHTING_INSTANCE: u32 = 0x006a_6e30;
pub const LIGHTING_INITIALIZE: u32 = 0x006a_8f80;
/// The boot text: `Instance_006dc800_Get()` (cdecl) and
/// `BootText_create_006ddc80(instance + 0x110)` (thiscall).
pub const BOOT_TEXT_OWNER: u32 = 0x006d_c8a0;
pub const BOOT_TEXT_CREATE: u32 = 0x006d_dc80;
/// The tweak utilities' getters (cdecl) and their `initialize(util)`
/// (thiscall), in the original's order.
pub const TWEAKS: [(u32, u32); 9] = [
    (0x006a_6e40, 0x006a_91e0),
    (0x006b_6330, 0x006b_9970),
    (0x006b_6360, 0x006b_9e20),
    (0x006b_6340, 0x006b_9aa0),
    (0x006b_6320, 0x006b_97f0),
    (0x006b_62f0, 0x006b_9360),
    (0x006b_6310, 0x006b_9600),
    (0x006b_6300, 0x006b_94a0),
    (0x006b_6350, 0x006b_9cd0),
];
/// Distortion, blur and blur-ex: getters (cdecl) and `initialize(effect;
/// renderer, width, height, texture, texture slot)` (`ret 0x14`).
pub const SCREEN_EFFECTS: [(u32, u32); 3] = [
    (0x006c_2290, 0x006c_2710),
    (0x006c_3580, 0x006c_38b0),
    (0x006c_4a70, 0x006c_4b00),
];
/// `MotionBlur_GetInstance()` and `initialize(blur; renderer, a, b)`
/// (`ret 0xc`).
pub const MOTION_BLUR_INSTANCE: u32 = 0x006c_5130;
pub const MOTION_BLUR_INITIALIZE: u32 = 0x006c_5360;
/// `BrokenScreenEffect_GetInstance()` and `initialize(effect; renderer,
/// device, texture, slot, word, width, height)` (`ret 0x1c`).
pub const BROKEN_SCREEN_INSTANCE: u32 = 0x006d_4de0;
pub const BROKEN_SCREEN_INITIALIZE: u32 = 0x006d_51d0;
/// `FogEffectManager_GetInstance()` and `initialize(fog; device, scene +
/// 0x1198, lighting + 0x158)` (`ret 0xc`).
pub const FOG_INSTANCE: u32 = 0x006e_32c0;
pub const FOG_INITIALIZE: u32 = 0x006e_3400;
/// `MorphManager_GetInstance()` and `initialize(morph; renderer)` (`ret 4`).
pub const MORPH_INSTANCE: u32 = 0x006a_bf40;
pub const MORPH_INITIALIZE: u32 = 0x006a_c1b0;
/// `PTextureD3D11_register_0058cf80(sampler)` (thiscall) and
/// `PhyreScene_setupSamplers_0065f290(scene)` (thiscall).
pub const SAMPLER_REGISTER: u32 = 0x0058_cf80;
pub const SETUP_SAMPLERS: u32 = 0x0065_f290;
/// The arrays' `reserve(array; n)` (`ret 4`), by element type.
pub const RESERVE_WORDS: u32 = 0x0066_7e80;
pub const RESERVE_FLAGS: u32 = 0x0066_7e00;
pub const RESERVE_LAYOUTS: u32 = 0x0066_8200;
pub const RESERVE_8080: u32 = 0x0066_8080;
pub const RESERVE_8000: u32 = 0x0066_8000;
pub const RESERVE_7F80: u32 = 0x0066_7f80;
pub const RESERVE_8280: u32 = 0x0066_8280;
pub const RESERVE_FREE_LIST: u32 = 0x0066_8380;
/// `Phyre_PCluster_create(pool; count, class, 0)` (`ret 0xc`) on the
/// scene's pool `+0x12d4`, and the four classes.
pub const CLUSTER_CREATE: u32 = 0x0044_1010;
pub const POOL_CLASSES: [u32; 4] = [0x00c9_98e8, 0x00c9_9c80, 0x00cc_ba18, 0x00c9_2090];
/// `user_aligned_malloc(n, align)`, `user_malloc(n)`, the user heap's
/// `free` and the in-image `memcpy` thunk (all cdecl).
pub const ALIGNED_MALLOC: u32 = 0x0068_6ff0;
pub const USER_MALLOC: u32 = 0x0068_7020;
pub const USER_FREE: u32 = 0x0068_7010;
pub const MEMCPY: u32 = 0x0094_9256;
/// `PhyreScene_createRWTTTarget(scene)` (thiscall).
pub const CREATE_RWTT_TARGET: u32 = 0x0065_9c30;
/// The movie player (`scene + 0xde8`): `initialize(player; 1, 0, 0)`
/// (`ret 0xc`) and `close(player)` (thiscall, the original's tail jump).
pub const VIDEO_INITIALIZE: u32 = 0x00a2_7cf0;
pub const VIDEO_CLOSE: u32 = 0x00a2_8350;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    /// The word of the texture slot at `slot` (`+4` its "none" byte): the
    /// texture, or 0.
    fn texture(h: &impl Host, slot: u32) -> u32 {
        if h.read::<u8>(slot.wrapping_add(4)) == 0 {
            h.read(slot)
        } else {
            0
        }
    }

    /// The five sampler bytes at `desc` (`+0x1c` bit 7 set: left alone)
    /// set to `values`, then the sampler registered.
    fn sampler(h: &mut impl Host, desc: u32, values: [u8; 5]) {
        if (h.read::<u32>(desc + 0x1c) >> 7) & 1 == 0 {
            for (k, v) in (0u32..).zip(values) {
                h.write(desc + k, v);
            }
        }
        h.call_sym(SAMPLER_REGISTER, Conv::Thiscall(desc), &[]);
    }

    /// Clears the array at `arr` (count, capacity, ...): `reserve(0)` when
    /// count and capacity are both negative, then count 0.
    fn clear_array(h: &mut impl Host, arr: u32, reserve: u32) {
        if h.read::<u32>(arr).cast_signed() < 0 && h.read::<u32>(arr + 4).cast_signed() < 0 {
            h.call_sym(reserve, Conv::Thiscall(arr), &[0]);
        }
        h.write(arr, 0u32);
    }

    /// Sizes the free list at `arr` to 3000 (`reserve(3000)` when count
    /// and capacity are both under it).
    fn size_free_list(h: &mut impl Host, arr: u32) {
        if h.read::<u32>(arr).cast_signed() < 3000 && h.read::<u32>(arr + 4).cast_signed() < 3000 {
            h.call_sym(RESERVE_FREE_LIST, Conv::Thiscall(arr), &[3000]);
        }
        h.write(arr, 3000u32);
    }

    /// `PhyreScene_bootInit(scene)` (thiscall, no frame) -> the movie
    /// player's `close` result (the original tail-jumps to it).
    ///
    /// The speed control, the five worlds (`+0x7c`, `+0xac`, `+0x8c`,
    /// `+0x9c`, `+0xbc`), the renderer (`+0xd0` = application + 0x124),
    /// two clusters added to the main world, the list heads, the lighting,
    /// the boot text, the nine tweak utilities, the screen effects, the
    /// motion blur, the broken screen, the fog, the morph manager, the
    /// two samplers (`+0xfaa8`, `+0xfacc`), the scene's samplers, room for
    /// 200 texture slots, the work arrays emptied, the mesh-instance
    /// pools (`+0x12d4`: four classes of 3000), their buffers and free
    /// lists, the 4000-byte flag array zeroed, the render-to-texture
    /// target, then the movie player initialized and closed.
    #[allow(clippy::too_many_lines)] // one original, kept in its order
    pub fn boot_init(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(SPD_CTRL_INIT, Conv::Cdecl, &[]);
        h.write(this + 0x10dcc, 1u8);
        for world in [0x7c, 0xac, 0x8c, 0x9c, 0xbc] {
            h.call_sym(WORLD_INITIALIZE, Conv::Thiscall(this + world), &[]);
        }
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        h.write(this + 0xd0, app.wrapping_add(0x124));
        for cluster in [0x11e4, 0x28c4] {
            h.call_sym(
                WORLD_ADD_CLUSTER,
                Conv::Thiscall(this + 0x7c),
                &[this + cluster],
            );
        }
        let device: u32 = h.read(this + 0xcc);
        for (head, first) in [(0xdc, 0xe0), (0x238, 0x23c), (0x394, 0x398), (0x7e0, 0x7e4)] {
            h.write(this + head, this + first);
        }
        h.write(this + 0xf9d0, 1u16);
        h.write(this + 0x654, this + 0x658);
        let lighting = h.call_sym(LIGHTING_INSTANCE, Conv::Cdecl, &[]);
        h.call_sym(LIGHTING_INITIALIZE, Conv::Thiscall(lighting), &[device]);
        let (get, init) = TWEAKS[0];
        let util = h.call_sym(get, Conv::Cdecl, &[]);
        h.call_sym(init, Conv::Thiscall(util), &[]);
        let owner = h.call_sym(BOOT_TEXT_OWNER, Conv::Cdecl, &[]);
        h.call_sym(
            BOOT_TEXT_CREATE,
            Conv::Thiscall(owner.wrapping_add(0x110)),
            &[],
        );
        for &(get, init) in &TWEAKS[1..] {
            let util = h.call_sym(get, Conv::Cdecl, &[]);
            h.call_sym(init, Conv::Thiscall(util), &[]);
        }
        for (get, init) in SCREEN_EFFECTS {
            let config: u32 = h.read(this + 0x10cc);
            let slot: u32 = h.read(config.wrapping_add(0x8c));
            let tex = texture(h, slot);
            let height: u32 = h.read(this + 0x10dd4);
            let width: u32 = h.read(this + 0x10dd0);
            let renderer: u32 = h.read(this + 0xd0);
            let effect = h.call_sym(get, Conv::Cdecl, &[]);
            h.call_sym(
                init,
                Conv::Thiscall(effect),
                &[renderer, width, height, tex, slot],
            );
        }
        let config: u32 = h.read(this + 0x10cc);
        let first = texture(h, h.read(config.wrapping_add(0xac)));
        let second = texture(h, h.read(config.wrapping_add(0x1_232c)));
        let renderer: u32 = h.read(this + 0xd0);
        let blur = h.call_sym(MOTION_BLUR_INSTANCE, Conv::Cdecl, &[]);
        h.call_sym(
            MOTION_BLUR_INITIALIZE,
            Conv::Thiscall(blur),
            &[renderer, second, first],
        );
        let config: u32 = h.read(this + 0x10cc);
        let word: u32 = h.read(config.wrapping_add(0x90));
        let slot: u32 = h.read(config.wrapping_add(0x8c));
        let tex = texture(h, slot);
        let args = [
            h.read(this + 0xd0),
            h.read(this + 0xcc),
            tex,
            slot,
            word,
            h.read(this + 0x10dd0),
            h.read(this + 0x10dd4),
        ];
        let broken = h.call_sym(BROKEN_SCREEN_INSTANCE, Conv::Cdecl, &[]);
        h.call_sym(BROKEN_SCREEN_INITIALIZE, Conv::Thiscall(broken), &args);
        let lighting = h.call_sym(LIGHTING_INSTANCE, Conv::Cdecl, &[]);
        let device: u32 = h.read(this + 0xcc);
        let fog = h.call_sym(FOG_INSTANCE, Conv::Cdecl, &[]);
        h.call_sym(
            FOG_INITIALIZE,
            Conv::Thiscall(fog),
            &[device, this + 0x1198, lighting.wrapping_add(0x158)],
        );
        let renderer: u32 = h.read(this + 0xd0);
        let morph = h.call_sym(MORPH_INSTANCE, Conv::Cdecl, &[]);
        h.call_sym(MORPH_INITIALIZE, Conv::Thiscall(morph), &[renderer]);
        h.write(this + 0xf958, 1u8);
        sampler(h, this + 0xfaa8, [5, 1, 1, 1, 1]);
        sampler(h, this + 0xfacc, [5, 1, 2, 2, 2]);
        h.call_sym(SETUP_SAMPLERS, Conv::Thiscall(this), &[]);
        for (arr, reserve) in [
            (0x34, RESERVE_WORDS),
            (0x4c, RESERVE_WORDS),
            (0x58, RESERVE_FLAGS),
            (0x40, RESERVE_LAYOUTS),
        ] {
            if h.read::<u32>(this + arr + 4).cast_signed() < 200 {
                h.call_sym(reserve, Conv::Thiscall(this + arr), &[200]);
            }
        }
        for (arr, reserve) in [
            (0xfb18, RESERVE_FLAGS),
            (0xfb30, RESERVE_8080),
            (0xfb24, RESERVE_8000),
            (0x28, RESERVE_FLAGS),
            (0xfb48, RESERVE_7F80),
            (0xfb54, RESERVE_8280),
        ] {
            clear_array(h, this + arr, reserve);
        }
        for k in 0..7u32 {
            h.write(this + 0xfb60 + 4 * k, 0u32);
        }
        // the mesh-instance pools and their buffers
        let pool = this + 0x12d4;
        let a = h.call_sym(
            CLUSTER_CREATE,
            Conv::Thiscall(pool),
            &[3000, h.rebase(POOL_CLASSES[0]), 0],
        );
        h.write(this + 0xfb7c, a);
        let b = h.call_sym(
            CLUSTER_CREATE,
            Conv::Thiscall(pool),
            &[3000, h.rebase(POOL_CLASSES[1]), 0],
        );
        h.write(this + 0xfb90, b);
        let c = h.call_sym(
            CLUSTER_CREATE,
            Conv::Thiscall(pool),
            &[3000, h.rebase(POOL_CLASSES[2]), 0],
        );
        h.write(this + 0xfba4, if c == 0 { 0 } else { c.wrapping_sub(4) });
        for (field, size) in [(0xfbb8, 48_000), (0xfbcc, 0x6_0ae0), (0xfbd0, 0x1_a5e0)] {
            let p = h.call_sym(ALIGNED_MALLOC, Conv::Cdecl, &[size, 0x10]);
            h.write(this + field, p);
        }
        let d = h.call_sym(
            CLUSTER_CREATE,
            Conv::Thiscall(pool),
            &[3000, h.rebase(POOL_CLASSES[3]), 0],
        );
        h.write(this + 0xfbd4, d);
        let lists = [0xfb80, 0xfb94, 0xfba8, 0xfbbc, 0xfbd8];
        for arr in lists {
            size_free_list(h, this + arr);
        }
        for k in 0..3000u32 {
            for arr in lists {
                let data: u32 = h.read(this + arr + 8);
                h.write(data.wrapping_add(k), 0u8);
            }
        }
        for (field, size) in [(0xfbec, 96_000), (0xfbe8, 0xd_ac00)] {
            let p = h.call_sym(ALIGNED_MALLOC, Conv::Cdecl, &[size, 0x10]);
            h.write(this + field, p);
        }
        // the 4000 flag bytes (count +0xfbf0, capacity +0xfbf4, data +0xfbf8)
        if h.read::<u32>(this + 0xfbf0).cast_signed() < 4000
            && h.read::<u32>(this + 0xfbf4).cast_signed() < 4000
        {
            let old: u32 = h.read(this + 0xfbf8);
            if old == 0 {
                let p = h.call_sym(USER_MALLOC, Conv::Cdecl, &[4000]);
                h.write(this + 0xfbf8, p);
            } else {
                let p = h.call_sym(USER_MALLOC, Conv::Cdecl, &[4000]);
                if p != 0 {
                    let n: u32 = h.read(this + 0xfbf0);
                    let old: u32 = h.read(this + 0xfbf8);
                    h.call_sym(MEMCPY, Conv::Cdecl, &[p, old, n]);
                }
                let old: u32 = h.read(this + 0xfbf8);
                h.call_sym(USER_FREE, Conv::Cdecl, &[old]);
                h.write(this + 0xfbf8, p);
            }
            h.write(this + 0xfbf4, 4000u32);
        }
        h.write(this + 0xfbf0, 4000u32);
        for k in 0..4000u32 {
            let data: u32 = h.read(this + 0xfbf8);
            h.write(data.wrapping_add(k), 0u8);
        }
        for field in [0xfb8c, 0xfba0, 0xfbb4, 0xfbc8, 0xfbe4, 0xfbfc] {
            h.write(this + field, 0u32);
        }
        h.call_sym(CREATE_RWTT_TARGET, Conv::Thiscall(this), &[]);
        let player = this + 0xde8;
        h.call_sym(VIDEO_INITIALIZE, Conv::Thiscall(player), &[1, 0, 0]);
        h.call_sym(VIDEO_CLOSE, Conv::Thiscall(player), &[])
    }
}

//! The guide map, the field's minimap (`rcbgGuide*`, `Guide_*`, `Yn_GuideMap*`;
//! `docs/formats/field.md`, `docs/renderer.md` "Minimap").
//!
//! `g_YnMapSystem` points at Yn's field state: `+0` the map, `+4` the
//! guide object. The map's `+8` is its scene ([`crate::scene`]), which
//! also holds the minimap models and which one shows.

use ffx_bridge::Global;

/// Yn's field state (`Yn_GuideMapGetMapNo`, `Yn_BattleBegin`).
pub const YN_MAP_SYSTEM: Global<u32> = Global::new(0x0193_bf80);
/// The guide flags; bit 0: the minimap may show (`rcbgGuideSetDispProg`).
pub const FLAGS: Global<u32> = Global::new(0x00c5_e350);
/// Frames to wait before fading in or out (`.data`, 2).
pub const FADE_DELAY: Global<i32> = Global::new(0x00c5_e364);
/// Steps of a fade (`.data`, 6).
pub const FADE_STEPS: Global<i32> = Global::new(0x00c5_e368);
/// The fade's state ([`FadeState`]).
pub const FADE_STATE: Global<i32> = Global::new(0x0193_bf6c);
/// Frames waited so far.
pub const FADE_WAIT: Global<i32> = Global::new(0x0193_bf70);
/// Steps faded in, `0..=FADE_STEPS`.
pub const FADE_LEVEL: Global<i32> = Global::new(0x0193_bf74);
/// The minimap's alpha, `0..=128` (`g_GuideFade`).
pub const FADE: Global<i32> = Global::new(0x0193_bf78);
/// The guide's `+0x6c`, kept over a battle (`rcbgGuideBattleBegin`).
pub const BATTLE_SAVE: Global<u32> = Global::new(0x0193_bf7c);
/// Where the minimap sits: 0..=3 the corners, 4 the centre
/// (`rcbgGuideSetTarget`; script native 0x00f3).
pub const TARGET: Global<u32> = Global::new(0x00c5_e354);
/// The minimap's size in pixels (`rcbgGuideSetSize`): `0x80`, `0x100`
/// centred.
pub const SIZE: Global<u32> = Global::new(0x00c5_e358);
/// The minimap's centre, x then y (`rcbgGuideSetPos`).
pub const POS: Global<[u32; 2]> = Global::new(0x00c5_e35c);
/// 0: `rcbgGuideUpdate` returns at once (`Guide_SetEnabled`, from the
/// debug tweak menu).
pub const ENABLED: Global<u32> = Global::new(0x00c5_e478);

/// The sea's matrix (`Yn_SeaGetMatrix`, `Yn_SeaSetMatrix`).
pub const SEA_MATRIX: Global<[f32; 16]> = Global::new(0x0193_bf28);
/// 10.0, an `f64` in `.rdata`: the minimap model's origin scale
/// (`Guide_RevealFog`).
pub const FOG_SCALE: Global<f64> = Global::new(0x00b9_22b0);
/// 40.0, an `f64` in `.rdata`: a fog cell's size (`Guide_RevealFog`).
pub const FOG_CELL: Global<f64> = Global::new(0x00b5_af28);

/// Size and centre of the minimap for each [`TARGET`]
/// (`rcbgGuideSetTarget`'s cases).
pub const TARGETS: [(u32, [u32; 2]); 5] = [
    (0x80, [0x50, 0x40]),
    (0x80, [0x1b0, 0x40]),
    (0x80, [0x50, 0x160]),
    (0x80, [0x1b0, 0x160]),
    (0x100, [0x100, 0xd0]),
];

/// Offsets in Yn's field state.
pub mod system {
    /// `+0x0`: the map.
    pub const MAP: u32 = 0x0;
    /// `+0x4`: the guide object.
    pub const GUIDE: u32 = 0x4;
}

/// Offsets in the map.
pub mod map {
    /// `+0x8`: its scene ([`crate::scene::layout`]).
    pub const SCENE: u32 = 0x8;
}

/// Offsets in the guide object (`rcbgGuideNew`).
pub mod object {
    /// `+0x0`: the map's own minimap model (MAP1 chunk 0xb), or null.
    pub const MODEL: u32 = 0x0;
    /// `+0x8`: the model made from the walk mesh.
    pub const WALK_MODEL: u32 = 0x8;
    /// `+0x6c`: how far the view may move down, a float: the minimap's
    /// offset is clamped to minus it (`rcbgGuideUpdateEx`); set by a
    /// script native (`0x0091b740`), kept over battles.
    pub const LIMIT: u32 = 0x6c;
    /// `+0x70`: non-zero when the fog of war is kept (`Guide_RevealFog`).
    pub const FOG: u32 = 0x70;
    /// `+0x74`: the blink counter of the markers.
    pub const BLINK: u32 = 0x74;
}

/// Where the minimap's fade is (`Guide_FadeStep`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FadeState {
    /// 0: hidden, waiting [`FADE_DELAY`] frames before fading in.
    Hidden,
    /// 1: fading in, a step a frame.
    FadingIn,
    /// 2: shown, waiting before fading out once asked to hide.
    Shown,
    /// 3: fading out.
    FadingOut,
}

/// The fade's counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fade {
    /// [`FADE_STATE`]; values outside 0..=3 are kept (and do nothing).
    pub state: i32,
    /// [`FADE_WAIT`].
    pub wait: i32,
    /// [`FADE_LEVEL`].
    pub level: i32,
}

impl Fade {
    /// The state, when it is one of the four.
    #[must_use]
    pub fn state(self) -> Option<FadeState> {
        Some(match self.state {
            0 => FadeState::Hidden,
            1 => FadeState::FadingIn,
            2 => FadeState::Shown,
            3 => FadeState::FadingOut,
            _ => return None,
        })
    }

    /// The alpha for a level: `level * 128 / steps` (signed, truncated).
    #[must_use]
    pub fn alpha(self, steps: i32) -> i32 {
        (self.level << 7) / steps
    }

    /// One frame of `Guide_FadeStep(show)`: the counters after it.
    #[must_use]
    pub fn step(mut self, show: bool, delay: i32, steps: i32) -> Self {
        use FadeState::{FadingIn, FadingOut, Hidden, Shown};
        match (show, self.state()) {
            (true, Some(Hidden)) => {
                self.wait = self.wait.wrapping_add(1);
                if self.wait >= delay {
                    self.wait = 0;
                    self.state = 1;
                }
            }
            (true, Some(FadingIn)) => {
                if self.level < steps {
                    self.level = self.level.wrapping_add(1);
                }
                if self.level >= steps {
                    self.state = 2;
                }
            }
            (true, Some(FadingOut)) => self.state = 1,
            (false, Some(FadingIn)) => self.state = 3,
            (false, Some(Shown)) => {
                self.wait = self.wait.wrapping_add(1);
                if self.wait > delay {
                    self.wait = 0;
                    self.state = 3;
                }
            }
            (false, Some(FadingOut)) => {
                if self.level > 0 {
                    self.level -= 1;
                }
                if self.level <= 0 {
                    self.state = 0;
                }
            }
            _ => {}
        }
        self
    }
}

/// The guide map where the game keeps it, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::Mem;

    use super::{
        BATTLE_SAVE, ENABLED, FADE, FADE_DELAY, FADE_LEVEL, FADE_STATE, FADE_STEPS, FADE_WAIT,
        FLAGS, Fade, POS, SIZE, TARGET, TARGETS, YN_MAP_SYSTEM, map, object, system,
    };
    use crate::{scene, x87};
    use ffx_bridge::x87::{Reg, ftol};

    /// The guide object, when Yn's field state and it exist.
    fn yn_guide(mem: &impl Mem) -> Option<u32> {
        let s = YN_MAP_SYSTEM.get(mem);
        if s == 0 {
            return None;
        }
        Some(mem.read::<u32>(s + system::GUIDE)).filter(|&g| g != 0)
    }

    /// The scene, when Yn's field state, its map and the scene exist.
    fn yn_scene(mem: &impl Mem) -> Option<u32> {
        let s = YN_MAP_SYSTEM.get(mem);
        if s == 0 {
            return None;
        }
        let m = mem.read::<u32>(s + system::MAP);
        if m == 0 {
            return None;
        }
        Some(mem.read::<u32>(m + map::SCENE)).filter(|&c| c != 0)
    }

    /// `rcbgGuideSetPos(guide, x, y)` (`0x0091d900`): the minimap's centre
    /// (global; the guide is not read).
    pub fn rcbg_guide_set_pos(mem: &mut impl Mem, _guide: u32, x: u32, y: u32) {
        POS.set(mem, [x, y]);
    }

    /// `rcbgGuideSetSize(guide, size)` (`0x0091d920`).
    pub fn rcbg_guide_set_size(mem: &mut impl Mem, _guide: u32, size: u32) {
        SIZE.set(mem, size);
    }

    /// `rcbgGuideSetTarget(guide, target)` (`0x0091d930`): stores the
    /// target, then its size and centre (other values: only the target).
    pub fn rcbg_guide_set_target(mem: &mut impl Mem, g: u32, target: u32) {
        TARGET.set(mem, target);
        if let Some(&(size, [x, y])) = TARGETS.get(target as usize) {
            rcbg_guide_set_size(mem, g, size);
            rcbg_guide_set_pos(mem, g, x, y);
        }
    }

    /// `rcbgGuideGetTarget()` (`0x0091d480`).
    pub fn rcbg_guide_get_target(mem: &impl Mem) -> u32 {
        TARGET.get(mem)
    }

    /// `rcbgGuideSetFlags(guide, bits)` (`0x0091d430`).
    pub fn rcbg_guide_set_flags(mem: &mut impl Mem, _guide: u32, bits: u32) {
        let f = FLAGS.get(mem);
        FLAGS.set(mem, f | bits);
    }

    /// `rcbgGuideClearFlags(guide, bits)` (`0x0091d420`).
    pub fn rcbg_guide_clear_flags(mem: &mut impl Mem, _guide: u32, bits: u32) {
        let f = FLAGS.get(mem);
        FLAGS.set(mem, f & !bits);
    }

    /// `rcbgGuideSetFlag3(guide, on)` (`0x0091d510`): flag bit 3.
    pub fn rcbg_guide_set_flag3(mem: &mut impl Mem, _guide: u32, on: i32) {
        let f = FLAGS.get(mem);
        FLAGS.set(mem, if on != 0 { f | 8 } else { f & !8 });
    }

    /// `rcbgGuideSetFlag8(guide, on)` (`0x0091d530`): flag bit 8 (read by
    /// `rcbgGuideUpdateEx`).
    pub fn rcbg_guide_set_flag8(mem: &mut impl Mem, _guide: u32, on: i32) {
        let f = FLAGS.get(mem);
        FLAGS.set(mem, if on != 0 { f | 0x100 } else { f & !0x100 });
    }

    /// `rcbgGuideGetScreen(guide, scale, offset)` (`0x0091da30`): the
    /// minimap's scale (size / 256) and its centre's offset from the
    /// screen centre (256, 208), as floats.
    pub fn rcbg_guide_get_screen(mem: &mut impl Mem, _guide: u32, scale: u32, offset: u32) {
        #[allow(clippy::cast_precision_loss)] // fild, then fstp m32
        let f = |v: u32| f64::from(v.cast_signed() as f32);
        let size = f(SIZE.get(mem));
        mem.write(scale, x87::store(size * (1.0 / 256.0)));
        let [x, y] = POS.get(mem).map(f);
        mem.write(offset, [x87::store(x - 256.0), x87::store(y - 208.0)]);
    }

    /// `rcbgGuideGetLimit(guide)` (`0x0091d450`).
    pub fn rcbg_guide_get_limit(mem: &impl Mem, g: u32) -> f32 {
        f32::from_bits(mem.read(g + object::LIMIT))
    }

    /// `rcbgGuideSetLimit(guide, limit)` (`0x0091d580`), the float as bits.
    pub fn rcbg_guide_set_limit(mem: &mut impl Mem, g: u32, limit: u32) {
        mem.write(g + object::LIMIT, x87::copy(limit));
    }

    /// `Guide_SetEnabled(on)` (`0x0091f640`).
    pub fn guide_set_enabled(mem: &mut impl Mem, on: u32) {
        ENABLED.set(mem, on);
    }

    /// `Yn_GuideMapSetMode(target)` (`0x00919e20`).
    pub fn yn_guide_map_set_mode(mem: &mut impl Mem, target: u32) {
        if let Some(g) = yn_guide(mem) {
            rcbg_guide_set_target(mem, g, target);
        }
    }

    /// `Yn_GuideMapSetFlags(bits)` (`0x00919c30`).
    pub fn yn_guide_map_set_flags(mem: &mut impl Mem, bits: u32) {
        if let Some(g) = yn_guide(mem) {
            rcbg_guide_set_flags(mem, g, bits);
        }
    }

    /// `Yn_GuideMapClearFlags(bits)` (`0x00919c00`).
    pub fn yn_guide_map_clear_flags(mem: &mut impl Mem, bits: u32) {
        if let Some(g) = yn_guide(mem) {
            rcbg_guide_clear_flags(mem, g, bits);
        }
    }

    /// `Yn_GuideMapSetFlag3(on)` (`0x00919d40`).
    pub fn yn_guide_map_set_flag3(mem: &mut impl Mem, on: i32) {
        if let Some(g) = yn_guide(mem) {
            rcbg_guide_set_flag3(mem, g, on);
        }
    }

    /// `Yn_GuideMapGetLimit()` (`0x00919ca0`): 0 without a guide.
    pub fn yn_guide_map_get_limit(mem: &impl Mem) -> f32 {
        yn_guide(mem).map_or(0.0, |g| rcbg_guide_get_limit(mem, g))
    }

    /// `Yn_GuideMapSetLimit(limit)` (`0x00919df0`), the float as bits.
    pub fn yn_guide_map_set_limit(mem: &mut impl Mem, limit: u32) {
        if let Some(g) = yn_guide(mem) {
            rcbg_guide_set_limit(mem, g, x87::copy(limit));
        }
    }

    /// `Yn_GuideMapSetPos(x, y)` (`0x00919e50`).
    pub fn yn_guide_map_set_pos(mem: &mut impl Mem, x: u32, y: u32) {
        if let Some(g) = yn_guide(mem) {
            rcbg_guide_set_pos(mem, g, x, y);
        }
    }

    /// `Yn_GuideMapSetSize(size)` (`0x00919e80`).
    pub fn yn_guide_map_set_size(mem: &mut impl Mem, size: u32) {
        if let Some(g) = yn_guide(mem) {
            rcbg_guide_set_size(mem, g, size);
        }
    }

    /// The scene, or 0 when Yn's field state has a map without one (the
    /// setters below then write near address 0, as the original).
    fn yn_scene_or_null(mem: &impl Mem) -> Option<u32> {
        let s = YN_MAP_SYSTEM.get(mem);
        if s == 0 {
            return None;
        }
        let m = mem.read::<u32>(s + system::MAP);
        Some(if m == 0 {
            0
        } else {
            mem.read::<u32>(m + map::SCENE)
        })
    }

    fn yn_scene_float(mem: &impl Mem, offset: u32) -> f32 {
        yn_scene(mem).map_or(0.0, |c| f32::from_bits(x87::copy(mem.read(c + offset))))
    }

    /// `Yn_SceneGetAC()` (`0x0091a6d0`): the scene's float at `+0xac`, 0.0
    /// without a scene.
    pub fn yn_scene_get_ac(mem: &impl Mem) -> f32 {
        yn_scene_float(mem, 0xac)
    }

    /// `Yn_SceneGetB8()` (`0x0091a700`): `+0xb8`.
    pub fn yn_scene_get_b8(mem: &impl Mem) -> f32 {
        yn_scene_float(mem, 0xb8)
    }

    /// `Yn_SceneGetBC()` (`0x0091a730`): `+0xbc`.
    pub fn yn_scene_get_bc(mem: &impl Mem) -> f32 {
        yn_scene_float(mem, 0xbc)
    }

    /// `Yn_SceneSetC0(a, b, c, d, e)` (`0x0091ad80`, script native
    /// `0x0091b550`), as `args`: scene `+0xc0..=+0xd0`, `b` a float; `+0xd4` = 0.0,
    /// `+0xd8` = 0.
    pub fn yn_scene_set_c0(mem: &mut impl Mem, args: [u32; 5]) {
        if let Some(sc) = yn_scene_or_null(mem) {
            mem.write(sc.wrapping_add(0xc0), args[0]);
            mem.write(sc.wrapping_add(0xc4), x87::copy(args[1]));
            mem.write(sc.wrapping_add(0xc8), args[2]);
            mem.write(sc.wrapping_add(0xd4), 0u32);
            mem.write(sc.wrapping_add(0xcc), args[3]);
            mem.write(sc.wrapping_add(0xd0), args[4]);
            mem.write(sc.wrapping_add(0xd8), 0u32);
        }
    }

    /// `Yn_SceneSetDC(a, b, c, d)` (`0x0091aa00`, script native
    /// `0x0091b610`): scene `+0xdc..=+0xe8`, `b` and `d` floats; `+0xec` =
    /// 0.0, `+0xf0` = 0.
    pub fn yn_scene_set_dc(mem: &mut impl Mem, a: u32, b: u32, c: u32, d: u32) {
        if let Some(sc) = yn_scene_or_null(mem) {
            mem.write(sc.wrapping_add(0xe0), x87::copy(b));
            mem.write(sc.wrapping_add(0xdc), a);
            mem.write(sc.wrapping_add(0xe8), x87::copy(d));
            mem.write(sc.wrapping_add(0xe4), c);
            mem.write(sc.wrapping_add(0xf0), 0u32);
            mem.write(sc.wrapping_add(0xec), 0u32);
        }
    }

    /// `Yn_SceneSet104(v)` (`0x0091a260`, script native `0x0091af80`).
    pub fn yn_scene_set_104(mem: &mut impl Mem, v: u32) {
        if let Some(sc) = yn_scene_or_null(mem) {
            mem.write(sc.wrapping_add(0x104), v);
        }
    }

    /// `Yn_PSimGetCnt()` (`0x00919ef0`): always one particle simulation.
    #[must_use]
    pub const fn yn_psim_get_cnt() -> u32 {
        1
    }

    /// `Yn_SeaGetMatrix()` (`0x0091ae00`): the sea's matrix.
    pub fn yn_sea_get_matrix(mem: &impl Mem) -> u32 {
        super::SEA_MATRIX.ptr(mem).addr
    }

    /// `Yn_MapPartCount()` (`0x0091a3d0`): the scene's layers, 0 without
    /// a scene.
    pub fn yn_map_part_count(mem: &impl Mem) -> i32 {
        yn_scene(mem).map_or(0, |c| mem.read(c + scene::layout::LAYER_COUNT))
    }

    /// Bit 1 of layer `part`'s animation, when `part` is below the count
    /// (`Yn_MapPartAnimeOn`/`Off`).
    fn map_part_anime(mem: &mut impl Mem, part: i32, on: bool) {
        if part >= yn_map_part_count(mem) {
            return;
        }
        // the count is not 0, so the scene exists
        let c = yn_scene(mem).unwrap_or(0);
        let layers = mem.read::<u32>(c + scene::layout::LAYERS);
        #[allow(clippy::cast_sign_loss)] // a negative part indexes back, as the original
        let at = layers
            .wrapping_add((part as u32).wrapping_mul(scene::layer::SIZE))
            .wrapping_add(scene::layer::ANIME);
        let f = mem.read::<u32>(at);
        mem.write(at, if on { f | 2 } else { f & !2 });
    }

    /// `Yn_MapPartAnimeOn(part)` (`0x00919f90`).
    pub fn yn_map_part_anime_on(mem: &mut impl Mem, part: i32) {
        map_part_anime(mem, part, true);
    }

    /// `Yn_MapPartAnimeOff(part)` (`0x00919f40`).
    pub fn yn_map_part_anime_off(mem: &mut impl Mem, part: i32) {
        map_part_anime(mem, part, false);
    }

    /// `Yn_SeaResetMatrix()` (`0x0091ae30`): the identity; returns
    /// `FFXVu0UnitMatrix`'s `eax`.
    pub fn yn_sea_reset_matrix(mem: &mut impl Mem) -> u32 {
        let m = super::SEA_MATRIX.ptr(mem).addr;
        ffx_ps2::vu0::bridge::matrix_identity(mem, m)
    }

    /// `Yn_SeaSetMatrix(m)` (`0x0091ae60`): `FFXVu0CopyMatrix`.
    pub fn yn_sea_set_matrix(mem: &mut impl Mem, m: u32) -> u32 {
        let at = super::SEA_MATRIX.ptr(mem).addr;
        ffx_ps2::vu0::bridge::matrix_copy(mem, at, m)
    }

    /// `Yn_PreBgLayerDispGet(part)` (`0x0091a770`): layer `part`'s first
    /// word (its display flags) for a part below the count, else 0. With
    /// no map the original reads the layers pointer at address 8, as here.
    pub fn yn_pre_bg_layer_disp_get(mem: &impl Mem, part: i32) -> u32 {
        if part >= yn_map_part_count(mem) {
            return 0;
        }
        let s = YN_MAP_SYSTEM.get(mem);
        let m = if s == 0 {
            0
        } else {
            mem.read::<u32>(s + system::MAP)
        };
        let scene = if m == 0 {
            0
        } else {
            mem.read::<u32>(m + map::SCENE)
        };
        let layers = mem.read::<u32>(scene.wrapping_add(scene::layout::LAYERS));
        #[allow(clippy::cast_sign_loss)] // a negative part indexes back, as the original
        let at = layers.wrapping_add((part as u32).wrapping_mul(scene::layer::SIZE));
        mem.read(at)
    }

    /// `Guide_RevealFog(guide, x, z)` (`0x0091f420`): marks the 13 cells
    /// of a diamond of radius 2 around the player's cell as seen in the
    /// guide's 64 x 64 fog bitmap (`+0x70`). The cell is
    /// `((x - ox * 10) / 40, (oz * 10 - z) / 40)` (`ox`, `oz` the minimap
    /// model's `+0x90` and `+0xa8`; 10.0 and 40.0 `f64` constants in
    /// `.rdata`), through `_ftol2_sse`, each coordinate clamped to 0..=63.
    /// Returns the z cell (what `eax` holds).
    pub fn guide_reveal_fog(mem: &mut impl Mem, g: u32, x: f32, z: f32) -> u32 {
        const DX: [i8; 13] = [0, -1, 0, 1, -2, -1, 0, 1, 2, -1, 0, 1, 0];
        const DZ: [i8; 13] = [-2, -1, -1, -1, 0, 0, 0, 0, 0, 1, 1, 1, 2];
        let o = guide_mesh_object(mem, g);
        let k = Reg::constant(super::FOG_SCALE.get(mem));
        let ox = (Reg::load(mem.read::<f32>(o + 0x90)) * k).store();
        let oz = k.mul_m32(mem.read::<f32>(o + 0xa8)).store();
        let cell = Reg::constant(super::FOG_CELL.get(mem));
        let ix = ftol(mem, (Reg::load(x).sub_m32(ox) / cell).value());
        let iz = ftol(mem, (Reg::load(oz).sub_m32(z) / cell).value());
        let fog = mem.read::<u32>(g + object::FOG);
        for (dx, dz) in DX.into_iter().zip(DZ) {
            let clamp = |v: u32| -> u32 {
                #[allow(clippy::cast_possible_wrap)]
                let v = v as i32;
                #[allow(clippy::cast_sign_loss)]
                if v < 0 { 0 } else { v.min(0x3f) as u32 }
            };
            #[allow(clippy::cast_sign_loss)]
            let cx = clamp(ix.wrapping_add(i32::from(dx) as u32));
            #[allow(clippy::cast_sign_loss)]
            let cz = clamp(iz.wrapping_add(i32::from(dz) as u32));
            let bit = cz * 64 + cx;
            let at = fog.wrapping_add(bit >> 3);
            let b = mem.read::<u8>(at);
            mem.write(at, b | (1 << (bit & 7)));
        }
        iz
    }

    /// `Guide_MeshObject(guide)` (`0x0091d460`): the map's own minimap
    /// model, else the one made from the walk mesh.
    pub fn guide_mesh_object(mem: &impl Mem, g: u32) -> u32 {
        match mem.read::<u32>(g + object::MODEL) {
            0 => mem.read(g + object::WALK_MODEL),
            m => m,
        }
    }

    /// `Guide_FadeStep(show)` (`0x0091d280`).
    pub fn guide_fade_step(mem: &mut impl Mem, show: i32) {
        let old = Fade {
            state: FADE_STATE.get(mem),
            wait: FADE_WAIT.get(mem),
            level: FADE_LEVEL.get(mem),
        };
        let steps = FADE_STEPS.get(mem);
        let new = old.step(show != 0, FADE_DELAY.get(mem), steps);
        if new.wait != old.wait {
            FADE_WAIT.set(mem, new.wait);
        }
        if new.level != old.level {
            FADE_LEVEL.set(mem, new.level);
        }
        if new.state != old.state {
            FADE_STATE.set(mem, new.state);
        }
        FADE.set(mem, new.alpha(steps));
    }

    /// `Guide_FadeOff()` (`0x0091d3a0`): hidden at once.
    pub fn guide_fade_off(mem: &mut impl Mem) {
        for g in [FADE_STATE, FADE_WAIT, FADE_LEVEL, FADE] {
            g.set(mem, 0);
        }
    }

    /// `rcbgGuideSetDispProg(guide, on)` (`0x0091d560`): whether the
    /// minimap may show (flag bit 0; the guide is not read).
    pub fn rcbg_guide_set_disp_prog(mem: &mut impl Mem, _guide: u32, on: i32) {
        let f = FLAGS.get(mem);
        FLAGS.set(mem, if on != 0 { f | 1 } else { f & !1 });
    }

    /// `rcbgGuideBattleBegin(guide)` (`0x0091ccc0`): keeps the guide's
    /// `+0x6c` over the battle.
    pub fn rcbg_guide_battle_begin(mem: &mut impl Mem, g: u32) {
        let v = x87::copy(mem.read(g + object::LIMIT));
        BATTLE_SAVE.set(mem, v);
    }

    /// `rcbgGuideBattleEnd(guide)` (`0x0091cce0`): puts it back.
    pub fn rcbg_guide_battle_end(mem: &mut impl Mem, g: u32) {
        let v = x87::copy(BATTLE_SAVE.get(mem));
        mem.write(g + object::LIMIT, v);
    }

    /// `Yn_BattleBegin()` (`0x00919a20`): for the guide, if Yn's field
    /// state exists (the guide itself is not checked).
    pub fn yn_battle_begin(mem: &mut impl Mem) {
        let s = YN_MAP_SYSTEM.get(mem);
        if s != 0 {
            let g = mem.read(s + system::GUIDE);
            rcbg_guide_battle_begin(mem, g);
        }
    }

    /// `Yn_BattleEnd()` (`0x00919a40`).
    pub fn yn_battle_end(mem: &mut impl Mem) {
        let s = YN_MAP_SYSTEM.get(mem);
        if s != 0 {
            let g = mem.read(s + system::GUIDE);
            rcbg_guide_battle_end(mem, g);
        }
    }

    /// `Yn_GuideMapSetDispProg(on)` (`0x00919d70`).
    pub fn yn_guide_map_set_disp_prog(mem: &mut impl Mem, on: i32) {
        let s = YN_MAP_SYSTEM.get(mem);
        if s == 0 {
            return;
        }
        let g = mem.read(s + system::GUIDE);
        if g != 0 {
            rcbg_guide_set_disp_prog(mem, g, on);
        }
    }

    /// `Yn_GuideMapGetMapNo()` (`0x00919c70`): the guide map shown, or -1
    /// without a map, guide or guide map list.
    pub fn yn_guide_map_get_map_no(mem: &impl Mem) -> i32 {
        let s = YN_MAP_SYSTEM.get(mem);
        if s == 0 || mem.read::<u32>(s + system::GUIDE) == 0 {
            return -1;
        }
        let m = mem.read::<u32>(s + system::MAP);
        if m == 0 {
            return -1;
        }
        match mem.read::<u32>(m + map::SCENE) {
            0 => -1,
            l => i32::from(mem.read::<i16>(l + scene::layout::GUIDE_CURRENT)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fade_goes_in_and_out() {
        let mut f = Fade {
            state: 0,
            wait: 0,
            level: 0,
        };
        let mut alphas = Vec::new();
        for _ in 0..10 {
            f = f.step(true, 2, 6);
            alphas.push(f.alpha(6));
        }
        assert_eq!(alphas, [0, 0, 21, 42, 64, 85, 106, 128, 128, 128]);
        assert_eq!(f.state(), Some(FadeState::Shown));
        for _ in 0..3 {
            f = f.step(false, 2, 6);
        }
        assert_eq!(f.state(), Some(FadeState::FadingOut));
    }
}

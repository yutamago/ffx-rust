//! The guide map, the field's minimap (`rcbgGuide*`, `Guide_*`, `Yn_GuideMap*`;
//! `docs/formats/field.md`, `docs/renderer.md` "Minimap").
//!
//! `g_YnMapSystem` points at Yn's field state: `+0` the map, `+4` the
//! guide object. The map's `+8` is its scene ([`crate::scene`]), which
//! also holds the minimap models and which one shows.

use ffx_bridge::Global;

/// Yn's field state (`Yn_GuideMapGetMapNo`, `Yn_BattleBegin`).
pub const YN_MAP_SYSTEM: Global<u32> = Global::new(0x0193_bf80);
/// The field's ID map (walk mesh), what `Sg_IdGetPtr` (`0x0083e970`)
/// returns.
pub const ID_MAP: Global<u32> = Global::new(0x0130_1a94);
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
/// `0x0086c1f0()`: the map the current event jumps to
/// (`rcbgGuideSetWalkFog`).
pub const EVENT_MAP_NO: u32 = 0x0086_c1f0;
/// `MsBattleGetNum()` (`0x007816b0`).
pub const MS_BATTLE_GET_NUM: u32 = 0x0078_16b0;
/// `MsDebugDrawBoard()` (`0x0092cf00`).
pub const MS_DEBUG_DRAW_BOARD: u32 = 0x0092_cf00;
/// `MsGetSaveWalkMap` (`0x00785400`): the saved walk maps.
pub const SAVE_WALK_MAP: u32 = 0x0078_5400;
/// 350.0 (`f32`, `.rdata`): a new guide's view limit (`rcbgGuideInit`).
pub const GUIDE_DEFAULT_LIMIT: u32 = 0x00b5_ef50;
/// The background matrices `rcbgInit` resets: `g_RcbgWorldMatrix`,
/// `g_RcbgScreenWorldMatrix`, `g_RcbgClipMatrix`, `g_RcbgClipWorldMatrix`.
pub const RCBG_MATRICES: [u32; 4] = [0x025d_59a0, 0x025d_5920, 0x025d_58e0, 0x025d_58a0];
/// `0x0092cef0`, where `rcbgInit` goes on in debug builds.
pub const RCBG_DEBUG_INIT: u32 = 0x0092_cef0;
/// `rcbgGuideSetModelSub` (`0x0091d880`).
pub const GUIDE_SET_MODEL_SUB: u32 = 0x0091_d880;
/// `rcbgGuideSetModelFromIdMap` (`0x0091d5e0`).
pub const GUIDE_SET_MODEL_FROM_ID_MAP: u32 = 0x0091_d5e0;
/// `0x00639d90(n)`: tells the renderer the scene number.
pub const GRAPHIC_SCENE_NO: u32 = 0x0063_9d90;
/// `graphicSet2DStartLayer` (`0x00642d90`).
pub const GRAPHIC_SET_2D_START_LAYER: u32 = 0x0064_2d90;
/// The low word of the guide's VFX instance handle (`rcbgGuideFini`).
pub const GUIDE_VFX: Global<u32> = Global::new(0x0230_fd20);
/// `graphicVFXGetRenderDataByInstance` (cdecl, a 64-bit handle).
pub const VFX_GET_RENDER_DATA: u32 = 0x0064_53d0;
/// `graphicVFXDestroy` (cdecl, a 64-bit handle).
pub const VFX_DESTROY: u32 = 0x0064_5280;
/// 0.5 (`f64`, `.rdata`; `g_Vu0FixedRound`): `Yn_GuideEdgeFade`'s half.
pub const EDGE_HALF: u32 = 0x00b9_2220;
/// 0.15 (`f64` of an `f32`, `.rdata`): where `Yn_GuideEdgeFade` starts.
pub const EDGE_INNER: u32 = 0x00b5_9870;
/// 0.2 (`f64` of an `f32`, `.rdata`): where it reaches 1.
pub const EDGE_OUTER: u32 = 0x00b3_eb60;
/// 1024.0 (`f32`, `.rdata`): the camera distance `rcbgPreFini` resets.
pub const CAMERA_DISTANCE: u32 = 0x00b5_5178;
/// `MsSetCameraMatrix` (cdecl, six words).
pub const MS_SET_CAMERA_MATRIX: u32 = 0x007c_06d0;
/// `sceDevVif0Reset` (`0x0092cce0`), which `rcbgFini` calls in debug
/// builds.
pub const VIF0_RESET: u32 = 0x0092_cce0;

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

    /// `rcbgGuideUpdateEx(guide, show)` (`0x0091da80`).
    pub const GUIDE_UPDATE_EX: u32 = 0x0091_da80;

    /// `rcbgGuideUpdate(guide)` (`0x0091da10`): `rcbgGuideUpdateEx(guide,
    /// 1)`, whose `eax` it returns.
    pub fn rcbg_guide_update<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        guide: u32,
        call: &mut C,
    ) -> u32 {
        let f = mem.rebase(GUIDE_UPDATE_EX);
        call(mem, f, &[guide, 1])
    }

    /// `Yn_GuideMapProcEx(show)` (`0x00919d00`): the Yn guide map
    /// (`g_YnMapSystem +4`), when there is one, updated
    /// (`rcbgGuideUpdateEx(guide, show)`, whose `eax` it returns); else 0.
    pub fn yn_guide_map_proc_ex<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        show: u32,
        call: &mut C,
    ) -> u32 {
        let s = YN_MAP_SYSTEM.get(mem);
        if s == 0 {
            return 0;
        }
        let guide = mem.read::<u32>(s.wrapping_add(system::GUIDE));
        if guide == 0 {
            return 0;
        }
        let f = mem.rebase(GUIDE_UPDATE_EX);
        call(mem, f, &[guide, show])
    }

    /// The renderer's layer offset call `0x00639d40(i, dx, dy)` (render).
    pub const MOVE_LAYER: u32 = 0x0063_9d40;
    /// The map (`g_GraphicMapId` 0x17e) and layer (0x1a) whose y
    /// [`yn_map_part_set_offset`] nudges.
    pub const NUDGED_LAYER: [u32; 2] = [0x17e, 0x1a];
    /// `g_GraphicMapId` (`0x01330844`).
    pub const GRAPHIC_MAP_ID: u32 = 0x0133_0844;

    /// The layer `part` of the Yn scene: the scene's layer array (`+8`)
    /// plus `part` layers (the scene 0 without a system or map, as the
    /// original reads it).
    fn yn_layer_at(mem: &impl Mem, part: u32) -> u32 {
        let c = yn_scene_or_zero(mem);
        let layers = mem.read::<u32>(c.wrapping_add(scene::layout::LAYERS));
        layers.wrapping_add(part.wrapping_mul(scene::layer::SIZE))
    }

    /// The Yn scene, or 0 without a system or map (not filtered for a null
    /// scene, as the originals read it).
    fn yn_scene_or_zero(mem: &impl Mem) -> u32 {
        let s = YN_MAP_SYSTEM.get(mem);
        let m = if s == 0 {
            0
        } else {
            mem.read::<u32>(s + system::MAP)
        };
        if m == 0 {
            0
        } else {
            mem.read::<u32>(m + map::SCENE)
        }
    }

    /// `Yn_MapPartSetOffset_0091ab30(part, x, y)` (`0x0091ab30`, name ours):
    /// for a `part` below the count ([`yn_map_part_count`]) the renderer's
    /// layer moves by the change (`0x00639d40(part, x - +0xc, y - +0x10)`)
    /// and the layer keeps the new `x`, `y` (`+0xc`, `+0x10`). On map 0x17e
    /// layer 0x1a's `y` is nudged: one down when `(y - 0x54) / 18` (signed)
    /// is 0, one up when it is 14. Returns the renderer call's `eax`, else
    /// the count.
    pub fn yn_map_part_set_offset<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        [part, x, y]: [u32; 3],
        call: &mut C,
    ) -> u32 {
        let count = yn_map_part_count(mem);
        if part.cast_signed() >= count {
            return count.cast_unsigned();
        }
        let layer = yn_layer_at(mem, part);
        let [map_id, nudged] = NUDGED_LAYER;
        let mut y = y.cast_signed();
        if part == nudged && mem.read::<u32>(mem.rebase(GRAPHIC_MAP_ID)) == map_id {
            match (y.wrapping_sub(0x54)) / 18 {
                0 => y = y.wrapping_sub(1),
                14 => y = y.wrapping_add(1),
                _ => {}
            }
        }
        let y = y.cast_unsigned();
        let dx = x.wrapping_sub(mem.read::<u32>(layer.wrapping_add(0xc)));
        let dy = y.wrapping_sub(mem.read::<u32>(layer.wrapping_add(0x10)));
        let f = mem.rebase(MOVE_LAYER);
        let eax = call(mem, f, &[part, dx, dy]);
        mem.write(layer.wrapping_add(0x10), y);
        mem.write(layer.wrapping_add(0xc), x);
        eax
    }

    /// `Yn_MapPartSet44_0091abc0(part, x, y)` (`0x0091abc0`): for a `part`
    /// below the count, `x` and `y` (`f32`s) go to the layer's `+0x44` and
    /// `+0x48` unless 0.0 (a NaN is stored), its first tween stops (`+0x78`,
    /// `+0x74` = 0) and the renderer gets the position (`0x00639d60(part,
    /// +4, +8, x, y)`, whose `eax` it returns); else the count.
    pub fn yn_map_part_set44<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        [part, x, y]: [u32; 3],
        call: &mut C,
    ) -> u32 {
        let count = yn_map_part_count(mem);
        if part.cast_signed() >= count {
            return count.cast_unsigned();
        }
        let layer = yn_layer_at(mem, part);
        let (x, y) = (x87::copy(x), x87::copy(y));
        // `fucom` with 0.0: only an equal compare skips the store
        let zero = |v: u32| f32::from_bits(v) == 0.0;
        if !zero(x) {
            mem.write(layer.wrapping_add(0x44), x);
        }
        if !zero(y) {
            mem.write(layer.wrapping_add(0x48), y);
        }
        mem.write(layer.wrapping_add(0x78), 0u32);
        mem.write(layer.wrapping_add(0x74), 0u32);
        let [a, b] = [4u32, 8].map(|o| mem.read::<u32>(layer.wrapping_add(o)));
        let f = mem.rebase(crate::scene::bridge::SET_LAYER_POS);
        call(mem, f, &[part, a, b, x, y])
    }

    /// `graphicChangeLayerAlpha(layer, alpha)` (`0x00639d20`, render).
    pub const GRAPHIC_CHANGE_LAYER_ALPHA: u32 = 0x0063_9d20;

    /// `Yn_MapPartSetAlpha_0091aa90(part, alpha)` (`0x0091aa90`, name
    /// ours): for a `part` below the count ([`yn_map_part_count`]) the
    /// layer array is taken (the scene's `+8`), the renderer's layer gets
    /// the alpha (`graphicChangeLayerAlpha(part, alpha)`) and the layer's
    /// `+0x43` byte the low byte of `alpha`. Returns the renderer call's
    /// `eax`, else the count.
    pub fn yn_map_part_set_alpha<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        [part, alpha]: [u32; 2],
        call: &mut C,
    ) -> u32 {
        let count = yn_map_part_count(mem);
        if part.cast_signed() >= count {
            return count.cast_unsigned();
        }
        // a negative part with no scene reads address 8, as the original
        let c = yn_scene(mem).unwrap_or(0);
        let layers = mem.read::<u32>(c.wrapping_add(scene::layout::LAYERS));
        let f = mem.rebase(GRAPHIC_CHANGE_LAYER_ALPHA);
        let eax = call(mem, f, &[part, alpha]);
        let at = layers.wrapping_add(part.wrapping_mul(scene::layer::SIZE));
        #[allow(clippy::cast_possible_truncation)] // the low byte
        mem.write(at.wrapping_add(0x43), alpha as u8);
        eax
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

    // ---- round 5: map part (scene layer) accessors of the script natives ----

    /// Layer `part` for a part below [`yn_map_part_count`]. Without a map
    /// the originals read the layer list at address 8, as here.
    fn yn_layer(mem: &impl Mem, part: i32) -> Option<u32> {
        if part >= yn_map_part_count(mem) {
            return None;
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
        Some(layers.wrapping_add((part as u32).wrapping_mul(scene::layer::SIZE)))
    }

    fn part_word(mem: &impl Mem, part: i32, offset: u32) -> u32 {
        yn_layer(mem, part).map_or(0, |l| mem.read(l.wrapping_add(offset)))
    }

    fn part_float(mem: &impl Mem, part: i32, offset: u32) -> f32 {
        yn_layer(mem, part).map_or(0.0, |l| {
            f32::from_bits(x87::copy(mem.read(l.wrapping_add(offset))))
        })
    }

    /// `Yn_MapPartGetWidth(part)` (`0x0091a620`): the layer's `+4`, 0 past
    /// the count.
    pub fn yn_map_part_get_width(mem: &impl Mem, part: i32) -> u32 {
        part_word(mem, part, 4)
    }

    /// `Yn_MapPartGetHeight(part)` (`0x0091a3f0`): `+8`.
    pub fn yn_map_part_get_height(mem: &impl Mem, part: i32) -> u32 {
        part_word(mem, part, 8)
    }

    /// `Yn_MapPartGet0C(part)` (`0x0091a440`).
    pub fn yn_map_part_get_0c(mem: &impl Mem, part: i32) -> u32 {
        part_word(mem, part, 0xc)
    }

    /// `Yn_MapPartGet10(part)` (`0x0091a490`).
    pub fn yn_map_part_get_10(mem: &impl Mem, part: i32) -> u32 {
        part_word(mem, part, 0x10)
    }

    /// `Yn_MapPartGet24(part)` (`0x0091a580`).
    pub fn yn_map_part_get_24(mem: &impl Mem, part: i32) -> u32 {
        part_word(mem, part, 0x24)
    }

    /// `Yn_MapPartGet28(part)` (`0x0091a5d0`).
    pub fn yn_map_part_get_28(mem: &impl Mem, part: i32) -> u32 {
        part_word(mem, part, 0x28)
    }

    /// `Yn_MapPartAnimeGetCount(part)` (`0x0091a2a0`): the animation's
    /// frame count (`+0x98`).
    pub fn yn_map_part_anime_get_count(mem: &impl Mem, part: i32) -> u32 {
        part_word(mem, part, 0x98)
    }

    /// `Yn_MapPartGetAlpha(part)` (`0x0091a2e0`): the colour's alpha byte
    /// (`+0x43`).
    pub fn yn_map_part_get_alpha(mem: &impl Mem, part: i32) -> u32 {
        yn_layer(mem, part).map_or(0, |l| u32::from(mem.read::<u8>(l.wrapping_add(0x43))))
    }

    /// `Yn_MapPartGet14(part)` (`0x0091a670`): a float, 0.0 past the count.
    pub fn yn_map_part_get_14(mem: &impl Mem, part: i32) -> f32 {
        part_float(mem, part, 0x14)
    }

    /// `Yn_MapPartGetScaleX(part)` (`0x0091a530`): `+0x44`.
    pub fn yn_map_part_get_scale_x(mem: &impl Mem, part: i32) -> f32 {
        part_float(mem, part, 0x44)
    }

    /// `Yn_MapPartGetScaleY(part)` (`0x0091a4e0`): `+0x48`.
    pub fn yn_map_part_get_scale_y(mem: &impl Mem, part: i32) -> f32 {
        part_float(mem, part, 0x48)
    }

    /// `Yn_MapPartGetPosX(part)` (`0x0091a330`): `+0x4c`.
    pub fn yn_map_part_get_pos_x(mem: &impl Mem, part: i32) -> f32 {
        part_float(mem, part, 0x4c)
    }

    /// `Yn_MapPartGetPosY(part)` (`0x0091a380`): `+0x50`.
    pub fn yn_map_part_get_pos_y(mem: &impl Mem, part: i32) -> f32 {
        part_float(mem, part, 0x50)
    }

    /// `Yn_MapPartSetSize(part, w, h)` (`0x0091ac80`): `+4`, `+8`.
    pub fn yn_map_part_set_size(mem: &mut impl Mem, part: i32, w: u32, h: u32) {
        if let Some(l) = yn_layer(mem, part) {
            mem.write(l.wrapping_add(4), [w, h]);
        }
    }

    /// `Yn_MapPartSet24(part, a, b)` (`0x0091ac40`): `+0x24`, `+0x28`.
    pub fn yn_map_part_set_24(mem: &mut impl Mem, part: i32, a: u32, b: u32) {
        if let Some(l) = yn_layer(mem, part) {
            mem.write(l.wrapping_add(0x24), [a, b]);
        }
    }

    /// `Yn_MapPartSet14(part, v)` (`0x0091acc0`): a float.
    pub fn yn_map_part_set_14(mem: &mut impl Mem, part: i32, v: u32) {
        if let Some(l) = yn_layer(mem, part) {
            mem.write(l.wrapping_add(0x14), x87::copy(v));
        }
    }

    /// `Yn_MapPartAnimeSetFrame(part, frame)` (`0x0091a930`): the current
    /// frame (`u16` at `+0x96`).
    pub fn yn_map_part_anime_set_frame(mem: &mut impl Mem, part: i32, frame: u16) {
        if let Some(l) = yn_layer(mem, part) {
            mem.write(l.wrapping_add(0x96), frame);
        }
    }

    /// `Yn_MapPartAnimeSetLastFrame(part)` (`0x0091a970`): the current
    /// frame to the count less 1 (16 bits).
    pub fn yn_map_part_anime_set_last_frame(mem: &mut impl Mem, part: i32) {
        if let Some(l) = yn_layer(mem, part) {
            let n = mem.read::<u16>(l.wrapping_add(0x98));
            mem.write(l.wrapping_add(0x96), n.wrapping_sub(1));
        }
    }

    /// `Yn_MapPartAnimeSetRate(part, rate)` (`0x0091a9c0`): `+0xa4`.
    pub fn yn_map_part_anime_set_rate(mem: &mut impl Mem, part: i32, rate: u32) {
        if let Some(l) = yn_layer(mem, part) {
            mem.write(l.wrapping_add(0xa4), rate);
        }
    }

    /// `Yn_MapPartAnimeSetMode(part, bits)` (`0x00919fe0`): bits 8..=11
    /// of the animation word replaced (`& !0xf00 | bits`, the bits as
    /// given).
    pub fn yn_map_part_anime_set_mode(mem: &mut impl Mem, part: i32, bits: u32) {
        if let Some(l) = yn_layer(mem, part) {
            let at = l.wrapping_add(scene::layer::ANIME);
            let w = mem.read::<u32>(at);
            mem.write(at, (w & 0xffff_f0ff) | bits);
        }
    }

    /// `Yn_MapPartAnimeFlag0Off(part)` (`0x0091a030`): clears bit 0 of the
    /// animation word.
    pub fn yn_map_part_anime_flag0_off(mem: &mut impl Mem, part: i32) {
        if let Some(l) = yn_layer(mem, part) {
            let at = l.wrapping_add(scene::layer::ANIME);
            let w = mem.read::<u32>(at);
            mem.write(at, w & !1);
        }
    }

    /// `Yn_MapPartAnimeFlag0On(part)` (`0x0091a080`): sets it.
    pub fn yn_map_part_anime_flag0_on(mem: &mut impl Mem, part: i32) {
        if let Some(l) = yn_layer(mem, part) {
            let at = l.wrapping_add(scene::layer::ANIME);
            let w = mem.read::<u32>(at);
            mem.write(at, w | 1);
        }
    }

    /// `Yn_MapPartSetPos(part, x, y)` (`0x0091aae0`): the position
    /// (`+0x4c`, `+0x50`), the move's counters (`+0x7c`, `+0x80`) 0.
    pub fn yn_map_part_set_pos(mem: &mut impl Mem, part: i32, x: u32, y: u32) {
        if let Some(l) = yn_layer(mem, part) {
            mem.write(l.wrapping_add(0x4c), x87::copy(x));
            mem.write(l.wrapping_add(0x80), 0u32);
            mem.write(l.wrapping_add(0x7c), 0u32);
            mem.write(l.wrapping_add(0x50), x87::copy(y));
        }
    }

    /// `Yn_MapPartMoveTo(part, x, y, frames)` (`0x0091a0f0`): a move from
    /// the position (`+0x64`, `+0x6c`) to `x`, `y` (`+0x68`, `+0x70`) over
    /// `frames` (`+0x7c`), the counter (`+0x80`) 0.
    pub fn yn_map_part_move_to(mem: &mut impl Mem, part: i32, x: u32, y: u32, frames: u32) {
        if let Some(l) = yn_layer(mem, part) {
            let px = mem.read::<u32>(l.wrapping_add(0x4c));
            mem.write(l.wrapping_add(0x7c), frames);
            mem.write(l.wrapping_add(0x64), x87::copy(px));
            mem.write(l.wrapping_add(0x80), 0u32);
            mem.write(l.wrapping_add(0x68), x87::copy(x));
            let py = mem.read::<u32>(l.wrapping_add(0x50));
            mem.write(l.wrapping_add(0x6c), x87::copy(py));
            mem.write(l.wrapping_add(0x70), x87::copy(y));
        }
    }

    /// `Yn_MapPartScaleTo(part, sx, sy, frames)` (`0x0091a150`): a scale
    /// change from the scale (`+0x54`, `+0x5c`) to `sx`, `sy` (`+0x58`,
    /// `+0x60`; a 0.0 target keeps the scale) over `frames` (`+0x74`), the
    /// counter (`+0x78`) 0.
    pub fn yn_map_part_scale_to(mem: &mut impl Mem, part: i32, sx: u32, sy: u32, frames: u32) {
        let Some(l) = yn_layer(mem, part) else { return };
        let zero = |v: u32| f32::from_bits(v) == 0.0;
        let scale_x = x87::copy(mem.read::<u32>(l.wrapping_add(0x44)));
        if zero(sx) {
            mem.write(l.wrapping_add(0x58), scale_x);
            mem.write(l.wrapping_add(0x54), scale_x);
        } else {
            mem.write(l.wrapping_add(0x54), scale_x);
            mem.write(l.wrapping_add(0x58), x87::copy(sx));
        }
        let scale_y = x87::copy(mem.read::<u32>(l.wrapping_add(0x48)));
        if zero(sy) {
            mem.write(l.wrapping_add(0x74), frames);
            mem.write(l.wrapping_add(0x78), 0u32);
            mem.write(l.wrapping_add(0x60), scale_y);
            mem.write(l.wrapping_add(0x5c), scale_y);
        } else {
            mem.write(l.wrapping_add(0x5c), scale_y);
            mem.write(l.wrapping_add(0x74), frames);
            mem.write(l.wrapping_add(0x78), 0u32);
            mem.write(l.wrapping_add(0x60), x87::copy(sy));
        }
    }

    /// `Yn_DepthToScreen(d)` (`0x009201a0`): the screen depth of a point
    /// `d` in front: `(0, 0, d * scale, 1)` (the scene's guide scale at
    /// `+0xf4`, or 10.0 without a scene) through the screen matrix
    /// (`FFXVu0ApplyMatrix`), `z / w` truncated (`fistp` with the rounding
    /// mode set to chop), the low 32 bits.
    pub fn yn_depth_to_screen(mem: &impl Mem, d: f32) -> u32 {
        let z = match yn_scene(mem) {
            Some(sc) => Reg::load(mem.read::<f32>(sc + 0xf4)).mul_m32(d).store(),
            None => (Reg::load(d) * Reg::constant(super::FOG_SCALE.get(mem))).store(),
        };
        let v = [0.0, 0.0, z, 1.0];
        let m = crate::geom::SCREEN_MATRIX.get(mem);
        let col = |i: usize| [m[i], m[4 + i], m[8 + i], m[12 + i]];
        let rz = ffx_system::math::mul4x4_component(2, v, col(2));
        let rw = ffx_system::math::mul4x4_component(3, v, col(3));
        let q = Reg::load(rz).div_m32(rw).value();
        // fistp m64 under round-toward-zero: the integer indefinite past
        // the range
        if q.is_nan() || q >= 9_223_372_036_854_775_808.0 || q < -9_223_372_036_854_775_808.0 {
            return 0;
        }
        #[allow(clippy::cast_possible_truncation)] // in range, then the low half
        let wide = q.trunc() as i64;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let low = wide as u32;
        low
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

    /// `Yn_NearestOnLine2D(out, a, b, p, clamp)` (`0x0091cc00`): the point
    /// of the line `a`→`b` (x, y) nearest `p`: `t = -((a - p) . d) / |d|^2`
    /// (0 when `|d|^2 <= 0`), clamped to 0..=1 with `clamp`, `out = a + t
    /// d`. The squared length, the dot product and `t` are stored as
    /// `f32` on the way. Returns `out`.
    pub fn yn_nearest_on_line_2d(
        mem: &mut impl Mem,
        out: u32,
        a: u32,
        b: u32,
        p: u32,
        clamp: u32,
    ) -> u32 {
        let ld = |mem: &_, at: u32| ffx_bridge::x87::load(Mem::read::<u32>(mem, at));
        let st = |x: f64| f64::from(f32::from_bits(x87::store(x)));
        let dx = st(ld(&*mem, b) - ld(&*mem, a));
        let dy = st(ld(&*mem, b.wrapping_add(4)) - ld(&*mem, a.wrapping_add(4)));
        let len = st(dx * dx + dy * dy);
        let mut t = if len > 0.0 {
            let s = st(
                (ld(&*mem, a.wrapping_add(4)) - ld(&*mem, p.wrapping_add(4))) * dy
                    + (ld(&*mem, a) - ld(&*mem, p)) * dx,
            );
            st(-s / len)
        } else {
            0.0
        };
        if clamp != 0 {
            t = t.clamp(0.0, 1.0);
        }
        let x = dx * t + ld(&*mem, a);
        mem.write(out, x87::store(x));
        let y = dy * t + ld(&*mem, a.wrapping_add(4));
        mem.write(out.wrapping_add(4), x87::store(y));
        out
    }

    /// `Yn_GuideEdgeFade(p, a, b, rect, out, work)` (`0x0091f530`, from
    /// `0x0091cf10`; name ours): with `h` half the rectangle's width
    /// (`|x0 - x1| * 0.5`) and `p`'s distance `r` from the segment
    /// `a`→`b` ([`yn_nearest_on_line_2d`] into `work`, clamped): beyond
    /// `0.2 h` writes 1.0 and returns 1; else writes `(r - 0.15 h) /
    /// (0.2 h - 0.15 h)` clamped to 0.0..=1.0 (a NaN kept) and returns 0.
    /// Every intermediate the original stores is an `f32`; the
    /// denominator stays at the x87's precision.
    pub fn yn_guide_edge_fade<M: Mem>(mem: &mut M, args: [u32; 5], work: u32) -> u32 {
        use ffx_bridge::x87::{div, load, mul, sqrt, sub};
        let [p, a, b, rect, out] = args;
        let st = |x: f64| load(x87::store(x));
        let rd = |mem: &M, at: u32| load(mem.read::<u32>(at));
        let c = |mem: &M, at: u32| mem.read::<f64>(mem.rebase(at));
        let w = st(sub(rd(&*mem, rect), rd(&*mem, rect.wrapping_add(8))).abs());
        let h = st(mul(w, c(&*mem, super::EDGE_HALF)));
        let lo = st(mul(c(&*mem, super::EDGE_INNER), h));
        let hi = st(mul(h, c(&*mem, super::EDGE_OUTER)));
        yn_nearest_on_line_2d(mem, work, a, b, p, 1);
        let dx = st(sub(rd(&*mem, p), rd(&*mem, work)));
        let dy = st(sub(
            rd(&*mem, p.wrapping_add(4)),
            rd(&*mem, work.wrapping_add(4)),
        ));
        let d2 = st(ffx_bridge::x87::add(mul(dx, dx), mul(dy, dy)));
        let h2 = st(mul(hi, hi));
        if h2 < d2 {
            mem.write(out, 1.0f32.to_bits());
            return 1;
        }
        let r = st(sqrt(d2));
        let t = st(sub(r, lo));
        let v = st(div(t, sub(hi, lo)));
        let res = if v < 0.0 {
            0.0f32.to_bits()
        } else if 1.0 < v {
            1.0f32.to_bits()
        } else {
            x87::store(v)
        };
        mem.write(out, res);
        0
    }

    /// `rcbgGuideFini(guide)` (`0x0091d3d0`): its VFX instance (handle:
    /// low word `0x0230fd20`, high word the guide plus the low word's
    /// sign) destroyed when `graphicVFXGetRenderDataByInstance` finds it,
    /// the walk-mesh model (`+8`) deleted. Returns the deletion's `eax`,
    /// or 0 without the model.
    pub fn rcbg_guide_fini<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        g: u32,
        call: &mut C,
    ) -> u32 {
        let lo = super::GUIDE_VFX.get(mem);
        let hi = g.wrapping_add(lo.cast_signed().wrapping_shr(31).cast_unsigned());
        let get = mem.rebase(super::VFX_GET_RENDER_DATA);
        if call(mem, get, &[lo, hi]) != 0 {
            let destroy = mem.rebase(super::VFX_DESTROY);
            call(mem, destroy, &[lo, hi]);
        }
        let m = mem.read::<u32>(g.wrapping_add(object::WALK_MODEL));
        if m == 0 {
            return 0;
        }
        let r = crate::model::bridge::rcbg_model_delete(mem, m, call);
        mem.write(g.wrapping_add(object::WALK_MODEL), 0u32);
        r
    }

    /// `rcbgGuideDelete(guide)` (`0x0091cef0`): [`rcbg_guide_fini`], then
    /// the guide freed.
    pub fn rcbg_guide_delete<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        g: u32,
        call: &mut C,
    ) -> u32 {
        rcbg_guide_fini(mem, g, call);
        crate::model::bridge::rc_free(mem, call, g)
    }

    /// `rcbgPreFini()` (`0x009216b0`): the map's scenes (`0x110` bytes
    /// each, count `+0`) finished, the scene array (`+4`), `+0xc` and the
    /// map freed, then `MsSetCameraMatrix(0, 0, 0, 0, 0, 1024.0)`. Returns
    /// its `eax`; 0 without a map system, the system without a map.
    pub fn rcbg_pre_fini<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        let s = YN_MAP_SYSTEM.get(mem);
        if s == 0 {
            return 0;
        }
        let map = mem.read::<u32>(s.wrapping_add(system::MAP));
        if map == 0 {
            return s;
        }
        let mut i = 0;
        while i < mem.read::<i32>(map) {
            let scenes = mem.read::<u32>(map.wrapping_add(4));
            scene::bridge::rcbg_scene_fini(
                mem,
                scenes.wrapping_add(i.cast_unsigned().wrapping_mul(0x110)),
                call,
            );
            i += 1;
        }
        crate::model::bridge::free_field(mem, call, map.wrapping_add(4));
        crate::model::bridge::free_field(mem, call, map.wrapping_add(0xc));
        let s = YN_MAP_SYSTEM.get(mem);
        if s != 0 {
            crate::model::bridge::free_field(mem, call, s.wrapping_add(system::MAP));
        }
        let at = mem.rebase(super::CAMERA_DISTANCE);
        let d = x87::copy(mem.read::<u32>(at));
        let f = mem.rebase(super::MS_SET_CAMERA_MATRIX);
        call(mem, f, &[0, 0, 0, 0, 0, d])
    }

    /// `rcbgFini()` (`0x00921aa0`): with a map system: the guide's minimap
    /// model's VFX instance (`+0x14`) destroyed and [`rcbg_pre_fini`]
    /// when there is a map; `sceDevVif0Reset` in debug builds
    /// (`FFXIsDebug` and `sceGsSetDefDrawEnv2`); the guide deleted, the
    /// chunks `+0x10`, `+0x14`, `+0x18` deleted, the system freed and
    /// cleared. Returns the last free's `eax`, 0 without a system.
    pub fn rcbg_fini<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        let s = YN_MAP_SYSTEM.get(mem);
        if s == 0 {
            return 0;
        }
        if mem.read::<u32>(s.wrapping_add(system::MAP)) != 0 {
            let g = mem.read::<u32>(s.wrapping_add(system::GUIDE));
            if g != 0 {
                let m = mem.read::<u32>(g.wrapping_add(object::MODEL));
                if m != 0 {
                    let h = mem.read::<u32>(m.wrapping_add(0x14));
                    if h != 0 {
                        let destroy = mem.rebase(super::VFX_DESTROY);
                        call(mem, destroy, &[0, h]);
                    }
                }
            }
            rcbg_pre_fini(mem, call);
        }
        if ffx_system::config::bridge::is_debug(mem) != 0
            && ffx_system::config::bridge::gs_set_def_draw_env2(mem) != 0
        {
            let reset = mem.rebase(super::VIF0_RESET);
            call(mem, reset, &[]);
        }
        let s = YN_MAP_SYSTEM.get(mem);
        let g = mem.read::<u32>(s.wrapping_add(system::GUIDE));
        if g != 0 {
            rcbg_guide_delete(mem, g, call);
            let s = YN_MAP_SYSTEM.get(mem);
            mem.write(s.wrapping_add(system::GUIDE), 0u32);
        }
        for off in [0x10u32, 0x14, 0x18] {
            let s = YN_MAP_SYSTEM.get(mem);
            let c = mem.read::<u32>(s.wrapping_add(off));
            if c != 0 {
                crate::rcdt::bridge::rcdt_chunk_delete(mem, c, call);
                let s = YN_MAP_SYSTEM.get(mem);
                mem.write(s.wrapping_add(off), 0u32);
            }
        }
        let s = YN_MAP_SYSTEM.get(mem);
        if s == 0 {
            return 0;
        }
        let r = crate::model::bridge::rc_free(mem, call, s);
        YN_MAP_SYSTEM.set(mem, 0);
        r
    }

    /// `Yn_MapSystemFini()` (`0x00919a60`): [`rcbg_fini`] when there is a
    /// map system (the guard jumps into it).
    pub fn yn_map_system_fini<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        if YN_MAP_SYSTEM.get(mem) == 0 {
            return 0;
        }
        rcbg_fini(mem, call)
    }

    /// The map (`*g_YnMapSystem`), 0 without a system.
    fn the_map(mem: &impl Mem) -> u32 {
        let s = YN_MAP_SYSTEM.get(mem);
        if s == 0 { 0 } else { mem.read::<u32>(s) }
    }

    /// `rcbgPreLoad(parse)` (`0x0092b390`): the scene count into the map,
    /// the scenes (`0x110` bytes each, `rcbgSceneInit`), the current one
    /// the first.
    pub fn rcbg_pre_load<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        p: u32,
        call: &mut C,
    ) {
        let map = the_map(mem);
        crate::rcdt::bridge::rcdt_parse_get_bin(mem, p, map, 4);
        let n = mem.read::<u32>(map);
        let t = crate::model::bridge::rc_alloc(mem, call, n.wrapping_mul(0x110));
        mem.write(map.wrapping_add(4), t);
        mem.write(map.wrapping_add(8), t);
        let mut i = 0;
        while i < mem.read::<i32>(map) {
            let s = mem
                .read::<u32>(map.wrapping_add(4))
                .wrapping_add(i.cast_unsigned().wrapping_mul(0x110));
            scene::bridge::rcbg_scene_init(mem, s);
            i += 1;
        }
    }

    /// `rcbgPreInit()` (`0x00921770`): [`rcbg_pre_fini`], a new map
    /// (`0x14` bytes, zeroed) with a `0x10000`-byte work block at `+0xc`.
    pub fn rcbg_pre_init<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(mem: &mut M, call: &mut C) {
        rcbg_pre_fini(mem, call);
        let m = crate::model::bridge::rc_calloc(mem, call, 1, 0x14);
        let s = YN_MAP_SYSTEM.get(mem);
        if s != 0 {
            mem.write(s, m);
        }
        let w = crate::model::bridge::rc_alloc(mem, call, 0x1_0000);
        mem.write(m.wrapping_add(0xc), w);
    }

    /// `rcbgPreDummyInit()` (`0x00921640`): a map when there is none, and
    /// one empty scene when it has none.
    pub fn rcbg_pre_dummy_init<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) {
        let mut m = the_map(mem);
        if m == 0 {
            m = crate::model::bridge::rc_calloc(mem, call, 1, 0x14);
            let s = YN_MAP_SYSTEM.get(mem);
            if s != 0 {
                mem.write(s, m);
            }
        }
        if mem.read::<u32>(m) == 0 {
            mem.write(m, 1u32);
            let sc = crate::model::bridge::rc_alloc(mem, call, 0x110);
            mem.write(m.wrapping_add(4), sc);
            scene::bridge::rcbg_scene_init(mem, sc);
            let first = mem.read::<u32>(m.wrapping_add(4));
            mem.write(m.wrapping_add(8), first);
        }
    }

    /// `rcbgPreAddObject(obj)` (`0x009218c0`): `obj` added to the current
    /// scene ([`rcbg_pre_dummy_init`] first when there is no map).
    pub fn rcbg_pre_add_object<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        obj: u32,
        call: &mut C,
    ) {
        if the_map(mem) == 0 {
            rcbg_pre_dummy_init(mem, call);
        }
        let m = the_map(mem);
        let s = if m == 0 {
            0
        } else {
            mem.read::<u32>(m.wrapping_add(8))
        };
        scene::bridge::rcbg_scene_add_object(mem, s, obj, call);
    }

    /// `rcbgGuideInit(guide)` (`0x0091d490`): the guide (`0x78` bytes)
    /// cleared, the view limit `+0x6c` 350.0, bit 8 of `g_GuideFlags` set,
    /// then [`guide_fade_off`].
    pub fn rcbg_guide_init(mem: &mut impl Mem, g: u32) {
        mem.write_bytes(g, &[0; 0x78]);
        let at = mem.rebase(super::GUIDE_DEFAULT_LIMIT);
        let v = x87::copy(mem.read::<u32>(at));
        mem.write(g.wrapping_add(object::LIMIT), v);
        let f = super::FLAGS.get(mem);
        super::FLAGS.set(mem, f | 0x100);
        guide_fade_off(mem);
    }

    /// `rcbgGuideNew()` (`0x0091d4e0`): a guide ([`rcbg_guide_init`]).
    pub fn rcbg_guide_new<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        let g = crate::model::bridge::rc_alloc(mem, call, 0x78);
        rcbg_guide_init(mem, g);
        g
    }

    /// `rcbgInit()` (`0x00921ce0`): [`rcbg_fini`] of an old system, a new
    /// one (`0x1c` bytes) with its guide, the four matrices
    /// `g_RcbgWorldMatrix`, `g_RcbgScreenWorldMatrix`, `g_RcbgClipMatrix`,
    /// `g_RcbgClipWorldMatrix` the identity; debug builds go on to
    /// `0x0092cef0`.
    pub fn rcbg_init<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(mem: &mut M, call: &mut C) {
        if YN_MAP_SYSTEM.get(mem) != 0 {
            rcbg_fini(mem, call);
        }
        let s = crate::model::bridge::rc_calloc(mem, call, 1, 0x1c);
        YN_MAP_SYSTEM.set(mem, s);
        let g = rcbg_guide_new(mem, call);
        let s = YN_MAP_SYSTEM.get(mem);
        mem.write(s.wrapping_add(system::GUIDE), g);
        for m in super::RCBG_MATRICES {
            let at = mem.rebase(m);
            ffx_ps2::vu0::bridge::matrix_identity(mem, at);
        }
        if ffx_system::config::bridge::is_debug(mem) != 0
            && ffx_system::config::bridge::gs_set_def_draw_env2(mem) != 0
        {
            let f = mem.rebase(super::RCBG_DEBUG_INIT);
            call(mem, f, &[]);
        }
    }

    /// `rcbgGuideSetModel(guide, model)` (`0x0091d590`): the walk model
    /// (`+8`) deleted; with `model` it becomes the minimap model (`+0`,
    /// `+4` 0, `rcbgGuideSetModelSub`), else the model comes from the id
    /// map (`rcbgGuideSetModelFromIdMap`); both through `call`.
    pub fn rcbg_guide_set_model<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        g: u32,
        model: u32,
        call: &mut C,
    ) -> u32 {
        let w = mem.read::<u32>(g.wrapping_add(object::WALK_MODEL));
        if w != 0 {
            crate::model::bridge::rcbg_model_delete(mem, w, call);
            mem.write(g.wrapping_add(object::WALK_MODEL), 0u32);
        }
        if model != 0 {
            mem.write(g, model);
            mem.write(g.wrapping_add(4), 0u32);
            let f = mem.rebase(super::GUIDE_SET_MODEL_SUB);
            call(mem, f, &[g])
        } else {
            let f = mem.rebase(super::GUIDE_SET_MODEL_FROM_ID_MAP);
            call(mem, f, &[g])
        }
    }

    /// `rcbgPreSetSceneNo(n)` (`0x009217e0`): with a map and `n` one of its
    /// scenes: `0x00639d90(n)`, `graphicSet2DStartLayer` the layers of the
    /// scenes before it, the current scene `n`
    /// ([`scene::bridge::rcbg_scene_chk_sort_layer`]), and the guide's
    /// model the last model of the first scene (of scene `n` when there is
    /// only one) ([`rcbg_guide_set_model`]).
    pub fn rcbg_pre_set_scene_no<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        n: i32,
        call: &mut C,
    ) {
        let m = the_map(mem);
        if m == 0 || n < 0 || n >= mem.read::<i32>(m) {
            return;
        }
        let f = mem.rebase(super::GRAPHIC_SCENE_NO);
        call(mem, f, &[n.cast_unsigned()]);
        let scenes = mem.read::<u32>(m.wrapping_add(4));
        let mut sum = 0u32;
        for i in 0..n.cast_unsigned() {
            sum = sum.wrapping_add(
                mem.read::<u32>(scenes.wrapping_add(i.wrapping_mul(0x110)).wrapping_add(4)),
            );
        }
        let f = mem.rebase(super::GRAPHIC_SET_2D_START_LAYER);
        call(mem, f, &[sum]);
        let s = mem
            .read::<u32>(m.wrapping_add(4))
            .wrapping_add(n.cast_unsigned().wrapping_mul(0x110));
        mem.write(m.wrapping_add(8), s);
        scene::bridge::rcbg_scene_chk_sort_layer(mem, s, call);
        let src = if mem.read::<i32>(m) > 1 {
            mem.read::<u32>(m.wrapping_add(4))
        } else {
            mem.read::<u32>(m.wrapping_add(8))
        };
        let k = i32::from(mem.read::<i16>(src.wrapping_add(0x1a)));
        let model = mem.read::<u32>(
            src.wrapping_add(0x1c)
                .wrapping_add(k.cast_unsigned().wrapping_mul(4)),
        );
        let g = mem.read::<u32>(YN_MAP_SYSTEM.get(mem).wrapping_add(system::GUIDE));
        rcbg_guide_set_model(mem, g, model, call);
    }

    /// `rcbgLoadBin(data)` (`0x00921d60`): a background file (an `RCBG`
    /// chunk, id 0, then chunks of a 16-byte header, `+4` the length in
    /// 16-byte units): 1 the map ([`rcbg_pre_init`], [`rcbg_pre_load`]),
    /// 2 the next scene ([`scene::bridge::rcbg_scene_load`]), 5 a model
    /// and 6 a transfer of that scene, 8 and 10 texture transfers of the
    /// current scene ([`rcbg_pre_add_object`]), 11 a model replacing the
    /// last of the scene named at `+0xa`, 12 the end; 7 and unknown ids
    /// are skipped. `work` stands for the original's stack chunk (8
    /// bytes) and parser (`0x8c`).
    pub fn rcbg_load_bin<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        data: u32,
        work: u32,
        call: &mut C,
    ) {
        use crate::model::bridge as mb;
        use crate::rcdt::bridge as rb;
        if mb::rcbg_chk_id(mem, data) != 0 {
            return;
        }
        let (chunk, p) = (work, work.wrapping_add(0x14));
        let (mut sc, mut offset) = (0u32, 0u32);
        let mut rec = data.wrapping_add(0x10);
        loop {
            let id = mb::rcbg_chk_id(mem, rec);
            let len = mem.read::<u32>(rec.wrapping_add(4)).wrapping_shl(4);
            let body = rec.wrapping_add(0x10);
            rb::rcdt_chunk_set_bin(mem, chunk, body);
            rb::rcdt_parse_init(mem, p);
            rb::rcdt_parse_set(mem, p, chunk);
            match id {
                1 => {
                    rcbg_pre_init(mem, call);
                    rcbg_pre_load(mem, p, call);
                }
                2 => {
                    let m = the_map(mem);
                    sc = mem.read::<u32>(m.wrapping_add(4)).wrapping_add(offset);
                    scene::bridge::rcbg_scene_load(mem, sc, p, call);
                    offset = offset.wrapping_add(0x110);
                }
                5 => scene::bridge::rcbg_scene_model_load(mem, sc, p, call),
                6 => scene::bridge::rcbg_scene_trx_load(mem, sc, p, call),
                8 | 10 => {
                    let t = if id == 8 {
                        mb::rcbg_trx_tex_new(mem, call)
                    } else {
                        mb::rcbg_fp_trx_tex_new(mem, call)
                    };
                    mb::rcbg_trx_tex_load(mem, t, p);
                    rcbg_pre_add_object(mem, t, call);
                }
                11 => {
                    if the_map(mem) == 0 {
                        rcbg_pre_dummy_init(mem, call);
                    }
                    let m = the_map(mem);
                    let s = u32::from(mem.read::<u16>(rec.wrapping_add(0xa)))
                        .wrapping_mul(0x110)
                        .wrapping_add(mem.read::<u32>(m.wrapping_add(4)));
                    let k = i32::from(mem.read::<i16>(s.wrapping_add(0x18)));
                    let old = mem.read::<u32>(
                        s.wrapping_add(0x1c)
                            .wrapping_add(k.cast_unsigned().wrapping_mul(4)),
                    );
                    if old != 0 {
                        mb::rcbg_model_delete(mem, old, call);
                    }
                    let model = mb::rcbg_model_new(mem, call);
                    mb::rcbg_model_load(mem, model, p, call);
                    let k = i32::from(mem.read::<i16>(s.wrapping_add(0x18)));
                    mem.write(
                        s.wrapping_add(0x1c)
                            .wrapping_add(k.cast_unsigned().wrapping_mul(4)),
                        model,
                    );
                    let k = mem.read::<i16>(s.wrapping_add(0x18));
                    mem.write(s.wrapping_add(0x18), k.wrapping_add(1));
                }
                12 => return,
                _ => {}
            }
            rec = body.wrapping_add(len);
        }
    }

    /// `Yn_PreBgSetData(data)` (`0x0091aa60`): with a map system and data,
    /// [`rcbg_load_bin`] and [`rcbg_pre_set_scene_no`] 0.
    pub fn yn_pre_bg_set_data<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        data: u32,
        work: u32,
        call: &mut C,
    ) {
        if YN_MAP_SYSTEM.get(mem) == 0 || data == 0 {
            return;
        }
        rcbg_load_bin(mem, data, work, call);
        rcbg_pre_set_scene_no(mem, 0, call);
    }

    /// `rcbgGuideSetWalkFog(guide)` (`0x0091ce50`): on the maps 0x246,
    /// 0x248, 0x24e and 0x24f (`0x0086c1f0`, through `call`), the guide's
    /// mesh object loses flag bit 1 and the fog of war (`+0x70`) points at
    /// that map's 0x200 bytes of the saved walk maps (`MsGetSaveWalkMap`).
    pub fn rcbg_guide_set_walk_fog<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        g: u32,
        call: &mut C,
    ) {
        let f = mem.rebase(super::EVENT_MAP_NO);
        let map = call(mem, f, &[]);
        let Some(i) = [0x246u32, 0x248, 0x24e, 0x24f]
            .iter()
            .position(|&m| m == map)
        else {
            return;
        };
        let o = guide_mesh_object(mem, g);
        if o != 0 {
            let fl = mem.read::<u32>(o.wrapping_add(0x10));
            mem.write(o.wrapping_add(0x10), fl & !2);
        }
        let f = mem.rebase(super::SAVE_WALK_MAP);
        let base = call(mem, f, &[]);
        let i = u32::try_from(i).unwrap_or(0);
        mem.write(g.wrapping_add(object::FOG), base.wrapping_add(i << 9));
    }

    /// `rcbgGuideSetModelSub(guide)` (`0x0091d880`): the minimap model
    /// drawn from the guide's matrix (`+0x40` = `+0x10`), alpha 0x80, its
    /// centre to `+0x50`, the bounding box's width and depth to `+0x64`
    /// and `+0x68`, the larger to `+0x60`; then
    /// [`rcbg_guide_set_walk_fog`].
    pub fn rcbg_guide_set_model_sub<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        g: u32,
        call: &mut C,
    ) {
        let o = guide_mesh_object(mem, g);
        mem.write(o.wrapping_add(0x40), g.wrapping_add(0x10));
        crate::model::bridge::rcbg_model_set_alpha(mem, o, 0x80);
        let b = o.wrapping_add(0x90);
        crate::geom::bridge::rc_bbox_get_center(mem, b, g.wrapping_add(0x50));
        let ld = |mem: &M, at: u32| ffx_bridge::x87::load(mem.read::<u32>(at));
        let w = x87::store(ld(mem, b.wrapping_add(0x10)) - ld(mem, b));
        mem.write(g.wrapping_add(0x64), w);
        let d = x87::store(ld(mem, b.wrapping_add(0x18)) - ld(mem, b.wrapping_add(8)));
        mem.write(g.wrapping_add(0x68), d);
        let a = ld(mem, g.wrapping_add(0x64));
        let m = if a > f64::from(f32::from_bits(d)) {
            x87::copy(mem.read::<u32>(g.wrapping_add(0x64)))
        } else {
            d
        };
        mem.write(g.wrapping_add(0x60), m);
        rcbg_guide_set_walk_fog(mem, g, call);
    }

    /// `rcbgGuideSetModelFromIdMap(g)` (`0x0091d5e0`): the guide's walk
    /// model rebuilt from the field's ID map (`Sg_IdGetPtr`, kept at
    /// `+0x4`): one triangle packet of all its polygons (at most 0x1ff, else
    /// nothing is built), its vertices (`i16` x, z pairs divided by the map's
    /// scale) bounded, packed to `i16` and the model scaled to fit, each
    /// triangle white with its three vertex indices; flags `+0x10 |= 6`,
    /// then `rcbgGuideSetModelSub`. `work` is the original's stack (0x20
    /// bytes).
    pub fn rcbg_guide_set_model_from_id_map<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        g: u32,
        work: u32,
        call: &mut C,
    ) {
        use crate::model::bridge as mb;
        let w = mem.read::<u32>(g.wrapping_add(object::WALK_MODEL));
        if w != 0 {
            mb::rcbg_model_delete(mem, w, call);
            mem.write(g.wrapping_add(object::WALK_MODEL), 0u32);
        }
        mem.write(g, 0u32);
        let id = super::ID_MAP.get(mem);
        mem.write(g.wrapping_add(4), id);
        if id == 0 {
            return;
        }
        let groups = |mem: &M| i32::from(mem.read::<i16>(id + 6));
        let mut count = 0i32;
        for i in 0..groups(mem).max(0).cast_unsigned() {
            let list = mem.read::<u32>(id + 0x1c);
            count = count.wrapping_add(i32::from(mem.read::<i16>(list.wrapping_add(i * 0x10 + 8))));
        }
        if count >= 0x200 {
            return; // rcPrint ("ID MAP no POLYGON oosugi desu"), a no-op
        }
        let verts = i32::from(mem.read::<i16>(id + 0xa));
        let model = mb::rcbg_model_new(mem, call);
        mb::rcbg_model_create_one_prim_h(
            mem,
            model,
            0,
            count.cast_unsigned(),
            verts.cast_unsigned(),
            0,
            work,
            call,
        );
        let pts = mb::rc_alloc(mem, call, verts.cast_unsigned().wrapping_shl(4));
        let inv = (Reg::ONE / Reg::load(mem.read::<f32>(id + 0xc))).store();
        let mut p = pts.wrapping_add(8);
        for i in 0..verts.max(0).cast_unsigned() {
            let v = mem.read::<u32>(id + 0x18);
            let x = mem.read::<i16>(v.wrapping_add(i * 8));
            mem.write(
                p.wrapping_sub(8),
                Reg::constant(f64::from(x)).mul_m32(inv).store(),
            );
            mem.write(p.wrapping_sub(4), 0f32);
            let v = mem.read::<u32>(id + 0x18);
            let z = mem.read::<i16>(v.wrapping_add(i * 8 + 4));
            mem.write(p, Reg::constant(f64::from(z)).mul_m32(inv).store());
            mem.write(p.wrapping_add(4), 0f32);
            p = p.wrapping_add(0x10);
        }
        let b = model + 0x90;
        crate::geom::bridge::rc_bbox_set(mem, b, pts, verts);
        let blob = mem.read::<u32>(model + 0x14);
        let packed = mem.read::<u32>(blob.wrapping_add(8)).wrapping_add(blob);
        let scale = crate::geom::bridge::rc_bbox_get_short_convert_scale(mem, b);
        let scale = ffx_bridge::x87::round_trip(scale);
        mb::rcbg_model_f_to_s0(mem, packed, pts, verts, scale.to_bits(), work);
        crate::geom::bridge::rcbg_model_set_scale(mem, model, scale);
        let mut r = mb::rcbg_model_get_prim(mem, model, 0);
        for i in 0..groups(mem).max(0).cast_unsigned() {
            let list = mem.read::<u32>(id + 0x1c);
            let polys = mem.read::<u32>(list.wrapping_add(i * 0x10 + 0xc));
            let mut q = polys.wrapping_add(4);
            let mut j = 0;
            loop {
                let list = mem.read::<u32>(id + 0x1c);
                let n = i32::from(mem.read::<i16>(list.wrapping_add(i * 0x10 + 8)));
                if j >= n {
                    break;
                }
                mem.write(r.wrapping_add(1), 0xff80_ffffu32);
                mem.write(r, 0xffu8);
                mem.write(r.wrapping_add(5), 0xff80_ffffu32);
                mem.write(r.wrapping_add(9), 0xffffu16);
                mem.write(r.wrapping_add(0xb), 0x80u8);
                let a = mem.read::<u16>(q.wrapping_sub(4));
                mem.write(r.wrapping_add(0xc), a);
                let a = mem.read::<u16>(q.wrapping_sub(2));
                mem.write(r.wrapping_add(0xe), a);
                let a = mem.read::<u16>(q);
                mem.write(r.wrapping_add(0x10), a);
                mem.write(r.wrapping_add(0x12), 0u16);
                r = r.wrapping_add(0x14);
                q = q.wrapping_add(0x10);
                j += 1;
            }
        }
        let f = mem.read::<u32>(model + 0x10);
        mem.write(model + 0x10, f | 6);
        if pts != 0 {
            mb::rc_free(mem, call, pts);
        }
        mem.write(g.wrapping_add(object::WALK_MODEL), model);
        let f = mem.rebase(super::GUIDE_SET_MODEL_SUB);
        call(mem, f, &[g]);
    }

    /// `Yn_GuideMapSetMapNo(n)` (`0x00919da0`): with a map system, its
    /// guide, map and scene, and `n` in range of the scene's guide models:
    /// `n` becomes the current one and the guide shows its model
    /// ([`rcbg_guide_set_model`], whose `eax` it returns; 0 otherwise is the
    /// original's leftover, not a result).
    pub fn yn_guide_map_set_map_no<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        n: i32,
        call: &mut C,
    ) -> u32 {
        let s = YN_MAP_SYSTEM.get(mem);
        if s == 0 || mem.read::<u32>(s + system::GUIDE) == 0 {
            return 0;
        }
        let m = mem.read::<u32>(s + system::MAP);
        if m == 0 {
            return 0;
        }
        let c = mem.read::<u32>(m + map::SCENE);
        if c == 0 || n < 0 || n >= i32::from(mem.read::<i16>(c + scene::layout::GUIDE_COUNT)) {
            return 0;
        }
        #[allow(clippy::cast_possible_truncation)] // `mov word`
        mem.write(c + scene::layout::GUIDE_CURRENT, n as i16);
        let model = mem.read::<u32>(c + scene::layout::GUIDE_MODELS + 4 * n.cast_unsigned());
        let g = mem.read::<u32>(YN_MAP_SYSTEM.get(mem) + system::GUIDE);
        rcbg_guide_set_model(mem, g, model, call)
    }

    /// `Yn_PreBgChangeStage(n)` (`0x0091a0d0`): [`rcbg_pre_set_scene_no`]
    /// when the map system exists (a tail call with the caller's argument).
    pub fn yn_pre_bg_change_stage<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        n: i32,
        call: &mut C,
    ) {
        if YN_MAP_SYSTEM.get(mem) != 0 {
            rcbg_pre_set_scene_no(mem, n, call);
        }
    }

    /// `rcbgPreAddFp(map)` (`0x00921a50`): the field program object a map
    /// needs (`0x61`: `rcbgFPm097New`, `0xad`: `rcbgFPm173New`, `0x235`
    /// and `0x236`: `rcbgFPm565New`) added to the pre-loaded map
    /// ([`rcbg_pre_add_object`], a tail call); other maps nothing.
    pub fn rcbg_pre_add_fp<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        map_no: i32,
        call: &mut C,
    ) {
        use crate::model::bridge as mb;
        let obj = match map_no {
            0x61 => mb::rcbg_fpm097_new(mem, call),
            0xad => mb::rcbg_fpm173_new(mem, call),
            0x235 | 0x236 => mb::rcbg_fpm565_new(mem, call),
            _ => return,
        };
        rcbg_pre_add_object(mem, obj, call);
    }

    /// `rcbgPreGetMapNo()` (`0x00921ba0`): the map whose field program a
    /// battle needs (`MsBattleGetNum`: `0x610000` 0x61, `0x2350000` 0x235,
    /// `0x2360000` 0x236, other battles 0), outside battle the event's map
    /// (`TkGetNowEventJumpMapNo`, a tail call).
    pub fn rcbg_pre_get_map_no<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        let f = mem.rebase(super::MS_BATTLE_GET_NUM);
        match call(mem, f, &[]) {
            0 => {
                let f = mem.rebase(super::EVENT_MAP_NO);
                call(mem, f, &[])
            }
            0x0061_0000 => 0x61,
            0x0235_0000 => 0x235,
            0x0236_0000 => 0x236,
            _ => 0,
        }
    }

    /// `Yn_SceneFindObject(kind, n)` (`0x00921c60`): the `n`th object of
    /// `kind` in the current scene ([`scene::bridge::rcbg_scene_get_object`]),
    /// else in each of the map's scenes in turn; 0 without a map or scene.
    pub fn yn_scene_find_object(mem: &impl Mem, kind: u32, n: u32) -> u32 {
        let s = YN_MAP_SYSTEM.get(mem);
        let m = if s == 0 {
            0
        } else {
            mem.read::<u32>(s + system::MAP)
        };
        if m == 0 {
            return 0;
        }
        let c = mem.read::<u32>(m + map::SCENE);
        if c == 0 {
            return 0;
        }
        let o = scene::bridge::rcbg_scene_get_object(mem, c, kind, n);
        if o != 0 {
            return o;
        }
        let count = mem.read::<i32>(m);
        let scenes = mem.read::<u32>(m + 4);
        for i in 0..count.max(0).cast_unsigned() {
            let o = scene::bridge::rcbg_scene_get_object(
                mem,
                scenes.wrapping_add(i * scene::layout::SIZE),
                kind,
                n,
            );
            if o != 0 {
                return o;
            }
        }
        0
    }

    /// `Yn_FpTrxTexGet(n)` (`0x00919ba0`): the `n`th field-program texture
    /// (kind `0x20006`, [`yn_scene_find_object`]).
    pub fn yn_fp_trx_tex_get(mem: &impl Mem, n: u32) -> u32 {
        yn_scene_find_object(mem, 0x0002_0006, n)
    }

    /// `Yn_FpTrxTexSetAddr(tex, image, clut)` (`0x00919be0`):
    /// `rcbgTrxTexSetAddr`.
    pub fn yn_fp_trx_tex_set_addr(mem: &mut impl Mem, t: u32, image: u32, clut: u32) {
        crate::model::bridge::rcbg_trx_tex_set_addr(mem, t, image, clut);
    }

    /// `Yn_PreBgSetScrBank(roll)` (`0x0091ad10`): with a current scene,
    /// its roll (`+0xac`) set and `rcbgSceneSetMatrix` run (a tail call).
    pub fn yn_pre_bg_set_scr_bank<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        roll: u32,
        call: &mut C,
    ) {
        if let Some(c) = yn_scene(mem) {
            mem.write(c + 0xac, x87::copy(roll));
            let f = mem.rebase(scene::SCENE_SET_MATRIX);
            call(mem, f, &[c]);
        }
    }

    /// `Yn_PreBgSetScrOffset(x, y)` (`0x0091ad40`): with a current scene,
    /// its screen offsets (`+0xb8`, `+0xbc`) set and `rcbgSceneSetMatrix`
    /// run.
    pub fn yn_pre_bg_set_scr_offset<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        x: u32,
        y: u32,
        call: &mut C,
    ) {
        if let Some(c) = yn_scene(mem) {
            mem.write(c + 0xb8, x87::copy(x));
            mem.write(c + 0xbc, x87::copy(y));
            let f = mem.rebase(scene::SCENE_SET_MATRIX);
            call(mem, f, &[c]);
        }
    }

    /// `rcbgPreAddFpObjects()` (`0x00922010`): a map (the dummy one when
    /// there is none, `rcbgPreDummyInit`), the debug board drawn once in a
    /// debug build (`FFXIsDebug`, `sceGsSetDefDrawEnv2`, `MsDebugDrawBoard`
    /// while the system's `+0x8` is 0), the field program objects of the
    /// map ([`rcbg_pre_add_fp`] of [`rcbg_pre_get_map_no`]), `+0x8` = 1.
    pub fn rcbg_pre_add_fp_objects<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) {
        if the_map(mem) == 0 {
            rcbg_pre_dummy_init(mem, call);
        }
        if ffx_system::config::bridge::is_debug(mem) != 0
            && ffx_system::config::bridge::gs_set_def_draw_env2(mem) != 0
        {
            let s = YN_MAP_SYSTEM.get(mem);
            if mem.read::<u32>(s + 8) == 0 {
                let f = mem.rebase(super::MS_DEBUG_DRAW_BOARD);
                call(mem, f, &[]);
            }
        }
        let no = rcbg_pre_get_map_no(mem, call);
        rcbg_pre_add_fp(mem, no.cast_signed(), call);
        let s = YN_MAP_SYSTEM.get(mem);
        mem.write(s + 8, 1u32);
    }

    /// `rcbgDelObjects(kind)` (`0x009219e0`): with a current scene,
    /// `rcbgSceneDelObjects(scene, kind)`.
    pub fn rcbg_del_objects<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        kind: u32,
        call: &mut C,
    ) {
        if let Some(c) = yn_scene(mem) {
            scene::bridge::rcbg_scene_del_objects(mem, c, kind, call);
        }
    }

    // ---- field agent r2, group F: the minimap fog ----

    /// The `.rdata` constants of `rcbgGuideUpdateWalk` (`f64` unless
    /// noted): the default x scale 0.75 (`f32`), 1/512, 0.5, 416, the
    /// layout x scale 0.75, the layouts' x offsets -1243 and -1317 and y
    /// offsets -1820 and -1860, 2.0, 10.0, the cell 40.0, -40.0, and 2^32
    /// (`f32`).
    pub const WALK_K: [u32; 15] = [
        0x00b4_3d18,
        0x00b4_1cc0,
        0x00b9_2220,
        0x00b4_1d30,
        0x00b5_1bd0,
        0x00b6_76d0,
        0x00b6_76d8,
        0x00b6_76e0,
        0x00b6_76e8,
        0x00b9_2270,
        0x00b9_22b0,
        0x00b5_af28,
        0x00b5_b8a8,
        0x00b1_1f14,
        0,
    ];
    /// `graphicFontGetScreenWH(w, h)` (`0x00640db0`).
    pub const FONT_SCREEN_WH: u32 = 0x0064_0db0;
    /// `graphicGetMiniMapType()` (`0x00641320`).
    pub const MINI_MAP_TYPE: u32 = 0x0064_1320;
    /// `user_malloc(size)` (`0x00687020`).
    pub const USER_MALLOC: u32 = 0x0068_7020;
    /// `user_free_thunk(p)` (`0x00687010`).
    pub const USER_FREE_THUNK: u32 = 0x0068_7010;
    /// `Guide_MeshObject(guide)` (`0x0091d460`).
    pub const MESH_OBJECT: u32 = 0x0091_d460;
    /// `rcPacketDataInit(p)` (`0x00927a20`).
    pub const PACKET_INIT: u32 = 0x0092_7a20;
    /// `rcPacketDataOpenAdr(p, addr)` (`0x00927a70`).
    pub const PACKET_OPEN_ADR: u32 = 0x0092_7a70;
    /// The packet address the minimap fog opens (`0x00c498dc`).
    pub const FOG_PACKET: u32 = 0x00c4_98dc;
    /// `rcCopyQword_0091cbd0(dst, src)` (`0x0091cbd0`).
    pub const COPY_QWORD_CBD0: u32 = 0x0091_cbd0;
    /// `FFXVu0ApplyMatrix(dst, m, v)` (`0x006ed2e0`).
    pub const APPLY_MATRIX: u32 = 0x006e_d2e0;
    /// `FFXVu0DivVectorXYZ(dst, v, w)` (`0x006ed420`).
    pub const DIV_VECTOR_XYZ: u32 = 0x006e_d420;
    /// `updateVFXRenderDataRects(lo, hi, rects)` (`0x0071a330`).
    pub const UPDATE_RECTS: u32 = 0x0071_a330;
    /// `graphicVFX2DUpdate(lo, hi)` (`0x00644bc0`).
    pub const VFX_2D_UPDATE: u32 = 0x0064_4bc0;
    /// `InitVFXRenderDataRects(lo, hi, rects, flags)` (`0x00711350`).
    pub const INIT_RECTS: u32 = 0x0071_1350;
    /// `graphicVFXSetMeshInstanceName(lo, hi, name)` (`0x006456a0`).
    pub const SET_MESH_NAME: u32 = 0x0064_56a0;
    /// `"RenderMiniMapFog"` (`0x00b0da48`).
    pub const FOG_NAME: u32 = 0x00b0_da48;

    /// `rcbgGuideUpdateWalk(guide, m, p)` (`0x0091ecb0`): the minimap's fog
    /// of the unwalked: the screen layout (`graphicFontGetScreenWH`, the
    /// minimap type's offsets), then for each of the 64 rows of 40-unit
    /// cells around `p` whose band (`p.y` and `p.w` -/+ 40) the row's
    /// edges reach after `m` and the perspective divide, one quad of two
    /// triangles per cell (0x70 bytes, the cell's walked bit of the guide's
    /// bitmap `+0x70` as 0 or 2^32 in each vertex's third float) into a
    /// `user_malloc` buffer handed to the VFX rects of instance
    /// (`g_pppCurrentProgram`, guide) (`"RenderMiniMapFog"`), with the GIF
    /// tags of the packet at `0x00c498dc` around them. `work` is 0xb0 bytes
    /// (the original's frame: the rect block, three vectors, the start
    /// point, the packet, the screen size).
    #[allow(clippy::too_many_lines)] // the original's steps in turn
    pub fn rcbg_guide_update_walk<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        [g, m, p]: [u32; 3],
        work: u32,
        call: &mut C,
    ) {
        let w = |o: u32| work.wrapping_add(o);
        let (rect, a, b, v, start, pk, sw, sh) = (
            w(0),
            w(0x50),
            w(0x60),
            w(0x70),
            w(0x80),
            w(0x90),
            w(0xa0),
            w(0xa4),
        );
        let kd = |mem: &M, k: usize| {
            let at = mem.rebase(WALK_K[k]);
            mem.read::<f64>(at)
        };
        let kf = |mem: &M, k: usize| {
            let at = mem.rebase(WALK_K[k]);
            mem.read::<u32>(at)
        };
        let ld = x87::load;
        let st = x87::store;
        // the layout: x scale and offset, y scale and offset
        let (mut sx, mut ox, mut oy, mut sy) = (x87::copy(kf(mem, 0)), 0u32, 0u32, 0x3f80_0000u32);
        let f = mem.rebase(FONT_SCREEN_WH);
        call(mem, f, &[sw, sh]);
        let fw = ld(st(f64::from(mem.read::<i32>(sw))));
        let unit = st(fw * kd(mem, 1));
        let half_w = st(fw * kd(mem, 2));
        let fh = ld(st(f64::from(mem.read::<i32>(sh))));
        let scale_h = st(fh / kd(mem, 3));
        let half_h = st(fh * kd(mem, 2));
        let f = mem.rebase(MINI_MAP_TYPE);
        let kind = call(mem, f, &[]);
        if (1..=4).contains(&kind) {
            sx = st(ld(unit) * kd(mem, 4));
            let xo = if kind <= 2 { kd(mem, 6) } else { kd(mem, 5) };
            ox = st(ld(unit) * xo - ld(half_w));
            sy = scale_h;
            let yo = if kind % 2 == 1 {
                kd(mem, 7)
            } else {
                kd(mem, 8)
            };
            oy = st(ld(scale_h) * yo - ld(half_h));
        }
        let f = mem.rebase(USER_MALLOC);
        let buf = call(mem, f, &[0x7_0000]);
        mem.write(rect, buf);
        mem.write(rect.wrapping_add(4), 0u32);
        for (k, word) in (0u32..).zip([
            0x3f80_0000u32,
            0,
            0,
            0,
            0,
            0x3f80_0000,
            0,
            0,
            0,
            0,
            0x3f80_0000,
            0,
            0,
            0,
            0,
            0x3f80_0000,
        ]) {
            mem.write(rect.wrapping_add(0xc + 4 * k), word);
        }
        let f = mem.rebase(MESH_OBJECT);
        let mo = call(mem, f, &[g]);
        let (two, ten) = (kd(mem, 9), kd(mem, 10));
        let x0 = st((ld(mem.read::<u32>(mo.wrapping_add(0x90))) - two) * ten);
        let z0 = st((two + ld(mem.read::<u32>(mo.wrapping_add(0xa8)))) * ten);
        for (k, word) in (0u32..).zip([x0, 0, z0, 0x3f80_0000]) {
            mem.write(start.wrapping_add(4 * k), word);
        }
        let f = mem.rebase(PACKET_INIT);
        call(mem, f, &[pk]);
        let f = mem.rebase(PACKET_OPEN_ADR);
        let addr = mem.read::<u32>(mem.rebase(FOG_PACKET));
        call(mem, f, &[pk, addr]);
        let q = mem.read::<u32>(pk.wrapping_add(4));
        for (k, word) in
            (0u32..).zip([0x3_0000u32, 0, 0x47, 0, 0, 0, 1, 0, 0x3100_0108, 0, 0x4e, 0])
        {
            mem.write(q.wrapping_add(4 * k), word);
        }
        let (step, minus_cell) = (kd(mem, 11), kd(mem, 12));
        let lo_y = st(ld(mem.read::<u32>(p.wrapping_add(4))) - step);
        let hi_y = st(ld(mem.read::<u32>(p.wrapping_add(0xc))) - minus_cell);
        let cursor_end = q.wrapping_add(0x30);
        let copy = mem.rebase(COPY_QWORD_CBD0);
        let apply = mem.rebase(APPLY_MATRIX);
        let div = mem.rebase(DIV_VECTOR_XYZ);
        let big = ld(x87::copy(kf(mem, 13)));
        let mut cur = buf;
        let mut count = 0u32;
        let rd = |mem: &M, at: u32| mem.read::<u32>(at);
        for r in 0..0x40u32 {
            call(mem, copy, &[a, start]);
            call(mem, copy, &[b, start]);
            let fr = ld(st(f64::from(r)));
            let az = st(ld(rd(mem, a.wrapping_add(8))) - fr * step);
            mem.write(a.wrapping_add(8), az);
            mem.write(b.wrapping_add(8), st(ld(az) - step));
            call(mem, apply, &[v, m, b]);
            let vw = x87::copy(rd(mem, v.wrapping_add(0xc)));
            call(mem, div, &[v, v, vw]);
            // skip a row past the far edge (fcompp: hi_y < v.y)
            if ld(hi_y) < ld(rd(mem, v.wrapping_add(4))) {
                continue;
            }
            call(mem, apply, &[v, m, a]);
            let vw = x87::copy(rd(mem, v.wrapping_add(0xc)));
            call(mem, div, &[v, v, vw]);
            if ld(lo_y) > ld(rd(mem, v.wrapping_add(4))) {
                continue;
            }
            let row = mem.read::<u32>(g.wrapping_add(0x70)).wrapping_add(r * 8);
            let mut bits = u32::from(mem.read::<u8>(row));
            let mut next = row.wrapping_add(1);
            let mut mask = 1u32;
            for _ in 0..0x40 {
                let walked = mask & bits != 0;
                let flag = if walked { st(-1.0 + big) } else { 0 };
                let mut vertex = |mem: &mut M, src: u32, at: u32| {
                    call(mem, apply, &[v, m, src]);
                    let w = ld(rd(mem, v.wrapping_add(0xc)));
                    for k in 0..3 {
                        let c = v.wrapping_add(4 * k);
                        let x = st(ld(rd(mem, c)) / w);
                        mem.write(c, x);
                    }
                    let x = st(ld(rd(mem, v)) * ld(sx) + ld(ox));
                    let y = st(ld(rd(mem, v.wrapping_add(4))) * ld(sy) + ld(oy));
                    mem.write(cur.wrapping_add(at), x);
                    mem.write(cur.wrapping_add(at + 4), y);
                    mem.write(cur.wrapping_add(at + 8), flag);
                };
                vertex(mem, a, 0);
                vertex(mem, b, 0x1c);
                let ax = st(ld(rd(mem, a)) + step);
                mem.write(a, ax);
                let bx = st(step + ld(rd(mem, b)));
                mem.write(b, bx);
                vertex(mem, a, 0x38);
                vertex(mem, b, 0x54);
                mask <<= 1;
                cur = cur.wrapping_add(0x70);
                count += 1;
                mem.write(rect.wrapping_add(4), count);
                if mask == 0x100 {
                    bits = u32::from(mem.read::<u8>(next));
                    next = next.wrapping_add(1);
                    mask = 1;
                }
            }
        }
        let program = crate::task::bridge::CURRENT_PROGRAM.get(mem);
        let (lo, hi) = (
            program,
            g.wrapping_sub(u32::from(program.cast_signed() < 0)),
        );
        mem.write(rect.wrapping_add(8), 0x1000u32);
        let f = mem.rebase(super::VFX_GET_RENDER_DATA);
        if call(mem, f, &[lo, hi]) == 0 {
            let f = mem.rebase(INIT_RECTS);
            call(mem, f, &[lo, hi, rect, 0x802]);
            let f = mem.rebase(SET_MESH_NAME);
            let name = mem.rebase(FOG_NAME);
            call(mem, f, &[lo, hi, name]);
        } else {
            let f = mem.rebase(UPDATE_RECTS);
            call(mem, f, &[lo, hi, rect]);
            let f = mem.rebase(VFX_2D_UPDATE);
            call(mem, f, &[lo, hi]);
        }
        for (k, word) in (0u32..).zip([0x5_0000u32, 0, 0x47, 0, 0x3100_0108, 1, 0x4e, 0]) {
            mem.write(cursor_end.wrapping_add(4 * k), word);
        }
        let f = mem.rebase(USER_FREE_THUNK);
        call(mem, f, &[cur.wrapping_sub(count.wrapping_mul(0x70))]);
    }

    /// `rcbgGuideDrawPlayer`'s `.rdata` constants: -10000 (`f32`, the "no
    /// direction yet" mark), 2π, 15, the colour bases 204, 96, 32, 128,
    /// the triangle size 6.0 (`f32`) and 10 (`f64` unless noted).
    pub const PLAYER_K: [u32; 9] = [
        0x00b4_1db0,
        0x00b3_9378,
        0x00b9_22c0,
        0x00b6_76b0,
        0x00b5_ee40,
        0x00b4_12f8,
        0x00b4_c170,
        0x00b9_2308,
        0x00b9_22b0,
    ];
    /// The player marker's colour (`0x00c5e380`, three floats) and the
    /// circle's (`0x00c5e3c0`, four).
    pub const PLAYER_COLOUR: u32 = 0x00c5_e380;
    /// See [`PLAYER_COLOUR`].
    pub const CIRCLE_COLOUR: u32 = 0x00c5_e3c0;
    /// `yiSegIntersect2(a, b, c, d, out)` (`0x0091cd00`).
    pub const SEG_INTERSECT: u32 = 0x0091_cd00;
    /// `Yn_GuideEdgeFade(p, a, b, rect, out)` (`0x0091f530`).
    pub const EDGE_FADE: u32 = 0x0091_f530;
    /// `Yn_NearestOnLine2D(out, a, b, p, clamp)` (`0x0091cc00`).
    pub const NEAREST_ON_LINE: u32 = 0x0091_cc00;
    /// `rcVec3fNormalize(v)` (`0x00921360`).
    pub const NORMALIZE: u32 = 0x0092_1360;
    /// `Math_WrapAngle(a)` (`0x0088bc20`, `ST0`).
    pub const WRAP_ANGLE: u32 = 0x0088_bc20;
    /// `Math_AngleStep(cur, target, step)` (`0x0088b570`, `ST0`).
    pub const ANGLE_STEP: u32 = 0x0088_b570;
    /// `rcPrimTri(x, y, r, angle, colour, ui)` (`0x009270c0`).
    pub const PRIM_TRI: u32 = 0x0092_70c0;
    /// `rcPrimCircleLine(x, y, r, colour, ui)` (`0x00925ea0`).
    pub const PRIM_CIRCLE_LINE: u32 = 0x0092_5ea0;

    /// `rcbgGuideDrawPlayer(guide, a, b, rect, _, alpha, p, q, dir, smooth)`
    /// (`0x0091cf10`, name ours; from `rcbgGuideUpdateEx`): the player's
    /// marker on the minimap. The step `a`..`b` is clipped to `rect`'s
    /// edges (`yiSegIntersect2`; else `b`), `Yn_GuideEdgeFade` says
    /// whether the player is past the edge `p`..`q`; then the heading of
    /// `a` to `b` (`_CIacos` of the normalised step, `2π -` it for a
    /// positive y), or, past the edge or with `*dir` at -10000, the
    /// direction to the edge (stored in `*dir`); with `smooth` and inside,
    /// the heading turns from `*dir` at most the edge distance times the
    /// angle left (`Math_AngleStep`). The triangle (`rcPrimTri`, the colour
    /// faded by the guide's level `+0x74`) and the circle
    /// (`rcPrimCircleLine`, radius the level times 10) whose `eax` it
    /// returns. `work` is 0x80 bytes (the original's frame).
    #[allow(clippy::too_many_lines)] // the original's steps in turn
    pub fn rcbg_guide_draw_player<M, C, F>(
        mem: &mut M,
        [g, a, b, rect, alpha, p, q, dir, smooth]: [u32; 9],
        work: u32,
        call: &mut C,
        fcall: &mut F,
    ) -> u32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
        F: FnMut(&mut M, u32, crate::camera::FArgs<'_>) -> f64,
    {
        let w = |o: u32| work.wrapping_add(o);
        let (p1, p2, p3, p4, out, t, col, dist) = (
            w(0),
            w(0x10),
            w(0x20),
            w(0x30),
            w(0x40),
            w(0x50),
            w(0x60),
            w(0x70),
        );
        let kd = |mem: &M, k: usize| {
            let at = mem.rebase(PLAYER_K[k]);
            mem.read::<f64>(at)
        };
        let kf = |mem: &M, k: usize| {
            let at = mem.rebase(PLAYER_K[k]);
            mem.read::<u32>(at)
        };
        let (ld, st) = (x87::load, x87::store);
        let rd = |mem: &M, at: u32| mem.read::<u32>(at);
        let r = |mem: &M, k: u32| x87::copy(rd(mem, rect.wrapping_add(4 * k)));
        for (v, x, y) in [(p1, 0u32, 1u32), (p2, 2, 1), (p3, 0, 3), (p4, 2, 3)] {
            let (x, y) = (r(mem, x), r(mem, y));
            for (k, word) in (0u32..).zip([x, y, 0, 0]) {
                mem.write(v.wrapping_add(4 * k), word);
            }
        }
        let f = mem.rebase(SEG_INTERSECT);
        let hit = [(p1, p2), (p3, p4), (p1, p3), (p2, p4)]
            .into_iter()
            .any(|(c, d)| call(mem, f, &[a, b, c, d, out]) != 0);
        let f = mem.rebase(EDGE_FADE);
        let near = call(mem, f, &[a, p, q, rect, dist]);
        if !hit {
            let f = mem.rebase(COPY_QWORD_CBD0);
            call(mem, f, &[out, b]);
        }
        let norm = mem.rebase(NORMALIZE);
        let acos = mem.rebase(crate::vu0::CI_ACOS);
        let two_pi = kd(mem, 1);
        // the heading of the unit vector at `t`: acos x, 2π less it for y > 0
        let heading = |mem: &mut M, call: &mut C, fcall: &mut F| {
            call(mem, norm, &[t]);
            let x = ld(rd(mem, t));
            let ang = st(fcall(mem, acos, crate::camera::FArgs::St0(x)));
            if 0.0 < ld(rd(mem, t.wrapping_add(4))) {
                st(two_pi - ld(ang))
            } else {
                ang
            }
        };
        let mut inside = near == 0;
        #[allow(clippy::float_cmp)] // fucompp: exact
        let marked = dir != 0 && ld(rd(mem, dir)) == ld(kf(mem, 0));
        if marked {
            inside = false;
        }
        if marked || near != 0 {
            let f = mem.rebase(NEAREST_ON_LINE);
            call(mem, f, &[t, p, q, a, 0]);
            for k in 0..2u32 {
                let v = st(ld(rd(mem, t.wrapping_add(4 * k))) - ld(rd(mem, a.wrapping_add(4 * k))));
                mem.write(t.wrapping_add(4 * k), v);
            }
            mem.write(t.wrapping_add(0xc), 0u32);
            mem.write(t.wrapping_add(8), 0u32);
            let ang = heading(mem, call, fcall);
            if dir != 0 {
                mem.write(dir, ang);
            }
        }
        for k in 0..2u32 {
            let v = st(ld(rd(mem, b.wrapping_add(4 * k))) - ld(rd(mem, a.wrapping_add(4 * k))));
            mem.write(t.wrapping_add(4 * k), v);
        }
        mem.write(t.wrapping_add(0xc), 0u32);
        mem.write(t.wrapping_add(8), 0u32);
        let mut ang = heading(mem, call, fcall);
        if smooth != 0 && inside && dir != 0 {
            let cur = x87::copy(rd(mem, dir));
            let d = st(ld(cur) - ld(ang));
            let f = mem.rebase(WRAP_ANGLE);
            let wr = st(fcall(mem, f, crate::camera::FArgs::Stack(&[d])));
            let step = st(ld(wr & 0x7fff_ffff) * ld(rd(mem, dist)));
            let f = mem.rebase(ANGLE_STEP);
            ang = st(fcall(
                mem,
                f,
                crate::camera::FArgs::Stack(&[cur, ang, step]),
            ));
        }
        let level = mem.read::<u32>(g.wrapping_add(0x74)) & 0xf;
        let lv = st(ld(st(f64::from(level))) / kd(mem, 2));
        let c = mem.rebase(PLAYER_COLOUR);
        for (k, base) in [(0u32, 3usize), (1, 4), (2, 5)] {
            let base = kd(mem, base);
            let ck = ld(rd(mem, c.wrapping_add(4 * k)));
            mem.write(col.wrapping_add(4 * k), st(base + (ck - base) * ld(lv)));
        }
        mem.write(col.wrapping_add(0xc), st(ld(alpha) * kd(mem, 6)));
        let (ox, oy) = (
            x87::copy(rd(mem, out)),
            x87::copy(rd(mem, out.wrapping_add(4))),
        );
        let f = mem.rebase(PRIM_TRI);
        call(mem, f, &[ox, oy, x87::copy(kf(mem, 7)), ang, col, 0]);
        let f = mem.rebase(COPY_QWORD_CBD0);
        let cc = mem.rebase(CIRCLE_COLOUR);
        call(mem, f, &[col, cc]);
        let cw = st(ld(rd(mem, col.wrapping_add(0xc))) * ld(alpha));
        mem.write(col.wrapping_add(0xc), cw);
        let radius = st(ld(lv) * kd(mem, 8));
        let (ox, oy) = (
            x87::copy(rd(mem, out)),
            x87::copy(rd(mem, out.wrapping_add(4))),
        );
        let f = mem.rebase(PRIM_CIRCLE_LINE);
        call(mem, f, &[ox, oy, radius, col, 0])
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

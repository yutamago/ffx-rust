//! The guide map, the field's minimap (`rcbgGuide*`, `Guide_*`, `Yn_GuideMap*`;
//! `docs/formats/field.md`, `docs/renderer.md` "Minimap").
//!
//! `g_YnMapSystem` points at Yn's field state: `+0` the map, `+4` the
//! guide object. The map's `+8` holds the guide map list (the minimap
//! models of the map and which one shows).

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

/// Offsets in Yn's field state.
pub mod system {
    /// `+0x0`: the map.
    pub const MAP: u32 = 0x0;
    /// `+0x4`: the guide object.
    pub const GUIDE: u32 = 0x4;
}

/// Offsets in the map.
pub mod map {
    /// `+0x8`: the guide map list.
    pub const GUIDE_MAPS: u32 = 0x8;
}

/// Offsets in the guide map list.
pub mod guide_maps {
    /// `+0x18`: how many (`i16`, `Yn_GuideMapSetMapNo` bounds).
    pub const COUNT: u32 = 0x18;
    /// `+0x1a`: the one shown (`i16`).
    pub const CURRENT: u32 = 0x1a;
    /// `+0x1c`: the models, one pointer each.
    pub const MODELS: u32 = 0x1c;
}

/// Offsets in the guide object (`rcbgGuideNew`).
pub mod object {
    /// `+0x0`: the map's own minimap model (MAP1 chunk 0xb), or null.
    pub const MODEL: u32 = 0x0;
    /// `+0x8`: the model made from the walk mesh.
    pub const WALK_MODEL: u32 = 0x8;
    /// `+0x6c`: a float kept over battles.
    pub const BATTLE_KEPT: u32 = 0x6c;
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
        BATTLE_SAVE, FADE, FADE_DELAY, FADE_LEVEL, FADE_STATE, FADE_STEPS, FADE_WAIT, FLAGS, Fade,
        YN_MAP_SYSTEM, guide_maps, map, object, system,
    };
    use crate::x87;

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
        let v = x87::copy(mem.read(g + object::BATTLE_KEPT));
        BATTLE_SAVE.set(mem, v);
    }

    /// `rcbgGuideBattleEnd(guide)` (`0x0091cce0`): puts it back.
    pub fn rcbg_guide_battle_end(mem: &mut impl Mem, g: u32) {
        let v = x87::copy(BATTLE_SAVE.get(mem));
        mem.write(g + object::BATTLE_KEPT, v);
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
        match mem.read::<u32>(m + map::GUIDE_MAPS) {
            0 => -1,
            l => i32::from(mem.read::<i16>(l + guide_maps::CURRENT)),
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

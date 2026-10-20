//! The player character's control from the pad (`Sg_ControlPlayer`): the
//! left stick (or the d-pad) ramps four direction levels, their
//! differences give the walking speed and the stick's angle, which turns
//! with the camera's yaw into the character's direction.
//!
//! The C runtime's `_CIatan2` and `_CIatan` (in `MsVectorRect2Polar`, for
//! the camera's yaw) are the caller's: the bridges take them as callbacks.

/// The original over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_battle::pos::bridge::Trig;
    use ffx_bridge::x87::{self, Reg};
    use ffx_bridge::{Global, Host, Mem};
    use ffx_chara::chr::bridge as chr;
    use ffx_chara::layout as l;

    /// The pad's left stick, y and x (`u8`, 0x80 at rest, up and right
    /// above it; `Sg_ControlMain` reads them with
    /// `TkGetControlerAnalogHist` axes 3 and 2).
    pub const PAD_LSTICK_Y: Global<u8> = Global::new(0x023c_44a4);
    pub const PAD_LSTICK_X: Global<u8> = Global::new(0x023c_44a5);
    /// The pad's held buttons (`Sg_ControlMain`, `TkGetControlerHist`).
    pub const PAD_HELD: Global<u32> = Global::new(0x023c_44ac);
    /// The player character (`Sg_SetPlayer`), 0 for none.
    pub const PLAYER: Global<u32> = Global::new(0x0130_0788);
    /// The four direction levels, 0..=256: up, down, right, left (the
    /// d-pad's 0x1000, 0x4000, 0x2000, 0x8000; `Sg_SetPlayer` clears
    /// them).
    pub const LEVELS: [Global<i32>; 4] = [
        Global::new(0x0130_078c),
        Global::new(0x0130_0790),
        Global::new(0x0130_0794),
        Global::new(0x0130_0798),
    ];
    /// The camera's yaw as last taken (`f32`).
    pub const CAMERA_YAW: Global<f32> = Global::new(0x0130_079c);
    /// The stick's angle (`f32`, `atan2(right - left, up - down)`).
    pub const STICK_DIR: Global<f32> = Global::new(0x0130_07a0);
    /// The stick's angle when the camera's yaw was last taken (`f32`).
    pub const STICK_DIR_TAKEN: Global<f32> = Global::new(0x0130_07a4);
    /// An offset from the stick's angle (`f32`; `0x0082d950` sets it).
    pub const DIR_OFFSET: Global<f32> = Global::new(0x0130_0780);
    /// The stick, y and x, after the dead zones (`i32`).
    pub const STICK_Y: Global<i32> = Global::new(0x0130_07ac);
    pub const STICK_X: Global<i32> = Global::new(0x0130_07a8);
    /// `g_PlayerSpeed`: walk and run (picked by button 0x40), the motion
    /// type 2 speed at `+8`.
    pub const SPEED: u32 = 0x0130_07b4;
    pub const SPEED_TYPE2: Global<f32> = Global::new(0x0130_07bc);
    /// `g_PlayerDiving`.
    pub const DIVING: Global<u8> = Global::new(0x0130_07c5);
    /// `Sg_PlayerSetDirFix`'s flag and direction (`f32`).
    pub const DIR_FIX_ON: Global<u8> = Global::new(0x0130_07c4);
    pub const DIR_FIX: Global<f32> = Global::new(0x0130_07c8);
    /// `g_CameraFollow`: 3 turns the camera's yaw only when the stick
    /// turns far enough.
    pub const CAMERA_FOLLOW: Global<u8> = Global::new(0x00c4_96d5);

    /// The constants (`.rdata`, `f32` unless said): the stick x's dead zone
    /// bounds (88, 168) and its rest value (128); the analog dead zone
    /// bounds (-96, 96); the speed's top (256) and floor (48); the gravity
    /// bound (2) and step (0.2); the turn needed to take the camera again
    /// (0.349); the taken angle's reset (-1000); 1/256 (`f64`).
    pub const X_ZONE_HI: Global<f32> = Global::new(0x00b5_9838);
    pub const X_ZONE_LO: Global<f32> = Global::new(0x00b5_9834);
    pub const X_REST: Global<f32> = Global::new(0x00b5_01bc);
    pub const ZONE_HI: Global<f32> = Global::new(0x00b4_3db8);
    pub const ZONE_LO: Global<f32> = Global::new(0x00b5_983c);
    pub const SPEED_TOP: Global<f32> = Global::new(0x00b5_01c0);
    pub const SPEED_FLOOR: Global<f32> = Global::new(0x00b5_83d8);
    pub const GRAV_BOUND: Global<f32> = Global::new(0x00b9_2268);
    pub const GRAV_STEP: Global<f32> = Global::new(0x00b4_12c4);
    pub const TURN: Global<f32> = Global::new(0x00b5_982c);
    pub const TAKEN_RESET: Global<f32> = Global::new(0x00b4_5bbc);
    pub const SPEED_SCALE: Global<f64> = Global::new(0x00b5_0198);

    /// `Sg_GetPlayerControlEnable()`.
    pub const SG_GET_PLAYER_CONTROL_ENABLE: u32 = 0x0082_d870;
    /// `graphicLevelGetNameHash()`.
    pub const GRAPHIC_LEVEL_GET_NAME_HASH: u32 = 0x0064_1da0;
    /// The two levels whose camera yaw is taken every frame.
    pub const FOLLOW_LEVELS: [u32; 2] = [0x19c, 0x220];

    /// `Sg_CameraGetYaw()` (a `jmp` to `MsCameraGetYaw`): the yaw in `ST0`.
    pub const SG_CAMERA_GET_YAW: u32 = 0x0083_f2d0;
    /// pi as the original rounds it (3.141592, `f64`) and 180 (`f64`): degrees
    /// to radians.
    pub const DEG_PI: Global<f64> = Global::new(0x00b5_82f0);
    pub const DEG_180: Global<f64> = Global::new(0x00b4_8378);

    /// `Sg_PlayerSetControlAngle(deg)` (`0x0082d950`, name ours;
    /// `Native_SetControlAngle`'s): takes the camera's yaw now
    /// (`Sg_CameraGetYaw`'s `ST0` through `fcall`, stored as `f32`) into
    /// [`CAMERA_YAW`], and sets [`DIR_OFFSET`] to `Angle_Wrap(f32(deg *
    /// pi / 180))` plus the stick's angle. Like `Angle_Wrap`, it never
    /// ends for an infinite `deg`.
    pub fn set_control_angle<M: Mem>(
        mem: &mut M,
        deg: f32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) {
        let f = mem.rebase(SG_CAMERA_GET_YAW);
        let yaw = fcall(mem, f, &[]);
        CAMERA_YAW.set(mem, f32::from_bits(x87::store(yaw)));
        let pi = Reg::constant(DEG_PI.get(mem));
        let half_turn = Reg::constant(DEG_180.get(mem));
        let rad = (Reg::load(deg) * pi / half_turn).store();
        let wrapped = crate::vec::bridge::angle_wrap(rad);
        let offset = Reg::load(wrapped).add_m32(STICK_DIR.get(mem)).store();
        DIR_OFFSET.set(mem, offset);
    }

    fn call_sym(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        let at = h.rebase(sym);
        h.call(at, args)
    }

    /// `f32` of an `i32` (`fild`, `fstp dword`).
    #[allow(clippy::cast_precision_loss)] // the original rounds alike
    fn f32_of(v: i32) -> f32 {
        f32::from_bits(x87::store(f64::from(v)))
    }

    /// A level ramping up toward a stick value `v` (`v` below it in the
    /// original's sense, the absolute value given): 32 more, held at `v`.
    fn ramp_up(h: &mut impl Host, g: Global<i32>, v: f64) {
        let c = g.get(h).wrapping_add(0x20);
        g.set(h, c);
        if v < f64::from(f32_of(c)) {
            g.set(h, x87::ftol(h, v).cast_signed());
        }
    }

    /// A level ramping down: 32 less, 0 at most; true when it hit 0.
    fn ramp_down(h: &mut impl Host, g: Global<i32>) -> bool {
        let c = g.get(h).wrapping_sub(0x20);
        g.set(h, c);
        if c < 0 {
            g.set(h, 0);
            true
        } else {
            false
        }
    }

    /// A d-pad level: 32 more up to 256 while `held`, else 32 less down to 0.
    fn digital(h: &mut impl Host, g: Global<i32>, held: bool) {
        let c = g.get(h);
        let v = if held {
            let v = c.wrapping_add(0x20);
            if v > 0x100 { 0x100 } else { v }
        } else {
            let v = c.wrapping_sub(0x20);
            if v < 0 { 0 } else { v }
        };
        g.set(h, v);
    }

    /// `Sg_ControlPlayer()` (`0x0082d180`): steers the player from the pad.
    ///
    /// The stick's y and x go through dead zones (y 0x59..=0xa7 and x
    /// between 88 and 168 count as 0x80) into [`STICK_Y`] and [`STICK_X`].
    /// With a player whose control is enabled, the d-pad (any of buttons
    /// 0xf000) ramps the four [`LEVELS`] by 32 up to 256 and down to 0;
    /// otherwise the stick, centred and doubled with its own dead zone
    /// (-96..96), ramps the level of its side up to its value and the
    /// opposite level down. The speed is the length of the level
    /// differences (above 256 held at 256, below 48 none), their `atan2`
    /// the stick's angle ([`STICK_DIR`], kept when the stick sits in the
    /// dead zone with every falling level at 0). The camera's yaw is taken
    /// each frame, or with follow mode 3 only when the angle turned 0.349
    /// from the last time (and every frame on two levels). The character
    /// gets the speed (times its walk, run or type 2 speed, over 256) and
    /// the direction `-(angle - offset - yaw)` (the fixed direction instead
    /// of the yaw when set); standing still, speed 0. Motion type 2 with
    /// button 0x40 while diving adds gravity below 2.
    ///
    /// `trig` is `_CIatan` for the camera's yaw, `atan2` the CRT's
    /// `_CIatan2(y, x)` (both `f32` bits in and out).
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)] // the original's axes
    pub fn control_player(
        h: &mut impl Host,
        trig: &dyn Fn(Trig, u32) -> u32,
        atan2: &dyn Fn(u32, u32) -> u32,
    ) {
        let mut sy = i32::from(PAD_LSTICK_Y.get(h));
        let mut fx = f64::from(f32_of(i32::from(PAD_LSTICK_X.get(h))));
        if fx < f64::from(X_ZONE_HI.get(h)) && f64::from(X_ZONE_LO.get(h)) < fx {
            fx = f64::from(x87::round_trip(X_REST.get(h)));
        }
        if sy.wrapping_sub(0x59).cast_unsigned() <= 0x4e {
            sy = 0x80;
        }
        let sx = x87::ftol(h, fx).cast_signed();
        let held = PAD_HELD.get(h);
        STICK_Y.set(h, sy);
        STICK_X.set(h, sx);
        let player = PLAYER.get(h);
        if player == 0 {
            return;
        }
        chr::ch_en_rot_hokan(h, player, 1);
        if call_sym(h, SG_GET_PLAYER_CONTROL_ENABLE, &[]) == 0 {
            chr::ch_set_f32(h, player, l::SP, 0.0);
            return;
        }
        let [up, down, right, left] = LEVELS;
        // the three "keep the angle" conditions: both axes in the dead
        // zone, a falling level of each axis that hit 0
        let (mut zone, mut v_floor, mut x_floor) = (false, false, false);
        if held & 0xf000 == 0 {
            // y doubled about the rest (up negative), x doubled the other way
            let y = f64::from(f32_of(STICK_Y.get(h).wrapping_mul(2).wrapping_sub(0x100)));
            let x = f64::from(f32_of(0x80i32.wrapping_sub(STICK_X.get(h)).wrapping_mul(2)));
            let (hi, lo) = (f64::from(ZONE_HI.get(h)), f64::from(ZONE_LO.get(h)));
            let y_zone = y < hi && lo < y;
            let y = if y_zone { 0.0 } else { y };
            let x_zone = x < hi && lo < x;
            zone = y_zone && x_zone;
            let x = if x_zone { 0.0 } else { x };
            if y < 0.0 {
                ramp_up(h, up, f64::from(f32::from_bits(x87::store(-y))));
            } else {
                v_floor = ramp_down(h, up);
            }
            if y > 0.0 {
                ramp_up(h, down, y);
                v_floor = false;
            } else if ramp_down(h, down) {
                v_floor = true;
            }
            if x < 0.0 {
                ramp_up(h, right, f64::from(f32::from_bits(x87::store(-x))));
            } else {
                x_floor = ramp_down(h, right);
            }
            if x > 0.0 {
                ramp_up(h, left, x);
                x_floor = false;
            } else if ramp_down(h, left) {
                x_floor = true;
            }
        } else {
            digital(h, up, held & 0x1000 != 0);
            digital(h, down, held & 0x4000 != 0);
            digital(h, right, held & 0x2000 != 0);
            digital(h, left, held & 0x8000 != 0);
        }
        let a = f32_of(right.get(h).wrapping_sub(left.get(h)));
        let b = f32_of(up.get(h).wrapping_sub(down.get(h)));
        let moving = !(a == 0.0 && b == 0.0);
        let mut speed = 0.0f32;
        if moving {
            let (ra, rb) = (Reg::load(a), Reg::load(b));
            let v = (ra * ra + rb * rb).sqrt().store();
            let top = SPEED_TOP.get(h);
            speed = if f64::from(v) > f64::from(top) {
                x87::round_trip(top)
            } else if f64::from(v) < f64::from(SPEED_FLOOR.get(h)) {
                0.0
            } else {
                v
            };
        }
        if ffx_chara::parts::bridge::ch_get_motion_type(h, player) == 2
            && held & 0x40 != 0
            && DIVING.get(h) != 0
            && f64::from(chr::ch_get_grav(h, player)) < f64::from(GRAV_BOUND.get(h))
        {
            let g = x87::round_trip(GRAV_STEP.get(h));
            chr::ch_add_grav(h, player, g);
        }
        if !(zone && x_floor && v_floor) {
            let d = f32::from_bits(atan2(a.to_bits(), b.to_bits()));
            STICK_DIR.set(h, x87::round_trip(d));
        }
        let yaw = |h: &mut _| {
            let y = crate::camera::bridge::camera_get_yaw(h, trig);
            CAMERA_YAW.set(h, x87::round_trip(y));
        };
        if CAMERA_FOLLOW.get(h) == 3 {
            if !moving {
                yaw(h);
                STICK_DIR_TAKEN.set(h, x87::round_trip(TAKEN_RESET.get(h)));
                DIR_OFFSET.set(h, 0.0);
                chr::ch_set_f32(h, player, l::SP, 0.0);
                return;
            }
            let turn = Reg::load(STICK_DIR.get(h))
                .sub_m32(STICK_DIR_TAKEN.get(h))
                .value()
                .abs();
            let wrap = crate::vec::bridge::angle_wrap(x87::round_trip(TURN.get(h)));
            if f64::from(wrap) < turn {
                yaw(h);
                STICK_DIR_TAKEN.set(h, x87::round_trip(STICK_DIR.get(h)));
                DIR_OFFSET.set(h, 0.0);
            }
            if call_sym(h, GRAPHIC_LEVEL_GET_NAME_HASH, &[]) == FOLLOW_LEVELS[0]
                || call_sym(h, GRAPHIC_LEVEL_GET_NAME_HASH, &[]) == FOLLOW_LEVELS[1]
            {
                yaw(h);
            }
        } else {
            yaw(h);
            DIR_OFFSET.set(h, 0.0);
            if !moving {
                chr::ch_set_f32(h, player, l::SP, 0.0);
                return;
            }
        }
        let base = if ffx_chara::parts::bridge::ch_get_motion_type(h, player) == 2 {
            SPEED_TYPE2.get(h)
        } else {
            let i = !(held >> 6) & 1;
            Global::<f32>::new(SPEED + 4 * i).get(h)
        };
        let k = Reg::constant(SPEED_SCALE.get(h));
        let sp = (Reg::load(base).mul_m32(speed) * k).store();
        chr::ch_set_f32(h, player, l::SP, sp);
        let t = Reg::load(STICK_DIR.get(h))
            .sub_m32(DIR_OFFSET.get(h))
            .store();
        let u = if DIR_FIX_ON.get(h) != 0 {
            DIR_FIX.get(h)
        } else {
            CAMERA_YAW.get(h)
        };
        let w = Reg::load(t).sub_m32(x87::round_trip(u)).store();
        chr::ch_set_f32(h, player, l::DIR, (-Reg::load(w)).store());
    }
}

//! A free camera for looking around the scene (the overlay's "Free
//! camera"), driven as Blender's 3D viewport is: the middle button orbits
//! about the view's centre point, Shift+middle pans, Ctrl+middle (up and
//! down) and the wheel zoom, Ctrl+wheel and Shift+wheel pan across and up;
//! the number pad gives the front (1), right (3) and top (7) views (Ctrl:
//! the opposite ones), orbits by 15° (2, 4, 6, 8), turns round (9), zooms
//! (+, −) and centres on the selected light (.). Shift+grave (the key left
//! of 1) walks, as Blender's walk navigation: the mouse looks (the pointer
//! is held), W/A/S/D or the arrows move, Q/E go down and up, Shift runs,
//! Alt is slow, the wheel or +/− sets the speed; the left button or Enter
//! keeps the view, the right button or Esc goes back. The hook puts it
//! into the game's own override camera (`Cam_OverrideSet`, see
//! `symbols/ffx.sym`), so the game draws, and culls, for it, with the
//! game's field of view narrowed or widened by the zoom. Positions in
//! render space (y up); the game's are x and y negated.

use std::collections::HashSet;

use ffx_gfx::Input;

/// World units in a metre (a character is about 1.75 m, 20 units).
const UNITS_PER_METRE: f32 = 20.0 / 1.75;
/// Blender's defaults: turntable orbit 0.4° a pixel, walking 2.5 m/s,
/// running five times as fast, 15° number pad steps, 1.2 zoom steps.
const ORBIT_PER_PIXEL: f32 = 0.4 * std::f32::consts::PI / 180.0;
const LOOK_PER_PIXEL: f32 = 0.0025;
const WALK_SPEED: f32 = 2.5 * UNITS_PER_METRE;
const RUN: f32 = 5.0;
const STEP: f32 = 15.0 * std::f32::consts::PI / 180.0;
const ZOOM_STEP: f32 = 1.2;
/// Just short of straight up or down (the game's camera is a look-at one).
const PITCH_LIMIT: f32 = 1.553;

/// What a middle-button drag does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Drag {
    Orbit,
    Pan,
    Zoom,
}

#[allow(clippy::struct_excessive_bools)] // independent states of the input
pub struct FreeCam {
    pub on: bool,
    /// Turning it on was asked for (it starts where the game's camera is,
    /// which the renderer knows).
    pub start_asked: bool,
    pub eye: [f32; 3],
    /// Radians: about y (0 looks along +z), and up.
    pub yaw: f32,
    pub pitch: f32,
    /// From the eye to the point it orbits (Blender's view distance).
    pub distance: f32,
    /// Walking, world units a second.
    pub speed: f32,
    /// The game's vertical field of view when it was turned on (radians),
    /// and how much narrower it is now.
    pub fov: f32,
    pub zoom: f32,
    /// Frames since it was turned on (the override reaches the game a
    /// frame or two later).
    pub frames: u32,
    /// The screen's height in pixels (pans follow the pointer).
    pub screen_height: f32,
    /// Centring on the selected light was asked for (number pad .).
    pub focus_asked: bool,
    /// Walking, and the view it started from (eye, yaw, pitch).
    walk: Option<([f32; 3], f32, f32)>,
    /// 1 or −1: whether (cos yaw, 0, −sin yaw) is the screen's right or
    /// its left (render space's handedness, from the game's view).
    hand: f32,
    held: HashSet<u32>,
    drag: Option<Drag>,
    pointer: Option<(f32, f32)>,
    /// Pointer moves since the last frame (dragging, walking).
    moved: (f32, f32),
    wheel: f32,
}

impl Default for FreeCam {
    fn default() -> Self {
        Self {
            on: false,
            start_asked: false,
            eye: [0.0; 3],
            yaw: 0.0,
            pitch: 0.0,
            distance: 100.0,
            speed: WALK_SPEED,
            fov: 1.0,
            zoom: 1.0,
            frames: 0,
            screen_height: 720.0,
            focus_asked: false,
            walk: None,
            hand: 1.0,
            held: HashSet::new(),
            drag: None,
            pointer: None,
            moved: (0.0, 0.0),
            wheel: 0.0,
        }
    }
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-6);
    [v[0] / l, v[1] / l, v[2] / l]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Virtual keys.
const SHIFT: u32 = 0x10;
const CTRL: u32 = 0x11;
const ALT: u32 = 0x12;
const ENTER: u32 = 0x0d;
const ESC: u32 = 0x1b;
/// The key left of 1: the grave accent on US keyboards (`VK_OEM_3`), ^ on
/// German ones (`VK_OEM_5`).
const GRAVE: [u32; 2] = [0xc0, 0xdc];
const PLUS: [u32; 2] = [0x6b, 0xbb];
const MINUS: [u32; 2] = [0x6d, 0xbd];

impl FreeCam {
    /// Starts at the game's camera (render space): its eye, the point it
    /// looks at (the orbit's centre), the screen's right, and its
    /// projection's `scale` (row 1, column 1).
    pub fn start(&mut self, eye: [f32; 3], look_at: [f32; 3], right: [f32; 3], scale: f32) {
        let to = [
            look_at[0] - eye[0],
            look_at[1] - eye[1],
            look_at[2] - eye[2],
        ];
        let d = normalize(to);
        self.eye = eye;
        self.yaw = d[0].atan2(d[2]);
        self.pitch = d[1]
            .clamp(-1.0, 1.0)
            .asin()
            .clamp(-PITCH_LIMIT, PITCH_LIMIT);
        self.distance = (to[0] * to[0] + to[1] * to[1] + to[2] * to[2])
            .sqrt()
            .clamp(1.0, 5000.0);
        let along = right[0] * self.yaw.cos() - right[2] * self.yaw.sin();
        self.hand = if along < 0.0 { -1.0 } else { 1.0 };
        self.fov = if scale.abs() > 1e-3 {
            2.0 * (1.0 / scale.abs()).atan()
        } else {
            1.0
        };
        self.zoom = 1.0;
        self.frames = 0;
        self.walk = None;
        self.drag = None;
        self.on = true;
    }

    /// Where it looks.
    #[must_use]
    pub fn forward(&self) -> [f32; 3] {
        [
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        ]
    }

    /// The screen's right, on the ground.
    #[must_use]
    pub fn right(&self) -> [f32; 3] {
        [self.hand * self.yaw.cos(), 0.0, -self.hand * self.yaw.sin()]
    }

    /// The screen's up.
    #[must_use]
    pub fn up(&self) -> [f32; 3] {
        let u = normalize(cross(self.right(), self.forward()));
        if u[1] < 0.0 { [-u[0], -u[1], -u[2]] } else { u }
    }

    /// The point it orbits.
    #[must_use]
    pub fn pivot(&self) -> [f32; 3] {
        let f = self.forward();
        [0, 1, 2].map(|k| self.eye[k] + f[k] * self.distance)
    }

    /// Walking (the renderer holds the pointer meanwhile).
    #[must_use]
    pub fn walking(&self) -> bool {
        self.on && self.walk.is_some()
    }

    /// Turns to (yaw, pitch) about the pivot.
    fn orbit_to(&mut self, yaw: f32, pitch: f32) {
        let p = self.pivot();
        self.yaw = yaw;
        self.pitch = pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT);
        let f = self.forward();
        self.eye = [0, 1, 2].map(|k| p[k] - f[k] * self.distance);
    }

    /// Moves to `distance` from the pivot.
    fn zoom_to(&mut self, distance: f32) {
        let p = self.pivot();
        self.distance = distance.clamp(0.5, 20000.0);
        let f = self.forward();
        self.eye = [0, 1, 2].map(|k| p[k] - f[k] * self.distance);
    }

    /// Moves eye and pivot by (x, y) pixels across the screen (the pivot's
    /// depth follows the pointer).
    fn pan(&mut self, x: f32, y: f32) {
        let fov = 2.0 * ((self.fov * 0.5).tan() / self.zoom.max(0.05)).atan();
        let per_pixel = 2.0 * self.distance * (fov * 0.5).tan() / self.screen_height.max(1.0);
        let (r, u) = (self.right(), self.up());
        for k in 0..3 {
            self.eye[k] += (-r[k] * x + u[k] * y) * per_pixel;
        }
    }

    /// Centres on `p`, keeping the direction (number pad .).
    pub fn focus(&mut self, p: [f32; 3]) {
        let f = self.forward();
        self.eye = [0, 1, 2].map(|k| p[k] - f[k] * self.distance);
    }

    fn key(&mut self, vk: u32) {
        let ctrl = self.held.contains(&CTRL);
        let shift = self.held.contains(&SHIFT);
        if shift && GRAVE.contains(&vk) {
            self.walk = match self.walk {
                None => Some((self.eye, self.yaw, self.pitch)),
                Some(_) => None,
            };
            self.drag = None;
            return;
        }
        if let Some((eye, yaw, pitch)) = self.walk {
            if vk == ESC {
                (self.eye, self.yaw, self.pitch) = (eye, yaw, pitch);
                self.walk = None;
            } else if vk == ENTER {
                self.walk = None;
            } else if PLUS.contains(&vk) {
                self.speed = (self.speed * ZOOM_STEP).min(5000.0);
            } else if MINUS.contains(&vk) {
                self.speed = (self.speed / ZOOM_STEP).max(0.5);
            }
            return;
        }
        let half = std::f32::consts::FRAC_PI_2;
        let opposite = if ctrl { std::f32::consts::PI } else { 0.0 };
        match vk {
            // Front (Ctrl: back), right (Ctrl: left), top (Ctrl: bottom).
            0x61 => self.orbit_to(opposite, 0.0),
            0x63 => self.orbit_to(-self.hand * half + opposite, 0.0),
            0x67 => self.orbit_to(0.0, if ctrl { PITCH_LIMIT } else { -PITCH_LIMIT }),
            // Orbits: 4 and 6 about the up axis, 8 and 2 over the top.
            0x64 => self.orbit_to(self.yaw + self.hand * STEP, self.pitch),
            0x66 => self.orbit_to(self.yaw - self.hand * STEP, self.pitch),
            0x68 => self.orbit_to(self.yaw, self.pitch - STEP),
            0x62 => self.orbit_to(self.yaw, self.pitch + STEP),
            0x69 => self.orbit_to(self.yaw + std::f32::consts::PI, self.pitch),
            0x6e => self.focus_asked = true,
            _ if PLUS.contains(&vk) => self.zoom_to(self.distance / ZOOM_STEP),
            _ if MINUS.contains(&vk) => self.zoom_to(self.distance * ZOOM_STEP),
            _ => {}
        }
    }

    /// An input event; `free`: the pointer is over the scene (not a panel).
    pub fn feed(&mut self, e: &Input, free: bool) {
        match *e {
            Input::Key { vk, pressed } => {
                if pressed {
                    if self.held.insert(vk) {
                        self.key(vk);
                    }
                } else {
                    self.held.remove(&vk);
                }
            }
            Input::Button { button, pressed } => {
                if let Some((eye, yaw, pitch)) = self.walk {
                    if pressed && button == 1 {
                        (self.eye, self.yaw, self.pitch) = (eye, yaw, pitch);
                        self.walk = None;
                    } else if pressed && button == 0 {
                        self.walk = None;
                    }
                } else if button == 2 {
                    self.drag = (pressed && free).then(|| {
                        if self.held.contains(&SHIFT) {
                            Drag::Pan
                        } else if self.held.contains(&CTRL) {
                            Drag::Zoom
                        } else {
                            Drag::Orbit
                        }
                    });
                }
            }
            Input::Move(x, y) => {
                if let Some((px, py)) = self.pointer
                    && self.drag.is_some()
                {
                    self.moved.0 += x - px;
                    self.moved.1 += y - py;
                }
                self.pointer = Some((x, y));
            }
            Input::Motion(x, y) if self.walk.is_some() => {
                self.moved.0 += x;
                self.moved.1 += y;
            }
            Input::Wheel(d) if free || self.walk.is_some() => self.wheel += d,
            _ => {}
        }
    }

    /// Moves it by what was held and dragged over `dt` seconds.
    pub fn update(&mut self, dt: f32) {
        let (mx, my) = std::mem::take(&mut self.moved);
        let wheel = std::mem::take(&mut self.wheel);
        if self.walk.is_some() {
            self.walk_step(mx, my, wheel, dt);
            return;
        }
        match self.drag {
            // The scene follows the pointer, as if held.
            Some(Drag::Orbit) => self.orbit_to(
                self.yaw + self.hand * mx * ORBIT_PER_PIXEL,
                self.pitch - my * ORBIT_PER_PIXEL,
            ),
            Some(Drag::Pan) => self.pan(mx, my),
            Some(Drag::Zoom) => self.zoom_to(self.distance * (my * 0.005).exp()),
            None => {}
        }
        if wheel != 0.0 {
            let step = self.screen_height * 0.05 * wheel;
            if self.held.contains(&CTRL) {
                self.pan(-step, 0.0);
            } else if self.held.contains(&SHIFT) {
                self.pan(0.0, step);
            } else {
                self.zoom_to(self.distance / ZOOM_STEP.powf(wheel));
            }
        }
    }

    fn walk_step(&mut self, mx: f32, my: f32, wheel: f32, dt: f32) {
        self.yaw += self.hand * mx * LOOK_PER_PIXEL;
        self.pitch = (self.pitch - my * LOOK_PER_PIXEL).clamp(-PITCH_LIMIT, PITCH_LIMIT);
        if wheel != 0.0 {
            self.speed = (self.speed * ZOOM_STEP.powf(wheel)).clamp(0.5, 5000.0);
        }
        let (f, r) = (self.forward(), self.right());
        let up = [0.0, 1.0, 0.0];
        let key = |vk: u32| self.held.contains(&vk);
        let mut step = [0.0f32; 3];
        let mut add = |v: [f32; 3], s: f32| {
            for k in 0..3 {
                step[k] += v[k] * s;
            }
        };
        if key(u32::from(b'W')) || key(0x26) {
            add(f, 1.0);
        }
        if key(u32::from(b'S')) || key(0x28) {
            add(f, -1.0);
        }
        if key(u32::from(b'D')) || key(0x27) {
            add(r, 1.0);
        }
        if key(u32::from(b'A')) || key(0x25) {
            add(r, -1.0);
        }
        if key(u32::from(b'E')) {
            add(up, 1.0);
        }
        if key(u32::from(b'Q')) {
            add(up, -1.0);
        }
        let pace = if key(SHIFT) {
            RUN
        } else if key(ALT) {
            1.0 / RUN
        } else {
            1.0
        };
        let dist = self.speed * pace * dt.clamp(0.0, 0.1);
        // (A zero step stays zero.)
        let step = normalize(step);
        for (e, s) in self.eye.iter_mut().zip(step) {
            *e += s * dist;
        }
    }

    /// The override for the game: eye and look-at in game space, the
    /// vertical field of view (radians) and the roll.
    #[must_use]
    pub fn game(&self) -> [f32; 8] {
        let f = self.forward();
        let look = [
            self.eye[0] + f[0] * 100.0,
            self.eye[1] + f[1] * 100.0,
            self.eye[2] + f[2] * 100.0,
        ];
        [
            -self.eye[0],
            -self.eye[1],
            self.eye[2],
            -look[0],
            -look[1],
            look[2],
            2.0 * ((self.fov * 0.5).tan() / self.zoom.max(0.05)).atan(),
            0.0,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(c: &mut FreeCam, vk: u32, pressed: bool) {
        c.feed(&Input::Key { vk, pressed }, true);
    }

    #[test]
    #[allow(clippy::float_cmp)] // Esc puts the very view back
    fn navigates_as_blender_does() {
        let mut c = FreeCam::default();
        // Looking along +z with the screen's right at −x.
        c.start([0.0, 10.0, 0.0], [0.0, 10.0, 50.0], [-1.0, 0.0, 0.0], 2.0);
        assert!(c.yaw.abs() < 1e-5 && c.pitch.abs() < 1e-5);
        assert!((c.distance - 50.0).abs() < 1e-4);
        assert!((c.right()[0] + 1.0).abs() < 1e-5, "{:?}", c.right());
        // A middle drag to the right orbits about the pivot: the eye goes
        // left (+x), the pivot stays.
        c.feed(&Input::Move(100.0, 100.0), true);
        c.feed(
            &Input::Button {
                button: 2,
                pressed: true,
            },
            true,
        );
        c.feed(&Input::Move(150.0, 100.0), true);
        c.update(0.016);
        let p = c.pivot();
        assert!(p[0].abs() < 1e-3 && (p[2] - 50.0).abs() < 1e-3, "{p:?}");
        assert!(c.eye[0] > 1.0, "{:?}", c.eye);
        c.feed(
            &Input::Button {
                button: 2,
                pressed: false,
            },
            true,
        );
        // The wheel away zooms in.
        c.feed(&Input::Wheel(1.0), true);
        c.update(0.016);
        assert!((c.distance - 50.0 / 1.2).abs() < 1e-3);
        // Walking: Shift+`, W goes ahead, Esc goes back.
        let before = c.eye;
        press(&mut c, SHIFT, true);
        press(&mut c, 0xc0, true);
        press(&mut c, SHIFT, false);
        assert!(c.walking());
        press(&mut c, u32::from(b'W'), true);
        c.update(0.1);
        assert_ne!(c.eye, before);
        press(&mut c, ESC, true);
        assert!(!c.walking() && c.eye == before);
        let g = c.game();
        assert!((g[1] + 10.0).abs() < 1e-3, "y is negated");
        // A projection scale of 2: tan(fov / 2) = 1/2; zoom 2 halves it.
        assert!((g[6] - 2.0 * 0.5f32.atan()).abs() < 1e-5);
        c.zoom = 2.0;
        assert!((c.game()[6] - 2.0 * 0.25f32.atan()).abs() < 1e-5);
    }
}

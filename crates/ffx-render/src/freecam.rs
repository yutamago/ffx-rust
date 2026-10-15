//! A free camera for looking around the scene (the overlay's "Free
//! camera"): right-drag looks, W/A/S/D move, Q/E go down and up, Shift is
//! faster, the wheel sets the speed, a middle drag pans. The hook puts it
//! into the game's own override camera (`Cam_OverrideSet`, see
//! `symbols/ffx.sym`), so the game draws, and culls, for it, with the
//! game's field of view narrowed or widened by the zoom. Positions in
//! render space (y up); the game's are x and y negated.

use std::collections::HashSet;

use ffx_gfx::Input;

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
    /// World units a second.
    pub speed: f32,
    /// The game's vertical field of view when it was turned on (radians),
    /// and how much narrower it is now.
    pub fov: f32,
    pub zoom: f32,
    /// Frames since it was turned on (the override reaches the game a
    /// frame or two later).
    pub frames: u32,
    held: HashSet<u32>,
    look: bool,
    pan: bool,
    pointer: Option<(f32, f32)>,
    /// Pointer moves since the last frame (looking, panning).
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
            speed: 60.0,
            fov: 1.0,
            zoom: 1.0,
            frames: 0,
            held: HashSet::new(),
            look: false,
            pan: false,
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

impl FreeCam {
    /// Starts at the game's camera (render space), whose projection has
    /// `scale` (row 1, column 1).
    pub fn start(&mut self, eye: [f32; 3], look_at: [f32; 3], scale: f32) {
        let d = normalize([
            look_at[0] - eye[0],
            look_at[1] - eye[1],
            look_at[2] - eye[2],
        ]);
        self.eye = eye;
        self.yaw = d[0].atan2(d[2]);
        self.pitch = d[1].clamp(-1.0, 1.0).asin();
        self.fov = if scale.abs() > 1e-3 {
            2.0 * (1.0 / scale.abs()).atan()
        } else {
            1.0
        };
        self.zoom = 1.0;
        self.frames = 0;
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

    /// An input event; `free`: the pointer is over the scene (not a panel).
    pub fn feed(&mut self, e: &Input, free: bool) {
        match *e {
            Input::Key { vk, pressed } => {
                if pressed {
                    self.held.insert(vk);
                } else {
                    self.held.remove(&vk);
                }
            }
            Input::Button { button, pressed } => {
                if button == 1 {
                    self.look = pressed && (free || self.look);
                } else if button == 2 {
                    self.pan = pressed && (free || self.pan);
                }
            }
            Input::Move(x, y) => {
                if let Some((px, py)) = self.pointer
                    && (self.look || self.pan)
                {
                    self.moved.0 += x - px;
                    self.moved.1 += y - py;
                }
                self.pointer = Some((x, y));
            }
            Input::Wheel(d) if free => self.wheel += d,
            _ => {}
        }
    }

    /// Moves it by what was held and dragged over `dt` seconds.
    pub fn update(&mut self, dt: f32) {
        let (mx, my) = std::mem::take(&mut self.moved);
        if self.look {
            self.yaw += mx * 0.004;
            self.pitch = (self.pitch - my * 0.004).clamp(-1.55, 1.55);
        }
        let wheel = std::mem::take(&mut self.wheel);
        if wheel != 0.0 {
            self.speed = (self.speed * 1.25f32.powf(wheel)).clamp(1.0, 5000.0);
        }
        let f = self.forward();
        let right = normalize([f[2], 0.0, -f[0]]);
        let up = [0.0, 1.0, 0.0];
        let key = |c: u8| self.held.contains(&u32::from(c));
        let mut step = [0.0f32; 3];
        let mut add = |v: [f32; 3], s: f32| {
            for k in 0..3 {
                step[k] += v[k] * s;
            }
        };
        if key(b'W') {
            add(f, 1.0);
        }
        if key(b'S') {
            add(f, -1.0);
        }
        if key(b'D') {
            add(right, 1.0);
        }
        if key(b'A') {
            add(right, -1.0);
        }
        if key(b'E') {
            add(up, 1.0);
        }
        if key(b'Q') {
            add(up, -1.0);
        }
        // Shift (0x10).
        let fast = if self.held.contains(&0x10) { 4.0 } else { 1.0 };
        let dist = self.speed * fast * dt.clamp(0.0, 0.1);
        // (A zero step stays zero.)
        let step = normalize(step);
        for (e, s) in self.eye.iter_mut().zip(step) {
            *e += s * dist;
        }
        if self.pan {
            let s = self.speed * 0.003;
            for k in 0..3 {
                self.eye[k] += -right[k] * mx * s + up[k] * my * s;
            }
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

    #[test]
    fn starts_where_the_game_looks_and_moves() {
        let mut c = FreeCam::default();
        c.start([0.0, 10.0, 0.0], [0.0, 10.0, 50.0], 2.0);
        assert!(c.yaw.abs() < 1e-5 && c.pitch.abs() < 1e-5);
        c.feed(
            &Input::Key {
                vk: u32::from(b'W'),
                pressed: true,
            },
            true,
        );
        c.update(0.1);
        assert!((c.eye[2] - 6.0).abs() < 1e-3, "{:?}", c.eye);
        let g = c.game();
        assert!((g[1] + 10.0).abs() < 1e-4, "y is negated");
        // A projection scale of 2: tan(fov / 2) = 1/2; zoom 2 halves it.
        assert!((g[6] - 2.0 * 0.5f32.atan()).abs() < 1e-5);
        c.zoom = 2.0;
        assert!((c.game()[6] - 2.0 * 0.25f32.atan()).abs() < 1e-5);
    }
}

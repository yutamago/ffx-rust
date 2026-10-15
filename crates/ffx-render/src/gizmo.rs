//! On-screen handles for the editor's selected light, as Blender draws
//! them: arrows along the world axes to move it, rings to turn it, a
//! circle at its reach to resize it; every light shows as an icon to
//! click. Drawn over the scene (under the panels) through the game's
//! camera; a drag on a handle changes the light and keeps the click from
//! picking.

use crate::enhanced::math;
use crate::enhanced::rig::{LightType, SceneLight};

/// What a drag does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Move,
    Rotate,
    Scale,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Handle {
    Axis(usize),
    Ring(usize),
    Radius,
}

struct Drag {
    handle: Handle,
    from: egui::Pos2,
    light: SceneLight,
    /// Where the handles stood and their length (world units).
    centre: [f32; 3],
    size: f32,
}

#[derive(Default)]
pub struct Gizmo {
    pub mode: Mode,
    drag: Option<Drag>,
}

/// What the gizmo needs of the frame.
pub struct View {
    /// The game's camera (rows, clip = vp · p).
    pub view_proj: math::Mat,
    pub eye: [f32; 3],
    /// What the camera looks at (where a directional light's handles go).
    pub look_at: [f32; 3],
    /// The screen in egui points.
    pub screen: egui::Rect,
}

const AXES: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
const COLOURS: [egui::Color32; 3] = [
    egui::Color32::from_rgb(230, 70, 70),
    egui::Color32::from_rgb(110, 200, 80),
    egui::Color32::from_rgb(70, 130, 240),
];
/// How long the handles look, in points.
const LENGTH: f32 = 80.0;
/// How near the pointer must be to take a handle, in points.
const REACH: f32 = 9.0;

impl View {
    /// Where `p` is on the screen, if in front of the camera.
    #[must_use]
    pub fn project(&self, p: [f32; 3]) -> Option<egui::Pos2> {
        let v = [p[0], p[1], p[2], 1.0];
        let c: Vec<f32> = self
            .view_proj
            .iter()
            .map(|row| row.iter().zip(&v).map(|(a, b)| a * b).sum())
            .collect();
        if c[3] <= 1e-4 {
            return None;
        }
        let (x, y) = (c[0] / c[3], c[1] / c[3]);
        Some(egui::pos2(
            self.screen.min.x + (x * 0.5 + 0.5) * self.screen.width(),
            self.screen.min.y + (0.5 - y * 0.5) * self.screen.height(),
        ))
    }

    /// World units that look `LENGTH` points long at `p`.
    fn size_at(&self, p: [f32; 3]) -> Option<f32> {
        let at = self.project(p)?;
        let longest = AXES
            .iter()
            .filter_map(|a| self.project(add(p, *a)).map(|q| (q - at).length()))
            .fold(0.0f32, f32::max);
        (longest > 1e-3).then(|| LENGTH / longest)
    }
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale(a: [f32; 3], s: f32) -> [f32; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}

/// Distance from `p` to the segment `a`–`b`.
fn to_segment(p: egui::Pos2, a: egui::Pos2, b: egui::Pos2) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_sq().max(1e-6)).clamp(0.0, 1.0);
    (p - (a + ab * t)).length()
}

/// `v` turned by `angle` (radians) about the unit `axis`.
fn rotate(v: [f32; 3], axis: [f32; 3], angle: f32) -> [f32; 3] {
    let (s, c) = angle.sin_cos();
    let d = math::dot(axis, v);
    let x = math::cross(axis, v);
    [
        v[0] * c + x[0] * s + axis[0] * d * (1.0 - c),
        v[1] * c + x[1] * s + axis[1] * d * (1.0 - c),
        v[2] * c + x[2] * s + axis[2] * d * (1.0 - c),
    ]
}

/// A ring about `axis` through the points around `centre`, `r` across.
fn ring(view: &View, centre: [f32; 3], axis: usize, r: f32) -> Vec<egui::Pos2> {
    let (u, v) = match axis {
        0 => (AXES[1], AXES[2]),
        1 => (AXES[2], AXES[0]),
        _ => (AXES[0], AXES[1]),
    };
    (0..=48)
        .filter_map(|k| {
            let t = k as f32 / 48.0 * std::f32::consts::TAU;
            view.project(add(
                centre,
                add(scale(u, r * t.cos()), scale(v, r * t.sin())),
            ))
        })
        .collect()
}

fn near_polyline(p: egui::Pos2, line: &[egui::Pos2]) -> f32 {
    line.windows(2)
        .map(|w| to_segment(p, w[0], w[1]))
        .fold(f32::MAX, f32::min)
}

impl Gizmo {
    /// Whether a handle is being dragged.
    #[must_use]
    pub fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// Draws every light's icon and the selected light's handles, and
    /// works the handles. Returns a light whose icon was clicked, and
    /// whether the pointer is taken (a click must not pick).
    #[allow(clippy::too_many_lines)] // drawing, then the drag
    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        view: &View,
        lights: &[SceneLight],
        selected: Option<usize>,
        edit: &mut Option<SceneLight>,
    ) -> (Option<usize>, bool) {
        let pen = ctx.layer_painter(egui::LayerId::background());
        let (pointer, pressed, down) = ctx.input(|i| {
            (
                i.pointer.interact_pos(),
                i.pointer.primary_pressed(),
                i.pointer.primary_down(),
            )
        });
        let over_panel = ctx.is_pointer_over_egui();
        // Icons.
        let mut clicked = None;
        for (k, l) in lights.iter().enumerate() {
            if !matches!(l.kind, LightType::Point | LightType::Spot) {
                continue;
            }
            let Some(at) = view.project(l.position) else {
                continue;
            };
            let chosen = selected == Some(k);
            let colour = if !l.enabled {
                egui::Color32::GRAY
            } else if chosen {
                egui::Color32::from_rgb(255, 200, 60)
            } else {
                egui::Color32::WHITE
            };
            pen.circle_stroke(at, 6.0, egui::Stroke::new(1.5, colour));
            for r in 0..8 {
                let a = r as f32 / 8.0 * std::f32::consts::TAU;
                let d = egui::vec2(a.cos(), a.sin());
                pen.line_segment(
                    [at + d * 8.0, at + d * 12.0],
                    egui::Stroke::new(1.0, colour),
                );
            }
            if l.kind == LightType::Spot
                && let Some(tip) =
                    view.project(add(l.position, scale(math::normalize(l.direction), 20.0)))
            {
                pen.line_segment([at, tip], egui::Stroke::new(1.0, colour));
            }
            if pressed
                && !over_panel
                && self.drag.is_none()
                && pointer.is_some_and(|p| (p - at).length() < 12.0)
            {
                clicked = Some(k);
            }
        }
        let Some(light) = edit.as_mut() else {
            if !down {
                self.drag = None;
            }
            return (clicked, clicked.is_some());
        };
        let placed = matches!(light.kind, LightType::Point | LightType::Spot);
        let aimed = matches!(light.kind, LightType::Directional | LightType::Spot);
        let centre = match &self.drag {
            Some(d) => d.centre,
            None if placed => light.position,
            None => view.look_at,
        };
        let size = match &self.drag {
            Some(d) => d.size,
            None => match view.size_at(centre) {
                Some(s) => s,
                None => return (clicked, clicked.is_some()),
            },
        };
        let Some(c) = view.project(centre) else {
            return (clicked, clicked.is_some());
        };
        // The handles of the mode, and which one the pointer is on.
        let mut nearest: Option<(Handle, f32)> = None;
        let mut consider = |h: Handle, d: f32| {
            if d < REACH && nearest.is_none_or(|n| d < n.1) {
                nearest = Some((h, d));
            }
        };
        let active = self.drag.as_ref().map(|d| d.handle);
        let width = |h: Handle| if active == Some(h) { 3.5 } else { 2.0 };
        match self.mode {
            Mode::Move if placed => {
                for (k, axis) in AXES.iter().enumerate() {
                    let Some(tip) = view.project(add(centre, scale(*axis, size))) else {
                        continue;
                    };
                    let stroke = egui::Stroke::new(width(Handle::Axis(k)), COLOURS[k]);
                    pen.line_segment([c, tip], stroke);
                    pen.circle_filled(tip, 4.0, COLOURS[k]);
                    if let Some(p) = pointer {
                        consider(Handle::Axis(k), to_segment(p, c, tip));
                    }
                }
            }
            Mode::Rotate if aimed => {
                for (k, colour) in COLOURS.iter().enumerate() {
                    let line = ring(view, centre, k, size * 0.8);
                    pen.add(egui::Shape::line(
                        line.clone(),
                        egui::Stroke::new(width(Handle::Ring(k)), *colour),
                    ));
                    if let Some(p) = pointer {
                        consider(Handle::Ring(k), near_polyline(p, &line));
                    }
                }
            }
            Mode::Scale if placed => {
                let r = light.attenuation_radius.max(0.1);
                // The reach as a circle facing the camera.
                let across = math::normalize(math::cross(
                    math::normalize([
                        centre[0] - view.eye[0],
                        centre[1] - view.eye[1],
                        centre[2] - view.eye[2],
                    ]),
                    [0.0, 1.0, 0.0],
                ));
                if let Some(edge) = view.project(add(centre, scale(across, r))) {
                    let radius = (edge - c).length();
                    pen.circle_stroke(
                        c,
                        radius,
                        egui::Stroke::new(
                            width(Handle::Radius),
                            egui::Color32::from_rgb(255, 200, 60),
                        ),
                    );
                    if let Some(p) = pointer {
                        consider(Handle::Radius, ((p - c).length() - radius).abs());
                    }
                }
            }
            _ => {}
        }
        // Which way the light shines.
        if aimed
            && let Some(tip) = view.project(add(
                centre,
                scale(math::normalize(light.direction), size * 1.2),
            ))
        {
            pen.arrow(
                c,
                tip - c,
                egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 230, 120)),
            );
        }
        // The drag.
        let mut taken = clicked.is_some();
        if let Some(p) = pointer {
            if pressed
                && !over_panel
                && self.drag.is_none()
                && let Some((handle, _)) = nearest
            {
                self.drag = Some(Drag {
                    handle,
                    from: p,
                    light: light.clone(),
                    centre,
                    size,
                });
            }
            if let Some(d) = &self.drag {
                taken = true;
                let moved = p - d.from;
                match d.handle {
                    Handle::Axis(k) => {
                        if let Some(tip) = view.project(add(d.centre, scale(AXES[k], d.size))) {
                            let along = tip - c;
                            let units = moved.dot(along) / along.length_sq().max(1e-6) * d.size;
                            light.position = add(d.light.position, scale(AXES[k], units));
                        }
                    }
                    Handle::Ring(k) => {
                        let a = d.from - c;
                        let b = p - c;
                        let angle = a.angle() - b.angle();
                        let towards = math::dot(
                            AXES[k],
                            math::normalize([
                                d.centre[0] - view.eye[0],
                                d.centre[1] - view.eye[1],
                                d.centre[2] - view.eye[2],
                            ]),
                        );
                        let angle = if towards > 0.0 { -angle } else { angle };
                        light.direction =
                            math::normalize(rotate(d.light.direction, AXES[k], angle));
                    }
                    Handle::Radius => {
                        let from = (d.from - c).length().max(1.0);
                        let now = (p - c).length();
                        light.attenuation_radius =
                            (d.light.attenuation_radius * now / from).clamp(1.0, 5000.0);
                    }
                }
            }
        }
        if !down {
            self.drag = None;
        }
        (clicked, taken || nearest.is_some() && pressed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turning_and_projecting() {
        let v = rotate(
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            std::f32::consts::FRAC_PI_2,
        );
        assert!((v[2] + 1.0).abs() < 1e-5, "{v:?}");
        let view = View {
            view_proj: math::identity(),
            eye: [0.0, 0.0, -5.0],
            look_at: [0.0; 3],
            screen: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(200.0, 100.0)),
        };
        let p = view.project([0.0, 0.0, 0.5]).unwrap();
        assert!((p.x - 100.0).abs() < 1e-4 && (p.y - 50.0).abs() < 1e-4);
        assert!(
            (to_segment(
                egui::pos2(0.0, 5.0),
                egui::pos2(0.0, 0.0),
                egui::pos2(10.0, 0.0)
            ) - 5.0)
                .abs()
                < 1e-5
        );
    }
}

//! The renderer's field minimap, in place of the game's (the hook sends
//! its state: `Msg::Minimap`): round, always centred on the player, turned
//! so that where the camera looks is up; the walkable area of the map's
//! mesh in light grey on a dark disc, the player as an arrow of his
//! facing, the game's markers as icons; markers beyond its edge sit on
//! the rim, fading in as they come nearer.

use ffx_gfx::{MapMarker, MapMesh, Minimap};

/// How the minimap looks and sits (the profile's settings).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    /// Diameter, a share of the screen's height.
    pub size: f32,
    /// World units from the centre to the edge (a character is about 20
    /// tall).
    pub range: f32,
    pub opacity: f32,
}

/// The mesh as triangles on the ground (render space x, z), and its
/// outline (edges of one triangle only).
struct Ground {
    id: u32,
    triangles: Vec<[[f32; 2]; 3]>,
    outline: Vec<[[f32; 2]; 2]>,
}

#[derive(Default)]
pub struct Map {
    /// The game's last state (positions in its space).
    pub state: Option<Minimap>,
    /// A mesh waiting for the player's position (to tell its matrix's
    /// order), and the mesh in use.
    raw: Option<MapMesh>,
    ground: Option<Ground>,
    /// Frames drawn (for the blinking markers).
    tick: u32,
}

/// Game space to render space (x and y negated), on the ground (x, z).
fn on_ground(p: [f32; 3]) -> [f32; 2] {
    [-p[0], p[2]]
}

/// A vertex through the mesh's matrix, rows or columns first.
fn transform(m: &[f32; 16], v: [i16; 3], rows: bool) -> [f32; 3] {
    let v = [f32::from(v[0]), f32::from(v[1]), f32::from(v[2]), 1.0];
    let at = |r: usize, c: usize| if rows { m[r * 4 + c] } else { m[c * 4 + r] };
    [0, 1, 2].map(|r| (0..4).map(|c| at(r, c) * v[c]).sum())
}

impl Ground {
    /// The mesh in the world: of the two orders of its matrix, the one
    /// whose map holds the player (else the nearer).
    fn new(mesh: &MapMesh, player: [f32; 3]) -> Self {
        let place = |rows: bool| -> Vec<[f32; 2]> {
            mesh.vertices
                .iter()
                .map(|v| on_ground(transform(&mesh.matrix, *v, rows)))
                .collect()
        };
        let p = on_ground(player);
        let score = |pts: &[[f32; 2]]| -> f32 {
            let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
            for q in pts {
                for k in 0..2 {
                    lo[k] = lo[k].min(q[k]);
                    hi[k] = hi[k].max(q[k]);
                }
            }
            (0..2)
                .map(|k| (lo[k] - p[k]).max(0.0) + (p[k] - hi[k]).max(0.0))
                .sum()
        };
        let (a, b) = (place(true), place(false));
        let pts = if score(&a) <= score(&b) { a } else { b };
        let triangles: Vec<[[f32; 2]; 3]> = mesh
            .triangles
            .iter()
            .filter_map(|t| {
                let g = |i: u16| pts.get(usize::from(i)).copied();
                Some([g(t[0])?, g(t[1])?, g(t[2])?])
            })
            .collect();
        // Edges that belong to one triangle only are the outline.
        let mut count = std::collections::HashMap::new();
        for t in &mesh.triangles {
            for k in 0..3 {
                let (a, b) = (t[k], t[(k + 1) % 3]);
                *count.entry((a.min(b), a.max(b))).or_insert(0u32) += 1;
            }
        }
        let outline = count
            .into_iter()
            .filter(|(_, n)| *n == 1)
            .filter_map(|((a, b), _)| Some([*pts.get(usize::from(a))?, *pts.get(usize::from(b))?]))
            .collect();
        Self {
            id: mesh.id,
            triangles,
            outline,
        }
    }
}

/// The convex polygon `poly` cut to the inside of the convex polygon
/// `clip` (both counter-clockwise or both clockwise).
fn cut(poly: &[[f32; 2]], clip: &[[f32; 2]]) -> Vec<[f32; 2]> {
    let mut out = poly.to_vec();
    for k in 0..clip.len() {
        let (a, b) = (clip[k], clip[(k + 1) % clip.len()]);
        let side = |p: [f32; 2]| (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
        let input = std::mem::take(&mut out);
        for i in 0..input.len() {
            let (p, q) = (input[i], input[(i + 1) % input.len()]);
            let (sp, sq) = (side(p), side(q));
            if sp >= 0.0 {
                out.push(p);
            }
            if (sp >= 0.0) != (sq >= 0.0) {
                let t = sp / (sp - sq);
                out.push([p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t]);
            }
        }
        if out.is_empty() {
            break;
        }
    }
    out
}

/// The part of the segment a–b inside the circle at the origin of radius r.
fn inside(a: [f32; 2], b: [f32; 2], r: f32) -> Option<([f32; 2], [f32; 2])> {
    let d = [b[0] - a[0], b[1] - a[1]];
    let qa = d[0] * d[0] + d[1] * d[1];
    let qb = 2.0 * (a[0] * d[0] + a[1] * d[1]);
    let qc = a[0] * a[0] + a[1] * a[1] - r * r;
    if qa < 1e-9 {
        return (qc <= 0.0).then_some((a, b));
    }
    let disc = qb * qb - 4.0 * qa * qc;
    if disc < 0.0 {
        return None;
    }
    let s = disc.sqrt();
    let t0 = ((-qb - s) / (2.0 * qa)).max(0.0);
    let t1 = ((-qb + s) / (2.0 * qa)).min(1.0);
    (t0 < t1).then(|| {
        let at = |t: f32| [a[0] + d[0] * t, a[1] + d[1] * t];
        (at(t0), at(t1))
    })
}

/// Where a marker shows: inside the map at its place (fully), beyond the
/// edge on the rim, fading out over 1.5 times the map's radius. `p` is
/// in pixels from the centre; returns the place and the opacity.
#[must_use]
pub fn place(p: [f32; 2], rim: f32, radius: f32) -> ([f32; 2], f32) {
    let d = (p[0] * p[0] + p[1] * p[1]).sqrt();
    if d <= rim {
        return (p, 1.0);
    }
    let fade = (1.0 - (d - rim) / (radius * 1.5)).clamp(0.0, 1.0);
    let s = rim / d;
    ([p[0] * s, p[1] * s], fade * fade * (3.0 - 2.0 * fade))
}

impl Map {
    /// The game's minimap state this frame.
    pub fn update(&mut self, mut m: Minimap) {
        if let Some(mesh) = m.mesh.take() {
            self.raw = Some(mesh);
        }
        if let Some(raw) = &self.raw
            && self.ground.as_ref().is_none_or(|g| g.id != raw.id)
            && m.markers.iter().any(|k| k.flags & 0x800 != 0)
        {
            self.ground = Some(Ground::new(raw, m.player));
        }
        self.state = Some(m);
    }

    /// Whether it shows now.
    #[must_use]
    pub fn shown(&self) -> bool {
        self.state.as_ref().is_some_and(|s| s.shown > 0.0) && self.ground.is_some()
    }

    /// Draws it in `screen` (points; top right), the camera looking along
    /// `axes.0` with screen right `axes.1` (render space).
    #[allow(clippy::too_many_lines)] // the disc, the ground, the markers, the player
    pub fn draw(
        &mut self,
        painter: &egui::Painter,
        screen: egui::Rect,
        axes: ([f32; 3], [f32; 3]),
        style: Style,
    ) {
        self.tick = self.tick.wrapping_add(1);
        let (Some(state), Some(ground)) = (&self.state, &self.ground) else {
            return;
        };
        let alpha = state.shown * style.opacity.clamp(0.0, 1.0);
        if alpha <= 0.0 {
            return;
        }
        let radius = (screen.height() * style.size * 0.5).max(20.0);
        let margin = screen.height() * 0.035;
        let centre = egui::pos2(
            screen.right() - margin - radius,
            screen.top() + margin + radius,
        );
        // Camera forward on the ground is up, its right is right.
        let flat = |v: [f32; 3], or: [f32; 2]| {
            let l = (v[0] * v[0] + v[2] * v[2]).sqrt();
            if l < 1e-4 { or } else { [v[0] / l, v[2] / l] }
        };
        let f = flat(axes.0, [0.0, 1.0]);
        let right = flat(axes.1, [f[1], -f[0]]);
        let me = on_ground(state.player);
        let scale = radius / style.range.max(1.0);
        let to_px = |p: [f32; 2]| -> [f32; 2] {
            let d = [p[0] - me[0], p[1] - me[1]];
            [
                (d[0] * right[0] + d[1] * right[1]) * scale,
                -(d[0] * f[0] + d[1] * f[1]) * scale,
            ]
        };
        let at = |p: [f32; 2]| centre + egui::vec2(p[0], p[1]);
        let a = |v: f32| (v * alpha * 255.0).round().clamp(0.0, 255.0) as u8;
        // The disc.
        painter.circle_filled(
            centre,
            radius,
            egui::Color32::from_rgba_unmultiplied(18, 20, 24, a(0.78)),
        );
        // The ground, cut to the disc.
        let n = 64;
        let edge = radius - 1.5;
        let circle: Vec<[f32; 2]> = (0..n)
            .map(|k| {
                let t = k as f32 / n as f32 * std::f32::consts::TAU;
                [edge * t.cos(), edge * t.sin()]
            })
            .collect();
        let fill = egui::Color32::from_rgba_unmultiplied(196, 198, 204, a(0.7));
        let mut mesh = egui::Mesh::default();
        for t in &ground.triangles {
            let mut tri = t.map(&to_px);
            // The circle runs counter-clockwise in screen terms (y down):
            // the triangle must run the same way.
            let cross = (tri[1][0] - tri[0][0]) * (tri[2][1] - tri[0][1])
                - (tri[1][1] - tri[0][1]) * (tri[2][0] - tri[0][0]);
            if cross < 0.0 {
                tri.swap(1, 2);
            }
            // Wholly outside: skip.
            if tri.iter().all(|p| p[0].abs() > edge && p[1].abs() > edge) {
                continue;
            }
            let poly = cut(&tri, &circle);
            if poly.len() < 3 {
                continue;
            }
            let base = mesh.vertices.len() as u32;
            for p in &poly {
                mesh.colored_vertex(at(*p), fill);
            }
            for k in 1..poly.len() as u32 - 1 {
                mesh.add_triangle(base, base + k, base + k + 1);
            }
        }
        painter.add(egui::Shape::mesh(mesh));
        let line = egui::Stroke::new(
            1.2,
            egui::Color32::from_rgba_unmultiplied(235, 236, 240, a(0.8)),
        );
        for e in &ground.outline {
            if let Some((p, q)) = inside(to_px(e[0]), to_px(e[1]), edge) {
                painter.line_segment([at(p), at(q)], line);
            }
        }
        // The rim.
        painter.circle_stroke(
            centre,
            radius,
            egui::Stroke::new(
                2.0,
                egui::Color32::from_rgba_unmultiplied(225, 228, 235, a(0.85)),
            ),
        );
        // The markers.
        let rim = radius - 9.0;
        let blink_on = (self.tick / 15).is_multiple_of(2);
        for k in &state.markers {
            if k.flags & 0x800 != 0 {
                continue;
            }
            if k.flags & 0x1000 != 0 && !blink_on {
                continue;
            }
            marker(painter, k, &to_px, &at, (rim, radius), alpha);
        }
        // The player: an arrow of his facing (game yaw: from +x toward +z;
        // render space has x negated).
        let facing = [-state.yaw.cos(), state.yaw.sin()];
        let dir = [
            facing[0] * right[0] + facing[1] * right[1],
            -(facing[0] * f[0] + facing[1] * f[1]),
        ];
        let dir = egui::vec2(dir[0], dir[1]).normalized();
        let side = egui::vec2(-dir.y, dir.x);
        let s = radius * 0.09;
        let tip = centre + dir * s * 1.3;
        let back = centre - dir * s * 0.8;
        let pts = [
            tip,
            back + side * s * 0.9,
            centre - dir * s * 0.35,
            back - side * s * 0.9,
        ];
        painter.add(egui::Shape::convex_polygon(
            vec![pts[0], pts[1], pts[2]],
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, a(1.0)),
            egui::Stroke::new(
                1.0,
                egui::Color32::from_rgba_unmultiplied(30, 30, 34, a(0.9)),
            ),
        ));
        painter.add(egui::Shape::convex_polygon(
            vec![pts[0], pts[2], pts[3]],
            egui::Color32::from_rgba_unmultiplied(235, 235, 240, a(1.0)),
            egui::Stroke::new(
                1.0,
                egui::Color32::from_rgba_unmultiplied(30, 30, 34, a(0.9)),
            ),
        ));
    }
}

/// One marker: the game's shape and colour, restyled; exits as a bar (and
/// an arrow on the rim beyond the edge); the destination always on the
/// rim when away.
fn marker(
    painter: &egui::Painter,
    k: &MapMarker,
    to_px: &dyn Fn([f32; 2]) -> [f32; 2],
    at: &dyn Fn([f32; 2]) -> egui::Pos2,
    (rim, radius): (f32, f32),
    alpha: f32,
) {
    let c = k.color;
    let colour = |o: f32| {
        egui::Color32::from_rgba_unmultiplied(
            (c[0] * 255.0) as u8,
            (c[1] * 255.0) as u8,
            (c[2] * 255.0) as u8,
            (o * alpha * 255.0).clamp(0.0, 255.0) as u8,
        )
    };
    let dark =
        |o: f32| egui::Color32::from_rgba_unmultiplied(20, 20, 24, (o * alpha * 230.0) as u8);
    let p = to_px(on_ground(k.pos));
    // Exits: the line where it is on the map.
    if let Some(l) = k.line {
        let (a, b) = (to_px([-l[0], l[1]]), to_px([-l[2], l[3]]));
        if let Some((p0, p1)) = inside(a, b, rim) {
            painter.line_segment([at(p0), at(p1)], egui::Stroke::new(4.0, colour(0.95)));
            return;
        }
    }
    let (q, o) = if k.flags & 0x2000 != 0 {
        // The destination: on the rim whenever it is away.
        let d = (p[0] * p[0] + p[1] * p[1]).sqrt().max(1e-3);
        if d <= rim {
            (p, 1.0)
        } else {
            ([p[0] * rim / d, p[1] * rim / d], 1.0)
        }
    } else {
        place(p, rim, radius)
    };
    if o <= 0.01 {
        return;
    }
    let pos = at(q);
    let s = radius * 0.055;
    let on_rim = (q[0] * q[0] + q[1] * q[1]).sqrt() >= rim - 0.5;
    if k.line.is_some() || on_rim && k.flags & 0x2000 != 0 {
        // An arrow pointing out, toward it.
        let out = egui::vec2(q[0], q[1]).normalized();
        let side = egui::vec2(-out.y, out.x);
        painter.add(egui::Shape::convex_polygon(
            vec![
                pos + out * s * 1.2,
                pos - out * s * 0.6 + side * s,
                pos - out * s * 0.6 - side * s,
            ],
            colour(o),
            egui::Stroke::new(1.0, dark(o)),
        ));
        return;
    }
    if k.flags & 0x20000 != 0 {
        painter.add(egui::Shape::convex_polygon(
            vec![
                pos + egui::vec2(0.0, -s),
                pos + egui::vec2(s, 0.0),
                pos + egui::vec2(0.0, s),
                pos + egui::vec2(-s, 0.0),
            ],
            colour(o),
            egui::Stroke::new(1.0, dark(o)),
        ));
    } else if k.flags & 0x100 != 0 {
        painter.rect(
            egui::Rect::from_center_size(pos, egui::vec2(s * 1.5, s * 1.5)),
            2.0,
            colour(o),
            egui::Stroke::new(1.0, dark(o)),
            egui::StrokeKind::Outside,
        );
    } else if k.flags & 0x200 != 0 {
        painter.add(egui::Shape::convex_polygon(
            vec![
                pos + egui::vec2(0.0, -s),
                pos + egui::vec2(s * 0.9, s * 0.7),
                pos + egui::vec2(-s * 0.9, s * 0.7),
            ],
            colour(o),
            egui::Stroke::new(1.0, dark(o)),
        ));
    } else {
        painter.circle(pos, s * 0.75, colour(o), egui::Stroke::new(1.0, dark(o)));
    }
    if k.flags & 0x400 != 0 {
        painter.circle_stroke(pos, s * 1.4, egui::Stroke::new(1.5, colour(o * 0.8)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markers_fade_onto_the_rim() {
        let (p, o) = place([10.0, 0.0], 50.0, 60.0);
        assert_eq!((p, o), ([10.0, 0.0], 1.0));
        let (p, o) = place([0.0, 95.0], 50.0, 60.0);
        assert!((p[1] - 50.0).abs() < 1e-4 && o > 0.0 && o < 1.0);
        let (_, o) = place([0.0, 500.0], 50.0, 60.0);
        assert!(o.abs() < 1e-6);
        // A square cut by a smaller square: the smaller one's area.
        let a = [[-2.0, -2.0], [2.0, -2.0], [2.0, 2.0], [-2.0, 2.0]];
        let b = [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]];
        assert_eq!(cut(&a, &b).len(), 4);
        assert!(
            inside([-5.0, 0.0], [5.0, 0.0], 2.0)
                .is_some_and(|(p, q)| (p[0] + 2.0).abs() < 1e-4 && (q[0] - 2.0).abs() < 1e-4)
        );
        assert!(inside([-5.0, 3.0], [5.0, 3.0], 2.0).is_none());
    }
}

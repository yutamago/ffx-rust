//! Culling and the depth key of `PCheckVisibleBoundsJob_execute`
//! (`0x009f3160`), per `PMeshInstanceBounds`.
//!
//! The x87 runs at 53 bits: products of two floats are exact, sums are
//! rounded to 53 bits, and a value is rounded to a float where the code
//! stores it (`fstp dword`).

// Matrix and box arithmetic names its terms as the formulas do.
#![allow(clippy::many_single_char_names)]

use crate::frame::Mat;

/// cb layout → `PMatrix4` memory (column-major, translation in 12..14).
#[must_use]
pub fn to_memory(m: &Mat) -> Mat {
    std::array::from_fn(|i| m[(i % 4) * 4 + i / 4])
}

fn f(x: f64) -> f32 {
    #[allow(clippy::cast_possible_truncation)] // the fstp dword
    {
        x as f32
    }
}

/// `PMatrix4_Multiply_004ca360(out, a, b)` on memory matrices: `a . b`,
/// each element a 53-bit sum of four exact products rounded once.
#[must_use]
pub fn multiply(a: &Mat, b: &Mat) -> Mat {
    std::array::from_fn(|i| {
        let (c, r) = (i / 4, i % 4);
        let mut s = 0f64;
        for k in 0..4 {
            s += f64::from(a[k * 4 + r]) * f64::from(b[c * 4 + k]);
        }
        f(s)
    })
}

/// `PMatrix4_TransformPoint` on a memory matrix: `m . (x, y, z, 1)`.
#[must_use]
pub fn transform_point(m: &Mat, p: [f32; 3]) -> [f32; 4] {
    std::array::from_fn(|r| {
        let mut s = 0f64;
        for (k, v) in p.iter().enumerate() {
            s += f64::from(m[k * 4 + r]) * f64::from(*v);
        }
        f(s + f64::from(m[12 + r]))
    })
}

/// `ClipOutcode` (`0x00652470`): bits 0-2 for x, y, z above w, 3-5 below
/// -w, each difference or sum stored as a float first; a NaN sets none.
#[must_use]
pub fn outcode(p: [f32; 4]) -> u32 {
    let w = f64::from(p[3]);
    let mut code = 0;
    for (k, x) in p.iter().take(3).enumerate() {
        let v = f64::from(*x);
        if f(v - w) > 0.0 {
            code |= 1 << k;
        }
        if f(w + v) < 0.0 {
            code |= 1 << (k + 3);
        }
    }
    code
}

/// `BoundingBox_IsVisible_00663a30(box, m)` (`docs/formats/render.md`,
/// round 19): the corner through `m` (`t`), the extent's axes as
/// `ext.x * row 0`, `ext.y * row 1`, `ext.z * row 2` of the memory
/// matrix (each product stored), the eight corners as stored sums, their
/// outcodes combined with `outcodes ANDed`: visible unless every corner is outside one plane.
#[must_use]
pub fn box_visible(min: [f32; 3], size: [f32; 3], m: &Mat) -> bool {
    let axis = |r: usize| -> [f32; 4] {
        std::array::from_fn(|k| f(f64::from(size[r]) * f64::from(m[r * 4 + k])))
    };
    let (a, b, c) = (axis(0), axis(1), axis(2));
    let t = transform_point(m, min);
    let add = |p: [f32; 4], q: [f32; 4]| -> [f32; 4] {
        std::array::from_fn(|k| f(f64::from(p[k]) + f64::from(q[k])))
    };
    let ta = add(t, a);
    let tb = add(t, b);
    let tab = add(ta, b);
    let corners = [
        add(tab, c),
        add(tb, c),
        add(ta, c),
        add(t, c),
        tab,
        tb,
        ta,
        t,
    ];
    corners.iter().fold(0x3f, |acc, p| acc & outcode(*p)) == 0
}

/// The culling matrix for an instance: the camera's `+0x70` times its
/// world (`PMatrix4_FromMatrix3x4` then `PMatrix4_Multiply`), or the
/// camera's alone without a world. Memory layout.
#[must_use]
pub fn cull_matrix(cull: &Mat, world: Option<&Mat>) -> Mat {
    let vp = to_memory(cull);
    match world {
        Some(w) => multiply(&vp, &to_memory(w)),
        None => vp,
    }
}

/// The bounds' centre through the world: `size * 0.5 + min` (each step
/// stored), then `PMatrix4_TransformPoint` (`0x004c9eb0`) with the world.
#[must_use]
pub fn centre(min: [f32; 3], size: [f32; 3], world: Option<&Mat>) -> [f32; 3] {
    let c: [f32; 3] =
        std::array::from_fn(|k| f(f64::from(f(f64::from(size[k]) * 0.5)) + f64::from(min[k])));
    match world {
        Some(w) => {
            let p = transform_point(&to_memory(w), c);
            [p[0], p[1], p[2]]
        }
        None => c,
    }
}

/// The depth key: the plane is the view's z row (`PVisibilityCheck`
/// `0x009f2580`: elements 2, 6, 10, 14 of `FromMatrix3x4(PCamera +0)`,
/// row 2 of `scene.View`), negated for the opaque pass (`dir >= 0`).
/// `s = px*cx`, `s += py*cy`, `s += pz*cz` (each sum stored, the product
/// exact), then `s + w` stored; with a depth bias `(key * scale + bias)`
/// stored once.
#[must_use]
pub fn depth_key(view: &Mat, opaque: bool, p: [f32; 3], bias: Option<(f32, f32)>) -> f32 {
    let mut plane = [view[8], view[9], view[10], view[11]];
    if opaque {
        plane = plane.map(|x| -x);
    }
    let mut s = f(f64::from(plane[0]) * f64::from(p[0]));
    s = f(f64::from(s) + f64::from(plane[1]) * f64::from(p[1]));
    s = f(f64::from(s) + f64::from(plane[2]) * f64::from(p[2]));
    let key = f(f64::from(s) + f64::from(plane[3]));
    match bias {
        Some((scale, b)) => f(f64::from(key) * f64::from(scale) + f64::from(b)),
        None => key,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: Mat = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];

    #[test]
    fn boxes_inside_and_outside_the_clip_volume() {
        assert!(box_visible([-0.5; 3], [1.0; 3], &ID));
        assert!(!box_visible([2.0, 0.0, 0.0], [1.0; 3], &ID));
        // straddling a plane: visible
        assert!(box_visible([0.5, 0.0, 0.0], [1.0; 3], &ID));
    }

    #[test]
    fn keys_are_negated_for_opaque() {
        let mut view = ID;
        view[11] = -10.0;
        let k = depth_key(&view, false, [0.0, 0.0, 3.0], None);
        assert_eq!(k, -7.0);
        assert_eq!(depth_key(&view, true, [0.0, 0.0, 3.0], None), 7.0);
        assert_eq!(
            depth_key(&view, false, [0.0, 0.0, 3.0], Some((2.0, 1.0))),
            -13.0
        );
    }

    #[test]
    fn memory_layout_is_the_transpose() {
        let mut m = ID;
        m[3] = 5.0;
        assert_eq!(to_memory(&m)[12], 5.0);
        assert_eq!(transform_point(&to_memory(&m), [1.0, 0.0, 0.0])[0], 6.0);
    }
}

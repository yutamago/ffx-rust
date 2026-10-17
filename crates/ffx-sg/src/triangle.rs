//! Which walk-mesh triangles `InsideTriangle` may test: a fixed table of
//! exceptions per map (`passThisTriangle`).

/// The original's decision table: `passThisTriangle(a, b, c, group, tag)`
/// (`0x00829870`; all `u16`, `InsideTriangle` passes the character's first
/// halfword, two of its own values, a group and the triangle's tag).
///
/// `u` is `tag / 100`. Two special cases come first: `(3, 2, 2)` and
/// `(0x67, 1, 1)` pass only tags with `u` 1. Otherwise group 0 and a tag
/// equal to `group 100 + c` pass; then per `a`:
/// - 2: `b` 0 passes, `b` 2 with `c` 0xc or 0xd checks `u` against the
///   group (groups 2/4 need `u` 2 or 4 and 1/3 need 1 or 3 for 0xc;
///   1/2 need 1 or 2 and 3/4 need 3 or 4 for 0xd), anything else fails;
/// - 4 (`b` 0, `c` 10): groups 1..9 need `u` 1..9, groups 10..18 `u`
///   10..18;
/// - 7 (`b` 0): `c` 6 as 0x29 below; `c` 2 groups 1, 2, 3 need `u` in
///   {1, 4}, {2, 5}, {3, 6};
/// - 0x29 (`b` 0, `c` 6): group 1 needs `u` in {1, 6, 7, 9, 14, 20} (for
///   7: {1, 6, 9, 14, 17, 20}), 2 in {2, 11, 13, 15, 16, 21}, 12 in {4,
///   12, 19}, 18 in {3, 7, 10, 18}, 5 in {5, 8};
/// - 0x68 (`b` 2, `c` 0xc): group 2 needs `u` outside 3..=10, 9 inside
///   3..=10.
///
/// Groups the table does not name pass; other `a` fail. Returns 1 or 0.
#[must_use]
#[allow(clippy::many_single_char_names, clippy::too_many_lines)]
pub fn pass_this_triangle(a: u16, b: u16, c: u16, group: u16, tag: u16) -> u32 {
    let u = u32::from(tag / 100);
    let one_of = |set: &[u32]| u32::from(set.contains(&u));
    if (a == 3 && b == 2 && c == 2) || (a == 0x67 && b == 1 && c == 1) {
        return u32::from(u == 1);
    }
    if group == 0 || u32::from(tag) == u32::from(group) * 100 + u32::from(c) {
        return 1;
    }
    let d = group;
    // a 7 or 0x29 with c 6, a group other than 1
    let wide = |d: u16| match d {
        2 => one_of(&[2, 0xd, 0xf, 0x15, 0x10, 0xb]),
        0xc => one_of(&[4, 0xc, 0x13]),
        0x12 => one_of(&[0x12, 3, 7, 10]),
        5 => one_of(&[5, 8]),
        _ => 1,
    };
    match a {
        2 => {
            if b == 0 {
                return 1;
            }
            if b != 2 {
                return 0;
            }
            match c {
                0xc => {
                    if (d == 2 || d == 4) && u != 2 && u != 4 {
                        return 0;
                    }
                    if d == 1 || d == 3 { one_of(&[1, 3]) } else { 1 }
                }
                0xd => {
                    if (d == 1 || d == 2) && u != 1 && u != 2 {
                        return 0;
                    }
                    if d == 3 || d == 4 { one_of(&[3, 4]) } else { 1 }
                }
                _ => 0,
            }
        }
        4 => {
            if b != 0 || c != 10 {
                return 0;
            }
            if d.wrapping_sub(1) <= 8 && !(1..=9).contains(&u) {
                return 0;
            }
            if d.wrapping_sub(10) > 8 {
                return 1;
            }
            u32::from((10..=0x12).contains(&u))
        }
        7 => {
            if b != 0 {
                return 0;
            }
            match c {
                6 if d == 1 => one_of(&[1, 6, 0x11, 9, 0xe, 0x14]),
                6 => wide(d),
                2 => match d {
                    1 => one_of(&[1, 4]),
                    2 => one_of(&[2, 5]),
                    3 => one_of(&[3, 6]),
                    _ => 1,
                },
                _ => 0,
            }
        }
        0x29 => {
            if b != 0 || c != 6 {
                return 0;
            }
            if d == 1 {
                one_of(&[1, 6, 7, 9, 0xe, 0x14])
            } else {
                wide(d)
            }
        }
        0x68 => {
            if b != 2 || c != 0xc {
                return 0;
            }
            match d {
                #[allow(clippy::cast_possible_truncation)] // the original's 16-bit compare
                2 => u32::from((u as u16).wrapping_sub(3) > 7),
                9 => u32::from((3..=10).contains(&u)),
                _ => 1,
            }
        }
        _ => 0,
    }
}

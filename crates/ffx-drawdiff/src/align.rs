//! Myers' O((N+M)D) difference of two sequences: the shortest edit script
//! of deletions from A and insertions from B, with the common items as
//! `Equal` pairs.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Equal(usize, usize),
    /// Only in A.
    Delete(usize),
    /// Only in B.
    Insert(usize),
}

/// Edit distances past this many steps are not searched: the rest is one
/// deletion and one insertion of everything left (moved pairing still
/// finds items there). Keeps the trace under ~64 M entries.
const MAX_TRACE: usize = 64 << 20;

/// The edit script from `a` to `b`, in order.
#[must_use]
#[allow(clippy::many_single_char_names)] // Myers' a, b, n, m, d, k, x, y
pub fn myers<T: PartialEq>(a: &[T], b: &[T]) -> Vec<Op> {
    let prefix = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    let suffix = a[prefix..]
        .iter()
        .rev()
        .zip(b[prefix..].iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    let (ma, mb) = (&a[prefix..a.len() - suffix], &b[prefix..b.len() - suffix]);
    let mut ops: Vec<Op> = (0..prefix).map(|i| Op::Equal(i, i)).collect();
    for op in middle(ma, mb) {
        ops.push(match op {
            Op::Equal(i, j) => Op::Equal(i + prefix, j + prefix),
            Op::Delete(i) => Op::Delete(i + prefix),
            Op::Insert(j) => Op::Insert(j + prefix),
        });
    }
    ops.extend((0..suffix).map(|i| Op::Equal(a.len() - suffix + i, b.len() - suffix + i)));
    ops
}

#[allow(
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)] // diagonals are signed; Myers' names
fn middle<T: PartialEq>(a: &[T], b: &[T]) -> Vec<Op> {
    let (n, m) = (a.len() as isize, b.len() as isize);
    if n == 0 || m == 0 {
        return (0..a.len())
            .map(Op::Delete)
            .chain((0..b.len()).map(Op::Insert))
            .collect();
    }
    let max = n + m;
    let off = max + 1;
    let width = (2 * max + 3) as usize;
    let mut v = vec![0isize; width];
    let mut trace: Vec<Vec<isize>> = Vec::new();
    let mut found = None;
    'search: for d in 0..=max {
        if trace.len() * width > MAX_TRACE {
            break;
        }
        trace.push(v.clone());
        let mut k = -d;
        while k <= d {
            let i = (k + off) as usize;
            let mut x = if k == -d || (k != d && v[i - 1] < v[i + 1]) {
                v[i + 1]
            } else {
                v[i - 1] + 1
            };
            let mut y = x - k;
            while x < n && y < m && a[x as usize] == b[y as usize] {
                x += 1;
                y += 1;
            }
            v[i] = x;
            if x >= n && y >= m {
                found = Some(d);
                break 'search;
            }
            k += 2;
        }
    }
    let Some(_) = found else {
        return (0..a.len())
            .map(Op::Delete)
            .chain((0..b.len()).map(Op::Insert))
            .collect();
    };
    let (mut x, mut y) = (n, m);
    let mut out = Vec::new();
    for (d, v) in trace.iter().enumerate().rev() {
        let d = d as isize;
        let k = x - y;
        let prev_k = if k == -d || (k != d && v[(k - 1 + off) as usize] < v[(k + 1 + off) as usize])
        {
            k + 1
        } else {
            k - 1
        };
        let prev_x = v[(prev_k + off) as usize];
        let prev_y = prev_x - prev_k;
        while x > prev_x && y > prev_y {
            out.push(Op::Equal((x - 1) as usize, (y - 1) as usize));
            x -= 1;
            y -= 1;
        }
        if d > 0 {
            if x == prev_x {
                out.push(Op::Insert(prev_y as usize));
            } else {
                out.push(Op::Delete(prev_x as usize));
            }
        }
        x = prev_x;
        y = prev_y;
    }
    out.reverse();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply(a: &[char], b: &[char], ops: &[Op]) {
        // Every item of each side appears once, in order.
        let mut ia = 0;
        let mut ib = 0;
        for op in ops {
            match *op {
                Op::Equal(i, j) => {
                    assert_eq!((i, j), (ia, ib));
                    assert_eq!(a[i], b[j]);
                    ia += 1;
                    ib += 1;
                }
                Op::Delete(i) => {
                    assert_eq!(i, ia);
                    ia += 1;
                }
                Op::Insert(j) => {
                    assert_eq!(j, ib);
                    ib += 1;
                }
            }
        }
        assert_eq!((ia, ib), (a.len(), b.len()));
    }

    fn edits(ops: &[Op]) -> usize {
        ops.iter().filter(|o| !matches!(o, Op::Equal(..))).count()
    }

    #[test]
    fn shortest_scripts() {
        for (a, b, d) in [
            ("abcabba", "cbabac", 5),
            ("", "abc", 3),
            ("abc", "", 3),
            ("abc", "abc", 0),
            ("abcd", "acd", 1),
            ("abcd", "abdc", 2),
            ("xaybzc", "abc", 3),
        ] {
            let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
            let ops = myers(&a, &b);
            apply(&a, &b, &ops);
            assert_eq!(edits(&ops), d, "{a:?} {b:?}");
        }
    }
}

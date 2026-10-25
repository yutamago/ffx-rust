//! The renderer's order of visible instances: `std::sort` as FFX.exe's
//! compiler instanced it for the culling jobs' 12-byte entries
//! (`std_sort_VisibleEntries` `0x009f17c0`), with
//! `PCheckVisibleBoundsJob_compareEntries` (`0x009f2d70`), and the merge
//! of the jobs' sorted runs (`mergeResults` `0x009f3660`).
//!
//! The sort is MSVC's introsort, read from the binary: insertion sort up
//! to 32 entries (`0x009f0d20`); above that the "fat" partition around a
//! median of three, or of nine above 40 entries (`0x009f1c60`,
//! `0x009f1230`), with 1.5 log2(n) divisions before falling back to heap
//! sort (`0x009f0f50`, `0x009f1970`, `0x009f0b60`). Partitioning moves
//! equal entries, so the order of entries that compare equal depends on
//! the input order above 32 entries; this reproduces it.

/// One visible instance: its mesh instance (an index here), its depth
/// key and its render order (`m_render_order`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Entry {
    pub instance: usize,
    pub key: f32,
    pub render_order: i32,
}

/// `compareEntries`: render orders with `ro / 100000000 == 1` first,
/// then `ro % 1000` ascending, then `(ro % 100000) / 1000` ascending,
/// then the key ascending (`<`; a NaN is never before anything). C
/// division and remainder (towards zero), as `idiv`.
#[must_use]
pub fn before(a: &Entry, b: &Entry) -> bool {
    let class = |ro: i32| ro / 100_000_000 == 1;
    if class(a.render_order) != class(b.render_order) {
        return class(a.render_order);
    }
    if a.render_order % 1000 != b.render_order % 1000 {
        return a.render_order % 1000 < b.render_order % 1000;
    }
    let mid = |ro: i32| (ro % 100_000) / 1000;
    if mid(a.render_order) != mid(b.render_order) {
        return mid(a.render_order) < mid(b.render_order);
    }
    a.key < b.key
}

/// Sorts as `std_sort_VisibleEntries` does (`ideal` starts at the count).
pub fn sort(v: &mut [Entry]) {
    let n = v.len();
    sort_range(v, 0, n, n.cast_signed());
}

fn sort_range(v: &mut [Entry], mut first: usize, mut last: usize, mut ideal: isize) {
    while last - first > 32 && ideal > 0 {
        let (mid_first, mid_last) = partition(v, first, last);
        ideal /= 2;
        ideal += ideal / 2;
        if mid_first - first < last - mid_last {
            sort_range(v, first, mid_first, ideal);
            first = mid_last;
        } else {
            sort_range(v, mid_last, last, ideal);
            last = mid_first;
        }
    }
    let count = last - first;
    if count > 32 {
        make_heap(v, first, last);
        sort_heap(v, first, last);
    } else if count > 1 {
        insertion_sort(v, first, last);
    }
}

fn insertion_sort(v: &mut [Entry], first: usize, last: usize) {
    for next in first + 1..last {
        let val = v[next];
        if before(&val, &v[first]) {
            v.copy_within(first..next, first + 1);
            v[first] = val;
        } else {
            let mut hole = next;
            while before(&val, &v[hole - 1]) {
                v[hole] = v[hole - 1];
                hole -= 1;
            }
            v[hole] = val;
        }
    }
}

fn med3(v: &mut [Entry], first: usize, mid: usize, last: usize) {
    if before(&v[mid], &v[first]) {
        v.swap(mid, first);
    }
    if before(&v[last], &v[mid]) {
        v.swap(last, mid);
        if before(&v[mid], &v[first]) {
            v.swap(mid, first);
        }
    }
}

/// `_Median` (`0x009f1230`): of three, or of nine above 40 entries
/// (`last` is the last entry, not past it).
fn median(v: &mut [Entry], first: usize, mid: usize, last: usize) {
    let count = last - first;
    if count > 40 {
        let step = (count + 1) / 8;
        med3(v, first, first + step, first + 2 * step);
        med3(v, mid - step, mid, mid + step);
        med3(v, last - 2 * step, last - step, last);
        med3(v, first + step, mid, last - step);
    } else {
        med3(v, first, mid, last);
    }
}

/// `_Unguarded_partition` (`0x009f1c60`): returns the range of entries
/// equal to the pivot.
fn partition(v: &mut [Entry], first: usize, last: usize) -> (usize, usize) {
    let mid = first + (last - first) / 2;
    median(v, first, mid, last - 1);
    let mut pfirst = mid;
    let mut plast = pfirst + 1;
    while first < pfirst
        && !before(&v[pfirst - 1], &v[pfirst])
        && !before(&v[pfirst], &v[pfirst - 1])
    {
        pfirst -= 1;
    }
    while plast < last && !before(&v[plast], &v[pfirst]) && !before(&v[pfirst], &v[plast]) {
        plast += 1;
    }
    let mut gfirst = plast;
    let mut glast = pfirst;
    loop {
        while gfirst < last {
            if before(&v[pfirst], &v[gfirst]) {
            } else if before(&v[gfirst], &v[pfirst]) {
                break;
            } else {
                if plast != gfirst {
                    v.swap(plast, gfirst);
                }
                plast += 1;
            }
            gfirst += 1;
        }
        while first < glast {
            if before(&v[glast - 1], &v[pfirst]) {
            } else if before(&v[pfirst], &v[glast - 1]) {
                break;
            } else {
                pfirst -= 1;
                if pfirst != glast - 1 {
                    v.swap(pfirst, glast - 1);
                }
            }
            glast -= 1;
        }
        if glast == first && gfirst == last {
            return (pfirst, plast);
        }
        if glast == first {
            if plast != gfirst {
                v.swap(pfirst, plast);
            }
            plast += 1;
            v.swap(pfirst, gfirst);
            pfirst += 1;
            gfirst += 1;
        } else if gfirst == last {
            glast -= 1;
            pfirst -= 1;
            if glast != pfirst {
                v.swap(glast, pfirst);
            }
            plast -= 1;
            v.swap(pfirst, plast);
        } else {
            glast -= 1;
            v.swap(gfirst, glast);
            gfirst += 1;
        }
    }
}

/// `_Adjust_heap` (`0x009f0b60`) with `_Push_heap`.
fn adjust_heap(v: &mut [Entry], base: usize, mut hole: usize, bottom: usize, val: Entry) {
    let top = hole;
    let mut idx = 2 * hole + 2;
    while idx < bottom {
        if before(&v[base + idx], &v[base + idx - 1]) {
            idx -= 1;
        }
        v[base + hole] = v[base + idx];
        hole = idx;
        idx = 2 * idx + 2;
    }
    if idx == bottom {
        v[base + hole] = v[base + bottom - 1];
        hole = bottom - 1;
    }
    while top < hole {
        let parent = (hole - 1) / 2;
        if !before(&v[base + parent], &val) {
            break;
        }
        v[base + hole] = v[base + parent];
        hole = parent;
    }
    v[base + hole] = val;
}

fn make_heap(v: &mut [Entry], first: usize, last: usize) {
    let bottom = last - first;
    let mut hole = bottom / 2;
    while hole > 0 {
        hole -= 1;
        let val = v[first + hole];
        adjust_heap(v, first, hole, bottom, val);
    }
}

/// `_Sort_heap` (`0x009f1970`): each step moves the top to the end and
/// sifts the old last entry in from the top.
fn sort_heap(v: &mut [Entry], first: usize, mut last: usize) {
    while last - first > 1 {
        let val = v[last - 1];
        v[last - 1] = v[first];
        adjust_heap(v, first, 0, last - 1 - first, val);
        last -= 1;
    }
}

/// `mergeResults` (`0x009f3660`): the jobs' sorted runs merged into one
/// list of instances. Each step takes the run whose next entry is in the
/// `/1e8` class over one that is not, else the smaller key (`<`); on a
/// tie the earlier job keeps it. The other `compareEntries` fields are
/// not looked at.
#[must_use]
pub fn merge(runs: &[Vec<Entry>]) -> Vec<usize> {
    let mut at = vec![0usize; runs.len()];
    let mut out = Vec::new();
    loop {
        let mut best: Option<(usize, bool, f32)> = None;
        for (j, run) in runs.iter().enumerate() {
            let Some(e) = run.get(at[j]) else { continue };
            let class = e.render_order / 100_000_000 == 1;
            best = match best {
                None => Some((j, class, e.key)),
                Some((b, bc, bk)) => {
                    let take = if bc == class { e.key < bk } else { class };
                    Some(if take { (j, class, e.key) } else { (b, bc, bk) })
                }
            };
        }
        let Some((j, _, _)) = best else { break };
        out.push(runs[j][at[j]].instance);
        at[j] += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(keys: &[(i32, f32)]) -> Vec<Entry> {
        keys.iter()
            .enumerate()
            .map(|(i, &(render_order, key))| Entry {
                instance: i,
                key,
                render_order,
            })
            .collect()
    }

    #[test]
    fn the_rule_order() {
        let a = Entry {
            instance: 0,
            key: 5.0,
            render_order: 100_000_003,
        };
        let b = Entry {
            instance: 1,
            key: 1.0,
            render_order: 2,
        };
        assert!(before(&a, &b), "the /1e8 class first");
        let c = Entry {
            render_order: 1_002,
            ..b
        };
        assert!(before(&b, &c), "same % 1000, thousands field next");
        let d = Entry { key: 0.5, ..b };
        assert!(before(&d, &b) && !before(&b, &b));
    }

    #[test]
    fn small_sorts_are_stable_insertion_sorts() {
        let mut v = entries(&[(1, 2.0), (0, 3.0), (1, 2.0), (0, 1.0)]);
        sort(&mut v);
        let order: Vec<usize> = v.iter().map(|e| e.instance).collect();
        assert_eq!(order, vec![3, 1, 0, 2]);
    }

    #[test]
    fn large_sorts_are_sorted_and_keep_every_entry() {
        // 300 entries with many equal keys: introsort's partitions.
        let keys: Vec<(i32, f32)> = (0..300)
            .map(|i: i32| {
                (
                    (i * 7) % 5,
                    f32::from(u8::try_from((i * 13) % 17).unwrap_or(0)),
                )
            })
            .collect();
        let mut v = entries(&keys);
        sort(&mut v);
        for w in v.windows(2) {
            assert!(!before(&w[1], &w[0]));
        }
        let mut seen: Vec<usize> = v.iter().map(|e| e.instance).collect();
        seen.sort_unstable();
        assert_eq!(seen, (0..300).collect::<Vec<_>>());
    }

    #[test]
    fn merging_runs() {
        let a = entries(&[(0, 1.0), (0, 4.0)]);
        let mut b = entries(&[(0, 2.0), (5, 3.0), (100_000_000, 9.0)]);
        for e in &mut b {
            e.instance += 10;
        }
        // the /1e8 entry jumps ahead of the smaller key in the other run
        assert_eq!(merge(&[a, b]), vec![0, 10, 11, 12, 1]);
    }
}

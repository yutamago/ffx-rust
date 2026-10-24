//! The comparison: align the two item lists, then compare the pairs field
//! by field.
//!
//! 1. Myers on the items' fingerprints: equal items are equal pairs.
//! 2. In each gap between them, Myers again on the match keys (shaders,
//!    vertex and index keys, start, count): those pairs are the same
//!    draw with other contents, compared field by field.
//! 3. What is left in A and B pairs up as moved: first by fingerprint (the
//!    same draw elsewhere in the order), then by match key (elsewhere, and
//!    changed).
//! 4. The rest is missing (only in A) or extra (only in B).

use std::collections::{HashMap, VecDeque};

use crate::align::{Op, myers};
use crate::capture::{Capture, FrameItems};
use crate::cb::Constants;
use crate::rules::Rule;
use crate::{Class, Difference, Item, Key, Value};

/// Which items are compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Part {
    #[default]
    All,
    /// Draws into the scene targets, and clears and copies of them.
    Scene,
}

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub part: Part,
    /// `ulp` rules.
    pub tolerances: Vec<Rule>,
    /// Allow-list rules.
    pub allow: Vec<Rule>,
}

/// One frame's result.
#[derive(Debug, Clone, Default)]
pub struct FrameDiff {
    pub scene: String,
    /// Items compared (after the part filter).
    pub a_items: usize,
    pub b_items: usize,
    /// Pairs found (equal, changed or moved).
    pub pairs: usize,
    /// Pairs that are equal in every field.
    pub equal: usize,
    pub diffs: Vec<Difference>,
    /// Float differences a tolerance covered.
    pub tolerated: usize,
}

impl FrameDiff {
    /// Differences no allow-list rule covers.
    #[must_use]
    pub fn unallowed(&self) -> usize {
        self.diffs.iter().filter(|d| d.allowed.is_none()).count()
    }
}

#[derive(Debug, Clone, Default)]
pub struct Comparison {
    pub frames: Vec<FrameDiff>,
}

impl Comparison {
    /// Differences no allow-list rule covers, over all frames.
    #[must_use]
    pub fn unallowed(&self) -> usize {
        self.frames.iter().map(FrameDiff::unallowed).sum()
    }
}

/// Compares every frame of `a` with the same frame of `b`; a frame only one
/// side has compares with an empty one.
#[must_use]
pub fn compare(a: &Capture, b: &Capture, opts: &Options) -> Comparison {
    let empty = FrameItems {
        number: 0,
        scene: String::new(),
        items: Vec::new(),
    };
    let n = a.frames.len().max(b.frames.len());
    Comparison {
        frames: (0..n)
            .map(|i| {
                let fa = a.frames.get(i).unwrap_or(&empty);
                let fb = b.frames.get(i).unwrap_or(&empty);
                compare_frames(fa, fb, opts)
            })
            .collect(),
    }
}

struct Ctx<'a> {
    scene: &'a str,
    opts: &'a Options,
    tolerated: usize,
    diffs: Vec<Difference>,
}

/// Compares two frames.
#[must_use]
#[allow(clippy::too_many_lines, clippy::many_single_char_names)] // the four steps; a, b sides, x, y indices
pub fn compare_frames(a: &FrameItems, b: &FrameItems, opts: &Options) -> FrameDiff {
    let pick = |f: &FrameItems| -> Vec<usize> {
        (0..f.items.len())
            .filter(|&i| opts.part == Part::All || f.items[i].scene)
            .collect()
    };
    let (ia, ib) = (pick(a), pick(b));
    let scene = if a.scene.is_empty() {
        &b.scene
    } else {
        &a.scene
    };
    let mut cx = Ctx {
        scene,
        opts,
        tolerated: 0,
        diffs: Vec::new(),
    };
    let fa: Vec<Key> = ia.iter().map(|&i| a.items[i].fingerprint).collect();
    let fb: Vec<Key> = ib.iter().map(|&i| b.items[i].fingerprint).collect();
    let mut equal = 0;
    let mut changed: Vec<(usize, usize)> = Vec::new();
    let mut lone_a: Vec<usize> = Vec::new();
    let mut lone_b: Vec<usize> = Vec::new();
    // Gaps between equal runs, aligned again on match keys.
    let mut gap_a: Vec<usize> = Vec::new();
    let mut gap_b: Vec<usize> = Vec::new();
    let close_gap = |gap_a: &mut Vec<usize>,
                     gap_b: &mut Vec<usize>,
                     changed: &mut Vec<(usize, usize)>,
                     lone_a: &mut Vec<usize>,
                     lone_b: &mut Vec<usize>| {
        let ka: Vec<Key> = gap_a.iter().map(|&i| a.items[i].match_key).collect();
        let kb: Vec<Key> = gap_b.iter().map(|&i| b.items[i].match_key).collect();
        for op in myers(&ka, &kb) {
            match op {
                Op::Equal(x, y) => changed.push((gap_a[x], gap_b[y])),
                Op::Delete(x) => lone_a.push(gap_a[x]),
                Op::Insert(y) => lone_b.push(gap_b[y]),
            }
        }
        gap_a.clear();
        gap_b.clear();
    };
    for op in myers(&fa, &fb) {
        match op {
            Op::Equal(..) => {
                close_gap(
                    &mut gap_a,
                    &mut gap_b,
                    &mut changed,
                    &mut lone_a,
                    &mut lone_b,
                );
                equal += 1;
            }
            Op::Delete(x) => gap_a.push(ia[x]),
            Op::Insert(y) => gap_b.push(ib[y]),
        }
    }
    close_gap(
        &mut gap_a,
        &mut gap_b,
        &mut changed,
        &mut lone_a,
        &mut lone_b,
    );
    let pairs_changed = changed.len();
    for (x, y) in changed {
        cx.fields(x, &a.items[x], y, &b.items[y]);
    }
    // Moved: by fingerprint, then by match key.
    let mut moved = 0;
    for by_fingerprint in [true, false] {
        let k = |it: &Item| {
            if by_fingerprint {
                it.fingerprint
            } else {
                it.match_key
            }
        };
        let mut queue: HashMap<Key, VecDeque<usize>> = HashMap::new();
        for &y in &lone_b {
            queue.entry(k(&b.items[y])).or_default().push_back(y);
        }
        let mut taken_b = std::collections::HashSet::new();
        let mut rest_a = Vec::new();
        for &x in &lone_a {
            match queue.get_mut(&k(&a.items[x])).and_then(VecDeque::pop_front) {
                Some(y) => {
                    taken_b.insert(y);
                    moved += 1;
                    let it = &a.items[x];
                    cx.diffs.push(Difference {
                        class: Class::Moved,
                        a: Some(x),
                        b: Some(y),
                        field: String::new(),
                        var: None,
                        va: Value::U(u32::try_from(x).unwrap_or(u32::MAX)),
                        vb: Value::U(u32::try_from(y).unwrap_or(u32::MAX)),
                        kind: it.kind,
                        vs: it.vs,
                        ps: it.ps,
                        allowed: None,
                    });
                    if !by_fingerprint {
                        cx.fields(x, it, y, &b.items[y]);
                    }
                }
                None => rest_a.push(x),
            }
        }
        lone_a = rest_a;
        lone_b.retain(|y| !taken_b.contains(y));
    }
    for (class, side, items) in [
        (Class::Missing, true, &lone_a),
        (Class::Extra, false, &lone_b),
    ] {
        for &i in items {
            let it = if side { &a.items[i] } else { &b.items[i] };
            cx.diffs.push(Difference {
                class,
                a: side.then_some(i),
                b: (!side).then_some(i),
                field: String::new(),
                var: None,
                va: Value::Absent,
                vb: Value::Absent,
                kind: it.kind,
                vs: it.vs,
                ps: it.ps,
                allowed: None,
            });
        }
    }
    for d in &mut cx.diffs {
        d.allowed = opts.allow.iter().position(|r| r.allows(scene, d));
    }
    // In A's order.
    cx.diffs
        .sort_by_key(|d| (d.a.or(d.b).unwrap_or(0), d.class));
    let diffs_in_pairs: std::collections::HashSet<(usize, usize)> = cx
        .diffs
        .iter()
        .filter(|d| d.class != Class::Moved)
        .filter_map(|d| Some((d.a?, d.b?)))
        .collect();
    FrameDiff {
        scene: scene.clone(),
        a_items: ia.len(),
        b_items: ib.len(),
        pairs: equal + pairs_changed + moved,
        equal: equal + pairs_changed + moved - diffs_in_pairs.len(),
        diffs: cx.diffs,
        tolerated: cx.tolerated,
    }
}

#[allow(clippy::many_single_char_names)] // x, y: the pair's indices; i, j, p, q: its fields
impl Ctx<'_> {
    #[allow(clippy::too_many_arguments)] // the pair and the field
    fn push(
        &mut self,
        x: usize,
        ia: &Item,
        y: usize,
        class: Class,
        field: String,
        var: Option<String>,
        va: Value,
        vb: Value,
    ) {
        if let (Value::F(fa), Value::F(fb)) = (&va, &vb)
            && self.opts.tolerances.iter().any(|r| {
                r.tolerates(
                    self.scene,
                    ia.kind,
                    ia.vs,
                    ia.ps,
                    &field,
                    var.as_deref(),
                    *fa,
                    *fb,
                )
            })
        {
            self.tolerated += 1;
            return;
        }
        self.diffs.push(Difference {
            class,
            a: Some(x),
            b: Some(y),
            field,
            var,
            va,
            vb,
            kind: ia.kind,
            vs: ia.vs,
            ps: ia.ps,
            allowed: None,
        });
    }

    /// Field by field: states and resources by name, constants by stage,
    /// slot, variable and word.
    fn fields(&mut self, x: usize, ia: &Item, y: usize, ib: &Item) {
        let (fa, fb) = (&ia.fields, &ib.fields);
        let (mut i, mut j) = (0, 0);
        while i < fa.len() || j < fb.len() {
            let ord = match (fa.get(i), fb.get(j)) {
                (Some(p), Some(q)) => p.name.cmp(&q.name),
                (Some(_), None) => std::cmp::Ordering::Less,
                _ => std::cmp::Ordering::Greater,
            };
            match ord {
                std::cmp::Ordering::Equal => {
                    let (p, q) = (&fa[i], &fb[j]);
                    if !p.value.same(&q.value) {
                        self.push(
                            x,
                            ia,
                            y,
                            p.class,
                            p.name.clone(),
                            None,
                            p.value.clone(),
                            q.value.clone(),
                        );
                    }
                    i += 1;
                    j += 1;
                }
                std::cmp::Ordering::Less => {
                    let p = &fa[i];
                    self.push(
                        x,
                        ia,
                        y,
                        p.class,
                        p.name.clone(),
                        None,
                        p.value.clone(),
                        Value::Absent,
                    );
                    i += 1;
                }
                std::cmp::Ordering::Greater => {
                    let q = &fb[j];
                    self.push(
                        x,
                        ia,
                        y,
                        q.class,
                        q.name.clone(),
                        None,
                        Value::Absent,
                        q.value.clone(),
                    );
                    j += 1;
                }
            }
        }
        let find = |list: &'_ [Constants], c: &Constants| -> Option<usize> {
            list.iter()
                .position(|d| d.stage == c.stage && d.slot == c.slot)
        };
        for ca in &ia.constants {
            let cb = find(&ib.constants, ca).map(|k| &ib.constants[k]);
            self.constants(x, ia, y, Some(ca), cb);
        }
        for cb in &ib.constants {
            if find(&ia.constants, cb).is_none() {
                self.constants(x, ia, y, None, Some(cb));
            }
        }
    }

    fn constants(
        &mut self,
        x: usize,
        ia: &Item,
        y: usize,
        ca: Option<&Constants>,
        cb: Option<&Constants>,
    ) {
        let Some(any) = ca.or(cb) else { return };
        let vars_a = ca.map_or(&[][..], |c| &c.vars[..]);
        let vars_b = cb.map_or(&[][..], |c| &c.vars[..]);
        let mut names: Vec<&str> = vars_a.iter().map(|v| &*v.name).collect();
        for v in vars_b {
            if !names.contains(&&*v.name) {
                names.push(&v.name);
            }
        }
        for name in names {
            let va = vars_a.iter().find(|v| &*v.name == name);
            let vb = vars_b.iter().find(|v| &*v.name == name);
            let Some(var) = va.or(vb) else { continue };
            let n = va
                .map_or(0, |v| v.words.len())
                .max(vb.map_or(0, |v| v.words.len()));
            for w in 0..n {
                let get = |v: Option<&crate::cb::Var>| -> Value {
                    match v.and_then(|v| v.words.get(w)) {
                        Some(&(Some(bits), true)) => Value::F(bits),
                        Some(&(Some(bits), false)) => Value::U(bits),
                        _ => Value::Absent,
                    }
                };
                let (p, q) = (get(va), get(vb));
                if p != q {
                    let field = format!("{}.cb{}.{}", any.stage, any.slot, var.word_name(w));
                    self.push(
                        x,
                        ia,
                        y,
                        Class::Constant,
                        field,
                        Some(name.to_owned()),
                        p,
                        q,
                    );
                }
            }
        }
    }
}

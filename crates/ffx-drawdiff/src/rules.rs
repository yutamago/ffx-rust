//! Tolerance and allow-list files (`docs/drawdiff.md`, "Rules").
//!
//! One rule per line, `#` starts a comment, `[scene]` starts the rules of
//! one scene (`[*]`, the default, for every scene):
//!
//! ```text
//! [cdsp0000]
//! constant ps=8b5157eb9c138a63 var=Time cause: the water's clock is the wall clock
//! ulp 1 var=scene.ViewProjection cause: ...
//! ```
//!
//! An allow-list line starts with a class (`missing`, `extra`, `moved`,
//! `state`, `constant`, `resource`, or `any`); a tolerance line with `ulp
//! <n>`. Selectors narrow it: `vs=<hex>`, `ps=<hex>`, `kind=draw|clear|
//! clear-depth|copy`, `field=<glob>`, `var=<glob>` (`*` matches anything).
//! Every rule needs a cause after `cause:`: what makes the difference, so
//! that nobody allows a difference nobody understands.

use crate::{Class, Difference, ItemKind, Key};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    /// Where it was written (`file:line`).
    pub at: String,
    /// `None`: every scene.
    pub scene: Option<String>,
    /// `None`: any class.
    pub class: Option<Class>,
    /// A tolerance: at most this many units in the last place.
    pub ulp: Option<u64>,
    pub kind: Option<ItemKind>,
    pub vs: Option<Key>,
    pub ps: Option<Key>,
    pub field: Option<String>,
    pub var: Option<String>,
    pub cause: String,
}

/// `*`-only glob.
#[must_use]
pub fn glob(pattern: &str, text: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == text;
    }
    let (first, last) = (parts[0], parts[parts.len() - 1]);
    if !text.starts_with(first) || text.len() < first.len() + last.len() || !text.ends_with(last) {
        return false;
    }
    let mut rest = &text[first.len()..text.len() - last.len()];
    for p in &parts[1..parts.len() - 1] {
        match rest.find(p) {
            Some(i) => rest = &rest[i + p.len()..],
            None => return false,
        }
    }
    true
}

fn kind(s: &str) -> Option<ItemKind> {
    [
        ItemKind::Draw,
        ItemKind::ClearTarget,
        ItemKind::ClearDepth,
        ItemKind::Copy,
    ]
    .into_iter()
    .find(|k| k.name() == s)
}

/// Parses a rules file; `name` is for the `at` of each rule.
///
/// # Errors
/// On a malformed line or a rule without a cause; the message says where.
pub fn parse(name: &str, text: &str) -> Result<Vec<Rule>, String> {
    let mut rules = Vec::new();
    let mut scene: Option<String> = None;
    for (n, raw) in text.lines().enumerate() {
        let at = format!("{name}:{}", n + 1);
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if let Some(s) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            scene = (s != "*").then(|| s.to_owned());
            continue;
        }
        let (head, cause) = line
            .split_once("cause:")
            .ok_or_else(|| format!("{at}: no `cause:`"))?;
        let cause = cause.trim();
        if cause.is_empty() || cause.eq_ignore_ascii_case("todo") {
            return Err(format!("{at}: the cause is not written"));
        }
        let mut words = head.split_whitespace();
        let mut rule = Rule {
            at: at.clone(),
            scene: scene.clone(),
            class: None,
            ulp: None,
            kind: None,
            vs: None,
            ps: None,
            field: None,
            var: None,
            cause: cause.to_owned(),
        };
        match words.next() {
            Some("ulp") => {
                let v = words
                    .next()
                    .and_then(|w| w.parse().ok())
                    .ok_or_else(|| format!("{at}: `ulp` needs a count"))?;
                rule.ulp = Some(v);
            }
            Some("any") => {}
            Some(c) => {
                rule.class =
                    Some(Class::parse(c).ok_or_else(|| format!("{at}: unknown class {c}"))?);
            }
            None => return Err(format!("{at}: empty rule")),
        }
        for w in words {
            let (k, v) = w
                .split_once('=')
                .ok_or_else(|| format!("{at}: `{w}` is not key=value"))?;
            let hex = || u64::from_str_radix(v, 16).map_err(|_| format!("{at}: bad key {v}"));
            match k {
                "vs" => rule.vs = Some(hex()?),
                "ps" => rule.ps = Some(hex()?),
                "kind" => {
                    rule.kind = Some(kind(v).ok_or_else(|| format!("{at}: unknown kind {v}"))?);
                }
                "field" => rule.field = Some(v.to_owned()),
                "var" => rule.var = Some(v.to_owned()),
                _ => return Err(format!("{at}: unknown selector {k}")),
            }
        }
        rules.push(rule);
    }
    Ok(rules)
}

impl Rule {
    fn selects(
        &self,
        scene: &str,
        kind: ItemKind,
        vs: Key,
        ps: Key,
        field: &str,
        var: Option<&str>,
    ) -> bool {
        self.scene.as_deref().is_none_or(|s| s == scene)
            && self.kind.is_none_or(|k| k == kind)
            && self.vs.is_none_or(|k| k == vs)
            && self.ps.is_none_or(|k| k == ps)
            && self.field.as_deref().is_none_or(|g| glob(g, field))
            && self
                .var
                .as_deref()
                .is_none_or(|g| var.is_some_and(|v| glob(g, v)))
    }

    /// Whether this allow-list rule covers `d` (never for a tolerance).
    #[must_use]
    pub fn allows(&self, scene: &str, d: &Difference) -> bool {
        self.ulp.is_none()
            && self.class.is_none_or(|c| c == d.class)
            && self.selects(scene, d.kind, d.vs, d.ps, &d.field, d.var.as_deref())
    }

    /// Whether this tolerance covers two float words of a field.
    #[must_use]
    #[allow(clippy::too_many_arguments)] // the difference's coordinates
    pub fn tolerates(
        &self,
        scene: &str,
        kind: ItemKind,
        vs: Key,
        ps: Key,
        field: &str,
        var: Option<&str>,
        a: u32,
        b: u32,
    ) -> bool {
        self.ulp
            .is_some_and(|n| crate::ulps(a, b).is_some_and(|u| u <= n))
            && self.selects(scene, kind, vs, ps, field, var)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globs() {
        assert!(glob("a*", "abc"));
        assert!(glob("*c", "abc"));
        assert!(glob("a*c", "abc"));
        assert!(glob("*b*", "abc"));
        assert!(!glob("a*d", "abc"));
        assert!(glob("abc", "abc"));
        assert!(!glob("ab", "abc"));
        assert!(!glob("ab*bc", "abc"));
    }

    #[test]
    fn rules_need_causes() {
        assert!(parse("t", "constant var=Time").is_err());
        assert!(parse("t", "constant var=Time cause: TODO").is_err());
        assert!(parse("t", "colour cause: x").is_err());
        let r = parse(
            "t",
            "# noise\n[cdsp0000]\nconstant ps=00ff var=Time* cause: the wall clock\n[*]\nulp 2 field=viewport.* cause: y\n",
        )
        .unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].scene.as_deref(), Some("cdsp0000"));
        assert_eq!(r[0].ps, Some(0xff));
        assert_eq!(r[1].scene, None);
        assert_eq!(r[1].ulp, Some(2));
        assert_eq!(r[1].at, "t:5");
    }
}

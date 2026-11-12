//! The movies' `ActionScript` 3 (`DoABC` tags), read with Ruffle's `swf`
//! crate: which functions a movie offers the game
//! (`ExternalInterface.addCallback`), which it calls in the game
//! (`ExternalInterface.call`), and how many arguments each offered function
//! requires (`docs/formats/pc-ui.md`, "Ruffle").
//!
//! Iggy runs an AS3 function the game calls with whatever arguments the
//! game passes; Flash Player and Ruffle throw `ArgumentError` #1063 when
//! fewer than the declared ones arrive. The game calls
//! `addChangeParaPageItem` with 4 arguments, `escmenu.swf` declares 5: under
//! Ruffle the call threw and the Change parameters page stayed empty.
//! [`Movie::required`] gives the count, and [`crate::stage::Stage`] pads the
//! missing ones with `undefined`, as Iggy's player does.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use anyhow::{Result, anyhow};
use ruffle_core::swf;
use ruffle_core::swf::avm2::read::Reader;
use ruffle_core::swf::avm2::types::{
    AbcFile, Index, Method, MethodFlags, Multiname, Namespace, Op, TraitKind,
};

/// One `DoABC` block.
pub struct Abc {
    pub file: AbcFile,
}

/// A method of a class (or a script's function) by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    /// `Class/name` (`-` for a script-level function).
    pub owner: String,
    pub name: String,
    /// Declared parameters.
    pub params: u32,
    /// Those without a default value.
    pub required: u32,
    /// `...rest`.
    pub rest: bool,
    /// The parameter types (`*` for none).
    pub types: Vec<String>,
}

/// What a movie's AS3 says about its interface with the game.
#[derive(Debug, Clone, Default)]
pub struct Movie {
    /// Every class method and script function.
    pub methods: Vec<Signature>,
    /// `ExternalInterface.addCallback` names (what the game can call).
    pub callbacks: BTreeSet<String>,
    /// The method each `addCallback` hands over, where the code shows it
    /// (`addCallback("name", this.f)` in a method of class `C`): name ->
    /// (`C`, `f`).
    pub targets: BTreeMap<String, (String, String)>,
    /// `ExternalInterface.call` names (what the movie calls in the game).
    pub calls: BTreeSet<String>,
}

impl Movie {
    /// Reads every `DoABC` tag of a `.swf` (compressed or not).
    ///
    /// # Errors
    /// Not a SWF, or an ABC block Ruffle can't read.
    pub fn from_swf(data: &[u8]) -> Result<Self> {
        let mut movie = Self::default();
        for abc in blocks(data)? {
            abc.collect(&mut movie);
        }
        Ok(movie)
    }

    /// The methods of class `owner` (or of any class, `None`) named `name`.
    fn named<'a>(
        &'a self,
        owner: Option<&'a str>,
        name: &'a str,
    ) -> impl Iterator<Item = &'a Signature> + 'a {
        self.methods
            .iter()
            .filter(move |m| m.name == name && owner.is_none_or(|o| m.owner == o))
    }

    /// The arguments an offered function (`addCallback` name) requires: its
    /// target method's, else the largest among the methods of that name;
    /// `None` when the movie has none.
    #[must_use]
    pub fn required(&self, name: &str) -> Option<u32> {
        self.arity(name).map(|a| a.required)
    }

    /// How many arguments an offered function takes: the method its
    /// `addCallback` hands over ([`Movie::targets`]), else the widest of the
    /// methods of that name (required the largest, at most the largest,
    /// unlimited when one has `...rest`).
    #[must_use]
    pub fn arity(&self, name: &str) -> Option<Arity> {
        let fold = |it: &mut dyn Iterator<Item = &Signature>| {
            let mut a: Option<Arity> = None;
            for m in it {
                let b = a.get_or_insert(Arity {
                    required: 0,
                    most: Some(0),
                });
                b.required = b.required.max(m.required);
                b.most = match (b.most, m.rest) {
                    (Some(n), false) => Some(n.max(m.params)),
                    _ => None,
                };
            }
            a
        };
        if let Some((owner, target)) = self.targets.get(name)
            && let Some(a) = fold(&mut self.named(Some(owner), target))
        {
            return Some(a);
        }
        fold(&mut self.named(None, name))
    }

    /// [`Movie::arity`] of every offered function.
    #[must_use]
    pub fn arities(&self) -> BTreeMap<String, Arity> {
        self.callbacks
            .iter()
            .filter_map(|c| self.arity(c).map(|a| (c.clone(), a)))
            .collect()
    }
}

/// The argument count an AS3 function accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Arity {
    /// Parameters without a default.
    pub required: u32,
    /// All parameters; `None` with `...rest`.
    pub most: Option<u32>,
}

impl Arity {
    /// `args` fitted the way Iggy's player calls a function: missing
    /// required arguments become `fill` (`undefined`; the AS3 parameter
    /// type then coerces it, as for any argument), extra ones are dropped.
    /// Ruffle, like Flash Player, throws `ArgumentError` #1063 instead.
    pub fn fit<T: Clone>(self, args: &mut Vec<T>, fill: &T) {
        let required = self.required as usize;
        if args.len() < required {
            args.resize(required, fill.clone());
        }
        if let Some(most) = self.most
            && args.len() > most as usize
        {
            args.truncate(most as usize);
        }
    }
}

/// The `DoABC` blocks of a `.swf`.
///
/// # Errors
/// Not a SWF, or an ABC block Ruffle can't read.
pub fn blocks(data: &[u8]) -> Result<Vec<Abc>> {
    let buf = swf::decompress_swf(data).map_err(|e| anyhow!("{e}"))?;
    let parsed = swf::parse_swf(&buf).map_err(|e| anyhow!("{e}"))?;
    let mut out = Vec::new();
    for tag in &parsed.tags {
        let bytes = match tag {
            swf::Tag::DoAbc(b) => *b,
            swf::Tag::DoAbc2(d) => d.data,
            _ => continue,
        };
        let file = Reader::new(bytes).read().map_err(|e| anyhow!("{e:?}"))?;
        out.push(Abc { file });
    }
    Ok(out)
}

impl Abc {
    /// A constant-pool string (`*` for index 0).
    #[must_use]
    pub fn string(&self, i: Index<String>) -> String {
        i.0.checked_sub(1)
            .and_then(|k| self.file.constant_pool.strings.get(k as usize))
            .map_or_else(
                || "*".to_owned(),
                |s| String::from_utf8_lossy(s).into_owned(),
            )
    }

    fn namespace(&self, i: Index<Namespace>) -> String {
        let ns =
            i.0.checked_sub(1)
                .and_then(|k| self.file.constant_pool.namespaces.get(k as usize));
        match ns {
            Some(
                Namespace::Namespace(s)
                | Namespace::Package(s)
                | Namespace::PackageInternal(s)
                | Namespace::Protected(s)
                | Namespace::Explicit(s)
                | Namespace::StaticProtected(s)
                | Namespace::Private(s),
            ) => self.string(*s),
            None => String::new(),
        }
    }

    /// A multiname as `name` or `package.name` (`*` for index 0).
    #[must_use]
    pub fn name(&self, i: Index<Multiname>) -> String {
        let Some(m) =
            i.0.checked_sub(1)
                .and_then(|k| self.file.constant_pool.multinames.get(k as usize))
        else {
            return "*".to_owned();
        };
        match m {
            Multiname::QName { namespace, name } | Multiname::QNameA { namespace, name } => {
                let ns = self.namespace(*namespace);
                let n = self.string(*name);
                if ns.is_empty() {
                    n
                } else {
                    format!("{ns}.{n}")
                }
            }
            Multiname::RTQName { name }
            | Multiname::RTQNameA { name }
            | Multiname::Multiname { name, .. }
            | Multiname::MultinameA { name, .. } => self.string(*name),
            Multiname::TypeName {
                base_type,
                parameters,
            } => {
                let p: Vec<String> = parameters.iter().map(|p| self.name(*p)).collect();
                format!("{}.<{}>", self.name(*base_type), p.join(","))
            }
            _ => "[runtime]".to_owned(),
        }
    }

    /// A multiname's bare name (no package).
    #[must_use]
    pub fn bare(&self, i: Index<Multiname>) -> String {
        let n = self.name(i);
        n.rsplit('.').next().unwrap_or(&n).to_owned()
    }

    fn method(&self, i: Index<Method>) -> Option<&Method> {
        self.file.methods.get(i.0 as usize)
    }

    fn signature(&self, owner: &str, name: String, m: &Method) -> Signature {
        let params = u32::try_from(m.params.len()).unwrap_or(u32::MAX);
        let optional = m
            .params
            .iter()
            .filter(|p| p.default_value.is_some())
            .count();
        Signature {
            owner: owner.to_owned(),
            name,
            params,
            required: params.saturating_sub(u32::try_from(optional).unwrap_or(0)),
            rest: m.flags.contains(MethodFlags::NEED_REST),
            types: m.params.iter().map(|p| self.name(p.kind)).collect(),
        }
    }

    /// The body's instructions (stops at the first one Ruffle can't read).
    #[must_use]
    pub fn ops(&self, method: Index<Method>) -> Vec<Op> {
        let Some(body) = self
            .method(method)
            .and_then(|m| m.body)
            .and_then(|b| self.file.method_bodies.get(b.0 as usize))
        else {
            return Vec::new();
        };
        let mut r = Reader::new(&body.code);
        let mut out = Vec::new();
        while let Ok(op) = r.read_op() {
            out.push(op);
        }
        out
    }

    /// Every class method, getter, setter and function with its owner.
    #[must_use]
    pub fn all_methods(&self) -> Vec<(String, String, Index<Method>)> {
        let mut out = Vec::new();
        for (inst, class) in self.file.instances.iter().zip(&self.file.classes) {
            let owner = self.name(inst.name);
            out.push((owner.clone(), "<init>".to_owned(), inst.init_method));
            out.push((owner.clone(), "<cinit>".to_owned(), class.init_method));
            for t in inst.traits.iter().chain(&class.traits) {
                if let TraitKind::Method { method, .. }
                | TraitKind::Getter { method, .. }
                | TraitKind::Setter { method, .. }
                | TraitKind::Function {
                    function: method, ..
                } = t.kind
                {
                    out.push((owner.clone(), self.bare(t.name), method));
                }
            }
        }
        for (i, s) in self.file.scripts.iter().enumerate() {
            let owner = format!("script{i}");
            out.push((owner.clone(), "<init>".to_owned(), s.init_method));
            for t in &s.traits {
                if let TraitKind::Method { method, .. }
                | TraitKind::Function {
                    function: method, ..
                } = t.kind
                {
                    out.push((owner.clone(), self.bare(t.name), method));
                }
            }
        }
        out
    }

    fn collect(&self, movie: &mut Movie) {
        for (owner, name, method) in self.all_methods() {
            if let Some(m) = self.method(method) {
                movie.methods.push(self.signature(&owner, name, m));
            }
            // `ExternalInterface.addCallback("name", f)` /
            // `ExternalInterface.call("name", ...)`: the last string pushed
            // after the last reference to `ExternalInterface`
            let mut armed = false;
            let mut first: Option<String> = None;
            let mut closure: Option<String> = None;
            for op in self.ops(method) {
                match op {
                    Op::GetLex { index }
                    | Op::FindPropStrict { index }
                    | Op::FindProperty { index }
                        if self.bare(index) == "ExternalInterface" =>
                    {
                        armed = true;
                        first = None;
                        closure = None;
                    }
                    Op::PushString { value } if armed && first.is_none() => {
                        first = Some(self.string(value));
                    }
                    Op::GetProperty { index } | Op::GetLex { index }
                        if armed && first.is_some() && closure.is_none() =>
                    {
                        closure = Some(self.bare(index));
                    }
                    Op::CallProperty { index, .. } | Op::CallPropVoid { index, .. } if armed => {
                        match self.bare(index).as_str() {
                            "addCallback" => {
                                if let Some(f) = first.take() {
                                    if let Some(c) = closure.take() {
                                        movie.targets.insert(f.clone(), (owner.clone(), c));
                                    }
                                    movie.callbacks.insert(f);
                                }
                                armed = false;
                            }
                            "call" => {
                                if let Some(f) = first.take() {
                                    movie.calls.insert(f);
                                }
                                armed = false;
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// A readable listing of one method (`owner` `Class` or `script<n>`,
    /// `name` the method's): its signature and instructions, names
    /// resolved.
    #[must_use]
    pub fn disassemble(&self, owner: &str, name: &str) -> String {
        let mut s = String::new();
        for (o, n, method) in self.all_methods() {
            if n != name || !(o == owner || o.ends_with(&format!(".{owner}"))) {
                continue;
            }
            s.push_str(&self.listing(&o, &n, method));
        }
        s
    }

    /// Every method whose listing has a line containing `needle`: `Class/name`
    /// and those lines.
    #[must_use]
    pub fn find(&self, needle: &str) -> String {
        let mut s = String::new();
        for (o, n, method) in self.all_methods() {
            let hits: Vec<String> = self
                .ops(method)
                .iter()
                .map(|op| self.op_text(op))
                .filter(|t| t.contains(needle))
                .collect();
            if !hits.is_empty() {
                let _ = writeln!(s, "{o}/{n}: {}", hits.join("; "));
            }
        }
        s
    }

    fn listing(&self, o: &str, n: &str, method: Index<Method>) -> String {
        let mut s = String::new();
        {
            if let Some(m) = self.method(method) {
                let sig = self.signature(o, n.to_owned(), m);
                let _ = writeln!(
                    s,
                    "{o}/{n}({}) required {}{}",
                    sig.types.join(", "),
                    sig.required,
                    if sig.rest { " ...rest" } else { "" }
                );
            }
            for (i, op) in self.ops(method).iter().enumerate() {
                let _ = writeln!(s, "  {i:4} {}", self.op_text(op));
            }
        }
        s
    }

    fn op_text(&self, op: &Op) -> String {
        let raw = format!("{op:?}");
        match op {
            Op::GetLex { index }
            | Op::FindPropStrict { index }
            | Op::FindProperty { index }
            | Op::GetProperty { index }
            | Op::SetProperty { index }
            | Op::InitProperty { index }
            | Op::Coerce { index }
            | Op::AsType { type_name: index }
            | Op::IsType { index } => {
                format!(
                    "{} {}",
                    raw.split(' ').next().unwrap_or(""),
                    self.name(*index)
                )
            }
            Op::CallProperty { index, num_args }
            | Op::CallPropVoid { index, num_args }
            | Op::CallPropLex { index, num_args }
            | Op::ConstructProp { index, num_args }
            | Op::CallSuper { index, num_args }
            | Op::CallSuperVoid { index, num_args } => format!(
                "{} {} ({num_args})",
                raw.split(' ').next().unwrap_or(""),
                self.name(*index)
            ),
            Op::PushString { value } => format!("PushString {:?}", self.string(*value)),
            _ => raw,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The game's movies, from the player's `FFX_Data.vbf` (skipped
    /// without `FFX_GAME_DIR`).
    fn escmenu() -> Option<Vec<u8>> {
        let dir = std::env::var_os("FFX_GAME_DIR")?;
        let vbf = std::path::Path::new(&dir).join("data/FFX_Data.vbf");
        crate::flash_movies(&vbf)
            .ok()?
            .into_iter()
            .find(|(n, _)| n == "escmenu.swf")
            .map(|(_, d)| d)
    }

    #[test]
    fn escmenu_offers_the_change_parameters_rows_with_five_arguments() {
        let Some(data) = escmenu() else {
            eprintln!("FFX_GAME_DIR unset: skipped");
            return;
        };
        let movie = Movie::from_swf(&data).unwrap();
        // what FFEscMenu calls (docs/formats/pc-ui.md, "Iggy calls")
        for name in [
            "SetLanguage",
            "setHomePage",
            "addVideoPageItem",
            "addChangeParaPageItem",
            "showEscMenu",
            "setHelpText",
        ] {
            assert!(movie.callbacks.contains(name), "{name} not offered");
        }
        // the game passes 4 (index, kind, label, button text); the fifth,
        // `initData:Array`, has no default
        assert_eq!(movie.required("addChangeParaPageItem"), Some(5));
        let arities = movie.arities();
        let a = arities["addChangeParaPageItem"];
        assert_eq!(
            a,
            Arity {
                required: 5,
                most: Some(5)
            }
        );
        let mut args = vec![1, 2, 3, 4];
        a.fit(&mut args, &0);
        assert_eq!(args, [1, 2, 3, 4, 0]);
        // two classes have a confirmSelect; the one offered is EscPage's,
        // without parameters
        assert_eq!(
            movie.targets["confirmSelect"],
            ("EscPage".to_owned(), "confirmSelect".to_owned())
        );
        assert_eq!(arities["confirmSelect"].most, Some(0));
        // setMouseVisible(b:Boolean = ...): the game's one argument fits
        let m = arities["setMouseVisible"];
        assert_eq!((m.required, m.most), (0, Some(1)));
        let mut args = vec![7];
        m.fit(&mut args, &0);
        assert_eq!(args, [7]);
        assert!(movie.calls.contains("EscMenuHelpTextRefresh"));
        assert!(movie.calls.contains("playEscSound"));
    }

    #[test]
    fn fit_drops_extra_arguments_unless_rest() {
        let mut args = vec![1, 2, 3];
        Arity {
            required: 0,
            most: Some(1),
        }
        .fit(&mut args, &0);
        assert_eq!(args, [1]);
        let mut args = vec![1, 2, 3];
        Arity {
            required: 1,
            most: None,
        }
        .fit(&mut args, &0);
        assert_eq!(args, [1, 2, 3]);
    }
}

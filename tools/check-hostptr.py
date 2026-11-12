#!/usr/bin/env python3
"""Host pointers and game addresses in the code ffx-host runs
(docs/host.md, "The arena": M5 stage 4's check "no code path builds a host
pointer from an arena address without the arena base").

  python3 -I tools/check-hostptr.py [-v] [--list]

In ffx-rust a game address is a u32 offset from the arena's base; a host
address is 64 bits and lies outside the arena. Two mistakes break that:

  host -> game  a host address (a pointer, `.as_ptr()`, `&raw`, a Box or
                Vec's address, `expose_provenance`, a reference cast to
                `*const`) narrowed to a game word (`as u32`, `as i32`,
                `u32::try_from`): handed to Mem or to game code it names
                some other byte of the arena, or nothing;
  game -> host  a game word made a host pointer without the base
                (`x as usize as *const T`, `with_exposed_provenance`,
                `as *mut` of an integer): it reads the host's own memory at
                that number.

Scanned: every crate in ffx-host's normal dependency tree (`cargo tree`),
`src/` only, and crates/ffx-hook/src/ported (compiled into ffx-host-ports
with `cfg(ffx_host)`). Code a `#[cfg(...)]` turns off in the 64-bit host
(`target_pointer_width = "32"`, `target_arch = "x86"`, `not(ffx_host)` in
ported/, `test`) is skipped: on the 32-bit hook a game address is the host
pointer. The `ported!` shims tools/host-ports.py holds (held.rs), and the
helpers that made them held, run their originals in ffx-host: their
findings count as held (WP6's work), not as problems.

Anything else is a problem unless tools/check-hostptr.allow lists it:
`path<TAB>text<TAB>reason`, the text a fragment of the finding's line
(whitespace collapsed). The exit status is 1 when a problem or a stale
allow line (one that matches nothing) is left. check-ported.py-style
gates run it with the other check-*.py.
"""
import argparse
import importlib.util
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
PORTED = ROOT / "crates/ffx-hook/src/ported"
ALLOW = ROOT / "tools/check-hostptr.allow"

_spec = importlib.util.spec_from_file_location("host_ports", ROOT / "tools/host-ports.py")
hp = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(hp)

# a pointer or host address in a cast's operand
POINTER = re.compile(
    r"\.as_(?:mut_)?ptr\(\)|&\s*raw\s+(?:const|mut)\b|\baddr_of(?:_mut)?!|\bBox::into_raw\b|"
    r"\bBox::leak\b|\.expose_provenance\(\)|\.addr\(\)|\bas\s+\*\s*(?:const|mut)\b|"
    r"\bptr::from_(?:ref|mut)\b|\.as_non_null_ptr\(\)|\bNonNull\b")
# an integer made a pointer
FROM_INT = re.compile(r"\bwith_exposed_provenance(?:_mut)?\s*\(|\bfrom_exposed_addr(?:_mut)?\s*\(|"
                      r"\bwithout_provenance(?:_mut)?\s*\(")
INT_CAST = re.compile(r"\bas\s+(?:usize|u64|isize|i64|u32)\b|\b(?:usize|u64)::from\s*\(")
NARROW = re.compile(r"\bas\s+(?:u32|i32)\b|\b(?:u32|i32)::try_from\s*\(")
TO_PTR = re.compile(r"\bas\s+\*\s*(?:const|mut)\b")


# ---- cfg ------------------------------------------------------------------

def cfg_value(expr, ffx_host):
    """True, False, or None (depends on the build: both count as on)."""
    expr = expr.strip()
    m = re.match(r"^(all|any|not)\s*\((.*)\)$", expr, re.S)
    if m:
        parts = split_args(m.group(2))
        vals = [cfg_value(p, ffx_host) for p in parts if p.strip()]
        if m.group(1) == "not":
            v = vals[0] if vals else None
            return None if v is None else not v
        if m.group(1) == "all":
            if any(v is False for v in vals):
                return False
            return None if any(v is None for v in vals) else True
        if any(v is True for v in vals):
            return True
        return None if any(v is None for v in vals) else False
    m = re.match(r'^(\w+)\s*=\s*"([^"]*)"$', expr)
    if m:
        key, val = m.groups()
        if key == "target_pointer_width":
            return val == "64"
        if key == "target_arch":
            return val == "x86_64"
        return None
    if expr == "test" or expr == "doc":
        return False
    if expr == "ffx_host":
        return ffx_host
    return None


def split_args(s):
    out, depth, cur = [], 0, []
    for c in s:
        if c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
        if c == "," and depth == 0:
            out.append("".join(cur))
            cur = []
        else:
            cur.append(c)
    out.append("".join(cur))
    return out


CFG = re.compile(r"#(!?)\[cfg\s*\(")


def item_end(code, at):
    """The end of the item (or statement, or expression) starting at `at`,
    past further attributes: its `{...}` block, or its `;`."""
    i = at
    while True:
        m = re.match(r"\s*#\s*!?\[", code[i:])
        if not m:
            break
        j = i + m.end() - 1
        depth = 0
        for k in range(j, len(code)):
            if code[k] == "[":
                depth += 1
            elif code[k] == "]":
                depth -= 1
                if depth == 0:
                    i = k + 1
                    break
        else:
            return len(code)
    depth = 0
    angle = 0  # generics (`Result<(), E>`): their commas don't end the item
    for k in range(i, len(code)):
        c = code[k]
        pair = code[k - 1:k + 1] if k else c
        after = code[k:k + 2]
        if c in "([":
            depth += 1
        elif c in ")]":
            depth -= 1
            if depth < 0:
                return k
        elif c == "<" and after not in ("<=", "<<") and pair != "<<":
            angle += 1
        elif c == ">" and pair not in ("->", "=>", ">>") and after not in (">=", ">>") and angle:
            angle -= 1
        elif c == "{" and depth == 0:
            return hp.block(code, k)[1]
        elif c == ";" and depth == 0:
            return k + 1
        elif c == "," and depth == 0 and angle == 0:
            return k + 1
    return len(code)


def blank_inactive(code, ffx_host):
    """`code` with what the 64-bit host doesn't compile blanked."""
    out = list(code)
    for m in CFG.finditer(code):
        depth, k = 1, m.end()
        while k < len(code) and depth:
            if code[k] == "(":
                depth += 1
            elif code[k] == ")":
                depth -= 1
            k += 1
        expr = code[m.end():k - 1]
        if cfg_value(expr, ffx_host) is not False:
            continue
        if m.group(1) == "!":  # an inner attribute: the whole file
            return re.sub(r"[^\n]", " ", code)
        close = code.index("]", k - 1) + 1
        end = item_end(code, close)
        for x in range(m.start(), end):
            if out[x] != "\n":
                out[x] = " "
    return "".join(out)


# ---- operands -------------------------------------------------------------

def match_open(code, j):
    """The index of the bracket opening the one closing at `j`."""
    pairs = {")": "(", "]": "[", ">": "<"}
    close, open_ = code[j], pairs[code[j]]
    depth = 0
    for k in range(j, -1, -1):
        if code[k] == close:
            depth += 1
        elif code[k] == open_:
            depth -= 1
            if depth == 0:
                return k
    return 0


def operand(code, p):
    """The operand of the cast whose `as` starts at `p` (casts chain:
    `x.as_ptr() as usize as u32` gives `x.as_ptr() as usize`)."""
    i = p
    while True:
        while i > 0 and code[i - 1].isspace():
            i -= 1
        j = i
        while j > 0:
            c = code[j - 1]
            if c in ")]":
                j = match_open(code, j - 1)
                continue
            if c == ">" and j > 1 and code[j - 2] != "-" and "<" in code[max(0, j - 80):j]:
                j = match_open(code, j - 1)
                continue
            if c.isalnum() or c in "_.:!'":
                j -= 1
                continue
            break
        # a pointer type in the chain (`as *const T as usize`)
        m = re.search(r"\*\s*(?:const|mut)\s+$", code[max(0, j - 12):j])
        if m:
            j -= len(m.group(0))
        # prefixes: `&raw const`, `&mut`, `&`, `*`, `-`
        m = re.search(r"(?:&\s*raw\s+(?:const|mut)|&\s*mut|&|\*|-)\s*$", code[max(0, j - 14):j])
        if m:
            j -= len(m.group(0))
        m = re.search(r"\bas\s+$", code[max(0, j - 8):j])
        if m:
            i = j - len(m.group(0))
            continue
        return code[j:p]


def call_arg(code, at):
    """The text of the call's argument list whose `(` ends at `at`."""
    depth = 1
    for k in range(at, len(code)):
        if code[k] == "(":
            depth += 1
        elif code[k] == ")":
            depth -= 1
            if depth == 0:
                return code[at:k]
    return code[at:]


LET_PTR = re.compile(r"\blet\s+(?:mut\s+)?(\w+)\s*(:\s*\*\s*(?:const|mut)\b[^=;]*)?=\s*([^;]*);")
PARAM_PTR = re.compile(r"\b(\w+)\s*:\s*\*\s*(?:const|mut)\b")
IDENT = re.compile(r"\b[a-z_][a-z0-9_]*\b")


def functions(code):
    """[(start, end, name)] of every `fn` with a body (signature included)."""
    out = []
    for m in hp.FN.finditer(code):
        try:
            b0, b1 = hp.block(code, m.end())
        except ValueError:
            continue
        semi = code.find(";", hp.params_end(code, m.end()))
        if 0 <= semi < b0:
            continue
        out.append((m.start(), b1, m.group(1)))
    return out


MAKES_PTR_START = re.compile(r"^(?:&\s*raw\s+(?:const|mut)\b|(?:std::|core::)?ptr::(?:from_ref|from_mut|addr_of)|"
                             r"addr_of(?:_mut)?!|Box::into_raw|Box::leak)")
MAKES_PTR_END = re.compile(r"(?:\.as_(?:mut_)?ptr\(\)|\bas\s+\*\s*(?:const|mut)\s+[\w:<>()\[\]; ]+|"
                           r"\.cast(?:_mut|_const)?(?:::<[^;]*>)?\(\))$")


def pointer_names(text):
    """Locals and parameters of one function that hold host pointers: a
    `*const`/`*mut` type, or a value that is one (its outermost expression,
    not a pointer handed to a call)."""
    names = {m.group(1) for m in PARAM_PTR.finditer(text)}
    for m in LET_PTR.finditer(text):
        rhs = m.group(3).strip()
        inner = re.fullmatch(r"unsafe\s*\{(.*)\}", rhs, re.S)
        if inner:
            rhs = inner.group(1).strip()
        if m.group(2) or MAKES_PTR_START.search(rhs) or MAKES_PTR_END.search(rhs):
            names.add(m.group(1))
    return names


def findings(code):
    """[(offset, kind, text)] in stripped, cfg-blanked `code`."""
    out = []
    fns = functions(code)
    names = {n for _, _, n in fns}

    def scope(at):
        inner = [f for f in fns if f[0] <= at < f[1]]
        return min(inner, key=lambda f: f[1] - f[0]) if inner else None

    cache = {}
    bound_cache = {}

    def bound(at):
        """Every name a `let`, a parameter or a closure binds in the
        function around `at`."""
        f = scope(at)
        if f is None:
            return set()
        if f not in bound_cache:
            text = code[f[0]:f[1]]
            bound_cache[f] = (set(re.findall(r"\blet\s+(?:mut\s+)?(\w+)", text))
                              | set(re.findall(r"\b(\w+)\s*:", text))
                              | set(re.findall(r"\|\s*(?:mut\s+)?(\w+)", text)))
        return bound_cache[f]

    def pointers(at):
        f = scope(at)
        if f is None:
            return set()
        if f not in cache:
            cache[f] = pointer_names(code[f[0]:f[1]])
        return cache[f]

    for m in NARROW.finditer(code):
        if m.group(0).startswith("as"):
            opnd = operand(code, m.start())
        else:
            opnd = call_arg(code, m.end())
        ids = set(IDENT.findall(opnd))
        fn_addr = re.fullmatch(r"\s*(\w+)\s+as\s+usize\s*", opnd)
        if POINTER.search(opnd) or ids & pointers(m.start()):
            out.append((m.start(), "host -> game", opnd.strip()))
        elif fn_addr and fn_addr.group(1) in names and fn_addr.group(1) not in bound(m.start()):
            # a Rust function's address (`f as usize as u32`)
            out.append((m.start(), "host -> game", opnd.strip()))
    for m in TO_PTR.finditer(code):
        opnd = operand(code, m.start()).strip()
        if INT_CAST.search(opnd) or re.fullmatch(r"\(?\s*(?:0x[0-9a-fA-F_]+|\d[\d_]*)\w*\s*\)?", opnd):
            out.append((m.start(), "game -> host", opnd))
        elif re.fullmatch(r"[a-z_]\w*", opnd) and opnd not in pointers(m.start()):
            # a plain variable that is no pointer here: an integer
            out.append((m.start(), "game -> host", opnd))
    for m in FROM_INT.finditer(code):
        out.append((m.start(), "game -> host", call_arg(code, m.end()).strip()))
    return out


# ---- what is scanned ------------------------------------------------------

def host_crates():
    """The workspace crates in ffx-host's normal dependency tree."""
    r = subprocess.run(["cargo", "tree", "-p", "ffx-host", "-e", "normal", "--prefix", "none",
                        "--offline", "-f", "{p}"], cwd=ROOT, capture_output=True, text=True, check=True,
                       env={**__import__("os").environ, "CARGO_NET_OFFLINE": "true"})
    dirs = set()
    for line in r.stdout.splitlines():
        m = re.search(r"\((/[^)]+)\)", line)
        if m:
            d = pathlib.Path(m.group(1))
            if ROOT in d.parents:
                dirs.add(d)
    return sorted(dirs)


def held_spans(path, code):
    """The spans of a ported/ file that tools/host-ports.py holds: the held
    shims' bodies and the helpers that taint them."""
    held, _ = hp.scan(path)
    held_names = {name for _, name, _ in held}
    spans = []
    for m in hp.ENTRY.finditer(code):
        if m.group(1) in held_names:
            spans.append(hp.block(code, m.end()))
    tainted = set()
    for _, _, why in held:
        tainted |= {w for w in why if re.fullmatch(r"[a-z_][a-z0-9_]*", w)}
    for m in hp.FN.finditer(code):
        if m.group(1) in tainted:
            try:
                spans.append(hp.block(code, m.end()))
            except ValueError:
                pass
    return spans


def line_of(text, at):
    s = text.rfind("\n", 0, at) + 1
    e = text.find("\n", at)
    return text.count("\n", 0, at) + 1, " ".join(text[s:e if e >= 0 else None].split())


def scan_file(path, ffx_host, ported):
    raw = path.read_text()
    code = blank_inactive(hp.strip(raw), ffx_host)
    spans = held_spans(path, code) if ported and path.name != "mod.rs" else []
    out = []
    for at, kind, text in findings(code):
        n, line = line_of(raw, at)
        held = any(a <= at < b for a, b in spans)
        out.append((path.relative_to(ROOT).as_posix(), n, kind, line, text, held))
    return out


def load_allow():
    rules = []
    if ALLOW.exists():
        for n, line in enumerate(ALLOW.read_text().splitlines(), 1):
            if not line.strip() or line.startswith("#"):
                continue
            parts = line.split("\t")
            if len(parts) < 3 or not parts[2].strip():
                sys.exit(f"{ALLOW.name}:{n}: want path<TAB>text<TAB>reason")
            rules.append((n, parts[0], " ".join(parts[1].split()), parts[2]))
    return rules


SELF_TEST = [
    # (snippet, ffx_host, how many findings)
    ("fn f(b: &mut [u8]) -> u32 { b.as_mut_ptr() as usize as u32 }", False, 1),
    ("fn f() -> u32 { let mut x = 0u32; std::ptr::from_mut(&mut x) as usize as u32 }", False, 1),
    ("fn f() -> u32 { let x = 0u32; u32::try_from(&raw const x as usize).unwrap_or(0) }", False, 1),
    ("fn f(b: &[u8]) -> u32 { let p = b.as_ptr(); p as u32 }", False, 1),
    ("fn f(p: *const u8) -> u32 { p as usize as u32 }", False, 1),
    ("fn cb() {} fn f() -> u32 { cb as usize as u32 }", False, 1),
    ("fn f(a: u32) -> u8 { unsafe { *(a as usize as *const u8) } }", False, 1),
    ("fn f(a: u32) -> *const u8 { a as *const u8 }", False, 1),
    ("fn f(a: usize) -> *const u8 { std::ptr::with_exposed_provenance(a) }", False, 1),
    # right: no pointer, the base added, a pointer cast to a pointer
    ("fn f(x: u64) -> u32 { x as u32 }", False, 0),
    ("fn f(base: *mut u8, a: u32) -> *mut u8 { unsafe { base.add(a as usize) } }", False, 0),
    ("fn f(p: *mut u32) -> *const u8 { p as *const u8 }", False, 0),
    ("fn f(r: &mut u32) { let ok = unsafe { g(&raw mut *r) }; let _ = ok as u32; }", False, 0),
    ("fn f() -> u32 { let pid = 3u64; pid as usize as u32 } fn pid() {}", False, 0),
    # cfg: off in the 64-bit host
    ('#[cfg(target_pointer_width = "32")]\nfn f(b: &[u8]) -> u32 { b.as_ptr() as u32 }', False, 0),
    ("#[cfg(not(ffx_host))]\nfn f(b: &[u8]) -> Result<(), E> { b.as_ptr() as u32; Ok(()) }", True, 0),
    ("#[cfg(not(ffx_host))]\nfn f(b: &[u8]) -> u32 { b.as_ptr() as u32 }", False, 1),
    ('#[cfg(all(unix, target_arch = "x86"))]\nfn f(b: &[u8]) -> u32 { b.as_ptr() as u32 }', False, 0),
    ("#[cfg(test)]\nmod tests { fn f(b: &[u8]) -> u32 { b.as_ptr() as u32 } }", False, 0),
    ("#[cfg(windows)]\nfn f(b: &[u8]) -> u32 { b.as_ptr() as u32 }", False, 1),
]


def self_test():
    bad = 0
    for snippet, ffx_host, want in SELF_TEST:
        got = findings(blank_inactive(hp.strip(snippet), ffx_host))
        if len(got) != want:
            bad += 1
            print(f"want {want}, got {len(got)} {got}: {snippet}")
    print(f"self-test: {len(SELF_TEST) - bad} of {len(SELF_TEST)} right")
    return 1 if bad else 0


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("-v", "--verbose", action="store_true", help="also the allowed and held findings")
    ap.add_argument("--list", action="store_true", help="the files scanned")
    ap.add_argument("--self-test", action="store_true", help="the patterns on fixed snippets")
    a = ap.parse_args()
    if a.self_test:
        return self_test()
    files = []
    for d in host_crates():
        for p in sorted((d / "src").rglob("*.rs")):
            files.append((p, False, False))
    for p in sorted(PORTED.glob("*.rs")):
        files.append((p, True, True))
    if a.list:
        for p, _, _ in files:
            print(p.relative_to(ROOT))
    rules = load_allow()
    used = set()
    problems, allowed, held = [], 0, 0
    for p, ffx_host, ported in files:
        for f in scan_file(p, ffx_host, ported):
            rel, n, kind, line, text, is_held = f
            rule = next((r for r in rules if r[1] == rel and r[2] in line), None)
            if rule:
                used.add(rule[0])
                allowed += 1
                if a.verbose:
                    print(f"allowed {rel}:{n}: {kind}: {line}  [{rule[3]}]")
            elif is_held:
                held += 1
                if a.verbose:
                    print(f"held    {rel}:{n}: {kind}: {line}")
            else:
                problems.append(f)
    for rel, n, kind, line, text, _ in problems:
        print(f"{rel}:{n}: {kind}: `{text}` in: {line}")
    stale = [r for r in rules if r[0] not in used]
    for n, rel, text, _ in stale:
        print(f"{ALLOW.name}:{n}: matches nothing: {rel}  {text}")
    print(f"check-hostptr: {len(problems)} problems, {len(stale)} stale allow lines "
          f"({len(files)} files; {allowed} allowed, {held} in held shims)")
    return 1 if problems or stale else 0


if __name__ == "__main__":
    sys.exit(main())

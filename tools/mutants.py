#!/usr/bin/env python3
"""mutants.py <worktree> <mutants> <file>...: mutation testing with one build for all mutants.

<mutants> is a Python or JSON list of (name, old, new) or [old, new] entries
(the second form is named #1, #2, ...). Each `old` is looked up in the given
files (paths relative to the worktree). It must occur exactly once across all
of them, either verbatim or, failing that, with any whitespace between its
tokens (survives cargo fmt reflows). If it occurs nowhere or in more than one
place, the mutant is NOMATCH: an ambiguous text would mutate a look-alike
instead of the intended site.

The worktree is synced (rsync) into a private copy, MUT_WORK (default
~/ffx-work/mutants/<worktree name>), so the worktree itself is never edited.
There each mutation site becomes a schema,
`if ffx_core::mutant(k) { new } else { old }` (in parentheses for an
expression, in braces for a match arm's body), around the smallest expression
or statement run holding the change (found by tools/mutants-rs, a syn helper
built on first use). The ffx-oracle test binary is built once with
`--features ffx-core/mutants`. A schema the compiler rejects (an error span
inside it) moves to the next larger node, and after a few rounds to the old
rebuild-per-mutant path, so no mutant is lost; errors whose spans point into
no schema are traced to their schemata by building halves, and only those
are dropped to the rebuild path. A mutant that does not build on its own is an
ERROR. Mutants then run in parallel (MUT_JOBS, default cores / 2),
each with FFX_MUTANT=k on the image and every snapshot in MUT_SNAPS.

Settings (environment, as in the agents' mut22.py; flags override):
  MUT_TEST    oracle test file (required), e.g. battle_cue
  MUT_FILTER  test-name filter (default none)
  MUT_PASSED  tests that must pass per run (default: what the unmutated run passes)
  MUT_SNAPS   snapshots after the image, comma list (default battle,field; "" = image only)
  MUT_JOBS    parallel runs (default cores / 2)
  MUT_TIMEOUT seconds per run (default max(120, 20 x the unmutated run))
  MUT_SCHEMA  0 = no schemata, every mutant rebuilds (the old way)

`mutants.py --self-test` checks the schema rewrite on a small built-in file
(match arms, overlapping deletions, a tail insertion, a const) with rustc
alone: one build, and every schema behaves as its mutant rebuilt.

Verdicts, one line per mutant in list order: KILLED, SURVIVED, ERROR or
NOMATCH, a tab, the name. A run with a failed test, or one that times out (a
hang is a behaviour change), kills the mutant; a run with no "test result" or
fewer passed tests than required is an ERROR. The unmutated binary runs first
on the image and each snapshot and must pass, or nothing runs. Progress and a
summary go to stderr.
"""

import argparse
import ast
import fcntl
import json
import os
import pathlib
import re
import shutil
import signal
import subprocess
import sys
import threading
import time
from concurrent.futures import ThreadPoolExecutor

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import timing_log  # noqa: E402

timing_log.install()

TOOLS = pathlib.Path(__file__).resolve().parent
HOME = pathlib.Path.home()
SNAPSHOTS = HOME / "ffx-work" / "snapshots"
MAX_ROUNDS = 8  # schema builds before the remaining rejects fall back
LIVE = set()  # running child processes, killed on interrupt
LIVE_LOCK = threading.Lock()


def log(*a):
    print(*a, file=sys.stderr, flush=True)


def load_mutants(path):
    text = pathlib.Path(path).read_text()
    try:
        data = json.loads(text)
    except ValueError:
        data = ast.literal_eval(text)
    out = []
    for i, m in enumerate(data):
        if len(m) == 3:
            out.append(tuple(m))
        elif len(m) == 2:
            out.append((f"#{i + 1}", m[0], m[1]))
        else:
            sys.exit(f"mutant {i + 1}: expected (name, old, new) or [old, new]")
    names = [m[0] for m in out]
    if len(set(names)) != len(names):
        sys.exit("mutant names must be unique")
    return out


def flex(a):
    toks = re.findall(r"\w+|[^\w\s]", a)
    return re.compile(r"\s*".join(re.escape(t) for t in toks))


def locate(a, texts):
    """(file, start, end) of the one place `a` matches, or None.

    The match must be unique across all the given files: an exact match
    first, else one where any whitespace may separate the tokens. A text
    that matches in more than one place is ambiguous and is not mutated (a
    look-alike elsewhere would be tested instead of the intended site)."""
    exact = [(f, s) for f, t in texts.items() for s in _all(t, a)]
    if len(exact) == 1:
        f, s = exact[0]
        return f, s, s + len(a)
    if exact:
        return None
    hits = [(f, m) for f, t in texts.items() for m in flex(a).finditer(t)]
    if len(hits) == 1:
        f, m = hits[0]
        return f, m.start(), m.end()
    return None


def _all(text, a):
    """Every start of `a` in `text`, overlapping ones included."""
    out, s = [], text.find(a)
    while s != -1:
        out.append(s)
        s = text.find(a, s + 1)
    return out


def site(text, s, e, b):
    """The minimal changed range of text[s:e] -> b: (start, end, new)."""
    a = text[s:e]
    p = 0
    while p < min(len(a), len(b)) and a[p] == b[p]:
        p += 1
    q = 0
    while q < min(len(a), len(b)) - p and a[len(a) - 1 - q] == b[len(b) - 1 - q]:
        q += 1
    return s + p, e - q, b[p : len(b) - q]


def run(cmd, timeout=None, **kw):
    """subprocess.run that the interrupt handler can kill; None on timeout."""
    p = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, **kw)
    with LIVE_LOCK:
        LIVE.add(p)
    try:
        out, _ = p.communicate(timeout=timeout)
        return p.returncode, out
    except subprocess.TimeoutExpired:
        p.kill()
        p.communicate()
        return None, ""
    finally:
        with LIVE_LOCK:
            LIVE.discard(p)


class Runner:
    def __init__(self, args):
        self.args = args
        self.wt = pathlib.Path(args.worktree).expanduser().resolve()
        self.work = pathlib.Path(args.work).expanduser()
        self.tree = self.work / "tree"
        self.target = self.work / "target"
        self.files = args.files
        cores = os.cpu_count() or 2
        self.jobs = args.jobs or max(1, cores // 2)
        self.threads = max(1, cores // self.jobs)
        self.snaps = [""] + [s for s in args.snaps.split(",") if s]
        self.env = dict(os.environ, CARGO_TARGET_DIR=str(self.target),
                        FFX_GAME_DIR=os.environ.get("FFX_GAME_DIR", str(HOME / "steam-games/FFX-X2-HD")))
        self.env["PATH"] = f"{HOME}/ffx-work/field/bin:{self.env.get('PATH', '')}"  # cmake shim
        self.env.pop("FFX_MUTANT", None)
        self.want = {}
        self.timeout = {}

    # -- tree -------------------------------------------------------------
    def sync(self):
        self.work.mkdir(parents=True, exist_ok=True)
        skip = ["/target", "/.git", "/.claude", "/app/src-tauri/target", "/app/ui/node_modules"]
        r = subprocess.run(["rsync", "-a", "--delete", *(f"--exclude={s}" for s in skip),
                            f"{self.wt}/", f"{self.tree}/"])
        if r.returncode:
            sys.exit("rsync failed")

    def need_core(self, rel):
        """Make the crate holding `rel` depend on ffx-core (in the copy only)."""
        parts = pathlib.PurePath(rel).parts
        if len(parts) < 2 or parts[0] != "crates" or parts[1] == "ffx-core":
            return
        toml = self.tree / "crates" / parts[1] / "Cargo.toml"
        t = toml.read_text()
        if re.search(r"^ffx-core\b", t, re.M):
            return
        t2 = re.sub(r"^\[dependencies\]\n", "[dependencies]\nffx-core.workspace = true\n", t, count=1, flags=re.M)
        if t2 == t:
            t2 = t + "\n[dependencies]\nffx-core.workspace = true\n"
        toml.write_text(t2)

    # -- build ------------------------------------------------------------
    def build(self):
        """cargo build of the test binary: (executable or None, [(file, byte, msg)])."""
        cmd = ["cargo", "test", "--release", "-p", "ffx-oracle", "--features", "ffx-core/mutants",
               "--test", self.args.test, "--no-run", "--message-format=json"]
        code, out = run(cmd, cwd=self.tree, env=self.env)
        exe, errors = None, []
        for line in out.splitlines():
            if not line.startswith("{"):
                continue
            try:
                msg = json.loads(line)
            except ValueError:
                continue
            if msg.get("reason") == "compiler-artifact" and msg["target"]["name"] == self.args.test \
                    and msg["profile"]["test"] and msg.get("executable"):
                exe = msg["executable"]
            elif msg.get("reason") == "compiler-message" and msg["message"]["level"] == "error":
                spans = [s for s in msg["message"]["spans"] if s["is_primary"]] or msg["message"]["spans"]
                if not spans:
                    errors.append((None, None, msg["message"]["message"]))
                for s in spans:
                    f = pathlib.Path(s["file_name"])
                    if f.is_absolute():
                        try:
                            f = f.relative_to(self.tree)
                        except ValueError:
                            pass
                    errors.append((str(f), s["byte_start"], msg["message"]["message"]))
        if code != 0 and not errors:
            errors.append((None, None, out[-2000:]))
        return (exe if code == 0 else None), errors

    # -- running ------------------------------------------------------------
    def run_tests(self, exe, snap, mutant=None):
        """(verdict, passed, seconds): verdict ok / killed / ERROR, as in mut22.py."""
        env = dict(self.env)
        if snap:
            env["FFX_SNAPSHOT"] = str(SNAPSHOTS / f"{snap}.snap")
        else:
            env.pop("FFX_SNAPSHOT", None)
        if mutant:
            env["FFX_MUTANT"] = str(mutant)
        cmd = [exe, f"--test-threads={self.threads}"] + ([self.args.filter] if self.args.filter else [])
        t0 = time.time()
        code, out = run(cmd, timeout=self.timeout.get(snap), cwd=self.tree / "crates/ffx-oracle", env=env)
        dt = time.time() - t0
        if code is None:
            return "killed", 0, dt  # a hang is a behaviour change
        m = re.search(r"test result: (\w+)\. (\d+) passed; (\d+) failed", out)
        if not m:
            return "ERROR", 0, dt
        if int(m.group(3)) > 0:
            return "killed", int(m.group(2)), dt
        return ("ok" if int(m.group(2)) >= self.want.get(snap, 1) else "ERROR"), int(m.group(2)), dt

    def verdict(self, exe, mutant=None):
        for snap in self.snaps:
            v, _, _ = self.run_tests(exe, snap, mutant)
            if v != "ok":
                return "KILLED" if v == "killed" else "ERROR"
        return "SURVIVED"

    def baseline(self, exe):
        for snap in self.snaps:
            self.want[snap] = 0
            v, passed, dt = self.run_tests(exe, snap)
            want = self.args.passed if self.args.passed is not None else max(passed, 1)
            if v != "ok" or passed < want:
                sys.exit(f"unmutated run on {snap or 'image'}: {v}, {passed} passed (need {want}); "
                         "fix the filter or the tests first")
            self.want[snap] = want
            self.timeout[snap] = self.args.timeout or max(120.0, 20 * dt)
            log(f"baseline {snap or 'image'}: {passed} passed in {dt:.1f}s (timeout {self.timeout[snap]:.0f}s)")


class Schemata:
    """The schema rewrite of one file."""

    def __init__(self, rel, text, core_path):
        self.rel, self.text, self.path = rel, text, core_path
        self.sites = {}  # mutant id -> (start, end, new)
        self.cands = {}  # mutant id -> candidates from the helper
        self.pick = {}  # mutant id -> index into cands
        self.ranges = []  # (out start, out end, ids, b ranges {id: (s, e)}) per wrapper

    def chosen(self, k):
        return self.cands[k][self.pick[k]]

    def advance(self, k):
        self.pick[k] += 1
        return self.pick[k] < len(self.cands[k])

    def resolve(self, active):
        """Drop to larger nodes until every pair nests or is apart; returns the ids that ran out."""
        out = []
        accepted = []
        for k in active:
            while True:
                c = self.chosen(k)
                ok = True
                for j in accepted:
                    d = self.chosen(j)
                    same = (c["start"], c["end"]) == (d["start"], d["end"])
                    if c["start"] == c["end"] or d["start"] == d["end"]:
                        # an empty span (an insertion) must not touch another
                        ok = c["end"] < d["start"] or d["end"] < c["start"]
                    else:
                        apart = c["end"] <= d["start"] or d["end"] <= c["start"]
                        nest = (c["start"] <= d["start"] and d["end"] <= c["end"]) or \
                               (d["start"] <= c["start"] and c["end"] <= d["end"])
                        ok = (apart or nest) and not (same and c["kind"] != d["kind"])
                    if not ok:
                        break
                if ok:
                    accepted.append(k)
                    break
                if not self.advance(k):
                    out.append(k)
                    break
        return accepted, out

    def render(self, ids):
        self.ranges = []
        items = sorted(ids, key=lambda k: (self.chosen(k)["start"], -self.chosen(k)["end"], k))
        text, ranges = self._render(0, len(self.text), items)
        self.ranges = ranges
        return text

    def _render(self, lo, hi, items):
        pieces, ranges, pos, at = [], [], lo, 0
        i = 0
        while i < len(items):
            c = self.chosen(items[i])
            s, e = c["start"], c["end"]
            group = [k for k in items[i:] if (self.chosen(k)["start"], self.chosen(k)["end"]) == (s, e)]
            inner = [k for k in items[i + len(group):] if self.chosen(k)["end"] <= e
                     and (self.chosen(k)["start"], self.chosen(k)["end"]) != (s, e)]
            pieces.append(self.text[pos:s])
            at += len(self.text[pos:s])
            a_text, a_ranges = self._render(s, e, inner)
            # an expression goes in parentheses; a match arm's body in braces
            # (a parenthesised block would need a comma after the arm)
            open_, close = {"expr": ("(", ")"), "arm": ("{", "}")}.get(c["kind"], ("", ""))
            w = open_
            b_ranges = {}
            for n, k in enumerate(group):
                w += ("if " if n == 0 else " else if ") + f"{self.path}({k}) {{\n"
                b_ranges[k] = (at + len(w), at + len(w) + len(self.chosen(k)["b"]))
                w += self.chosen(k)["b"] + "\n}"
            w += " else {\n"
            a_at = at + len(w)
            w += a_text + "\n}" + close
            ranges.append((at, at + len(w), group, b_ranges))
            ranges += [(rs + a_at, re_ + a_at, g, {k: (x + a_at, y + a_at) for k, (x, y) in br.items()})
                       for rs, re_, g, br in a_ranges]
            pieces.append(w)
            at += len(w)
            pos = e
            i += len(group) + len(inner)
        pieces.append(self.text[pos:hi])
        return "".join(pieces), ranges

    def blame(self, byte):
        """The mutants an error at output byte `byte` points at, or None."""
        best = None
        for s, e, group, b_ranges in self.ranges:
            if s <= byte < e and (best is None or e - s < best[1] - best[0]):
                best = (s, e, group, b_ranges)
        if best is None:
            return None
        for k, (s, e) in best[3].items():
            if s <= byte <= e:
                return [k]
        return list(best[2])


def helper():
    exe_dir = HOME / "ffx-work" / "mutants" / "helper-target"
    exe = exe_dir / "release" / "mutants-rs"
    src = TOOLS / "mutants-rs"
    newest = max(p.stat().st_mtime for p in [*src.rglob("*.rs"), src / "Cargo.toml"])
    if not exe.exists() or exe.stat().st_mtime < newest:
        log("building tools/mutants-rs")
        r = subprocess.run(["cargo", "build", "--release", "-q", "--manifest-path", str(src / "Cargo.toml")],
                           env=dict(os.environ, CARGO_TARGET_DIR=str(exe_dir)))
        if r.returncode:
            sys.exit("tools/mutants-rs did not build")
    return exe


def make_schemata(hx, rel, text, sites, core):
    """(Schemata of `rel`, ids without a node) for {id: (start, end, new)}; (None, ids) if the helper fails."""
    ks = list(sites)
    sc = Schemata(rel, text, core)
    req = [{"start": len(text[: sites[k][0]].encode()), "end": len(text[: sites[k][1]].encode()),
            "new": sites[k][2]} for k in ks]
    out = subprocess.run([str(hx)], input=json.dumps({"src": text, "sites": req}), capture_output=True, text=True)
    if out.returncode:
        log(f"{rel}: mutants-rs failed: {out.stderr.strip()}")
        return None, ks
    btext = text.encode()
    none = []
    for k, cands in zip(ks, json.loads(out.stdout)):
        # helper offsets are bytes; the schema works on str offsets
        for c in cands if len(btext) != len(text) else []:
            c["start"] = len(btext[: c["start"]].decode())
            c["end"] = len(btext[: c["end"]].decode())
        if cands:
            sc.cands[k], sc.pick[k] = cands, 0
        else:
            none.append(k)
    return sc, none


SELF_TEST_SRC = """\
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};

static G: AtomicU32 = AtomicU32::new(0);
const LIMIT: u32 = 4;

pub fn mutant(k: u32) -> bool {
    std::env::var("M").ok().and_then(|v| v.parse().ok()) == Some(k)
}

fn go(x: u32) {
    G.fetch_add(x, Relaxed);
}

fn call(x: u32) -> u32 {
    x * 0x101
}

pub fn step(state: u32, done: &mut u32) -> u32 {
    match state {
        0 => {
            go(1);
            *done = 1;
        }
        5 => *done = 1,
        6 => go(2),
        _ if state > LIMIT => {}
        _ => return 9,
    }
    call(3)
}

fn main() {
    let mut out = Vec::new();
    for s in [0, 5, 6, 7, 1] {
        let mut d = 0;
        let r = step(s, &mut d);
        out.push((r, d, G.load(Relaxed)));
    }
    println!("{out:?}");
}
"""

# (name, old, new); "const" has no node (mutant() is not a const fn) and rebuilds
SELF_TEST_MUTANTS = [
    ("arm-unbraced", "5 => *done = 1,", "5 => go(5),"),
    ("arm-literal", "5 => *done = 1,", "5 => *done = 2,"),
    ("arm-call", "6 => go(2),", "6 => go(3),"),
    ("arm-del-go", "go(1);\n            *done = 1;", "*done = 1;"),  # overlaps the next one in whitespace
    ("arm-del-done", "go(1);\n            *done = 1;", "go(1);"),
    ("tail-append", "call(3)\n}", "call(3) & 0xff\n}"),  # an insertion right after a tail expression
    ("guard", "_ if state > LIMIT => {}", "_ if state >= LIMIT => {}"),
    ("const", "const LIMIT: u32 = 4;", "const LIMIT: u32 = 5;"),
]


def self_test():
    """Schemata of match arms, overlapping deletions and tail insertions must build in one pass and
    behave as the mutants rebuilt one by one (needs only rustc)."""
    import tempfile

    hx = helper()
    text = SELF_TEST_SRC
    sites = {}
    for k, (name, a, b) in enumerate(SELF_TEST_MUTANTS, 1):
        assert text.count(a) == 1, name
        s = text.index(a)
        sites[k] = site(text, s, s + len(a), b)
    sc, none = make_schemata(hx, "self-test.rs", text, sites, "crate::mutant")
    fails = []
    want_none = [k for k, m in enumerate(SELF_TEST_MUTANTS, 1) if m[0] == "const"]
    if sc is None or none != want_none:
        fails.append(f"without a node: {none}, expected {want_none}")
    acc, gone = sc.resolve(sorted(sc.cands))
    if gone:
        fails.append(f"no compatible node: {gone}")
    with tempfile.TemporaryDirectory(prefix="mutants-self-test-") as d:
        d = pathlib.Path(d)

        def compile_(src, name):
            (d / f"{name}.rs").write_text(src)
            r = subprocess.run(["rustc", "--edition", "2024", "-A", "warnings", "-o", str(d / name),
                                str(d / f"{name}.rs")], capture_output=True, text=True)
            return (d / name) if r.returncode == 0 else r.stderr

        exe = compile_(sc.render(acc), "schema")
        if not isinstance(exe, pathlib.Path):
            fails.append("the schema build failed:\n" + exe[:3000])
        else:
            for k in [0, *acc]:
                s, e, new = sites[k] if k else (0, 0, "")
                ref = compile_(text[:s] + new + text[e:], f"m{k}")
                if not isinstance(ref, pathlib.Path):
                    fails.append(f"mutant {k} does not build alone:\n{ref[:1000]}")
                    continue
                env = dict(os.environ, M=str(k))
                got = subprocess.run([str(exe)], env=env, capture_output=True, text=True).stdout
                want = subprocess.run([str(ref)], capture_output=True, text=True).stdout
                name = SELF_TEST_MUTANTS[k - 1][0] if k else "unmutated"
                log(f"{'ok' if got == want else 'DIFFERS'}\t{name} ({sc.chosen(k)['kind'] if k else '-'})")
                if got != want:
                    fails.append(f"{name}: schema prints {got.strip()}, rebuilt prints {want.strip()}")
    for f in fails:
        log(f)
    log("self-test " + ("failed" if fails else f"passed: {len(acc)} schemata in one build, {len(none)} without a node"))
    sys.exit(1 if fails else 0)


def main():
    if sys.argv[1:] == ["--self-test"]:
        self_test()
    env = os.environ
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("worktree")
    ap.add_argument("mutants")
    ap.add_argument("files", nargs="+")
    ap.add_argument("--test", default=env.get("MUT_TEST"))
    ap.add_argument("--filter", default=env.get("MUT_FILTER", ""))
    ap.add_argument("--passed", type=int, default=int(env["MUT_PASSED"]) if env.get("MUT_PASSED") else None)
    ap.add_argument("--snaps", default=env.get("MUT_SNAPS", "battle,field"))
    ap.add_argument("--jobs", type=int, default=int(env.get("MUT_JOBS", "0")))
    ap.add_argument("--timeout", type=float, default=float(env.get("MUT_TIMEOUT", "0")))
    ap.add_argument("--no-schema", action="store_true", default=env.get("MUT_SCHEMA") == "0")
    ap.add_argument("--work", default=env.get("MUT_WORK"))
    ap.add_argument("--dry-run", action="store_true", help="print the schema diff, build nothing")
    args = ap.parse_args()
    if not args.test:
        sys.exit("MUT_TEST (or --test) names the oracle test file")
    if not args.work:
        args.work = str(HOME / "ffx-work" / "mutants" / pathlib.Path(args.worktree).expanduser().resolve().name)
    t_start = time.time()
    muts = load_mutants(args.mutants)
    r = Runner(args)
    r.work.mkdir(parents=True, exist_ok=True)
    lock = open(r.work / "lock", "w")
    try:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except OSError:
        sys.exit(f"{r.work} is in use by another run; set MUT_WORK to a different directory")

    def stop(signum, _frame):
        with LIVE_LOCK:
            for p in LIVE:
                p.kill()
        timing_log.finish(128 + signum)
        os._exit(128 + signum)

    for sig in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(sig, stop)

    r.sync()
    orig = {f: (r.tree / f).read_text() for f in r.files}
    where = {}
    for k, (name, a, b) in enumerate(muts, 1):
        loc = locate(a, orig)
        if loc:
            f, s, e = loc
            where[k] = (f, site(orig[f], s, e, b))

    # schemata per file
    schema = {}
    fallback = []
    if args.no_schema:
        fallback = sorted(where)
    else:
        hx = helper()
        for f in r.files:
            ks = [k for k in where if where[k][0] == f]
            if not ks:
                continue
            core = "crate::mutant" if pathlib.PurePath(f).parts[:2] == ("crates", "ffx-core") else "::ffx_core::mutant"
            sc, none = make_schemata(hx, f, orig[f], {k: where[k][1] for k in ks}, core)
            if sc is None:
                fallback += ks
                continue
            fallback += none
            schema[f] = sc
            r.need_core(f)

    exe = None
    active = {f: sorted(sc.cands) for f, sc in schema.items()}

    def attempt(act):
        """Write the schemata of `act` ({file: ids}) into the copy and build."""
        for f, sc in schema.items():
            (r.tree / f).write_text(sc.render(act.get(f, [])))
        return r.build()

    def attribute(errors):
        """(ids whose schema an error points into, errors no schema explains)."""
        blamed, loose = set(), []
        for f, byte, msg in errors:
            sc = schema.get(f)
            ks = None
            if sc and byte is not None:
                ks = sc.blame(len((r.tree / f).read_bytes()[:byte].decode(errors="replace")))
            if ks:
                blamed.update(ks)
            else:
                loose.append((f, msg))
        return blamed, loose

    def culprits(cands, base, dirty=False):
        """The (file, id) in `cands` whose schemata, built with `base`, give errors outside every schema.

        Bisects with builds of subsets (`dirty`: this set is known to give
        them); attributed errors in those builds are left to the rounds below."""
        if not cands:
            return []
        if dirty:
            return halves(cands, base)
        act = {}
        for f, k in base + cands:
            act.setdefault(f, []).append(k)
        t0 = time.time()
        exe_s, errs = attempt(act)
        loose = [] if exe_s else attribute(errs)[1]
        log(f"  bisect {len(cands)} of {len(base) + len(cands)}: "
            f"{'clean' if not loose else f'{len(loose)} unattributed'}, {time.time() - t0:.0f}s")
        if not loose:
            return []
        return halves(cands, base)

    def halves(cands, base):
        if len(cands) == 1:
            return cands
        half = len(cands) // 2
        bad = culprits(cands[:half], base)
        # the whole set is dirty: with nothing to blame in the first half, the second half is too
        return bad + culprits(cands[half:], base + [c for c in cands[:half] if c not in bad], dirty=not bad)

    rnd = 0
    while any(active.values()) and rnd <= MAX_ROUNDS:
        last = rnd >= MAX_ROUNDS - 1
        for f, sc in schema.items():
            acc, gone = sc.resolve(active[f])
            active[f] = acc
            fallback += gone
        if args.dry_run:
            for f, sc in schema.items():
                (r.tree / f).write_text(sc.render(active[f]))
                subprocess.run(["diff", "-u", str(r.wt / f), str(r.tree / f)])
            log(f"dry run: {sum(map(len, active.values()))} schemata, {len(fallback)} without a node")
            return
        t0 = time.time()
        exe, errors = attempt(active)
        rnd += 1
        log(f"schema build {rnd}: {sum(map(len, active.values()))} mutants, "
            f"{len(errors)} errors, {time.time() - t0:.0f}s")
        if exe:
            break
        blamed, loose = attribute(errors)
        if loose:
            # no span points into a schema (a cascade, or a cause elsewhere):
            # find the schemata behind it by building halves, drop only those
            for f, msg in loose[:5]:
                log(f"unattributed error ({f}): {msg[:300]}")
            bad = culprits([(f, k) for f in active for k in active[f]], [], dirty=True)
            log("dropped from the schema build: " + " ".join(muts[k - 1][0] for _, k in bad))
            for f, k in bad:
                active[f].remove(k)
                fallback.append(k)
        for f, sc in schema.items():
            keep = []
            for k in active[f]:
                if k not in blamed:
                    keep.append(k)
                elif not last and sc.advance(k):
                    keep.append(k)  # try the next larger node
                else:
                    fallback.append(k)
            active[f] = keep
    if not exe:
        # nothing left as a schema (or the rounds ran out): build the plain tree for the baseline
        for f in schema:
            fallback += active[f]
            active[f] = []
            (r.tree / f).write_text(orig[f])
        exe, errors = r.build()
        if not exe:
            sys.exit("the unmutated tree does not build:\n" + "\n".join(str(e) for e in errors[:5]))
    bin_dir = r.work / "bin"
    bin_dir.mkdir(exist_ok=True)
    schema_exe = str(bin_dir / f"{args.test}-schema")
    shutil.copy2(exe, schema_exe)
    r.baseline(schema_exe)
    in_schema = sorted(k for f in active for k in active[f])
    fallback = sorted(set(fallback))
    log(f"{len(in_schema)} mutants as schemata, {len(fallback)} rebuild per mutant, "
        f"{len(muts) - len(where)} unmatched; {r.jobs} jobs")
    if fallback:
        log("rebuilt per mutant: " + " ".join(muts[k - 1][0] for k in fallback))

    # results, printed in list order as they arrive
    results = {k: "NOMATCH" for k in range(1, len(muts) + 1) if k not in where}
    plock = threading.Lock()
    nxt = [1]

    def done(k, v):
        with plock:
            results[k] = v
            while nxt[0] in results:
                print(f"{results[nxt[0]]}\t{muts[nxt[0] - 1][0]}", flush=True)
                nxt[0] += 1

    with plock:
        while nxt[0] in results:
            print(f"{results[nxt[0]]}\t{muts[nxt[0] - 1][0]}", flush=True)
            nxt[0] += 1

    def guarded(k, fn):
        try:
            v = fn()
        except Exception as e:  # noqa: BLE001 - a crashed run is an ERROR, never a lost line
            log(f"{muts[k - 1][0]}: {e!r}")
            v = "ERROR"
        done(k, v)

    def rebuilt(k):
        f, (s, e, new) = where[k]
        (r.tree / f).write_text(orig[f][:s] + new + orig[f][e:])
        exe_k, _ = r.build()
        (r.tree / f).write_text(orig[f])
        return r.verdict(exe_k) if exe_k else "ERROR"

    lane = threading.Thread(target=lambda: [guarded(k, lambda k=k: rebuilt(k)) for k in fallback])
    lane.start()
    with ThreadPoolExecutor(r.jobs) as pool:
        for k in in_schema:
            pool.submit(guarded, k, lambda k=k: r.verdict(schema_exe, k))
    lane.join()
    counts = {}
    for v in results.values():
        counts[v] = counts.get(v, 0) + 1
    log(f"done in {time.time() - t_start:.0f}s: " + ", ".join(f"{n} {v}" for v, n in sorted(counts.items()))
        + f"; {len(fallback)} rebuilt")


if __name__ == "__main__":
    main()

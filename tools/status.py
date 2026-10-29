#!/usr/bin/env python3
"""Writes the status page: what is going on now, how fast, where the time
goes, and when each phase is likely done. Agents run it through
tools/status.sh, which returns at once.

The porting numbers are tools/progress.py's, taken from a branch (main by
default) through git rather than from the checkout, so every worktree writes
the same page. The history replays the branch's first-parent commits; each
commit's counts are cached by the files they come from, so a run only
computes the commits it has not seen.

Output ($FFX_WORK/status): index.html (tools/status.html), data.js (the
numbers), live.js (the "now" part; --live writes only that, in well under a
second) and history-<N>.json (the cache, one per count of functions N: a
checkout's symbols decide N, and worktrees on other commits would otherwise
throw each other's cache away; the newest few are kept). The open page
reloads both scripts
every 30 s; `tools/status.sh --watch` keeps live.js fresh.

Read from the file system, not git (missing or broken, the section says so):

- $FFX_WORK/agents/phases.json (--phases): the milestones and the phases
  the forecasts are for. Each phase is one of
    {"id", "stage", "title", "kind": "lists", "lists": [glob], "exclude": [glob]}
        the addresses in the files (paths relative to $FFX_WORK; the first
        0x... word of each line), less functions under 5 bytes. Done is
        hooked on the branch, or listed in symbols/m4-not-portable.tsv,
        m5-not-portable.tsv or m5-dropped.tsv;
    {"id", "stage", "title", "kind": "magic", "dir": "m5/magic"}
        tools/magic-worklist.py's runs: units left in each worklistN.tsv,
        DLLs covered in the newest worklistN-dlls.tsv;
    {"id", "stage", "title", "kind": "checklist",
     "items": [{"title", "state": "todo|running|verifying|done", "note"}]}
        kept by hand.
  "milestones": [{"id", "title", "state", "started", "finished"}].
- $FFX_WORK/agents/tasks.json (--tasks): the coordinator's task list:
    {"updated", "phase", "round", "next", "queued": [str],
     "tasks": [{"id", "title", "branch", "state", "started", "finished",
                "result"}],          # state is free text: running, paused,
                                     # done (merge pending), merged
                                     # (verifying), pushed, ... (task_state())
     "history": [{"round", "finished", "result"}]}
- the worktrees of the repository and the processes running in them
  (/proc), for which agents are busy and with what;
- $FFX_WORK/timing/*.tsv (docs/workflow.md, "Build and test timing"), for
  where the machine's time goes; $FFX_WORK/agents/metrics.tsv for finished
  agent runs (functions, tokens, minutes); the agents' Claude Code
  transcripts for where their own time goes (model, thinking, tools;
  transcripts(), read incrementally, offsets in transcripts.json). Both
  over the last 1, 3 and 24 h and the 24 h before (windows());
- $FFX_WORK/agents/pc2-<sha>.log, the second machine's replay runs of a
  merge, for what main has been verified at.

Usage: tools/status.py [--ref main] [--out DIR] [--tasks FILE] [--phases FILE] [--live]
"""
import argparse
import datetime
import fnmatch
import glob
import importlib.util
import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
import time

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import progress  # noqa: E402

REPO = progress.REPO
WORK = progress.WORK
# a commit's counts change only when one of these does
SOURCES = ["symbols/ffx.sym", "symbols/classes.tsv", "crates/ffx-hook/src/lib.rs", "crates/ffx-hook/src/ported",
           "symbols/hook-hits.txt"]
CACHE_VERSION = 4
STATES = ["library", "unnamed", "named", "wrapped", "replaced"]
# listed there, a function counts as done for a phase: not ported on purpose
RESOLVED_FILES = ["symbols/m4-not-portable.tsv", "symbols/m5-not-portable.tsv", "symbols/m5-dropped.tsv"]
# the forecast's paces: functions per day over the last N hours
PACES = [6, 24, 72]
# the timing log's top-level commands, by what they are for
TIME_GROUPS = [
    ("Parity tests", lambda k: k == "cargo test"),
    ("Clippy", lambda k: k in ("cargo clippy", "cargo xwin clippy")),
    ("32-bit builds", lambda k: k.startswith("cargo xwin")),
    ("Host builds", lambda k: k.startswith("cargo")),
    ("Replays", lambda k: k.startswith("run-all")),
    ("Mutants", lambda k: k.startswith("mutants")),
    ("Batch verify", lambda k: k.startswith("verify-batch")),
    ("Port checks", lambda k: k.startswith("check-")),
]
# processes that say nothing about what an agent is doing
QUIET = {"claude", "node", "codex", "bash", "sh", "zsh", "sleep", "timeout", "tee", "cat", "flock", "setsid",
         "nice", "env", "head", "tail", "grep", "sed", "awk", "sort", "xargs", "git", "less", "wc", "tr", "cut"}


# helpers that are no agent's work: MCP and language servers, the guards, this tool
NOISE = re.compile(r"\bmcp\b|-mcp\b|mcp-server|language-?server|\blsp\b|tsserver|status\.py|guard\.sh")


def _timing_report():
    spec = importlib.util.spec_from_file_location("timing_report", pathlib.Path(__file__).with_name("timing-report.py"))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def git(*args):
    return subprocess.run(["git", "-C", str(REPO), *args], check=True, capture_output=True, text=True).stdout


class GitTree:
    """progress.WorkTree's interface over one commit's tree."""

    def __init__(self, cat, rev):
        self.cat, self.rev = cat, rev

    def read(self, rel):
        return self.cat.blob(f"{self.rev}:{rel}")

    def ls(self, rel, suffix):
        out = git("ls-tree", "--name-only", f"{self.rev}:{rel}") if self.read_kind(rel) == "tree" else ""
        return [f"{rel}/{n}" for n in sorted(out.split()) if n.endswith(suffix)]

    def read_kind(self, rel):
        return self.cat.kind(f"{self.rev}:{rel}")


class Cat:
    """One `git cat-file --batch` for every blob a run reads."""

    def __init__(self):
        self.p = subprocess.Popen(["git", "-C", str(REPO), "cat-file", "--batch"],
                                  stdin=subprocess.PIPE, stdout=subprocess.PIPE)

    def _ask(self, spec):
        self.p.stdin.write(spec.encode() + b"\n")
        self.p.stdin.flush()
        head = self.p.stdout.readline().split()
        if len(head) < 3:  # "<spec> missing"
            return None, None
        data = self.p.stdout.read(int(head[2]))
        self.p.stdout.read(1)
        return head[1].decode(), data

    def blob(self, spec):
        kind, data = self._ask(spec)
        return data.decode("utf-8") if kind == "blob" else None

    def kind(self, spec):
        return self._ask(spec)[0]


def source_keys(revs):
    """rev -> the object ids of SOURCES there: equal keys mean equal counts."""
    specs = [f"{r}:{src}" for r in revs for src in SOURCES]
    out = subprocess.run(["git", "-C", str(REPO), "cat-file", "--batch-check=%(objectname)"],
                         input="\n".join(specs) + "\n", check=True, capture_output=True, text=True).stdout.split("\n")
    ids = [line.split()[0] if line and not line.endswith("missing") else "-" for line in out[:len(specs)]]
    n = len(SOURCES)
    return {r: " ".join(ids[i * n:(i + 1) * n]) for i, r in enumerate(revs)}


def snapshot(rows, tree):
    """(counts per module, hooked) at one tree, or None before the hook table."""
    if tree.read("crates/ffx-hook/src/lib.rs") is None or tree.read("symbols/ffx.sym") is None:
        return None, {}
    names = progress.symbols(tree)
    hooked = progress.hooks(tree)
    module = progress.placement(rows, names)
    stats = progress.count(rows, module, names, hooked, progress.classes(tree))
    # replaced functions that ran in-game (hook-hit mode, symbols/hook-hits.txt)
    ran = {int(x, 16) for x in (tree.read("symbols/hook-hits.txt") or "").split()}
    for a, _, size in rows:
        if a in ran and hooked.get(a) == "replaced":
            stats[module[a]]["ran"] += 1
            stats[module[a]]["ran_bytes"] += size
    keep = ("functions", "bytes", "named", "named_bytes", "replaced", "replaced_bytes",
            "wrapped", "logic", "logic_bytes", "to_port", "to_port_bytes", "ran", "ran_bytes",
            "stub", "stub_bytes")
    return {m: {k: s.get(k, 0) for k in keep} for m, s in stats.items()}, hooked


def history(rows, ref, cache):
    """One entry per first-parent commit of ref that changed the counts."""
    log = [line.split("\t", 3) for line in
           git("log", "--first-parent", "--reverse", "--format=%H%x09%ct%x09%P%x09%s", ref).splitlines()]
    # a merge says what it merged by its branch's last subject
    merged = [ps.split()[1] for _, _, ps, subject in log if len(ps.split()) > 1]
    tips = dict(line.split("\t", 1) for line in
                git("log", "--no-walk=unsorted", "--format=%H%x09%s", *merged).splitlines()) if merged else {}
    log = [(sha, ct, tips.get(ps.split()[1], subject) if len(ps.split()) > 1 else subject)
           for sha, ct, ps, subject in log]
    keys = source_keys([sha for sha, _, _ in log])
    cat = Cat()
    by_key = cache.setdefault("by_key", {})
    entries, prev_key, prev_sha, prev_hooked = [], None, None, None
    for sha, ct, subject in log:
        key = keys[sha]
        if key == prev_key:
            continue
        tree = GitTree(cat, sha)
        cached = by_key.get(key)
        hooked = None
        if cached is None:
            counts, hooked = snapshot(rows, tree)
            cached = by_key[key] = {"counts": counts, "added": {}}
        # the ports a commit added: the replaced set against the one before
        added = cached["added"].get(prev_key or "")
        if cached["counts"] and added is None:
            if hooked is None:
                hooked = progress.hooks(tree)
            if prev_hooked is None:
                before = by_key.get(prev_key or "")
                prev_hooked = progress.hooks(GitTree(cat, prev_sha)) if before and before["counts"] else {}
            added = sorted(a for a, s in hooked.items() if s == "replaced" and prev_hooked.get(a) != "replaced")
            cached["added"][prev_key or ""] = added
        prev_key, prev_sha, prev_hooked = key, sha, hooked
        if cached["counts"]:
            entries.append({"sha": sha[:8], "t": int(ct), "subject": subject[:120],
                            "counts": cached["counts"], "added": added})
    return entries


def functions_map(rows, names, hooked, module, category):
    """The binary, function by function in address order, for the map."""
    mods = sorted(progress.KIND, key=lambda m: (progress.KIND[m] != "game", m))
    idx = {m: i for i, m in enumerate(mods)}
    # every category classes.tsv uses (a new one gets its own index), "" for none
    known = ["logic", "glue", "platform", "debug", "dead"]
    cats = known + sorted(set(category.values()) - set(known) - {""}) + [""]
    out = {"modules": mods, "kinds": [progress.KIND[m] for m in mods], "categories": cats, "states": STATES,
           "addr": [], "size": [], "module": [], "state": [], "category": [], "named": [], "names": {}}
    for a, ghidra_name, size in sorted(rows):
        m = module[a]
        name = names.get(a)
        if progress.KIND[m] == "library":
            state = "replaced" if hooked.get(a) == "replaced" else "library"
        else:
            state = hooked.get(a) or ("named" if name else "unnamed")
        i = len(out["addr"])
        out["addr"].append(a)
        out["size"].append(max(size, 1))
        out["module"].append(idx[m])
        out["state"].append(STATES.index(state))
        out["named"].append(1 if name else 0)
        out["category"].append(cats.index(category.get(a, "")))
        if name or not ghidra_name.startswith("FUN_"):
            out["names"][i] = name or ghidra_name
    return out


# ---- phases and forecasts ------------------------------------------------------

def read_json(path):
    """(data, None) or (None, why)."""
    path = pathlib.Path(path)
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        return None, f"{path} does not exist"
    except (OSError, ValueError) as e:
        return None, f"{path}: {e}"
    if not isinstance(data, dict):
        return None, f"{path}: not a JSON object"
    data["path"], data["mtime"] = str(path), int(path.stat().st_mtime)
    return data, None


def addresses(text):
    """The first 0x... word of every line that has one."""
    out = []
    for line in text.splitlines():
        m = re.match(r"\s*(0x[0-9a-fA-F]+)", line)
        if m:
            out.append(int(m[1], 16))
    return out


def forecast(series, now, col=1):
    """series: [[t, left, ...]] oldest first, col the column to use. The pace
    over each of PACES (left gone per day; a window reaching before the
    series counts from its start) and when the rest is done at it."""
    out, left_now = [], series[-1][col]
    for h in PACES:
        t0 = max(now - h * 3600, series[0][0])
        before = series[0][col]
        for point in series:
            if point[0] <= t0:
                before = point[col]
        span = (now - t0) / 86400
        rate = (before - left_now) / span if span > 1 / 24 else 0
        out.append({"hours": h, "span": round(span * 24, 1), "per_day": round(rate, 1),
                    "eta": int(now + left_now / rate * 86400) if rate > 0 and left_now else None})
    return out


def list_phase(p, hooked, resolved, history, size_of, now):
    target, per_list = {}, []
    t_start = None
    files = sorted({f for g in p.get("lists", []) for f in glob.glob(str(WORK / g))})
    skip = {str(WORK / g) for g in p.get("exclude", [])}
    files = [f for f in files if not any(fnmatch.fnmatch(f, s) for s in skip)]
    for f in files:
        try:
            text = pathlib.Path(f).read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        t_start = min(t_start or now, int(os.stat(f).st_mtime))
        name = pathlib.Path(f).stem
        mine = [a for a in dict.fromkeys(addresses(text)) if size_of.get(a, 5) >= 5 and a not in target]
        for a in mine:
            target[a] = name
        per_list.append([name, mine])
    if not target:
        return {"error": "no addresses in " + ", ".join(p.get("lists", []))}
    done_now = {a for a in target if a in hooked or a in resolved}
    # the series: replaced over the branch's history (the commits' added
    # sets), from when the lists were written, ending with the head's count
    total_bytes = sum(size_of.get(a, 0) for a in target)
    pre = [a for a in target if a in resolved]
    seen, done, done_b, series, start = set(), len(pre), sum(size_of.get(a, 0) for a in pre), [], None
    for e in history:
        if e["t"] >= t_start and start is None:
            start = [t_start, len(target) - done, total_bytes - done_b]
        new = [a for a in e["added"] if a in target and a not in seen and a not in resolved]
        seen.update(new)
        done += len(new)
        done_b += sum(size_of.get(a, 0) for a in new)
        if e["t"] >= t_start and new:
            series.append([e["t"], len(target) - done, total_bytes - done_b])
    series.insert(0, start or [t_start, len(target) - done, total_bytes - done_b])
    left = len(target) - len(done_now)
    done_bytes = sum(size_of.get(a, 0) for a in done_now)
    series.append([int(now), left, total_bytes - done_bytes])
    lists = []
    for name, mine in per_list:
        if not mine:
            continue
        d = [a for a in mine if a in done_now]
        lists.append({"name": name, "total": len(mine), "done": len(d),
                      "bytes": sum(size_of.get(a, 0) for a in mine),
                      "done_bytes": sum(size_of.get(a, 0) for a in d)})
    return {"unit": "functions", "total": len(target), "done": len(done_now), "left": left,
            "bytes": total_bytes, "done_bytes": done_bytes, "series": series,
            "pace": forecast(series, now), "pace_bytes": forecast(series, now, 2), "lists": lists,
            "targets": {a: (1 if a in done_now else 0) for a in target}}


def magic_phase(p, now):
    d = WORK / p.get("dir", "m5/magic")
    snaps = []
    for f in sorted(d.glob("worklist*.tsv")):
        if f.name.endswith("-dlls.tsv"):
            continue
        text = f.read_text(encoding="utf-8", errors="replace")
        if "identity" not in text.split("\n", 1)[0]:
            continue  # the first file is the DLL -> list map
        units = sum(1 for line in text.splitlines() if line and not line.startswith("#"))
        snaps.append([int(f.stat().st_mtime), units, f])
    if not snaps:
        return {"error": f"no worklistN.tsv in {d}"}
    snaps.sort()
    out = {"unit": "code identities", "left": snaps[-1][1], "source": str(snaps[-1][2])}
    dlls = snaps[-1][2].with_name(snaps[-1][2].stem + "-dlls.tsv")
    if dlls.exists():
        n = done = fns = covered = 0
        for line in dlls.read_text(encoding="utf-8").splitlines():
            f = line.split("\t")
            if line.startswith("#") or len(f) < 7:
                continue
            n += 1
            fns += int(f[2]) - int(f[3])
            covered += int(f[4])
            done += f[5] == "0" and f[6] == "0"
        out.update(dlls=n, dlls_done=done, instances=fns, covered=covered)
    # the units' total is not in the files: the first run's left plus what
    # the ports covered by then is unknown, so the bar is the DLLs' functions
    # paced at the last run: between runs nothing is measured
    series = [[t, units] for t, units, _ in snaps]
    out.update(series=series, pace=forecast(series, series[-1][0]),
               snapshots=len(snaps), measured=snaps[-1][0])
    return out


def phases(cfg, hooked, resolved, history, size_of, now):
    out = []
    for p in cfg.get("phases", []):
        if not isinstance(p, dict):
            continue
        base = {k: p.get(k) for k in ("id", "stage", "title", "kind")}
        try:
            if p.get("kind") == "lists":
                base.update(list_phase(p, hooked, resolved, history, size_of, now))
            elif p.get("kind") == "magic":
                base.update(magic_phase(p, now))
            else:
                items = [i for i in p.get("items", []) if isinstance(i, dict)]
                base.update(items=items, total=len(items), done=sum(i.get("state") == "done" for i in items))
        except (OSError, ValueError) as e:
            base["error"] = str(e)
        out.append(base)
    return out


# ---- now: agents, the machine, the queue -----------------------------------------

def task_state(s):
    """tasks.json's free-text state -> running | paused | awaiting merge |
    verifying | merged | pushed | done | other."""
    s = (s or "").lower()
    if s.startswith("running"):
        return "running"
    if s.startswith("paused"):
        return "paused"
    if re.search(r"merge pending|to merge|not merged|merge after|merge queue", s):
        return "awaiting merge"
    if re.search(r"verifying|verify pending", s):
        return "verifying"
    if "pushed" in s:
        return "pushed"
    if "merged" in s:
        return "merged"
    if "done" in s:
        return "done"
    return "other"


def processes(paths):
    """path -> [(start, comm, cmdline)] of the processes doing work whose cwd
    is in it (the deepest path wins: agent worktrees sit inside the main
    checkout), and the paths an interactive session (claude, codex) sits in.

    Work is what a session's shell tool runs (a descendant of a Bash call,
    whose shell sources ~/.claude/shell-snapshots) or what runs on its own
    (detached replays, verify runs); a session's helpers (MCP servers,
    language servers) and its launcher are not."""
    btime = 0
    with open("/proc/stat") as f:
        for line in f:
            if line.startswith("btime"):
                btime = int(line.split()[1])
    hz = os.sysconf("SC_CLK_TCK")
    me = os.getpid()
    info = {}
    for pid in os.listdir("/proc"):
        if not pid.isdigit():
            continue
        try:
            with open(f"/proc/{pid}/stat") as f:
                stat = f.read()
            with open(f"/proc/{pid}/cmdline", "rb") as f:
                cmd = f.read().replace(b"\0", b" ").decode("utf-8", "replace").strip()
        except OSError:
            continue
        try:
            cwd = os.readlink(f"/proc/{pid}/cwd")
        except OSError:
            cwd = ""
        rest = stat[stat.rindex(")") + 2:].split()
        info[int(pid)] = (stat[stat.index("(") + 1:stat.rindex(")")], cmd, cwd, int(rest[1]), btime + int(rest[19]) / hz)
    sessions = {p for p, (comm, *_) in info.items() if comm in ("claude", "codex")}
    launchers = set()
    for p in sessions:
        q = info[p][3]
        while q in info and q not in launchers:
            launchers.add(q)
            q = info[q][3]

    def is_work(pid):
        while pid in info:
            comm, cmd, _, ppid, _ = info[pid]
            if "shell-snapshots" in cmd:
                return True
            if pid in sessions:
                return False
            pid = ppid
        return True

    roots = sorted(paths, key=len, reverse=True)
    out = {p: [] for p in paths}
    seated = set()
    for pid, (comm, cmd, cwd, _, start) in info.items():
        root = next((r for r in roots if cwd == r or cwd.startswith(r + "/")), None)
        if root is None:
            continue
        if pid in sessions:
            seated.add(root)
        elif pid != me and pid not in launchers and is_work(pid) and not NOISE.search(cmd):
            out[root].append((start, comm, cmd))
    return out, seated


def worktrees(ref):
    """The repository's worktrees: path, branch, head, locked."""
    out, cur = [], None
    for line in git("worktree", "list", "--porcelain").splitlines() + [""]:
        if line.startswith("worktree "):
            cur = {"path": line[9:], "branch": "", "head": "", "locked": False}
        elif cur is None:
            continue
        elif line.startswith("HEAD "):
            cur["head"] = line[5:]
        elif line.startswith("branch "):
            cur["branch"] = line[7:].removeprefix("refs/heads/")
        elif line.startswith("locked"):
            cur["locked"] = True
        elif not line:
            out.append(cur)
            cur = None
    heads = [w["head"] for w in out if w["head"]]
    info = dict((h, (int(t), s)) for h, t, s in (line.split("\t", 2) for line in git(
        "log", "--no-walk=unsorted", "--format=%H%x09%ct%x09%s", *heads).splitlines())) if heads else {}
    for w in out:
        w["commit_t"], w["subject"] = info.get(w["head"], (0, ""))
        w["subject"] = w["subject"][:100]
        try:
            w["ahead"] = int(git("rev-list", "--count", f"{ref}..{w['head']}").strip()) if w["head"] else 0
        except subprocess.CalledProcessError:
            w["ahead"] = 0
    return out


def windows(now):
    """The windows the time tables compare: the last 1, 3 and 24 h, and the
    24 h before those (the 24 h window's baseline)."""
    return {"1": (now - 3600, now), "3": (now - 3 * 3600, now), "24": (now - 86400, now),
            "prev24": (now - 2 * 86400, now - 86400)}


def timing(now, nproc):
    """The timing log of the last 48 h: per window (windows()) and group
    (TIME_GROUPS) the seconds, runs, failures and time at load > nproc of
    the commands started in it; per hour and group; per worktree the last
    command and seconds in the last 1 and 24 h.
    Top-level commands only for the groups (a cargo run by mutants.py is in
    mutants.py's time); every command for the worktrees' last one."""
    tr = _timing_report()
    since = datetime.datetime.fromtimestamp(now - 48 * 3600).astimezone()
    paths = [p for p in (WORK / "timing").glob("*.tsv") if p.stat().st_mtime >= now - 48 * 3600]
    rows = tr.load(paths, since)
    names = [g for g, _ in TIME_GROUPS] + ["Other"]
    win = windows(now)
    groups = {w: {g: {"group": g, "seconds": 0, "runs": 0, "failed": 0, "busy": 0} for g in names} for w in win}
    hours = [[0] * len(names) for _ in range(48)]
    trees = {}
    for r in rows:
        t = r["t"].timestamp()
        w = trees.setdefault(r["worktree"], {"last": None, "h1": 0, "h24": 0, "first": t})
        end = t + r["wall"]
        if w["last"] is None or end > w["last"]["end"]:
            w["last"] = {"t": int(t), "end": int(end), "wall": round(r["wall"], 1), "code": r["code"],
                         "cmd": r["cmd"][:140]}
        w["first"] = min(w["first"], t)
        if r.get("parent", "-") != "-":
            continue
        if t >= now - 3600:
            w["h1"] += r["wall"]
        if t >= now - 86400:
            w["h24"] += r["wall"]
        g = next((g for g, test in TIME_GROUPS if test(r["kind"])), "Other")
        h = int((now - t) // 3600)
        if 0 <= h < 48:
            hours[47 - h][names.index(g)] += r["wall"]
        for w, (a, b) in win.items():
            if a <= t < b:
                s = groups[w][g]
                s["seconds"] += r["wall"]
                s["runs"] += 1
                s["failed"] += r["code"] not in ("0", "")
                s["busy"] += r["wall"] if tr.busy(r, nproc) else 0
    for w in trees.values():
        w["h1"], w["h24"], w["first"] = round(w["h1"]), round(w["h24"]), int(w["first"])
    return {"windows": {w: [{k: round(v) if isinstance(v, float) else v for k, v in g.items()} for g in gs.values()]
                        for w, gs in groups.items()},
            "names": names, "hours": [[round(x) for x in h] for h in hours],
            "hours_end": int(now // 3600 * 3600 + 3600), "trees": trees}


# the transcripts' tools, by what they are for
TOOL_GROUPS = [
    ("Bash", lambda n: n == "Bash"),
    ("Reading and editing", lambda n: n in ("Read", "Edit", "Write", "MultiEdit", "Grep", "Glob", "NotebookEdit")),
    ("Waiting on agents", lambda n: n in ("Agent", "Task", "SendMessage", "TaskStop", "TaskOutput")),
    ("Waiting on monitors", lambda n: n == "Monitor" or n.endswith("background_wait") or n.endswith("thread_wait")),
]
def _ts(text):
    return datetime.datetime.fromisoformat(text.rstrip("Z") + "+00:00").timestamp()


def transcripts(now, cache_path):
    """The agents' time from their Claude Code transcripts
    (~/.claude/projects/*ffx-decomp*/[<session>/subagents/]*.jsonl), read
    on from where the last run stopped (the byte offsets and open requests
    are cached in cache_path). Per model call the time from its API request
    to its last content block (thinkingDurationMs of it thinking) and its
    output and thinking tokens; per tool call the time from the tool_use to
    its tool_result. Subagents are "agents", the rest "sessions" (the
    coordinator, interactive sessions). Kept: the last 48 h."""
    try:
        cache = json.loads(cache_path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        cache = {}
    if cache.get("version") != 1:
        cache = {"version": 1, "files": {}}
    files = cache["files"]
    root = pathlib.Path.home() / ".claude" / "projects"
    keep = now - 48 * 3600
    seen = set()
    for d in root.glob("*ffx-decomp*"):
        for f in list(d.glob("*.jsonl")) + list(d.glob("*/subagents/*.jsonl")):
            try:
                st = f.stat()
            except OSError:
                continue
            if st.st_mtime < keep:
                continue
            key = str(f)
            seen.add(key)
            c = files.get(key)
            if c is None or c["size"] > st.st_size:
                c = files[key] = {"offset": 0, "size": 0, "req": None, "asst": None, "tools": {}, "iv": [],
                                  "agent": "/subagents/" in key}
            if st.st_size == c["size"]:
                continue
            with open(f, "rb") as fh:
                fh.seek(c["offset"])
                data = fh.read()
            end = data.rfind(b"\n") + 1
            c["offset"] += end
            c["size"] = st.st_size
            for raw in data[:end].splitlines():
                head = raw[:400]
                if b'"type":"api-request"' in head:
                    kind = "req"
                elif b'"role":"assistant"' in head:  # its "type" comes after the message
                    kind = "asst"
                elif b'"type":"user"' in head and b'"tool_result"' in raw:
                    kind = "user"
                else:
                    continue
                try:
                    r = json.loads(raw)
                    t = _ts(r["timestamp"])
                except (ValueError, KeyError):
                    continue
                if kind != "req" and r.get("type") != ("assistant" if kind == "asst" else "user"):
                    continue
                if kind != "asst" and c["req"] and c["asst"]:
                    # the call ended with its last content block
                    a = c["asst"]
                    if a[0] >= keep:
                        c["iv"].append([c["req"], a[0], "model", a[1], a[2], a[3]])
                    c["req"], c["asst"] = None, None
                if kind == "req":
                    c["req"] = t
                elif kind == "asst":
                    m = r.get("message") or {}
                    u = m.get("usage") or {}
                    think = (r.get("thinkingDurationMs") or 0) / 1000
                    prev = c["asst"] or [t, 0, 0, 0]
                    c["asst"] = [t, prev[1] + think, u.get("output_tokens", 0),
                                 (u.get("output_tokens_details") or {}).get("thinking_tokens", 0)]
                    for b in m.get("content") or []:
                        if isinstance(b, dict) and b.get("type") == "tool_use":
                            c["tools"][b.get("id", "")] = [t, b.get("name", "?")]
                else:
                    for b in (r.get("message") or {}).get("content") or []:
                        if isinstance(b, dict) and b.get("type") == "tool_result":
                            start = c["tools"].pop(b.get("tool_use_id", ""), None)
                            if start and t >= keep:
                                c["iv"].append([start[0], t, "tool", start[1]])
            c["iv"] = [i for i in c["iv"] if i[1] >= keep]
            # tool calls never answered (a killed agent) don't wait forever
            c["tools"] = {k: v for k, v in c["tools"].items() if v[0] >= keep}
    for key in list(files):
        if key not in seen:
            del files[key]
    tmp = cache_path.with_suffix(".tmp")
    tmp.write_text(json.dumps(cache), encoding="utf-8")
    tmp.replace(cache_path)

    # per window: model and thinking time, tokens, calls; tool time by group
    out = {}
    for w, (a, b) in windows(now).items():
        rows = {}
        live = set()
        for key, c in files.items():
            who = "agents" if c["agent"] else "sessions"
            for iv in c["iv"]:
                lo, hi = max(a, iv[0]), min(b, iv[1])
                if hi <= lo:
                    continue
                frac = (hi - lo) / max(1e-6, iv[1] - iv[0])
                live.add(key)
                if iv[2] == "model":
                    name = "Thinking" if who == "agents" else "Sessions: model"
                    if who == "agents":
                        think = iv[3] * frac
                        r = rows.setdefault("Thinking", {"group": "Thinking", "seconds": 0, "calls": 0, "tokens": 0})
                        r["seconds"] += think
                        r["tokens"] += iv[5] * frac
                        name = "Writing (model, not thinking)"
                        r = rows.setdefault(name, {"group": name, "seconds": 0, "calls": 0, "tokens": 0})
                        r["seconds"] += (hi - lo) - think
                        r["tokens"] += (iv[4] - iv[5]) * frac
                        r["calls"] += 1
                        continue
                    r = rows.setdefault(name, {"group": name, "seconds": 0, "calls": 0, "tokens": 0})
                    r["seconds"] += hi - lo
                    r["calls"] += 1
                    r["tokens"] += iv[4] * frac
                elif who == "agents":
                    g = next((g for g, test in TOOL_GROUPS if test(iv[3])), "Other tools")
                    r = rows.setdefault(g, {"group": g, "seconds": 0, "calls": 0, "tokens": 0})
                    r["seconds"] += hi - lo
                    r["calls"] += 1
        out[w] = {"rows": [{k: round(v) if isinstance(v, float) else v for k, v in r.items()} for r in rows.values()],
                  "transcripts": len(live), "agents": sum(1 for k in live if files[k]["agent"])}
    return out


def agent_runs(path):
    """Finished agent runs from metrics.tsv (rows "date, name, done, text"):
    functions, k tokens and minutes as the text gives them."""
    out = []
    try:
        lines = pathlib.Path(path).read_text(encoding="utf-8").splitlines()
    except OSError:
        return out
    for line in lines[1:]:
        f = line.split("\t")
        if len(f) < 4 or f[2] != "done":
            continue
        text = " ".join(f[3:])
        num = lambda pat: (lambda m: int(m[1].replace(",", "")) if m else None)(re.search(pat, text))  # noqa: E731
        fns = (num(r"(\d[\d,]*)\s*(?:more\s+)?(?:fns|rows|functions|units|code identities)\b")
               or num(r"^(\d[\d,]*)\s*(?:\(|;|$)"))
        out.append({"date": f[0], "name": f[1], "functions": fns, "ktokens": num(r"(\d+)k tokens"),
                    "minutes": num(r"(\d+) min"), "text": text[:160]})
    return out[-40:]


def machine():
    load = os.getloadavg()
    mem = {}
    with open("/proc/meminfo") as f:
        for line in f:
            k, v = line.split(":")
            mem[k] = int(v.split()[0]) * 1024
    disk = shutil.disk_usage(WORK)
    return {"nproc": os.cpu_count(), "load": [round(x, 2) for x in load], "mem_total": mem.get("MemTotal", 0),
            "mem_avail": mem.get("MemAvailable", 0), "disk_free": disk.free, "disk_total": disk.total}


def guards():
    """The background guards (ffx-cleanup policy): running or not, and the
    last line of their logs."""
    cmds = []
    for pid in os.listdir("/proc"):
        if pid.isdigit():
            try:
                with open(f"/proc/{pid}/cmdline", "rb") as f:
                    cmds.append(f.read().replace(b"\0", b" ").decode("utf-8", "replace"))
            except OSError:
                pass
    out = []
    for name, pat, log in [("memguard", "memguard.sh", WORK / "agents/memguard.log"),
                           ("diskguard", "diskguard.sh", WORK / "agents/diskguard.log"),
                           ("status watch", "status.sh --watch", WORK / "status/status.log")]:
        last, t = "", 0
        try:
            last = log.read_text(encoding="utf-8", errors="replace").rstrip("\n").rsplit("\n", 1)[-1][:160]
            t = int(log.stat().st_mtime)
        except OSError:
            pass
        out.append({"name": name, "running": any(pat in c for c in cmds), "last": last, "t": t})
    return out


def verification(ref):
    """The last first-parent commits of ref with the second machine's replay
    run of each ($FFX_WORK/agents/pc2-<sha>.log), when there is one."""
    logs = {p.stem[4:]: p for p in (WORK / "agents").glob("pc2-*.log") if re.fullmatch(r"[0-9a-f]{7,40}", p.stem[4:])}
    out = []
    for line in git("log", "--first-parent", "-12", "--format=%H%x09%ct%x09%s", ref).splitlines():
        sha, ct, subject = line.split("\t", 2)
        e = {"sha": sha[:9], "t": int(ct), "subject": subject[:100]}
        log = next((p for k, p in logs.items() if sha.startswith(k)), None)
        if log:
            text = log.read_text(encoding="utf-8", errors="replace")
            runs = re.findall(r"^== ([\w.-]+): traces: (.*)$", text, re.M)
            bad = [n for n, r in runs if "same events" not in r]
            exits = [c for c in re.findall(r"\bexit (\d+)\b", text) if c != "0"]
            mtime = int(log.stat().st_mtime)
            e.update(log=str(log), log_t=mtime, replays=len(runs), bad=bad, exits=len(exits),
                     state="running" if time.time() - mtime < 180 else "failed" if bad or exits else "passed")
        out.append(e)
    return out


def live(args, now):
    tasks, why = read_json(args.tasks)
    if tasks is not None:
        for key in ("tasks", "history", "queued"):
            items = tasks.get(key)
            tasks[key] = [t for t in items if isinstance(t, (dict, str))] if isinstance(items, list) else []
        for t in tasks["tasks"]:
            if isinstance(t, dict):
                t["kind"] = task_state(t.get("state"))
    trees = worktrees(args.ref)
    procs, seated = processes([w["path"] for w in trees])
    tm = timing(now, os.cpu_count() or 1)
    try:
        tm["agents"] = transcripts(now, pathlib.Path(args.out) / "transcripts.json")
    except OSError as e:
        tm["agents"] = {"error": str(e)}
    for w in trees:
        mine = [p for p in procs[w["path"]] if p[1] not in QUIET]
        top = min(mine) if mine else None
        w["procs"] = len(mine)
        # an interactive session (Claude Code, T3) sitting in it
        w["session"] = w["path"] in seated
        w["now"] = {"since": int(top[0]), "comm": top[1], "cmd": top[2][:160]} if top else None
        w["timing"] = tm["trees"].get(os.path.basename(w["path"]))
    try:
        unpushed = int(git("rev-list", "--count", f"origin/{args.ref}..{args.ref}").strip())
    except subprocess.CalledProcessError:
        unpushed = None
    del tm["trees"]
    # the main checkout: the common git directory's parent
    common = pathlib.Path(git("rev-parse", "--path-format=absolute", "--git-common-dir").strip())
    return {"generated": int(now), "ref": args.ref, "repo": str(common.parent),
            "tasks": tasks if tasks is not None else {"error": why},
            "worktrees": trees, "machine": machine(), "guards": guards(), "timing": tm,
            "runs": agent_runs(WORK / "agents" / "metrics.tsv"), "verify": verification(args.ref),
            "unpushed": unpushed}


def write_js(path, var, data):
    tmp = path.with_suffix(".tmp")
    tmp.write_text(f"window.{var} = " + json.dumps(data, separators=(",", ":")) + ";\n", encoding="utf-8")
    tmp.replace(path)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ref", default="main")
    ap.add_argument("--out", default=str(WORK / "status"))
    ap.add_argument("--tasks", default=str(WORK / "agents" / "tasks.json"))
    ap.add_argument("--phases", default=str(WORK / "agents" / "phases.json"))
    ap.add_argument("--live", action="store_true", help="write live.js only")
    args = ap.parse_args()
    out = pathlib.Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    started = time.time()
    page = (pathlib.Path(__file__).resolve().parent / "status.html").read_text(encoding="utf-8")
    if args.live:
        write_js(out / "live.js", "FFX_LIVE", live(args, started))
        (out / "index.html").write_text(page, encoding="utf-8")
        return

    rows = progress.functions()
    cache_path = out / f"history-{len(rows)}.json"
    cache = {}
    if cache_path.exists():
        cache = json.loads(cache_path.read_text(encoding="utf-8"))
        if cache.get("version") != CACHE_VERSION or cache.get("rows") != len(rows):
            cache = {}
    cache["version"], cache["rows"] = CACHE_VERSION, len(rows)

    head = git("rev-parse", args.ref).strip()
    entries = history(rows, head, cache)
    for old in sorted(out.glob("history-*.json"), key=lambda f: f.stat().st_mtime)[:-3]:
        old.unlink(missing_ok=True)
    tmp = cache_path.with_suffix(".tmp")
    tmp.write_text(json.dumps(cache), encoding="utf-8")
    tmp.replace(cache_path)

    cat = Cat()
    tree = GitTree(cat, head)
    names = progress.symbols(tree)
    hooked = progress.hooks(tree)
    module = progress.placement(rows, names)
    category = progress.classes(tree)
    fmap = functions_map(rows, names, hooked, module, category)
    where = {a: i for i, a in enumerate(fmap["addr"])}
    size_of = {a: s for a, _, s in rows}

    # the phases, on the commits' added sets as addresses (before the names go in)
    cfg, why = read_json(args.phases)
    resolved = {a for f in RESOLVED_FILES for a in addresses(tree.read(f) or "")}
    plist = phases(cfg, hooked, resolved, entries, size_of, started) if cfg else None
    # per function on the map: 0 in no list, 1 left, 2 done; per module, left
    fmap["m5"] = [0] * len(fmap["addr"])
    m5_left = {}
    for p in plist or []:
        for a, done in p.pop("targets", {}).items():
            i = where.get(a)
            if i is None:
                continue
            fmap["m5"][i] = 2 if done else 1
            if not done:
                m = fmap["modules"][fmap["module"][i]]
                per = m5_left.setdefault(m, [0, 0])
                per[0] += 1
                per[1] += fmap["size"][i]

    # in-game coverage: hook-hit mode's union (symbols/hook-hits.txt, tools/hook-hits.py)
    # against the ports that existed when it was recorded
    ran = {int(x, 16) for x in (tree.read("symbols/hook-hits.txt") or "").split()}
    rec = git("log", "-1", "--format=%H%x09%ct", head, "--", "symbols/hook-hits.txt").strip().split("\t")
    rec_t = int(rec[1]) if len(rec) == 2 else 0
    then = set()
    for e in entries:
        if e["t"] <= rec_t:
            then.update(e["added"])
    hits = {}
    replaced = STATES.index("replaced")
    for i, a in enumerate(fmap["addr"]):
        if fmap["state"][i] != replaced:
            continue
        # module -> [ported then, of them ran, bytes then, bytes ran, ported now]
        h = hits.setdefault(fmap["modules"][fmap["module"][i]], [0, 0, 0, 0, 0])
        h[4] += 1
        if a in then:
            h[0] += 1
            h[2] += fmap["size"][i]
            if a in ran:
                h[1] += 1
                h[3] += fmap["size"][i]

    # name and module of each newly replaced function, for the feed
    for e in entries:
        e["added"] = [[a, fmap["names"].get(where.get(a), f"sub_{a:08x}"), fmap["modules"][fmap["module"][where[a]]]]
                      for a in e["added"] if a in where]

    data = {
        "generated": int(time.time()),
        "ref": args.ref,
        "head": head[:8],
        "head_subject": git("log", "-1", "--format=%s", head).strip(),
        "head_time": int(git("log", "-1", "--format=%ct", head).strip()),
        "kinds": progress.KIND,
        "history": entries,
        "map": fmap,
        "hook_hits": hits,
        "hits_meta": {"sha": rec[0][:8], "t": rec_t, "hits": len(ran)} if rec_t else None,
        "milestones": cfg.get("milestones", []) if cfg else [],
        "phases": plist if cfg else {"error": why},
        "phases_file": {"path": args.phases, "mtime": cfg["mtime"]} if cfg else None,
        "m5_left": m5_left,
        "took": 0,
    }
    data["took"] = round(time.time() - started, 1)
    write_js(out / "data.js", "FFX_STATUS", data)
    write_js(out / "live.js", "FFX_LIVE", live(args, time.time()))
    (out / "index.html").write_text(page, encoding="utf-8")
    stamp = datetime.datetime.now().strftime("%H:%M:%S")
    print(f"{stamp} status: {args.ref} {head[:8]}, {len(entries)} snapshots, {data['took']} s -> {out / 'index.html'}")


if __name__ == "__main__":
    main()

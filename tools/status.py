#!/usr/bin/env python3
"""Writes the status page: how far the rewrite is, per module, and how it got
there. Agents run it through tools/status.sh, which returns at once.

The numbers are tools/progress.py's, taken from a branch (main by default)
through git rather than from the checkout, so every worktree writes the same
page. The history replays the branch's first-parent commits; each commit's
counts are cached by the files they come from, so a run only computes the
commits it has not seen.

Output ($FFX_WORK/status): index.html (tools/status.html), data.js (the
numbers; the open page reloads it every 30 s) and history.json (the cache).

The "Current work" section comes from $FFX_WORK/agents/tasks.json, which
the coordinator keeps by hand (read from the file system, not git; missing
or broken, the section says so). Update it when a round starts, when a task
changes state and when a round ends, then run tools/status.sh:

    {
      "updated": "2026-10-24",            # when the file was last edited
      "phase": "M3 close-out: coverage push and naming",
      "next": "naming pass, then M4",     # optional: what follows the phase
      "round": "coverage 1",
      "tasks": [                          # the current round, in any order
        {"id": "cov-hooks",               # short, unique in the round
         "title": "Hook-hit mode and report",
         "branch": "cov-hooks",           # the agent's branch (its owner)
         "state": "running",              # running | merged | verified | pushed
                                          # (anything else is shown as is)
         "started": "2026-10-24",         # date or ISO time
         "finished": null,                # set when merged
         "result": "one line"}            # empty while running
      ],
      "history": [                        # past rounds, newest first
        {"round": "M3 logic porting", "finished": "2026-10-24",
         "result": "10,263 ported"}
      ]
    }

At the end of a round, add it to the top of "history" (round, finished,
result) and replace "round" and "tasks" with the next one's.

"Left to port" is broken down by symbols/classes.tsv's category: every game
function that is not replaced and whose category is not one of NOT_PORTED
(glue, platform, debug, dead: not ported 1:1) counts, grouped by category,
so a new category shows up as its own group (`stub`: too short to hook,
from tools/classify.py). Within "logic", HELD_M5 and HELD count apart; see
left_to_port().

Usage: tools/status.py [--ref main] [--out DIR] [--tasks FILE]
"""
import argparse
import datetime
import json
import os
import pathlib
import re
import subprocess
import sys
import time

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import progress  # noqa: E402

REPO = progress.REPO
# a commit's counts change only when one of these does
SOURCES = ["symbols/ffx.sym", "symbols/classes.tsv", "crates/ffx-hook/src/lib.rs", "crates/ffx-hook/src/ported",
           "symbols/hook-hits.txt"]
CACHE_VERSION = 4
STATES = ["library", "unnamed", "named", "wrapped", "replaced"]

# "left to port": categories that are not ported 1:1 (docs/rewrite-plan.md)
NOT_PORTED = {"glue", "platform", "debug", "dead"}
# logic held for M5, the main loop (docs/rewrite-plan.md)
HELD_M5 = {
    0x00820ae0: "the main loop",
    0x00822490: "the start-up that runs Sg_MainLoop",
    0x00822840: "the per-frame update",
}
# ported, parity-tested and deliberately not hooked (symbols/ffx.sym says why)
HELD = {
    0x0075b230: "pppKeLnsLpTOtDraw: its in-game result comes from stale memory",
    0x00819c90: "opu_v_u2: never-written locals, an AV risk as a hook",
}
# the groups' order and labels on the page; others show by category name
LEFT_GROUPS = {
    "logic": "To port",
    "held": "Ported, held unhooked on purpose",
    "held-m5": "Held for M5 (the main loop)",
    "stub": "Too short to hook (1-4 bytes)",
}
# one line per replay, from docs/harness.md; a replay missing here shows the
# first line of its script's header
REPLAYS = {
    "field-walk": "Save 1 on the salvage ship deck: walk about and talk; random generators, party records, pad; "
                  "shots before and after",
    "battle-attack": "The deck's first battle (cdsp00): wait for the battle proper, attack twice; actor HP, generators",
    "menu-browse": "Main menu: items, abilities, status, config, back out; pad and party records; "
                   "shots at fixed frames",
    "boot": "Boot before any load, at the game's own rate: logo, the in-engine opening, the title menu; shots only",
    "battle-magic": "Magic 481 (magic_0481.dll), the one overlay that reaches op_shape_draw, its event effect flags "
                    "set by hand",
    "battle-overdrive": "Save 5, Djose highroad: Spiral Cut with the timing bar, Valefor summoned, Energy Blast, "
                        "dismissed, Wakka's menu",
    "coverage-session": "Hook-hit and coverage runs only (no parity), on saves 0 1 2 3 5: boot, walk and talk, "
                        "a battle, every menu, the PC menu",
}


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


def left_to_port(fmap):
    """The game's functions not replaced yet that count as left, by group:
    [{key, label, functions, bytes, modules: {m: [fn, bytes]}, items}],
    items ([addr, name, size, why]) only for groups of 20 or fewer."""
    groups = {}
    replaced = STATES.index("replaced")
    for i, a in enumerate(fmap["addr"]):
        m = fmap["module"][i]
        cat = fmap["categories"][fmap["category"][i]]
        if fmap["kinds"][m] != "game" or fmap["state"][i] == replaced or not cat or cat in NOT_PORTED:
            continue
        key, why = cat, ""
        if cat == "logic":
            if a in HELD_M5:
                key, why = "held-m5", HELD_M5[a]
            elif a in HELD:
                key, why = "held", HELD[a]
        g = groups.setdefault(key, {"key": key, "label": LEFT_GROUPS.get(key, key), "functions": 0, "bytes": 0,
                                    "modules": {}, "items": []})
        size = fmap["size"][i]
        g["functions"] += 1
        g["bytes"] += size
        per = g["modules"].setdefault(fmap["modules"][m], [0, 0])
        per[0] += 1
        per[1] += size
        g["items"].append([a, fmap["names"].get(i, f"sub_{a:08x}"), size, why])
    order = list(LEFT_GROUPS)
    out = sorted(groups.values(), key=lambda g: (order.index(g["key"]) if g["key"] in order else len(order), g["key"]))
    for g in out:
        g["items"] = sorted(g["items"], key=lambda x: -x[2]) if len(g["items"]) <= 20 else []
    return out


def load_tasks(path):
    """tasks.json (the module docstring), or {"error": why}."""
    path = pathlib.Path(path)
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
        mtime = int(path.stat().st_mtime)
    except FileNotFoundError:
        return {"error": f"{path} does not exist"}
    except (OSError, ValueError) as e:
        return {"error": f"{path}: {e}"}
    if not isinstance(data, dict):
        return {"error": f"{path}: not a JSON object"}
    for key in ("tasks", "history"):
        items = data.get(key)
        data[key] = [t for t in items if isinstance(t, dict)] if isinstance(items, list) else []
    data["path"], data["mtime"] = str(path), mtime
    return data


def replays(tree):
    """The replays run-all.sh runs, with its eye list and a line each."""
    script = tree.read("tools/replays/run-all.sh") or ""

    def array(name):
        m = re.search(rf"^{name}=\(([^)]*)\)", script, re.M)
        return m.group(1).split() if m else []

    eye = set(array("eye"))
    names = array("replays")
    if tree.read("tools/replays/coverage-session.txt") is not None:
        names.append("coverage-session")
    out = []
    for n in names:
        about = REPLAYS.get(n)
        if about is None:
            head = (tree.read(f"tools/replays/{n}.txt") or "").splitlines()
            about = head[0].lstrip("# ").strip() if head and head[0].startswith("#") else ""
        out.append({"name": n, "eye": n in eye, "parity": n != "coverage-session", "about": about})
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ref", default="main")
    ap.add_argument("--out", default=str(progress.WORK / "status"))
    ap.add_argument("--tasks", default=str(progress.WORK / "agents" / "tasks.json"))
    args = ap.parse_args()
    out = pathlib.Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    started = time.time()

    rows = progress.functions()
    cache_path = out / "history.json"
    cache = {}
    if cache_path.exists():
        cache = json.loads(cache_path.read_text(encoding="utf-8"))
        if cache.get("version") != CACHE_VERSION or cache.get("rows") != len(rows):
            cache = {}
    cache["version"], cache["rows"] = CACHE_VERSION, len(rows)

    head = git("rev-parse", args.ref).strip()
    entries = history(rows, head, cache)
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

    # name and module of each newly replaced function, for the feed
    where = {a: i for i, a in enumerate(fmap["addr"])}
    for e in entries:
        e["added"] = [[a, fmap["names"].get(where.get(a), f"sub_{a:08x}"), fmap["modules"][fmap["module"][where[a]]]]
                      for a in e["added"] if a in where]

    # hook-hit mode's union (tools/hook-hits.py): per module, the replaced
    # functions and bytes that ran in a replay or the coverage session
    ran = {int(x, 16) for x in (tree.read("symbols/hook-hits.txt") or "").split()}
    hits = {}
    for i, a in enumerate(fmap["addr"]):
        if fmap["state"][i] == STATES.index("replaced"):
            h = hits.setdefault(fmap["modules"][fmap["module"][i]], [0, 0, 0, 0])
            h[0] += 1
            h[2] += fmap["size"][i]
            if a in ran:
                h[1] += 1
                h[3] += fmap["size"][i]

    data = {
        "generated": int(time.time()),
        "ref": args.ref,
        "head": head[:8],
        "head_subject": git("log", "-1", "--format=%s", head).strip(),
        "head_time": int(git("log", "-1", "--format=%ct", head).strip()),
        "kinds": progress.KIND,
        "history": entries,
        "map": fmap,
        # module -> [replaced, of them ran, replaced bytes, ran bytes]
        "hook_hits": hits,
        "left": left_to_port(fmap),
        "tasks": load_tasks(args.tasks),
        "replays": replays(tree),
        "took": 0,
    }
    data["took"] = round(time.time() - started, 1)
    js = out / "data.js"
    tmp = js.with_suffix(".tmp")
    tmp.write_text("window.FFX_STATUS = " + json.dumps(data, separators=(",", ":")) + ";\n", encoding="utf-8")
    tmp.replace(js)
    page = (pathlib.Path(__file__).resolve().parent / "status.html").read_text(encoding="utf-8")
    (out / "index.html").write_text(page, encoding="utf-8")
    stamp = datetime.datetime.now().strftime("%H:%M:%S")
    print(f"{stamp} status: {args.ref} {head[:8]}, {len(entries)} snapshots, {data['took']} s -> {out / 'index.html'}")


if __name__ == "__main__":
    main()

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

Usage: tools/status.py [--ref main] [--out DIR]
"""
import argparse
import datetime
import json
import os
import pathlib
import subprocess
import sys
import time

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import progress  # noqa: E402

REPO = progress.REPO
# a commit's counts change only when one of these does
SOURCES = ["symbols/ffx.sym", "symbols/classes.tsv", "crates/ffx-hook/src/lib.rs", "crates/ffx-hook/src/ported"]
CACHE_VERSION = 3
STATES = ["library", "unnamed", "named", "wrapped", "replaced"]


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
    keep = ("functions", "bytes", "named", "named_bytes", "replaced", "replaced_bytes",
            "wrapped", "logic", "logic_bytes", "to_port", "to_port_bytes")
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
    cats = ["logic", "glue", "platform", "debug", "dead", ""]
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
        out["category"].append(cats.index(category.get(a, "")) if category.get(a, "") in cats else 5)
        if name or not ghidra_name.startswith("FUN_"):
            out["names"][i] = name or ghidra_name
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ref", default="main")
    ap.add_argument("--out", default=str(progress.WORK / "status"))
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

    data = {
        "generated": int(time.time()),
        "ref": args.ref,
        "head": head[:8],
        "head_subject": git("log", "-1", "--format=%s", head).strip(),
        "head_time": int(git("log", "-1", "--format=%ct", head).strip()),
        "kinds": progress.KIND,
        "history": entries,
        "map": fmap,
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

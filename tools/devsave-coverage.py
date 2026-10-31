#!/usr/bin/env python3
"""Summary of a developer-save coverage sweep (tools/devsave-sweep.sh;
docs/harness.md, "Developer saves").

  FFX_GAME_DIR=... tools/devsave-coverage.py SWEEP_DIR [--before HITS...] [--out FILE]

SWEEP_DIR holds the sweep's <n>.txt reports and <n>.hits coverage files
(FFX.exe functions reached, original code). --before names the coverage
that existed before (hits files, `0x<addr>` first words; default: the
M4 coverage run's ~/ffx-work/m4/coverage2/hits/*.hits, every function
armed over the replays and sessions, plus symbols/hook-hits.txt).

It writes:
- per save: whether it loaded, its map and scenario, the battle, how many
  functions it reached and how many no earlier run had;
- the ported functions (`ported!` lines in crates/ffx-hook/src/ported)
  the sweep reached, and how many of those had no real run before;
- the functions symbols/m5-dropped.tsv calls dropped or dead that ran
  (bugs in the drop list), with the saves that reached them;
- the spell overlays loaded, and the entry points that ran in each
  (FFX_MAGIC_EMU's unload reports) that an ffx-magic port binds to
  (`magic-sig --bind`, built from this checkout).
The lists of new functions go next to the summary as TSV files.
"""
import argparse
import collections
import glob
import os
import pathlib
import re
import subprocess
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import importlib  # noqa: E402

REPO = pathlib.Path(__file__).resolve().parent.parent
hook_hits = importlib.import_module("hook-hits")
WORK = pathlib.Path(os.environ.get("FFX_WORK", pathlib.Path.home() / "ffx-work"))
GAME = pathlib.Path(os.environ.get("FFX_GAME_DIR", pathlib.Path.home() / "steam-games/FFX-X2-HD"))
# called by the harness's devsave command itself, not by the game
HARNESS_CALLED = {0x008B55E0: "TkDebugLoadViewerSave (the devsave command calls it)"}


def names():
    out = {}
    for line in (REPO / "symbols/ffx.sym").read_text(encoding="utf-8").splitlines():
        w = line.split()
        if len(w) >= 3 and w[1] == "function":
            try:
                out[int(w[0], 16)] = w[2]
            except ValueError:
                pass
    return out


def dropped():
    out = {}
    for line in (REPO / "symbols/m5-dropped.tsv").read_text(encoding="utf-8").splitlines():
        if line.startswith("#") or not line.strip():
            continue
        f = line.split("\t")
        out[int(f[0], 16)] = (f[1], f[2] if len(f) > 2 else "", f[4] if len(f) > 4 else "")
    return out


def report(path):
    r = collections.defaultdict(list)
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        k, _, v = line.partition("\t")
        r[k].append(v)
    return r


def overlays(logs):
    """(loaded dll names, {dll: {offset: calls}}) from the log lines."""
    loaded, entries, cur = set(), collections.defaultdict(dict), None
    for line in logs:
        m = re.search(r"overlay-emu: loaded (magic_\d+\.dll)", line)
        if m:
            loaded.add(m.group(1))
        m = re.search(r"overlay-emu: (magic_\d+\.dll) cost:", line)
        if m:
            cur = m.group(1)
            continue
        m = re.match(r"\s*\+(0x[0-9a-f]+) (unicorn|rust): (\d+) calls", line)
        if m and cur:
            entries[cur][int(m.group(1), 16)] = int(m.group(3))
    return loaded, entries


def magic_bind(dll, offsets):
    """offset -> port name (or None) for entry points of one overlay."""
    path = GAME / "magicFiles/FFX" / dll
    exe = REPO / "target/release/examples/magic-sig"
    if not path.exists() or not exe.exists() or not offsets:
        return {}
    data = path.read_bytes()
    pe = int.from_bytes(data[0x3C:0x40], "little")
    base = int.from_bytes(data[pe + 0x34:pe + 0x38], "little")
    args = [f"{base + o:x}" for o in sorted(offsets)]
    out = subprocess.run([str(exe), "--bind", str(path), *args], capture_output=True, text=True, check=False).stdout
    res = {}
    for line in out.splitlines():
        a, _, name = line.partition(" ")
        res[int(a, 16) - base] = None if name.strip() in ("", "-") else name.strip()
    return res


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("sweep")
    ap.add_argument("--before", nargs="*")
    ap.add_argument("--out")
    a = ap.parse_args()
    sweep = pathlib.Path(a.sweep)
    before_files = a.before if a.before is not None else (
        sorted(glob.glob(str(WORK / "m4/coverage2/hits/*.hits"))) + [str(REPO / "symbols/hook-hits.txt")])
    before = set()
    for f in before_files:
        before |= hook_hits.read_hits(f)
    sym = names()
    ported = hook_hits.ported_by_file()
    drop = dropped()

    per, union, first_save = {}, set(), {}
    reached_by = collections.defaultdict(list)
    logs = []
    for rep in sorted(sweep.glob("*.txt"), key=lambda p: int(p.stem) if p.stem.isdigit() else 0):
        if not rep.stem.isdigit():
            continue
        n = int(rep.stem)
        r = report(rep)
        hits_path = sweep / f"{n}.hits"
        hits = hook_hits.read_hits(hits_path) if hits_path.exists() else set()
        for h in hits:
            reached_by[h].append(n)
            first_save.setdefault(h, n)
        union |= hits
        logs += [v for v in r.get("log", [])]
        per[n] = (r, hits)

    lines = []
    w = lines.append
    w(f"# Developer-save coverage sweep: {sweep}")
    w("")
    w(f"Coverage before (union of {len(before_files)} files): {len(before)} functions")
    w(f"Reached in the sweep: {len(union)} functions, {len(union - before)} not reached before")
    w("")
    w("## Saves")
    w("")
    w("save\tend\tmap\tscenario\tbattle\thits\tnew")
    crashed = []
    for n, (r, hits) in sorted(per.items()):
        end = (r.get("end") or ["?"])[0]
        load = " ".join(r.get("log", [""])[:1])
        m = re.search(r"map (\d+) entrance \d+ scenario (\d+)", " ".join(r.get("log", [])))
        battle = (r.get("battle") or ["-"])[0]
        if end != "ok":
            crashed.append((n, end))
        w(f"{n}\t{end[:60]}\t{m.group(1) if m else '-'}\t{m.group(2) if m else '-'}\t{battle[:70]}\t{len(hits)}\t{len(hits - before)}")
        del load
    w("")
    w(f"Saves that did not finish: {len(crashed)}")
    for n, e in crashed:
        w(f"- {n}: {e}")
    w("")

    ran_ported = {p for p in ported if p in union}
    new_ported = ran_ported - before
    w("## Ported functions (ported! lines)")
    w("")
    w(f"{len(ported)} hooked ports; {len(ran_ported)} ran in the sweep (original code), "
      f"{len(new_ported)} of them with no real run before; "
      f"{len([p for p in ported if p not in union and p not in before])} still never seen running")
    by_file = collections.Counter(ported[p][0] for p in new_ported)
    w("")
    w("New per hook file: " + ", ".join(f"{k} {v}" for k, v in by_file.most_common()))
    w("")

    ran_dropped = sorted(d for d in drop if d in union)
    w("## Dropped or dead functions that ran (symbols/m5-dropped.tsv)")
    w("")
    w(f"{len(ran_dropped)} of {len(drop)} ran in the sweep "
      f"({len([d for d in drop if d in before])} ran in the coverage before already):")
    w("")
    w("address\tname\tmodule\tsaves\tdrop evidence")
    for d in ran_dropped:
        name, module, why = drop[d]
        note = HARNESS_CALLED.get(d, "")
        saves = reached_by[d]
        s = ",".join(map(str, saves[:8])) + (f" (+{len(saves) - 8})" if len(saves) > 8 else "")
        w(f"0x{d:08x}\t{name}\t{module}\t{s}\t{(note + '; ' if note else '')}{why[:120]}")
    w("")

    loaded, entries = overlays(logs)
    w("## Spell overlays (FFX_MAGIC_EMU)")
    w("")
    w(f"{len(loaded)} loaded: {', '.join(sorted(loaded))}")
    bound_total, ran_total = 0, 0
    magic_rows = []
    for dll in sorted(entries):
        b = magic_bind(dll, entries[dll])
        ran_total += len(entries[dll])
        ported_here = [o for o in entries[dll] if b.get(o)]
        bound_total += len(ported_here)
        for o in sorted(entries[dll]):
            magic_rows.append(f"{dll}\t+{o:#07x}\t{entries[dll][o]}\t{b.get(o) or '-'}")
    w(f"Entry points that ran (unload reports): {ran_total}; with an ffx-magic port bound: {bound_total}")
    no_report = sorted(loaded - set(entries))
    if no_report:
        w(f"Loaded but not unloaded before the game ended (no report): {', '.join(no_report)}")
    w("")

    text = "\n".join(lines) + "\n"
    out = pathlib.Path(a.out) if a.out else sweep / "summary.md"
    out.write_text(text, encoding="utf-8")
    (out.parent / "new-ported.tsv").write_text(
        "".join(f"0x{p:08x}\t{ported[p][1]}\t{ported[p][0]}\t{first_save[p]}\n" for p in sorted(new_ported)),
        encoding="utf-8")
    (out.parent / "new-functions.tsv").write_text(
        "".join(f"0x{h:08x}\t{sym.get(h, '-')}\t{first_save[h]}\t{'ported' if h in ported else ''}\n"
                for h in sorted(union - before)), encoding="utf-8")
    (out.parent / "magic-entries.tsv").write_text("".join(r + "\n" for r in magic_rows), encoding="utf-8")
    print(text)


if __name__ == "__main__":
    main()

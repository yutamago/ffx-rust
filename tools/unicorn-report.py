#!/usr/bin/env python3
"""Sums ffx-rust's Unicorn profiles: what original code still ran emulated.

  tools/unicorn-report.py TSV... [--sym FILE | --no-sym] [--top N]
                          [--out FILE] [--tsv FILE] [--gate]
  tools/unicorn-report.py --self-test

Each TSV is one run's `ffx-rust --profile` file (crates/ffx-host
report::functions_tsv: module, start, name, calls, blocks, samples_ms; a
header line first). tools/replays/run-all.sh with FFX_HARNESS_EXE=rust and
FFX_HOST_PROFILE=1 keeps one per replay as <run>/rust/<replay>.profile.tsv
and runs this on them at the end (docs/harness.md, "Unicorn coverage in
ffx-rust"). The run's name is the file name without .profile.tsv / .tsv.

Rows are grouped by module:
  FFX.exe        the image's .text, named by symbols/ffx.sym; the rows in
                 the movie libraries (libvpx, mkvparser, libogg, libVorbis:
                 docs/formats/platform.md "Code", replaced in stage 2) are
                 counted apart as "movie libraries"
  MSVCR110       MSVCR110.dll (by its nearest export)
  MSVCP110       MSVCP110.dll
  overlays       each magic_*.dll (by its nearest entry the game can reach)
  other          anything else
--sym (default: symbols/ffx.sym of this checkout) names FFX.exe's rows
again, by the function containing each start, so a profile written without
an ffx.sym (rows named by their block's address) sums by function too;
--no-sym keeps the names as written.

The stage 1 gate's wording for ffx-rust (docs/rewrite-plan.md, M5 stage 1):
no FFX.exe function outside the movie libraries ran in Unicorn. --gate exits
1 when one did (and 2 when a file can't be read). --tsv writes the summed
rows (module, start, name, calls, blocks, samples_ms, runs).
"""
import argparse
import bisect
import pathlib
import sys
from collections import defaultdict

REPO = pathlib.Path(__file__).resolve().parent.parent

# docs/formats/platform.md, "Code": the third-party movie libraries linked
# into FFX.exe, [start, end)
MOVIE_LIBRARIES = [
    (0x00409370, 0x0041E5B0, "libvpx"),
    (0x0041E5B0, 0x0042E1D0, "mkvparser"),
    (0x0042E1D0, 0x0042EE40, "libogg"),
    (0x00A7C920, 0x00A8D6BE, "libvpx DSP"),
    (0x00A8EE40, 0x00AA51E0, "libVorbis"),
]

GROUPS = ["FFX.exe", "movie libraries", "MSVCR110", "MSVCP110", "overlays", "other"]
HEADER = ["module", "start", "name", "calls", "blocks", "samples_ms"]


def movie_library(addr):
    for lo, hi, name in MOVIE_LIBRARIES:
        if lo <= addr < hi:
            return name
    return None


def group_of(module, start):
    m = module.lower()
    if m == "ffx.exe":
        return "movie libraries" if movie_library(start) else "FFX.exe"
    if m.startswith("msvcr110"):
        return "MSVCR110"
    if m.startswith("msvcp110"):
        return "MSVCP110"
    if m.startswith("magic_") and m.endswith(".dll"):
        return "overlays"
    return "other"


class Symbols:
    """Function starts and names from ffx.sym (`<addr> function <name>`)."""

    def __init__(self, text=""):
        names = {}
        for line in text.splitlines():
            p = line.split("#", 1)[0].split()
            if len(p) >= 3 and p[1] == "function":
                try:
                    names[int(p[0], 16)] = p[2]
                except ValueError:
                    continue
        self.starts = sorted(names)
        self.names = names

    def containing(self, addr):
        i = bisect.bisect_right(self.starts, addr)
        if i == 0:
            return None
        a = self.starts[i - 1]
        return a, self.names[a]


def read_tsv(text):
    """The rows of one profile: dicts with the header's keys, numbers as
    ints. Raises ValueError on a malformed file."""
    lines = [x for x in text.splitlines() if x.strip()]
    if not lines or lines[0].split("\t")[:len(HEADER)] != HEADER:
        raise ValueError("not an ffx-rust profile (header " + repr(lines[0] if lines else "") + ")")
    rows = []
    for n, line in enumerate(lines[1:], 2):
        f = line.split("\t")
        if len(f) < len(HEADER):
            raise ValueError(f"line {n}: {len(f)} fields")
        rows.append({
            "module": f[0],
            "start": int(f[1], 16),
            "name": f[2],
            "calls": int(f[3]),
            "blocks": int(f[4]),
            "samples": int(f[5]),
        })
    return rows


def run_name(path):
    name = pathlib.Path(path).name
    for suffix in (".profile.tsv", ".tsv"):
        if name.endswith(suffix):
            return name[: -len(suffix)]
    return name


def rename(rows, syms):
    """FFX.exe rows named by the function containing their start, merged
    where several fall into one function (block rows of a run without
    ffx.sym). Calls count only at the function's own start."""
    out = {}
    for r in rows:
        r = dict(r)
        if r["module"].lower() == "ffx.exe" and syms is not None:
            hit = syms.containing(r["start"])
            if hit:
                start, name = hit
                if start != r["start"]:
                    r["calls"] = 0
                r["start"], r["name"] = start, name
        key = (r["module"], r["start"])
        if key in out:
            o = out[key]
            for k in ("calls", "blocks", "samples"):
                o[k] += r[k]
        else:
            out[key] = r
    return list(out.values())


def key_of(r):
    """What makes two runs' rows one function: the module and the start,
    for an overlay its name (file+offset): each load maps it at a fresh
    address."""
    if group_of(r["module"], r["start"]) == "overlays":
        return (r["module"].lower(), r["name"])
    return (r["module"], r["start"])


def summarise(runs):
    """runs: [(name, rows)]. Returns (per_run, summed): per_run[name][group]
    = (functions, blocks); summed: key_of(row) -> row with 'runs'."""
    per_run = {}
    summed = {}
    for name, rows in runs:
        g = defaultdict(lambda: [0, 0])
        for r in rows:
            k = group_of(r["module"], r["start"])
            g[k][0] += 1
            g[k][1] += r["blocks"]
            key = key_of(r)
            s = summed.setdefault(key, {**r, "calls": 0, "blocks": 0, "samples": 0, "runs": []})
            for f in ("calls", "blocks", "samples"):
                s[f] += r[f]
            if name not in s["runs"]:
                s["runs"].append(name)
        per_run[name] = {k: tuple(v) for k, v in g.items()}
    return per_run, summed


def gate_rows(summed):
    """The stage 1 gate's failures: FFX.exe rows outside the movie
    libraries."""
    return sorted((r for r in summed.values() if group_of(r["module"], r["start"]) == "FFX.exe"),
                  key=lambda r: (-r["blocks"], r["start"]))


def report(per_run, summed, top=40, groups=None):
    """The report; `groups` limits the function lists to those modules
    (None: every module)."""
    out = []
    w = max([len(n) for n in per_run] + [4])
    out.append("Unicorn profile: functions (basic blocks) per run and module")
    out.append(f"{'run':<{w}}  " + "  ".join(f"{g:>20}" for g in GROUPS))
    for name in per_run:
        cells = []
        for g in GROUPS:
            f, b = per_run[name].get(g, (0, 0))
            cells.append(f"{f:>8} ({b:>9})")
        out.append(f"{name:<{w}}  " + "  ".join(f"{c:>20}" for c in cells))
    out.append("")
    out.append(f"summed over {len(per_run)} runs (distinct functions):")
    by_group = defaultdict(list)
    for r in summed.values():
        by_group[group_of(r["module"], r["start"])].append(r)
    for g in GROUPS:
        rows = by_group.get(g, [])
        out.append(f"  {g:<16} {len(rows):>7} functions, {sum(r['blocks'] for r in rows):>12} blocks, "
                   f"{sum(r['calls'] for r in rows):>10} calls, {sum(r['samples'] for r in rows):>8} ms")
    dlls = defaultdict(lambda: [0, 0])
    for r in by_group.get("overlays", []):
        dlls[r["module"]][0] += 1
        dlls[r["module"]][1] += r["blocks"]
    if dlls:
        out.append("  overlays: " + ", ".join(f"{m} {f} ({b})" for m, (f, b) in sorted(dlls.items())))
    for g in GROUPS:
        rows = sorted(by_group.get(g, []), key=lambda r: (-r["blocks"], r["start"]))
        if not rows or (groups is not None and g not in groups):
            continue
        out.append("")
        shown = rows if top <= 0 else rows[:top]
        out.append(f"{g}: {len(rows)} functions" + (f", the {len(shown)} with most blocks" if len(shown) < len(rows) else ""))
        for r in shown:
            runs = r["runs"]
            where = ", ".join(runs[:3]) + (f" +{len(runs) - 3}" if len(runs) > 3 else "")
            lib = movie_library(r["start"]) if g == "movie libraries" else None
            out.append(f"  {r['start']:#010x} {r['name']}{' [' + lib + ']' if lib else ''}: "
                       f"{r['blocks']} blocks, {r['calls']} calls, {r['samples']} ms; {len(runs)} runs ({where})")
    fails = gate_rows(summed)
    out.append("")
    out.append(f"stage 1 gate (no FFX.exe function outside the movie libraries in Unicorn): "
               f"{'PASS' if not fails else 'FAIL'}, {len(fails)} functions")
    return "\n".join(out) + "\n"


def summed_tsv(summed):
    lines = ["\t".join(HEADER + ["runs"])]
    for r in sorted(summed.values(), key=lambda r: (r["module"], r["start"])):
        lines.append(f"{r['module']}\t{r['start']:#010x}\t{r['name']}\t{r['calls']}\t{r['blocks']}\t"
                     f"{r['samples']}\t{','.join(r['runs'])}")
    return "\n".join(lines) + "\n"


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    ap.add_argument("tsv", nargs="*")
    ap.add_argument("--sym", default=str(REPO / "symbols/ffx.sym"))
    ap.add_argument("--no-sym", action="store_true")
    ap.add_argument("--top", type=int, default=40, help="rows listed per module (0: all)")
    ap.add_argument("--out", help="also write the report here (in full with --brief)")
    ap.add_argument("--brief", action="store_true",
                    help="on stdout list only the FFX.exe functions (the gate's) besides the sums")
    ap.add_argument("--tsv", dest="tsv_out", help="write the summed rows here")
    ap.add_argument("--gate", action="store_true", help="exit 1 when the stage 1 gate fails")
    ap.add_argument("--self-test", action="store_true")
    a = ap.parse_args(argv)
    if a.self_test:
        return self_test()
    if not a.tsv:
        ap.error("no profile files")
    syms = None
    if not a.no_sym:
        p = pathlib.Path(a.sym)
        syms = Symbols(p.read_text(encoding="utf-8")) if p.is_file() else None
        if syms is None:
            print(f"unicorn-report: no {p}; FFX.exe names as written", file=sys.stderr)
    runs = []
    bad = 0
    for path in a.tsv:
        try:
            rows = read_tsv(pathlib.Path(path).read_text(encoding="utf-8"))
        except (OSError, ValueError) as e:
            print(f"unicorn-report: {path}: {e}", file=sys.stderr)
            bad = 1
            continue
        runs.append((run_name(path), rename(rows, syms)))
    per_run, summed = summarise(runs)
    text = report(per_run, summed, a.top)
    sys.stdout.write(report(per_run, summed, a.top, ["FFX.exe"]) if a.brief else text)
    if a.out:
        pathlib.Path(a.out).write_text(text, encoding="utf-8")
    if a.tsv_out:
        pathlib.Path(a.tsv_out).write_text(summed_tsv(summed), encoding="utf-8")
    if bad:
        return 2
    return 1 if a.gate and gate_rows(summed) else 0


# ---- self-test: hand-made profiles

SYM = """\
# a comment line
0x00401000 function Second_00401000 # a comment
0x00401020 function memcmp_00401020
0x00409370 function vpx_codec_destroy
0x00a8fd80 function vorbis_info_clear
"""

RUN_A = """\
module\tstart\tname\tcalls\tblocks\tsamples_ms
FFX.exe\t0x00401020\tmemcmp_00401020\t3\t12\t1
FFX.exe\t0x00401030\t0x00401030\t0\t5\t0
FFX.exe\t0x00409370\tvpx_codec_destroy\t1\t4\t0
MSVCR110.dll\t0x10001000\tMSVCR110.dll!memset\t7\t7\t2
MSVCP110.dll\t0x10201000\tMSVCP110.dll!?_Xlength_error@std@@YAXPBD@Z\t1\t3\t0
magic_0481.dll\t0x20001040\tmagic_0481.dll+0x1040\t2\t9\t1
ffx-host-stub\t0x00000010\tx\t1\t1\t0
"""

RUN_B = """\
module\tstart\tname\tcalls\tblocks\tsamples_ms
MSVCR110.dll\t0x10001000\tMSVCR110.dll!memset\t1\t1\t0
magic_0390.dll\t0x20101000\tmagic_0390.dll+0x0\t1\t2\t0
magic_0481.dll\t0x20301040\tmagic_0481.dll+0x1040\t1\t1\t0
FFX.exe\t0x00a8fd90\t0x00a8fd90\t0\t6\t0
"""


def self_test():
    import tempfile
    import unittest

    class T(unittest.TestCase):
        def setUp(self):
            self.syms = Symbols(SYM)

        def test_groups(self):
            self.assertEqual(group_of("FFX.exe", 0x00401020), "FFX.exe")
            self.assertEqual(group_of("FFX.exe", 0x00409370), "movie libraries")
            self.assertEqual(group_of("FFX.exe", 0x0042EE3F), "movie libraries")
            self.assertEqual(group_of("FFX.exe", 0x0042EE40), "FFX.exe")
            self.assertEqual(group_of("FFX.exe", 0x00A8EE40), "movie libraries")
            self.assertEqual(group_of("MSVCR110.dll", 1), "MSVCR110")
            self.assertEqual(group_of("msvcp110.dll", 1), "MSVCP110")
            self.assertEqual(group_of("magic_0481.dll", 1), "overlays")
            self.assertEqual(group_of("magic_0481.txt", 1), "other")
            self.assertEqual(group_of("ffx-host-stub", 1), "other")

        def test_symbols(self):
            self.assertIsNone(self.syms.containing(0x00400FFF))
            self.assertEqual(self.syms.containing(0x00401000), (0x00401000, "Second_00401000"))
            self.assertEqual(self.syms.containing(0x0040101F), (0x00401000, "Second_00401000"))
            self.assertEqual(self.syms.containing(0x00401030), (0x00401020, "memcmp_00401020"))

        def test_read_rejects_other_files(self):
            with self.assertRaises(ValueError):
                read_tsv("address\tname\tcalls\n0x1\tx\t1\n")
            with self.assertRaises(ValueError):
                read_tsv(RUN_A.splitlines()[0] + "\nFFX.exe\t0x1\tx\n")
            self.assertEqual(read_tsv(RUN_A.splitlines()[0] + "\n"), [])

        def test_rename_merges_blocks_into_their_function(self):
            rows = rename(read_tsv(RUN_A), self.syms)
            memcmp = [r for r in rows if r["start"] == 0x00401020]
            self.assertEqual(len(memcmp), 1)
            self.assertEqual(memcmp[0]["blocks"], 17)
            self.assertEqual(memcmp[0]["calls"], 3)  # the inner block's row is no entry
            self.assertEqual(memcmp[0]["name"], "memcmp_00401020")
            self.assertEqual(len(rows), 6)
            # without symbols the rows stay as written
            self.assertEqual(len(rename(read_tsv(RUN_A), None)), 7)

        def test_summary_and_gate(self):
            runs = [("a", rename(read_tsv(RUN_A), self.syms)), ("b", rename(read_tsv(RUN_B), self.syms))]
            per_run, summed = summarise(runs)
            self.assertEqual(per_run["a"]["FFX.exe"], (1, 17))
            self.assertEqual(per_run["a"]["movie libraries"], (1, 4))
            self.assertEqual(per_run["a"]["MSVCR110"], (1, 7))
            self.assertEqual(per_run["a"]["MSVCP110"], (1, 3))
            self.assertEqual(per_run["a"]["overlays"], (1, 9))
            self.assertEqual(per_run["a"]["other"], (1, 1))
            self.assertNotIn("FFX.exe", per_run["b"])
            self.assertEqual(per_run["b"]["movie libraries"], (1, 6))
            # one overlay function, loaded at another address in each run
            o = summed[("magic_0481.dll", "magic_0481.dll+0x1040")]
            self.assertEqual((o["blocks"], o["runs"]), (10, ["a", "b"]))
            self.assertEqual(per_run["b"]["overlays"], (2, 3))
            memset = summed[("MSVCR110.dll", 0x10001000)]
            self.assertEqual((memset["calls"], memset["blocks"], memset["samples"]), (8, 8, 2))
            self.assertEqual(memset["runs"], ["a", "b"])
            self.assertEqual([r["start"] for r in gate_rows(summed)], [0x00401020])
            text = report(per_run, summed, top=5)
            self.assertIn("FAIL, 1 functions", text)
            self.assertIn("magic_0390.dll 1 (2), magic_0481.dll 1 (10)", text)
            self.assertIn("[libVorbis]", text)
            brief = report(per_run, summed, top=5, groups=["FFX.exe"])
            self.assertIn("0x00401020 memcmp_00401020", brief)
            self.assertNotIn("[libVorbis]", brief)
            self.assertIn("movie libraries        2 functions", brief)
            _, clean = summarise(runs[1:])
            self.assertEqual(gate_rows(clean), [])
            self.assertIn("PASS, 0 functions", report(*summarise(runs[1:])))

        def test_summed_tsv_reads_back(self):
            _, summed = summarise([("a", rename(read_tsv(RUN_A), self.syms))])
            back = read_tsv(summed_tsv(summed))
            self.assertEqual(len(back), len(summed))
            self.assertEqual(sum(r["blocks"] for r in back), 41)

        def test_command_line(self):
            with tempfile.TemporaryDirectory() as d:
                d = pathlib.Path(d)
                (d / "ffx.sym").write_text(SYM)
                (d / "battle-magic.profile.tsv").write_text(RUN_A)
                (d / "movie-fmv.profile.tsv").write_text(RUN_B)
                (d / "broken.tsv").write_text("nothing\n")
                args = ["--sym", str(d / "ffx.sym"), "--tsv", str(d / "sum.tsv"), "--out", str(d / "r.txt")]
                import contextlib
                import io
                with contextlib.redirect_stdout(io.StringIO()):
                    self.assertEqual(main([str(d / "battle-magic.profile.tsv"), *args, "--gate"]), 1)
                    self.assertEqual(main([str(d / "movie-fmv.profile.tsv"), *args, "--gate"]), 0)
                    self.assertEqual(main([str(d / "battle-magic.profile.tsv"), *args]), 0)
                    with contextlib.redirect_stderr(io.StringIO()):
                        self.assertEqual(main([str(d / "broken.tsv"), str(d / "movie-fmv.profile.tsv"), *args]), 2)
                self.assertIn("movie-fmv", (d / "r.txt").read_text())
                self.assertEqual(run_name(d / "battle-magic.profile.tsv"), "battle-magic")
                self.assertEqual(run_name("x.tsv"), "x")

    suite = unittest.defaultTestLoader.loadTestsFromTestCase(T)
    ok = unittest.TextTestRunner(verbosity=1).run(suite).wasSuccessful()
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())

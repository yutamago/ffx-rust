#!/usr/bin/env python3
"""timing-report.py <worktree|all> [--since TIME] [--busy N]: how long builds
and tests took, from ~/ffx-work/timing/*.tsv (docs/workflow.md, "Build and
test timing").

Per worktree and tool kind (`cargo test`, `cargo xwin`, `mutants.py`, ...):
total wall time, count, mean and max, and how much of it ran while the
machine was overloaded (1-minute load above nproc, the mean of the load at
start and end). Then the ten slowest commands.

"waiting" counts top-level runs only: a cargo started by mutants.py or
verify-batch.py (the parent column) is inside its parent's time already.
The last line is the total for the coordinator's metrics.tsv.

<worktree> is a directory name (agent-a94...), a path or "all". TIME is
ISO (2026-10-21T09:00, 2026-10-21) or relative (90m, 6h, 2d).
"""

import argparse
import datetime as dt
import os
import pathlib
import re
import sys
from collections import defaultdict

COLS = ["start", "wall", "code", "tool", "cmd", "load0", "load1", "worktree", "parent"]


def parse_since(s):
    m = re.fullmatch(r"(\d+(?:\.\d+)?)([smhd])", s)
    if m:
        unit = {"s": 1, "m": 60, "h": 3600, "d": 86400}[m[2]]
        return dt.datetime.now().astimezone() - dt.timedelta(seconds=float(m[1]) * unit)
    t = dt.datetime.fromisoformat(s)
    return t if t.tzinfo else t.astimezone()


def kind(tool, cmd):
    if tool != "cargo":
        return tool
    words = cmd.split()[1:]
    # cargo's global options before the subcommand
    while words and words[0].startswith(("-", "+")):
        takes = words[0] in ("-C", "-Z", "--config", "--manifest-path", "--color")
        words = words[2:] if takes else words[1:]
    if not words:
        return "cargo"
    if words[0] == "xwin" and len(words) > 1:
        return f"cargo xwin {words[1]}"
    return f"cargo {words[0]}"


def load(paths, since):
    rows = []
    for p in paths:
        for line in p.read_text(errors="replace").splitlines():
            f = line.split("\t")
            if len(f) < 8:
                continue
            r = dict(zip(COLS, f))
            r.setdefault("parent", "-")
            try:
                r["t"] = dt.datetime.fromisoformat(r["start"])
                r["wall"] = float(r["wall"])
            except ValueError:
                continue
            if since and r["t"] < since:
                continue
            r["kind"] = kind(r["tool"], r["cmd"])
            rows.append(r)
    return rows


def busy(r, n):
    try:
        return (float(r["load0"]) + float(r["load1"])) / 2 > n
    except ValueError:
        return False


def fmt(s):
    if s >= 3600:
        return f"{s / 3600:.1f} h"
    if s >= 60:
        return f"{s / 60:.1f} m"
    return f"{s:.1f} s"


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("worktree")
    ap.add_argument("--since", type=parse_since)
    ap.add_argument("--busy", type=float, default=os.cpu_count(), help="load counted as overloaded (default nproc)")
    ap.add_argument("--top", type=int, default=10)
    ap.add_argument("--dir", default=os.environ.get("FFX_TIMING_DIR") or str(pathlib.Path.home() / "ffx-work/timing"))
    a = ap.parse_args()
    d = pathlib.Path(a.dir)
    if a.worktree == "all":
        paths = sorted(d.glob("*.tsv"))
    else:
        paths = [d / (pathlib.Path(a.worktree).name + ".tsv")]
        paths = [p for p in paths if p.exists()]
    rows = load(paths, a.since)
    since = f" since {a.since:%Y-%m-%d %H:%M}" if a.since else ""
    if not rows:
        print(f"no timing lines for {a.worktree}{since} in {d}")
        print("waiting on builds/tests: 0.0 min")
        return
    print(f"{len(rows)} runs{since}, overloaded = load > {a.busy:g}; "
          "waiting = top-level runs (nested ones are inside their parent's time)")
    grand = 0.0
    by_wt = defaultdict(list)
    for r in rows:
        by_wt[r["worktree"]].append(r)
    for wt in sorted(by_wt):
        rs = by_wt[wt]
        top = [r for r in rs if r["parent"] == "-"]
        total = sum(r["wall"] for r in top)
        grand += total
        hot = sum(r["wall"] for r in top if busy(r, a.busy))
        print(f"\n== {wt}: waiting {fmt(total)} in {len(top)} top-level runs "
              f"({fmt(hot)} overloaded, {fmt(total - hot)} not)")
        print(f"  {'kind':<22}{'count':>6}{'total':>10}{'mean':>10}{'max':>10}{'overloaded':>12}{'not':>10}{'failed':>8}")
        kinds = defaultdict(list)
        for r in rs:
            kinds[r["kind"]].append(r)
        for k, ks in sorted(kinds.items(), key=lambda kv: -sum(r["wall"] for r in kv[1])):
            w = [r["wall"] for r in ks]
            h = sum(r["wall"] for r in ks if busy(r, a.busy))
            nested = sum(r["parent"] != "-" for r in ks)
            label = k + (f" ({nested} nested)" if nested else "")
            failed = sum(r["code"] != "0" for r in ks)
            print(f"  {label:<22}{len(ks):>6}{fmt(sum(w)):>10}{fmt(sum(w) / len(w)):>10}{fmt(max(w)):>10}"
                  f"{fmt(h):>12}{fmt(sum(w) - h):>10}{failed:>8}")
        print(f"  slowest {min(a.top, len(rs))}:")
        for r in sorted(rs, key=lambda r: -r["wall"])[: a.top]:
            via = f" (via {r['parent']})" if r["parent"] != "-" else ""
            print(f"  {fmt(r['wall']):>9}  {r['start'][5:16].replace('T', ' ')}  exit {r['code']:<3} "
                  f"load {r['load0']}->{r['load1']}  {r['cmd'][:110]}{via}")
    print(f"\nwaiting on builds/tests: {grand / 60:.1f} min")


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Per-function claims, so agents on the global worklist never port the same
function twice (docs/workflow.md, "Claims").

  tools/claim.py ADDR|NAME... --agent ID     claim (all or nothing)
  tools/claim.py ADDR|NAME... --agent ID --force   also take over stale claims
  tools/claim.py --release ADDR|NAME... --agent ID
  tools/claim.py --release --all --agent ID  every claim of ID (on commit)
  tools/claim.py --list [--agent ID]         claims, oldest first; stale marked

A claim is the file $FFX_CLAIMS/<0xaddr> (default ~/ffx-work/claims), made
with O_CREAT|O_EXCL, so of two agents claiming at once exactly one wins. It
holds one JSON line: agent, time, name. A claim older than 12 hours is
stale: --list marks it, `worklist.py --unclaimed` lists its function again,
and --force takes it over. --agent defaults to $FFX_AGENT. Exits 1 when a
claim or release fails.
"""
import argparse
import json
import os
import pathlib
import sys
import time

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

CLAIMS = pathlib.Path(os.environ.get("FFX_CLAIMS", pathlib.Path.home() / "ffx-work/claims"))
STALE = 12 * 3600


def path(a):
    return CLAIMS / f"0x{a:08x}"


def read(p):
    """The claim at path p as a dict (agent, time, name), or None."""
    try:
        info = json.loads(p.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return None
    info["stale"] = time.time() - info.get("time", 0) > STALE
    return info


def claims():
    """address -> claim dict, for every claim file."""
    out = {}
    if CLAIMS.is_dir():
        for p in CLAIMS.iterdir():
            if p.name.startswith("0x") and len(p.name) == 10:
                info = read(p)
                if info:
                    out[int(p.name, 16)] = info
    return out


def resolve(keys):
    """ADDR or NAME arguments -> (address, name) pairs; exits on an unknown one."""
    import progress

    rows = progress.functions()
    names = progress.symbols()
    dump = {a: n for a, n, _ in rows}
    by_name = {n: a for a, n in dump.items()} | {n: a for a, n in names.items()}
    out = []
    for k in keys:
        try:
            a = int(k, 16)
        except ValueError:
            a = by_name.get(k)
        if a is None or a not in dump:
            sys.exit(f"{k}: not a function start in the dump")
        out.append((a, names.get(a) or dump[a]))
    return out


def create(a, info):
    """Atomically creates the claim file; False when it exists."""
    try:
        fd = os.open(path(a), os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o644)
    except FileExistsError:
        return False
    with os.fdopen(fd, "w", encoding="utf-8") as f:
        f.write(json.dumps(info) + "\n")
    return True


def take_stale(a, agent):
    """Moves a stale claim aside (rename: one taker wins) and puts it back
    when it turned out fresh (someone renewed it in between)."""
    tomb = CLAIMS / f".stale-{a:08x}-{agent}-{os.getpid()}"
    try:
        os.rename(path(a), tomb)
    except FileNotFoundError:
        return
    info = read(tomb)
    if info and not info["stale"]:
        try:
            os.link(tomb, path(a))
        except FileExistsError:
            pass
    tomb.unlink(missing_ok=True)


def age(t):
    h = (time.time() - t) / 3600
    return f"{h:.1f} h" if h >= 1 else f"{h * 60:.0f} min"


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("functions", nargs="*")
    ap.add_argument("--agent", default=os.environ.get("FFX_AGENT"))
    ap.add_argument("--release", action="store_true")
    ap.add_argument("--all", action="store_true", help="with --release: every claim of --agent")
    ap.add_argument("--list", action="store_true")
    ap.add_argument("--force", action="store_true", help="take over stale claims; release others' claims")
    args = ap.parse_args()
    CLAIMS.mkdir(parents=True, exist_ok=True)

    if args.list:
        rows = sorted(claims().items(), key=lambda x: x[1].get("time", 0))
        for a, c in rows:
            if args.agent and c.get("agent") != args.agent:
                continue
            mark = "  STALE" if c["stale"] else ""
            print(f"0x{a:08x} {c.get('name', ''):32} {c.get('agent', '?'):16} {age(c.get('time', 0))}{mark}")
        return 0
    if not args.agent:
        sys.exit("--agent ID (or $FFX_AGENT) is required")

    if args.release:
        if args.all:
            targets = [(a, c.get("name", "")) for a, c in claims().items() if c.get("agent") == args.agent]
        else:
            targets = resolve(args.functions)
        bad = 0
        for a, name in targets:
            c = read(path(a))
            if c is None:
                print(f"0x{a:08x} {name}: not claimed")
            elif c.get("agent") != args.agent and not args.force:
                print(f"0x{a:08x} {name}: claimed by {c.get('agent')}, not {args.agent} (--force releases it)")
                bad += 1
            else:
                path(a).unlink(missing_ok=True)
                print(f"0x{a:08x} {name}: released")
        return 1 if bad else 0

    if not args.functions:
        sys.exit("nothing to claim")
    made, bad = [], 0
    for a, name in resolve(args.functions):
        c = read(path(a))
        if c and c.get("agent") == args.agent:
            print(f"0x{a:08x} {name}: already yours")
            continue
        if c and c["stale"] and args.force:
            take_stale(a, args.agent)
        if create(a, {"agent": args.agent, "time": time.time(), "name": name}):
            made.append(a)
            print(f"0x{a:08x} {name}: claimed")
            continue
        c = read(path(a))
        held = f"{c.get('agent')}, {age(c.get('time', 0))}" if c else "someone"
        stale = " (stale: --force takes it over)" if c and c["stale"] else ""
        print(f"0x{a:08x} {name}: claimed by {held}{stale}")
        bad += 1
    if bad:
        # all or nothing: give back what this call took
        for a in made:
            path(a).unlink(missing_ok=True)
        if made:
            print(f"released the {len(made)} claim(s) this call made")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())

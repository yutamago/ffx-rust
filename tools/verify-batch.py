#!/usr/bin/env python3
"""Verifies a merge batch by running only what it can affect
(docs/workflow.md): the crates changed since BASE and every crate that
depends on them, their ffx-oracle tests on the image alone and on both
snapshots, then the hook build and the replays.

Oracle tests are chosen narrowly: the test files of the crates the batch
touches (not of every crate depending on them), the test files it changed,
and the modules whose hook shims it changed. Every FULL_EVERY-th run, or with
--full, the files of every dependent crate run as well.

  tools/verify-batch.py [BASE] [--full] [--no-replays] [--dry-run]

BASE defaults to origin/main (the last verified, pushed state). Exits 1 on
the first failure.
"""
import json
import os
import pathlib
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
GAME = pathlib.Path(os.environ.get("FFX_GAME_DIR", pathlib.Path.home() / "steam-games/FFX-X2-HD"))
SNAPS = pathlib.Path.home() / "ffx-work/snapshots"
# module crate -> its ffx-oracle test files (prefix match on the file stem)
ORACLE = {
    "ffx-battle": ["battle"], "ffx-chara": ["chara"], "ffx-field": ["field"],
    "ffx-menu": ["menu"], "ffx-text": ["text"], "ffx-script": ["script"],
    "ffx-system": ["system"], "ffx-ps2": ["ps2"], "ffx-vfx": ["vfx"],
    "ffx-sg": ["main"], "ffx-kernel": ["kernel", "rng"], "ffx-core": ["parity"],
}
# changes here affect every oracle test
EVERYTHING = ("crates/ffx-oracle/src/", "crates/ffx-bridge/", "Cargo.lock")
# a wide run (every dependent crate's tests) every this many verifications
FULL_EVERY = 4
COUNT = pathlib.Path.home() / "ffx-work/verify-count"


def run(cmd, env=None, dry=False):
    print("$", " ".join(cmd), flush=True)
    if dry:
        return
    r = subprocess.run(cmd, cwd=REPO, env={**os.environ, **(env or {})})
    if r.returncode:
        sys.exit(f"failed: {' '.join(cmd)}")


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    dry = "--dry-run" in sys.argv
    base = args[0] if args else "origin/main"
    changed = subprocess.run(["git", "diff", "--name-only", f"{base}...HEAD"], cwd=REPO,
                             capture_output=True, text=True, check=True).stdout.split()
    meta = json.loads(subprocess.run(["cargo", "metadata", "--format-version", "1", "--no-deps"], cwd=REPO,
                                     capture_output=True, text=True, check=True).stdout)
    crates = {p["name"]: pathlib.Path(p["manifest_path"]).parent.relative_to(REPO).as_posix()
              for p in meta["packages"]}
    deps = {p["name"]: {d["name"] for d in p["dependencies"] if d["name"] in crates} for p in meta["packages"]}
    touched = {n for n, d in crates.items() if any(f.startswith(d + "/") for f in changed)}
    affected = set(touched)
    while True:  # every crate depending on an affected one
        more = {n for n, d in deps.items() if d & affected} - affected
        if not more:
            break
        affected |= more
    every = any(f.startswith(EVERYTHING) for f in changed)
    n = int(COUNT.read_text()) + 1 if COUNT.exists() else 1
    wide = "--full" in sys.argv or n % FULL_EVERY == 0
    if not dry:
        COUNT.write_text(str(n))
    tests = sorted({t for c in (affected if wide else touched) for t in ORACLE.get(c, [])})
    tests += [pathlib.Path(f).stem for f in changed if f.startswith("crates/ffx-oracle/tests/")]
    files = sorted(p.stem for p in (REPO / "crates/ffx-oracle/tests").glob("*.rs"))
    if every:
        oracle = files
    else:
        # ported shims changed without their crate: test that module too
        for f in changed:
            if f.startswith("crates/ffx-hook/src/ported/"):
                stem = pathlib.Path(f).stem
                tests.append({"map": "field", "ps2_emu": "ps2"}.get(stem, stem))
        oracle = [f for f in files if any(f == t or f.startswith(t + "_") for t in tests)]
    print(f"changed since {base}: {len(changed)} files; crates touched: {sorted(touched)}")
    print(f"affected crates: {sorted(affected)}")
    print(f"oracle tests ({'wide' if wide or every else 'narrow'}, run {n}): {oracle}")
    unit = sorted(affected - {"ffx-oracle", "ffx-hook", "ffx-dinput8", "ffx-launch", "ffx-render", "ffx-studio"})
    env = {"FFX_GAME_DIR": str(GAME)}
    if unit:
        cmd = ["cargo", "test", "--release", "--lib"]
        for c in unit:
            cmd += ["-p", c]
        run(cmd, env, dry)
    if oracle:
        cmd = ["cargo", "test", "--release", "-p", "ffx-oracle"]
        for t in oracle:
            cmd += ["--test", t]
        # build once, then run the image and both snapshots side by side
        run(cmd + ["--no-run"], env, dry)
        os.environ.pop("FFX_SNAPSHOT", None)
        procs = []
        for snap in ("", "field", "battle"):
            e = {**os.environ, **env}
            if snap:
                e["FFX_SNAPSHOT"] = str(SNAPS / f"{snap}.snap")
            name = snap or "image only"
            print(f"== oracle {name}: $ {' '.join(cmd)}", flush=True)
            if dry:
                continue
            out = open(f"/tmp/verify-oracle-{snap or 'image'}.log", "w+")
            procs.append((name, out, subprocess.Popen(cmd, cwd=REPO, env=e, stdout=out, stderr=subprocess.STDOUT)))
        failed = []
        for name, out, p in procs:
            p.wait()
            out.seek(0)
            print(f"== oracle {name}", flush=True)
            sys.stdout.write(out.read())
            sys.stdout.flush()
            if p.returncode:
                failed.append(name)
        if failed:
            sys.exit(f"failed: {' '.join(cmd)} ({', '.join(failed)})")
    if any(f.startswith("crates/") for f in changed):
        run(["python3", "-I", "tools/check-ported.py"], env, dry)
        run(["python3", "-I", "tools/check-clobber.py"], env, dry)
        run(["tools/build-hook.sh"], None, dry)
        if "--no-replays" not in sys.argv:
            run(["tools/replays/run-all.sh"], env, dry)
    print("batch verified")


if __name__ == "__main__":
    main()

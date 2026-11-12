#!/usr/bin/env python3
"""Verifies a merge batch by running only what it can affect
(docs/workflow.md): the crates changed since BASE and every crate that
depends on them, their ffx-oracle tests on the image alone and on both
snapshots, then the hook build and the replays.

Before building anything it checks that no function is hooked twice: a
merge that brings in a second `ported!` for an address already ported or
hooked by hand elsewhere (two branches porting one function) fails here,
not in check-ported after the build.

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

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import progress  # noqa: E402
import timing_log  # noqa: E402

timing_log.install()

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
# a change to these rebuilds everything: every crate, check, the hook and the replays
# (the compiler itself: rust-toolchain.toml, 2026-10-28)
# (Cargo.lock only when an external package changes: tools/verify-batch.py
# `lock_changed`; a new workspace crate also rewrites it, 2026-10-29)
EVERYTHING = ("crates/ffx-oracle/src/", "crates/ffx-bridge/", "rust-toolchain.toml")
# a wide run (every dependent crate's tests) every this many verifications
FULL_EVERY = 4
COUNT = pathlib.Path.home() / "ffx-work/verify-count"


def lock_packages(text):
    """The external packages in a Cargo.lock: (name, version, source)."""
    out, cur = set(), {}
    for line in text.splitlines() + ["[[package]]"]:
        if line.strip() == "[[package]]":
            if cur.get("source"):
                out.add((cur.get("name"), cur.get("version"), cur["source"]))
            cur = {}
        elif " = " in line:
            k, v = line.split(" = ", 1)
            cur[k.strip()] = v.strip().strip('"')
    return out


def lock_changed(base):
    """Whether Cargo.lock's external packages (crates.io, git) differ from
    BASE's: a new compiler input for every crate. Workspace crates added or
    removed only change their own entries, which the crate rules cover."""
    old = subprocess.run(["git", "show", f"{base}:Cargo.lock"], cwd=REPO,
                         capture_output=True, text=True).stdout
    return lock_packages(old) != lock_packages((REPO / "Cargo.lock").read_text())


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
    # nothing personal goes in: user paths, non-noreply commit e-mails
    if subprocess.run(["python3", "-I", "tools/check-privacy.py", base], cwd=REPO).returncode:
        sys.exit("failed: tools/check-privacy.py")
    # two branches porting one function: fail before the build
    dup = progress.duplicate_hooks(REPO)
    for line in dup:
        print(f"duplicate hook: {line}")
    if dup:
        sys.exit(f"failed: {len(dup)} function(s) hooked twice (merge one port, drop the other)")
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
    every = any(f.startswith(EVERYTHING) for f in changed) or ("Cargo.lock" in changed and lock_changed(base))
    if every:
        affected = set(crates)
    n = int(COUNT.read_text()) + 1 if COUNT.exists() else 1
    wide = "--full" in sys.argv or n % FULL_EVERY == 0
    if not dry:
        COUNT.write_text(str(n))
    tests = sorted({t for c in (affected if wide else touched) for t in ORACLE.get(c, [])})
    tests += [pathlib.Path(f).stem for f in changed if f.startswith("crates/ffx-oracle/tests/")]
    # the harvested_<module> files and tests/common/ (their shared helpers) run as one family
    harvest = [f for f in changed if f.startswith("crates/ffx-oracle/tests/")
               and (pathlib.Path(f).stem.startswith("harvested") or "/common/" in f)]
    if harvest:
        tests.append("harvested")
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
    if every or any(f.startswith("crates/") for f in changed):
        run(["python3", "-I", "tools/check-ported.py"], env, dry)
        run(["python3", "-I", "tools/check-clobber.py"], env, dry)
        run(["python3", "-I", "tools/check-callwidth.py"], env, dry)
        run(["python3", "-I", "tools/check-thisarg.py"], env, dry)
        run(["python3", "-I", "tools/check-argslots.py"], env, dry)
        run(["python3", "-I", "tools/check-callconv.py"], env, dry)
        run(["python3", "-I", "tools/check-tailargs.py"], env, dry)
        # no host pointer narrowed to a game address, no game address
        # dereferenced without the arena base (M5 stage 4, docs/host.md)
        run(["python3", "-I", "tools/check-hostptr.py"], env, dry)
        # clippy, host and 32-bit: each branch runs it, but a merge can put
        # together changes that are clean apart (mixer r3's rust_audio broke
        # the host build of ffx-hook unseen, 2026-10-28)
        lint = sorted(affected)
        if lint:
            cmd = ["cargo", "clippy", "--all-targets"]
            for c in lint:
                cmd += ["-p", c]
            run(cmd + ["--", "-D", "warnings"], None, dry)
        hook = sorted(affected & {"ffx-hook", "ffx-dinput8"})
        if hook:
            # cargo-xwin's environment (its CRT cache set up under a lock,
            # tools/xwin-env.sh), then plain cargo: builds run side by side
            pkgs = " ".join(f"-p {c}" for c in hook)
            cmd = ["bash", "-c", 'eval "$(tools/xwin-env.sh i686-pc-windows-msvc)" && '
                   f"cargo clippy --release --target i686-pc-windows-msvc {pkgs} -- -D warnings"]
            run(cmd, None, dry)
            # only when the hook or a crate it links changed (a change to
            # tests alone left this 2.6 min build with nothing to check)
            run(["tools/build-hook.sh"], None, dry)
    # the replays also check their own scripts and comparison tools
    replay_inputs = ("crates/", "tools/replays/", "tools/shot-compare.py", "tools/replay-compare.py")
    # (the oracle tests and docs can't change what the games do)
    game_inputs = [f for f in changed if f.startswith(replay_inputs) and not f.startswith("crates/ffx-oracle/")]
    if (every or game_inputs) and "--no-replays" not in sys.argv:
        run(["tools/replays/run-all.sh"], env, dry)
    print("batch verified")
    if not dry:  # the status page, in the background (tools/status.sh)
        subprocess.run([str(REPO / "tools/status.sh")], check=False)


if __name__ == "__main__":
    main()

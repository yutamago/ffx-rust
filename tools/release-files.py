#!/usr/bin/env python3
"""release-files.py: the licence notices and the content checks of the
release package (tools/package-release.sh; docs/player.md; M5 stage 5).

  tools/release-files.py notices OUT SPEC...   THIRD-PARTY.txt for the trees
  tools/release-files.py check DIR [--target T] [--dev] [--without-steam] [--name WORD]...
  tools/release-files.py selftest DIR [...]    check DIR, then that check
                                               catches a planted problem
  tools/release-files.py sums DIR              DIR/SHA256SUMS.txt (as sha256sum
                                               writes it, every other file, sorted)
  tools/release-files.py pkgdir NAME           a package's source folder
                                               (for a git dependency the
                                               checkout's root)

SPEC is crate:target[:features] (features comma-separated): the crate's
normal dependency tree with default features off plus those features, as
tools/check-licences.py lists the release trees (its `tree`, reused).
`notices` writes every package with its licence, the GPL-family ones
called out, and the licence files each package ships (LICENSE*, COPYING*,
NOTICE*, THIRD_PARTY*, ...), grouped by text. Exit 1 when a tree has a
GPL package and --allow-gpl is not given (the --dev package).

`check` fails (exit 1) when the staged tree holds anything but the files a
package for `--target` may hold (`package()` below; default the Windows
one), a program (PE, ELF or Mach-O) other than the package's programs,
game data or game binaries by name, or text from this machine: a user
path (C:\\Users\\<name>, /mnt/c/Users/<name>, /home/<name>, as
tools/check-privacy.py finds them), the user's real name (its hashed
words), $HOME, $USER and every --name word (the Windows user name, passed
by the script at run time). Binaries are searched through their ASCII and
UTF-16 strings.
"""
import argparse
import hashlib
import importlib.util
import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

REPO = pathlib.Path(__file__).resolve().parent.parent


def _load(name):
    spec = importlib.util.spec_from_file_location(name.replace("-", "_"), REPO / "tools" / f"{name}.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


licences = _load("check-licences")
privacy = _load("check-privacy")

# What a package may hold (paths relative to its root folder), by target.
# Its programs are the only executable files (PE, ELF, Mach-O).
WINDOWS = "x86_64-pc-windows-msvc"
TARGETS = {WINDOWS, "x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu", "aarch64-apple-darwin"}
COMMON = {
    "README.txt",
    "ffx.sym",
    "SHA256SUMS.txt",
    "licences/LICENSE-MIT",
    "licences/LICENSE-APACHE",
    "licences/THIRD-PARTY.txt",
    "licences/zvidlib-vp8-LICENSE",
    "licences/zvidlib-vp8-THIRD_PARTY_NOTICES.md",
    "licences/ffx-movie-NOTICE",
    "licences/ruffle-LICENSE.md",
}
STEAM = {"steam_api64.dll", "licences/steam_api64-NOTICE.txt"}


def programs(target=WINDOWS, steam=True):
    """The executable files of `target`'s package."""
    if target == WINDOWS:
        return {"ffx-rust.exe", "ffx-launch.exe"} | ({"steam_api64.dll"} if steam else set())
    return {"ffx-rust"}


def package(target=WINDOWS, steam=True):
    """Every file of `target`'s package (Steam's only on Windows)."""
    files = COMMON | programs(target, steam)
    if target == WINDOWS:
        files |= {"play-ffx-rust.cmd"} | (STEAM if steam else set())
    return files


def is_program(data):
    """Whether `data` starts like an executable: PE, ELF, Mach-O (thin or fat)."""
    return data[:2] == b"MZ" or data[:4] in (
        b"\x7fELF", b"\xcf\xfa\xed\xfe", b"\xce\xfa\xed\xfe", b"\xfe\xed\xfa\xcf",
        b"\xfe\xed\xfa\xce", b"\xca\xfe\xba\xbe")


# only in a --dev package: Unicorn (GPL-2.0) is linked into its ffx-rust.exe
DEV_ONLY = {"licences/unicorn-NOTICE.txt", "licences/unicorn-COPYING"}
# never, whatever the name list says: the game's own files
GAME = re.compile(r"(?i)(\.vbf$|^ffx(-2)?\.exe$|^magic_\d+\.dll$|^fmod|^iggy|^msvc[pr]1\d0\.dll$|"
                  r"^steam_api\.dll$|^ffx(&|_)x-2|\.(bin|dat|ftcx|dds|fsb|fev|swf|sav)$|^gamesetting\.ini$)")
# strings Unicorn's C code leaves in a binary that links it
UNICORN_MARKS = (b"UC_IGNORE_REG_BREAK", b"unicorn-engine")
LICENCE_FILE = re.compile(r"(?i)^(licen[cs]e|copying|notice|copyright|unlicense|third[-_]party|authors|patents)")


def parse_spec(spec):
    crate, _, rest = spec.partition(":")
    target, _, features = rest.partition(":")
    return crate, target, features


def metadata():
    r = subprocess.run(["cargo", "metadata", "--format-version", "1", "--offline"],
                       cwd=REPO, capture_output=True, text=True, encoding="utf-8", check=True)
    return json.loads(r.stdout)


def licence_files(pkg):
    """The licence files a package ships: in its folder; for a git
    dependency (a crate inside another workspace, Ruffle's) also the
    checkout's root, where theirs is."""
    d = pathlib.Path(pkg["manifest_path"]).parent
    dirs = [d]
    if (pkg.get("source") or "").startswith("git+"):
        p = d
        while "checkouts" in p.parts and p.parent.name != "checkouts" and p != p.parent:
            p = p.parent
            dirs.append(p)
    out = []
    for dd in dirs:
        if dd.is_dir():
            out += sorted(f for f in dd.iterdir() if f.is_file() and LICENCE_FILE.match(f.name))
        if out:
            break
    return out


def notices(out, specs, allow_gpl):
    meta = metadata()
    members = set(meta["workspace_members"])
    by_key = {}
    for p in meta["packages"]:
        by_key[(p["name"], p["version"])] = p
    seen = {}
    gpl = []
    for spec in specs:
        crate, target, features = parse_spec(spec)
        tree = licences.tree(crate, target, features)
        if tree is None:
            print(f"{spec}: cargo tree failed", file=sys.stderr)
            return 2
        for line, lic in tree.items():
            m = re.match(r"(\S+) v(\S+)", line)
            if not m:
                continue
            key = (m.group(1), m.group(2))
            seen[key] = lic
            if licences.gpl_bound(lic):
                gpl.append((spec, *key, lic))
    texts = {}
    rows = []
    for key in sorted(seen):
        p = by_key.get(key)
        lic = seen[key] or (p and p.get("license")) or ""
        ours = p is not None and p["id"] in members
        files = licence_files(p) if p else []
        if not lic and p and p.get("license_file"):
            lic = f"see {p['license_file']}"
        src = "this project" if ours else (p.get("repository") or "") if p else ""
        rows.append((key[0], key[1], lic or "(none given)", src))
        for f in files:
            body = f.read_text(encoding="utf-8", errors="replace").strip()
            h = hashlib.sha256(body.encode()).hexdigest()
            texts.setdefault(h, [body, set()])[1].add(f"{key[0]} {key[1]} ({f.name})")
    lines = [
        "ffx-rust: third-party notices",
        "",
        "Every package compiled into the package's programs, from the release",
        "dependency trees (cargo tree -e normal --no-default-features,",
        f"generated by tools/release-files.py): {', '.join(specs)}.",
        "The packages marked \"this project\" are under MIT OR Apache-2.0",
        "(LICENSE-MIT, LICENSE-APACHE), with the notices of the code they",
        "carry below (ffx-movie: libVorbis; zvidlib-vp8: libvpx/RFC 6386).",
        "Where a package ships no licence file of its own, the standard text",
        "of its licence applies (MIT and Apache-2.0: LICENSE-MIT, LICENSE-APACHE).",
        "steam_api64.dll (Windows) is Valve's, not covered here: steam_api64-NOTICE.txt.",
        "",
    ]
    if gpl:
        lines += ["GPL-family packages (a development package only, never a release):"]
        lines += [f"  {n} {v}: {l} (in {s})" for s, n, v, l in gpl] + [""]
    w = max(len(f"{n} {v}") for n, v, _, _ in rows)
    lines += [f"{len(rows)} packages", ""]
    lines += [f"{n + ' ' + v:<{w}}  {l}  {s}".rstrip() for n, v, l, s in rows]
    lines += ["", "", "Licence texts, as the packages ship them", "=" * 40, ""]
    for h, (body, who) in sorted(texts.items(), key=lambda t: sorted(t[1][1])[0]):
        lines += ["-" * 72, "Used by: " + ", ".join(sorted(who)), "-" * 72, "", body, "", ""]
    pathlib.Path(out).write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
    print(f"notices: {len(rows)} packages, {len(texts)} distinct licence texts, "
          f"{len(gpl)} GPL-family -> {out}")
    if gpl and not allow_gpl:
        for s, n, v, l in gpl:
            print(f"  GPL in the release tree: {n} {v}: {l} ({s})")
        return 1
    return 0


STRINGS = re.compile(rb"[\x20-\x7e\t]{4,}")
WIDE = re.compile(rb"(?:[\x20-\x7e]\x00){4,}")


def strings(data):
    """The text a file holds: its ASCII runs and its UTF-16LE runs."""
    yield from (m.group(0).decode("latin-1") for m in STRINGS.finditer(data))
    yield from (m.group(0).decode("utf-16-le") for m in WIDE.finditer(data))


def check(root, dev, names, target=WINDOWS, steam=True):
    root = pathlib.Path(root)
    progs = programs(target, steam)
    main = "ffx-rust.exe" if target == WINDOWS else "ffx-rust"
    allowed = package(target, steam) | (DEV_ONLY if dev else set())
    problems = []
    present = set()
    words = {w.lower() for w in names if len(w) >= 3}
    for env in ("USER", "LOGNAME"):
        v = os.environ.get(env, "")
        if len(v) >= 3:
            words.add(v.lower())
    # account names that are no one's (the CI runners' `runner`,
    # `runneradmin`), as check-privacy treats them in paths
    words -= privacy.GENERIC
    home = os.environ.get("HOME", "")
    literal = [s for s in {home, str(REPO)} if len(s) > 1]
    for f in sorted(root.rglob("*")):
        rel = f.relative_to(root).as_posix()
        if f.is_dir():
            continue
        present.add(rel)
        if rel not in allowed:
            problems.append(f"{rel}: not a file the {target} package holds (release-files.py package())")
        if GAME.search(f.name):
            problems.append(f"{rel}: a game file by its name")
        data = f.read_bytes()
        if is_program(data) and rel not in progs:
            problems.append(f"{rel}: a program other than {sorted(progs)}")
        found = set()
        if not dev and rel == main and any(m in data for m in UNICORN_MARKS):
            found.add("Unicorn (GPL-2.0) linked in: a release must build without dev-unicorn")
        for s in strings(data):
            low = s.lower()
            for m in privacy.USER_PATH.finditer(s):
                who = m.group(1).lower()
                if who not in privacy.GENERIC and who not in privacy.ALLOW \
                        and not who.startswith(("%", "$", "<")):
                    found.add(f"user path {m.group(0)!r}")
            for lit in literal:
                if lit in s:
                    found.add(f"this machine's path {lit!r}")
            for w in words:
                if re.search(rf"(?<![a-z]){re.escape(w)}(?![a-z])", low):
                    found.add(f"the user name {w!r}")
            if privacy.has_name(s):
                found.add("the user's real name")
        problems += [f"{rel}: {p}" for p in sorted(found)]
    for need in sorted(allowed - present - DEV_ONLY):
        problems.append(f"{need}: missing")
    if dev:
        for need in sorted(DEV_ONLY - present):
            problems.append(f"{need}: missing (a --dev package carries Unicorn's licence)")
    for p in problems:
        print(f"package check: {p}")
    print(f"package check: {len(present)} files, {len(problems)} problems ({root})")
    return 1 if problems else 0


def selftest(root, dev, names, target=WINDOWS, steam=True):
    """The staged tree passes; a planted user path, game file and stray
    program each fail it."""
    if check(root, dev, names, target, steam):
        return 1
    main = "ffx-rust.exe" if target == WINDOWS else "ffx-rust"
    flags = ["--target", target, *(["--dev"] if dev else []), *([] if steam else ["--without-steam"])]
    user = os.environ.get("USER") or "someone"
    if user.lower() in privacy.GENERIC:
        user = "someone"  # a CI runner's account: its paths are no one's
    plants = [
        ("README.txt", f"\nC:\\Users\\{user}x\\Documents\n".encode(), "user path"),
        ("README.txt", b"\n/home/" + user.encode() + b"/ffx\n", "user path"),
        ("data/FFX_Data.vbf", b"x", "a game file"),
        ("tool.dll", b"MZ\x90\x00", "a program"),
        ("libtool.so", b"\x7fELF\x02\x01", "a program"),
        ("libtool.dylib", b"\xcf\xfa\xed\xfe", "a program"),
    ]
    if not dev:
        plants.append((main, b"\0Set UC_IGNORE_REG_BREAK=1\0", "Unicorn"))
    bad = 0
    for rel, data, want in plants:
        with tempfile.TemporaryDirectory() as t:
            copy = pathlib.Path(t) / "pkg"
            shutil.copytree(root, copy)
            p = copy / rel
            p.parent.mkdir(parents=True, exist_ok=True)
            with open(p, "ab") as fh:
                fh.write(data)
            r = subprocess.run([sys.executable, "-I", __file__, "check", str(copy), *flags,
                                *[a for n in names for a in ("--name", n)]],
                               capture_output=True, text=True, encoding="utf-8")
            ok = r.returncode == 1 and want in r.stdout
            print(f"selftest: planted {want} in {rel}: {'caught' if ok else 'MISSED'}")
            bad += not ok
    return 1 if bad else 0


def sums(root):
    """`root/SHA256SUMS.txt`: every other file's SHA-256, by path in byte
    order, in sha256sum's format (no GNU tools needed: macOS, Git Bash)."""
    root = pathlib.Path(root)
    out = root / "SHA256SUMS.txt"
    rels = sorted((f.relative_to(root).as_posix() for f in root.rglob("*")
                   if f.is_file() and f != out), key=lambda r: r.encode())
    lines = [f"{hashlib.sha256((root / r).read_bytes()).hexdigest()}  {r}\n" for r in rels]
    out.write_text("".join(lines), encoding="utf-8", newline="\n")
    return 0


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n", 1)[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    n = sub.add_parser("notices")
    n.add_argument("out")
    n.add_argument("specs", nargs="+")
    n.add_argument("--allow-gpl", action="store_true")
    for name in ("check", "selftest"):
        c = sub.add_parser(name)
        c.add_argument("dir")
        c.add_argument("--dev", action="store_true")
        c.add_argument("--target", default=WINDOWS, choices=sorted(TARGETS))
        c.add_argument("--without-steam", action="store_true")
        c.add_argument("--name", action="append", default=[])
    sub.add_parser("sums").add_argument("dir")
    d = sub.add_parser("pkgdir")
    d.add_argument("name")
    a = ap.parse_args()
    if a.cmd == "pkgdir":
        p = next((p for p in metadata()["packages"] if p["name"] == a.name), None)
        if p is None:
            sys.exit(f"pkgdir: no package {a.name}")
        files = licence_files(p)
        print(files[0].parent if files else pathlib.Path(p["manifest_path"]).parent)
        sys.exit(0)
    if a.cmd == "sums":
        sys.exit(sums(a.dir))
    if a.cmd == "notices":
        sys.exit(notices(a.out, a.specs, a.allow_gpl))
    if a.cmd == "check":
        sys.exit(check(a.dir, a.dev, a.name, a.target, not a.without_steam))
    sys.exit(selftest(a.dir, a.dev, a.name, a.target, not a.without_steam))


if __name__ == "__main__":
    main()

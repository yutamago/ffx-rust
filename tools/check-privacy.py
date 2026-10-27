#!/usr/bin/env python3
"""Checks that nothing personal goes into the repository: no user paths
(C:\\Users\\<name>, /mnt/c/Users/<name>, /home/<name>) in tracked text files,
and only no-reply e-mail addresses on the commits not yet pushed.

The game's own strings (a PS2 developer's host0:/home/... path quoted in
symbols) are allowed through ALLOW. Windows paths in the repo are derived at
run time instead (GetTempPath, %USERPROFILE%).

  tools/check-privacy.py [BASE]   BASE defaults to origin/main
"""
import pathlib
import re
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
USER_PATH = re.compile(
    r"(?i)(?:[A-Z]:\\{1,2}Users\\{1,2}|/mnt/[a-z]/Users/|/home/)([A-Za-z][\w.-]*)")
# a path segment that is no one's name
GENERIC = {"public", "default", "user", "username", "you", "me", "runner"}
# quoted from FFX.exe: the PS2 developers' own paths
ALLOW = {"yonishi"}
NOREPLY = re.compile(r"@users\.noreply\.github\.com$")


def git(*args):
    return subprocess.run(["git", "-C", str(REPO), *args], capture_output=True,
                          text=True, check=True).stdout


def main():
    base = sys.argv[1] if len(sys.argv) > 1 else "origin/main"
    problems = []
    for name in git("grep", "-I", "-l", "-E", r"Users|/home/").split("\n"):
        if not name:
            continue
        for n, line in enumerate((REPO / name).read_text(errors="replace").split("\n"), 1):
            for m in USER_PATH.finditer(line):
                who = m.group(1).lower()
                if who not in GENERIC and who not in ALLOW and not who.startswith(("%", "$", "<")):
                    problems.append(f"{name}:{n}: user path {m.group(0)!r}")
    for line in git("log", "--format=%h %ae %ce", f"{base}..HEAD").split("\n"):
        if not line:
            continue
        h, author, committer = line.split()
        for mail in (author, committer):
            if not NOREPLY.search(mail):
                problems.append(f"commit {h}: e-mail {mail} (commit with the repo's own identity)")
    for p in problems:
        print(p)
    print(f"check-privacy: {len(problems)} problems")
    sys.exit(1 if problems else 0)


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Checks that nothing personal goes into the repository: no user paths
(C:\\Users\\<name>, /mnt/c/Users/<name>, /home/<name>) in tracked text files,
and only no-reply e-mail addresses on the commits not yet pushed.

The game's own strings (a PS2 developer's host0:/home/... path quoted in
symbols) are allowed through ALLOW. Windows paths in the repo are derived at
run time instead (GetTempPath, %USERPROFILE%).

Also, the user's real name in any form, anywhere: tracked files, and the
commits not yet pushed (author and committer names, messages, added lines).
The name is not written here: words are compared by the SHA-256 of their
lowercase form (NAME_WORDS), so hyphenated forms, the old machine name
and a pasted Windows user path all match while the repo never contains it.

  tools/check-privacy.py [BASE]          BASE defaults to origin/main
  tools/check-privacy.py --range REV...  only the commits in that range (pre-push)
"""
import hashlib
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
# no-reply addresses only: a user's, or GitHub's own for web commits
NOREPLY = re.compile(r"(?:@users\.noreply\.github\.com|^noreply@github\.com)$")
# SHA-256 of the lowercase words of the user's real name (user, 2026-10-28)
NAME_WORDS = {
    "b6f2920002873556366ad9f9a44711e4f34b596a892bd175427071e4064a89cc",
    "27f67b0ffe4bfbc98628ca48cf87d0ba1a122e1f1c4b50f7404530cecf0ebd23",
    "d2dae6d1b4625413eade8cafcb06d6d000fdb57d963fc3c5c497084d42288319",
}
WORD = re.compile(r"[a-z]+")


def has_name(text):
    """Whether `text` contains a word of the user's real name."""
    return any(hashlib.sha256(w.encode()).hexdigest() in NAME_WORDS
               for w in set(WORD.findall(text.lower())))


def commit_problems(rev_range):
    """Name and e-mail problems in the commits of `rev_range` (a list of git args)."""
    problems = []
    for h in git("rev-list", *rev_range).split():
        meta = git("log", "-1", "--format=%an%n%ae%n%cn%n%ce%n%B", h)
        author, amail, committer, cmail = meta.split("\n")[:4]
        for mail in (amail, cmail):
            if not NOREPLY.search(mail):
                problems.append(f"commit {h[:9]}: e-mail {mail} (commit with the repo's own identity)")
        if has_name(meta):
            problems.append(f"commit {h[:9]}: the user's real name in its author, committer or message")
        added = "\n".join(l[1:] for l in git("show", "--format=", "--no-color", "--unified=0", h).split("\n")
                          if l.startswith("+") and not l.startswith("+++"))
        if has_name(added):
            problems.append(f"commit {h[:9]}: the user's real name in a line it adds")
    return problems


# the repository git runs in: this one, or with --range the one the caller is
# in (the pre-push hook runs in whichever worktree pushes)
GIT_DIR = [str(REPO)]


def git(*args):
    return subprocess.run(["git", "-C", GIT_DIR[0], *args], capture_output=True,
                          text=True, check=True).stdout


def main():
    if len(sys.argv) > 2 and sys.argv[1] == "--range":
        GIT_DIR[0] = "."
        problems = commit_problems(sys.argv[2:])
        for p in problems:
            print(p)
        print(f"check-privacy: {len(problems)} problems")
        sys.exit(1 if problems else 0)
    base = sys.argv[1] if len(sys.argv) > 1 else "origin/main"
    problems = []
    for name in git("ls-files").split("\n"):
        f = REPO / name
        if name and f.is_file() and f.stat().st_size < 64 << 20:
            data = f.read_bytes()
            if b"\0" not in data[:8192] and has_name(data.decode(errors="replace")):
                problems.append(f"{name}: the user's real name")
    for name in git("grep", "-I", "-l", "-E", r"Users|/home/").split("\n"):
        if not name:
            continue
        for n, line in enumerate((REPO / name).read_text(errors="replace").split("\n"), 1):
            for m in USER_PATH.finditer(line):
                who = m.group(1).lower()
                if who not in GENERIC and who not in ALLOW and not who.startswith(("%", "$", "<")):
                    problems.append(f"{name}:{n}: user path {m.group(0)!r}")
    problems += commit_problems([f"{base}..HEAD"])
    for p in problems:
        print(p)
    print(f"check-privacy: {len(problems)} problems")
    sys.exit(1 if problems else 0)


if __name__ == "__main__":
    main()

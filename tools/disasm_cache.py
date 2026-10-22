"""FFX.exe disassembly per address range, cached in memory and on disk.

`for_exe(path)` returns the cache of one executable. `get(a, size)` gives
{address: instruction text} for the `size` bytes from `a` (objdump -d -M
intel, comments after '#' dropped). Ranges are kept in
$FFX_WORK/cache/clobber-<key>.pickle, the key a hash of the executable, the
objdump version and the line pattern, so a new FFX.exe or binutils starts
afresh. `prefetch` disassembles missing ranges in parallel; `save` merges
what this run added into the file (keeping what concurrent runs added).
Used by check-clobber.py, check-ported.py and callarity.py (packet.py too).
"""
import hashlib
import os
import pathlib
import pickle
import re
import subprocess
import tempfile
from concurrent.futures import ThreadPoolExecutor

WORK = pathlib.Path(os.environ.get("FFX_WORK", pathlib.Path.home() / "ffx-work"))
INSN = re.compile(r"^\s*([0-9a-f]+):\t(.*)$")


def objdump(exe, a, size):
    out = subprocess.run(["objdump", "-d", "--no-show-raw-insn", "-M", "intel", f"--start-address={a:#x}",
                          f"--stop-address={a + size:#x}", str(exe)], capture_output=True, text=True,
                         check=True).stdout
    code = {}
    for line in out.splitlines():
        m = INSN.match(line)
        if m:
            code[int(m.group(1), 16)] = m.group(2).split("#")[0].strip()
    return code


class Cache:
    """(address, size) -> disassembly of one executable."""

    def __init__(self, exe):
        self.exe = pathlib.Path(exe)
        self.code = {}
        self.added = False
        self.path = None
        self.use_disk = True
        self.loaded = False

    def _file(self):
        if self.path is None:
            h = hashlib.sha256(self.exe.read_bytes())
            ver = subprocess.run(["objdump", "--version"], capture_output=True, text=True, check=True).stdout
            h.update(ver.splitlines()[0].encode() if ver else b"")
            h.update(INSN.pattern.encode())
            self.path = WORK / "cache" / f"clobber-{h.hexdigest()[:16]}.pickle"
        return self.path

    def load(self):
        """Reads the disk cache (once)."""
        if not self.use_disk or self.loaded:
            return
        self.loaded = True
        try:
            with open(self._file(), "rb") as f:
                disk = pickle.load(f)
        except (OSError, EOFError, pickle.UnpicklingError):
            return
        disk.update(self.code)
        self.code = disk

    def save(self):
        if not self.use_disk or not self.added:
            return
        path = self._file()
        path.parent.mkdir(parents=True, exist_ok=True)
        merged = {}
        try:  # keep what a concurrent run added
            with open(path, "rb") as f:
                merged = pickle.load(f)
        except (OSError, EOFError, pickle.UnpicklingError):
            pass
        merged.update(self.code)
        fd, tmp = tempfile.mkstemp(dir=path.parent, prefix=path.name + ".")
        with os.fdopen(fd, "wb") as f:
            pickle.dump(merged, f, protocol=pickle.HIGHEST_PROTOCOL)
        os.replace(tmp, path)
        self.added = False

    def get(self, a, size):
        """The cached dict itself: callers must not change it."""
        key = (a, size)
        code = self.code.get(key)
        if code is None:
            code = self.code[key] = objdump(self.exe, a, size)
            self.added = True
        return code

    def prefetch(self, ranges, jobs=None):
        todo = sorted({r for r in ranges if r not in self.code})
        if not todo:
            return
        jobs = jobs or os.cpu_count() or 1
        with ThreadPoolExecutor(max_workers=max(1, jobs)) as pool:
            for key, code in zip(todo, pool.map(lambda r: objdump(self.exe, *r), todo, chunksize=16)):
                self.code[key] = code
        self.added = True


_CACHES = {}


def for_exe(exe):
    key = pathlib.Path(exe).expanduser().resolve()
    if key not in _CACHES:
        _CACHES[key] = Cache(key)
    return _CACHES[key]

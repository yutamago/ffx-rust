"""Wall-time log for the repo's Python tools (docs/workflow.md, "Build and
test timing").

`timing_log.install()` at the top of a tool appends one line to
~/ffx-work/timing/<worktree>.tsv when the process exits, in the format of
the cargo shim (tools/timing/cargo-shim.c). It writes nothing to stdout or
stderr. Children see FFX_TIMING_PARENT=<tool>, so their cargo lines say which
tool ran them. FFX_TIMING=0 turns it off; FFX_TIMING_DIR moves the log.
A tool that leaves through os._exit calls `timing_log.finish(code)` first.
"""

import atexit
import os
import pathlib
import sys
import threading
import time

_state = {}


def _load():
    try:
        with open("/proc/loadavg") as f:
            return f.read().split()[0]
    except OSError:
        return "-"


def install(tool=None):
    """Starts the clock. Does nothing when the calling module is imported
    rather than run (packet.py imports check-clobber.py)."""
    if sys._getframe(1).f_globals.get("__name__") != "__main__":
        return
    if _state or os.environ.get("FFX_TIMING") == "0":
        return
    root = pathlib.Path(__file__).resolve().parent.parent
    tool = tool or pathlib.Path(sys.argv[0]).name
    cmd = " ".join([tool, *sys.argv[1:]]).replace("\t", " ").replace("\n", " ")[:500]
    _state.update(
        wt=os.environ.get("FFX_TIMING_WORKTREE") or root.name,
        tool=tool,
        cmd=cmd,
        iso=time.strftime("%Y-%m-%dT%H:%M:%S%z"),
        t0=time.monotonic(),
        load0=_load(),
        parent=os.environ.get("FFX_TIMING_PARENT") or "-",
        code=0,
    )
    iso = _state["iso"]
    _state["iso"] = iso[:-2] + ":" + iso[-2:]  # +0200 -> +02:00
    os.environ["FFX_TIMING_PARENT"] = tool
    os.environ.setdefault("FFX_TIMING_WORKTREE", root.name)  # mutants.py builds in a copy

    real_exit = sys.exit

    def exit_(code=None):
        if threading.current_thread() is threading.main_thread():
            _state["code"] = code
        real_exit(code)

    sys.exit = exit_
    real_hook = sys.excepthook

    def hook(kind, value, tb):
        _state["code"] = 130 if issubclass(kind, KeyboardInterrupt) else 1
        real_hook(kind, value, tb)

    sys.excepthook = hook
    atexit.register(finish)


def finish(code=None):
    """Writes the line (once). code overrides what sys.exit recorded."""
    if not _state or _state.get("done"):
        return
    _state["done"] = True
    try:
        if code is None:
            code = _state["code"]
        if code is None:
            code = 0
        elif not isinstance(code, int):
            code = 1  # sys.exit("message")
        wall = time.monotonic() - _state["t0"]
        d = os.environ.get("FFX_TIMING_DIR") or str(pathlib.Path.home() / "ffx-work" / "timing")
        os.makedirs(d, exist_ok=True)
        name = _state["wt"]
        line = "\t".join(
            [_state["iso"], f"{wall:.3f}", str(code), _state["tool"], _state["cmd"],
             _state["load0"], _load(), name, _state["parent"]]
        ) + "\n"
        fd = os.open(os.path.join(d, name + ".tsv"), os.O_WRONLY | os.O_APPEND | os.O_CREAT, 0o644)
        try:
            os.write(fd, line.encode())
        finally:
            os.close(fd)
    except Exception:  # noqa: BLE001 - logging must never break the tool
        pass

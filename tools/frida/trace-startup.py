#!/usr/bin/env python3
"""Spawns FFX.exe under frida-server on Windows and logs startup: Steam API
calls and their results, and a backtrace (in FFX.exe addresses) at exit.

Usage: uv run --with frida==17.23.2 tools/frida/trace-startup.py [game dir] [seconds]
"""
import os
import subprocess
import sys
import time

import frida

GAME_DIR = sys.argv[1] if len(sys.argv) > 1 else "~/steam-games/FFX-X2-HD"
SECONDS = float(sys.argv[2]) if len(sys.argv) > 2 else 20

SCRIPT = r"""
const exe = Process.enumerateModules()[0];
const rel = p => {
  const m = Process.findModuleByAddress(p);
  if (m === null) return p.toString();
  if (m.name.toLowerCase() === 'ffx.exe') return '0x' + p.sub(m.base).add(0x400000).toString(16);
  return m.name + '+0x' + p.sub(m.base).toString(16);
};
const bt = ctx => Thread.backtrace(ctx, Backtracer.ACCURATE).map(rel).join(' <- ');

function hookExit(mod, name) {
  const m = Process.findModuleByName(mod);
  const f = m && m.findExportByName(name);
  if (!f) return;
  Interceptor.attach(f, { onEnter(args) {
    send({ kind: 'exit', fn: mod + '!' + name, code: args[0].toInt32(), bt: bt(this.context) });
  }});
}

function hookSteam() {
  const m = Process.findModuleByName('steam_api.dll');
  if (!m) return false;
  for (const e of m.enumerateExports()) {
    if (!/^Steam/.test(e.name)) continue;
    Interceptor.attach(e.address, {
      onEnter(args) { this.a0 = args[0]; },
      onLeave(ret) { send({ kind: 'steam', fn: e.name, arg0: this.a0.toString(), ret: ret.toString(), from: rel(this.returnAddress) }); }
    });
  }
  return true;
}

for (const [m, n] of [['kernel32.dll', 'ExitProcess'], ['kernel32.dll', 'TerminateProcess'],
                      ['msvcr110.dll', 'exit'], ['msvcr110.dll', '_exit'], ['ntdll.dll', 'NtTerminateProcess']])
  hookExit(m, n);
send({ kind: 'info', msg: 'steam_api hooked: ' + hookSteam() + ', exe base ' + exe.base });
"""

def winpath(p):
    return subprocess.run(["wslpath", "-w", p], capture_output=True, text=True, check=True).stdout.strip()

def main():
    game = os.path.expanduser(GAME_DIR)
    device = frida.get_device_manager().add_remote_device("127.0.0.1:27042")
    pid = device.spawn([winpath(game + "/FFX.exe")], cwd=winpath(game))
    session = device.attach(pid)
    script = session.create_script(SCRIPT)
    script.on("message", lambda m, _: print(m.get("payload", m), flush=True))
    script.load()
    device.resume(pid)
    deadline = time.time() + SECONDS
    while time.time() < deadline and not session.is_detached:
        time.sleep(0.2)
    print({"kind": "done", "pid": pid, "exited": session.is_detached})

main()

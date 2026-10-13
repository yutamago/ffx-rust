#!/usr/bin/env python3
"""Reads memory of the running FFX.exe through frida-server.

Usage: uv run --with frida==17.23.2 tools/frida/peek.py <ffx address> <length> [--deref]

The address is an FFX.exe address (image base 0x400000) and is rebased onto
the running module. With --deref, the address holds a pointer and the bytes
are read from where it points. Prints hex.
"""
import subprocess
import subprocess
import sys

import frida

addr, length = int(sys.argv[1], 16), int(sys.argv[2], 0)
deref = "--deref" in sys.argv

device = frida.get_device_manager().add_remote_device("127.0.0.1:27042")
# Frida does not list processes started from a \\wsl.localhost path, so ask Windows.
tasks = subprocess.run(["tasklist.exe", "/FI", "IMAGENAME eq FFX.exe", "/FO", "CSV", "/NH"],
                       capture_output=True, text=True).stdout
pid = int(tasks.split(",")[1].strip('"'))
session = device.attach(pid)
script = session.create_script(f"""
const base = Process.enumerateModules()[0].base;
let p = base.add({addr} - 0x400000);
if ({str(deref).lower()}) p = p.readPointer();
send(p.isNull() ? null : Array.from(new Uint8Array(p.readByteArray({length}))));
""")
result = []
script.on("message", lambda m, _: result.append(m.get("payload")))
script.load()
session.detach()
data = result[0] if result else None
print("null pointer" if data is None else bytes(data).hex())

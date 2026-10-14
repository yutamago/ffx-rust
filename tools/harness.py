#!/usr/bin/env python3
"""Sends harness commands to a running game (tools/run-game.sh --harness PORT).

Usage: tools/harness.py [--port P] '<command>' ['<command>' ...]
Prints each reply. See crates/ffx-hook/src/harness.rs for the commands.
"""
import socket
import sys


class Harness:
    def __init__(self, port=27100, host="127.0.0.1"):
        self.sock = socket.create_connection((host, port), timeout=600)
        self.file = self.sock.makefile("rw", newline="\n")

    def __call__(self, command):
        self.file.write(command + "\n")
        self.file.flush()
        if command == "quit":
            return "ok"
        reply = self.file.readline().rstrip("\n")
        if command.split()[0] == "events" and reply.startswith("ok "):
            n = int(reply.split()[1])
            reply += "".join("\n" + self.file.readline().rstrip("\n") for _ in range(n))
        return reply


if __name__ == "__main__":
    args = sys.argv[1:]
    port = 27100
    if args[:1] == ["--port"]:
        port, args = int(args[1]), args[2:]
    h = Harness(port)
    for command in args:
        print(h(command))

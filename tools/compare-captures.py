#!/usr/bin/env python3
"""Compares the hash records (hashes.txt) of two hash-mode gfxcaptures.

drawdiff compares the scene streams (order, states, decoded constants,
texture and ring keys); this compares what hash-mode streams carry only as
hashes in hashes.txt: render target and back buffer contents (`t`, `p`),
the held scene inputs (`held`), the files opened, and every draw line's
fields except the constant windows (`cb`, which drawdiff decodes).

    python3 -I tools/compare-captures.py <run A> <run B> --allow <file> [capture...] [-v]

Runs are folders of captures (`~/ffx-work/m4/captures/<run>/<capture>`);
docs/drawdiff.md, "Scene gates". Lines are compared in order, field by
field. frames.txt (tags, render-thread samples) is not compared: it races
the main thread.

The noise comes from the scene's drawdiff allow file, so one file holds
every allowance with its cause:
- `any vs=<hash>`: that shader's draws are dropped, with the target lines
  (t, c, z, x, xr) of targets only they draw into, and every target hash
  (t, p) after its first draw in a frame (what it draws is composed into
  later targets);
- `resource vs=<hash> field=vb<n>`: that draw's vertex slot n;
- `resource vs=<hash> field=ps.srv<n>` (or vs.srv<n>): that draw's
  texture slot n;
- constant rules: constant windows are skipped here anyway.
Other rules (state, missing, extra, moved) have no counterpart in the
hash text and are ignored here: drawdiff applies them.
A target or back buffer hash that differs after a draw with an allowed
difference is reported as "follows" and not counted.
"""
import difflib
import os
import re
import sys


class Noise:
    def __init__(self, path):
        self.drop_vs = set()
        self.vb = {}   # vs -> {slot}
        self.srv = {}  # vs -> {"p1", "v0", ...}
        if not path:
            return
        for raw in open(path):
            line = raw.split('#', 1)[0].strip()
            if not line or line.startswith('['):
                continue
            words = line.split(' cause:', 1)[0].split()
            sel = dict(w.split('=', 1) for w in words[1:] if '=' in w)
            vs = sel.get('vs')
            if not vs:
                continue
            if words[0] == 'any':
                self.drop_vs.add(vs)
            elif words[0] == 'resource':
                f = sel.get('field', '')
                m = re.fullmatch(r'vb(\d+)', f)
                if m:
                    self.vb.setdefault(vs, set()).add(m.group(1))
                m = re.fullmatch(r'(ps|vs)\.srv(\d+)', f)
                if m:
                    self.srv.setdefault(vs, set()).add(m.group(1)[0] + m.group(2))


def fields(line):
    """{name: value} of a d line (name = the token before the value)."""
    w = line.split(' ')
    out = {'head': ' '.join(w[:9])}
    i = 9
    while i < len(w) - 1:
        out[w[i]] = w[i + 1]
        i += 2
    return out


def drop_shaders(lines, drop):
    """The lines without the dropped shaders' draws and their targets' lines."""
    if not drop:
        return lines
    frames, cur = [], []
    for line in lines:
        if line.startswith('frame ') and cur:
            frames.append(cur)
            cur = []
        cur.append(line)
    frames.append(cur)
    out = []
    for fr in frames:
        mine, other = set(), set()
        for line in fr:
            if line.startswith('d '):
                f = fields(line)
                roles = {r.split(':', 1)[1] for r in f.get('rt', '').split(',') if ':' in r}
                (mine if f.get('vs') in drop else other).update(roles)
        only = mine - other
        started = False
        for line in fr:
            w = line.split(' ')
            if w[0] == 'd' and fields(line).get('vs') in drop:
                started = True
                continue
            if w[0] in ('t', 'p') and started:
                continue
            if w[0] in ('c', 'z', 'xr') and len(w) > 1 and w[1] in only:
                continue
            if w[0] == 't' and len(w) > 2 and w[2] in only:
                continue
            if w[0] == 'x' and len(w) > 2 and (w[1] in only or w[2] in only):
                continue
            out.append(line)
    return out


def without(value, prefixes):
    return [x for x in value.split(',') if not any(x.startswith(p + ':') for p in prefixes)]


def noisy_draw(a, b, noise):
    """The differing fields of two d lines, minus the allowed noise."""
    fa, fb = fields(a), fields(b)
    vs = fa.get('vs')
    bad = []
    for k in sorted(set(fa) | set(fb)):
        if fa.get(k) == fb.get(k) or k == 'cb':
            continue
        if k == 'vb' and vs in noise.vb:
            if without(fa[k], noise.vb[vs]) == without(fb.get(k, ''), noise.vb[vs]):
                continue
        if k == 'srv' and vs in noise.srv:
            if without(fa[k], noise.srv[vs]) == without(fb.get(k, ''), noise.srv[vs]):
                continue
        bad.append(k)
    return bad


def key(line):
    """What two lines must share to be paired: a draw's head and shaders,
    a target line without its hash."""
    w = line.split(' ')
    if w[0] == 'd':
        f = fields(line)
        return ('d', f['head'].split(' ', 2)[2], f.get('vs'), f.get('ps'))
    if w[0] in ('t', 'p'):
        return tuple(w[:-1])
    return tuple(w)


def split_frames(lines):
    out = []
    for line in lines:
        if line.startswith('frame ') or not out:
            out.append([])
        out[-1].append(line)
    return out


def compare(a, b, noise, verbose):
    la = drop_shaders(open(a).read().splitlines(), noise.drop_vs)
    lb = drop_shaders(open(b).read().splitlines(), noise.drop_vs)
    fa, fb = split_frames(la), split_frames(lb)
    if len(fa) != len(fb):
        return [f'{len(fa)} against {len(fb)} frames'], 0
    bad, follows = [], 0
    for xa, xb in zip(fa, fb):
        r = compare_frame(xa, xb, noise, verbose)
        bad += r[0]
        follows += r[1]
    return bad, follows


def compare_frame(la, lb, noise, verbose):
    ka, kb = [key(x) for x in la], [key(x) for x in lb]
    bad, follows = [], 0
    noisy = False
    for op, i1, i2, j1, j2 in difflib.SequenceMatcher(None, ka, kb, autojunk=False).get_opcodes():
        if op != 'equal':
            for x in la[i1:i2]:
                bad.append('only A: ' + x[:160])
            for y in lb[j1:j2]:
                bad.append('only B: ' + y[:160])
            continue
        for x, y in zip(la[i1:i2], lb[j1:j2]):
            if x == y:
                continue
            kind = x.split(' ', 1)[0]
            if kind == 'd':
                diff = noisy_draw(x, y, noise)
                noisy = True
                if diff:
                    bad.append(f'{x[:40]}: {diff}')
                continue
            if kind in ('t', 'p') and noisy:
                follows += 1
                if verbose:
                    print('   follows:', x, '|', y.rsplit(' ', 1)[-1])
                continue
            bad.append(f'{x[:200]} | {y[:200]}')
    return bad, follows


def main():
    argv = sys.argv[1:]
    verbose = '-v' in argv
    allow = None
    if '--allow' in argv:
        i = argv.index('--allow')
        allow = argv[i + 1]
        del argv[i:i + 2]
    args = [a for a in argv if not a.startswith('-')]
    if len(args) < 3:
        print(__doc__)
        return 2
    noise = Noise(allow)
    status = 0
    for cap in args[2:]:
        try:
            bad, follows = compare(f'{args[0]}/{cap}/hashes.txt', f'{args[1]}/{cap}/hashes.txt', noise, verbose)
        except OSError as e:
            print(f'== {cap}: missing ({e})')
            status = 1
            continue
        print(f'== {cap}: {len(bad)} differences beyond the noise; {follows} target hashes follow a noisy draw')
        for line in bad[:int(os.environ.get("LINES_MAX", "20"))]:
            print('  ' + line)
        if bad:
            status = 1
    return status


if __name__ == '__main__':
    sys.exit(main())

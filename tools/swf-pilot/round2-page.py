#!/usr/bin/env python3
"""The SWF pilot's round 2 page: before (stock Ruffle) and after (the Ruffle
fork with GDraw's mipmaps), for the crops the user picked and the full
screens, plus the measurement table (tools/swf-pilot/measure.sh).

  python3 -I tools/swf-pilot/round2-page.py [pilot dir]

Reads <pilot>/variants/<high|fork-mips>/img and variants/table.md, writes
<pilot>/round2.html (the pilot dir defaults to ~/ffx-work/m5/swf-pilot).
"""
import html
import os
import pathlib
import sys

CROPS = [
    ("crop-pilot-esc-idle-120-196", "pilot-esc-idle: a button, text and frame (220x48 at 120,196, 4x, nearest)"),
    ("crop-pilot-load-idle-105-145", "pilot-load-idle: the portraits, bitmaps (150x75 at 105,145, 4x, nearest)"),
    ("crop-pilot-load-idle-335-180", "pilot-load-idle: text on the save list (4x)"),
    ("crop-pilot-video-600-140", "pilot-video: a Video row (3x)"),
]
SCREENS = ["pilot-esc-idle", "pilot-sys-idle", "pilot-video", "pilot-keyboard", "pilot-load-idle", "pilot-popup"]
BEFORE, AFTER = "variants/high", "variants/fork-mips"


def table(md):
    rows = [r.strip().strip("|").split("|") for r in md.splitlines() if r.startswith("|")]
    out = ["<table>"]
    for i, r in enumerate(rows):
        if i == 1:
            continue
        tag = "th" if i == 0 else "td"
        out.append("<tr>" + "".join(f"<{tag}>{html.escape(c.strip().strip('`'))}</{tag}>" for c in r) + "</tr>")
    out.append("</table>")
    return "\n".join(out)


def stat(pilot, variant, name):
    for line in (pilot / variant / "stats.tsv").read_text().splitlines():
        n, s = line.split("\t", 1)
        if n == name:
            return s
    return ""


def main():
    pilot = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else os.path.expanduser("~/ffx-work/m5/swf-pilot"))
    md = (pilot / "variants" / "table.md").read_text()
    h = ["<!doctype html><meta charset=utf-8><title>SWF pilot round 2: drawing like GDraw</title>",
         "<style>body{font:14px sans-serif;background:#222;color:#ddd;margin:16px}figure{display:inline-block;"
         "margin:4px;vertical-align:top}figcaption{font-size:12px;color:#aaa}img.s{width:420px;cursor:zoom-in}"
         "img.s.big{width:1280px}td,th{border:1px solid #555;padding:3px 6px;font-size:12px}table{border-collapse:collapse}"
         ".stat{font-family:monospace;font-size:12px;color:#9c9}</style>",
         "<h1>SWF pilot round 2: Ruffle drawing like Iggy's GDraw</h1>",
         "<p><a href=index.html>the full comparison with the fix</a> &middot; <a href=round1/index.html>round 1 (stock Ruffle)</a>"
         " &middot; <a href=notes.html>notes</a></p>",
         "<p>Before: stock Ruffle (pinned rev, 4x MSAA). After: the Ruffle fork with GDraw's bitmap handling: every bitmap "
         "gets a full mip chain and is sampled trilinear, every bitmap smoothed, blur passes rounded instead of truncated, "
         "and getTimer on the player's clock (<code>iggy-replay --virtual-clock --force-smooth --mipmaps</code>). "
         "The menus' text is bitmap glyphs 89 and 107 pixels high drawn at about a quarter of that: without mipmaps they "
         "alias, which is the sharpness and the steps.</p>",
         "<h2>The crops</h2>"]
    for crop, caption in CROPS:
        h.append(f"<div><b>{html.escape(caption)}</b><br>")
        for src, label in [(f"{BEFORE}/img/{crop}-iggy.png", "Iggy (original)"),
                           (f"{BEFORE}/img/{crop}-ruffle.png", "Ruffle before"),
                           (f"{AFTER}/img/{crop}-ruffle.png", "Ruffle after")]:
            h.append(f"<figure><img src={src}><figcaption>{label}</figcaption></figure>")
        h.append("</div>")
    h.append("<h2>Full screens (Iggy, before, after, the differences before and after)</h2>")
    for s in SCREENS:
        h.append(f"<div><b>{s}</b><br>")
        for src, label in [(f"{BEFORE}/img/{s}-iggy.png", "Iggy"), (f"{BEFORE}/img/{s}-ruffle.png", "Ruffle before"),
                           (f"{AFTER}/img/{s}-ruffle.png", "Ruffle after"), (f"{BEFORE}/img/{s}-diff.png", "difference before"),
                           (f"{AFTER}/img/{s}-diff.png", "difference after")]:
            h.append(f"<figure><img class=s src={src}><figcaption>{label}</figcaption></figure>")
        h.append(f"<div class=stat>before: {html.escape(stat(pilot, BEFORE, s))}<br>after: {html.escape(stat(pilot, AFTER, s))}</div></div>")
    h.append("<h2>Measurements: % of pixels off by more than 8 levels (of 255)</h2>")
    h.append("<p>Every variant replays the same log. Stock Ruffle runs paced to game time; the <code>fork-*</code> rows add "
             "<code>--virtual-clock</code> (not shown in the arguments column). quality: Ruffle's stage quality (MSAA "
             "samples: low 1, medium 2 (unsupported here, so 1), high and best 4, 8x8 8, 16x16 16, clamped by the "
             "adapter); ss k: drawn k times larger and box-filtered down.</p>")
    h.append(table(md))
    h.append("<script>document.querySelectorAll('img.s').forEach(i=>i.onclick=()=>i.classList.toggle('big'))</script>")
    (pilot / "round2.html").write_text("\n".join(h))
    print(pilot / "round2.html")


if __name__ == "__main__":
    main()

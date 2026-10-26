//! Continuation round batch 5 cases (`ffx_prender::h2_more11`), written
//! by hand: one per port, the same call as its shim.

use super::*;

use ffx_prender::h2_more11::bridge as h11;

/// The scene-wide set-up: the old definitions (inline, or storage apart
/// owned or borrowed), the entries' old name words (tagged or not) and
/// flag words, the layout size, the allocations (failing now and then),
/// the constant-buffer array's count (equal to `n` or not) and its
/// buffers' slot counts.
fn prep_scene_init(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    a[0] = [0, 1, 2, 3, 4, 7][rng.below(6) as usize];
    let data = e.rebase(0x00ca_8514);
    let count = e.rebase(0x00ca_8510);
    let inline = e.rebase(0x00ca_8330);
    let old = if rng.chance(3) { inline } else { e.ptr(rng) };
    e.w32(data, old);
    let c = rng.below(40) | [0, 0x8000_0000, 0x4000_0000][rng.below(3) as usize];
    let stride = e.rebase(0x00ca_8518);
    let junk = rng.next();
    e.w32(stride, junk);
    e.w32(count, c);
    for k in 0..30 {
        let entry = inline + 0x10 * k;
        let w = e.ptr(rng) | u32::from(rng.chance(2));
        e.w32(entry + 4, w);
        let f = rng.next();
        e.w32(entry + 8, f);
    }
    e.set_out(0x005a_0600, rng.below(0x400));
    let p = if rng.chance(6) { 0 } else { e.pool + POOL_PTRS };
    e.set_ret(0x0042_fb80, p);
    let array = e.rebase(0x00ca_8524);
    let n = a[0].wrapping_add(1) & !1;
    let cur = if rng.chance(2) { n } else { rng.below(9) };
    e.w32(array, cur);
    let buffers = e.pool + POOL_PTRS + 0x400;
    e.w32(array + 4, buffers);
    for k in 0..8 {
        let s = [0, 8, 3, 0x8000_0008][rng.below(4) as usize];
        e.w32(buffers + 60 * k + 0xc, s);
    }
    let r = u32::from(rng.chance(4));
    e.set_ret(0x0059_9a80, r);
}

pub static CASES: &[Case] = &[Case {
    name: "Phyre_PRendering_PSceneWideParameters_initialize",
    addr: 0x004f_d9e0,
    conv: K::C,
    kinds: "v",
    out: O::U32,
    callees: &[
        (0x004a_b4d0, 2, 0, false),
        (0x0094_9256, 3, 0, false),
        (0x0042_fb20, 1, 0, false),
        (0x0059_e530, 0, 0, false),
        (0x005a_0600, 5, 0, false),
        (0x0042_fb80, 2, 0, false),
        (0x0094_964c, 3, 0, false),
        (0x004f_d960, 2, 0, false),
        (0x004f_ed50, 2, 2, true),
        (0x0059_9a80, 1, 1, true),
        (0x0043_17f0, 0, 0, true),
    ],
    prep: prep_scene_init,
    port: |g, a| {
        let frame = g.frame();
        h11::initialize(g, a[0], frame)
    },
    frame: 0xc,
    gap: 0,
    writes: &[(0x005a_0600, 3)],
}];

//! List `p1-load`, continuation round 3, batch 3: `PCluster::
//! fixupInstances` (`0x00441620`).
//!
//! A cluster (as it uses it): `+8` the ring of its import names (a node
//! 0x2c bytes before its link `+0x2c`; `+0x18` the name of the asset it
//! stands for), `+0x1c` its instance lists (a ring through `+0`; `+0x2c`
//! a list's class descriptor), `+0x44` flags (bit 1: fixed up).

use ffx_graphic::host::{Conv, Host};

/// The cursor's seek and next (thiscall on the cursor, eight words:
/// element, chunk end, stride, count left, data offset, the list head,
/// the list, the class).
const CURSOR_SEEK: u32 = 0x0044_1b80;
const CURSOR_NEXT: u32 = 0x0044_1c60;
/// The asset lookup by name (cdecl `(name)`).
const FIND_ASSET: u32 = 0x0045_d280;
/// `PAssetReferenceImport`'s, `PAssetReference`'s, `PString`'s (the
/// import's name class), `PClassDescriptorDynamic`'s and `PCluster`'s
/// class descriptors (as this tells them).
const IMPORT_CLASS: u32 = 0x00c9_4030;
const REFERENCE_CLASS: u32 = 0x00c9_3f98;
const NAMED_CLASS: u32 = 0x00c9_0760;
const DYNAMIC_CLASS: u32 = 0x00c9_1330;
const SKIP_CLASS: u32 = 0x00c9_1180;

/// Whether `class` or one of its parents (`+0x40`) is `want`.
fn derives(h: &impl Host, class: u32, want: u32) -> bool {
    let mut c = class;
    while c != 0 {
        if c == want {
            return true;
        }
        c = h.read(c + 0x40);
    }
    false
}

/// The node of the cluster's import ring at `link`, or none at the head,
/// a null link or a null node.
fn import_of(head: u32, link: u32) -> Option<u32> {
    if link == head || link == 0 {
        return None;
    }
    let node = link.wrapping_sub(0x2c);
    (node != 0).then_some(node)
}

/// The next instance list after the link word `w` (none at the ring's
/// `head`).
fn list_of(head: u32, w: u32) -> u32 {
    if w == head { 0 } else { w }
}

/// Whether the C strings at `a` and `b` are equal.
fn same_name(h: &impl Host, a: u32, b: u32) -> bool {
    let mut i = 0u32;
    loop {
        let (x, y): (u8, u8) = (h.read(a.wrapping_add(i)), h.read(b.wrapping_add(i)));
        if x != y {
            return false;
        }
        if x == 0 {
            return true;
        }
        i = i.wrapping_add(1);
    }
}

/// The ports over game memory.
pub mod bridge {
    use super::{
        CURSOR_NEXT, CURSOR_SEEK, Conv, DYNAMIC_CLASS, FIND_ASSET, Host, IMPORT_CLASS, NAMED_CLASS,
        REFERENCE_CLASS, SKIP_CLASS, derives, import_of, list_of, same_name,
    };

    /// The asset import `node` stands for, when one of the cluster's
    /// `PAssetReferenceImport` instances (walked with the cursor `it`)
    /// has its name (a named class, the name word less bit 0) and the
    /// asset found by that name (`0x0045d280`) is a dynamic class:
    /// that asset's `+0x1c` less 0x18.
    fn replacement(h: &mut impl Host, cluster: u32, it: u32, name: u32) -> Option<u32> {
        let head = cluster + 0x1c;
        let first = list_of(head, h.read(head));
        let class = h.rebase(IMPORT_CLASS);
        h.write(it, [0, 0, 0, 0, 0, head, first, class]);
        h.call_sym(CURSOR_SEEK, Conv::Thiscall(it), &[]);
        let named = h.rebase(NAMED_CLASS);
        let dynamic = h.rebase(DYNAMIC_CLASS);
        let mut left: u32 = h.read(it + 0xc);
        while left != 0 {
            let mut cur: u32 = h.read(it);
            let mut end: u32 = h.read(it + 4);
            let mut check = true;
            loop {
                if check {
                    let off: u32 = h.read(it + 0x10);
                    let obj = off.wrapping_add(cur);
                    let d: u32 = h.read(obj);
                    if d != 0 && derives(h, d, named) {
                        let w: u32 = h.read(obj + 4);
                        let s = w & !1;
                        if s != 0 && same_name(h, s, name) {
                            let asset = h.call_sym(FIND_ASSET, Conv::Cdecl, &[s]);
                            if asset != 0 {
                                let ad: u32 = h.read(asset + 0x20);
                                if ad != 0 && derives(h, ad, dynamic) {
                                    return Some(h.read(asset + 0x1c));
                                }
                            }
                        }
                    }
                }
                let stride: u32 = h.read(it + 8);
                cur = cur.wrapping_add(stride);
                left = left.wrapping_sub(1);
                h.write(it, cur);
                h.write(it + 0xc, left);
                check = end != cur;
                if !check {
                    end = h.read(end);
                    h.write(it + 4, end);
                }
                if left == 0 {
                    break;
                }
            }
            h.call_sym(CURSOR_NEXT, Conv::Thiscall(it), &[]);
            left = h.read(it + 0xc);
        }
        None
    }

    /// `PCluster::fixupInstances(cluster)` (`0x00441620`, thiscall) ->
    /// 0, or the first error of a list's class: once (`+0x44` bit 1).
    /// Each import node with a name whose asset is a dynamic class
    /// object (found through the cluster's `PAssetReferenceImport`
    /// instances, walked with the cursor `it`: eight words, the
    /// original's frame) is replaced: the asset object (`+0x1c` less
    /// 0x18), unless null or the node itself, takes its place as the
    /// class of every instance list naming it (`+0x2c`), and the node is
    /// deleted (slot 7, `(1)`). Then each instance list whose class is
    /// not `PAssetReference`, `PAssetReferenceImport`,
    /// `PClassDescriptorDynamic` or `0x00c91180` gets its class's slot 14
    /// `(list)`; the first non-zero result is returned at once (the bit
    /// stays clear).
    pub fn fixup_instances(h: &mut impl Host, it: u32, cluster: u32) -> u32 {
        if h.read::<u8>(cluster + 0x44) & 2 != 0 {
            return 0;
        }
        let names = cluster + 8;
        let lists = cluster + 0x1c;
        let mut node = import_of(names, h.read(names));
        while let Some(cur) = node {
            node = import_of(names, h.read(cur + 0x2c));
            let name: u32 = h.read(cur + 0x18);
            if name == 0 {
                continue;
            }
            let Some(asset) = replacement(h, cluster, it, name) else {
                continue;
            };
            if asset == 0 {
                continue;
            }
            let obj = asset.wrapping_sub(0x18);
            if obj == 0 || obj == cur {
                continue;
            }
            let mut l = list_of(lists, h.read(lists));
            while l != 0 {
                if h.read::<u32>(l + 0x2c) == cur {
                    h.write(l + 0x2c, obj);
                }
                l = list_of(lists, h.read(l));
            }
            h.call_virtual(cur, 7, &[1]);
        }
        let skip = [REFERENCE_CLASS, IMPORT_CLASS, DYNAMIC_CLASS, SKIP_CLASS].map(|c| h.rebase(c));
        let mut l = list_of(lists, h.read(lists));
        while l != 0 {
            let class: u32 = h.read(l + 0x2c);
            if !skip.contains(&class) {
                let r = h.call_virtual(class, 14, &[l]);
                if r != 0 {
                    return r;
                }
            }
            l = list_of(lists, h.read(l));
        }
        let f: u32 = h.read(cluster + 0x44);
        h.write(cluster + 0x44, f | 2);
        0
    }
}

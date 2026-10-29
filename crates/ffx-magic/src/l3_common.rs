//! Small functions list 3 owns that several overlays share (internal:
//! the DLLs call them directly with LTCG register conventions). Each doc
//! names the DLL and address it was ported from, its size and how many
//! DLLs carry it.

use crate::runtime::slot_target;
use crate::{Game, Image, Sig};

/// Fades a character (`this` in `ecx`: `+0x0` the character's record,
/// `+0x4` an `f32` alpha, `+0x8` a word): for a monster (record byte
/// `+0x90` bit 0) `Ch_SetHideEffect(+0x54, 0)` (slot 257) and
/// `Ch_SetTransparent(+0x54, alpha, word)` (slot 38), else
/// `MsSetChrVisible(+0x50, 1)` (slot 296) and
/// `MsSetChrTransparent(+0x50, alpha, word)` (slot 541). When the alpha
/// equals the constant at operand `+0x83` (`ucomiss`: equal and ordered),
/// the record's `+0x58` gets the word and its `+0x90` bit 1 is set.
/// Returns `eax`: the word on that path, else the last slot's result
/// with `lahf`'s flags in `ah`.
///
/// `magic_0159` 0x100023f0 (159 bytes, 7 overlays).
#[allow(clippy::float_cmp)] // ucomiss: exact equality
pub fn chr_fade<G: Game>(g: &mut G, img: &Image, f: u32, this: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let rec = g.read::<u32>(this);
    let mut eax = if g.read::<u8>(rec + 0x90) & 1 != 0 {
        let s257 = slot_target(g, w(0x14), 257);
        g.call(s257, this, &[g.read::<u32>(rec + 0x54), 0]);
        let rec = g.read::<u32>(this);
        let args = [
            g.read::<u32>(rec + 0x54),
            g.read::<u32>(this + 4),
            g.read::<u32>(this + 8),
        ];
        let s38 = slot_target(g, w(0x37), 38);
        g.call(s38, this, &args).eax
    } else {
        let s296 = slot_target(g, w(0x49), 296);
        g.call(s296, this, &[g.read::<u32>(rec + 0x50), 1]);
        let rec = g.read::<u32>(this);
        let args = [
            g.read::<u32>(rec + 0x50),
            g.read::<u32>(this + 4),
            g.read::<u32>(this + 8),
        ];
        let s541 = slot_target(g, w(0x6c), 541);
        g.call(s541, this, &args).eax
    };
    let alpha = g.read::<f32>(this + 4);
    let k = g.read::<f32>(w(0x83));
    // `lahf` puts ucomiss's flags into ah: ZF (0x40), PF (0x04) and CF
    // (0x01) as the comparison sets them, bit 1 always
    let ah: u32 = match alpha.partial_cmp(&k) {
        None => 0x47,
        Some(std::cmp::Ordering::Less) => 0x03,
        Some(std::cmp::Ordering::Equal) => 0x42,
        Some(std::cmp::Ordering::Greater) => 0x02,
    };
    eax = (eax & 0xffff_00ff) | (ah << 8);
    if alpha == k {
        let rec = g.read::<u32>(this);
        eax = g.read::<u32>(this + 8);
        g.write::<u32>(rec + 0x58, eax);
        let flags = g.read::<u16>(rec + 0x90);
        g.write::<u16>(rec + 0x90, flags | 2);
    }
    eax
}

/// [`chr_fade`].
pub const CHR_FADE: Sig = Sig {
    len: 159,
    hash: 0x9f1e_5a66_149a_22fb,
    calls: &[],
    imports: &[],
};

/// Unlinks the current node (`G_CUR`, operand `+0x02`) of a doubly linked
/// list whose head sentinel holds the first (`G_FIRST`, `+0x15`) and last
/// (`G_LAST`, `+0x1a`) pointers: drops the node's use count (`+0x18`,
/// a word), links its neighbours past it (the node's own `+0x10`/`+0xc`
/// when a neighbour is the node itself, else the neighbour's `+0x8`/
/// `+0x4`), resets both ends to the sentinel (immediates `+0x47`, `+0x51`)
/// and clears `G_CUR`. Returns `eax`: `G_FIRST` as read the second time,
/// or the caller's `eax` when there is no current node.
///
/// `magic_0159` 0x10006590 (96 bytes, 16 overlays; twice in some).
pub fn list_unlink_current<G: Game>(g: &mut G, img: &Image, f: u32, eax: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let node = g.read::<u32>(w(0x02));
    if node == 0 {
        return eax;
    }
    let uses = g.read::<u16>(node + 0x18);
    g.write::<u16>(node + 0x18, uses.wrapping_add(0xffff));
    let first = g.read::<u32>(w(0x15));
    let last = g.read::<u32>(w(0x1a));
    if first == node {
        g.write::<u32>(node + 0x10, last);
    } else {
        g.write::<u32>(first + 0x8, last);
    }
    let last = g.read::<u32>(w(0x2c));
    let first = g.read::<u32>(w(0x31));
    if last == node {
        g.write::<u32>(node + 0xc, first);
    } else {
        g.write::<u32>(last + 0x4, first);
    }
    g.write::<u32>(w(0x43), w(0x47));
    g.write::<u32>(w(0x4d), w(0x51));
    g.write::<u32>(w(0x57), 0);
    first
}

/// [`list_unlink_current`].
pub const LIST_UNLINK_CURRENT: Sig = Sig {
    len: 96,
    hash: 0x1b0d_5a05_b1b8_b2a7,
    calls: &[],
    imports: &[],
};

/// Shows the effect's character: `Ch_SetHideEffect(G_CHR, 1)` (slot 257,
/// the character at operand `+0x18`) when the flag byte (operand `+0x02`)
/// has bit 0, else `MsSetChrVisible(G_ID, 0)` (slot 296, `+0x2c`).
/// Returns the slot's `eax`.
///
/// `magic_0159` 0x10006680 (54 bytes, 2 overlays).
pub fn chr_show<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    if g.read::<u8>(w(0x02)) & 1 != 0 {
        let s257 = slot_target(g, w(0x08), 257);
        let chr = g.read::<u32>(w(0x18));
        g.call(s257, 0, &[chr, 1]).eax
    } else {
        let s296 = slot_target(g, w(0x08), 296);
        let id = g.read::<u32>(w(0x2c));
        g.call(s296, 0, &[id, 0]).eax
    }
}

/// [`chr_show`].
pub const CHR_SHOW: Sig = Sig {
    len: 54,
    hash: 0x5d9a_70cf_4553_0ab5,
    calls: &[],
    imports: &[],
};

/// Takes a node from a free list (count `G_N`, a word at operand `+0x04`;
/// head `G_FREE`, `+0x13`): unlinks the head from its ring (`+0x0` next,
/// `+0x4` previous), moves `G_FREE` to the previous node (or 0 for the
/// last one), decrements the count and resets the node: `+0x0`, `+0x4`,
/// `+0xc`, `+0x14` cleared, `+0x8` itself, `+0x10` the callback at
/// operand `+0x70`, byte `+0x18` cleared. Returns the node, or 0 when the
/// list is empty.
///
/// `magic_0166` 0x10002680 (124 bytes, 2 overlays).
pub fn pool_take<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let n = g.read::<u16>(w(0x04));
    if n == 0 {
        return 0;
    }
    let node = g.read::<u32>(w(0x13));
    let next = g.read::<u32>(node);
    let prev = g.read::<u32>(node + 4);
    g.write::<u32>(next + 4, prev);
    let prev = g.read::<u32>(node + 4);
    let next = g.read::<u32>(node);
    g.write::<u32>(prev, next);
    if n > 1 {
        let prev = g.read::<u32>(node + 4);
        g.write::<u32>(w(0x30), prev);
    } else {
        g.write::<u32>(w(0x38), 0);
    }
    g.write::<u16>(w(0x4b), n.wrapping_add(0xffff));
    g.write::<u32>(node, 0);
    g.write::<u32>(node + 4, 0);
    g.write::<u32>(node + 0xc, 0);
    g.write::<u32>(node + 0x14, 0);
    g.write::<u32>(node + 8, node);
    g.write::<u32>(node + 0x10, w(0x70));
    g.write::<u8>(node + 0x18, 0);
    node
}

/// [`pool_take`].
pub const POOL_TAKE: Sig = Sig {
    len: 124,
    hash: 0xe49b_1b56_333b_21b3,
    calls: &[],
    imports: &[],
};

/// Where a node list's words are: the last node (read twice, written
/// once: three operands in the static lists), the sentinel the last
/// pointer holds when the list is empty, the first node and the node
/// count (a word).
#[derive(Debug, Clone, Copy)]
pub struct ListAt {
    pub last: u32,
    pub last2: u32,
    pub sentinel: u32,
    pub first: u32,
    pub count: u32,
    pub last_w: u32,
}

impl ListAt {
    /// A list object at `list`: `+0xc` first, `+0x10` last, `+0x18`
    /// count; the object is its own sentinel.
    #[must_use]
    pub fn of(list: u32) -> ListAt {
        ListAt {
            last: list + 0x10,
            last2: list + 0x10,
            sentinel: list,
            first: list + 0xc,
            count: list + 0x18,
            last_w: list + 0x10,
        }
    }
}

/// A node's own list fields: `+0x1a` (word) = 0xff, `+0x10` and `+0xc`
/// itself (its own children list, empty), `+0x14` its owner list.
pub fn node_init<G: Game>(g: &mut G, node: u32, list: u32) {
    g.write::<u16>(node + 0x1a, 0xff);
    g.write::<u32>(node + 0x10, node);
    g.write::<u32>(node + 0xc, node);
    g.write::<u32>(node + 0x14, list);
}

/// Appends `node` to a list: `+0x4` = `list`, `+0x8` = the old last
/// node; the old last's `+0x4` (or the list's first word, when empty)
/// gets the node; the count goes up and the node is the last. Returns
/// the old last node (as read the second time: the original's `eax`).
#[allow(clippy::similar_names)] // `last`/`last2`: the two reads
pub fn attach<G: Game>(g: &mut G, node: u32, list: u32, at: ListAt) -> u32 {
    g.write::<u32>(node + 4, list);
    let last = g.read::<u32>(at.last);
    g.write::<u32>(node + 8, last);
    let last = g.read::<u32>(at.last2);
    if last == at.sentinel {
        g.write::<u32>(at.first, node);
    } else {
        g.write::<u32>(last + 4, node);
    }
    let n = g.read::<u16>(at.count);
    g.write::<u16>(at.count, n.wrapping_add(1));
    g.write::<u32>(at.last_w, node);
    last
}

/// Takes a node (`+0x14` its list, `+0x4`/`+0x8` next and previous) out
/// of its list, when it is in one: the count drops, the neighbours are
/// linked past it (the list's own last/first words when a neighbour is
/// the list), and the node points at itself with no list.
pub fn detach<G: Game>(g: &mut G, node: u32) {
    let list = g.read::<u32>(node + 0x14);
    if list == 0 {
        return;
    }
    let n = g.read::<u16>(list + 0x18);
    g.write::<u16>(list + 0x18, n.wrapping_add(0xffff));
    let prev = g.read::<u32>(node + 8);
    let next = g.read::<u32>(node + 4);
    if next == list {
        g.write::<u32>(list + 0x10, prev);
    } else {
        g.write::<u32>(next + 8, prev);
    }
    let prev = g.read::<u32>(node + 8);
    let next = g.read::<u32>(node + 4);
    if prev == list {
        g.write::<u32>(list + 0xc, next);
    } else {
        g.write::<u32>(prev + 4, next);
    }
    g.write::<u32>(node + 8, node);
    g.write::<u32>(node + 4, node);
    g.write::<u32>(node + 0x14, 0);
}

/// Constructs a node (`ecx`) of the static list at operand `+0x0f`
/// ([`node_init`], [`attach`]: `+0x16` the list word, last `+0x1b`/`+0x23`,
/// sentinel `+0x28`, first `+0x30`, count `+0x3c`, last written through
/// `+0x42`): `+0x28` = 1.0, byte `+0x2d` cleared, vtable at operand
/// `+0x53` last. Returns the old last node (`eax`).
///
/// `magic_0153` 0x100079d0 (88 bytes, 2 overlays).
pub fn scale_node<G: Game>(g: &mut G, img: &Image, f: u32, this: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    node_init(g, this, w(0x0f));
    let eax = attach(
        g,
        this,
        w(0x16),
        ListAt {
            last: w(0x1b),
            last2: w(0x23),
            sentinel: w(0x28),
            first: w(0x30),
            count: w(0x3c),
            last_w: w(0x42),
        },
    );
    g.write::<u32>(this + 0x28, 0x3f80_0000);
    g.write::<u8>(this + 0x2d, 0);
    g.write::<u32>(this, w(0x53));
    eax
}

/// [`scale_node`].
pub const SCALE_NODE: Sig = Sig {
    len: 88,
    hash: 0x0670_2f8a_c55e_8e99,
    calls: &[],
    imports: &[],
};

/// Constructs a node from a set-up record (`ecx`: `+0x0` the node, `+0x4`
/// its list or 0, `+0x8`/`+0xc` two words): [`node_init`], [`attach`]
/// when there is a list; `+0x2c`/`+0x30` the record's two words,
/// `+0x34..+0x3c` cleared, word `+0x4c` and byte `+0x4e` cleared, the
/// vtable at operand `+0x65`. Returns the record's `+0xc` (`eax`).
///
/// `magic_0153` 0x100047a0 (107 bytes, 2 overlays).
pub fn record_node<G: Game>(g: &mut G, img: &Image, f: u32, this: u32) -> u32 {
    let node = g.read::<u32>(this);
    let list = g.read::<u32>(this + 4);
    node_init(g, node, list);
    if list != 0 {
        attach(g, node, list, ListAt::of(list));
    }
    g.write::<u32>(node + 0x2c, g.read::<u32>(this + 8));
    let eax = g.read::<u32>(this + 0xc);
    g.write::<u32>(node + 0x30, eax);
    g.write::<u32>(node + 0x34, 0);
    g.write::<u32>(node + 0x38, 0);
    g.write::<u32>(node + 0x3c, 0);
    g.write::<u16>(node + 0x4c, 0);
    g.write::<u8>(node + 0x4e, 0);
    g.write::<u32>(node, img.word(f + 0x65));
    eax
}

/// [`record_node`].
pub const RECORD_NODE: Sig = Sig {
    len: 107,
    hash: 0x4b98_630b_7404_c70f,
    calls: &[],
    imports: &[],
};

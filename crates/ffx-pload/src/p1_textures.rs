//! List `p1-load`, round 5: `PBinary::loadClusterTextures` (`0x00465ab0`,
//! thiscall `(loader; stream, cluster, name)`, `ret 0xc`, `eax` 0 or an
//! error). `PBinary::loadCluster` reaches it through the thunk
//! `0x00465aa0` after the cluster's body is read.
//!
//! The loader (`0x00463d10`/`0x004647a0`) holds the byte counts of the
//! data that follows the cluster in the stream: `+0x14` the data blocks',
//! `+0x18` the index blocks', `+0x1c` the textures'. When any is set,
//! under the render scope lock (`0x00463d50`/`0x00463e90`, one word in the
//! frame):
//! 1. the index blocks' bytes are read into a buffer (`PhyreMalloc(n,
//!    0x10)`, `readExact`), then each instance of the classes `0x00cb1120`
//!    (the object) and `0x00cb0ff0` (its `+0x30`) is created
//!    (`PIndexDataBlockD3D11_create`) and its `+0x2c` cleared; the
//!    buffer freed;
//! 2. the same for the data blocks (class `0x00cb1250`,
//!    `PDataBlockD3D11_create`, `+0x30` cleared);
//! 3. the textures: a buffer of twice their bytes, the render interface
//!    `0x00c94f00` made once (guard `0x00c9628c` bit 0, its `atexit`), the
//!    texture quality `q` (`getTextureQuality`). The 2D (`0x00ca37d0`), 3D
//!    (`0x00ca3b60`) and cube (`0x00ca3ef0`) textures are counted, and
//!    whether any has mip levels (`+0xc`, levels less one). When one has,
//!    each texture's header (0x74 / 0x68 / 0x68 bytes) is copied into an
//!    array per kind and the arrays, the counts and a copy of `name` go to
//!    the render interface's reload registry (`0x0058c010`), for
//!    `setTextureQuality`'s reloads. Each texture's levels are read in
//!    turn into the buffer; the top `q` levels are dropped (read into the
//!    same place, not kept) when the texture has at least `q` levels
//!    less one. For a 2D texture `q` first backs off one level at a time
//!    while the top level shifted by `q` is under 4 wide or high, and the
//!    smaller `q` stays for every texture after it (the original keeps it
//!    in one frame word). The texture's dimensions are then set to the
//!    smaller size, it is created from the buffer, and bound unless it
//!    already is; a texture whose format is `0x00ca34cc` is error 7.
//!
//! Then, without the lock, each texture object (class `0x00ca2930`) is
//! registered (`0x0058cf80`), and the render state blocks (`0x00cb34e0`),
//! the vertex and fragment programs (`0x00cb3740`, `0x00cb37d8`) and the
//! instances of `0x00cb3870` and `0x00cb3908` created; the first error is
//! returned.
//!
//! The instance walks: the iterator (eight words at `ebp - 0x30`: element,
//! chunk end, stride, count left, data offset, the list head, the list,
//! the class) is set up and sought with the original's per-class copy of
//! the seek. Some walks step in the frame and go on to the next list with
//! the matching copy of the iterator's next; the others keep the cursor in
//! registers and inline the next list's search ([`Walk`]). The port does
//! each as its original does.

use ffx_graphic::host::{Conv, Host};

const SCOPE_ENTER: u32 = 0x0046_3d50;
const SCOPE_LEAVE: u32 = 0x0046_3e90;
const MALLOC: u32 = 0x0042_fb80;
const FREE: u32 = 0x0042_fb20;
const READ_EXACT: u32 = 0x0057_dd30;
const ITER_BEGIN: u32 = 0x0046_38f0;
const INDEX_CREATE: u32 = 0x0056_1e90;
const DATA_CREATE: u32 = 0x0056_20c0;
const RENDER_INTERFACE: u32 = 0x00c9_4f00;
const RENDER_INTERFACE_GUARD: u32 = 0x00c9_628c;
const RENDER_INTERFACE_CTOR: u32 = 0x004f_ee70;
const RENDER_INTERFACE_DTOR: u32 = 0x00af_b710;
const ATEXIT: u32 = 0x0094_9207;
const GET_TEXTURE_QUALITY: u32 = 0x0059_47d0;
const REGISTER_RELOAD: u32 = 0x0058_c010;
/// The format no texture may have (error 7).
const BAD_FORMAT: u32 = 0x00ca_34cc;
const BAD_FORMAT_ERROR: u32 = 7;

/// A texture kind's class, seek, header bytes and calls: set the smaller
/// size, create, whether bound, bind.
struct Kind {
    class: u32,
    seek: u32,
    header: u32,
    resize: u32,
    create: u32,
    bound: u32,
    bind: u32,
}

const TEXTURE_2D: Kind = Kind {
    class: 0x00ca_37d0,
    seek: 0x0046_4f30,
    header: 0x74,
    resize: 0x004c_40c0,
    create: 0x0058_fc20,
    bound: 0x0059_5f90,
    bind: 0x0058_db00,
};
const TEXTURE_3D: Kind = Kind {
    class: 0x00ca_3b60,
    seek: 0x0046_4fa0,
    header: 0x68,
    resize: 0x004c_5980,
    create: 0x0059_03a0,
    bound: 0x0059_5fa0,
    bind: 0x0058_db50,
};
const TEXTURE_CUBE: Kind = Kind {
    class: 0x00ca_3ef0,
    seek: 0x0046_5010,
    header: 0x68,
    resize: 0x004c_7260,
    create: 0x0059_0800,
    bound: 0x0059_5fb0,
    bind: 0x0058_dba0,
};
/// The 2D and 3D counts' copies of the iterator's next.
const NEXT_2D: u32 = 0x0046_53e0;
const NEXT_3D: u32 = 0x0046_5440;

/// The index blocks: next (`ITER_BEGIN` seeks class `0x00cb1120`), then
/// the `+0x30` blocks' class, seek and next.
const INDEX_NEXT: u32 = 0x0046_50e0;
const INDEX_INNER_CLASS: u32 = 0x00cb_0ff0;
const INDEX_INNER_SEEK: u32 = 0x0046_4c20;
const INDEX_INNER_NEXT: u32 = 0x0046_5140;
/// The data blocks.
const DATA_CLASS: u32 = 0x00cb_1250;
const DATA_SEEK: u32 = 0x0046_4b40;
const DATA_NEXT: u32 = 0x0046_5080;

/// The device objects created after the textures: class, seek, next
/// (`None`: the walk inlines it), create.
const DEVICE_OBJECTS: [(u32, u32, Option<u32>, u32); 6] = [
    (0x00ca_2930, 0x0046_4c90, None, 0x0058_cf80),
    (0x00cb_34e0, 0x0046_4e50, None, 0x0058_d4e0),
    (0x00cb_3740, 0x0046_4ec0, Some(0x0046_5380), 0x0058_d780),
    (0x00cb_37d8, 0x0046_4d70, Some(0x0046_5260), 0x0058_d1c0),
    (0x00cb_3870, 0x0046_4de0, Some(0x0046_52c0), 0x0058_d350),
    (0x00cb_3908, 0x0046_4d00, Some(0x0046_5200), 0x0058_d030),
];

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

/// The iterator's cursor as the inlined walks keep it (in registers and
/// spill words the callees never see).
struct Walk {
    cur: u32,
    chunk: u32,
    stride: u32,
    left: u32,
    offset: u32,
    head: u32,
    list: u32,
    class: u32,
}

impl Walk {
    /// The cursor of the iterator at `it` (after its seek).
    fn of(h: &impl Host, it: u32) -> Self {
        let w: [u32; 8] = h.read(it);
        Self {
            cur: w[0],
            chunk: w[1],
            stride: w[2],
            left: w[3],
            offset: w[4],
            head: w[5],
            list: w[6],
            class: w[7],
        }
    }

    /// The current element's object.
    fn object(&self) -> u32 {
        self.offset.wrapping_add(self.cur)
    }

    /// Past the current element (the original's step: one stride, and
    /// at a chunk's end the next chunk and one more stride); then the
    /// next list when this one is done. Whether there is an element.
    fn step(&mut self, h: &impl Host) -> bool {
        loop {
            self.cur = self.cur.wrapping_add(self.stride);
            self.left = self.left.wrapping_sub(1);
            if self.chunk == self.cur {
                self.chunk = h.read(self.chunk);
                if self.left != 0 {
                    continue;
                }
            } else if self.left != 0 {
                return true;
            }
            return self.next_list(h);
        }
    }

    /// The inlined search for the next list of the class with instances
    /// (`+0x40`), and its first element. Unlike the iterator's next it
    /// ends the walk at a list whose count (`+0x3c`) is 0.
    fn next_list(&mut self, h: &impl Host) -> bool {
        loop {
            if self.list == 0 {
                return false;
            }
            let w: u32 = h.read(self.list);
            self.list = if w == self.head { 0 } else { w };
            if self.list == 0 {
                return false;
            }
            let class: u32 = h.read(self.list + 0x2c);
            if derives(h, class, self.class) && h.read::<u32>(self.list + 0x40) != 0 {
                break;
            }
        }
        let class: u32 = h.read(self.list + 0x2c);
        self.left = h.read(self.list + 0x3c);
        self.stride = h.read(class + 0x1c);
        self.offset = h.read(class + 0x8c);
        self.cur = h.read(self.list + 0x34);
        self.chunk = h.read(self.list + 8);
        if self.left == 0 {
            return false;
        }
        while self.chunk == self.cur {
            self.chunk = h.read(self.chunk);
            self.cur = self.cur.wrapping_add(self.stride);
            self.left = self.left.wrapping_sub(1);
            if self.left == 0 {
                return false;
            }
        }
        true
    }
}

/// The ports over game memory.
pub mod bridge {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// The frame's bytes the port keeps at the original's offsets: the
    /// scope lock's word at `ebp - 0x60` up to the iterator's last word
    /// at `ebp - 0x14`.
    pub const FRAME: u32 = 0x60;
    /// Where `ebp` is in it.
    const EBP: u32 = 0x60;

    /// A callee's `al`.
    fn al(eax: u32) -> bool {
        eax & 0xff != 0
    }

    /// `PhyreMalloc(n, 0x10)`.
    fn malloc(h: &mut impl Host, n: u32) -> u32 {
        h.call_sym(MALLOC, Conv::Cdecl, &[n, 0x10])
    }

    /// The iterator at `it` over the cluster's lists (head `head`) for
    /// `class`, sought with `seek`.
    fn begin(h: &mut impl Host, it: u32, head: u32, class: u32, seek: u32) {
        let w: u32 = h.read(head);
        let first = if w == head { 0 } else { w };
        let class = h.rebase(class);
        h.write(it, [0, 0, 0, 0, 0, head, first, class]);
        h.call_sym(seek, Conv::Thiscall(it), &[]);
    }

    /// A walk that steps in the frame's iterator and goes on with the
    /// iterator's `next`: `visit(cur, offset)` on each element; its first
    /// error ends the walk.
    fn walk_frame<H: Host>(
        h: &mut H,
        it: u32,
        next: u32,
        mut visit: impl FnMut(&mut H, u32, u32) -> Result<(), u32>,
    ) -> Result<(), u32> {
        loop {
            let mut left: u32 = h.read(it + 0xc);
            if left == 0 {
                return Ok(());
            }
            'element: loop {
                let cur: u32 = h.read(it);
                let offset: u32 = h.read(it + 0x10);
                visit(h, cur, offset)?;
                loop {
                    let stride: u32 = h.read(it + 8);
                    let cur = h.read::<u32>(it).wrapping_add(stride);
                    left = left.wrapping_sub(1);
                    h.write(it, cur);
                    h.write(it + 0xc, left);
                    let chunk: u32 = h.read(it + 4);
                    if chunk == cur {
                        let next_chunk: u32 = h.read(chunk);
                        h.write(it + 4, next_chunk);
                        if left != 0 {
                            continue;
                        }
                    } else if left != 0 {
                        continue 'element;
                    }
                    break 'element;
                }
            }
            h.call_sym(next, Conv::Thiscall(it), &[]);
        }
    }

    /// A walk with the cursor inlined ([`Walk`]): `visit(object)` on each
    /// element; its first error ends the walk.
    fn walk_inline<H: Host>(
        h: &mut H,
        it: u32,
        mut visit: impl FnMut(&mut H, u32) -> Result<(), u32>,
    ) -> Result<(), u32> {
        let mut w = Walk::of(h, it);
        if w.left == 0 {
            return Ok(());
        }
        loop {
            visit(h, w.object())?;
            if !w.step(h) {
                return Ok(());
            }
        }
    }

    /// `loadClusterTextures(loader; stream, cluster, name)`: 0 or the
    /// error. `frame` is [`FRAME`] bytes at the original's `ebp - 0x60`.
    pub fn load_cluster_textures(
        h: &mut impl Host,
        loader: u32,
        [stream, cluster, name]: [u32; 3],
        frame: u32,
    ) -> u32 {
        let ebp = frame + EBP;
        let it = ebp - 0x30;
        let lock = ebp - 0x60;
        let head = cluster.wrapping_add(0x1c);
        let data: u32 = h.read(loader + 0x14);
        let index: u32 = h.read(loader + 0x18);
        let textures: u32 = h.read(loader + 0x1c);
        if textures | data | index != 0 {
            h.call_sym(SCOPE_ENTER, Conv::Thiscall(lock), &[]);
            let r = blocks_and_textures(h, (stream, cluster, name), it, (index, data, textures));
            h.call_sym(SCOPE_LEAVE, Conv::Thiscall(lock), &[]);
            if let Err(r) = r {
                return r;
            }
        }
        for (class, seek, next, create) in DEVICE_OBJECTS {
            begin(h, it, head, class, seek);
            let r = match next {
                Some(next) => walk_frame(h, it, next, |h, cur, off| {
                    create_one(h, create, off.wrapping_add(cur))
                }),
                None => walk_inline(h, it, |h, obj| create_one(h, create, obj)),
            };
            if let Err(r) = r {
                return r;
            }
        }
        0
    }

    /// A device object's create (thiscall on `obj`): its error, if any.
    fn create_one(h: &mut impl Host, create: u32, obj: u32) -> Result<(), u32> {
        match h_call(h, create, obj) {
            0 => Ok(()),
            r => Err(r),
        }
    }

    /// A thiscall without arguments on `obj`.
    fn h_call(h: &mut impl Host, sym: u32, obj: u32) -> u32 {
        h.call_sym(sym, Conv::Thiscall(obj), &[])
    }

    /// Steps 1 to 3, under the lock.
    fn blocks_and_textures(
        h: &mut impl Host,
        (stream, cluster, name): (u32, u32, u32),
        it: u32,
        (index, data, textures): (u32, u32, u32),
    ) -> Result<(), u32> {
        let head = cluster.wrapping_add(0x1c);
        if index != 0 {
            let buf = malloc(h, index);
            read(h, stream, buf, index)?;
            h.call_sym(ITER_BEGIN, Conv::Thiscall(it), &[cluster]);
            let _ = walk_frame(h, it, INDEX_NEXT, |h, cur, off| {
                let obj = off.wrapping_add(cur);
                h.call_sym(INDEX_CREATE, Conv::Thiscall(obj), &[buf]);
                h.write(obj.wrapping_add(0x2c), 0u32);
                Ok(())
            });
            begin(h, it, head, INDEX_INNER_CLASS, INDEX_INNER_SEEK);
            let _ = walk_frame(h, it, INDEX_INNER_NEXT, |h, cur, off| {
                let obj = cur.wrapping_add(0x30).wrapping_add(off);
                h.call_sym(INDEX_CREATE, Conv::Thiscall(obj), &[buf]);
                h.write(off.wrapping_add(cur).wrapping_add(0x5c), 0u32);
                Ok(())
            });
            h.call_sym(FREE, Conv::Cdecl, &[buf]);
        }
        if data != 0 {
            let buf = malloc(h, data);
            read(h, stream, buf, data)?;
            begin(h, it, head, DATA_CLASS, DATA_SEEK);
            let _ = walk_frame(h, it, DATA_NEXT, |h, cur, off| {
                let obj = off.wrapping_add(cur);
                h.call_sym(DATA_CREATE, Conv::Thiscall(obj), &[buf]);
                h.write(obj.wrapping_add(0x30), 0u32);
                Ok(())
            });
            h.call_sym(FREE, Conv::Cdecl, &[buf]);
        }
        if textures != 0 {
            load_textures(h, (stream, cluster, name), it, textures)?;
        }
        Ok(())
    }

    /// `readExact(stream; buf, n)`: the error, if any.
    fn read(h: &mut impl Host, stream: u32, buf: u32, n: u32) -> Result<(), u32> {
        match h.call_sym(READ_EXACT, Conv::Thiscall(stream), &[buf, n]) {
            0 => Ok(()),
            r => Err(r),
        }
    }

    /// The texture pass's state shared by the three kinds.
    struct Pass {
        stream: u32,
        /// The levels' buffer.
        buf: u32,
        /// The top levels to drop (the 2D textures may lower it).
        quality: u32,
        /// Whether the headers are kept (some texture has mip levels).
        keep: bool,
    }

    /// The count passes: how many textures of each kind, and whether any
    /// has mip levels.
    fn count_textures(h: &mut impl Host, it: u32, head: u32) -> ([u32; 3], bool) {
        let mut keep = false;
        let mut counts = [0u32; 3];
        for (k, (kind, next)) in [
            (&TEXTURE_2D, Some(NEXT_2D)),
            (&TEXTURE_3D, Some(NEXT_3D)),
            (&TEXTURE_CUBE, None),
        ]
        .into_iter()
        .enumerate()
        {
            begin(h, it, head, kind.class, kind.seek);
            let n = &mut counts[k];
            let keep = &mut keep;
            let _ = match next {
                Some(next) => walk_frame(h, it, next, |h, cur, off| {
                    count(h, off.wrapping_add(cur), n, keep);
                    Ok(())
                }),
                None => walk_inline(h, it, |h, obj| {
                    count(h, obj, n, keep);
                    Ok(())
                }),
            };
        }
        (counts, keep)
    }

    /// Step 3.
    fn load_textures(
        h: &mut impl Host,
        (stream, cluster, name): (u32, u32, u32),
        it: u32,
        bytes: u32,
    ) -> Result<(), u32> {
        let head = cluster.wrapping_add(0x1c);
        let buf = malloc(h, bytes.wrapping_add(bytes));
        let guard = h.rebase(RENDER_INTERFACE_GUARD);
        let flags: u32 = h.read(guard);
        let ri = h.rebase(RENDER_INTERFACE);
        if flags & 1 == 0 {
            h.write(guard, flags | 1);
            h.call_sym(RENDER_INTERFACE_CTOR, Conv::Thiscall(ri), &[]);
            let dtor = h.rebase(RENDER_INTERFACE_DTOR);
            h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
        }
        let quality = h.call_sym(GET_TEXTURE_QUALITY, Conv::Thiscall(ri), &[]);
        let stream_word: u32 = h.read(stream.wrapping_add(0xc));
        let (counts, keep) = count_textures(h, it, head);
        let mut copies = [0u32; 3];
        for (k, (kind, n)) in [&TEXTURE_2D, &TEXTURE_3D, &TEXTURE_CUBE]
            .into_iter()
            .zip(counts)
            .enumerate()
        {
            if keep && n != 0 {
                copies[k] = malloc(h, n.wrapping_mul(kind.header));
            }
        }
        let mut copy_name = 0;
        if keep {
            let mut s = Vec::new();
            loop {
                let c: u8 = h.read(name.wrapping_add(s.len() as u32));
                s.push(c);
                if c == 0 {
                    break;
                }
            }
            copy_name = malloc(h, s.len() as u32);
            if copy_name != 0 {
                h.write_bytes(copy_name, &s);
            }
        }
        let mut pass = Pass {
            stream,
            buf,
            quality,
            keep,
        };
        let mut cursors = copies;
        for (k, kind) in [&TEXTURE_2D, &TEXTURE_3D, &TEXTURE_CUBE]
            .into_iter()
            .enumerate()
        {
            begin(h, it, head, kind.class, kind.seek);
            walk_inline(h, it, |h, obj| {
                if pass.keep {
                    let mut header = vec![0u8; kind.header as usize];
                    h.read_bytes(obj, &mut header);
                    h.write_bytes(cursors[k], &header);
                    cursors[k] = cursors[k].wrapping_add(kind.header);
                }
                match k {
                    0 => texture_2d(h, &mut pass, obj),
                    1 => texture_3d(h, &pass, obj),
                    _ => texture_cube(h, &pass, obj),
                }
            })?;
        }
        h.call_sym(FREE, Conv::Cdecl, &[buf]);
        if keep {
            h.call_sym(
                REGISTER_RELOAD,
                Conv::Thiscall(ri),
                &[
                    copy_name,
                    cluster,
                    stream_word,
                    counts[0],
                    copies[0],
                    counts[1],
                    copies[1],
                    counts[2],
                    copies[2],
                    bytes,
                ],
            );
        }
        Ok(())
    }

    /// The count passes' visit: one more texture, and whether it has mip
    /// levels.
    fn count(h: &impl Host, obj: u32, n: &mut u32, keep: &mut bool) {
        if h.read::<u32>(obj.wrapping_add(0xc)) != 0 {
            *keep = true;
        }
        *n = n.wrapping_add(1);
    }

    /// One level's bytes: `bits` per pixel (the format's `+0xc`) over
    /// `w` x `h`, both rounded up to 4 for a block format (`+0x10` bit 0).
    fn level_bytes(h: &impl Host, format: u32, mut w: u32, mut ht: u32) -> u32 {
        if h.read::<u8>(format.wrapping_add(0x10)) & 1 != 0 {
            w = w.wrapping_add(3) & !3;
            ht = ht.wrapping_add(3) & !3;
        }
        let bits: u32 = h.read(format.wrapping_add(0xc));
        bits.wrapping_mul(w).wrapping_mul(ht) >> 3
    }

    /// Reads level `k` of a texture with `mips` levels less one into the
    /// buffer at `*dst`; a kept level moves `*dst` past it.
    fn read_level(
        h: &mut impl Host,
        pass: &Pass,
        dst: &mut u32,
        (k, mips): (u32, u32),
        n: u32,
    ) -> Result<(), u32> {
        read(h, pass.stream, *dst, n)?;
        if !(k < pass.quality && mips >= pass.quality) {
            *dst = dst.wrapping_add(n);
        }
        Ok(())
    }

    /// The end of each texture: created from the buffer, error 7 for the
    /// bad format, bound unless it is.
    fn finish(h: &mut impl Host, pass: &Pass, kind: &Kind, obj: u32) -> Result<(), u32> {
        let r = h.call_sym(kind.create, Conv::Thiscall(obj), &[pass.buf, 1, 0, 0]);
        if r != 0 {
            return Err(r);
        }
        if h.read::<u32>(obj) == h.rebase(BAD_FORMAT) {
            return Err(BAD_FORMAT_ERROR);
        }
        if !al(h_call(h, kind.bound, obj)) {
            let r = h_call(h, kind.bind, obj);
            if r != 0 {
                return Err(r);
            }
        }
        Ok(())
    }

    /// The x86 `shr` (the count's low five bits).
    fn shr(v: u32, n: u32) -> u32 {
        v >> (n & 31)
    }

    /// A 2D texture (`+0xc` levels less one, `+0x1c` width, `+0x20`
    /// height).
    fn texture_2d(h: &mut impl Host, pass: &mut Pass, obj: u32) -> Result<(), u32> {
        let mips: u32 = h.read(obj + 0xc);
        let mut w: u32 = h.read(obj + 0x1c);
        let mut ht: u32 = h.read(obj + 0x20);
        let q = pass.quality;
        if mips >= q && q != 0 {
            // back off while the smaller top level is under 4 on a side
            let (mut a, mut b) = (shr(w, q), shr(ht, q));
            let mut q = q;
            while a < 4 || b < 4 {
                a = a.wrapping_add(a);
                b = b.wrapping_add(b);
                q = q.wrapping_sub(1);
                if q == 0 {
                    break;
                }
            }
            pass.quality = q;
        }
        let mut dst = pass.buf;
        let mut k = 0u32;
        loop {
            let cw = if w != 0 { w } else { h.read(obj + 0x1c) };
            let ch = if ht != 0 { ht } else { h.read(obj + 0x20) };
            let format: u32 = h.read(obj);
            let n = level_bytes(h, format, cw, ch);
            read_level(h, pass, &mut dst, (k, mips), n)?;
            w = (w >> 1).max(1);
            ht = (ht >> 1).max(1);
            k = k.wrapping_add(1);
            if k > mips {
                break;
            }
        }
        let q = pass.quality;
        if q != 0 && mips >= q {
            let levels: u32 = h.read(obj + 0xc);
            let w: u32 = h.read(obj + 0x1c);
            let ht: u32 = h.read(obj + 0x20);
            let format: u32 = h.read(obj);
            h.call_sym(
                TEXTURE_2D.resize,
                Conv::Thiscall(obj),
                &[shr(w, q), shr(ht, q), format, levels.wrapping_sub(q)],
            );
        }
        finish(h, pass, &TEXTURE_2D, obj)
    }

    /// A 3D texture (`+0x24` its depth): no back-off.
    fn texture_3d(h: &mut impl Host, pass: &Pass, obj: u32) -> Result<(), u32> {
        let mips: u32 = h.read(obj + 0xc);
        let mut ht: u32 = h.read(obj + 0x20);
        let mut d: u32 = h.read(obj + 0x24);
        let mut w: u32 = h.read(obj + 0x1c);
        let mut dst = pass.buf;
        let mut k = 0u32;
        loop {
            let cw = if w != 0 { w } else { h.read(obj + 0x1c) };
            let ch = if ht != 0 { ht } else { h.read(obj + 0x20) };
            let cd = if d != 0 { d } else { h.read(obj + 0x24) };
            let format: u32 = h.read(obj);
            let n = level_bytes(h, format, cw, ch).wrapping_mul(cd);
            read_level(h, pass, &mut dst, (k, mips), n)?;
            w = (w >> 1).max(1);
            ht = (ht >> 1).max(1);
            d = (d >> 1).max(1);
            k = k.wrapping_add(1);
            if k > mips {
                break;
            }
        }
        let q = pass.quality;
        if q != 0 && mips >= q {
            let levels: u32 = h.read(obj + 0xc);
            let w: u32 = h.read(obj + 0x1c);
            let ht: u32 = h.read(obj + 0x20);
            let d: u32 = h.read(obj + 0x24);
            let format: u32 = h.read(obj);
            h.call_sym(
                TEXTURE_3D.resize,
                Conv::Thiscall(obj),
                &[
                    shr(w, q),
                    shr(ht, q),
                    shr(d, q),
                    format,
                    levels.wrapping_sub(q),
                ],
            );
        }
        finish(h, pass, &TEXTURE_3D, obj)
    }

    /// A cube map: six square faces of `+0x1c` on a side.
    fn texture_cube(h: &mut impl Host, pass: &Pass, obj: u32) -> Result<(), u32> {
        let mips: u32 = h.read(obj + 0xc);
        let mut dst = pass.buf;
        for _ in 0..6 {
            let mut w: u32 = h.read(obj + 0x1c);
            let mut k = 0u32;
            loop {
                let cw = if w != 0 { w } else { h.read(obj + 0x1c) };
                let format: u32 = h.read(obj);
                let n = level_bytes(h, format, cw, cw);
                read_level(h, pass, &mut dst, (k, mips), n)?;
                w = (w >> 1).max(1);
                k = k.wrapping_add(1);
                if k > mips {
                    break;
                }
            }
        }
        let q = pass.quality;
        if q != 0 && mips >= q {
            let levels: u32 = h.read(obj + 0xc);
            let w: u32 = h.read(obj + 0x1c);
            let format: u32 = h.read(obj);
            h.call_sym(
                TEXTURE_CUBE.resize,
                Conv::Thiscall(obj),
                &[shr(w, q), format, levels.wrapping_sub(q)],
            );
        }
        finish(h, pass, &TEXTURE_CUBE, obj)
    }
}

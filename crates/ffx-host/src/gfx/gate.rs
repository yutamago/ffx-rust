//! Scene-gate captures in `ffx-rust` (docs/drawdiff.md, "Scene gates";
//! docs/harness.md, "Two processes"): the harness's `gfxcapture <dir>
//! [hash] [frames N] [now] [async]` and `gfxcapture result`, written in the
//! form the hook's hash-mode capture writes (`crates/ffx-hook/src/gfx/
//! capture.rs`, `snapshot` and `light_msg`), so `drawdiff` compares an
//! `ffx-rust.exe` capture with the original's references.
//!
//! With `FFX_HOST_GFX_CAPTURE=1` ([`super::Config::gate`]) the layer keeps a
//! [`Store`]: the description messages it streams (views, shaders,
//! layouts, states) and every 2D texture's contents as the game uploads
//! them (initial data, `UpdateSubresource`, `Unmap`), packed at the
//! layout's row pitch; buffers are read from their arena copies. Nothing
//! else is recorded, so memory stays the live set's.
//!
//! A capture armed in lockstep at boundary `f` holds the presents that
//! show updates `f + 1` .. `f + N` (the harness's flip labels), as the
//! hook's (armed after the present of update `f`); a present that shows a
//! later update than the next one wanted fails the capture. The frames
//! keep the copies the hook records (`CopyResource` between two textures
//! the game made; not region copies, resolves or the back buffer's). Each
//! object a captured frame uses (draws, clears, copies; views pull in
//! their resources) is declared once, with its state at the window's end,
//! as the hook declares it in hash mode:
//!
//! - a texture: its description and one subresource holding the 8-byte
//!   content hash (FNV-1a of each subresource's texel rows, folded per
//!   subresource, in the hook's order), 0 for render and depth targets;
//!   the GPU's contents read back as the hook reads them (the D3D11 back
//!   end's `read_texture`), the uploaded ones on the null device;
//! - a static index buffer: its bytes; another static buffer: the FNV-1a
//!   hash of its bytes; a dynamic buffer: its bytes at the window's start
//!   (the hook declares none and adds each draw's bytes before it; the
//!   frames here carry the layer's own updates, so a draw reads the same
//!   bytes either way);
//! - views, shaders, layouts and states: the messages the layer streamed.
//!
//! Then one `World` message (the scene: the last `.ebp` opened; the camera
//! channel's eye and look-at point) and the frames. Not written: the
//! hook's `hashes.txt` (render target and back buffer contents need GPU
//! read-backs) and `capture.ffxref`; `tools/scene-gate.sh` runs drawdiff
//! alone when a run has no `hashes.txt`.

use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use ffx_gfx::{BufferDesc, Cmd, Frame, Id, Msg, Subresource, TextureDesc};

/// `D3D11_BIND_INDEX_BUFFER`, `_RENDER_TARGET`, `_DEPTH_STENCIL`.
const BIND_INDEX: u32 = 0x2;
const BIND_RENDER_TARGET: u32 = 0x20;
const BIND_DEPTH_STENCIL: u32 = 0x40;
/// `D3D11_USAGE_DYNAMIC`.
const USAGE_DYNAMIC: u32 = 2;

/// What a `gfxcapture` asked for (the hook's words; `draws A B` is the
/// hook's full-mode render target dump, not served here).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    pub dir: PathBuf,
    pub hash: bool,
    pub frames: u32,
    pub now: bool,
    pub detach: bool,
}

impl Options {
    /// `<dir> [hash] [frames N] [now] [async]`.
    ///
    /// # Errors
    /// A missing number or folder, or `draws`.
    pub fn parse(words: &[&str]) -> Result<Self, String> {
        let mut o = Self {
            dir: PathBuf::new(),
            hash: false,
            frames: 1,
            now: false,
            detach: false,
        };
        let mut dir = Vec::new();
        let mut i = 0;
        while i < words.len() {
            match words[i] {
                "hash" => o.hash = true,
                "now" => o.now = true,
                "async" => o.detach = true,
                "frames" => {
                    o.frames = words
                        .get(i + 1)
                        .and_then(|w| w.parse::<u32>().ok())
                        .ok_or("gfxcapture: a number is missing")?
                        .max(1);
                    i += 1;
                }
                "draws" => return Err("gfxcapture: draws: not served by ffx-rust".into()),
                w => dir.push(w),
            }
            i += 1;
        }
        if dir.is_empty() {
            return Err("gfxcapture: no folder".into());
        }
        o.dir = PathBuf::from(dir.join(" "));
        Ok(o)
    }

    /// Whether these words ask for a gate capture (any keyword), not the
    /// whole recording (`gfxcapture <dir>`, `FFX_HOST_GFX_RECORD=1`).
    #[must_use]
    pub fn wanted(words: &[&str]) -> bool {
        words
            .iter()
            .any(|w| matches!(*w, "hash" | "now" | "async" | "frames" | "result"))
    }
}

/// A 2D texture's description and contents.
struct Texture {
    desc: TextureDesc,
    /// Per subresource (D3D order): row bytes, rows, depth (the layer's
    /// `texture_subs`) and the packed bytes.
    subs: Vec<((u32, u32, u32), Vec<u8>)>,
}

/// A capture in progress.
struct Window {
    opts: Options,
    /// The first update a captured frame shows.
    first: u32,
    frames: Vec<(u32, Frame)>,
    /// Objects the frames use, in order of first use (a view's resource
    /// before it).
    declared: BTreeSet<Id>,
    used: Vec<Id>,
    /// Dynamic buffers' bytes as the frame before the window left them.
    rings: HashMap<Id, Vec<u8>>,
    world: Option<Msg>,
    reply: Option<Sender<String>>,
}

/// What the layer keeps for gate captures ([`super::Config::gate`]).
#[derive(Default)]
pub struct Store {
    textures: HashMap<Id, Texture>,
    /// Views, shaders, layouts, states: as streamed.
    others: HashMap<Id, Msg>,
    window: Option<Window>,
}

/// What [`Store::present`] needs from the layer.
pub struct Live<'a> {
    /// A buffer's description and its bytes now (the arena copy).
    pub buffer: &'a dyn Fn(Id) -> Option<(BufferDesc, Vec<u8>)>,
    /// A 2D texture's contents on the GPU, as the hook's capture reads
    /// them back (`read_texture_all`: for each slice, `max(desc mips, 1)`
    /// subresources at their row pitch); `None` without a device (the
    /// uploaded contents then).
    pub texture: &'a mut dyn FnMut(Id, &TextureDesc) -> Option<Vec<Subresource>>,
    /// The scene and camera (`World`).
    pub world: &'a dyn Fn() -> Msg,
}

impl Store {
    /// Takes in one message the layer streams.
    pub fn object(&mut self, m: &Msg, layout: impl Fn(&TextureDesc) -> Vec<(u32, u32, u32)>) {
        match m {
            Msg::Texture {
                id,
                desc,
                subresources,
            } => {
                let subs = layout(desc)
                    .into_iter()
                    .enumerate()
                    .map(|(i, (row, rows, depth))| {
                        let mut packed = vec![0u8; (row * rows * depth) as usize];
                        if let Some(s) = subresources.get(i) {
                            let pitch = if s.row_pitch == 0 { row } else { s.row_pitch };
                            copy_rows(&mut packed, row, 0, 0, &s.data, pitch, row, rows * depth);
                        }
                        ((row, rows, depth), packed)
                    })
                    .collect();
                self.others.remove(id);
                self.textures.insert(*id, Texture { desc: *desc, subs });
            }
            Msg::View { id, .. }
            | Msg::Shader { id, .. }
            | Msg::Layout { id, .. }
            | Msg::State { id, .. } => {
                self.textures.remove(id);
                self.others.insert(*id, m.clone());
            }
            Msg::Release { id } => {
                self.textures.remove(id);
                self.others.remove(id);
            }
            _ => {}
        }
    }

    /// New bytes for part of texture `id`'s subresource `sub`: `rows`
    /// rows of `row` bytes from `data` (pitch `pitch`), at byte `x` of row
    /// `y` (`UpdateSubresource`'s box in blocks; the whole subresource
    /// from 0, 0; `Unmap`).
    #[allow(clippy::too_many_arguments)] // the box and the source's layout
    pub fn texture_rows(
        &mut self,
        id: Id,
        sub: u32,
        x: u32,
        y: u32,
        data: &[u8],
        pitch: u32,
        row: u32,
        rows: u32,
    ) {
        if let Some(t) = self.textures.get_mut(&id)
            && let Some(((dst_row, _, _), bytes)) = t.subs.get_mut(sub as usize)
        {
            copy_rows(bytes, *dst_row, x, y, data, pitch, row, rows);
        }
    }

    /// Arms a capture: the presents of updates `first`.. are captured.
    ///
    /// # Errors
    /// One is already armed.
    pub fn arm(&mut self, opts: Options, first: u32, reply: Sender<String>) -> Result<(), String> {
        if self.window.as_ref().is_some_and(|w| w.reply.is_some()) {
            return Err("a capture is already running".into());
        }
        self.window = Some(Window {
            opts,
            first,
            frames: Vec::new(),
            declared: BTreeSet::new(),
            used: Vec::new(),
            rings: HashMap::new(),
            world: None,
            reply: Some(reply),
        });
        Ok(())
    }

    /// Notes the dynamic buffers' bytes after a present before the window
    /// (`ring(id)`: the layer's last view of the bytes).
    pub fn snapshot_rings(&mut self, rings: impl Iterator<Item = (Id, Vec<u8>)>) {
        if let Some(w) = self.window.as_mut()
            && w.frames.is_empty()
        {
            w.rings = rings.collect();
        }
    }

    /// Whether a capture waits for presents, and which update it wants
    /// next.
    #[must_use]
    pub fn wants(&self) -> Option<u32> {
        let w = self.window.as_ref()?;
        w.reply.as_ref()?;
        Some(w.first + u32::try_from(w.frames.len()).unwrap_or(u32::MAX))
    }

    /// A present that shows update `label` (`None`: not labelled), with
    /// `frame` its commands: taken into the capture when it is the next
    /// one wanted; the capture is written after its last frame. Returns
    /// whether the frame before the window has just been presented (the
    /// layer snapshots the rings then).
    pub fn present(&mut self, label: Option<u32>, frame: &Frame, live: &mut Live<'_>) -> bool {
        let Some(want) = self.wants() else {
            return false;
        };
        let Some(label) = label else {
            return false;
        };
        if label < want {
            return label + 1 == want;
        }
        if label > want {
            self.fail(&format!(
                "the present of update {want} was missed (this one shows update {label})"
            ));
            return false;
        }
        let Some(mut w) = self.window.take() else {
            return false;
        };
        if w.world.is_none() {
            w.world = Some((live.world)());
        }
        // the copies the hook records: between two textures the game made
        // (`CreateTexture2D`), not the back buffer's
        let mut frame = frame.clone();
        frame.cmds.retain(|c| match c {
            Cmd::CopyTexture { dst, src } => {
                self.textures.contains_key(dst) && self.textures.contains_key(src)
            }
            _ => true,
        });
        for id in frame_objects(&frame) {
            self.use_object(&mut w, id);
        }
        w.frames.push((label, frame));
        if w.frames.len() >= w.opts.frames as usize {
            let reply = w.reply.take();
            let msgs = self.declarations(&w, live);
            let r = write(&w, &msgs);
            if let Some(tx) = reply {
                let _ = tx.send(r);
            }
        } else {
            self.window = Some(w);
        }
        false
    }

    fn fail(&mut self, why: &str) {
        if let Some(tx) = self.window.take().and_then(|mut w| w.reply.take()) {
            let _ = tx.send(format!("err {why}"));
        }
    }

    /// Notes object `id` (a view's resource first) once.
    fn use_object(&self, w: &mut Window, id: Id) {
        if id == 0 || !w.declared.insert(id) {
            return;
        }
        if let Some(Msg::View { resource, .. }) = self.others.get(&id) {
            self.use_object(w, *resource);
        }
        w.used.push(id);
    }

    /// The objects' messages at the window's end, as the hook's `snapshot`
    /// makes them then (views after the rest).
    fn declarations(&self, w: &Window, live: &mut Live<'_>) -> Vec<Msg> {
        let mut msgs = Vec::new();
        let mut views = Vec::new();
        for &id in &w.used {
            match self.others.get(&id) {
                Some(m @ Msg::View { .. }) => {
                    views.push(m.clone());
                    continue;
                }
                Some(m) => {
                    msgs.push(m.clone());
                    continue;
                }
                None => {}
            }
            if let Some(t) = self.textures.get(&id) {
                msgs.push(texture_msg(id, t, w.opts.hash, live));
            } else if let Some((desc, bytes)) = (live.buffer)(id) {
                let data = if desc[1] == USAGE_DYNAMIC {
                    w.rings.get(&id).cloned().unwrap_or_default()
                } else if !w.opts.hash || desc[2] & BIND_INDEX != 0 {
                    bytes
                } else {
                    fnv(&bytes).to_le_bytes().to_vec()
                };
                msgs.push(Msg::Buffer { id, desc, data });
            }
        }
        msgs.extend(views);
        msgs
    }
}

/// A texture's message: in hash mode one subresource holding its content
/// hash (0 for targets); else its contents. The GPU's when the device
/// reads them back, else the uploaded ones.
fn texture_msg(id: Id, t: &Texture, hash: bool, live: &mut Live<'_>) -> Msg {
    let target = t.desc[8] & (BIND_RENDER_TARGET | BIND_DEPTH_STENCIL) != 0;
    if hash && target {
        return light(id, t.desc, 0);
    }
    let gpu = (live.texture)(id, &t.desc);
    if hash {
        let h = match &gpu {
            Some(subs) => texture_hash(
                &t.desc,
                &subs
                    .iter()
                    .map(|s| (s.row_pitch, s.data.as_slice()))
                    .collect::<Vec<_>>(),
            ),
            None => texture_hash(&t.desc, &uploaded(t)),
        };
        return light(id, t.desc, h);
    }
    let subresources = gpu.unwrap_or_else(|| {
        t.subs
            .iter()
            .map(|((row, _, _), data)| Subresource {
                row_pitch: *row,
                data: data.clone(),
            })
            .collect()
    });
    Msg::Texture {
        id,
        desc: t.desc,
        subresources,
    }
}

/// The hash-mode texture message.
fn light(id: Id, desc: TextureDesc, h: u64) -> Msg {
    Msg::Texture {
        id,
        desc,
        subresources: vec![Subresource {
            row_pitch: 0,
            data: h.to_le_bytes().to_vec(),
        }],
    }
}

/// The uploaded contents in the hook's read-back order: for each slice,
/// `max(desc mips, 1)` subresources, `mip + slice x mips` of the layout
/// (the hook's index, also when the description asks for the full chain).
fn uploaded(t: &Texture) -> Vec<(u32, &[u8])> {
    let (mips, slices) = (t.desc[2].max(1), t.desc[3].max(1));
    let mut out = Vec::new();
    for slice in 0..slices {
        for mip in 0..mips {
            match t.subs.get((mip + slice * mips) as usize) {
                Some(((row, _, _), data)) => out.push((*row, data.as_slice())),
                None => return out,
            }
        }
    }
    out
}

/// The capture files: `capture.ffxgfx` (objects, views after their
/// resources, the `World`, the frames) and `frames.txt` (the update each
/// frame shows); the harness reply.
fn write(w: &Window, msgs: &[Msg]) -> String {
    let mut out = ffx_gfx::CAPTURE_MAGIC.to_vec();
    for m in msgs {
        m.write(&mut out);
    }
    if let Some(m) = &w.world {
        m.write(&mut out);
    }
    let mut text = String::new();
    let mut draws = 0usize;
    for (k, (label, f)) in w.frames.iter().enumerate() {
        Msg::Frame(f.clone()).write(&mut out);
        let n = f.cmds.iter().filter(|c| matches!(c, Cmd::Draw(_))).count();
        draws += n;
        let _ = writeln!(
            text,
            "frame {k} update {label} present {} draws {n}",
            f.number
        );
    }
    let r = std::fs::create_dir_all(&w.opts.dir)
        .and_then(|()| std::fs::write(w.opts.dir.join("capture.ffxgfx"), &out))
        .and_then(|()| std::fs::write(w.opts.dir.join("frames.txt"), text));
    match r {
        Ok(()) => format!(
            "ok {draws} draws, {} frames (updates {}..{})",
            w.frames.len(),
            w.first,
            w.first + u32::try_from(w.frames.len()).unwrap_or(1) - 1
        ),
        Err(e) => format!("err {}: {e}", w.opts.dir.display()),
    }
}

/// The objects a frame names: draws' (views, not their resources),
/// clears' views, copies' textures, in order.
fn frame_objects(f: &Frame) -> Vec<Id> {
    let mut out = Vec::new();
    for c in &f.cmds {
        match c {
            Cmd::Draw(d) => {
                out.extend([
                    d.layout, d.vs, d.ps, d.index.0, d.blend, d.depth, d.raster, d.dsv,
                ]);
                out.extend(d.vertex.iter().map(|v| v.1));
                out.extend(
                    d.vs_srv
                        .iter()
                        .chain(&d.ps_srv)
                        .chain(&d.vs_samplers)
                        .chain(&d.ps_samplers)
                        .chain(&d.rtv)
                        .map(|v| v.1),
                );
            }
            Cmd::ClearTarget { view, .. } | Cmd::ClearDepth { view, .. } => out.push(*view),
            Cmd::CopyTexture { dst, src } => out.extend([*dst, *src]),
            Cmd::UpdateBuffer { .. } => {}
        }
    }
    out.retain(|&id| id != 0);
    out
}

/// FNV-1a, as the hook's `gfx::hash`.
fn fnv(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// The bytes of one row of texels at `width`, as the hook's capture
/// counts them (`row_bytes`).
fn row_bytes(format: u32, width: u32) -> Option<u32> {
    let blocks = width.div_ceil(4);
    Some(match format {
        70..=72 | 79..=81 => blocks * 8,
        73..=78 | 82..=84 | 94..=99 => blocks * 16,
        1..=4 => width * 16,
        5..=8 => width * 12,
        9..=22 => width * 8,
        23..=47 | 85..=93 => width * 4,
        48..=59 => width * 2,
        60..=65 => width,
        _ => return None,
    })
}

/// The hook's texture content hash (`texture_hash`): over `subs` in its
/// read-back order (for each slice, `max(desc mips, 1)` subresources, each
/// `(row pitch, bytes)`), each subresource's texel rows (`rows` from the
/// mip's height, in blocks for compressed formats, `row_bytes` of each
/// row) FNV-1a hashed and folded in. 0 when the read-back is short (the
/// hook's read-back error).
fn texture_hash(d: &TextureDesc, subs: &[(u32, &[u8])]) -> u64 {
    let (mips, slices) = (d[2].max(1), d[3].max(1));
    if subs.len() < (mips * slices) as usize {
        return 0;
    }
    let compressed = matches!(d[4], 70..=84 | 94..=99);
    let mut h = 0xcbf2_9ce4_8422_2325_u64;
    for (i, &(pitch, data)) in subs.iter().enumerate() {
        let mip = u32::try_from(i).unwrap_or(0) % mips;
        let (w, ht) = ((d[0] >> mip).max(1), (d[1] >> mip).max(1));
        let rows = if compressed { ht.div_ceil(4) } else { ht };
        let used = row_bytes(d[4], w).unwrap_or(pitch).min(pitch) as usize;
        let mut part = 0xcbf2_9ce4_8422_2325_u64;
        for r in 0..rows as usize {
            let at = r * pitch as usize;
            for &b in data.get(at..at + used).unwrap_or(&[]) {
                part ^= u64::from(b);
                part = part.wrapping_mul(0x0100_0000_01b3);
            }
        }
        h = (h ^ part).wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// Copies `rows` rows of `row` bytes from `src` (pitch `pitch`) into
/// `dst` (pitch `dst_pitch`) at byte `x` of row `y`, as far as both reach.
#[allow(clippy::too_many_arguments)] // the two layouts and the place
fn copy_rows(
    dst: &mut [u8],
    dst_pitch: u32,
    x: u32,
    y: u32,
    src: &[u8],
    pitch: u32,
    row: u32,
    rows: u32,
) {
    for r in 0..rows as usize {
        let from = r * pitch as usize;
        let to = (y as usize + r) * dst_pitch as usize + x as usize;
        let n = (row as usize)
            .min(src.len().saturating_sub(from))
            .min(dst.len().saturating_sub(to))
            .min((dst_pitch.saturating_sub(x)) as usize);
        if n == 0 {
            continue;
        }
        dst[to..to + n].copy_from_slice(&src[from..from + n]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(desc: &TextureDesc) -> Vec<(u32, u32, u32)> {
        super::super::texture_subs(2, desc)
    }

    /// The hash of the uploaded contents.
    fn th(t: &Texture) -> u64 {
        texture_hash(&t.desc, &uploaded(t))
    }

    #[test]
    fn the_gpu_contents_hash_at_their_pitch() {
        // the same texels at a 32-byte pitch (padding 0xee) as packed
        let mut s = Store::default();
        s.object(&tex4(1, 8, 7), layout);
        let gpu: Vec<u8> = (0..128)
            .map(|i| if i % 32 < 16 { 7 } else { 0xee })
            .collect();
        assert_eq!(
            texture_hash(&s.textures[&1].desc, &[(32, gpu.as_slice())]),
            th(&s.textures[&1])
        );
        // short: the hook's read-back error
        assert_eq!(texture_hash(&s.textures[&1].desc, &[]), 0);
    }

    /// A 4 x 4 RGBA texture with one mip.
    fn tex4(id: Id, bind: u32, fill: u8) -> Msg {
        Msg::Texture {
            id,
            desc: [4, 4, 1, 1, 28, 1, 0, 0, bind, 0, 0],
            subresources: vec![Subresource {
                row_pitch: 32, // padded: 16 bytes of texels per row
                data: (0..128)
                    .map(|i| if i % 32 < 16 { fill } else { 0xee })
                    .collect(),
            }],
        }
    }

    #[test]
    fn options_parse_as_the_hooks() {
        let o =
            Options::parse(&["C:\\o\\kami0100", "hash", "frames", "3", "now", "async"]).unwrap();
        assert_eq!(o.dir, PathBuf::from("C:\\o\\kami0100"));
        assert!(o.hash && o.now && o.detach);
        assert_eq!(o.frames, 3);
        assert!(Options::wanted(&["d", "hash"]));
        assert!(!Options::wanted(&["d"]));
        assert!(Options::parse(&["hash"]).is_err());
        assert!(Options::parse(&["d", "draws", "1", "2"]).is_err());
    }

    #[test]
    fn the_texture_hash_skips_the_row_padding() {
        let mut s = Store::default();
        s.object(&tex4(1, 8, 7), layout);
        let t = &s.textures[&1];
        // packed at 16 bytes a row
        assert_eq!(t.subs[0].1, vec![7u8; 64]);
        let mut part = 0xcbf2_9ce4_8422_2325_u64;
        for _ in 0..64 {
            part ^= 7;
            part = part.wrapping_mul(0x0100_0000_01b3);
        }
        let want = (0xcbf2_9ce4_8422_2325_u64 ^ part).wrapping_mul(0x0100_0000_01b3);
        assert_eq!(th(t), want);
        // an update of row 2, bytes 4..12 changes it; the same bytes again
        // give it back
        s.texture_rows(1, 0, 4, 2, &[9; 8], 8, 8, 1);
        assert_ne!(th(&s.textures[&1]), want);
        s.texture_rows(1, 0, 4, 2, &[7; 8], 8, 8, 1);
        assert_eq!(th(&s.textures[&1]), want);
    }

    #[test]
    #[allow(clippy::too_many_lines)] // the objects, the frames, the stream
    fn a_capture_declares_what_its_frames_use_as_the_hook_does() {
        let mut s = Store::default();
        s.object(&tex4(1, 8, 7), layout); // a shader resource
        s.object(&tex4(2, BIND_RENDER_TARGET, 1), layout); // a target
        s.object(
            &Msg::View {
                id: 10,
                kind: ffx_gfx::ViewKind::ShaderResource,
                resource: 1,
                desc: None,
            },
            layout,
        );
        s.object(
            &Msg::View {
                id: 11,
                kind: ffx_gfx::ViewKind::RenderTarget,
                resource: 2,
                desc: None,
            },
            layout,
        );
        let buffers = |id: Id| -> Option<(BufferDesc, Vec<u8>)> {
            match id {
                20 => Some(([8, 0, 1, 0, 0, 0], vec![1, 2, 3, 4, 5, 6, 7, 8])), // static vertex
                21 => Some(([4, 0, BIND_INDEX, 0, 0, 0], vec![0, 0, 1, 0])),    // static index
                22 => Some(([8, USAGE_DYNAMIC, 1, 0x10000, 0, 0], vec![9; 8])), // a ring now
                _ => None,
            }
        };
        let world = || Msg::World {
            scene: "kami0100".into(),
            camera: None,
        };
        let mut no_gpu = |_: Id, _: &TextureDesc| None;
        let mut live = Live {
            buffer: &buffers,
            texture: &mut no_gpu,
            world: &world,
        };
        let dir = std::env::temp_dir().join(format!("ffx-host-gate-{}", std::process::id()));
        let (tx, rx) = std::sync::mpsc::channel();
        s.arm(
            Options {
                dir: dir.clone(),
                hash: true,
                frames: 2,
                now: true,
                detach: true,
            },
            6,
            tx,
        )
        .unwrap();
        let draw = ffx_gfx::Draw {
            kind: ffx_gfx::DrawKind::Indexed,
            count: 3,
            instances: 1,
            start: 0,
            base: 0,
            first_instance: 0,
            topology: 4,
            layout: 0,
            vs: 0,
            ps: 0,
            vertex: vec![(0, 20, 8, 0), (1, 22, 8, 0)],
            index: (21, 57, 0),
            vs_cb: Vec::new(),
            ps_cb: Vec::new(),
            vs_srv: Vec::new(),
            ps_srv: vec![(0, 10)],
            vs_samplers: Vec::new(),
            ps_samplers: Vec::new(),
            blend: 0,
            blend_factor: [1.0; 4],
            sample_mask: u32::MAX,
            depth: 0,
            stencil_ref: 0,
            raster: 0,
            viewport: [0.0; 6],
            scissor: [0; 4],
            rtv: vec![(0, 11)],
            dsv: 0,
        };
        let frame = |n| Frame {
            number: n,
            width: 4,
            height: 4,
            cmds: vec![
                Cmd::ClearTarget {
                    view: 11,
                    color: [0.0; 4],
                },
                Cmd::Draw(Box::new(draw.clone())),
            ],
        };
        // update 4: not yet; update 5: the frame before (the rings now)
        assert!(!s.present(Some(4), &frame(1), &mut live));
        assert!(s.present(Some(5), &frame(2), &mut live));
        s.snapshot_rings([(22, vec![3u8; 8])].into_iter());
        assert_eq!(s.wants(), Some(6));
        s.present(Some(6), &frame(3), &mut live);
        assert_eq!(s.wants(), Some(7));
        s.present(Some(7), &frame(4), &mut live);
        assert_eq!(s.wants(), None);
        let r = rx.try_recv().unwrap();
        assert_eq!(r, "ok 2 draws, 2 frames (updates 6..7)");
        let bytes = std::fs::read(dir.join("capture.ffxgfx")).unwrap();
        let msgs = ffx_gfx::read_all(&bytes[8..]).unwrap();
        let kinds: Vec<String> = msgs
            .iter()
            .map(|m| match m {
                Msg::Texture {
                    id, subresources, ..
                } => format!("t{id}:{}", hex(&subresources[0].data)),
                Msg::Buffer { id, data, .. } => format!("b{id}:{}", hex(data)),
                Msg::View { id, resource, .. } => format!("v{id}>{resource}"),
                Msg::World { scene, .. } => format!("w{scene}"),
                Msg::Frame(f) => format!("f{}", f.number),
                _ => "?".into(),
            })
            .collect();
        let t1 = th(&s.textures[&1]).to_le_bytes();
        assert_eq!(
            kinds,
            vec![
                // the clear's view pulls the target in (hash 0) first; then
                // the draw's index buffer, vertex buffers, texture
                "t2:0000000000000000".to_string(),
                "b21:00000100".into(),
                format!("b20:{}", hex(&fnv(&[1, 2, 3, 4, 5, 6, 7, 8]).to_le_bytes())),
                "b22:0303030303030303".into(), // the ring at the window's start
                format!("t1:{}", hex(&t1)),
                "v11>2".into(),
                "v10>1".into(),
                "wkami0100".into(),
                "f3".into(),
                "f4".into(),
            ]
        );
        let text = std::fs::read_to_string(dir.join("frames.txt")).unwrap();
        assert!(text.starts_with("frame 0 update 6 present 3 draws 1\n"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_skipped_update_fails_the_capture() {
        let mut s = Store::default();
        let (tx, rx) = std::sync::mpsc::channel();
        let opts = Options::parse(&["d", "hash", "frames", "3"]).unwrap();
        s.arm(opts, 6, tx).unwrap();
        let none = |_: Id| None;
        let world = || Msg::World {
            scene: String::new(),
            camera: None,
        };
        let mut no_gpu = |_: Id, _: &TextureDesc| None;
        let mut live = Live {
            buffer: &none,
            texture: &mut no_gpu,
            world: &world,
        };
        let f = Frame {
            number: 0,
            width: 1,
            height: 1,
            cmds: Vec::new(),
        };
        s.present(Some(7), &f, &mut live);
        assert_eq!(
            rx.try_recv().unwrap(),
            "err the present of update 6 was missed (this one shows update 7)"
        );
        assert_eq!(s.wants(), None);
    }

    /// Copies from the back buffer (no texture the game made) are left
    /// out, as the hook's capture leaves them out; the GPU's contents are
    /// hashed when the device reads them back.
    #[test]
    fn copies_and_read_backs_as_the_hook() {
        let mut s = Store::default();
        s.object(&tex4(1, 8, 7), layout);
        s.object(&tex4(2, 8, 5), layout);
        let dir = std::env::temp_dir().join(format!("ffx-host-gate-c-{}", std::process::id()));
        let (tx, rx) = std::sync::mpsc::channel();
        let opts = Options::parse(&[dir.to_str().unwrap(), "hash"]).unwrap();
        s.arm(opts, 1, tx).unwrap();
        let none = |_: Id| None;
        let world = || Msg::World {
            scene: String::new(),
            camera: None,
        };
        // texture 2 on the GPU: other bytes than uploaded (a copy target)
        let gpu = vec![9u8; 64];
        let mut read = |id: Id, _: &TextureDesc| {
            (id == 2).then(|| {
                vec![Subresource {
                    row_pitch: 16,
                    data: gpu.clone(),
                }]
            })
        };
        let mut live = Live {
            buffer: &none,
            texture: &mut read,
            world: &world,
        };
        let f = Frame {
            number: 0,
            width: 1,
            height: 1,
            cmds: vec![
                Cmd::CopyTexture { dst: 2, src: 1 },
                Cmd::CopyTexture { dst: 2, src: 99 }, // the back buffer
            ],
        };
        s.present(Some(1), &f, &mut live);
        assert!(rx.try_recv().unwrap().starts_with("ok 0 draws, 1 frames"));
        let bytes = std::fs::read(dir.join("capture.ffxgfx")).unwrap();
        let msgs = ffx_gfx::read_all(&bytes[8..]).unwrap();
        let Some(Msg::Frame(frame)) = msgs.last() else {
            panic!("no frame")
        };
        assert_eq!(frame.cmds, vec![Cmd::CopyTexture { dst: 2, src: 1 }]);
        let hashes: Vec<(Id, Vec<u8>)> = msgs
            .iter()
            .filter_map(|m| match m {
                Msg::Texture {
                    id, subresources, ..
                } => Some((*id, subresources[0].data.clone())),
                _ => None,
            })
            .collect();
        let gpu_hash = texture_hash(&s.textures[&2].desc, &[(16, gpu.as_slice())]);
        assert_eq!(
            hashes,
            vec![
                (2, gpu_hash.to_le_bytes().to_vec()),
                (1, th(&s.textures[&1]).to_le_bytes().to_vec()),
            ]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn hex(b: &[u8]) -> String {
        b.iter().fold(String::new(), |mut s, x| {
            let _ = write!(s, "{x:02x}");
            s
        })
    }
}

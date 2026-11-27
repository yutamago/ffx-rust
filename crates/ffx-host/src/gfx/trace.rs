//! The back end's calls on file, and frames as pictures (docs/host.md,
//! "The wgpu back end"):
//!
//! - `FFX_HOST_GFX_TRACE=<file>`: every call the layer makes on its back
//!   end ([`super::Backend`]) goes into `<file>` as well ([`Tee`]), with
//!   its arguments and data. [`replay`] plays such a file into another back
//!   end: the same frames drawn by Direct3D 11 and by wgpu on one GPU, for
//!   pixel parity without a second run of the game (`examples/gfx-replay.rs`,
//!   `tools/m6/host-parity.py`).
//! - `FFX_HOST_GFX_SHOTS=<dir>`: the back buffer of every
//!   `FFX_HOST_GFX_SHOT_EVERY`-th `Present` (default 100) as
//!   `<dir>/present-NNNNN.png`, taken before the `Present` as the harness's
//!   shots are.
//!
//! The file is the magic, then one record per call: a tag byte and the
//! call's fields, numbers little-endian, byte strings with a 32-bit length.

// Lengths are those of the game's buffers and lists (far below 4 GiB);
// one arm per call.
#![allow(clippy::cast_possible_truncation, clippy::too_many_lines)]

use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use ffx_gfx::{DrawKind, Element, StateKind, Subresource};

use super::{Backend, Create, Op, SwapChainDesc};

/// The trace file's first bytes.
pub const MAGIC: &[u8; 8] = b"FFXGFXT1";

/// Writes the fields of one record.
#[derive(Default)]
struct Enc(Vec<u8>);

impl Enc {
    fn u8(&mut self, v: u8) -> &mut Self {
        self.0.push(v);
        self
    }
    fn u32(&mut self, v: u32) -> &mut Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    fn f32(&mut self, v: f32) -> &mut Self {
        self.u32(v.to_bits())
    }
    fn bytes(&mut self, b: &[u8]) -> &mut Self {
        self.u32(u32::try_from(b.len()).unwrap_or(u32::MAX));
        self.0.extend_from_slice(b);
        self
    }
    fn words(&mut self, w: &[u32]) -> &mut Self {
        self.u32(u32::try_from(w.len()).unwrap_or(u32::MAX));
        for &x in w {
            self.u32(x);
        }
        self
    }
    fn opt<T>(&mut self, v: Option<T>, f: impl FnOnce(&mut Self, T)) -> &mut Self {
        match v {
            Some(v) => {
                self.u8(1);
                f(self, v);
            }
            None => {
                self.u8(0);
            }
        }
        self
    }
}

/// Reads the fields of the records.
struct Dec<'a> {
    b: &'a [u8],
    at: usize,
}

impl Dec<'_> {
    fn take(&mut self, n: usize) -> Option<&[u8]> {
        let s = self.b.get(self.at..self.at + n)?;
        self.at += n;
        Some(s)
    }
    fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|s| s[0])
    }
    fn u32(&mut self) -> Option<u32> {
        self.take(4)
            .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }
    fn f32(&mut self) -> Option<f32> {
        self.u32().map(f32::from_bits)
    }
    fn bytes(&mut self) -> Option<Vec<u8>> {
        let n = self.u32()? as usize;
        self.take(n).map(<[u8]>::to_vec)
    }
    fn words(&mut self) -> Option<Vec<u32>> {
        let n = self.u32()? as usize;
        (0..n).map(|_| self.u32()).collect()
    }
    fn arr<const N: usize>(&mut self) -> Option<[u32; N]> {
        let mut a = [0; N];
        for x in &mut a {
            *x = self.u32()?;
        }
        Some(a)
    }
    fn opt(&mut self) -> Option<bool> {
        self.u8().map(|v| v != 0)
    }
}

const DEVICE: u8 = 1;
const BACK_BUFFER: u8 = 2;
const CREATE: u8 = 3;
const RELEASE: u8 = 4;
const OP: u8 = 5;
const READ: u8 = 6;
const QUERY: u8 = 7;
const PRESENT: u8 = 8;
const READ_TEXTURE: u8 = 9;

fn stage_kind(k: StateKind) -> u8 {
    match k {
        StateKind::Blend => 0,
        StateKind::Depth => 1,
        StateKind::Raster => 2,
        StateKind::Sampler => 3,
    }
}

fn kind_stage(k: u8) -> StateKind {
    match k {
        0 => StateKind::Blend,
        1 => StateKind::Depth,
        2 => StateKind::Raster,
        _ => StateKind::Sampler,
    }
}

fn draw_code(k: ffx_gfx::DrawKind) -> u8 {
    k as u8
}

fn code_draw(c: u8) -> DrawKind {
    match c {
        0 => DrawKind::Indexed,
        1 => DrawKind::Plain,
        2 => DrawKind::IndexedInstanced,
        _ => DrawKind::Instanced,
    }
}

fn encode_create(e: &mut Enc, id: u32, what: &Create<'_>) {
    e.u8(CREATE).u32(id);
    match what {
        Create::Buffer { desc, init } => {
            e.u8(0).words(desc).opt(*init, |e, b| {
                e.bytes(b);
            });
        }
        Create::Texture { dims, desc, init } => {
            e.u8(1).u8(*dims).words(desc).u32(init.len() as u32);
            for (row, slice, data) in *init {
                e.u32(*row).u32(*slice).bytes(data);
            }
        }
        Create::View {
            class,
            resource,
            desc,
        } => {
            e.u8(2)
                .u32(*class as u32)
                .u32(*resource)
                .opt(*desc, |e, d| {
                    e.bytes(d);
                });
        }
        Create::Shader { stage, bytecode } => {
            e.u8(3).u8(*stage).bytes(bytecode);
        }
        Create::Layout { elements, bytecode } => {
            e.u8(4).u32(elements.len() as u32);
            for el in *elements {
                e.bytes(el.semantic.as_bytes())
                    .u32(el.index)
                    .u32(el.format)
                    .u32(el.slot)
                    .u32(el.offset)
                    .u8(u8::from(el.per_instance))
                    .u32(el.step);
            }
            e.bytes(bytecode);
        }
        Create::State { kind, desc } => {
            e.u8(5).u8(stage_kind(*kind)).bytes(desc);
        }
        Create::Query(q) => {
            e.u8(6).u32(q[0]).u32(q[1]);
        }
    }
}

fn encode_op(e: &mut Enc, op: &Op<'_>) {
    e.u8(OP);
    match op {
        Op::InputLayout(l) => {
            e.u8(0).u32(*l);
        }
        Op::VertexBuffers(s, b, st, o) => {
            e.u8(1).u32(*s).words(b).words(st).words(o);
        }
        Op::IndexBuffer(b, f, o) => {
            e.u8(2).u32(*b).u32(*f).u32(*o);
        }
        Op::Topology(t) => {
            e.u8(3).u32(*t);
        }
        Op::Shader(s, id) => {
            e.u8(4).u8(*s).u32(*id);
        }
        Op::ConstantBuffers(s, start, b, r) => {
            e.u8(5).u8(*s).u32(*start).words(b).opt(*r, |e, (f, n)| {
                e.words(f).words(n);
            });
        }
        Op::Resources(s, start, v) => {
            e.u8(6).u8(*s).u32(*start).words(v);
        }
        Op::Samplers(s, start, v) => {
            e.u8(7).u8(*s).u32(*start).words(v);
        }
        Op::Targets(v, d) => {
            e.u8(8).words(v).u32(*d);
        }
        Op::Blend(s, f, m) => {
            e.u8(9).u32(*s);
            for x in f {
                e.f32(*x);
            }
            e.u32(*m);
        }
        Op::Depth(s, r) => {
            e.u8(10).u32(*s).u32(*r);
        }
        Op::Raster(s) => {
            e.u8(11).u32(*s);
        }
        Op::Viewports(v) => {
            e.u8(12).u32(v.len() as u32);
            for vp in *v {
                for x in vp {
                    e.f32(*x);
                }
            }
        }
        Op::Scissors(v) => {
            e.u8(13).u32(v.len() as u32);
            for r in *v {
                for x in r {
                    e.u32(x.cast_unsigned());
                }
            }
        }
        Op::Draw(k, p) => {
            e.u8(14).u8(draw_code(*k));
            for x in p {
                e.u32(*x);
            }
        }
        Op::ClearTarget(v, c) => {
            e.u8(15).u32(*v);
            for x in c {
                e.f32(*x);
            }
        }
        Op::ClearDepth(v, f, d, s) => {
            e.u8(16).u32(*v).u32(*f).f32(*d).u8(*s);
        }
        Op::Copy(d, s) => {
            e.u8(17).u32(*d).u32(*s);
        }
        Op::Resolve(w) => {
            e.u8(18).words(w);
        }
        Op::CopyRegion(w, b) => {
            e.u8(19).words(w).opt(*b, |e, b| {
                e.words(&b);
            });
        }
        Op::Update(d, sub, b, data, row, depth) => {
            e.u8(20).u32(*d).u32(*sub).opt(*b, |e, b| {
                e.words(&b);
            });
            e.bytes(data).u32(*row).u32(*depth);
        }
        Op::Unmapped(id, sub, row, depth, data, spans) => {
            e.u8(21)
                .u32(*id)
                .u32(*sub)
                .u32(*row)
                .u32(*depth)
                .bytes(data);
            e.opt(*spans, |e, s| {
                e.u32(s.len() as u32);
                for &(o, n) in s {
                    e.u32(o).u32(n);
                }
            });
        }
        Op::Begin(q) => {
            e.u8(22).u32(*q);
        }
        Op::End(q) => {
            e.u8(23).u32(*q);
        }
        Op::GenerateMips(v) => {
            e.u8(24).u32(*v);
        }
        Op::ClearState => {
            e.u8(25);
        }
        Op::Flush => {
            e.u8(26);
        }
    }
}

/// A back end that writes every call to a trace file before passing it
/// on.
pub struct Tee {
    inner: Box<dyn Backend>,
    out: Option<BufWriter<std::fs::File>>,
    buf: Enc,
}

impl Tee {
    /// The trace file at `path`, begun.
    ///
    /// # Errors
    /// When the file can't be made.
    pub fn create(path: &Path) -> std::io::Result<BufWriter<std::fs::File>> {
        let mut out = BufWriter::with_capacity(1 << 20, std::fs::File::create(path)?);
        out.write_all(MAGIC)?;
        Ok(out)
    }

    /// `inner`, its calls also written to `out` (from [`Tee::create`]).
    #[must_use]
    pub fn wrap(inner: Box<dyn Backend>, out: BufWriter<std::fs::File>) -> Self {
        Self {
            inner,
            out: Some(out),
            buf: Enc::default(),
        }
    }

    fn emit(&mut self, f: impl FnOnce(&mut Enc)) {
        self.buf.0.clear();
        f(&mut self.buf);
        if let Some(out) = self.out.as_mut()
            && out.write_all(&self.buf.0).is_err()
        {
            crate::log::note("gfx trace: write failed; trace ends here");
            self.out = None;
        }
    }
}

impl Backend for Tee {
    fn name(&self) -> &'static str {
        self.inner.name()
    }
    fn create_device(&mut self, desc: &SwapChainDesc, flags: u32) -> u32 {
        self.emit(|e| {
            e.u8(DEVICE).words(&desc.0).u32(flags);
        });
        self.inner.create_device(desc, flags)
    }
    fn back_buffer(&mut self, id: u32) -> u32 {
        self.emit(|e| {
            e.u8(BACK_BUFFER).u32(id);
        });
        self.inner.back_buffer(id)
    }
    fn create(&mut self, id: u32, what: &Create<'_>) -> u32 {
        self.emit(|e| encode_create(e, id, what));
        self.inner.create(id, what)
    }
    fn release(&mut self, id: u32) {
        self.emit(|e| {
            e.u8(RELEASE).u32(id);
        });
        self.inner.release(id);
    }
    fn op(&mut self, op: &Op<'_>) {
        self.emit(|e| encode_op(e, op));
        self.inner.op(op);
    }
    fn read(&mut self, id: u32, sub: u32, row: u32, out: &mut [u8]) -> bool {
        self.emit(|e| {
            e.u8(READ).u32(id).u32(sub).u32(row).u32(out.len() as u32);
        });
        self.inner.read(id, sub, row, out)
    }
    fn query_data(&mut self, id: u32, out: &mut [u8]) -> u32 {
        self.emit(|e| {
            e.u8(QUERY).u32(id).u32(out.len() as u32);
        });
        self.inner.query_data(id, out)
    }
    fn present(&mut self, sync: u32, flags: u32) -> u32 {
        self.emit(|e| {
            e.u8(PRESENT).u32(sync).u32(flags);
        });
        if let Some(out) = self.out.as_mut() {
            let _ = out.flush();
        }
        self.inner.present(sync, flags)
    }
    fn back_buffer_image(&mut self) -> Option<(u32, u32, Vec<u8>)> {
        self.inner.back_buffer_image()
    }
    fn read_texture(&mut self, id: u32, desc: &[u32; 11]) -> Option<Vec<Subresource>> {
        self.emit(|e| {
            e.u8(READ_TEXTURE).u32(id).words(desc);
        });
        self.inner.read_texture(id, desc)
    }
    fn object(&self, id: u32) -> Option<usize> {
        self.inner.object(id)
    }
}

/// What [`replay`] did.
#[derive(Debug, Default)]
pub struct Replayed {
    /// Calls played.
    pub calls: u64,
    /// `Present`s played.
    pub presents: u32,
    /// The pictures written.
    pub shots: Vec<PathBuf>,
}

/// Plays the trace `bytes` into `backend`. `shot(n)` says whether to save
/// the back buffer before `Present` number `n` (from 1) into `dir`;
/// `stop` ends the replay after that many `Present`s.
///
/// # Errors
/// A trace that doesn't parse, or a picture that can't be written.
pub fn replay(
    bytes: &[u8],
    backend: &mut dyn Backend,
    dir: &Path,
    shot: &dyn Fn(u32) -> bool,
    stop: Option<u32>,
) -> Result<Replayed, String> {
    if bytes.get(..8) != Some(&MAGIC[..]) {
        return Err("not a gfx trace".into());
    }
    let mut d = Dec { b: bytes, at: 8 };
    let mut r = Replayed::default();
    while d.at < bytes.len() {
        let at = d.at;
        play(&mut d, backend, dir, shot, &mut r)
            .ok_or_else(|| format!("trace: bad record at byte {at}"))??;
        r.calls += 1;
        if stop.is_some_and(|s| r.presents >= s) {
            break;
        }
    }
    Ok(r)
}

fn play(
    d: &mut Dec<'_>,
    b: &mut dyn Backend,
    dir: &Path,
    shot: &dyn Fn(u32) -> bool,
    r: &mut Replayed,
) -> Option<Result<(), String>> {
    match d.u8()? {
        DEVICE => {
            let desc: [u32; 15] = d.words()?.try_into().ok()?;
            let flags = d.u32()?;
            b.create_device(&SwapChainDesc(desc), flags);
        }
        BACK_BUFFER => {
            b.back_buffer(d.u32()?);
        }
        CREATE => {
            let id = d.u32()?;
            play_create(d, b, id)?;
        }
        RELEASE => b.release(d.u32()?),
        OP => play_op(d, b)?,
        READ => {
            let (id, sub, row, len) = (d.u32()?, d.u32()?, d.u32()?, d.u32()?);
            let mut out = vec![0u8; len as usize];
            b.read(id, sub, row, &mut out);
        }
        QUERY => {
            let (id, len) = (d.u32()?, d.u32()?);
            let mut out = vec![0u8; len as usize];
            b.query_data(id, &mut out);
        }
        PRESENT => {
            let (sync, flags) = (d.u32()?, d.u32()?);
            let n = r.presents + 1;
            if shot(n)
                && let Some((w, h, bgra)) = b.back_buffer_image()
            {
                let path = dir.join(format!("present-{n:05}.png"));
                if let Err(e) = write_png(&path, w, h, &bgra) {
                    return Some(Err(format!("{}: {e}", path.display())));
                }
                r.shots.push(path);
            }
            b.present(sync, flags);
            r.presents = n;
        }
        READ_TEXTURE => {
            let id = d.u32()?;
            let w = d.words()?;
            let desc: [u32; 11] = w.try_into().ok()?;
            b.read_texture(id, &desc);
        }
        _ => return None,
    }
    Some(Ok(()))
}

fn play_create(d: &mut Dec<'_>, b: &mut dyn Backend, id: u32) -> Option<()> {
    match d.u8()? {
        0 => {
            let desc: [u32; 6] = d.words()?.try_into().ok()?;
            let init = if d.opt()? { Some(d.bytes()?) } else { None };
            b.create(
                id,
                &Create::Buffer {
                    desc,
                    init: init.as_deref(),
                },
            );
        }
        1 => {
            let dims = d.u8()?;
            let desc = d.words()?;
            let n = d.u32()?;
            let mut init = Vec::new();
            for _ in 0..n {
                init.push((d.u32()?, d.u32()?, d.bytes()?));
            }
            b.create(
                id,
                &Create::Texture {
                    dims,
                    desc: &desc,
                    init: &init,
                },
            );
        }
        2 => {
            let class = d.u32()? as usize;
            let resource = d.u32()?;
            let desc = if d.opt()? { Some(d.bytes()?) } else { None };
            b.create(
                id,
                &Create::View {
                    class,
                    resource,
                    desc: desc.as_deref(),
                },
            );
        }
        3 => {
            let stage = d.u8()?;
            let code = d.bytes()?;
            b.create(
                id,
                &Create::Shader {
                    stage,
                    bytecode: &code,
                },
            );
        }
        4 => {
            let n = d.u32()?;
            let mut elements = Vec::new();
            for _ in 0..n {
                elements.push(Element {
                    semantic: String::from_utf8_lossy(&d.bytes()?).into_owned(),
                    index: d.u32()?,
                    format: d.u32()?,
                    slot: d.u32()?,
                    offset: d.u32()?,
                    per_instance: d.u8()? != 0,
                    step: d.u32()?,
                });
            }
            let code = d.bytes()?;
            b.create(
                id,
                &Create::Layout {
                    elements: &elements,
                    bytecode: &code,
                },
            );
        }
        5 => {
            let kind = kind_stage(d.u8()?);
            let desc = d.bytes()?;
            b.create(id, &Create::State { kind, desc: &desc });
        }
        6 => {
            let q = [d.u32()?, d.u32()?];
            b.create(id, &Create::Query(q));
        }
        _ => return None,
    }
    Some(())
}

fn play_op(d: &mut Dec<'_>, b: &mut dyn Backend) -> Option<()> {
    match d.u8()? {
        0 => b.op(&Op::InputLayout(d.u32()?)),
        1 => {
            let s = d.u32()?;
            let (bufs, st, o) = (d.words()?, d.words()?, d.words()?);
            b.op(&Op::VertexBuffers(s, &bufs, &st, &o));
        }
        2 => b.op(&Op::IndexBuffer(d.u32()?, d.u32()?, d.u32()?)),
        3 => b.op(&Op::Topology(d.u32()?)),
        4 => b.op(&Op::Shader(d.u8()?, d.u32()?)),
        5 => {
            let (s, start, bufs) = (d.u8()?, d.u32()?, d.words()?);
            let r = if d.opt()? {
                Some((d.words()?, d.words()?))
            } else {
                None
            };
            b.op(&Op::ConstantBuffers(
                s,
                start,
                &bufs,
                r.as_ref().map(|(f, n)| (f.as_slice(), n.as_slice())),
            ));
        }
        6 => {
            let (s, start, v) = (d.u8()?, d.u32()?, d.words()?);
            b.op(&Op::Resources(s, start, &v));
        }
        7 => {
            let (s, start, v) = (d.u8()?, d.u32()?, d.words()?);
            b.op(&Op::Samplers(s, start, &v));
        }
        8 => {
            let v = d.words()?;
            b.op(&Op::Targets(&v, d.u32()?));
        }
        9 => {
            let s = d.u32()?;
            let f = [d.f32()?, d.f32()?, d.f32()?, d.f32()?];
            b.op(&Op::Blend(s, f, d.u32()?));
        }
        10 => b.op(&Op::Depth(d.u32()?, d.u32()?)),
        11 => b.op(&Op::Raster(d.u32()?)),
        12 => {
            let n = d.u32()?;
            let mut v = Vec::new();
            for _ in 0..n {
                v.push([d.f32()?, d.f32()?, d.f32()?, d.f32()?, d.f32()?, d.f32()?]);
            }
            b.op(&Op::Viewports(&v));
        }
        13 => {
            let n = d.u32()?;
            let mut v = Vec::new();
            for _ in 0..n {
                let a = d.arr::<4>()?;
                v.push(a.map(u32::cast_signed));
            }
            b.op(&Op::Scissors(&v));
        }
        14 => {
            let k = code_draw(d.u8()?);
            b.op(&Op::Draw(k, d.arr::<5>()?));
        }
        15 => {
            let v = d.u32()?;
            let c = [d.f32()?, d.f32()?, d.f32()?, d.f32()?];
            b.op(&Op::ClearTarget(v, c));
        }
        16 => b.op(&Op::ClearDepth(d.u32()?, d.u32()?, d.f32()?, d.u8()?)),
        17 => b.op(&Op::Copy(d.u32()?, d.u32()?)),
        18 => {
            let w: [u32; 5] = d.words()?.try_into().ok()?;
            b.op(&Op::Resolve(w));
        }
        19 => {
            let w: [u32; 7] = d.words()?.try_into().ok()?;
            let bx = if d.opt()? {
                Some(<[u32; 6]>::try_from(d.words()?).ok()?)
            } else {
                None
            };
            b.op(&Op::CopyRegion(w, bx));
        }
        20 => {
            let (id, sub) = (d.u32()?, d.u32()?);
            let bx = if d.opt()? {
                Some(<[u32; 6]>::try_from(d.words()?).ok()?)
            } else {
                None
            };
            let data = d.bytes()?;
            b.op(&Op::Update(id, sub, bx, &data, d.u32()?, d.u32()?));
        }
        21 => {
            let (id, sub, row, depth) = (d.u32()?, d.u32()?, d.u32()?, d.u32()?);
            let data = d.bytes()?;
            let spans = if d.opt()? {
                let n = d.u32()?;
                let mut s = Vec::new();
                for _ in 0..n {
                    s.push((d.u32()?, d.u32()?));
                }
                Some(s)
            } else {
                None
            };
            b.op(&Op::Unmapped(id, sub, row, depth, &data, spans.as_deref()));
        }
        22 => b.op(&Op::Begin(d.u32()?)),
        23 => b.op(&Op::End(d.u32()?)),
        24 => b.op(&Op::GenerateMips(d.u32()?)),
        25 => b.op(&Op::ClearState),
        26 => b.op(&Op::Flush),
        _ => return None,
    }
    Some(())
}

/// Reads a whole trace file.
///
/// # Errors
/// When it can't be read.
pub fn read_file(path: &Path) -> std::io::Result<Vec<u8>> {
    let mut v = Vec::new();
    std::fs::File::open(path)?.read_to_end(&mut v)?;
    Ok(v)
}

/// The `FFX_HOST_GFX_SHOTS` settings: the folder and every how many
/// `Present`s.
#[must_use]
pub fn shots_from_env() -> Option<(PathBuf, u32)> {
    let dir = std::env::var_os("FFX_HOST_GFX_SHOTS").filter(|v| !v.is_empty())?;
    let every = std::env::var("FFX_HOST_GFX_SHOT_EVERY")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&n: &u32| n > 0)
        .unwrap_or(100);
    Some((PathBuf::from(dir), every))
}

/// Writes `w` x `h` BGRA rows as an RGB PNG (stored, uncompressed blocks
/// through `miniz_oxide`).
///
/// # Errors
/// When the file can't be written.
pub fn write_png(path: &Path, w: u32, h: u32, bgra: &[u8]) -> std::io::Result<()> {
    let mut raw = Vec::with_capacity((w as usize * 3 + 1) * h as usize);
    for row in bgra.chunks_exact(w as usize * 4).take(h as usize) {
        raw.push(0);
        for px in row.as_chunks::<4>().0 {
            raw.extend_from_slice(&[px[2], px[1], px[0]]);
        }
    }
    let z = miniz_oxide::deflate::compress_to_vec_zlib(&raw, 3);
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut chunk = |tag: &[u8; 4], body: &[u8]| {
        png.extend_from_slice(&u32::try_from(body.len()).unwrap_or(0).to_be_bytes());
        let mut c = tag.to_vec();
        c.extend_from_slice(body);
        png.extend_from_slice(&c);
        png.extend_from_slice(&crc32(&c).to_be_bytes());
    };
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    chunk(b"IHDR", &ihdr);
    chunk(b"IDAT", &z);
    chunk(b"IEND", &[]);
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    std::fs::write(path, png)
}

fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xffff_ffffu32;
    for &b in data {
        c ^= u32::from(b);
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xedb8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
    }
    !c
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Records what it is given, to compare a replay with the original
    /// calls.
    #[derive(Default)]
    struct Log(Vec<String>);

    impl Backend for Log {
        fn name(&self) -> &'static str {
            "log"
        }
        fn create_device(&mut self, d: &SwapChainDesc, f: u32) -> u32 {
            self.0.push(format!("device {d:?} {f}"));
            0
        }
        fn back_buffer(&mut self, id: u32) -> u32 {
            self.0.push(format!("bb {id}"));
            0
        }
        fn create(&mut self, id: u32, what: &Create<'_>) -> u32 {
            self.0.push(format!("create {id} {what:?}"));
            0
        }
        fn release(&mut self, id: u32) {
            self.0.push(format!("release {id}"));
        }
        fn op(&mut self, op: &Op<'_>) {
            self.0.push(format!("op {op:?}"));
        }
        fn read(&mut self, id: u32, sub: u32, row: u32, out: &mut [u8]) -> bool {
            self.0.push(format!("read {id} {sub} {row} {}", out.len()));
            false
        }
        fn query_data(&mut self, id: u32, out: &mut [u8]) -> u32 {
            self.0.push(format!("query {id} {}", out.len()));
            0
        }
        fn present(&mut self, s: u32, f: u32) -> u32 {
            self.0.push(format!("present {s} {f}"));
            0
        }
    }

    #[test]
    fn a_trace_replays_the_calls_it_recorded() {
        let dir = std::env::temp_dir().join(format!("ffx-trace-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.trace");
        let calls = |b: &mut dyn Backend| {
            b.create_device(&SwapChainDesc([7; 15]), 2);
            b.back_buffer(9);
            b.create(
                10,
                &Create::Buffer {
                    desc: [64, 2, 1, 1, 0, 0],
                    init: Some(&[1, 2, 3]),
                },
            );
            b.create(
                11,
                &Create::Texture {
                    dims: 2,
                    desc: &[4, 4, 1, 1, 28, 1, 0, 0, 8, 0, 0],
                    init: &[(16, 0, vec![5; 64])],
                },
            );
            b.create(
                12,
                &Create::Layout {
                    elements: &[Element {
                        semantic: "POSITION".into(),
                        index: 0,
                        format: 6,
                        slot: 0,
                        offset: 0,
                        per_instance: false,
                        step: 0,
                    }],
                    bytecode: b"DXBC",
                },
            );
            b.create(
                13,
                &Create::State {
                    kind: StateKind::Raster,
                    desc: &[3, 0, 0, 0],
                },
            );
            b.op(&Op::VertexBuffers(0, &[10], &[12], &[0]));
            b.op(&Op::ConstantBuffers(b'p', 1, &[10], Some((&[0], &[16]))));
            b.op(&Op::Viewports(&[[0.0, 0.0, 640.0, 480.0, 0.0, 1.0]]));
            b.op(&Op::Scissors(&[[-1, 2, 3, 4]]));
            b.op(&Op::Draw(DrawKind::IndexedInstanced, [1, 2, 3, 4, 5]));
            b.op(&Op::Unmapped(10, 0, 64, 64, &[9; 64], Some(&[(4, 8)])));
            b.op(&Op::Update(
                11,
                0,
                Some([0, 0, 0, 4, 4, 1]),
                &[1; 64],
                16,
                0,
            ));
            b.op(&Op::CopyRegion([1, 2, 3, 4, 5, 6, 7], None));
            b.op(&Op::ClearDepth(5, 3, 0.5, 7));
            b.op(&Op::ClearState);
            let mut buf = [0u8; 8];
            b.query_data(14, &mut buf);
            b.present(1, 0);
            b.release(10);
        };
        let mut tee = Tee::wrap(Box::new(Log::default()), Tee::create(&path).unwrap());
        calls(&mut tee);
        drop(tee);
        let mut direct = Log::default();
        calls(&mut direct);
        let mut replayed = Log::default();
        let r = replay(
            &read_file(&path).unwrap(),
            &mut replayed,
            &dir,
            &|_| false,
            None,
        )
        .unwrap();
        assert_eq!(replayed.0, direct.0);
        assert_eq!(r.presents, 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn crc_matches_the_png_reference() {
        // the CRC of "IEND" every PNG ends with
        assert_eq!(crc32(b"IEND"), 0xae42_6082);
    }
}

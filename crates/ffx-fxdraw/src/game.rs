//! The game state behind a frame's effect draws, read from a snapshot
//! taken at the capture's held point (`docs/formats/effects-draw.md`).
//!
//! - `g_PhyreScene`'s VFX lists: the mesh instances each `PhyreScene`
//!   pass renders, in order, and the draw events `DrawMostVFX` runs
//!   between them;
//! - `g_VFXRenderDataTable`: every effect instance's `VFXDynamicGeometry`
//!   (key, mesh instance, render data and its segments' vertex arrays);
//! - the instance key's ppp object and program (`InitVFXRenderData`'s
//!   key: `(object << 32) + program index`).

use crate::mem::Mem;

/// `g_PhyreScene` (`0x00ccb9d8`): the pointer to the `PhyreScene`.
pub const SCENE: u32 = 0x00cc_b9d8;
/// `g_VFXRenderDataTable` (`0x00ccca6c`): the pointer to the
/// `ClassVFXRenderDataTable`.
pub const TABLE: u32 = 0x00cc_ca6c;
/// `g_pppProgramTable` (`0x00c3a508`): 418 records of `0x28` bytes.
pub const PROGRAM_TABLE: u32 = 0x00c3_a508;
pub const PROGRAM_RECORDS: u32 = 418;
/// `VFXDynamicGeometry::vftable` (`0x00b47e40`).
pub const VFX_GEOMETRY_VTABLE: u32 = 0x00b4_7e40;

/// Which `PhyreScene` list (and so which pass) holds a mesh instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum List {
    /// `+0x38cc` (count `+0xf8cc`): `PhyreScene_DrawMostVFX` (`0x00650670`),
    /// Transparent pass, into the scene colour against the scene depth.
    Most,
    /// `+0x385c` (count `+0x3854`): `PhyreScene_RenderDTTVFX` (`0x006531b0`).
    Dtt,
    /// `+0x3868` (count `+0x3860`): `PhyreScene_DrawUI2DVFX` (`0x00650ea0`).
    Ui2d,
    /// `+0x3888` (count `+0x388c`): `PhyreScene_RenderVFXAfterPostProcess`
    /// (`0x006538d0`).
    AfterPost,
    /// `+0x3880` (count `+0x3878`): kind 0 with the geometry's byte `+0x50`
    /// set (`0x006565e0`).
    List3880,
}

impl List {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Most => "DrawMostVFX",
            Self::Dtt => "RenderDTTVFX",
            Self::Ui2d => "DrawUI2DVFX",
            Self::AfterPost => "RenderVFXAfterPostProcess",
            Self::List3880 => "list+0x3880",
        }
    }
}

/// A `PhyreScene_addVFXDrawEvent` record (16 bytes at `+0xfb5c`, count
/// `+0xfb54`): run by `DrawMostVFX` once `at` mesh instances are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    /// 0 camera/characters, 1 copy buffer, 2 screen fade, 3, 4 specular,
    /// 5 draw character, 6 z pre-pass (`PhyreScene_processVFXDrawEvent`).
    pub kind: u32,
    pub at: u32,
    pub words: [u32; 2],
}

/// One segment of an effect's render data (`0x6c` bytes; `docs/formats/render.md`,
/// `graphicDynGeoMemManagerGetBufferForSegment`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub addr: u32,
    pub vertices: u32,
    pub indices: u32,
    /// The arrays of the segment and of its two buffer slots, as
    /// (vertices, indices, positions, normals, colours, uvs, index array).
    pub buffers: Vec<Arrays>,
}

/// Vertex arrays of one buffer: 12 bytes a vertex (positions), 12
/// (normals), 16 (colours), 8 (uvs), 2 an index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Arrays {
    pub vertices: u32,
    pub indices: u32,
    pub positions: u32,
    pub normals: u32,
    pub colours: u32,
    pub uvs: u32,
    pub index: u32,
}

/// A `VFXDynamicGeometry` of the table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Geometry {
    pub addr: u32,
    pub key: u64,
    pub vtable: u32,
    /// `+0xc`: the mesh instance the scene lists hold.
    pub mesh_instance: u32,
    pub render_data: u32,
    pub segments: Vec<Segment>,
    /// In the table's delete pool (`ClassVFXRenderDataTable_InsertToDeletePool`),
    /// not in the map: destroyed, its segments kept until its counter runs
    /// out (`ClassVFXRenderDataTable_UpdateDeletePool` `0x0069fe50`).
    pub pooled: bool,
}

impl Geometry {
    /// The ppp object of the key (its high word).
    #[must_use]
    pub fn object(&self) -> u32 {
        (self.key >> 32) as u32
    }

    /// The key's low word: the program index in the object's part
    /// (`g_pppCurrentProgram`), or a caller's own value.
    #[must_use]
    pub fn low(&self) -> u32 {
        (self.key & 0xffff_ffff) as u32
    }
}

/// The scene's lists and the table, as the snapshot holds them.
#[derive(Debug, Clone, Default)]
pub struct State {
    pub scene: u32,
    pub lists: Vec<(List, Vec<u32>)>,
    pub events: Vec<Event>,
    pub geometries: Vec<Geometry>,
}

impl State {
    /// Reads the state; `None` when `g_PhyreScene` is not set.
    #[must_use]
    pub fn read(m: &Mem) -> Option<Self> {
        let scene = m.ptr(m.image(SCENE))?;
        let fixed = |base: u32, count: u32| -> Vec<u32> {
            let n = m.i32(scene + count).unwrap_or(0).clamp(0, 0xc00);
            (0..n.cast_unsigned())
                .map(|i| m.u32(scene + base + 4 * i).unwrap_or(0))
                .collect()
        };
        let array = |ptr: u32, count: u32| -> Vec<u32> {
            let n = m.i32(scene + count).unwrap_or(0).clamp(0, 0x10000);
            let Some(p) = m.ptr(scene + ptr) else {
                return Vec::new();
            };
            (0..n.cast_unsigned())
                .map(|i| m.u32(p + 4 * i).unwrap_or(0))
                .collect()
        };
        let lists = vec![
            (List::Most, fixed(0x38cc, 0xf8cc)),
            (List::Dtt, array(0x385c, 0x3854)),
            (List::Ui2d, array(0x3868, 0x3860)),
            (List::AfterPost, array(0x3888, 0x388c)),
            (List::List3880, array(0x3880, 0x3878)),
        ];
        let mut events = Vec::new();
        let n = m.u32(scene + 0xfb54).unwrap_or(0).min(0x1000);
        if let Some(p) = m.ptr(scene + 0xfb5c) {
            for i in 0..n {
                let e = p + 16 * i;
                events.push(Event {
                    kind: m.u32(e).unwrap_or(u32::MAX),
                    at: m.u32(e + 4).unwrap_or(0),
                    words: [m.u32(e + 8).unwrap_or(0), m.u32(e + 12).unwrap_or(0)],
                });
            }
        }
        Some(Self {
            scene,
            lists,
            events,
            geometries: geometries(m),
        })
    }

    /// The list (and place in it) that holds a mesh instance.
    #[must_use]
    pub fn find_instance(&self, mesh_instance: u32) -> Option<(List, usize)> {
        self.lists.iter().find_map(|(l, v)| {
            v.iter()
                .position(|&x| x == mesh_instance && x != 0)
                .map(|i| (*l, i))
        })
    }

    /// The geometry whose mesh instance is `mi`.
    #[must_use]
    pub fn geometry_of(&self, mi: u32) -> Option<&Geometry> {
        self.geometries.iter().find(|g| g.mesh_instance == mi)
    }
}

/// Every geometry of `g_VFXRenderDataTable`'s map, in key order (an
/// in-order walk of the MSVC tree: `+0` left, `+8` right, `+0xd` is-nil,
/// `+0x10` key, `+0x18` geometry).
#[must_use]
pub fn geometries(m: &Mem) -> Vec<Geometry> {
    let mut out = Vec::new();
    let Some(table) = m.ptr(m.image(TABLE)) else {
        return out;
    };
    let Some(head) = m.ptr(table) else {
        return out;
    };
    let mut stack = Vec::new();
    let mut node = m.u32(head + 4).unwrap_or(head);
    let mut guard = 0;
    loop {
        while node != head && node != 0 && m.u8(node + 0xd) == Some(0) && guard < 100_000 {
            guard += 1;
            stack.push(node);
            node = m.u32(node).unwrap_or(head);
        }
        let Some(n) = stack.pop() else { break };
        let lo = m.u32(n + 0x10).unwrap_or(0);
        let hi = m.u32(n + 0x14).unwrap_or(0);
        if let Some(g) = m.ptr(n + 0x18) {
            out.push(geometry(m, g, (u64::from(hi) << 32) | u64::from(lo)));
        }
        node = m.u32(n + 8).unwrap_or(head);
    }
    // The delete pool: count `+8` (bit 31 a flag), keys `+0xc` (8 bytes
    // each), counters `+0x14`, geometries `+0x1c`.
    let n = m.u32(table + 8).unwrap_or(0) & 0x7fff_ffff;
    if let (Some(keys), Some(geoms)) = (m.ptr(table + 0xc), m.ptr(table + 0x1c)) {
        for i in 0..n.min(0x10000) {
            let lo = m.u32(keys + 8 * i).unwrap_or(0);
            let hi = m.u32(keys + 8 * i + 4).unwrap_or(0);
            if lo == 0 && hi == 0 {
                continue;
            }
            if let Some(g) = m.ptr(geoms + 4 * i) {
                let mut geo = geometry(m, g, (u64::from(hi) << 32) | u64::from(lo));
                geo.pooled = true;
                out.push(geo);
            }
        }
    }
    out
}

fn arrays(m: &Mem, at: u32) -> Arrays {
    let w = |o: u32| m.u32(at + o).unwrap_or(0);
    Arrays {
        vertices: w(0),
        indices: w(4),
        positions: w(8),
        colours: w(0x10),
        normals: w(0xc),
        uvs: w(0x14),
        index: w(0x18),
    }
}

/// A geometry and its render data's segments.
#[must_use]
pub fn geometry(m: &Mem, g: u32, key: u64) -> Geometry {
    let render_data = m.u32(g + 0x24).unwrap_or(0);
    let mut segments = Vec::new();
    if render_data != 0 {
        let n = m.i32(render_data + 0x90).unwrap_or(0).clamp(0, 256);
        if let Some(base) = m.ptr(render_data + 0x94) {
            for i in 0..n.cast_unsigned() {
                let s = base + 0x6c * i;
                // The segment's own record (+4 counts, +0xc.. arrays) and
                // its two buffer slots (+0x2c + 0x1c n).
                let mut buffers = vec![arrays(m, s + 4)];
                buffers.push(arrays(m, s + 0x2c));
                buffers.push(arrays(m, s + 0x48));
                segments.push(Segment {
                    addr: s,
                    vertices: m.u32(s + 4).unwrap_or(0),
                    indices: m.u32(s + 8).unwrap_or(0),
                    buffers,
                });
            }
        }
    }
    Geometry {
        addr: g,
        key,
        vtable: m.u32(g).unwrap_or(0),
        mesh_instance: m.u32(g + 0xc).unwrap_or(0),
        render_data,
        segments,
        pooled: false,
    }
}

/// What an instance key says about its maker (`docs/formats/effects-draw.md`,
/// "Instance keys"). The key is `(lo, hi)` as the setup calls pass it
/// (`graphicVFXGetRenderDataByInstance(lo, hi)`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Maker {
    /// A ppp program: `(g_pppCurrentProgram, object)` from the program
    /// draws `pppRunPartStd` calls; the record and name when the walk
    /// resolves.
    Ppp {
        object: u32,
        index: u32,
        program: Option<(u32, String)>,
        /// The record has a draw (`+0xc`) or a runner (`+4`): only those
        /// call the setup functions. `false` means the key's index names a
        /// program that cannot have made the instance (an open question:
        /// `docs/formats/effects-draw.md`, "Instance keys").
        drawer: bool,
    },
    /// The op effect system: `(0, record)` (`opp_md`, `opp_anm`,
    /// `opp_anmm`, `opp_anm_yodare`, `opus_0283_draw`, `opus_0582_*`,
    /// `opu_mdc_break`, `opp_md_break_init`), the record in `FFX.exe`'s
    /// op pool or the heap.
    Op { record: u32 },
    /// Any other pair (fixed ids: `opus_0706_*` `(0x15, …)`, `(1|2, 0x2c2)`,
    /// `opus_0154_wave_draw` `(b, 0x9a)`, `setPoly*0434` `(0|1, id)`, the
    /// event sprites, the minimap fog, …).
    Other { lo: u32, hi: u32 },
}

impl Maker {
    /// Classifies a geometry's key.
    #[must_use]
    pub fn of(m: &Mem, overlay: Option<&Overlay>, g: &Geometry) -> Self {
        let (lo, hi) = (g.low(), g.object());
        // part i of a several-part effect sits at key + 10000 i
        let index = lo % 10_000;
        if hi != 0 && index < 0x100 {
            if let Some(p) = ppp_program(m, overlay, hi, index) {
                let rec = m.image(p.0);
                let drawer = m.u32(rec + 4).unwrap_or(0) != 0 || m.u32(rec + 0xc).unwrap_or(0) != 0;
                return Self::Ppp {
                    object: hi,
                    index,
                    program: Some(p),
                    drawer,
                };
            }
            // a ppp object whose program list we cannot read: the list in
            // an overlay no longer loaded, or the object itself in the
            // overlay's image (snapshots leave modules out)
            let def = m.ptr(hi + 8);
            let prog = def.and_then(|d| m.ptr(d));
            let in_overlay = overlay.is_some_and(|o| o.contains(hi));
            if lo != 0 && (prog.is_some() || in_overlay) {
                return Self::Ppp {
                    object: hi,
                    index,
                    program: None,
                    drawer: false,
                };
            }
        }
        if lo == 0 && hi != 0 {
            return Self::Op { record: hi };
        }
        Self::Other { lo, hi }
    }

    /// A short label.
    #[must_use]
    pub fn label(&self, m: &Mem) -> String {
        match self {
            Self::Ppp {
                object,
                index,
                program: Some((rec, name)),
                drawer,
            } => format!(
                "ppp {name} ({rec:#010x}){} obj {object:#x} prog {index}",
                if *drawer { "" } else { " NO-DRAW" }
            ),
            Self::Ppp { object, index, .. } => {
                format!("ppp ? obj {object:#x} prog {index}")
            }
            Self::Op { record } => format!("op record {:#x}", m.unrebase(*record)),
            Self::Other { lo, hi } => format!("key ({lo:#x}, {hi:#x})"),
        }
    }

    #[must_use]
    pub fn class(&self) -> &'static str {
        match self {
            Self::Ppp {
                program: Some(_),
                drawer: true,
                ..
            } => "ppp",
            Self::Ppp {
                program: Some(_), ..
            } => "ppp-nodraw",
            Self::Ppp { .. } => "ppp?",
            Self::Op { .. } => "op",
            Self::Other { .. } => "other",
        }
    }
}

/// `g_MagicFile_Handle` (`0x00c64cac`): the loaded spell overlay's module
/// base (`magicFiles/FFX/magic_<id>.dll`), its low half 0; -1 before.
pub const MAGIC_HANDLE: u32 = 0x00c6_4cac;
/// The playing spell's magic ID (`0x00c64ca0`; -1 when none).
pub const MAGIC_ID: u32 = 0x00c6_4ca0;

/// A spell overlay as its file maps it (sections at their RVAs). Snapshots
/// leave other modules' images out, and the overlays keep their ppp
/// program definitions in `.data`: on disk each program entry's first word
/// is its `g_pppProgramTable` index, which the loader turns into the
/// record's address.
#[derive(Debug, Clone)]
pub struct Overlay {
    pub id: u32,
    pub base: u32,
    pub image: Vec<u8>,
}

impl Overlay {
    /// Maps a PE file's headers and sections; `None` when it is no PE.
    #[must_use]
    pub fn map(id: u32, base: u32, file: &[u8]) -> Option<Self> {
        let rd = |at: usize| -> Option<u32> {
            file.get(at..at + 4)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        };
        let rd16 = |at: usize| -> Option<u16> {
            file.get(at..at + 2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
        };
        let pe = rd(0x3c)? as usize;
        if file.get(pe..pe + 4)? != b"PE\0\0" {
            return None;
        }
        let sections = usize::from(rd16(pe + 6)?);
        let opt = usize::from(rd16(pe + 20)?);
        let size = rd(pe + 24 + 56)? as usize;
        let headers = rd(pe + 24 + 60)? as usize;
        let mut image = vec![0u8; size.min(64 << 20)];
        let n = headers.min(file.len()).min(image.len());
        image[..n].copy_from_slice(&file[..n]);
        for i in 0..sections {
            let o = pe + 24 + opt + 40 * i;
            let va = rd(o + 12)? as usize;
            let raw_size = rd(o + 16)? as usize;
            let raw = rd(o + 20)? as usize;
            let src = file.get(raw..raw + raw_size)?;
            let n = raw_size.min(image.len().saturating_sub(va));
            image[va..va + n].copy_from_slice(&src[..n]);
        }
        Some(Self { id, base, image })
    }

    /// Loads `<game>/magicFiles/FFX/magic_<id>.dll` at the snapshot's
    /// overlay base, when a spell is loaded.
    #[must_use]
    pub fn from_game(m: &Mem, game: &std::path::Path) -> Option<Self> {
        let id = m.i32(m.image(MAGIC_ID))?;
        let base = m.u32(m.image(MAGIC_HANDLE))?;
        if id < 0 || base == u32::MAX || base & 0xffff != 0 {
            return None;
        }
        let path = game.join(format!("magicFiles/FFX/magic_{id:04}.dll"));
        let file = std::fs::read(path).ok()?;
        Self::map(id.cast_unsigned(), base, &file)
    }

    #[must_use]
    pub fn contains(&self, addr: u32) -> bool {
        addr.wrapping_sub(self.base) < u32::try_from(self.image.len()).unwrap_or(0)
    }

    #[must_use]
    pub fn u32(&self, addr: u32) -> Option<u32> {
        let at = addr.wrapping_sub(self.base) as usize;
        self.image
            .get(at..at + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    #[must_use]
    pub fn u16(&self, addr: u32) -> Option<u16> {
        let at = addr.wrapping_sub(self.base) as usize;
        self.image
            .get(at..at + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
    }
}

/// The ppp program record of an effect instance: the object's birth
/// record (`+8`), its program list (`+0`), entry `index` (`+0x28 + 0x10
/// index`, count `+0x26`), as `pppRunPartStd` (`0x00716c40`) walks them,
/// and the record's name (`+0`, a C string pointer). A program list inside
/// the spell overlay is read from its file, where the entry holds the
/// table index. `None` when the walk leaves readable memory or the record
/// is not in `g_pppProgramTable`.
#[must_use]
pub fn ppp_program(
    m: &Mem,
    overlay: Option<&Overlay>,
    object: u32,
    index: u32,
) -> Option<(u32, String)> {
    let def = m.ptr(object + 8)?;
    let prog = m.ptr(def)?;
    let table = m.image(PROGRAM_TABLE);
    let end = table + 0x28 * PROGRAM_RECORDS;
    let entry = prog + 0x28 + 0x10 * index;
    let (count, first) = match (m.u16(prog + 0x26), overlay) {
        (Some(c), _) => (c, m.u32(entry)?),
        (None, Some(o)) if o.contains(prog) => (o.u16(prog + 0x26)?, o.u32(entry)?),
        _ => return None,
    };
    if index >= u32::from(count) {
        return None;
    }
    let rec = if first < PROGRAM_RECORDS {
        table + 0x28 * first
    } else {
        first
    };
    if !(table..end).contains(&rec) || !(rec - table).is_multiple_of(0x28) {
        return None;
    }
    let name = m.ptr(rec).and_then(|p| m.cstr(p, 64))?;
    Some((m.unrebase(rec), name))
}

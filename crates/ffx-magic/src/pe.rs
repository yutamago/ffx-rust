//! The parts of a PE32 DLL the loader needs: sections, base relocations,
//! imports and exports. Read as data from the player's file; nothing is
//! executed here.

use crate::PeError as Error;

/// One section, as the image maps it.
#[derive(Debug, Clone)]
pub struct Section {
    pub name: String,
    /// Relative virtual address and size in memory.
    pub rva: u32,
    pub size: u32,
    /// Where its bytes are in the file (`raw_size` may be shorter than
    /// `size`: the rest is zero).
    pub raw: u32,
    pub raw_size: u32,
    /// `IMAGE_SCN_MEM_EXECUTE`.
    pub exec: bool,
}

/// One imported function: its IAT slot (RVA) and name.
#[derive(Debug, Clone)]
pub struct Import {
    pub dll: String,
    pub name: String,
    pub iat: u32,
}

/// A parsed DLL.
#[derive(Debug, Clone)]
pub struct Pe {
    pub preferred_base: u32,
    pub image_size: u32,
    pub headers_size: u32,
    pub entry: u32,
    pub sections: Vec<Section>,
    /// RVAs of the `HIGHLOW` fixups.
    pub relocs: Vec<u32>,
    pub imports: Vec<Import>,
    /// Name -> RVA.
    pub exports: Vec<(String, u32)>,
}

fn u16_at(d: &[u8], at: usize) -> Result<u16, Error> {
    d.get(at..at + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or_else(|| Error::BadImage(format!("read past the end at {at:#x}")))
}

fn u32_at(d: &[u8], at: usize) -> Result<u32, Error> {
    d.get(at..at + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| Error::BadImage(format!("read past the end at {at:#x}")))
}

fn cstr(d: &[u8], at: usize) -> Result<String, Error> {
    let tail = d
        .get(at..)
        .ok_or_else(|| Error::BadImage(format!("name past the end at {at:#x}")))?;
    let n = tail.iter().position(|&b| b == 0).unwrap_or(tail.len());
    Ok(String::from_utf8_lossy(&tail[..n]).into_owned())
}

impl Pe {
    /// Parses a PE32 (i386) DLL.
    ///
    /// # Errors
    /// Not a PE32 image, or a table points outside the file.
    #[allow(clippy::too_many_lines)] // headers, then the three tables in order
    pub fn parse(d: &[u8]) -> Result<Pe, Error> {
        if d.get(..2) != Some(b"MZ") {
            return Err(Error::BadImage("no MZ header".into()));
        }
        let pe = u32_at(d, 0x3c)? as usize;
        if d.get(pe..pe + 4) != Some(b"PE\0\0") {
            return Err(Error::BadImage("no PE header".into()));
        }
        if u16_at(d, pe + 4)? != 0x14c {
            return Err(Error::BadImage("not i386".into()));
        }
        let nsec = usize::from(u16_at(d, pe + 6)?);
        let optsz = usize::from(u16_at(d, pe + 20)?);
        let opt = pe + 24;
        if u16_at(d, opt)? != 0x10b {
            return Err(Error::BadImage("not PE32".into()));
        }
        let entry = u32_at(d, opt + 16)?;
        let preferred_base = u32_at(d, opt + 28)?;
        let image_size = u32_at(d, opt + 56)?;
        let headers_size = u32_at(d, opt + 60)?;
        let dir = |i: usize| -> Result<(u32, u32), Error> {
            Ok((u32_at(d, opt + 96 + 8 * i)?, u32_at(d, opt + 100 + 8 * i)?))
        };
        let mut sections = Vec::with_capacity(nsec);
        for i in 0..nsec {
            let s = opt + optsz + 40 * i;
            let raw_name = d
                .get(s..s + 8)
                .ok_or_else(|| Error::BadImage("section table past the end".into()))?;
            let name = String::from_utf8_lossy(raw_name)
                .trim_end_matches('\0')
                .to_owned();
            let vsize = u32_at(d, s + 8)?;
            let rva = u32_at(d, s + 12)?;
            let raw_size = u32_at(d, s + 16)?;
            let raw = u32_at(d, s + 20)?;
            let ch = u32_at(d, s + 36)?;
            sections.push(Section {
                name,
                rva,
                size: vsize.max(raw_size),
                raw,
                raw_size: raw_size.min(vsize.max(raw_size)),
                exec: ch & 0x2000_0000 != 0,
            });
        }
        let mut pe = Pe {
            preferred_base,
            image_size,
            headers_size,
            entry,
            sections,
            relocs: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
        };
        let layout = pe.clone();
        let off = |rva: u32| layout.file_offset(rva);

        // exports
        let (exp, _) = dir(0)?;
        if exp != 0 {
            let e = off(exp)?;
            let count = u32_at(d, e + 24)? as usize;
            let funcs = off(u32_at(d, e + 28)?)?;
            let names = off(u32_at(d, e + 32)?)?;
            let ords = off(u32_at(d, e + 36)?)?;
            for i in 0..count {
                let name = cstr(d, off(u32_at(d, names + 4 * i)?)?)?;
                let ord = usize::from(u16_at(d, ords + 2 * i)?);
                pe.exports.push((name, u32_at(d, funcs + 4 * ord)?));
            }
        }

        // imports
        let (imp, _) = dir(1)?;
        if imp != 0 {
            let mut desc = off(imp)?;
            loop {
                let names = u32_at(d, desc)?;
                let name = u32_at(d, desc + 12)?;
                let thunks = u32_at(d, desc + 16)?;
                if name == 0 {
                    break;
                }
                let dll = cstr(d, off(name)?)?;
                let list = if names == 0 { thunks } else { names };
                let mut i = 0;
                loop {
                    let v = u32_at(d, off(list + 4 * i)?)?;
                    if v == 0 {
                        break;
                    }
                    let fname = if v & 0x8000_0000 != 0 {
                        format!("#{}", v & 0xffff)
                    } else {
                        cstr(d, off(v + 2)?)?
                    };
                    pe.imports.push(Import {
                        dll: dll.clone(),
                        name: fname,
                        iat: thunks + 4 * i,
                    });
                    i += 1;
                }
                desc += 20;
            }
        }

        // base relocations
        let (rel, rel_size) = dir(5)?;
        if rel != 0 {
            let start = off(rel)?;
            let mut at = start;
            while at + 8 <= start + rel_size as usize {
                let page = u32_at(d, at)?;
                let block = u32_at(d, at + 4)? as usize;
                if block < 8 {
                    break;
                }
                for k in 0..(block - 8) / 2 {
                    let e = u16_at(d, at + 8 + 2 * k)?;
                    match e >> 12 {
                        0 => {}
                        3 => pe.relocs.push(page + u32::from(e & 0xfff)),
                        t => return Err(Error::BadImage(format!("relocation type {t}"))),
                    }
                }
                at += block;
            }
        }
        Ok(pe)
    }

    /// The file offset of an RVA inside a section's raw data.
    ///
    /// # Errors
    /// The RVA is in no section's file data.
    pub fn file_offset(&self, rva: u32) -> Result<usize, Error> {
        if rva < self.headers_size {
            return Ok(rva as usize);
        }
        self.sections
            .iter()
            .find(|s| rva >= s.rva && rva < s.rva + s.raw_size)
            .map(|s| (rva - s.rva + s.raw) as usize)
            .ok_or_else(|| Error::BadImage(format!("RVA {rva:#x} has no file data")))
    }

    /// The export's RVA.
    #[must_use]
    pub fn export(&self, name: &str) -> Option<u32> {
        self.exports.iter().find(|(n, _)| n == name).map(|e| e.1)
    }

    /// The image laid out at `base`: headers and sections copied, every
    /// fixup moved by `base - preferred_base`. Imports are left as the
    /// file has them.
    ///
    /// # Errors
    /// A section or fixup outside the image.
    pub fn layout(&self, file: &[u8], base: u32) -> Result<Vec<u8>, Error> {
        let mut img = vec![0u8; self.image_size as usize];
        let h = (self.headers_size as usize).min(file.len()).min(img.len());
        img[..h].copy_from_slice(&file[..h]);
        for s in &self.sections {
            let (a, n) = (s.rva as usize, s.raw_size as usize);
            let src = file
                .get(s.raw as usize..s.raw as usize + n)
                .ok_or_else(|| Error::BadImage(format!("section {} past the file", s.name)))?;
            img.get_mut(a..a + n)
                .ok_or_else(|| Error::BadImage(format!("section {} past the image", s.name)))?
                .copy_from_slice(src);
        }
        let delta = base.wrapping_sub(self.preferred_base);
        for &r in &self.relocs {
            let at = r as usize;
            let w = img
                .get_mut(at..at + 4)
                .ok_or_else(|| Error::BadImage(format!("fixup {r:#x} past the image")))?;
            let v = u32::from_le_bytes([w[0], w[1], w[2], w[3]]).wrapping_add(delta);
            w.copy_from_slice(&v.to_le_bytes());
        }
        Ok(img)
    }

    /// The executable section's range relative to the image base,
    /// page-rounded: the embedder keeps it inaccessible natively.
    ///
    /// # Errors
    /// No executable section, or more than one.
    pub fn text_range(&self) -> Result<(u32, u32), Error> {
        let mut exec = self.sections.iter().filter(|s| s.exec);
        let t = exec
            .next()
            .ok_or_else(|| Error::BadImage("no executable section".into()))?;
        if exec.next().is_some() {
            return Err(Error::BadImage("more than one executable section".into()));
        }
        Ok((t.rva & !0xfff, (t.rva + t.size + 0xfff) & !0xfff))
    }

    /// The image's footprint, 64 KiB-rounded.
    #[must_use]
    pub fn span(&self) -> u32 {
        self.image_size.next_multiple_of(IMAGE_ALIGN)
    }
}

/// 64 KiB, the granularity images are placed at.
pub const IMAGE_ALIGN: u32 = 0x1_0000;

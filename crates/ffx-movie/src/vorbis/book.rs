//! Codebooks: libvorbis 1.3.5's `codebook.c` (unpacking and decoding) and
//! `sharedbook.c` (the decode tables), as linked into FFX.exe.
//!
//! Float rules as the game's build computes them (MSVC, SSE2): the
//! unquantized values are `fabs(q) * delta + mindel + last` in double,
//! stored as float (`0x00a92823..0x00a92871`).

#![allow(clippy::many_single_char_names)] // libvorbis' names

use super::bits::Reader;

/// `ov_ilog`.
#[must_use]
pub fn ilog(mut v: u32) -> i32 {
    let mut r = 0;
    while v != 0 {
        r += 1;
        v >>= 1;
    }
    r
}

/// `static_codebook`: a codebook as the setup header stores it.
#[derive(Debug, Clone, Default)]
pub struct StaticBook {
    pub dim: i32,
    pub entries: i32,
    pub lengthlist: Vec<i32>,
    pub maptype: i32,
    pub q_min: i32,
    pub q_delta: i32,
    pub q_quant: i32,
    pub q_sequencep: i32,
    pub quantlist: Vec<i32>,
}

/// `_book_maptype1_quantvals`.
fn maptype1_quantvals(b: &StaticBook) -> i32 {
    if b.dim <= 0 {
        // the original loops forever here (malformed books only)
        return 0;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    let mut vals = f64::from((b.entries as f32).powf(1.0 / b.dim as f32)).floor() as i32;
    // (the C computes pow in double from the float arguments; the integer
    // refinement below makes the guess exact either way)
    loop {
        let mut acc: i32 = 1;
        let mut acc1: i32 = 1;
        for _ in 0..b.dim {
            acc = acc.wrapping_mul(vals);
            acc1 = acc1.wrapping_mul(vals + 1);
        }
        if acc <= b.entries && acc1 > b.entries {
            return vals;
        } else if acc > b.entries {
            vals -= 1;
        } else {
            vals += 1;
        }
    }
}

/// `vorbis_staticbook_unpack`.
#[must_use]
pub fn unpack(opb: &mut Reader) -> Option<StaticBook> {
    let mut s = StaticBook::default();
    if opb.read(24) != 0x0056_4342 {
        return None;
    }
    s.dim = opb.read(16);
    s.entries = opb.read(24);
    if s.entries == -1 {
        return None;
    }
    if ilog(s.dim.cast_unsigned()) + ilog(s.entries.cast_unsigned()) > 24 {
        return None;
    }
    let entries = usize::try_from(s.entries).ok()?;
    match opb.read(1) {
        0 => {
            let unused = opb.read(1);
            if (s.entries * if unused != 0 { 1 } else { 5 } + 7) >> 3 > opb.storage() - opb.bytes()
            {
                return None;
            }
            s.lengthlist = vec![0; entries];
            for l in &mut s.lengthlist {
                if unused != 0 {
                    if opb.read(1) != 0 {
                        let num = opb.read(5);
                        if num == -1 {
                            return None;
                        }
                        *l = num + 1;
                    } else {
                        *l = 0;
                    }
                } else {
                    let num = opb.read(5);
                    if num == -1 {
                        return None;
                    }
                    *l = num + 1;
                }
            }
        }
        1 => {
            let mut length = opb.read(5) + 1;
            if length == 0 {
                return None;
            }
            s.lengthlist = vec![0; entries];
            let mut i = 0i32;
            while i < s.entries {
                let num = opb.read(ilog((s.entries - i).cast_unsigned()));
                if num == -1 {
                    return None;
                }
                if length > 32 || num > s.entries - i || (num > 0 && (num - 1) >> (length - 1) > 1)
                {
                    return None;
                }
                for _ in 0..num {
                    s.lengthlist[usize::try_from(i).ok()?] = length;
                    i += 1;
                }
                length += 1;
            }
        }
        _ => return None,
    }
    s.maptype = opb.read(4);
    match s.maptype {
        0 => {}
        1 | 2 => {
            s.q_min = opb.read(32);
            s.q_delta = opb.read(32);
            s.q_quant = opb.read(4) + 1;
            s.q_sequencep = opb.read(1);
            if s.q_sequencep == -1 {
                return None;
            }
            let quantvals = if s.maptype == 1 {
                if s.dim == 0 {
                    0
                } else {
                    maptype1_quantvals(&s)
                }
            } else {
                s.entries * s.dim
            };
            if (quantvals * s.q_quant + 7) >> 3 > opb.storage() - opb.bytes() {
                return None;
            }
            s.quantlist = (0..quantvals).map(|_| opb.read(s.q_quant)).collect();
            if quantvals != 0 && s.quantlist.last() == Some(&-1) {
                return None;
            }
        }
        _ => return None,
    }
    Some(s)
}

/// `_float32_unpack`: the packed float of the setup header, via `ldexp`
/// in double, returned as float.
#[must_use]
pub fn float32_unpack(val: i32) -> f32 {
    let v = val.cast_unsigned();
    let mut mant = f64::from(v & 0x001f_ffff);
    if v & 0x8000_0000 != 0 {
        mant = -mant;
    }
    let exp = ((v & 0x7fe0_0000) >> 21).cast_signed();
    #[allow(clippy::cast_possible_truncation)]
    let r = (mant * 2f64.powi(exp - 20 - 768)) as f32;
    r
}

/// `_make_words`: the codewords of the used entries, bit-reversed.
fn make_words(l: &[i32], sparsecount: usize) -> Option<Vec<u32>> {
    let n = l.len();
    let mut marker = [0u32; 33];
    let mut r = vec![0u32; if sparsecount != 0 { sparsecount } else { n }];
    let mut count = 0usize;
    for &length in l {
        if length > 0 {
            let lu = length.cast_unsigned() as usize;
            let mut entry = marker[lu];
            if length < 32 && (entry >> length) != 0 {
                return None;
            }
            *r.get_mut(count)? = entry;
            count += 1;
            let mut j = lu;
            while j > 0 {
                if marker[j] & 1 != 0 {
                    if j == 1 {
                        marker[1] += 1;
                    } else {
                        marker[j] = marker[j - 1] << 1;
                    }
                    break;
                }
                marker[j] = marker[j].wrapping_add(1);
                j -= 1;
            }
            for j in lu + 1..33 {
                if (marker[j] >> 1) == entry {
                    entry = marker[j];
                    marker[j] = marker[j - 1] << 1;
                } else {
                    break;
                }
            }
        } else if sparsecount == 0 {
            count += 1;
        }
    }
    if !(count == 1 && marker[2] == 2) {
        for (i, &m) in marker.iter().enumerate().skip(1) {
            if m & (u32::MAX >> (32 - i)) != 0 {
                return None;
            }
        }
    }
    let mut count = 0usize;
    for &len in l {
        let mut temp = 0u32;
        for j in 0..len.max(0) {
            temp <<= 1;
            temp |= (r[count] >> j) & 1;
        }
        if sparsecount != 0 {
            if len != 0 {
                r[count] = temp;
                count += 1;
            }
        } else {
            r[count] = temp;
            count += 1;
        }
    }
    Some(r)
}

/// `bitreverse`.
#[must_use]
pub fn bitreverse(x: u32) -> u32 {
    x.reverse_bits()
}

/// `codebook`: a decode-ready book (`vorbis_book_init_decode`).
#[derive(Debug, Clone, Default)]
pub struct Book {
    pub dim: i32,
    pub entries: i32,
    pub used_entries: i32,
    pub valuelist: Vec<f32>,
    pub codelist: Vec<u32>,
    pub dec_index: Vec<i32>,
    pub dec_codelengths: Vec<i32>,
    pub dec_firsttable: Vec<u32>,
    pub dec_firsttablen: i32,
    pub dec_maxlength: i32,
}

/// `_book_unquantize(b, n, sparsemap)`.
fn unquantize(b: &StaticBook, n: usize, sparsemap: &[usize]) -> Vec<f32> {
    if b.maptype != 1 && b.maptype != 2 {
        return Vec::new();
    }
    let mindel = float32_unpack(b.q_min);
    let delta = float32_unpack(b.q_delta);
    let dim = usize::try_from(b.dim).unwrap_or(0);
    let mut r = vec![0f32; n * dim];
    let quantvals = if b.maptype == 1 {
        maptype1_quantvals(b)
    } else {
        0
    };
    let mut count = 0usize;
    for j in 0..usize::try_from(b.entries).unwrap_or(0) {
        if b.lengthlist[j] == 0 {
            continue;
        }
        let mut last = 0f32;
        let mut indexdiv = 1i32;
        for k in 0..dim {
            let q = if b.maptype == 1 {
                let index = (i32::try_from(j).unwrap_or(0) / indexdiv) % quantvals;
                b.quantlist[usize::try_from(index).unwrap_or(0)]
            } else {
                b.quantlist[j * dim + k]
            };
            #[allow(clippy::cast_precision_loss)] // (float)quantlist[i], cvtdq2ps
            let qf = q as f32;
            #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
            let val = (f64::from(qf).abs() * f64::from(delta) + f64::from(mindel) + f64::from(last))
                as f32;
            if b.q_sequencep != 0 {
                last = val;
            }
            r[sparsemap[count] * dim + k] = val;
            indexdiv = indexdiv.wrapping_mul(quantvals);
        }
        count += 1;
    }
    r
}

impl Book {
    /// `vorbis_book_init_decode`.
    #[must_use]
    pub fn init_decode(s: &StaticBook) -> Option<Self> {
        let n = s.lengthlist.iter().filter(|&&l| l > 0).count();
        let mut c = Book {
            entries: s.entries,
            used_entries: i32::try_from(n).ok()?,
            dim: s.dim,
            ..Book::default()
        };
        if n == 0 {
            return Some(c);
        }
        let codes: Vec<u32> = make_words(&s.lengthlist, n)?
            .into_iter()
            .map(bitreverse)
            .collect();
        // sort the codes (unique), keeping where each came from
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by_key(|&i| codes[i]);
        let mut sortindex = vec![0usize; n];
        for (i, &position) in order.iter().enumerate() {
            sortindex[position] = i;
        }
        c.codelist = vec![0; n];
        for i in 0..n {
            c.codelist[sortindex[i]] = codes[i];
        }
        c.valuelist = unquantize(s, n, &sortindex);
        c.dec_index = vec![0; n];
        c.dec_codelengths = vec![0; n];
        let mut k = 0usize;
        for (i, &l) in s.lengthlist.iter().enumerate() {
            if l > 0 {
                c.dec_index[sortindex[k]] = i32::try_from(i).ok()?;
                c.dec_codelengths[sortindex[k]] = l;
                c.dec_maxlength = c.dec_maxlength.max(l);
                k += 1;
            }
        }
        if n == 1 && c.dec_maxlength == 1 {
            c.dec_firsttablen = 1;
            c.dec_firsttable = vec![1, 1];
        } else {
            c.dec_firsttablen = (ilog(c.used_entries.cast_unsigned()) - 4).clamp(5, 8);
            let tabn = 1usize << c.dec_firsttablen;
            c.dec_firsttable = vec![0; tabn];
            let ft = c.dec_firsttablen;
            for i in 0..n {
                let len = c.dec_codelengths[i];
                if len <= ft {
                    let orig = bitreverse(c.codelist[i]);
                    for j in 0..(1u32 << (ft - len)) {
                        c.dec_firsttable[(orig | (j << len)) as usize] =
                            u32::try_from(i + 1).ok()?;
                    }
                }
            }
            let mask = 0xffff_fffeu32 << (31 - ft);
            let (mut lo, mut hi) = (0usize, 0usize);
            for i in 0..tabn {
                let word = u32::try_from(i).ok()? << (32 - ft);
                let slot = bitreverse(word) as usize;
                if c.dec_firsttable[slot] == 0 {
                    while lo + 1 < n && c.codelist[lo + 1] <= word {
                        lo += 1;
                    }
                    while hi < n && word >= (c.codelist[hi] & mask) {
                        hi += 1;
                    }
                    let loval = u32::try_from(lo).ok()?.min(0x7fff);
                    let hival = u32::try_from(n - hi).ok()?.min(0x7fff);
                    c.dec_firsttable[slot] = 0x8000_0000 | (loval << 15) | hival;
                }
            }
        }
        Some(c)
    }

    /// `decode_packed_entry_number`.
    fn decode_packed(&self, b: &mut Reader) -> i32 {
        let mut read = self.dec_maxlength;
        let (mut lo, mut hi);
        let lok = b.look(self.dec_firsttablen);
        if lok >= 0 {
            let entry = self.dec_firsttable[lok.cast_unsigned() as usize];
            if entry & 0x8000_0000 != 0 {
                lo = ((entry >> 15) & 0x7fff).cast_signed();
                hi = self.used_entries - (entry & 0x7fff).cast_signed();
            } else {
                let e = entry.cast_signed() - 1;
                b.adv(self.dec_codelengths[e.cast_unsigned() as usize]);
                return e;
            }
        } else {
            lo = 0;
            hi = self.used_entries;
        }
        let mut lok = b.look(read);
        while lok < 0 && read > 1 {
            read -= 1;
            lok = b.look(read);
        }
        if lok < 0 {
            return -1;
        }
        let testword = bitreverse(lok.cast_unsigned());
        while hi - lo > 1 {
            let p = (hi - lo) >> 1;
            let test = i32::from(self.codelist[(lo + p).cast_unsigned() as usize] > testword);
            lo += p & (test - 1);
            hi -= p & (-test);
        }
        let len = self.dec_codelengths[lo.cast_unsigned() as usize];
        if len <= read {
            b.adv(len);
            return lo;
        }
        b.adv(read);
        -1
    }

    /// `vorbis_book_decode`: the original entry number, -1 at the end.
    pub fn decode(&self, b: &mut Reader) -> i32 {
        if self.used_entries > 0 {
            let p = self.decode_packed(b);
            if p >= 0 {
                return self.dec_index[p.cast_unsigned() as usize];
            }
        }
        -1
    }

    fn values(&self, entry: i32) -> &[f32] {
        let dim = self.dim.cast_unsigned() as usize;
        let start = entry.cast_unsigned() as usize * dim;
        self.valuelist.get(start..start + dim).unwrap_or(&[])
    }

    /// `vorbis_book_decodevs_add` (residue 0): interleaved by `step`.
    pub fn decodevs_add(&self, a: &mut [f32], b: &mut Reader, n: i32) -> i32 {
        if self.used_entries > 0 && self.dim > 0 {
            let step = usize::try_from(n / self.dim).unwrap_or(0);
            let mut entries = Vec::with_capacity(step);
            for _ in 0..step {
                let e = self.decode_packed(b);
                if e == -1 {
                    return -1;
                }
                entries.push(e);
            }
            let dim = self.dim.cast_unsigned() as usize;
            for i in 0..dim {
                let o = i * step;
                for (j, &e) in entries.iter().enumerate() {
                    if let (Some(x), Some(&t)) = (a.get_mut(o + j), self.values(e).get(i)) {
                        *x += t;
                    }
                }
            }
        }
        0
    }

    /// `vorbis_book_decodev_add` (residue 1).
    pub fn decodev_add(&self, a: &mut [f32], b: &mut Reader, n: i32) -> i32 {
        if self.used_entries > 0 {
            let n = usize::try_from(n).unwrap_or(0);
            let mut i = 0usize;
            while i < n {
                let e = self.decode_packed(b);
                if e == -1 {
                    return -1;
                }
                for &t in self.values(e) {
                    if let Some(x) = a.get_mut(i) {
                        *x += t;
                    }
                    i += 1;
                }
                if self.dim == 0 {
                    break;
                }
            }
        }
        0
    }

    /// `vorbis_book_decodevv_add` (residue 2): interleaved over `ch`
    /// channels from `offset`.
    pub fn decodevv_add(
        &self,
        a: &mut [&mut [f32]],
        offset: i32,
        ch: i32,
        b: &mut Reader,
        n: i32,
    ) -> i32 {
        if self.used_entries > 0 && ch > 0 {
            let mut chptr = 0usize;
            let ch_u = ch.cast_unsigned() as usize;
            let mut i = offset / ch;
            let end = (offset + n) / ch;
            while i < end {
                let e = self.decode_packed(b);
                if e == -1 {
                    return -1;
                }
                for &t in self.values(e) {
                    if let Some(x) = a[chptr].get_mut(i.cast_unsigned() as usize) {
                        *x += t;
                    }
                    chptr += 1;
                    if chptr == ch_u {
                        chptr = 0;
                        i += 1;
                    }
                }
                if self.dim == 0 {
                    break;
                }
            }
        }
        0
    }
}

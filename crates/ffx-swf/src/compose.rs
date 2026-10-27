//! Image helpers for the pilot's comparison: alpha compositing (straight
//! alpha, integer arithmetic) and difference images.

use image::{Rgb, RgbImage, Rgba, RgbaImage};

fn u8_of(v: u32) -> u8 {
    u8::try_from(v).unwrap_or(u8::MAX)
}

/// `top` over `base`, both straight alpha, into `base`.
pub fn over(base: &mut RgbaImage, top: &RgbaImage) {
    for (b, t) in base.pixels_mut().zip(top.pixels()) {
        let (ta, ba) = (u32::from(t[3]), u32::from(b[3]));
        // alpha scaled by 255
        let oa = ta * 255 + ba * (255 - ta);
        if oa == 0 {
            *b = Rgba([0, 0, 0, 0]);
            continue;
        }
        for c in 0..3 {
            let num = u32::from(t[c]) * ta * 255 + u32::from(b[c]) * ba * (255 - ta);
            b[c] = u8_of((num + oa / 2) / oa);
        }
        b[3] = u8_of((oa + 127) / 255);
    }
}

/// `top` (straight alpha) over an opaque `base`.
#[must_use]
pub fn over_opaque(base: &RgbImage, top: &RgbaImage) -> RgbImage {
    let mut out = base.clone();
    for (o, t) in out.pixels_mut().zip(top.pixels()) {
        let a = u32::from(t[3]);
        for c in 0..3 {
            o[c] = u8_of((u32::from(t[c]) * a + u32::from(o[c]) * (255 - a) + 127) / 255);
        }
    }
    out
}

/// Per-pixel difference of two images of one size: the image (the largest
/// channel difference, amplified 4x, red over a dimmed copy of `a`) and the
/// counts of pixels differing by more than 8 and 32, and the mean absolute
/// channel difference.
#[must_use]
#[allow(clippy::many_single_char_names)]
pub fn difference(a: &RgbImage, b: &RgbImage) -> (RgbImage, Diff) {
    let mut out = RgbImage::new(a.width(), a.height());
    let mut d = Diff::default();
    let mut sum = 0u64;
    let mut signed = [0i64; 3];
    for ((pa, pb), o) in a.pixels().zip(b.pixels()).zip(out.pixels_mut()) {
        let m = (0..3).map(|c| pa[c].abs_diff(pb[c])).max().unwrap_or(0);
        sum += (0..3)
            .map(|c| u64::from(pa[c].abs_diff(pb[c])))
            .sum::<u64>();
        for (c, acc) in signed.iter_mut().enumerate() {
            *acc += i64::from(pb[c]) - i64::from(pa[c]);
        }
        if m > 8 {
            d.over8 += 1;
        }
        if m > 32 {
            d.over32 += 1;
        }
        let grey = u8_of((u32::from(pa[0]) + u32::from(pa[1]) + u32::from(pa[2])) / 12);
        let r = u8_of(u32::from(m) * 4);
        *o = Rgb([r.max(grey), grey, grey]);
    }
    d.pixels = u64::from(a.width()) * u64::from(a.height());
    d.mean_milli = if d.pixels == 0 {
        0
    } else {
        sum * 1000 / (3 * d.pixels)
    };
    let n = i64::try_from(d.pixels.max(1)).unwrap_or(i64::MAX);
    d.bias_milli = signed.map(|v| v * 1000 / n);
    (out, d)
}

/// What [`difference`] counted.
#[derive(Debug, Default, Clone, Copy)]
pub struct Diff {
    pub pixels: u64,
    pub over8: u64,
    pub over32: u64,
    /// Mean absolute channel difference, in thousandths.
    pub mean_milli: u64,
    /// Mean of `b - a` per channel (R, G, B), in thousandths: a colour or
    /// brightness shift.
    pub bias_milli: [i64; 3],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn over_and_difference() {
        let mut base = RgbaImage::from_pixel(1, 1, Rgba([0, 0, 255, 255]));
        over(
            &mut base,
            &RgbaImage::from_pixel(1, 1, Rgba([255, 0, 0, 128])),
        );
        assert_eq!(base.get_pixel(0, 0).0, [128, 0, 127, 255]);
        let mut clear = RgbaImage::new(1, 1);
        over(
            &mut clear,
            &RgbaImage::from_pixel(1, 1, Rgba([10, 20, 30, 40])),
        );
        assert_eq!(clear.get_pixel(0, 0).0, [10, 20, 30, 40]);
        let a = RgbImage::from_pixel(2, 1, Rgb([100, 100, 100]));
        let b = over_opaque(&a, &RgbaImage::from_pixel(2, 1, Rgba([200, 100, 100, 255])));
        let (_, d) = difference(&a, &b);
        assert_eq!((d.over8, d.over32, d.mean_milli), (2, 2, 33_333));
    }
}

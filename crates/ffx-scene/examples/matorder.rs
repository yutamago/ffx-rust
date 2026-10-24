//! Scratch: which summation order gives WorldViewProjection bit for bit.
use ffx_scene::capture::Capture;

type M = [f32; 16];

fn get(d: &ffx_gfx::Draw, off: usize) -> Option<M> {
    let data = &d.vs_cb.iter().find(|c| c.0 == 0)?.1;
    let b = data.get(off..off + 64)?;
    let mut m = [0f32; 16];
    for (i, c) in b.chunks_exact(4).enumerate() {
        m[i] = f32::from_le_bytes(c.try_into().ok()?);
    }
    Some(m)
}

fn var_offset(code: &[u8], name: &str) -> Option<usize> {
    let r = ffx_dxbc::Container::parse(code).ok()?.reflection().ok()??;
    for b in &r.buffers {
        for v in &b.variables {
            if v.name == name {
                return Some(v.offset as usize);
            }
            for m in &v.members {
                if format!("{}.{}", v.name, m.name) == name {
                    return Some((v.offset + m.offset) as usize);
                }
            }
        }
    }
    None
}

fn prod(a: &M, b: &M, f: fn([f32; 4], [f32; 4]) -> f32) -> M {
    let mut o = [0f32; 16];
    for i in 0..4 {
        for j in 0..4 {
            let x = [a[i * 4], a[i * 4 + 1], a[i * 4 + 2], a[i * 4 + 3]];
            let y = [b[j], b[4 + j], b[8 + j], b[12 + j]];
            o[i * 4 + j] = f(x, y);
        }
    }
    o
}

fn main() {
    let path = std::env::args().nth(1).expect("capture");
    let cap = Capture::read(&std::fs::read(path).expect("read")).expect("capture");
    let orders: Vec<(&str, fn([f32; 4], [f32; 4]) -> f32)> = vec![
        ("seq 0123", |x, y| ((x[0] * y[0] + x[1] * y[1]) + x[2] * y[2]) + x[3] * y[3]),
        ("seq 3210", |x, y| ((x[3] * y[3] + x[2] * y[2]) + x[1] * y[1]) + x[0] * y[0]),
        ("pair (01)(23)", |x, y| (x[0] * y[0] + x[1] * y[1]) + (x[2] * y[2] + x[3] * y[3])),
        ("pair (02)(13)", |x, y| (x[0] * y[0] + x[2] * y[2]) + (x[1] * y[1] + x[3] * y[3])),
        ("seq 0213", |x, y| ((x[0] * y[0] + x[2] * y[2]) + x[1] * y[1]) + x[3] * y[3]),
        ("f64 once", |x, y| {
            (f64::from(x[0]) * f64::from(y[0])
                + f64::from(x[1]) * f64::from(y[1])
                + f64::from(x[2]) * f64::from(y[2])
                + f64::from(x[3]) * f64::from(y[3])) as f32
        }),
        ("fma seq", |x, y| {
            x[3].mul_add(y[3], x[2].mul_add(y[2], x[1].mul_add(y[1], x[0] * y[0])))
        }),
    ];
    let mut score = vec![0usize; orders.len()];
    let mut n = 0;
    for (_, d) in cap.draws() {
        if Capture::scene_targets(d).is_none() {
            continue;
        }
        let Some(code) = cap.shader(d.vs) else { continue };
        // args: capture target first second (target = first . second, stored order)
        let names: Vec<String> = std::env::args().skip(2).collect();
        let (Some(owvp), Some(ovp), Some(ow)) = (
            var_offset(code, &names[0]),
            var_offset(code, &names[1]),
            var_offset(code, &names[2]),
        ) else {
            continue;
        };
        let (Some(w), Some(vp), Some(wvp)) = (get(d, ow), get(d, ovp), get(d, owvp)) else {
            continue;
        };
        n += 1;
        for (k, (_, f)) in orders.iter().enumerate() {
            if prod(&vp, &w, *f).iter().zip(&wvp).all(|(a, b)| a.to_bits() == b.to_bits()) {
                score[k] += 1;
            }
        }
    }
    for (k, (name, _)) in orders.iter().enumerate() {
        println!("{name}: {}/{n}", score[k]);
    }
}

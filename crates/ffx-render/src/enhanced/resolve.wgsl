// The last enhanced pass: the tone-mapped image (rendered at the render
// scale) into the game's targets, with FXAA or the supersampling filter;
// the game's normal encoding and depth come along at its resolution.

struct Resolve {
    // x: render scale, y: FXAA on, zw: the game target's size
    params: vec4<f32>,
    // Rows of the view matrix, for the game's view-space normals.
    view: array<vec4<f32>, 4>,
    // x: the scene's clear alpha (where nothing was drawn), y: sharpening
    clear: vec4<f32>,
};

@group(0) @binding(0) var<uniform> res: Resolve;
@group(0) @binding(1) var ldr_tex: texture_2d<f32>;
@group(0) @binding(2) var ldr_samp: sampler;
@group(0) @binding(3) var normal_tex: texture_2d<f32>;
@group(0) @binding(4) var depth_tex: texture_depth_2d;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
};

@vertex
fn vs(@builtin(vertex_index) id: u32) -> VsOut {
    let uv = vec2<f32>(f32((id << 1u) & 2u), f32(id & 2u));
    var out: VsOut;
    out.pos = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    return out;
}

fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.299, 0.587, 0.114));
}

fn sample(uv: vec2<f32>) -> vec3<f32> {
    return textureSampleLevel(ldr_tex, ldr_samp, uv, 0.0).rgb;
}

// FXAA in the spirit of 3.11 (edge search along the local gradient).
fn fxaa(uv: vec2<f32>, texel: vec2<f32>) -> vec3<f32> {
    let c = sample(uv);
    let n = luma(sample(uv + vec2<f32>(0.0, -texel.y)));
    let s = luma(sample(uv + vec2<f32>(0.0, texel.y)));
    let e = luma(sample(uv + vec2<f32>(texel.x, 0.0)));
    let w = luma(sample(uv + vec2<f32>(-texel.x, 0.0)));
    let m = luma(c);
    let lo = min(m, min(min(n, s), min(e, w)));
    let hi = max(m, max(max(n, s), max(e, w)));
    let range = hi - lo;
    if (range < max(0.0312, hi * 0.125)) {
        return c;
    }
    let nw = luma(sample(uv + vec2<f32>(-texel.x, -texel.y)));
    let ne = luma(sample(uv + vec2<f32>(texel.x, -texel.y)));
    let sw = luma(sample(uv + vec2<f32>(-texel.x, texel.y)));
    let se = luma(sample(uv + vec2<f32>(texel.x, texel.y)));
    let horizontal = abs(nw + ne - 2.0 * n) + 2.0 * abs(w + e - 2.0 * m) + abs(sw + se - 2.0 * s);
    let vertical = abs(nw + sw - 2.0 * w) + 2.0 * abs(n + s - 2.0 * m) + abs(ne + se - 2.0 * e);
    let is_h = horizontal >= vertical;
    let step_len = select(texel.x, texel.y, is_h);
    var l1 = select(w, n, is_h);
    var l2 = select(e, s, is_h);
    let g1 = abs(l1 - m);
    let g2 = abs(l2 - m);
    var step = step_len;
    var edge_luma = (l2 + m) * 0.5;
    var gradient = g2;
    if (g1 >= g2) {
        step = -step_len;
        edge_luma = (l1 + m) * 0.5;
        gradient = g1;
    }
    var edge_uv = uv;
    if (is_h) {
        edge_uv.y = edge_uv.y + step * 0.5;
    } else {
        edge_uv.x = edge_uv.x + step * 0.5;
    }
    let along = select(vec2<f32>(0.0, texel.y), vec2<f32>(texel.x, 0.0), is_h);
    var p1 = edge_uv - along;
    var p2 = edge_uv + along;
    var d1 = luma(sample(p1)) - edge_luma;
    var d2 = luma(sample(p2)) - edge_luma;
    var done1 = abs(d1) >= gradient * 0.25;
    var done2 = abs(d2) >= gradient * 0.25;
    for (var i = 0; i < 10; i = i + 1) {
        if (done1 && done2) {
            break;
        }
        if (!done1) {
            p1 = p1 - along * 1.5;
            d1 = luma(sample(p1)) - edge_luma;
            done1 = abs(d1) >= gradient * 0.25;
        }
        if (!done2) {
            p2 = p2 + along * 1.5;
            d2 = luma(sample(p2)) - edge_luma;
            done2 = abs(d2) >= gradient * 0.25;
        }
    }
    let dist1 = select(uv.y - p1.y, uv.x - p1.x, is_h);
    let dist2 = select(p2.y - uv.y, p2.x - uv.x, is_h);
    let shorter = min(dist1, dist2);
    let length = dist1 + dist2;
    let pixel_offset = 0.5 - shorter / length;
    let correct = ((select(d2, d1, dist1 < dist2)) < 0.0) != (m < edge_luma);
    var offset = select(0.0, pixel_offset, correct);
    // Subpixel aliasing.
    let avg = (2.0 * (n + s + e + w) + nw + ne + sw + se) / 12.0;
    let sub = clamp(abs(avg - m) / range, 0.0, 1.0);
    let sub2 = (-2.0 * sub + 3.0) * sub * sub;
    offset = max(offset, sub2 * sub2 * 0.75);
    var final_uv = uv;
    if (is_h) {
        final_uv.y = final_uv.y + offset * step;
    } else {
        final_uv.x = final_uv.x + offset * step;
    }
    return sample(final_uv);
}

struct Out {
    @location(0) color: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @builtin(frag_depth) depth: f32,
};

@fragment
fn fs(in: VsOut) -> Out {
    let size = res.params.zw;
    let uv = in.pos.xy / size;
    let scale = res.params.x;
    let src = vec2<f32>(textureDimensions(ldr_tex));
    var c: vec3<f32>;
    if (res.params.y > 0.5) {
        c = fxaa(uv, 1.0 / src);
    } else if (scale > 1.01) {
        // Supersampled: a tent over the rendered pixels under this one.
        let t = 1.0 / src;
        c = (sample(uv + vec2<f32>(-0.25, -0.25) * t * scale) + sample(uv + vec2<f32>(0.25, -0.25) * t * scale)
            + sample(uv + vec2<f32>(-0.25, 0.25) * t * scale) + sample(uv + vec2<f32>(0.25, 0.25) * t * scale)) * 0.25;
    } else {
        c = sample(uv);
    }
    if (res.clear.y > 0.0) {
        // Sharpening: away from the four neighbours (a rendered pixel
        // away), kept within their range so edges do not ring.
        let t = 1.0 / src;
        let n = sample(uv + vec2<f32>(0.0, -t.y));
        let s = sample(uv + vec2<f32>(0.0, t.y));
        let e = sample(uv + vec2<f32>(t.x, 0.0));
        let w = sample(uv + vec2<f32>(-t.x, 0.0));
        let lo = min(min(min(n, s), min(e, w)), c);
        let hi = max(max(max(n, s), max(e, w)), c);
        c = clamp(c + (c - (n + s + e + w) * 0.25) * res.clear.y, lo, hi);
    }
    var out: Out;
    let ipx = vec2<i32>(floor(in.pos.xy * scale));
    let nm = textureLoad(normal_tex, ipx, 0);
    let n = nm.xyz;
    // The game's alpha: its glow passes read it.
    var alpha = res.clear.x;
    if (textureLoad(depth_tex, ipx, 0) < 1.0) {
        alpha = (nm.w - select(0.0, 1.0, nm.w >= 0.75)) * 2.0;
    }
    out.color = vec4<f32>(c, alpha);
    let vn = vec3<f32>(dot(res.view[0].xyz, n), dot(res.view[1].xyz, n), dot(res.view[2].xyz, n));
    out.normal = vec4<f32>(normalize(vn), 1.0);
    out.depth = textureLoad(depth_tex, ipx, 0);
    return out;
}

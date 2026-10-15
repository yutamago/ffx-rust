// Temporal anti-aliasing: the camera is jittered by a sub-pixel offset
// each frame; this pass finds where each pixel was last frame (by its
// depth and last frame's camera), takes the history there, keeps it within
// the colours around the pixel now (so moving things do not smear), and
// blends a little of the new frame in. The history's alpha is each pixel's
// view depth: history from another depth (something that has moved away,
// or that hid this point last frame) is dropped.

struct Taa {
    // Both without the jitter: a still camera finds each pixel where it
    // was, and the history is not resampled.
    inv_view_proj: mat4x4<f32>,
    // Rows of last frame's view-projection.
    prev_view_proj: array<vec4<f32>, 4>,
    // x: weight of the new frame, y: history valid, zw: size
    params: vec4<f32>,
    // x: how far history may stray from the colours now (standard
    // deviations)
    clamp: vec4<f32>,
};

@group(0) @binding(0) var<uniform> taa: Taa;
@group(0) @binding(1) var current_tex: texture_2d<f32>;
@group(0) @binding(2) var history_tex: texture_2d<f32>;
@group(0) @binding(3) var depth_tex: texture_depth_2d;
@group(0) @binding(4) var samp: sampler;
// The G-buffer's motion vectors: rg where the point was last frame minus
// where it is (uv), b its view depth then, a 1 where they were written.
@group(0) @binding(5) var motion_tex: texture_2d<f32>;

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

fn ycocg(c: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        dot(c, vec3<f32>(0.25, 0.5, 0.25)),
        dot(c, vec3<f32>(0.5, 0.0, -0.5)),
        dot(c, vec3<f32>(-0.25, 0.5, -0.25)),
    );
}

fn rgb(c: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(c.x + c.y - c.z, c.x + c.z, c.x - c.y - c.z);
}

// The history with a Catmull-Rom filter (five bilinear taps): sharper than
// bilinear where the camera moves.
fn history(uv: vec2<f32>, size: vec2<f32>) -> vec3<f32> {
    let p = uv * size;
    let t1 = floor(p - 0.5) + 0.5;
    let f = p - t1;
    let w0 = f * (-0.5 + f * (1.0 - 0.5 * f));
    let w1 = 1.0 + f * f * (-2.5 + 1.5 * f);
    let w2 = f * (0.5 + f * (2.0 - 1.5 * f));
    let w3 = f * f * (-0.5 + 0.5 * f);
    let w12 = w1 + w2;
    let o12 = w2 / w12;
    let a = (t1 - 1.0) / size;
    let b = (t1 + 2.0) / size;
    let c = (t1 + o12) / size;
    var sum = vec3<f32>(0.0);
    sum = sum + textureSampleLevel(history_tex, samp, vec2<f32>(c.x, a.y), 0.0).rgb * (w12.x * w0.y);
    sum = sum + textureSampleLevel(history_tex, samp, vec2<f32>(a.x, c.y), 0.0).rgb * (w0.x * w12.y);
    sum = sum + textureSampleLevel(history_tex, samp, c, 0.0).rgb * (w12.x * w12.y);
    sum = sum + textureSampleLevel(history_tex, samp, vec2<f32>(b.x, c.y), 0.0).rgb * (w3.x * w12.y);
    sum = sum + textureSampleLevel(history_tex, samp, vec2<f32>(c.x, b.y), 0.0).rgb * (w12.x * w3.y);
    let weight = w12.x * w0.y + w0.x * w12.y + w12.x * w12.y + w3.x * w12.y + w12.x * w3.y;
    return max(sum / weight, vec3<f32>(0.0));
}

// The view depth (clip w) of the point at `ndc` with depth `z`: the
// inverse view-projection gives the point divided by its w.
fn view_depth(ndc: vec2<f32>, z: f32) -> f32 {
    let h = taa.inv_view_proj * vec4<f32>(ndc, z, 1.0);
    // Kept within half-float range (the history holds it).
    return min(1.0 / max(abs(h.w), 1e-8), 60000.0);
}

// Whether last frame's history near `prev` (uv) saw a point at depth `w`:
// the best match of the four texels around it, within a tenth.
fn same_depth(prev: vec2<f32>, size: vec2<f32>, w: f32) -> bool {
    let base = vec2<i32>(floor(prev * size - 0.5));
    let imax = vec2<i32>(size) - 1;
    var best = 1e9;
    for (var k = 0; k < 4; k = k + 1) {
        let q = clamp(base + vec2<i32>(k & 1, k >> 1u), vec2<i32>(0), imax);
        best = min(best, abs(textureLoad(history_tex, q, 0).a - w));
    }
    return best <= 0.1 * w + 0.05;
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    let size = taa.params.zw;
    let px = vec2<i32>(in.pos.xy);
    let cur = textureLoad(current_tex, px, 0).rgb;
    let uv = in.pos.xy / size;
    let ndc = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    let own = view_depth(ndc, textureLoad(depth_tex, px, 0));
    if (taa.params.y < 0.5) {
        return vec4<f32>(cur, own);
    }
    // The colours around this pixel now; the nearest depth among them
    // (edges keep the foreground's motion).
    var lo = vec3<f32>(1e9);
    var hi = vec3<f32>(-1e9);
    var mean = vec3<f32>(0.0);
    var sq = vec3<f32>(0.0);
    var depth = 1.0;
    var near = px;
    let imax = vec2<i32>(size) - 1;
    for (var y = -1; y <= 1; y = y + 1) {
        for (var x = -1; x <= 1; x = x + 1) {
            let q = clamp(px + vec2<i32>(x, y), vec2<i32>(0), imax);
            let c = ycocg(textureLoad(current_tex, q, 0).rgb);
            lo = min(lo, c);
            hi = max(hi, c);
            mean = mean + c;
            sq = sq + c * c;
            let d = textureLoad(depth_tex, q, 0);
            if (d < depth) {
                depth = d;
                near = q;
            }
        }
    }
    mean = mean / 9.0;
    let sigma = sqrt(max(sq / 9.0 - mean * mean, vec3<f32>(0.0)));
    lo = max(lo, mean - sigma * taa.clamp.x);
    hi = min(hi, mean + sigma * taa.clamp.x);
    // Where this point was last frame, and its depth then: by its motion
    // where the G-buffer drew it (moving things move with it), else by
    // depth and the camera alone.
    var prev: vec2<f32>;
    var then: f32;
    let m = textureLoad(motion_tex, near, 0);
    if (m.a > 0.5) {
        prev = uv + m.xy;
        then = m.z;
    } else {
        let h = taa.inv_view_proj * vec4<f32>(ndc, depth, 1.0);
        let w = vec4<f32>(h.xyz / h.w, 1.0);
        let pc = vec4<f32>(dot(taa.prev_view_proj[0], w), dot(taa.prev_view_proj[1], w), dot(taa.prev_view_proj[2], w), dot(taa.prev_view_proj[3], w));
        prev = vec2<f32>(pc.x / pc.w * 0.5 + 0.5, 0.5 - pc.y / pc.w * 0.5);
        then = pc.w;
    }
    if (any(prev < vec2<f32>(0.0)) || any(prev > vec2<f32>(1.0)) || then <= 0.0 || !same_depth(prev, size, min(then, 60000.0))) {
        return vec4<f32>(cur, own);
    }
    let hist = ycocg(history(prev, size));
    let clamped = clamp(hist, lo, hi);
    // Fast motion: trust the new frame more.
    let motion = length((prev - uv) * size);
    let alpha = clamp(taa.params.x + motion * 0.02, taa.params.x, 0.5);
    let out = mix(rgb(clamped), cur, alpha);
    return vec4<f32>(out, own);
}

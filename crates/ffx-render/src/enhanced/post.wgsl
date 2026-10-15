// Camera effects on the HDR image, before tone mapping: depth of field
// (focus on what the camera looks at, or the centre of the screen),
// motion blur (along the
// G-buffer's motion vectors, the camera's motion elsewhere) and bloom (a
// chain of ever smaller images, added back up).

struct Post {
    inv_view_proj: mat4x4<f32>,
    // Rows of last frame's view-projection (motion where the G-buffer drew
    // nothing).
    prev_view_proj: array<vec4<f32>, 4>,
    // x: blur strength, y: largest blur radius (pixels), z: focus speed
    // (0–1 per frame), w: focus distance (0: at the centre of the screen)
    dof: vec4<f32>,
    // x: shutter (fraction of a frame), y: longest blur (pixels), z: camera
    // motion too (1) or only moving things (0), w: frame number (noise)
    blur: vec4<f32>,
    // x: bloom threshold, y: its soft knee, z: the radius of the upsampling
    // filter (texels)
    bloom: vec4<f32>,
    // The camera's look-at point: xy NDC, z view depth, w 1 when on screen
    // (else the focus is at the centre of the screen).
    focus_at: vec4<f32>,
};

@group(0) @binding(0) var<uniform> post: Post;
@group(0) @binding(1) var src_tex: texture_2d<f32>;
@group(0) @binding(2) var depth_tex: texture_depth_2d;
@group(0) @binding(3) var motion_tex: texture_2d<f32>;
@group(0) @binding(4) var samp: sampler;
// The focus distance in x (the focus pass keeps it).
@group(0) @binding(5) var<storage, read> focus: vec4<f32>;
// Depth of field: per 16x16 tile, the largest blur radius (pixels) of what
// lies nearer than the focus.
@group(0) @binding(6) var tile_tex: texture_2d<f32>;

const TILE: i32 = 16;

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

fn ndc_of(px: vec2<f32>, size: vec2<f32>) -> vec2<f32> {
    let uv = px / size;
    return vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
}

// The view depth (clip w) of the pixel at `px` (centre) with depth `z`.
fn view_depth(px: vec2<f32>, size: vec2<f32>, z: f32) -> f32 {
    let h = post.inv_view_proj * vec4<f32>(ndc_of(px, size), z, 1.0);
    return 1.0 / max(abs(h.w), 1e-8);
}

fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p + post.blur.w * 0.618, vec2<f32>(12.9898, 78.233))) * 43758.547);
}

// ---- Depth of field ----

// How blurred a point at view depth `d` is, 0–1 of the largest radius.
fn coc(d: f32) -> f32 {
    let f = max(focus.x, 0.01);
    return clamp(post.dof.x * abs(1.0 - f / max(d, 0.01)), 0.0, 1.0);
}

// The blur radius in pixels of the pixel at `q`, and its view depth.
fn coc_at(q: vec2<i32>, size: vec2<f32>) -> vec2<f32> {
    let d = view_depth(vec2<f32>(q) + 0.5, size, textureLoad(depth_tex, q, 0));
    return vec2<f32>(coc(d) * post.dof.y, d);
}

// One tile's largest near blur: what is nearer than the focus spreads over
// what is behind it, so the gather must reach that far.
@fragment
fn fs_dof_tiles(in: VsOut) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(depth_tex));
    let imax = vec2<i32>(size) - 1;
    let base = vec2<i32>(in.pos.xy) * TILE;
    var near = 0.0;
    for (var y = 0; y < TILE; y = y + 1) {
        for (var x = 0; x < TILE; x = x + 1) {
            let c = coc_at(min(base + vec2<i32>(x, y), imax), size);
            if (c.y < focus.x) {
                near = max(near, c.x);
            }
        }
    }
    return vec4<f32>(near, 0.0, 0.0, 1.0);
}

// A gather in two layers. The background layer: samples at the pixel's
// depth or behind it, each only as far as its own blur reaches (and no
// farther than this pixel's, so a sharp thing keeps its edges). The
// foreground layer: nearer samples whose blur reaches this pixel, with
// how much of the pixel they cover (their light spread over their own
// circle), so a blurred thing in front fades out over what is behind it
// instead of ending at its outline.
@fragment
fn fs_dof(in: VsOut) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(src_tex));
    let px = vec2<i32>(in.pos.xy);
    let centre = textureLoad(src_tex, px, 0);
    let c0 = coc_at(px, size);
    let r0 = c0.x;
    let d0 = c0.y;
    // The widest near blur around: the tiles within reach.
    let tiles = vec2<i32>(textureDimensions(tile_tex)) - 1;
    let tile = px / TILE;
    let k = i32(ceil(post.dof.y / f32(TILE)));
    var reach = r0;
    for (var y = -k; y <= k; y = y + 1) {
        for (var x = -k; x <= k; x = x + 1) {
            let t = clamp(tile + vec2<i32>(x, y), vec2<i32>(0), tiles);
            reach = max(reach, textureLoad(tile_tex, t, 0).r);
        }
    }
    if (reach < 0.5) {
        return centre;
    }
    let imax = vec2<i32>(size) - 1;
    let spin = hash(in.pos.xy) * 6.2831853;
    var back = centre.rgb;
    var back_w = 1.0;
    var front = vec3<f32>(0.0);
    var front_w = 0.0;
    var cover = 0.0;
    let n = 48;
    // Each sample stands for this much of the disc.
    let share = reach * reach / f32(n);
    for (var i = 0; i < n; i = i + 1) {
        // Golden-angle spiral, evenly over the disc.
        let t = (f32(i) + 0.5) / f32(n);
        let a = f32(i) * 2.3999632 + spin;
        let o = vec2<f32>(cos(a), sin(a)) * sqrt(t) * reach;
        let dist = length(o);
        let q = clamp(vec2<i32>(in.pos.xy + o), vec2<i32>(0), imax);
        let cs = coc_at(q, size);
        let c = textureLoad(src_tex, q, 0).rgb;
        if (cs.y < d0 * 0.97 && cs.x > r0) {
            let w = clamp(cs.x - dist + 1.0, 0.0, 1.0);
            front = front + c * w;
            front_w = front_w + w;
            cover = cover + w * share / max(cs.x * cs.x, 1.0);
        } else {
            let w = clamp(min(cs.x, r0) - dist + 1.0, 0.0, 1.0);
            back = back + c * w;
            back_w = back_w + w;
        }
    }
    let b = back / back_w;
    if (front_w <= 0.0) {
        return vec4<f32>(b, centre.a);
    }
    let alpha = clamp(cover, 0.0, 1.0);
    return vec4<f32>(mix(b, front / front_w, alpha), centre.a);
}

// ---- Motion blur ----

// Where the point at `px` was last frame minus where it is, in pixels:
// the camera's part, and with moving things their own.
fn camera_motion(px: vec2<f32>, size: vec2<f32>, z: f32) -> vec2<f32> {
    let h = post.inv_view_proj * vec4<f32>(ndc_of(px, size), z, 1.0);
    let w = vec4<f32>(h.xyz / h.w, 1.0);
    let pc = vec4<f32>(dot(post.prev_view_proj[0], w), dot(post.prev_view_proj[1], w), dot(post.prev_view_proj[2], w), dot(post.prev_view_proj[3], w));
    if (pc.w <= 0.0) {
        return vec2<f32>(0.0);
    }
    let prev = vec2<f32>(pc.x / pc.w * 0.5 + 0.5, 0.5 - pc.y / pc.w * 0.5) * size;
    return prev - px;
}

fn motion_at(q: vec2<i32>, size: vec2<f32>) -> vec2<f32> {
    let p = vec2<f32>(q) + 0.5;
    let cam = camera_motion(p, size, textureLoad(depth_tex, q, 0));
    let m = textureLoad(motion_tex, q, 0);
    var v = cam;
    if (m.a > 0.5) {
        v = m.xy * size;
    }
    if (post.blur.z < 0.5) {
        v = v - cam;
    }
    return v * post.blur.x;
}

@fragment
fn fs_motion(in: VsOut) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(src_tex));
    let px = vec2<i32>(in.pos.xy);
    let centre = textureLoad(src_tex, px, 0);
    var v = motion_at(px, size);
    let len = length(v);
    if (len < 0.5) {
        return centre;
    }
    v = v * min(1.0, post.blur.y / len);
    let d0 = view_depth(in.pos.xy, size, textureLoad(depth_tex, px, 0));
    let imax = vec2<i32>(size) - 1;
    let jitter = hash(in.pos.xy) - 0.5;
    var sum = centre.rgb;
    var weight = 1.0;
    let n = 12;
    for (var i = 0; i < n; i = i + 1) {
        // Spread over the shutter, centred on now.
        let t = (f32(i) + 0.5 + jitter) / f32(n) - 0.5;
        let o = v * t;
        let q = clamp(vec2<i32>(in.pos.xy + o), vec2<i32>(0), imax);
        // Something still in front does not smear over what moves behind it.
        let d = view_depth(vec2<f32>(q) + 0.5, size, textureLoad(depth_tex, q, 0));
        var w = 1.0;
        if (d < d0 * 0.95) {
            w = clamp(length(motion_at(q, size)) - length(o) + 1.0, 0.0, 1.0);
        }
        sum = sum + textureLoad(src_tex, q, 0).rgb * w;
        weight = weight + w;
    }
    return vec4<f32>(sum / weight, centre.a);
}

// ---- Bloom ----

fn bright(c: vec3<f32>) -> vec3<f32> {
    // A soft threshold: below it nothing, above it everything, a knee in
    // between.
    let l = max(c.r, max(c.g, c.b));
    let k = max(post.bloom.y, 1e-4);
    let soft = clamp(l - post.bloom.x + k, 0.0, 2.0 * k);
    let w = max(soft * soft / (4.0 * k), l - post.bloom.x) / max(l, 1e-4);
    return c * w;
}

// 13 taps over 4x4 source texels (one destination texel at half size).
fn down13(uv: vec2<f32>, t: vec2<f32>) -> vec3<f32> {
    let a = textureSampleLevel(src_tex, samp, uv + t * vec2<f32>(-2.0, -2.0), 0.0).rgb;
    let b = textureSampleLevel(src_tex, samp, uv + t * vec2<f32>(0.0, -2.0), 0.0).rgb;
    let c = textureSampleLevel(src_tex, samp, uv + t * vec2<f32>(2.0, -2.0), 0.0).rgb;
    let d = textureSampleLevel(src_tex, samp, uv + t * vec2<f32>(-2.0, 0.0), 0.0).rgb;
    let e = textureSampleLevel(src_tex, samp, uv, 0.0).rgb;
    let f = textureSampleLevel(src_tex, samp, uv + t * vec2<f32>(2.0, 0.0), 0.0).rgb;
    let g = textureSampleLevel(src_tex, samp, uv + t * vec2<f32>(-2.0, 2.0), 0.0).rgb;
    let h = textureSampleLevel(src_tex, samp, uv + t * vec2<f32>(0.0, 2.0), 0.0).rgb;
    let i = textureSampleLevel(src_tex, samp, uv + t * vec2<f32>(2.0, 2.0), 0.0).rgb;
    let j = textureSampleLevel(src_tex, samp, uv + t * vec2<f32>(-1.0, -1.0), 0.0).rgb;
    let k = textureSampleLevel(src_tex, samp, uv + t * vec2<f32>(1.0, -1.0), 0.0).rgb;
    let l = textureSampleLevel(src_tex, samp, uv + t * vec2<f32>(-1.0, 1.0), 0.0).rgb;
    let m = textureSampleLevel(src_tex, samp, uv + t * vec2<f32>(1.0, 1.0), 0.0).rgb;
    return e * 0.125 + (a + c + g + i) * 0.03125 + (b + d + f + h) * 0.0625 + (j + k + l + m) * 0.125;
}

// The destination's size: each level is half the one before, rounded down.
fn half_size() -> vec2<f32> {
    return vec2<f32>(max(textureDimensions(src_tex) / 2u, vec2<u32>(1u)));
}

@fragment
fn fs_down_first(in: VsOut) -> @location(0) vec4<f32> {
    let dst = half_size();
    let t = 1.0 / vec2<f32>(textureDimensions(src_tex));
    // Fireflies (single very bright pixels) would flicker: tamed first.
    let c = min(down13(in.pos.xy / dst, t), vec3<f32>(64.0));
    return vec4<f32>(bright(c), 1.0);
}

@fragment
fn fs_down(in: VsOut) -> @location(0) vec4<f32> {
    let t = 1.0 / vec2<f32>(textureDimensions(src_tex));
    return vec4<f32>(down13(in.pos.xy / half_size(), t), 1.0);
}

// A 3x3 tent over the smaller level, added onto this one (twice its size,
// give or take a texel).
@fragment
fn fs_up(in: VsOut) -> @location(0) vec4<f32> {
    let src = vec2<f32>(textureDimensions(src_tex));
    let uv = in.pos.xy / (src * 2.0);
    let t = post.bloom.z / src;
    var c = textureSampleLevel(src_tex, samp, uv, 0.0).rgb * 4.0;
    c = c + (textureSampleLevel(src_tex, samp, uv + vec2<f32>(-t.x, 0.0), 0.0).rgb
        + textureSampleLevel(src_tex, samp, uv + vec2<f32>(t.x, 0.0), 0.0).rgb
        + textureSampleLevel(src_tex, samp, uv + vec2<f32>(0.0, -t.y), 0.0).rgb
        + textureSampleLevel(src_tex, samp, uv + vec2<f32>(0.0, t.y), 0.0).rgb) * 2.0;
    c = c + textureSampleLevel(src_tex, samp, uv + vec2<f32>(-t.x, -t.y), 0.0).rgb
        + textureSampleLevel(src_tex, samp, uv + vec2<f32>(t.x, -t.y), 0.0).rgb
        + textureSampleLevel(src_tex, samp, uv + vec2<f32>(-t.x, t.y), 0.0).rgb
        + textureSampleLevel(src_tex, samp, uv + vec2<f32>(t.x, t.y), 0.0).rgb;
    return vec4<f32>(c / 16.0, 1.0);
}

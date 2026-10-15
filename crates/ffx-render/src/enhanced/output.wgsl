// Tone mapping: HDR to 8-bit display colour with the chosen curve and
// grade, at the render scale (resolve.wgsl brings it to the game).

struct Post {
    // x: exposure (multiplier), y: tone curve (0 game, 1 ACES, 2 AgX-like),
    // z: saturation, w: contrast
    tone: vec4<f32>,
    // Rows of the view matrix, for the game's view-space normals.
    view: array<vec4<f32>, 4>,
    // x: bloom strength
    bloom: vec4<f32>,
};

@group(0) @binding(0) var<uniform> post: Post;
@group(0) @binding(1) var hdr_tex: texture_2d<f32>;
@group(0) @binding(2) var normal_tex: texture_2d<f32>;
@group(0) @binding(3) var depth_tex: texture_depth_2d;
@group(0) @binding(4) var bloom_tex: texture_2d<f32>;
@group(0) @binding(5) var bloom_samp: sampler;

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

// The game's own curve: its pixel shaders end with this on values where
// 0.5 is white, in gamma space.
fn game_curve(c: vec3<f32>) -> vec3<f32> {
    let lo = min(c, vec3<f32>(0.4));
    let mid = min(max(c, vec3<f32>(0.4)), vec3<f32>(0.58));
    return clamp(lo * 2.1 + mid * 1.08 - 0.47, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn aces(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

// A smooth filmic curve in the spirit of AgX: log encoding and a sigmoid,
// gentle on saturated highlights.
fn agx_like(x: vec3<f32>) -> vec3<f32> {
    let lmin = -10.0;
    let lmax = 6.5;
    let l = clamp((log2(max(x, vec3<f32>(1e-10))) - lmin) / (lmax - lmin), vec3<f32>(0.0), vec3<f32>(1.0));
    let l2 = l * l;
    let l4 = l2 * l2;
    let s = 15.5 * l4 * l2 - 40.14 * l4 * l + 31.96 * l4 - 6.868 * l2 * l + 0.4298 * l2 + 0.1191 * l - 0.00232;
    return clamp(s, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn to_display(x: vec3<f32>) -> vec3<f32> {
    return pow(max(x, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.2));
}

struct Out {
    @location(0) color: vec4<f32>,
};

@fragment
fn fs(in: VsOut) -> Out {
    let px = vec2<i32>(in.pos.xy);
    let size = vec2<f32>(textureDimensions(hdr_tex));
    var hdr = textureLoad(hdr_tex, px, 0).rgb;
    hdr = hdr + textureSampleLevel(bloom_tex, bloom_samp, in.pos.xy / size, 0.0).rgb * post.bloom.x;
    let x = hdr * post.tone.x;
    var c: vec3<f32>;
    if (post.tone.y < 0.5) {
        c = game_curve(0.5 * to_display(x));
    } else if (post.tone.y < 1.5) {
        c = to_display(aces(x));
    } else {
        c = agx_like(x);
    }
    // Grade in display space.
    let luma = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
    c = mix(vec3<f32>(luma), c, post.tone.z);
    c = clamp((c - 0.5) * post.tone.w + 0.5, vec3<f32>(0.0), vec3<f32>(1.0));
    var out: Out;
    out.color = vec4<f32>(c, 1.0);
    return out;
}

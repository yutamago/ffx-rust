// Screen-space ambient occlusion: samples a hemisphere around each
// pixel's normal against the depth buffer, then a depth-aware blur.

struct Ao {
    inv_view_proj: mat4x4<f32>,
    // Rows of the camera's view-projection.
    view_proj: array<vec4<f32>, 4>,
    // x: radius (world units), y: strength, z: width, w: height
    params: vec4<f32>,
    // x: blur direction x, y: direction y
    blur: vec4<f32>,
};

@group(0) @binding(0) var<uniform> ao: Ao;
@group(0) @binding(1) var depth_tex: texture_depth_2d;
@group(0) @binding(2) var normal_tex: texture_2d<f32>;
@group(0) @binding(3) var src_tex: texture_2d<f32>;

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

fn world(px: vec2<f32>, depth: f32) -> vec3<f32> {
    let ndc = vec2<f32>(px.x / ao.params.z * 2.0 - 1.0, 1.0 - px.y / ao.params.w * 2.0);
    let h = ao.inv_view_proj * vec4<f32>(ndc, depth, 1.0);
    return h.xyz / h.w;
}

fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    let px = vec2<i32>(in.pos.xy);
    let depth = textureLoad(depth_tex, px, 0);
    if (depth >= 1.0) {
        return vec4<f32>(1.0);
    }
    let p = world(in.pos.xy, depth);
    let n = normalize(textureLoad(normal_tex, px, 0).xyz);
    // A tangent frame, rotated per pixel.
    let a = hash(in.pos.xy) * 6.2831;
    var t = normalize(cross(n, select(vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(1.0, 0.0, 0.0), abs(n.y) > 0.9)));
    t = t * cos(a) + cross(n, t) * sin(a);
    let b = cross(n, t);
    let radius = ao.params.x;
    var occlusion = 0.0;
    let count = 12;
    for (var i = 0; i < count; i = i + 1) {
        // Points in the hemisphere, denser near the centre.
        let fi = f32(i);
        let r = (fi + 0.5) / f32(count);
        let phi = fi * 2.39996;
        let h = sqrt(1.0 - r);
        let dir = t * cos(phi) * sqrt(r) + b * sin(phi) * sqrt(r) + n * h;
        let q = p + dir * radius * mix(0.2, 1.0, r * r);
        let c = vec4<f32>(dot(ao.view_proj[0], vec4<f32>(q, 1.0)), dot(ao.view_proj[1], vec4<f32>(q, 1.0)), dot(ao.view_proj[2], vec4<f32>(q, 1.0)), dot(ao.view_proj[3], vec4<f32>(q, 1.0)));
        let s = c.xyz / c.w;
        let uv = vec2<f32>(s.x * 0.5 + 0.5, 0.5 - s.y * 0.5);
        if (any(uv < vec2<f32>(0.0)) || any(uv >= vec2<f32>(1.0))) {
            continue;
        }
        let spx = vec2<i32>(uv * ao.params.zw);
        let sd = textureLoad(depth_tex, spx, 0);
        if (sd < s.z) {
            // Something in front of the sample point: occluded, fading
            // with distance so far-off geometry does not darken.
            let sp = world(vec2<f32>(spx) + 0.5, sd);
            let range = smoothstep(0.0, 1.0, radius / max(length(sp - p), 1e-3));
            occlusion = occlusion + range;
        }
    }
    let v = 1.0 - occlusion / f32(count) * ao.params.y;
    return vec4<f32>(clamp(v, 0.0, 1.0), 0.0, 0.0, 1.0);
}

@fragment
fn fs_blur(in: VsOut) -> @location(0) vec4<f32> {
    let px = vec2<i32>(in.pos.xy);
    let size = vec2<i32>(textureDimensions(src_tex));
    let d0 = textureLoad(depth_tex, px, 0);
    let dir = vec2<i32>(ao.blur.xy);
    var sum = 0.0;
    var weight = 0.0;
    for (var i = -4; i <= 4; i = i + 1) {
        let q = clamp(px + dir * i, vec2<i32>(0), size - 1);
        let d = textureLoad(depth_tex, q, 0);
        let w = exp(-f32(i * i) / 8.0) * exp(-abs(d - d0) * 2000.0);
        sum = sum + textureLoad(src_tex, q, 0).r * w;
        weight = weight + w;
    }
    return vec4<f32>(sum / max(weight, 1e-4), 0.0, 0.0, 1.0);
}

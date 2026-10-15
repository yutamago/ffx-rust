// Volumetric light: marches each view ray through the key light's
// shadow cascades and gathers light scattered toward the eye where the
// air is lit (light shafts), plus a uniform haze. Half resolution; the
// tone pass composites it as colour = colour × transmittance + scatter.

struct Vol {
    inv_view_proj: mat4x4<f32>,
    shadow: array<vec4<f32>, 12>,
    eye: vec4<f32>,
    // xyz: direction the light travels, w: intensity
    sun: vec4<f32>,
    sun_color: vec4<f32>,
    // x: density, y: anisotropy (g), z: steps, w: max distance
    params: vec4<f32>,
    // x: width, y: height (of this pass), z: haze, w: frame
    screen: vec4<f32>,
    // The haze's colour (the sky light).
    haze_color: vec4<f32>,
    view_z: vec4<f32>,
    splits: vec4<f32>,
};

@group(0) @binding(0) var<uniform> vol: Vol;
@group(0) @binding(1) var depth_tex: texture_depth_2d;
@group(0) @binding(2) var shadow_tex: texture_depth_2d_array;
@group(0) @binding(3) var shadow_samp: sampler_comparison;

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

fn lit(p: vec3<f32>) -> f32 {
    let d = abs(dot(vol.view_z, vec4<f32>(p, 1.0)));
    var c = 0;
    if (d > vol.splits.x) {
        c = 1;
    }
    if (d > vol.splits.y) {
        c = 2;
    }
    if (d > vol.splits.z) {
        return 1.0;
    }
    let wp = vec4<f32>(p, 1.0);
    let h = vec4<f32>(dot(vol.shadow[c * 4], wp), dot(vol.shadow[c * 4 + 1], wp), dot(vol.shadow[c * 4 + 2], wp), dot(vol.shadow[c * 4 + 3], wp));
    let s = h.xyz / h.w;
    let uv = vec2<f32>(s.x * 0.5 + 0.5, 0.5 - s.y * 0.5);
    if (any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) || s.z > 1.0) {
        return 1.0;
    }
    return textureSampleCompareLevel(shadow_tex, shadow_samp, uv, c, s.z);
}

fn phase(cos_theta: f32, g: f32) -> f32 {
    let g2 = g * g;
    return (1.0 - g2) / (4.0 * 3.14159 * pow(1.0 + g2 - 2.0 * g * cos_theta, 1.5));
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    let full = vec2<f32>(textureDimensions(depth_tex));
    let uv = in.pos.xy / vol.screen.xy;
    let dpx = vec2<i32>(uv * full);
    let depth = textureLoad(depth_tex, clamp(dpx, vec2<i32>(0), vec2<i32>(full) - 1), 0);
    let ndc = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    let h = vol.inv_view_proj * vec4<f32>(ndc, depth, 1.0);
    let end = h.xyz / h.w;
    let start = vol.eye.xyz;
    var ray = end - start;
    var len = length(ray);
    let dir = ray / max(len, 1e-4);
    len = min(len, vol.params.w);
    let steps = i32(vol.params.z);
    let step_len = len / f32(steps);
    // Jitter the start per pixel and frame to trade banding for noise.
    let jitter = fract(sin(dot(in.pos.xy + vol.screen.w * 0.618, vec2<f32>(12.9898, 78.233))) * 43758.5453);
    let l = -normalize(vol.sun.xyz);
    let ph = phase(dot(dir, l), vol.params.y);
    let sigma = vol.params.x;
    var transmittance = 1.0;
    var scatter = vec3<f32>(0.0);
    for (var i = 0; i < steps; i = i + 1) {
        let t = (f32(i) + jitter) * step_len;
        let p = start + dir * t;
        let light = lit(p);
        let seg = exp(-sigma * step_len);
        // In-scattered light over this segment: key light where lit,
        // plus the haze's own share.
        let s = (vol.sun_color.rgb * vol.sun.w * ph * light + vol.haze_color.rgb * vol.screen.z) * (1.0 - seg);
        scatter = scatter + s * transmittance;
        transmittance = transmittance * seg;
    }
    return vec4<f32>(scatter, transmittance);
}

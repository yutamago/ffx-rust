// Material maps made from a colour texture, for textures that have none:
// luminance as a height field gives a tangent-space normal (Sobel), local
// detail gives roughness. Output: rg normal x, y (0.5 + n/2), b
// roughness, a cavity (darker in crevices).

struct Gen {
    // x: normal strength, y: base roughness, z: roughness from detail,
    // w: texel size of the source level used
    params: vec4<f32>,
    // x: source mip level
    level: vec4<f32>,
};

@group(0) @binding(0) var<uniform> gen: Gen;
@group(0) @binding(1) var src: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) id: u32) -> VsOut {
    let uv = vec2<f32>(f32((id << 1u) & 2u), f32(id & 2u));
    var out: VsOut;
    out.pos = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    out.uv = uv;
    return out;
}

fn height(uv: vec2<f32>) -> f32 {
    let c = textureSampleLevel(src, samp, uv, gen.level.x).rgb;
    return dot(c, vec3<f32>(0.299, 0.587, 0.114));
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    let t = vec2<f32>(textureDimensions(src, u32(gen.level.x)));
    let d = 1.0 / t;
    let uv = in.uv;
    let tl = height(uv + vec2<f32>(-d.x, -d.y));
    let tc = height(uv + vec2<f32>(0.0, -d.y));
    let tr = height(uv + vec2<f32>(d.x, -d.y));
    let ml = height(uv + vec2<f32>(-d.x, 0.0));
    let mc = height(uv);
    let mr = height(uv + vec2<f32>(d.x, 0.0));
    let bl = height(uv + vec2<f32>(-d.x, d.y));
    let bc = height(uv + vec2<f32>(0.0, d.y));
    let br = height(uv + vec2<f32>(d.x, d.y));
    let gx = (tr + 2.0 * mr + br) - (tl + 2.0 * ml + bl);
    let gy = (bl + 2.0 * bc + br) - (tl + 2.0 * tc + tr);
    let n = normalize(vec3<f32>(-gx * gen.params.x, -gy * gen.params.x, 1.0));
    // Detail: how much the pixel differs from its neighbours.
    let mean = (tl + tc + tr + ml + mr + bl + bc + br) / 8.0;
    let detail = clamp(abs(mc - mean) * 8.0, 0.0, 1.0);
    let rough = clamp(gen.params.y + detail * gen.params.z, 0.05, 1.0);
    let cavity = clamp(1.0 - max(mean - mc, 0.0) * 4.0, 0.0, 1.0);
    return vec4<f32>(n.xy * 0.5 + 0.5, rough, cavity);
}

// Box filter into the next mip level.
@fragment
fn fs_down(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureSampleLevel(src, samp, in.uv, gen.level.x);
    let n = normalize(vec3<f32>(c.xy * 2.0 - 1.0, 1.0));
    return vec4<f32>(n.xy * 0.5 + 0.5, c.z, c.w);
}

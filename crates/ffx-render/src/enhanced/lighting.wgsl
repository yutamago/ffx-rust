// Lighting pass of the enhanced renderer: physically based shading of
// the G-buffer in linear HDR. The game's baked vertex light stays as the
// indirect term (its art direction); a key light with soft shadows and
// sky light are added; the game's fog applies on top.

struct Light {
    inv_view_proj: mat4x4<f32>,
    // Rows of the shadow map's view-projection.
    shadow: array<vec4<f32>, 4>,
    eye: vec4<f32>,
    // xyz: direction the light travels, w: intensity
    sun: vec4<f32>,
    sun_color: vec4<f32>,
    // rgb: sky ambient, w: strength
    sky: vec4<f32>,
    ground: vec4<f32>,
    // rgb: fog colour (linear), w: limit
    fog: vec4<f32>,
    // x: near, y: far, z: curve, w: on
    fog_range: vec4<f32>,
    // x: baked light weight, y: roughness, z: specular, w: shadow on
    mix: vec4<f32>,
    // x: shadow map texel size, y: filter radius (texels), z: bias, w: ao strength
    shadow_params: vec4<f32>,
    // x: width, y: height
    screen: vec4<f32>,
    // The view matrix's third row: view-space depth, as the game's fog
    // uses.
    view_z: vec4<f32>,
    // xyz: direction the fill light travels, w: intensity; colour
    fill: vec4<f32>,
    fill_color: vec4<f32>,
};

@group(0) @binding(0) var<uniform> light: Light;
@group(0) @binding(1) var albedo_tex: texture_2d<f32>;
@group(0) @binding(2) var normal_tex: texture_2d<f32>;
@group(0) @binding(3) var baked_tex: texture_2d<f32>;
@group(0) @binding(4) var depth_tex: texture_depth_2d;
@group(0) @binding(5) var shadow_tex: texture_depth_2d;
@group(0) @binding(6) var shadow_samp: sampler_comparison;
@group(0) @binding(7) var ao_tex: texture_2d<f32>;

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

const PI: f32 = 3.14159265;

fn world_position(px: vec2<f32>, depth: f32) -> vec3<f32> {
    let ndc = vec2<f32>(px.x / light.screen.x * 2.0 - 1.0, 1.0 - px.y / light.screen.y * 2.0);
    let h = light.inv_view_proj * vec4<f32>(ndc, depth, 1.0);
    return h.xyz / h.w;
}

fn shadow_factor(p: vec3<f32>, n: vec3<f32>) -> f32 {
    if (light.mix.w < 0.5) {
        return 1.0;
    }
    let wp = vec4<f32>(p + n * light.shadow_params.z, 1.0);
    let c = vec4<f32>(dot(light.shadow[0], wp), dot(light.shadow[1], wp), dot(light.shadow[2], wp), dot(light.shadow[3], wp));
    let s = c.xyz / c.w;
    let uv = vec2<f32>(s.x * 0.5 + 0.5, 0.5 - s.y * 0.5);
    if (any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) || s.z > 1.0) {
        return 1.0;
    }
    // Poisson-ish 12-tap filter, rotated per pixel to hide banding.
    let r = light.shadow_params.x * light.shadow_params.y;
    var taps = array<vec2<f32>, 12>(
        vec2<f32>(-0.326, -0.406), vec2<f32>(-0.840, -0.074), vec2<f32>(-0.696, 0.457),
        vec2<f32>(-0.203, 0.621), vec2<f32>(0.962, -0.195), vec2<f32>(0.473, -0.480),
        vec2<f32>(0.519, 0.767), vec2<f32>(0.185, -0.893), vec2<f32>(0.507, 0.064),
        vec2<f32>(0.896, 0.412), vec2<f32>(-0.322, -0.933), vec2<f32>(-0.792, -0.598));
    let a = fract(sin(dot(p.xz, vec2<f32>(12.9898, 78.233))) * 43758.5453) * 6.2831;
    let rot = mat2x2<f32>(cos(a), sin(a), -sin(a), cos(a));
    var sum = 0.0;
    for (var i = 0; i < 12; i = i + 1) {
        sum = sum + textureSampleCompareLevel(shadow_tex, shadow_samp, uv + rot * taps[i] * r, s.z);
    }
    return sum / 12.0;
}

fn ggx(n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, rough: f32, f0: f32) -> f32 {
    let h = normalize(v + l);
    let nh = max(dot(n, h), 0.0);
    let nv = max(dot(n, v), 1e-3);
    let nl = max(dot(n, l), 1e-3);
    let a = rough * rough;
    let a2 = a * a;
    let d = a2 / (PI * pow(nh * nh * (a2 - 1.0) + 1.0, 2.0));
    let k = (rough + 1.0) * (rough + 1.0) / 8.0;
    let g = (nv / (nv * (1.0 - k) + k)) * (nl / (nl * (1.0 - k) + k));
    let f = f0 + (1.0 - f0) * pow(1.0 - max(dot(h, v), 0.0), 5.0);
    return d * g * f / (4.0 * nv * nl);
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    let px = vec2<i32>(in.pos.xy);
    let depth = textureLoad(depth_tex, px, 0);
    if (depth >= 1.0) {
        return vec4<f32>(0.0);
    }
    let albedo = textureLoad(albedo_tex, px, 0).rgb;
    let nm = textureLoad(normal_tex, px, 0);
    let baked = textureLoad(baked_tex, px, 0).rgb;
    let ao = mix(1.0, textureLoad(ao_tex, px, 0).r, light.shadow_params.w);
    let n = normalize(nm.xyz);
    let p = world_position(in.pos.xy, depth);
    let v = normalize(light.eye.xyz - p);
    let l = -normalize(light.sun.xyz);
    let nl = max(dot(n, l), 0.0);
    let shadow = shadow_factor(p, n);
    let rough = light.mix.y;
    let direct = light.sun_color.rgb * light.sun.w * nl * shadow;
    let spec = ggx(n, v, l, rough, 0.04) * light.mix.z;
    let lf = -normalize(light.fill.xyz);
    let fill = light.fill_color.rgb * light.fill.w * max(dot(n, lf), 0.0);
    let hemi = mix(light.ground.rgb, light.sky.rgb, n.y * 0.5 + 0.5) * light.sky.w;
    let indirect = baked * light.mix.x + hemi;
    var color = albedo * (indirect * ao + direct + fill) + direct * spec;
    // The game's fog, by distance from the eye.
    if (light.fog_range.w > 0.5) {
        let d = abs(dot(light.view_z, vec4<f32>(p, 1.0)));
        let t = clamp((d - light.fog_range.x) / (light.fog_range.y - light.fog_range.x + 0.01), 0.0, 1.0);
        let f = min(pow(t, light.fog_range.z), light.fog.w);
        color = mix(color, light.fog.rgb, f);
    }
    return vec4<f32>(color, 1.0);
}

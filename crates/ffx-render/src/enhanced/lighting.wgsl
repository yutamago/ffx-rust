// Lighting pass of the enhanced renderer: physically based shading of
// the G-buffer in linear HDR. The game's baked vertex light stays as the
// indirect term (its art direction); a key light with soft shadows and
// sky light are added; the game's fog applies on top.

struct Light {
    inv_view_proj: mat4x4<f32>,
    // Rows of the three shadow cascades' view-projections.
    shadow: array<vec4<f32>, 12>,
    eye: vec4<f32>,
    // xyz: direction the light travels, w: intensity
    sun: vec4<f32>,
    sun_color: vec4<f32>,
    // rgb: sky ambient, w: strength
    sky: vec4<f32>,
    // rgb: ground colour, w: how much sky light scenery gets on top of its
    // baked light (which has the map's ambient already)
    ground: vec4<f32>,
    // rgb: fog colour (linear), w: limit
    fog: vec4<f32>,
    // x: near, y: far, z: curve, w: on
    fog_range: vec4<f32>,
    // x: baked light weight, y: roughness, z: specular, w: shadow on
    mix: vec4<f32>,
    // x: shadow map texel size, y: filter radius (texels), z: bias, w: ao strength
    shadow_params: vec4<f32>,
    // x: width, y: height, z: strength of the shadows only the cascades
    // have (characters') on scenery, when rays are on
    screen: vec4<f32>,
    // The view matrix's third row: view-space depth, as the game's fog
    // uses.
    view_z: vec4<f32>,
    // xyz: direction the fill light travels, w: intensity; colour
    fill: vec4<f32>,
    fill_color: vec4<f32>,
    // x: how much the key light reshapes scenery's baked light (0: not
    // at all, 1: fully by its shadow and facing), y: scenery specular,
    // z: ambient occlusion on baked light, w: characters' falloff (0: the
    // game's soft wrap, 1: physical)
    scenery: vec4<f32>,
    // View distances where cascades 0, 1 and 2 end; w: debug view (0
    // none, 1 shadow, 2 occlusion, 3 normal, 4 albedo, 5 baked light,
    // 6 cascades, 7 skin, 8 probe light)
    splits: vec4<f32>,
    // x: probe light strength, y: how much it replaces the characters'
    // ambient, z: skin scattering, w: probes on
    gi: vec4<f32>,
    // x: ray-traced shadows on, y: the key light's angular radius,
    // z: frame (for the noise), w: how far rays look for occluders
    rt: vec4<f32>,
};

struct Probes {
    count: vec4<f32>,
    pos: array<vec4<f32>, 2>,
    sh: array<vec4<f32>, 18>,
};

@group(0) @binding(0) var<uniform> light: Light;
@group(0) @binding(1) var albedo_tex: texture_2d<f32>;
@group(0) @binding(2) var normal_tex: texture_2d<f32>;
@group(0) @binding(3) var baked_tex: texture_2d<f32>;
@group(0) @binding(4) var depth_tex: texture_depth_2d;
@group(0) @binding(5) var shadow_tex: texture_depth_2d_array;
@group(0) @binding(6) var shadow_samp: sampler_comparison;
@group(0) @binding(7) var ao_tex: texture_2d<f32>;
@group(0) @binding(8) var<storage, read> probes: Probes;

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
    let d = abs(dot(light.view_z, vec4<f32>(p, 1.0)));
    var c = 0;
    if (d > light.splits.x) {
        c = 1;
    }
    if (d > light.splits.y) {
        c = 2;
    }
    if (d > light.splits.z) {
        return 1.0;
    }
    // Normal offset grows with the cascade's texel size.
    let wp = vec4<f32>(p + n * light.shadow_params.z * f32(1 << u32(c * 2)), 1.0);
    let r0 = light.shadow[c * 4];
    let r1 = light.shadow[c * 4 + 1];
    let r2 = light.shadow[c * 4 + 2];
    let r3 = light.shadow[c * 4 + 3];
    let h = vec4<f32>(dot(r0, wp), dot(r1, wp), dot(r2, wp), dot(r3, wp));
    let s = h.xyz / h.w;
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
        sum = sum + textureSampleCompareLevel(shadow_tex, shadow_samp, uv + rot * taps[i] * r, c, s.z);
    }
    var f = sum / 12.0;
    // Fade out the last cascade's far edge.
    if (c == 2) {
        f = mix(f, 1.0, smoothstep(light.splits.z * 0.85, light.splits.z, d));
    }
    return f;
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

// Irradiance / π from the nearest probe's spherical harmonics.
fn probe_light(p: vec3<f32>, n: vec3<f32>) -> vec3<f32> {
    var best = 0u;
    var best_d = 1e30;
    let count = u32(probes.count.x);
    for (var i = 0u; i < count; i = i + 1u) {
        let d = distance(probes.pos[i].xyz, p);
        if (d < best_d) {
            best_d = d;
            best = i;
        }
    }
    let b = best * 9u;
    // Cosine-lobe convolution (π, 2π/3, π/4), divided by π.
    let c1 = 2.0 / 3.0;
    let c2 = 0.25;
    var e = probes.sh[b].rgb * 0.282095;
    e = e + c1 * 0.488603 * (probes.sh[b + 1u].rgb * n.y + probes.sh[b + 2u].rgb * n.z + probes.sh[b + 3u].rgb * n.x);
    e = e + c2 * (1.092548 * (probes.sh[b + 4u].rgb * n.x * n.y + probes.sh[b + 5u].rgb * n.y * n.z + probes.sh[b + 7u].rgb * n.x * n.z)
        + 0.315392 * probes.sh[b + 6u].rgb * (3.0 * n.z * n.z - 1.0)
        + 0.546274 * probes.sh[b + 8u].rgb * (n.x * n.x - n.y * n.y));
    return max(e, vec3<f32>(0.0));
}

// How much a colour looks like skin: red over green over blue, moderately
// saturated, not too dark, hue between red and orange.
fn skin_weight(albedo: vec3<f32>) -> f32 {
    let c = sqrt(max(albedo, vec3<f32>(0.0)));
    let hi = max(c.r, max(c.g, c.b));
    let lo = min(c.r, min(c.g, c.b));
    let sat = (hi - lo) / max(hi, 1e-3);
    if (c.r < c.g || c.g < c.b) {
        return 0.0;
    }
    let hue = 60.0 * (c.g - c.b) / max(hi - lo, 1e-3);
    return smoothstep(0.12, 0.22, sat) * (1.0 - smoothstep(0.6, 0.75, sat)) * smoothstep(0.15, 0.3, hi)
        * smoothstep(5.0, 12.0, hue) * (1.0 - smoothstep(30.0, 38.0, hue));
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    let px = vec2<i32>(in.pos.xy);
    let depth = textureLoad(depth_tex, px, 0);
    if (depth >= 1.0) {
        return vec4<f32>(0.0);
    }
    let albedo_rough = textureLoad(albedo_tex, px, 0);
    let albedo = albedo_rough.rgb;
    let nm = textureLoad(normal_tex, px, 0);
    let baked_fog = textureLoad(baked_tex, px, 0);
    let baked = baked_fog.rgb;
    let ao = mix(1.0, textureLoad(ao_tex, px, 0).r, light.shadow_params.w);
    let n = normalize(nm.xyz);
    // The material is the whole part of w (half the game's alpha the rest).
    let character = nm.w >= 0.75;
    let p = world_position(in.pos.xy, depth);
    let v = normalize(light.eye.xyz - p);
    let l = -normalize(light.sun.xyz);
    let nl = max(dot(n, l), 0.0);
    // With rays, the cascades hold only what rays do not see (characters,
    // cut-out surfaces): two shadows, of different strength on scenery.
    let cascade = shadow_factor(p, n);
    let traced_on = light.rt.x > 0.5 && light.mix.w > 0.5;
    var traced = 1.0;
    // No ray where the cascades already have shadow, nor for scenery
    // facing away from the key light (characters' soft wrap lights past
    // the terminator).
    if (traced_on && cascade > 0.0 && (character || nl > 0.0)) {
        traced = rt_shadow(p, n, l, in.pos.xy, light.rt.y, light.rt.z);
    }
    let shadow = min(cascade, traced);
    let rough = select(light.mix.y, albedo_rough.a, albedo_rough.a > 0.01);
    let spec = ggx(n, v, l, rough, 0.04) * light.mix.z;
    let hemi = mix(light.ground.rgb, light.sky.rgb, n.y * 0.5 + 0.5) * light.sky.w;
    var color: vec3<f32>;
    if (!character) {
        // Scenery: its baked light is the lighting (the artists' light,
        // local lights included). The key light only adds its shadows:
        // darker where it is blocked, on the sides that face it (the far
        // sides are dark in the baked light already). Its direction need
        // not match the baked light's, so it never brightens.
        // Shadows of characters (and what else only the cascades have)
        // ground them: stronger than the scenery's own.
        let facing = smoothstep(0.0, 0.2, nl);
        var shape = mix(1.0, mix(1.0, shadow, facing), light.scenery.x);
        if (traced_on) {
            shape = mix(1.0, mix(1.0, traced, facing), light.scenery.x)
                * mix(1.0, mix(1.0, cascade, facing), light.screen.z);
        }
        let base = baked * light.mix.x * max(shape, 0.0) * mix(1.0, ao, light.scenery.z);
        let gloss = spec * light.sun_color.rgb * light.sun.w * shadow * nl * light.scenery.y;
        color = albedo * (base + hemi * ao * light.ground.w) + gloss * min(dot(baked, vec3<f32>(0.33)) * 2.0, 1.0);
    } else {
        // Characters: lit like the game lights them, key and fill light
        // with real shadows, ambient as sky.
        // The game's soft wrap ((n·l)/2 + 1/2)², with real shadow.
        let wrap = pow(dot(n, l) * 0.5 + 0.5, 2.0);
        // Skin scatters red light past the terminator and into the edge
        // of shadows: per channel, the falloff and shadow soften.
        let skin = skin_weight(albedo) * light.gi.z;
        let soften = vec3<f32>(1.0) - skin * vec3<f32>(0.55, 0.15, 0.0);
        let shape = pow(vec3<f32>(max(mix(wrap, nl, light.scenery.w), 0.0)), soften);
        let shadowed = pow(vec3<f32>(mix(1.0, shadow, 0.85)), soften);
        let direct = light.sun_color.rgb * light.sun.w * shape * shadowed;
        let lf = -normalize(light.fill.xyz);
        let fill = light.fill_color.rgb * light.fill.w * pow(vec3<f32>(dot(n, lf) * 0.5 + 0.5), 2.0 * soften);
        var indirect = baked * light.mix.x + hemi;
        if (light.gi.w > 0.5 && probes.count.x > 0.5) {
            indirect = mix(indirect, probe_light(p, n) * light.gi.x, light.gi.y);
        }
        color = albedo * (indirect * ao + direct + fill) + light.sun_color.rgb * light.sun.w * nl * shadow * spec;
    }
    let debug = i32(light.splits.w);
    if (debug == 1) {
        return vec4<f32>(vec3<f32>(shadow * nl), 1.0);
    } else if (debug == 2) {
        return vec4<f32>(vec3<f32>(ao), 1.0);
    } else if (debug == 3) {
        return vec4<f32>(n * 0.5 + 0.5, 1.0);
    } else if (debug == 4) {
        return vec4<f32>(albedo, 1.0);
    } else if (debug == 5) {
        return vec4<f32>(baked, 1.0);
    } else if (debug == 7) {
        return vec4<f32>(vec3<f32>(skin_weight(albedo) * f32(character)), 1.0);
    } else if (debug == 8) {
        if (probes.count.x > 0.5) {
            return vec4<f32>(probe_light(p, n), 1.0);
        }
        return vec4<f32>(0.0);
    } else if (debug == 6) {
        let dz = abs(dot(light.view_z, vec4<f32>(p, 1.0)));
        let c = select(select(select(vec3<f32>(1.0), vec3<f32>(0.2, 0.2, 1.0), dz < light.splits.z), vec3<f32>(0.2, 1.0, 0.2), dz < light.splits.y), vec3<f32>(1.0, 0.2, 0.2), dz < light.splits.x);
        return vec4<f32>(c * (0.3 + 0.7 * shadow), 1.0);
    }
    // The game's fog, by distance from the eye.
    if (light.fog_range.w > 0.5) {
        let d = abs(dot(light.view_z, vec4<f32>(p, 1.0)));
        let t = clamp((d - light.fog_range.x) / (light.fog_range.y - light.fog_range.x + 0.01), 0.0, 1.0);
        let f = min(pow(t, light.fog_range.z), light.fog.w) * baked_fog.a;
        // The game blends its fog in gamma space: so do we (in linear it
        // lifts the dark parts).
        let g = mix(pow(max(color, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.2)), pow(light.fog.rgb, vec3<f32>(1.0 / 2.2)), f);
        color = pow(g, vec3<f32>(2.2));
    }
    return vec4<f32>(color, 1.0);
}

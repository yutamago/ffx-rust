// Water of the enhanced renderer, drawn into the HDR image after the
// lighting: the game's two scrolling normal maps and colour textures,
// screen-space reflection with the game's cube map behind it, refraction
// of the scene under the surface with absorption by depth, the key light
// on the surface, and the game's fog. Vertex input struct and accessors
// (a_pos … a_tan) are generated per draw, as for the G-buffer.

struct Camera {
    view_proj: array<vec4<f32>, 4>,
    view: array<vec4<f32>, 4>,
    inv_view_proj: mat4x4<f32>,
    eye: vec4<f32>,
    misc: vec4<f32>,
};

struct Water {
    world: array<vec4<f32>, 4>,
    // Texture coordinate offsets: colour 0, colour 1 (xy, zw) …
    scroll_tex: vec4<f32>,
    // … and normal maps 1 and 2.
    scroll_nml: vec4<f32>,
    color: vec4<f32>,
    // rgb: the game's specular colour, w: colour texture blend
    specular: vec4<f32>,
    // x: the game blends its water with what is behind (by its alpha)
    misc: vec4<f32>,
};

struct Scene {
    shadow: array<vec4<f32>, 12>,
    // xyz: direction the key light travels, w: intensity
    sun: vec4<f32>,
    sun_color: vec4<f32>,
    // rgb: fog colour, w: limit
    fog: vec4<f32>,
    // near, far, curve, on
    fog_range: vec4<f32>,
    view_z: vec4<f32>,
    splits: vec4<f32>,
    // x: reflection strength, y: refraction distortion, z: absorption,
    // w: cube map brightness
    params: vec4<f32>,
    // rgb: deep water colour, w: shadow on
    deep: vec4<f32>,
    // x: width, y: height
    screen: vec4<f32>,
};

@group(0) @binding(0) var<uniform> cam: Camera;
@group(1) @binding(0) var<uniform> water: Water;
@group(1) @binding(1) var tex0: texture_2d<f32>;
@group(1) @binding(2) var tex1: texture_2d<f32>;
@group(1) @binding(3) var nml0: texture_2d<f32>;
@group(1) @binding(4) var nml1: texture_2d<f32>;
@group(1) @binding(5) var cube: texture_cube<f32>;
@group(1) @binding(6) var samp: sampler;
@group(2) @binding(0) var<uniform> scene: Scene;
@group(2) @binding(1) var hdr_tex: texture_2d<f32>;
@group(2) @binding(2) var depth_tex: texture_depth_2d;
@group(2) @binding(3) var shadow_tex: texture_depth_2d_array;
@group(2) @binding(4) var shadow_samp: sampler_comparison;
@group(2) @binding(5) var linear_samp: sampler;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) nrm: vec3<f32>,
    @location(2) tan: vec3<f32>,
    @location(3) uv_tex: vec4<f32>,
    @location(4) uv_nml: vec4<f32>,
    @location(5) col: vec4<f32>,
};

fn rows3(r0: vec4<f32>, r1: vec4<f32>, r2: vec4<f32>, p: vec4<f32>) -> vec3<f32> {
    return vec3<f32>(dot(r0, p), dot(r1, p), dot(r2, p));
}

@vertex
fn vs(v: VsIn) -> VsOut {
    var out: VsOut;
    let p = vec4<f32>(a_pos(v), 1.0);
    let world = rows3(water.world[0], water.world[1], water.world[2], p);
    let w4 = vec4<f32>(world, 1.0);
    out.clip = vec4<f32>(dot(cam.view_proj[0], w4), dot(cam.view_proj[1], w4), dot(cam.view_proj[2], w4), dot(cam.view_proj[3], w4));
    out.world = world;
    let n = a_nrm(v);
    let t = a_tan(v);
    out.nrm = vec3<f32>(dot(water.world[0].xyz, n), dot(water.world[1].xyz, n), dot(water.world[2].xyz, n));
    out.tan = vec3<f32>(dot(water.world[0].xyz, t), dot(water.world[1].xyz, t), dot(water.world[2].xyz, t));
    out.uv_tex = vec4<f32>(a_uv0(v), a_uv1(v)) + water.scroll_tex;
    out.uv_nml = vec4<f32>(a_uv2(v), a_uv3(v)) + water.scroll_nml;
    out.col = a_col(v);
    return out;
}

fn shadow_at(p: vec3<f32>) -> f32 {
    if (scene.deep.w < 0.5) {
        return 1.0;
    }
    let d = abs(dot(scene.view_z, vec4<f32>(p, 1.0)));
    var c = 0;
    if (d > scene.splits.x) {
        c = 1;
    }
    if (d > scene.splits.y) {
        c = 2;
    }
    if (d > scene.splits.z) {
        return 1.0;
    }
    let wp = vec4<f32>(p, 1.0);
    let h = vec4<f32>(dot(scene.shadow[c * 4], wp), dot(scene.shadow[c * 4 + 1], wp), dot(scene.shadow[c * 4 + 2], wp), dot(scene.shadow[c * 4 + 3], wp));
    let s = h.xyz / h.w;
    let uv = vec2<f32>(s.x * 0.5 + 0.5, 0.5 - s.y * 0.5);
    if (any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) || s.z > 1.0) {
        return 1.0;
    }
    return textureSampleCompareLevel(shadow_tex, shadow_samp, uv, c, s.z);
}

fn project(p: vec3<f32>) -> vec3<f32> {
    let w = vec4<f32>(p, 1.0);
    let c = vec4<f32>(dot(cam.view_proj[0], w), dot(cam.view_proj[1], w), dot(cam.view_proj[2], w), dot(cam.view_proj[3], w));
    let s = c.xyz / c.w;
    return vec3<f32>(s.x * 0.5 + 0.5, 0.5 - s.y * 0.5, s.z);
}

fn view_depth(p: vec3<f32>) -> f32 {
    return abs(dot(cam.view[2], vec4<f32>(p, 1.0)));
}

fn unproject(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let ndc = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    let h = cam.inv_view_proj * vec4<f32>(ndc, depth, 1.0);
    return h.xyz / h.w;
}

// A short screen-space march for the reflection; w is the confidence.
fn ssr(p: vec3<f32>, r: vec3<f32>) -> vec4<f32> {
    let dist = max(length(cam.eye.xyz - p), 1.0);
    let step = r * dist * 0.06;
    var q = p;
    for (var i = 0; i < 24; i = i + 1) {
        q = q + step * (1.0 + f32(i) * 0.15);
        let s = project(q);
        if (any(s.xy < vec2<f32>(0.0)) || any(s.xy > vec2<f32>(1.0)) || s.z > 1.0 || s.z < 0.0) {
            return vec4<f32>(0.0);
        }
        let px = vec2<i32>(s.xy * scene.screen.xy);
        let d = textureLoad(depth_tex, px, 0);
        if (d < s.z && d < 1.0) {
            let hit = unproject(s.xy, d);
            // Close enough to the ray: a hit.
            if (length(hit - q) < length(step) * 3.0) {
                let edge = min(min(s.x, 1.0 - s.x), min(s.y, 1.0 - s.y));
                let fade = clamp(edge * 8.0, 0.0, 1.0) * (1.0 - f32(i) / 24.0);
                return vec4<f32>(textureLoad(hdr_tex, px, 0).rgb, fade);
            }
        }
    }
    return vec4<f32>(0.0);
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    var n = normalize(in.nrm);
    let t = normalize(in.tan - n * dot(in.tan, n));
    let b = cross(n, t);
    // The game adds its two normal maps (each 0–1) and recentres.
    let m = textureSample(nml0, samp, in.uv_nml.xy).xyz + textureSample(nml1, samp, in.uv_nml.zw).xyz - vec3<f32>(1.0);
    let mn = normalize(m);
    n = normalize(t * mn.x + b * mn.y + n * mn.z);
    let v = normalize(cam.eye.xyz - in.world);
    if (dot(n, v) < 0.0) {
        n = reflect(n, v);
    }
    let fresnel = 0.02 + 0.98 * pow(1.0 - max(dot(n, v), 0.0), 5.0);
    // Reflection: the scene where the march finds it, the cube map behind.
    let r = reflect(-v, n);
    let env = textureSample(cube, samp, r).rgb;
    // The game's water colour: its two colour textures and material colour
    // tint the cube map, as do the vertex colours.
    let c0 = textureSample(tex0, samp, in.uv_tex.xy);
    let c1 = textureSample(tex1, samp, in.uv_tex.zw);
    let tint = (c0.rgb + c1.rgb) * water.specular.w * water.color.rgb;
    // The game's own opacity of its water.
    let alpha = clamp((c0.a + 1.0 + c1.a) * water.specular.w * water.color.a * in.col.a, 0.0, 1.0);
    let surface = max(tint * env * in.col.rgb, vec3<f32>(0.0));
    let env_lin = pow(surface, vec3<f32>(2.2)) * scene.params.w;
    let hit = ssr(in.world, r);
    let reflection = mix(env_lin, hit.rgb, hit.a) * scene.params.x;
    // Refraction: what lies under the surface, darker and bluer with
    // depth.
    let here = project(in.world);
    let offset = n.xz * scene.params.y;
    var ruv = here.xy + offset;
    let size = vec2<i32>(scene.screen.xy);
    var under_depth = textureLoad(depth_tex, clamp(vec2<i32>(ruv * scene.screen.xy), vec2<i32>(0), size - 1), 0);
    if (under_depth < here.z) {
        // In front of the water: no offset there.
        ruv = here.xy;
        under_depth = textureLoad(depth_tex, clamp(vec2<i32>(ruv * scene.screen.xy), vec2<i32>(0), size - 1), 0);
    }
    let under = unproject(ruv, under_depth);
    let thickness = max(view_depth(under) - view_depth(in.world), 0.0);
    let absorb = exp(-thickness * scene.params.z);
    let hue = tint / max(max(tint.r, max(tint.g, tint.b)), 1e-3);
    let tint_lin = pow(max(hue, vec3<f32>(0.0)), vec3<f32>(2.2));
    let behind = textureSampleLevel(hdr_tex, linear_samp, ruv, 0.0).rgb;
    let deep = scene.deep.rgb * tint_lin;
    let refraction = mix(deep, behind * mix(vec3<f32>(1.0), tint_lin, 0.3), absorb);
    var color = mix(refraction, reflection, fresnel);
    // The key light on the surface.
    let l = -normalize(scene.sun.xyz);
    let h = normalize(l + v);
    let spec = pow(max(dot(n, h), 0.0), 400.0) * 40.0 * fresnel;
    color = color + scene.sun_color.rgb * scene.sun.w * spec * shadow_at(in.world) * max(dot(n, l), 0.0);
    // The game's fog.
    if (scene.fog_range.w > 0.5) {
        let d = view_depth(in.world);
        let f = min(pow(clamp((d - scene.fog_range.x) / (scene.fog_range.y - scene.fog_range.x + 0.01), 0.0, 1.0), scene.fog_range.z), scene.fog.w);
        let g = mix(pow(max(color, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.2)), pow(scene.fog.rgb, vec3<f32>(1.0 / 2.2)), f);
        color = pow(g, vec3<f32>(2.2));
    }
    if (water.misc.x > 0.5) {
        // Where the game's water is faint (seen from under it, say), so is
        // this.
        let clear = textureSampleLevel(hdr_tex, linear_samp, here.xy, 0.0).rgb;
        color = mix(clear, color, alpha);
    }
    return vec4<f32>(color, 1.0);
}

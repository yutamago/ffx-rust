// G-buffer pass of the enhanced renderer: the game's scenery and
// characters with their own geometry, textures and transforms, written
// as albedo, normal and baked light for the lighting pass. Variants are
// chosen by the Rust side, which defines the constants below:
//   SKINNED, CLOTH, HAS_COLOR, HAS_UV, ALPHA_TEST, SHADOW (depth only).

struct Camera {
    // Rows, as the game's constant buffers hold them (dp4 with each).
    view_proj: array<vec4<f32>, 4>,
    view: array<vec4<f32>, 4>,
    inv_view_proj: mat4x4<f32>,
    eye: vec4<f32>,
    // x: GlobalTime, y: normal map strength, z: material maps on,
    // w: roughness scale
    misc: vec4<f32>,
};

struct Item {
    world: array<vec4<f32>, 4>,
    cloth: array<vec4<f32>, 4>,
    color: vec4<f32>,
    // x: alpha threshold, yz: texture coordinate offset, w: material
    // (0 scenery, 1 character, 2 emissive)
    params: vec4<f32>,
    // rgb: light to use for "baked" when there are no vertex colours;
    // w: how much fog the draw takes (the game's fgEff × clrScaleFactor)
    ambient: vec4<f32>,
    // The game's clrScaleFactor: a final colour multiplier.
    scale: vec4<f32>,
    // x: surface detail strength for this draw
    extra: vec4<f32>,
};

@group(0) @binding(0) var<uniform> cam: Camera;
@group(1) @binding(0) var<uniform> item: Item;
@group(1) @binding(1) var<uniform> bones: array<vec4<f32>, 320>;
@group(1) @binding(2) var albedo_tex: texture_2d<f32>;
@group(1) @binding(3) var albedo_samp: sampler;
// Generated material: rg normal, b roughness, a cavity.
@group(1) @binding(4) var material_tex: texture_2d<f32>;

// VsIn and its accessors a_pos(v) … a_cloth(v) are generated per variant
// (only the attributes the draw has).

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) nrm: vec3<f32>,
    @location(1) col: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) world: vec3<f32>,
};

fn rows3(r0: vec4<f32>, r1: vec4<f32>, r2: vec4<f32>, p: vec4<f32>) -> vec3<f32> {
    return vec3<f32>(dot(r0, p), dot(r1, p), dot(r2, p));
}

@vertex
fn vs(v: VsIn) -> VsOut {
    var out: VsOut;
    let p = vec4<f32>(a_pos(v), 1.0);
    var world: vec3<f32>;
    var nrm: vec3<f32>;
    if (SKINNED) {
        world = vec3<f32>(0.0);
        nrm = vec3<f32>(0.0);
        for (var k = 0u; k < 4u; k = k + 1u) {
            let b = a_bi(v)[k] * 4u;
            let w = a_bw(v)[k];
            world = world + w * rows3(bones[b], bones[b + 1u], bones[b + 2u], p);
            nrm = nrm + w * vec3<f32>(dot(bones[b].xyz, a_nrm(v)), dot(bones[b + 1u].xyz, a_nrm(v)), dot(bones[b + 2u].xyz, a_nrm(v)));
        }
        if (CLOTH) {
            // Simulated cloth vertices come in COLOR1, in cloth space.
            let t = min(dot(a_cloth(v).xyz, a_cloth(v).xyz) * 10000.0, 1.0);
            let c = rows3(item.cloth[0], item.cloth[1], item.cloth[2], vec4<f32>(a_cloth(v).xyz, 1.0));
            world = mix(world, c, t);
        }
    } else {
        world = rows3(item.world[0], item.world[1], item.world[2], p);
        nrm = vec3<f32>(dot(item.world[0].xyz, a_nrm(v)), dot(item.world[1].xyz, a_nrm(v)), dot(item.world[2].xyz, a_nrm(v)));
    }
    let wp = vec4<f32>(world, 1.0);
    out.clip = vec4<f32>(dot(cam.view_proj[0], wp), dot(cam.view_proj[1], wp), dot(cam.view_proj[2], wp), dot(cam.view_proj[3], wp));
    out.nrm = nrm;
    out.world = world;
    if (HAS_COLOR) {
        out.col = a_col(v);
    } else {
        out.col = vec4<f32>(item.ambient.rgb, 1.0);
    }
    if (HAS_UV) {
        out.uv = a_uv(v) + item.params.yz;
    } else {
        out.uv = vec2<f32>(0.0);
    }
    return out;
}

struct GOut {
    @location(0) albedo: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) baked: vec4<f32>,
};

@fragment
fn fs(in: VsOut) -> GOut {
    var c = item.color;
    if (HAS_UV) {
        c = c * textureSample(albedo_tex, albedo_samp, in.uv);
    }
    if (ALPHA_TEST) {
        if (c.a < item.params.x) {
            discard;
        }
    }
    var n = normalize(in.nrm);
    var rough = cam.misc.w;
    var cavity = 1.0;
    if (HAS_UV) {
        let m = textureSample(material_tex, albedo_samp, in.uv);
        if (cam.misc.z > 0.5) {
            // A tangent frame from screen derivatives (no vertex tangents).
            let dp1 = dpdx(in.world);
            let dp2 = dpdy(in.world);
            let du1 = dpdx(in.uv);
            let du2 = dpdy(in.uv);
            let p2 = cross(dp2, n);
            let p1 = cross(n, dp1);
            let t = p2 * du1.x + p1 * du2.x;
            let b = p2 * du1.y + p1 * du2.y;
            let inv = inverseSqrt(max(max(dot(t, t), dot(b, b)), 1e-20));
            let nt = vec3<f32>((m.xy * 2.0 - 1.0) * cam.misc.y * item.extra.x, 1.0);
            let pn = normalize(t * inv * nt.x + b * inv * nt.y + n * nt.z);
            if (all(pn == pn)) {
                n = pn;
            }
            rough = clamp(m.z * cam.misc.w / 0.7, 0.05, 1.0);
            cavity = m.w;
        }
    }
    var out: GOut;
    out.albedo = vec4<f32>(c.rgb * item.scale.rgb * cavity, rough);
    out.normal = vec4<f32>(n, item.params.w);
    // The game's vertex colours are light in gamma space, with 0.5 as full.
    out.baked = vec4<f32>(pow(max(in.col.rgb * 2.0, vec3<f32>(0.0)), vec3<f32>(2.2)), item.ambient.w);
    return out;
}

@fragment
fn fs_shadow(in: VsOut) {
    if (ALPHA_TEST && HAS_UV) {
        let a = item.color.a * textureSample(albedo_tex, albedo_samp, in.uv).a;
        if (a < item.params.x) {
            discard;
        }
    }
}

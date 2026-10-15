// G-buffer pass of the enhanced renderer: the game's scenery and
// characters with their own geometry, textures and transforms, written
// as albedo, normal and baked light for the lighting pass. Variants are
// chosen by the Rust side, which defines the constants below:
//   SKINNED, CLOTH, HAS_COLOR, HAS_UV, ALPHA_TEST, SHADOW (depth only),
//   SEAL (the edges as lines, to fill gaps between meshes).

struct Camera {
    // Rows, as the game's constant buffers hold them (dp4 with each).
    view_proj: array<vec4<f32>, 4>,
    view: array<vec4<f32>, 4>,
    inv_view_proj: mat4x4<f32>,
    eye: vec4<f32>,
    // x: GlobalTime, y: normal map strength, z: material maps on,
    // w: roughness scale
    misc: vec4<f32>,
    // Rows of this frame's view-projection without the TAA jitter, and of
    // last frame's (for motion vectors).
    now_view_proj: array<vec4<f32>, 4>,
    prev_view_proj: array<vec4<f32>, 4>,
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
    // x: surface detail strength for this draw, y: roughness (below 0:
    // from the material map), z: metallic, w: specular scale
    extra: vec4<f32>,
    // x: emission, y: skin detection (0–1: how much skin-coloured pixels
    // take the skin material), z: skin roughness, w: surface detail on skin
    look: vec4<f32>,
    // rgb: tint (multiplies the colour), w: the draw's id + 1 (picking),
    // plus 2^23 when the editor highlights it
    tint: vec4<f32>,
    // Last frame's World of the same draw (this frame's when it is new).
    prev_world: array<vec4<f32>, 4>,
};

@group(0) @binding(0) var<uniform> cam: Camera;
@group(1) @binding(0) var<uniform> item: Item;
@group(1) @binding(1) var<uniform> bones: array<vec4<f32>, 320>;
@group(1) @binding(2) var albedo_tex: texture_2d<f32>;
@group(1) @binding(3) var albedo_samp: sampler;
// Generated material: rg normal, b roughness, a cavity.
@group(1) @binding(4) var material_tex: texture_2d<f32>;
// Last frame's bones of the same draw.
@group(1) @binding(5) var<uniform> prev_bones: array<vec4<f32>, 320>;

// VsIn and its accessors a_pos(v) … a_cloth(v) are generated per variant
// (only the attributes the draw has).

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) nrm: vec3<f32>,
    @location(1) col: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) world: vec3<f32>,
    // Unjittered clip positions now and last frame.
    @location(4) now_clip: vec4<f32>,
    @location(5) prev_clip: vec4<f32>,
};

fn clip(m: array<vec4<f32>, 4>, p: vec3<f32>) -> vec4<f32> {
    let wp = vec4<f32>(p, 1.0);
    return vec4<f32>(dot(m[0], wp), dot(m[1], wp), dot(m[2], wp), dot(m[3], wp));
}

fn rows3(r0: vec4<f32>, r1: vec4<f32>, r2: vec4<f32>, p: vec4<f32>) -> vec3<f32> {
    return vec3<f32>(dot(r0, p), dot(r1, p), dot(r2, p));
}

@vertex
fn vs(v: VsIn) -> VsOut {
    var out: VsOut;
    let p = vec4<f32>(a_pos(v), 1.0);
    var world: vec3<f32>;
    var prev: vec3<f32>;
    var nrm: vec3<f32>;
    if (SKINNED) {
        world = vec3<f32>(0.0);
        prev = vec3<f32>(0.0);
        nrm = vec3<f32>(0.0);
        for (var k = 0u; k < 4u; k = k + 1u) {
            let b = a_bi(v)[k] * 4u;
            let w = a_bw(v)[k];
            world = world + w * rows3(bones[b], bones[b + 1u], bones[b + 2u], p);
            prev = prev + w * rows3(prev_bones[b], prev_bones[b + 1u], prev_bones[b + 2u], p);
            nrm = nrm + w * vec3<f32>(dot(bones[b].xyz, a_nrm(v)), dot(bones[b + 1u].xyz, a_nrm(v)), dot(bones[b + 2u].xyz, a_nrm(v)));
        }
        if (CLOTH) {
            // Simulated cloth vertices come in COLOR1, in cloth space; last
            // frame's are not known, so they move as the skin under them.
            let t = min(dot(a_cloth(v).xyz, a_cloth(v).xyz) * 10000.0, 1.0);
            let c = rows3(item.cloth[0], item.cloth[1], item.cloth[2], vec4<f32>(a_cloth(v).xyz, 1.0));
            let moved = prev - world;
            world = mix(world, c, t);
            prev = world + moved;
        }
    } else {
        world = rows3(item.world[0], item.world[1], item.world[2], p);
        prev = rows3(item.prev_world[0], item.prev_world[1], item.prev_world[2], p);
        nrm = vec3<f32>(dot(item.world[0].xyz, a_nrm(v)), dot(item.world[1].xyz, a_nrm(v)), dot(item.world[2].xyz, a_nrm(v)));
    }
    out.now_clip = clip(cam.now_view_proj, world);
    out.prev_clip = clip(cam.prev_view_proj, prev);
    var wp = vec4<f32>(world, 1.0);
    if (SEAL) {
        // Sealing lines sit 0.2% further than their surface: they lose to
        // it and to anything as near, and fill only the gaps.
        wp = vec4<f32>(world + (world - cam.eye.xyz) * 0.002, 1.0);
    }
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
    // rg: where this point was last frame minus where it is (uv), b: its
    // view depth then, a: 1 (no motion where nothing was drawn).
    @location(3) motion: vec4<f32>,
    // r: metallic, g: specular / 2, b: emission / 8, a: skin
    @location(4) material: vec4<f32>,
    // The draw's id + 1 (0: nothing), for picking in the editor; the top
    // bit: highlighted.
    @location(5) id: u32,
};

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

fn screen_uv(c: vec4<f32>) -> vec2<f32> {
    let n = c.xy / c.w;
    return vec2<f32>(n.x * 0.5 + 0.5, 0.5 - n.y * 0.5);
}

@fragment
fn fs(in: VsOut) -> GOut {
    var c = item.color;
    if (HAS_UV) {
        c = c * textureSample(albedo_tex, albedo_samp, in.uv);
    }
    c = vec4<f32>(c.rgb * item.tint.rgb, c.a);
    if (ALPHA_TEST) {
        if (c.a < item.params.x) {
            discard;
        }
    }
    // The back of a two-sided draw (hair cards, cloth) turns its normal
    // round: the normal faces away from the triangle seen (its geometric
    // normal, toward the camera).
    var n = normalize(in.nrm);
    var g = cross(dpdx(in.world), dpdy(in.world));
    if (dot(g, cam.eye.xyz - in.world) < 0.0) {
        g = -g;
    }
    if (dot(n, g) < 0.0) {
        n = -n;
    }
    var rough = cam.misc.w;
    var cavity = 1.0;
    // Skin (by its colour) has its own roughness and less surface detail;
    // not on cut-out textures (hair: blond is skin-coloured), told by the
    // smallest mip, the texture's mean alpha.
    var skin = skin_weight(c.rgb * item.scale.rgb) * item.look.y;
    if (HAS_UV) {
        let levels = textureNumLevels(albedo_tex);
        if (levels > 3u) {
            let mean = textureSampleLevel(albedo_tex, albedo_samp, vec2<f32>(0.5), f32(levels - 1u)).a;
            skin = skin * smoothstep(0.85, 0.97, mean);
        }
    }
    let detail = item.extra.x * mix(1.0, item.look.w, skin);
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
            let nt = vec3<f32>((m.xy * 2.0 - 1.0) * cam.misc.y * detail, 1.0);
            let pn = normalize(t * inv * nt.x + b * inv * nt.y + n * nt.z);
            if (all(pn == pn)) {
                n = pn;
            }
            rough = clamp(m.z * cam.misc.w / 0.7, 0.05, 1.0);
            cavity = m.w;
        }
    }
    if (item.extra.y >= 0.0) {
        rough = item.extra.y;
    }
    rough = clamp(mix(rough, item.look.z, skin), 0.05, 1.0);
    var out: GOut;
    out.albedo = vec4<f32>(c.rgb * item.scale.rgb * cavity, rough);
    out.material = vec4<f32>(item.extra.z, clamp(item.extra.w * 0.5, 0.0, 1.0), clamp(item.look.x / 8.0, 0.0, 1.0), skin);
    let raw = u32(item.tint.w);
    out.id = (raw & 0x7fffffu) | select(0u, 0x80000000u, raw >= 0x800000u);
    // w: the material (0 scenery, 1 character) plus half the game's
    // alpha (its glow passes read the scene's alpha).
    // (The game's: colour alpha × vertex alpha, 0.5 as full.)
    out.normal = vec4<f32>(n, item.params.w + 0.5 * clamp(c.a * in.col.a * 2.0, 0.0, 1.0));
    // The game's vertex colours are light in gamma space, with 0.5 as full.
    out.baked = vec4<f32>(pow(max(in.col.rgb * 2.0, vec3<f32>(0.0)), vec3<f32>(2.2)), item.ambient.w);
    out.motion = vec4<f32>(screen_uv(in.prev_clip) - screen_uv(in.now_clip), in.prev_clip.w, 1.0);
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

// Depth and the draw id only (at the game's size).
@fragment
fn fs_edge(in: VsOut) -> @location(0) u32 {
    if (ALPHA_TEST && HAS_UV) {
        let a = item.color.a * textureSample(albedo_tex, albedo_samp, in.uv).a;
        if (a < item.params.x) {
            discard;
        }
    }
    let raw = u32(item.tint.w);
    return (raw & 0x7fffffu) | select(0u, 0x80000000u, raw >= 0x800000u);
}

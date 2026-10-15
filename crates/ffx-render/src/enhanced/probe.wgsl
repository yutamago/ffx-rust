// Light probes for characters: each probe is a small cube of the scenery
// around a character, drawn into G-buffer layers (six per probe). This
// pass turns each cube into the radiance the scenery shows (albedo × its
// baked light, the sky where nothing was drawn) and projects it onto
// nine spherical harmonics, for the lighting pass's indirect light.

struct Info {
    // Inverse view-projections of the faces, six per probe.
    faces: array<mat4x4<f32>, 12>,
    // xyz: probe positions
    pos: array<vec4<f32>, 2>,
    // rgb: sky radiance
    sky: vec4<f32>,
    // x: probe count, y: baked light weight, z: face size, w: the new
    // harmonics' share (the rest is the last ones', for the same place)
    params: vec4<f32>,
};

struct Probes {
    // x: probe count (written with the frame's uniforms)
    count: vec4<f32>,
    pos: array<vec4<f32>, 2>,
    // Nine coefficients (rgb) per probe.
    sh: array<vec4<f32>, 18>,
};

@group(0) @binding(0) var<uniform> info: Info;
@group(0) @binding(1) var albedo_tex: texture_2d_array<f32>;
@group(0) @binding(2) var baked_tex: texture_2d_array<f32>;
@group(0) @binding(3) var depth_tex: texture_depth_2d_array;
@group(0) @binding(4) var<storage, read_write> probes: Probes;

var<workgroup> acc: array<array<vec3<f32>, 10>, 64>;

fn basis(d: vec3<f32>) -> array<f32, 9> {
    return array<f32, 9>(
        0.282095,
        0.488603 * d.y,
        0.488603 * d.z,
        0.488603 * d.x,
        1.092548 * d.x * d.y,
        1.092548 * d.y * d.z,
        0.315392 * (3.0 * d.z * d.z - 1.0),
        1.092548 * d.x * d.z,
        0.546274 * (d.x * d.x - d.y * d.y),
    );
}

@compute @workgroup_size(64)
fn main(@builtin(local_invocation_index) lid: u32, @builtin(workgroup_id) wid: vec3<u32>) {
    let probe = wid.x;
    let size = u32(info.params.z);
    let total = 6u * size * size;
    var sum: array<vec3<f32>, 10>;
    for (var k = 0u; k < 10u; k = k + 1u) {
        sum[k] = vec3<f32>(0.0);
    }
    let centre = info.pos[probe].xyz;
    for (var i = lid; i < total; i = i + 64u) {
        let face = i / (size * size);
        let x = i % size;
        let y = (i / size) % size;
        let layer = i32(probe * 6u + face);
        let uv = (vec2<f32>(f32(x), f32(y)) + 0.5) / f32(size);
        let ndc = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
        let h = info.faces[probe * 6u + face] * vec4<f32>(ndc, 0.5, 1.0);
        let d = normalize(h.xyz / h.w - centre);
        // Solid angle of the texel on a 90° face.
        let w = 4.0 / (f32(size * size) * pow(1.0 + dot(ndc, ndc), 1.5));
        let px = vec2<i32>(i32(x), i32(y));
        var radiance = info.sky.rgb;
        if (textureLoad(depth_tex, px, layer, 0) < 1.0) {
            radiance = textureLoad(albedo_tex, px, layer, 0).rgb * textureLoad(baked_tex, px, layer, 0).rgb * info.params.y;
        }
        var y9 = basis(d);
        for (var k = 0u; k < 9u; k = k + 1u) {
            sum[k] = sum[k] + radiance * y9[k] * w;
        }
        sum[9] = sum[9] + vec3<f32>(w);
    }
    for (var k = 0u; k < 10u; k = k + 1u) {
        acc[lid][k] = sum[k];
    }
    workgroupBarrier();
    if (lid == 0u) {
        var t: array<vec3<f32>, 10>;
        for (var k = 0u; k < 10u; k = k + 1u) {
            t[k] = vec3<f32>(0.0);
        }
        for (var j = 0u; j < 64u; j = j + 1u) {
            for (var k = 0u; k < 10u; k = k + 1u) {
                t[k] = t[k] + acc[j][k];
            }
        }
        // Normalise to the whole sphere (4π).
        let norm = 12.566371 / max(t[9].x, 1e-6);
        // Eased from the last ones where the probe stayed.
        let last = probes.pos[probe];
        var share = 1.0;
        if (last.w > 0.5 && distance(last.xyz, centre) < 8.0) {
            share = clamp(info.params.w, 0.0, 1.0);
        }
        for (var k = 0u; k < 9u; k = k + 1u) {
            let old = probes.sh[probe * 9u + k].xyz;
            probes.sh[probe * 9u + k] = vec4<f32>(mix(old, t[k] * norm, share), 0.0);
        }
        probes.pos[probe] = vec4<f32>(centre, 1.0);
    }
}

// Light from the game's effects (fire, magic, glowing things): the
// effects' additive draws, drawn alone onto black, blurred wide at low
// resolution, light the surfaces around them. Three entry points: down
// (to an eighth of the size), a separable blur, and the light added to
// the scene image (apply).

struct Fx {
    // x: strength, yz: blur direction (texels)
    params: vec4<f32>,
};

@group(0) @binding(0) var<uniform> fx: Fx;
// down: the effects alone; blur: its input; apply: the scene image.
@group(0) @binding(1) var a_tex: texture_2d<f32>;
// apply: the blurred light.
@group(0) @binding(2) var b_tex: texture_2d<f32>;
@group(0) @binding(3) var albedo_tex: texture_2d<f32>;
@group(0) @binding(4) var depth_tex: texture_depth_2d;
@group(0) @binding(5) var samp: sampler;

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

fn added(uv: vec2<f32>) -> vec3<f32> {
    return pow(max(textureSampleLevel(a_tex, samp, uv, 0.0).rgb, vec3<f32>(0.0)), vec3<f32>(2.2));
}

@fragment
fn fs_down(in: VsOut) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(a_tex));
    let out_size = size / 8.0;
    let uv = in.pos.xy / out_size;
    let t = 1.0 / size;
    // 16 bilinear taps: the 8×8 pixels under this one.
    var sum = vec3<f32>(0.0);
    for (var y = 0; y < 4; y = y + 1) {
        for (var x = 0; x < 4; x = x + 1) {
            let o = (vec2<f32>(f32(x), f32(y)) * 2.0 - 3.0) * t;
            sum = sum + added(uv + o);
        }
    }
    return vec4<f32>(sum / 16.0, 1.0);
}

@fragment
fn fs_blur(in: VsOut) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(a_tex));
    let uv = in.pos.xy / size;
    let step = fx.params.yz / size;
    var sum = vec3<f32>(0.0);
    var weight = 0.0;
    for (var i = -6; i <= 6; i = i + 1) {
        let w = exp(-f32(i * i) / 18.0);
        sum = sum + textureSampleLevel(a_tex, samp, uv + step * f32(i), 0.0).rgb * w;
        weight = weight + w;
    }
    return vec4<f32>(sum / weight, 1.0);
}

@fragment
fn fs_apply(in: VsOut) -> @location(0) vec4<f32> {
    let px = vec2<i32>(in.pos.xy);
    let old = textureLoad(a_tex, px, 0);
    let size = vec2<f32>(textureDimensions(a_tex));
    let uv = in.pos.xy / size;
    let inner = vec2<f32>(textureDimensions(albedo_tex));
    let ipx = clamp(vec2<i32>(uv * inner), vec2<i32>(0), vec2<i32>(inner) - 1);
    if (textureLoad(depth_tex, ipx, 0) >= 1.0) {
        return old;
    }
    let light = textureSampleLevel(b_tex, samp, uv, 0.0).rgb;
    let albedo = textureLoad(albedo_tex, ipx, 0).rgb;
    let lin = pow(old.rgb, vec3<f32>(2.2)) + albedo * light * fx.params.x;
    return vec4<f32>(pow(lin, vec3<f32>(1.0 / 2.2)), old.a);
}

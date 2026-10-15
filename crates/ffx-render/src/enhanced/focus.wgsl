// Depth of field's focus: the view depth at the centre of the screen (the
// nearest of a few points there), eased toward over a few frames so it
// does not jump.

struct Post {
    inv_view_proj: mat4x4<f32>,
    prev_view_proj: array<vec4<f32>, 4>,
    dof: vec4<f32>,
    blur: vec4<f32>,
    bloom: vec4<f32>,
};

@group(0) @binding(0) var<uniform> post: Post;
@group(0) @binding(1) var depth_tex: texture_depth_2d;
@group(0) @binding(2) var<storage, read_write> focus: vec4<f32>;

@compute @workgroup_size(1)
fn main() {
    let size = vec2<f32>(textureDimensions(depth_tex));
    var nearest = 1e9;
    for (var y = -2; y <= 2; y = y + 1) {
        for (var x = -2; x <= 2; x = x + 1) {
            let px = size * (vec2<f32>(0.5) + vec2<f32>(f32(x), f32(y)) * 0.03);
            let uv = px / size;
            let ndc = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
            let h = post.inv_view_proj * vec4<f32>(ndc, textureLoad(depth_tex, vec2<i32>(px), 0), 1.0);
            nearest = min(nearest, 1.0 / max(abs(h.w), 1e-8));
        }
    }
    var want = nearest;
    if (post.dof.w > 0.0) {
        want = post.dof.w;
    }
    // y: 1 once set (the first frame takes it as it is).
    if (focus.y < 0.5 || post.dof.z >= 1.0) {
        focus = vec4<f32>(want, 1.0, 0.0, 0.0);
    } else {
        focus = vec4<f32>(mix(focus.x, want, post.dof.z), 1.0, 0.0, 0.0);
    }
}

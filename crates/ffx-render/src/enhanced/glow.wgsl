// Lights from the game's glowing effects: per 32x32 tile of the effects
// drawn alone (colour) and their nearest depth, the light there and where
// it is. The renderer gathers the tiles into point lights (`glow.rs`).

struct Glow {
    inv_view_proj: mat4x4<f32>,
    // xy: image size, z: tiles across, w: the faintest light that counts
    size: vec4<f32>,
};

// a: Σ luminance, Σ luminance × position; b: Σ colour, Σ luminance × the
// world area a pixel covers there (how strong the source is, wherever it
// stands).
struct Tile {
    a: vec4<f32>,
    b: vec4<f32>,
};

@group(0) @binding(0) var<uniform> glow: Glow;
@group(0) @binding(1) var color_tex: texture_2d<f32>;
@group(0) @binding(2) var depth_tex: texture_depth_2d;
@group(0) @binding(3) var<storage, read_write> tiles: array<Tile>;

const TILE: u32 = 32u;

fn world(px: vec2<f32>, z: f32) -> vec3<f32> {
    let uv = px / glow.size.xy;
    let h = glow.inv_view_proj * vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, z, 1.0);
    return h.xyz / h.w;
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let across = u32(glow.size.z);
    let down = (u32(glow.size.y) + TILE - 1u) / TILE;
    if (id.x >= across || id.y >= down) {
        return;
    }
    let size = vec2<u32>(glow.size.xy);
    var a = vec4<f32>(0.0);
    var b = vec4<f32>(0.0);
    for (var y = 0u; y < TILE; y = y + 1u) {
        for (var x = 0u; x < TILE; x = x + 1u) {
            let q = id.xy * TILE + vec2<u32>(x, y);
            if (q.x >= size.x || q.y >= size.y) {
                continue;
            }
            let z = textureLoad(depth_tex, vec2<i32>(q), 0);
            if (z >= 1.0) {
                continue;
            }
            let c = pow(max(textureLoad(color_tex, vec2<i32>(q), 0).rgb, vec3<f32>(0.0)), vec3<f32>(2.2));
            let lum = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
            if (lum < glow.size.w) {
                continue;
            }
            let pc = vec2<f32>(q) + 0.5;
            let p = world(pc, z);
            let area = length(cross(world(pc + vec2<f32>(1.0, 0.0), z) - p, world(pc + vec2<f32>(0.0, 1.0), z) - p));
            a = a + vec4<f32>(lum, lum * p);
            b = b + vec4<f32>(c, lum * area);
        }
    }
    tiles[id.y * across + id.x] = Tile(a, b);
}

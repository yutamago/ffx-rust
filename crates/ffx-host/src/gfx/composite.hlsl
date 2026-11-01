// ffx-host's Iggy composite (crates/ffx-host/src/gfx/composite.rs): the
// Flash player's premultiplied image, one texel per pixel, over the target.
struct V {
    float4 pos : SV_Position;
    float2 uv : TEXCOORD0;
};

V vs_main(uint id : SV_VertexID) {
    V o;
    float2 t = float2(id & 1, id >> 1);
    o.uv = t;
    o.pos = float4(t.x * 2.0 - 1.0, 1.0 - t.y * 2.0, 0.0, 1.0);
    return o;
}

Texture2D<float4> image : register(t0);

float4 ps_main(V i) : SV_Target {
    uint w, h;
    image.GetDimensions(w, h);
    return image.Load(int3(int2(i.uv * float2(w, h)), 0));
}

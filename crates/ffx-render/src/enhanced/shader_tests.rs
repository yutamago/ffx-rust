//! The enhanced passes' shaders parse and validate (as wgpu would check
//! them when the game first needs them).

use super::{
    AO_WGSL, EFFECT_WGSL, LIGHTING_WGSL, MATERIAL_WGSL, NO_RT_WGSL, OUTPUT_WGSL, PROBE_WGSL,
    RESOLVE_WGSL, RT_SHADOW_WGSL, TAA_WGSL, VOLUMETRIC_WGSL, Variant, gbuffer_source, water_source,
};

fn validate(name: &str, source: &str) {
    let module = naga::front::wgsl::parse_str(source)
        .unwrap_or_else(|e| panic!("{name}: {}", e.emit_to_string(source)));
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap_or_else(|e| panic!("{name}: {}", e.emit_to_string(source)));
}

#[test]
fn passes() {
    for (name, source) in [
        ("ao", AO_WGSL),
        ("effect", EFFECT_WGSL),
        ("material", MATERIAL_WGSL),
        ("output", OUTPUT_WGSL),
        ("probe", PROBE_WGSL),
        ("resolve", RESOLVE_WGSL),
        ("taa", TAA_WGSL),
        ("volumetric", VOLUMETRIC_WGSL),
        ("post", super::post::POST_WGSL),
        ("focus", super::post::FOCUS_WGSL),
    ] {
        validate(name, source);
    }
    validate("water", &water_source(&(0..8).collect::<Vec<_>>()));
    validate("water, position only", &water_source(&[0]));
    validate("lighting", &format!("{NO_RT_WGSL}{LIGHTING_WGSL}"));
    validate(
        "lighting (ray traced)",
        &format!("{RT_SHADOW_WGSL}{LIGHTING_WGSL}"),
    );
}

#[test]
fn gbuffer_variants() {
    for bits in 0..32u32 {
        let v = Variant {
            skinned: bits & 1 != 0,
            cloth: bits & 3 == 3,
            color: bits & 4 != 0,
            uv: bits & 8 != 0,
            alpha_test: bits & 16 != 0,
            shadow: false,
        };
        let present: Vec<u32> = (0..7).collect();
        validate(&format!("gbuffer {v:?}"), &gbuffer_source(v, &present));
        validate(
            &format!("gbuffer {v:?}, position only"),
            &gbuffer_source(v, &[0]),
        );
    }
}

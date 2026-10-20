//! The harvested-state store's layout check (docs/hooking.md "Harvested call
//! states"). It lives in this one file so that it runs once. The parity tests
//! are in `harvested_<module>.rs`; their shared helpers are in `tests/common/mod.rs`.

use ffx_oracle::CallState;

/// The store's format round-trips, and a state of the first target applies
/// to a fresh oracle (layout check).
#[test]
fn harvested_states_load() {
    let Some(cases) = ffx_oracle::harvested(0x006e_d2e0) else {
        return;
    };
    let c = &cases[0];
    let again = CallState::from_bytes(&c.state.to_bytes(), c.state.key).unwrap();
    assert_eq!(again, c.state);
    let (a, _) = c.oracles().unwrap();
    assert_eq!(a.layout().image_base, c.state.layout.image_base);
}

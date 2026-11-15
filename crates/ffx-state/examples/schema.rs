//! Prints the API's schema (`ffx_state::schema::text`): the block
//! docs/state-api.md keeps.

fn main() {
    print!("{}", ffx_state::schema::text());
}

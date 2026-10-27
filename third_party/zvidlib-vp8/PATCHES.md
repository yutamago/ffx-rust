# zvidlib-vp8 0.5.0, vendored

Source: crates.io `zvidlib-vp8` 0.5.0 (<https://github.com/lsegal/zvidlib>),
MIT licence (`LICENSE`); its own third-party notice (the RFC 6386 / libvpx
tables, BSD-3-Clause) is in `THIRD_PARTY_NOTICES.md`.

`ffx-movie` uses its software VP8 decoder for the movies
(`docs/formats/platform.md`, "Movies"): it decodes every frame of every
movie in `FFX_Data.vbf` bit for bit as FFX.exe's libvpx 1.4.0 does.
The published crate only hands out RGBA pictures; the game needs the I420
planes, so the decoder is vendored with these changes and nothing else:

- `src/lib.rs`: `mod decoder` is `pub` (doc-hidden).
- `src/decoder.rs`: `Picture`, `Decoder`, `Decoder::new` and
  `Decoder::decode` are `pub` instead of `pub(crate)`.
- `Cargo.toml`: the `hardware` feature is empty and off (no
  `zvidlib-hardware`: NVDEC/Media Foundation/VideoToolbox are not used),
  no dev-dependencies, the `Cargo.toml.orig`, `Cargo.lock` and
  `.cargo_vcs_info.json` of the package are dropped.

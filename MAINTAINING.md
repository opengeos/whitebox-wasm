# Maintaining whitebox-wasm

This repository **vendors** the pure-Rust geospatial crates from
[`jblindsay/whitebox_next_gen`](https://github.com/jblindsay/whitebox_next_gen)
and exposes them through WebAssembly. Vendoring (rather than depending on
published crates) is necessary because the crates are not on crates.io and
because a handful of small, local patches are needed for WASM. This document
explains those patches and how to pull in upstream updates.

## Repository layout

| Path | What it is | Target |
|---|---|---|
| `crates/wbgeotiff`, `wbprojection`, `wbhdf`, `wbvector`, `wblidar`, `wbtopology`, `wbspatialstats` | vendored engine crates | browser `wasm32-unknown-unknown` |
| `crates/whitebox-wasm` | the `wasm-bindgen` wrapper (the npm package) | browser |
| `crates/wbcore`, `wbraster`, `wbtools_oss` | vendored, used only by the tool runner | WASI `wasm32-wasip1` |
| `crates/whitebox-cli` | WASI binary that runs the 733 `wbtools_oss` tools on files | WASI |
| `crates/kdtree-wasm` | patched copy of the `kdtree` crate (see below) | both |

The browser npm package is built **only** from `whitebox-wasm` and its
dependencies; it does not include `wbcore`/`wbraster`/`wbtools_oss`/`whitebox-cli`.

## WASM compatibility reference (the issues and how they were solved)

1. **`getrandom` on `wasm32-unknown-unknown`** - `rand` pulls `getrandom`, which
   refuses the browser target by default. Fixed in `crates/whitebox-wasm/Cargo.toml`
   with a target-gated dependency enabling the `js` feature:
   ```toml
   [target.'cfg(target_arch = "wasm32")'.dependencies]
   getrandom = { version = "0.2", features = ["js"] }
   ```
   (Not needed for the WASI build - WASI provides randomness natively.)

2. **`kdtree 0.8.0` ships `criterion` as a normal dependency** (a packaging bug;
   it belongs in `dev-dependencies`), and `criterion` has a `compile_error!` for
   `wasm + rayon`. This blocks `wbtools_oss` on both WASM targets. Fixed by the
   vendored `crates/kdtree-wasm` (criterion removed) plus the workspace
   `[patch.crates-io] kdtree = { path = "crates/kdtree-wasm" }`.

3. **`ureq` -> `rustls` -> `ring`** (used by one tool, `DownloadOsmVectorTool`) -
   `ring` needs OS entropy and `rustls` needs a wall clock, so it fails on
   `wasm32-unknown-unknown`. It is **not** a blocker on WASI (which provides
   both), so the WASI tool runner builds it unchanged. For a future browser build
   of `wbtools_oss`, make `ureq` optional behind a `network` feature and do HTTP
   in JS (as `CogStream` does).

4. **Path-based file I/O** - `wbtools_oss` tools read/write rasters by file path.
   That cannot run in the browser (no filesystem) but works under **WASI**, where
   `--dir host::/guest` maps real directories into the sandbox. This is why the
   tool runner targets `wasm32-wasip1`. Browser-side readers instead use
   byte-based entry points (see the patch set).

## Local patch set (re-apply after re-vendoring)

Re-copying a crate's `src/` from upstream **overwrites these edits**, so they
must be re-applied. All are small, additive, and backward-compatible.

| Crate / file | Change | Why |
|---|---|---|
| `wbgeotiff/src/reader.rs` | `from_bytes` parses from a borrowed cursor (one copy, not three); add `peek_meta()` + `GeoTiffMeta`; add `CogLevel`/`CogLayout` + `parse_cog_layout()` + `decode_tile_f64()`; add `GeoTiff::proj_string()` | header-only metadata for huge files; COG range-request streaming |
| `wbgeotiff/src/geo_keys.rs` | add `GeoKeyDirectory::to_proj_string()` | lon/lat for user-defined projections (e.g. NLCD Albers) |
| `wbgeotiff/src/cog.rs` | add `write_u8_to_vec` / `write_f32_to_vec` / `write_f64_to_vec` (+ `encode_to_vec`) | write COGs to bytes (no filesystem) |
| `wbgeotiff/src/lib.rs` | re-export `CogLayout, CogLevel, GeoTiffMeta` | expose the above |
| `wbvector/src/geopackage/mod.rs` | add `from_bytes(Vec<u8>) -> Layer` | read GeoPackage from memory |
| `wblidar/src/laz/reader.rs` | add `LazReader::header()` passthrough | LAZ metadata without decoding all points |
| `wbhdf/Cargo.toml` | `thiserror = "1"` instead of `{ workspace = true }` | optional now that the root defines `[workspace.dependencies]` |
| `crates/kdtree-wasm/Cargo.toml` | remove the `criterion` dependency + `[[bench]]` | issue 2 above |

> Tip for future maintainability: prefer putting new functions in a **separate
> file** (e.g. `wbgeotiff/src/wasm_ext.rs` with `pub mod wasm_ext;`) so that
> re-copying upstream `src/` does not clobber them. The patches above are inline
> only because they extend existing `impl` blocks.

Everything else lives in `crates/whitebox-wasm/src/` (the wrapper) and
`crates/whitebox-cli/src/` (the runner), which are **not** vendored - upstream
updates never touch them.

## Updating from upstream

1. **Re-vendor the changed crates.** For each crate, copy `src/` and `Cargo.toml`
   from `whitebox_next_gen/crates/<name>/`, then strip `[dev-dependencies]`,
   `[[bench]]`, and `[[example]]` from the manifest (they pull `criterion`/test
   data we don't ship). A starting point:
   ```bash
   UP=/path/to/whitebox_next_gen/crates
   for c in wbgeotiff wbprojection wbhdf wbvector wblidar wbtopology \
            wbspatialstats wbcore wbraster wbtools_oss; do
     rm -rf crates/$c/src && cp -r $UP/$c/src crates/$c/src
     cp $UP/$c/Cargo.toml crates/$c/Cargo.toml   # then strip dev-deps/benches/examples
   done
   ```
2. **Re-apply the patch set** above (the table). `git diff` against the previous
   vendored version is the fastest way to see what was lost.
3. **Re-check the blockers.** If upstream bumped `kdtree`, verify it still ships
   `criterion` as a normal dependency; if upstream fixed it (or moved to a
   different nearest-neighbour crate), drop `crates/kdtree-wasm` and the
   `[patch.crates-io]`. Re-run the getrandom/ureq checks if `rand`/`ureq` versions
   changed.
4. **Build and test both targets:**
   ```bash
   # browser package
   wasm-pack build crates/whitebox-wasm --release --target web --out-dir pkg
   node examples/node-demo.mjs
   cargo test -p wbgeotiff -p whitebox-wasm

   # WASI tool runner
   cargo build -p whitebox-cli --target wasm32-wasip1 --release
   wasmtime run target/wasm32-wasip1/release/whitebox.wasm list
   ```
5. **Bump the npm version** (`crates/whitebox-wasm/Cargo.toml`) and tag
   `vX.Y.Z` to publish (see [README](README.md#releasing)).

## Upstreaming

The cleanest long-term fix is to push these changes upstream so vendoring becomes
a straight copy:
- **`kdtree`**: move `criterion` to `dev-dependencies` (patch 2).
- **`whitebox_next_gen`**: add the byte-based / in-memory entry points (the patch
  set) to the engine crates, and make `ureq` optional behind a `network` feature
  in `wbtools_oss`.

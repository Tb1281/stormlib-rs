[![Rust](https://github.com/Tb1281/stormlib-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/Tb1281/stormlib-rs/actions/workflows/rust.yml)

Rust [StormLib](https://github.com/ladislav-zezula/StormLib) binding for working with Blizzard MPQ archives

Fork of [wc3tools/stormlib-rs](https://github.com/wc3tools/stormlib-rs), kept as a
git dependency (not published to crates.io). StormLib itself is MIT-licensed
(see `deps/StormLib/LICENSE`); the Rust wrapper code has no license file in
the upstream repo.

## Use it

```toml
[dependencies]
stormlib = { git = "https://github.com/Tb1281/stormlib-rs", branch = "master" }
```

```rust
fn test_read_utf8() {
  let mut archive = Archive::open(
    "../../samples/中文.w3x",
    OpenArchiveFlags::MPQ_OPEN_NO_LISTFILE | OpenArchiveFlags::MPQ_OPEN_NO_ATTRIBUTES,
  )
  .unwrap();
  let mut f = archive.open_file("war3map.j").unwrap();
  assert_eq!(
    f.read_all().unwrap(),
    std::fs::read("../../samples/war3map.j").unwrap()
  );
}
```

## Building

Prerequisites: a C++ compiler and CMake. System zlib/bzip2 dev packages are
**not** needed — the build uses StormLib's bundled copies.

StormLib lives in a git submodule, so clone with submodules:

```sh
git clone --recurse-submodules https://github.com/Tb1281/stormlib-rs
# or, in an existing checkout:
git submodule update --init --recursive
```

To use a different StormLib checkout instead of the submodule, set
`STORMLIB_DIR` to its path. The pinned submodule commit is
`44362ca` (StormLib 9.25).

```sh
cargo test --all
```

## Layout

- `crates/stormlib` — safe wrapper (`Archive` / `File`, read-only for now)
- `crates/stormlib-sys` — raw FFI bindings + `build.rs` (builds StormLib via CMake)
- `crates/stormlib-bindgen` — regenerates the checked-in bindings in
  `crates/stormlib-sys/src/bindings_{linux,macos,windows}.rs` from
  `StormLib.h`; run it manually on each OS after a StormLib update, the
  bindings are not generated at build time
- `deps/StormLib` — StormLib submodule (upstream: ladislav-zezula/StormLib)

## Known limits

- Read-only API only: open / has_file / open_file / get_size / read_all.
  No archive creation, file adding, listing, or patching yet.
- `read_all` loads the whole file into memory; paths must be UTF-8 on
  non-Windows platforms.
- The Windows bindings file is much larger than the Linux/macOS ones
  (it pulls in Windows header types); that is pre-existing, not a bug
  introduced by this fork.

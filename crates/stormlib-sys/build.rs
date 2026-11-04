extern crate cmake;

use std::env;
use std::path::PathBuf;

fn main() {
  println!("cargo::rerun-if-env-changed=STORMLIB_DIR");

  // Gets StormLib source path from env STORMLIB_DIR, defaulting to the
  // git submodule at the workspace root (deps/StormLib). The path is
  // resolved against this crate's manifest dir, not the build cwd.
  let stormlib_path = match env::var("STORMLIB_DIR") {
    Ok(dir) => PathBuf::from(dir),
    Err(_) => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../deps/StormLib"),
  };

  // A git checkout without --recurse-submodules (or a plain archive
  // download, which never contains submodules) leaves this dir empty.
  // Fail with an actionable message instead of a cryptic cmake error.
  if !stormlib_path.join("CMakeLists.txt").is_file() {
    panic!(
      "StormLib sources not found at {}. \
       Clone with submodules (`git clone --recurse-submodules ...` or \
       `git submodule update --init --recursive`), or point STORMLIB_DIR \
       at a StormLib checkout.",
      stormlib_path.display()
    );
  }

  println!("cargo::rerun-if-changed={}", stormlib_path.display());

  // Builds StormLib using cmake
  let mut cfg = cmake::Config::new(&stormlib_path);

  #[cfg(target_os = "windows")]
  {
    cfg.cxxflag("-D UNICODE").cxxflag("-D _UNICODE");
  }

  #[cfg(target_os = "macos")]
  {
    cfg.cflag("-Wno-implicit-function-declaration");
  }

  let dst = cfg
    .define("BUILD_SHARED_LIBS", "OFF")
    // Use StormLib's bundled zlib/bzip2 instead of the system ones, so
    // consumers don't need zlib1g-dev / bzip2 dev packages installed.
    // (StormLib falls back to the bundled copies when the system libs
    // are missing anyway, but build.rs used to link -lz/-lbz2
    // unconditionally, which then failed at link time.)
    .define("STORM_USE_BUNDLED_LIBRARIES", "ON")
    .build();

  let lib = dst.join("lib");

  // For gcc should link static storm first! (before adding -l flags)
  // That's very important and I've wasted an hour finding it >_<
  // source: https://stackoverflow.com/questions/29199107/c-lz-library-link-order-undefined-reference-to-symbol-inflateinit2
  println!("cargo::rustc-link-search=native={}", lib.display());

  #[cfg(target_os = "windows")]
  {
    println!("cargo::rustc-link-lib=static=StormLib");
    println!("cargo::rustc-link-lib=user32");
  }

  #[cfg(target_os = "macos")]
  {
    println!("cargo::rustc-link-lib=static=storm");
    println!("cargo::rustc-link-lib=dylib=c++");
  }

  #[cfg(target_os = "linux")]
  {
    println!("cargo::rustc-link-lib=static=storm");
    println!("cargo::rustc-link-lib=stdc++");
  }
}

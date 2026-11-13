//! macOS: SDL3 is linked statically, and its Objective-C code uses
//! `@available`, which calls `__isPlatformVersionAtLeast` from clang's
//! runtime library. Rust links without that library, so add it here.

use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    let out = Command::new("clang").arg("--print-resource-dir").output().expect("run clang --print-resource-dir");
    let dir = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let lib = PathBuf::from(dir).join("lib/darwin/libclang_rt.osx.a");
    assert!(lib.exists(), "clang runtime not found at {}", lib.display());
    println!("cargo:rustc-link-arg={}", lib.display());
}

use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=tools/windows-version.rc.in");
    println!("cargo:rerun-if-changed=Cargo.toml");
    match env::var("CARGO_CFG_TARGET_OS").as_deref() {
        Ok("windows") => {}
        Ok("macos") => return link_clang_runtime(),
        _ => return,
    }

    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let script = out.join("version.rc");
    let numeric = format!("{},{},{},0", env!("CARGO_PKG_VERSION_MAJOR"),
        env!("CARGO_PKG_VERSION_MINOR"), env!("CARGO_PKG_VERSION_PATCH"));
    let resource = include_str!("tools/windows-version.rc.in")
        .replace("@NUMERIC_VERSION@", &numeric)
        .replace("@VERSION@", env!("CARGO_PKG_VERSION"))
        .replace("@AUTHOR@", env!("CARGO_PKG_AUTHORS"));
    fs::write(&script, resource).expect("write Windows version resource");

    let msvc = env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    let object = out.join(if msvc { "version.res" } else { "version.o" });
    let status = if msvc {
        Command::new("rc.exe").arg("/nologo").arg("/fo").arg(&object).arg(&script).status()
    } else {
        let compiler = if env::var("HOST") == env::var("TARGET") {
            "windres".to_owned()
        } else {
            format!("{}-w64-mingw32-windres", env::var("CARGO_CFG_TARGET_ARCH").unwrap())
        };
        Command::new(compiler).args(["-O", "coff", "-i"]).arg(&script)
            .arg("-o").arg(&object).status()
    }.expect("run Windows resource compiler");
    assert!(status.success(), "Windows resource compiler failed: {status}");
    println!("cargo:rustc-link-arg-bin=bounce={}", object.display());
}

/// macOS: SDL3 is linked statically, and its Objective-C code uses
/// `@available`, which calls `__isPlatformVersionAtLeast` from clang's
/// runtime library. Rust links without that library, so add it here.
fn link_clang_runtime() {
    let out = Command::new("clang").arg("--print-resource-dir").output().expect("run clang --print-resource-dir");
    let dir = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let lib = PathBuf::from(dir).join("lib/darwin/libclang_rt.osx.a");
    assert!(lib.exists(), "clang runtime not found at {}", lib.display());
    println!("cargo:rustc-link-arg={}", lib.display());
}

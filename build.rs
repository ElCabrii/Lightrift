use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=assets/lightrift.ico");
    println!("cargo:rerun-if-changed=packaging/windows.rc");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let gnu = env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu");
    let resource = out.join(if gnu { "windows.o" } else { "windows.res" });
    let compiler =
        env::var_os("RC").unwrap_or_else(|| if gnu { "windres" } else { "rc.exe" }.into());
    let mut command = Command::new(compiler);
    if gnu {
        command
            .args(["-I", ".", "-i", "packaging/windows.rc", "-O", "coff", "-o"])
            .arg(&resource);
    } else {
        command
            .args(["/nologo", "/I", ".", "/fo"])
            .arg(&resource)
            .arg("packaging/windows.rc");
    }
    let status = command
        .status()
        .expect("Windows resource compiler required: windres (GNU) or rc.exe (Windows SDK)");
    assert!(status.success(), "Windows resource compilation failed");
    println!("cargo:rustc-link-arg-bin=lightrift={}", resource.display());
}

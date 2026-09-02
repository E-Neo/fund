use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

const GUEST_MANIFEST: &str = "crates/fund-strategies/Cargo.toml";

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let guest_target = out_dir.join("guest-target");
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    let manifest = manifest_dir.join(GUEST_MANIFEST);
    rerun_if_changed(manifest.parent().unwrap());
    let status = Command::new(&cargo)
        .arg("build")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--target")
        .arg("wasm32-wasip2")
        .arg("--release")
        .arg("--target-dir")
        .arg(&guest_target)
        .status()
        .unwrap_or_else(|err| panic!("failed to run cargo for guest strategies: {err}"));
    if !status.success() {
        panic!("cargo build for guest strategies failed");
    }

    let artifact = guest_target
        .join("wasm32-wasip2")
        .join("release")
        .join("fund_strategies.wasm");
    let dest = out_dir.join("fund_strategies.wasm");
    std::fs::copy(&artifact, &dest)
        .unwrap_or_else(|err| panic!("failed to copy fund_strategies component: {err}"));
}

fn rerun_if_changed(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        if path.is_dir() {
            if name == "target" {
                continue;
            }
            rerun_if_changed(&path);
        } else {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
}

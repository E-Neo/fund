use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

const GUEST_MANIFEST: &str = "crates/fund-strategies/Cargo.toml";

fn main() {
    // When building for the wasm32 client, this build script is also invoked.
    // Skip everything to avoid recursion and because the client doesn't need
    // the embedded guest component.
    if env::var("TARGET").is_ok_and(|t| t.contains("wasm32")) {
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    build_strategy_guest(&manifest_dir, &out_dir, &cargo);
    if env::var("FUND_SKIP_CLIENT").is_ok_and(|v| v == "1") {
        return;
    }
    build_client_wasm(&manifest_dir, &out_dir, &cargo);
}

fn build_strategy_guest(manifest_dir: &Path, out_dir: &Path, cargo: &str) {
    let manifest = manifest_dir.join(GUEST_MANIFEST);
    rerun_if_changed(manifest.parent().unwrap());
    let guest_target = out_dir.join("guest-target");
    let status = Command::new(cargo)
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

fn build_client_wasm(manifest_dir: &Path, out_dir: &Path, cargo: &str) {
    let manifest = manifest_dir.join("Cargo.toml");
    let client_target = out_dir.join("client-target");
    let status = Command::new(cargo)
        .arg("build")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--target")
        .arg("wasm32-unknown-unknown")
        .arg("--release")
        .arg("--lib")
        .arg("--no-default-features")
        .arg("--features")
        .arg("hydrate")
        .arg("--target-dir")
        .arg(&client_target)
        .status()
        .unwrap_or_else(|err| panic!("failed to run cargo for client wasm: {err}"));
    if !status.success() {
        panic!("cargo build for client wasm failed");
    }

    let wasm = client_target
        .join("wasm32-unknown-unknown")
        .join("release")
        .join("fund.wasm");
    let pkg = out_dir.join("pkg");
    std::fs::create_dir_all(&pkg).unwrap();
    let status = Command::new("wasm-bindgen")
        .arg("--target")
        .arg("web")
        .arg("--out-dir")
        .arg(&pkg)
        .arg(&wasm)
        .status()
        .unwrap_or_else(|err| panic!("failed to run wasm-bindgen: {err}"));
    if !status.success() {
        panic!("wasm-bindgen failed");
    }
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

use std::env;

fn main() {
    println!("cargo:rerun-if-env-changed=HYDCRAFT_UPDATER_VERSION");
    println!("cargo:rerun-if-env-changed=HYDCRAFT_UPDATER_COMMIT");
    println!("cargo:rerun-if-env-changed=HYDCRAFT_UPDATER_PLATFORM");

    let version = env::var("HYDCRAFT_UPDATER_VERSION")
        .unwrap_or_else(|_| env::var("CARGO_PKG_VERSION").expect("Cargo package version is set"));
    let commit = env::var("HYDCRAFT_UPDATER_COMMIT").unwrap_or_else(|_| "local".into());
    let platform = env::var("HYDCRAFT_UPDATER_PLATFORM").unwrap_or_else(|_| {
        match env::var("TARGET").as_deref() {
            Ok("x86_64-pc-windows-msvc") => "windows-x86_64".into(),
            Ok("aarch64-apple-darwin")
            | Ok("x86_64-apple-darwin")
            | Ok("universal-apple-darwin") => "macos-universal".into(),
            Ok(target) => target.into(),
            Err(_) => "unknown".into(),
        }
    });

    println!("cargo:rustc-env=HYDCRAFT_UPDATER_VERSION={version}");
    println!("cargo:rustc-env=HYDCRAFT_UPDATER_COMMIT={commit}");
    println!("cargo:rustc-env=HYDCRAFT_UPDATER_PLATFORM={platform}");
    tauri_build::build()
}

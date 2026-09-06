fn main() {
    tauri_build::build();
    link_app_manifest_into_test_binaries();
}

fn link_app_manifest_into_test_binaries() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let out_dir = std::env::var("OUT_DIR").expect("cargo always sets OUT_DIR for build scripts");
    let resource_lib = std::path::Path::new(&out_dir).join("resource.lib");
    println!("cargo:rustc-link-arg-tests={}", resource_lib.display());
}

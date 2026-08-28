fn main() {
    tauri_build::build();
    link_app_manifest_into_test_binaries();
}

/// Gives `cargo test` binaries the Windows resource that `tauri_build::build()`
/// gives the app binary.
///
/// The application manifest is why this exists, but it is not all that is in
/// the resource: the same `resource.lib` also carries VERSIONINFO and
/// `icons/icon.ico` as icon id 32512 (`tauri-build-2.6.3/src/lib.rs:622-682`).
/// Five VERSIONINFO fields for this repository, not six, and four of the five
/// are conditional on `tauri.conf.json` rather than guaranteed:
/// `FileVersion` and `ProductVersion` need a `version` that `Version::parse`
/// accepts (`:632-639`), `ProductName` needs `productName` (`:642-644`), and
/// `LegalCopyright` needs `bundle.copyright` (`:664-666`) — which this
/// configuration does not set, which is why the count is five. Only
/// `CompanyName` (`:655`, it has a fallback) and `FileDescription` (`:662`) are
/// written whatever the config says. Drop `productName` or `version` and the
/// count is four; the number above is a measurement of this repository, not a
/// property of `tauri-build`. Linking it into test binaries
/// gives them those too. Harmless — nothing looks at a test binary's version
/// block or icon — but it is a whole resource, not a manifest, and a reader who
/// went looking for a way to link *only* the manifest would find there is none
/// short of building a second `.rc`.
///
/// `tauri_build::build()` compiles a Win32 resource containing the application
/// manifest into `$OUT_DIR/resource.lib`, then emits it as
/// `cargo:rustc-link-arg-bins`. `-bins`, not `-tests`: the manifest reaches
/// `aeroworship.exe` and reaches nothing else. That asymmetry is invisible
/// until a test builds a Tauri `App`, and then it is fatal at load time rather
/// than at assert time. The chain, traced end to end on tauri 2.11.5:
///
/// 1. The production feature `common-controls-v6` (deliberate, ADR-0013) makes
///    `muda` import `TaskDialogIndirect` statically, so it lands in the import
///    table of every binary in this package - test binaries included.
/// 2. `C:\Windows\System32\comctl32.dll` exports `SetWindowSubclass` but not
///    `TaskDialogIndirect`. That symbol lives only in the side-by-side
///    ComCtl32 v6 assembly, and the loader binds to v6 only for a binary whose
///    embedded manifest asks for it.
/// 3. A test binary has no resource directory at all, so it binds to v5, the
///    import cannot be resolved, and the process dies before `main` with
///    `STATUS_ENTRYPOINT_NOT_FOUND` (exit 127). An external `<exe>.manifest`
///    file does not help; modern Windows ignores it when the binary has none
///    embedded.
///
/// So the manifest has to be linked into test binaries too. The condition
/// mirrors the one inside `tauri_build`: it builds `resource.lib` when the
/// *target* triple is Windows, not when the host is, so this reads
/// `CARGO_CFG_TARGET_OS` rather than using `cfg!(windows)` - a build script is
/// compiled for the host and would answer the wrong question.
///
/// This covers test targets of this package only. It is not a general fix for
/// anything else that links `muda`.
fn link_app_manifest_into_test_binaries() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let out_dir = std::env::var("OUT_DIR").expect("cargo always sets OUT_DIR for build scripts");
    let resource_lib = std::path::Path::new(&out_dir).join("resource.lib");
    println!("cargo:rustc-link-arg-tests={}", resource_lib.display());
}

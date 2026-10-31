fn main() {
    tauri_build::build();

    // Tauri only embeds its Windows manifest in the binaries. A test executable that links Tauri
    // needs the same Common-Controls 6 dependency, or it fails to start with
    // STATUS_ENTRYPOINT_NOT_FOUND. The bindings test relies on this.
    #[cfg(windows)]
    {
        let manifest =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("windows/test.manifest");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-tests=/MANIFESTINPUT:{}",
            manifest.display()
        );
    }
}

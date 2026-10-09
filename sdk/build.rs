//! Generates `include/ntd_sdk.h` (via `cbindgen`) and
//! `bindings/csharp/NextTabletDriver.Sdk/NativeMethods.g.cs` (via
//! `csbindgen`) from the `extern "C"` functions and `#[repr(C)]` types in
//! `src/ffi.rs`. Both are written into the crate directory (not `OUT_DIR`)
//! because they're distributable parts of the SDK, alongside the compiled
//! `cdylib`/`staticlib`.

use std::env;
use std::path::Path;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let crate_dir = Path::new(&crate_dir);

    println!("cargo:rerun-if-changed=src/ffi.rs");
    println!("cargo:rerun-if-changed=cbindgen.toml");

    generate_c_header(crate_dir);
    generate_csharp_bindings(crate_dir);
}

fn generate_c_header(crate_dir: &Path) {
    let config = cbindgen::Config::from_root_or_default(crate_dir);

    match cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_config(config)
        .generate()
    {
        Ok(bindings) => {
            let include_dir = crate_dir.join("include");
            if let Err(e) = std::fs::create_dir_all(&include_dir) {
                println!("cargo:warning=failed to create sdk/include: {e}");
                return;
            }
            bindings.write_to_file(include_dir.join("ntd_sdk.h"));
        }
        Err(e) => {
            // A failed header generation shouldn't fail the whole build --
            // the header is a distributable convenience for C/C++ consumers,
            // not something the Rust build itself depends on.
            println!("cargo:warning=cbindgen failed to generate ntd_sdk.h: {e}");
        }
    }
}

fn generate_csharp_bindings(crate_dir: &Path) {
    let csharp_dir = crate_dir.join("bindings/csharp/NextTabletDriver.Sdk");
    if let Err(e) = std::fs::create_dir_all(&csharp_dir) {
        println!("cargo:warning=failed to create sdk/bindings/csharp/NextTabletDriver.Sdk: {e}");
        return;
    }

    let ffi_path = crate_dir.join("src/ffi.rs");
    if let Err(e) = csbindgen::Builder::default()
        .input_extern_file(ffi_path)
        .csharp_dll_name("ntd_sdk")
        .csharp_namespace("NextTabletDriver.Sdk")
        .csharp_class_name("NativeMethods")
        .csharp_class_accessibility("public")
        .generate_csharp_file(csharp_dir.join("NativeMethods.g.cs"))
    {
        // Same rationale as the C header: a failed generation shouldn't fail
        // the whole build, this is a distributable convenience for .NET
        // consumers (Unity in particular).
        println!("cargo:warning=csbindgen failed to generate NativeMethods.g.cs: {e}");
    }
}

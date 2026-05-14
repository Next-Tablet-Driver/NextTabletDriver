//! # Build Script
//!
//! This build script handles Windows-specific resource compilation.
//! It embeds metadata such as icons and file information directly into the
//! final executable during the compilation process.

/// Configures and compiles Windows resources (icon, metadata).
///
/// # Platform Specifics
/// This function only executes when the target OS is Windows. It uses the `winres`
/// crate to attach a resource file to the binary.
///
/// # Metadata Embedded
/// * **Icon**: Located at `resources/icon.ico`.
/// * **Product Name**: `NextTabletDriver`.
/// * **Company**: iSweat.
///
/// # Technical Note
/// The `winres` crate interacts with the Windows SDK (specifically `rc.exe` or `windres.exe`)
/// to compile the `.rc` file into a COFF object that the Rust linker can include.
#[cfg(windows)]
fn main() {
    // Only compile resources if we are targeting Windows
    let mut res = winres::WindowsResource::new();

    // Path to the application icon (visible in File Explorer and Taskbar)
    res.set_icon("resources/icon.ico");

    // Metadata visible in File Properties -> Details
    res.set("ProductName", "NextTabletDriver");
    res.set("FileDescription", "Next Tablet Driver");
    res.set("CompanyName", "iSweat");

    // Compiles the resource. Fails if the Windows SDK tools are missing.
    if let Err(e) = res.compile() {
        eprintln!(
            "Erreur lors de la compilation des ressources Windows : {e}"
        );
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn main() {
    // Linux or MacOS
}

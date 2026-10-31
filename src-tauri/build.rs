fn main() {
    // The icon is embedded in the executable, which is where the taskbar and the file explorer
    // read it. Cargo does not watch it on its own, so a new icon would not reach the executable.
    println!("cargo:rerun-if-changed=icons");
    println!("cargo:rerun-if-changed=tauri.conf.json");
    tauri_build::build();
}

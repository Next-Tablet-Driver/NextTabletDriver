//! The committed TypeScript bindings must be exactly what the Rust commands, events and types
//! generate (roadmap P0.9). CI runs this test, so a command changed without regenerating the
//! bindings fails the build.
//!
//! After changing a command, an event or a type the frontend sees, regenerate the file with
//! `UPDATE_BINDINGS=1 cargo test -p app --test bindings`.

#![allow(clippy::expect_used, reason = "a test that panics is a failing test")]

/// Where the generated bindings are committed.
const BINDINGS: &str = "../../frontend/src/lib/infrastructure/tauri/bindings.ts";

#[test]
fn bindings_are_up_to_date() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(BINDINGS);
    let fresh = std::env::temp_dir().join(format!("ntd-bindings-{}.ts", std::process::id()));
    app_lib::specta_builder()
        .export(specta_typescript::Typescript::default(), &fresh)
        .expect("the bindings can be generated");
    let generated = std::fs::read_to_string(&fresh).expect("the generated file is readable");
    let _ = std::fs::remove_file(&fresh);

    if std::env::var_os("UPDATE_BINDINGS").is_some() {
        std::fs::write(&path, &generated).expect("the bindings can be written");
        return;
    }
    let committed = std::fs::read_to_string(&path).unwrap_or_default();
    // Git may convert line endings on checkout; the content is what matters.
    assert_eq!(
        committed.replace("\r\n", "\n"),
        generated.replace("\r\n", "\n"),
        "{BINDINGS} is out of date; regenerate it with UPDATE_BINDINGS=1"
    );
}

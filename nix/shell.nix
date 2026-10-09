{ pkgs ? import <nixpkgs> {}
, rustPlatform ? pkgs.rustPlatform
  # Full toolchain (rustc, cargo, clippy, rustfmt, rust-src) matching the package build.
, devToolchain ? null
}:

let
  package = import ./default.nix { inherit pkgs rustPlatform; };
in
pkgs.mkShell {
  inputsFrom = [ package ];

  buildInputs = with pkgs; [
    cargo-tauri
    nodejs
  ] ++ (if devToolchain != null then [ devToolchain ] else [
    cargo
    rustc
    rustfmt
    rustPackages.clippy
  ]);

  RUST_SRC_PATH =
    if devToolchain != null
    then "${devToolchain}/lib/rustlib/src/rust/library"
    else "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
}

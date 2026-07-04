{ pkgs ? import <nixpkgs> {} }:

let
  package = import ./default.nix { inherit pkgs; };
in
pkgs.mkShell {
  inputsFrom = [ package ];

  buildInputs = with pkgs; [
    cargo
    rustc
    rustfmt
    rustPackages.clippy
  ];

  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
}

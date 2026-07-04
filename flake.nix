{
  description = "Tablet Driver for Osu! and Drawing";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };
      in
      {
        packages.default = import ./nix/default.nix { inherit pkgs; };
        
        devShells.default = import ./nix/shell.nix { inherit pkgs; };
      }
    );
}

{
  description = "Tablet Driver for Osu! and Drawing";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, home-manager }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };
      in
      {
        # Only Windows and Linux are supported targets; systemd (libudev) and
        # the X11 tooling this package depends on aren't available on Darwin.
        packages = pkgs.lib.optionalAttrs pkgs.stdenv.isLinux {
          default = import ./nix/default.nix { inherit pkgs; };
        };

        devShells = pkgs.lib.optionalAttrs pkgs.stdenv.isLinux {
          default = import ./nix/shell.nix { inherit pkgs; };
        };

        # Exercise the NixOS/home-manager modules against a throwaway package
        # (pkgs.hello, already in the binary cache) instead of the real driver
        # build, so CI catches module wiring bugs (bad option types, eval
        # errors on defaults, ...) without paying for a full Rust rebuild.
        checks = pkgs.lib.optionalAttrs pkgs.stdenv.isLinux {
          nixos-module = (nixpkgs.lib.nixosSystem {
            inherit system;
            modules = [
              self.nixosModules.default
              {
                services.nexttabletdriver = {
                  enable = true;
                  user = "testuser";
                  package = pkgs.hello;
                };
                users.users.testuser.isNormalUser = true;
                boot.isContainer = true;
                system.stateVersion = "24.11";
                fileSystems."/" = {
                  device = "/dev/null";
                  fsType = "tmpfs";
                };
              }
            ];
          }).config.system.build.toplevel;

          home-manager-module = (home-manager.lib.homeManagerConfiguration {
            inherit pkgs;
            modules = [
              self.homeManagerModules.default
              {
                home.username = "testuser";
                home.homeDirectory = "/home/testuser";
                home.stateVersion = "24.11";
                services.nexttabletdriver = {
                  enable = true;
                  package = pkgs.hello;
                };
              }
            ];
          }).activationPackage;
        };
      }
    ) // {
      nixosModules.default = import ./nix/nixos-module.nix self;

      homeManagerModules.default = import ./nix/home-manager-module.nix self;
    };
}

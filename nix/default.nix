{ pkgs ? import <nixpkgs> {} }:

pkgs.rustPlatform.buildRustPackage rec {
  pname = "next-tablet-driver";
  version = "1.26.0407.00"; # Version from lib.rs

  src = ../.;

  cargoLock = {
    lockFile = ../Cargo.lock;
  };

  nativeBuildInputs = with pkgs; [
    pkg-config
    wrapGAppsHook
  ];

  buildInputs = with pkgs; [
    gtk3
    glib
    xdotool
    systemd # provides libudev
    libusb1
    libxkbcommon
  ];

  meta = with pkgs.lib; {
    description = "Tablet Driver for Osu! and Drawing";
    homepage = "https://github.com/Next-Tablet-Driver/NextTabletDriver";
    license = licenses.mit;
    mainProgram = "next_tablet_driver";
  };
}

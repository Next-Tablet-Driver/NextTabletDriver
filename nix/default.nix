{ pkgs ? import <nixpkgs> {} }:

pkgs.rustPlatform.buildRustPackage rec {
  pname = "next-tablet-driver";
  version = "1.26.1708.00"; # Version from lib.rs

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

  postInstall = ''
    install -Dm644 ${../scripts/99-nexttabletdriver.rules} \
      $out/lib/udev/rules.d/99-nexttabletdriver.rules
  '';

  meta = with pkgs.lib; {
    description = "Tablet Driver for Osu! and Drawing";
    homepage = "https://github.com/Next-Tablet-Driver/NextTabletDriver";
    license = licenses.mit;
    mainProgram = "next_tablet_driver";
  };
}

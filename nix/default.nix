{ pkgs ? import <nixpkgs> {}
, rustPlatform ? pkgs.rustPlatform
}:

rustPlatform.buildRustPackage {
  pname = "next-tablet-driver";
  version = "2.0.0";

  src = ../.;

  # The Tauri app lives in src-tauri/ (it pulls the core crate in by path).
  cargoRoot = "src-tauri";
  buildAndTestSubdir = "src-tauri";
  cargoLock.lockFile = ../src-tauri/Cargo.lock;

  # Svelte frontend, built by vite and embedded into the binary by Tauri.
  npmRoot = "frontend";
  npmDeps = pkgs.fetchNpmDeps {
    src = ../frontend;
    # Update this hash whenever frontend/package-lock.json changes: `nix build .#default`
    # fails with the expected hash, paste it here.
    hash = "sha256-yRTdFSGC4PAow5Rv0vh0QWiFUdkNOg1fxD6kMWu1OPo=";
  };

  nativeBuildInputs = with pkgs; [
    cargo-tauri.hook
    nodejs
    npmHooks.npmConfigHook
    pkg-config
    wrapGAppsHook3
  ];

  buildInputs = with pkgs; [
    webkitgtk_4_1
    gtk3
    glib
    glib-networking
    libayatana-appindicator
    openssl
    xdotool
    systemd # provides libudev
    libusb1
  ];

  # The updater artifacts need a signing key, which a Nix build does not have.
  tauriBuildFlags = [ "--config" ''{"bundle":{"createUpdaterArtifacts":false}}'' ];

  postInstall = ''
    install -Dm644 ${../scripts/99-nexttabletdriver.rules} \
      $out/lib/udev/rules.d/99-nexttabletdriver.rules

    # The NixOS / home-manager modules start `next_tablet_driver`.
    for name in NextTabletDriver app; do
      if [ -e "$out/bin/$name" ] && [ ! -e "$out/bin/next_tablet_driver" ]; then
        ln -s "$name" "$out/bin/next_tablet_driver"
      fi
    done
  '';

  meta = with pkgs.lib; {
    description = "Tablet Driver for Osu! and Drawing";
    homepage = "https://github.com/Next-Tablet-Driver/NextTabletDriver";
    license = licenses.mit;
    mainProgram = "next_tablet_driver";
  };
}

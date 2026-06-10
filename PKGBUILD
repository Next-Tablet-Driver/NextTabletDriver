pkgname=nexttabletdriver-git
pkgver=1.1.0
pkgrel=1
pkgdesc="Tablet Driver for Osu! and Drawing"
arch=('x86_64')
url="https://github.com/Next-Tablet-Driver/NextTabletDriver"
license=('MIT')
depends=('gtk3' 'hidapi' 'libxkbcommon' 'libglvnd')
makedepends=('cargo' 'pkgconf' 'git' 'jq')
provides=("nexttabletdriver")
conflicts=("nexttabletdriver" "nexttabletdriver-bin")
source=("git+https://github.com/Next-Tablet-Driver/NextTabletDriver.git")
md5sums=('SKIP')

pkgver() {
  cd "$srcdir/NextTabletDriver"
  local _ver="$(git describe --long --tags 2>/dev/null)"
  if [ -n "$_ver" ]; then
    echo "$_ver" | sed 's/\([^-]*-g\)/r\1/;s/-/./g'
  else
    printf "1.1.0.r%s.g%s" "$(git rev-list --count HEAD)" "$(git rev-parse --short HEAD)"
  fi
}

build() {
  cd "$srcdir/NextTabletDriver"
  export RUSTUP_TOOLCHAIN=stable
  export CARGO_TARGET_DIR=target
  cargo build --release --locked --target x86_64-unknown-linux-gnu
  
  # Generate udev rules
  bash scripts/generate_udev_rules.sh
}

package() {
  cd "$srcdir/NextTabletDriver"
  
  # Install main executable
  install -Dm755 "target/x86_64-unknown-linux-gnu/release/next_tablet_driver" "$pkgdir/usr/bin/next_tablet_driver"
  
  # Install udev rules
  install -Dm644 "scripts/99-nexttabletdriver.rules" "$pkgdir/usr/lib/udev/rules.d/99-nexttabletdriver.rules"
  
  # Install icon
  install -Dm644 "resources/icon.png" "$pkgdir/usr/share/pixmaps/nexttabletdriver.png"
  
  # Install license
  install -Dm644 "LICENSE" "$pkgdir/usr/share/licenses/$pkgname/LICENSE"

  # Create desktop entry
  install -dm755 "$pkgdir/usr/share/applications"
  cat <<EOF > "$pkgdir/usr/share/applications/nexttabletdriver.desktop"
[Desktop Entry]
Name=NextTabletDriver
Comment=Tablet Driver for Osu! and Drawing
Exec=next_tablet_driver
Icon=nexttabletdriver
Terminal=false
Type=Application
Categories=Utility;HardwareSettings;
EOF
  chmod 644 "$pkgdir/usr/share/applications/nexttabletdriver.desktop"
}

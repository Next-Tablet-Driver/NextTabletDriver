# Maintainer: iSweat <https://github.com/iSweat-exe>

pkgname=nexttabletdriver-git
pkgver=1.1.0
pkgrel=1
pkgdesc="Low-latency tablet driver for osu! and drawing"
arch=('x86_64')
url="https://github.com/Next-Tablet-Driver/NextTabletDriver"
license=('MIT')
depends=('gcc-libs' 'glibc' 'gtk3' 'hidapi' 'libglvnd' 'libxkbcommon' 'wayland')
makedepends=('cargo' 'git' 'jq' 'pkgconf')
provides=('nexttabletdriver')
conflicts=('nexttabletdriver' 'nexttabletdriver-bin')
source=('nexttabletdriver::git+https://github.com/Next-Tablet-Driver/NextTabletDriver.git')
sha256sums=('SKIP')

pkgver() {
  cd nexttabletdriver

  local ver
  ver="$(git describe --long --tags --abbrev=7 2>/dev/null)"

  if [[ -n "${ver}" ]]; then
    printf '%s' "${ver}" | sed 's/^v//;s/\([^-]*-g\)/r\1/;s/-/./g'
  else
    printf '1.1.0.r%s.g%s' "$(git rev-list --count HEAD)" "$(git rev-parse --short=7 HEAD)"
  fi
}

prepare() {
  cd nexttabletdriver

  cargo fetch --locked --target "${CARCH}-unknown-linux-gnu"
  bash scripts/generate_udev_rules.sh
}

build() {
  cd nexttabletdriver

  export CARGO_TARGET_DIR=target
  cargo build --frozen --release --target "${CARCH}-unknown-linux-gnu"
}

package() {
  cd nexttabletdriver

  install -Dm755 "target/${CARCH}-unknown-linux-gnu/release/next_tablet_driver" \
    "${pkgdir}/usr/bin/next_tablet_driver"

  install -Dm644 scripts/99-nexttabletdriver.rules \
    "${pkgdir}/usr/lib/udev/rules.d/99-nexttabletdriver.rules"

  install -Dm644 resources/icon.png \
    "${pkgdir}/usr/share/pixmaps/nexttabletdriver.png"

  install -Dm644 LICENSE \
    "${pkgdir}/usr/share/licenses/${pkgname}/LICENSE"

  install -Dm644 /dev/stdin "${pkgdir}/usr/share/applications/nexttabletdriver.desktop" <<'EOF'
[Desktop Entry]
Name=NextTabletDriver
Comment=Low-latency tablet driver for osu! and drawing
Exec=next_tablet_driver
Icon=nexttabletdriver
Terminal=false
Type=Application
Categories=Utility;HardwareSettings;
Keywords=tablet;driver;osu;drawing;pen;
EOF
}

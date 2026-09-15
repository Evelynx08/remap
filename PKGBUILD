pkgname=bookos-teclas
pkgver=0.1.0
pkgrel=1
pkgdesc="Remapeo de teclas para el escritorio BookOS"
arch=('x86_64')
license=('MIT')
depends=('webkit2gtk-4.1' 'gtk3' 'libsoup3')
makedepends=('rust' 'cargo' 'pkgconf' 'base-devel')
source=()
options=('!strip' '!debug')

build() {
  cd "$startdir/src-tauri"
  cargo build --release --locked
}

package() {
  install -Dm755 "$startdir/src-tauri/target/release/bookos-teclas" \
    "$pkgdir/usr/bin/bookos-teclas"

  install -Dm644 "$startdir/bookos-teclas.desktop" \
    "$pkgdir/usr/share/applications/bookos-teclas.desktop"

  install -Dm644 "$startdir/src-tauri/icons/icon.png" \
    "$pkgdir/usr/share/icons/hicolor/512x512/apps/bookos-teclas.png"
  install -Dm644 "$startdir/src-tauri/icons/icon.svg" \
    "$pkgdir/usr/share/icons/hicolor/scalable/apps/bookos-teclas.svg"
}

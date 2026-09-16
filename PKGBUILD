pkgname=bookos-teclas
pkgver=0.1.0
pkgrel=1
pkgdesc="Remapeo de teclas para el escritorio BookOS"
arch=('x86_64')
license=('MIT')
depends=('webkit2gtk-4.1' 'gtk3' 'libsoup3' 'polkit' 'systemd')
makedepends=('rust' 'cargo' 'pkgconf' 'base-devel')
install=bookos-teclas.install
source=()
options=('!strip' '!debug')

build() {
  cd "$startdir/src-tauri"
  cargo build --release --locked
  cd "$startdir/teclasd"
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

  # El servicio que aplica los remapeos fuera de la sesión BookOS.
  install -Dm755 "$startdir/teclasd/target/release/bookos-teclasd" \
    "$pkgdir/usr/bin/bookos-teclasd"
  install -Dm644 "$startdir/teclasd/sistema/bookos-teclasd.service" \
    "$pkgdir/usr/lib/systemd/system/bookos-teclasd.service"
  install -Dm644 "$startdir/teclasd/sistema/org.bookos.Teclas.conf" \
    "$pkgdir/usr/share/dbus-1/system.d/org.bookos.Teclas.conf"
  install -Dm644 "$startdir/teclasd/sistema/org.bookos.Teclas.service" \
    "$pkgdir/usr/share/dbus-1/system-services/org.bookos.Teclas.service"
  install -Dm644 "$startdir/teclasd/sistema/org.bookos.teclas.policy" \
    "$pkgdir/usr/share/polkit-1/actions/org.bookos.teclas.policy"
  install -Dm644 "$startdir/teclasd/sistema/bookos-teclasd.quirks" \
    "$pkgdir/usr/share/libinput/50-bookos-teclasd.quirks"
}

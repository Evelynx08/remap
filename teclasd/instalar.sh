#!/bin/sh
# Instala bookos-teclasd en sistemas sin PKGBUILD (Fedora). Como root, después
# de `cargo build --release` en este directorio.
set -eu
cd "$(dirname "$0")"

install -Dm755 target/release/bookos-teclasd /usr/bin/bookos-teclasd
install -Dm644 sistema/bookos-teclasd.service /usr/lib/systemd/system/bookos-teclasd.service
install -Dm644 sistema/org.bookos.Teclas.conf /usr/share/dbus-1/system.d/org.bookos.Teclas.conf
install -Dm644 sistema/org.bookos.Teclas.service /usr/share/dbus-1/system-services/org.bookos.Teclas.service
install -Dm644 sistema/org.bookos.teclas.policy /usr/share/polkit-1/actions/org.bookos.teclas.policy
install -Dm644 sistema/bookos-teclasd.quirks /usr/share/libinput/50-bookos-teclasd.quirks

# Con SELinux, que cada fichero lleve la etiqueta de su ruta y no la de donde
# se compiló.
if command -v restorecon >/dev/null; then
  restorecon -F /usr/bin/bookos-teclasd /usr/lib/systemd/system/bookos-teclasd.service \
    /usr/share/dbus-1/system.d/org.bookos.Teclas.conf \
    /usr/share/dbus-1/system-services/org.bookos.Teclas.service \
    /usr/share/polkit-1/actions/org.bookos.teclas.policy \
    /usr/share/libinput/50-bookos-teclasd.quirks
fi

systemctl daemon-reload
# El bus no deja publicar `org.bookos.Teclas` hasta que relee su política.
systemctl reload dbus
systemctl enable --now bookos-teclasd.service

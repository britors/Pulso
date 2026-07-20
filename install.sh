#!/usr/bin/env bash
set -euo pipefail
prefix="${PREFIX:-$HOME/.local}"
install -Dm755 target/release/pulso "$prefix/bin/pulso"
install -Dm644 data/br.com.w3ti.Pulso.desktop "$prefix/share/applications/br.com.w3ti.Pulso.desktop"
install -Dm644 data/icons/hicolor/scalable/apps/br.com.w3ti.Pulso.svg "$prefix/share/icons/hicolor/scalable/apps/br.com.w3ti.Pulso.svg"
install -Dm644 data/glib-2.0/schemas/br.com.w3ti.Pulso.gschema.xml "$prefix/share/glib-2.0/schemas/br.com.w3ti.Pulso.gschema.xml"
install -d "$prefix/share/locale/pt_BR/LC_MESSAGES"
msgfmt po/pt_BR.po -o "$prefix/share/locale/pt_BR/LC_MESSAGES/pulso.mo"
glib-compile-schemas "$prefix/share/glib-2.0/schemas"

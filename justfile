#!/usr/bin/env just --justfile

install:
    cargo build --release
    mkdir -p ~/.local/share/pop-launcher/plugins/stackoverflow
    install -Dm0755 target/release/pop-launcher-stackoverflow ~/.local/share/pop-launcher/plugins/stackoverflow/pop-launcher-stackoverflow
    install -Dm644 plugin.ron ~/.local/share/pop-launcher/plugins/stackoverflow/plugin.ron
    sudo install -Dm644 LogoGlyph.svg /usr/share/pixmaps/stackoverflow.svg
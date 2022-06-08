#!/usr/bin/env just --justfile

install:
    cargo build --release
    mkdir -p ~/.local/share/pop-launcher/plugins/stackoverflow
    install -Dm0755 target/release/pop-launcher-stackoverflow ~/.local/share/pop-launcher/plugins/stackoverflow/pop-launcher-stackoverflow
    install -Dm644 plugin.ron ~/.local/share/pop-launcher/plugins/stackoverflow/plugin.ron
    sudo install -Dm644 LogoGlyph.svg /usr/share/pixmaps/stackoverflow.svg
    echo "Getting an access token from stackoverflow, please copy the access_token in url in your plugin config"
    xdg-open "https://stackoverflow.com/oauth/dialog?client_id=23582&scope=no_expiry&redirect_uri=https://stackoverflow.com/oauth/login_success"
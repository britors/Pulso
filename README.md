# Pulso

Pulso é um editor desktop de apresentações em Rust, GTK4 e libadwaita. O formato nativo `.pulso` é autocontido e a exportação PDF usa o mesmo renderizador cairo do editor.

## Desenvolvimento

Requer Rust stable, GTK 4.18+, libadwaita 1.7+ e gettext.

```sh
cargo run
cargo test
cargo clippy -- -D warnings
```

Para instalar localmente:

```sh
cargo build --release
./install.sh
```

Licença: GPLv3 ou posterior.

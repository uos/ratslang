<img src="https://codeberg.org/stelzo/ratslang/raw/branch/main/docs/docs/images/ratslang.svg" alt="Ratslang logo" width="160" align="right">

# Ratslang

A compact configuration language for physical systems, with typed units,
readable defaults, and Rust integration.

```awk
sensor {
  distance = 25cm
  timeout = 500ms
}
backend = CPU
```

Ratslang supports namespaces, includes, references, arrays, ranges, and physical
quantities. Comments can describe settings and declare constraints. Ratslang also
provides serde integration, source-preserving edits, a configuration CLI, and an
LSP.

## Getting started

```toml
[dependencies]
ratslang = "0.5.0"
```

```rust
let config = ratslang::compile_code("sensor.distance = 25cm\n")?;
let distance = config.vars.resolve("sensor.distance")?;
```

See [getting started](https://ratslang.steado.tech/getting-started/) for serde configuration and
defaults. The core compiler requires Rust 1.85.1 or newer. The LSP requires Rust
1.95 or newer.

## Documentation

| Guide | Covers |
| --- | --- |
| [Language](https://ratslang.steado.tech/language/values/) | Values, assignments, arrays, ranges, and references |
| [Units](https://ratslang.steado.tech/language/units/) | Time, length, mass, angle, frequency, velocity, and acceleration |
| [Schemas](https://ratslang.steado.tech/schemas/) | Enums, bounds, required settings, and deprecations |
| [Serde](https://ratslang.steado.tech/rust/serde/) | Derived Rust config types and typed uom quantities |
| [Editing](https://ratslang.steado.tech/rust/document-editing/) | Preserve comments, includes, and original source |
| [Compiler](https://ratslang.steado.tech/rust/compiler/) | Resolved values, macros, and provenance |
| [API reference](https://docs.rs/ratslang) | Published Rust APIs |

[Documentation source](https://codeberg.org/stelzo/ratslang/src/branch/main/docs/docs) ·
[Website](https://ratslang.steado.tech/) ·
[Build the documentation](https://codeberg.org/stelzo/ratslang/src/branch/main/docs/README.md)

## Editor support

The LSP uses local or HTTPS defaults for completion, hover docs, diagnostics,
definitions, and fixes. See the [editor guide](https://ratslang.steado.tech/editors/).

### Install the server

```sh
cargo install ratslang-lsp --locked
```

Ensure `~/.cargo/bin` is on the editor's PATH.

### Helix

Follow the [Helix setup](https://ratslang.steado.tech/editors/helix/) for LSP and tree-sitter.

### Neovim

Follow the [Neovim 0.11+ setup](https://ratslang.steado.tech/editors/neovim/) for the native client.

### VS Code

Follow the [VS Code setup](https://ratslang.steado.tech/editors/vscode/) to install the extension.

## Command-line tools

```sh
cargo install ratslang-cli --locked
```

Use `check`, `fmt`, `explain`, `generate`, `minimize`, and `migrate` to inspect and
edit configuration. See the [CLI guide](https://ratslang.steado.tech/tools/cli/).

## License

MIT OR Apache-2.0. See [LICENSE-MIT](https://codeberg.org/stelzo/ratslang/src/branch/main/LICENSE-MIT) and
[LICENSE-APACHE](https://codeberg.org/stelzo/ratslang/src/branch/main/LICENSE-APACHE).

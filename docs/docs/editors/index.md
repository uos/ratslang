# Editor setup

The LSP provides completion, hover documentation, diagnostics, document symbols,
go-to-definition, and fixes based on the same compiler and schema rules used by
the application.

## Install the server

The packages contain the server and the [command-line tools](../tools/cli.md).

=== "Ubuntu"

    Add the steado PPA once, then install the package:

    ```sh
    curl -fsSL "https://ppa.steado.tech/ubuntu/key.gpg" | gpg --dearmor \
      | sudo tee /usr/share/keyrings/steado-archive-keyring.gpg >/dev/null
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/steado-archive-keyring.gpg] https://ppa.steado.tech/ubuntu $(. /etc/os-release && echo $UBUNTU_CODENAME) main" \
      | sudo tee /etc/apt/sources.list.d/steado.list
    sudo apt update
    sudo apt install ratslang
    ```

=== "Arch Linux"

    ```sh
    yay -S ratslang-bin
    ```

    `ratslang` builds from source instead.

=== "macOS"

    ```sh
    brew install uos/tools/ratslang
    ```

=== "Python"

    ```sh
    uv tool install ratslang-lsp
    ```

    `pipx install ratslang-lsp` and `pip install ratslang-lsp` work too.

=== "Cargo"

    [cargo-binstall](https://github.com/cargo-bins/cargo-binstall) downloads the
    release binary instead of compiling:

    ```sh
    cargo binstall ratslang-lsp
    ```

    Install Rust 1.95 or newer with Cargo and a C compiler (needed for tree-sitter)
    to compile it:

    ```sh
    cargo install ratslang-lsp --locked
    ```

    Cargo installs `ratslang-lsp` into `~/.cargo/bin`. Put that directory on PATH
    for your editor, or use the executable's absolute path in the settings below.
    On Windows the executable is `ratslang-lsp.exe` in `%USERPROFILE%\.cargo\bin`.
    To update, run `cargo install ratslang-lsp --locked --force`.

The server communicates over stdio and needs no arguments. Logs go to stderr.
After an update, restart your editor's language server.

## Choose an editor

- [Helix](helix.md): native LSP support and the tree-sitter grammar.
- [Neovim 0.11+](neovim.md): native LSP and optional native tree-sitter support.
- [VS Code](vscode.md): the Ratslang extension starts the separately installed server.

## Select a schema

```ratslang
# @schema ./default.rl
rat.mood = SNEAKY
```

Local and HTTP(S) defaults provide property names, enum choices, docs, and
constraints. See [schemas](../schemas.md) for path resolution and limitations.

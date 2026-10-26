# Ratslang for VS Code

Syntax highlighting, completion, hover docs, diagnostics and quick fixes for
[Ratslang](https://stelzo.codeberg.page/ratslang/) `.rl` configuration files.

```rl
# @schema ./default.rl
rat.mood = HANGRY
rat.scurry_speed = 5km/h
```

With a `# @schema` line, completion and hover come from your project's
defaults, and values outside their bounds are marked as you type.

## Language server

The extension starts `ratslang-lsp`, which is installed separately. Install
Rust 1.95 or newer and a C compiler, then:

```sh
cargo install ratslang-lsp --locked
```

If VS Code does not find it on PATH, set the path in `settings.json`:

```json
{
  "ratslang.serverPath": "/home/you/.cargo/bin/ratslang-lsp"
}
```

Startup errors appear in **View → Output → Ratslang**. For Remote SSH, WSL or
containers, install the server where the extension runs.

## More

The [editor guide](https://stelzo.codeberg.page/ratslang/editors/vscode/) covers
setup in detail, and the [schema guide](https://stelzo.codeberg.page/ratslang/schemas/)
covers the rules a defaults file can carry.

To work on the extension, run `npm ci` in `vscode/` and press F5 for an
Extension Development Host. `npm test` runs the activation checks.

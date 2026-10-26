# VS Code

Install the server using the [editor overview](index.md#install-the-server),
then the **Ratslang** extension (`stelzo.ratslang`) for VS Code 1.105 or newer:

=== "VS Code"

    ```sh
    code --install-extension stelzo.ratslang
    ```

    It is on the [Visual Studio Marketplace](https://marketplace.visualstudio.com/items?itemName=stelzo.ratslang).

=== "VSCodium"

    ```sh
    codium --install-extension stelzo.ratslang
    ```

    It is on [Open VSX](https://open-vsx.org/extension/stelzo/ratslang), the
    extension registry VSCodium uses.

=== "VSIX file"

    Every [release](https://codeberg.org/stelzo/ratslang/releases) carries the
    extension as a `.vsix` file:

    ```sh
    code --install-extension ratslang.vsix
    ```

If VS Code cannot find the server on PATH, set its absolute executable path in
user `settings.json` (replace the example username):

```json
{
  "ratslang.serverPath": "/home/your-user/.cargo/bin/ratslang-lsp"
}
```

Run **Developer: Reload Window**, then open a `.rl` file. Its language mode should
be **Ratslang**. Use `Ctrl-Space` for completion, hover over a setting for its docs,
and use F12 for go-to-definition. Startup errors appear in **View → Output →
Ratslang**. Reload the window after changing `ratslang.serverPath` or updating the
server binary. For Remote SSH, WSL, or containers, install the extension and
server in the environment where the extension runs and use its executable path.
The server binary is installed separately. The extension does not bundle it.

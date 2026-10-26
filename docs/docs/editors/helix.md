# Helix

Install the server using the [editor overview](index.md#install-the-server).

Add the following to `~/.config/helix/languages.toml`. If Ratslang is already
configured, update its existing entries rather than adding duplicate ones.

```toml
[language-server.ratslang-lsp]
command = "ratslang-lsp"

[[language]]
name = "ratslang"
scope = "source.ratslang"
file-types = ["rl", "ratslang"]
roots = []
comment-token = "#"
indent = { tab-width = 2, unit = "  " }
language-servers = ["ratslang-lsp"]
```

Restart Helix and open a `.rl` file. Run `hx --health ratslang` in a terminal to
check that the executable is found. Use `Space k` for hover documentation and
`Ctrl-x` in insert mode for completion. After replacing the server binary, run
`:lsp-restart` in Helix.

For syntax highlighting, also add a grammar entry to `languages.toml`:

```toml
[[grammar]]
name = "ratslang"
source = { git = "https://codeberg.org/stelzo/tree-sitter-ratslang", rev = "6869b2e67f7d9fc4ccc26cdf9782b603eb770216" }
```

This pins the grammar shipped in `tree-sitter-ratslang` 0.2.0 (parser ABI 15).
Your Helix build must support ABI 15. Copy the matching queries and build the
parser:

```sh
git clone https://codeberg.org/stelzo/tree-sitter-ratslang.git
git -C tree-sitter-ratslang checkout 6869b2e67f7d9fc4ccc26cdf9782b603eb770216
mkdir -p ~/.config/helix/runtime/queries/ratslang
cp tree-sitter-ratslang/queries/*.scm ~/.config/helix/runtime/queries/ratslang/
hx --grammar fetch
hx --grammar build
hx --health ratslang
```

For local development, replace `source` with
`source = { path = "/absolute/path/to/tree-sitter-ratslang" }` and copy queries
from that checkout. After grammar updates, copy the matching queries again,
fetch/build, and restart Helix.

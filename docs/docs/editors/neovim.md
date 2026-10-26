# Neovim

Install the server using the [editor overview](index.md#install-the-server).

Neovim 0.11 or newer has the required LSP APIs built in. No LSP plugin is needed.
Add this to `~/.config/nvim/init.lua` or a Lua module loaded by it:

```lua
vim.filetype.add({ extension = { rl = 'ratslang', ratslang = 'ratslang' } })

vim.lsp.config('ratslang', {
  cmd = { 'ratslang-lsp' },
  filetypes = { 'ratslang' },
  -- Also start for standalone override files outside a Git repository.
  root_dir = function(bufnr, on_dir)
    local path = vim.api.nvim_buf_get_name(bufnr)
    on_dir(path ~= '' and vim.fs.dirname(path) or vim.uv.cwd())
  end,
  on_attach = function(client, bufnr)
    vim.lsp.completion.enable(true, client.id, bufnr, { autotrigger = true })
    vim.keymap.set('i', '<C-Space>', vim.lsp.completion.get, { buffer = bufnr })
    vim.keymap.set('n', 'K', vim.lsp.buf.hover, { buffer = bufnr })
    vim.keymap.set('n', 'gd', vim.lsp.buf.definition, { buffer = bufnr })
  end,
})
vim.lsp.enable('ratslang')
```

This enables native completion. If you already use a completion plugin, configure
it for LSP instead of enabling native completion in `on_attach`. To show
completion documentation in Neovim's popup, add:

```lua
vim.opt.completeopt = { 'menu', 'menuone', 'noselect', 'popup' }
```

Restart Neovim and open a `.rl` file. `:set filetype?` should show `ratslang`.
`:checkhealth vim.lsp` should show the running client. Press `K` for documentation,
`gd` for a definition, or `Ctrl-Space` in insert mode for completion (`Ctrl-y`
accepts an item). Native auto-completion also triggers on the server's trigger
characters. Restart Neovim after updating the server executable.

For optional syntax highlighting, compile the grammar parser
with a C compiler and copy the queries into Neovim's runtime path (Linux/macOS):

```sh
git clone https://codeberg.org/stelzo/tree-sitter-ratslang.git
git -C tree-sitter-ratslang checkout 6869b2e67f7d9fc4ccc26cdf9782b603eb770216
cd tree-sitter-ratslang
mkdir -p ~/.config/nvim/parser ~/.config/nvim/queries/ratslang
cc -O2 -shared -fPIC -Isrc src/parser.c -o ~/.config/nvim/parser/ratslang.so
cp queries/*.scm ~/.config/nvim/queries/ratslang/
```

Then add to `init.lua`:

```lua
vim.api.nvim_create_autocmd('FileType', {
  pattern = 'ratslang',
  callback = function(event)
    vim.treesitter.start(event.buf, 'ratslang')
  end,
})
```

This uses Neovim's built-in tree-sitter support and needs no parser-management
plugin. If you use nvim-treesitter, register the tree-sitter-ratslang repository as a custom parser
using your installed version's instructions instead. Avoid installing two
copies of the parser. See the [Neovim LSP reference](https://neovim.io/doc/user/lsp.html)
for the native client APIs.

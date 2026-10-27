# ratslang-cli

The `ratslang` command checks, formats, explains and migrates
[Ratslang](https://ratslang.steado.tech/) configuration files.

```sh
ratslang check friday.rl
ratslang explain friday.rl rat.whisker.length
ratslang fmt friday.rl --write
ratslang generate default.rl
ratslang minimize friday.rl
ratslang migrate friday.rl
```

The [CLI guide](https://ratslang.steado.tech/tools/cli/) shows each
command by example. The language server is
[ratslang-lsp](https://crates.io/crates/ratslang-lsp).

## License

MIT OR Apache-2.0

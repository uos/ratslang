# Namespaces and includes

## Namespaces

Dotted names and blocks can be combined:

```ratslang
rat.whiskers.left.length = 4cm
rat {
  whiskers {
    right.length = 4cm
  }
  nap_after = 20min
}
```

The fully qualified names are `rat.whiskers.left.length`,
`rat.whiskers.right.length`, and `rat.nap_after`. References inside a namespace resolve through the compiler's
namespace rules. Reassignments keep their original position in evaluation order.

## Include another file

Assign a path to the current namespace to include another Ratslang file.

```ratslang title="whiskers.rl"
count = 24
length = 4cm
```

```ratslang title="rat.rl"
whiskers {
  = ./whiskers.rl
}
whiskers.length = 5cm
```

Relative includes resolve against the containing file's directory. Values from
`whiskers.rl` receive the `whiskers` prefix here. A later assignment overrides
the included value. An include at the root uses `= ./whiskers.rl` without a
block.

Use `compile_file` when reading files so the compiler can resolve includes from
the correct directory. For source strings, `compile_code_with_state` accepts an
explicit parent directory. Include nesting is limited to 64 files.

## Defaults and overrides

Applications can compile defaults and user configuration separately, then use
[`from_variables_with_defaults`](../rust/serde.md#defaults-and-user-overrides)
for a typed view. The CLI's `--schema` option loads defaults for validation.
A `# @schema` comment alone does not include application values.

See [schemas](../schemas.md) for validation and
[the CLI](../tools/cli.md) for layered command-line overrides and provenance.

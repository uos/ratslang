# Getting started

## A basic file

Ratslang files normally end in `.rl`. Comments start with `#` and settings use
`name = value`. Units are part of the value.

```ratslang title="rat.rl"
rat.name = "Remy"
rat {
  whisker.length = 4cm
  nap_after = 20min
}
mood = CURIOUS
```

A dotted setting and a nested block describe the same namespace. Unquoted names
such as `CURIOUS` become strings when they do not refer to an existing variable.
Use quotes when a string must stay a string.

## Use the library

Add the library to your `Cargo.toml`:

```toml
[dependencies]
ratslang = "0.5.0"
```

```rust
use ratslang::{compile_code, Rhs, Val};

let config = compile_code("rat.whisker.length = 4cm\n")?;
if let Some(Rhs::Val(Val::UnitedVal(value))) = config.vars.resolve("rat.whisker.length")? {
    println!("{}", value.to_source()?);
}
```

Enable the `serde` feature to read configuration into typed Rust structs:

```toml
[dependencies]
ratslang = { version = "0.5.0", features = ["serde"] }
serde = { version = "1", features = ["derive"] }
```

The core compiler requires Rust 1.85.1 or newer. Building the LSP requires Rust
1.95 or newer and a C compiler.

## Add defaults and constraints

Keep the defaults in their own file. Comments serve as documentation, while
annotations define validation rules.

```ratslang title="default.rl"
# How far the whiskers reach.
# @type length
# @min 1cm
# @max 10cm
rat.whisker.length = 4cm

# Running speed in the maze.
# @type velocity
# @max 2m/s
rat.scurry_speed = 1.2m/s

# @enum CURIOUS, SNEAKY, HANGRY
rat.mood = CURIOUS
```

Create an override file containing only the settings that change:

```ratslang title="friday.rl"
# @schema ./default.rl
rat.mood = HANGRY
rat.scurry_speed = 5km/h
```

The schema comment enables editor assistance and selects local defaults for the
CLI. Applications explicitly load default and user layers. The comment itself
does not import values into an application.

Check it with the [`ratslang` command-line tool](tools/cli.md):

```sh
ratslang check friday.rl --schema default.rl
```

Continue with [serde](rust/serde.md) for typed configuration, or
[editor setup](editors/index.md) for completion and hover documentation.

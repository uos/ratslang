# Serde and typed configuration

Enable the `serde` feature to deserialize Rust structs directly. Reusable
`Quantity<Length>` and `Quantity<Time>` wrappers preserve physical dimensions
inside arrays, vectors, matrices, optional values, and ranges. Existing direct
uom fields can still use the `length` and `time` field adapters.

```rust
use ratslang::{Length, Time};
use ratslang::serde::{Quantity, Range};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
enum Mood {
    Curious,
    Sneaky,
    #[serde(alias = "hungry")]
    Hangry,
}

#[derive(Serialize, Deserialize)]
struct Rat {
    nose_offset: [Quantity<Length>; 3],
    naps: Vec<Quantity<Time>>,
    cheese_range: Range<Quantity<Length>>,
    #[serde(with = "ratslang::serde::case_insensitive")]
    mood: Mood,
}

let rat: Rat = ratslang::from_str(
    "nose_offset = [5cm, 0m, 1cm]\nnaps = [20min, 2h]\ncheese_range = 1cm..\nmood = HUNGRY\n",
)?;
let source = ratslang::to_string(&rat)?;
```

The enum adapter accepts ASCII case variants of derived variant names, Serde
renames, and aliases. Serialization keeps the declared canonical spelling.
Quantity wrappers serialize with readable units and reject mismatched dimensions.



## Defaults and user overrides

Compile both layers and deserialize directly into the application's derived
configuration type:

```rust
use ratslang::compile_code;
use ratslang::serde::from_variables_with_defaults;
use serde::Deserialize;

#[derive(Deserialize)]
struct Rat {
    cheese_budget: u32,
    name: String,
}

let defaults = compile_code("cheese_budget = 4\nname = \"Remy\"\n")?;
let user = compile_code("cheese_budget = 8\n")?;
let rat: Rat = from_variables_with_defaults(&defaults.vars, &user.vars)?;
assert_eq!(rat.cheese_budget, 8);
assert_eq!(rat.name, "Remy");
```

This applies default constraints to consumed overrides and records reads, so
applications can report unused or unknown settings. Required settings must be
explicitly present in the user layer. Use `from_evaluated` when a single compiled
configuration is sufficient. Use `ConfigReader` for typed namespace access.

## Physical values

Use `Quantity<T>` for quantities inside containers, or a field adapter for a
direct `uom` field. The adapters are `length`, `time`, `mass`, `angle`, `frequency`,
`velocity`, and `acceleration` in `ratslang::serde`. They support serialization
and deserialization. `Range<Quantity<T>>` retains optional endpoints.

Serialization chooses readable units. Dimension mismatches and non-finite values
are errors. Ratslang's physical serde integration uses typed `uom` quantities.
See [units](../language/units.md) for literals and direct adapter examples.

## Writing configuration

`to_string` serializes typed values into a new source document. It normalizes
structure and unit spelling. To retain comments, includes, unknown settings,
and the original formatting while changing typed fields, use
[`Document<T>`](document-editing.md).

# Editing while preserving source

`Document<T>` pairs a serde configuration with its original source. Parsing and
writing an unchanged document preserves its bytes, including comments, units,
annotations, includes, and fields the Rust type does not represent.

```rust
use ratslang::serde::Document;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct Show {
    drones: u32,
}

let source = "# How many drones fly.\ndrones = 120\nlights = TWINKLE\n";
let mut document = Document::<Show>::parse(source)?;
assert_eq!(document.to_source()?, source);
document.value.drones = 150;
let updated = document.to_source()?;
assert!(updated.starts_with(source));
```

Edits become trailing override assignments. This preserves earlier expressions,
references, comments, and includes while allowing Ratslang's assignment semantics
to produce the new value. The compiler verifies the resulting source before
returning it. Annotations on changed values are retained.

`source()` returns the original text. `to_source()` produces the edited text.
`ratslang::to_string(&document)` uses the same source-preserving path.
Removing a represented field returns an error.

For file-level formatting, minimization, and renaming, use the
[CLI's source-preserving commands](../tools/cli.md).

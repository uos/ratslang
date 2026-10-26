# Compiler and resolved values

For a derived application configuration, start with [serde](serde.md). Use the
compiler directly when inspecting values, recording reads, or building tools.

Add this to your `Cargo.toml`.

~~~toml
[dependencies]
ratslang = "0.5.0"
~~~

First, you compile a Ratslang file to get a cleaned Abstract Syntax Tree (AST) with all variables resolved.

~~~rust
let file = std::path::Path::new("./your_file.rl");
let ast = ratslang::compile_file(&file.to_path_buf(), None, None).unwrap();
~~~

Then, you can safely read the variables you need — either with the provided helper macros for concise code, or manually using Rust's powerful pattern matching.

## Helper macros

These examples use an `anyhow::Result<()>` function context and bring the
compiler types and helper macros into scope.

~~~rust
use anyhow::anyhow;
use ratslang::*;

// Local configs combining user vars and optional defaults
struct Configs {
    user: ratslang::VariableHistory,
    defaults: ratslang::VariableHistory,
}

let file = std::path::Path::new("./your_file.rl");
let ast = compile_file(&file.to_path_buf(), None, None).unwrap();
let configs = Configs { user: ast.vars, defaults: ratslang::VariableHistory::new(vec![]) };

// Simple values
let name: String = resolve_string!(configs, "name")?;
let enabled: bool = resolve_bool!(configs, "variable")?;
let k: i64 = resolve_int!(configs, "k_neighbors")?;
let ratio: f64 = resolve_float!(configs, "something_else")?;

// Paths
let path: String = resolve_path!(configs, "_file")?; // e.g., `_file = /abs/or/relative`

// Ranges (with sensible defaults used when bounds are missing)
let (d_min, d_max) = resolve_length_range_meters_float!(configs, "len", 0.0, 10.0)?; // meters as f64
let (t_min, t_max) = resolve_time_range_seconds_float!(configs, "time_is_running", 0.0, 60.0)?; // seconds as f64
let (i_min, i_max) = resolve_int_range!(configs, "my.super.long.prefix.var", 0, 100)?;
~~~

## Working with uom types

Physical values in Ratslang are stored as `uom` types (`Length` and `Time`), giving you full access to type-safe unit conversions:

~~~rust
use ratslang::{compile_code, Rhs, Val, UnitVal, Unit};
use ratslang::{meter, millimeter, micrometer, nanometer};  // Length units
use ratslang::{second, millisecond, microsecond, nanosecond};  // Time units

let code = r#"
    whisker_tip = 500um
    reflex = 1500ns
"#;

let ast = compile_code(code).unwrap();

// Get a length value and convert to any unit
if let Some(Rhs::Val(Val::UnitedVal(uv))) = ast.vars.resolve("whisker_tip").unwrap() {
    // Access the underlying uom::si::f64::Length
    let length = uv.as_length().unwrap();
    
    // Convert to any unit with full precision using uom getters
    println!("Tip: {} micrometers", length.get::<micrometer>());  // 500.0
    println!("Tip: {} millimeters", length.get::<millimeter>());  // 0.5
    println!("Tip: {} meters", length.get::<meter>());            // 0.0005
    println!("Tip: {} nanometers", length.get::<nanometer>());    // 500000.0
}

// Get a time value
if let Some(Rhs::Val(Val::UnitedVal(uv))) = ast.vars.resolve("reflex").unwrap() {
    let time = uv.as_time().unwrap();
    
    // Convert to any unit with full precision using uom getters
    println!("Reflex: {} nanoseconds", time.get::<nanosecond>());   // 1500.0
    println!("Reflex: {} microseconds", time.get::<microsecond>()); // 1.5
    println!("Reflex: {} milliseconds", time.get::<millisecond>()); // 0.0015
}
~~~

## Manual resolution

~~~rust
use anyhow::anyhow;
use ratslang::{compile_file, Rhs, Val, NumVal};

// Local configs combining user vars and optional defaults
struct Configs {
    user: ratslang::VariableHistory,
    defaults: ratslang::VariableHistory,
}

let file = std::path::Path::new("./your_file.rl");
let ast = compile_file(&file.to_path_buf(), None, None).unwrap();
let vars = ast.vars.filter_ns(&["_my_namespace"]);

// Resolve a variable and pattern-match its type manually
let value = vars
    .resolve("_some_var")?
    .map_or(Ok("a_default_value".to_owned()), |rhs| {
        Ok(match rhs {
            Rhs::Val(Val::StringVal(s)) => s,
            _ => {
                return Err(anyhow!(
                    "Unexpected type for _my_namespace._some_var, expected String."
                ));
            }
        })
    })?;

// Or use the generic resolve_var! macro
use ratslang::resolve_var;
let configs = Configs { user: vars, defaults: ratslang::VariableHistory::new(vec![]) };
let k_neighbors: usize = resolve_var!(configs, k_neighborhood, as usize,
    Rhs::Val(Val::NumVal(NumVal::Integer(i))) => { i }
)?;
~~~


## Provenance and tooling

`VariableHistory::explain` exposes the effective value, winning definition,
reference sources, include chain, and reassignment history. Accessing provenance
does not mark settings as read. The same compiler-backed operations used by the
CLI are available in `ratslang::tooling`.

See the [API reference](https://docs.rs/ratslang) for the published crate, and
run `cargo doc -p ratslang --all-features --open` for APIs in the current checkout.

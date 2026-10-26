# Comment annotations

Annotations allow you to mark sections of your configuration for selective extraction and filtering. This is particularly useful for generating minimal configurations, feature-specific settings, or environment-dependent parameters.

### Basic Usage

Annotate a single line with `# @annotation_name`:

~~~ratslang
# @minimal
nap_after = 20min

# @dev
squeak = EVERY_TURN

cheese_budget = 3
~~~

Annotate an entire namespace block:

~~~ratslang
# @minimal
whiskers {
    count = 24
    length = 4cm
}

whiskers.calibration_file = /path/to/whiskers.yaml
~~~

### Extracting Annotations

Use `to_string_filtered()` to extract only annotated sections:

~~~rust
use ratslang::compile_code;

let source = r#"
    # @minimal
    nap_after = 20min

    # @minimal
    whiskers {
        count = 24
    }

    whiskers.length = 4cm
"#;

let ast = compile_code(source).unwrap();
let minimal_config = ast.to_string_filtered("minimal").unwrap();
// Result: only nap_after and the whiskers block
~~~

### Use Cases

- **Minimal configurations:** Use `@minimal` to generate deployment-ready configs with only essential parameters
- **Feature flags:** Use `@feature_name` to extract configurations for specific features  
- **Environment-specific settings:** Use `@prod`, `@dev`, `@test` for environment variants
- **Documentation examples:** Use `@documented` to generate user-facing configuration examples

[Constraint annotations](../schemas.md) are interpreted by the compiler and
schema tools. Other annotations such as `@minimal` are application-defined tags.
Annotation filtering retains multiline prose comments together.

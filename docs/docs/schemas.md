# Schemas and constraints

A defaults file is also the schema. Values are the defaults, comments are the
documentation, and annotations are the rules.

```ratslang title="default.rl"
# Underwater glider, one sawtooth dive after another.

# Ballast mass, including the pump.
# @type mass
# @min 100g
# @max 2kg
ballast = 450g

# @enum SAWTOOTH, SPIRAL, HOVER
dive.pattern = SAWTOOTH

# Depths where the sensors take a sample.
# @type array<length>
# @min 0m
dive.sample_depths = [10m, 50m, 200m]

# The recovery boat calls each glider by name.
# @required
glider.name = ""

# @renamed_to dive.depth
max_depth = 200m

# @type length
# @max 1000m
dive.depth = 200m
```

A user file only sets what changes. It keeps the rules from the defaults.

```ratslang title="mission.rl"
# @schema ./default.rl
glider.name = "Nemo"
ballast = 2lb
```

```text
$ ratslang check mission.rl
mission.rl: valid
```

Bounds compare across units, so `2lb` passes the `2kg` limit. Everything else
is caught:

```text
ballast = 3kg                    ballast: value 3kg must be <= 2kg
ballast = 2m                     ballast: expected Mass, got 2m
dive.pattern = LOOPING           dive.pattern: expected one of [SAWTOOTH, SPIRAL, HOVER], got "LOOPING"
dive.sample_depths = [10m, -5m]  dive.sample_depths: element [1]: value -5m must be >= 0m
(no glider.name)                 error: Required setting glider.name must be explicitly assigned
max_depth = 400m                 warning: max_depth: This setting was renamed; use dive.depth
dive.dpeth = 400m                warning: Unknown setting dive.dpeth; did you mean dive.depth?
```

The same checks run in the [editor](editors/index.md), with fixes for renamed
and misspelled settings, and in applications through
[`from_variables_with_defaults`](rust/serde.md#defaults-and-user-overrides).

## Annotations

| Annotation | Meaning |
| --- | --- |
| `@type TYPE` | Value type |
| `@enum SAWTOOTH, HOVER` | Allowed values. `@enum_case insensitive` ignores case |
| `@min VALUE`, `@max VALUE` | Inclusive bounds. `@min_exclusive` and `@max_exclusive` exclude them |
| `@required` | The user file must set it |
| `@recommended` | Included by `ratslang generate` |
| `@deprecated MESSAGE` | Warn when it is set |
| `@renamed_to KEY` | Warn and offer to migrate to `KEY` |

Types are `bool`, `int`, `float`, `number`, `string`, `path`, `length`, `time`,
`mass`, `angle`, `frequency`, `velocity` and `acceleration`, composed with
`array<T>` and `range<T>`.

## Attaching a schema

The `# @schema` line in the leading comment points the editor and the CLI at
the defaults. It is a path relative to the file, or an HTTPS URL for the editor.
Applications load the defaults themselves.

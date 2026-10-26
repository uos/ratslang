# Values and syntax

## Scalars

```ratslang
# Drone light show, New Year's Eve.
lights = TWINKLE
drones = 120
wind_correction = -0.125
title = "Happy New Year"
formation = SPIRAL
choreography = ./shows/midnight.csv
```

| Value | Syntax |
| --- | --- |
| Boolean | `true`, `false` |
| Integer | `42`, `-100` |
| Float | `1.25`, `-0.5`, `1.5e-7`, `2E+6` |
| String | `"any text"`, or an unquoted name |
| Path | A value starting with `/` or `./` |
| Quantity | A number immediately followed by a [unit](units.md) |

A flag that names a state, such as `lights_on = true`, reads better as an enum.
`lights = TWINKLE` says what happens, and `lights = STROBE` fits in later
without renaming the setting.

Numbers use decimal notation, optionally with an exponent: `1.5e-7`, and a
quantity such as `1.5e-3m`. A number with an exponent is always a float, `3e2`
included. The exponent needs digits, so `1Em` is still one exametre. A number
or unit conversion that overflows is an error.

Quoted strings are raw: backslashes stay as written, and a quote ends
the string. The parser accepts multiline strings. The value serializer rejects
strings containing quotes or newlines because it cannot represent them safely.

An unquoted path extends to the next space or newline. Delimiters immediately
after a path become part of that path. Use strings for path-like values inside
arrays when you need surrounding punctuation.

## Arrays and matrices

```ratslang
colors = ["gold", "white"]
launch_delays = [0s, 200ms, 400ms]
formation_rotation = [[1, 0, 0],
                      [0, 1, 0],
                      [0, 0, 1]]
```

Arrays can be nested and may have a trailing comma. Put continuation newlines
after commas. The language permits mixed arrays. A schema or Rust target type
can require consistent element types.

## Ranges

```ratslang
altitude = 80m..120m
warmup = 5s..
payload = ..250g
wind = ..
drone_ids = 1..120
brightness = 0.25..0.75
```

Either bound can be omitted. A range is a pair of optional limits, rather than
an arithmetic expression. Use the same physical dimension at both ends.
Integer/float mixed endpoints are rejected. The serde `Range<T>` wrapper keeps
missing bounds as `None`.

## Assignments and references

```ratslang
spacing = 1.5m
safety_gap = spacing
spacing = 2m
left_wing, right_wing = 10m, 20m
fade_in, fade_out = 500ms
```

Settings are mutable. References copy the value available at assignment time:
`safety_gap` remains `1.5m` in this example. Multiple keys can receive paired values or
share one broadcast value. Names begin with an ASCII letter, `_`, or `:` and may
contain letters, digits, underscores, and colons. Use `.` to separate namespaces.
An initial `_` conventionally marks an internal application setting.

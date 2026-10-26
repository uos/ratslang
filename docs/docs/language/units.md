# Units and dimensions

All quantities use typed `uom::si::f64` values. All dimensions are listed below:

| Dimension | Literal spellings | Rust type / serde adapter |
| --- | --- | --- |
| Length | SI-prefixed metres (`ym`–`Ym`, including `um`/`µm`/`μm`), `ft`, `in`, `mi`, `yd`, and astronomical units | `Length` / `length` |
| Time | SI-prefixed seconds (`ys`–`Ys`, including `us`/`µs`), `min`, `h`, `d`, and `a` | `Time` / `time` |
| Mass | SI-prefixed grams (`yg`–`Yg`, including `ug`/`µg`, `mg`, `g`, `kg`), `t`/`tonne`/`tonnes`, `lb`/`pound`/`pounds`, `oz`/`ounce`/`ounces` | `Mass` / `mass` |
| Angle | `deg`, `rad`, `mrad`, `urad`, `µrad` | `Angle` / `angle` |
| Frequency | `mHz`, `Hz`, `kHz`, `MHz`, `GHz` | `Frequency` / `frequency` |
| Velocity | `mm/s`, `cm/s`, `m/s`, `km/h` | `Velocity` / `velocity` |
| Acceleration | `mm/s2`, `cm/s2`, `m/s2` (also `/s^2`) | `Acceleration` / `acceleration` |

```ratslang
# Lunar rover, landing site Shackleton rim.

# @type angle
# @min 0deg
# @max 180deg
camera.field_of_view = 90deg
mast.pan_resolution = 500urad

# @type frequency
# @min 1Hz
imu.rate = 2kHz

# 0.54km/h and 0.15m/s are the same speed.
# @type velocity
# @max 0.2m/s
drive.top_speed = 0.54km/h

# The Moon pulls a sixth as hard.
# @type acceleration
gravity = 1.62m/s2

# @type mass
# @min 500g
# @max 5kg
arm.payload = 2kg
sample_mass = 250mg
regolith_scoops = [20g, 35g, 50g]
allowed_payload = 500g..5kg
lander = 1.5t
souvenir = 2lb
```

These values work in arrays, matrices, ranges, constraints, and schema
completion/hover. Bounds compare across units while rejecting mismatched
dimensions. With serde enabled, use `Quantity<Mass>`, `Quantity<Angle>`, `Quantity<Frequency>`,
`Quantity<Velocity>`, or `Quantity<Acceleration>`, or attach the corresponding
`ratslang::serde` field adapter to a direct uom field. Serialization chooses
readable units. It may normalize degrees to radians or km/h to m/s.

---

For direct uom fields, the same adapters work in both directions:

```rust
use ratslang::{Mass, kilogram};
use ratslang::serde::Quantity;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Payload {
    #[serde(with = "ratslang::serde::mass")]
    maximum: Mass,
    samples: Vec<Quantity<Mass>>,
}
let payload: Payload = ratslang::from_str(
    "maximum = 2kg\nsamples = [250mg, 1g, 5ug]\n",
)?;
assert_eq!(payload.maximum.get::<kilogram>(), 2.0);
let source = ratslang::to_string(&payload)?;
```

## Time and length spellings

Time accepts SI-prefixed seconds, `min`/`minute`/`minutes`, `h`/`hour`/`hours`,
`d`/`day`/`days`, `a`/`year`/`years`, `shake`/`shakes`, sidereal variants such as
`second_sidereal`, and `year_tropical`. Use `2min`, rather than `2mins`.

Length also accepts `feet`, `inches`, `yards`, `miles`, `chains`, `rods`, `fathom`,
`angstrom`/`Å`, `micron`/`µ`, `mil`, `microinch`, `bohr_radius`, `fermi`,
`au`/`ua`/`astronomical_unit`, `light_year`/`l.y.`, `pc`, and `nautical_mile`.
Plural word forms are accepted where declared by the lexer. Spellings are
case-sensitive: `mHz` and `MHz` differ by a factor of a billion.

Serialization selects readable units while preserving the underlying SI value.
For example, small lengths may become `um` and a mass may become `250mg`.
Use types re-exported by Ratslang (or `ratslang::uom`) to keep the same `uom`
version as the library.

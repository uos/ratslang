//! # Ratslang
//!
//! A compact configuration language for physical systems with native support for time and length units.
//!
//! Ratslang solves the unit conversion problem that plagues physical system configuration, while keeping
//! the implementation simple and declarative. Physical units are stored directly as [`uom`](https://crates.io/crates/uom)
//! types (`Length` and `Time`) for full type-safety and precision.
//!
//! ## Features
//!
//! - **Time units:** `ns`, `us`, `ms`, `s`, `min`, `h`, `d`, `a` (year), `shake` (and SI prefixes, see [Supported Time Units](#supported-time-units))
//! - **Length units:** `nm`, `um`, `mm`, `cm`, `m` (see [Supported Length Units](#supported-length-units))
//! - **Ranges:** `1mm..100m`, `5s..`, `..10m`, `..`
//! - **Namespaces:** Dot notation (`sensor.range`) or blocks
//! - **Includes:** Compose configuration files with namespace scoping
//! - **Annotations:** Mark configuration sections with `# @annotation_name` for selective filtering and extraction
//! - **Types:** Booleans, integers, floats, strings, paths, arrays
//! - **Type-safe units:** Values stored directly as `uom` types - no conversion or rounding
//!
//! ## Example
//!
//! ```rust
//! use ratslang::compile_code;
//!
//! let source = r#"
//!     distance = 100mm..2m
//!     scan_rate = 10.5s
//!     
//!     sensor {
//!         type = Lidar
//!         enabled = true
//!     }
//! "#;
//!
//! let result = compile_code(source).unwrap();
//! let distance = result.vars.resolve("distance").unwrap();
//! ```
//!
//! ## Usage
//!
//! Compile and resolve variables:
//!
//! ```rust
//! use ratslang::{compile_code, Rhs, Val, NumVal};
//!
//! let code = r#"
//!     timeout = 5000
//!     sensor.type = Lidar
//!     sensor.range = 10
//! "#;
//!
//! let ast = compile_code(code).unwrap();
//!
//! // Manual resolution
//! let timeout = ast.vars.resolve("timeout").unwrap().unwrap();
//! assert_eq!(timeout, Rhs::Val(Val::NumVal(NumVal::Integer(5000))));
//!
//! let name = ast.vars.resolve("sensor.type").unwrap().unwrap();
//! assert_eq!(name, Rhs::Val(Val::StringVal("Lidar".to_string())));
//!
//! // Filter namespace
//! let sensor = ast.vars.filter_ns(&["sensor"]);
//! let range = sensor.resolve("range").unwrap().unwrap();
//! assert_eq!(range, Rhs::Val(Val::NumVal(NumVal::Integer(10))));
//! ```
//!
//! > **Note:** Ratslang is deliberately simple—no arithmetic, loops, or conditionals.
//!
//! ## Serde
//!
//! Enable `features = ["serde"]` for `ratslang::from_str` and `ratslang::to_string`.
//! This also enables the `uom` integration feature. Mark typed quantity fields with
//! `#[serde(with = "ratslang::serde::length")]` or `ratslang::serde::time` to preserve
//! dimensions and emit readable units such as `1um` and `250ms`. See the `serde`
//! module for a complete example and supported data types.
//! `serde::Quantity<Length>` and `serde::Quantity<Time>` work in arrays,
//! vectors, matrices, optional fields, and ranges without per-field adapters.
//! Use `serde::case_insensitive` for derived unit enums and their Serde aliases.
//! Use `ratslang::serde::Document<T>` to preserve source comments and annotations
//! when round-tripping a configuration through serde.
//! If the configuration is already compiled, use `ratslang::from_evaluated`
//! to deserialize it directly, retaining its include resolution and usage tracking.
//!
//! ## Type and value constraints
//!
//! Comments directly above assignments can declare `# @enum CPU, Backend`,
//! `# @type integer`, `# @min 0`, and `# @max 64`. Enum choices match strings,
//! identifiers, or paths exactly. Bounds are inclusive; physical quantities
//! compare across units, so `# @min 1mm` accepts `50cm`.
//! The compiler validates final values after includes, references, and overrides.
//! Serde also applies default configuration constraints to user overrides.
//! See [`constraints`] for examples and the supported type declarations.
//!
//! ## Syntax Highlighting
//!
//! Syntax highlighting is available with [this tree-sitter grammar](https://codeberg.org/stelzo/tree-sitter-ratslang) or [this VS Code extension](https://marketplace.visualstudio.com/items?itemName=stelzo.ratslang).
//!
//! ## Annotations
//!
//! Annotations allow you to mark sections of configuration for selective extraction and filtering.
//! Use `# @annotation_name` to mark lines or entire blocks that you want to extract later.
//!
//! ### Syntax
//!
//! Annotate a single line:
//! ```ignore
//! # @minimal
//! timeout = 5000
//! ```
//!
//! Annotate an entire namespace block:
//! ```ignore
//! # @minimal
//! sensor {
//!     type = Lidar
//!     range = 100m
//! }
//! ```
//!
//! ### Usage
//!
//! Extract annotated configuration (see also: [`Evaluated::to_string_filtered`]):
//!
//! ```no_run
//! use ratslang::compile_code;
//!
//! let source = r#"
//! # @minimal
//! timeout = 5000
//!
//! # @minimal
//! sensor {
//!     type = Lidar
//! }
//!
//! sensor.range = 100m
//! "#;
//!
//! let ast = compile_code(source).unwrap();
//! let minimal_config = ast.to_string_filtered("minimal").unwrap();
//! println!("{}", minimal_config);
//! ```
//!
//! ### Use Cases
//!
//! - **Minimal configurations:** Mark only essential parameters with `@minimal` for deployment or documentation
//! - **Feature flags:** Use `@feature_name` to extract configurations for specific features
//! - **Environment-specific:** Use `@prod`, `@dev`, `@test` for environment-specific settings
//! - **Documentation:** Mark parameters with `@documented` for user-facing config examples
//!
//! ## Supported Time Units
//!
//! ### Base Units
//!
//! - **SI Prefixes:** `fs` (femto), `ps` (pico), `ns` (nano), `us` (micro), `ms` (milli), `s` (second),
//!   `min` (minute), `h` (hour), `d` (day), `ks` (kilo)
//!
//! ### Common Units
//!
//! - `s` – second (base unit)
//! - `min` – minute (60 seconds)
//! - `h` – hour (3600 seconds)
//! - `d` – day (86400 seconds)
//!
//! ### Astronomical and Special Units
//!
//! - `a` – year (365 days, matching `uom`)
//! - `shake` – 10 nanoseconds (unit used in nuclear physics)
//! - `sidereal_year` – sidereal year (365.256363004 days)
//! - `tropical_year` – tropical year (365.242190402 days)
//!
//! ### Notes
//!
//! - SI prefixes combine with base units: `ms`, `us`, `ns`, `ps`, `fs`
//! - All time values are stored internally as `uom::si::f64::Time` with no conversion loss
//! - Ranges work with all time units: `100ms..5s`, `1h..`, `..30s`
//!
//! ## Supported Length Units
//!
//! ### SI Prefixes
//!
//! - `pm` (pico), `nm` (nano), `um` (micro), `mm` (milli), `cm` (centi), `m` (meter),
//!   `km` (kilo), `Mm` (mega), `Gm` (giga), `Tm` (tera), `Pm` (peta), `Em` (exa), `Zm` (zetta), `Ym` (yotta)
//!
//! ### Imperial Units
//!
//! - `in` or `inch` – inch (25.4 mm)
//! - `ft` or `foot` – foot (304.8 mm)
//! - `yd` or `yard` – yard (914.4 mm)
//! - `mi` or `mile` – mile (1609.344 m)
//!
//! ### Astronomical Units
//!
//! - `au` or `AU` – astronomical unit (~149.6 million km)
//! - `ly` or `light_year` – light-year (~9.46 trillion km)
//! - `pc` or `parsec` – parsec (~30.86 trillion km)
//!
//! ### Nautical Units
//!
//! - `nmi` or `nautical_mile` – nautical mile (1852 m)
//! - `fathom` – fathom (1.8288 m, nautical depth measurement)
//!
//! ### Small Units
//!
//! - `angstrom` – 10^-10 meters (used in atomic physics and chemistry)
//! - `bohr_radius` – Bohr radius (~0.529 Ångströms, atomic unit)
//! - `fermi` – femtometer (10^-15 m, nuclear unit)
//!
//! ### Notes
//!
//! - Ranges work with all length units: `1mm..100m`, `5ft..`, `..10mi`
//! - Mixed units in expressions are not supported; each value must use consistent units

use core::fmt;
pub mod constraints;
pub mod tooling;

/// Where the calling crate's release publishes its defaults schema:
/// `<repository>/releases/download/v<version>/default.rl`, from the crate's
/// own `repository` and `version` in Cargo.toml, so every binary names the
/// schema of exactly its release. The release has to attach the defaults
/// file under that name; pass another asset name as the argument.
///
/// ```
/// let url = ratslang::release_schema_url!();
/// assert!(url.ends_with(&format!("/releases/download/v{}/default.rl", env!("CARGO_PKG_VERSION"))));
/// ```
#[macro_export]
macro_rules! release_schema_url {
    () => {
        $crate::release_schema_url!("default.rl")
    };
    ($asset:literal) => {
        concat!(
            env!("CARGO_PKG_REPOSITORY"),
            "/releases/download/v",
            env!("CARGO_PKG_VERSION"),
            "/",
            $asset
        )
    };
}
pub use constraints::{Constraints, ValueType};
mod finite;
mod format;
mod physical;
mod provenance;
pub use format::FormatError;
pub use provenance::{Definition, Explanation, SourceLocation};
#[cfg(feature = "serde")]
pub mod serde;
#[cfg(feature = "serde")]
pub use crate::serde::{from_evaluated, from_str, to_ast, to_string};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    path::PathBuf,
    rc::Rc,
    str::FromStr,
};

use anyhow::anyhow;
use ariadne::{Color, Label, Report, ReportKind, Source};
use logos::{Lexer, Logos};

use chumsky::{
    input::{Stream, ValueInput},
    prelude::*,
};

// Re-export uom types for users
pub use uom;
pub use uom::si::f64::{Acceleration, Angle, Frequency, Length, Mass, Time, Velocity};
pub use uom::si::length::{
    angstrom, astronomical_unit, attometer, bohr_radius, centimeter, chain, decameter, decimeter,
    exameter, fathom, femtometer, fermi, foot, gigameter, hectometer, inch, kilometer, light_year,
    megameter, meter, microinch, micrometer, micron, mil, mile, millimeter, nanometer,
    nautical_mile, parsec, petameter, picometer, rod, terameter, yard, yoctometer, yottameter,
    zeptometer, zettameter,
};
pub use uom::si::mass::{gram, kilogram, microgram, milligram, nanogram, ounce, pound};
pub use uom::si::time::{
    attosecond, centisecond, day, day_sidereal, decasecond, decisecond, exasecond, femtosecond,
    gigasecond, hectosecond, hour, hour_sidereal, kilosecond, megasecond, microsecond, millisecond,
    minute, nanosecond, petasecond, picosecond, second, second_sidereal, shake, terasecond, year,
    year_sidereal, year_tropical, yoctosecond, yottasecond, zeptosecond, zettasecond,
};
pub use uom::si::{
    acceleration::meter_per_second_squared,
    angle::{degree, radian},
    frequency::hertz,
    velocity::meter_per_second,
};

fn split_num_unit(slice: &str) -> Option<(f64, &str)> {
    let mut idx = slice
        .find(|c: char| c.is_alphabetic() || c == 'µ')
        .unwrap_or(slice.len());
    // An exponent is part of the number: `1.5e-3m` is 1.5e-3 metres. It needs
    // digits after the `e`, so the `E` of a unit such as `Em` is left alone.
    if let Some(exponent) = slice[idx..].strip_prefix(['e', 'E']) {
        let digits = exponent.trim_start_matches(['+', '-']);
        let rest = digits.trim_start_matches(|c: char| c.is_ascii_digit());
        let count = digits.len() - rest.len();
        if count > 0 {
            idx = slice.len() - digits.len() + count;
        }
    }
    if idx == 0 || idx == slice.len() {
        return None;
    }
    let (num, unit) = slice.split_at(idx);
    let val: f64 = num.parse().ok()?;
    Some((val, unit))
}

fn parse_length_prefixed<'a>(lex: &mut Lexer<'a, Token<'a>>) -> Option<Length> {
    let (val, unit) = split_num_unit(lex.slice())?;
    let l = match unit {
        "ym" => Length::new::<yoctometer>(val),
        "zm" => Length::new::<zeptometer>(val),
        "am" => Length::new::<attometer>(val),
        "fm" => Length::new::<femtometer>(val),
        "pm" => Length::new::<picometer>(val),
        "nm" => Length::new::<nanometer>(val),
        "um" | "µm" => Length::new::<micrometer>(val),
        "mm" => Length::new::<millimeter>(val),
        "cm" => Length::new::<centimeter>(val),
        "dm" => Length::new::<decimeter>(val),
        "m" => Length::new::<meter>(val),
        "dam" => Length::new::<decameter>(val),
        "hm" => Length::new::<hectometer>(val),
        "km" => Length::new::<kilometer>(val),
        "Mm" => Length::new::<megameter>(val),
        "Gm" => Length::new::<gigameter>(val),
        "Tm" => Length::new::<terameter>(val),
        "Pm" => Length::new::<petameter>(val),
        "Em" => Length::new::<exameter>(val),
        "Zm" => Length::new::<zettameter>(val),
        "Ym" => Length::new::<yottameter>(val),
        "angstrom" | "ångström" => Length::new::<angstrom>(val),
        "ft" | "foot" | "feet" => Length::new::<foot>(val),
        "in" | "inch" | "inches" => Length::new::<inch>(val),
        "mi" | "mile" | "miles" => Length::new::<mile>(val),
        "yd" | "yard" | "yards" => Length::new::<yard>(val),
        "ch" | "chain" | "chains" => Length::new::<chain>(val),
        "rd" | "rod" | "rods" => Length::new::<rod>(val),
        "fathom" | "fathoms" => Length::new::<fathom>(val),
        "ua" | "au" | "astronomical_unit" | "astronomical_units" => {
            Length::new::<astronomical_unit>(val)
        }
        "l.y." | "light_year" | "light_years" => Length::new::<light_year>(val),
        "pc" | "parsec" | "parsecs" => Length::new::<parsec>(val),
        "M" | "nautical_mile" | "nautical_miles" => Length::new::<nautical_mile>(val),
        "micron" | "microns" => Length::new::<micron>(val),
        "mil" | "mils" => Length::new::<mil>(val),
        "a₀" | "bohr_radius" | "bohr_radiuses" => Length::new::<bohr_radius>(val),
        "fermi" | "fermis" => Length::new::<fermi>(val),
        "Å" => Length::new::<angstrom>(val),
        "µ" => Length::new::<micron>(val),
        "μm" | "microinch" | "microinches" => Length::new::<microinch>(val),
        _ => return None,
    };
    Some(l)
}

fn parse_time_prefixed<'a>(lex: &mut Lexer<'a, Token<'a>>) -> Option<Time> {
    let (val, unit) = split_num_unit(lex.slice())?;
    let t = match unit {
        "ys" => Time::new::<yoctosecond>(val),
        "zs" => Time::new::<zeptosecond>(val),
        "as" => Time::new::<attosecond>(val),
        "fs" => Time::new::<femtosecond>(val),
        "ps" => Time::new::<picosecond>(val),
        "ns" => Time::new::<nanosecond>(val),
        "us" | "µs" => Time::new::<microsecond>(val),
        "ms" => Time::new::<millisecond>(val),
        "cs" => Time::new::<centisecond>(val),
        "ds" => Time::new::<decisecond>(val),
        "s" => Time::new::<second>(val),
        "das" => Time::new::<decasecond>(val),
        "hs" => Time::new::<hectosecond>(val),
        "ks" => Time::new::<kilosecond>(val),
        "Ms" => Time::new::<megasecond>(val),
        "Gs" => Time::new::<gigasecond>(val),
        "Ts" => Time::new::<terasecond>(val),
        "Ps" => Time::new::<petasecond>(val),
        "Es" => Time::new::<exasecond>(val),
        "Zs" => Time::new::<zettasecond>(val),
        "Ys" => Time::new::<yottasecond>(val),
        "min" | "minute" | "minutes" => Time::new::<minute>(val),
        "h" | "hour" | "hours" => Time::new::<hour>(val),
        "d" | "day" | "days" => Time::new::<day>(val),
        "a" | "year" | "years" => Time::new::<year>(val),
        "shake" | "shakes" => Time::new::<shake>(val),
        "second_sidereal" | "seconds_sidereal" => Time::new::<second_sidereal>(val),
        "day_sidereal" | "days_sidereal" => Time::new::<day_sidereal>(val),
        "hour_sidereal" | "hours_sidereal" => Time::new::<hour_sidereal>(val),
        "year_sidereal" | "years_sidereal" => Time::new::<year_sidereal>(val),
        "year_tropical" | "years_tropical" => Time::new::<year_tropical>(val),
        _ => return None,
    };
    Some(t)
}

fn rm_first_and_last<'a>(lex: &mut Lexer<'a, Token<'a>>) -> &'a str {
    let slice = lex.slice();
    &slice[1..slice.len() - 1]
}

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\f\r]+")]
#[logos(skip r"ł(?:@[a-zA-Z_]+)?")]
enum Token<'a> {
    Error,

    // -- Control --
    #[token("+")]
    OpPlus,

    #[token("-")]
    OpMinus,

    #[token(",")]
    Comma,

    #[regex(r"(#.*)?\n+")]
    NewLine,

    #[token("(")]
    BracketOpen,

    #[token(")")]
    BracketClose,

    #[token("{")]
    BlockStart,

    #[token("}")]
    BlockEnd,

    #[token("[")]
    LBracket,

    #[token("]")]
    RBracket,

    // -- Operators --
    #[regex("=")]
    OpAssignToLeft,

    #[token("..", priority = 2)]
    OpRange,

    #[token(".", priority = 3)]
    Dot,

    #[regex(r"\.?\/+[^ \n]*")]
    Path(&'a str),

    #[regex(r#""[^"]*""#, rm_first_and_last)]
    String(&'a str),

    // -- Keywords --
    #[token("if")]
    KwIf,

    #[token("~")]
    DoNotCareOptim,

    // -- Expressions --
    #[token("true")]
    True,

    #[token("false")]
    False,

    // Length tokens - support all SI prefixes
    #[regex(r"[+-]?(?:\d+\.\d+|\d+)(?:[eE][+-]?\d+)?(?:astronomical_units?|nautical_miles?|light_years?|microinches?|bohr_radiuses?|ångströms?|astronomical_unit|nautical_mile|light_year|microinch|bohr_radius|ångström|angstroms?|angstrom|feet|inches|yards|miles|chains|rods|fathoms?|fermis?|fermi|microns?|micron|mils?|ym|zm|am|fm|pm|nm|um|µm|mm|cm|dm|dam|hm|km|Mm|Gm|Tm|Pm|Em|Zm|Ym|ft|in|mi|yd|ch|rd|ua|au|pc|l\.y\.|Å|µ|μm|m)", parse_length_prefixed, priority = 3)]
    Length(Length),

    // Time tokens - support all SI prefixes on seconds plus minute/hour/day/year/shake
    #[regex(r"[+-]?(?:\d+\.\d+|\d+)(?:[eE][+-]?\d+)?(?:shakes?|years?|minutes?|hours?|days?|seconds?_sidereal|days?_sidereal|hours?_sidereal|years?_sidereal|years?_tropical|ys|zs|as|fs|ps|ns|us|µs|ms|cs|ds|s|das|hs|ks|Ms|Gs|Ts|Ps|Es|Zs|Ys|min|h|d|a)", parse_time_prefixed, priority = 3)]
    Time(Time),

    #[regex(r"[+-]?(?:\d+\.\d+|\d+)(?:[eE][+-]?\d+)?(?:milligrams?|micrograms?|nanograms?|kilograms?|grams?|tonnes?|metric_ton|pounds?|ounces?|lb|oz|yg|zg|ag|fg|pg|ng|ug|µg|μg|mg|cg|dg|dag|hg|kg|Mg|Gg|Tg|Pg|Eg|Zg|Yg|g|t|deg|rad|mrad|urad|µrad|mHz|Hz|kHz|MHz|GHz|m/s2|m/s\^2|cm/s2|cm/s\^2|mm/s2|mm/s\^2|m/s|cm/s|mm/s|km/h)", physical::parse, priority = 4)]
    Physical(UnitVal),

    #[regex(r"[+-]?(?:\d+\.\d+(?:[eE][+-]?\d+)?|\d+[eE][+-]?\d+)", |lex| lex.slice().parse().ok())]
    FloatingNumber(f64),

    #[regex(r"[+-]?\d+", |lex| lex.slice().parse().ok())]
    IntegerNumber(i64),

    #[regex(r"[a-zA-Z_:]+[a-zA-Z_:\d]*", |lex| lex.slice())]
    Identifier(&'a str),
}

impl fmt::Display for Token<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Path(path) => write!(f, "Path({:?})", path),
            Self::String(string) => write!(f, "String({:?})", string),
            Self::Comma => write!(f, ","),
            Self::OpPlus => write!(f, "+"),
            Self::OpMinus => write!(f, "-"),
            Self::Error => write!(f, "<error>"),
            Self::NewLine => write!(f, "\\n"),
            Self::BracketOpen => write!(f, "["),
            Self::BracketClose => write!(f, "]"),
            Self::BlockStart => write!(f, "{{"),
            Self::BlockEnd => write!(f, "}}"),
            Self::LBracket => write!(f, "("),
            Self::RBracket => write!(f, ")"),
            Self::OpAssignToLeft => write!(f, "<-"),
            Self::OpRange => write!(f, ".."),
            Self::KwIf => write!(f, "if"),
            Self::DoNotCareOptim => write!(f, "~"),
            Self::True => write!(f, "true"),
            Self::False => write!(f, "false"),
            Self::IntegerNumber(s) => write!(f, "{}", s),
            Self::FloatingNumber(s) => write!(f, "{}", s),
            Self::Length(l) => write!(f, "{}m", l.get::<meter>()),
            Self::Physical(v) => write!(f, "{}", v.to_source().map_err(|_| fmt::Error)?),
            Self::Time(t) => write!(f, "{}s", t.get::<second>()),
            Self::Identifier(s) => write!(f, "{}", s),
            Self::Dot => write!(f, "."),
        }
    }
}

/// Represents a numeric value that can be either a floating-point or integer.
#[derive(Debug, Clone, Copy)]
pub enum NumVal {
    /// A floating-point number.
    Floating(f64),
    /// An integer number.
    Integer(i64),
}

impl PartialEq for NumVal {
    fn eq(&self, other: &Self) -> bool {
        let tolerance = 1e-6; // Adjust this value as needed
        match (self, other) {
            (NumVal::Floating(fl), NumVal::Floating(fr)) => (fl - fr).abs() < tolerance,
            (NumVal::Floating(_), NumVal::Integer(_)) => false,
            (NumVal::Integer(_), NumVal::Floating(_)) => false,
            (NumVal::Integer(il), NumVal::Integer(ir)) => il == ir,
        }
    }
}

impl Eq for NumVal {}

impl NumVal {
    /// Converts a floating-point value to an integer by rounding.
    /// Integer values are returned unchanged.
    pub fn integerize_unit_val(self) -> NumVal {
        if let NumVal::Floating(f) = self {
            NumVal::Integer(f.round() as i64)
        } else {
            self
        }
    }

    /// Returns the value as an integer, converting from float if necessary.
    ///
    /// # Panics
    ///
    /// Panics if the conversion fails (should not happen after `integerize_unit_val`).
    pub fn as_int(&self) -> i64 {
        match self.integerize_unit_val() {
            NumVal::Integer(i) => i,
            _ => panic!("integerize did not integerize"),
        }
    }
}

/// Physical unit types supported by the language.
///
/// This enum distinguishes between supported physical dimensions
/// while the actual values are stored with full precision using the `uom` crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Mass,
    Angle,
    Frequency,
    Velocity,
    Acceleration,
    /// Time unit (stored internally using uom's Time type in seconds)
    Time,
    /// Length/distance unit (stored internally using uom's Length type in meters)
    Length,
}

/// A physical quantity with its unit, using the `uom` crate for type-safe dimensional analysis.
///
/// Values are stored directly as typed `uom` quantities preserving full type safety
/// and precision. No conversion or reconstruction is needed when accessing values.
#[derive(Debug, Clone, Copy)]
pub enum UnitVal {
    MassVal(Mass),
    AngleVal(Angle),
    FrequencyVal(Frequency),
    VelocityVal(Velocity),
    AccelerationVal(Acceleration),
    /// A length/distance value stored as uom::si::f64::Length
    LengthVal(Length),
    /// A time/duration value stored as uom::si::f64::Time  
    TimeVal(Time),
}

impl UnitVal {
    /// Creates a new length value from a `uom::si::f64::Length`
    pub fn from_length(length: Length) -> Self {
        Self::LengthVal(length)
    }

    /// Creates a new time value from a `uom::si::f64::Time`
    pub fn from_time(time: Time) -> Self {
        Self::TimeVal(time)
    }

    /// Returns the unit type (Length or Time)
    pub fn unit(&self) -> Unit {
        match self {
            Self::LengthVal(_) => Unit::Length,
            Self::MassVal(_) => Unit::Mass,
            Self::TimeVal(_) => Unit::Time,
            Self::AngleVal(_) => Unit::Angle,
            Self::FrequencyVal(_) => Unit::Frequency,
            Self::VelocityVal(_) => Unit::Velocity,
            Self::AccelerationVal(_) => Unit::Acceleration,
        }
    }

    /// Returns the value as a `uom::si::f64::Length` if this is a length unit.
    /// Returns None for other dimensions.
    pub fn as_length(&self) -> Option<Length> {
        match self {
            Self::LengthVal(l) => Some(*l),
            _ => None,
        }
    }

    /// Returns the value as a `uom::si::f64::Time` if this is a time unit.
    /// Returns None for other dimensions.
    pub fn as_time(&self) -> Option<Time> {
        match self {
            Self::TimeVal(t) => Some(*t),
            _ => None,
        }
    }
}

impl PartialEq for UnitVal {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::LengthVal(l1), Self::LengthVal(l2)) => {
                // Use relative tolerance for floating point comparison (in nanometers for precision)
                let tolerance: f64 = 1e-9;
                let v1 = l1.get::<nanometer>();
                let v2 = l2.get::<nanometer>();
                (v1 - v2).abs() < tolerance.max(v1.abs() * tolerance)
            }
            (Self::TimeVal(t1), Self::TimeVal(t2)) => {
                // Use relative tolerance for floating point comparison (in nanoseconds for precision)
                let tolerance: f64 = 1e-9;
                let v1 = t1.get::<nanosecond>();
                let v2 = t2.get::<nanosecond>();
                (v1 - v2).abs() < tolerance.max(v1.abs() * tolerance)
            }
            (a, b) if a.unit() == b.unit() => {
                let (a, b) = (a.si_value(), b.si_value());
                (a - b).abs() <= 1e-12_f64.max(a.abs().max(b.abs()) * 1e-12)
            }
            _ => false, // Different unit types are never equal
        }
    }
}

/// A value in the language, which can be a number (with or without units), string, or boolean.
#[derive(Debug, Clone, PartialEq)]
pub enum Val {
    /// A numeric value with a physical unit (using uom for precision).
    UnitedVal(UnitVal),
    /// A numeric value without units.
    NumVal(NumVal),
    /// A string value.
    StringVal(String),
    /// A boolean value.
    BoolVal(bool),
}

/// Right-hand side of an assignment, representing various expression types.
#[derive(Debug, Clone, PartialEq)]
pub enum Rhs {
    /// A range expression with optional lower and upper bounds.
    Range { from: Option<Val>, to: Option<Val> },
    /// An unbounded range expression (`..`).
    EmptyRange,
    /// A variable reference.
    Var(Var),
    /// A file path.
    Path(String),
    /// A literal value.
    Val(Val),
    /// An array of expressions.
    Array(Vec<Box<Self>>),
}

/// A variable reference with optional namespace qualification.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Var {
    /// Predefined variable (starts with `_`).
    Predef {
        /// Variable name (without the leading `_`).
        name: String,
        /// Namespace path.
        namespace: Vec<String>,
    },
    /// User-defined variable.
    User {
        /// Variable name.
        name: String,
        /// Namespace path.
        namespace: Vec<String>,
    },
}

fn vec_prepend<T: Clone>(prepend: &[T], source: &[T]) -> Vec<T> {
    let mut out = prepend.to_vec();
    out.extend(source.iter().cloned());
    out
}

impl Var {
    /// Prepends a namespace path to this variable's existing namespace.
    pub fn add_namespace(self, ns: &[String]) -> Self {
        match self {
            Var::User { name, namespace } => Var::User {
                name,
                namespace: vec_prepend(ns, &namespace),
            },
            Var::Predef { name, namespace } => Var::Predef {
                name,
                namespace: vec_prepend(ns, &namespace),
            },
        }
    }
}

fn format_var(var: &Var) -> String {
    let mut path = match var {
        Var::Predef { name: _, namespace } | Var::User { name: _, namespace } => {
            namespace.join(".")
        }
    };

    if !path.is_empty() {
        path.push('.');
    }

    match var {
        Var::Predef { name, .. } => {
            path.push('_');
            path.push_str(name);
        }
        Var::User { name, .. } => {
            path.push_str(name);
        }
    }

    path
}

impl FromStr for Var {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let splitted = s.split(".").collect::<Vec<_>>();

        let l = splitted.len();
        if l < 1 {
            return Err(anyhow!("Nothing to parse"));
        }

        let last = splitted[l - 1];

        Ok({
            let mut v: Vec<String> = vec![];
            if l > 1 {
                v.extend(splitted.into_iter().take(l - 1).map(String::from));
            }
            if let Some(predef) = last.strip_prefix("_") {
                Self::Predef {
                    name: predef.to_owned(),
                    namespace: v,
                }
            } else {
                Self::User {
                    name: last.to_owned(),
                    namespace: v,
                }
            }
        })
    }
}

/// The result of compiling and evaluating ratslang code.
#[derive(Debug, Clone)]
pub struct Evaluated {
    /// The variable history containing all defined variables and their values.
    pub vars: VariableHistory,
    /// Original source code for comment-preserving filtering
    pub source_text: String,
    /// Maps annotation names to line ranges (0-indexed, inclusive)
    pub annotations: HashMap<String, Vec<(usize, usize)>>,
}

impl Default for Evaluated {
    fn default() -> Self {
        Self::new()
    }
}

impl Evaluated {
    /// Creates a new empty evaluation result.
    pub fn new() -> Self {
        Self {
            vars: VariableHistory::new(vec![]),
            source_text: String::new(),
            annotations: HashMap::new(),
        }
    }

    /// Filters the configuration to include only statements with a specific annotation,
    /// preserving all comments and namespace context. Returns a string representation of the filtered config.
    ///
    /// Returns a string containing the filtered configuration with comments preserved, or None if annotation not found.
    pub fn to_string_filtered(&self, annotation: &str) -> Option<String> {
        self.annotations.get(annotation).map(|line_ranges| {
            let lines: Vec<&str> = self.source_text.lines().collect();

            // Precompute namespace stack per line
            let mut ns_stack: Vec<String> = Vec::new();
            let mut ns_for_line: Vec<Vec<String>> = Vec::with_capacity(lines.len());
            for line in &lines {
                let trimmed = line.trim();

                if trimmed.starts_with('}') {
                    ns_stack.pop();
                }

                ns_for_line.push(ns_stack.clone());

                if trimmed.ends_with('{') {
                    let name = trimmed.trim_end_matches('{').trim();
                    if !name.is_empty() {
                        ns_stack.push(name.to_string());
                    }
                }
            }

            // Collect content lines per namespace path
            #[derive(Default)]
            struct Node {
                entries: Vec<String>,
                children: std::collections::BTreeMap<String, Node>,
            }

            let mut root = Node::default();

            for (start_line, end_line) in line_ranges {
                let start = (*start_line).min(lines.len());
                let end = (*end_line + 1).min(lines.len());

                for line_idx in start..end {
                    let line = lines[line_idx];
                    let trimmed = line.trim();

                    // Skip structural lines and annotation markers
                    if trimmed.is_empty()
                        || trimmed.starts_with('}')
                        || trimmed.ends_with('{')
                        || (trimmed.starts_with('#') && trimmed.contains('@'))
                    {
                        continue;
                    }

                    let content = line.trim_start().to_string();
                    let ns_path = &ns_for_line[line_idx];

                    // Insert into tree
                    let mut node = &mut root;
                    for part in ns_path {
                        node = node.children.entry(part.clone()).or_default();
                    }
                    node.entries.push(content);
                }
            }

            // Render tree starting from root
            fn render_child(child: &Node, name: &str, depth: usize, out: &mut String) {
                let indent = "  ";

                // Helper to check if a node is a collapsible chain
                fn is_collapsible_chain(node: &Node) -> bool {
                    let assignment_count = node.entries.iter().filter(|e| e.contains('=')).count();
                    if node.children.is_empty()
                        && !node.entries.is_empty()
                        && assignment_count == 1
                        && node.entries.iter().all(|e| {
                            let t = e.trim_start();
                            t.starts_with('#') || t.contains('=')
                        })
                    {
                        return true;
                    }

                    if node.entries.is_empty() && node.children.len() == 1 {
                        let (_, child) = node.children.iter().next().unwrap();
                        return is_collapsible_chain(child);
                    }

                    false
                }

                // Helper to render a collapsible chain as dotted notation
                fn render_collapsible_dotted(
                    node: &Node,
                    path: &str,
                    depth: usize,
                    out: &mut String,
                ) {
                    let indent = "  ";
                    let assignment_count = node.entries.iter().filter(|e| e.contains('=')).count();

                    // Base case: leaf with single assignment
                    if node.children.is_empty() && assignment_count == 1 {
                        let mut pending_comments = Vec::new();
                        for entry in &node.entries {
                            if entry.trim_start().starts_with('#') {
                                pending_comments.push(entry.as_str());
                            } else if entry.contains('=') {
                                for comment in &pending_comments {
                                    out.push_str(&indent.repeat(depth));
                                    out.push_str(comment);
                                    out.push('\n');
                                }
                                pending_comments.clear();

                                if let Some(hash_pos) = entry.find('#') {
                                    let code_part = entry[..hash_pos].trim_end();
                                    let comment_part = entry[hash_pos..].trim();

                                    if !code_part.is_empty() {
                                        out.push_str(&indent.repeat(depth));
                                        out.push_str(comment_part);
                                        out.push('\n');
                                        out.push_str(&indent.repeat(depth));
                                        out.push_str(path);
                                        out.push('.');
                                        out.push_str(code_part);
                                        out.push('\n');
                                        return;
                                    }
                                }

                                out.push_str(&indent.repeat(depth));
                                out.push_str(path);
                                out.push('.');
                                out.push_str(entry);
                                out.push('\n');
                            }
                        }
                        return;
                    }

                    // Recursive case: single child intermediate node
                    if node.entries.is_empty() && node.children.len() == 1 {
                        let (child_name, child_node) = node.children.iter().next().unwrap();
                        let new_path = format!("{}.{}", path, child_name);
                        render_collapsible_dotted(child_node, &new_path, depth, out);
                    }
                }

                // If this node is collapsible, render as dotted notation
                if is_collapsible_chain(child) {
                    if !out.is_empty() && !out.ends_with("\n\n") {
                        out.push('\n');
                    }
                    render_collapsible_dotted(child, name, depth, out);
                    return;
                }

                // Intermediate single-child but not collapsible: just pass through
                if child.entries.is_empty() && child.children.len() == 1 {
                    let (grandchild_name, grandchild) = child.children.iter().next().unwrap();
                    let collapsed_name = format!("{}.{}", name, grandchild_name);
                    render_child(grandchild, &collapsed_name, depth, out);
                    return;
                }

                // Multi-entry or multi-child: render as block
                if !out.is_empty() && !out.ends_with("\n\n") {
                    out.push('\n');
                }

                out.push_str(&indent.repeat(depth));
                out.push_str(name);
                out.push_str(" {\n");

                let mut first_entry = true;
                let mut prev_is_comment = false;
                for entry in &child.entries {
                    let is_comment = entry.trim_start().starts_with('#');
                    if !first_entry && !prev_is_comment && is_comment {
                        out.push('\n');
                    }
                    first_entry = false;
                    prev_is_comment = is_comment;

                    out.push_str(&indent.repeat(depth + 1));
                    out.push_str(entry);
                    out.push('\n');
                }

                for (child_name, grandchild) in &child.children {
                    render_child(grandchild, child_name, depth + 1, out);
                }

                out.push_str(&indent.repeat(depth));
                out.push_str("}\n");
            }
            let mut result = String::new();

            // Render root entries
            let mut first_entry = true;
            let mut prev_is_comment = false;
            for entry in &root.entries {
                let is_comment = entry.trim_start().starts_with('#');
                if !first_entry && !prev_is_comment && is_comment {
                    result.push('\n');
                }
                first_entry = false;
                prev_is_comment = is_comment;

                result.push_str(entry);
                result.push('\n');
            }

            // Render root children
            for (name, child) in &root.children {
                render_child(child, name, 0, &mut result);
            }
            result
        })
    }
}

/// The root of the abstract syntax tree.
#[derive(Debug, Clone)]
pub struct Root {
    /// List of top-level statements (variable definitions).
    pub vardefs: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub enum StatementKind {
    VariableDef(Statement),
    VariableNamespaceBlock(Vec<Statement>),
}

#[derive(Debug, Clone)]
pub enum StatementKindPass1 {
    VariableDef(Statement),
    Include {
        namespace: Vec<String>,
        path: String,
    },
    VariableNamespaceBlock {
        ns: Vec<String>,
        stmts: Vec<Box<StatementKindPass1>>,
    },
}

#[derive(Debug, Clone)]
pub enum StatementKindOwnedPass1 {
    Include {
        namespace: Vec<String>,
        path: String,
    },
    VariableDef(Statement),
    VariableNamespaceBlock {
        ns: Vec<String>,
        stmts: Vec<Box<StatementKindOwnedPass1>>,
    },
}

#[derive(Debug, Clone)]
pub enum StatementKindOwned {
    VariableDef(Statement),
}

#[allow(unreachable_patterns)] // remove when adding more statments than just variables
fn do_pass1(
    pass1: Vec<StatementKindOwnedPass1>,
    current_parent: &std::path::Path,
    spans: &mut impl Iterator<Item = std::ops::Range<usize>>,
    context: &provenance::Context<'_>,
) -> anyhow::Result<provenance::Parsed> {
    let mut out = vec![];
    let mut locations = vec![];

    for stmt in pass1 {
        match stmt {
            StatementKindOwnedPass1::VariableDef(statement) => {
                let span = spans
                    .next()
                    .ok_or_else(|| anyhow!("missing assignment location"))?;
                locations.push((context.location(span), context.included_from.clone()));
                out.push(StatementKindOwned::VariableDef(statement));
            }
            StatementKindOwnedPass1::VariableNamespaceBlock { ns, stmts } => {
                let v = stmts.into_iter().map(|f| *f).collect::<Vec<_>>();

                let passed = do_pass1(v, current_parent, spans, context)?;
                locations.extend(passed.locations);
                out.extend(
                    passed
                        .ast
                        .into_iter()
                        .map(|t| match t {
                            StatementKindOwned::VariableDef(statement) => Ok(
                                StatementKindOwned::VariableDef(statement.add_namespace(&ns)),
                            ),
                            _ => Err(anyhow!(
                                "Only variable statements allowed inside a namespace block."
                            )),
                        })
                        .try_fold(Vec::new(), |mut acc, res| match res {
                            Ok(ok) => {
                                acc.push(ok);
                                Ok(acc)
                            }
                            Err(e) => Err(e),
                        })?,
                );
            }
            StatementKindOwnedPass1::Include { namespace, path } => {
                let to_parse = PathBuf::from_str(&path)?;
                let to_parse = if to_parse.is_relative() {
                    if to_parse.is_dir() {
                        return Err(anyhow!("File is a directory: {}", to_parse.display()));
                    } else {
                        current_parent.join(path).to_owned()
                    }
                } else {
                    to_parse
                };
                let span = spans
                    .next()
                    .ok_or_else(|| anyhow!("missing include location"))?;
                let mut included_from = context.included_from.clone();
                included_from.push(context.location(span));
                anyhow::ensure!(
                    included_from.len() <= 64,
                    "include nesting exceeds 64 files"
                );
                let nast = parse_to_ast(&to_parse, included_from)?;
                locations.extend(nast.locations);
                out.extend(ast_add_namespace(&namespace, nast.ast));
            }
        }
    }

    Ok(provenance::Parsed {
        ast: out,
        locations,
    })
}

fn parse_to_ast(
    path: &std::path::Path,
    included_from: Vec<SourceLocation>,
) -> anyhow::Result<provenance::Parsed> {
    let file = std::fs::read_to_string(path);
    let file = match file {
        Ok(file) => Ok(file),
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => Err(anyhow!("Could not find file: {}", path.display())),
            std::io::ErrorKind::PermissionDenied => {
                Err(anyhow!("No permission to read file: {}", path.display()))
            }
            std::io::ErrorKind::IsADirectory => {
                Err(anyhow!("File is a directory: {}", path.display()))
            }
            _ => Err(anyhow!("Unexpected error for file: {}", path.display())),
        },
    }?;

    let file = if file.ends_with('\n') {
        file
    } else {
        file + "\n"
    };

    // parse file to be included recursively
    let token_iter = Token::lexer(&file).spanned().map(|(tok, span)| match tok {
        Ok(tok) => (tok, span.into()),
        Err(()) => (Token::Error, span.into()),
    });
    let token_stream =
        Stream::from_iter(token_iter).map((0..file.len()).into(), |(t, s): (_, _)| (t, s));
    let locations: provenance::Locations = Default::default();
    match parser_with_locations(&file, Some(locations.clone()))
        .parse(token_stream)
        .into_result()
    {
        Ok(sexpr) => {
            let mut ast: Vec<StatementKindOwnedPass1> = Vec::new();
            for expr in sexpr {
                ast.push(expr.into());
            }

            let ast = do_pass1(
                ast,
                path.parent()
                    .ok_or(anyhow!("Can not get parent directory of file."))?,
                &mut locations.borrow().values().cloned(),
                &provenance::Context {
                    source: &file,
                    file: Some(path.to_path_buf()),
                    byte_offset: 0,
                    line_offset: 0,
                    included_from,
                },
            )?; // recursive call
            return Ok(ast);
        }
        Err(errs) => {
            for err in errs {
                Report::build(ReportKind::Error, err.span().into_range())
                    .with_code(3)
                    .with_message(err.to_string())
                    .with_label(
                        Label::new(err.span().into_range())
                            .with_message(err.reason().to_string())
                            .with_color(Color::Red),
                    )
                    .finish()
                    .eprint(Source::from(&file))
                    .unwrap();
            }
        }
    }

    Err(anyhow!("Could not parse ratslang code."))
}

impl From<StatementKindPass1> for StatementKindOwnedPass1 {
    fn from(value: StatementKindPass1) -> Self {
        match value {
            StatementKindPass1::VariableNamespaceBlock { ns, stmts } => {
                StatementKindOwnedPass1::VariableNamespaceBlock {
                    ns: ns.into_iter().collect::<Vec<_>>(),
                    stmts: stmts
                        .into_iter()
                        .map(|bstmt| Box::new(StatementKindOwnedPass1::from(*bstmt)))
                        .collect::<Vec<_>>(),
                }
            }
            StatementKindPass1::VariableDef(statement) => {
                StatementKindOwnedPass1::VariableDef(statement)
            }
            StatementKindPass1::Include { namespace, path } => {
                StatementKindOwnedPass1::Include { namespace, path }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum StatementPass1 {
    AssignLeft {
        lhs: Vec<Var>,
        rhs: Rhs,
    },
    AssignRight {
        lhs: Rhs,
        rhs: Vec<Var>,
    },
    Include {
        namespace: Vec<String>,
        path: String,
    },
}

/// A statement in the language (currently only variable assignments).
#[derive(Debug, Clone)]
pub enum Statement {
    /// Assignment statement with one or more variables on the left and expressions on the right.
    AssignLeft {
        /// Variables being assigned to.
        lhs: Vec<Var>,
        /// Expressions being assigned.
        rhs: Vec<Rhs>,
        /// Optional annotations for this statement (e.g., "minimal")
        annotations: Vec<String>,
    },
}

impl Statement {
    fn add_namespace(self, ns: &[String]) -> Self {
        match self {
            Statement::AssignLeft {
                lhs,
                rhs,
                annotations,
            } => {
                let nlhs = lhs
                    .into_iter()
                    .map(|v| v.add_namespace(ns))
                    .collect::<Vec<_>>();
                Statement::AssignLeft {
                    lhs: nlhs,
                    rhs,
                    annotations,
                }
            }
        }
    }
}

#[allow(unreachable_patterns)] // remove when adding more statments than just variables
fn ast_add_namespace(ns: &[String], ast: Vec<StatementKindOwned>) -> Vec<StatementKindOwned> {
    ast.into_iter()
        .map(|stmt| match stmt {
            StatementKindOwned::VariableDef(statement) => {
                StatementKindOwned::VariableDef(statement.add_namespace(ns))
            }
            _ => stmt,
        })
        .collect()
}

#[derive(Clone, Copy, Debug)]
pub enum Operator {
    Left, // "="
}

fn parser<'a, I>(
    source: &'a str,
) -> impl Parser<'a, I, Vec<StatementKindPass1>, extra::Err<Rich<'a, Token<'a>>>>
where
    I: ValueInput<'a, Token = Token<'a>, Span = SimpleSpan>,
{
    parser_with_locations(source, None)
}
fn parser_with_locations<'a, I>(
    source: &'a str,
    locations: Option<provenance::Locations>,
) -> impl Parser<'a, I, Vec<StatementKindPass1>, extra::Err<Rich<'a, Token<'a>>>>
where
    I: ValueInput<'a, Token = Token<'a>, Span = SimpleSpan>,
{
    let newlines = just(Token::NewLine).repeated();

    let mut file_content = Recursive::declare();

    let rhs = recursive(|rhs| {
        let val = {
            // Time values: directly from uom Time token
            let physical = select! { Token::Physical(value) => Val::UnitedVal(value) };
            let time = select! {
                Token::Time(t) => t,
            }
            .map(|t| Val::UnitedVal(UnitVal::from_time(t)));

            // Length values: directly from uom Length token
            let way = select! {
                Token::Length(l) => l,
            }
            .map(|l| Val::UnitedVal(UnitVal::from_length(l)));

            let number = select! {
                Token::IntegerNumber(i) => NumVal::Integer(i),
                Token::FloatingNumber(f) => NumVal::Floating(f),
            }
            .map(Val::NumVal);

            choice((time, way, number, physical)).try_map(|value, span| {
                value
                    .validate_finite()
                    .map_err(|error| Rich::custom(span, error.to_string()))?;
                Ok(value)
            })
        };

        let bounded_range = val
            .then_ignore(just(Token::OpRange))
            .then(val)
            .try_map(|(from, to), span| {
                if let (Val::NumVal(from_n), Val::NumVal(to_n)) = (&from, &to) {
                    if std::mem::discriminant(from_n) != std::mem::discriminant(to_n) {
                        return Err(Rich::custom(
                            span,
                            "Cannot create a range between an integer and a float.",
                        ));
                    }
                }
                Ok(Rhs::Range {
                    from: Some(from),
                    to: Some(to),
                })
            });

        let lower_bounded_range =
            val
                .then_ignore(just(Token::OpRange))
                .map(|from| Rhs::Range {
                    from: Some(from),
                    to: None,
                });

        let upper_bounded_range =
            just(Token::OpRange)
                .ignore_then(val)
                .map(|to| Rhs::Range {
                    from: None,
                    to: Some(to),
                });

        let empty_range = just(Token::OpRange).to(Rhs::EmptyRange);

        let primitive = select! {
            Token::String(s) => Rhs::Val(Val::StringVal(s.to_owned())),
            Token::True => Rhs::Val(Val::BoolVal(true)),
            Token::False => Rhs::Val(Val::BoolVal(false)),
        };

        let var_path = select! { Token::Identifier(ident) => ident.to_string() }
            .separated_by(just(Token::Dot))
            .at_least(1)
            .collect::<Vec<String>>();

        let variable = var_path.map(|path| {
            let name = path.last().unwrap().clone();
            let namespace = path.iter().take(path.len() - 1).cloned().collect();
            if let Some(name) = name.strip_prefix('_') {
                Rhs::Var(Var::Predef {
                    name: name.to_string(),
                    namespace,
                })
            } else {
                Rhs::Var(Var::User { name, namespace })
            }
        });

        let array = rhs
            .clone()
            .separated_by(just(Token::Comma).then_ignore(newlines.clone()))
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(just(Token::LBracket), just(Token::RBracket))
            .map(|arr| Rhs::Array(arr.into_iter().map(Box::new).collect()));

        choice((
            bounded_range,
            lower_bounded_range,
            upper_bounded_range,
            empty_range,
            val.map(Rhs::Val),
            primitive,
            select! { Token::Path(p) => Rhs::Path(p.to_owned()) },
            variable,
            array,
        ))
    });

    let multi_rhs = rhs
        .separated_by(just(Token::Comma))
        .at_least(1)
        .collect::<Vec<_>>();

    let op = select! { Token::OpAssignToLeft => Operator::Left };

    let lhs_paths = select! { Token::Identifier(ident) => ident.to_string() }
        .separated_by(just(Token::Dot))
        .at_least(1)
        .collect::<Vec<String>>()
        .separated_by(just(Token::Comma))
        .at_least(1)
        .collect::<Vec<Vec<String>>>();

    let include_locations = locations.clone();
    let include = just(Token::OpAssignToLeft)
        .ignore_then(select! { Token::Path(p) => p.to_owned() })
        .then_ignore(just(Token::NewLine).repeated().at_least(1).or(end()))
        .try_map(move |path, span: SimpleSpan| {
            constraints::check_assignment_target(source, span.start())
                .map_err(|error| Rich::custom(span, error.to_string()))?;
            if let Some(locations) = &include_locations {
                locations
                    .borrow_mut()
                    .insert(span.start(), span.into_range());
            }
            Ok(StatementKindPass1::Include {
                namespace: vec![],
                path,
            })
        });

    let block = file_content
        .clone()
        .repeated()
        .collect::<Vec<_>>()
        .delimited_by(
            just(Token::BlockStart).then_ignore(newlines.clone()),
            just(Token::BlockEnd),
        );

    let statement_starting_with_path = lhs_paths
        .then(
            op.then(multi_rhs)
                .then_ignore(just(Token::NewLine).repeated().at_least(1).or(end()))
                .map(|(op, rhs)| (Some((op, rhs)), None))
                .or(block.map(|stmts| (None, Some(stmts)))),
        )
        .try_map(
            move |(lhs_paths, (assignment_opt, block_opt)), span: SimpleSpan| {
                if let Some(stmts) = block_opt {
                    constraints::check_assignment_target(source, span.start())
                        .map_err(|error| Rich::custom(span, error.to_string()))?;
                    if lhs_paths.len() > 1 {
                        return Err(Rich::custom(
                            span,
                            "Namespace blocks cannot be defined for multiple paths at once.",
                        ));
                    }
                    Ok(StatementKindPass1::VariableNamespaceBlock {
                        ns: lhs_paths.into_iter().next().unwrap(),
                        stmts: stmts.into_iter().map(Box::new).collect(),
                    })
                } else if let Some((op, rhs)) = assignment_opt {
                    let vars = lhs_paths
                        .into_iter()
                        .map(|path| {
                            let name = path.last().unwrap().clone();
                            let namespace = path.iter().take(path.len() - 1).cloned().collect();
                            if let Some(name) = name.strip_prefix('_') {
                                Var::Predef {
                                    name: name.to_string(),
                                    namespace,
                                }
                            } else {
                                Var::User { name, namespace }
                            }
                        })
                        .collect::<Vec<Var>>();

                    match op {
                        Operator::Left => {
                            if rhs.len() > 1 && vars.len() != rhs.len() {
                                return Err(Rich::custom(
                                    span,
                                    "Mismatched number of variables and values in assignment.",
                                ));
                            }
                            if rhs.is_empty() {
                                return Err(Rich::custom(span, "Missing right operand."));
                            }
                            if let Some(locations) = &locations {
                                locations
                                    .borrow_mut()
                                    .insert(span.start(), span.into_range());
                            }
                            Ok(StatementKindPass1::VariableDef(Statement::AssignLeft {
                                lhs: vars,
                                rhs,
                                annotations: constraints::annotations_before(source, span.start()),
                            }))
                        }
                    }
                } else {
                    unreachable!()
                }
            },
        );

    file_content
        .define(choice((include, statement_starting_with_path)).then_ignore(newlines.clone()));

    newlines
        .clone()
        .ignore_then(file_content.repeated().collect())
        .then_ignore(end())
}

/// Compiles a ratslang source file.
///
/// # Arguments
///
/// * `path` - Path to the source file
/// * `start_line` - Optional starting line number (1-indexed, inclusive)
/// * `end_line` - Optional ending line number (1-indexed, inclusive)
///
/// # Returns
///
/// Returns an `Evaluated` result containing the parsed and resolved variables,
/// or an error if parsing fails.
///
/// # Examples
///
/// ```rust,no_run
/// use std::path::PathBuf;
/// use ratslang::compile_file;
///
/// let path = PathBuf::from("config.rl");
/// let result = compile_file(&path, None, None).unwrap();
/// ```
pub fn compile_file(
    path: &std::path::Path,
    start_line: Option<usize>,
    end_line: Option<usize>,
) -> anyhow::Result<Evaluated> {
    compile_file_with_state(path, start_line, end_line, None, std::io::stderr(), true)
}

/// Compiles a ratslang source file with existing variable state.
///
/// # Arguments
///
/// * `path` - Path to the source file
/// * `start_line` - Optional starting line number (1-indexed, inclusive)
/// * `end_line` - Optional ending line number (1-indexed, inclusive)
/// * `var_state` - Optional pre-existing variable state to build upon
/// * `out` - Writer for error output
/// * `rich_out` - Whether to use rich formatted error output
///
/// # Returns
///
/// Returns an `Evaluated` result containing the parsed and resolved variables,
/// or an error if parsing fails.
pub fn compile_file_with_state(
    path: &std::path::Path,
    start_line: Option<usize>,
    end_line: Option<usize>,
    var_state: Option<HashMap<Var, Rhs>>,
    out: impl std::io::Write,
    rich_out: bool,
) -> anyhow::Result<Evaluated> {
    let file = std::fs::read_to_string(path);
    let file = match file {
        Ok(file) => Ok(file),
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => Err(anyhow!("Could not find file: {}", path.display())),
            std::io::ErrorKind::PermissionDenied => {
                Err(anyhow!("No permission to read file: {}", path.display()))
            }
            std::io::ErrorKind::IsADirectory => {
                Err(anyhow!("File is a directory: {}", path.display()))
            }
            _ => Err(anyhow!("Unexpected error for file: {}", path.display())),
        },
    }?;
    let start_line = start_line.map(|l| l.saturating_sub(1)).unwrap_or_default();
    let lines = file.split_inclusive('\n');
    let end_line = end_line.unwrap_or(lines.clone().count()).saturating_sub(1);
    let selected_lines = lines
        .skip(start_line)
        .take(end_line.saturating_add(1).saturating_sub(start_line))
        .collect::<String>()
        + "\n"; // Keep original CRLF and byte positions; ensure a final newline.

    let dir = path.parent().ok_or(anyhow!(
        "Could not get parent directory of source code file."
    ))?;
    let offset = file
        .split_inclusive('\n')
        .take(start_line)
        .map(str::len)
        .sum();
    let result = compile_with_context(
        &selected_lines,
        dir,
        var_state,
        out,
        rich_out,
        provenance::Context {
            source: &selected_lines,
            file: Some(path.to_path_buf()),
            byte_offset: offset,
            line_offset: start_line,
            included_from: vec![],
        },
    )?;
    Ok(result)
}

/// Tracks the history of variable definitions and provides resolution capabilities.
#[derive(Debug, Clone)]
pub struct VariableHistory {
    ast: Vec<StatementKindOwned>,
    /// Cache of resolved variables for faster lookups.
    pub var_cache: HashMap<Var, Rhs>,
    usage: Rc<RefCell<UsageStats>>,
    warn_on_drop: bool,
    /// The namespace context for this filtered view. When resolving variables,
    /// this namespace is prepended to variable names.
    namespace_context: Vec<String>,
    trace: Rc<provenance::Trace>,
}

fn is_sub_namespace(sub: &[String], super_namespace: &[String]) -> bool {
    if sub.len() > super_namespace.len() {
        return false;
    }

    if sub.is_empty() {
        return false; // Empty is not a sub-namespace
    }

    if super_namespace.is_empty() {
        return false;
    }

    for i in 0..sub.len() {
        if sub[i] != super_namespace[i] {
            return false;
        }
    }

    true
}

fn remove_prefix_from_target(prefix_definer: &[String], target_vec: &[String]) -> Vec<String> {
    let prefix_len = prefix_definer.len();
    let target_len = target_vec.len();
    let compare_len = std::cmp::min(prefix_len, target_len);

    let mut common_prefix_len = 0;
    for i in 0..compare_len {
        if prefix_definer[i] == target_vec[i] {
            common_prefix_len += 1;
        } else {
            break;
        }
    }

    target_vec[common_prefix_len..].to_vec()
}

fn truncate_namespace_var(ns: &[String], var: &Var) -> Var {
    match var {
        Var::User { name, namespace } => Var::User {
            name: name.clone(),
            namespace: { remove_prefix_from_target(ns, namespace) },
        },
        Var::Predef { name, namespace } => Var::Predef {
            name: name.clone(),
            namespace: { remove_prefix_from_target(ns, namespace) },
        },
    }
}

fn truncate_namespace_rhs(ns: &[String], rhs: &Rhs) -> Rhs {
    match rhs {
        Rhs::Array(items) => Rhs::Array(
            items
                .iter()
                .map(|arr_item| Box::new(truncate_namespace_rhs(ns, arr_item)))
                .collect::<Vec<_>>(),
        ),
        Rhs::Range { from: _, to: _ } => rhs.clone(),
        Rhs::EmptyRange => Rhs::EmptyRange,
        Rhs::Var(var) => Rhs::Var(truncate_namespace_var(ns, var)),
        Rhs::Path(_) => rhs.clone(),
        Rhs::Val(_) => rhs.clone(),
    }
}

fn truncate_namespace_stmt(ns: &[String], stmt: &Statement) -> Statement {
    match stmt {
        Statement::AssignLeft {
            lhs,
            rhs,
            annotations,
        } => {
            let l = lhs
                .iter()
                .filter(|v| match v {
                    Var::Predef {
                        name: _,
                        namespace: var_ns,
                    } => is_sub_namespace(ns, var_ns),
                    Var::User {
                        name: _,
                        namespace: var_ns,
                    } => is_sub_namespace(ns, var_ns),
                })
                .map(|l| truncate_namespace_var(ns, l))
                .collect::<Vec<_>>();
            let r = rhs
                .iter()
                .map(|r_item| truncate_namespace_rhs(ns, r_item))
                .collect();

            Statement::AssignLeft {
                lhs: l,
                rhs: r,
                annotations: annotations.clone(),
            }
        }
    }
}

fn truncate_namespace(ns: &[String], stmt: &StatementKindOwned) -> StatementKindOwned {
    match stmt {
        StatementKindOwned::VariableDef(statement) => {
            StatementKindOwned::VariableDef(truncate_namespace_stmt(ns, statement))
        }
    }
}
#[derive(Debug, Default, Clone)]
struct UsageStats {
    assigned: HashSet<Var>,
    evaluated: HashSet<Var>,
}

impl UsageStats {
    fn record_assignment<'a, I>(&mut self, vars: I)
    where
        I: IntoIterator<Item = &'a Var>,
    {
        for var in vars {
            self.assigned.insert(var.clone());
        }
    }

    fn record_evaluation(&mut self, var: &Var) {
        self.evaluated.insert(var.clone());
    }
}

fn stmt_in_ns(namespace: &[String], stmt: &Statement) -> bool {
    match stmt {
        Statement::AssignLeft { lhs, .. } => lhs.iter().any(|r| match r {
            Var::Predef {
                name: _,
                namespace: var_ns,
            } => is_sub_namespace(namespace, var_ns),
            Var::User {
                name: _,
                namespace: var_ns,
            } => is_sub_namespace(namespace, var_ns),
        }),
    }
}

impl VariableHistory {
    /// Creates a new variable history from an AST.
    pub fn new(ast: Vec<StatementKindOwned>) -> Self {
        let mut usage = UsageStats::default();
        for stmt in ast.iter() {
            #[allow(irrefutable_let_patterns)]
            // remove when adding more statments than just variables
            if let StatementKindOwned::VariableDef(Statement::AssignLeft { lhs, .. }) = stmt {
                usage.record_assignment(lhs);
            }
        }

        VariableHistory {
            ast,
            var_cache: HashMap::new(),
            usage: Rc::new(RefCell::new(usage)),
            warn_on_drop: true,
            namespace_context: vec![],
            trace: Rc::new(provenance::Trace::default()),
        }
    }

    /// Sets the initial variable state, useful for REPL or incremental compilation.
    pub fn with_state(mut self, state: HashMap<Var, Rhs>) -> Self {
        self.var_cache = state;
        {
            let mut usage = self.usage.borrow_mut();
            usage.record_assignment(self.var_cache.keys());
        }
        self
    }

    pub fn with_drop_warning(mut self, warn: bool) -> Self {
        self.warn_on_drop = warn;
        self
    }

    /// Populates the variable cache by evaluating all statements in the AST.
    pub fn populate_cache(&mut self) {
        for StatementKindOwned::VariableDef(statement) in &self.ast {
            let Statement::AssignLeft { lhs, rhs, .. } = statement;
            if rhs.len() == 1 {
                // Broadcast single RHS to all LHS
                let rhs_val = rhs.first().unwrap();
                for l in lhs {
                    self.var_cache.insert(l.clone(), rhs_val.clone());
                }
            } else {
                // Pair LHS with RHS
                for (l, r) in lhs.iter().zip(rhs.iter()) {
                    self.var_cache.insert(l.clone(), r.clone());
                }
            }
        }
    }

    fn filter_by_namespace(
        &self,
        namespace: &[String],
        up_to: Option<usize>,
        trunc_ns: bool,
    ) -> Vec<StatementKindOwned> {
        let mut vars_in_ns = vec![];

        // use previous cached vars as well
        for (var, rhs) in self.var_cache.iter() {
            let mut wvar = var.clone();
            if is_sub_namespace(
                namespace,
                match var {
                    Var::Predef { name: _, namespace } => namespace,
                    Var::User { name: _, namespace } => namespace,
                },
            ) {
                if trunc_ns {
                    wvar = truncate_namespace_var(namespace, &wvar);
                }

                vars_in_ns.push(StatementKindOwned::VariableDef(Statement::AssignLeft {
                    lhs: vec![wvar],
                    rhs: vec![rhs.clone()],
                    annotations: vec![],
                }));
            }
        }

        for stmt in self.ast.iter().take(up_to.unwrap_or(usize::MAX)) {
            match &stmt {
                StatementKindOwned::VariableDef(rstmt) => {
                    if stmt_in_ns(namespace, rstmt) {
                        let rhs = if trunc_ns {
                            truncate_namespace(namespace, stmt)
                        } else {
                            stmt.clone()
                        };
                        vars_in_ns.push(rhs);
                    }
                }
            }
        }

        vars_in_ns
    }

    fn resolve_recursive(&self, var: &Var, up_to: Option<usize>) -> anyhow::Result<Option<Rhs>> {
        // use previous state if already resolved there
        if let Some(cache_hit) = self.var_cache.get(var) {
            return Ok(Some(cache_hit.clone()));
        }

        let mut val = None;
        for (i, stmt) in self
            .ast
            .iter()
            .enumerate()
            .take(up_to.unwrap_or(usize::MAX))
        {
            match &stmt {
                &StatementKindOwned::VariableDef(statement) => match statement {
                    Statement::AssignLeft { lhs, rhs, .. } => {
                        if let Some(index) = lhs.iter().position(|l| l == var) {
                            let rhs_to_process = if rhs.len() == 1 {
                                rhs.first()
                            } else {
                                rhs.get(index)
                            };

                            if let Some(rhs_val) = rhs_to_process {
                                val = Some(match rhs_val {
                                    Rhs::Var(var_to_resolve) => {
                                        match self.resolve_recursive(var_to_resolve, Some(i))? {
                                            None => {
                                                Rhs::Val(Val::StringVal(match var_to_resolve {
                                                    Var::User { name, namespace: _ } => {
                                                        name.clone()
                                                    }
                                                    Var::Predef { .. } => {
                                                        return Err(anyhow!(
                                                            "Predef variables can not be assigned."
                                                        ));
                                                    }
                                                }))
                                            }
                                            Some(resolved_rhs) => resolved_rhs,
                                        }
                                    }
                                    _ => rhs_val.clone(),
                                });
                            }
                        }
                    }
                },
            }
        }
        Ok(val)
    }

    fn prepend_ns(var: &Var, ns: &[String]) -> Var {
        match var {
            Var::Predef {
                name,
                namespace: var_ns,
            } => {
                let v = vec_prepend(ns, var_ns);

                Var::Predef {
                    name: name.clone(),
                    namespace: v,
                }
            }
            Var::User {
                name,
                namespace: var_ns,
            } => {
                let v = vec_prepend(ns, var_ns);

                Var::User {
                    name: name.clone(),
                    namespace: v,
                }
            }
        }
    }

    /// Resolves all variables within a specific namespace.
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace path to filter by
    ///
    /// # Returns
    ///
    /// A vector of tuples containing variables and their resolved values within the namespace.
    pub fn resolve_ns(&self, namespace: &[&str]) -> Vec<(Var, Rhs)> {
        let owned_ns = namespace
            .iter()
            .map(|s| (*s).to_owned())
            .collect::<Vec<_>>();
        self.filter_by_namespace(&owned_ns, None, true)
            .iter()
            .flat_map(|stmt| match stmt {
                StatementKindOwned::VariableDef(statement) => match statement {
                    Statement::AssignLeft { lhs, .. } => lhs
                        .iter()
                        .map(|lh| {
                            (
                                lh,
                                self.resolve_var(&Self::prepend_ns(lh, &owned_ns))
                                    .expect("Should have failed in Pass 2"),
                            )
                        })
                        .collect::<Vec<_>>(),
                },
            })
            .fold(Vec::new(), |mut acc, res| match res {
                (var, Some(rhs)) => {
                    acc.push((truncate_namespace_var(&owned_ns, var), rhs));
                    acc
                }
                (_, None) => acc, // skip if not set to anything
            })
    }

    /// Creates a new `VariableHistory` containing only variables from a specific namespace.
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace path to filter by
    pub fn filter_ns(&self, namespace: &[&str]) -> Self {
        let owned_ns = namespace
            .iter()
            .map(|s| (*s).to_owned())
            .collect::<Vec<_>>();
        // Truncate namespaces in the filtered AST for easier resolution
        let filtered = self.filter_by_namespace(&owned_ns, None, true);

        // Combine parent namespace context with the filter namespace
        let mut new_context = self.namespace_context.clone();
        new_context.extend(owned_ns);

        // Create a new history manually without calling Self::new() to avoid
        // recording assignments for the truncated namespace variables
        VariableHistory {
            ast: filtered,
            var_cache: HashMap::new(),
            usage: self.usage.clone(), // Share the same usage stats
            warn_on_drop: false,       // Don't warn on filtered namespaces
            namespace_context: new_context,
            trace: self.trace.clone(),
        }
    }

    /// Resolves a variable by name, returning its value.
    ///
    /// # Arguments
    ///
    /// * `var` - Variable name, optionally with namespace (e.g., "my.namespace.var")
    ///
    /// # Returns
    ///
    /// Returns `Some(Rhs)` if the variable is found, `None` if not found, or an error if parsing fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use ratslang::compile_code;
    /// let code = "my_var = 42";
    /// let result = compile_code(code).unwrap();
    /// let value = result.vars.resolve("my_var").unwrap();
    /// ```
    pub fn resolve(&self, var: &str) -> anyhow::Result<Option<Rhs>> {
        self.resolve_var(&Var::from_str(var)?)
    }

    /// Records evaluation statistics for a variable without resolving it.
    ///
    /// This is useful when a configuration is parsed but you want to mark that
    /// it has been consulted without actually retrieving the value.
    pub fn evaluate_stats_only(&self, var: &str) -> anyhow::Result<()> {
        let parsed = Var::from_str(var)?;

        let full_var = if self.namespace_context.is_empty() {
            parsed
        } else {
            Self::prepend_ns(&parsed, &self.namespace_context)
        };

        self.usage.borrow_mut().record_evaluation(&full_var);
        Ok(())
    }

    fn resolve_var(&self, var: &Var) -> anyhow::Result<Option<Rhs>> {
        let full_var = if self.namespace_context.is_empty() {
            var.clone()
        } else {
            Self::prepend_ns(var, &self.namespace_context)
        };

        self.usage.borrow_mut().record_evaluation(&full_var);
        let value = self.resolve_recursive(var, None)?;
        if let Some(value) = &value {
            value.validate_finite()?;
        }
        Ok(value)
    }
}

impl VariableHistory {
    #[cfg(test)]
    fn usage_counts(&self) -> (usize, usize) {
        let usage = self.usage.borrow();
        (usage.assigned.len(), usage.evaluated.len())
    }

    /// Returns the assigned and evaluated variable names from the configuration.
    ///
    /// Useful for build scripts or configuration validation to detect unused variables.
    pub fn usage_vars(&self) -> (Vec<String>, Vec<String>) {
        let usage = self.usage.borrow();
        let mut assigned = usage.assigned.iter().map(format_var).collect::<Vec<_>>();
        let mut evaluated = usage.evaluated.iter().map(format_var).collect::<Vec<_>>();
        assigned.sort();
        evaluated.sort();
        (assigned, evaluated)
    }
}

impl Drop for VariableHistory {
    fn drop(&mut self) {
        if !self.warn_on_drop {
            return;
        }

        let usage = self.usage.borrow();
        if usage.assigned.is_empty() && usage.evaluated.is_empty() {
            log::warn!("File was parsed but never used.");
            return;
        }

        for var in usage.assigned.difference(&usage.evaluated) {
            log::warn!("Variable '{}' was never used.", format_var(var));
        }
    }
}

fn resolve_rhs_recursive(
    rhs: &mut Rhs,
    var_history: &VariableHistory,
    i: usize,
) -> anyhow::Result<()> {
    match rhs {
        Rhs::Array(items) => {
            for rhs_item in items.iter_mut() {
                resolve_rhs_recursive(&mut *rhs_item, var_history, i)?;
            }
        }
        Rhs::Var(var) => {
            // implicit strings when mentioned but not found in context earlier
            *rhs = match var_history.resolve_recursive(var, Some(i))? {
                None => Rhs::Val(Val::StringVal(match var {
                    Var::User { name, namespace: _ } => name.clone(),
                    Var::Predef {
                        name: _,
                        namespace: _,
                    } => {
                        return Err(anyhow!("Predef variables can not be assigned."));
                    }
                })),
                Some(rhs) => rhs,
            };
        }
        _ => {}
    };

    Ok(())
}

fn resolve_stmt(
    stmt: &mut Statement,
    var_history: &VariableHistory,
    i: usize,
) -> anyhow::Result<()> {
    match stmt {
        Statement::AssignLeft { lhs: _, rhs, .. } => {
            for r in rhs.iter_mut() {
                resolve_rhs_recursive(r, var_history, i)?;
            }
        }
    }

    Ok(())
}

/// Parses annotations from source code.
/// Annotations are marked with `# @annotation_name` and end with an empty line.
/// Returns a map of annotation names to line ranges (0-indexed, inclusive).
fn parse_annotations(source_text: &str) -> HashMap<String, Vec<(usize, usize)>> {
    let mut annotations: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
    let lines: Vec<&str> = source_text.lines().collect();
    let mut current_annotation: Option<(String, usize)> = None;

    for (line_idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Check for annotation start: # @annotation_name
        if trimmed.starts_with("#") {
            let comment_content = trimmed.trim_start_matches('#').trim();
            if let Some(at_pos) = comment_content.find("@") {
                if at_pos == 0 {
                    let annotation_name = comment_content[1..].trim().to_string();
                    if constraints::is_metadata(&annotation_name) {
                        continue;
                    }

                    // Detect if this annotation is directly before a namespace block
                    let first_nonempty = (line_idx + 1..lines.len())
                        .find(|&k| !lines[k].trim().is_empty());

                    let mut handled_block = false;
                    if let Some(first) = first_nonempty {
                        let first_trim = lines[first].trim();
                        if first_trim.ends_with('{') {
                            // Compute matching closing brace
                            let mut depth = 0isize;
                            let mut end_line = lines.len().saturating_sub(1);
                            for (k, line) in lines.iter().enumerate().skip(first) {
                                let t = line.trim();
                                if t.ends_with('{') {
                                    depth += 1;
                                }
                                if t.starts_with('}') {
                                    depth -= 1;
                                    if depth == 0 {
                                        end_line = k;
                                        break;
                                    }
                                }
                            }

                            annotations
                                .entry(annotation_name.clone())
                                .or_default()
                                .push((first, end_line));
                            handled_block = true;
                        }
                    }

                    if handled_block {
                        continue;
                    }

                    // If there was a previous unclosed annotation, close it
                    if let Some((prev_annotation, start_line)) = current_annotation.take() {
                        annotations
                            .entry(prev_annotation)
                            .or_default()
                            .push((start_line, line_idx.saturating_sub(1)));
                    }
                    // Start new annotation (non-block)
                    current_annotation = Some((annotation_name, line_idx));
                }
            }
        }
        // Check for annotation end: empty line
        else if trimmed.is_empty() {
            if let Some((annotation_name, start_line)) = current_annotation.take() {
                // End line is the line before the empty line
                annotations
                    .entry(annotation_name)
                    .or_default()
                    .push((start_line + 1, line_idx - 1));
            }
        }
    }

    // If there's an unclosed annotation at the end, close it
    if let Some((annotation_name, start_line)) = current_annotation {
        let end_line = lines.len().saturating_sub(1);
        annotations
            .entry(annotation_name)
            .or_default()
            .push((start_line + 1, end_line));
    }

    annotations
}

/// Directory that relative includes of source code resolve from. Where there is no
/// working directory, as in the browser, that is `.`.
pub(crate) fn working_dir() -> std::path::PathBuf {
    std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
}

/// Compiles ratslang source code from a string.
///
/// # Arguments
///
/// * `source_code_raw` - The source code to compile
///
/// # Returns
///
/// Returns an `Evaluated` result containing the parsed and resolved variables,
/// or an error if parsing fails.
///
/// # Examples
///
/// ```rust
/// use ratslang::compile_code;
///
/// let code = r#"
///     distance = 100mm
///     time = 5s
///     name = "example"
/// "#;
///
/// let result = compile_code(code).unwrap();
/// let distance = result.vars.resolve("distance").unwrap();
/// ```
///
/// Relative includes resolve from the working directory.
pub fn compile_code(source_code_raw: &str) -> anyhow::Result<Evaluated> {
    let currdir = working_dir();
    let original_source = source_code_raw.to_string();
    let mut result = compile_code_with_state(
        &(source_code_raw.to_owned() + "\n"), // hack to fix one-line token problems
        &currdir,
        None,
        std::io::stderr(),
        true,
    )?;

    // Update source and annotations without moving vars
    result.source_text = original_source.clone();
    result.annotations = parse_annotations(&original_source);
    Ok(result)
}

/// A parser diagnostic with a byte range in the input source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceDiagnostic {
    pub range: std::ops::Range<usize>,
    pub message: String,
}

/// Report syntax errors using the compiler's parser without resolving includes.
/// This is suitable for editor buffers that may contain incomplete statements.
pub fn syntax_diagnostics(source: &str) -> Vec<SourceDiagnostic> {
    let original_len = source.len();
    let source_with_newline = format!("{source}\n");
    let source = source_with_newline.as_str();
    let tokens = Token::lexer(source)
        .spanned()
        .map(|(token, span)| (token.unwrap_or(Token::Error), span.into()));
    let stream = Stream::from_iter(tokens)
        .map((0..source.len()).into(), |(token, span): (_, _)| {
            (token, span)
        });
    parser(source)
        .parse(stream)
        .into_errors()
        .into_iter()
        .map(|error| SourceDiagnostic {
            range: error.span().start.min(original_len)..error.span().end.min(original_len),
            message: error.reason().to_string(),
        })
        .collect()
}

/// Compiles ratslang source code with existing variable state.
///
/// # Arguments
///
/// * `source_code_raw` - The source code to compile
/// * `source_code_parent_dir` - Directory path for resolving relative includes
/// * `var_state` - Optional pre-existing variable state to build upon
/// * `out` - Writer for error output
/// * `rich_out` - Whether to use rich formatted error output
///
/// # Returns
///
/// Returns an `Evaluated` result containing the parsed and resolved variables,
/// or an error if parsing fails.
pub fn compile_code_with_state(
    source_code_raw: &str,
    source_code_parent_dir: &std::path::Path,
    var_state: Option<HashMap<Var, Rhs>>,
    out: impl std::io::Write,
    rich_out: bool,
) -> anyhow::Result<Evaluated> {
    compile_with_context(
        source_code_raw,
        source_code_parent_dir,
        var_state,
        out,
        rich_out,
        provenance::Context {
            source: source_code_raw,
            file: None,
            byte_offset: 0,
            line_offset: 0,
            included_from: vec![],
        },
    )
}
fn compile_with_context(
    source_code_raw: &str,
    source_code_parent_dir: &std::path::Path,
    var_state: Option<HashMap<Var, Rhs>>,
    mut out: impl std::io::Write,
    rich_out: bool,
    context: provenance::Context<'_>,
) -> anyhow::Result<Evaluated> {
    let token_iter = Token::lexer(source_code_raw)
        .spanned()
        // Convert logos errors into tokens. We want parsing to be recoverable and not fail at the lexing stage, so
        // we have a dedicated `Token::Error` variant that represents a token error that was previously encountered
        .map(|(tok, span)| match tok {
            // Turn the `Range<usize>` spans logos gives us into chumsky's `SimpleSpan` via `Into`, because it's easier
            // to work with
            Ok(tok) => (tok, span.into()),
            Err(()) => (Token::Error, span.into()),
        });

    // Turn the token iterator into a stream that chumsky can use for things like backtracking
    let token_stream = Stream::from_iter(token_iter)
        // Tell chumsky to split the (Token, SimpleSpan) stream into its parts so that it can handle the spans for us
        // This involves giving chumsky an 'end of input' span: we just use a zero-width span at the end of the string
        .map((0..source_code_raw.len()).into(), |(t, s): (_, _)| (t, s));

    let locations: provenance::Locations = Default::default();
    // Parse the token stream with our chumsky parser
    match parser_with_locations(source_code_raw, Some(locations.clone()))
        .parse(token_stream)
        .into_result()
    {
        Ok(sexpr) => {
            let owned = sexpr
                .into_iter()
                .map(|kind| kind.into())
                .collect::<Vec<StatementKindOwnedPass1>>();

            // Pass 1 -- parsing included files and expanding namespaces
            let parsed = do_pass1(
                owned,
                source_code_parent_dir,
                &mut locations.borrow().values().cloned(),
                &context,
            )?;
            return evaluate_parsed(parsed, var_state, source_code_raw);
        }
        Err(errs) => {
            let src = Source::from(source_code_raw);
            for err in errs {
                if rich_out {
                    let e = Report::build(ReportKind::Error, err.span().into_range())
                        .with_code(3)
                        .with_message(err.to_string())
                        .with_label(
                            Label::new(err.span().into_range())
                                .with_message(err.reason().to_string())
                                .with_color(Color::Red),
                        )
                        .finish();

                    e.write(src.clone(), &mut out).unwrap();
                } else {
                    let msg = format!(
                        "<{}-{}>  {}",
                        err.span().start,
                        err.span().end,
                        err.reason(),
                    );
                    out.write_all(msg.as_bytes()).unwrap();
                }
            }
        }
    }

    Err(anyhow!("Could not parse ratslang code."))
}

fn evaluate_parsed(
    parsed: provenance::Parsed,
    var_state: Option<HashMap<Var, Rhs>>,
    source_code_raw: &str,
) -> anyhow::Result<Evaluated> {
    let original = parsed.ast.clone();
    let mut ast = parsed.ast; // could print parse errors from included files

    // Pass 2 -- needs entire ast for resolving. TODO could still be combined to Pass 1 with a subset of the ast to t-1.
    // Resolve all variables here, else implicit strings in nested assigns are handled as variables.
    // Use the repl state in case the user already run some variables from before which we want to build upon
    let var_history = VariableHistory::new(ast.clone())
        .with_state(var_state.clone().unwrap_or_default())
        .with_drop_warning(false);
    for (i, node) in ast.iter_mut().enumerate() {
        match node {
            StatementKindOwned::VariableDef(statement) => {
                resolve_stmt(statement, &var_history, i)?;
            }
        }
    }

    // Create the final VariableHistory that shares usage stats with var_history
    // so that when it's dropped, it will warn about unused variables
    let mut vars = VariableHistory::new(ast.clone()).with_state(var_state.unwrap_or_default());
    vars.usage = var_history.usage.clone();
    vars.trace = Rc::new(provenance::Trace::build(
        &original,
        &ast,
        &parsed.locations,
        &vars.var_cache,
    ));
    if let Err(error) = vars.validate_constraints() {
        vars.warn_on_drop = false;
        return Err(error);
    }

    // Parse annotations from source code
    let annotations = parse_annotations(source_code_raw);

    Ok(Evaluated {
        vars,
        source_text: source_code_raw.to_string(),
        annotations,
    })
}

// -- Macros --

/// Resolves a length range from millimeters to meters as f32.
///
/// Handles various range syntaxes:
/// - Empty range: `..` returns default bounds
/// - Bounded range: `100mm..2000mm` converts both to meters
/// - Partial bounds: `100mm..` or `..2000mm` uses defaults for missing bounds
///
/// # Examples
///
/// ```rust
/// use ratslang::{compile_code, resolve_var, resolve_length_range_meters_float, Rhs, Val, NumVal, Unit, meter};
/// use anyhow::anyhow;
///
/// # fn main() -> anyhow::Result<()> {
/// struct Configs {
///     user: ratslang::VariableHistory,
///     defaults: ratslang::VariableHistory,
/// }
///
/// let code = "distance = 1000mm..5000mm";
/// let ast = compile_code(code)?;
///
/// let configs = Configs {
///     user: ast.vars,
///     defaults: ratslang::VariableHistory::new(vec![]),
/// };
///
/// let (min, max) = resolve_length_range_meters_float!(configs, "distance", 0.0, 10.0)?;
/// assert_eq!(min, 1.0);
/// assert_eq!(max, 5.0);
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! resolve_length_range_meters_float {
    ($ns:expr, $field:expr, $default_from:expr, $default_to:expr) => {{
        let convert_to_meters = |val: &Val| -> anyhow::Result<f64> {
            match val {
                Val::UnitedVal(uv) if uv.unit() == Unit::Length => {
                    Ok(uv.as_length().unwrap().get::<meter>())
                },
                _ => Err(anyhow!("Expected a united length value for range '{}'", $field))
            }
        };

        let rhs_val = resolve_var!($ns, $field, as &Rhs,
            r @ Rhs::EmptyRange | r @ Rhs::Range { .. } => { r }
        )?;
        match rhs_val {
            Rhs::EmptyRange => Ok(($default_from, $default_to)),
            Rhs::Range { from, to } => {
                let from_val = from.as_ref().map_or(Ok($default_from), |v| convert_to_meters(v))?;
                let to_val = to.as_ref().map_or(Ok($default_to), |v| convert_to_meters(v))?;
                Ok((from_val, to_val))
            }
            _ => Err(anyhow!("Expected a range expression for field '{}'", $field))
        }
    }};
}

/// Resolves a variable from user or default configuration.
///
/// This macro attempts to resolve a variable from the user's configuration first,
/// falling back to defaults if not found. It supports pattern matching on the result
/// and automatic type conversion.
///
/// # Examples
///
/// ```rust
/// use ratslang::{compile_code, resolve_var, Rhs, Val, NumVal};
/// use anyhow::anyhow;
///
/// # fn main() -> anyhow::Result<()> {
/// // Structure to hold user and default configurations
/// struct Configs {
///     user: ratslang::VariableHistory,
///     defaults: ratslang::VariableHistory,
/// }
///
/// let user_code = "timeout = 5000";
/// let default_code = "timeout = 3000\nmax_retries = 10";
///
/// let user_ast = compile_code(user_code)?;
/// let default_ast = compile_code(default_code)?;
///
/// let configs = Configs {
///     user: user_ast.vars,
///     defaults: default_ast.vars,
/// };
///
/// // Resolve timeout (found in user config)
/// let timeout: i64 = resolve_var!(configs, timeout, as i64,
///     Rhs::Val(Val::NumVal(NumVal::Integer(i))) => { i }
/// )?;
/// assert_eq!(timeout, 5000);
///
/// // Resolve max_retries (found in defaults)
/// let retries: i64 = resolve_var!(configs, max_retries, as i64,
///     Rhs::Val(Val::NumVal(NumVal::Integer(i))) => { i }
/// )?;
/// assert_eq!(retries, 10);
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! resolve_var {
    // Arm for variable names passed as identifiers
    ($asts:expr, $var_name_ident:ident, as $target_type:ty, $($pattern:pat_param)|+ => $extraction_block:block) => {{
        let __var_name_str = stringify!($var_name_ident);
        let __resolved_opt = match $asts.user.resolve(__var_name_str)? {
            Some(val) => Some(val),
            None => $asts.defaults.resolve(__var_name_str)?,
        };

        match __resolved_opt {
            Some(__resolved_val) => match __resolved_val {
                $($pattern)|+ => {
                    let __extracted_val = $extraction_block;
                    Ok(__extracted_val)
                }
                _ => Err(anyhow!(format!(
                    "Pattern mismatch for '{}'. Expected pattern for type `{}`.",
                    __var_name_str,
                    stringify!($target_type)
                ))),
            }?
            .try_into()
            .map_err(|e| {
                anyhow!(
                    "Failed to convert '{}' to type `{}`: {:?}",
                    __var_name_str,
                    stringify!($target_type),
                    e
                )
            }),
            None => Err(anyhow!(concat!(
                "Required variable '",
                stringify!($var_name_ident),
                "' not found in any configuration."
            ))),
        }
    }};

    // Arm for variable names passed as string expressions
    ($asts:expr, $var_name_expr:expr, as $target_type:ty, $($pattern:pat_param)|+ => $extraction_block:block) => {{
        let __var_name_str = $var_name_expr;
        let __resolved_opt = match $asts.user.resolve(__var_name_str)? {
            Some(val) => Some(val),
            None => $asts.defaults.resolve(__var_name_str)?,
        };

        match __resolved_opt {
            Some(__resolved_val) => match __resolved_val {
                $($pattern)|+ => {
                    let __extracted_val = $extraction_block;
                    Ok(__extracted_val)
                }
                _ => Err(anyhow!(format!(
                    "Pattern mismatch for '{}'. Expected pattern for type `{}`.",
                    __var_name_str,
                    stringify!($target_type)
                ))),
            }?
            .try_into()
            .map_err(|e| {
                anyhow!(
                    "Failed to convert '{}' to type `{}`: {:?}",
                    __var_name_str,
                    stringify!($target_type),
                    e
                )
            }),
            None => Err(anyhow!(format!(
                "Required variable '{}' not found in any configuration.",
                __var_name_str
            ))),
        }
    }};
}

/// Resolves a numeric range, converting integers to floats if needed.
///
/// Handles empty ranges (`..`), bounded ranges, and partial ranges.
/// Automatically converts integer values to f32.
///
/// # Examples
///
/// ```rust
/// use ratslang::{compile_code, resolve_var, resolve_float_force_range, Rhs, Val, NumVal};
/// use anyhow::anyhow;
///
/// # fn main() -> anyhow::Result<()> {
/// struct Configs {
///     user: ratslang::VariableHistory,
///     defaults: ratslang::VariableHistory,
/// }
///
/// let code = "range = 10..100";
/// let ast = compile_code(code)?;
///
/// let configs = Configs {
///     user: ast.vars,
///     defaults: ratslang::VariableHistory::new(vec![]),
/// };
///
/// let (min, max) = resolve_float_force_range!(configs, "range", 0.0, 200.0)?;
/// assert_eq!(min, 10.0);
/// assert_eq!(max, 100.0);
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! resolve_float_force_range {
    ($ns:expr, $field:expr, $default_from:expr, $default_to:expr) => {{
        let rhs_val = resolve_var!($ns, $field, as &Rhs,
            r @ Rhs::EmptyRange | r @ Rhs::Range { .. } => { r }
        )?;
        match rhs_val {
            Rhs::EmptyRange => Ok(($default_from, $default_to)),
            Rhs::Range { from, to } => {
                let from_val = match from {
                    Some(Val::NumVal(NumVal::Floating(f))) => f as f64,
                    Some(Val::NumVal(NumVal::Integer(i))) => i as f64,
                    None => $default_from,
                    _ => return Err(anyhow!(format!("Expected numeric value for 'from' in float range '{}'", $field))),
                };
                let to_val = match to {
                    Some(Val::NumVal(NumVal::Floating(f))) => f as f64,
                    Some(Val::NumVal(NumVal::Integer(i))) => i as f64,
                    None => $default_to,
                    _ => return Err(anyhow!(format!("Expected numeric value for 'to' in float range '{}'", $field))),
                };
                Ok((from_val, to_val))
            },
            _ => Err(anyhow!(format!("Expected a range expression for field '{}'", $field)))
        }
    }};
}

/// Resolves a floating-point range without implicit integer conversion.
///
/// Unlike `resolve_float_force_range`, this macro requires explicit float values
/// and will return an error if integers are provided.
///
/// # Examples
///
/// ```rust
/// use ratslang::{compile_code, resolve_var, resolve_float_range, Rhs, Val, NumVal};
/// use anyhow::anyhow;
///
/// # fn main() -> anyhow::Result<()> {
/// struct Configs {
///     user: ratslang::VariableHistory,
///     defaults: ratslang::VariableHistory,
/// }
///
/// let code = "ratio = 0.25..0.75";
/// let ast = compile_code(code)?;
///
/// let configs = Configs {
///     user: ast.vars,
///     defaults: ratslang::VariableHistory::new(vec![]),
/// };
///
/// let (min, max) = resolve_float_range!(configs, "ratio", 0.0, 1.0)?;
/// assert_eq!(min, 0.25);
/// assert_eq!(max, 0.75);
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! resolve_float_range {
    ($ns:expr, $field:expr, $default_from:expr, $default_to:expr) => {{
        let rhs_val = resolve_var!($ns, $field, as &Rhs,
            r @ Rhs::EmptyRange | r @ Rhs::Range { .. } => { r }
        )?;
        match rhs_val {
            Rhs::EmptyRange => Ok(($default_from, $default_to)),
            Rhs::Range { from, to } => {
                let from_val = match from {
                    Some(Val::NumVal(NumVal::Floating(f))) => f as f64,
                    Some(Val::NumVal(NumVal::Integer(_))) => return Err(anyhow!(format!("Expected floating value for 'from' in float range '{}'", $field))),
                    None => $default_from,
                    _ => return Err(anyhow!(format!("Expected numeric value for 'from' in float range '{}'", $field))),
                };
                let to_val = match to {
                    Some(Val::NumVal(NumVal::Floating(f))) => f as f64,
                    Some(Val::NumVal(NumVal::Integer(_))) => return Err(anyhow!(format!("Expected floating value for 'from' in float range '{}'", $field))),
                    None => $default_to,
                    _ => return Err(anyhow!(format!("Expected numeric value for 'to' in float range '{}'", $field))),
                };
                Ok((from_val, to_val))
            },
            _ => Err(anyhow!(format!("Expected a range expression for field '{}'", $field)))
        }
    }};
}

/// Resolves a time range from milliseconds to seconds as f32.
///
/// Handles various range syntaxes:
/// - Empty range: `..` returns default bounds
/// - Bounded range: `100ms..5s` converts both to seconds
/// - Partial bounds: `500ms..` or `..10s` uses defaults for missing bounds
///
/// # Examples
///
/// ```rust
/// use ratslang::{compile_code, resolve_var, resolve_time_range_seconds_float, Rhs, Val, NumVal, Unit, second};
/// use anyhow::anyhow;
///
/// # fn main() -> anyhow::Result<()> {
/// struct Configs {
///     user: ratslang::VariableHistory,
///     defaults: ratslang::VariableHistory,
/// }
///
/// let code = "timeout = 1000ms..5s";
/// let ast = compile_code(code)?;
///
/// let configs = Configs {
///     user: ast.vars,
///     defaults: ratslang::VariableHistory::new(vec![]),
/// };
///
/// let (min, max) = resolve_time_range_seconds_float!(configs, "timeout", 0.0, 10.0)?;
/// assert_eq!(min, 1.0);
/// assert_eq!(max, 5.0);
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! resolve_time_range_seconds_float {
    ($ns:expr, $field:expr, $default_from:expr, $default_to:expr) => {{
        let convert_to_seconds = |val: &Val| -> anyhow::Result<f64> {
            match val {
                Val::UnitedVal(uv) if uv.unit() == Unit::Time => {
                    Ok(uv.as_time().unwrap().get::<second>())
                },
                _ => Err(anyhow!("Expected a united time value for range '{}'", $field))
            }
        };

        let rhs_val = resolve_var!($ns, $field, as &Rhs,
            r @ Rhs::EmptyRange | r @ Rhs::Range { .. } => { r }
        )?;
        match rhs_val {
            Rhs::EmptyRange => Ok(($default_from, $default_to)),
            Rhs::Range { from, to } => {
                let from_val = from.as_ref().map_or(Ok($default_from), |v| convert_to_seconds(v))?;
                let to_val = to.as_ref().map_or(Ok($default_to), |v| convert_to_seconds(v))?;
                Ok((from_val, to_val))
            }
            _ => Err(anyhow!("Expected a range expression for field '{}'", $field))
        }
    }};
}

/// Resolves an integer range without type conversion.
///
/// Handles empty ranges (`..`), bounded ranges, and partial ranges.
/// Only accepts integer values, no automatic conversion.
///
/// # Examples
///
/// ```rust
/// use ratslang::{compile_code, resolve_var, resolve_int_range, Rhs, Val, NumVal};
/// use anyhow::anyhow;
///
/// # fn main() -> anyhow::Result<()> {
/// struct Configs {
///     user: ratslang::VariableHistory,
///     defaults: ratslang::VariableHistory,
/// }
///
/// let code = "count = 10..100";
/// let ast = compile_code(code)?;
///
/// let configs = Configs {
///     user: ast.vars,
///     defaults: ratslang::VariableHistory::new(vec![]),
/// };
///
/// let (min, max) = resolve_int_range!(configs, "count", 0, 200)?;
/// assert_eq!(min, 10);
/// assert_eq!(max, 100);
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! resolve_int_range {
    ($ns:expr, $field:expr, $default_from:expr, $default_to:expr) => {{
        let rhs_val = resolve_var!($ns, $field, as &Rhs,
            r @ Rhs::EmptyRange | r @ Rhs::Range { .. } => { r }
        )?;
        match rhs_val {
            Rhs::EmptyRange => Ok(($default_from, $default_to)),
            Rhs::Range { from, to } => {
                let from_val = match from {
                    Some(Val::NumVal(NumVal::Integer(i))) => i,
                    None => $default_from,
                    _ => return Err(anyhow!(format!("Expected integer value for 'from' in integer range '{}'", $field))),
                };
                let to_val = match to {
                    Some(Val::NumVal(NumVal::Integer(i))) => i,
                    None => $default_to,
                    _ => return Err(anyhow!(format!("Expected integer value for 'to' in integer range '{}'", $field))),
                };
                Ok((from_val, to_val))
            },
            _ => Err(anyhow!(format!("Expected a range expression for field '{}'", $field)))
        }
    }};
}

/// Resolves a string value from user or default configuration.
///
/// # Examples
///
/// ```rust
/// use ratslang::{compile_code, resolve_string, resolve_var, Rhs, Val};
/// use anyhow::anyhow;
///
/// # fn main() -> anyhow::Result<()> {
/// struct Configs {
///     user: ratslang::VariableHistory,
///     defaults: ratslang::VariableHistory,
/// }
///
/// let code = "name = \"example\"";
/// let ast = compile_code(code)?;
///
/// let configs = Configs {
///     user: ast.vars,
///     defaults: ratslang::VariableHistory::new(vec![]),
/// };
///
/// let name: String = resolve_string!(configs, "name")?;
/// assert_eq!(name, "example");
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! resolve_string {
    ($ns:expr, $field:expr) => {{
        resolve_var!($ns, $field, as String,
            Rhs::Val(Val::StringVal(s)) => { s.clone() }
        )
    }};
}

/// Resolves a boolean value from user or default configuration.
///
/// # Examples
///
/// ```rust
/// use ratslang::{compile_code, resolve_bool, resolve_var, Rhs, Val};
/// use anyhow::anyhow;
///
/// # fn main() -> anyhow::Result<()> {
/// struct Configs {
///     user: ratslang::VariableHistory,
///     defaults: ratslang::VariableHistory,
/// }
///
/// let code = "enabled = true";
/// let ast = compile_code(code)?;
///
/// let configs = Configs {
///     user: ast.vars,
///     defaults: ratslang::VariableHistory::new(vec![]),
/// };
///
/// let enabled: bool = resolve_bool!(configs, "enabled")?;
/// assert_eq!(enabled, true);
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! resolve_bool {
    ($ns:expr, $field:expr) => {{
        resolve_var!($ns, $field, as bool,
            Rhs::Val(Val::BoolVal(b)) => { b }
        )
    }};
}

/// Resolves an integer value from user or default configuration.
///
/// # Examples
///
/// ```rust
/// use ratslang::{compile_code, resolve_int, resolve_var, Rhs, Val, NumVal};
/// use anyhow::anyhow;
///
/// # fn main() -> anyhow::Result<()> {
/// struct Configs {
///     user: ratslang::VariableHistory,
///     defaults: ratslang::VariableHistory,
/// }
///
/// let code = "count = 42";
/// let ast = compile_code(code)?;
///
/// let configs = Configs {
///     user: ast.vars,
///     defaults: ratslang::VariableHistory::new(vec![]),
/// };
///
/// let count: i64 = resolve_int!(configs, "count")?;
/// assert_eq!(count, 42);
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! resolve_int {
    ($ns:expr, $field:expr) => {{
        resolve_var!($ns, $field, as i64,
            Rhs::Val(Val::NumVal(NumVal::Integer(i))) => { i }
        )
    }};
}

/// Resolves a floating-point value from user or default configuration.
///
/// Accepts both float and integer values, converting integers to floats.
///
/// # Examples
///
/// ```rust
/// use ratslang::{compile_code, resolve_float};
/// use anyhow::anyhow;
///
/// # fn main() -> anyhow::Result<()> {
/// struct Configs {
///     user: ratslang::VariableHistory,
///     defaults: ratslang::VariableHistory,
/// }
///
/// let code = "ratio = 3.14";
/// let ast = compile_code(code)?;
///
/// let configs = Configs {
///     user: ast.vars,
///     defaults: ratslang::VariableHistory::new(vec![]),
/// };
///
/// let ratio: f64 = resolve_float!(configs, "ratio")?;
/// assert_eq!(ratio, 3.14);
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! resolve_float {
    ($ns:expr, $field:expr) => {{
        use $crate::{NumVal, Rhs, Val};
        let __var_name_str = $field;
        let __resolved_opt = match $ns.user.resolve(__var_name_str)? {
            Some(val) => Some(val),
            None => $ns.defaults.resolve(__var_name_str)?,
        };

        match __resolved_opt {
            Some(Rhs::Val(Val::NumVal(NumVal::Floating(f)))) => Ok(f),
            Some(Rhs::Val(Val::NumVal(NumVal::Integer(i)))) => Ok(i as f64),
            Some(_) => Err(anyhow!(format!(
                "Pattern mismatch for '{}'. Expected numeric value.",
                __var_name_str
            ))),
            None => Err(anyhow!(format!(
                "Required variable '{}' not found in any configuration.",
                __var_name_str
            ))),
        }
    }};
}

/// Resolves a path value from user or default configuration.
///
/// # Examples
///
/// ```rust
/// use ratslang::{compile_code, resolve_path, resolve_var, Rhs};
/// use anyhow::anyhow;
///
/// # fn main() -> anyhow::Result<()> {
/// struct Configs {
///     user: ratslang::VariableHistory,
///     defaults: ratslang::VariableHistory,
/// }
///
/// let code = "_file = /path/to/file";
/// let ast = compile_code(code)?;
///
/// let configs = Configs {
///     user: ast.vars,
///     defaults: ratslang::VariableHistory::new(vec![]),
/// };
///
/// let path: String = resolve_path!(configs, "_file")?;
/// assert_eq!(path, "/path/to/file");
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! resolve_path {
    ($ns:expr, $field:expr) => {{
        resolve_var!($ns, $field, as String,
            Rhs::Path(p) => { p.clone() }
        )
    }};
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod annotation_tests;

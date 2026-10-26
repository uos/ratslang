//! Shared source formatting for the ratslang AST, independent of serde.

use crate::{NumVal, Rhs, Root, Statement, Unit, UnitVal, Val, Var};
use std::fmt;

/// An AST value cannot be represented as ratslang source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatError(String);
impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for FormatError {}
type Result<T> = std::result::Result<T, FormatError>;
fn error(message: impl Into<String>) -> FormatError {
    FormatError(message.into())
}

impl Root {
    /// Format AST assignments as a ratslang document with readable physical units.
    ///
    /// Returns an error for values the language cannot represent, such as
    /// non-finite numbers or strings containing quotes or newlines.
    pub fn to_source(&self) -> Result<String> {
        let mut output = String::new();
        for statement in &self.vardefs {
            let Statement::AssignLeft {
                lhs,
                rhs,
                annotations,
            } = statement;
            if lhs.is_empty() || rhs.is_empty() {
                return Err(error("assignments require variables and expressions"));
            }
            for annotation in annotations {
                if annotation.trim().is_empty() || annotation.contains(['\r', '\n']) {
                    return Err(error("invalid annotation name"));
                }
                output.push_str("# @");
                output.push_str(annotation);
                output.push('\n');
            }
            for (i, var) in lhs.iter().enumerate() {
                if i > 0 {
                    output.push_str(", ");
                }
                write_var(var, &mut output)?;
            }
            output.push_str(" = ");
            for (i, value) in rhs.iter().enumerate() {
                if i + 1 < rhs.len() && matches!(value, Rhs::Path(_)) {
                    return Err(error("a path cannot precede another assignment expression"));
                }
                if i > 0 {
                    output.push_str(", ");
                }
                write_rhs(value, &mut output)?;
            }
            output.push('\n');
        }
        Ok(output)
    }
}
impl Rhs {
    /// Format an AST expression as ratslang source.
    pub fn to_source(&self) -> Result<String> {
        let mut output = String::new();
        write_rhs(self, &mut output)?;
        Ok(output)
    }
}
impl Val {
    /// Format a literal as ratslang source, selecting readable physical units.
    pub fn to_source(&self) -> Result<String> {
        let mut output = String::new();
        write_val(self, &mut output)?;
        Ok(output)
    }
}
impl UnitVal {
    /// Format a quantity using a unit suited to its magnitude (for example `1um`).
    pub fn to_source(&self) -> Result<String> {
        let value = self.si_value();
        let (scaled, unit) = readable_unit(self.unit(), value);
        let number = number(scaled)?;
        Ok(format!(
            "{}{unit}",
            number.strip_suffix(".0").unwrap_or(&number)
        ))
    }
}

pub(crate) fn valid_key(key: &str) -> bool {
    let mut chars = key.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !matches!(key, "true" | "false" | "if")
}
fn write_var(var: &Var, out: &mut String) -> Result<()> {
    let path = crate::format_var(var);
    for key in path.split('.') {
        if !valid_key(key) {
            return Err(error(format!("invalid ratslang key: {key:?}")));
        }
    }
    out.push_str(&path);
    Ok(())
}
fn number(value: f64) -> Result<String> {
    if !value.is_finite() {
        return Err(error("non-finite numbers have no ratslang representation"));
    }
    let mut s = value.to_string();
    if !s.contains('.') {
        s.push_str(".0");
    }
    Ok(s)
}
fn write_val(value: &Val, out: &mut String) -> Result<()> {
    match value {
        Val::BoolVal(b) => out.push_str(if *b { "true" } else { "false" }),
        Val::NumVal(NumVal::Integer(i)) => out.push_str(&i.to_string()),
        Val::NumVal(NumVal::Floating(f)) => out.push_str(&number(*f)?),
        Val::StringVal(s) => {
            if s.contains(['"', '\n', '\r']) {
                return Err(error("ratslang strings cannot contain quotes or newlines"));
            }
            out.push('"');
            out.push_str(s);
            out.push('"');
        }
        Val::UnitedVal(quantity) => out.push_str(&quantity.to_source()?),
    }
    Ok(())
}
fn write_rhs(value: &Rhs, out: &mut String) -> Result<()> {
    match value {
        Rhs::Val(v) => write_val(v, out)?,
        Rhs::Var(var) => write_var(var, out)?,
        Rhs::EmptyRange => out.push_str(".."),
        Rhs::Range { from, to } => {
            if let Some(value) = from {
                write_val(value, out)?;
            }
            out.push_str("..");
            if let Some(value) = to {
                write_val(value, out)?;
            }
        }
        Rhs::Path(path) => {
            use logos::Logos;
            let mut tokens = crate::Token::lexer(path);
            if !matches!(tokens.next(), Some(Ok(crate::Token::Path(p))) if p == path)
                || tokens.next().is_some()
            {
                return Err(error("invalid ratslang path"));
            }
            out.push_str(path);
        }
        Rhs::Array(values) => {
            out.push('[');
            for (i, value) in values.iter().enumerate() {
                // The path lexer consumes delimiters, so a path cannot be emitted
                // as an array element without changing its meaning.
                if matches!(value.as_ref(), Rhs::Path(_)) {
                    return Err(error("paths cannot be formatted inside arrays"));
                }
                if i > 0 {
                    out.push_str(", ");
                }
                write_rhs(value, out)?;
            }
            out.push(']');
        }
    }
    Ok(())
}

fn readable_unit(dimension: Unit, value: f64) -> (f64, &'static str) {
    const LENGTH_UNITS: &[(f64, &str)] = &[
        (1e-24, "ym"),
        (1e-21, "zm"),
        (1e-18, "am"),
        (1e-15, "fm"),
        (1e-12, "pm"),
        (1e-9, "nm"),
        (1e-6, "um"),
        (1e-3, "mm"),
        (1e-2, "cm"),
        (1., "m"),
        (1e3, "km"),
        (1e6, "Mm"),
        (1e9, "Gm"),
        (1e12, "Tm"),
        (1e15, "Pm"),
        (1e18, "Em"),
        (1e21, "Zm"),
        (1e24, "Ym"),
    ];
    const TIME_UNITS: &[(f64, &str)] = &[
        (1e-24, "ys"),
        (1e-21, "zs"),
        (1e-18, "as"),
        (1e-15, "fs"),
        (1e-12, "ps"),
        (1e-9, "ns"),
        (1e-6, "us"),
        (1e-3, "ms"),
        (1., "s"),
        (60., "min"),
        (3600., "h"),
        (86400., "d"),
        (1e6, "Ms"),
        // Match uom::si::time::year (365 days), as used by the parser.
        (31536000., "a"),
        (1e9, "Gs"),
        (1e12, "Ts"),
        (1e15, "Ps"),
        (1e18, "Es"),
        (1e21, "Zs"),
        (1e24, "Ys"),
    ];
    let units: &[(f64, &str)] = match dimension {
        Unit::Mass => &[
            (1e-27, "yg"),
            (1e-24, "zg"),
            (1e-21, "ag"),
            (1e-18, "fg"),
            (1e-15, "pg"),
            (1e-12, "ng"),
            (1e-9, "ug"),
            (1e-6, "mg"),
            (1e-3, "g"),
            (1., "kg"),
            (1e3, "t"),
            (1e6, "Gg"),
            (1e9, "Tg"),
            (1e12, "Pg"),
            (1e15, "Eg"),
            (1e18, "Zg"),
            (1e21, "Yg"),
        ],
        Unit::Length => LENGTH_UNITS,
        Unit::Time => TIME_UNITS,
        Unit::Angle => &[(1e-6, "urad"), (1e-3, "mrad"), (1., "rad")],
        Unit::Frequency => &[
            (1e-3, "mHz"),
            (1., "Hz"),
            (1e3, "kHz"),
            (1e6, "MHz"),
            (1e9, "GHz"),
        ],
        Unit::Velocity => &[(1e-3, "mm/s"), (1e-2, "cm/s"), (1., "m/s")],
        Unit::Acceleration => &[(1e-3, "mm/s2"), (1e-2, "cm/s2"), (1., "m/s2")],
    };
    if value == 0. {
        return (
            value,
            match dimension {
                Unit::Mass => "kg",
                Unit::Length => "m",
                Unit::Time => "s",
                Unit::Angle => "rad",
                Unit::Frequency => "Hz",
                Unit::Velocity => "m/s",
                Unit::Acceleration => "m/s2",
            },
        );
    }
    let &(scale, unit) = units
        .iter()
        .rev()
        .find(|(scale, _)| value.abs() >= *scale)
        .unwrap_or(&units[0]);
    let scaled = value / scale;
    // Remove conversion noise only when parsing the shorter decimal reproduces
    // the exact SI value; no precision is discarded (e.g. 250mg, not 250.000...mg).
    for decimals in 0..=12 {
        let factor = 10_f64.powi(decimals);
        let candidate = (scaled * factor).round() / factor;
        if candidate * scale == value {
            return (candidate, unit);
        }
    }
    if !(scaled * scale).is_finite() {
        return (
            value,
            match dimension {
                Unit::Mass => "kg",
                Unit::Length => "m",
                Unit::Time => "s",
                Unit::Angle => "rad",
                Unit::Frequency => "Hz",
                Unit::Velocity => "m/s",
                Unit::Acceleration => "m/s2",
            },
        );
    }
    (scaled, unit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Length, Time, compile_code, micrometer, millisecond};

    fn integer(i: i64) -> Val {
        Val::NumVal(NumVal::Integer(i))
    }

    #[test]
    fn ast_expressions_round_trip_through_compiler() {
        let expressions = [
            Rhs::Val(Val::BoolVal(true)),
            Rhs::Val(Val::StringVal("hello".into())),
            Rhs::Val(Val::NumVal(NumVal::Floating(1.0))),
            Rhs::Val(Val::UnitedVal(UnitVal::from_length(Length::new::<
                micrometer,
            >(1.)))),
            Rhs::Val(Val::UnitedVal(UnitVal::from_time(
                Time::new::<millisecond>(250.),
            ))),
            Rhs::Range {
                from: Some(integer(1)),
                to: Some(integer(4)),
            },
            Rhs::Range {
                from: Some(integer(1)),
                to: None,
            },
            Rhs::Range {
                from: None,
                to: Some(integer(4)),
            },
            Rhs::EmptyRange,
            Rhs::Path("./example.rl".into()),
            Rhs::Array(vec![
                Box::new(Rhs::Val(integer(-2))),
                Box::new(Rhs::Array(vec![])),
            ]),
        ];
        for expression in expressions {
            let source = expression.to_source().unwrap();
            let compiled = compile_code(&format!("value = {source}\n")).unwrap();
            assert_eq!(
                compiled.vars.resolve("value").unwrap(),
                Some(expression),
                "{source}"
            );
        }
    }

    #[test]
    fn ast_document_preserves_assignments_references_and_annotations() {
        let root = Root {
            vardefs: vec![
                Statement::AssignLeft {
                    lhs: vec!["base".parse().unwrap()],
                    rhs: vec![Rhs::Val(integer(7))],
                    annotations: vec![],
                },
                Statement::AssignLeft {
                    lhs: vec![
                        "sensor.first".parse().unwrap(),
                        "sensor.second".parse().unwrap(),
                    ],
                    rhs: vec![Rhs::Var("base".parse().unwrap()), Rhs::Val(integer(9))],
                    annotations: vec!["minimal".into()],
                },
            ],
        };
        let source = root.to_source().unwrap();
        assert_eq!(
            source,
            "base = 7\n# @minimal\nsensor.first, sensor.second = base, 9\n"
        );
        let compiled = compile_code(&source).unwrap();
        assert_eq!(
            compiled.vars.resolve("sensor.first").unwrap(),
            Some(Rhs::Val(integer(7)))
        );
        assert_eq!(
            compiled.vars.resolve("sensor.second").unwrap(),
            Some(Rhs::Val(integer(9)))
        );
        assert!(compiled.annotations.contains_key("minimal"));
    }

    #[test]
    fn rejects_unrepresentable_ast_values() {
        for value in [
            Val::NumVal(NumVal::Floating(f64::INFINITY)),
            Val::NumVal(NumVal::Floating(f64::NAN)),
            Val::StringVal("a\"b".into()),
        ] {
            assert!(value.to_source().is_err());
        }
        assert!(Rhs::Path("plain_identifier".into()).to_source().is_err());
        assert!(
            Rhs::Array(vec![Box::new(Rhs::Path("./file".into()))])
                .to_source()
                .is_err()
        );
        assert!(Rhs::Var("true".parse().unwrap()).to_source().is_err());
        let root = Root {
            vardefs: vec![Statement::AssignLeft {
                lhs: vec![],
                rhs: vec![],
                annotations: vec![],
            }],
        };
        assert!(root.to_source().is_err());
    }
}

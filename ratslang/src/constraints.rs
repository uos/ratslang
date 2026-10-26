//! Type and value constraints declared in comments above assignments.
//!
//! ```
//! let config = ratslang::compile_code(r#"
//! ## @enum CPU, Backend
//! backend = CPU
//!
//! ## @type integer
//! ## @min 1
//! ## @max 64
//! threads = 8
//!
//! ## @min 1mm
//! ## @max 10m
//! distance = 50cm
//! "#)?;
//! assert_eq!(config.vars.constraints()?["threads"].minimum,
//!     Some(ratslang::Val::NumVal(ratslang::NumVal::Integer(1))));
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! Enum choices apply to strings, bare identifiers, and paths, using exact,
//! case-sensitive matching (`@enum_case insensitive` permits ASCII case variants).
//! Numeric bounds are inclusive and compare physical
//! quantities in SI base units. `@type` also accepts `array<T>` and `range<T>`;
//! bounds apply to numeric scalars or every element of an array.
//! `@min_exclusive` / `@max_exclusive` exclude the boundary. All numeric values
//! must be finite, including unit conversions, arrays, and range endpoints.
//! Existing `@type = ...`,
//! `@type = enum[A,B]`, and `@options = [A,B]` schema spellings are supported.
//! Declarations persist across later overrides of the same variable. Unknown
//! annotations remain ordinary filtering tags such as `@recommended`.

use crate::{
    NumVal, Rhs, Statement, StatementKindOwned, Token, UnitVal, Val, VariableHistory, meter, second,
};
use anyhow::{Result, anyhow, bail, ensure};
use logos::Logos;
use std::{
    cmp::Ordering,
    collections::{BTreeMap, HashSet},
};

/// The shape of an annotated configuration value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueType {
    Boolean,
    Integer,
    Number,
    Float,
    String,
    Path,
    Mass,
    Length,
    Time,
    Angle,
    Frequency,
    Velocity,
    Acceleration,
    Array(Box<ValueType>),
    Range(Box<ValueType>),
}
impl std::fmt::Display for ValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Array(inner) => write!(f, "array<{inner}>"),
            Self::Range(inner) => write!(f, "range<{inner}>"),
            other => f.write_str(match other {
                Self::Boolean => "boolean",
                Self::Integer => "integer",
                Self::Number => "number",
                Self::Float => "float",
                Self::String => "string",
                Self::Path => "path",
                Self::Mass => "mass",
                Self::Length => "length",
                Self::Time => "time",
                Self::Angle => "angle",
                Self::Frequency => "frequency",
                Self::Velocity => "velocity",
                Self::Acceleration => "acceleration",
                _ => unreachable!(),
            }),
        }
    }
}
impl ValueType {
    fn parse(source: &str) -> Result<Self> {
        let source = source.trim();
        if let Some(inner) = source
            .strip_prefix("array<")
            .and_then(|s| s.strip_suffix('>'))
        {
            return Ok(Self::Array(Box::new(Self::parse(inner)?)));
        }
        if let Some(inner) = source
            .strip_prefix("range<")
            .and_then(|s| s.strip_suffix('>'))
        {
            return Ok(Self::Range(Box::new(Self::parse(inner)?)));
        }
        Ok(match source {
            "bool" | "boolean" => Self::Boolean,
            "int" | "integer" => Self::Integer,
            "number" => Self::Number,
            "float" => Self::Float,
            "string" => Self::String,
            "path" => Self::Path,
            "mass" => Self::Mass,
            "length" => Self::Length,
            "angle" => Self::Angle,
            "frequency" => Self::Frequency,
            "velocity" => Self::Velocity,
            "acceleration" => Self::Acceleration,
            "time" => Self::Time,
            _ => bail!("unknown @type {source:?}"),
        })
    }
    fn accepts(&self, value: &Rhs) -> bool {
        match (self, value) {
            (Self::Boolean, Rhs::Val(Val::BoolVal(_))) => true,
            (Self::Integer, Rhs::Val(Val::NumVal(NumVal::Integer(_)))) => true,
            (Self::Number, Rhs::Val(Val::NumVal(_))) => true,
            (Self::Float, Rhs::Val(Val::NumVal(NumVal::Floating(_)))) => true,
            (Self::String, Rhs::Val(Val::StringVal(_))) => true,
            (Self::Path, Rhs::Path(_) | Rhs::Val(Val::StringVal(_))) => true,
            (Self::Mass, Rhs::Val(Val::UnitedVal(UnitVal::MassVal(_)))) => true,
            (Self::Length, Rhs::Val(Val::UnitedVal(UnitVal::LengthVal(_)))) => true,
            (Self::Angle, Rhs::Val(Val::UnitedVal(UnitVal::AngleVal(_)))) => true,
            (Self::Frequency, Rhs::Val(Val::UnitedVal(UnitVal::FrequencyVal(_)))) => true,
            (Self::Velocity, Rhs::Val(Val::UnitedVal(UnitVal::VelocityVal(_)))) => true,
            (Self::Acceleration, Rhs::Val(Val::UnitedVal(UnitVal::AccelerationVal(_)))) => true,
            (Self::Time, Rhs::Val(Val::UnitedVal(UnitVal::TimeVal(_)))) => true,
            (Self::Array(inner), Rhs::Array(values)) => values.iter().all(|v| inner.accepts(v)),
            (Self::Range(inner), Rhs::Range { from, to }) => from
                .iter()
                .chain(to.iter())
                .all(|v| inner.accepts(&Rhs::Val(v.clone()))),
            (Self::Range(_), Rhs::EmptyRange) => true,
            _ => false,
        }
    }
    fn element_type(&self) -> &Self {
        match self {
            Self::Array(inner) => inner.element_type(),
            ty => ty,
        }
    }
    fn numeric(&self) -> bool {
        matches!(
            self,
            Self::Integer
                | Self::Float
                | Self::Number
                | Self::Mass
                | Self::Length
                | Self::Time
                | Self::Angle
                | Self::Frequency
                | Self::Velocity
                | Self::Acceleration
        )
    }
}

/// Rules attached to one variable. Bounds retain their physical dimension.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Constraints {
    pub value_type: Option<ValueType>,
    pub allowed: Option<Vec<String>>,
    pub minimum: Option<Val>,
    pub maximum: Option<Val>,
    pub minimum_exclusive: bool,
    pub maximum_exclusive: bool,
    pub case_insensitive: Option<bool>,
    /// Require an explicit assignment in the user layer, even when a default exists.
    pub required: Option<bool>,
    pub deprecated: Option<String>,
    /// Fully qualified replacement setting name.
    pub renamed_to: Option<String>,
}
impl Constraints {
    /// Parse comment payloads without the leading `# @`, using the same rules
    /// as the compiler. Useful for editor tooling on incomplete documents.
    pub fn from_annotations<'a>(annotations: impl IntoIterator<Item = &'a str>) -> Result<Self> {
        let mut constraints = Self::default();
        for annotation in annotations {
            constraints.apply(annotation)?;
        }
        constraints.definition()?;
        Ok(constraints)
    }
    /// Overlay explicitly declared rules, retaining unspecified earlier rules.
    pub fn overlay(&self, later: &Self) -> Self {
        Self {
            value_type: later.value_type.clone().or_else(|| self.value_type.clone()),
            allowed: later.allowed.clone().or_else(|| self.allowed.clone()),
            minimum: later.minimum.clone().or_else(|| self.minimum.clone()),
            maximum: later.maximum.clone().or_else(|| self.maximum.clone()),
            minimum_exclusive: if later.minimum.is_some() {
                later.minimum_exclusive
            } else {
                self.minimum_exclusive
            },
            maximum_exclusive: if later.maximum.is_some() {
                later.maximum_exclusive
            } else {
                self.maximum_exclusive
            },
            case_insensitive: later.case_insensitive.or(self.case_insensitive),
            required: later.required.or(self.required),
            deprecated: later.deprecated.clone().or_else(|| self.deprecated.clone()),
            renamed_to: later.renamed_to.clone().or_else(|| self.renamed_to.clone()),
        }
    }
    fn definition(&self) -> Result<()> {
        if let Some(allowed) = &self.allowed {
            ensure!(!allowed.is_empty(), "@enum requires at least one choice");
            ensure!(
                self.minimum.is_none() && self.maximum.is_none(),
                "enum choices cannot have numeric min/max bounds"
            );
            if let Some(ty) = &self.value_type {
                ensure!(
                    matches!(ty, ValueType::String | ValueType::Path),
                    "enum choices require a string or path type"
                );
            }
        }
        for bound in self.minimum.iter().chain(self.maximum.iter()) {
            compare(bound, bound)?;
            if let Some(ty) = &self.value_type {
                let ty = ty.element_type();
                ensure!(
                    ty.numeric()
                        && (matches!(
                            ty,
                            ValueType::Integer | ValueType::Float | ValueType::Number
                        ) && matches!(bound, Val::NumVal(_))
                            || ty.accepts(&Rhs::Val(bound.clone()))),
                    "min/max bounds must match the declared {ty:?} type"
                );
            }
        }
        if let (Some(min), Some(max)) = (&self.minimum, &self.maximum) {
            ensure!(
                compare(min, max)? == Ordering::Less
                    || compare(min, max)? == Ordering::Equal
                        && !self.minimum_exclusive
                        && !self.maximum_exclusive,
                "minimum must not exceed maximum"
            );
        }
        Ok(())
    }
    /// Validate one evaluated expression, identifying the variable in errors.
    pub fn validate(&self, name: &str, value: &Rhs) -> Result<()> {
        self.validate_inner(value)
            .map_err(|error| anyhow!("{name}: {error}"))
    }
    fn validate_inner(&self, value: &Rhs) -> Result<()> {
        self.definition()?;
        value.validate_finite()?;
        if let Some(ty) = &self.value_type {
            ensure!(
                ty.accepts(value),
                "expected {ty:?}, got {}",
                value.to_source()?
            );
        }
        if let Some(allowed) = &self.allowed {
            let text = match value {
                Rhs::Val(Val::StringVal(text)) | Rhs::Path(text) => text,
                _ => bail!(
                    "expected an enum choice ({}), got {}",
                    allowed.join(", "),
                    value.to_source()?
                ),
            };
            ensure!(
                allowed.iter().any(|choice| choice == text
                    || self.case_insensitive == Some(true) && choice.eq_ignore_ascii_case(text)),
                "expected one of [{}], got {text:?}",
                allowed.join(", ")
            );
        }
        self.validate_numeric(value)
    }
    fn validate_numeric(&self, value: &Rhs) -> Result<()> {
        if self.minimum.is_none() && self.maximum.is_none() {
            return Ok(());
        }
        if let Rhs::Array(values) = value {
            for (i, value) in values.iter().enumerate() {
                self.validate_numeric(value)
                    .map_err(|e| anyhow!("element [{i}]: {e}"))?;
            }
            return Ok(());
        }
        let Rhs::Val(value) = value else {
            bail!("numeric constraints require a scalar or array of numeric values");
        };
        for (bound, minimum, exclusive) in [
            (&self.minimum, true, self.minimum_exclusive),
            (&self.maximum, false, self.maximum_exclusive),
        ] {
            if let Some(bound) = bound {
                let order = compare(value, bound)?;
                let valid = if minimum {
                    order != Ordering::Less
                } else {
                    order != Ordering::Greater
                };
                ensure!(
                    valid && (!exclusive || order != Ordering::Equal),
                    "value {} must be {} {}",
                    value.to_source()?,
                    match (minimum, exclusive) {
                        (true, true) => ">",
                        (true, false) => ">=",
                        (false, true) => "<",
                        (false, false) => "<=",
                    },
                    bound.to_source()?
                );
            }
        }
        Ok(())
    }
    fn apply(&mut self, annotation: &str) -> Result<()> {
        let (key, value) = parts(annotation);
        match key {
            "required" => {
                self.required = Some(match value {
                    "" | "true" => true,
                    "false" => false,
                    _ => bail!("@required expects true or false"),
                })
            }
            "deprecated" => {
                self.deprecated = Some(if value.is_empty() {
                    "This setting is deprecated".into()
                } else {
                    value.trim_matches('"').into()
                })
            }
            "renamed_to" => {
                let value = value.trim_matches('"');
                ensure!(
                    !value.is_empty() && value.split('.').all(crate::format::valid_key),
                    "@renamed_to requires a fully qualified setting name"
                );
                self.renamed_to = Some(value.into());
            }
            "enum" | "allowed" | "options" => self.allowed = Some(choices(value)?),
            "type" => {
                let value = value.trim_matches('"');
                if let Some(options) = value
                    .strip_prefix("enum[")
                    .and_then(|s| s.strip_suffix(']'))
                {
                    self.allowed = Some(choices(options)?);
                } else {
                    self.value_type = Some(ValueType::parse(value)?);
                }
            }
            "min" | "minimum" | "min_exclusive" => {
                self.minimum = Some(bound(value)?);
                self.minimum_exclusive = key == "min_exclusive";
            }
            "max" | "maximum" | "max_exclusive" => {
                self.maximum = Some(bound(value)?);
                self.maximum_exclusive = key == "max_exclusive";
            }
            "finite" => bail!("@finite is not supported; all numbers must always be finite"),
            "enum_case" => {
                self.case_insensitive = Some(match value {
                    "insensitive" => true,
                    "sensitive" => false,
                    _ => bail!("@enum_case expects sensitive or insensitive"),
                })
            }
            _ => {}
        }
        Ok(())
    }
}

fn parts(annotation: &str) -> (&str, &str) {
    let end = annotation
        .find(|c: char| c.is_whitespace() || c == '=')
        .unwrap_or(annotation.len());
    let value = annotation[end..]
        .trim()
        .strip_prefix('=')
        .unwrap_or(annotation[end..].trim())
        .trim();
    (&annotation[..end], value)
}
pub(crate) fn is_metadata(annotation: &str) -> bool {
    matches!(
        parts(annotation).0,
        "required"
            | "deprecated"
            | "renamed_to"
            | "enum"
            | "allowed"
            | "options"
            | "type"
            | "min"
            | "minimum"
            | "max"
            | "maximum"
            | "min_exclusive"
            | "max_exclusive"
            | "finite"
            | "enum_case"
    )
}
pub(crate) fn annotations_before(source: &str, start: usize) -> Vec<String> {
    let mut annotations = Vec::new();
    for line in source[..start].lines().rev() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some(comment) = line.strip_prefix('#') else {
            break;
        };
        if let Some(annotation) = comment.trim().strip_prefix('@') {
            annotations.push(annotation.trim().to_owned());
        }
    }
    annotations.reverse();
    annotations
}
pub(crate) fn check_assignment_target(source: &str, start: usize) -> Result<()> {
    ensure!(
        !annotations_before(source, start)
            .iter()
            .any(|a| is_metadata(a)),
        "type and value constraints must annotate an assignment, not an include or namespace block"
    );
    Ok(())
}
fn choices(source: &str) -> Result<Vec<String>> {
    let source = source.trim();
    let source = if source.starts_with('[') {
        source
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
            .ok_or_else(|| anyhow!("unclosed enum choices"))?
    } else {
        source
    };
    let mut values = Vec::new();
    let mut quoted = false;
    let mut start = 0;
    for (i, c) in source.char_indices() {
        if c == '"' {
            quoted = !quoted;
        }
        if c == ',' && !quoted {
            values.push(&source[start..i]);
            start = i + 1;
        }
    }
    ensure!(!quoted, "unclosed quote in enum choices");
    values.push(&source[start..]);
    let mut unique = HashSet::new();
    values
        .into_iter()
        .map(|s| {
            let s = s.trim();
            let s = if s.starts_with('"') {
                s.strip_prefix('"')
                    .and_then(|s| s.strip_suffix('"'))
                    .ok_or_else(|| anyhow!("invalid quoted enum choice"))?
            } else {
                ensure!(
                    !s.chars().any(char::is_whitespace) && !s.contains(['"', '[', ']']),
                    "enum choices containing spaces must be quoted"
                );
                s
            };
            ensure!(!s.contains('"'), "invalid quote in enum choice");
            ensure!(!s.is_empty(), "enum choices cannot be empty");
            ensure!(unique.insert(s.to_owned()), "duplicate enum choice {s:?}");
            Ok(s.to_owned())
        })
        .collect()
}
fn bound(source: &str) -> Result<Val> {
    let mut tokens = Token::lexer(source);
    let value = match tokens.next() {
        Some(Ok(Token::IntegerNumber(v))) => Val::NumVal(NumVal::Integer(v)),
        Some(Ok(Token::FloatingNumber(v))) => Val::NumVal(NumVal::Floating(v)),
        Some(Ok(Token::Length(v))) => Val::UnitedVal(UnitVal::LengthVal(v)),
        Some(Ok(Token::Physical(v))) => Val::UnitedVal(v),
        Some(Ok(Token::Time(v))) => Val::UnitedVal(UnitVal::TimeVal(v)),
        _ => bail!("expected one numeric min/max bound, optionally with length or time units"),
    };
    ensure!(
        tokens.next().is_none(),
        "expected one numeric min/max bound"
    );
    compare(&value, &value)?;
    Ok(value)
}
fn compare(left: &Val, right: &Val) -> Result<Ordering> {
    if let (Val::NumVal(NumVal::Integer(a)), Val::NumVal(NumVal::Integer(b))) = (left, right) {
        return Ok(a.cmp(b));
    }
    let (a, b) = match (left, right) {
        (Val::NumVal(NumVal::Integer(a)), Val::NumVal(NumVal::Floating(b))) => {
            return compare_integer_float(*a, *b);
        }
        (Val::NumVal(NumVal::Floating(a)), Val::NumVal(NumVal::Integer(b))) => {
            return Ok(compare_integer_float(*b, *a)?.reverse());
        }
        (Val::NumVal(NumVal::Floating(a)), Val::NumVal(NumVal::Floating(b))) => (*a, *b),
        (Val::UnitedVal(UnitVal::LengthVal(a)), Val::UnitedVal(UnitVal::LengthVal(b))) => {
            (a.get::<meter>(), b.get::<meter>())
        }
        (Val::UnitedVal(UnitVal::TimeVal(a)), Val::UnitedVal(UnitVal::TimeVal(b))) => {
            (a.get::<second>(), b.get::<second>())
        }
        (Val::UnitedVal(a), Val::UnitedVal(b)) if a.unit() == b.unit() => {
            (a.si_value(), b.si_value())
        }
        _ => bail!("min/max values must be numeric and have matching dimensions"),
    };
    ensure!(
        a.is_finite() && b.is_finite(),
        "min/max values must be finite"
    );
    Ok(a.partial_cmp(&b).unwrap())
}

fn compare_integer_float(integer: i64, float: f64) -> Result<Ordering> {
    ensure!(float.is_finite(), "min/max values must be finite");
    if float >= 9223372036854775808.0 {
        return Ok(Ordering::Less);
    }
    if float < -9223372036854775808.0 {
        return Ok(Ordering::Greater);
    }
    Ok(match integer.cmp(&(float as i64)) {
        Ordering::Equal if float.fract() > 0.0 => Ordering::Less,
        Ordering::Equal if float.fract() < 0.0 => Ordering::Greater,
        order => order,
    })
}

impl VariableHistory {
    /// Collect declared constraints, including declarations from included files.
    /// Namespace-filtered histories return relative names. This does not mark
    /// values used. Later declarations replace only the rules they specify.
    pub fn constraints(&self) -> Result<BTreeMap<String, Constraints>> {
        let mut fields: BTreeMap<String, Constraints> = BTreeMap::new();
        for statement in &self.ast {
            let StatementKindOwned::VariableDef(Statement::AssignLeft {
                lhs, annotations, ..
            }) = statement;
            let metadata: Vec<_> = annotations.iter().filter(|a| is_metadata(a)).collect();
            if metadata.is_empty() {
                continue;
            }
            for var in lhs {
                let name = crate::format_var(var);
                let field = fields.entry(name.clone()).or_default();
                for annotation in &metadata {
                    field
                        .apply(annotation)
                        .map_err(|e| anyhow!("{name}: {e}"))?;
                }
            }
        }
        for (name, field) in &fields {
            field.definition().map_err(|e| anyhow!("{name}: {e}"))?;
        }
        Ok(fields)
    }
    /// Check final resolved values without changing application read tracking.
    pub fn validate_constraints(&self) -> Result<()> {
        // Check every literal and initial-state value, including nested values.
        // References can only copy these values; no application reads are recorded.
        for (name, value) in &self.var_cache {
            value
                .validate_finite()
                .map_err(|error| anyhow!("{}: {error}", crate::format_var(name)))?;
        }
        for statement in &self.ast {
            let StatementKindOwned::VariableDef(Statement::AssignLeft { lhs, rhs, .. }) = statement;
            for value in rhs {
                value.validate_finite().map_err(|error| {
                    anyhow!(
                        "{}: {error}",
                        lhs.first()
                            .map(crate::format_var)
                            .unwrap_or_else(|| "<assignment>".into())
                    )
                })?;
            }
        }

        for (name, field) in self.constraints()? {
            if let Some(value) = self.resolve_recursive(&name.parse()?, None)? {
                field.validate(&name, &value)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "constraints_tests.rs"]
mod tests;

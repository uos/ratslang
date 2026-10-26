//! Serde support for ratslang configurations.
//!
//! Enable the `serde` feature (which enables `uom` integration). Use
//! `#[serde(with = "ratslang::serde::length")]` or `time` on quantity fields:
//! uom's own Serde implementation uses bare base-unit numbers and cannot retain
//! the dimension. These adapters emit native ratslang quantities instead.
//!
//! ```
//! use serde::{Deserialize, Serialize};
//! use ratslang::{Length, Time, micrometer, millisecond};
//!
//! #[derive(Debug, Serialize, Deserialize, PartialEq)]
//! struct Config {
//!     #[serde(with = "ratslang::serde::length")]
//!     distance: Length,
//!     #[serde(with = "ratslang::serde::time")]
//!     timeout: Time,
//! }
//! let config = Config {
//!     distance: Length::new::<micrometer>(1.0),
//!     timeout: Time::new::<millisecond>(250.0),
//! };
//! let source = ratslang::to_string(&config)?;
//! assert!(source.contains("distance = 1um"));
//! assert_eq!(ratslang::from_str::<Config>(&source)?, config);
//! # Ok::<(), ratslang::serde::Error>(())
//! ```
//!
//! The document root must be a struct or string-keyed map. Nested maps become
//! namespaces. `None` fields are omitted; ratslang has no null literal. Arrays
//! can contain literals and other arrays, but cannot contain maps or nulls.
//!
//! Serialization builds the compiler AST via [to_ast], then delegates text and
//! unit formatting to [Root::to_source]. Deserialization uses the existing
//! compiler and visits its evaluated AST expressions.
//!
//! ## Preserve source, comments, and annotations
//!
//! Plain Rust structs retain values. Wrap them in [`Document<T>`] to retain the
//! source as well. Unchanged documents serialize byte for byte; edits append
//! overrides with the original annotations. Unknown fields and includes stay in
//! place. Removing fields returns an error rather than silently restoring their
//! original values. [`Value`] supports arbitrary configurations without a schema.
//!
//! ```
//! use ratslang::serde::{Document, Value};
//! let source = "# @recommended\n# Keep this comment\nresolution = 50cm\nresolution.type = Multi\nrange = 4m..\n";
//! let document: Document<Value> = ratslang::from_str(source)?;
//! assert_eq!(ratslang::to_string(&document)?, source);
//! # Ok::<(), ratslang::serde::Error>(())
//! ```
//!
//! ## Values with children
//!
//! A node such as `resolution = 50cm` with `resolution.type = Multi` is a map
//! whose `$value` entry holds the length and whose `type` entry holds `Multi`.
//! In a typed child struct, use `#[serde(rename = "$value", with =
//! "ratslang::serde::length")]` on the value field. The same convention works
//! for booleans, strings, and other expressions with child namespaces.
//!
//! ## Ranges
//!
//! Use [`Range<T>`] for optional bounds, or put
//! `#[serde(with = "ratslang::serde::range")]` on a standard Rust range or a
//! struct with `start`/`end` fields. This explicit adapter preserves `a..b`,
//! `a..`, `..b`, and `..` without mistaking ordinary structs for ranges.

use crate::{Length, NumVal, Rhs, Root, Statement, Time, UnitVal, Val, Var, meter, second};
use ::serde::{
    Serialize,
    de::{self, DeserializeOwned, IntoDeserializer, Visitor},
    ser,
};
use std::{collections::BTreeMap, fmt};

const MASS: &str = "$ratslang::Mass";
const LENGTH: &str = "$ratslang::Length";
const TIME: &str = "$ratslang::Time";
const ANGLE: &str = "$ratslang::Angle";
const FREQUENCY: &str = "$ratslang::Frequency";
const VELOCITY: &str = "$ratslang::Velocity";
const ACCELERATION: &str = "$ratslang::Acceleration";

const RANGE: &str = "$ratslang::Range";
const RAW: &str = "$ratslang::Value";
const DOCUMENT: &str = "$ratslang::Document";

#[path = "serde_document.rs"]
mod document;
pub use document::Document;
#[path = "serde_range.rs"]
pub mod range;
pub use range::Range;
#[path = "serde_value.rs"]
mod dynamic;
#[path = "serde_helpers.rs"]
mod helpers;
pub use helpers::{Quantity, case_insensitive};

/// A serialization, parsing, or deserialization error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(String);
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
impl ser::Error for Error {
    fn custom<T: fmt::Display>(message: T) -> Self {
        Self(message.to_string())
    }
}
impl de::Error for Error {
    fn custom<T: fmt::Display>(message: T) -> Self {
        Self(message.to_string())
    }
}
type Result<T> = std::result::Result<T, Error>;
fn error(message: impl fmt::Display) -> Error {
    Error(message.to_string())
}

// Serde adds only namespaces and omitted fields; expressions use the compiler AST.
/// An evaluated AST expression or namespace for configurations without a Rust schema.
///
/// Nodes that have both a value and children store the value under `$value`.
/// Use [`Document<Value>`] to retain comments and original source as well.
#[derive(Debug, Clone)]
pub enum Value {
    None,
    Rhs(Rhs),
    Map(BTreeMap<String, Value>),
    #[doc(hidden)]
    Source(String, Box<Value>),
    #[doc(hidden)]
    Deferred(
        std::rc::Rc<crate::VariableHistory>,
        String,
        Option<std::rc::Rc<crate::VariableHistory>>,
        Option<crate::Constraints>,
    ),
}
impl Value {
    fn resolve(self) -> Result<Self> {
        if let Self::Rhs(rhs) = &self {
            rhs.validate_finite().map_err(error)?;
        }
        match self {
            Self::Deferred(vars, name, probe, constraints) => {
                if let Some(probe) = probe {
                    probe.evaluate_stats_only(&name).map_err(error)?;
                }
                let rhs = vars
                    .resolve(&name)
                    .map_err(error)?
                    .ok_or_else(|| error(format!("variable {name} has no value")))?;
                if let Some(constraints) = constraints {
                    constraints.validate(&name, &rhs).map_err(error)?;
                }
                from_rhs(rhs)
            }
            value => Ok(value),
        }
    }
    fn literal(value: Val) -> Self {
        Self::Rhs(Rhs::Val(value))
    }
    fn into_rhs(self) -> Result<Rhs> {
        match self.resolve()? {
            Self::Rhs(rhs) => Ok(rhs),
            Self::None => Err(error(
                "null values are only supported as omitted map fields",
            )),
            Self::Map(_) => Err(error("ratslang arrays cannot contain maps or structs")),
            Self::Source(_, _) => Err(error("a document cannot be an array element")),
            Self::Deferred(_, _, _, _) => unreachable!(),
        }
    }
}

/// Serialize a struct or map to a ratslang document.
pub fn to_string<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    match value.serialize(Serializer)? {
        Value::Source(source, _) => Ok(source),
        value => value_to_ast(value)?.to_source().map_err(error),
    }
}

/// Convert a struct or map into compiler AST assignments.
///
/// Inspect the result or format it using [Root::to_source].
pub fn to_ast<T: Serialize + ?Sized>(value: &T) -> Result<Root> {
    let value = value.serialize(Serializer)?;
    let value = if let Value::Source(source, _) = value {
        source_value(&source)?
    } else {
        value
    };
    value_to_ast(value)
}

fn value_to_ast(value: Value) -> Result<Root> {
    let Value::Map(map) = value else {
        return Err(error("the document root must be a struct or map"));
    };
    let mut root = Root {
        vardefs: Vec::new(),
    };
    append_assignments(map, "", &mut root)?;
    Ok(root)
}

/// Compile a ratslang document and deserialize its evaluated values.
///
/// Includes are resolved by the existing compiler relative to the current
/// directory. References, overrides, and namespaces use normal ratslang rules.
pub fn from_str<T: DeserializeOwned>(source: &str) -> Result<T> {
    from_evaluated(&compile_source(source)?)
}

/// Deserialize an already compiled configuration without recompiling its source.
///
/// Includes, references, initial state, and overrides use the supplied evaluation.
/// Fields are resolved only when serde reads them; ignored fields are not marked
/// used. Read tracking is shared with `evaluated.vars.usage_vars()`.
/// This function does not merge separate default and user configurations.
///
/// ```
/// use serde::Deserialize;
/// #[derive(Deserialize)]
/// struct Config { count: usize }
/// let evaluated = ratslang::compile_code("count = 3\n")?;
/// let config: Config = ratslang::from_evaluated(&evaluated)?;
/// assert_eq!(config.count, 3);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn from_evaluated<T: DeserializeOwned>(evaluated: &crate::Evaluated) -> Result<T> {
    T::deserialize(Value::Source(
        evaluated.source_text.clone(),
        Box::new(evaluated_value(evaluated)?),
    ))
}

fn compile_source(source: &str) -> Result<crate::Evaluated> {
    let mut diagnostics = Vec::new();
    let mut evaluated = crate::compile_code_with_state(
        &format!("{source}\n"),
        &crate::working_dir(),
        None,
        &mut diagnostics,
        false,
    )
    .map_err(|cause| {
        let details = String::from_utf8_lossy(&diagnostics);
        if details.is_empty() {
            error(cause)
        } else {
            error(format!("{cause}: {details}"))
        }
    })?;
    evaluated.source_text = source.to_owned();
    evaluated.vars = evaluated.vars.with_drop_warning(false);
    Ok(evaluated)
}

fn source_value(source: &str) -> Result<Value> {
    evaluated_value(&compile_source(source)?)
}

fn evaluated_value(evaluated: &crate::Evaluated) -> Result<Value> {
    variables_value(&evaluated.vars, None)
}

/// Deserialize variable histories, preferring each user value over its default.
///
/// Histories may be namespace-filtered, for example to read a camera with a
/// different default namespace. Resolution retains each history's context and
/// shares its usage tracking; ignored settings remain unread.
/// When a consumed value comes from defaults, the corresponding user key is
/// also recorded as read, matching `resolve_var!`'s fallback probes. This lets
/// callers distinguish recognized settings from unknown overrides.
/// Default comment constraints are applied to consumed user overrides. Explicit
/// user constraint declarations replace the corresponding default rules.
pub fn from_variables_with_defaults<T: DeserializeOwned>(
    defaults: &crate::VariableHistory,
    user: &crate::VariableHistory,
) -> Result<T> {
    let names = variable_names(user);
    let default_rules = defaults.constraints().map_err(error)?;
    let user_rules = user.constraints().map_err(error)?;
    for (name, rules) in default_rules {
        let rules = user_rules
            .get(&name)
            .map(|later| rules.overlay(later))
            .unwrap_or(rules);
        if rules.required == Some(true) && !names.contains(&name) {
            return Err(error(format!(
                "{name}: required setting must be explicitly assigned"
            )));
        }
    }
    T::deserialize(variables_value(user, Some(defaults))?)
}

/// Reusable typed access to differently shaped configuration namespaces.
///
/// Build once from already compiled user/default histories. Each call uses
/// serde and the declared constraints, and only the requested value is marked
/// as read. A node with both a value and children returns its scalar value.
#[derive(Clone)]
pub struct ConfigReader {
    root: Value,
    defaults: crate::VariableHistory,
    user: crate::VariableHistory,
}
impl ConfigReader {
    pub fn new(defaults: &crate::VariableHistory, user: &crate::VariableHistory) -> Result<Self> {
        Ok(Self {
            root: variables_value(user, Some(defaults))?,
            defaults: defaults.clone().with_drop_warning(false),
            user: user.clone().with_drop_warning(false),
        })
    }
    fn missing<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.user.evaluate_stats_only(path).map_err(error)?;
        self.defaults.evaluate_stats_only(path).map_err(error)?;
        T::deserialize(Value::None).map_err(|e| error(format!("{path}: {e}")))
    }
    pub fn deserialize<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let mut value = &self.root;
        for part in path.split('.') {
            let Value::Map(map) = value else {
                return self.missing(path);
            };
            let Some(next) = map.get(part) else {
                return self.missing(path);
            };
            value = next;
        }
        if let Value::Map(map) = value {
            if let Some(scalar) = map.get("$value") {
                value = scalar;
            }
        }
        T::deserialize(value.clone()).map_err(|e| error(format!("{path}: {e}")))
    }
}

fn variable_names(vars: &crate::VariableHistory) -> std::collections::BTreeSet<String> {
    vars.ast
        .iter()
        .flat_map(|statement| {
            let crate::StatementKindOwned::VariableDef(Statement::AssignLeft { lhs, .. }) =
                statement;
            lhs.iter().map(crate::format_var)
        })
        .chain(vars.var_cache.keys().map(crate::format_var))
        .collect()
}

fn variables_value(
    user: &crate::VariableHistory,
    defaults: Option<&crate::VariableHistory>,
) -> Result<Value> {
    let mut root = BTreeMap::new();
    // Clone once: VariableHistory shares usage tracking with the caller.
    let vars = std::rc::Rc::new(user.clone().with_drop_warning(false));
    let default_vars = defaults.map(|vars| std::rc::Rc::new(vars.clone().with_drop_warning(false)));
    let user_names = variable_names(user);
    let mut constraints = defaults
        .map(|vars| vars.constraints())
        .transpose()
        .map_err(error)?
        .unwrap_or_default();
    for (name, later) in user.constraints().map_err(error)? {
        let earlier = constraints.entry(name).or_default();
        *earlier = earlier.overlay(&later);
    }
    let mut names = user_names.clone();
    if let Some(defaults) = defaults {
        names.extend(variable_names(defaults));
    }
    for name in names {
        let history = if user_names.contains(&name) {
            &vars
        } else {
            default_vars.as_ref().unwrap()
        };
        insert(
            &mut root,
            &name.split('.').collect::<Vec<_>>(),
            Value::Deferred(
                history.clone(),
                name.clone(),
                (!user_names.contains(&name)).then(|| vars.clone()),
                constraints.remove(&name),
            ),
        )?;
    }
    Ok(Value::Map(root))
}

fn insert(map: &mut BTreeMap<String, Value>, path: &[&str], value: Value) -> Result<()> {
    if path.len() == 1 {
        map.insert(path[0].into(), value);
        return Ok(());
    }
    let entry = map
        .entry(path[0].into())
        .or_insert_with(|| Value::Map(BTreeMap::new()));
    if !matches!(entry, Value::Map(_)) {
        let local = std::mem::replace(entry, Value::None);
        *entry = Value::Map(BTreeMap::from([("$value".into(), local)]));
    }
    let Value::Map(child) = entry else {
        unreachable!()
    };
    insert(child, &path[1..], value)
}

fn from_rhs(rhs: Rhs) -> Result<Value> {
    match rhs {
        Rhs::Var(_) => Err(error("unresolved variable")),
        rhs => Ok(Value::Rhs(rhs)),
    }
}

fn parse_expression(source: &str) -> Result<Rhs> {
    let Value::Map(mut values) = source_value(&format!("value = {source}\n"))? else {
        unreachable!()
    };
    if values.len() != 1 {
        return Err(error("expected one expression"));
    }
    values
        .remove("value")
        .ok_or_else(|| error("expected an expression"))?
        .into_rhs()
}

fn range_fields(from: Option<Val>, to: Option<Val>) -> BTreeMap<String, Value> {
    BTreeMap::from([
        (
            "start".into(),
            from.map(Value::literal).unwrap_or(Value::None),
        ),
        ("end".into(), to.map(Value::literal).unwrap_or(Value::None)),
    ])
}

fn append_assignments(map: BTreeMap<String, Value>, prefix: &str, root: &mut Root) -> Result<()> {
    for (key, value) in map {
        let local = key == "$value" && !prefix.is_empty();
        if !local && !crate::format::valid_key(&key) {
            return Err(error(format!("invalid ratslang key: {key:?}")));
        }
        let path = if local {
            prefix.to_owned()
        } else if prefix.is_empty() {
            key
        } else {
            format!("{prefix}.{key}")
        };
        match value.resolve()? {
            Value::None => {}
            Value::Map(child) => {
                if child.is_empty() {
                    return Err(error(format!(
                        "empty map at {path} has no ratslang representation"
                    )));
                }
                let start = root.vardefs.len();
                append_assignments(child, &path, root)?;
                if root.vardefs.len() == start {
                    return Err(error(format!(
                        "namespace {path} has no serializable fields"
                    )));
                }
            }
            Value::Rhs(rhs) => root.vardefs.push(Statement::AssignLeft {
                lhs: vec![path.parse::<Var>().map_err(error)?],
                rhs: vec![rhs],
                annotations: Vec::new(),
            }),
            Value::Source(_, _) => return Err(error("documents must be at the root")),
            Value::Deferred(_, _, _, _) => unreachable!(),
        }
    }
    Ok(())
}

struct Serializer;
macro_rules! signed {
    ($($method:ident: $ty:ty),*) => {$(fn $method(self, v: $ty) -> Result<Value> { Ok(Value::literal(Val::NumVal(NumVal::Integer(v as i64)))) })*};
}
macro_rules! unsigned {
    ($($method:ident: $ty:ty),*) => {$(
        fn $method(self, v: $ty) -> Result<Value> {
            let n = i64::try_from(v as u64)
                .map_err(|_| error("integer exceeds ratslang's i64 range"))?;
            Ok(Value::literal(Val::NumVal(NumVal::Integer(n))))
        }
    )*};
}
impl ser::Serializer for Serializer {
    type Ok = Value;
    type Error = Error;
    type SerializeSeq = Sequence;
    type SerializeTuple = Sequence;
    type SerializeTupleStruct = Sequence;
    type SerializeTupleVariant = Sequence;
    type SerializeMap = Mapping;
    type SerializeStruct = Mapping;
    type SerializeStructVariant = Mapping;
    signed!(serialize_i8:i8, serialize_i16:i16, serialize_i32:i32, serialize_i64:i64);
    unsigned!(serialize_u8:u8, serialize_u16:u16, serialize_u32:u32, serialize_u64:u64);
    fn serialize_bool(self, v: bool) -> Result<Value> {
        Ok(Value::literal(Val::BoolVal(v)))
    }
    fn serialize_f32(self, v: f32) -> Result<Value> {
        self.serialize_f64(v as f64)
    }
    fn serialize_f64(self, v: f64) -> Result<Value> {
        if !v.is_finite() {
            return Err(error("non-finite numbers have no ratslang representation"));
        }
        Ok(Value::literal(Val::NumVal(NumVal::Floating(v))))
    }
    fn serialize_char(self, v: char) -> Result<Value> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_str(self, v: &str) -> Result<Value> {
        Ok(Value::literal(Val::StringVal(v.into())))
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<Value> {
        Ok(Value::Rhs(Rhs::Array(
            v.iter()
                .map(|b| Box::new(Rhs::Val(Val::NumVal(NumVal::Integer(*b as i64)))))
                .collect(),
        )))
    }
    fn serialize_none(self) -> Result<Value> {
        Ok(Value::None)
    }
    fn serialize_some<T: Serialize + ?Sized>(self, v: &T) -> Result<Value> {
        v.serialize(self)
    }
    fn serialize_unit(self) -> Result<Value> {
        Err(error("ratslang has no unit/null literal"))
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<Value> {
        self.serialize_unit()
    }
    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
    ) -> Result<Value> {
        self.serialize_str(variant)
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        name: &'static str,
        v: &T,
    ) -> Result<Value> {
        let value = v.serialize(self)?;
        if name == DOCUMENT || name == RAW {
            let Value::Rhs(Rhs::Val(Val::StringVal(source))) = value else {
                return Err(error("source marker requires a string"));
            };
            if name == DOCUMENT {
                return Ok(Value::Source(source, Box::new(Value::None)));
            }
            parse_expression(&source).map(Value::Rhs)
        } else if name == RANGE {
            range::into_range(value).map(Value::Rhs)
        } else if matches!(
            name,
            MASS | LENGTH | TIME | ANGLE | FREQUENCY | VELOCITY | ACCELERATION
        ) {
            let Value::Rhs(Rhs::Val(Val::NumVal(NumVal::Floating(f)))) = value else {
                return Err(error("quantity must contain an f64"));
            };
            let quantity = match name {
                MASS => UnitVal::MassVal(crate::Mass::new::<crate::kilogram>(f)),
                LENGTH => UnitVal::from_length(Length::new::<meter>(f)),
                TIME => UnitVal::from_time(Time::new::<second>(f)),
                ANGLE => UnitVal::AngleVal(crate::Angle::new::<crate::radian>(f)),
                FREQUENCY => UnitVal::FrequencyVal(crate::Frequency::new::<crate::hertz>(f)),
                VELOCITY => {
                    UnitVal::VelocityVal(crate::Velocity::new::<crate::meter_per_second>(f))
                }
                ACCELERATION => UnitVal::AccelerationVal(crate::Acceleration::new::<
                    crate::meter_per_second_squared,
                >(f)),
                _ => unreachable!(),
            };
            Ok(Value::literal(Val::UnitedVal(quantity)))
        } else {
            Ok(value)
        }
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        v: &T,
    ) -> Result<Value> {
        Ok(Value::Map(BTreeMap::from([(
            variant.into(),
            v.serialize(self)?,
        )])))
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Sequence> {
        Ok(Sequence(Vec::new(), None))
    }
    fn serialize_tuple(self, len: usize) -> Result<Sequence> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_struct(self, _: &'static str, len: usize) -> Result<Sequence> {
        self.serialize_tuple(len)
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: usize,
    ) -> Result<Sequence> {
        Ok(Sequence(Vec::new(), Some(variant)))
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Mapping> {
        Ok(Mapping {
            values: BTreeMap::new(),
            key: None,
            variant: None,
        })
    }
    fn serialize_struct(self, _: &'static str, len: usize) -> Result<Mapping> {
        self.serialize_map(Some(len))
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Mapping> {
        let mut map = self.serialize_map(Some(len))?;
        map.variant = Some(variant);
        Ok(map)
    }
}
struct Sequence(Vec<Rhs>, Option<&'static str>);
impl ser::SerializeSeq for Sequence {
    type Ok = Value;
    type Error = Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<()> {
        self.0.push(v.serialize(Serializer)?.into_rhs()?);
        Ok(())
    }
    fn end(self) -> Result<Value> {
        let value = Value::Rhs(Rhs::Array(self.0.into_iter().map(Box::new).collect()));
        Ok(if let Some(tag) = self.1 {
            Value::Map(BTreeMap::from([(tag.into(), value)]))
        } else {
            value
        })
    }
}
macro_rules! sequence {
    ($trait:ident, $method:ident) => {
        impl ser::$trait for Sequence {
            type Ok = Value;
            type Error = Error;
            fn $method<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<()> {
                ser::SerializeSeq::serialize_element(self, v)
            }
            fn end(self) -> Result<Value> {
                ser::SerializeSeq::end(self)
            }
        }
    };
}
sequence!(SerializeTuple, serialize_element);
sequence!(SerializeTupleStruct, serialize_field);
sequence!(SerializeTupleVariant, serialize_field);
struct Mapping {
    values: BTreeMap<String, Value>,
    key: Option<String>,
    variant: Option<&'static str>,
}
impl ser::SerializeMap for Mapping {
    type Ok = Value;
    type Error = Error;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<()> {
        let Value::Rhs(Rhs::Val(Val::StringVal(key))) = key.serialize(Serializer)? else {
            return Err(error("map keys must be strings"));
        };
        self.key = Some(key);
        Ok(())
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        let key = self
            .key
            .take()
            .ok_or_else(|| error("map value without a key"))?;
        if self
            .values
            .insert(key, value.serialize(Serializer)?)
            .is_some()
        {
            return Err(error("duplicate map key"));
        }
        Ok(())
    }
    fn end(self) -> Result<Value> {
        let value = Value::Map(self.values);
        Ok(if let Some(tag) = self.variant {
            Value::Map(BTreeMap::from([(tag.into(), value)]))
        } else {
            value
        })
    }
}
macro_rules! mapping {
    ($trait:ident) => {
        impl ser::$trait for Mapping {
            type Ok = Value;
            type Error = Error;
            fn serialize_field<T: Serialize + ?Sized>(
                &mut self,
                key: &'static str,
                value: &T,
            ) -> Result<()> {
                ser::SerializeMap::serialize_key(self, key)?;
                ser::SerializeMap::serialize_value(self, value)
            }
            fn end(self) -> Result<Value> {
                ser::SerializeMap::end(self)
            }
        }
    };
}
mapping!(SerializeStruct);
mapping!(SerializeStructVariant);

impl<'de> IntoDeserializer<'de, Error> for Value {
    type Deserializer = Self;
    fn into_deserializer(self) -> Self {
        self
    }
}
impl<'de> de::Deserializer<'de> for Value {
    type Error = Error;
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if let Self::Rhs(rhs) = &self {
            rhs.validate_finite().map_err(error)?;
        }
        match self {
            Self::Deferred(_, _, _, _) => self.resolve()?.deserialize_any(visitor),
            Self::None => visitor.visit_none(),
            Self::Source(_, value) => value.deserialize_any(visitor),
            Self::Rhs(Rhs::Val(Val::BoolVal(b))) => visitor.visit_bool(b),
            Self::Rhs(Rhs::Val(Val::NumVal(NumVal::Integer(i)))) => visitor.visit_i64(i),
            Self::Rhs(Rhs::Val(Val::NumVal(NumVal::Floating(f)))) => visitor.visit_f64(f),
            Self::Rhs(Rhs::Val(Val::StringVal(s)) | Rhs::Path(s)) => visitor.visit_string(s),
            Self::Rhs(Rhs::Array(values)) => {
                let values = values
                    .into_iter()
                    .map(|rhs| from_rhs(*rhs))
                    .collect::<Result<Vec<_>>>()?;
                let mut access = de::value::SeqDeserializer::new(values.into_iter());
                let result = visitor.visit_seq(&mut access)?;
                access.end()?;
                Ok(result)
            }
            Self::Map(values) => {
                let mut access = de::value::MapDeserializer::new(values.into_iter());
                let result = visitor.visit_map(&mut access)?;
                access.end()?;
                Ok(result)
            }
            Self::Rhs(Rhs::Val(Val::UnitedVal(_))) => Err(error(
                "use the ratslang length or time serde adapter for quantities",
            )),
            Self::Rhs(Rhs::Range { from, to }) => {
                Value::Map(range_fields(from, to)).deserialize_any(visitor)
            }
            Self::Rhs(Rhs::EmptyRange) => {
                Value::Map(range_fields(None, None)).deserialize_any(visitor)
            }
            Self::Rhs(Rhs::Var(_)) => Err(error("unresolved variable")),
        }
    }
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let value = self.resolve()?;
        if matches!(value, Self::None) {
            visitor.visit_none()
        } else {
            visitor.visit_some(value)
        }
    }
    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_unit()
    }
    fn deserialize_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        let (from, to) = match self.resolve()? {
            Self::Source(_, value) => return value.deserialize_struct(name, fields, visitor),
            Self::Rhs(Rhs::Range { from, to }) => (from, to),
            Self::Rhs(Rhs::EmptyRange) => (None, None),
            other => return other.deserialize_any(visitor),
        };
        let mut values = range_fields(from, to);
        for key in ["start", "end"] {
            if !fields.contains(&key) && !matches!(values.remove(key), Some(Value::None)) {
                return Err(error(format!(
                    "target range cannot represent the {key} bound"
                )));
            }
        }
        Self::Map(values).deserialize_any(visitor)
    }
    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        let self_value = self.resolve()?;
        if let Self::Source(source, value) = self_value {
            return if name == DOCUMENT {
                visitor.visit_newtype_struct(Self::Map(BTreeMap::from([
                    ("source".into(), Self::literal(Val::StringVal(source))),
                    ("value".into(), *value),
                ])))
            } else {
                value.deserialize_newtype_struct(name, visitor)
            };
        }
        if name == RAW {
            return match self_value {
                Self::Rhs(rhs) => visitor.visit_newtype_struct(Self::literal(Val::StringVal(
                    rhs.to_source().map_err(error)?,
                ))),
                other => visitor.visit_newtype_struct(other),
            };
        }
        if name == RANGE {
            return match self_value {
                Self::Rhs(Rhs::Range { .. } | Rhs::EmptyRange) => {
                    visitor.visit_newtype_struct(self_value)
                }
                _ => Err(error("expected a native ratslang range")),
            };
        }
        if matches!(
            name,
            MASS | LENGTH | TIME | ANGLE | FREQUENCY | VELOCITY | ACCELERATION
        ) {
            match self_value {
                Self::Rhs(Rhs::Val(Val::UnitedVal(quantity)))
                    if quantity_marker(quantity.unit()) == name =>
                {
                    visitor.visit_newtype_struct(Self::literal(Val::NumVal(NumVal::Floating(
                        quantity.si_value(),
                    ))))
                }
                _ => Err(error(format!(
                    "expected a {} quantity with matching units",
                    name.trim_start_matches("$ratslang::")
                ))),
            }
        } else {
            visitor.visit_newtype_struct(self_value)
        }
    }
    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let value = self.resolve()?;
        if let Self::Source(_, inner) = value {
            return inner.deserialize_f32(visitor);
        }
        if let Self::Rhs(Rhs::Val(Val::NumVal(NumVal::Floating(number)))) = &value {
            if !(*number as f32).is_finite() {
                return Err(error("number cannot be represented as a finite f32"));
            }
        }
        value.deserialize_any(visitor)
    }
    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        match self.resolve()? {
            Self::Source(_, value) => value.deserialize_enum("", &[], visitor),
            Self::Rhs(Rhs::Val(Val::StringVal(tag)) | Rhs::Path(tag)) => {
                visitor.visit_enum(de::value::StringDeserializer::<Error>::new(tag))
            }
            Self::Map(map) if map.len() == 1 => {
                let (tag, value) = map.into_iter().next().unwrap();
                visitor.visit_enum(Variant(tag, value))
            }
            _ => Err(error("expected an enum name or a single-variant namespace")),
        }
    }
    ::serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f64 char str string
        bytes byte_buf unit unit_struct seq tuple tuple_struct map identifier
    }
}
struct Variant(String, Value);
impl<'de> de::EnumAccess<'de> for Variant {
    type Error = Error;
    type Variant = Value;
    fn variant_seed<V: de::DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Value)> {
        Ok((
            seed.deserialize(de::value::StringDeserializer::<Error>::new(self.0))?,
            self.1,
        ))
    }
}
impl<'de> de::VariantAccess<'de> for Value {
    type Error = Error;
    fn unit_variant(self) -> Result<()> {
        Err(error("expected a bare enum name"))
    }
    fn newtype_variant_seed<T: de::DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value> {
        seed.deserialize(self)
    }
    fn tuple_variant<V: Visitor<'de>>(self, _: usize, visitor: V) -> Result<V::Value> {
        de::Deserializer::deserialize_seq(self, visitor)
    }
    fn struct_variant<V: Visitor<'de>>(
        self,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        de::Deserializer::deserialize_map(self, visitor)
    }
}

fn quantity_marker(unit: crate::Unit) -> &'static str {
    match unit {
        crate::Unit::Mass => MASS,
        crate::Unit::Length => LENGTH,
        crate::Unit::Time => TIME,
        crate::Unit::Angle => ANGLE,
        crate::Unit::Frequency => FREQUENCY,
        crate::Unit::Velocity => VELOCITY,
        crate::Unit::Acceleration => ACCELERATION,
    }
}
macro_rules! quantity_adapter {
    ($module:ident, $ty:ident, $unit:ident, $marker:ident) => {
        /// Serde field adapter for a typed uom quantity in native ratslang syntax.
        pub mod $module {
            use super::*;
            use crate::{$ty, $unit};
            pub fn serialize<S: ser::Serializer>(
                value: &$ty,
                serializer: S,
            ) -> std::result::Result<S::Ok, S::Error> {
                serializer.serialize_newtype_struct($marker, &value.get::<$unit>())
            }
            pub fn deserialize<'de, D: de::Deserializer<'de>>(
                deserializer: D,
            ) -> std::result::Result<$ty, D::Error> {
                struct QuantityVisitor;
                impl<'de> Visitor<'de> for QuantityVisitor {
                    type Value = $ty;
                    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        f.write_str("a physical quantity with matching units")
                    }
                    fn visit_newtype_struct<D: de::Deserializer<'de>>(
                        self,
                        d: D,
                    ) -> std::result::Result<$ty, D::Error> {
                        let value = <f64 as ::serde::Deserialize>::deserialize(d)?;
                        if !value.is_finite() {
                            return Err(de::Error::custom("quantity must be finite"));
                        }
                        Ok($ty::new::<$unit>(value))
                    }
                }
                deserializer.deserialize_newtype_struct($marker, QuantityVisitor)
            }
        }
    };
}
quantity_adapter!(mass, Mass, kilogram, MASS);
quantity_adapter!(length, Length, meter, LENGTH);
quantity_adapter!(time, Time, second, TIME);
quantity_adapter!(angle, Angle, radian, ANGLE);
quantity_adapter!(frequency, Frequency, hertz, FREQUENCY);
quantity_adapter!(velocity, Velocity, meter_per_second, VELOCITY);
quantity_adapter!(
    acceleration,
    Acceleration,
    meter_per_second_squared,
    ACCELERATION
);

#[cfg(test)]
#[path = "serde_tests.rs"]
mod tests;

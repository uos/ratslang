//! Preserve native range syntax on structs with `start` and/or `end` fields.
//!
//! Use `#[serde(with = "ratslang::serde::range")]` on a range field, or use
//! [`Range`] for optional bounds. Quantity endpoints use the length/time adapters
//! on a newtype wrapper, just as ordinary quantity fields do.
//!
//! ```
//! use serde::{Deserialize, Serialize};
//! use ratslang::{Length, serde::Range};
//! #[derive(Serialize, Deserialize)]
//! struct Distance(#[serde(with = "ratslang::serde::length")] Length);
//! #[derive(Serialize, Deserialize)]
//! struct Config { range: Range<Distance> }
//! let config: Config = ratslang::from_str("range = 4m..\n")?;
//! assert_eq!(ratslang::to_string(&config)?, "range = 4m..\n");
//! # Ok::<(), ratslang::serde::Error>(())
//! ```

use super::*;
use ::serde::{Deserialize, Deserializer, Serializer};
use std::marker::PhantomData;

/// A native ratslang range, including open bounds and the unbounded `..`.
#[derive(Debug, Clone, PartialEq)]
pub struct Range<T> {
    pub start: Option<T>,
    pub end: Option<T>,
}
#[derive(Serialize, Deserialize)]
struct Fields<T> {
    start: Option<T>,
    end: Option<T>,
}

impl<T: Serialize> Serialize for Range<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serialize(
            &Fields {
                start: self.start.as_ref(),
                end: self.end.as_ref(),
            },
            serializer,
        )
    }
}
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Range<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let fields: Fields<T> = deserialize(deserializer)?;
        Ok(Self {
            start: fields.start,
            end: fields.end,
        })
    }
}

pub fn serialize<T: Serialize + ?Sized, S: Serializer>(
    value: &T,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    serializer.serialize_newtype_struct(RANGE, value)
}
pub fn deserialize<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<T, D::Error> {
    struct RangeVisitor<T>(PhantomData<T>);
    impl<'de, T: Deserialize<'de>> Visitor<'de> for RangeVisitor<T> {
        type Value = T;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a native ratslang range")
        }
        fn visit_newtype_struct<D: Deserializer<'de>>(
            self,
            d: D,
        ) -> std::result::Result<T, D::Error> {
            T::deserialize(d)
        }
    }
    deserializer.deserialize_newtype_struct(RANGE, RangeVisitor(PhantomData))
}

pub(super) fn into_range(value: Value) -> Result<Rhs> {
    let Value::Map(mut fields) = value else {
        return Err(error("range adapter expects start/end fields"));
    };
    let mut endpoint = |key| -> Result<Option<Val>> {
        match fields.remove(key) {
            None | Some(Value::None) => Ok(None),
            Some(Value::Rhs(Rhs::Val(value))) => Ok(Some(value)),
            _ => Err(error("range endpoints must be literals")),
        }
    };
    let from = endpoint("start")?;
    let to = endpoint("end")?;
    if !fields.is_empty() {
        return Err(error("range adapter only accepts start/end fields"));
    }
    if matches!((&from, &to), (Some(Val::UnitedVal(a)), Some(Val::UnitedVal(b))) if a.unit() != b.unit())
    {
        return Err(error("range endpoints must have matching dimensions"));
    }
    Ok(if from.is_none() && to.is_none() {
        Rhs::EmptyRange
    } else {
        Rhs::Range { from, to }
    })
}

//! Serde transport for arbitrary evaluated AST expressions and namespaces.
use super::*;
use ::serde::{Deserialize, Deserializer, Serializer, ser::SerializeMap};

impl Serialize for Value {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        match self {
            Self::None => serializer.serialize_none(),
            Self::Rhs(rhs) => serializer
                .serialize_newtype_struct(RAW, &rhs.to_source().map_err(ser::Error::custom)?),
            Self::Map(values) => {
                let mut map = serializer.serialize_map(Some(values.len()))?;
                for (key, value) in values {
                    map.serialize_entry(key, value)?;
                }
                map.end()
            }
            Self::Source(source, _) => serializer.serialize_newtype_struct(DOCUMENT, source),
            Self::Deferred(vars, name, probe, constraints) => Value::Deferred(
                vars.clone(),
                name.clone(),
                probe.clone(),
                constraints.clone(),
            )
            .resolve()
            .map_err(ser::Error::custom)?
            .serialize(serializer),
        }
    }
}
impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct ValueVisitor;
        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = Value;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an evaluated ratslang value or namespace")
            }
            fn visit_newtype_struct<D: Deserializer<'de>>(
                self,
                d: D,
            ) -> std::result::Result<Value, D::Error> {
                d.deserialize_any(self)
            }
            fn visit_str<E: de::Error>(self, source: &str) -> std::result::Result<Value, E> {
                parse_expression(source).map(Value::Rhs).map_err(E::custom)
            }
            fn visit_string<E: de::Error>(self, source: String) -> std::result::Result<Value, E> {
                self.visit_str(&source)
            }
            fn visit_map<A: de::MapAccess<'de>>(
                self,
                mut access: A,
            ) -> std::result::Result<Value, A::Error> {
                let mut map = BTreeMap::new();
                while let Some((key, value)) = access.next_entry::<String, Value>()? {
                    if map.insert(key, value).is_some() {
                        return Err(de::Error::custom("duplicate key"));
                    }
                }
                Ok(Value::Map(map))
            }
            fn visit_none<E: de::Error>(self) -> std::result::Result<Value, E> {
                Ok(Value::None)
            }
        }
        deserializer.deserialize_newtype_struct(RAW, ValueVisitor)
    }
}

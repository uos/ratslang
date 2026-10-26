//! Reusable field types and adapters for derived configurations.

use super::*;
use ::serde::{Deserialize, Deserializer, Serializer as SerdeSerializer};

/// A uom quantity that preserves its dimension in Ratslang without field adapters.
///
/// Implemented for mass, length, time, angle, frequency, velocity, and acceleration. Use it in optional fields, vectors,
/// arrays, matrices, and [`Range`] bounds. The inner uom value is accessible via
/// `.0`, [`Self::into_inner`], or dereferencing.
///
/// ```
/// use ratslang::{Length, Time, meter, millisecond};
/// use ratslang::serde::{Quantity, Range};
/// use serde::{Deserialize, Serialize};
/// #[derive(Debug, PartialEq, Serialize, Deserialize)]
/// struct Config {
///     translation: [Quantity<Length>; 3],
///     delays: Vec<Quantity<Time>>,
///     distance: Range<Quantity<Length>>,
/// }
/// let config: Config = ratslang::from_str(
///     "translation = [1m, 2cm, 3mm]\ndelays = [1ms, 2s]\ndistance = 1cm..\n"
/// )?;
/// assert_eq!(config.translation[0].get::<meter>(), 1.0);
/// assert_eq!(config.delays[0].get::<millisecond>(), 1.0);
/// assert_eq!(ratslang::from_str::<Config>(&ratslang::to_string(&config)?)?, config);
/// # Ok::<(), ratslang::serde::Error>(())
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd)]
pub struct Quantity<T>(pub T);
impl<T> Quantity<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}
impl<T> From<T> for Quantity<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}
impl<T> std::ops::Deref for Quantity<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}
macro_rules! quantity {
    ($ty:ty, $adapter:ident) => {
        impl Serialize for Quantity<$ty> {
            fn serialize<S: SerdeSerializer>(
                &self,
                serializer: S,
            ) -> std::result::Result<S::Ok, S::Error> {
                $adapter::serialize(&self.0, serializer)
            }
        }
        impl<'de> Deserialize<'de> for Quantity<$ty> {
            fn deserialize<D: Deserializer<'de>>(
                deserializer: D,
            ) -> std::result::Result<Self, D::Error> {
                $adapter::deserialize(deserializer).map(Self)
            }
        }
    };
}
quantity!(crate::Mass, mass);
quantity!(Length, length);
quantity!(Time, time);
quantity!(crate::Angle, angle);
quantity!(crate::Frequency, frequency);
quantity!(crate::Velocity, velocity);
quantity!(crate::Acceleration, acceleration);

/// ASCII case-insensitive matching for derived unit enums, including `rename`/`alias`.
/// Serialization retains Serde's declared canonical spelling.
///
/// ```
/// use serde::{Deserialize, Serialize};
/// #[derive(Debug, PartialEq, Serialize, Deserialize)]
/// enum Backend {
///     #[serde(alias = "host")]
///     CPU,
///     GPU,
/// }
/// #[derive(Serialize, Deserialize)]
/// struct Config {
///     #[serde(with = "ratslang::serde::case_insensitive")]
///     backend: Backend,
/// }
/// let config: Config = ratslang::from_str("backend = HOST\n")?;
/// assert_eq!(config.backend, Backend::CPU);
/// assert_eq!(ratslang::to_string(&config)?, "backend = \"CPU\"\n");
/// # Ok::<(), ratslang::serde::Error>(())
/// ```
pub mod case_insensitive {
    use super::*;
    pub fn serialize<T: Serialize, S: SerdeSerializer>(
        value: &T,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        value.serialize(serializer)
    }
    pub fn deserialize<'de, T: DeserializeOwned, D: Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<T, D::Error> {
        let name = String::deserialize(deserializer)?;
        T::deserialize(EnumName(name)).map_err(de::Error::custom)
    }
    struct EnumName(String);
    impl<'de> Deserializer<'de> for EnumName {
        type Error = de::value::Error;
        fn deserialize_any<V: Visitor<'de>>(
            self,
            _: V,
        ) -> std::result::Result<V::Value, Self::Error> {
            Err(de::Error::custom("case_insensitive requires a unit enum"))
        }
        fn deserialize_enum<V: Visitor<'de>>(
            self,
            _: &'static str,
            variants: &'static [&'static str],
            visitor: V,
        ) -> std::result::Result<V::Value, Self::Error> {
            let canonical = variants
                .iter()
                .find(|variant| **variant == self.0)
                .or_else(|| {
                    variants
                        .iter()
                        .find(|variant| variant.eq_ignore_ascii_case(&self.0))
                })
                .copied()
                .unwrap_or(&self.0);
            visitor.visit_enum(canonical.to_owned().into_deserializer())
        }
        ::serde::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct
            map struct identifier ignored_any
        }
    }
}

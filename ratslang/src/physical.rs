//! Additional physical dimensions and their literal spellings.
use crate::*;
use uom::si::{
    acceleration::meter_per_second_squared,
    angle::{degree, radian},
    frequency::hertz,
    velocity::meter_per_second,
};

pub(crate) fn parse<'a>(lex: &mut Lexer<'a, Token<'a>>) -> Option<UnitVal> {
    let (value, unit) = split_num_unit(lex.slice())?;
    Some(match unit {
        "gram" | "grams" | "g" => UnitVal::MassVal(Mass::new::<kilogram>(value * 1e-3)),
        "kilogram" | "kilograms" | "kg" => UnitVal::MassVal(Mass::new::<kilogram>(value)),
        "tonne" | "tonnes" | "metric_ton" | "t" => {
            UnitVal::MassVal(Mass::new::<kilogram>(value * 1e3))
        }
        "lb" | "pound" | "pounds" => UnitVal::MassVal(Mass::new::<pound>(value)),
        "oz" | "ounce" | "ounces" => UnitVal::MassVal(Mass::new::<ounce>(value)),
        "yg" => UnitVal::MassVal(Mass::new::<kilogram>(value * 1e-27)),
        "zg" => UnitVal::MassVal(Mass::new::<kilogram>(value * 1e-24)),
        "ag" => UnitVal::MassVal(Mass::new::<kilogram>(value * 1e-21)),
        "fg" => UnitVal::MassVal(Mass::new::<kilogram>(value * 1e-18)),
        "pg" => UnitVal::MassVal(Mass::new::<kilogram>(value * 1e-15)),
        "ng" | "nanogram" | "nanograms" => UnitVal::MassVal(Mass::new::<kilogram>(value * 1e-12)),
        "ug" | "microgram" | "micrograms" => UnitVal::MassVal(Mass::new::<kilogram>(value * 1e-09)),
        "µg" => UnitVal::MassVal(Mass::new::<kilogram>(value * 1e-09)),
        "μg" => UnitVal::MassVal(Mass::new::<kilogram>(value * 1e-09)),
        "mg" | "milligram" | "milligrams" => UnitVal::MassVal(Mass::new::<kilogram>(value * 1e-06)),
        "cg" => UnitVal::MassVal(Mass::new::<kilogram>(value * 1e-05)),
        "dg" => UnitVal::MassVal(Mass::new::<kilogram>(value * 0.0001)),
        "dag" => UnitVal::MassVal(Mass::new::<kilogram>(value * 0.01)),
        "hg" => UnitVal::MassVal(Mass::new::<kilogram>(value * 0.1)),
        "Mg" => UnitVal::MassVal(Mass::new::<kilogram>(value * 1000.0)),
        "Gg" => UnitVal::MassVal(Mass::new::<kilogram>(value * 1000000.0)),
        "Tg" => UnitVal::MassVal(Mass::new::<kilogram>(value * 1000000000.0)),
        "Pg" => UnitVal::MassVal(Mass::new::<kilogram>(value * 1000000000000.0)),
        "Eg" => UnitVal::MassVal(Mass::new::<kilogram>(value * 1000000000000000.0)),
        "Zg" => UnitVal::MassVal(Mass::new::<kilogram>(value * 1e+18)),
        "Yg" => UnitVal::MassVal(Mass::new::<kilogram>(value * 1e+21)),
        "deg" => UnitVal::AngleVal(Angle::new::<degree>(value)),
        "rad" => UnitVal::AngleVal(Angle::new::<radian>(value)),
        "mrad" => UnitVal::AngleVal(Angle::new::<radian>(value * 1e-3)),
        "urad" | "µrad" => UnitVal::AngleVal(Angle::new::<radian>(value * 1e-6)),
        "Hz" => UnitVal::FrequencyVal(Frequency::new::<hertz>(value)),
        "mHz" => UnitVal::FrequencyVal(Frequency::new::<hertz>(value * 1e-3)),
        "kHz" => UnitVal::FrequencyVal(Frequency::new::<hertz>(value * 1e3)),
        "MHz" => UnitVal::FrequencyVal(Frequency::new::<hertz>(value * 1e6)),
        "GHz" => UnitVal::FrequencyVal(Frequency::new::<hertz>(value * 1e9)),
        "m/s" => UnitVal::VelocityVal(Velocity::new::<meter_per_second>(value)),
        "cm/s" => UnitVal::VelocityVal(Velocity::new::<meter_per_second>(value * 1e-2)),
        "mm/s" => UnitVal::VelocityVal(Velocity::new::<meter_per_second>(value * 1e-3)),
        "km/h" => UnitVal::VelocityVal(Velocity::new::<meter_per_second>(value / 3.6)),
        "m/s2" | "m/s^2" => {
            UnitVal::AccelerationVal(Acceleration::new::<meter_per_second_squared>(value))
        }
        "cm/s2" | "cm/s^2" => {
            UnitVal::AccelerationVal(Acceleration::new::<meter_per_second_squared>(value * 1e-2))
        }
        "mm/s2" | "mm/s^2" => {
            UnitVal::AccelerationVal(Acceleration::new::<meter_per_second_squared>(value * 1e-3))
        }
        _ => return None,
    })
}
impl UnitVal {
    /// Value in SI base units, retaining dimension separately via `unit()`.
    pub fn si_value(&self) -> f64 {
        match self {
            Self::MassVal(v) => v.get::<kilogram>(),
            Self::LengthVal(v) => v.get::<meter>(),
            Self::TimeVal(v) => v.get::<second>(),
            Self::AngleVal(v) => v.get::<radian>(),
            Self::FrequencyVal(v) => v.get::<hertz>(),
            Self::VelocityVal(v) => v.get::<meter_per_second>(),
            Self::AccelerationVal(v) => v.get::<meter_per_second_squared>(),
        }
    }
    pub fn as_mass(&self) -> Option<Mass> {
        if let Self::MassVal(value) = self {
            Some(*value)
        } else {
            None
        }
    }
    pub fn as_angle(&self) -> Option<Angle> {
        if let Self::AngleVal(v) = self {
            Some(*v)
        } else {
            None
        }
    }
    pub fn as_frequency(&self) -> Option<Frequency> {
        if let Self::FrequencyVal(v) = self {
            Some(*v)
        } else {
            None
        }
    }
    pub fn as_velocity(&self) -> Option<Velocity> {
        if let Self::VelocityVal(v) = self {
            Some(*v)
        } else {
            None
        }
    }
    pub fn as_acceleration(&self) -> Option<Acceleration> {
        if let Self::AccelerationVal(v) = self {
            Some(*v)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod mass_tests {
    use super::*;
    #[test]
    fn mass_units_are_typed_convertible_and_readable() {
        for (literal, kilograms) in [
            ("2kg", 2.),
            ("500g", 0.5),
            ("250mg", 0.00025),
            ("1ug", 1e-9),
            ("1µg", 1e-9),
            ("1μg", 1e-9),
            ("1ng", 1e-12),
            ("1t", 1000.),
            ("2tonnes", 2000.),
            ("1Mg", 1000.),
            ("1Yg", 1e21),
        ] {
            let config = compile_code(&format!("mass = {literal}\n")).unwrap();
            let Rhs::Val(Val::UnitedVal(mass)) = config.vars.resolve("mass").unwrap().unwrap()
            else {
                panic!()
            };
            assert_eq!(mass.unit(), Unit::Mass);
            assert_eq!(
                mass.as_mass().unwrap().get::<kilogram>(),
                kilograms,
                "{literal}"
            );
            let source = mass.to_source().unwrap();
            let reparsed = compile_code(&format!("mass = {source}\n")).unwrap();
            let Rhs::Val(Val::UnitedVal(value)) = reparsed.vars.resolve("mass").unwrap().unwrap()
            else {
                panic!()
            };
            assert_eq!(value.si_value(), mass.si_value(), "{literal}: {source}");
            if literal == "250mg" {
                assert_eq!(source, "250mg");
            }
        }
        let config = compile_code("lb = 1lb\noz = 1oz\n").unwrap();
        let Rhs::Val(Val::UnitedVal(mass)) = config.vars.resolve("lb").unwrap().unwrap() else {
            panic!()
        };
        assert_eq!(mass.as_mass().unwrap().get::<pound>(), 1.);
        assert!(compile_code("# @type mass\n# @min 500g\n# @max 2kg\nload = 1kg\n").is_ok());
        assert!(
            compile_code_with_state(
                "# @type mass\n# @min 500g\nload = 1m\n",
                std::path::Path::new("."),
                None,
                std::io::sink(),
                false
            )
            .is_err()
        );
    }
}

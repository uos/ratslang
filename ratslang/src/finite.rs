//! Finiteness is a language invariant, independent of optional schema rules.
use crate::*;
impl Val {
    pub fn validate_finite(&self) -> anyhow::Result<()> {
        let finite = match self {
            Self::NumVal(NumVal::Floating(value)) => value.is_finite(),
            Self::UnitedVal(value) => value.si_value().is_finite(),
            _ => true,
        };
        anyhow::ensure!(
            finite,
            "numeric values must be finite; number or unit conversion overflowed"
        );
        Ok(())
    }
}
impl Rhs {
    pub fn validate_finite(&self) -> anyhow::Result<()> {
        match self {
            Self::Val(value) => value.validate_finite(),
            Self::Array(values) => {
                for (index, value) in values.iter().enumerate() {
                    value
                        .validate_finite()
                        .map_err(|error| anyhow!("element [{index}]: {error}"))?;
                }
                Ok(())
            }
            Self::Range { from, to } => {
                for value in from.iter().chain(to.iter()) {
                    value.validate_finite()?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_number_positions_reject_literal_and_unit_conversion_overflow() {
        let overflowing = format!("{}.0", "9".repeat(310));
        let large = format!("1{}.0", "0".repeat(308));
        for expression in [
            overflowing.clone(),
            format!("[[1, {overflowing}]]"),
            format!("{overflowing}.."),
            format!("..{overflowing}"),
            format!("{large}Ym"),
            format!("{large}Ys"),
            format!("{large}kHz"),
            format!("{large}Yg"),
            format!("{overflowing}deg"),
            format!("{overflowing}m/s"),
            format!("{overflowing}m/s2"),
        ] {
            for prefix in ["", "# @finite true\n", "# @finite false\n"] {
                let source = format!("{prefix}value = {expression}\n");
                let errors = syntax_diagnostics(&source);
                assert!(
                    errors.iter().any(|e| e.message.contains("finite")),
                    "{source}: {errors:?}"
                );
                assert!(
                    compile_code_with_state(
                        &source,
                        std::path::Path::new("."),
                        None,
                        std::io::sink(),
                        false
                    )
                    .is_err()
                );
            }
        }
        assert!(compile_code(&format!("value = {large}\n")).is_ok());
    }
    #[test]
    fn nonfinite_initial_state_and_programmatic_values_cannot_bypass_validation() {
        for rhs in [
            Rhs::Val(Val::NumVal(NumVal::Floating(f64::NAN))),
            Rhs::Array(vec![Box::new(Rhs::Val(Val::NumVal(NumVal::Floating(
                f64::INFINITY,
            ))))]),
            Rhs::Range {
                from: None,
                to: Some(Val::UnitedVal(UnitVal::MassVal(Mass::new::<kilogram>(
                    f64::INFINITY,
                )))),
            },
        ] {
            let state = HashMap::from([("bad".parse().unwrap(), rhs.clone())]);
            let result = compile_code_with_state(
                "\n",
                std::path::Path::new("."),
                Some(state),
                std::io::sink(),
                false,
            );
            assert!(result.unwrap_err().to_string().contains("finite"));
            assert!(Constraints::default().validate("bad", &rhs).is_err());
        }
    }
    #[test]
    fn removed_finite_annotations_are_rejected() {
        for annotation in ["finite", "finite true", "finite false", "finite = true"] {
            let error = compile_code_with_state(
                &format!("# @{annotation}\nvalue = 1\n"),
                std::path::Path::new("."),
                None,
                std::io::sink(),
                false,
            )
            .unwrap_err();
            assert!(
                error.to_string().contains("@finite is not supported"),
                "{error}"
            );
        }
    }
}

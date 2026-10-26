use super::*;
use ::serde::{Deserialize, Deserializer, Serializer};
use chumsky::{
    Parser,
    input::{Input, Stream},
};
use logos::Logos;

/// A typed configuration together with its original comments and source text.
///
/// `from_str::<Document<T>>` and `to_string(&document)` preserve an unchanged
/// document byte for byte. Changes are written as trailing override assignments,
/// leaving original comments, annotations, includes, and unknown fields intact.
/// Removing represented fields is rejected because ratslang has no unset syntax.
#[derive(Debug)]
pub struct Document<T> {
    pub value: T,
    source: String,
    baseline: BTreeMap<String, String>,
    annotations: BTreeMap<String, Vec<String>>,
}

// Reuse the existing comment-preserving filter and AST parser to identify
// annotated assignments without evaluating references outside the selection.
fn annotated_assignments(source: &str) -> Result<BTreeMap<String, Vec<String>>> {
    let mut document = crate::Evaluated::new();
    document.source_text = source.into();
    document.annotations = crate::parse_annotations(source);
    let mut result: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut names: Vec<_> = document.annotations.keys().cloned().collect();
    names.sort();
    for name in names {
        let selection = document.to_string_filtered(&name).unwrap();
        let tokens = crate::Token::lexer(&selection)
            .spanned()
            .map(|(token, span)| (token.unwrap_or(crate::Token::Error), span.into()));
        let stream = Stream::from_iter(tokens)
            .map((0..selection.len()).into(), |(token, span): (_, _)| {
                (token, span)
            });
        let locations: crate::provenance::Locations = Default::default();
        let parsed = crate::parser_with_locations(&selection, Some(locations.clone()))
            .parse(stream)
            .into_result()
            .map_err(|_| error(format!("could not parse annotation {name}")))?;
        let statements = crate::do_pass1(
            parsed.into_iter().map(Into::into).collect(),
            &crate::working_dir(),
            &mut locations.borrow().values().cloned(),
            &crate::provenance::Context {
                source: &selection,
                file: None,
                byte_offset: 0,
                line_offset: 0,
                included_from: vec![],
            },
        )
        .map_err(error)?;
        for statement in statements.ast {
            let crate::StatementKindOwned::VariableDef(Statement::AssignLeft { lhs, .. }) =
                statement;
            for var in lhs {
                result
                    .entry(crate::format_var(&var))
                    .or_default()
                    .push(name.clone());
            }
        }
    }
    Ok(result)
}

fn assignments<T: Serialize>(value: &T) -> Result<BTreeMap<String, String>> {
    to_ast(value)?
        .vardefs
        .into_iter()
        .map(|statement| {
            let Statement::AssignLeft { lhs, rhs, .. } = statement;
            Ok((
                crate::format_var(&lhs[0]),
                rhs[0].to_source().map_err(error)?,
            ))
        })
        .collect()
}
impl<T: Serialize + DeserializeOwned> Document<T> {
    /// Parse a typed configuration while retaining the complete original source.
    pub fn parse(source: &str) -> Result<Self> {
        let value = from_str(source)?;
        Self::from_parts(source.to_owned(), value)
    }
}
impl<T: Serialize> Document<T> {
    fn from_parts(source: String, value: T) -> Result<Self> {
        let baseline = assignments(&value)?;
        let annotations = annotated_assignments(&source)?;
        Ok(Self {
            value,
            source,
            baseline,
            annotations,
        })
    }
}
impl<T: Serialize> Document<T> {
    /// The original, unmodified source.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Return the source with changed typed values appended as overrides.
    pub fn to_source(&self) -> Result<String> {
        let current = assignments(&self.value)?;
        if current == self.baseline {
            return Ok(self.source.clone());
        }
        if self.baseline.keys().any(|key| !current.contains_key(key)) {
            return Err(error(
                "cannot remove a field from a preserved document; ratslang has no unset syntax",
            ));
        }
        let mut output = self.source.clone();
        for (key, expression) in current {
            if self.baseline.get(&key) == Some(&expression) {
                continue;
            }
            if !output.ends_with('\n') {
                output.push('\n');
            }
            if !output.ends_with("\n\n") {
                output.push('\n');
            }
            if let Some(annotations) = self.annotations.get(&key) {
                for name in annotations {
                    output.push_str(&format!("# @{name}\n{key} = {expression}\n\n"));
                }
            } else {
                output.push_str(&format!("{key} = {expression}\n"));
            }
        }
        // Verify changes through the compiler, including references and includes.
        source_value(&output)?;
        Ok(output)
    }
}
impl<T: Serialize> Serialize for Document<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer
            .serialize_newtype_struct(DOCUMENT, &self.to_source().map_err(ser::Error::custom)?)
    }
}
impl<'de, T: Serialize + DeserializeOwned> Deserialize<'de> for Document<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct DocumentVisitor<T>(std::marker::PhantomData<T>);
        impl<'de, T: Serialize + DeserializeOwned> Visitor<'de> for DocumentVisitor<T> {
            type Value = Document<T>;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a source-preserving ratslang document")
            }
            fn visit_newtype_struct<D: Deserializer<'de>>(
                self,
                d: D,
            ) -> std::result::Result<Document<T>, D::Error> {
                #[derive(Deserialize)]
                struct Content<T> {
                    source: String,
                    value: T,
                }
                let Content { source, value } = Content::<T>::deserialize(d)?;
                Document::from_parts(source, value).map_err(de::Error::custom)
            }
        }
        deserializer.deserialize_newtype_struct(DOCUMENT, DocumentVisitor(std::marker::PhantomData))
    }
}

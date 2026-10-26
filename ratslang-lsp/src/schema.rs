use crate::{DocumentState, document::comments};
use ratslang::{Constraints, NumVal, Rhs, UnitVal, Val, ValueType};
use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
};
use tower_lsp_server::ls_types::{Location, Uri};

#[derive(Debug, Clone, Default)]
pub struct Schema {
    pub properties: BTreeMap<String, SchemaProperty>,
    pub namespaces: BTreeMap<String, String>,
}
#[derive(Debug, Clone, Default)]
pub struct SchemaProperty {
    pub value_type: Option<String>,
    pub description: Option<String>,
    pub default: Option<String>,
    pub recommended: bool,
    pub constraints: Constraints,
    pub location: Option<Location>,
    pub value: Option<Rhs>,
    pub provenance: Option<ratslang::Explanation>,
}
pub fn type_name(ty: &ValueType) -> String {
    ty.to_string()
}
pub fn value_type(value: &Rhs) -> String {
    match value {
        Rhs::Val(Val::BoolVal(_)) => "boolean".into(),
        Rhs::Val(Val::StringVal(_)) => "string".into(),
        Rhs::Val(Val::NumVal(NumVal::Integer(_))) => "integer".into(),
        Rhs::Val(Val::NumVal(_)) => "float".into(),
        Rhs::Val(Val::UnitedVal(UnitVal::MassVal(_))) => "mass".into(),
        Rhs::Val(Val::UnitedVal(UnitVal::LengthVal(_))) => "length".into(),
        Rhs::Val(Val::UnitedVal(UnitVal::AngleVal(_))) => "angle".into(),
        Rhs::Val(Val::UnitedVal(UnitVal::FrequencyVal(_))) => "frequency".into(),
        Rhs::Val(Val::UnitedVal(UnitVal::VelocityVal(_))) => "velocity".into(),
        Rhs::Val(Val::UnitedVal(UnitVal::AccelerationVal(_))) => "acceleration".into(),
        Rhs::Val(Val::UnitedVal(UnitVal::TimeVal(_))) => "time".into(),
        Rhs::Path(_) => "path".into(),
        Rhs::Array(values) => format!(
            "array<{}>",
            values
                .first()
                .map(|v| value_type(v))
                .unwrap_or("unknown".into())
        ),
        Rhs::Range { from, to } => format!(
            "range<{}>",
            from.as_ref()
                .or(to.as_ref())
                .map(|v| value_type(&Rhs::Val(v.clone())))
                .unwrap_or("unknown".into())
        ),
        Rhs::EmptyRange => "range".into(),
        Rhs::Var(_) => "reference".into(),
    }
}
pub fn inferred_type(value: &Rhs) -> Option<ValueType> {
    Some(match value {
        Rhs::Val(Val::BoolVal(_)) => ValueType::Boolean,
        Rhs::Val(Val::StringVal(_)) => ValueType::String,
        Rhs::Val(Val::NumVal(_)) => ValueType::Number,
        Rhs::Val(Val::UnitedVal(UnitVal::MassVal(_))) => ValueType::Mass,
        Rhs::Val(Val::UnitedVal(UnitVal::LengthVal(_))) => ValueType::Length,
        Rhs::Val(Val::UnitedVal(UnitVal::AngleVal(_))) => ValueType::Angle,
        Rhs::Val(Val::UnitedVal(UnitVal::FrequencyVal(_))) => ValueType::Frequency,
        Rhs::Val(Val::UnitedVal(UnitVal::VelocityVal(_))) => ValueType::Velocity,
        Rhs::Val(Val::UnitedVal(UnitVal::AccelerationVal(_))) => ValueType::Acceleration,
        Rhs::Val(Val::UnitedVal(UnitVal::TimeVal(_))) => ValueType::Time,
        Rhs::Path(_) => ValueType::Path,
        Rhs::Array(values) => ValueType::Array(Box::new(inferred_type(values.first()?)?)),
        Rhs::Range { from, to } => {
            let bound = from.as_ref().or(to.as_ref())?;
            let inner = match bound {
                Val::NumVal(NumVal::Integer(_)) => ValueType::Integer,
                Val::NumVal(NumVal::Floating(_)) => ValueType::Float,
                _ => inferred_type(&Rhs::Val(bound.clone()))?,
            };
            ValueType::Range(Box::new(inner))
        }
        _ => return None,
    })
}
impl SchemaProperty {
    pub fn documentation(&self) -> String {
        let mut lines = Vec::new();
        if let Some(description) = &self.description {
            lines.push(description.clone());
        }
        if let Some(ty) = &self.value_type {
            lines.push(format!("**Type:** `{ty}`"));
        }
        if let Some(options) = &self.constraints.allowed {
            lines.push(format!(
                "**Allowed:** {}",
                options
                    .iter()
                    .map(|o| format!("`{o}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        for (bound, label, exclusive) in [
            (
                &self.constraints.minimum,
                "Minimum",
                self.constraints.minimum_exclusive,
            ),
            (
                &self.constraints.maximum,
                "Maximum",
                self.constraints.maximum_exclusive,
            ),
        ] {
            if let Some(bound) = bound
                && let Ok(value) = bound.to_source()
            {
                lines.push(format!(
                    "**{label}:** `{value}`{}",
                    if exclusive { " (exclusive)" } else { "" }
                ));
            }
        }
        if let Some(default) = &self.default {
            lines.push(format!("**Default:** `{default}`"));
        }
        if self.constraints.required == Some(true) {
            lines.push("**Required:** explicitly assign this setting in your overrides.".into());
        }
        if let Some(message) = &self.constraints.deprecated {
            lines.push(format!("**Deprecated:** {message}"));
        }
        if let Some(target) = &self.constraints.renamed_to {
            lines.push(format!("**Renamed to:** `{target}`"));
        }
        if self.recommended {
            lines.push("**Recommended**".into());
        }
        if let Some(location) = &self.location {
            lines.push(format!(
                "**Default source:** [{}:{}]({})",
                location.uri.as_str(),
                location.range.start.line + 1,
                location.uri.as_str()
            ));
        }
        lines.join("\n\n")
    }
}
impl Schema {
    pub fn from_ratslang_source(source: &str) -> Self {
        Self::load(source, Path::new("."), None).unwrap_or_default()
    }
    pub fn load(source: &str, parent: &Path, uri: Option<&Uri>) -> Result<Self, String> {
        let uri = uri
            .cloned()
            .unwrap_or_else(|| "untitled:schema.rl".parse().unwrap());
        let doc = DocumentState::new(uri, source.into(), 0);
        let mut schema = Self::default();
        schema.collect(&doc, parent, "", &mut HashSet::new(), 0)?;
        let mut compiled = ratslang::compile_code_with_state(
            &format!("{source}\n"),
            parent,
            None,
            std::io::sink(),
            false,
        )
        .map_err(|e| e.to_string())?;
        compiled.vars = compiled.vars.with_drop_warning(false);
        let constraints = compiled.vars.constraints().map_err(|e| e.to_string())?;
        for name in compiled.vars.usage_vars().0 {
            if name.split('.').any(|part| part.starts_with('_')) {
                continue;
            }
            let prop = schema.properties.entry(name.clone()).or_default();
            prop.provenance = compiled.vars.explain(&name).map_err(|e| e.to_string())?;
            if let Some(rules) = constraints.get(&name) {
                prop.constraints = rules.clone();
            }
            if let Some(value) = compiled.vars.resolve(&name).map_err(|e| e.to_string())? {
                prop.value_type = Some(
                    prop.constraints
                        .value_type
                        .as_ref()
                        .map(type_name)
                        .unwrap_or_else(|| value_type(&value)),
                );
                prop.default = value.to_source().ok();
                prop.value = Some(value);
            }
        }
        Ok(schema)
    }
    fn collect(
        &mut self,
        doc: &DocumentState,
        parent: &Path,
        prefix: &str,
        active: &mut HashSet<PathBuf>,
        depth: usize,
    ) -> Result<(), String> {
        if depth >= 64 {
            return Err("include nesting exceeds 64 files".into());
        }
        let full = |name: &str| {
            if prefix.is_empty() {
                name.into()
            } else {
                format!("{prefix}.{name}")
            }
        };
        for (name, ns) in &doc.symbols.namespaces {
            self.namespaces.insert(full(name), ns.documentation.clone());
        }
        let mut statements: Vec<_> = doc
            .symbols
            .assignments
            .iter()
            .enumerate()
            .map(|(i, a)| (a.range.start, false, i))
            .chain(
                doc.symbols
                    .includes
                    .iter()
                    .enumerate()
                    .map(|(i, a)| (a.range.start, true, i)),
            )
            .collect();
        statements.sort_by_key(|s| s.0);
        for (_, is_include, index) in statements {
            if is_include {
                let include = &doc.symbols.includes[index];
                let path = parent
                    .join(&include.path)
                    .canonicalize()
                    .map_err(|e| format!("include {}: {e}", include.path))?;
                if !active.insert(path.clone()) {
                    return Err(format!("cyclic include: {}", path.display()));
                }
                let source = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
                let uri = Uri::from_file_path(&path).ok_or("invalid include URI")?;
                let included = DocumentState::new(uri, source, 0);
                self.collect(
                    &included,
                    path.parent().unwrap_or(parent),
                    full(&include.namespace).trim_end_matches('.'),
                    active,
                    depth + 1,
                )?;
                active.remove(&path);
            } else {
                let assignment = &doc.symbols.assignments[index];
                if assignment.is_internal {
                    continue;
                }
                let prop = self
                    .properties
                    .entry(full(&assignment.full_path))
                    .or_default();
                if !assignment.documentation.is_empty() {
                    prop.description = Some(assignment.documentation.clone());
                }
                prop.recommended |= assignment.annotations.iter().any(|a| a == "recommended")
                    || doc.symbols.namespaces.iter().any(|(name, ns)| {
                        ns.recommended && assignment.full_path.starts_with(&format!("{name}."))
                    });
                prop.location = Some(Location::new(doc.uri.clone(), assignment.selection_range));
                prop.default = Some(assignment.expression.clone());
                prop.constraints = prop.constraints.overlay(
                    &Constraints::from_annotations(
                        assignment.annotations.iter().map(String::as_str),
                    )
                    .map_err(|e| format!("{}: {e}", assignment.full_path))?,
                );
            }
        }
        Ok(())
    }
    /// Suggest only a unique nearby schema key; unrelated and ambiguous names stay unknown.
    pub fn suggest_setting(&self, name: &str) -> Option<&str> {
        ratslang::tooling::suggest_setting(name, self.properties.keys().map(String::as_str))
    }
    pub fn get_property(&self, path: &str) -> Option<&SchemaProperty> {
        self.properties.get(path)
    }
    pub fn get_children_at_path(&self, path: &str) -> (Vec<String>, Vec<String>) {
        let prefix = if path.is_empty() {
            String::new()
        } else {
            format!("{path}.")
        };
        let mut props = std::collections::BTreeSet::new();
        let mut namespaces = std::collections::BTreeSet::new();
        for name in self.properties.keys().chain(self.namespaces.keys()) {
            if let Some(rest) = name.strip_prefix(&prefix) {
                if let Some((first, _)) = rest.split_once('.') {
                    namespaces.insert(first.to_owned());
                } else if self.properties.contains_key(name) {
                    props.insert(rest.to_owned());
                } else if !rest.is_empty() {
                    namespaces.insert(rest.to_owned());
                }
            }
        }
        (
            props.into_iter().collect(),
            namespaces.into_iter().collect(),
        )
    }
    pub fn get_recommended_properties(&self) -> Vec<(String, String)> {
        self.properties
            .iter()
            .filter(|(_, p)| p.recommended || p.constraints.required == Some(true))
            .filter_map(|(name, p)| p.default.as_ref().map(|v| (name.clone(), v.clone())))
            .collect()
    }
    pub fn local_property(doc: &DocumentState, name: &str) -> Option<SchemaProperty> {
        let assignment = doc
            .symbols
            .assignments
            .iter()
            .rev()
            .find(|a| a.full_path == name)?;
        Some(SchemaProperty {
            constraints: Constraints::from_annotations(
                assignment.annotations.iter().map(String::as_str),
            )
            .ok()?,
            description: Some(assignment.documentation.clone()),
            default: Some(assignment.expression.clone()),
            location: Some(Location::new(doc.uri.clone(), assignment.selection_range)),
            ..Default::default()
        })
    }
    pub fn incomplete_annotations(doc: &DocumentState, byte: usize) -> Constraints {
        let start = doc.content[..byte].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let (annotations, _) = comments(&doc.content, start);
        Constraints::from_annotations(annotations.iter().map(String::as_str)).unwrap_or_default()
    }
}

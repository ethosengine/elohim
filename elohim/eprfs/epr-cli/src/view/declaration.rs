//! A bounded data-only selection language. No executable expressions or repository defaults.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

use super::{invalid, Result};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Declaration {
    pub version: u32,
    pub purpose: String,
    pub sources: BTreeMap<String, Source>,
    pub subjects: String,
    pub identity: String,
    pub select: Selection,
    pub exclude: Option<Selection>,
    pub fields: Vec<String>,
    #[serde(default)]
    pub relationships: BTreeMap<String, Relationship>,
    #[serde(default)]
    pub readings: BTreeMap<String, Reading>,
    pub bounds: Bounds,
    pub capture: CaptureSelection,
    #[serde(default)]
    pub evidence_roots: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presentation: Option<Presentation>,
}
/// Display bindings only. These cannot define or change native acceptance rules.
#[derive(Debug, Clone, Deserialize, Serialize, ts_rs::TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Presentation {
    pub title_field: String,
    pub status_field: String,
    pub columns: Vec<Column>,
    pub terminal_values: Vec<String>,
    pub readings: Vec<String>,
    pub reconciliation_reading: Option<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Column {
    pub value: String,
    pub label: String,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub path: Locations,
    pub reader: ArtifactReader,
    /// Basename glob. A missing pattern means one exact file.
    pub pattern: Option<String>,
    #[serde(default)]
    pub optional: bool,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Locations {
    One(String),
    Many(Vec<String>),
}
impl Locations {
    pub fn paths(&self) -> Vec<&str> {
        match self {
            Self::One(path) => vec![path],
            Self::Many(paths) => paths.iter().map(String::as_str).collect(),
        }
    }
}
#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactReader {
    Frontmatter,
    TaggedText,
    Json,
    Bytes,
    Flows,
    Actors,
    Labels,
    Habit,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CaptureSelection {
    Selected,
    All,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "op", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Selection {
    Exists { field: String },
    Eq { field: String, value: Value },
    Contains { field: String, value: Value },
    All { rules: Vec<Selection> },
    Any { rules: Vec<Selection> },
}
impl Selection {
    pub fn matches(&self, fields: &Value) -> bool {
        match self {
            Self::Exists { field } => fields.get(field).is_some_and(|v| !v.is_null()),
            Self::Eq { field, value } => fields.get(field) == Some(value),
            Self::Contains { field, value } => fields
                .get(field)
                .and_then(Value::as_array)
                .is_some_and(|v| v.contains(value)),
            Self::All { rules } => rules.iter().all(|r| r.matches(fields)),
            Self::Any { rules } => rules.iter().any(|r| r.matches(fields)),
        }
    }
    fn validate(&self, depth: usize) -> Result<()> {
        if depth > 16 {
            return Err(invalid("selection exceeds depth 16"));
        }
        match self {
            Self::Exists { field } => {
                if field.is_empty() {
                    return Err(invalid("selection needs a field"));
                }
            }
            Self::Eq { field, value } | Self::Contains { field, value } => {
                if field.is_empty() || value.is_array() || value.is_object() || value.is_null() {
                    return Err(invalid("selection needs a field and scalar value"));
                }
            }
            Self::All { rules } | Self::Any { rules } => {
                if rules.is_empty() || rules.len() > 64 {
                    return Err(invalid("all/any needs 1..64 rules"));
                }
                for rule in rules {
                    rule.validate(depth + 1)?;
                }
            }
        }
        Ok(())
    }
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Relationship {
    pub field: String,
    pub source: String,
    /// "$path" uses the existing cite/reference parser; other names match source metadata.
    pub target: String,
    #[serde(default)]
    pub required: bool,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Reading {
    pub reader: NativeReader,
    pub input: String,
    pub relationship: Option<String>,
    pub subject_field: Option<String>,
    pub measure: Option<String>,
    #[serde(default)]
    pub required: bool,
}
#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum NativeReader {
    Reconciliation,
    Notes,
    Measurements,
    Habit,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Bounds {
    pub max_files: usize,
    pub max_bytes: usize,
    pub max_depth: usize,
}
impl Declaration {
    pub fn validate(&self) -> Result<()> {
        if self.version != 1 || self.purpose.trim().is_empty() || self.identity.is_empty() {
            return Err(invalid("declaration needs version 1, purpose and identity"));
        }
        if self.bounds.max_files == 0
            || self.bounds.max_files > 100_000
            || self.bounds.max_bytes == 0
            || self.bounds.max_bytes > 256 * 1024 * 1024
            || self.bounds.max_depth > 32
        {
            return Err(invalid(
                "bounds exceed limits (100000 files, 256 MiB, depth 32)",
            ));
        }
        let source = self
            .sources
            .get(&self.subjects)
            .ok_or_else(|| invalid("missing subjects binding"))?;
        if !matches!(
            source.reader,
            ArtifactReader::Frontmatter | ArtifactReader::Json | ArtifactReader::TaggedText
        ) || source.optional
        {
            return Err(invalid(
                "subjects require a nonoptional frontmatter/json binding",
            ));
        }
        if let Some(p) = &self.presentation {
            let valid_field = |f: &str| f == "$identity" || self.fields.iter().any(|v| v == f);
            let bounded = |v: &str| !v.is_empty() && v.len() <= 256;
            let columns: std::collections::BTreeSet<_> =
                p.columns.iter().map(|c| &c.value).collect();
            if !valid_field(&p.title_field)
                || !valid_field(&p.status_field)
                || p.columns.len() > 64
                || columns.len() != p.columns.len()
                || p.terminal_values.len() > 64
                || p.readings.len() > 64
                || p.columns
                    .iter()
                    .any(|c| !bounded(&c.value) || !bounded(&c.label))
                || p.terminal_values.iter().any(|v| !bounded(v))
                || p.readings.iter().any(|r| !self.readings.contains_key(r))
                || p.reconciliation_reading.as_ref().is_some_and(|r| {
                    self.readings
                        .get(r)
                        .is_none_or(|s| s.reader != NativeReader::Reconciliation)
                })
            {
                return Err(invalid("invalid or unbounded presentation bindings"));
            }
        }
        if self.evidence_roots.len() > 32 {
            return Err(invalid("too many evidence roots"));
        }
        for root in &self.evidence_roots {
            super::capture::relative(root)?;
        }
        self.select.validate(0)?;
        if let Some(exclude) = &self.exclude {
            exclude.validate(0)?;
        }
        for source in self.sources.values() {
            let paths = source.path.paths();
            if paths.is_empty() || paths.len() > self.bounds.max_files {
                return Err(invalid("invalid source path count"));
            }
            if matches!(
                source.reader,
                ArtifactReader::Flows | ArtifactReader::Actors | ArtifactReader::Labels
            ) && (paths.len() != 1 || source.pattern.is_some())
            {
                return Err(invalid("flow, actor and label bindings require one exact file; append order is local to that log"));
            }
            for path in paths {
                super::capture::relative(path)?;
            }
            if let Some(pattern) = &source.pattern {
                if pattern.contains('/') || pattern.contains('\\') {
                    return Err(invalid("pattern must be a basename glob"));
                }
                glob::Pattern::new(pattern).map_err(invalid)?;
            }
        }
        for rel in self.relationships.values() {
            if !self.sources.contains_key(&rel.source) {
                return Err(invalid("unknown relationship source"));
            }
        }
        for reading in self.readings.values() {
            if reading.reader != NativeReader::Measurements
                && (reading.measure.is_some() || reading.subject_field.is_some())
            {
                return Err(invalid(
                    "measure and subject_field apply only to measurements",
                ));
            }
            if let Some(pin) = &reading.measure {
                crate::flow::measures::MeasureRef::parse(
                    pin,
                    std::path::Path::new("view declaration"),
                )
                .map_err(invalid)?;
            }
            let source = self
                .sources
                .get(&reading.input)
                .ok_or_else(|| invalid("unknown reading input"))?;
            let expected = if reading.reader == NativeReader::Habit {
                ArtifactReader::Habit
            } else {
                ArtifactReader::Flows
            };
            if source.reader != expected {
                return Err(invalid("reading input has incompatible reader"));
            }
            if reading.reader != NativeReader::Habit && reading.relationship.is_some() {
                return Err(invalid("only habit readings support a relationship; bind flow readers to exact log inputs"));
            }
            if let Some(rel) = &reading.relationship {
                if self
                    .relationships
                    .get(rel)
                    .is_none_or(|r| r.source != reading.input)
                {
                    return Err(invalid("reading relationship must target its input"));
                }
            }
        }
        for kind in [ArtifactReader::Actors, ArtifactReader::Labels] {
            if self.sources.values().filter(|s| s.reader == kind).count() > 1 {
                return Err(invalid("only one actors or labels binding is supported"));
            }
        }
        Ok(())
    }
}

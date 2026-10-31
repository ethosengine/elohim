//! Native readers over explicitly bound, captured resources. No live store is opened.
use super::{
    declaration::{ArtifactReader, Declaration, NativeReader, Reading},
    invalid, Result,
};
use crate::flow::{self, acceptance::EvidenceResources, FlowError, FlowResult};
use cid::Cid;
use elohim_epr_rea::{ActorRecord, FlowRecord};
use serde_json::{json, Value};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};

pub struct Inputs<'a> {
    pub bytes: &'a BTreeMap<String, Vec<u8>>,
    pub files: &'a BTreeMap<String, Vec<String>>,
    pub flows: BTreeMap<String, Vec<(Cid, FlowRecord)>>,
    pub actors: BTreeMap<Cid, String>,
    pub labels: flow::Labels,
    pub used: RefCell<BTreeSet<String>>,
    habits: BTreeMap<String, flow::registers::HabitEntry>,
    folds: BTreeMap<String, Vec<flow::report::Fold>>,
}
impl<'a> Inputs<'a> {
    pub fn new(
        declaration: &Declaration,
        bytes: &'a BTreeMap<String, Vec<u8>>,
        files: &'a BTreeMap<String, Vec<String>>,
    ) -> Result<Self> {
        let mut input = Self {
            bytes,
            files,
            flows: BTreeMap::new(),
            actors: BTreeMap::new(),
            labels: BTreeMap::new(),
            used: RefCell::new(BTreeSet::new()),
            habits: BTreeMap::new(),
            folds: BTreeMap::new(),
        };
        for (name, binding) in &declaration.sources {
            for path in &files[name] {
                if !matches!(
                    binding.reader,
                    ArtifactReader::Flows
                        | ArtifactReader::Actors
                        | ArtifactReader::Labels
                        | ArtifactReader::Habit
                ) {
                    continue;
                }
                let text = std::str::from_utf8(&bytes[path]).map_err(invalid)?;
                match binding.reader {
                    ArtifactReader::Flows => input
                        .flows
                        .entry(name.clone())
                        .or_default()
                        .extend(elohim_epr_rea::decode_flow_records(text).map_err(invalid)?),
                    ArtifactReader::Actors => {
                        for (cid, record) in
                            elohim_epr_rea::decode_actor_records(text).map_err(invalid)?
                        {
                            if let ActorRecord::Claim(claim) = record {
                                input.actors.insert(cid, claim.claimed.0);
                            }
                        }
                    }
                    ArtifactReader::Habit => {
                        let mut habit =
                            flow::registers::parse_habit(std::path::Path::new(path), text)
                                .map_err(invalid)?;
                        habit.source = Some(path.clone());
                        input.habits.insert(path.clone(), habit);
                    }
                    ArtifactReader::Labels => {
                        let labels: flow::Labels = serde_json::from_str(text).map_err(invalid)?;
                        for (key, value) in labels {
                            if input.labels.insert(key, value).is_some() {
                                return Err(invalid("duplicate label binding"));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        input.folds = input
            .flows
            .iter()
            .map(|(name, records)| (name.clone(), flow::report::folds_from_records(records)))
            .collect();
        Ok(input)
    }
    pub fn read(
        &self,
        spec: &Reading,
        subject: &Value,
        path: &str,
        related: &[String],
    ) -> Result<Value> {
        let paths = &self.files[&spec.input];
        if paths.is_empty() {
            return unavailable(spec, "input binding unavailable");
        }
        if spec.reader == NativeReader::Habit {
            let selected: Vec<_> = paths
                .iter()
                .filter(|p| spec.relationship.is_none() || related.contains(p))
                .collect();
            if selected.is_empty() {
                return unavailable(spec, "no resolved habit relationship");
            }
            let habits: Vec<_> = selected.iter().map(|path| &self.habits[*path]).collect();
            return Ok(
                json!({"state":"available", "reader":"habit-v1", "inputs":selected, "value":habits}),
            );
        }
        let records = self
            .flows
            .get(&spec.input)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let scope = flow::body_cid(&self.read_text(path).map_err(invalid)?);
        let value = match spec.reader {
            NativeReader::Reconciliation => {
                let result = flow::reconciliation::reconcile_with(
                    self,
                    &scope,
                    Some(path),
                    records,
                    &self.labels,
                    json!({"id":"native-acceptance-reconciliation", "version":"acceptance-reconciliation-v1", "source_cid": eprfs_core::BlobCid::compute_raw(concat!(include_str!("../flow/reconciliation.rs"), include_str!("../flow/acceptance.rs")).as_bytes()).to_string()}),
                );
                if result.assertions.is_empty() && result.historical_candidates.is_empty() {
                    return unavailable(
                        spec,
                        "no structured workflow evidence for this source version",
                    );
                }
                serde_json::to_value(result).map_err(invalid)?
            }
            NativeReader::Notes => {
                let notes = flow::read::notes_on(records, &scope, 1000);
                if notes.is_empty() {
                    return unavailable(spec, "no structured notes for this source version");
                }
                serde_json::to_value(notes).map_err(invalid)?
            }
            NativeReader::Measurements => {
                let key = if let Some(field) = &spec.subject_field {
                    subject.get(field).and_then(Value::as_str).ok_or_else(|| {
                        invalid(format!("missing measurement subject field: {field}"))
                    })?
                } else {
                    path
                };
                let folds: Vec<_> = self
                    .folds
                    .get(&spec.input)
                    .into_iter()
                    .flatten()
                    .filter(|f| {
                        f.subject == key
                            && spec
                                .measure
                                .as_ref()
                                .is_none_or(|m| f.measure.to_string() == *m)
                    })
                    .collect();
                if folds.is_empty() {
                    return unavailable(spec, "no structured measurements for this subject");
                }
                serde_json::to_value(folds).map_err(invalid)?
            }
            NativeReader::Habit => unreachable!(),
        };
        Ok(json!({"state":"available", "reader":spec.reader, "inputs":paths, "value":value}))
    }
}
impl EvidenceResources for Inputs<'_> {
    fn read_text(&self, path: &str) -> FlowResult<String> {
        self.used.borrow_mut().insert(path.into());
        self.bytes
            .get(path)
            .and_then(|b| String::from_utf8(b.clone()).ok())
            .ok_or_else(|| {
                FlowError::InvalidArguments(format!("supporting resource not captured: {path}"))
            })
    }
    fn actor_identity(&self, id: &Cid) -> FlowResult<String> {
        self.actors
            .get(id)
            .cloned()
            .ok_or_else(|| FlowError::InvalidArguments(format!("actor claim unavailable: {id}")))
    }
}
fn unavailable(spec: &Reading, reason: &str) -> Result<Value> {
    if spec.required {
        return Err(invalid(format!("required reading unavailable: {reason}")));
    }
    Ok(json!({"state":"unavailable", "reader":spec.reader, "reason":reason}))
}

pub fn fields(reader: &ArtifactReader, bytes: &[u8]) -> Result<Value> {
    match reader {
        ArtifactReader::TaggedText => {
            let text = std::str::from_utf8(bytes).map_err(invalid)?;
            let tags: Vec<_> = text
                .lines()
                .filter(|line| line.trim_start().starts_with('@'))
                .flat_map(str::split_whitespace)
                .filter_map(|word| word.strip_prefix('@'))
                .collect();
            Ok(json!({"tags":tags, "format":"tagged-text"}))
        }
        ArtifactReader::Json => {
            let value: Value = serde_json::from_slice(bytes).map_err(invalid)?;
            if !value.is_object() {
                return Err(invalid("JSON artifact must be an object"));
            }
            Ok(value)
        }
        ArtifactReader::Frontmatter | ArtifactReader::Habit => {
            let text = std::str::from_utf8(bytes).map_err(invalid)?;
            if text.lines().next().is_none_or(|line| line.trim() != "---") {
                return Ok(json!({}));
            }
            let (lines, _) = flow::cites::split_frontmatter(text)
                .ok_or_else(|| invalid("unterminated frontmatter"))?;
            // Validate the supported scalar/list subset with YAML, then use the shared reader.
            let value: Value = serde_yaml::from_str(&lines.join("\n")).map_err(invalid)?;
            if !value.is_object() {
                return Err(invalid("frontmatter must be a mapping"));
            }
            let fm = flow::parse_frontmatter(text);
            let mut out = serde_json::Map::new();
            for (key, value) in value.as_object().unwrap() {
                // Keep typed scalars and folded prose from YAML; shared list parsing owns list syntax.
                if value.is_array() && value.as_array().unwrap().iter().all(Value::is_string) {
                    out.insert(key.clone(), json!(fm.list(key)));
                } else {
                    out.insert(key.clone(), value.clone());
                }
            }
            Ok(Value::Object(out))
        }
        _ => Ok(json!({})),
    }
}

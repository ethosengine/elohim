//! Declared local projections. Rebuildable views and private byte snapshots confer no authority.
mod capture;
pub mod composition;
pub mod declaration;
mod evidence;
mod readings;
mod snapshot;

use declaration::{CaptureSelection, Declaration};
use eprfs_core::BlobCid;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
pub use snapshot::{reopen, save, snapshot_bytes};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    process::ExitCode,
};

pub type Result<T> = std::result::Result<T, String>;
fn invalid(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Subject {
    pub identity: String,
    pub source: String,
    pub version: String,
    pub fields: BTreeMap<String, Value>,
    pub relationships: BTreeMap<String, Vec<Value>>,
    pub readings: BTreeMap<String, Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Projection {
    pub version: u32,
    pub purpose: String,
    pub declaration: String,
    pub declaration_cid: String,
    pub method: String,
    pub reader_versions: BTreeMap<String, String>,
    pub subjects: Vec<Subject>,
    pub sources: BTreeMap<String, String>,
    pub input_versions: BTreeMap<String, String>,
    pub governance: BTreeMap<String, Value>,
    pub diagnostics: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub composition: Option<composition::CompositionReading>,
}
/// Exact source bytes and reading produced in one optimistic transaction.
pub struct Captured {
    pub projection: Projection,
    pub bytes: BTreeMap<String, Vec<u8>>,
    // Transaction-only witnesses; reopening immutable sources never consults a checkout.
    pub(crate) absent: std::collections::BTreeSet<String>,
}

pub fn project(root: &Path, declaration_path: &str) -> Result<Captured> {
    project_checked(root, declaration_path, || {})
}
fn project_checked(
    root: &Path,
    declaration_path: &str,
    before_verify: impl FnOnce(),
) -> Result<Captured> {
    let mut capture = capture::Capture::new(root, 100_000, 256 * 1024 * 1024)?;
    capture.govern(declaration_path)?;
    let raw = capture.read(declaration_path, false)?.unwrap();
    let header: Value = serde_json::from_slice(&raw).map_err(invalid)?;
    if header["version"] == 2 {
        return composition::project(root, declaration_path, capture, raw, before_verify);
    }
    let declaration: Declaration = serde_json::from_value(header).map_err(invalid)?;
    declaration.validate()?;
    capture.set_limits(declaration.bounds.max_files, declaration.bounds.max_bytes)?;
    let files = capture::discover(&capture, &declaration)?;
    let mut metadata = BTreeMap::new();
    let mut diagnostics = Vec::new();
    for (binding, paths) in &files {
        if paths.is_empty() {
            if !declaration.sources[binding].optional {
                return Err(invalid(format!("required binding has no files: {binding}")));
            }
            diagnostics.push(format!("optional binding unavailable: {binding}"));
        }
        for path in paths {
            capture.govern(path)?;
            let bytes = capture.read(path, false)?.unwrap();
            metadata.insert(
                (binding.clone(), path.clone()),
                readings::fields(&declaration.sources[binding].reader, &bytes)
                    .map_err(|e| invalid(format!("{path}: {e}")))?,
            );
        }
    }
    if capture.bytes.len() > declaration.bounds.max_files
        || capture.bytes.values().map(Vec::len).sum::<usize>() > declaration.bounds.max_bytes
    {
        return Err(invalid("capture exceeds declared bounds"));
    }
    evidence::capture(
        &declaration,
        &files,
        &metadata,
        &mut capture,
        &mut diagnostics,
    )?;
    let inputs = readings::Inputs::new(&declaration, &capture.bytes, &files)?;
    let mut subjects = Vec::new();
    let mut ids = BTreeSet::new();
    let mut selected = BTreeSet::from([declaration_path.to_string()]);
    // Supporting bindings are captured whole, so native evidence can only refer to declared bytes.
    for (binding, paths) in &files {
        if binding != &declaration.subjects {
            selected.extend(paths.iter().cloned());
        }
    }
    for path in &files[&declaration.subjects] {
        let fields = &metadata[&(declaration.subjects.clone(), path.clone())];
        if !declaration.select.matches(fields)
            || declaration
                .exclude
                .as_ref()
                .is_some_and(|s| s.matches(fields))
        {
            continue;
        }
        let path_value = Value::String(path.clone());
        let identity = (if declaration.identity == "$path" {
            Some(&path_value)
        } else {
            fields.get(&declaration.identity)
        })
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            invalid(format!(
                "missing subject identity {}: {path}",
                declaration.identity
            ))
        })?
        .to_string();
        if !ids.insert(identity.clone()) {
            return Err(invalid(format!("duplicate subject identity: {identity}")));
        }
        selected.insert(path.clone());
        let mut relationships = BTreeMap::new();
        let mut resolved = BTreeMap::new();
        for (name, relation) in &declaration.relationships {
            let values = match fields.get(&relation.field) {
                Some(Value::Array(items)) => items.clone(),
                Some(value) => vec![value.clone()],
                None => vec![],
            };
            if relation.required && values.is_empty() {
                return Err(invalid(format!(
                    "required relationship {name} missing on {identity}"
                )));
            }
            let mut edges = Vec::new();
            let mut targets = Vec::new();
            for value in values {
                let raw = value
                    .as_str()
                    .ok_or_else(|| invalid("relationship reference must be a string"))?;
                let cite = crate::flow::cites::parse_cite(raw);
                let reference = if relation.target == "$path" {
                    if cite.legacy_path {
                        cite.reference.as_str()
                    } else if !cite.path.is_empty() {
                        cite.path.as_str()
                    } else {
                        raw
                    }
                } else {
                    raw
                };
                let matches: Vec<_> = files[&relation.source]
                    .iter()
                    .filter(|p| {
                        if relation.target == "$path" {
                            p.as_str() == reference
                        } else {
                            metadata[&(relation.source.clone(), (*p).clone())]
                                .get(&relation.target)
                                .and_then(Value::as_str)
                                == Some(reference)
                        }
                    })
                    .cloned()
                    .collect();
                if matches.len() > 1 {
                    return Err(invalid(format!("ambiguous relationship: {name} {raw}")));
                }
                let target = matches.first();
                if target.is_none() {
                    let message = format!("unresolved relationship {identity}/{name}: {raw}");
                    if relation.required {
                        return Err(invalid(message));
                    }
                    diagnostics.push(message);
                }
                edges.push(json!({"reference":raw, "source":target, "version":target.map(|p| BlobCid::compute_raw(&capture.bytes[p]).to_string()), "state":if target.is_some() {"resolved"} else {"unresolved"}}));
                targets.extend(matches);
            }
            selected.extend(targets.iter().cloned());
            resolved.insert(name.clone(), targets);
            relationships.insert(name.clone(), edges);
        }
        let mut native = BTreeMap::new();
        for (name, reading) in &declaration.readings {
            let related = reading
                .relationship
                .as_ref()
                .and_then(|r| resolved.get(r))
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            native.insert(name.clone(), inputs.read(reading, fields, path, related)?);
        }
        subjects.push(Subject {
            identity,
            source: path.clone(),
            version: BlobCid::compute_raw(&capture.bytes[path]).to_string(),
            fields: declaration
                .fields
                .iter()
                .map(|key| (key.clone(), fields.get(key).cloned().unwrap_or(Value::Null)))
                .collect(),
            relationships,
            readings: native,
        });
    }
    selected.extend(capture.governance_sources.iter().cloned());
    selected.extend(inputs.used.borrow().iter().cloned());
    drop(inputs);
    subjects.sort_by(|a, b| a.identity.cmp(&b.identity));
    before_verify();
    capture.verify()?;
    if capture::discover(&capture, &declaration)? != files {
        return Err(invalid("concurrent input membership change"));
    }
    let versions = capture.versions();
    // Governance manifests are always retained along with the declaration and supporting inputs.
    let source_paths: BTreeSet<_> = files.values().flatten().cloned().collect();
    let bytes: BTreeMap<_, _> = capture
        .bytes
        .into_iter()
        .filter(|(p, _)| {
            matches!(declaration.capture, CaptureSelection::All)
                || selected.contains(p)
                || !source_paths.contains(p)
        })
        .collect();
    let projection = Projection {
        version: 1,
        purpose: declaration.purpose,
        declaration: declaration_path.into(),
        declaration_cid: BlobCid::compute_raw(&raw).to_string(),
        method: format!("epr-view-v1/{}", env!("CARGO_PKG_VERSION")),
        subjects,
        reader_versions: [
            (
                "view",
                concat!(
                    include_str!("mod.rs"),
                    include_str!("declaration.rs"),
                    include_str!("capture.rs"),
                    include_str!("readings.rs"),
                    include_str!("evidence.rs"),
                    include_str!("snapshot.rs")
                ),
            ),
            (
                "reconciliation",
                concat!(
                    include_str!("../flow/reconciliation.rs"),
                    include_str!("../flow/acceptance.rs")
                ),
            ),
            ("measurements", include_str!("../flow/report.rs")),
            ("notes", include_str!("../flow/read.rs")),
            ("habit", include_str!("../flow/habit_census.rs")),
            (
                "frontmatter",
                concat!(
                    include_str!("../flow/mod.rs"),
                    include_str!("../flow/cites.rs")
                ),
            ),
        ]
        .into_iter()
        .map(|(name, source)| {
            (
                name.into(),
                BlobCid::compute_raw(source.as_bytes()).to_string(),
            )
        })
        .collect(),
        sources: bytes
            .iter()
            .map(|(p, b)| (p.clone(), BlobCid::compute_raw(b).to_string()))
            .collect(),
        input_versions: versions,
        governance: capture.governance,
        diagnostics,
        composition: None,
    };
    Ok(Captured {
        projection,
        bytes,
        absent: capture.absent,
    })
}

pub fn compare(baseline: &Projection, current: &Projection) -> Value {
    let old: BTreeMap<_, _> = baseline.subjects.iter().map(|s| (&s.identity, s)).collect();
    let now: BTreeMap<_, _> = current.subjects.iter().map(|s| (&s.identity, s)).collect();
    let cohort: Vec<_> = old.keys().map(|id| json!({"identity":id, "baseline":old[id], "current":now.get(id), "state":if now.contains_key(id) {"present"} else {"missing"}})).collect();
    let additions: Vec<_> = now.keys().filter(|id| !old.contains_key(*id)).collect();
    let missing: Vec<_> = old.keys().filter(|id| !now.contains_key(*id)).collect();
    let changed_readings: Vec<_> = old
        .iter()
        .filter_map(|(id, previous)| {
            now.get(id)
                .filter(|next| previous.readings != next.readings)
                .map(|_| *id)
        })
        .collect();
    let changed_fields: Vec<_> = old
        .iter()
        .filter_map(|(id, previous)| {
            now.get(id)
                .filter(|next| previous.fields != next.fields)
                .map(|_| *id)
        })
        .collect();
    json!({"cohort":cohort, "additions":additions, "missing":missing, "changed_readings":changed_readings, "changed_fields":changed_fields})
}

pub fn run(args: &[String]) -> Result<ExitCode> {
    if args.iter().any(|s| s == "--help" || s == "-h") {
        println!("epr view <declaration> --root <repository> [--save <store>] [--baseline <cid> --store <store>] [--json]\nepr view --snapshot <cid> --store <store> [--source <path>] [--json]");
        return Ok(ExitCode::SUCCESS);
    }
    let mut options = BTreeMap::new();
    let mut declaration = None;
    let mut json_output = false;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--json" => json_output = true,
            "--root" | "--save" | "--store" | "--baseline" | "--snapshot" | "--source" => {
                let value = iter
                    .next()
                    .filter(|s| !s.starts_with("--"))
                    .ok_or_else(|| invalid(format!("missing value for {arg}")))?;
                if options.insert(arg.as_str(), value.as_str()).is_some() {
                    return Err(invalid(format!("duplicate option: {arg}")));
                }
            }
            _ if !arg.starts_with('-') && declaration.is_none() => declaration = Some(arg.as_str()),
            _ => return Err(invalid(format!("unsupported argument: {arg}"))),
        }
    }
    if options.contains_key("--source") && !options.contains_key("--snapshot") {
        return Err(invalid("--source requires --snapshot"));
    }
    if options.contains_key("--store")
        && !options.contains_key("--snapshot")
        && !options.contains_key("--baseline")
    {
        return Err(invalid("--store requires --snapshot or --baseline"));
    }
    let mut snapshot_cid = None;
    let mut source_output = None;
    let projection;
    if let Some(cid) = options.get("--snapshot") {
        if declaration.is_some()
            || options.contains_key("--root")
            || options.contains_key("--save")
            || options.contains_key("--baseline")
        {
            return Err(invalid("snapshot reopening cannot read a checkout or save"));
        }
        let store = options
            .get("--store")
            .ok_or_else(|| invalid("--snapshot needs --store"))?;
        let captured = reopen(Path::new(store), cid)?;
        if let Some(path) = options.get("--source") {
            use base64::Engine;
            capture::relative(path)?;
            let bytes = captured
                .bytes
                .get(*path)
                .ok_or_else(|| invalid("source not retained by snapshot"))?;
            let (encoding, contents) = match std::str::from_utf8(bytes) {
                Ok(text) => ("utf-8", text.to_string()),
                Err(_) => (
                    "base64",
                    base64::engine::general_purpose::STANDARD.encode(bytes),
                ),
            };
            source_output = Some(
                json!({"path":path, "cid":captured.projection.sources[*path], "encoding":encoding, "contents":contents}),
            );
        }
        projection = captured.projection;
        snapshot_cid = Some(cid.to_string());
    } else {
        let path = declaration.ok_or_else(|| invalid("declaration path required"))?;
        let root = options
            .get("--root")
            .ok_or_else(|| invalid("--root required"))?;
        let captured = project(Path::new(root), path)?;
        if let Some(store) = options.get("--save") {
            snapshot_cid = Some(save(Path::new(store), &captured)?);
        }
        projection = captured.projection;
    }
    let baseline = if let Some(cid) = options.get("--baseline") {
        let store = options
            .get("--store")
            .ok_or_else(|| invalid("--baseline needs --store"))?;
        Some(compare(
            &reopen(Path::new(store), cid)?.projection,
            &projection,
        ))
    } else {
        None
    };
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(
                &json!({"projection":projection, "snapshot":snapshot_cid, "comparison":baseline, "source":source_output})
            )
            .map_err(invalid)?
        );
    } else if let Some(source) = source_output {
        print!("{}", source["contents"].as_str().unwrap_or_default());
    } else {
        println!(
            "{} — {} subjects, {} captured sources",
            projection.purpose,
            projection.subjects.len(),
            projection.sources.len()
        );
        for subject in &projection.subjects {
            println!(
                "{}  {}",
                subject.identity,
                serde_json::to_string(&subject.fields).map_err(invalid)?
            );
            for (name, reading) in &subject.readings {
                println!("  {name}: {}", reading);
            }
        }
        for diagnostic in &projection.diagnostics {
            println!("! {diagnostic}");
        }
        if let Some(cid) = snapshot_cid {
            println!("Snapshot: {cid}");
        }
        if let Some(comparison) = baseline {
            println!("Comparison: {comparison}");
        }
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    #[test]
    fn inline_lists_share_frontmatter_reader() {
        let fm = crate::flow::parse_frontmatter("---\ntags: [quiet, 'two words', \"a,b\"]\n---\n");
        assert_eq!(fm.list("tags"), &["quiet", "two words", "a,b"]);
    }
    #[test]
    fn concurrent_content_and_membership_changes_refuse() {
        for membership in [false, true] {
            let root = tempfile::TempDir::new().unwrap();
            std::fs::create_dir(root.path().join("tasks")).unwrap();
            std::fs::write(
                root.path().join("view.json"),
                include_str!("../../tests/fixtures/views/workshop/view.json"),
            )
            .unwrap();
            std::fs::write(
                root.path().join("tasks/table.md"),
                include_str!("../../tests/fixtures/views/workshop/tasks/table.md"),
            )
            .unwrap();
            let result = super::project_checked(root.path(), "view.json", || {
                let path = if membership {
                    "tasks/new.md"
                } else {
                    "tasks/table.md"
                };
                std::fs::write(root.path().join(path), "changed").unwrap();
            });
            assert!(result.err().unwrap().contains("concurrent input"));
        }
    }
}

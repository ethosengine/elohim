//! Follow only explicitly declared native acceptance dependencies inside configured roots.
use super::{
    capture::Capture,
    declaration::{ArtifactReader, Declaration},
    invalid, Result,
};
use crate::flow::{acceptance, body_cid};
use elohim_epr_rea::FlowRecord;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn capture(
    declaration: &Declaration,
    files: &BTreeMap<String, Vec<String>>,
    metadata: &BTreeMap<(String, String), Value>,
    capture: &mut Capture,
    diagnostics: &mut Vec<String>,
) -> Result<()> {
    if declaration.evidence_roots.is_empty() {
        return Ok(());
    }
    let scopes: BTreeSet<_> = files[&declaration.subjects]
        .iter()
        .filter(|path| {
            let fields = &metadata[&(declaration.subjects.clone(), (*path).clone())];
            declaration.select.matches(fields)
                && !declaration
                    .exclude
                    .as_ref()
                    .is_some_and(|s| s.matches(fields))
        })
        .map(|path| {
            std::str::from_utf8(&capture.bytes[path])
                .map(body_cid)
                .map_err(invalid)
        })
        .collect::<Result<_>>()?;
    let mut records = Vec::new();
    for (binding, spec) in &declaration.sources {
        if spec.reader != ArtifactReader::Flows {
            continue;
        }
        for path in &files[binding] {
            records.extend(
                elohim_epr_rea::decode_flow_records(
                    std::str::from_utf8(&capture.bytes[path]).map_err(invalid)?,
                )
                .map_err(invalid)?,
            );
        }
    }
    let intents: BTreeSet<_> = records
        .iter()
        .filter_map(|(id, r)| match r {
            FlowRecord::Intent(i) if scopes.contains(&i.in_scope_of) => Some(*id),
            _ => None,
        })
        .collect();
    let mut commitments = BTreeSet::new();
    let mut paths = BTreeMap::new();
    for (id, r) in &records {
        if let FlowRecord::Commitment(c) = r {
            if scopes.contains(&c.in_scope_of) || c.satisfies.iter().any(|id| intents.contains(id))
            {
                commitments.insert(*id);
                if c.resource_spec
                    .classified_as
                    .first()
                    .is_some_and(|s| s == "a2o:scenario-green")
                {
                    if let Some(path) = c.resource_spec.classified_as.get(1) {
                        paths.insert(path.clone(), false);
                    }
                }
            }
        }
    }
    for (_, r) in &records {
        if let FlowRecord::Event(e) = r {
            if commitments.contains(&e.resource)
                && acceptance::slot(e, "purpose:") == Some("acceptance")
            {
                if let Some(path) = acceptance::slot(e, "report-path:") {
                    paths.insert(path.into(), true);
                }
            }
        }
    }
    for (path, report) in paths {
        let Some(bytes) = read(declaration, capture, diagnostics, &path)? else {
            continue;
        };
        if report {
            match std::str::from_utf8(&bytes)
                .map_err(invalid)
                .and_then(|s| acceptance::report_dependencies(s).map_err(invalid))
            {
                Ok(dependencies) => {
                    for dependency in dependencies {
                        read(declaration, capture, diagnostics, &dependency)?;
                    }
                }
                Err(error) => {
                    diagnostics.push(format!("unreadable acceptance report {path}: {error}"))
                }
            }
        }
    }
    Ok(())
}
fn read(
    declaration: &Declaration,
    capture: &mut Capture,
    diagnostics: &mut Vec<String>,
    path: &str,
) -> Result<Option<Vec<u8>>> {
    super::capture::relative(path)?;
    if !declaration
        .evidence_roots
        .iter()
        .any(|root| path == root || path.starts_with(&format!("{root}/")))
    {
        diagnostics.push(format!("evidence outside declared roots: {path}"));
        return Ok(None);
    }
    capture.govern(path)?;
    let bytes = capture.read(path, true)?;
    if bytes.is_none() {
        diagnostics.push(format!("evidence unavailable: {path}"));
    }
    Ok(bytes)
}

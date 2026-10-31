//! Versioned, data-only composition of declared collections. No workflow acts or authority.
use super::{
    capture::Capture,
    declaration::{Bounds, Declaration, Presentation},
    invalid, Captured, Projection, Result, Subject,
};
use eprfs_core::BlobCid;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Definition {
    version: u32,
    purpose: String,
    groups: Vec<Group>,
    links: Vec<Link>,
    presentation: Presentation,
    bounds: Bounds,
    app_manifest: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Group {
    id: String,
    label: String,
    view: String,
    role: String,
    condition_field: Option<String>,
    claim_prefix: Option<String>,
    title_prefix: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Link {
    from: String,
    field: String,
    to: String,
    target_field: String,
    prefix: Option<String>,
    label: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct GroupReading {
    pub id: String,
    pub label: String,
    pub role: String,
    pub presentation: Presentation,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct WorkLink {
    pub from: String,
    pub to: Option<String>,
    pub reference: String,
    pub label: String,
    pub state: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CompositionReading {
    pub groups: Vec<GroupReading>,
    pub links: Vec<WorkLink>,
    pub authority: String,
    pub views: Vec<String>,
}
fn text_values(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::String(text)) => vec![text.clone()],
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        _ => vec![],
    }
}
fn subject_value(subject: &Subject, field: &str) -> String {
    match field {
        "$identity" => subject
            .fields
            .get("$nativeIdentity")
            .and_then(Value::as_str)
            .unwrap_or(&subject.identity)
            .into(),
        "$path" => subject.source.clone(),
        _ => subject
            .fields
            .get(field)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .into(),
    }
}
fn bounded(text: &str) -> bool {
    !text.is_empty() && text.len() <= 256
}

pub(super) fn project(
    root: &Path,
    path: &str,
    mut capture: Capture,
    raw: Vec<u8>,
    before_verify: impl FnOnce(),
) -> Result<Captured> {
    let definition: Definition = serde_json::from_slice(&raw).map_err(invalid)?;
    if definition.version != 2
        || !bounded(&definition.purpose)
        || definition.groups.is_empty()
        || definition.groups.len() > 16
        || definition.links.len() > 64
    {
        return Err(invalid("unsupported or unbounded work composition"));
    }
    if definition.bounds.max_files == 0
        || definition.bounds.max_files > 100_000
        || definition.bounds.max_bytes == 0
        || definition.bounds.max_bytes > 256 * 1024 * 1024
        || definition.bounds.max_depth > 32
    {
        return Err(invalid("invalid composition bounds"));
    }
    capture.set_limits(definition.bounds.max_files, definition.bounds.max_bytes)?;
    let mut group_ids = BTreeSet::new();
    for group in &definition.groups {
        if !bounded(&group.id)
            || !group
                .id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || !group_ids.insert(group.id.clone())
            || !bounded(&group.label)
            || !matches!(group.role.as_str(), "focus" | "work" | "support")
            || group.claim_prefix.as_ref().is_some_and(|p| !bounded(p))
            || group.title_prefix.as_ref().is_some_and(|p| !bounded(p))
        {
            return Err(invalid("invalid composition group"));
        }
        super::capture::relative(&group.view)?;
    }
    if !definition.groups.iter().any(|g| g.role == "focus")
        || !definition.groups.iter().any(|g| g.role == "work")
    {
        return Err(invalid("composition needs focus and work groups"));
    }
    for link in &definition.links {
        if !group_ids.contains(&link.from)
            || !group_ids.contains(&link.to)
            || !bounded(&link.field)
            || !bounded(&link.target_field)
            || !bounded(&link.label)
            || link.prefix.as_ref().is_some_and(|p| !bounded(p))
        {
            return Err(invalid("invalid composition link"));
        }
    }
    if let Some(manifest) = &definition.app_manifest {
        capture.govern(manifest)?;
        let bytes = capture.read(manifest, false)?.unwrap();
        let manifest: Value = serde_json::from_slice(&bytes).map_err(invalid)?;
        if manifest
            .pointer("/workComposition/$ref")
            .and_then(Value::as_str)
            != Some(path)
        {
            return Err(invalid("app manifest must reference this work composition"));
        }
    }
    let mut subjects = Vec::new();
    let mut groups = Vec::new();
    let mut versions = BTreeMap::new();
    let mut diagnostics = Vec::new();
    let mut membership = Vec::new();
    for group in &definition.groups {
        capture.govern(&group.view)?;
        let bytes = capture.read(&group.view, false)?.unwrap();
        // Child views are v1 only: nested compositions and recursive execution are refused.
        let child: Declaration = serde_json::from_slice(&bytes).map_err(invalid)?;
        child.validate()?;
        if child.bounds.max_files > definition.bounds.max_files
            || child.bounds.max_bytes > definition.bounds.max_bytes
            || child.bounds.max_depth > definition.bounds.max_depth
        {
            return Err(invalid("collection bounds exceed composition bounds"));
        }
        let presentation = child
            .presentation
            .as_ref()
            .ok_or_else(|| invalid("composition collections require presentation"))?
            .clone();
        let files = super::capture::discover(&capture, &child)?;
        let result = super::project(root, &group.view)?;
        capture.absent.extend(result.absent.iter().cloned());
        // Composition selection spans child captures. Retain every consulted input
        // so a later child cannot hide a change to an earlier selection candidate.
        for (source, version) in &result.projection.input_versions {
            capture.govern(source)?;
            let bytes = capture.read(source, false)?.unwrap();
            if BlobCid::compute_raw(&bytes).to_string() != *version {
                return Err(invalid("concurrent composition selection change"));
            }
        }
        for (source, bytes) in &result.bytes {
            capture.govern(source)?;
            if capture.read(source, false)?.as_ref() != Some(bytes) {
                return Err(invalid("concurrent composition input change"));
            }
        }
        diagnostics.extend(result.projection.diagnostics);
        versions.insert(
            format!("{}/declaration", group.id),
            result.projection.declaration_cid.clone(),
        );
        for (name, version) in result.projection.reader_versions {
            versions.insert(format!("{}/{}", group.id, name), version);
        }
        for mut subject in result.projection.subjects {
            let text = std::str::from_utf8(&capture.bytes[&subject.source]).map_err(invalid)?;
            let mut title = subject_value(&subject, &presentation.title_field);
            if let Some(prefix) = &group.title_prefix {
                if let Some(line) = text
                    .lines()
                    .find_map(|line| line.trim_start().strip_prefix(prefix))
                {
                    title = line.trim().into();
                }
            }
            let mut claims = Vec::new();
            if let Some(prefix) = &group.claim_prefix {
                let lines: Vec<_> = text.lines().collect();
                let mut at = 0;
                while at < lines.len() {
                    if !lines[at].trim_start().starts_with(prefix) {
                        at += 1;
                        continue;
                    }
                    let start = at;
                    at += 1;
                    while at < lines.len()
                        && !lines[at].trim().is_empty()
                        && !lines[at].trim_start().starts_with(prefix)
                    {
                        at += 1;
                    }
                    claims.push(json!({"text":lines[start..at].join("\n"), "line":start+1, "source":subject.source, "version":subject.version, "authority":"authored claim"}));
                }
            }
            let conditions = group
                .condition_field
                .as_ref()
                .map(|f| text_values(subject.fields.get(f)))
                .unwrap_or_default();
            let status = subject_value(&subject, &presentation.status_field);
            subject
                .fields
                .insert("$nativeIdentity".into(), json!(subject.identity));
            subject.fields.insert("$group".into(), json!(group.id));
            subject.fields.insert("$title".into(), json!(title));
            subject.fields.insert("$status".into(), json!(status));
            subject.fields.insert("$claims".into(), json!(claims));
            subject
                .fields
                .insert("$conditions".into(), json!(conditions));
            subject.identity = format!("{}:{}", group.id, subject.identity);
            subjects.push(subject);
        }
        groups.push(GroupReading {
            id: group.id.clone(),
            label: group.label.clone(),
            role: group.role.clone(),
            presentation,
        });
        membership.push((child, files));
    }
    let mut links = Vec::new();
    for link in &definition.links {
        for subject in subjects.iter().filter(|s| s.fields["$group"] == link.from) {
            for reference in text_values(subject.fields.get(&link.field)) {
                let key = if let Some(prefix) = &link.prefix {
                    let Some(value) = reference.strip_prefix(prefix) else {
                        continue;
                    };
                    value.to_string()
                } else if link.target_field == "$path" {
                    let cite = crate::flow::cites::parse_cite(&reference);
                    if cite.legacy_path {
                        cite.reference
                    } else if !cite.path.is_empty() {
                        cite.path
                    } else {
                        reference.clone()
                    }
                } else {
                    reference.clone()
                };
                let matches: Vec<_> = subjects
                    .iter()
                    .filter(|s| {
                        s.fields["$group"] == link.to && subject_value(s, &link.target_field) == key
                    })
                    .collect();
                let state = match matches.len() {
                    0 => "unresolved",
                    1 => "resolved",
                    _ => "ambiguous",
                };
                if state != "resolved" {
                    diagnostics.push(format!(
                        "{state} composition link {} / {}: {reference}",
                        subject.identity, link.label
                    ));
                }
                links.push(WorkLink {
                    from: subject.identity.clone(),
                    to: if matches.len() == 1 {
                        Some(matches[0].identity.clone())
                    } else {
                        None
                    },
                    reference,
                    label: link.label.clone(),
                    state: state.into(),
                });
            }
        }
    }
    before_verify();
    capture.verify()?;
    for (child, files) in membership {
        if super::capture::discover(&capture, &child)? != files {
            return Err(invalid("concurrent composition membership change"));
        }
    }
    versions.insert(
        "composition".into(),
        BlobCid::compute_raw(include_bytes!("composition.rs")).to_string(),
    );
    let sources = capture.versions();
    let projection = Projection {
        version: 2,
        purpose: definition.purpose,
        declaration: path.into(),
        declaration_cid: BlobCid::compute_raw(&raw).to_string(),
        method: "epr-work-composition-v2".into(),
        reader_versions: versions,
        subjects,
        sources: sources.clone(),
        input_versions: sources,
        governance: capture.governance,
        diagnostics,
        composition: Some(CompositionReading {
            groups,
            links,
            authority: "Local interpretation; no adopted commons rule asserted".into(),
            views: vec![
                "Progress".into(),
                "Board".into(),
                "Burndown".into(),
                "Sources".into(),
            ],
        }),
    };
    // Presentation stays captured in the root declaration; it never confers acceptance.
    if definition.presentation.title_field != "$title"
        || definition.presentation.status_field != "$status"
    {
        return Err(invalid(
            "composition presentation must use normalized title and status bindings",
        ));
    }
    Ok(Captured {
        projection,
        bytes: capture.bytes,
        absent: capture.absent,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use elohim_epr_rea::{
        Commitment, CommitmentState, FlowRecord, FlowStore, ReaVerb, ResourceSpec, SidecarFlowStore,
    };
    #[test]
    fn evidence_appearing_after_child_capture_refuses_the_composition() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let subject = "---\nid: task\nstatus: open\n---\nTask body\n";
        std::fs::write(root.join("task.md"), subject).unwrap();
        let mut store = SidecarFlowStore::open(root).unwrap();
        store
            .append(FlowRecord::Commitment(Commitment {
                action: ReaVerb::Produce,
                provider: crate::flow::repo_agent(),
                receiver: crate::flow::repo_agent(),
                resource_spec: ResourceSpec {
                    classified_as: vec!["a2o:scenario-green".into(), "missing.feature".into()],
                    quantity: None,
                },
                in_scope_of: crate::flow::body_cid(subject),
                valid_from: None,
                valid_until: None,
                state: CommitmentState::Active,
                satisfies: vec![],
                bound: None,
            }))
            .unwrap();
        let bounds = json!({"max_files":100,"max_bytes":1048576,"max_depth":8});
        let child = json!({"version":1,"purpose":"fixture", "sources":{
            "items":{"path":"task.md","reader":"frontmatter"},
            "records":{"path":".eprfs/status/flows.jsonl","reader":"flows"}},
            "subjects":"items","identity":"id","fields":["status"],
            "select":{"op":"eq","field":"id","value":"task"},
            "relationships":{},"readings":{},"capture":"selected","bounds":bounds,
            "evidence_roots":["missing.feature"],
            "presentation":{"titleField":"$identity","statusField":"status","columns":[],"terminalValues":[],"readings":[]}});
        std::fs::write(root.join("child.json"), child.to_string()).unwrap();
        let composition = json!({"version":2,"purpose":"fixture","bounds":bounds,"links":[],
            "groups":[{"id":"goals","label":"Goals","role":"focus","view":"child.json"},
                {"id":"work","label":"Work","role":"work","view":"child.json"}],
            "presentation":{"titleField":"$title","statusField":"$status","columns":[],"terminalValues":[],"readings":[]}});
        std::fs::write(root.join("composition.json"), composition.to_string()).unwrap();
        let result = super::super::project_checked(root, "composition.json", || {
            std::fs::write(root.join("missing.feature"), "New evidence").unwrap();
        });
        assert!(
            matches!(result, Err(error) if error.contains("concurrent input appeared: missing.feature"))
        );
    }
}

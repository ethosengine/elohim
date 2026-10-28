//! Harness event normalization. Patch reconstruction never writes or executes proposed code.
use super::declarations::{local_path, read, MAX_BYTES};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Change {
    pub operation: String,
    pub path: String,
    pub prior: Option<String>,
    pub proposed: Option<String>,
    pub from: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Proposal {
    pub repository: String,
    pub session: String,
    pub changes: Vec<Change>,
}

pub fn capabilities(harness: &str) -> Value {
    let events = match harness {
        "claude" => vec!["Write", "Edit"],
        "codex" => vec!["apply_patch"],
        _ => vec![],
    };
    json!({"harness": harness, "adapter": if events.is_empty() {"deferred"} else {"implemented"},
        "supportedEvents": events, "unsupported": ["arbitrary shell writes", "other tool paths", "binary files"],
        "registration": "inspect harness configuration separately", "trust": "runtime review required; not inferred from files",
        "deadlineMs": 2000, "maxCandidates": 3, "packetBytes": 8192})
}

pub fn capabilities_at(root: &Path, harness: &str) -> Value {
    let mut report = capabilities(harness);
    report["nativeEvaluatorAvailable"] = json!(true);
    let (config, wrapper) = match harness {
        "claude" => (
            ".claude/settings.json",
            ".claude/hooks/native-seam-guidance.py",
        ),
        "codex" => (".codex/hooks.json", ".codex/hooks/native-seam-guidance.py"),
        _ => {
            report["artifactPresent"] = json!(false);
            report["registered"] = json!(false);
            return report;
        }
    };
    report["artifactPresent"] = json!(root.join(wrapper).is_file());
    let config = read(root, config)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok());
    report["registered"] = json!(config
        .as_ref()
        .and_then(|c| c["hooks"]["PreToolUse"].as_array())
        .is_some_and(|groups| groups
            .iter()
            .any(|g| g["hooks"].as_array().is_some_and(|hs| hs
                .iter()
                .any(|h| h["command"].as_str().is_some_and(|c| c.contains(wrapper)))))));
    report["trust"] = json!("unknown: runtime-owned trust review; not inferred from registration");
    report
}

fn relative(root: &Path, cwd: &Path, path: &str) -> Result<String, String> {
    let path = Path::new(path);
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };
    let p = absolute
        .strip_prefix(root)
        .map_err(|_| "path outside repository")?;
    let rel = p.to_str().ok_or("path is not UTF-8")?.to_owned();
    local_path(root, &rel)?;
    Ok(rel)
}

fn prior(root: &Path, path: &str) -> Result<Option<String>, String> {
    let p = local_path(root, path)?;
    if p.exists() {
        read(root, path).map(Some)
    } else {
        Ok(None)
    }
}

fn field<'a>(value: &'a Value, name: &str) -> Result<&'a str, String> {
    value[name]
        .as_str()
        .ok_or_else(|| format!("missing string {name}"))
}

pub fn normalize(root: &Path, harness: &str, event: &Value) -> Result<Proposal, String> {
    let session = field(event, "session_id")?.to_owned();
    let cwd = event["cwd"].as_str().map(Path::new).unwrap_or(root);
    if !cwd.is_absolute() || !cwd.starts_with(root) {
        return Err("event cwd outside repository".into());
    }
    let input = &event["tool_input"];
    let tool = field(event, "tool_name")?;
    let changes = match (harness, tool) {
        ("claude", "Write" | "Edit") => {
            let path = relative(root, cwd, field(input, "file_path")?)?;
            let old = prior(root, &path)?;
            let proposed = if tool == "Write" {
                field(input, "content")?.to_owned()
            } else {
                let old = old.as_deref().ok_or("Edit target absent")?;
                let needle = field(input, "old_string")?;
                let replacement = field(input, "new_string")?;
                let count = old.matches(needle).count();
                if needle.is_empty() || count == 0 || (count != 1 && input["replace_all"] != true) {
                    return Err("Edit old_string is absent, empty or ambiguous".into());
                }
                if input["replace_all"] == true {
                    old.replace(needle, replacement)
                } else {
                    old.replacen(needle, replacement, 1)
                }
            };
            vec![Change {
                operation: if old.is_some() { "edit" } else { "create" }.into(),
                path,
                prior: old,
                proposed: Some(proposed),
                from: None,
            }]
        }
        ("codex", "apply_patch") => patch(root, cwd, field(input, "command")?)?,
        _ => {
            return Err(format!(
                "incomplete coverage: {harness}/{tool} invocation adapter unsupported"
            ))
        }
    };
    let proposal = Proposal {
        repository: root
            .canonicalize()
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .into(),
        session,
        changes,
    };
    validate(root, &proposal)?;
    Ok(proposal)
}

pub fn validate(root: &Path, proposal: &Proposal) -> Result<(), String> {
    if proposal.session.is_empty() || proposal.changes.is_empty() || proposal.changes.len() > 64 {
        return Err("proposal requires session and 1..64 changes".into());
    }
    if Path::new(&proposal.repository)
        .canonicalize()
        .map_err(|e| e.to_string())?
        != root.canonicalize().map_err(|e| e.to_string())?
    {
        return Err("proposal repository differs".into());
    }
    let mut paths = std::collections::HashSet::new();
    let mut size = 0;
    for change in &proposal.changes {
        local_path(root, &change.path)?;
        if !paths.insert(&change.path) {
            return Err("overlapping multi-file operations".into());
        }
        if let Some(from) = &change.from {
            local_path(root, from)?;
            if !paths.insert(from) {
                return Err("overlapping rename".into());
            }
        }
        let actual = prior(root, change.from.as_deref().unwrap_or(&change.path))?;
        if actual != change.prior {
            return Err(format!("stale prior content: {}", change.path));
        }
        let valid = match change.operation.as_str() {
            "create" => {
                change.prior.is_none() && change.proposed.is_some() && change.from.is_none()
            }
            "edit" => change.prior.is_some() && change.proposed.is_some() && change.from.is_none(),
            "delete" => {
                change.prior.is_some() && change.proposed.is_none() && change.from.is_none()
            }
            "rename" => {
                change.prior.is_some()
                    && change.proposed.is_some()
                    && change.from.is_some()
                    && prior(root, &change.path)?.is_none()
            }
            _ => false,
        };
        if !valid {
            return Err(format!("incomplete {} operation", change.operation));
        }
        size += change.prior.as_ref().map_or(0, String::len)
            + change.proposed.as_ref().map_or(0, String::len);
    }
    if size > MAX_BYTES {
        return Err("proposal exceeds 512 KiB".into());
    }
    Ok(())
}

fn patch(root: &Path, cwd: &Path, patch: &str) -> Result<Vec<Change>, String> {
    if patch.len() > MAX_BYTES {
        return Err("patch exceeds 512 KiB".into());
    }
    let lines: Vec<&str> = patch.lines().collect();
    if lines.first() != Some(&"*** Begin Patch") || lines.last() != Some(&"*** End Patch") {
        return Err("malformed patch envelope".into());
    }
    let mut i = 1;
    let mut changes = Vec::new();
    while i + 1 < lines.len() {
        let header = lines[i];
        i += 1;
        if let Some(path) = header.strip_prefix("*** Add File: ") {
            let path = relative(root, cwd, path)?;
            if prior(root, &path)?.is_some() {
                return Err("Add target already exists".into());
            }
            let mut content = String::new();
            while i < lines.len() && lines[i].starts_with('+') {
                content.push_str(&lines[i][1..]);
                content.push('\n');
                i += 1;
            }
            changes.push(Change {
                operation: "create".into(),
                path,
                prior: None,
                proposed: Some(content),
                from: None,
            });
        } else if let Some(path) = header.strip_prefix("*** Delete File: ") {
            let path = relative(root, cwd, path)?;
            let old = read(root, &path)?;
            changes.push(Change {
                operation: "delete".into(),
                path,
                prior: Some(old),
                proposed: None,
                from: None,
            });
        } else if let Some(path) = header.strip_prefix("*** Update File: ") {
            let source = relative(root, cwd, path)?;
            let old = read(root, &source)?;
            let mut path = source.clone();
            let mut from = None;
            if let Some(target) = lines.get(i).and_then(|l| l.strip_prefix("*** Move to: ")) {
                path = relative(root, cwd, target)?;
                from = Some(source);
                i += 1;
            }
            let mut content: Vec<String> = old.lines().map(String::from).collect();
            let mut cursor = 0;
            let mut hunks = 0;
            while i < lines.len()
                && (lines[i].starts_with("@@") || lines[i].starts_with([' ', '+', '-']))
            {
                if let Some(anchor) = lines[i].strip_prefix("@@ ") {
                    let hits: Vec<_> = content
                        .iter()
                        .enumerate()
                        .skip(cursor)
                        .filter(|(_, l)| l.as_str() == anchor)
                        .map(|(n, _)| n)
                        .collect();
                    if hits.len() != 1 {
                        return Err("patch jump context absent or ambiguous".into());
                    }
                    cursor = hits[0] + 1;
                    i += 1;
                } else if lines[i] == "@@" {
                    i += 1;
                }
                let mut before = Vec::new();
                let mut after = Vec::new();
                while i < lines.len() {
                    let line = lines[i];
                    match line.as_bytes().first() {
                        Some(b' ') => {
                            before.push(line[1..].to_owned());
                            after.push(line[1..].to_owned());
                        }
                        Some(b'-') => before.push(line[1..].to_owned()),
                        Some(b'+') => after.push(line[1..].to_owned()),
                        _ => break,
                    }
                    i += 1;
                }
                let eof = lines.get(i) == Some(&"*** End of File");
                if eof {
                    i += 1;
                }
                if before.is_empty() {
                    return Err("patch hunk lacks exact context".into());
                }
                let hits: Vec<_> = (cursor..=content.len().saturating_sub(before.len()))
                    .filter(|n| {
                        content.get(*n..*n + before.len()) == Some(before.as_slice())
                            && (!eof || *n + before.len() == content.len())
                    })
                    .collect();
                if hits.len() != 1 {
                    return Err("patch context absent or ambiguous".into());
                }
                let at = hits[0];
                cursor = at + after.len();
                content.splice(at..at + before.len(), after);
                hunks += 1;
            }
            if hunks == 0 && from.is_none() {
                return Err("empty update".into());
            }
            let proposed = if hunks == 0 {
                old.clone()
            } else if content.is_empty() {
                String::new()
            } else {
                format!("{}\n", content.join("\n"))
            };
            changes.push(Change {
                operation: if from.is_some() { "rename" } else { "edit" }.into(),
                path,
                prior: Some(old),
                proposed: Some(proposed),
                from,
            });
        } else {
            return Err("unsupported patch directive".into());
        }
    }
    Ok(changes)
}

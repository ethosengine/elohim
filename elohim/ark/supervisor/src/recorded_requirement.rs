//! The artifact store owns provenance. Its executable is trusted local configuration.
use std::{env, fs, path::PathBuf, process::Command};

use cid::Cid;
use serde::Deserialize;

use super::{OwnProduce, RecordedRequirement};

/// Resolution gaps never become compatibility permits.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ResolveGap {
    #[error("no recorded requirement")]
    NotRecorded,
    #[error("requirement not held locally")]
    Unavailable,
    #[error("{0}")]
    ReaderGap(String),
    #[error("conflicting recorded requirements (MergeError)")]
    Conflict,
    #[error("epr does not know output {0}")]
    UnknownOutput(Cid),
    #[error("epr has no requirement reader yet")]
    NoReader,
    #[error("malformed requirement record: {0}")]
    Malformed(String),
}

#[derive(Deserialize)]
struct ReaderReply {
    output: String,
    known: bool,
    requires: Option<String>,
    #[serde(rename = "self")]
    local: bool,
    produce: Option<String>,
    path: Option<PathBuf>,
    unmeasured: Option<String>,
    unavailable: Option<Vec<String>>,
    gap: Option<String>,
    conflict: Option<String>,
}

/// Resolve only through `epr artifact requires`. EPR_BIN overrides PATH; this is
/// the trusted artifact-store reader, not a publisher-controlled command.
/// The temporary directory is private and removed on every return path.
pub fn resolve_recorded(output: Cid) -> Result<RecordedRequirement, ResolveGap> {
    let tmp = tempfile::tempdir().map_err(|e| ResolveGap::Malformed(e.to_string()))?;
    let path = tmp.path().join("requires.json");
    let reader = env::var_os("EPR_BIN").unwrap_or_else(|| "epr".into());
    let result = Command::new(reader)
        .args([
            "artifact",
            "requires",
            "--output",
            &output.to_string(),
            "--write",
        ])
        .arg(&path)
        .arg("--json")
        .output()
        .map_err(|_| ResolveGap::NoReader)?;
    let reply = serde_json::from_slice::<ReaderReply>(&result.stdout);
    if !result.status.success() {
        let diagnostic = format!(
            "{} {}",
            String::from_utf8_lossy(&result.stderr),
            String::from_utf8_lossy(&result.stdout)
        )
        .to_ascii_lowercase();
        // Clap's missing verb and unknown output both exit 2. Only an explicit
        // command/usage diagnostic is classified as an unavailable reader.
        let missing_verb = [
            "unrecognized subcommand",
            "unrecognised subcommand",
            "unknown subcommand",
            "unexpected argument",
            "usage: epr artifact",
        ]
        .iter()
        .any(|marker| diagnostic.contains(marker));
        if missing_verb && reply.is_err() {
            return Err(ResolveGap::NoReader);
        }
        if !matches!(result.status.code(), Some(2..=4)) {
            return Err(ResolveGap::Malformed("artifact reader failed".into()));
        }
    }
    let reply = reply.map_err(|_| ResolveGap::Malformed("invalid reader response".into()))?;
    if reply.output.parse::<Cid>().ok() != Some(output) {
        return Err(ResolveGap::Malformed("output mismatch".into()));
    }
    // Exit status alone is not provenance. Validate the K13 answer before
    // classifying it, including the output identity on non-success answers.
    match result.status.code() {
        Some(2) if !reply.known => return Err(ResolveGap::UnknownOutput(output)),
        Some(3) => {
            if let Some(gap) = reply.gap {
                return Err(ResolveGap::ReaderGap(gap));
            }
            if reply.unavailable.is_some_and(|cids| {
                !cids.is_empty() && cids.iter().all(|cid| cid.parse::<Cid>().is_ok())
            }) {
                return Err(ResolveGap::Unavailable);
            }
        }
        Some(4) if reply.conflict.is_some() => return Err(ResolveGap::Conflict),
        Some(0) if reply.known => {
            if reply.requires.is_none()
                && reply.unmeasured.as_deref() == Some("no recorded requirement")
            {
                return Err(ResolveGap::NotRecorded);
            }
        }
        _ => return Err(ResolveGap::Malformed("inconsistent reader response".into())),
    }
    if !result.status.success() {
        return Err(ResolveGap::Malformed("inconsistent reader response".into()));
    }
    if reply
        .requires
        .as_deref()
        .and_then(|cid| cid.parse::<Cid>().ok())
        .is_none()
        || reply.path.as_ref() != Some(&path)
    {
        return Err(ResolveGap::Malformed(
            "invalid requirement identity or path".into(),
        ));
    }
    let bytes = fs::read(&path).map_err(|_| ResolveGap::Unavailable)?;
    let requires = serde_json::from_slice(&bytes)
        .map_err(|_| ResolveGap::Malformed("invalid requirement document".into()))?;
    let own_produce = if reply.local {
        // A local record with no Produce has no waiver eligibility. A claimed
        // but malformed CID refuses instead of silently weakening the evidence.
        reply
            .produce
            .map(|produce| {
                produce
                    .parse::<Cid>()
                    .map(|produce| OwnProduce {
                        produce,
                        executable: output,
                    })
                    .map_err(|_| ResolveGap::Malformed("invalid local Produce CID".into()))
            })
            .transpose()?
    } else {
        None
    };
    Ok(RecordedRequirement {
        output,
        requires,
        own_produce,
    })
}

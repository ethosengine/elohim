//! The artifact store owns provenance. Its executable is trusted local configuration.
use std::{env, fs, path::PathBuf, process::Command};

use cid::Cid;
use serde::Deserialize;

use super::{OwnProduce, RecordedRequirement};

/// Resolution gaps never become compatibility permits.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ResolveGap {
    #[error("no recorded requirement or document unavailable locally")]
    NotRecorded,
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
    requires: String,
    #[serde(rename = "self")]
    local: bool,
    produce: Option<String>,
    path: PathBuf,
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
        return Err(if missing_verb {
            ResolveGap::NoReader
        } else {
            match result.status.code() {
                Some(3) => ResolveGap::NotRecorded,
                Some(2) => ResolveGap::UnknownOutput(output),
                _ => ResolveGap::Malformed("artifact reader failed".into()),
            }
        });
    }
    let reply: ReaderReply = serde_json::from_slice(&result.stdout)
        .map_err(|_| ResolveGap::Malformed("invalid reader response".into()))?;
    if reply.output.parse::<Cid>().ok() != Some(output) {
        return Err(ResolveGap::Malformed("output mismatch".into()));
    }
    if reply.requires.parse::<Cid>().is_err() || reply.path != path {
        return Err(ResolveGap::Malformed(
            "invalid requirement identity or path".into(),
        ));
    }
    let bytes = fs::read(&path).map_err(|_| ResolveGap::NotRecorded)?;
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

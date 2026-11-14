//! The artifact store owns provenance. Its executable is trusted local configuration.
use std::{
    env, fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use cid::Cid;
use elohim_epr::cid::{compute_bytes_cid, ByteCid};
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

const MAX_DOCUMENT_BYTES: u64 = 1024 * 1024;
const READER_TIMEOUT: Duration = Duration::from_secs(30);

fn bounded_read(path: &Path) -> Result<Vec<u8>, ResolveGap> {
    let file = fs::File::open(path).map_err(|_| ResolveGap::Unavailable)?;
    let mut bytes = Vec::new();
    file.take(MAX_DOCUMENT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ResolveGap::Unavailable)?;
    if bytes.len() as u64 > MAX_DOCUMENT_BYTES {
        return Err(ResolveGap::Malformed("document exceeds 1 MiB".into()));
    }
    Ok(bytes)
}

/// Resolve only through `epr artifact requires`. EPR_BIN overrides PATH; this is
/// the trusted artifact-store reader, not a publisher-controlled command.
/// The temporary directory is private and removed on every return path.
pub fn resolve_recorded(output: Cid) -> Result<RecordedRequirement, ResolveGap> {
    let tmp = tempfile::tempdir().map_err(|e| ResolveGap::Malformed(e.to_string()))?;
    let path = tmp.path().join("requires.json");
    let reader = env::var_os("EPR_BIN").unwrap_or_else(|| "epr".into());
    // Files avoid pipe backpressure and descendant-held pipes extending the deadline.
    let stdout_path = tmp.path().join("stdout");
    let stderr_path = tmp.path().join("stderr");
    let capture = |path: &Path| {
        fs::File::create(path)
            .map(Stdio::from)
            .map_err(|e| ResolveGap::ReaderGap(e.to_string()))
    };
    let mut child = Command::new(reader)
        .args([
            "artifact",
            "requires",
            "--output",
            &output.to_string(),
            "--write",
        ])
        .arg(&path)
        .arg("--json")
        .stdin(Stdio::null())
        .stdout(capture(&stdout_path)?)
        .stderr(capture(&stderr_path)?)
        .spawn()
        .map_err(|_| ResolveGap::NoReader)?;
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < READER_TIMEOUT => {
                thread::sleep(Duration::from_millis(10));
            }
            result => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ResolveGap::ReaderGap(match result {
                    Err(error) => format!("artifact reader wait failed: {error}"),
                    _ => "artifact reader timed out after 30 s".into(),
                }));
            }
        }
    };
    let stdout = bounded_read(&stdout_path)?;
    let stderr = bounded_read(&stderr_path)?;
    let reply = serde_json::from_slice::<ReaderReply>(&stdout);
    if !status.success() {
        let diagnostic = format!(
            "{} {}",
            String::from_utf8_lossy(&stderr),
            String::from_utf8_lossy(&stdout)
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
        if !matches!(status.code(), Some(2..=4)) {
            return Err(ResolveGap::ReaderGap("artifact reader failed".into()));
        }
    }
    let reply = reply.map_err(|_| ResolveGap::Malformed("invalid reader response".into()))?;
    if reply.output.parse::<Cid>().ok() != Some(output) {
        return Err(ResolveGap::Malformed("output mismatch".into()));
    }
    // Exit status alone is not provenance. Validate the K13 answer before
    // classifying it, including the output identity on non-success answers.
    match status.code() {
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
    if !status.success() {
        return Err(ResolveGap::Malformed("inconsistent reader response".into()));
    }
    let expected = reply
        .requires
        .as_deref()
        .and_then(|cid| ByteCid::parse(cid).ok())
        .ok_or_else(|| ResolveGap::Malformed("invalid requirement identity or path".into()))?;
    if reply.path.as_ref() != Some(&path) {
        return Err(ResolveGap::Malformed(
            "invalid requirement identity or path".into(),
        ));
    }
    let bytes = bounded_read(&path)?;
    if compute_bytes_cid(&bytes) != expected {
        return Err(ResolveGap::Malformed(
            "requirement CID does not match written bytes".into(),
        ));
    }
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

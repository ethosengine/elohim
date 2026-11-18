//! `inputs diff <key-a> <key-b>`: compare two explicit Produce keys, this device's own or an
//! admitted sibling's, by the method and input documents each key names. Each document is read
//! from this device's payload store (or a `--documents` directory), verified against its own CID,
//! and only then compared; a document not held is named with its cure, never guessed at.

use eprfs_core::{EprfsError, EprfsStorage};
use serde::Deserialize;

use super::classify::{self, Class, InputDelta, MethodDelta};
use super::*;

/// Flags only the two-key form takes, read before the shared artifact options.
#[derive(Debug, Default)]
pub(super) struct PairFlags {
    /// Directories holding documents by CID file name; each is verified before use.
    pub documents: Vec<PathBuf>,
    /// Ask this device's `epr peer serve` for a sibling's missing input document.
    pub fetch: bool,
    /// Compare whatever is held when a document is missing, still exiting 3.
    pub partial: bool,
}

/// One side of the comparison: the key, the Produce record it names, and its documents.
struct Side<'a> {
    name: &'static str,
    key: BlobCid,
    row: &'a peer::SiblingArtifact,
    derivation: DerivationKey,
    own: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Provenance {
    device: String,
    own: bool,
    key: String,
    method: String,
    input: String,
    commit: String,
    manifest: String,
    at: String,
    /// Where each document was read from, or `null` when it is not held.
    method_document: Option<String>,
    input_document: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    a: Provenance,
    b: Provenance,
    identical: bool,
    order_only: bool,
    method: Option<Vec<MethodDelta>>,
    inputs: Option<Vec<InputDelta>>,
    missing: Vec<String>,
}

/// Where one held, verified document came from.
struct Document {
    bytes: Vec<u8>,
    from: String,
}

/// The largest method or input document this comparison reads. Real ones are a few kilobytes;
/// a record naming a larger object (an artifact's bytes, say) is refused before it is compared.
pub(super) const MAX_DOCUMENT_BYTES: u64 = 8 * 1024 * 1024;

fn too_large(what: &str, len: u64) -> ArtifactError {
    ArtifactError::Gap(format!(
        "the document {what} is {len} bytes, over the {MAX_DOCUMENT_BYTES}-byte document cap"
    ))
}

/// This device's payload store, when one exists.
struct LocalStore {
    store: std::sync::Arc<dyn EprfsStorage>,
    root: PathBuf,
    format: crate::payload::PayloadFormat,
}

/// A carried document `dir/<cid>`: opened without following a symlink and without blocking on a
/// FIFO, accepted only as a regular file, and read no further than the cap.
fn carried(dir: &Path, cid: &BlobCid) -> ArtifactResult<Option<Document>> {
    use std::io::Read;
    use std::os::unix::fs::OpenOptionsExt;
    let path = dir.join(cid.to_string());
    let shown = path.display().to_string();
    let file = match std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(&path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) if error.raw_os_error() == Some(libc::ELOOP) => {
            return Err(ArtifactError::Gap(format!(
                "the document {shown} is a symbolic link; a carried document must be a regular \
                 file in the --documents directory"
            )))
        }
        Err(error) => {
            return Err(ArtifactError::Gap(format!(
                "the document {shown} does not open: {error}"
            )))
        }
    };
    let metadata = file
        .metadata()
        .map_err(|e| ArtifactError::Gap(format!("the document {shown} does not stat: {e}")))?;
    if !metadata.is_file() {
        return Err(ArtifactError::Gap(format!(
            "the document {shown} is not a regular file"
        )));
    }
    if metadata.len() > MAX_DOCUMENT_BYTES {
        return Err(too_large(&shown, metadata.len()));
    }
    let mut bytes = Vec::new();
    file.take(MAX_DOCUMENT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| ArtifactError::Gap(format!("the document {shown} does not read: {e}")))?;
    if bytes.len() as u64 > MAX_DOCUMENT_BYTES {
        return Err(too_large(&shown, bytes.len() as u64));
    }
    if !cid.verifies(&bytes) {
        return Err(ArtifactError::Gap(format!(
            "the document {shown} fails its CID {cid}"
        )));
    }
    Ok(Some(Document { bytes, from: shown }))
}

/// The document named by `cid`, verified: this device's payload store first, then each
/// `--documents` directory. `Ok(None)` only when no place holds it; a held copy that does not
/// hash to `cid`, or exceeds the cap, refuses.
fn held(
    local: Option<&LocalStore>,
    documents: &[PathBuf],
    cid: &BlobCid,
    runtime: &tokio::runtime::Runtime,
) -> ArtifactResult<Option<Document>> {
    if let Some(local) = local {
        let shown = format!("{cid} in {}", local.root.display());
        // The directory format's object is `<root>/<cid>`; its size is checked before the read.
        // The storage trait exposes no size for the blob-store format, whose read is capped
        // after it returns, before any comparison.
        if local.format == crate::payload::PayloadFormat::DirectoryV1 {
            if let Ok(metadata) = std::fs::symlink_metadata(local.root.join(cid.to_string())) {
                if metadata.len() > MAX_DOCUMENT_BYTES {
                    return Err(too_large(&shown, metadata.len()));
                }
            }
        }
        match runtime.block_on(local.store.fetch_blob(cid, FetchPolicy::LocalOnly)) {
            Ok(blob) if blob.bytes.len() as u64 > MAX_DOCUMENT_BYTES => {
                return Err(too_large(&shown, blob.bytes.len() as u64));
            }
            Ok(blob) if blob.cid == *cid && cid.verifies(&blob.bytes) => {
                return Ok(Some(Document {
                    bytes: blob.bytes.to_vec(),
                    from: local.root.display().to_string(),
                }));
            }
            Ok(_) => {
                return Err(ArtifactError::Gap(format!(
                    "the document {shown} fails its CID"
                )))
            }
            Err(EprfsError::BlobNotFound(_)) => {}
            Err(error) => {
                return Err(ArtifactError::Gap(format!(
                    "the document {shown} does not read: {error}"
                )))
            }
        }
    }
    for dir in documents {
        if let Some(document) = carried(dir, cid)? {
            return Ok(Some(document));
        }
    }
    Ok(None)
}

fn side<'a>(
    name: &'static str,
    subject: &str,
    found: &'a peer::SiblingArtifacts,
) -> ArtifactResult<Side<'a>> {
    let key = BlobCid::parse(subject).map_err(|e| {
        ArtifactError::InvalidArguments(format!("inputs diff {subject}: not a key CID: {e}"))
    })?;
    let row = newest_record(found.rows.iter().filter(|row| row.key == key.to_string()))?
        .ok_or_else(|| {
            ArtifactError::Gap(format!(
                "no Produce record for key {key} in this device's flow store, its own or folded \
                 from an admitted sibling; a sibling's Produce arrives through the peer fold \
                 (`epr artifact holders --key {key}` lists who recorded it)"
            ))
        })?;
    let derivation = recorded_key(row)?;
    Ok(Side {
        name,
        key,
        own: row.device == found.device,
        row,
        derivation,
    })
}

fn who(side: &Side<'_>) -> String {
    if side.own {
        format!("{} (this device)", side.row.device)
    } else {
        format!("{} (sibling)", side.row.device)
    }
}

/// The refusal for a document no place holds, with the one cure that exists for it today.
fn missing(side: &Side<'_>, kind: &str, cid: &BlobCid, store_dir: &Path) -> String {
    let at = format!(
        "{}'s {kind} document {cid} (key {}, device {}) is not held in {} or any --documents \
         directory",
        side.name,
        side.key,
        who(side),
        store_dir.display()
    );
    let cure = match (side.own, kind) {
        (true, _) => format!(
            "this device recorded the key, so its put stored the document; pass the \
             --payload-store that recorded it, or a --documents DIR holding {cid}"
        ),
        (false, "input") => format!(
            "rerun with --fetch to draw it from {} through this device's epr peer serve (an \
             input document only, CID-verified; it records no Produce or Use), or save \
             `epr artifact inputs show {}` from the producing device as DIR/{cid} and pass \
             --documents DIR",
            side.row.device, side.key
        ),
        (false, _) => format!(
            "no read path draws a method document from a sibling today (its tier serves one only \
             as a member of a record, and no requester asks for it); copy {cid} from the \
             producing device's payload store into a directory and pass --documents DIR"
        ),
    };
    format!("{at}; cure: {cure}")
}

fn decode_method(cid: &BlobCid, bytes: &[u8]) -> ArtifactResult<MethodDescriptor> {
    let value = elohim_epr::cbor::decode(bytes)
        .map_err(|e| ArtifactError::Gap(format!("method document {cid} is not DAG-CBOR: {e}")))?;
    let descriptor = MethodDescriptor::deserialize(value).map_err(|e| {
        ArtifactError::Gap(format!(
            "method document {cid} is not a method descriptor: {e}"
        ))
    })?;
    let canonical = descriptor
        .cid()
        .map_err(|e| ArtifactError::Gap(format!("method document {cid} does not encode: {e}")))?;
    if canonical != *cid {
        return Err(ArtifactError::Gap(format!(
            "method document {cid} decodes to a descriptor whose canonical CID is {canonical}"
        )));
    }
    Ok(descriptor)
}

fn decode_input(cid: &BlobCid, bytes: Vec<u8>) -> ArtifactResult<String> {
    String::from_utf8(bytes)
        .map_err(|e| ArtifactError::Gap(format!("input document {cid} is not UTF-8 lines: {e}")))
}

fn provenance(side: &Side<'_>, method: Option<&Document>, input: Option<&Document>) -> Provenance {
    Provenance {
        device: side.row.device.clone(),
        own: side.own,
        key: side.key.to_string(),
        method: side.derivation.method.to_string(),
        input: side.derivation.input.cid().to_string(),
        commit: side.row.commit.clone(),
        manifest: side.row.manifest.clone(),
        at: side.row.at.clone(),
        method_document: method.map(|doc| doc.from.clone()),
        input_document: input.map(|doc| doc.from.clone()),
    }
}

/// One input line for a terminal: a carriage return shown as `\r`, a missing final newline said.
fn shown(line: &classify::Located) -> String {
    let text = line.line.replace('\r', "\\r");
    if line.terminated {
        text
    } else {
        format!("{text} (no final newline)")
    }
}

fn print_text(report: &Report) {
    for delta in report.method.iter().flatten() {
        println!(
            "method {} {} {}: {}",
            delta.field,
            delta.change.label(),
            delta.class.label(),
            delta.reason
        );
        if let Some(a) = &delta.a {
            println!("  - {a}");
        }
        if let Some(b) = &delta.b {
            println!("  + {b}");
        }
    }
    for delta in report.inputs.iter().flatten() {
        println!(
            "input {} {} {}: {}",
            delta.label,
            delta.change.label(),
            delta.class.label(),
            delta.reason
        );
        if let Some(a) = &delta.a {
            println!("  - [{}] {}", a.position, shown(a));
        }
        if let Some(b) = &delta.b {
            println!("  + [{}] {}", b.position, shown(b));
        }
    }
    let count = |classes: &mut dyn Iterator<Item = Class>| {
        let classes: Vec<Class> = classes.collect();
        let of = |class: Class| classes.iter().filter(|c| **c == class).count();
        format!(
            "{} (byte-affecting {}, key-only {}, unresolved {})",
            classes.len(),
            of(Class::ByteAffecting),
            of(Class::KeyOnly),
            of(Class::Unresolved)
        )
    };
    let method = report.method.as_ref().map_or("not compared".into(), |d| {
        count(&mut d.iter().map(|delta| delta.class))
    });
    let inputs = report.inputs.as_ref().map_or("not compared".into(), |d| {
        count(&mut d.iter().map(|delta| delta.class))
    });
    let shape = if report.identical {
        "; identical"
    } else if report.order_only {
        "; order-only"
    } else {
        ""
    };
    println!("summary: method {method}; inputs {inputs}{shape}");
}

fn runtime() -> ArtifactResult<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?)
}

pub(super) fn diff_keys(
    first: &str,
    second: &str,
    flags: &PairFlags,
    options: &Options,
) -> ArtifactResult<ExitCode> {
    let found = peer::custody_artifacts(&options.repo, options.handle.as_deref())?;
    let a = side("a", first, &found)?;
    let b = side("b", second, &found)?;
    let store_dir = peer::payload_store_dir(&peer::Options {
        root: options.repo.clone(),
        payload_store: options.payload_store.clone(),
        ..peer::Options::default()
    })?;
    // A failed draw is kept with its side: with --partial the rest is still compared.
    let mut fetch_failed: Vec<(&'static str, String)> = Vec::new();
    if flags.fetch {
        for side in [&a, &b] {
            if side.own {
                continue;
            }
            let runtime = runtime()?;
            let local = open_store(&store_dir, &runtime)?;
            let input = side.derivation.input.cid();
            if held(local.as_ref(), &flags.documents, input, &runtime)?.is_none() {
                drop(local);
                if let Err(error) =
                    draw_input_document(options, &side.row.device, side.row, &side.derivation)
                {
                    if !flags.partial {
                        return Err(error);
                    }
                    fetch_failed.push((side.name, error.to_string()));
                }
            }
        }
    }
    let runtime = runtime()?;
    let local = open_store(&store_dir, &runtime)?;
    let mut gaps = Vec::new();
    let mut documents = Vec::new();
    for side in [&a, &b] {
        let mut pair = Vec::new();
        for (kind, cid) in [
            ("method", &side.derivation.method),
            ("input", side.derivation.input.cid()),
        ] {
            let document = held(local.as_ref(), &flags.documents, cid, &runtime)?;
            if document.is_none() {
                let mut gap = missing(side, kind, cid, &store_dir);
                if let Some((_, why)) = fetch_failed
                    .iter()
                    .find(|(name, _)| kind == "input" && *name == side.name)
                {
                    gap.push_str(&format!("; --fetch did not draw it: {why}"));
                }
                gaps.push(gap);
            }
            pair.push(document);
        }
        documents.push(pair);
    }
    drop(local);
    if !gaps.is_empty() && !flags.partial {
        return Err(ArtifactError::Gap(format!(
            "{}\n(--partial compares whatever is held and still exits 3)",
            gaps.join("\n")
        )));
    }
    let [a_docs, b_docs] = [&documents[0], &documents[1]];
    let (a_method, b_method) = (
        a.derivation.method.to_string(),
        b.derivation.method.to_string(),
    );
    let (a_input, b_input) = (
        a.derivation.input.cid().to_string(),
        b.derivation.input.cid().to_string(),
    );
    let method = match (&a_docs[0], &b_docs[0]) {
        (Some(left), Some(right)) => {
            let mut deltas = classify::method_deltas(
                &decode_method(&a.derivation.method, &left.bytes)?,
                &decode_method(&b.derivation.method, &right.bytes)?,
            );
            classify::unexplained_method(&mut deltas, &a_method, &b_method);
            Some(deltas)
        }
        _ => None,
    };
    let inputs = match (&a_docs[1], &b_docs[1]) {
        (Some(left), Some(right)) => {
            let mut deltas = classify::input_deltas(
                &decode_input(a.derivation.input.cid(), left.bytes.clone())?,
                &decode_input(b.derivation.input.cid(), right.bytes.clone())?,
            );
            classify::unexplained_input(&mut deltas, &a_input, &b_input);
            Some(deltas)
        }
        _ => None,
    };
    let changes: Vec<classify::Change> = method
        .iter()
        .flatten()
        .map(|d| d.change)
        .chain(inputs.iter().flatten().map(|d| d.change))
        .collect();
    // Identity is the verified CIDs', never the absence of a reported difference.
    let identical = a_method == b_method && a_input == b_input;
    let report = Report {
        a: provenance(&a, a_docs[0].as_ref(), a_docs[1].as_ref()),
        b: provenance(&b, b_docs[0].as_ref(), b_docs[1].as_ref()),
        identical,
        order_only: !identical
            && !changes.is_empty()
            && changes.iter().all(|c| *c == classify::Change::OrderOnly),
        method,
        inputs,
        missing: gaps,
    };
    for (name, side) in [("a", &report.a), ("b", &report.b)] {
        eprintln!(
            "{name}: device={}{} key={} method={} input={} commit={} manifest={} at={}",
            side.device,
            if side.own { " (this device)" } else { "" },
            side.key,
            side.method,
            side.input,
            side.commit,
            side.manifest,
            side.at
        );
    }
    for gap in &report.missing {
        eprintln!("inputs unavailable: {gap}");
    }
    if options.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_text(&report);
    }
    Ok(ExitCode::from(if !report.missing.is_empty() {
        EXIT_GAP
    } else {
        u8::from(!report.identical)
    }))
}

/// This device's payload store, opened shared, only when one already exists: an absent or empty
/// directory is no store (opening it would select a format and write a marker), so every document
/// then reads as not held here.
fn open_store(dir: &Path, runtime: &tokio::runtime::Runtime) -> ArtifactResult<Option<LocalStore>> {
    match crate::payload::detect(dir).map_err(|e| ArtifactError::Gap(e.to_string()))? {
        crate::payload::Detected::New => return Ok(None),
        crate::payload::Detected::Marked(_) | crate::payload::Detected::UnmarkedDirectory => {}
    }
    runtime.block_on(async {
        crate::payload::open_payload_store(dir, crate::payload::Access::Shared)
            .await
            .map(|opened| {
                Some(LocalStore {
                    store: opened.store,
                    root: opened.root,
                    format: opened.format,
                })
            })
            .map_err(|e| ArtifactError::Gap(e.to_string()))
    })
}

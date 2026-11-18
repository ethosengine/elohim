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

/// The document named by `cid`, verified: this device's payload store first, then each
/// `--documents` directory. `Ok(None)` only when no place holds it; a held copy that does not
/// hash to `cid` refuses.
fn held(
    store: Option<&dyn EprfsStorage>,
    store_dir: &Path,
    documents: &[PathBuf],
    cid: &BlobCid,
    runtime: &tokio::runtime::Runtime,
) -> ArtifactResult<Option<Document>> {
    if let Some(store) = store {
        match runtime.block_on(store.fetch_blob(cid, FetchPolicy::LocalOnly)) {
            Ok(blob) if blob.cid == *cid && cid.verifies(&blob.bytes) => {
                return Ok(Some(Document {
                    bytes: blob.bytes.to_vec(),
                    from: store_dir.display().to_string(),
                }));
            }
            Ok(_) => {
                return Err(ArtifactError::Gap(format!(
                    "the document {cid} in {} fails its CID",
                    store_dir.display()
                )))
            }
            Err(EprfsError::BlobNotFound(_)) => {}
            Err(error) => {
                return Err(ArtifactError::Gap(format!(
                    "the document {cid} in {} does not read: {error}",
                    store_dir.display()
                )))
            }
        }
    }
    for dir in documents {
        let path = dir.join(cid.to_string());
        match std::fs::read(&path) {
            Ok(bytes) if cid.verifies(&bytes) => {
                return Ok(Some(Document {
                    bytes,
                    from: path.display().to_string(),
                }))
            }
            Ok(_) => {
                return Err(ArtifactError::Gap(format!(
                    "the document {} fails its CID {cid}",
                    path.display()
                )))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(ArtifactError::Gap(format!(
                    "the document {} does not read: {error}",
                    path.display()
                )))
            }
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
            println!("  - [{}] {}", a.position, a.line);
        }
        if let Some(b) = &delta.b {
            println!("  + [{}] {}", b.position, b.line);
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
    if flags.fetch {
        for side in [&a, &b] {
            if side.own {
                continue;
            }
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            let store = open_store(&store_dir, &runtime)?;
            let input = side.derivation.input.cid();
            if held(
                store.as_deref(),
                &store_dir,
                &flags.documents,
                input,
                &runtime,
            )?
            .is_none()
            {
                drop(store);
                draw_input_document(options, &side.row.device, side.row, &side.derivation)?;
            }
        }
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let store = open_store(&store_dir, &runtime)?;
    let mut gaps = Vec::new();
    let mut documents = Vec::new();
    for side in [&a, &b] {
        let mut pair = Vec::new();
        for (kind, cid) in [
            ("method", &side.derivation.method),
            ("input", side.derivation.input.cid()),
        ] {
            let document = held(
                store.as_deref(),
                &store_dir,
                &flags.documents,
                cid,
                &runtime,
            )?;
            if document.is_none() {
                gaps.push(missing(side, kind, cid, &store_dir));
            }
            pair.push(document);
        }
        documents.push(pair);
    }
    drop(store);
    if !gaps.is_empty() && !flags.partial {
        return Err(ArtifactError::Gap(format!(
            "{}\n(--partial compares whatever is held and still exits 3)",
            gaps.join("\n")
        )));
    }
    let [a_docs, b_docs] = [&documents[0], &documents[1]];
    let method = match (&a_docs[0], &b_docs[0]) {
        (Some(left), Some(right)) => Some(classify::method_deltas(
            &decode_method(&a.derivation.method, &left.bytes)?,
            &decode_method(&b.derivation.method, &right.bytes)?,
        )),
        _ => None,
    };
    let inputs = match (&a_docs[1], &b_docs[1]) {
        (Some(left), Some(right)) => Some(classify::input_deltas(
            &decode_input(a.derivation.input.cid(), left.bytes.clone())?,
            &decode_input(b.derivation.input.cid(), right.bytes.clone())?,
        )),
        _ => None,
    };
    let changes: Vec<classify::Change> = method
        .iter()
        .flatten()
        .map(|d| d.change)
        .chain(inputs.iter().flatten().map(|d| d.change))
        .collect();
    let report = Report {
        a: provenance(&a, a_docs[0].as_ref(), a_docs[1].as_ref()),
        b: provenance(&b, b_docs[0].as_ref(), b_docs[1].as_ref()),
        identical: changes.is_empty() && gaps.is_empty(),
        order_only: !changes.is_empty()
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

/// This device's payload store, opened shared; `None` when no store exists at `dir` yet, so
/// every document then reads as not held (or from a `--documents` directory).
fn open_store(
    dir: &Path,
    runtime: &tokio::runtime::Runtime,
) -> ArtifactResult<Option<std::sync::Arc<dyn EprfsStorage>>> {
    if !dir.exists() {
        return Ok(None);
    }
    runtime.block_on(async {
        crate::payload::open_payload_store(dir, crate::payload::Access::Shared)
            .await
            .map(|opened| Some(opened.store))
            .map_err(|e| ArtifactError::Gap(e.to_string()))
    })
}

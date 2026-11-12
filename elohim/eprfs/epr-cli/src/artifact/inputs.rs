//! Read and compare the exact documents named by artifact keys, at self reach only.

use std::collections::BTreeMap;

use eprfs_core::FetchPolicy;
use eprfs_peer::RequestedHolder;

use super::*;

pub(super) fn run(args: &[String]) -> ArtifactResult<ExitCode> {
    if matches!(
        args.first().map(String::as_str),
        Some("--help" | "-h" | "help")
    ) {
        println!("{}", usage());
        return Ok(ExitCode::SUCCESS);
    }
    let verb = args.first().map(String::as_str);
    let subject = args
        .get(1)
        .ok_or_else(|| ArtifactError::InvalidArguments(usage()))?;
    let mut device = None;
    let mut rest = Vec::new();
    let mut flags = args[2..].iter();
    while let Some(flag) = flags.next() {
        if flag == "--device" {
            device = Some(
                flags
                    .next()
                    .ok_or_else(|| {
                        ArtifactError::InvalidArguments("--device needs a did:key".into())
                    })?
                    .clone(),
            );
        } else {
            rest.push(flag.clone());
        }
    }
    let options = Options::parse(&rest)?;
    let outcome = match verb {
        Some("show") if device.is_none() => show(subject, &options),
        Some("diff") => diff(
            subject,
            device.as_deref().ok_or_else(|| {
                ArtifactError::InvalidArguments("inputs diff needs --device <did:key>".into())
            })?,
            &options,
        ),
        _ => return Err(ArtifactError::InvalidArguments(usage())),
    };
    match outcome {
        Ok(exit) => Ok(exit),
        Err(error) => {
            eprintln!("inputs unavailable: {error}");
            Ok(ExitCode::from(EXIT_GAP))
        }
    }
}

fn recorded_key(row: &peer::SiblingArtifact) -> ArtifactResult<DerivationKey> {
    let cid = |value: Option<&str>| {
        value
            .ok_or_else(|| ArtifactError::Gap("record has no method/input CID".into()))
            .and_then(custody::parse_cid)
    };
    let key = DerivationKey {
        method: cid(row.method.as_deref())?,
        input: KeyInput::Raw(cid(row.input.as_deref())?),
    };
    if key_cid(&key)
        .map_err(|e| ArtifactError::Gap(e.to_string()))?
        .to_string()
        != row.key
    {
        return Err(ArtifactError::Gap(
            "record's method/input disagree with its key CID".into(),
        ));
    }
    Ok(key)
}

/// A descriptor means this checkout's computed key. A key CID means the local recorded key,
/// so inspecting an old build never silently substitutes today's compiler or checkout.
fn local_key(subject: &str, options: &Options) -> ArtifactResult<DerivationKey> {
    if let Ok(cid) = BlobCid::parse(subject) {
        let found = peer::custody_artifacts(&options.repo, options.handle.as_deref())?;
        let row = found
            .rows
            .iter()
            .rev()
            .find(|row| row.device == found.device && row.key == cid.to_string())
            .ok_or_else(|| ArtifactError::Gap(format!("no local record for key {cid}")))?;
        recorded_key(row)
    } else {
        let descriptor: MethodDescriptor = serde_json::from_slice(&std::fs::read(subject)?)?;
        Ok(clean_key(&options.repo, &descriptor)?.key)
    }
}

fn read_document(options: &Options, input: &BlobCid) -> ArtifactResult<String> {
    let path = peer::payload_store_dir(&peer::Options {
        root: options.repo.clone(),
        payload_store: options.payload_store.clone(),
        ..peer::Options::default()
    })?;
    if !path.exists() {
        return Err(ArtifactError::Gap(format!(
            "no local payload store at {}",
            path.display()
        )));
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let store = crate::payload::open_payload_store(&path, crate::payload::Access::Shared)
            .await
            .map_err(|e| ArtifactError::Gap(e.to_string()))?
            .store;
        let blob = store
            .fetch_blob(input, FetchPolicy::LocalOnly)
            .await
            .map_err(|e| ArtifactError::Gap(format!("input document {input}: {e}")))?;
        if blob.cid != *input || !input.verifies(&blob.bytes) {
            return Err(ArtifactError::Gap(
                "local input document CID mismatch".into(),
            ));
        }
        String::from_utf8(blob.bytes.to_vec())
            .map_err(|e| ArtifactError::Gap(format!("input document is not UTF-8 lines: {e}")))
    })
}

fn show(subject: &str, options: &Options) -> ArtifactResult<ExitCode> {
    let key = local_key(subject, options)?;
    print!("{}", read_document(options, key.input.cid())?);
    Ok(ExitCode::SUCCESS)
}

fn candidate(row: &peer::SiblingArtifact) -> String {
    format!(
        "input={} commit={} time={}",
        row.input.as_deref().unwrap_or("unrecorded"),
        row.commit,
        row.at
    )
}

/// Produce carries occurred_at (exposed as at), not a production-ordered fold position.
fn same_source<'a>(
    rows: &'a [peer::SiblingArtifact],
    device: &str,
    local: &peer::SiblingArtifact,
) -> ArtifactResult<&'a peer::SiblingArtifact> {
    let mut candidates: Vec<_> = rows
        .iter()
        .filter(|row| row.device == device && row.method == local.method)
        .collect();
    candidates.sort_by_key(|row| (&row.input, &row.commit, &row.at));
    let mut matching = Vec::new();
    for row in &candidates {
        if row.commit == local.commit {
            let at = chrono::DateTime::parse_from_rfc3339(&row.at).map_err(|error| {
                ArtifactError::Gap(format!(
                    "invalid Produce occurred_at: {}: {error}",
                    candidate(row)
                ))
            })?;
            matching.push((at, *row));
        }
    }
    let newest = matching.iter().map(|(at, _)| at).max().ok_or_else(|| {
        ArtifactError::Gap(format!(
            "no same-source sibling record for commit {}; pass a literal key; candidates (up to 5):\n{}",
            local.commit,
            candidates.iter().take(5).map(|row| candidate(row)).collect::<Vec<_>>().join("\n")
        ))
    })?;
    let tied: Vec<_> = matching
        .iter()
        .filter(|(at, _)| at == newest)
        .map(|(_, row)| *row)
        .collect();
    let selected = tied[0];
    if tied.iter().any(|row| row.input != selected.input) {
        return Err(ArtifactError::Gap(format!(
            "ambiguous newest same-source Produce records; pass a literal key:\n{}",
            tied.iter()
                .map(|row| candidate(row))
                .collect::<Vec<_>>()
                .join("\n")
        )));
    }
    Ok(selected)
}

fn diff(subject: &str, device: &str, options: &Options) -> ArtifactResult<ExitCode> {
    let key = local_key(subject, options)?;
    let local = read_document(options, key.input.cid())?;
    let exact = BlobCid::parse(subject).ok();
    let found = peer::custody_artifacts(&options.repo, options.handle.as_deref())?;
    if device == found.device || !device.starts_with("did:key:") {
        return Err(ArtifactError::Gap(
            "--device must name an admitted sibling".into(),
        ));
    }
    let local_cid = key_cid(&key)
        .map_err(|e| ArtifactError::Gap(e.to_string()))?
        .to_string();
    let local_row = found
        .rows
        .iter()
        .rev()
        .find(|row| row.device == found.device && row.key == local_cid)
        .ok_or_else(|| ArtifactError::Gap(format!("no local record for key {local_cid}")))?;
    recorded_key(local_row)?;
    // The custody reader filters by roster admission. Descriptor comparisons hold source
    // constant; a literal key retains its exact match, regardless of the recorded commit.
    let row = match exact {
        Some(cid) => found
            .rows
            .iter()
            .rev()
            .find(|row| row.device == device && row.key == cid.to_string())
            .ok_or_else(|| ArtifactError::Gap("no admitted sibling record for this key".into()))?,
        None => same_source(&found.rows, device, local_row)?,
    };
    let remote = recorded_key(row)?;
    let peer_dir = peer::peer_dir()?;
    if ServeLock::held(&peer_dir) != Some(true) {
        return Err(gap(&FetchGap::DaemonDown));
    }
    let request = FetchRequest {
        inputs_only: true,
        nonce: FetchRequest::fresh_nonce(),
        authority: FetchAuthority::Derivation {
            key_cid: key_cid(&remote).map_err(|e| ArtifactError::Gap(e.to_string()))?,
            key: remote.clone(),
        },
        output: custody::parse_cid(&row.output)?,
        expect_output: None,
        holders: vec![RequestedHolder {
            device: device.into(),
            manifest: custody::parse_cid(&row.manifest)?,
        }],
        // Input-only requests do not install a file or record Use. These legacy artifact queue
        // fields are intentionally empty and are never used by the input branch of the daemon.
        dest: PathBuf::new(),
        scope: String::new(),
        transport: FetchTransport::default(),
        stall_s: options.stall.unwrap_or(FETCH_STALL.as_secs()),
        total_s: options.total.unwrap_or(FETCH_TOTAL.as_secs()),
    };
    let queue = fetch_queue_dir(&peer_dir);
    ensure_queue_dir(&queue)?;
    let stem = request.stem();
    let path = fetch_request_path(&queue, &stem);
    let expected = FetchFor::of(&request);
    write_queue_file(&path, &request)?;
    let wait = Duration::from_secs(
        request
            .total_s
            .saturating_add(request.stall_s)
            .saturating_add(30),
    );
    let done = wait_for_answer(&fetch_done_path(&queue, &stem), &expected, wait);
    match done {
        Some(FetchDone::InputDocument { cid, .. }) if cid == *remote.input.cid() => {}
        Some(FetchDone::Gap { gap: why, .. }) => return Err(gap(&why)),
        Some(_) => {
            return Err(ArtifactError::Gap(
                "unexpected input document answer".into(),
            ))
        }
        None => {
            if std::fs::remove_file(&path).is_err() {
                let _ = write_queue_file(&fetch_cancel_path(&queue, &stem), &expected);
            }
            return Err(ArtifactError::Gap(
                "input document fetch timed out and was withdrawn".into(),
            ));
        }
    }
    let sibling = read_document(options, remote.input.cid())?;
    let lines = line_diff(&local, &sibling);
    eprintln!("inputs diff method={}", key.method);
    for (side, record) in [("this device", local_row), ("sibling", row)] {
        eprintln!(
            "{side}: device={} key={} input={} commit={}",
            record.device,
            record.key,
            record.input.as_deref().unwrap_or("unrecorded"),
            record.commit
        );
    }
    print!("{lines}");
    Ok(ExitCode::from(u8::from(!lines.is_empty())))
}

/// Sorted multiset difference: repeated lines remain meaningful and no context is printed.
fn line_diff(local: &str, sibling: &str) -> String {
    let mut counts: BTreeMap<&str, i64> = BTreeMap::new();
    for line in local.lines() {
        *counts.entry(line).or_default() -= 1;
    }
    for line in sibling.lines() {
        *counts.entry(line).or_default() += 1;
    }
    let mut result = String::new();
    for (line, count) in counts {
        for _ in 0..count.unsigned_abs() {
            result.push(if count < 0 { '-' } else { '+' });
            result.push_str(line);
            result.push('\n');
        }
    }
    result
}

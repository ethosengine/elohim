//! Receipts and bounded reads — the only code that touches source bytes on a reader's behalf.
use super::*; // keeps every `use` the functions relied on; tighten in a later pass

// ───────────────────────────────────────────────────────────────────────────────────────────────
// The privacy gate
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// Refuse any attempt to route a private recall record into a governed plane.
///
/// This function exists so the invariant has ONE named home a mutation test can delete. A recall
/// verb that imported, contributed, projected, witnessed or fed back a receipt would move a
/// reasoning trace from the private side of the line to the fruit side, which the canon forbids at
/// constitutional binding level. The check is on the PATH because that is what a caller would
/// actually hand to a memory verb.
pub fn refuse_private_import(root: &Path, candidate: &str) -> FlowResult<()> {
    let normalized = candidate.trim_start_matches("./").replace('\\', "/");
    let under = |dir: &str| {
        normalized == dir
            || normalized.starts_with(&format!("{dir}/"))
            || root.join(&normalized).starts_with(root.join(dir))
    };
    // `.claude/` became a declared source root on 2026-09-11 so the tooling layer is reachable
    // through the entry. That widening admitted ONE subtree it must not: a sibling checkout is
    // another repository's working tree, and reading it here would attribute its bytes and its
    // assertions to this one.
    if FOREIGN_TREES.iter().any(|dir| under(dir)) {
        return Err(refused(format!(
            "`{candidate}` is another checkout's working tree, not this repository's declared              source. Name the path inside this tree that carries the same concern."
        )));
    }
    if [RECALL_DIR_REL, LEGACY_RECEIPTS_REL]
        .iter()
        .any(|dir| under(dir))
    {
        return Err(refused(format!(
            "`{candidate}` is a private recall record: receipts and continuations are never \
             imported, projected, witnessed or targeted by feedback \
             (genesis/docs/architecture/private-thought-governed-fruit.md §2). \
             A recall session exposes what was read and what was concluded, and nothing else."
        )));
    }
    Ok(())
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Containment and bounded reads
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// Resolve `relative` inside the repository AND inside one of the contract's declared source roots.
///
/// Two gates, not one: the repository boundary keeps a `..` out, and the declared-source-root gate
/// is what makes the contract's `source_roots` a real bound rather than documentation.
pub(crate) fn contained(root: &Path, relative: &str, allowed: &[String]) -> FlowResult<PathBuf> {
    if Path::new(relative).is_absolute() {
        return Err(refused("outside declared source scope"));
    }
    let path = confine_under(root, &root.join(relative))
        .map_err(|_| refused("outside declared source scope"))?;
    let inside = allowed.iter().any(|prefix| {
        let base = root.join(prefix);
        // `starts_with` on components, so `genesis/` never admits `genesis-other/`.
        path.starts_with(&base)
    });
    if !inside {
        return Err(refused("outside declared source scope"));
    }
    // The refusal that a widened root cannot lift. `contained` is what every bounded read and
    // every discovery traversal passes through, so the exclusion is stated once, here.
    if FOREIGN_TREES
        .iter()
        .any(|dir| path.starts_with(root.join(dir)))
    {
        return Err(refused(format!(
            "`{relative}` is another checkout's working tree, not this repository's declared              source. Name the path inside this tree that carries the same concern."
        )));
    }
    Ok(path)
}

/// Read at most `max` bytes of one line, stopping at (and including) the newline.
pub(crate) fn read_line_bounded(
    reader: &mut BufReader<File>,
    max: usize,
    out: &mut Vec<u8>,
) -> std::io::Result<()> {
    while out.len() < max {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            break;
        }
        let take = available.len().min(max - out.len());
        if let Some(index) = available[..take].iter().position(|&b| b == b'\n') {
            out.extend_from_slice(&available[..=index]);
            reader.consume(index + 1);
            return Ok(());
        }
        out.extend_from_slice(&available[..take]);
        reader.consume(take);
    }
    Ok(())
}

/// One bounded excerpt with its own fingerprint — the unit of evidence this executor emits.
///
/// The frontier is the point of the function. Every way a read can come up short (scan budget,
/// short file, byte budget, a line the budget would split, a source that changed mid-read, invalid
/// UTF-8) names itself, and when ANY of them fires the excerpt is withheld entirely. A partial
/// excerpt that looked complete would be evidence about bytes nobody read.
pub fn excerpt(root: &Path, contract: &Contract, source: &str, lines: &str) -> FlowResult<Value> {
    let path = contained(root, source, &contract.source_roots())?;
    let (start, end) = parse_range(lines)?;
    let scan_limit = contract.limit_usize("scan_bytes");
    let byte_limit = contract.limit_usize("source_bytes");
    let scan_seconds = contract.limit_secs("scan_seconds");

    let before = path.metadata().map_err(|source| FlowError::Read {
        path: path.clone(),
        source,
    })?;
    let began = Instant::now();
    let mut scanned = 0usize;
    let mut selected: Vec<u8> = Vec::new();
    let mut number = 0usize;
    let mut frontier: Vec<String> = Vec::new();

    let file = File::open(&path).map_err(|source| FlowError::Read {
        path: path.clone(),
        source,
    })?;
    let mut reader = BufReader::new(file);
    while number < end {
        let remaining = scan_limit.saturating_sub(scanned);
        if remaining == 0 || began.elapsed().as_secs_f64() > scan_seconds {
            frontier.push("scan budget exhausted before requested range completed".into());
            break;
        }
        let mut line = Vec::new();
        read_line_bounded(&mut reader, remaining, &mut line)?;
        scanned += line.len();
        if line.is_empty() {
            frontier.push("source ends before requested range completed".into());
            break;
        }
        number += 1;
        if number >= start {
            if selected.len() + line.len() > byte_limit {
                frontier.push("source byte budget exhausted; requested excerpt withheld".into());
                break;
            }
            selected.extend_from_slice(&line);
        }
        if !line.ends_with(b"\n") && scanned >= scan_limit {
            frontier.push("scan budget may split a line; requested excerpt withheld".into());
            break;
        }
    }

    // A source that vanished MID-READ is a frontier, not an error: the bytes it took to discover
    // that are already spent, and returning an error would drop the usage this operation must
    // still be charged for.
    match path.metadata() {
        Ok(after)
            if before.len() == after.len() && modified_nanos(&before) == modified_nanos(&after) => {
        }
        Ok(_) => frontier.push("source changed while reading".into()),
        Err(_) => frontier.push("source disappeared while reading".into()),
    }
    let content = match String::from_utf8(selected.clone()) {
        Ok(text) => text,
        Err(_) => {
            frontier.push("invalid UTF-8 in selected range".into());
            String::new()
        }
    };
    let sources = if frontier.is_empty() {
        json!([{
            "path": source,
            "lines": lines,
            "content": content,
            "fingerprint": format!("sha256:{}", hex(&Sha256::digest(&selected))),
            "fingerprint_scope": "exact excerpt bytes only; not a complete source fingerprint",
        }])
    } else {
        json!([])
    };
    Ok(json!({
        "sources": sources,
        "usage": {"source_files": 1, "source_bytes": selected.len(), "scan_bytes": scanned},
        "unresolved": frontier,
    }))
}

fn modified_nanos(meta: &std::fs::Metadata) -> u128 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

fn parse_range(lines: &str) -> FlowResult<(usize, usize)> {
    let malformed = || refused("--lines requires an inclusive positive START:END range");
    let (a, b) = lines.split_once(':').ok_or_else(malformed)?;
    let digits = |s: &str| {
        (!s.is_empty() && !s.starts_with('0') && s.bytes().all(|c| c.is_ascii_digit()))
            .then(|| s.parse::<usize>().ok())
            .flatten()
    };
    let (start, end) = (
        digits(a).ok_or_else(malformed)?,
        digits(b).ok_or_else(malformed)?,
    );
    if end < start {
        return Err(malformed());
    }
    Ok((start, end))
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Receipts — private, content-named, never overwritten
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// Write one receipt under the session's private store, or prove the existing bytes are identical.
///
/// Exclusive-create then compare, never truncate: a receipt is evidence of what was read, and
/// silently replacing one would make the continuation's pin point at bytes nobody saw.
pub fn save_receipt(
    root: &Path,
    session: &str,
    role: &str,
    method: &str,
    value: &Value,
) -> FlowResult<Value> {
    let raw = format!("{}\n", serde_json::to_string(value)?).into_bytes();
    let digest = hex(&Sha256::digest(&raw));
    let dir = root.join(RECALL_DIR_REL).join(session).join("receipts");
    std::fs::create_dir_all(&dir)?;
    let name = format!("{role}-{digest}.json");
    let path = dir.join(&name);
    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .custom_flags(libc_nofollow())
        .mode(0o600)
        .open(&path)
    {
        Ok(mut file) => {
            file.write_all(&raw)?;
            file.flush()?;
            file.sync_all()?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let meta = path.symlink_metadata()?;
            if !meta.file_type().is_file() {
                return Err(refused("existing receipt must be a regular file"));
            }
            let mut existing = Vec::new();
            File::open(&path)?
                .take(raw.len() as u64 + 1)
                .read_to_end(&mut existing)?;
            if existing != raw {
                return Err(refused("existing evidence receipt bytes differ"));
            }
        }
        Err(error) => {
            return Err(FlowError::Read {
                path,
                source: error,
            })
        }
    }
    Ok(json!({
        "path": format!("{RECALL_DIR_REL}/{session}/receipts/{name}"),
        "sha256": digest,
        "bytes": raw.len(),
        "cid": BlobCid::compute_raw(&raw).to_string(),
        "method": method,
        "privacy": "Private session record; never imported, projected, witnessed or targeted by feedback.",
    }))
}

pub(crate) fn load_receipt(root: &Path, reference: &Value, limit: usize) -> FlowResult<Value> {
    let relative = reference
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| refused("invalid receipt path"))?;
    if relative
        .split('/')
        .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(refused("invalid receipt path"));
    }
    let path = root.join(relative);
    let meta = path.symlink_metadata().map_err(|_| {
        refused("receipt unavailable within bound or changed; cannot reuse evidence")
    })?;
    if !meta.file_type().is_file() {
        return Err(refused("receipt must be a regular file"));
    }
    let mut raw = Vec::new();
    File::open(&path)
        .and_then(|f| f.take(limit as u64 + 1).read_to_end(&mut raw).map(|_| ()))
        .map_err(|_| {
            refused("receipt unavailable within bound or changed; cannot reuse evidence")
        })?;
    if raw.len() > limit || hex(&Sha256::digest(&raw)) != reference["sha256"].as_str().unwrap_or("")
    {
        return Err(refused(
            "receipt unavailable within bound or changed; cannot reuse evidence",
        ));
    }
    Ok(serde_json::from_slice(&raw)?)
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Receipt adoption — relocate the kit's store under the private owner, CIDs re-verified
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// Relocate `.claude/memory-kit/recall-executions/` into `.eprfs/status/recall/<session>/`.
///
/// Verbatim bytes, never rewritten: an adopted continuation whose `method` is the Python executor's
/// digest map keeps it, and is reported `resumable: false` naming the reason. Rewriting it to the
/// current contract CID would be tampering with the evidence the relocation exists to preserve.
pub fn adopt_receipts(root: &Path, from: &Path, method: &str, dry_run: bool) -> FlowResult<Value> {
    if !from.is_dir() {
        return Err(refused(format!(
            "--adopt-receipts names no directory: {}",
            from.display()
        )));
    }
    let mut rows = Vec::new();
    let (mut adopted, mut skipped, mut verified, mut mismatched) = (0usize, 0usize, 0usize, 0usize);
    let mut names: Vec<String> = std::fs::read_dir(from)?
        .flatten()
        .filter(|e| {
            e.path()
                .symlink_metadata()
                .is_ok_and(|m| m.file_type().is_file())
        })
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    for name in names {
        let source = from.join(&name);
        let Some(stem) = name.strip_suffix(".json") else {
            skipped += 1;
            rows.push(json!({"name": name, "action": "skipped",
                             "reason": "not a JSON receipt or continuation"}));
            continue;
        };
        let raw = std::fs::read(&source)?;
        let digest = hex(&Sha256::digest(&raw));
        let cid = BlobCid::compute_raw(&raw).to_string();
        let (session, destination, kind, pin_state) = match split_receipt_name(stem) {
            Some((session, role, pinned)) => {
                let ok = pinned == digest;
                if ok {
                    verified += 1;
                } else {
                    mismatched += 1;
                }
                (
                    session.clone(),
                    format!("{RECALL_DIR_REL}/{session}/receipts/{role}-{pinned}.json"),
                    "receipt",
                    if ok { "verified" } else { "digest-mismatch" },
                )
            }
            None => (
                stem.to_string(),
                format!("{RECALL_DIR_REL}/{stem}/continuation.json"),
                "continuation",
                "recomputed",
            ),
        };
        if !valid_session(&session) {
            skipped += 1;
            rows.push(json!({"name": name, "action": "skipped",
                             "reason": "session label is not a simple local name"}));
            continue;
        }
        let resumable = kind == "continuation"
            && serde_json::from_slice::<Value>(&raw)
                .ok()
                .and_then(|v| v.get("method").and_then(Value::as_str).map(str::to_string))
                .as_deref()
                == Some(method);
        let target = root.join(&destination);
        let mut action = "adopted";
        if pin_state == "digest-mismatch" {
            action = "refused";
        } else if target.exists() {
            let existing = std::fs::read(&target)?;
            action = if existing == raw {
                "already-adopted"
            } else {
                "conflict"
            };
        }
        if !dry_run && action == "adopted" {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&target, &raw)?;
            let mut permissions = std::fs::metadata(&target)?.permissions();
            std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o600);
            std::fs::set_permissions(&target, permissions)?;
        }
        if action == "adopted" {
            adopted += 1;
        }
        rows.push(json!({
            "name": name, "kind": kind, "session": session, "destination": destination,
            "sha256": digest, "cid": cid, "pin": pin_state, "action": action,
            "resumable": resumable,
            "resumable_reason": if resumable { Value::Null } else {
                json!("pinned method differs from the current contract CID; the receipt is retained, a new session is required") },
        }));
    }
    Ok(json!({
        "operation": "adopt-receipts",
        "from": rel_to_root(root, from),
        "into": RECALL_DIR_REL,
        "dry_run": dry_run,
        "method": method,
        "counts": {"examined": rows.len(), "adopted": adopted, "skipped": skipped,
                   "receipt_digests_verified": verified, "receipt_digests_mismatched": mismatched},
        "rows": rows,
        "privacy": "Relocated verbatim into the private store; adoption imports nothing into any governed plane.",
        "meaning": "Bytes are copied unchanged and their CIDs recomputed. A continuation pinned to another algorithm stays unresumable rather than being rewritten.",
    }))
}

/// `<session>-<role>-<64 hex>` → the three parts, when the name really is a content-named receipt.
pub(crate) fn split_receipt_name(stem: &str) -> Option<(String, String, String)> {
    let (head, digest) = stem.rsplit_once('-')?;
    if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let (session, role) = head.rsplit_once('-')?;
    if !matches!(role, "projection" | "baseline" | "close") {
        return None;
    }
    Some((session.to_string(), role.to_string(), digest.to_string()))
}

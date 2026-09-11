//! Discovery — the deterministic local traversal that is the pinned recipe's default provider
//! (`discover`/`discover_scored`), plus the focused door that ranks it by a question's terms
//! (`first_screen`, `matching_habits`, `focus_area`) and the outline machinery that locates a
//! term inside a source once discovery has named it a candidate (`outline`, `best_section`).
//!
//! Moved out of `mod.rs` verbatim (governed-discovery station zero, task 0.4) so `providers.rs`
//! can wrap the traversal in the `Provider` trait without the two seams sharing one 3,700-line
//! file. Behaviour is unchanged — only the location moved, plus two visibility widenings
//! (`first_screen`, `outline`) so `mod.rs`'s `execute()` can still reach them.
use super::*;

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Discovery — deterministic local traversal, the default provider
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// Metadata-only bounded traversal: never a whole-repository filename list, never a relevance
/// ranking, and never a claim about the corpus outside the window it actually inspected.
pub fn discover(
    root: &Path,
    contract: &Contract,
    scope: &str,
    query: &str,
    tags: &[String],
    group_by: &str,
    name: &str,
) -> FlowResult<Value> {
    discover_scored(root, contract, scope, query, &[], tags, group_by, name)
}

/// The same bounded traversal, ranked by how many of `terms` a row's declared metadata carries.
///
/// One traversal serves both doors. `search` and `source --tag` pass no terms and get today's
/// deterministic window; the focused first screen passes the question's terms and gets the rows
/// that mention the most of them first. Ranking is over DECLARED metadata only — it is candidate
/// discovery, never authority, which is why every row still arrives with a `read` command rather
/// than an excerpt.
#[allow(clippy::too_many_arguments)]
pub fn discover_scored(
    root: &Path,
    contract: &Contract,
    scope: &str,
    query: &str,
    terms: &[String],
    tags: &[String],
    group_by: &str,
    name: &str,
) -> FlowResult<Value> {
    let base = contained(root, scope, &contract.source_roots())?;
    if !base.is_dir() {
        return Err(refused("discovery scope must be a directory"));
    }
    let pattern = glob::Pattern::new(name).map_err(|_| refused("--name is not a valid glob"))?;
    let excluded: BTreeSet<String> = contract
        .value
        .pointer("/discovery/exclude_directories")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    let (scan_bytes_limit, scan_files_limit, scan_entries_limit) = (
        contract.limit_usize("scan_bytes"),
        contract.limit_usize("scan_files"),
        contract.limit_usize("scan_entries"),
    );
    let metadata_bytes = contract.limit_usize("metadata_bytes");
    // The per-file window term matching may read. It is NOT the frontmatter window: membership is
    // still established from `metadata_bytes` of byte-exact header, and a wider body window never
    // widens what qualifies a document. It is spent only when there is something to match — a pure
    // tag filter reads no more than it ever did, so tag discovery keeps its file coverage.
    let body_scan_bytes = contract
        .value
        .pointer("/limits/body_scan_bytes")
        .and_then(Value::as_u64)
        .map_or(metadata_bytes, |value| value as usize)
        .max(metadata_bytes);
    let matching = !terms.is_empty() || !query.is_empty();
    let per_file = if matching {
        body_scan_bytes
    } else {
        metadata_bytes
    };
    let scan_seconds = contract.limit_secs("scan_seconds");
    let search_results = contract.limit_usize("search_results");
    // With terms the window is not closed early AT ALL: every row the scan budget allows is
    // collected, ranked, and only then truncated to the declared result count. Stopping at a small
    // multiple of `search_results` made the ranking a ranking of whatever the traversal happened to
    // reach first — which, once body text became searchable and nearly every document matched
    // SOMETHING, was indistinguishable from no ranking. The traversal is still bounded by
    // scan_bytes/scan_files/scan_entries/scan_seconds exactly as before; only the RESULT window
    // moved, and the frontier still names the budget that stopped the walk.
    let collect_cap = if terms.is_empty() {
        search_results
    } else {
        scan_files_limit.max(search_results)
    };

    let began = Instant::now();
    let mut stack = vec![base];
    let mut candidates: Vec<Value> = Vec::new();
    let mut frontier: Vec<String> = Vec::new();
    // Candidates whose frontmatter would not parse. They are SKIPPED, COUNTED and NAMED rather
    // than aborting the traversal (2026-09-11 ruling): the honesty rule exists so a window never
    // hides what it could not read, and naming each one satisfies that, while stopping on the first
    // one made the whole screen useless because of a handful of documents nobody chose.
    let mut unreadable: Vec<String> = Vec::new();
    let mut budget_cut = false;
    // Eligible documents bigger than the window they were read with. A term past the window is
    // invisible, and invisible is not absent — so the count is reported rather than implied.
    let mut partially_scanned = 0usize;
    let (mut scan_bytes, mut scanned_files, mut scanned_entries) = (0usize, 0usize, 0usize);

    'outer: while let Some(directory) = stack.pop() {
        if !frontier.is_empty() {
            break;
        }
        let entries = match std::fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) => {
                frontier.push(format!("metadata read failed: {}", error.kind()));
                break;
            }
        };
        for entry in entries.flatten() {
            scanned_entries += 1;
            if scanned_entries > scan_entries_limit
                || scanned_files >= scan_files_limit
                || scan_bytes >= scan_bytes_limit
                || began.elapsed().as_secs_f64() > scan_seconds
            {
                frontier.push(
                    "discovery budget exhausted; narrow the scope or continue with a named question"
                        .into(),
                );
                break 'outer;
            }
            let file_name = entry.file_name().to_string_lossy().to_string();
            let Ok(meta) = entry.path().symlink_metadata() else {
                continue;
            };
            if meta.file_type().is_symlink() {
                continue;
            }
            if meta.is_dir() {
                if !excluded.contains(&file_name) {
                    stack.push(entry.path());
                }
                continue;
            }
            if !pattern.matches(&file_name) || !file_name.ends_with(".md") || !meta.is_file() {
                continue;
            }
            scanned_files += 1;
            let budget = per_file.min(scan_bytes_limit.saturating_sub(scan_bytes));
            let mut data = Vec::new();
            let opened = File::open(entry.path())
                .and_then(|f| f.take(budget as u64).read_to_end(&mut data).map(|_| ()));
            if opened.is_err() {
                frontier.push("metadata read failed: OSError".into());
                break 'outer;
            }
            scan_bytes += data.len();
            let relative = rel_to_root(root, &entry.path());
            let truncated = meta.len() as usize > data.len();
            // A BOUNDED read ends wherever the budget says, which is very often mid-character. The
            // cut is an artifact of this reader, not a fault in the document, so the tail is
            // trimmed back to the last complete character rather than condemning the file: on
            // 2026-09-11 that mistake reported `.claude/skills/converge/SKILL.md` — 708 bytes of
            // perfectly valid frontmatter — as unparseable, because an em dash sat across byte
            // 8192. The frontmatter itself is still EXACT bytes: if the trim lands before the
            // closing delimiter the header simply cannot be proven and the row is skipped as
            // incomplete, which is true.
            // Membership is read from the METADATA window only, whatever the body window read.
            let head_len = metadata_bytes.min(data.len());
            let Some(head) = trim_to_character_boundary(&data[..head_len]) else {
                unreadable.push(relative);
                continue;
            };
            let Some(header) = frontmatter_header(&head, head.len(), meta.len() as usize) else {
                // A document that opens `---` and whose boundary this read could not prove is a
                // candidate we could not read. A file with no frontmatter at all is not a candidate
                // in the first place, and is passed over as it always was.
                if head.starts_with("---\n") {
                    // Only a read that actually hit its cap can be blamed on the budget; anything
                    // shorter simply has no closing delimiter.
                    budget_cut |= head_len >= metadata_bytes;
                    unreadable.push(relative);
                }
                continue;
            };
            if truncated {
                partially_scanned += 1;
            }
            // The body window is the whole read, trimmed the same way; the header slice above is a
            // prefix of it, so `header.len()` still indexes correctly.
            let text = if head_len == data.len() {
                head
            } else {
                match trim_to_character_boundary(&data) {
                    Some(text) => text,
                    None => head,
                }
            };
            let Ok(meta_value) = serde_yaml::from_str::<serde_yaml::Value>(&header[4..]) else {
                unreadable.push(relative);
                continue;
            };
            let Some(mapping) = meta_value.as_mapping() else {
                continue;
            };
            // Field lookup by iteration rather than `Mapping::get`, so the reader does not depend
            // on which `Index` impls a given serde_yaml minor happens to expose.
            let field = |key: &str| {
                mapping
                    .iter()
                    .find(|(k, _)| k.as_str() == Some(key))
                    .map(|(_, v)| v)
            };
            let actual_tags = match field("tags") {
                None => Some(Vec::new()),
                Some(serde_yaml::Value::Sequence(items)) => items
                    .iter()
                    .map(|item| item.as_str().map(str::to_string))
                    .collect::<Option<Vec<String>>>(),
                Some(_) => None,
            };
            // A `tags:` field that is not a list of strings is not category membership; the row is
            // skipped rather than coerced, exactly as the oracle skips it.
            let Some(actual_tags) = actual_tags else {
                continue;
            };
            let title = field("title")
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_else(|| file_name.clone());
            let description = field("description")
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_default();
            // The bytes after the closing delimiter were ALREADY READ under `metadata_bytes`.
            // Matching them costs no second read and no wider budget, and it is where an agent's
            // own words actually live: a fresh reader asking "re-mine", "marker", "stamp" on
            // 2026-09-11 got zero candidates over a corpus whose bodies say all three, because
            // discovery only ever looked at declared metadata.
            let body_prefix = text
                .get(header.len()..)
                .and_then(|rest| rest.split_once('\n'))
                .map(|(_, body)| body)
                .unwrap_or_default();
            let declared = [
                relative.as_str(),
                title.as_str(),
                description.as_str(),
                &actual_tags.join(" "),
            ]
            .join(" ")
            .to_lowercase();
            let body = body_prefix.to_lowercase();
            if !tags.iter().all(|t| actual_tags.contains(t))
                || (!query.is_empty() && {
                    let needle = query.to_lowercase();
                    !declared.contains(&needle) && !body.contains(&needle)
                })
            {
                continue;
            }
            // Where a term was found is evidence about how strong the hit is, so it is reported
            // rather than folded into one number. A declared hit outranks a body-only hit; both
            // remain candidate discovery, never authority.
            let (mut declared_hits, mut body_only_hits) = (0usize, 0usize);
            let mut kinds: BTreeSet<&str> = BTreeSet::new();
            let mut matched_terms: Map<String, Value> = Map::new();
            for term in terms {
                let needle = term.to_lowercase();
                let in_path = relative.to_lowercase().contains(&needle);
                let in_title = title.to_lowercase().contains(&needle);
                let in_description = description.to_lowercase().contains(&needle);
                let in_tag = actual_tags
                    .iter()
                    .any(|t| t.to_lowercase().contains(&needle));
                let in_body = body.contains(&needle);
                if in_path {
                    kinds.insert("path");
                }
                if in_title {
                    kinds.insert("title");
                }
                if in_description {
                    kinds.insert("description");
                }
                if in_tag {
                    kinds.insert("tag");
                }
                if in_body {
                    kinds.insert("body");
                }
                // How OFTEN, not merely whether: a document that names the concern once in
                // passing and one that is about it are not equal evidence, and the occurrences are
                // already inside the bytes this read covered.
                let occurrences = declared.matches(&needle).count() + body.matches(&needle).count();
                if in_path || in_title || in_description || in_tag {
                    declared_hits += 1;
                    matched_terms.insert(
                        needle,
                        json!({"found_in": "declared", "occurrences": occurrences}),
                    );
                } else if in_body {
                    body_only_hits += 1;
                    matched_terms.insert(
                        needle,
                        json!({"found_in": "body", "occurrences": occurrences}),
                    );
                }
            }
            if !terms.is_empty() && declared_hits + body_only_hits == 0 {
                continue;
            }
            candidates.push(json!({
                "path": relative,
                "title": title,
                "tags": actual_tags,
                "term_hits": declared_hits + body_only_hits,
                "declared_hits": declared_hits,
                "match": kinds.iter().collect::<Vec<_>>(),
                "matched_terms": matched_terms,
                "match_scope": "declared frontmatter and the bounded body prefix this read already \
                                covered; a body hit beyond the metadata budget is not visible here",
                "metadata_fingerprint": hex(&Sha256::digest(header.as_bytes())),
            }));
            if candidates.len() >= collect_cap {
                frontier.push("result window reached; remaining corpus not inspected".into());
                break 'outer;
            }
        }
    }

    // The account of what was skipped, said once and never silently: a named list bounded the same
    // way every other list in this view is, plus a count on the frontier so a reader scanning only
    // the unresolved lines still learns that candidates went unread.
    let mut omissions: Vec<Value> = Vec::new();
    if partially_scanned > 0 {
        omissions.push(json!(format!(
            "{partially_scanned} candidate(s) scanned to the body-scan window only"
        )));
    }
    if !unreadable.is_empty() {
        unreadable.sort();
        let shown = unreadable.len().min(8);
        let mut named = unreadable[..shown].join(", ");
        if unreadable.len() > shown {
            named.push_str(&format!(", +{} more", unreadable.len() - shown));
        }
        omissions.push(json!(format!(
            "{} candidate(s) unreadable (frontmatter did not parse) and were skipped: {named}",
            unreadable.len()
        )));
        frontier.push(format!("unreadable candidates: {}", unreadable.len()));
        if budget_cut {
            frontier.push(
                "some candidate frontmatter exceeded the metadata budget; raise                  limits.metadata_bytes to read it"
                    .into(),
            );
        }
    }

    candidates.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
    if !terms.is_empty() {
        // Two rules, and the second is what makes the first useful.
        //
        // A declared hit outranks a body hit ON THE SAME TERM — weighted per term (declared ×2)
        // rather than as a lexicographic override, so one hit on a word every document shares can
        // never outrank several hits on the words that name the concern.
        //
        // And a term is worth what it distinguishes. Counting matched terms equally made
        // `commands` and `index` — carried by forty of forty-four skills — worth as much as
        // `mempalace` and `re-mine`, carried by two, which is how the document that answered the
        // reader's question came fifth with the window already wide enough to hold the answer. The
        // weight is the term's rarity IN THE SCANNED WINDOW, so it is derived from what this call
        // actually read and needs no corpus statistics kept anywhere.
        let total = candidates.len().max(1) as f64;
        let mut frequency: BTreeMap<String, usize> = BTreeMap::new();
        for row in &candidates {
            for term in row["matched_terms"].as_object().into_iter().flatten() {
                *frequency.entry(term.0.clone()).or_insert(0) += 1;
            }
        }
        let score = |row: &Value| -> f64 {
            row["matched_terms"]
                .as_object()
                .map(|matched| {
                    matched
                        .iter()
                        .map(|(term, hit)| {
                            let occurrences =
                                hit["occurrences"].as_u64().unwrap_or(1).max(1) as f64;
                            // Sublinear in the count: the tenth mention says less than the second.
                            let weight = 1.0 + occurrences.ln();
                            let shared = *frequency.get(term).unwrap_or(&1) as f64;
                            // A term nearly every candidate carries distinguishes nothing. The
                            // floor keeps it contributing a little rather than nothing, so a
                            // reader's ordinary words are not simply discarded.
                            let rarity = (total / shared).ln().max(0.01);
                            let placement = if hit["found_in"] == "declared" {
                                2.0
                            } else {
                                1.0
                            };
                            weight * rarity * placement
                        })
                        .sum()
                })
                .unwrap_or(0.0)
        };
        candidates.sort_by(|a, b| {
            score(b)
                .partial_cmp(&score(a))
                .unwrap_or(Ordering::Equal)
                .then_with(|| {
                    b["declared_hits"]
                        .as_u64()
                        .cmp(&a["declared_hits"].as_u64())
                })
                .then_with(|| a["path"].as_str().cmp(&b["path"].as_str()))
        });
        candidates.truncate(search_results);
    }
    let mut groups: Map<String, Value> = Map::new();
    for row in &candidates {
        let keys: Vec<String> = if group_by == "tag" {
            row["tags"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default()
        } else {
            let path = row["path"].as_str().unwrap_or_default();
            vec![Path::new(path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .filter(|p| !p.is_empty())
                .unwrap_or_else(|| ".".into())]
        };
        for key in keys {
            let previous = groups.get(&key).and_then(Value::as_u64).unwrap_or(0);
            groups.insert(key, json!(previous + 1));
        }
    }
    Ok(json!({
        "candidates": candidates,
        "groups": groups,
        "omissions": omissions,
        "unreadable_candidates": unreadable.len(),
        "partially_scanned_candidates": partially_scanned,
        "aggregation_scope": "returned candidates only",
        "selection": if terms.is_empty() {
            "bounded filesystem traversal; sorted returned window, no relevance ranking"
        } else {
            "bounded filesystem traversal ranked by term overlap over declared metadata and the bounded body window already read, each term weighted by how often it occurs and how rare it is in that window, a declared hit worth twice a body hit; candidate discovery, not authority"
        },
        "usage": {"scan_bytes": scan_bytes, "scanned_files": scanned_files,
                  "scanned_entries": scanned_entries, "search_queries": 1},
        "unresolved": frontier,
        "next": "Select a path and explicit --source/--lines; membership does not establish authority.",
    }))
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// The focused door — the area's habits, its last delta, and the competing sources
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// Where the habit register is projected. Read, never written, by this executor.
pub const HABITS_REL: &str = "genesis/manifests/habits.yaml";

/// Words that carry no area, so they never select a habit or a source.
const STOPWORDS: [&str; 40] = [
    "about", "after", "again", "against", "because", "before", "being", "between", "could", "does",
    "doing", "down", "from", "does", "have", "here", "how", "into", "just", "like", "make", "more",
    "most", "much", "must", "only", "other", "over", "same", "should", "some", "such", "than",
    "that", "them", "then", "there", "this", "were", "what",
];

/// The question's distinctive terms: what an area match is made of.
fn question_terms(need: &str) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for raw in need.split(|c: char| {
        !(c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '/' || c == '.')
    }) {
        let term = raw
            .trim_matches(|c| c == '.' || c == '/')
            .to_ascii_lowercase();
        if term.len() < 4 || STOPWORDS.contains(&term.as_str()) || seen.contains(&term) {
            continue;
        }
        seen.push(term);
        if seen.len() >= 12 {
            break;
        }
    }
    seen
}

/// One habit as the first screen needs it: what it promises, whether it holds, and what moved last.
fn habit_row(id: &str, status: &str, invariant: &str, evidence: &str, score: usize) -> Value {
    // The FIRST line of the ledger is the newest entry — the atoms are written newest-first — so
    // "what moved last" is one line, never the whole ledger.
    let delta = evidence
        .split("\n\n")
        .map(one_line)
        .find(|paragraph| !paragraph.is_empty())
        .map(|paragraph| clip(&paragraph, 240))
        .unwrap_or_default();
    json!({
        "id": id,
        "status": status,
        "delta": delta,
        "invariant": truncate(invariant, 400),
        "match_score": score,
    })
}

/// The habits whose id or invariant words the question's terms touch.
///
/// The register is a GENERATED projection of the habit atoms, read here by a bounded line scan
/// rather than a whole-document YAML load: it is 400 KB of evidence ledgers, and the first screen
/// needs four fields per habit. Its bytes are charged under their own declared budget, because
/// they are neither source evidence the reader quoted nor a native projection.
fn matching_habits(
    root: &Path,
    contract: &Contract,
    terms: &[String],
    usage: &mut Value,
) -> FlowResult<(Vec<Value>, Vec<String>)> {
    let mut unresolved: Vec<String> = Vec::new();
    let path = match contained(root, HABITS_REL, &contract.source_roots()) {
        Ok(path) => path,
        Err(_) => return Ok((Vec::new(), unresolved)),
    };
    if !path.is_file() {
        return Ok((Vec::new(), unresolved));
    }
    let budget = contract
        .value
        .pointer("/limits/habit_register_bytes")
        .and_then(Value::as_u64)
        .unwrap_or(1_048_576) as usize;
    let mut data = Vec::new();
    File::open(&path)
        .and_then(|file| {
            file.take(budget as u64 + 1)
                .read_to_end(&mut data)
                .map(|_| ())
        })
        .map_err(|source| FlowError::Read { path, source })?;
    if data.len() > budget {
        data.truncate(budget);
        unresolved.push(
            "habit register exceeds its declared budget; only the leading rows were inspected"
                .into(),
        );
    }
    add_usage(usage, &json!({"habit_register_bytes": data.len()}));
    let text = String::from_utf8_lossy(&data).into_owned();

    let (mut id, mut status, mut invariant, mut evidence) =
        (String::new(), String::new(), String::new(), String::new());
    let mut folding: Option<&'static str> = None;
    let mut rows: Vec<Value> = Vec::new();
    let mut flush = |id: &str, status: &str, invariant: &str, evidence: &str| {
        if id.is_empty() {
            return;
        }
        let lowered = id.to_ascii_lowercase();
        let parts: Vec<&str> = lowered.split('-').collect();
        let in_id = terms
            .iter()
            .filter(|term| lowered.contains(term.as_str()) || parts.contains(&term.as_str()))
            .count();
        let in_invariant = terms
            .iter()
            .filter(|term| invariant.to_ascii_lowercase().contains(term.as_str()))
            .count();
        let score = in_id * 3 + in_invariant;
        if score >= 3 {
            rows.push(habit_row(id, status, invariant, evidence, score));
        }
    };
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("  - id: ") {
            flush(&id, &status, &invariant, &evidence);
            id = rest.trim().to_string();
            status.clear();
            invariant.clear();
            evidence.clear();
            folding = None;
            continue;
        }
        if folding.is_some() && line.trim().is_empty() {
            let target = match folding {
                Some("invariant") => &mut invariant,
                _ => &mut evidence,
            };
            target.push_str("\n\n");
            continue;
        }
        if let Some(rest) = line.strip_prefix("    ") {
            if !rest.starts_with(' ') && !rest.starts_with('-') {
                folding = None;
                if let Some(value) = rest.strip_prefix("status:") {
                    status = value.trim().to_string();
                } else if rest.starts_with("invariant:") {
                    folding = Some("invariant");
                } else if rest.starts_with("evidence:") {
                    folding = Some("evidence");
                }
                continue;
            }
            // The register folds these scalars with `>`, so a wrapped line is a continuation and
            // a BLANK line is the paragraph break. Reassembling that way is what makes "the first
            // line of the newest delta" a sentence rather than the first 100 columns of one.
            let target = match folding {
                Some("invariant") => &mut invariant,
                Some("evidence") => &mut evidence,
                _ => continue,
            };
            if rest.trim().is_empty() {
                target.push_str("\n\n");
            } else {
                target.push_str(rest.trim());
                target.push(' ');
            }
        }
    }
    flush(&id, &status, &invariant, &evidence);
    rows.sort_by(|a, b| b["match_score"].as_u64().cmp(&a["match_score"].as_u64()));
    rows.truncate(3);
    Ok((rows, unresolved))
}

/// The directory this question is about, or `None` when it is about the whole tree.
fn focus_area(root: &Path, contract: &Contract, scope: &str, terms: &[String]) -> Option<String> {
    let roots = contract.source_roots();
    let named = |candidate: &str| {
        contained(root, candidate, &roots)
            .ok()
            .filter(|path| path.is_dir())
            .map(|_| candidate.trim_end_matches('/').to_string())
    };
    if scope != "." {
        if let Some(area) = named(scope) {
            return Some(area);
        }
    }
    terms
        .iter()
        .filter(|term| term.contains('/'))
        .find_map(|term| named(term))
}

/// The first screen a focused question gets: the area's habits and the sources competing to answer
/// it, before any stale-edge group.
///
/// Whole-scope opens get `None` — the convergence question is answered by the grouped stale-edge
/// view, and putting a ranked source list in front of it would answer a question nobody asked.
pub(super) fn first_screen(
    args: &Args,
    contract: &Contract,
    state: &Value,
    usage: &mut Value,
) -> FlowResult<Option<Value>> {
    if !args.need_explicit {
        return Ok(None);
    }
    let terms = question_terms(args.need.trim());
    if terms.is_empty() {
        return Ok(None);
    }
    let scope = state["scope"].as_str().unwrap_or(".");
    let (habits, mut unresolved) = matching_habits(&args.root, contract, &terms, usage)?;
    let area = focus_area(&args.root, contract, scope, &terms);
    if area.is_none() && habits.is_empty() {
        return Ok(None);
    }
    let mut candidates: Vec<Value> = Vec::new();
    let mut omissions: Vec<String> = Vec::new();
    let mut ranking = Value::Null;
    if let Some(area) = &area {
        // A term that names the area itself matches every file under it, so it ranks nothing. The
        // discriminating terms are the ones the area does NOT already carry.
        let lowered = area.to_ascii_lowercase();
        let ranking_terms: Vec<String> = terms
            .iter()
            .filter(|term| !term.contains('/') && !lowered.contains(term.as_str()))
            .cloned()
            .collect();
        let mut found = discover_scored(
            &args.root,
            contract,
            area,
            "",
            &ranking_terms,
            &[],
            "directory",
            "*.md",
        )?;
        let charged = found["usage"].take();
        add_usage(usage, &charged);
        candidates = found["candidates"].as_array().cloned().unwrap_or_default();
        // Each candidate is located, not merely named: the section its terms land in, and a read
        // range bounded to one excerpt. The outline scan is charged to the scan counters.
        for candidate in candidates.iter_mut() {
            let Some(path) = candidate["path"].as_str().map(str::to_string) else {
                continue;
            };
            if let Some(section) = best_section(args, contract, &path, &terms) {
                add_usage(usage, &section["usage"]);
                candidate["best_section"] = json!({
                    "title": section["title"],
                    "lines": section["read_lines"],
                    "hits": section["hits"],
                    "window_complete": section["window_complete"],
                });
            }
        }
        ranking = found["selection"].take();
        for message in found["omissions"].as_array().cloned().unwrap_or_default() {
            omissions.push(message.as_str().unwrap_or_default().to_string());
        }
        for message in found["unresolved"].as_array().cloned().unwrap_or_default() {
            unresolved.push(message.as_str().unwrap_or_default().to_string());
        }
    } else {
        unresolved.push(
            "no directory scope in this question; name --scope <dir> for ranked candidate sources"
                .into(),
        );
    }
    Ok(Some(json!({
        "area": area.clone().unwrap_or_else(|| scope.to_string()),
        "terms": terms,
        "habits": habits,
        "candidates": candidates,
        "provider": "local",
        "ranking": ranking,
        "authority": "Candidate discovery over declared metadata. Membership and rank establish \
                      neither authority nor currency; read the passage.",
        "omissions": omissions,
        "unresolved": unresolved,
    })))
}

/// Decode a bounded read, discarding a character the budget cut in half.
///
/// A byte budget ends wherever it ends, very often mid-character; that is a property of the reader,
/// not a fault in the document. `None` means not even the first character survived.
fn trim_to_character_boundary(data: &[u8]) -> Option<String> {
    match std::str::from_utf8(data) {
        Ok(text) => Some(text.to_string()),
        Err(error) => match error.valid_up_to() {
            0 => None,
            boundary => Some(String::from_utf8_lossy(&data[..boundary]).into_owned()),
        },
    }
}

/// The complete `---` fenced header, or `None` when the budget could not prove one is complete.
///
/// The subtle case is the last one: a delimiter that lands exactly at the end of the buffer with no
/// trailing newline may be a `---` prefix the budget cut in half. When more file remains unread,
/// that is not a proven document boundary, and treating it as one would let a truncated read
/// establish category membership.
fn frontmatter_header(text: &str, read_bytes: usize, file_bytes: usize) -> Option<String> {
    if !text.starts_with("---\n") {
        return None;
    }
    let body = &text[4..];
    let (start, end) = find_close(body)?;
    if end == body.len() && !text.ends_with('\n') && read_bytes < file_bytes {
        return None;
    }
    Some(text[..4 + start].to_string())
}

/// Locate a `^---[ \t]*\r?$` line in `body`, returning its (start, end) byte offsets.
fn find_close(body: &str) -> Option<(usize, usize)> {
    let mut position = 0usize;
    loop {
        let line_end = body[position..]
            .find('\n')
            .map(|i| position + i)
            .unwrap_or(body.len());
        let line = &body[position..line_end];
        if let Some(rest) = line.strip_prefix("---") {
            let rest = rest.strip_suffix('\r').unwrap_or(rest);
            if rest.chars().all(|c| c == ' ' || c == '\t') {
                return Some((position, line_end));
            }
        }
        if line_end >= body.len() {
            return None;
        }
        position = line_end + 1;
    }
}

/// Bounded table of contents, so the agent chooses a passage before loading it.
/// One section of an outline, with where the question's terms actually land inside it.
///
/// Naming a document is half an answer. A reader that opens a 24,000-byte skill and is handed six
/// headings, none of which mentions its question, closes the file — which is exactly what happened
/// on 2026-09-11: the answer was under "Phase 4 — verify the experience, reconcile and retain
/// learning", a heading that says nothing about re-mining an index.
pub(super) fn outline(args: &Args, contract: &Contract, path: &str) -> FlowResult<Value> {
    outline_with_terms(args, contract, path, &question_terms(args.need.trim()))
}

fn outline_with_terms(
    args: &Args,
    contract: &Contract,
    path: &str,
    terms: &[String],
) -> FlowResult<Value> {
    let source = contained(&args.root, path, &contract.source_roots())?;
    let bound = contract.limit_usize("scan_bytes");
    let mut raw = Vec::new();
    File::open(&source)
        .and_then(|f| f.take(bound as u64 + 1).read_to_end(&mut raw).map(|_| ()))
        .map_err(|error| FlowError::Read {
            path: source.clone(),
            source: error,
        })?;
    let complete = raw.len() <= bound;
    let text = String::from_utf8_lossy(&raw[..raw.len().min(bound)]).to_string();
    let lines: Vec<&str> = text.lines().collect();
    let is_code = source.extension().and_then(|e| e.to_str()) == Some("py");
    let mut headings: Vec<Value> = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if line.starts_with('#')
            || (is_code && (line.starts_with("def ") || line.starts_with("class ")))
        {
            headings.push(json!({
                "line": index + 1,
                "title": line.trim_start_matches('#').trim(),
            }));
        }
    }
    for index in 0..headings.len() {
        let end = if index + 1 < headings.len() {
            headings[index + 1]["line"].as_u64().unwrap_or(1) - 1
        } else {
            lines.len() as u64
        };
        headings[index]["end_line"] = json!(end);
    }
    // Where the question's terms land, section by section, plus a read range bounded to what one
    // excerpt may return. The bytes counted here are scan bytes; only `read` charges source_bytes.
    let source_bytes = contract.limit_usize("source_bytes");
    let mut oversized_sections = 0usize;
    for heading in headings.iter_mut() {
        let start = heading["line"].as_u64().unwrap_or(1) as usize;
        let end = (heading["end_line"].as_u64().unwrap_or(1) as usize).max(start);
        let section: String = lines[start.saturating_sub(1)..end.min(lines.len())]
            .join("\n")
            .to_lowercase();
        let mut hits: Map<String, Value> = Map::new();
        let mut total = 0usize;
        for term in terms {
            let count = section.matches(&term.to_lowercase()).count();
            if count > 0 {
                hits.insert(term.clone(), json!(count));
                total += count;
            }
        }
        // The emitted range never promises more than one excerpt can carry: lines are taken from
        // the section's start until the byte budget is spent, and a section that does not fit says
        // so rather than handing over a range `read` would refuse.
        let (mut window_end, mut bytes) = (start, 0usize);
        for (offset, line) in lines[start.saturating_sub(1)..end.min(lines.len())]
            .iter()
            .enumerate()
        {
            let next = bytes + line.len() + 1;
            if next > source_bytes && window_end > start {
                break;
            }
            bytes = next;
            window_end = start + offset;
        }
        heading["hits"] = json!(hits);
        heading["hit_total"] = json!(total);
        heading["read_lines"] = json!(format!("{start}:{window_end}"));
        heading["window_complete"] = json!(window_end >= end);
        if window_end < end {
            oversized_sections += 1;
        }
    }
    let truncated = !complete || headings.len() > 40;
    headings.truncate(40);
    let mut omissions: Vec<Value> = Vec::new();
    if truncated {
        omissions.push(json!("outline incomplete; use an explicit range"));
    }
    if oversized_sections > 0 {
        omissions.push(json!(format!(
            "{oversized_sections} section(s) exceed the per-excerpt byte budget; the offered range \
             is the first window of each — continue with an explicit later range"
        )));
    }
    Ok(json!({
        "path": path,
        "headings": headings,
        "line_count": lines.len(),
        "terms": terms,
        "hit_scope": "term occurrences inside each section of this outline; scan bytes, not evidence",
        "omissions": omissions,
        "usage": {"scan_bytes": raw.len(), "scanned_files": 1},
    }))
}

/// The section of `path` whose text carries most of the question's terms, if any does.
fn best_section(args: &Args, contract: &Contract, path: &str, terms: &[String]) -> Option<Value> {
    let outline = outline_with_terms(args, contract, path, terms).ok()?;
    let mut best: Option<Value> = None;
    for heading in outline["headings"].as_array()? {
        if heading["hit_total"].as_u64().unwrap_or(0) == 0 {
            continue;
        }
        let better = best
            .as_ref()
            .is_none_or(|current| heading["hit_total"].as_u64() > current["hit_total"].as_u64());
        if better {
            best = Some(heading.clone());
        }
    }
    let mut section = best?;
    section["usage"] = outline["usage"].clone();
    Some(section)
}

//! Discovery — the deterministic local traversal that is the pinned recipe's default provider
//! (`discover`/`discover_scored`), plus the focused door that ranks it by a question's terms
//! (`first_screen`, `matching_habits`, `focus_area`). Locating a term INSIDE a source once
//! discovery has named it a candidate (`outline`, `best_section`) lives in `passage.rs`.
//!
//! Moved out of `mod.rs` verbatim (governed-discovery station zero, task 0.4) so `providers.rs`
//! can wrap the traversal in the `Provider` trait without the two seams sharing one 3,700-line
//! file. Behaviour is unchanged — only the location moved, plus two visibility widenings
//! (`first_screen`, `outline`) so `mod.rs`'s `execute()` can still reach them.
use super::passage::{best_section, section_link};
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
    discover_scored(
        root,
        contract,
        scope,
        query,
        &[],
        tags,
        group_by,
        &[name.to_string()],
    )
}

/// Lower-level text with `_` and `-` read as word breaks and runs of whitespace collapsed, so a
/// phrase and an identifier spelling of the same words compare equal.
fn identifier_words(text: &str) -> String {
    text.replace(['_', '-'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Suffixes light stemming strips, longest first (`discovery.stemming = "suffix-strip-v1"`) —
/// tried in this order so an `"ings"`-shaped tail strips as `"ing"`, not as the plain `"s"` that
/// would leave a spurious trailing `"g"`.
const STEM_SUFFIXES: [&str; 4] = ["ing", "es", "ed", "s"];

/// The stem of `term` when stripping the first matching [`STEM_SUFFIXES`] entry leaves at least
/// four characters, else `term` itself. `"habits"` -> `"habit"` (kept: 5 chars); `"stamps"` ->
/// `"stamp"` (kept: 5 chars); `"cid"` (no matching suffix) -> `"cid"` unchanged; `"mined"` ->
/// `"min"` is BELOW the four-character floor, so it is left unstemmed rather than reduced to a
/// fragment common enough to match nearly everything. A literal suffix strip, not a lexical
/// dictionary — it will miss irregular inflections (`"prove"`/`"proof"`) it was never asked to
/// know, and that is the declared, modest shape of it.
pub(super) fn stem(term: &str) -> &str {
    for suffix in STEM_SUFFIXES {
        if let Some(stripped) = term.strip_suffix(suffix) {
            if stripped.chars().count() >= 4 {
                return stripped;
            }
        }
    }
    term
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
    names: &[String],
) -> FlowResult<Value> {
    let base = contained(root, scope, &contract.source_roots())?;
    if !base.is_dir() {
        return Err(refused("discovery scope must be a directory"));
    }
    // S1 (2026-09-22): one or more declared globs, matched in the SAME single traversal and
    // charged to the SAME scan/body-scan budget below — never a second read per glob, never a
    // wider window. `first_screen` is the caller that passes more than one (the contract's
    // `discovery.first_screen_globs`); every other caller still passes exactly one, unchanged.
    let mut patterns = Vec::with_capacity(names.len());
    for name in names {
        patterns.push(glob::Pattern::new(name).map_err(|_| refused("--name is not a valid glob"))?);
    }
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
            if !patterns.iter().any(|p| p.matches(&file_name)) || !meta.is_file() {
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
            // S1 (2026-09-22): the `---` frontmatter fence is a GOVERNANCE-DOC convention — a
            // `.md` file always carries one (or is not a candidate, unchanged from before). Every
            // other declared glob (`.py`/`.json`/`.yaml`/`.rs`/`.feature`/…) was never expected to
            // carry one: widening `first_screen_globs` to reach them only matters if they can
            // become candidates AT ALL, so a non-`.md` file with no fence becomes a BARE
            // candidate instead of being silently excluded the way the fence requirement always
            // excluded it — title = filename, no declared tags/description/content_class, and its
            // whole bounded read stands in for both the "declared" and "body" text a frontmatter'd
            // `.md` splits in two.
            let is_markdown = file_name.to_ascii_lowercase().ends_with(".md");
            let has_fence = head.starts_with("---\n");
            let (title, description, actual_tags, content_class, text, header_bytes);
            if has_fence || is_markdown {
                let Some(header) = frontmatter_header(&head, head.len(), meta.len() as usize)
                else {
                    // A document that opens `---` and whose boundary this read could not prove is
                    // a candidate we could not read. A `.md` file with no frontmatter at all is
                    // not a candidate in the first place, and is passed over as it always was.
                    if head.starts_with("---\n") {
                        // Only a read that actually hit its cap can be blamed on the budget;
                        // anything shorter simply has no closing delimiter.
                        budget_cut |= head_len >= metadata_bytes;
                        unreadable.push(relative);
                    }
                    continue;
                };
                if truncated {
                    partially_scanned += 1;
                }
                // The body window is the whole read, trimmed the same way; the header slice above
                // is a prefix of it, so `header.len()` still indexes correctly.
                let whole = if head_len == data.len() {
                    head
                } else {
                    match trim_to_character_boundary(&data) {
                        Some(whole) => whole,
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
                // Field lookup by iteration rather than `Mapping::get`, so the reader does not
                // depend on which `Index` impls a given serde_yaml minor happens to expose.
                let field = |key: &str| {
                    mapping
                        .iter()
                        .find(|(k, _)| k.as_str() == Some(key))
                        .map(|(_, v)| v)
                };
                let tags_field = match field("tags") {
                    None => Some(Vec::new()),
                    Some(serde_yaml::Value::Sequence(items)) => items
                        .iter()
                        .map(|item| item.as_str().map(str::to_string))
                        .collect::<Option<Vec<String>>>(),
                    Some(_) => None,
                };
                // A `tags:` field that is not a list of strings is not category membership; the
                // row is skipped rather than coerced, exactly as the oracle skips it.
                let Some(tags_field) = tags_field else {
                    continue;
                };
                title = field("title")
                    .and_then(|v| v.as_str().map(str::to_string))
                    .unwrap_or_else(|| file_name.clone());
                description = field("description")
                    .and_then(|v| v.as_str().map(str::to_string))
                    .unwrap_or_default();
                // The content floor's own field (governed-discovery station 1.2, `render.rs`
                // `RenderFloor::declared`'s `unfilterable`): a correction/counter-evidence/
                // accountability/own-community candidate is read here, from declared frontmatter
                // only, so the render layer can keep it past a narrow lens's `choice_count`
                // without re-reading the source. Absent frontmatter is honest absence (`None`),
                // never a guessed default — an ordinary document with no opinion on its own class
                // is not silently classed as unfilterable.
                content_class = field("content_class").and_then(|v| v.as_str().map(str::to_string));
                actual_tags = tags_field;
                header_bytes = header.as_bytes().to_vec();
                text = whole;
            } else {
                // Bare candidate: no frontmatter to declare a title/tags/description, so the WHOLE
                // bounded read stands in as its own "declared and body" text — a code/config
                // file's filename is the only metadata it has.
                if truncated {
                    partially_scanned += 1;
                }
                let whole = if head_len == data.len() {
                    head
                } else {
                    match trim_to_character_boundary(&data) {
                        Some(whole) => whole,
                        None => head,
                    }
                };
                title = file_name.clone();
                description = String::new();
                actual_tags = Vec::new();
                content_class = None;
                header_bytes = Vec::new();
                text = whole;
            }
            // The bytes after the closing delimiter were ALREADY READ under `metadata_bytes`.
            // Matching them costs no second read and no wider budget, and it is where an agent's
            // own words actually live: a fresh reader asking "re-mine", "marker", "stamp" on
            // 2026-09-11 got zero candidates over a corpus whose bodies say all three, because
            // discovery only ever looked at declared metadata. A bare candidate has no header to
            // skip past — its whole bounded read stands in as the "body" too.
            let body_prefix = if header_bytes.is_empty() {
                text.as_str()
            } else {
                text.get(header_bytes.len()..)
                    .and_then(|rest| rest.split_once('\n'))
                    .map(|(_, body)| body)
                    .unwrap_or_default()
            };
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
                    // A phrase also matches across identifier separators: "concurrent heavy"
                    // finds `max_concurrent_heavy` and `concurrent-heavy`, because a reader asks
                    // in words and configuration names things in identifiers.
                    let needle = query.to_lowercase();
                    let spaced = identifier_words(&needle);
                    !declared.contains(&needle)
                        && !body.contains(&needle)
                        && !identifier_words(&declared).contains(&spaced)
                        && !identifier_words(&body).contains(&spaced)
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
                // S1 (2026-09-22, `discovery.stemming`): a term also matches via its STEM (see
                // `stem` below this function) — a candidate carrying only the base form of an
                // inflected term the reader typed (`habits` -> `habit`) is not a miss.
                let stemmed = stem(&needle);
                let hits = |haystack: &str| {
                    haystack.contains(&needle) || (stemmed != needle && haystack.contains(stemmed))
                };
                let in_path = hits(&relative.to_lowercase());
                let in_title = hits(&title.to_lowercase());
                let in_description = hits(&description.to_lowercase());
                let in_tag = actual_tags.iter().any(|t| hits(&t.to_lowercase()));
                let in_body = hits(&body);
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
                // already inside the bytes this read covered. The exact needle is counted first;
                // its stem is counted only when the exact needle never occurred at all, so a
                // literal hit is never inflated by also counting its own stem's substrings.
                let mut occurrences =
                    declared.matches(&needle).count() + body.matches(&needle).count();
                if occurrences == 0 && stemmed != needle {
                    occurrences = declared.matches(stemmed).count() + body.matches(stemmed).count();
                }
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
            // Fingerprint the same bytes the ORIGINAL frontmatter-only reader always did — its
            // declared header — and, for a bare candidate with no header at all, its whole bounded
            // read stands in (never a slice at an arbitrary byte offset, which risks landing
            // mid-character in a string this function did not itself prove a boundary for).
            let fingerprint_bytes: Vec<u8> = if header_bytes.is_empty() {
                text.as_bytes().to_vec()
            } else {
                header_bytes.clone()
            };
            candidates.push(json!({
                "path": relative,
                "title": title,
                "tags": actual_tags,
                "content_class": content_class,
                "term_hits": declared_hits + body_only_hits,
                "declared_hits": declared_hits,
                "match": kinds.iter().collect::<Vec<_>>(),
                "matched_terms": matched_terms,
                "match_scope": "declared frontmatter and the bounded body prefix this read already \
                                covered; a body hit beyond the metadata budget is not visible here",
                "metadata_fingerprint": hex(&Sha256::digest(&fingerprint_bytes)),
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
const STOPWORDS: [&str; 49] = [
    "about", "after", "again", "against", "because", "before", "being", "between", "could", "does",
    "doing", "down", "from", "have", "here", "how", "into", "just", "like", "make", "more", "most",
    "much", "must", "only", "other", "over", "same", "should", "some", "such", "than", "that",
    "their", "them", "then", "there", "they", "this", "were", "what", "when", "where", "which",
    "while", "will", "with", "would", "your",
];

/// The contract's declared vocabulary of short (<4 char) tokens worth keeping as terms
/// (`discovery.short_terms`) — undeclared short tokens stay noise (a bare `ci` or `pr` loose in
/// ordinary prose would otherwise flood matching). Absent on an older contract reads as empty, so
/// a pre-S1 contract keeps today's "drop every short token" behaviour exactly.
fn short_terms(contract: &Contract) -> BTreeSet<String> {
    contract
        .value
        .pointer("/discovery/short_terms")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_ascii_lowercase)
                .collect()
        })
        .unwrap_or_default()
}

/// The question's distinctive terms: what an area match is made of.
///
/// A token under four characters is kept only when the contract's `discovery.short_terms`
/// declares it (`fn short_terms`) — that is what lets `top`/`red` survive tokenizing "Which habit
/// is top red right now" without every three-letter word in ordinary prose becoming a term. Two
/// ADJACENT declared-short tokens in the source text (`top red`) also mint the two-word phrase as
/// an additional term, matched as a phrase by the ordinary substring matching every other term
/// already uses — no separate phrase-matching code path.
pub(super) fn question_terms(contract: &Contract, need: &str) -> Vec<String> {
    let allowed_short = short_terms(contract);
    let is_kept_short = |term: &str| -> bool {
        !term.is_empty()
            && term.len() < 4
            && allowed_short.contains(term)
            && !STOPWORDS.contains(&term)
    };
    let raw_tokens: Vec<String> = need
        .split(|c: char| {
            !(c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '/' || c == '.')
        })
        .map(|raw| {
            raw.trim_matches(|c| c == '.' || c == '/')
                .to_ascii_lowercase()
        })
        .collect();

    let mut seen: Vec<String> = Vec::new();
    for (index, term) in raw_tokens.iter().enumerate() {
        if term.is_empty() || STOPWORDS.contains(&term.as_str()) || seen.contains(term) {
            continue;
        }
        let keep = term.len() >= 4 || allowed_short.contains(term.as_str());
        if keep {
            seen.push(term.clone());
            if seen.len() >= 12 {
                break;
            }
        }
        if is_kept_short(term) {
            if let Some(next) = raw_tokens.get(index + 1) {
                if is_kept_short(next) {
                    let phrase = format!("{term} {next}");
                    if !seen.contains(&phrase) {
                        seen.push(phrase);
                        if seen.len() >= 12 {
                            break;
                        }
                    }
                }
            }
        }
    }
    seen
}

/// `DELTA 2026-09-11 (…)` / `GREEN 2026-09-11 (…)` / `RED WRITTEN 2026-09-11 (…)` / `RED
/// 2026-09-11 (…)` → `2026-09-11`. `None` for a first line that carries no such leading token —
/// rendered as an honest omission rather than a fabricated date.
pub(super) fn parse_delta_date(line: &str) -> Option<String> {
    let rest = line
        .strip_prefix("DELTA ")
        .or_else(|| line.strip_prefix("GREEN "))
        .or_else(|| line.strip_prefix("RED WRITTEN "))
        .or_else(|| line.strip_prefix("RED "))?;
    let token = rest.split_whitespace().next()?;
    let bytes = token.as_bytes();
    let digit = |b: u8| b.is_ascii_digit();
    let valid = bytes.len() == 10
        && bytes[..4].iter().copied().all(digit)
        && bytes[4] == b'-'
        && bytes[5..7].iter().copied().all(digit)
        && bytes[7] == b'-'
        && bytes[8..10].iter().copied().all(digit);
    valid.then(|| token.to_string())
}

/// The entry carrying the latest ledger date (`date_of` → `YYYY-MM-DD`, `None` for an undated
/// entry). Atoms are written in both directions — prepended (newest-first) or appended
/// (oldest-first) — so a tie on that date is broken by the ledger's own direction: when the FIRST
/// dated entry already holds the latest date the ledger reads newest-first and the first tied
/// entry wins; otherwise it reads oldest-first and the last tied entry wins.
pub(super) fn newest_by_date<T>(
    entries: &[T],
    date_of: impl Fn(&T) -> Option<String>,
) -> Option<&T> {
    let latest = entries.iter().filter_map(&date_of).max()?;
    let newest_first = entries.iter().find_map(&date_of).as_ref() == Some(&latest);
    let mut tied = entries
        .iter()
        .filter(|entry| date_of(entry).as_ref() == Some(&latest));
    if newest_first {
        tied.next()
    } else {
        tied.last()
    }
}

/// One habit as the first screen needs it: what it promises, whether it holds, and what moved last.
fn habit_row(id: &str, status: &str, invariant: &str, evidence: &str, score: usize) -> Value {
    // "What moved last" is the entry with the LATEST date, not simply the first paragraph: atoms
    // are appended at the bottom as often as prepended at the top. Among entries sharing that
    // date the ledger's own direction breaks the tie (`newest_by_date`); an undated ledger falls
    // back to its first paragraph.
    let paragraphs: Vec<String> = evidence
        .split("\n\n")
        .map(one_line)
        .filter(|paragraph| !paragraph.is_empty())
        .collect();
    let newest =
        newest_by_date(&paragraphs, |p| parse_delta_date(p)).or_else(|| paragraphs.first());
    let delta = newest.map(|p| clip(p, 240)).unwrap_or_default();
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
/// The globs a `search` spans: an explicit `--name` wins; a TAG filter stays on markdown, where
/// frontmatter tags live; otherwise the same declared file types `open` discovers — so a question
/// about a JSON or YAML source is not silently confined to markdown (fresh reader, 2026-09-22:
/// `search --query "concurrent heavy cargo"` could not see `pool-policy.json` at all).
pub(super) fn search_globs(args: &Args, contract: &Contract) -> Vec<String> {
    if args.name_explicit || !args.tags.is_empty() {
        vec![args.name.clone()]
    } else {
        first_screen_globs(contract)
    }
}

/// The declared globs `first_screen` discovers across (`discovery.first_screen_globs`) — falls
/// back to `["*.md"]`, today's only glob, for an older contract that does not declare the key, so
/// the key is purely additive and never silently widens an unaware caller.
pub(super) fn first_screen_globs(contract: &Contract) -> Vec<String> {
    contract
        .value
        .pointer("/discovery/first_screen_globs")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|globs| !globs.is_empty())
        .unwrap_or_else(|| vec!["*.md".to_string()])
}

pub(super) fn first_screen(
    args: &Args,
    contract: &Contract,
    state: &Value,
    usage: &mut Value,
) -> FlowResult<Option<Value>> {
    if !args.need_explicit {
        return Ok(None);
    }
    let terms = question_terms(contract, args.need.trim());
    if terms.is_empty() {
        return Ok(None);
    }
    let scope = state["scope"].as_str().unwrap_or(".");
    let (habits, mut unresolved) = matching_habits(&args.root, contract, &terms, usage)?;
    let area = focus_area(&args.root, contract, scope, &terms);

    // S1 (2026-09-22): a whole-tree focused question with no directory-shaped area gets a
    // METADATA-FIRST screen from the AUTHORITY SET — root `CLAUDE.md` plus each matched habit's
    // own atom and the existing repo paths its `checks:`/`refs:` entries name — before any
    // repository body scan. Gated on `scope == "."` specifically (not merely `area.is_none()`,
    // which a non-root scope that simply matched nothing also produces) — a scoped-but-unmatched
    // question is unaffected. Falls through when the set is empty or answers nothing, so the
    // whole-scope ceremony door still renders exactly as it did before this station.
    if area.is_none() && scope == "." {
        if let Some(screen) = root_authority_screen(args, contract, &terms, &habits, usage)? {
            return Ok(Some(screen));
        }
    }

    if area.is_none() && habits.is_empty() {
        return Ok(None);
    }
    let mut candidates: Vec<Value> = Vec::new();
    let mut omissions: Vec<String> = Vec::new();
    let mut ranking = Value::Null;
    let mut continuation = Value::Null;
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
            &first_screen_globs(contract),
        )?;
        let charged = found["usage"].take();
        add_usage(usage, &charged);
        candidates = found["candidates"].as_array().cloned().unwrap_or_default();
        // The habit register is a GENERATED projection (never an authority to edit or cite) and its
        // rows already render as this screen's habits block; once the globs admit YAML it would
        // otherwise compete as a source with the atoms it is projected from.
        candidates.retain(|candidate| candidate["path"].as_str() != Some(HABITS_REL));
        // Each candidate is located, not merely named: the section its terms land in, and a read
        // range bounded to one excerpt. The outline scan is charged to the scan counters.
        for candidate in candidates.iter_mut() {
            let Some(path) = candidate["path"].as_str().map(str::to_string) else {
                continue;
            };
            if let Some(section) = best_section(args, contract, &path, &terms) {
                add_usage(usage, &section["usage"]);
                candidate["best_section"] = section_link(&section);
            }
        }
        // Proximity: a candidate whose ONE passage carries more of the question's distinct terms
        // together is nearer the authority than one that merely mentions them all somewhere.
        // Distinct terms (not the per-file weighted score) is the cross-file comparable measure;
        // discovery's own order breaks ties, so the sort is stable.
        let together = |candidate: &Value| {
            candidate["best_section"]["hits"]
                .as_object()
                .map_or(0, |hits| hits.len())
        };
        candidates.sort_by_key(|candidate| std::cmp::Reverse(together(candidate)));
        ranking = found["selection"].take();
        if let Some(selection) = ranking.as_str() {
            ranking = json!(format!(
                "{selection}; then re-ranked by the distinct question terms one located passage carries together"
            ));
        }
        for message in found["omissions"].as_array().cloned().unwrap_or_default() {
            omissions.push(message.as_str().unwrap_or_default().to_string());
        }
        // S1 (2026-09-22): a budget cut is never left as a dead end. When the traversal's own
        // exhaustion line fires, the view names a CONCRETE narrowed continuation — the densest
        // subdirectory this window actually reached — rather than raising any budget.
        const EXHAUSTED: &str =
            "discovery budget exhausted; narrow the scope or continue with a named question";
        for message in found["unresolved"].as_array().cloned().unwrap_or_default() {
            let text = message.as_str().unwrap_or_default().to_string();
            if text == EXHAUSTED {
                if let Some(dir) = densest_subdir(&found["groups"]) {
                    unresolved.push(format!(
                        "{text}; the densest subdirectory this window reached was {dir} — \
                         continue narrowed there"
                    ));
                    continuation = action(
                        args,
                        &format!("Continue narrowed in {dir}"),
                        "open",
                        &[("scope", json!(dir)), ("need", json!(args.need.clone()))],
                    );
                    continue;
                }
            }
            unresolved.push(text);
        }
    } else {
        unresolved.push(
            "no directory scope in this question; name --scope <dir> for ranked candidate sources"
                .into(),
        );
    }
    let mut screen = json!({
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
    });
    if !continuation.is_null() {
        screen["continuation"] = continuation;
    }
    Ok(Some(screen))
}

/// The root-scope AUTHORITY SET a whole-tree focused question is screened against BEFORE any
/// repository body scan: the root gospel `CLAUDE.md`, plus — for each matched habit — its own
/// atom and the first existing repo path named in each of its `checks:`/`refs:` frontmatter
/// entries. Ranked by where the question's terms land inside each document's own outline (the
/// same per-section scoring [`best_section`] already does for an area's candidates) rather than
/// by walking a directory. `Ok(None)` when the set is empty or none of its documents' outlines
/// carry a hit, so the caller falls through to the whole-scope ceremony door exactly as before
/// this station.
fn root_authority_screen(
    args: &Args,
    contract: &Contract,
    terms: &[String],
    habits: &[Value],
    usage: &mut Value,
) -> FlowResult<Option<Value>> {
    let mut paths: Vec<String> = Vec::new();
    let roots = contract.source_roots();
    let mut push_if_file = |candidate: String| {
        if paths.contains(&candidate) {
            return;
        }
        // Every authority-set path passes the same declared-scope gate a question's read does:
        // a candidate the reader could not then `read` would be a choice the entry cannot keep.
        // Root `CLAUDE.md` and the root `.epr-meta/` governance home are DECLARED source roots
        // (contract v13) rather than fixed inputs read around the gate.
        if contained(&args.root, &candidate, &roots)
            .ok()
            .filter(|p| p.is_file())
            .is_some()
        {
            paths.push(candidate);
        }
    };
    push_if_file("CLAUDE.md".to_string());

    let atom_budget = contract.limit_usize("habit_register_bytes");
    for habit in habits {
        let Some(id) = habit["id"].as_str() else {
            continue;
        };
        let Some(atom) = find_habit_atom(&args.root, contract, id, atom_budget) else {
            continue;
        };
        let rel = rel_to_root(&args.root, &atom);
        push_if_file(rel);
        let mut data = Vec::new();
        if File::open(&atom)
            .and_then(|f| {
                f.take(atom_budget as u64 + 1)
                    .read_to_end(&mut data)
                    .map(|_| ())
            })
            .is_err()
            || data.len() > atom_budget
        {
            continue;
        }
        add_usage(usage, &json!({"habit_atom_bytes": data.len()}));
        let text = String::from_utf8_lossy(&data).into_owned();
        let Some(front) = frontmatter_value(&text) else {
            continue;
        };
        for key in ["checks", "refs"] {
            for entry in yaml_string_list(&front, key) {
                if let Some(token) = first_path_token(&entry) {
                    // Arbitrary NAMED repo evidence (feature files, scripts, plans) goes through
                    // the declared `source_roots` gate like any other question evidence — unlike
                    // `CLAUDE.md` and the atom itself, these are not this executor's own
                    // well-known cross-cutting inputs.
                    push_if_file(token);
                }
            }
        }
    }
    if paths.is_empty() {
        return Ok(None);
    }
    // The authority set spends the SAME declared scan budget every other discovery path does:
    // one running total across its members, at most `scan_files` members, and the returned list
    // cut to `search_results` — never a second, uncounted budget just because it is small.
    let (scan_limit, file_limit) = (
        contract.limit_usize("scan_bytes"),
        contract.limit_usize("scan_files").max(1),
    );
    let mut omissions: Vec<String> = Vec::new();
    if paths.len() > file_limit {
        omissions.push(format!(
            "authority set named {} paths; only the first {file_limit} (limits.scan_files) were located",
            paths.len()
        ));
        paths.truncate(file_limit);
    }
    let mut scanned = 0usize;
    let mut candidates: Vec<Value> = Vec::new();
    for path in &paths {
        if scanned >= scan_limit {
            omissions.push(format!(
                "authority set stopped at limits.scan_bytes ({scan_limit}); later members were not located"
            ));
            break;
        }
        if let Some(section) = best_section(args, contract, path, terms) {
            scanned += section["usage"]["scan_bytes"].as_u64().unwrap_or(0) as usize;
            add_usage(usage, &section["usage"]);
            let title = Path::new(path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.clone());
            candidates.push(json!({
                "path": path,
                "title": title,
                "term_hits": section["hit_total"],
                "best_section": section_link(&section),
            }));
        }
    }
    if candidates.is_empty() {
        return Ok(None);
    }
    candidates.sort_by(|a, b| {
        b["term_hits"]
            .as_u64()
            .cmp(&a["term_hits"].as_u64())
            .then_with(|| a["path"].as_str().cmp(&b["path"].as_str()))
    });
    let result_limit = contract.limit_usize("search_results").max(1);
    if candidates.len() > result_limit {
        omissions.push(format!(
            "{} authority candidates located; the first {result_limit} (limits.search_results) are shown",
            candidates.len()
        ));
        candidates.truncate(result_limit);
    }
    Ok(Some(json!({
        "area": ".",
        "terms": terms,
        "habits": habits,
        "candidates": candidates,
        "provider": "local",
        "ranking": "authority set (metadata-first), no repository body scan",
        "authority": "Candidate discovery over declared metadata. Membership and rank establish \
                      neither authority nor currency; read the passage.",
        "omissions": omissions,
        "unresolved": Vec::<String>::new(),
    })))
}

/// The count each grouped directory reached inside a `discover_scored` view — used only to name
/// the densest subdirectory a budget-cut traversal actually reached, never to imply anything
/// about the corpus outside it. `None` when there is no narrower group to offer (an empty set, or
/// the traversal never left the scope's own root).
fn densest_subdir(groups: &Value) -> Option<String> {
    let map = groups.as_object()?;
    map.iter()
        .filter(|(key, _)| key.as_str() != ".")
        .max_by(|a, b| {
            a.1.as_u64()
                .unwrap_or(0)
                .cmp(&b.1.as_u64().unwrap_or(0))
                .then_with(|| b.0.cmp(a.0))
        })
        .map(|(key, _)| key.clone())
}

/// The full YAML frontmatter of a document, or `None` when it has no closed `---` fence. Unlike
/// [`frontmatter_header`]'s bounded-read boundary proof, this runs once against a whole (already
/// small, already budget-read) habit atom, because a `checks:`/`refs:` list can run to any length
/// before the caller knows which entry names a path.
fn frontmatter_value(text: &str) -> Option<serde_yaml::Value> {
    let mut lines = text.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    let mut buffer = String::new();
    for line in lines {
        if line.trim() == "---" {
            return serde_yaml::from_str(&buffer).ok();
        }
        buffer.push_str(line);
        buffer.push('\n');
    }
    None
}

/// The string sequence at `key` in a parsed frontmatter value, or empty when absent, not a
/// sequence, or not made entirely of strings.
fn yaml_string_list(value: &serde_yaml::Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(serde_yaml::Value::as_sequence)
        .map(|seq| {
            seq.iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// The first token in `text` that LOOKS like a repository path — contains a `/` once surrounding
/// punctuation is trimmed, and is not a bare `@concern:` tag or a lone separator. A candidate,
/// never a proof: [`root_authority_screen`] still confirms it exists under the declared source
/// scope before trusting it — a token like `sample/judge` (prose, not a path) is filtered there by
/// that existence check, not by this lexical scan.
fn first_path_token(text: &str) -> Option<String> {
    for raw in text.split_whitespace() {
        let opened = raw.trim_start_matches(['(', '"', '\'']);
        let cleaned = opened.trim_end_matches(|c: char| {
            matches!(c, ')' | ',' | ':' | ';' | '"' | '\'' | '.' | '—')
        });
        if cleaned.is_empty()
            || cleaned == "/"
            || cleaned.starts_with('@')
            || !cleaned.contains('/')
        {
            continue;
        }
        return Some(cleaned.to_string());
    }
    None
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Bootstrap — the session's own top red as its intent (governed-discovery station 2.1)
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// Where the flows sidecar is projected. Read, never written, by this executor — the second of the
/// two well-known inputs a `--purpose bootstrap` open declares.
pub const FLOWS_REL: &str = ".eprfs/status/flows.jsonl";

/// A bounded whole-file read of one FIXED, well-known register — never gated by `source_roots`
/// (those bound what a QUESTION may traverse; `habits.yaml` and `flows.jsonl` are cross-cutting
/// paths this executor already knows by name, the same way `HABITS_REL` is read unconditionally
/// above) but still confined under the repository root. Whole-file-or-nothing, never the bounded
/// EXCERPT reader `receipts.rs` uses for a source passage: the bytes returned here are hashed into
/// a content-address (`BlobCid::compute_raw`), and a CID over a partial read would misrepresent
/// the file it claims to identify — there is no excerpt-CID concept anywhere else in this
/// executor either.
///
/// Capped by `habit_register_bytes` for BOTH files (`usage_key` only labels which counter the
/// bytes are charged to, fix round 1 finding 3 — the budget itself is intentionally shared: it is
/// the only byte limit either small register has, and the contract's bytes must not move for this
/// fix). Absence, an escape, or a budget overrun is **named** in `omissions` rather than silently
/// dropped — ruling: "missing files are named in omissions, never silently skipped." This is
/// `flows.jsonl`'s reader; `habits.yaml` is fail-closed instead (fix round 1 finding 2) and reads
/// through [`read_bootstrap_habits`], never this function.
fn read_bootstrap_input(
    root: &Path,
    contract: &Contract,
    rel: &str,
    usage_key: &str,
    usage: &mut Value,
    omissions: &mut Vec<String>,
) -> FlowResult<Option<Vec<u8>>> {
    let path = match confine_under(root, &root.join(rel)) {
        Ok(path) => path,
        Err(_) => {
            omissions.push(format!(
                "{rel} escapes the repository; omitted from projection inputs"
            ));
            return Ok(None);
        }
    };
    if !path.is_file() {
        omissions.push(format!("{rel} is absent; omitted from projection inputs"));
        return Ok(None);
    }
    let budget = contract.limit_usize("habit_register_bytes");
    let mut data = Vec::new();
    File::open(&path)
        .and_then(|file| {
            file.take(budget as u64 + 1)
                .read_to_end(&mut data)
                .map(|_| ())
        })
        .map_err(|source| FlowError::Read { path, source })?;
    if data.len() > budget {
        omissions.push(format!(
            "{rel} exceeds its declared budget; omitted from projection inputs"
        ));
        return Ok(None);
    }
    add_usage(usage, &json!({usage_key: data.len()}));
    Ok(Some(data))
}

/// `genesis/manifests/habits.yaml`, fail-closed (fix round 1, finding 2): unlike
/// [`read_bootstrap_input`], there is no "proceed without it" for the register itself — an absent,
/// unreadable, or over-budget register REFUSES the whole `open`, naming the path and the fault, so
/// a caller re-projects it rather than being silently oriented by a register this executor could
/// not actually read. A register that reads fine but genuinely parses to no red habit is a
/// DIFFERENT case (`top_red_habit` returning `Ok(None)`) and is not refused — see
/// [`bootstrap_projection`].
fn read_bootstrap_habits(
    root: &Path,
    contract: &Contract,
    usage: &mut Value,
) -> FlowResult<Vec<u8>> {
    let path = confine_under(root, &root.join(HABITS_REL))
        .map_err(|_| bootstrap_register_refusal("escapes the repository"))?;
    if !path.is_file() {
        return Err(bootstrap_register_refusal("absent"));
    }
    let budget = contract.limit_usize("habit_register_bytes");
    let mut data = Vec::new();
    File::open(&path)
        .and_then(|file| {
            file.take(budget as u64 + 1)
                .read_to_end(&mut data)
                .map(|_| ())
        })
        .map_err(|source| {
            refused(format!(
                "bootstrap cannot read the habit register: {HABITS_REL}: {source}"
            ))
        })?;
    if data.len() > budget {
        return Err(bootstrap_register_refusal("exceeds its declared budget"));
    }
    add_usage(usage, &json!({"habit_register_bytes": data.len()}));
    Ok(data)
}

/// One `bootstrap cannot read the habit register` refusal, named the same way whichever reason
/// fired — `remedy_for` (`refusal.rs`) matches on this exact prefix to name
/// `.claude/scripts/habits-project.py` as the `next:` line.
fn bootstrap_register_refusal(reason: &str) -> FlowError {
    refused(format!(
        "bootstrap cannot read the habit register: {HABITS_REL}: {reason}"
    ))
}

/// The register's own first check line for one habit, from its `checks:` sequence — the FIRST
/// entry only, exactly as declared order names it. `None` when the habit carries no checks.
fn first_check(habit: &serde_yaml::Value) -> String {
    habit
        .get("checks")
        .and_then(serde_yaml::Value::as_sequence)
        .and_then(|checks| checks.first())
        .and_then(serde_yaml::Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// One habit as `--purpose bootstrap` needs it: which one, its own first check line, and — when
/// the register row itself carries one (fix round 2, finding 1: `declared:` or `atom:`, no
/// generated `habits.yaml` row does today, but a hand-authored fixture or a future census may) —
/// its atom's own declared path, so [`bootstrap_projection`] can skip the repo-wide search
/// entirely when the register already names it. A named struct rather than a bare tuple so a
/// caller reads `.id`/`.check` instead of `.0`/`.1`, and so [`bootstrap_projection`]'s return type
/// stays simple enough for clippy's `type_complexity` lint without an `#[allow]`.
pub(super) struct TopRedHabit {
    pub(super) id: String,
    pub(super) check: String,
    pub(super) declared: Option<String>,
}

/// The register's own top red — never a term match. First habit in DECLARED order with
/// `active: true` and `status: red`; else the first with `status: red`; else `Ok(None)` for a
/// register that parses fine and genuinely carries no red habit (an honest orient, not a fault).
/// `Err` only when the bytes do not parse as the register's own shape at all — a malformed
/// document, or one with no `habits:` sequence — which [`bootstrap_projection`] turns into a
/// fail-closed refusal (fix round 1, finding 2) rather than the same "no red habit; orient" an
/// all-green register earns honestly. A full parse (not the hand-rolled line scan
/// `matching_habits` uses to stay cheap under a term filter) because this reads the whole small
/// register exactly once and needs its real structure, including the inline-array `checks:` shape
/// a fixture may use as well as the block-folded shape the generated register actually carries.
fn top_red_habit(text: &str) -> Result<Option<TopRedHabit>, String> {
    let doc: serde_yaml::Value = serde_yaml::from_str(text).map_err(|error| error.to_string())?;
    let habits = doc
        .get("habits")
        .and_then(serde_yaml::Value::as_sequence)
        .ok_or_else(|| "no `habits:` sequence".to_string())?;
    let mut first_red: Option<TopRedHabit> = None;
    let mut first_active_red: Option<TopRedHabit> = None;
    for habit in habits {
        if habit.get("status").and_then(serde_yaml::Value::as_str) != Some("red") {
            continue;
        }
        let id = habit
            .get("id")
            .and_then(serde_yaml::Value::as_str)
            .unwrap_or_default()
            .to_string();
        let check = first_check(habit);
        let declared = habit
            .get("declared")
            .or_else(|| habit.get("atom"))
            .and_then(serde_yaml::Value::as_str)
            .map(str::to_string);
        if first_red.is_none() {
            first_red = Some(TopRedHabit {
                id: id.clone(),
                check: check.clone(),
                declared: declared.clone(),
            });
        }
        let active = habit
            .get("active")
            .and_then(serde_yaml::Value::as_bool)
            .unwrap_or(false);
        if active && first_active_red.is_none() {
            first_active_red = Some(TopRedHabit {
                id,
                check,
                declared,
            });
        }
    }
    Ok(first_active_red.or(first_red))
}

/// Directories a repo-wide atom search must never descend into — the same set
/// `discover`/`discover_scored` already read from the contract's own `/discovery/
/// exclude_directories` (version-control, dependency and build trees; `worktrees` is literally
/// another checkout). Reused rather than re-declared so the two traversals can never disagree
/// about what is "irrelevant tree," and a repo that adds a new one only has to say so once.
pub(super) fn atom_search_excluded(contract: &Contract) -> BTreeSet<String> {
    contract
        .value
        .pointer("/discovery/exclude_directories")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// `.epr-meta/<id>.habit.md`, found anywhere under `root` — a habit atom's directory is not
/// knowable from the register alone (fix round 2, finding 1): `habits.yaml` carries no
/// `declared:`/`atom:` field for any row today (`.claude/scripts/_lib/epr_habits.py`'s
/// `project_habit` deliberately drops `_source`/`_dir` — the census's own reference to the file
/// it read — from every projected row; `TopRedHabit::declared` above is the forward-compatible
/// path for a register that one day does carry it, but this is the operative one). The walk is
/// small in practice (a few dozen `.epr-meta` directories across the whole monorepo) but still
/// BOUNDED — by `habit_register_bytes`, the same budget the register itself reads under, applied
/// here to the running total of scanned directory-ENTRY NAME bytes (never file content, and never
/// the atom's own bytes once found — that read is a separate, ordinary bounded read through
/// [`read_bootstrap_input`]). `None` on an exhausted budget or no match; never a refusal — a habit
/// atom that cannot be located is supplementary evidence going missing, not the register itself
/// failing to read (contrast [`read_bootstrap_habits`], which does refuse).
pub(super) fn find_habit_atom(
    root: &Path,
    contract: &Contract,
    id: &str,
    budget: usize,
) -> Option<PathBuf> {
    let excluded = atom_search_excluded(contract);
    let target = format!("{id}.habit.md");
    let mut stack = vec![root.to_path_buf()];
    let mut scanned_bytes = 0usize;
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut children: Vec<_> = entries.flatten().collect();
        children.sort_by_key(std::fs::DirEntry::file_name);
        for entry in children {
            let name = entry.file_name().to_string_lossy().into_owned();
            scanned_bytes += name.len();
            if scanned_bytes > budget {
                return None;
            }
            let Ok(meta) = entry.path().symlink_metadata() else {
                continue;
            };
            if meta.file_type().is_symlink() || !meta.is_dir() || excluded.contains(&name) {
                continue;
            }
            if name == ".epr-meta" {
                let candidate = entry.path().join(&target);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
            stack.push(entry.path());
        }
    }
    None
}

/// The newest `DELTA`/`GREEN`/`RED WRITTEN` paragraph in a habit atom's BODY — the markdown below
/// its closing frontmatter fence — as `(first line clipped to 160, "START:END" 1-based inclusive
/// line range within the whole file)`. The FIRST paragraph, since atoms are written newest-first
/// (the same convention `habit_row`'s evidence-first-paragraph extraction already relies on for
/// the register's OWN evidence field — which is this exact body text, verbatim: `census()` in
/// `.claude/scripts/_lib/epr_habits.py` promotes an atom's body straight into the projected row's
/// `evidence:`). Read the atom directly rather than trusting the register's copy here because the
/// LINE RANGE — needed for a `read --lines` command to point at — only exists against the real
/// file. `None` for an atom with no frontmatter fence at all, or an empty body — rendered as
/// `last delta: none recorded`, never fabricated.
fn atom_last_delta(text: &str) -> Option<(String, String)> {
    let mut lines = text.lines().enumerate();
    if lines.next().map(|(_, line)| line.trim()) != Some("---") {
        return None;
    }
    let body_start = lines
        .find(|(_, line)| line.trim() == "---")
        .map(|(index, _)| index + 2)?;
    let all_lines: Vec<&str> = text.lines().collect();
    let (mut start, mut end) = (None, body_start);
    for (offset, line) in all_lines.iter().enumerate().skip(body_start - 1) {
        let line_no = offset + 1;
        if line.trim().is_empty() {
            if start.is_some() {
                break;
            }
            continue;
        }
        start.get_or_insert(line_no);
        end = line_no;
    }
    let start = start?;
    Some((clip(all_lines[start - 1], 160), format!("{start}:{end}")))
}

/// The first `genesis/a2o/features/….feature` token a check names, or `None` when it names no
/// feature file — fix round 2, finding 2's `source` Linked choice. A plain substring scan (this
/// repo's a2o checks always spell the path this way) rather than a regex dependency for one fixed
/// prefix.
pub(super) fn first_feature_path(check: &str) -> Option<String> {
    const PREFIX: &str = "genesis/a2o/features/";
    let start = check.find(PREFIX)?;
    let rest = &check[start..];
    let end = rest.find(".feature")? + ".feature".len();
    Some(rest[..end].to_string())
}

/// `--purpose bootstrap`'s whole contribution: the session's [`TopRedHabit`], its habit atom's
/// path and last delta (fix round 2, finding 1), the `ProjectionRequest`-shaped `inputs` (raw CIDs
/// of the exact bytes read, `{path, cid}`), and every omission along the way.
pub(super) struct BootstrapProjection {
    pub(super) top_red: Option<TopRedHabit>,
    pub(super) atom: Option<String>,
    pub(super) last_delta: Option<(String, String)>,
    pub(super) inputs: Vec<Value>,
    pub(super) omissions: Vec<String>,
}

/// The `eprfs_agent::memory::ProjectionRequest` struct itself is not imported here — its
/// `collective: FileRef` field has no meaning for a bootstrap orientation (no collective-memory
/// request is being authored), so this returns [`BootstrapProjection`]'s fields (`purpose`/
/// `audience` are constants the caller adds) instead of a partially-populated struct.
///
/// Fail-closed on the register itself (fix round 1, finding 2): `habits.yaml` reads through
/// [`read_bootstrap_habits`], which refuses rather than returning `None`, and a register that
/// reads but does not PARSE as the register's own shape refuses here too — never orienting a
/// reader from a broken register the same way an honest all-green one would. `flows.jsonl` keeps
/// the earlier best-effort behaviour: absent or unreadable is named in `omissions`, never a
/// refusal, because it is supplementary evidence, not the register itself. The top red's atom and
/// last delta (fix round 2) are the SAME best-effort class as `flows.jsonl`: a habit atom the
/// search could not locate, or one whose body carries no delta paragraph, is named or left honestly
/// empty — never a reason to refuse an otherwise-readable register.
pub(super) fn bootstrap_projection(
    root: &Path,
    contract: &Contract,
    usage: &mut Value,
) -> FlowResult<BootstrapProjection> {
    let mut omissions: Vec<String> = Vec::new();
    let mut inputs: Vec<Value> = Vec::new();

    let habits_data = read_bootstrap_habits(root, contract, usage)?;
    let top_red = top_red_habit(&String::from_utf8_lossy(&habits_data)).map_err(|error| {
        refused(format!(
            "bootstrap cannot read the habit register: {HABITS_REL}: {error}"
        ))
    })?;
    inputs.push(json!({"path": HABITS_REL, "cid": BlobCid::compute_raw(&habits_data).to_string()}));

    if let Some(data) = read_bootstrap_input(
        root,
        contract,
        FLOWS_REL,
        "flows_register_bytes",
        usage,
        &mut omissions,
    )? {
        inputs.push(json!({"path": FLOWS_REL, "cid": BlobCid::compute_raw(&data).to_string()}));
    }

    let mut atom: Option<String> = None;
    let mut last_delta: Option<(String, String)> = None;
    if let Some(habit) = &top_red {
        let budget = contract.limit_usize("habit_register_bytes");
        // The register row's own atom path when it carries one (fix round 2, finding 1) — no
        // generated `habits.yaml` row does today, but skipping the search when it is already
        // named is the whole point of carrying it. Else, the bounded repo-wide search.
        let located = match &habit.declared {
            Some(declared) => confine_under(root, &root.join(declared))
                .ok()
                .filter(|path| path.is_file()),
            None => find_habit_atom(root, contract, &habit.id, budget),
        };
        match located {
            Some(path) => {
                let rel = rel_to_root(root, &path);
                if let Some(data) = read_bootstrap_input(
                    root,
                    contract,
                    &rel,
                    "habit_atom_bytes",
                    usage,
                    &mut omissions,
                )? {
                    last_delta = atom_last_delta(&String::from_utf8_lossy(&data));
                    inputs
                        .push(json!({"path": rel, "cid": BlobCid::compute_raw(&data).to_string()}));
                    atom = Some(rel);
                }
            }
            None => omissions.push(format!(
                "habit atom for {} was not located within budget",
                habit.id
            )),
        }
    }

    if top_red.is_none() {
        omissions.push("no red habit; orient".into());
    }

    Ok(BootstrapProjection {
        top_red,
        atom,
        last_delta,
        inputs,
        omissions,
    })
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

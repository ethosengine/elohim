//! SEMANTIC — the native semantic candidate route (governed-discovery station 4, task 4.4):
//! `search --provider semantic`, ranking over the fold `index.rs` keeps.
//!
//! The method is the declaration. The recipe's `ceremony.providers.<key>` names the
//! `IndexMeasure` (`measure`) and which embedder's store to read (`embedder`: `pinned`, the live
//! default, or `fixture` for tests) — never an environment switch. One answer:
//!
//! 1. the QUESTION TEXT (`--query`, else the session's `--need`) is embedded once, under the
//!    provider envelope (`EmbedBudget::query`) — not the lexical term list, which is the local
//!    route's shape;
//! 2. the cosine of it against every live (non-demoted) chunk vector in the store, each file
//!    keeping its best chunk ([`BestPerFile`]); cosine only — no standing, no behaviour signal;
//! 3. the top `limits.search_results` files inside the search scope and the declared source
//!    roots, ties broken by path — `search`'s window is `search_results` for every route (parity
//!    with the local route); the lens's `choice_count` cut belongs to the first screen, not to a
//!    search;
//! 4. each candidate prints `producer`, the measure CID as `method`, the `model`, the fold's lag at
//!    answer time and a `best_section` in the first screen's shape, so its linked `read` lands on
//!    the passage; the answer names the embedding `procedure` (the CID the model manifest pins)
//!    and the label the store was folded under (`built_by`), so a candidate list names the exact
//!    procedure behind its vectors.
//!
//! The vector scan reads a DERIVED store: it is reported in `usage` (`semantic_chunks_scanned`,
//! `semantic_query_ms`, `provider_seconds`, and `embedding_processes` when an embedding process
//! actually ran — answered or not), never charged to `source_bytes` or `scan_bytes`.
//! Locating the winner's heading reads the source file itself, and that outline read is charged
//! to the scan counters exactly as the first screen's is.
//!
//! Absence is honest, never an error: no fold, an embedder that cannot run, or a store built under
//! another method each answer with ONE `unresolved` line, no candidates and `ranking_known:
//! false` — through `retrieve()` and through the `Provider` seam alike, so a caller fusing routes
//! sees an absent route, never a known empty ranking. A stale fold answers
//! and says how stale (`fold N files behind`). The private chain never reaches this file: the
//! fold excluded it, and this route reads nothing but the store and the outline of a candidate.
use super::discovery::{offered_on_first_screen, question_terms};
use super::embedder::{EmbedBudget, ModelManifest, FIXTURE_FITNESS, MODEL_MANIFEST_REL};
use super::index::{Absent, EmbedderChoice, FoldReader, SemanticFold};
use super::passage::{outline_at, section_link};
use super::providers::lines;
use super::providers::{Provider, ProviderId, ProviderResult};
use super::*;

/// Every candidate's and every answer's `producer`.
pub(super) const PRODUCER: &str = "semantic";

/// The answer when the declared store does not exist.
pub(super) const NO_FOLD: &str = "semantic: no fold — run epr flow memory index fold";

/// The answer when the store's meta names another method than the declaration.
pub(super) const OTHER_METHOD: &str = "semantic: the fold was built under another method — refold";

const SELECTION: &str = "cosine of the embedded question against every live chunk in the declared \
                         fold; each file's best chunk; top limits.search_results files, ties by \
                         path; not authority";

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Ranking — cosine and best chunk per file, pure
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// The cosine of two vectors; 0 when either has no length or their widths differ.
pub(super) fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    let (mut dot, mut na, mut nb) = (0f32, 0f32, 0f32);
    for (x, y) in a.iter().zip(b) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na.sqrt() * nb.sqrt())
    }
}

/// A score at the 4 decimals every candidate prints (the lexical route prints its own at the same
/// precision).
pub(super) fn rounded(score: f64) -> f64 {
    (score * 10_000.0).round() / 10_000.0
}

/// One file's best chunk.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Hit {
    pub path: String,
    pub id: i64,
    pub score: f64,
}

/// Each file's best chunk, offered one chunk at a time. Within a file the higher score (a cosine
/// here, a negated `bm25()` on the lexical route) wins and
/// the earlier chunk (lower id) breaks a tie; across files [`BestPerFile::ranked`] orders by the
/// printed (4-decimal) score, then by path, so two runs over one store print one order.
#[derive(Default)]
pub(super) struct BestPerFile {
    best: BTreeMap<String, (f64, i64)>,
}

impl BestPerFile {
    pub(super) fn offer(&mut self, id: i64, path: &str, score: f64) {
        match self.best.get_mut(path) {
            Some((kept, kept_id)) => {
                if score > *kept || (score == *kept && id < *kept_id) {
                    *kept = score;
                    *kept_id = id;
                }
            }
            None => {
                self.best.insert(path.to_string(), (score, id));
            }
        }
    }

    pub(super) fn ranked(self) -> Vec<Hit> {
        let mut hits: Vec<Hit> = self
            .best
            .into_iter()
            .map(|(path, (score, id))| Hit { path, id, score })
            .collect();
        hits.sort_by(|a, b| {
            rounded(b.score)
                .total_cmp(&rounded(a.score))
                .then_with(|| a.path.cmp(&b.path))
        });
        hits
    }
}

/// A stored vector (little-endian `f32` × `dims`), or `None` when its width is not the measure's.
fn decode(blob: &[u8], dims: usize) -> Option<Vec<f32>> {
    if dims == 0 || blob.len() != dims * 4 {
        return None;
    }
    Some(
        blob.as_chunks::<4>()
            .0
            .iter()
            .map(|b| f32::from_le_bytes(*b))
            .collect(),
    )
}

/// Whether `path` lies in the search scope (`.` or empty is the whole fold).
pub(super) fn in_scope(path: &str, scope: &str) -> bool {
    let scope = scope.trim_end_matches('/');
    scope.is_empty()
        || scope == "."
        || path == scope
        || path
            .strip_prefix(scope)
            .is_some_and(|rest| rest.starts_with('/'))
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Locating the passage
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// The heading title a chunk's section label names in the outline: a markdown label is its ATX
/// line (`## Detail` → `Detail`), a Python label its `def`/`class` line; a window is `lines a-b`.
fn heading_title(section: &str) -> String {
    section.trim_start_matches('#').trim().to_string()
}

/// `lines a-b` → `a:b`.
fn window_lines(section: &str) -> Option<String> {
    let (first, last) = section.strip_prefix("lines ")?.split_once('-')?;
    let (first, last): (usize, usize) = (first.parse().ok()?, last.parse().ok()?);
    Some(format!("{first}:{last}"))
}

/// The first line (1-based) of `text` in the file, read to the same `scan_bytes` bound the outline
/// reads and charged to the same scan counters; `None` when the text is not found in that window.
fn line_of(
    root: &Path,
    contract: &Contract,
    path: &str,
    text: &str,
    usage: &mut Value,
) -> Option<usize> {
    let bound = contract.limit_usize("scan_bytes");
    let mut raw = Vec::new();
    File::open(root.join(path))
        .and_then(|f| f.take(bound as u64).read_to_end(&mut raw))
        .ok()?;
    add_usage(usage, &json!({"scan_bytes": raw.len(), "scanned_files": 1}));
    let source = String::from_utf8_lossy(&raw);
    let at = source.find(text.trim_end())?;
    Some(source[..at].matches('\n').count() + 1)
}

/// Where the winning chunk sits in the file: the outline heading of the same title (the range the
/// first screen would offer; where a title repeats, the one whose range holds the chunk), else the
/// chunk's own line range — a window's label, or where its text sits in the file. `None` when the
/// file no longer reads. Shared with the lexical route, which locates its best chunk the same way.
pub(super) fn locate(
    root: &Path,
    contract: &Contract,
    path: &str,
    section: &str,
    text: &str,
    terms: &[String],
    usage: &mut Value,
) -> Option<Value> {
    let outline = outline_at(root, contract, path, terms).ok()?;
    add_usage(usage, &outline["usage"]);
    let title = heading_title(section);
    let same: Vec<&Value> = outline["headings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|heading| !title.is_empty() && heading["title"].as_str() == Some(&title))
        .collect();
    let heading = match same.len() {
        0 => None,
        1 => Some(same[0]),
        _ => {
            let at = line_of(root, contract, path, text, usage).unwrap_or(0) as u64;
            let holds = |h: &&Value| {
                h["line"].as_u64().unwrap_or(0) <= at && at <= h["end_line"].as_u64().unwrap_or(0)
            };
            same.iter().copied().find(holds).or(same.first().copied())
        }
    };
    if let Some(heading) = heading {
        return Some(section_link(heading));
    }
    let lines = match window_lines(section) {
        Some(lines) => lines,
        None => {
            let start = line_of(root, contract, path, text, usage)?;
            let end = start + text.trim_end().lines().count().max(1) - 1;
            format!("{start}:{end}")
        }
    };
    Some(section_link(&json!({
        "title": section,
        "read_lines": lines,
        "hits": {},
        "window_complete": true,
    })))
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// The answer
// ───────────────────────────────────────────────────────────────────────────────────────────────

fn unavailable(error: FlowError) -> String {
    match error {
        FlowError::Unavailable(_) => format!("semantic: {error}"),
        other => format!("semantic: unavailable: {other}"),
    }
}

fn absent(why: Absent) -> String {
    match why {
        Absent::NoFold => NO_FOLD.to_string(),
        Absent::OtherMethod => OTHER_METHOD.to_string(),
        Absent::Unreadable(why) => {
            format!("semantic: the fold is unreadable ({why}) — run epr flow memory index fold")
        }
    }
}

/// One question answered by the semantic provider `declaration` (a `ceremony.providers` entry of
/// kind `semantic`), within `scope`. Never an error: every absence is one `unresolved` line.
pub(super) fn search(
    root: &Path,
    contract: &Contract,
    declaration: &Value,
    query: &str,
    scope: &str,
) -> Value {
    let began = Instant::now();
    let mut answer = json!({
        "producer": PRODUCER,
        "ranking_known": true,
        "method": Value::Null,
        "model": Value::Null,
        "embedder": Value::Null,
        "procedure": Value::Null,
        "built_by": Value::Null,
        "fold_lag": Value::Null,
        "scope": scope,
        "selection": SELECTION,
        "candidates": [],
        "omissions": [],
        "unresolved": [],
        "usage": {"search_queries": 1, "semantic_chunks_scanned": 0, "provider_seconds": 0},
    });
    if let Err(line) = answer_into(&mut answer, root, contract, declaration, query, scope) {
        // It could not rank: an absent route, never a known empty ranking.
        answer["ranking_known"] = json!(false);
        answer["candidates"] = json!([]);
        answer["unresolved"] = json!([line]);
    }
    answer["usage"]["semantic_query_ms"] = json!(began.elapsed().as_millis() as u64);
    answer
}

fn answer_into(
    answer: &mut Value,
    root: &Path,
    contract: &Contract,
    declaration: &Value,
    query: &str,
    scope: &str,
) -> Result<(), String> {
    let choice = match declaration.get("embedder").and_then(Value::as_str) {
        None | Some("pinned") => Ok(EmbedderChoice::Pinned),
        Some("fixture") => Ok(EmbedderChoice::Fixture),
        Some(other) => Err(format!(
            "semantic: the recipe declares embedder `{other}`; the set is pinned|fixture"
        )),
    };
    let rel = declaration
        .get("measure")
        .and_then(Value::as_str)
        .ok_or("semantic: the recipe declares no measure")?;
    // The measure loads first, so its CID is printed wherever it loaded — an embedder the recipe
    // spells wrongly still answers under a named method.
    let fold = SemanticFold::declare(
        root,
        contract.clone(),
        rel,
        choice.as_ref().copied().unwrap_or_default(),
    )
    .map_err(|error| format!("semantic: the declared measure does not load: {error}"))?;
    let method = fold.measure();
    answer["method"] = json!(method);
    let choice = choice?;
    answer["embedder"] = json!(choice.name());
    let model = match choice {
        EmbedderChoice::Pinned => {
            // The exact embedding procedure the ranking rests on: the CID the model manifest pins
            // (a manifest that does not load names none; the embedder then answers why).
            answer["procedure"] = json!(ModelManifest::load(&root.join(MODEL_MANIFEST_REL))
                .ok()
                .and_then(|manifest| manifest.procedure));
            json!(fold.model())
        }
        EmbedderChoice::Fixture => {
            answer["fitness"] = json!(FIXTURE_FITNESS);
            Value::Null
        }
    };
    answer["model"] = model.clone();
    let reader: FoldReader = fold.open(root).map_err(absent)?;
    answer["built_by"] = json!(reader.built_by());
    if query.trim().is_empty() {
        return Err("semantic: no question text to embed; name --query or --need".into());
    }

    // The embedder whose store this is — proven by its label BEFORE it runs, so a store folded by
    // another procedure never spawns one.
    let budget = EmbedBudget::query(contract).map_err(unavailable)?;
    let (embedder, label) = fold.embedder(root).map_err(unavailable)?;
    if reader.built_by() != label {
        return Err(OTHER_METHOD.to_string());
    }
    // The question, embedded once.
    let embedding_began = Instant::now();
    let (embedding, spawned) = embedder.embed_metered(&[query.to_string()], budget);
    if spawned {
        // A process ran: its cost is metered whether or not it answered.
        answer["usage"]["embedding_processes"] = json!(1);
    }
    let embedding = embedding.map_err(unavailable);
    answer["usage"]["provider_seconds"] =
        json!((embedding_began.elapsed().as_secs_f64() * 1e6).round() / 1e6);
    let question = embedding?
        .vectors
        .into_iter()
        .next()
        .ok_or("semantic: unavailable: the embedder returned no vector")?;
    let dims = fold.dims();
    if question.len() != dims {
        return Err(format!(
            "semantic: unavailable: the embedder replied {} dims; the measure pins {dims}",
            question.len()
        ));
    }

    // The lag at answer time: a stale fold answers, and says so.
    let mut omissions: Vec<String> = Vec::new();
    let lag = match fold.lag(root, &reader) {
        Ok(lag) => {
            if lag > 0 {
                omissions.push(format!(
                    "fold {lag} files behind ({}); a file changed since its fold ranks by its \
                     folded text",
                    fold.lag_bound()
                ));
            }
            json!(lag)
        }
        Err(why) => {
            omissions.push(format!("fold lag unknown: {why}"));
            Value::Null
        }
    };
    answer["fold_lag"] = lag.clone();

    // Cosine over every live chunk; each file keeps its best.
    let mut best = BestPerFile::default();
    let (mut malformed, mut in_view) = (0usize, 0usize);
    let scanned = reader
        .scan(|id, path, blob| {
            if !in_scope(path, scope) {
                return;
            }
            in_view += 1;
            match decode(blob, dims) {
                Some(vector) => best.offer(id, path, f64::from(cosine(&question, &vector))),
                None => malformed += 1,
            }
        })
        .map_err(|error| format!("semantic: the fold could not be read: {error}"))?;
    answer["usage"]["semantic_chunks_scanned"] = json!(scanned);
    if in_view == 0 {
        omissions.push(format!("semantic: no folded chunks under {scope}"));
    }
    if malformed > 0 {
        omissions.push(format!(
            "{malformed} chunk(s) carry a vector that is not {dims} wide and were not ranked"
        ));
    }

    // The window: `limits.search_results` files, the same window every route's `search` returns
    // (the lens's `choice_count` cuts the first screen, not a search). Each is located at its
    // passage; a file gone from the tree or outside the declared source roots is named, not
    // offered; a cosine at or below zero is not a ranking and is not offered.
    let limit = contract.limit_usize("search_results").max(1);
    let terms = question_terms(contract, query);
    let roots = contract.source_roots();
    let mut candidates: Vec<Value> = Vec::new();
    let (mut outside, mut unranked, mut withheld) = (0usize, 0usize, 0usize);
    for hit in best.ranked() {
        if candidates.len() >= limit {
            break;
        }
        if hit.score <= 0.0 {
            unranked += 1;
            continue;
        }
        // The question bank and the generated register are never an answer, on any screen a
        // provider feeds (the first-screen offer rule, one predicate for every route).
        if !offered_on_first_screen(contract, &hit.path) {
            withheld += 1;
            continue;
        }
        if !root.join(&hit.path).is_file() {
            omissions.push(format!("{}: folded file no longer present", hit.path));
            continue;
        }
        if contained(root, &hit.path, &roots).is_err() {
            outside += 1;
            continue;
        }
        let (section, text) = reader
            .chunk(hit.id)
            .map_err(|error| format!("semantic: the fold could not be read: {error}"))?;
        let mut candidate = json!({
            "path": hit.path,
            "score": rounded(hit.score),
            "producer": PRODUCER,
            "method": method,
            "model": model,
            "fold_lag": lag,
        });
        if let Some(section) = locate(
            root,
            contract,
            &hit.path,
            &section,
            &text,
            &terms,
            &mut answer["usage"],
        ) {
            candidate["best_section"] = section;
        }
        candidates.push(candidate);
    }
    if outside > 0 {
        omissions.push(format!(
            "{outside} ranked file(s) lie outside the recipe's declared source roots and were \
             passed over"
        ));
    }
    if withheld > 0 {
        omissions.push(format!("{withheld} non-authority hit(s) withheld"));
    }
    if unranked > 0 {
        omissions.push(format!(
            "{unranked} file(s) scored a cosine at or below zero — no ranking — and were not \
             offered"
        ));
    }
    answer["candidates"] = json!(candidates);
    answer["omissions"] = json!(omissions);
    Ok(())
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// The Provider seam
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// The recipe's semantic provider, named by the `ceremony.providers` key it is declared under.
pub(super) struct Semantic {
    pub key: String,
}

impl Provider for Semantic {
    fn id(&self) -> ProviderId {
        PRODUCER.to_string()
    }

    fn candidates(
        &self,
        query: &str,
        _terms: &[String],
        scope: &Path,
        contract: &Contract,
        session_root: &Path,
    ) -> FlowResult<ProviderResult> {
        let declaration = contract
            .value
            .pointer(&format!("/ceremony/providers/{}", self.key))
            .cloned()
            .unwrap_or(Value::Null);
        let scope = super::providers::scope_string(scope, session_root);
        let mut answer = search(session_root, contract, &declaration, query, &scope);
        Ok(ProviderResult {
            ranked: answer["candidates"].as_array().cloned().unwrap_or_default(),
            ranking_known: answer["ranking_known"] == json!(true),
            method: answer["method"].as_str().map(str::to_string),
            usage: answer["usage"].take(),
            unresolved: lines(&answer["unresolved"]),
            omissions: lines(&answer["omissions"]),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_is_the_normalised_dot_product_and_zero_without_length() {
        assert!((cosine(&[1.0, 0.0], &[2.0, 0.0]) - 1.0).abs() < 1e-6);
        assert!(cosine(&[1.0, 0.0], &[0.0, 3.0]).abs() < 1e-6);
        assert!((cosine(&[1.0, 1.0], &[-1.0, -1.0]) + 1.0).abs() < 1e-6);
        assert_eq!(cosine(&[0.0, 0.0], &[1.0, 0.0]), 0.0);
        assert_eq!(cosine(&[1.0], &[1.0, 0.0]), 0.0, "widths differ");
    }

    #[test]
    fn each_file_keeps_its_best_chunk_and_ties_break_by_path() {
        let question = [1.0f32, 0.0];
        let mut best = BestPerFile::default();
        for (id, path, vector) in [
            (1, "b.md", [0.0f32, 1.0]),
            (2, "b.md", [1.0, 0.0]),
            (3, "a.md", [1.0, 0.0]),
            (4, "c.md", [1.0, 1.0]),
            (5, "a.md", [1.0, 0.0]),
        ] {
            best.offer(id, path, f64::from(cosine(&question, &vector)));
        }
        let ranked = best.ranked();
        let order: Vec<(&str, i64)> = ranked.iter().map(|h| (h.path.as_str(), h.id)).collect();
        // a.md and b.md tie at 1.0: path decides; within a.md the earlier chunk (id 3) is kept;
        // b.md's best is its second chunk.
        assert_eq!(order, vec![("a.md", 3), ("b.md", 2), ("c.md", 4)]);
        assert_eq!(
            rounded(ranked[2].score),
            (std::f64::consts::FRAC_1_SQRT_2 * 10_000.0).round() / 10_000.0,
            "cosine 1/√2, printed at 4 decimals"
        );
    }

    #[test]
    fn a_vector_is_little_endian_f32_of_the_measures_width() {
        let blob: Vec<u8> = [1.5f32, -2.0]
            .iter()
            .flat_map(|x| x.to_le_bytes())
            .collect();
        assert_eq!(decode(&blob, 2), Some(vec![1.5, -2.0]));
        assert_eq!(decode(&blob, 3), None);
    }

    /// The seam a fusing caller reads: an absent fold is `ranking_known: false` with its reason,
    /// and the same provider over a fold is a known ranking with nothing unresolved.
    #[test]
    fn the_provider_seam_carries_absence_and_a_known_ranking() {
        use super::super::index::{self, FoldOptions, FoldRun};
        let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .unwrap();
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let mut value = crate::flow::memory::recall::tests_support::minimal_contract();
        value["ceremony"]["providers"]["semantic"]["embedder"] = json!("fixture");
        let measure = value["ceremony"]["providers"]["semantic"]["measure"]
            .as_str()
            .unwrap()
            .to_string();
        for (rel, bytes) in [
            (
                CONTRACT_REL.to_string(),
                serde_json::to_vec(&value).unwrap(),
            ),
            (measure.clone(), std::fs::read(repo.join(&measure)).unwrap()),
            (
                "note.md".to_string(),
                b"# Stewardship\nThe commons is tended.\n".to_vec(),
            ),
        ] {
            std::fs::create_dir_all(root.join(&rel).parent().unwrap()).unwrap();
            std::fs::write(root.join(&rel), bytes).unwrap();
        }
        let contract = Contract::load(&root.join(CONTRACT_REL)).unwrap();
        let semantic = Semantic {
            key: "semantic".into(),
        };

        let absent = semantic
            .candidates("tended commons", &[], root, &contract, root)
            .unwrap();
        assert!(
            !absent.ranking_known,
            "an absent route is not a known ranking"
        );
        assert_eq!(absent.unresolved, vec![NO_FOLD.to_string()]);
        assert!(absent.ranked.is_empty());
        assert!(
            absent.method.is_some(),
            "the measure loaded, so its CID is named"
        );

        let opts = FoldOptions {
            embedder: EmbedderChoice::Fixture,
            ..FoldOptions::default()
        };
        assert!(matches!(
            index::fold(root, &opts).unwrap(),
            FoldRun::Done(_)
        ));
        let found = semantic
            .candidates("tended commons", &[], root, &contract, root)
            .unwrap();
        assert!(found.ranking_known);
        assert!(found.unresolved.is_empty(), "{:?}", found.unresolved);
        assert_eq!(found.ranked[0]["path"], "note.md");
        assert_eq!(found.method, absent.method);
    }

    #[test]
    fn scope_and_labels_read_as_declared() {
        assert!(in_scope("genesis/a.md", "genesis"));
        assert!(in_scope("genesis/a.md", "genesis/"));
        assert!(in_scope(".claude/x.md", "."));
        assert!(!in_scope("genesis-other/a.md", "genesis"));
        assert_eq!(heading_title("## Who fixes it"), "Who fixes it");
        assert_eq!(heading_title("def fold():"), "def fold():");
        assert_eq!(window_lines("lines 3-40"), Some("3:40".to_string()));
        assert_eq!(window_lines("# lines"), None);
    }
}

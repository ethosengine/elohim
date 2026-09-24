//! LEXICAL — the FTS5/BM25 candidate route over the shared fold (governed-discovery station 4,
//! task 4.8): `search --provider lexical`, and a producer the first screen's fusion recipe may name.
//!
//! The method is the declaration. The recipe's `ceremony.providers.<key>` (kind `lexical`) names
//! its own `IndexMeasure` (`measure`: `recall-lexical-index@1`, `ranking: bm25`, no model pin), the
//! semantic measure whose fold it reads (`fold`), and which embedder's store that is (`embedder`:
//! `pinned`, the live default, or `fixture` for tests). The lexical measure folds nothing: it
//! declares the fold measure's own chunk rule and surfaces, and a fold cut or covered otherwise is
//! refused. One answer:
//!
//! 1. the question's terms (`discovery::question_terms`), each suffix-stemmed (`discovery::stem`)
//!    and quoted as an FTS5 prefix string (`"fold"*` matches `folded`, `folds`, `folding`), OR-joined
//!    — every term quoted, so no question text is ever FTS5 syntax (`NEAR`, `OR`, `*`, `^`, a
//!    column filter are words here); no tokenizer change (that is a schema change and a re-fold);
//! 2. FTS5 `bm25()` over every LIVE chunk that matches, each file keeping its best chunk; the score
//!    printed is `bm25()` negated (higher ranks first) at 4 decimals — no standing, no behaviour
//!    signal;
//! 3. the top `limits.search_results` files inside the search scope and the declared source roots,
//!    ties broken by path — the same window every route's `search` returns;
//! 4. each candidate prints `producer: lexical`, the lexical measure CID as `method`, the fold's lag
//!    at answer time and a `best_section` located exactly as the semantic route locates its chunk.
//!
//! The FTS query reads a DERIVED store: it is reported in `usage` (`lexical_query_ms`,
//! `lexical_chunks_matched`), never charged to `source_bytes`; locating a winner's heading reads
//! the source file and is charged to the scan counters, as on every route.
//!
//! Absence is honest, never an error: no fold, a fold built under another method, or a question
//! with no term to match each answer with ONE `unresolved` line, no candidates and `ranking_known:
//! false` — through `retrieve()` and the `Provider` seam alike. A stale fold answers and says how
//! stale (`fold N files behind`). This route never spawns an embedding process: BM25 reads the
//! chunk text, which is the same whichever embedder folded it.
use super::discovery::{question_terms, stem};
use super::embedder::FIXTURE_FITNESS;
use super::index::{Absent, EmbedderChoice, FoldReader, LexicalMeasure, SemanticFold};
use super::providers::{lines, scope_string, Provider, ProviderId, ProviderResult};
use super::semantic::{in_scope, locate, rounded, BestPerFile};
use super::*;

/// Every candidate's and every answer's `producer`.
pub(super) const PRODUCER: &str = "lexical";

/// The answer when the fold's store does not exist.
pub(super) const NO_FOLD: &str = "lexical: no fold — run epr flow memory index fold";

/// The answer when the store, or the fold measure, is not this measure's method.
pub(super) const OTHER_METHOD: &str = "lexical: the fold was built under another method — refold";

/// The answer when the question carries no term to match.
pub(super) const NO_TERMS: &str =
    "lexical: the question carries no term to match; name --query or --need with words of four \
     or more characters";

const SELECTION: &str = "FTS5 bm25() over every live chunk in the shared fold, each question term \
                         stemmed and matched as a quoted prefix; each file's best chunk; top \
                         limits.search_results files, ties by path; score = -bm25(); not authority";

/// The FTS5 match expression for `terms`: each term with at least one letter or digit, stemmed,
/// quoted as an FTS5 string (an inner `"` doubled) and marked a prefix, OR-joined. `None` when no
/// term is left. Quoting is what makes question text inert: inside a string, FTS5 reads only the
/// tokenizer's tokens — never an operator, a column filter or a `NEAR` group.
pub(super) fn match_expression(terms: &[String]) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    for term in terms {
        if !term.chars().any(char::is_alphanumeric) {
            continue;
        }
        let quoted = format!("\"{}\"*", stem(term).replace('"', "\"\""));
        if !parts.contains(&quoted) {
            parts.push(quoted);
        }
    }
    (!parts.is_empty()).then(|| parts.join(" OR "))
}

fn absent(why: Absent) -> String {
    match why {
        Absent::NoFold => NO_FOLD.to_string(),
        Absent::OtherMethod => OTHER_METHOD.to_string(),
        Absent::Unreadable(why) => {
            format!("lexical: the fold is unreadable ({why}) — run epr flow memory index fold")
        }
    }
}

/// One question — its `terms` — answered by the lexical provider `declaration` (a
/// `ceremony.providers` entry of kind `lexical`), within `scope`. Never an error: every absence is
/// one `unresolved` line.
pub(super) fn search(
    root: &Path,
    contract: &Contract,
    declaration: &Value,
    terms: &[String],
    scope: &str,
) -> Value {
    let began = Instant::now();
    let mut answer = json!({
        "producer": PRODUCER,
        "ranking_known": true,
        "method": Value::Null,
        "fold": Value::Null,
        "embedder": Value::Null,
        "fold_lag": Value::Null,
        "match": Value::Null,
        "scope": scope,
        "selection": SELECTION,
        "candidates": [],
        "omissions": [],
        "unresolved": [],
        "usage": {"search_queries": 1, "lexical_chunks_matched": 0},
    });
    if let Err(line) = answer_into(&mut answer, root, contract, declaration, terms, scope) {
        // It could not rank: an absent route, never a known empty ranking.
        answer["ranking_known"] = json!(false);
        answer["candidates"] = json!([]);
        answer["unresolved"] = json!([line]);
    }
    answer["usage"]["lexical_query_ms"] = json!(began.elapsed().as_millis() as u64);
    answer
}

fn answer_into(
    answer: &mut Value,
    root: &Path,
    contract: &Contract,
    declaration: &Value,
    terms: &[String],
    scope: &str,
) -> Result<(), String> {
    let choice = match declaration.get("embedder").and_then(Value::as_str) {
        None | Some("pinned") => Ok(EmbedderChoice::Pinned),
        Some("fixture") => Ok(EmbedderChoice::Fixture),
        Some(other) => Err(format!(
            "lexical: the recipe declares embedder `{other}`; the set is pinned|fixture"
        )),
    };
    let rel = declaration
        .get("measure")
        .and_then(Value::as_str)
        .ok_or("lexical: the recipe declares no measure")?;
    // The measure loads first, so its CID is printed wherever it loaded.
    let measure = LexicalMeasure::declare(root, contract.clone(), rel)
        .map_err(|error| format!("lexical: the declared measure does not load: {error}"))?;
    let method = measure.cid();
    answer["method"] = json!(method);
    let choice = choice?;
    answer["embedder"] = json!(choice.name());
    if choice == EmbedderChoice::Fixture {
        answer["fitness"] = json!(FIXTURE_FITNESS);
    }
    let fold_rel = declaration
        .get("fold")
        .and_then(Value::as_str)
        .ok_or("lexical: the recipe declares no fold")?;
    let fold = SemanticFold::declare(root, contract.clone(), fold_rel, choice)
        .map_err(|error| format!("lexical: the declared fold measure does not load: {error}"))?;
    answer["fold"] = json!(fold.measure());
    if !measure.shares(&fold) {
        return Err(OTHER_METHOD.to_string());
    }
    let reader: FoldReader = fold.open(root).map_err(absent)?;
    let expression = match_expression(terms).ok_or(NO_TERMS)?;
    answer["match"] = json!(expression);

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

    // BM25 over every live matching chunk; each file keeps its best (bm25 is lower-is-better, so
    // the kept score is its negation).
    let mut best = BestPerFile::default();
    let mut in_view = 0usize;
    let matched = reader
        .lexical(&expression, |id, path, rank| {
            if in_scope(path, scope) {
                in_view += 1;
                best.offer(id, path, -rank);
            }
        })
        .map_err(|error| format!("lexical: the fold could not be read: {error}"))?;
    answer["usage"]["lexical_chunks_matched"] = json!(matched);
    if in_view == 0 {
        omissions.push(format!(
            "lexical: no folded chunk under {scope} matches the question's terms"
        ));
    }

    // The window: `limits.search_results` files; a file gone from the tree or outside the declared
    // source roots is named, not offered.
    let limit = contract.limit_usize("search_results").max(1);
    let roots = contract.source_roots();
    let mut candidates: Vec<Value> = Vec::new();
    let mut outside = 0usize;
    for hit in best.ranked() {
        if candidates.len() >= limit {
            break;
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
            .map_err(|error| format!("lexical: the fold could not be read: {error}"))?;
        let mut candidate = json!({
            "path": hit.path,
            "score": rounded(hit.score),
            "producer": PRODUCER,
            "method": method,
            "fold_lag": lag,
        });
        if let Some(section) = locate(
            root,
            contract,
            &hit.path,
            &section,
            &text,
            terms,
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
    answer["candidates"] = json!(candidates);
    answer["omissions"] = json!(omissions);
    Ok(())
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// The Provider seam
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// The recipe's lexical provider, named by the `ceremony.providers` key it is declared under.
pub(super) struct Lexical {
    pub key: String,
}

impl Provider for Lexical {
    fn id(&self) -> ProviderId {
        PRODUCER.to_string()
    }

    /// Matches the `terms` a caller already derived from the question (the first screen's own);
    /// with none given, the question text's own terms.
    fn candidates(
        &self,
        query: &str,
        terms: &[String],
        scope: &Path,
        contract: &Contract,
        session_root: &Path,
    ) -> FlowResult<ProviderResult> {
        let declaration = contract
            .value
            .pointer(&format!("/ceremony/providers/{}", self.key))
            .cloned()
            .unwrap_or(Value::Null);
        let derived;
        let terms = if terms.is_empty() {
            derived = question_terms(contract, query);
            &derived[..]
        } else {
            terms
        };
        let scope = scope_string(scope, session_root);
        let mut answer = search(session_root, contract, &declaration, terms, &scope);
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

    fn terms(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| w.to_string()).collect()
    }

    #[test]
    fn each_term_is_stemmed_quoted_and_a_prefix_or_joined() {
        assert_eq!(
            match_expression(&terms(&["folding", "stamps", "habit"])).as_deref(),
            Some("\"fold\"* OR \"stamp\"* OR \"habit\"*")
        );
        // A term that stems to one already present is asked once.
        assert_eq!(
            match_expression(&terms(&["habits", "habit"])).as_deref(),
            Some("\"habit\"*")
        );
        assert_eq!(match_expression(&[]), None);
        assert_eq!(
            match_expression(&terms(&["-", "./"])),
            None,
            "no letter or digit"
        );
    }

    /// FTS5 syntax in a term is a word, never an operator: every term is one quoted string, and a
    /// quote inside it is doubled, so it cannot close the string early.
    #[test]
    fn fts_syntax_in_a_term_stays_inside_its_string() {
        assert_eq!(
            match_expression(&terms(&["near(a", "b\" OR c", "col:x^"])).as_deref(),
            Some("\"near(a\"* OR \"b\"\" OR c\"* OR \"col:x^\"*")
        );
    }

    #[test]
    fn an_absent_fold_is_named_on_the_seam_never_a_known_empty_ranking() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .unwrap();
        let mut value = crate::flow::memory::recall::tests_support::minimal_contract();
        let fold = value["ceremony"]["providers"]["semantic"]["measure"]
            .as_str()
            .unwrap()
            .to_string();
        let measure = ".epr-meta/elohim/algorithms/recall-lexical-index.json".to_string();
        value["ceremony"]["providers"]["lexical"] = json!({
            "kind": "lexical", "measure": measure, "fold": fold, "embedder": "fixture",
            "optional": true,
        });
        for rel in [&fold, &measure] {
            std::fs::create_dir_all(root.join(rel).parent().unwrap()).unwrap();
            std::fs::copy(repo.join(rel), root.join(rel)).unwrap();
        }
        let contract = Contract::from_value(value).unwrap();
        let out = Lexical {
            key: "lexical".into(),
        }
        .candidates("stewardship commons", &[], root, &contract, root)
        .unwrap();
        assert!(!out.ranking_known);
        assert_eq!(out.unresolved, vec![NO_FOLD.to_string()]);
        assert!(out.ranked.is_empty());
        assert!(
            out.method.is_some(),
            "the measure loaded, so its CID is named"
        );
        assert!(
            out.usage.get("embedding_processes").is_none(),
            "the lexical route never runs an embedder"
        );
    }
}

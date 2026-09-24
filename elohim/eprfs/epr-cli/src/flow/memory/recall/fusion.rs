//! FUSION — reciprocal-rank fusion for ORDER on the focused first screen (governed-discovery
//! station 4, task 4.5).
//!
//! The recipe is a declaration, not a constant: `discovery.first_screen_fusion` in the pinned
//! contract (`{"recipe": "rrf-v1", "k": 60, "producers": ["local", "semantic"], "order_only":
//! true}`), and its method CID is `atom_cid` of that object — the canonical dag-cbor address the
//! honesty floor prints beside the recipe's own.
//!
//! **Order only.** A path's fused score is Σ 1/(k + rank) over the producers that returned it
//! (rank 1-based, in each producer's own order); ties break by path. Nothing a candidate carries
//! — no producer's raw score, no term count, no standing, no human signal — is read by [`fuse`]:
//! it sees each producer's ORDER and the path, and nothing else. Every fused candidate keeps its
//! producer ranks (`ranks: {local: n|null, semantic: n|null}`, one key per recipe producer — a
//! recipe may also name `lexical`, task 4.8) so the reader sees exactly what was fused.
//!
//! **When it runs.** Only on a focused open whose `--need` was typed (the first screen does not
//! exist otherwise, so `open --purpose bootstrap` never asks the semantic route anything), over
//! the focused area if one resolved, else the session scope. An absent, unavailable or
//! other-method route is ONE omission line naming why and fusion proceeds over the producers that
//! answered; with only `local` left, or no other producer's rank on the screen, it stays the
//! lexical screen — never an exit 2 for the whole screen. A stale fold fuses, and its `fold N
//! files behind` line rides into the screen's omissions. Every producer's order passes the same
//! first-screen offer rule (`discovery::offered_on_first_screen`) before it is fused. The call is
//! part of the screen, charged as `first_screen_<producer>_calls`, never as the packet's
//! `search_queries`. The lens cut and the content floor apply after fusion, in `render.rs`:
//! fusion orders; the lens chooses how many.
use elohim_epr_rea::atom_cid;

use super::discovery::{
    frontmatter_header, frontmatter_value, offered_on_first_screen, trim_to_character_boundary,
};
use super::providers::{providers_for, ProviderResult};
use super::*;

/// Where the pinned contract declares the first screen's fusion recipe.
pub(super) const RECIPE_POINTER: &str = "/discovery/first_screen_fusion";

/// The one fusion recipe this executor runs.
pub(super) const RRF_V1: &str = "rrf-v1";

/// The producer whose order is the first screen's own lexical order.
pub(super) const LOCAL: &str = "local";

/// The declared fusion recipe, read and checked.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Recipe {
    pub name: String,
    pub k: u64,
    pub producers: Vec<String>,
    /// `atom_cid` of the declared object — the method every fused screen names.
    pub cid: String,
}

impl Recipe {
    /// `None` when the contract declares no fusion (the screen is the lexical screen, as before
    /// this task); `Some(Err(reason))` when it declares one this executor does not run.
    pub(super) fn declared(contract: &Contract) -> Option<Result<Self, String>> {
        let object = contract.value.pointer(RECIPE_POINTER)?;
        Some(Self::from_declared(object))
    }

    fn from_declared(object: &Value) -> Result<Self, String> {
        let name = object["recipe"].as_str().unwrap_or_default();
        if name != RRF_V1 {
            return Err(format!(
                "fusion: the recipe declares `{name}`; this executor runs {RRF_V1} only"
            ));
        }
        if object["order_only"].as_bool() != Some(true) {
            return Err(
                "fusion: the recipe is not order_only; fusion here orders, it never sums a \
                 producer's score"
                    .to_string(),
            );
        }
        let k = object["k"]
            .as_u64()
            .filter(|k| *k > 0)
            .ok_or("fusion: the recipe's k must be a positive integer")?;
        let producers: Vec<String> = object["producers"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect();
        if producers.first().map(String::as_str) != Some(LOCAL) || producers.len() < 2 {
            return Err(format!(
                "fusion: the recipe's producers must begin with `{LOCAL}` (the screen's own \
                 order) and name another"
            ));
        }
        let cid = atom_cid(object)
            .map_err(|error| format!("fusion: the recipe has no canonical address: {error}"))?;
        Ok(Self {
            name: name.to_string(),
            k,
            producers,
            cid: cid.to_string(),
        })
    }
}

/// One fused candidate: its path, its reciprocal-rank score, and its rank in each producer's
/// order (`None` where that producer did not return it), in the recipe's producer order.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Fused {
    pub path: String,
    pub score: f64,
    pub ranks: Vec<(String, Option<usize>)>,
    /// The candidate as its first returning producer (in recipe order) shaped it, with `ranks`
    /// and, for every later producer that also returned it, that producer's own fields under
    /// `native.<producer>`. Its `best_section` — the passage its linked read lands on — is the one
    /// the best-ranked producer located (ties → the earlier producer, `local` first), named by
    /// `passage_by`; a displaced passage is kept under `native.<producer>.best_section`.
    pub candidate: Value,
}

/// Reciprocal-rank fusion of `producers` (each an id and its candidates in its own order), for
/// order only: a path's score is Σ 1/(k + rank) over the producers that returned it, rank
/// 1-based among that producer's distinct paths; higher first, ties by path. Only each
/// candidate's `path` and its position are read for ORDER — no field a candidate carries can move
/// it. The PASSAGE follows the ranking: the fused candidate reads the section the producer that
/// ranked it best located (ties → earlier in recipe order, so `local`), and says which in
/// `passage_by` — the linked read lands where the ranking producer matched (station 4
/// integration, the q-hook-binary seam).
pub(super) fn fuse(producers: &[(String, Vec<Value>)], k: u64) -> Vec<Fused> {
    struct Entry {
        score: f64,
        ranks: Vec<Option<usize>>,
        /// Each producer's own candidate for this path, in recipe order.
        own: Vec<Option<Value>>,
    }
    let mut entries: BTreeMap<String, Entry> = BTreeMap::new();
    for (index, (_, candidates)) in producers.iter().enumerate() {
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for candidate in candidates {
            let Some(path) = candidate["path"].as_str() else {
                continue;
            };
            if !seen.insert(path) {
                continue;
            }
            let rank = seen.len();
            let entry = entries.entry(path.to_string()).or_insert_with(|| Entry {
                score: 0.0,
                ranks: vec![None; producers.len()],
                own: vec![None; producers.len()],
            });
            entry.score += 1.0 / (k as f64 + rank as f64);
            entry.ranks[index] = Some(rank);
            entry.own[index] = Some(candidate.clone());
        }
    }
    let mut fused: Vec<Fused> = entries
        .into_iter()
        .map(|(path, entry)| {
            let ids: Vec<&String> = producers.iter().map(|(producer, _)| producer).collect();
            // The producer whose located passage the candidate reads: the best rank among those
            // that located one; `min_by_key` keeps the first of equals, so a tie goes to the
            // earlier producer in recipe order.
            let passage = (0..ids.len())
                .filter(|&i| {
                    entry.own[i]
                        .as_ref()
                        .is_some_and(|own| !own["best_section"].is_null())
                })
                .min_by_key(|&i| entry.ranks[i].unwrap_or(usize::MAX));
            let shaper = entry.own.iter().position(Option::is_some);
            let mut candidate = json!({"path": path});
            let mut native = Map::new();
            for (i, own) in entry.own.into_iter().enumerate() {
                let Some(mut own) = own else {
                    continue;
                };
                if Some(i) == shaper {
                    candidate = own;
                } else if let Some(fields) = own.as_object_mut() {
                    fields.remove("path");
                    native.insert(ids[i].clone(), own);
                }
            }
            if let (Some(chosen), Some(shaper)) = (passage, shaper) {
                if chosen != shaper {
                    let section = native
                        .get(ids[chosen].as_str())
                        .map_or(Value::Null, |own| own["best_section"].clone());
                    let displaced = std::mem::replace(&mut candidate["best_section"], section);
                    if !displaced.is_null() {
                        let own = native
                            .entry(ids[shaper].clone())
                            .or_insert_with(|| json!({}));
                        own["best_section"] = displaced;
                    }
                }
                candidate["passage_by"] = json!(ids[chosen]);
            }
            let ranks: Vec<(String, Option<usize>)> =
                ids.into_iter().cloned().zip(entry.ranks).collect();
            candidate["ranks"] = Value::Object(
                ranks
                    .iter()
                    .map(|(producer, rank)| (producer.clone(), json!(rank)))
                    .collect(),
            );
            if !native.is_empty() {
                candidate["native"] = Value::Object(native);
            }
            Fused {
                path,
                score: entry.score,
                ranks,
                candidate,
            }
        })
        .collect();
    fused.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.path.cmp(&b.path))
    });
    fused
}

/// Fuse `screen`'s lexical candidates with the recipe's other producers, in place, asking each
/// over `scope` with the typed question. Nothing happens when the contract declares no fusion.
/// A producer that could not rank (absent, unavailable, undeclared) adds ONE omission line and is
/// left out of the fusion — charged as part of the screen only if an embedding process ran; a
/// known ranking's own omissions (a stale fold's `fold N files behind`) ride into the screen's and
/// its usage is charged as part of the screen.
pub(super) fn fuse_screen(
    args: &Args,
    contract: &Contract,
    screen: &mut Value,
    scope: &str,
    terms: &[String],
    usage: &mut Value,
) {
    let recipe = match Recipe::declared(contract) {
        None => return,
        Some(Ok(recipe)) => recipe,
        Some(Err(reason)) => {
            push_omission(screen, reason);
            return;
        }
    };
    let local: Vec<Value> = offered(
        contract,
        screen["candidates"].as_array().cloned().unwrap_or_default(),
    );
    let mut orders: Vec<(String, Vec<Value>)> = vec![(LOCAL.to_string(), local)];
    let mut methods: Vec<Value> = vec![json!({"id": LOCAL, "method": contract.method_cid()})];
    let declared = providers_for(contract);
    for producer in recipe.producers.iter().skip(1) {
        // Each producer that cannot rank is ONE omission line, and fusion proceeds over the
        // producers that answered.
        let Some(provider) = declared.iter().find(|p| p.id() == *producer) else {
            push_omission(
                screen,
                format!("{producer}: not declared by the recipe's ceremony.providers"),
            );
            continue;
        };
        let answer = match provider.candidates(
            args.need.trim(),
            terms,
            Path::new(scope),
            contract,
            &args.root,
        ) {
            Ok(answer) => answer,
            Err(error) => {
                push_omission(screen, format!("{producer}: unavailable: {error}"));
                continue;
            }
        };
        // Metered whenever the call answered or an embedding process actually ran — a spawned
        // then failed embedder still cost its seconds; a route refused before any spawn (no
        // fold, another method, a label mismatch) cost nothing and is charged nothing.
        let ran = answer.usage["embedding_processes"].as_u64().unwrap_or(0) > 0;
        if answer.ranking_known || ran {
            charge_screen_call(usage, producer, &answer);
        }
        if !answer.ranking_known {
            let why = if answer.unresolved.is_empty() {
                "ranking unknown; not fused".to_string()
            } else {
                answer.unresolved.join("; ")
            };
            let prefix = format!("{producer}:");
            push_omission(
                screen,
                if why.starts_with(&prefix) {
                    why
                } else {
                    format!("{prefix} {why}")
                },
            );
            continue;
        }
        for line in answer.omissions {
            push_omission(screen, line);
        }
        methods.push(json!({"id": producer, "method": answer.method}));
        orders.push((producer.clone(), offered(contract, answer.ranked)));
    }
    if orders.len() < 2 {
        return;
    }
    let fused = fuse(&orders, recipe.k);
    // No fusion token over nothing: when no other producer's rank reached the screen, it stays
    // the lexical screen (its omissions still say what the others answered).
    if fused
        .iter()
        .all(|f| f.ranks.iter().skip(1).all(|(_, rank)| rank.is_none()))
    {
        return;
    }
    let candidates: Vec<Value> = fused
        .into_iter()
        .map(|fused| {
            let mut candidate = fused.candidate;
            if candidate["ranks"][LOCAL].is_null() {
                // A candidate only another producer returned was never read for its frontmatter:
                // its declared title and content class are read now, so the content floor keeps
                // a correction past the lens cut whichever producer found it.
                let (title, class) =
                    declared_fields(&args.root, contract, &fused.path, &mut *usage);
                if let Some(title) = title {
                    candidate["title"] = json!(title);
                }
                candidate["content_class"] = json!(class);
            }
            if candidate["title"].as_str().is_none_or(str::is_empty) {
                // A producer that reads no frontmatter names no title; the file's name stands in,
                // as the authority set's own candidates do.
                candidate["title"] = json!(Path::new(&fused.path)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or(fused.path));
            }
            candidate
        })
        .collect();
    let lexical = screen["ranking"]
        .as_str()
        .unwrap_or("the screen's own order")
        .to_string();
    screen["candidates"] = json!(candidates);
    screen["provider"] = json!(recipe.producers.join(" + "));
    screen["ranking"] = json!(format!(
        "{} reciprocal-rank order over {} (k {}; order only — no producer score, no standing); \
         local: {lexical}",
        recipe.name,
        recipe.producers.join(" + "),
        recipe.k
    ));
    screen["fusion"] = json!({
        "recipe": recipe.name,
        "cid": recipe.cid,
        "k": recipe.k,
        "order_only": true,
        "producers": methods,
    });
}

/// A candidate's declared `title` and `content_class`, read from at most `limits.metadata_bytes`
/// of its own frontmatter and charged to the scan counters (discovery, never evidence). For a
/// candidate that a producer reading no frontmatter (the semantic route) put on the first screen,
/// so the content floor holds for it exactly as for a lexical candidate (station 4, task 4.5).
/// Each field is `None` when the file declares none or the read cannot prove its header.
fn declared_fields(
    root: &Path,
    contract: &Contract,
    path: &str,
    usage: &mut Value,
) -> (Option<String>, Option<String>) {
    let limit = contract.limit_usize("metadata_bytes");
    let mut data = Vec::new();
    let Ok(file) = File::open(root.join(path)) else {
        return (None, None);
    };
    let file_bytes = file.metadata().map_or(0, |m| m.len() as usize);
    if file.take(limit as u64).read_to_end(&mut data).is_err() {
        return (None, None);
    }
    add_usage(
        usage,
        &json!({"scan_bytes": data.len(), "scanned_files": 1}),
    );
    let Some(text) = trim_to_character_boundary(&data) else {
        return (None, None);
    };
    // The header's boundary is proven against the bounded read before it is parsed.
    if frontmatter_header(&text, data.len(), file_bytes).is_none() {
        return (None, None);
    }
    let Some(value) = frontmatter_value(&text) else {
        return (None, None);
    };
    let field = |key: &str| value.get(key).and_then(|v| v.as_str().map(str::to_string));
    (field("title"), field("content_class"))
}

/// Only the candidates a first screen may offer, in their producer's order
/// ([`offered_on_first_screen`] — one rule for every producer).
fn offered(contract: &Contract, candidates: Vec<Value>) -> Vec<Value> {
    candidates
        .into_iter()
        .filter(|c| {
            c["path"]
                .as_str()
                .is_some_and(|path| offered_on_first_screen(contract, path))
        })
        .collect()
}

/// A producer's first-screen call is part of the SCREEN, not the packet's explicit search
/// (contract v20): its usage is charged under its own keys — `first_screen_<producer>_calls`
/// plus the producer's measured ones — and never to `search_queries`.
fn charge_screen_call(usage: &mut Value, producer: &str, answer: &ProviderResult) {
    let mut charged = answer.usage.clone();
    if let Some(fields) = charged.as_object_mut() {
        fields.remove("search_queries");
        fields.insert(format!("first_screen_{producer}_calls"), json!(1));
    }
    add_usage(usage, &charged);
}

/// One omission line on the screen; a line already there is not repeated (two producers reading
/// one stale fold each report the same lag — the screen says it once).
fn push_omission(screen: &mut Value, line: String) {
    if !screen["omissions"].is_array() {
        screen["omissions"] = json!([]);
    }
    if let Some(omissions) = screen["omissions"].as_array_mut() {
        let line = json!(line);
        if !omissions.contains(&line) {
            omissions.push(line);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(path: &str) -> Value {
        json!({"path": path})
    }

    fn order(fused: &[Fused]) -> Vec<&str> {
        fused.iter().map(|f| f.path.as_str()).collect()
    }

    /// RRF with k = 60: Σ 1/(60 + rank) over the producers that returned a path.
    fn rrf(ranks: &[usize]) -> f64 {
        ranks.iter().map(|r| 1.0 / (60.0 + *r as f64)).sum()
    }

    #[test]
    fn the_fused_order_is_reciprocal_rank_over_the_producer_orders() {
        let local = vec![candidate("a.md"), candidate("b.md"), candidate("c.md")];
        let semantic = vec![candidate("c.md"), candidate("d.md"), candidate("a.md")];
        let fused = fuse(
            &[("local".into(), local), ("semantic".into(), semantic)],
            60,
        );
        // a: 1/61 + 1/63; c: 1/63 + 1/61 — equal, path decides; b: 1/62; d: 1/62 — path decides.
        assert_eq!(order(&fused), vec!["a.md", "c.md", "b.md", "d.md"]);
        assert!((fused[0].score - rrf(&[1, 3])).abs() < 1e-12);
        assert!((fused[2].score - rrf(&[2])).abs() < 1e-12);
        assert_eq!(
            fused[3].ranks,
            vec![
                ("local".to_string(), None),
                ("semantic".to_string(), Some(2))
            ]
        );
        assert_eq!(
            fused[0].candidate["ranks"],
            json!({"local": 1, "semantic": 3})
        );
        assert_eq!(
            fused[3].candidate["ranks"],
            json!({"local": null, "semantic": 2})
        );
    }

    /// Ruling 3: no producer's raw score, no term count and no standing or human signal enters
    /// the order — a candidate carrying a `standing` field, and one with a huge local `term_hits`,
    /// fuse exactly as their ranks say.
    #[test]
    fn standing_and_raw_scores_never_enter_the_order() {
        let local = vec![
            json!({"path": "low.md", "term_hits": 1}),
            json!({"path": "huge.md", "term_hits": 1_000_000, "score": 99.0}),
        ];
        let semantic = vec![
            json!({"path": "stood.md", "standing": {"attested": 1000, "reach": "commons"},
                   "score": 0.99}),
            json!({"path": "low.md", "score": 0.01}),
        ];
        let fused = fuse(
            &[("local".into(), local), ("semantic".into(), semantic)],
            60,
        );
        // low: 1/61 + 1/62; huge: 1/62; stood: 1/61 — ranks alone.
        assert_eq!(order(&fused), vec!["low.md", "stood.md", "huge.md"]);
        let bare = fuse(
            &[
                (
                    "local".into(),
                    vec![candidate("low.md"), candidate("huge.md")],
                ),
                (
                    "semantic".into(),
                    vec![candidate("stood.md"), candidate("low.md")],
                ),
            ],
            60,
        );
        assert_eq!(
            order(&fused),
            order(&bare),
            "the fields never move the order"
        );
        let scores: Vec<f64> = fused.iter().map(|f| f.score).collect();
        let bare_scores: Vec<f64> = bare.iter().map(|f| f.score).collect();
        assert_eq!(scores, bare_scores);
        // The signal is carried, never summed.
        assert_eq!(fused[1].candidate["standing"]["attested"], 1000);
    }

    /// A semantic-only candidate enters with its own `best_section`; a local-only one keeps its
    /// own; one both returned is shaped by the first producer and keeps the other's fields.
    #[test]
    fn each_candidate_keeps_its_producer_native_fields() {
        let local = vec![
            json!({"path": "both.md", "title": "Both", "best_section": {"lines": "1:2"}}),
            json!({"path": "lex.json", "title": "lex.json", "best_section": {"lines": "3:4"}}),
        ];
        let semantic = vec![
            json!({"path": "sem.md", "producer": "semantic", "method": "m",
                   "best_section": {"lines": "5:9"}}),
            json!({"path": "both.md", "producer": "semantic", "method": "m",
                   "best_section": {"lines": "7:8"}}),
        ];
        let fused = fuse(
            &[("local".into(), local), ("semantic".into(), semantic)],
            60,
        );
        let by = |path: &str| {
            fused
                .iter()
                .find(|f| f.path == path)
                .unwrap_or_else(|| panic!("{path} fused"))
                .candidate
                .clone()
        };
        assert_eq!(by("sem.md")["best_section"]["lines"], "5:9");
        assert_eq!(by("sem.md")["producer"], "semantic");
        assert_eq!(by("lex.json")["best_section"]["lines"], "3:4");
        let both = by("both.md");
        assert_eq!(
            both["best_section"]["lines"], "1:2",
            "the first producer shapes it"
        );
        assert_eq!(both["title"], "Both");
        assert_eq!(both["native"]["semantic"]["best_section"]["lines"], "7:8");
        assert!(both["native"]["semantic"].get("path").is_none());
        assert!(
            both.get("producer").is_none(),
            "never relabelled by a later producer"
        );
    }

    /// Integration fix (station 4): a fused candidate's passage comes from the producer that
    /// ranked it higher — the linked read lands where the ranking producer matched. Ties go to
    /// `local`; the displaced passage is kept under `native.<producer>`, never lost.
    #[test]
    fn the_passage_follows_the_producer_that_ranked_the_path_higher() {
        let local = vec![
            json!({"path": "first.md", "best_section": {"lines": "1:2", "title": "L first"}}),
            json!({"path": "hook.py", "title": "hook.py",
                   "best_section": {"lines": "188:237", "title": "def emit"}}),
            json!({"path": "tie.md", "best_section": {"lines": "5:6", "title": "L tie"}}),
            json!({"path": "lex.md", "best_section": {"lines": "9:9", "title": "L only"}}),
        ];
        let semantic = vec![
            json!({"path": "hook.py", "producer": "semantic",
                   "best_section": {"lines": "40:71", "title": "def resolve_bin"}}),
            json!({"path": "first.md", "producer": "semantic",
                   "best_section": {"lines": "30:31", "title": "S first"}}),
            json!({"path": "tie.md", "producer": "semantic",
                   "best_section": {"lines": "7:8", "title": "S tie"}}),
            json!({"path": "sem.md", "producer": "semantic",
                   "best_section": {"lines": "3:4", "title": "S only"}}),
        ];
        let fused = fuse(
            &[("local".into(), local), ("semantic".into(), semantic)],
            60,
        );
        let by = |path: &str| {
            fused
                .iter()
                .find(|f| f.path == path)
                .unwrap_or_else(|| panic!("{path} fused"))
                .candidate
                .clone()
        };
        // local #2 · semantic #1: the semantic chunk is the passage.
        let hook = by("hook.py");
        assert_eq!(hook["best_section"]["lines"], "40:71", "{hook}");
        assert_eq!(hook["best_section"]["title"], "def resolve_bin");
        assert_eq!(hook["passage_by"], "semantic");
        assert_eq!(
            hook["title"], "hook.py",
            "the candidate is still shaped by local"
        );
        assert_eq!(
            hook["native"]["local"]["best_section"]["lines"], "188:237",
            "the displaced passage is kept, never lost"
        );
        // local #1 · semantic #2: local keeps it.
        let first = by("first.md");
        assert_eq!(first["best_section"]["lines"], "1:2");
        assert_eq!(first["passage_by"], "local");
        assert_eq!(
            first["native"]["semantic"]["best_section"]["lines"],
            "30:31"
        );
        // local #3 · semantic #3: a tie goes to local.
        let tie = by("tie.md");
        assert_eq!(tie["best_section"]["lines"], "5:6");
        assert_eq!(tie["passage_by"], "local");
        // One producer: its own passage, named.
        assert_eq!(by("lex.md")["passage_by"], "local");
        assert_eq!(by("sem.md")["passage_by"], "semantic");
        assert_eq!(by("sem.md")["best_section"]["lines"], "3:4");
    }

    /// A better-ranked producer that located no passage does not erase the other's.
    #[test]
    fn a_producer_that_located_no_passage_never_takes_it() {
        let local = vec![
            json!({"path": "x.md"}),
            json!({"path": "y.md", "best_section": {"lines": "2:3", "title": "L y"}}),
        ];
        let semantic = vec![
            json!({"path": "y.md", "producer": "semantic"}),
            json!({"path": "x.md", "producer": "semantic"}),
        ];
        let fused = fuse(
            &[("local".into(), local), ("semantic".into(), semantic)],
            60,
        );
        let y = &fused.iter().find(|f| f.path == "y.md").unwrap().candidate;
        assert_eq!(y["best_section"]["lines"], "2:3", "{y}");
        assert_eq!(y["passage_by"], "local");
        let x = &fused.iter().find(|f| f.path == "x.md").unwrap().candidate;
        assert!(
            x.get("best_section").is_none() && x.get("passage_by").is_none(),
            "{x}"
        );
    }

    /// Station 4 integration (Task 4.8 ruling): the question bank is the exam sheet, not an
    /// answer — the path the contract's own `question_bank` names is never offered on a first
    /// screen, read from the contract, never a literal; the habit register stays refused.
    #[test]
    fn the_declared_question_bank_is_never_offered_on_a_first_screen() {
        let mut value = crate::flow::memory::recall::tests_support::minimal_contract();
        let bank = value["question_bank"].as_str().expect("bank").to_string();
        assert_eq!(bank, ".epr-meta/elohim/algorithms/recall-questions.json");
        let contract = Contract::from_value(value.clone()).unwrap();
        assert!(!offered_on_first_screen(&contract, &bank));
        assert!(!offered_on_first_screen(
            &contract,
            "genesis/manifests/habits.yaml"
        ));
        assert!(offered_on_first_screen(&contract, "genesis/exam.json"));

        value["question_bank"] = json!("genesis/exam.json");
        let moved = Contract::from_value(value.clone()).unwrap();
        assert!(!offered_on_first_screen(&moved, "genesis/exam.json"));
        assert!(
            offered_on_first_screen(&moved, &bank),
            "the rule reads the contract's declaration, not a literal"
        );

        value.as_object_mut().unwrap().remove("question_bank");
        let none = Contract::from_value(value).unwrap();
        assert!(offered_on_first_screen(&none, &bank));
        assert!(!offered_on_first_screen(
            &none,
            "genesis/manifests/habits.yaml"
        ));

        let offered_paths: Vec<Value> = offered(
            &contract,
            vec![
                json!({"path": bank}),
                json!({"path": "genesis/exam.json"}),
                json!({"path": "genesis/manifests/habits.yaml"}),
            ],
        );
        assert_eq!(offered_paths, vec![json!({"path": "genesis/exam.json"})]);
    }

    #[test]
    fn a_path_repeated_by_one_producer_is_ranked_once_and_an_empty_producer_fuses() {
        let fused = fuse(
            &[
                (
                    "local".into(),
                    vec![candidate("a.md"), candidate("a.md"), candidate("b.md")],
                ),
                ("semantic".into(), Vec::new()),
            ],
            60,
        );
        assert_eq!(order(&fused), vec!["a.md", "b.md"]);
        assert_eq!(fused[1].ranks[0], ("local".to_string(), Some(2)));
        assert_eq!(fused[1].ranks[1], ("semantic".to_string(), None));
    }

    /// Two producers reading one stale fold each report its lag; the screen says it once.
    #[test]
    fn an_identical_omission_line_is_printed_once() {
        let mut screen = json!({"omissions": ["fold 3 files behind"]});
        push_omission(&mut screen, "fold 3 files behind".to_string());
        push_omission(&mut screen, "lexical: no fold".to_string());
        assert_eq!(
            screen["omissions"],
            json!(["fold 3 files behind", "lexical: no fold"])
        );
    }

    #[test]
    fn the_recipe_is_declared_and_its_cid_is_the_objects_atom_cid() {
        let mut value = crate::flow::memory::recall::tests_support::minimal_contract();
        let object = json!({"recipe": "rrf-v1", "k": 60, "producers": ["local", "semantic"],
                            "order_only": true});
        value["discovery"]["first_screen_fusion"] = object.clone();
        let contract = Contract::from_value(value.clone()).unwrap();
        let recipe = Recipe::declared(&contract)
            .expect("declared")
            .expect("runs");
        assert_eq!(recipe.name, RRF_V1);
        assert_eq!(recipe.k, 60);
        assert_eq!(recipe.producers, vec!["local", "semantic"]);
        assert_eq!(recipe.cid, atom_cid(&object).unwrap().to_string());

        for (bad, why) in [
            (
                json!({"recipe": "sum-v1", "k": 60, "producers": ["local", "semantic"],
                    "order_only": true}),
                "rrf-v1",
            ),
            (
                json!({"recipe": "rrf-v1", "k": 60, "producers": ["local", "semantic"],
                    "order_only": false}),
                "order",
            ),
            (
                json!({"recipe": "rrf-v1", "k": 0, "producers": ["local", "semantic"],
                    "order_only": true}),
                "k",
            ),
            (
                json!({"recipe": "rrf-v1", "k": 60, "producers": ["semantic"],
                    "order_only": true}),
                "local",
            ),
        ] {
            value["discovery"]["first_screen_fusion"] = bad;
            let contract = Contract::from_value(value.clone()).unwrap();
            let refused = Recipe::declared(&contract).expect("declared").unwrap_err();
            assert!(refused.contains(why), "{refused}");
        }
        value["discovery"]
            .as_object_mut()
            .unwrap()
            .remove("first_screen_fusion");
        let contract = Contract::from_value(value).unwrap();
        assert!(Recipe::declared(&contract).is_none());
    }
}

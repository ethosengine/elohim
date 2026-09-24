//! FUSE — reciprocal-rank fusion for ORDER, and the declared recipe it runs under.
//!
//! The recipe is a declaration, not a constant: an object like `{"recipe": "rrf-v1", "k": 60,
//! "producers": ["local", "semantic"], "order_only": true}`, and its method CID is `atom_cid` of
//! that object — the canonical dag-cbor address every fused answer prints beside its own.
//!
//! Two recipes run here:
//!
//! - **`rrf-v1`** — the recall executor's first screen: the producers begin with `local` (the
//!   screen's own lexical order) and name at least one other;
//! - **`rrf-v2`** — any non-empty producer list, so a single producer (the storage peer's lexical
//!   fold) is a recipe whose CID an answer can print.
//!
//! **Order only.** A unit's fused score is Σ 1/(k + rank) over the producers that returned it
//! (rank 1-based, in each producer's own order); ties break by path. Nothing a candidate carries
//! — no producer's raw score, no term count, no standing, no human signal — is read by [`fuse`]:
//! it sees each producer's ORDER and the path, and nothing else.
//!
//! Lifted out of the recall executor (post-station-4 sprint, ruling R-S1): [`fuse`] is unchanged;
//! [`Recipe::from_declared`] gained the `rrf-v2` arm.
use std::collections::{BTreeMap, BTreeSet};

use elohim_epr_rea::atom_cid;
use serde_json::{json, Map, Value};

/// The recall executor's first-screen recipe: `local` first, and another producer.
pub const RRF_V1: &str = "rrf-v1";

/// Any non-empty producer list — a single producer included.
pub const RRF_V2: &str = "rrf-v2";

/// The producer whose order is the recall first screen's own lexical order.
pub const LOCAL: &str = "local";

/// The declared fusion recipe, read and checked.
#[derive(Debug, Clone, PartialEq)]
pub struct Recipe {
    pub name: String,
    pub k: u64,
    pub producers: Vec<String>,
    /// `atom_cid` of the declared object — the method every fused screen names.
    pub cid: String,
}

impl Recipe {
    /// Read and check a declared recipe object; `Err(reason)` when it is not one of the recipes
    /// this crate runs.
    pub fn from_declared(object: &Value) -> Result<Self, String> {
        let name = object["recipe"].as_str().unwrap_or_default();
        if name != RRF_V1 && name != RRF_V2 {
            return Err(format!(
                "fusion: the recipe declares `{name}`; the recipes are {RRF_V1} and {RRF_V2}"
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
        if name == RRF_V2 && producers.is_empty() {
            return Err("fusion: the recipe names no producer".to_string());
        }
        if name == RRF_V1
            && (producers.first().map(String::as_str) != Some(LOCAL) || producers.len() < 2)
        {
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
pub struct Fused {
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
pub fn fuse(producers: &[(String, Vec<Value>)], k: u64) -> Vec<Fused> {
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

//! `GET /db/content/search` — one answer over the content lexical fold (post-station-4 sprint,
//! ruling R-S4, route half).
//!
//! [`answer`] runs the declared method end to end and prints it:
//!
//! 1. the question's terms ([`question_terms`]; this recipe declares no short vocabulary, so a
//!    term under four characters is named in `omissions`, never silently searched or dropped) and
//!    their FTS5 expression ([`match_expression`]);
//! 2. the fold's bm25 order, best chunk per unit ([`BestPerUnit`]), fused for ORDER by the
//!    declared `rrf-v2` recipe ([`fuse`]) — one producer, `lexical`, under the measure's CID;
//! 3. each ranked unit joined to its content row at the serving trust floor
//!    ([`MinTrust::Amber`], the same floor `GET /db/content` reads at), then the reader's own
//!    filters (`contentType`, `reach`, `tags`);
//! 4. the reach gate, per candidate, AFTER ranking and BEFORE any snippet is read — a withheld
//!    row contributes nothing to the answer but one counted line in `omissions`;
//! 5. facets over the admitted set only; the lens level's `choice_count` cut, then the page
//!    (`offset`, `limit` clamped to `1..=100`); a snippet (≤ [`SNIPPET_CHARS`]) for the page only.
//!
//! The fold and its lag are answer envelopes: no attestation beside the store is an observed
//! absence (`rankingKnown: false`, no candidates), never an empty list dressed as a ranking. A lens
//! is never refused (an unknown level reads at the default, `provenance: "defaulted"`); a recipe
//! pin that names another recipe still answers, with an `unresolved` line.
use std::collections::{BTreeMap, BTreeSet, HashMap};

use diesel::SqliteConnection;
use elohim_epr_index::fuse::fuse;
use elohim_epr_index::rank::BestPerUnit;
use elohim_epr_index::terms::{match_expression, question_terms, stem, STOPWORDS};
use elohim_epr_rea::FoldState;
use elohim_views::{
    ContentSearchAbsentReason, ContentSearchCandidateView, ContentSearchFacetsView,
    ContentSearchFoldAnswer, ContentSearchFoldLagAnswer, ContentSearchFoldLagView,
    ContentSearchFoldView, ContentSearchLensView, ContentSearchProducerView,
    ContentSearchRecipeView, ContentSearchSectionView, ContentSearchUnreachableReason,
    ContentSearchView, FacetCountView,
};
use serde::Deserialize;
use serde_json::json;

use super::fold::SearchIndex;
use super::measure::{Declared, LEXICAL_PRODUCER};
use super::reader::{Reader, ANONYMOUS_TIER};
use crate::db::content_diesel::{get_content_with_tags, MinTrust};
use crate::db::AppContext;
use crate::views::ContentView;

/// The page size when the reader names none.
pub const DEFAULT_LIMIT: u32 = 20;

/// The largest page the route serves; a larger `limit` is clamped, never refused.
pub const MAX_LIMIT: u32 = 100;

/// The most characters a snippet carries (ellipses included).
pub const SNIPPET_CHARS: usize = 240;

/// How much text a snippet keeps before the first matched term.
const SNIPPET_LEAD: usize = 60;

/// The measure's fold-lag unit.
const LAG_UNIT: &str = "units";

/// The query string of `GET /db/content/search`, camelCase on the wire. `tags` is one
/// comma-separated parameter; every other field is scalar.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentSearchQuery {
    #[serde(default)]
    pub q: String,
    #[serde(default)]
    pub lens: Option<String>,
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub reach: Option<String>,
    #[serde(default)]
    pub tags: Option<String>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
    #[serde(default)]
    pub recipe: Option<String>,
}

fn given(value: &Option<String>) -> Option<&str> {
    value.as_deref().map(str::trim).filter(|v| !v.is_empty())
}

impl ContentSearchQuery {
    /// Parse a raw query string; `Err` names the parameter that is not what it says it is.
    pub fn parse(query_str: &str) -> Result<Self, String> {
        serde_urlencoded::from_str(query_str).map_err(|e| format!("invalid search query: {e}"))
    }

    /// The page size: `limit` clamped to `1..=MAX_LIMIT`, [`DEFAULT_LIMIT`] when absent.
    pub fn limit(&self) -> u32 {
        match self.limit {
            None => DEFAULT_LIMIT,
            Some(n) => n.clamp(1, i64::from(MAX_LIMIT)) as u32,
        }
    }

    /// The page offset, never negative.
    pub fn offset(&self) -> u64 {
        self.offset.unwrap_or(0).max(0) as u64
    }

    /// The tag filter: the comma-separated `tags`, trimmed, empties dropped.
    pub fn tag_filter(&self) -> Vec<String> {
        self.tags
            .as_deref()
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_string)
            .collect()
    }

    fn keeps(&self, row: &ContentView, tags: &[String]) -> bool {
        given(&self.content_type).is_none_or(|ct| row.content_type == ct)
            && given(&self.reach).is_none_or(|r| row.reach == r)
            && (tags.is_empty() || tags.iter().any(|t| row.tags.contains(t)))
    }
}

/// The reach gate an answer consults per candidate: `(conn, reach, content_id) -> admitted`.
pub type ReachGate<'a> = dyn FnMut(&mut SqliteConnection, &str, &str) -> bool + 'a;

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// The frame every answer carries, whatever the fold holds: the question, the recipe, the lens it
/// resolved, and the lines for any parameter it could not honour as asked. Nothing is ranked.
fn frame(declared: &Declared, query: &ContentSearchQuery) -> ContentSearchView {
    let mut unresolved = Vec::new();
    let (level, provenance) = match given(&query.lens) {
        Some(level) if declared.lens_levels.contains_key(level) => (level.to_string(), "requested"),
        Some(level) => {
            unresolved.push(format!(
                "lens `{level}` is not declared by the recipe's lens table; read at `{}`",
                declared.lens_default
            ));
            (declared.lens_default.clone(), "defaulted")
        }
        None => (declared.lens_default.clone(), "defaulted"),
    };
    let choice_count = declared
        .lens_levels
        .get(&level)
        .map_or(0, |l| l.choice_count);
    if let Some(pinned) = given(&query.recipe) {
        if pinned != declared.recipe.cid {
            unresolved.push(format!(
                "recipe pinned {pinned} is not the served {}",
                declared.recipe.cid
            ));
        }
    }
    ContentSearchView {
        query: query.q.clone(),
        ranking_known: false,
        recipe: ContentSearchRecipeView {
            name: declared.recipe.name.clone(),
            cid: declared.recipe.cid.clone(),
            k: declared.recipe.k,
            order_only: true,
            producers: declared
                .recipe
                .producers
                .iter()
                .map(|id| ContentSearchProducerView {
                    id: id.clone(),
                    method: declared.measure_cid.clone(),
                })
                .collect(),
        },
        lens: ContentSearchLensView {
            level,
            choice_count,
            cid: declared.lens_cid.clone(),
            provenance: provenance.to_string(),
        },
        selection: "nothing ranked".to_string(),
        fold: ContentSearchFoldAnswer::Absent {
            reason: ContentSearchAbsentReason::ObservedAbsent,
        },
        fold_lag: ContentSearchFoldLagAnswer::Absent {
            reason: ContentSearchAbsentReason::ObservedAbsent,
        },
        candidates: Vec::new(),
        facets: ContentSearchFacetsView::default(),
        omissions: Vec::new(),
        unresolved,
        total_count: 0,
    }
}

/// The answer of a peer that runs no fold at all (its declarations were refused at boot, or its
/// store could not be opened): the recipe is still printed, the fold is unreachable (`refused`),
/// and nothing is ranked.
pub fn answer_unlit(declared: &Declared, query: &ContentSearchQuery) -> ContentSearchView {
    let mut view = frame(declared, query);
    view.fold = ContentSearchFoldAnswer::Unreachable {
        reason: ContentSearchUnreachableReason::Refused,
    };
    view.fold_lag = ContentSearchFoldLagAnswer::Unreachable {
        reason: ContentSearchUnreachableReason::Refused,
    };
    view.selection = "nothing ranked: this peer runs no content fold".to_string();
    view.unresolved
        .push("content search is not lit on this peer; no fold was read".to_string());
    view
}

/// The question's tokens under four characters that were not searched (this recipe declares no
/// short vocabulary), tokenized as [`question_terms`] tokenizes.
fn short_tokens(need: &str, kept: &[String]) -> Vec<String> {
    let mut short: Vec<String> = Vec::new();
    for raw in need.split(|c: char| {
        !(c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '/' || c == '.')
    }) {
        let token = raw
            .trim_matches(|c| c == '.' || c == '/')
            .to_ascii_lowercase();
        if token.is_empty()
            || token.len() >= 4
            || STOPWORDS.contains(&token.as_str())
            || kept.contains(&token)
            || short.contains(&token)
        {
            continue;
        }
        short.push(token);
    }
    short
}

/// A short excerpt of `text` around the first place a term's stem occurs, at most
/// [`SNIPPET_CHARS`] characters, whitespace collapsed.
pub fn snippet(text: &str, terms: &[String]) -> String {
    let flat: Vec<char> = text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .collect();
    if flat.len() <= SNIPPET_CHARS {
        return flat.into_iter().collect();
    }
    // Lower-case per character so positions stay aligned with `flat`.
    let lower: Vec<char> = flat
        .iter()
        .map(|c| c.to_lowercase().next().unwrap_or(*c))
        .collect();
    let hit = terms
        .iter()
        .filter_map(|term| {
            let needle: Vec<char> = stem(term)
                .chars()
                .map(|c| c.to_lowercase().next().unwrap_or(c))
                .collect();
            if needle.is_empty() || needle.len() > lower.len() {
                return None;
            }
            lower
                .windows(needle.len())
                .position(|w| w == needle.as_slice())
        })
        .min()
        .unwrap_or(0);
    let start = hit.saturating_sub(SNIPPET_LEAD);
    let lead = if start > 0 { "…" } else { "" };
    let room = SNIPPET_CHARS - lead.chars().count();
    let mut end = (start + room).min(flat.len());
    let tail = if end < flat.len() {
        end -= 1;
        "…"
    } else {
        ""
    };
    format!(
        "{lead}{}{tail}",
        flat[start..end].iter().collect::<String>().trim()
    )
}

fn facet(counts: BTreeMap<String, u64>) -> Vec<FacetCountView> {
    let mut facet: Vec<FacetCountView> = counts
        .into_iter()
        .map(|(value, count)| FacetCountView { value, count })
        .collect();
    facet.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.value.cmp(&b.value)));
    facet
}

fn fold_state(state: &FoldState) -> &'static str {
    match state {
        FoldState::Complete => "complete",
        FoldState::Degraded { .. } => "degraded",
        FoldState::Failed { .. } => "failed",
    }
}

/// One search answer over `index` for `reader` (see the module docs). `admits` is the reach gate,
/// consulted once per candidate that passed the trust floor and the reader's filters.
pub fn answer(
    index: &SearchIndex,
    conn: &mut SqliteConnection,
    reader: &Reader,
    query: &ContentSearchQuery,
    admits: &mut ReachGate<'_>,
) -> ContentSearchView {
    let declared = index.declared();
    let mut view = frame(declared, query);

    // The fold: no attestation beside the store is an observed absence — nothing is ranked.
    let snapshot = index.snapshot();
    let (Some(attestation), Some(attestation_cid)) = (
        snapshot.attestation.as_ref(),
        snapshot.attestation_cid.clone(),
    ) else {
        view.selection =
            "nothing ranked: this peer holds no attested fold of its content yet".to_string();
        return view;
    };
    view.fold = ContentSearchFoldAnswer::Present {
        value: ContentSearchFoldView {
            measure: declared.measure_cid.clone(),
            state: fold_state(&attestation.state).to_string(),
            attestation_cid,
            at: attestation.at.max(0),
        },
    };
    let limit = declared.fold_lag_limit();
    view.fold_lag = match index.behind(conn) {
        Ok(behind) => {
            if behind > limit {
                view.omissions.push(format!(
                    "the fold is {} behind the content rows, past its declared {limit}",
                    plural(behind as usize, "row", "rows")
                ));
            }
            ContentSearchFoldLagAnswer::Present {
                value: ContentSearchFoldLagView {
                    behind,
                    limit,
                    unit: LAG_UNIT.to_string(),
                    within: behind <= limit,
                },
            }
        }
        Err(why) => {
            view.unresolved.push(format!("fold lag unmeasured: {why}"));
            ContentSearchFoldLagAnswer::Unreachable {
                reason: ContentSearchUnreachableReason::Unverifiable,
            }
        }
    };

    // The question's terms.
    let terms = question_terms(&BTreeSet::new(), &query.q);
    let short = short_tokens(&query.q, &terms);
    if !short.is_empty() {
        view.omissions.push(format!(
            "terms under four characters not searched: {} (this recipe declares no short \
             vocabulary)",
            short.join(", ")
        ));
    }
    let Some(expression) = match_expression(&terms) else {
        view.unresolved.push(if query.q.trim().is_empty() {
            "no question asked (q is empty); nothing ranked".to_string()
        } else {
            "no searchable term in the question; nothing ranked".to_string()
        });
        view.selection = "nothing ranked: no searchable term".to_string();
        return view;
    };

    // The lexical producer's order: each unit's best chunk by negated bm25 (higher is better).
    let mut best = BestPerUnit::default();
    let read = index.with_store(|store| {
        store.lexical(&expression, |id, unit_id, rank| {
            best.offer(id, unit_id, -rank)
        })
    });
    if let Err(error) = read {
        view.fold = ContentSearchFoldAnswer::Unreachable {
            reason: ContentSearchUnreachableReason::Unverifiable,
        };
        view.unresolved
            .push(format!("the fold store could not be read: {error}"));
        view.selection = "nothing ranked: the fold store could not be read".to_string();
        return view;
    }
    let hits = best.ranked();
    let chunk_of: HashMap<String, i64> = hits.iter().map(|h| (h.unit_id.clone(), h.id)).collect();
    let order = hits.iter().map(|h| json!({ "path": h.unit_id })).collect();
    let fused = fuse(&[(LEXICAL_PRODUCER.to_string(), order)], declared.recipe.k);
    view.ranking_known = true;

    // Join, filter, then the reach gate — before any snippet is read.
    let ctx = AppContext::default_lamad();
    let tag_filter = query.tag_filter();
    let mut not_served = 0usize;
    let mut refused = 0usize;
    let mut admitted: Vec<(f64, ContentView)> = Vec::new();
    for candidate in &fused {
        let row: ContentView =
            match get_content_with_tags(conn, &ctx, &candidate.path, MinTrust::Amber) {
                Ok(Some(row)) => row.into(),
                Ok(None) | Err(_) => {
                    not_served += 1;
                    continue;
                }
            };
        if !query.keeps(&row, &tag_filter) {
            continue;
        }
        if !admits(conn, &row.reach, &row.id) {
            refused += 1;
            continue;
        }
        admitted.push((candidate.score, row));
    }
    if refused > 0 {
        let who = match (&reader.agent_cid, reader.tier.as_str()) {
            (None, _) => "an anonymous reader",
            (Some(_), ANONYMOUS_TIER) => "a reader this peer could not resolve",
            (Some(_), _) => "this reader",
        };
        view.omissions.push(format!(
            "{} withheld by the reach gate for {who}",
            plural(refused, "ranked candidate", "ranked candidates")
        ));
    }
    if not_served > 0 {
        view.omissions.push(format!(
            "{} not served: below the serving trust floor, or gone since the fold",
            plural(not_served, "ranked unit", "ranked units")
        ));
    }

    // Facets over the admitted set only.
    let mut by_type = BTreeMap::<String, u64>::new();
    let mut by_reach = BTreeMap::<String, u64>::new();
    let mut by_tag = BTreeMap::<String, u64>::new();
    for (_, row) in &admitted {
        *by_type.entry(row.content_type.clone()).or_default() += 1;
        *by_reach.entry(row.reach.clone()).or_default() += 1;
        for tag in row.tags.iter().collect::<BTreeSet<_>>() {
            *by_tag.entry(tag.clone()).or_default() += 1;
        }
    }
    view.facets = ContentSearchFacetsView {
        content_type: facet(by_type),
        reach: facet(by_reach),
        tags: facet(by_tag),
    };

    // The lens cut, then the page.
    let choices = view.lens.choice_count as usize;
    if admitted.len() > choices {
        view.omissions.push(format!(
            "{} beyond the {} lens's {choices} choices",
            plural(
                admitted.len() - choices,
                "admitted candidate",
                "admitted candidates"
            ),
            view.lens.level
        ));
        admitted.truncate(choices);
    }
    view.total_count = admitted.len() as u64;
    let (offset, page_size) = (query.offset() as usize, query.limit() as usize);
    let page: Vec<(f64, ContentView)> = admitted.into_iter().skip(offset).take(page_size).collect();

    // Snippets for the page only.
    let sections: HashMap<String, (String, String)> = index.with_store(|store| {
        page.iter()
            .filter_map(|(_, row)| {
                let id = chunk_of.get(&row.id)?;
                store
                    .chunk(*id)
                    .ok()
                    .map(|section| (row.id.clone(), section))
            })
            .collect()
    });
    let shown = page.len();
    view.candidates = page
        .into_iter()
        .map(|(score, row)| {
            let best_section =
                sections
                    .get(&row.id)
                    .map(|(title, text)| ContentSearchSectionView {
                        title: title.clone(),
                        snippet: snippet(text, &terms),
                    });
            ContentSearchCandidateView {
                content_id: row.id,
                title: row.title,
                content_type: row.content_type,
                reach: row.reach,
                trust: row.trust,
                tags: row.tags,
                score,
                producer: LEXICAL_PRODUCER.to_string(),
                method: declared.measure_cid.clone(),
                best_section,
            }
        })
        .collect();
    view.selection = format!(
        "{shown} of {} admitted candidates (offset {offset}, limit {page_size}) in {} order over \
         the {LEXICAL_PRODUCER} fold's bm25; the {} lens keeps the first {choices}",
        view.total_count, declared.recipe.name, view.lens.level
    );
    view
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snippet_is_short_and_lands_on_the_term() {
        let long = format!("{} watershed {}", "a ".repeat(300), "b ".repeat(300));
        let cut = snippet(&long, &["watershed".to_string()]);
        assert!(
            cut.chars().count() <= SNIPPET_CHARS,
            "{}",
            cut.chars().count()
        );
        assert!(cut.contains("watershed"), "{cut}");
        assert!(cut.starts_with('…') && cut.ends_with('…'), "{cut}");
        assert_eq!(snippet("  short\n text ", &[]), "short text");
    }

    #[test]
    fn short_tokens_are_named_not_searched() {
        let terms = question_terms(&BTreeSet::new(), "the DAO watershed");
        assert_eq!(terms, ["watershed"]);
        assert_eq!(short_tokens("the DAO watershed", &terms), ["the", "dao"]);
    }

    #[test]
    fn query_parses_camel_case_and_clamps() {
        let q = ContentSearchQuery::parse(
            "q=soil&contentType=concept&reach=commons&tags=a,%20b,,&limit=500&offset=-3",
        )
        .unwrap();
        assert_eq!(q.content_type.as_deref(), Some("concept"));
        assert_eq!(q.tag_filter(), ["a", "b"]);
        assert_eq!(q.limit(), MAX_LIMIT);
        assert_eq!(q.offset(), 0);
        assert!(ContentSearchQuery::parse("limit=many").is_err());
    }
}

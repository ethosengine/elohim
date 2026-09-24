//! The governed declarations the content search runs under: the `content-lexical-index`
//! [`IndexMeasure`] and the `rrf-v2` content search recipe (post-station-4 sprint, ruling R-S2).
//!
//! Both live in `elohim/elohim-storage/.epr-meta/elohim/algorithms/` and are compiled in, so the
//! peer folds and ranks under exactly the bytes the repository governs. [`Declared::load`] runs at
//! boot and refuses a declaration this peer cannot execute honestly:
//!
//! - the measure must deserialize into [`IndexMeasure`] and pass [`IndexMeasure::validate`];
//! - its `_chunk_rule` object must address (`atom_cid`) to its `chunkRule` CID and parse as a
//!   [`ChunkRule`] — the method is the declaration, never an approximation of it;
//! - it ranks by `bm25` and pins no model (lexical only; semantic in storage is out);
//! - its surface is the `Content` kind and nothing else;
//! - the recipe must parse as an `rrf-v2` [`Recipe`] whose producers are exactly `lexical`, name
//!   this measure's file, and declare a lens table whose default is one of its levels.
//!
//! **What each CID addresses.** The measure CID is [`IndexMeasure::cid`] over the typed value (the
//! underscore keys are documentation and are not part of it). The recipe CID is `atom_cid` over
//! [`RECIPE_ADDRESSED_KEYS`] only — `recipe`, `k`, `order_only`, `producers`: `lens_table` has its
//! own CID ([`Declared::lens_cid`]) and `measure` is addressed by the measure's own CID, so a lens
//! retune or a measure move re-addresses what it changes and nothing else.
use std::collections::BTreeMap;

use elohim_epr::kind::EprKind;
use elohim_epr_index::chunk::ChunkRule;
use elohim_epr_index::fuse::{Recipe, RRF_V2};
use elohim_epr_rea::{atom_cid, IndexMeasure, RankingMethod};
use serde_json::{Map, Value};

/// The measure declaration's bytes, compiled in.
pub const MEASURE_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/.epr-meta/elohim/algorithms/content-lexical-index.json"
));

/// The recipe declaration's bytes, compiled in.
pub const RECIPE_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/.epr-meta/elohim/algorithms/content-search-recipe.json"
));

/// The measure's file name; the recipe's `measure` path must end with it.
pub const MEASURE_FILE: &str = "content-lexical-index.json";

/// The one producer the recipe fuses: the lexical fold's bm25 order.
pub const LEXICAL_PRODUCER: &str = "lexical";

/// The recipe keys its CID addresses; every other key (the lens table, the measure path, the
/// underscore documentation) is addressed elsewhere or not at all.
pub const RECIPE_ADDRESSED_KEYS: [&str; 4] = ["recipe", "k", "order_only", "producers"];

/// The candidate cap's headroom when the recipe declares none: `(offset + limit)` times this,
/// floored at the lens level's `choice_count`.
pub const DEFAULT_CANDIDATE_HEADROOM: u32 = 4;

/// One level of the recipe's lens table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LensLevel {
    pub choice_count: u32,
}

/// The measure and recipe, loaded, checked, and addressed.
#[derive(Debug, Clone)]
pub struct Declared {
    pub measure: IndexMeasure,
    /// [`IndexMeasure::cid`], as its string.
    pub measure_cid: String,
    pub chunk_rule: ChunkRule,
    pub recipe: Recipe,
    /// The recipe's declared lens table: its default level and every level.
    pub lens_default: String,
    pub lens_levels: BTreeMap<String, LensLevel>,
    /// `atom_cid` of the declared `lens_table` object.
    pub lens_cid: String,
    /// The recipe's `candidate_headroom`: how many pages deep the fused set may be walked before
    /// the cut (see [`Declared::candidate_cap`]). Un-addressed, like the lens table — it bounds
    /// this peer's WORK, never the order it ranks in.
    pub candidate_headroom: u32,
}

fn refused(what: impl std::fmt::Display) -> String {
    format!("content search declaration refused: {what}")
}

impl Declared {
    /// The compiled-in declarations, checked. Called at boot; an `Err` names what is wrong.
    pub fn load() -> Result<Self, String> {
        Self::from_json(MEASURE_JSON, RECIPE_JSON)
    }

    /// Load from explicit bytes (the compiled-in pair at boot; a variant in tests).
    pub fn from_json(measure_json: &str, recipe_json: &str) -> Result<Self, String> {
        let raw: Value = serde_json::from_str(measure_json).map_err(refused)?;
        let measure: IndexMeasure = serde_json::from_value(raw.clone()).map_err(refused)?;
        measure.validate().map_err(refused)?;
        if measure.ranking != RankingMethod::Bm25 || measure.embedding.is_some() {
            return Err(refused(
                "the content fold is lexical: ranking bm25, no model pin",
            ));
        }
        if measure.surfaces.kinds() != [EprKind::Content] {
            return Err(refused(
                "the content fold's surface is the Content kind alone",
            ));
        }
        let rule = raw.get("_chunk_rule").unwrap_or(&Value::Null);
        match atom_cid(rule) {
            Ok(cid) if cid == measure.chunk_rule => {}
            _ => {
                return Err(refused(
                    "_chunk_rule does not address to chunkRule — the method is the declaration",
                ))
            }
        }
        let chunk_rule = ChunkRule::from_declared(rule).map_err(refused)?;
        let measure_cid = measure.cid().map_err(refused)?.to_string();

        let recipe_raw: Value = serde_json::from_str(recipe_json).map_err(refused)?;
        let recipe =
            Recipe::from_declared(&Value::Object(addressed(&recipe_raw))).map_err(refused)?;
        if recipe.name != RRF_V2 || recipe.producers != [LEXICAL_PRODUCER] {
            return Err(refused(format!(
                "the content search recipe is {RRF_V2} over `{LEXICAL_PRODUCER}` alone"
            )));
        }
        let names_measure = recipe_raw["measure"]
            .as_str()
            .is_some_and(|path| path.rsplit('/').next() == Some(MEASURE_FILE));
        if !names_measure {
            return Err(refused(format!(
                "the recipe's measure must name {MEASURE_FILE}"
            )));
        }
        let table = &recipe_raw["lens_table"];
        let lens_default = table["default"]
            .as_str()
            .ok_or_else(|| refused("lens_table.default is missing"))?
            .to_string();
        let mut lens_levels = BTreeMap::new();
        for (name, level) in table["levels"].as_object().into_iter().flatten() {
            let choice_count = level["choice_count"]
                .as_u64()
                .and_then(|n| u32::try_from(n).ok())
                .ok_or_else(|| refused(format!("lens level {name}: choice_count")))?;
            lens_levels.insert(name.clone(), LensLevel { choice_count });
        }
        if !lens_levels.contains_key(&lens_default) {
            return Err(refused("lens_table.default names no declared level"));
        }
        let lens_cid = atom_cid(table).map_err(refused)?.to_string();

        let candidate_headroom = match recipe_raw.get("candidate_headroom") {
            None => DEFAULT_CANDIDATE_HEADROOM,
            Some(value) => value
                .as_u64()
                .and_then(|n| u32::try_from(n).ok())
                .filter(|n| *n >= 1)
                .ok_or_else(|| {
                    refused("candidate_headroom is a whole number of pages, 1 or more")
                })?,
        };

        Ok(Self {
            measure,
            measure_cid,
            chunk_rule,
            recipe,
            lens_default,
            lens_levels,
            lens_cid,
            candidate_headroom,
        })
    }

    /// The fold-lag bound as whole units (declarations carry no floats; the protocol type does).
    pub fn fold_lag_limit(&self) -> u64 {
        self.measure.fold_lag.limit.max(0.0) as u64
    }

    /// The most ranked candidates one answer may examine: the reader's own page (`offset +
    /// limit`) times the declared [`Self::candidate_headroom`], never below the lens level's
    /// `choices` — the answer's own cut would otherwise be the wider of the two.
    ///
    /// This bounds the WORK, not the ranking: everything past the cap is behind the head of an
    /// order the reader is already paging, and the cut is named in the answer's `omissions`
    /// (ruling R-S11, delta review W1). Without it, a `limit=1` question walked the whole corpus,
    /// joining and reach-authorizing every matched row.
    pub fn candidate_cap(&self, offset: usize, limit: usize, choices: usize) -> usize {
        offset
            .saturating_add(limit)
            .saturating_mul(self.candidate_headroom as usize)
            .max(choices)
            .max(1)
    }
}

/// The recipe object restricted to [`RECIPE_ADDRESSED_KEYS`] — what its CID addresses.
pub fn addressed(recipe: &Value) -> Map<String, Value> {
    RECIPE_ADDRESSED_KEYS
        .iter()
        .filter_map(|key| recipe.get(*key).map(|v| (key.to_string(), v.clone())))
        .collect()
}

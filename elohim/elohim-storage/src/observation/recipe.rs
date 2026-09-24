//! The observation lifestream recipe — the governed declaration every
//! lifestream view renders through (ruling R-A4 of the post-station-4 plan).
//!
//! The recipe lives at
//! `elohim/elohim-storage/.epr-meta/elohim/algorithms/observation-lifestream-recipe.json`
//! and is compiled into the binary, so the node renders exactly the bytes the
//! repository governs. Its CID ([`recipe_cid`]) is BLAKE3 over those bytes in
//! the same `blake3:<hex>` rendering `ObservationLog` uses for `log_cid`; every
//! stream response prints it, so a person can see which recipe arranged their
//! stream. The recipe is the whole of the ranking: `audience` (`self` — only the
//! requester's own rows), `ranking` (recency, then dwell), the default window,
//! the lens table (`all | content | long-dwell`, thresholds included — the
//! browser holds none) and the omissions policy. Governed declarations carry no
//! floats; every threshold is an integer.

use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

/// The recipe bytes, compiled in.
pub const RECIPE_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/.epr-meta/elohim/algorithms/observation-lifestream-recipe.json"
));

/// The observation lifestream recipe (mirror of the JSON declaration).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LifestreamRecipe {
    pub id: String,
    pub version: u32,
    pub purpose: String,
    /// Whose rows the recipe may render: `self` — the requester's own only.
    pub audience: String,
    /// Sort keys in order: `recency` (newest first), then `dwell_ms` (longest first).
    pub ranking: Vec<String>,
    /// The window applied when a request names none (`Nd` | `Nh`).
    pub window_default: String,
    /// Lens name → the filter it applies.
    pub lenses: BTreeMap<String, LensSpec>,
    /// Omission name → what the view says when it applies.
    pub omissions_policy: BTreeMap<String, String>,
}

/// One lens: the rows it keeps. An empty lens keeps every row.
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LensSpec {
    /// Keep only this observation kind.
    #[serde(default)]
    pub kind: Option<String>,
    /// Keep only rows whose payload `dwell_ms` is at least this.
    #[serde(default)]
    pub min_dwell_ms: Option<u64>,
}

impl LifestreamRecipe {
    /// The lens names the recipe declares.
    pub fn lens_names(&self) -> BTreeSet<&str> {
        self.lenses.keys().map(String::as_str).collect()
    }
}

/// The parsed recipe. The bytes are compiled in and covered by this module's
/// tests, so a malformed recipe fails the tests, never a request.
pub fn recipe() -> &'static LifestreamRecipe {
    static RECIPE: OnceLock<LifestreamRecipe> = OnceLock::new();
    RECIPE.get_or_init(|| {
        serde_json::from_str(RECIPE_JSON)
            .expect("observation-lifestream-recipe.json matches LifestreamRecipe (tested)")
    })
}

/// `blake3:<hex>` over the recipe bytes — the CID every stream response prints.
pub fn recipe_cid() -> &'static str {
    static CID: OnceLock<String> = OnceLock::new();
    CID.get_or_init(|| format!("blake3:{}", blake3::hash(RECIPE_JSON.as_bytes()).to_hex()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECIPE_FILE: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/.epr-meta/elohim/algorithms/observation-lifestream-recipe.json"
    );

    #[test]
    fn recipe_cid_is_stable_and_matches_file_bytes() {
        let bytes = std::fs::read(RECIPE_FILE).expect("the recipe file exists");
        assert_eq!(
            RECIPE_JSON.as_bytes(),
            bytes.as_slice(),
            "compiled bytes = file bytes"
        );
        let expected = format!("blake3:{}", blake3::hash(&bytes).to_hex());
        println!("observation-lifestream recipe cid: {expected}");
        assert_eq!(recipe_cid(), expected);
        assert_eq!(recipe_cid(), recipe_cid(), "stable across calls");
        let hex = recipe_cid()
            .strip_prefix("blake3:")
            .expect("blake3: rendering");
        assert_eq!(hex.len(), 64);
        assert!(hex
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }

    #[test]
    fn recipe_declares_self_audience_and_lens_table() {
        let r = recipe();
        assert_eq!(r.id, "observation-lifestream");
        assert_eq!(r.version, 1);
        assert_eq!(r.audience, "self");
        assert_eq!(
            r.ranking,
            vec!["recency".to_string(), "dwell_ms".to_string()]
        );
        assert_eq!(r.window_default, "7d");
        let lenses: Vec<&str> = r.lenses.keys().map(String::as_str).collect();
        assert_eq!(lenses, vec!["all", "content", "long-dwell"]);
        assert_eq!(r.lenses["all"], LensSpec::default());
        assert_eq!(
            r.lenses["content"].kind.as_deref(),
            Some("lamad:content-viewed")
        );
        assert_eq!(r.lenses["content"].min_dwell_ms, None);
        assert_eq!(
            r.lenses["long-dwell"].kind.as_deref(),
            Some("lamad:content-viewed")
        );
        assert_eq!(r.lenses["long-dwell"].min_dwell_ms, Some(60_000));
        for key in ["signature", "window", "payload"] {
            assert!(
                r.omissions_policy.contains_key(key),
                "omissions_policy names {key}"
            );
        }

        // Governed declarations carry no floats.
        fn no_floats(v: &serde_json::Value) -> bool {
            match v {
                serde_json::Value::Number(n) => n.is_u64() || n.is_i64(),
                serde_json::Value::Array(a) => a.iter().all(no_floats),
                serde_json::Value::Object(o) => o.values().all(no_floats),
                _ => true,
            }
        }
        let raw: serde_json::Value = serde_json::from_str(RECIPE_JSON).unwrap();
        assert!(no_floats(&raw), "the recipe carries a float");
    }
}

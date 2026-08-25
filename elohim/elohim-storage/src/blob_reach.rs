//! Reach enforcement for the blob BYTE route (`GET /blob/{hash}`).
//!
//! ## The hole this closes
//!
//! Reach lives on the CONTENT row, never on the blob. `PUT /blob/{hash}` stamps
//! its shard manifest `"commons"` unconditionally (`http.rs`, `create_manifest`),
//! so the bytes carry no audience of their own. The content route enforces reach
//! in two layers (`http.rs`, the `/db/content/{id}` arm): Layer 1 refuses a
//! non-public row to a caller who resolved to no identity, and Layer 1.5 runs the
//! real authorizer for tiers above `community`. The byte route enforced NEITHER —
//! its only gate fired when `agent_id` was `Some`, and the predicate behind it
//! (`policy_cache::can_serve`) is a device-policy filter whose `reach_level` the
//! caller passes as `None`.
//!
//! Measured on the two-peer local mesh, 2026-08-24, fully anonymous (no
//! `Authorization`, no `X-Agent-Cid`), against `community`-reach content:
//!
//! ```text
//!   GET /db/content/community-garden-club -> 403 {"requiredReach":"community"}
//!   GET /blob/bafkrei…                    -> 200  (the same row's bytes)
//! ```
//!
//! Reproduced through both doorways (including a COLD one that had never cached
//! the blob) and against both storage peers directly — so it is a property of
//! this route, not of any doorway cache. 8 of 8 sampled gated rows leaked; the
//! full corpus held 38 reach-gated blob-bearing rows.
//!
//! This is concern class **C7 (advertise/serve symmetry)**: the content route
//! advertises an audience the byte route did not honor.
//!
//! ## The rule, and why it is stated this way
//!
//! > A blob is servable to a caller iff SOME content row referencing it is
//! > servable to that caller.
//!
//! "Some", not "all", and that direction is load-bearing: content addressing
//! deduplicates, so one blob is routinely referenced by several rows (12 such
//! CIDs in the measured corpus). If ANY referencing row is public, the bytes are
//! already public through that row and refusing them here would break a
//! legitimate read while protecting nothing.
//!
//! **No reach vocabulary is canonized here.** `elohim/elohim-storage/CLAUDE.md`
//! forbids that while the multi-vocabulary drift is open, so this module invents
//! no tier, no ordering and no table. It asks the two questions the content route
//! already asks, through the helpers that already answer them
//! ([`crate::epr_service::reach_level_index`]), and nothing else.
//!
//! ## Honest absence (C4)
//!
//! A blob with NO referencing content row serves. That is deliberate, not a
//! fallthrough: shard bytes, app-bundle artifacts, seeded assets and every blob
//! written by `PUT /blob/{hash}` before a content row exists have no audience to
//! read, and failing closed on them would dark every one of those paths — a
//! self-inflicted outage protecting nothing, since a blob nothing references
//! carries no row's audience. The absence is answered honestly ("no row claims
//! these bytes"), never guessed at.

use crate::epr_service::reach_level_index;

/// One content row that references a blob's bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobReference {
    pub content_id: String,
    pub reach: String,
}

impl BlobReference {
    pub fn new(content_id: impl Into<String>, reach: impl Into<String>) -> Self {
        Self {
            content_id: content_id.into(),
            reach: reach.into(),
        }
    }
}

/// What the byte route must do for this caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlobServeVerdict {
    /// Serve the bytes. Either no content row claims them, or at least one
    /// referencing row is itself servable to this caller with no further check.
    Serve,
    /// Every referencing row is reach-gated and the caller resolved to no
    /// identity. `required_reach` names the LEAST restricted gated tier among
    /// them — the same field the content route reports as `requiredReach`, so
    /// the two routes answer a refused caller identically.
    RequireIdentity { required_reach: String },
    /// The caller resolved to an identity, and every referencing row sits above
    /// `community` — so presence of an identity is not enough and the real
    /// authorizer must run, exactly as the content route's Layer 1.5 does. Serve
    /// iff it grants against ANY candidate.
    Authorize { candidates: Vec<BlobReference> },
}

/// Does this reach serve to a caller who presented no identity?
///
/// The SAME test the content route's Layer 1 applies (`reach == "commons" ||
/// reach == "public"`), expressed through the shared index so an added tier
/// cannot silently become anonymously-servable here while staying gated there.
/// `reach_level_index` maps an UNRECOGNIZED tier to `u8::MAX` (most restricted),
/// so a typo or a new vocabulary word fails CLOSED.
pub fn serves_anonymously(reach: &str) -> bool {
    reach_level_index(reach) == reach_level_index("public")
}

/// Is this reach above the tier where identity-presence alone suffices?
///
/// Mirrors the content route's Layer 1.5 trigger (`index > index("community")`).
pub fn requires_authorization(reach: &str) -> bool {
    reach_level_index(reach) > reach_level_index("community")
}

/// The Layer-1 decision for a blob, given every content row that references it.
///
/// PURE — no DB, no clock, no request. `identity_resolved` is whether the caller
/// resolved to an explicit agent identity (the content route's
/// `extract_agent_cid_explicit(&req).is_some()`); header PRESENCE is not
/// identity, which is why the caller passes a bool it has already resolved
/// rather than the header.
pub fn blob_serve_verdict(refs: &[BlobReference], identity_resolved: bool) -> BlobServeVerdict {
    // C4 — honest absence: no row claims these bytes, so there is no audience to
    // enforce. See the module docs for why this is not a fallthrough.
    if refs.is_empty() {
        return BlobServeVerdict::Serve;
    }

    // Any publicly-reachable referencing row makes the bytes public already.
    if refs.iter().any(|r| serves_anonymously(&r.reach)) {
        return BlobServeVerdict::Serve;
    }

    if !identity_resolved {
        // Report the least restricted gated tier — the one a caller could most
        // plausibly satisfy — matching the content route's single-row answer.
        let required_reach = refs
            .iter()
            .min_by_key(|r| reach_level_index(&r.reach))
            .map(|r| r.reach.clone())
            .unwrap_or_else(|| "restricted".to_string());
        return BlobServeVerdict::RequireIdentity { required_reach };
    }

    // Identity resolved. Rows at or below `community` serve on identity alone,
    // exactly as the content route treats them.
    if refs.iter().any(|r| !requires_authorization(&r.reach)) {
        return BlobServeVerdict::Serve;
    }

    BlobServeVerdict::Authorize {
        candidates: refs.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(id: &str, reach: &str) -> BlobReference {
        BlobReference::new(id, reach)
    }

    #[test]
    fn unreferenced_blob_serves_honest_absence() {
        // Shard bytes / app artifacts / a blob PUT before its content row.
        assert_eq!(blob_serve_verdict(&[], false), BlobServeVerdict::Serve);
        assert_eq!(blob_serve_verdict(&[], true), BlobServeVerdict::Serve);
    }

    #[test]
    fn public_or_commons_row_serves_anonymously() {
        assert_eq!(
            blob_serve_verdict(&[r("a", "public")], false),
            BlobServeVerdict::Serve
        );
        assert_eq!(
            blob_serve_verdict(&[r("a", "commons")], false),
            BlobServeVerdict::Serve
        );
    }

    /// The measured defect, as its own regression test. Before the cure this
    /// blob answered 200 to an anonymous caller while `/db/content/{id}`
    /// answered 403.
    #[test]
    fn community_row_refuses_an_anonymous_caller() {
        assert_eq!(
            blob_serve_verdict(&[r("community-garden-club", "community")], false),
            BlobServeVerdict::RequireIdentity {
                required_reach: "community".to_string()
            }
        );
    }

    #[test]
    fn community_row_serves_a_resolved_identity_without_authorization() {
        // Parity with the content route: Layer 1.5 only fires ABOVE community.
        assert_eq!(
            blob_serve_verdict(&[r("community-garden-club", "community")], true),
            BlobServeVerdict::Serve
        );
    }

    #[test]
    fn above_community_defers_to_the_authorizer() {
        match blob_serve_verdict(&[r("love-map", "intimate")], true) {
            BlobServeVerdict::Authorize { candidates } => {
                assert_eq!(candidates.len(), 1);
                assert_eq!(candidates[0].content_id, "love-map");
            }
            other => panic!("expected Authorize, got {other:?}"),
        }
    }

    /// Content addressing deduplicates: one blob, several rows. If ANY row is
    /// public the bytes are already public through it, so refusing here would
    /// break a legitimate read while protecting nothing. 12 such CIDs were
    /// measured in the local corpus.
    #[test]
    fn a_shared_blob_serves_when_any_referencing_row_is_public() {
        let refs = [r("secret-thing", "intimate"), r("public-thing", "public")];
        assert_eq!(blob_serve_verdict(&refs, false), BlobServeVerdict::Serve);
    }

    #[test]
    fn all_gated_rows_report_the_least_restricted_tier() {
        let refs = [r("a", "intimate"), r("b", "community")];
        assert_eq!(
            blob_serve_verdict(&refs, false),
            BlobServeVerdict::RequireIdentity {
                required_reach: "community".to_string()
            }
        );
    }

    /// An unrecognized tier must never become anonymously servable here while
    /// the content route still gates it — `reach_level_index` maps it to
    /// `u8::MAX`, so it fails CLOSED on both questions.
    #[test]
    fn an_unrecognized_reach_fails_closed() {
        assert!(!serves_anonymously("brand-new-tier"));
        assert!(requires_authorization("brand-new-tier"));
        match blob_serve_verdict(&[r("x", "brand-new-tier")], false) {
            BlobServeVerdict::RequireIdentity { .. } => {}
            other => panic!("an unknown tier must refuse an anonymous caller, got {other:?}"),
        }
    }

    /// Address-form independence (red-team 2026-08-24). The wiring in
    /// `HttpServer::blob_reach_refusal` must enumerate BOTH renderings of a
    /// digest — `sha256-<hex>` and the raw-codec `bafkrei…` CID — because a
    /// content row may store either. This test pins the property the wiring
    /// relies on: the two renderings of one digest are byte-distinct strings, so
    /// a candidate set that includes only the request's own form misses a row
    /// storing the other. (The construction itself is `BlobStore::hash_to_cid`,
    /// round-trip-tested in blob_store.rs; here we assert they are NOT equal so a
    /// future refactor that dropped one rendering would fail loudly.)
    #[test]
    fn the_two_renderings_of_one_digest_are_distinct_and_both_needed() {
        let hex = "26d7ced97ee329025135f0ad4791b3e24d526b200b9147943450cb9141480406";
        let sha_form = format!("sha256-{hex}");
        let cid = crate::blob_store::BlobStore::hash_to_cid(hex).expect("valid sha256 hex");
        let cid_form = cid.to_string();
        assert!(
            cid_form.starts_with("bafkrei"),
            "raw-codec CID renders as bafkrei…, got {cid_form}"
        );
        assert_ne!(
            sha_form, cid_form,
            "the two renderings differ, so a candidate set MUST carry both to match a row \
             storing either — this is the address-form bypass the wiring closes"
        );
    }

    #[test]
    fn public_and_commons_are_the_same_anonymous_tier() {
        assert!(serves_anonymously("public"));
        assert!(serves_anonymously("commons"));
        assert!(!serves_anonymously("community"));
        assert!(!requires_authorization("community"));
        assert!(requires_authorization("intimate"));
    }
}

// ---------------------------------------------------------------------------
// Storage projection lookup
// ---------------------------------------------------------------------------

/// Every content row that references any of `hash_candidates`.
///
/// The byte route receives a blob address in whichever form the caller had —
/// `sha256-<hex>`, `blake3-<hex>`, or a `bafkrei…` CID — and the content row may
/// record it in any of three columns (`blob_hash`, `blob_cid`,
/// `server_blob_hash`), so the caller passes every form it has resolved and this
/// matches across all three. A caller that resolved fewer forms gets a smaller
/// reference set, which can only ever make the verdict MORE permissive — so the
/// caller must pass the aliases it already looked up for backend selection.
///
/// bounded-work: one indexed SELECT over `content`, capped at
/// [`MAX_REFERENCES`] rows. A blob referenced by more rows than that is
/// answered from the capped set; the cap is above the largest fan-out measured
/// in the corpus (12) by two orders of magnitude.
pub fn lookup_references(
    conn: &mut diesel::SqliteConnection,
    hash_candidates: &[String],
) -> diesel::QueryResult<Vec<BlobReference>> {
    use crate::db::diesel_schema::content::dsl as c;
    use diesel::prelude::*;

    if hash_candidates.is_empty() {
        return Ok(Vec::new());
    }

    let rows: Vec<(String, String)> = c::content
        .filter(
            c::blob_hash
                .eq_any(hash_candidates)
                .or(c::blob_cid.eq_any(hash_candidates))
                .or(c::server_blob_hash.eq_any(hash_candidates)),
        )
        .select((c::id, c::reach))
        .limit(MAX_REFERENCES)
        .load(conn)?;

    Ok(rows
        .into_iter()
        .map(|(content_id, reach)| BlobReference { content_id, reach })
        .collect())
}

/// Cap on referencing rows read for one blob decision. See `lookup_references`.
pub const MAX_REFERENCES: i64 = 1000;

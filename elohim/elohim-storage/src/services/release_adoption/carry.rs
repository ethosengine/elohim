//! The `carry_from` wire mirror and its cursor fold — shared by the two things
//! that drive a lineage carry.
//!
//! # Why this is its own module
//!
//! Rung 6 landed the carry inside [`super::apply`], because exactly one caller
//! existed: [`super::apply::HappLineageVehicle`], which walks a role's OWN v1
//! chain to the cursor's end inside a single apply. Task 12 adds a second
//! caller with a different shape — [`crate::services::lineage_bridge`], a
//! TICKER that takes ONE page per tick per neighbour and never loops. The two
//! share the wire contract and nothing else, so the contract moved here and
//! the loop stayed where its safety argument lives.
//!
//! Everything in this module is re-exported from [`super::apply`], so every
//! pre-Task-12 path (`apply::CarryInput`, `apply::CarryReceipt`,
//! `apply::CARRY_PAGE_LIMIT`) still resolves at exactly the same name.
//!
//! # Idempotency is a property of the ZOME, and it is not landed yet
//!
//! Nothing on this side makes a carry idempotent. Entry-hash idempotency (skip
//! `create_entry` when the hash is already on the chain; skip the witness for a
//! page whose proofs are all already witnessed) is Task 20's work IN THE ZOME,
//! and it announces itself on the wire as [`CarryReceipt::already_carried`].
//! Until it lands, re-carrying a cursor re-creates every entry and authors a
//! duplicate witness. Callers that repeat must therefore ask
//! [`CarryReceipt::reports_idempotency`] before re-walking anything.
//!
//! # The mirror is the contract
//!
//! These types mirror the v2 cell's `node_registry_coordinator` externs rather
//! than importing them: the zome crate is a wasm target this crate does not
//! link, and its `CarryInput`/`CarryReceipt` sit behind the `lineage-witness`
//! cargo feature besides. The mirror is pinned by the round-trip and additive
//! decode tests below — a zome that changes shape without changing them is a
//! decode failure at runtime, which is why they are worth their length.

use std::sync::Arc;

use super::{AdoptionRefusal, RefusalReason};

/// How many v1 records ONE `carry_from` call moves, on the apply path.
///
/// **C3/C6a — the work is bounded on the WASM side, before the call is made.**
/// `HcClient::call_zome` has no timeout and no cancellation path, so a
/// caller-side deadline would merely abandon a conductor that keeps running.
/// The only honest bound is a small batch the extern can always finish, driven
/// by a cursor: 32 records per call, as many calls as the chain needs, each one
/// individually cheap enough that the sweep tick is never held hostage.
pub const CARRY_PAGE_LIMIT: u32 = 32;

/// The hard ceiling on pages ONE role's carry may walk in ONE apply.
///
/// At [`CARRY_PAGE_LIMIT`] this is ~131k records — far beyond the rehearsal
/// corpus, and finite. A zome that never sets `next_cursor` to `None` would
/// otherwise loop this vehicle forever inside a controller sweep; the ceiling
/// turns that into a typed `apply_failed` an operator can read.
pub(super) const MAX_CARRY_PAGES: u32 = 4_096;

/// The zome the lineage carry runs in, on the v2 (side) cell.
pub const CARRY_ZOME: &str = "node_registry_coordinator";
/// The coordinator function that carries one page.
pub const CARRY_FN: &str = "carry_from";

/// WHOSE predecessor records one [`CarryInput`] page should pull — the wire
/// mirror of the v2 cell's `CarrySource` (node-registry coordinator, Task 18).
///
/// `Own` reads the predecessor cell's own chain through `export_records` (the
/// self-carry path, §2.1). `Held(agent)` reads a NEIGHBOUR's chain through
/// `export_held_records` (the held-carry path, §2.2), where this peer is a
/// COURIER: it authors the witness and never the carried content.
///
/// **`Held(<our own key>)` is refused by the zome**, deliberately — a
/// mis-labelled self-carry is a lie about who authored what, so the caller
/// filters itself out of `known_agents` rather than being silently corrected.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CarrySource {
    /// The predecessor cell's own chain — what a request omitting `source`
    /// decodes to, on both sides of the wire.
    #[default]
    Own,
    /// A neighbour's chain, as the predecessor cell can see it.
    Held(holochain_types::prelude::AgentPubKey),
}

impl CarrySource {
    /// Whether this is the default. Drives `skip_serializing_if` below.
    pub fn is_own(&self) -> bool {
        matches!(self, CarrySource::Own)
    }
}

/// Wire mirror of the v2 cell's `carry_from` INPUT.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CarryInput {
    /// The v1 cell to read FROM — the base app's provisioned cell for this
    /// role. Passed explicitly rather than discovered in-wasm: the v2 cell has
    /// no way to know which of a conductor's cells is its own predecessor, and
    /// guessing is the one thing a migration must never do.
    pub v1_cell: holochain_client::CellId,
    /// `None` starts at the beginning. Cursor-driven, so a carry that is
    /// interrupted resumes rather than restarting.
    ///
    /// On the HELD path the cursor is an ordinal into the courier's own
    /// integrated view of a neighbour's chain — a list gossip can GROW
    /// underneath a walk. [`crate::services::lineage_bridge`] watches
    /// [`CarryReceipt::v1_digest`] for exactly that and restarts the walk
    /// rather than trusting a stale ordinal.
    ///
    /// **Restarting is only safe against a Task-20 zome.** Re-carrying a cursor
    /// on a v2 that predates entry-hash idempotency re-creates the entry (same
    /// entry hash, NEW action) and commits a SECOND `NotarizationWitness` for
    /// the page. Within one [`fold_carry`] sweep this cannot happen — the
    /// non-advancing-cursor refusal stops it — but a RETRIED apply, and any
    /// repeating sweep, would double-carry. The bridge refuses the re-walk
    /// rather than performing it; see
    /// [`crate::services::lineage_bridge::next_sweep`].
    pub cursor: Option<u32>,
    pub limit: u32,
    /// **Additive.** Whose records to carry, defaulting to [`CarrySource::Own`].
    ///
    /// `skip_serializing_if` is what keeps the APPLY path's bytes unchanged:
    /// the landed vehicle carries `Own`, emits no `source` key at all (exactly
    /// as it did before this field existed), and the zome's own
    /// `#[serde(default)]` decodes the absence as `Own`. Only the held path
    /// puts a key on the wire.
    #[serde(default, skip_serializing_if = "CarrySource::is_own")]
    pub source: CarrySource,
}

/// Wire mirror of the v2 cell's `carry_from` OUTPUT — ONE page.
///
/// # Scope follows [`CarryInput::source`]
///
/// On [`CarrySource::Own`] every field describes the predecessor's whole
/// chain, because `export_records` walked it locally and completely. On
/// [`CarrySource::Held`] they describe the COURIER'S VIEW of the neighbour's
/// chain — a subset, possibly gapped, because gossip is asynchronous. **A held
/// page is never self-evidencing**: no combination of its own fields proves the
/// neighbour's chain was carried whole. That is why storage never claims
/// completeness from a held sweep (Task 12) and the harness cross-checks
/// against the neighbour's own export instead.
///
/// # The decode contract
///
/// `v1_digest` and `witness_hash` are `String`. A coordinator that returns a
/// native `EntryHash`/`ActionHash` here sends a msgpack BYTE ARRAY, and this
/// decode fails with "invalid value: byte array, expected a string" — the
/// 2026-06-13 signal-decode class that has now bitten this codebase three
/// times. The zome renders the canonical base64 (`HoloHash`'s `Display`)
/// before returning; the storage side never re-derives a hash.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CarryReceipt {
    /// Records this page carried. On a RETRY this also counts records this
    /// chain already held — see [`Self::already_carried`], which is what makes
    /// "how many did we newly move" answerable.
    pub carried: u32,
    /// Where the next page starts, or `None` when the export THIS PAGE DREW
    /// FROM is exhausted. The apply loop's ONLY termination condition — never
    /// a count comparison, because a count the caller derived is not evidence
    /// the chain ended.
    ///
    /// On the `Own` path that is end-of-chain. On the `Held` path it is only
    /// END-OF-LOCAL-VIEW: the courier holds no further records of the
    /// neighbour, which is not a claim about the neighbour's chain.
    pub next_cursor: Option<u32>,
    /// The digest over the chain walk this page drew from.
    pub v1_digest: String,
    /// The `NotarizationWitness` this page authored, base64, or the EMPTY
    /// STRING for a page that authored none (every record was already carried).
    pub witness_hash: String,
    /// **Additive.** The record count of the walk this page drew from, READ
    /// from the export's own `total` — never derived from `carried`, so the
    /// `sum(carried) == v1_total` check can actually fail.
    ///
    /// This is the ONLY honest source for
    /// [`super::LineageCarryReceipt::v1_count`].
    #[serde(default)]
    pub v1_total: Option<u32>,
    /// **Additive.** How many of `carried` were re-created NATIVELY on this
    /// chain (self-carry, §2.1) — held-carries are excluded. On a held sweep
    /// this is 0 by construction: the courier authors the witness, never the
    /// content.
    #[serde(default)]
    pub self_carried: u32,
    /// **Additive.** The highest action SEQUENCE the predecessor observed for
    /// the chain it exported — the one number in this receipt that reaches
    /// past the courier's own view, and so the only way a held receipt can be
    /// checked for truncation at all.
    ///
    /// It spans every action (genesis, `InitZomesComplete`, links and app
    /// entries alike), so the distance from `v1_total` is normally large and is
    /// **not** a staleness measure. The usable comparison is across VIEWS: a
    /// courier reporting a lower `v1_observed_head` than another peer does for
    /// the same chain has not caught up.
    ///
    /// `None` from a page that predates the field or from an authority that
    /// observed nothing — never a fabricated 0, which would read as "chain
    /// head at genesis".
    #[serde(default)]
    pub v1_observed_head: Option<u32>,
    /// **Additive.** How many of `carried` were ALREADY carried before this
    /// page ran (Task 20, epic §7 C6b): a self-carry whose entry hash was
    /// already on this chain, or a held-carry whose entry hash already had a
    /// witness from this lineage. They contribute to `carried` (the content IS
    /// here) but pushed no proof and authored no second witness.
    ///
    /// **`None` means the zome predates the field, and that distinction is
    /// load-bearing**, which is why this is an `Option<u32>` where the zome
    /// declares a plain `u32`: a `0` from a Task-20 zome says "this page
    /// carried nothing that was already here", while an ABSENT field says "this
    /// cell cannot tell me whether it is idempotent". A trailing sweep may
    /// re-walk a view only under the first; under the second a re-walk
    /// re-creates every entry and authors a duplicate witness every tick, so
    /// [`crate::services::lineage_bridge::next_sweep`] halts instead.
    #[serde(default)]
    pub already_carried: Option<u32>,
}

impl CarryReceipt {
    /// Records this page moved that were NOT already here — `carried` minus
    /// [`Self::already_carried`], saturating.
    ///
    /// The honest accumulator for a repeating sweep. Summing `carried` itself
    /// across re-walks of the same view would report a multiple of the truth.
    ///
    /// An ABSENT `already_carried` reads as 0 here, which is correct for the
    /// only pages a pre-Task-20 sweep is allowed to take: the FORWARD ones,
    /// where nothing was already here. The re-walk that would make this reading
    /// wrong is refused upstream rather than mis-counted here.
    pub fn newly_carried(&self) -> u32 {
        self.carried
            .saturating_sub(self.already_carried.unwrap_or(0))
    }

    /// Whether the cell that answered this page can state its own idempotency
    /// — i.e. whether Task 20's `already_carried` is on the wire at all.
    ///
    /// `false` is the honest "I do not know", never "no records were already
    /// here": a v2 whose zome predates Task 20 calls `create_entry`
    /// unconditionally, so re-carrying a cursor re-creates the entry (same
    /// entry hash, NEW action) and commits a SECOND `NotarizationWitness`.
    pub fn reports_idempotency(&self) -> bool {
        self.already_carried.is_some()
    }
}

/// One page of the carry, over a client connected to the v2 (side) app.
pub async fn call_carry_from(
    client: &Arc<crate::hc_client::HcClient>,
    input: CarryInput,
) -> Result<CarryReceipt, String> {
    let payload = rmp_serde::to_vec_named(&input).map_err(|e| format!("encode CarryInput: {e}"))?;
    let bytes = client
        .call_zome(CARRY_ZOME, CARRY_FN, payload)
        .await
        .map_err(|e| e.to_string())?;
    rmp_serde::from_slice(&bytes).map_err(|e| format!("decode CarryReceipt: {e}"))
}

/// Fold a cursor-driven carry into one [`super::LineageCarryReceipt`].
///
/// Split out from the vehicle and parameterised by the page fetcher so the
/// FOLD — which is where an off-by-one silently under-reports a migration —
/// is unit-testable with no conductor, no cell and no installed app.
///
/// `fetch` returns `Err(String)` for a failed page; the fold turns it into an
/// `apply_failed` naming the cursor it died on, because "the carry failed" with
/// no position is not a diagnosis.
///
/// **This loop is the APPLY path only.** A held sweep must never run it: the
/// held `next_cursor: None` means end-of-local-view, so a loop would terminate
/// on a claim it is not entitled to, and it would hold one uncancellable
/// `call_zome` after another inside a tick. The bridge takes one page per tick
/// instead (C3 liveness).
pub(super) async fn fold_carry<F, Fut>(
    role: &str,
    mut fetch: F,
) -> Result<super::LineageCarryReceipt, AdoptionRefusal>
where
    F: FnMut(Option<u32>) -> Fut,
    Fut: std::future::Future<Output = Result<CarryReceipt, String>>,
{
    let mut cursor: Option<u32> = None;
    let mut carried: u32 = 0;
    let mut first_digest: Option<String> = None;
    let mut v1_total: Option<u32> = None;
    let mut witness_hashes: Vec<String> = Vec::new();

    for page_no in 0..MAX_CARRY_PAGES {
        let page = fetch(cursor).await.map_err(|e| {
            AdoptionRefusal::new(
                RefusalReason::ApplyFailed,
                format!(
                    "{CARRY_FN}(role='{role}', cursor={cursor:?}) failed on page {page_no}: {e}"
                ),
            )
        })?;
        carried = carried.saturating_add(page.carried);
        if first_digest.is_none() {
            first_digest = Some(page.v1_digest.clone());
        }
        // LAST non-`None` wins: v1's total is a fact about v1, so the freshest
        // statement of it is the one to keep. A page that says nothing leaves
        // the previous statement standing rather than erasing it.
        if page.v1_total.is_some() {
            v1_total = page.v1_total;
        }
        // A page that authored no witness contributes none — an empty string
        // in the audit trail would read as a witness that exists and is blank.
        if !page.witness_hash.is_empty() {
            witness_hashes.push(page.witness_hash);
        }

        let Some(next) = page.next_cursor else {
            return Ok(super::LineageCarryReceipt {
                role: role.to_string(),
                carried,
                // Whatever v1 ITSELF said its total was — `None` when v1 never
                // told us, which reads as "unknown" and never as "equal to
                // what we carried". Deriving this from `carried` would make
                // `carried == v1_count` true by construction and therefore
                // worthless as the completeness proof it exists to be.
                v1_count: v1_total,
                // The LAST page's digest — what the carry ended up with.
                digest: page.v1_digest,
                v1_digest: first_digest.unwrap_or_default(),
                witness_hashes,
            });
        };
        // A cursor that does not ADVANCE would walk the same page until the
        // ceiling — the same records re-carried, the same witness re-authored,
        // and a receipt whose `carried` is a multiple of the truth.
        if let Some(prev) = cursor {
            if next <= prev {
                return Err(AdoptionRefusal::new(
                    RefusalReason::ApplyFailed,
                    format!(
                        "{CARRY_FN}(role='{role}') returned a cursor that does not advance \
                         ({prev} → {next}) — refusing rather than re-carrying the same page"
                    ),
                ));
            }
        }
        cursor = Some(next);
    }

    Err(AdoptionRefusal::new(
        RefusalReason::ApplyFailed,
        format!(
            "{CARRY_FN}(role='{role}') did not reach the end of v1 within {MAX_CARRY_PAGES} pages \
             ({carried} carried so far) — refusing rather than looping a controller sweep"
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The APPLY path's wire is unchanged by the addition of `source`: a
    /// `CarrySource::Own` input emits NO `source` key, byte-for-byte what the
    /// landed vehicle sent before Task 12.
    #[test]
    fn an_own_carry_input_puts_no_source_key_on_the_wire() {
        let cell = holochain_client::CellId::new(
            holochain_types::prelude::DnaHash::from_raw_36(vec![0x42; 36]),
            holochain_types::prelude::AgentPubKey::from_raw_36(vec![0x24; 36]),
        );
        let own = CarryInput {
            v1_cell: cell.clone(),
            cursor: Some(32),
            limit: CARRY_PAGE_LIMIT,
            source: CarrySource::Own,
        };
        let encoded = rmp_serde::to_vec_named(&own).unwrap();

        // The pre-Task-12 shape, verbatim. Byte equality is the assertion —
        // not "the string 'source' is absent", which a msgpack payload could
        // satisfy by accident.
        #[derive(serde::Serialize)]
        struct LandedCarryInput {
            v1_cell: holochain_client::CellId,
            cursor: Option<u32>,
            limit: u32,
        }
        let landed = rmp_serde::to_vec_named(&LandedCarryInput {
            v1_cell: cell.clone(),
            cursor: Some(32),
            limit: CARRY_PAGE_LIMIT,
        })
        .unwrap();
        assert_eq!(
            encoded, landed,
            "an Own carry must be byte-identical to the landed apply wire"
        );

        // …and it still decodes back to `Own` through the zome's own default.
        let back: CarryInput = rmp_serde::from_slice(&encoded).unwrap();
        assert_eq!(back, own);

        // A HELD carry does put the key on the wire — that is the whole point.
        let held = CarryInput {
            v1_cell: cell,
            cursor: None,
            limit: 16,
            source: CarrySource::Held(holochain_types::prelude::AgentPubKey::from_raw_36(vec![
                0x11;
                36
            ])),
        };
        let encoded = rmp_serde::to_vec_named(&held).unwrap();
        let back: CarryInput = rmp_serde::from_slice(&encoded).unwrap();
        assert_eq!(back, held);
        assert!(!back.source.is_own());
    }

    /// `newly_carried` is what a repeating sweep accumulates — a re-walk of the
    /// same view reports every record as `carried` AND `already_carried`, and
    /// must add nothing.
    #[test]
    fn newly_carried_excludes_what_was_already_here() {
        let fresh = CarryReceipt {
            carried: 5,
            next_cursor: None,
            v1_digest: "d".into(),
            witness_hash: "uhCEkW".into(),
            v1_total: Some(5),
            self_carried: 0,
            v1_observed_head: Some(33),
            already_carried: Some(0),
        };
        assert_eq!(fresh.newly_carried(), 5);
        assert!(fresh.reports_idempotency());

        let rewalk = CarryReceipt {
            already_carried: Some(5),
            witness_hash: String::new(),
            ..fresh.clone()
        };
        assert_eq!(rewalk.newly_carried(), 0);

        // Saturating, never a panic, on a zome that reports nonsense.
        let nonsense = CarryReceipt {
            carried: 1,
            already_carried: Some(9),
            ..fresh
        };
        assert_eq!(nonsense.newly_carried(), 0);
    }

    /// Every field the Task-18/20 zome added is ADDITIVE: a page from a zome
    /// that predates them decodes with honest defaults rather than failing.
    #[test]
    fn the_task_18_fields_decode_additively() {
        let without = rmp_serde::to_vec_named(&serde_json::json!({
            "carried": 1,
            "next_cursor": serde_json::Value::Null,
            "v1_digest": "digest",
            "witness_hash": "uhCEkWitness",
        }))
        .unwrap();
        let decoded: CarryReceipt = rmp_serde::from_slice(&without).expect("additive decode");
        assert_eq!(decoded.v1_total, None);
        assert_eq!(decoded.self_carried, 0);
        assert_eq!(
            decoded.v1_observed_head, None,
            "never a fabricated 0 — that would read as 'chain head at genesis'"
        );
        assert_eq!(
            decoded.already_carried, None,
            "absent is NEVER 0 here — 0 would read as 'this zome is idempotent \
             and nothing was already here', which is the exact claim a \
             pre-Task-20 cell cannot make"
        );
        assert!(!decoded.reports_idempotency());
    }
}

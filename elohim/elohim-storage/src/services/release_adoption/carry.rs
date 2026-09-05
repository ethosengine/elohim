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

/// The zome's NAMED refusal for a resumed page whose walk is no longer the walk
/// it started — matched as a prefix of the Guest error text.
///
/// Named identically on both export paths in the coordinator ON PURPOSE, so a
/// driver's "restart at 0" decision is not path-dependent. Matching a message
/// is not a contract anyone would choose, but it is the only channel a
/// `WasmErrorInner::Guest` gives us, and the zome's doc comment says so in as
/// many words. A rename on either side is caught by the mesh deliverable, not
/// by a compiler.
pub const CHAIN_MOVED: &str = "chain moved — restart at 0";

/// Whether a failed page is the zome's [`CHAIN_MOVED`] refusal rather than a
/// transport or logic failure.
///
/// The refusal travels wrapped in whatever `HcClient::call_zome` renders around
/// a Guest error, so this is a substring test and never an equality one.
pub fn is_chain_moved(error: &str) -> bool {
    error.contains(CHAIN_MOVED)
}

/// What a multi-page walk learned on its FIRST page and need not learn again —
/// the wire mirror of the v2 cell's `ExportResume` (node-registry coordinator,
/// Task 24 + fix round 1).
///
/// # Why a caller bothers
///
/// Before this token, every page of the predecessor's `export_records` re-walked
/// the WHOLE chain and re-hashed it to report the same page-independent
/// `digest`/`total` — carrying N records cost N/`limit` whole-chain walks, on
/// the one path a migration must run to completion. A driver that hands back
/// the `resume` its previous page returned pays that walk ONCE.
///
/// # It is a pin, not a cache — and the export CHECKS it
///
/// Handing one back CLAIMS that the walk is the same walk. A page whose chain
/// head or record count no longer matches is refused outright ([`CHAIN_MOVED`])
/// rather than served against a stale digest. That refusal is what makes the
/// shortcut safe, and it is why both drivers on this side treat it as "restart
/// at 0" rather than as a failure.
///
/// # Opaque here, on purpose
///
/// Nothing on this side mints, edits or interprets a resume: the token
/// describes the PREDECESSOR's chain and only the predecessor can speak to it.
/// Storage carries it back verbatim or not at all. The fields are named only so
/// the msgpack shape matches byte-for-byte — a mirror that renamed one would
/// decode into defaults and silently un-pin every page.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ExportResume {
    /// The chain head the walk is pinned to, canonical base64 — a `String` and
    /// never a native hash, the same discipline [`CarryReceipt::witness_hash`]
    /// keeps against the msgpack byte-array decode class.
    pub head: String,
    /// The whole-walk digest established on the first page.
    pub digest: String,
    /// The app-record count of the whole walk. Checked on every resumed page:
    /// a walk whose count changed is refused, which is what catches a courier's
    /// view growing underneath a held ordinal even when the head has not moved.
    pub total: u32,
    /// The highest observed sequence, carried forward so a page answered by a
    /// momentarily blind authority does not erase what an earlier page
    /// established.
    #[serde(default)]
    pub observed_head: Option<u32>,
    /// **Additive (Task 24 fix round 1).** Where the page cursor SITS on the
    /// chain: `(app-entry ordinal, action_seq)` — the field that makes a
    /// resumed page cost its own window rather than a whole-chain ordinal
    /// walk. `None` from a page that could not name the next position, or from
    /// a coordinator that predates the field; either way the next page walks in
    /// full, which is slower and never wrong.
    #[serde(default)]
    pub cursor_seq: Option<(u32, u32)>,
}

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
    /// **Additive.** The [`ExportResume`] the previous page's
    /// [`CarryReceipt::resume`] carried, passed through verbatim to whichever
    /// export this page calls (Task 24/28, G8).
    ///
    /// `skip_serializing_if` keeps the un-pinned wire byte-identical to what
    /// this driver sent before the field existed — a page with no pin emits no
    /// `resume` key at all, and the zome's own `#[serde(default)]` reads the
    /// absence as "start this walk from scratch". So a storage build that never
    /// pins costs exactly what it cost before, and a v2 that predates the field
    /// ignores the key it does not know.
    ///
    /// Handing one back is a CLAIM that this is the same walk. When it is not,
    /// the export refuses with [`CHAIN_MOVED`] rather than serving a stale
    /// digest, and both drivers on this side answer that by restarting at
    /// cursor 0 with no pin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume: Option<ExportResume>,
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
    /// **Additive.** The predecessor's [`ExportResume`] for this walk, reported
    /// verbatim from its export page. Hand it back on the next [`CarryInput`]
    /// and the predecessor stops re-walking its whole chain per page (Task 24,
    /// G8); ignore it and nothing changes.
    ///
    /// `None` from a v2 whose coordinator predates the field — which is not an
    /// error and not a refusal, just a walk that keeps paying the first-page
    /// cost on every page.
    #[serde(default)]
    pub resume: Option<ExportResume>,
    /// **Additive.** How many action rows the predecessor's POSITION scan read
    /// to find where this page starts — **the metric risk row R1 reads**, from
    /// its export page's `scanned`.
    ///
    /// Carry cost stays linear in chain length iff this stays bounded on
    /// RESUMED pages and, the property that actually matters, independent of
    /// how far into the chain the page sits. An unpinned page reports the whole
    /// chain's action count, because finding an arbitrary ordinal costs exactly
    /// that; a pinned page reports only its own probe span.
    ///
    /// **The producer half is not landed.** The predecessor's `ExportPage`
    /// carries `scanned`, but the v2 cell's `carry_from` does not yet copy it
    /// onto the receipt, so today this decodes `None` on every page and the
    /// projections below read "not reported" rather than a fabricated 0. This
    /// mirror is deliberately landed ahead of it: the reader is correct now, so
    /// R1's metric becomes visible the moment the coordinator forwards the
    /// field, with no second storage change.
    #[serde(default)]
    pub scanned: Option<u32>,
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
///
/// # The resume pin (Task 24/28, G8)
///
/// `fetch` is handed the previous page's [`ExportResume`] alongside the cursor,
/// so the predecessor pins the walk instead of re-deriving its digest and total
/// on every page. The pin is an OPTIMISATION and never a correctness input:
/// this fold reads nothing out of it, and a `None` throughout is exactly the
/// pre-pin behaviour.
///
/// A pinned page whose walk has moved is refused by name ([`CHAIN_MOVED`]) and
/// this fold answers it the way a digest change is answered: **the walk starts
/// over at cursor 0 with no pin, and every accumulator resets with it.** Keeping
/// the counts across a restart would report a `carried` that is a multiple of
/// the truth and a `v1_digest` from a walk that no longer exists. The restart
/// cannot loop: the first page after it carries no pin, and an unpinned page is
/// never refused for this reason — and each restart still spends one of the
/// [`MAX_CARRY_PAGES`] iterations, so even a pathological predecessor ends in a
/// typed refusal rather than a hang.
pub(super) async fn fold_carry<F, Fut>(
    role: &str,
    mut fetch: F,
) -> Result<super::LineageCarryReceipt, AdoptionRefusal>
where
    F: FnMut(Option<u32>, Option<ExportResume>) -> Fut,
    Fut: std::future::Future<Output = Result<CarryReceipt, String>>,
{
    let mut cursor: Option<u32> = None;
    let mut resume: Option<ExportResume> = None;
    let mut carried: u32 = 0;
    let mut first_digest: Option<String> = None;
    let mut v1_total: Option<u32> = None;
    let mut witness_hashes: Vec<String> = Vec::new();

    for page_no in 0..MAX_CARRY_PAGES {
        let page = match fetch(cursor, resume.clone()).await {
            Ok(page) => page,
            // The pin no longer describes the predecessor's chain. Not a
            // failure — an instruction, and the only correct answer to it is
            // the one the refusal names.
            Err(e) if is_chain_moved(&e) => {
                tracing::warn!(
                    role,
                    cursor = ?cursor,
                    page_no,
                    carried,
                    "lineage carry: the predecessor refused the resume pin ({CHAIN_MOVED}) — the \
                     chain moved underneath this walk, so the carry restarts at cursor 0 with no \
                     pin and the page counts so far are discarded"
                );
                cursor = None;
                resume = None;
                carried = 0;
                first_digest = None;
                v1_total = None;
                witness_hashes.clear();
                continue;
            }
            Err(e) => {
                return Err(AdoptionRefusal::new(
                    RefusalReason::ApplyFailed,
                    format!(
                        "{CARRY_FN}(role='{role}', cursor={cursor:?}) failed on page {page_no}: {e}"
                    ),
                ))
            }
        };
        // Carried forward verbatim: the token describes the PREDECESSOR's
        // chain, so the only honest thing to do with it is hand it back.
        resume = page.resume.clone();
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
            resume: None,
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
            resume: None,
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
            resume: None,
            scanned: None,
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
        assert_eq!(
            decoded.resume, None,
            "a v2 that predates the pin simply pays the first-page cost every page"
        );
        assert_eq!(
            decoded.scanned, None,
            "never a fabricated 0 — that would read as a page whose position scan read nothing"
        );
    }

    /// A page that DOES carry the pin round-trips it verbatim, field for field.
    /// The mirror only works if the msgpack shape matches the zome's; a renamed
    /// or re-typed field would decode into a default and silently un-pin every
    /// subsequent page rather than failing.
    #[test]
    fn the_resume_pin_round_trips_through_the_page_wire() {
        let page = CarryReceipt {
            carried: 32,
            next_cursor: Some(32),
            v1_digest: "digest".into(),
            witness_hash: "uhCEkWitness".into(),
            v1_total: Some(200),
            self_carried: 32,
            v1_observed_head: Some(1_207),
            already_carried: Some(0),
            resume: Some(ExportResume {
                head: "uhCkkHead".into(),
                digest: "digest".into(),
                total: 200,
                observed_head: Some(1_207),
                cursor_seq: Some((32, 196)),
            }),
            scanned: Some(41),
        };
        let bytes = rmp_serde::to_vec_named(&page).unwrap();
        let back: CarryReceipt = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(page, back);

        // …and it decodes from the field names the coordinator actually emits,
        // rather than only from our own encoder's.
        let from_zome_shape = rmp_serde::to_vec_named(&serde_json::json!({
            "carried": 1,
            "next_cursor": 1,
            "v1_digest": "digest",
            "witness_hash": "uhCEkWitness",
            "resume": {
                "head": "uhCkkHead",
                "digest": "digest",
                "total": 200,
                "observed_head": 1_207,
                "cursor_seq": [32, 196],
            },
            "scanned": 41,
        }))
        .unwrap();
        let decoded: CarryReceipt = rmp_serde::from_slice(&from_zome_shape).expect("zome shape");
        assert_eq!(decoded.resume, page.resume);
        assert_eq!(decoded.scanned, Some(41));
    }

    /// An UNPINNED input is byte-identical to the wire this driver sent before
    /// the field existed — the whole reason `resume` is `skip_serializing_if`.
    /// A pinned one puts exactly one more key on it.
    #[test]
    fn an_unpinned_carry_input_puts_no_resume_key_on_the_wire() {
        let cell = holochain_client::CellId::new(
            holochain_types::prelude::DnaHash::from_raw_36(vec![0x42; 36]),
            holochain_types::prelude::AgentPubKey::from_raw_36(vec![0x24; 36]),
        );
        let unpinned = CarryInput {
            v1_cell: cell.clone(),
            cursor: Some(32),
            limit: CARRY_PAGE_LIMIT,
            source: CarrySource::Own,
            resume: None,
        };

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
            rmp_serde::to_vec_named(&unpinned).unwrap(),
            landed,
            "an unpinned carry must stay byte-identical to the landed apply wire"
        );

        let pinned = CarryInput {
            resume: Some(ExportResume {
                head: "uhCkkHead".into(),
                digest: "digest".into(),
                total: 200,
                observed_head: Some(1_207),
                cursor_seq: Some((32, 196)),
            }),
            ..unpinned.clone()
        };
        let encoded = rmp_serde::to_vec_named(&pinned).unwrap();
        assert_ne!(
            encoded, landed,
            "a pinned carry does put the key on the wire"
        );
        let back: CarryInput = rmp_serde::from_slice(&encoded).unwrap();
        assert_eq!(back, pinned);
    }

    fn pinned_page(
        carried: u32,
        next_cursor: Option<u32>,
        digest: &str,
        pin: Option<&str>,
    ) -> CarryReceipt {
        CarryReceipt {
            carried,
            next_cursor,
            v1_digest: digest.to_string(),
            witness_hash: format!("uhCEkWitness{carried}"),
            v1_total: Some(39),
            self_carried: carried,
            v1_observed_head: Some(41),
            already_carried: Some(0),
            resume: pin.map(|head| ExportResume {
                head: head.to_string(),
                digest: digest.to_string(),
                total: 39,
                observed_head: Some(41),
                cursor_seq: Some((next_cursor.unwrap_or(0), 196)),
            }),
            scanned: Some(8),
        }
    }

    /// **The pin travels.** Page one is asked with no pin; every later page is
    /// asked with the pin the page before it returned — which is the only thing
    /// that makes the predecessor's walk cost its own window rather than the
    /// whole chain.
    #[tokio::test]
    async fn the_fold_threads_the_resume_pin_into_the_next_page() {
        let pages = std::sync::Mutex::new(vec![
            pinned_page(32, Some(32), "digest-a", Some("uhCkkHead")),
            pinned_page(7, None, "digest-a", Some("uhCkkHead")),
        ]);
        let asked: std::sync::Mutex<Vec<(Option<u32>, Option<ExportResume>)>> =
            std::sync::Mutex::new(Vec::new());

        let carry = fold_carry("node_registry", |cursor, resume| {
            asked.lock().unwrap().push((cursor, resume));
            let page = pages.lock().unwrap().remove(0);
            async move { Ok(page) }
        })
        .await
        .expect("a carry that reaches the cursor's end");

        assert_eq!(carry.carried, 39);
        let asked = asked.lock().unwrap();
        assert_eq!(asked[0].0, None);
        assert_eq!(
            asked[0].1, None,
            "the first page has nothing to pin — the pin is what the walk LEARNED"
        );
        assert_eq!(asked[1].0, Some(32));
        assert_eq!(
            asked[1].1.as_ref().map(|r| r.head.as_str()),
            Some("uhCkkHead"),
            "page two carries page one's pin back verbatim"
        );
        assert_eq!(
            asked[1].1.as_ref().and_then(|r| r.cursor_seq),
            Some((32, 196)),
            "including `cursor_seq` — the field that makes the resumed page cheap"
        );
    }

    /// **`chain moved` is an instruction, not a failure.** A refused pinned page
    /// restarts the walk at cursor 0 with NO pin, and the counts restart with it
    /// — a fold that kept them would report a `carried` that is a multiple of
    /// the truth and a `v1_digest` from a walk that no longer exists.
    #[tokio::test]
    async fn a_chain_moved_refusal_restarts_the_walk_at_zero_with_no_pin() {
        let pages: std::sync::Mutex<Vec<Result<CarryReceipt, String>>> =
            std::sync::Mutex::new(vec![
                Ok(pinned_page(32, Some(32), "digest-a", Some("uhCkkOldHead"))),
                Err(format!(
                    "{CHAIN_MOVED}: the resume token pins the chain head uhCkkOldHead, but this \
                     walk now sees uhCkkNewHead."
                )),
                Ok(pinned_page(32, Some(32), "digest-b", Some("uhCkkNewHead"))),
                Ok(pinned_page(9, None, "digest-b", Some("uhCkkNewHead"))),
            ]);
        let asked: std::sync::Mutex<Vec<(Option<u32>, Option<ExportResume>)>> =
            std::sync::Mutex::new(Vec::new());

        let carry = fold_carry("node_registry", |cursor, resume| {
            asked.lock().unwrap().push((cursor, resume));
            let page = pages.lock().unwrap().remove(0);
            async move { page }
        })
        .await
        .expect("a restarted carry still finishes");

        let asked = asked.lock().unwrap();
        assert_eq!(asked.len(), 4);
        assert_eq!(
            (asked[2].0, asked[2].1.clone()),
            (None, None),
            "the page after the refusal starts at cursor 0 with no pin — an unpinned page is the \
             one thing this refusal can never be raised against, so the restart cannot loop"
        );
        assert_eq!(
            carry.carried, 41,
            "32 + 9 from the SECOND walk only — the discarded walk's 32 does not survive"
        );
        assert_eq!(
            carry.v1_digest, "digest-b",
            "the first digest is the first digest of the walk that finished"
        );
        assert_eq!(
            carry.witness_hashes.len(),
            2,
            "the abandoned walk's witness is not claimed by this receipt"
        );
    }
}

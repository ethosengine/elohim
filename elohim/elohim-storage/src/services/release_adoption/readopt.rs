//! **Task 13c — the storage wiring of `readopt_from`** (Holochain Evolution
//! Epic MVP, Station 7): the half of the revert that brings the window-time v2
//! facts home.
//!
//! # What this module is for
//!
//! Task 13a landed the revert TRIGGER and left the records half as a typed
//! seam ([`super::revert::ReadoptStatus`]) that said, honestly, "the v1
//! coordinator extern this needs is not on this build". Task 13b put the
//! extern there ([`readopt_from`] on the pristine v1 `node_registry_coordinator`
//! — coordinator-only, DNA-hash-neutral). This module is the driver: it pages
//! that extern until the successor's export is exhausted and folds the pages
//! into one [`ReadoptSummary`] the revert receipt carries.
//!
//! The story's line is the specification: *"james's record authored on v2
//! during the window is re-authored by james on v1 with the same entry hash …
//! and any v2-authored record not yet re-authored on v1 is reported by its
//! author's passport as pending, never as lost"*. Every count here exists so
//! that sentence has numbers behind it.
//!
//! # Which side runs it, and why that decides the ordering
//!
//! `readopt_from` runs ON V1 and reaches ACROSS to the v2 cell for one bounded
//! page of that cell's own `export_records`. Both facts matter to the caller:
//!
//! - **It is a v1 call**, so the client must be connected to the BASE app —
//!   which is exactly where [`super::revert::perform_revert`] has already
//!   returned authoring to by the time this runs.
//! - **It reaches into v2**, so the v2 cell must still be RUNNING. That is why
//!   the readopt happens BEFORE `disable_app` in the revert path and not after
//!   — a disabled app's cells answer no cross-cell call, and a readopt placed
//!   after the disable would fail every time while looking like an ordering
//!   detail. See [`super::revert::perform_revert`]'s numbered order.
//!
//! # The mirror is the contract
//!
//! [`ReadoptInput`] / [`ReadoptReceipt`] mirror the v1 coordinator's own
//! structs rather than importing them (the zome is a wasm target this crate
//! does not link). The zome declares them with a plain
//! `#[derive(Serialize, Deserialize)]` and NO `rename_all`, so these fields are
//! snake_case on the wire — deliberately unlike the camelCase view types this
//! crate exports, and pinned by the round-trip tests below.
//!
//! # Bounded, and never a completeness claim on its own
//!
//! One page commits at most [`READOPT_PAGE_LIMIT`] entries (the zome clamps to
//! its own `READOPT_CAP` of 16 regardless of what we ask for), and the fold is
//! ceilinged at [`MAX_READOPT_PAGES`] so a zome that never nulls its cursor
//! turns into a typed error rather than an unbounded loop inside a controller
//! sweep — the same discipline, for the same reason, as
//! [`super::carry::fold_carry`].
//!
//! The summary answers "what did this walk do", never "did every window-time
//! fact come home". [`ReadoptSummary::complete`] is the closest this side gets,
//! and it is explicitly a comparison the caller may find UNKNOWABLE (`None`)
//! when the successor never reported a total.

use std::sync::Arc;

/// The zome `readopt_from` lives in — the same coordinator on both lineage
/// ends, which is what lets a PRISTINE v1 answer at all.
pub const READOPT_ZOME: &str = "node_registry_coordinator";
/// The coordinator function that re-authors one page.
pub const READOPT_FN: &str = "readopt_from";

/// How many v2 records ONE `readopt_from` call re-authors.
///
/// The zome clamps to its own `READOPT_CAP` (16) whatever we send, so this is
/// the same number stated on the side that has to live with the latency: one
/// page IS the write batch, because re-adoption commits one entry per record.
/// C3 liveness — `call_zome` has no cancellation path, so the only honest bound
/// is a batch the extern can always finish.
pub const READOPT_PAGE_LIMIT: u32 = 16;

/// The hard ceiling on pages ONE role's readopt may walk in ONE revert.
///
/// At [`READOPT_PAGE_LIMIT`] this is ~65k records — orders of magnitude beyond
/// a window-time tail (the window is the span between a crossing and its
/// revert, measured in hours), and finite. A zome that never sets `next_cursor`
/// to `None` would otherwise loop the revert forever inside a controller sweep.
const MAX_READOPT_PAGES: u32 = 4_096;

/// Wire mirror of the v1 cell's `readopt_from` INPUT.
///
/// snake_case, because the zome's struct carries no `rename_all` — pinned by
/// [`tests::the_readopt_wire_is_snake_case_and_round_trips`].
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ReadoptInput {
    /// The SUCCESSOR cell to read this agent's window-time records back from.
    /// Named by the caller rather than discovered in-wasm: v1 predates the
    /// crossing and declares no successor, so there is nothing on that chain to
    /// derive it from — and guessing is the one thing a migration must not do.
    pub v2_cell: holochain_client::CellId,
    /// `None` starts at the beginning of the successor's export. Cursor-driven,
    /// so an interrupted readopt resumes rather than restarting.
    pub cursor: Option<u32>,
    pub limit: u32,
}

/// Wire mirror of the v1 cell's `readopt_from` OUTPUT — ONE page.
///
/// # The decode contract
///
/// `v2_digest` is a `String` because the zome renders the canonical base64
/// before returning; a coordinator returning a native `EntryHash` here would
/// send a msgpack BYTE ARRAY and this decode would fail with "invalid value:
/// byte array, expected a string" — the same signal-decode class
/// [`super::carry::CarryReceipt`] documents.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ReadoptReceipt {
    /// This agent's own v2 records re-created NATIVELY on v1 by this page —
    /// new actions over the SAME entry hashes.
    pub readopted: u32,
    /// Records already on v1 before the page ran. On the FIRST sweep this is
    /// normally large and that is correct: v2's chain opens with the carried
    /// re-creations of v1's own records, whose entry hashes are on v1 already.
    /// It reads "v1 has this fact", never "a retry ran".
    pub already_present: u32,
    /// Records carrying an app entry type v1 does not know — v2's
    /// `NotarizationWitness` above all. Skipped, counted, never an error:
    /// witnesses are v2's own bookkeeping about the crossing, and refusing a
    /// page over one would make revert impossible for exactly the chains that
    /// took the crossing.
    ///
    /// `#[serde(default)]` mirrors the zome's own attribute — additive, so a
    /// receipt from a build without the field reads 0 rather than failing.
    #[serde(default)]
    pub foreign: u32,
    /// Where the next page starts, or `None` when the successor's export is
    /// exhausted. The fold's ONLY termination condition — never a count
    /// comparison, because a count the caller derived is not evidence the
    /// chain ended.
    pub next_cursor: Option<u32>,
    /// The successor's whole-chain digest, verbatim from its export page, so a
    /// multi-page walk can be checked to have drawn from ONE chain.
    pub v2_digest: String,
    /// The successor's app-record count, READ from its export's own `total` and
    /// never derived from `readopted` — so a completeness check can actually
    /// fail. `None` when the successor bundle predates that field.
    pub v2_total: Option<u32>,
}

/// What a whole readopt walk amounted to, for ONE role.
///
/// Lands on [`super::revert::RevertReceipt::readopt`] and therefore on
/// `GET /admin/adoption`, which is the only place an operator can answer "did
/// my window-time facts come home?" without a conductor in hand.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadoptSummary {
    /// Records re-authored on v1 across every page.
    pub readopted: u32,
    /// Records that were already on v1 across every page.
    pub already_present: u32,
    /// Records skipped as an entry type v1 does not know (v2's witnesses).
    pub foreign: u32,
    /// How many pages the walk took. `0` with non-zero counts is impossible;
    /// `0` with zero counts means the successor's export was empty.
    pub pages: u32,
    /// The successor's chain digest — the LAST page's, which is what the walk
    /// ended on. Empty only when no page was taken at all.
    pub v2_digest: String,
    /// The successor's own app-record count, as IT reported it. `None` means
    /// the successor never said — which reads as "unknown", never as "equal to
    /// what we readopted".
    pub v2_total: Option<u32>,
}

impl ReadoptSummary {
    /// Every record on the successor's chain this walk accounted for — moved,
    /// already here, or knowingly skipped as foreign.
    pub fn accounted(&self) -> u32 {
        self.readopted
            .saturating_add(self.already_present)
            .saturating_add(self.foreign)
    }

    /// Did the walk account for the successor's whole chain?
    ///
    /// `None` — not `false` — when the successor never reported a total. A
    /// completeness question we could not ask must never render as a
    /// completeness answer, which is the same rule
    /// [`super::LineageCarryReceipt::v1_count`] keeps on the forward path.
    pub fn complete(&self) -> Option<bool> {
        self.v2_total.map(|total| self.accounted() >= total)
    }
}

/// One page of the readopt, over a client connected to the V1 (base) app.
pub async fn call_readopt_from(
    client: &Arc<crate::hc_client::HcClient>,
    input: ReadoptInput,
) -> Result<ReadoptReceipt, String> {
    let payload =
        rmp_serde::to_vec_named(&input).map_err(|e| format!("encode ReadoptInput: {e}"))?;
    let bytes = client
        .call_zome(READOPT_ZOME, READOPT_FN, payload)
        .await
        .map_err(|e| e.to_string())?;
    rmp_serde::from_slice(&bytes).map_err(|e| format!("decode ReadoptReceipt: {e}"))
}

/// Fold a cursor-driven readopt into one [`ReadoptSummary`].
///
/// Parameterised by the page fetcher so the FOLD — where an off-by-one silently
/// under-reports what came home — is unit-testable with no conductor, no cell
/// and no installed app.
///
/// # Failure is PARTIAL, never nothing
///
/// `Err` carries the reason AND the summary of the pages that DID land, because
/// a revert whose readopt died on page three has still brought two pages' worth
/// of facts home, and a receipt that reported only the error would read as
/// though nothing moved. That is the exact shape the story forbids: *"reported
/// as pending, never as lost"*.
pub async fn fold_readopt<F, Fut>(
    role: &str,
    mut fetch: F,
) -> Result<ReadoptSummary, (String, ReadoptSummary)>
where
    F: FnMut(Option<u32>) -> Fut,
    Fut: std::future::Future<Output = Result<ReadoptReceipt, String>>,
{
    let mut cursor: Option<u32> = None;
    let mut summary = ReadoptSummary::default();

    for page_no in 0..MAX_READOPT_PAGES {
        let page = match fetch(cursor).await {
            Ok(page) => page,
            Err(e) => {
                return Err((
                    format!(
                        "{READOPT_FN}(role='{role}', cursor={cursor:?}) failed on page \
                         {page_no}: {e}"
                    ),
                    summary,
                ))
            }
        };
        summary.readopted = summary.readopted.saturating_add(page.readopted);
        summary.already_present = summary.already_present.saturating_add(page.already_present);
        summary.foreign = summary.foreign.saturating_add(page.foreign);
        summary.pages = summary.pages.saturating_add(1);
        summary.v2_digest = page.v2_digest;
        // LAST non-`None` wins: v2's total is a fact about v2, so the freshest
        // statement of it stands. A page that says nothing leaves the previous
        // statement alone rather than erasing it.
        if page.v2_total.is_some() {
            summary.v2_total = page.v2_total;
        }

        let Some(next) = page.next_cursor else {
            return Ok(summary);
        };
        // A cursor that does not ADVANCE would re-walk the same page to the
        // ceiling. The re-adoption itself is idempotent by entry hash, so the
        // damage is not duplicate entries — it is a `already_present` count
        // that is a multiple of the truth, and a sweep that never returns.
        if let Some(prev) = cursor {
            if next <= prev {
                return Err((
                    format!(
                        "{READOPT_FN}(role='{role}') returned a cursor that does not advance \
                         ({prev} → {next}) — refusing rather than re-walking the same page"
                    ),
                    summary,
                ));
            }
        }
        cursor = Some(next);
    }

    Err((
        format!(
            "{READOPT_FN}(role='{role}') did not exhaust the successor's export within \
             {MAX_READOPT_PAGES} pages — refusing rather than looping a controller sweep"
        ),
        summary,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell() -> holochain_client::CellId {
        holochain_client::CellId::new(
            holochain_types::prelude::DnaHash::from_raw_36(vec![0x42; 36]),
            holochain_types::prelude::AgentPubKey::from_raw_36(vec![0x24; 36]),
        )
    }

    fn page(
        readopted: u32,
        already_present: u32,
        foreign: u32,
        next_cursor: Option<u32>,
        v2_total: Option<u32>,
    ) -> ReadoptReceipt {
        ReadoptReceipt {
            readopted,
            already_present,
            foreign,
            next_cursor,
            v2_digest: "v2digest".to_string(),
            v2_total,
        }
    }

    /// The zome declares `ReadoptInput`/`ReadoptReceipt` with no `rename_all`,
    /// so the wire is snake_case. A camelCase mirror would decode to zeroes on
    /// every field and report a readopt that moved nothing — silently.
    #[test]
    fn the_readopt_wire_is_snake_case_and_round_trips() {
        let input = ReadoptInput {
            v2_cell: cell(),
            cursor: Some(16),
            limit: READOPT_PAGE_LIMIT,
        };
        let encoded = rmp_serde::to_vec_named(&input).unwrap();
        // The key check is a BYTE search rather than a JSON decode: `CellId`
        // serialises as a msgpack byte array, which is not representable as
        // `serde_json::Value` at all — the same fact that makes the hash
        // fields on every receipt in this crate `String`s.
        for key in ["v2_cell", "cursor", "limit"] {
            assert!(
                encoded.windows(key.len()).any(|w| w == key.as_bytes()),
                "the zome's field '{key}' must be on the wire verbatim"
            );
        }
        assert_eq!(
            rmp_serde::from_slice::<ReadoptInput>(&encoded).unwrap(),
            input
        );

        let receipt = page(3, 5, 1, Some(16), Some(9));
        let encoded = rmp_serde::to_vec_named(&receipt).unwrap();
        let as_json: serde_json::Value = rmp_serde::from_slice(&encoded).unwrap();
        for key in [
            "readopted",
            "already_present",
            "foreign",
            "next_cursor",
            "v2_digest",
            "v2_total",
        ] {
            assert!(
                as_json.get(key).is_some(),
                "the zome's field '{key}' must be on the wire verbatim: {as_json}"
            );
        }
        assert_eq!(
            rmp_serde::from_slice::<ReadoptReceipt>(&encoded).unwrap(),
            receipt
        );
    }

    /// `foreign` is `#[serde(default)]` on BOTH sides — a receipt from a build
    /// that predates it decodes as 0 rather than failing the whole revert.
    #[test]
    fn a_receipt_without_foreign_decodes_additively() {
        let without = rmp_serde::to_vec_named(&serde_json::json!({
            "readopted": 2,
            "already_present": 1,
            "next_cursor": serde_json::Value::Null,
            "v2_digest": "d",
            "v2_total": 3,
        }))
        .unwrap();
        let decoded: ReadoptReceipt = rmp_serde::from_slice(&without).expect("additive decode");
        assert_eq!(decoded.foreign, 0);
        assert_eq!(decoded.readopted, 2);
    }

    /// **The fold, over two pages.** Counts add, the LAST digest wins, the
    /// total is v2's own, and the walk stops on a null cursor — never on a
    /// count comparison.
    #[tokio::test]
    async fn the_readopt_fold_walks_two_pages_into_one_summary() {
        let pages = std::sync::Mutex::new(vec![
            page(2, 14, 0, Some(16), Some(20)),
            ReadoptReceipt {
                v2_digest: "final".to_string(),
                ..page(1, 2, 1, None, Some(20))
            },
        ]);
        let seen = std::sync::Mutex::new(Vec::new());

        let summary = fold_readopt("node_registry", |cursor| {
            seen.lock().unwrap().push(cursor);
            let next = pages.lock().unwrap().remove(0);
            async move { Ok(next) }
        })
        .await
        .expect("both pages land");

        assert_eq!(*seen.lock().unwrap(), vec![None, Some(16)]);
        assert_eq!(summary.readopted, 3);
        assert_eq!(summary.already_present, 16);
        assert_eq!(summary.foreign, 1);
        assert_eq!(summary.pages, 2);
        assert_eq!(summary.v2_digest, "final", "the LAST page's digest");
        assert_eq!(summary.v2_total, Some(20));
        assert_eq!(summary.accounted(), 20);
        assert_eq!(summary.complete(), Some(true));
    }

    /// A successor that never reports a total leaves completeness UNKNOWN —
    /// never "complete", which would be a claim we cannot make.
    #[tokio::test]
    async fn completeness_is_unknown_when_the_successor_reports_no_total() {
        let summary = fold_readopt("node_registry", |_| async { Ok(page(1, 0, 0, None, None)) })
            .await
            .expect("one page");
        assert_eq!(summary.v2_total, None);
        assert_eq!(summary.complete(), None);
    }

    /// **Partial, never nothing.** A page that dies still reports what the
    /// pages before it brought home — the story's "pending, never lost".
    #[tokio::test]
    async fn a_failed_page_names_its_cursor_and_keeps_what_landed() {
        let calls = std::sync::Mutex::new(0u32);
        let (reason, partial) = fold_readopt("node_registry", |_| {
            let mut n = calls.lock().unwrap();
            *n += 1;
            let first = *n == 1;
            async move {
                if first {
                    Ok(page(4, 0, 0, Some(16), Some(9)))
                } else {
                    Err("websocket closed".to_string())
                }
            }
        })
        .await
        .expect_err("the second page fails");

        assert!(reason.contains("cursor=Some(16)"), "{reason}");
        assert!(reason.contains("page 1"), "{reason}");
        assert!(reason.contains("websocket closed"), "{reason}");
        assert_eq!(partial.readopted, 4, "page one's work is still reported");
        assert_eq!(partial.pages, 1);
    }

    /// A cursor that does not advance is refused rather than re-walked. Every
    /// page here answers with the SAME cursor — a zome stuck in place, which
    /// would otherwise be walked to the page ceiling with `already_present`
    /// climbing to a multiple of the truth.
    #[tokio::test]
    async fn a_cursor_that_does_not_advance_is_refused() {
        let (reason, partial) = fold_readopt("node_registry", |_| async {
            Ok(page(1, 0, 0, Some(16), Some(4)))
        })
        .await
        .expect_err("the second page does not advance");
        assert!(reason.contains("does not advance"), "{reason}");
        assert_eq!(
            partial.pages, 2,
            "one page taken, one repeat detected — never a third"
        );
    }
}

//! Pure-iroh bootstrap: learn peers from a doorway when gossip cannot reach us.
//!
//! The iroh peer book is filled by verified `elohim/transport/manifest`
//! announcements — which arrive over GOSSIP. In dual mode that works because
//! libp2p gossipsub already has peers at boot. In **pure iroh** it is circular:
//! iroh-gossip needs a seeded book to reach anyone, and the book is seeded by
//! iroh-gossip. Every responder mounts, nothing ever dials.
//!
//! This module breaks the circle through the one web2 surface the protocol
//! already concedes: a doorway holds a bounded, ephemeral pile of signed
//! announcements. Peers POST theirs on each announce tick; a peer with an empty
//! book GETs the pile.
//!
//! ## The invariant survives the concession
//!
//! The doorway is a *bulletin board*, not an authority. It may store the JSON
//! entirely opaquely — it is never trusted to have checked anything. The book's
//! invariant ("an entry only ever comes from an announcement whose signature
//! verified against its own `node_id`") is preserved because
//! [`verify_and_upsert`] runs [`TransportManifestAnnouncement::verify`] HERE,
//! on the client, before the book ever sees an entry. A doorway that lied,
//! tampered, or was itself compromised can therefore cost us availability
//! (peers we never learn) but never redirection (dials to an address whose
//! NodeId did not sign for it).
//!
//! Monotonicity survives too: [`IrohPeerBook::upsert`] refuses an announcement
//! older than the one it already holds, so a doorway replaying a stale manifest
//! cannot roll a peer's addresses backwards.
//!
//! ## Inert unless configured
//!
//! [`doorway_base_url`] returns `None` when the node has no doorway, and every
//! entry point here degrades to a no-op — a household node with no web2 edge
//! keeps exactly the behaviour it had.

use std::time::Duration;

use futures_util::StreamExt;

use tracing::{debug, info, warn};

use super::peer_book::IrohPeerBook;
use crate::p2p::gossip_dispatch::TransportManifestSink;
use crate::p2p::transport_manifest_gossip::TransportManifestAnnouncement;

/// Explicit doorway base URL (scheme://host[:port]). Highest precedence.
pub const DOORWAY_URL_ENV: &str = "ELOHIM_DOORWAY_URL";

/// The doorway's bounded manifest bulletin board.
pub const MANIFESTS_PATH: &str = "/p2p/manifests";

/// Hard cap on the GET body we will buffer from a doorway. The doorway serves
/// at most `MAX_MANIFEST_ENTRIES` (64) × `MAX_MANIFEST_BODY_BYTES` (8 KiB)
/// announcements plus JSON framing; 1 MiB is generous headroom over that. A
/// doorway that lied, was compromised, or is malicious cannot make us buffer a
/// multi-GB body into memory (OOM) before we ever verify a signature — we abort
/// the read past this bound and treat it as an unreadable board (empty result).
pub const MAX_MANIFEST_RESPONSE_BYTES: usize = 1024 * 1024;

/// Hard cap on how many entries we process from one board read, mirroring the
/// doorway's own `MAX_MANIFEST_ENTRIES`. Bounds the per-poll ed25519-verify CPU
/// a doorway can spend on our behalf even within the byte cap.
pub const MAX_BOOTSTRAP_ENTRIES: usize = 64;

/// Timeout for one doorway round-trip. A doorway is a convenience, never a
/// dependency — a slow one must not hold the announce tick or the joiner.
pub const DOORWAY_TIMEOUT: Duration = Duration::from_secs(5);

/// Where this node's doorway lives, or `None` — in which case every doorway
/// leg here is inert.
///
/// Resolution order:
/// 1. [`DOORWAY_URL_ENV`] — the explicit base.
/// 2. The **origin** of `DOORWAY_CAPABILITY_URL`, which storage already reads
///    to load its render-capability profile (`views_convert::infrastructure`).
///    That variable points at a full endpoint path, so only its
///    `scheme://host:port` is reused — the doorway serving `/admin/capability`
///    is the same doorway serving [`MANIFESTS_PATH`]. Reusing what the node
///    already knows beats asking operators to configure the same host twice.
pub fn doorway_base_url() -> Option<String> {
    if let Ok(explicit) = std::env::var(DOORWAY_URL_ENV) {
        let trimmed = explicit.trim().trim_end_matches('/');
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    let capability = std::env::var("DOORWAY_CAPABILITY_URL").ok()?;
    origin_of(&capability)
}

/// `scheme://host[:port]` of a URL, or `None` if it does not parse.
fn origin_of(url: &str) -> Option<String> {
    let parsed = reqwest::Url::parse(url).ok()?;
    let host = parsed.host_str()?;
    let scheme = parsed.scheme();
    match parsed.port() {
        Some(port) => Some(format!("{scheme}://{host}:{port}")),
        None => Some(format!("{scheme}://{host}")),
    }
}

/// A short-lived HTTP client for doorway calls.
pub fn doorway_client() -> Option<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(DOORWAY_TIMEOUT)
        .build()
        .ok()
}

/// Publish one signed announcement to the doorway's bulletin board.
///
/// Best-effort by construction: a failure is logged at debug and the next
/// announce tick retries. The gossip publish is the primary channel; this is
/// the leg that lets a peer with NO gossip reach be discovered at all.
pub async fn post_manifest(
    client: &reqwest::Client,
    base_url: &str,
    ann: &TransportManifestAnnouncement,
) -> bool {
    let url = format!("{base_url}{MANIFESTS_PATH}");
    match client.post(&url).json(ann).send().await {
        Ok(resp) if resp.status().is_success() => {
            debug!(url = %url, node = %ann.iroh_node_id, "doorway: manifest posted");
            true
        }
        Ok(resp) => {
            debug!(url = %url, status = %resp.status(), "doorway: manifest POST rejected — retry next tick");
            false
        }
        Err(e) => {
            debug!(url = %url, error = %e, "doorway: manifest POST failed — retry next tick");
            false
        }
    }
}

/// Read the doorway's bulletin board.
///
/// Accepts either a bare JSON array of announcements or an object wrapping one
/// under `manifests`/`items` — the doorway may store opaquely and we do not
/// want a cosmetic envelope difference to read as "no peers".
pub async fn fetch_manifests(client: &reqwest::Client, base_url: &str) -> Vec<serde_json::Value> {
    let url = format!("{base_url}{MANIFESTS_PATH}");
    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            debug!(url = %url, error = %e, "doorway: manifest GET failed");
            return Vec::new();
        }
    };
    if !resp.status().is_success() {
        debug!(url = %url, status = %resp.status(), "doorway: manifest GET returned non-success");
        return Vec::new();
    }
    // Reject an oversized body BEFORE buffering it — a Content-Length past the
    // cap never gets read at all. `reqwest` does not bound `.json()`/`.bytes()`
    // on its own, so an unbounded read here is a malicious-doorway OOM vector.
    if let Some(len) = resp.content_length() {
        if len > MAX_MANIFEST_RESPONSE_BYTES as u64 {
            warn!(
                url = %url, content_length = len, cap = MAX_MANIFEST_RESPONSE_BYTES,
                "doorway: manifest GET body exceeds cap by Content-Length — refused unread"
            );
            return Vec::new();
        }
    }
    // Stream the body with a running accumulator that aborts past the cap, so a
    // chunked response (no Content-Length) cannot buffer past the bound either.
    let mut buf: Vec<u8> = Vec::new();
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(bytes) => {
                if buf.len() + bytes.len() > MAX_MANIFEST_RESPONSE_BYTES {
                    warn!(
                        url = %url, cap = MAX_MANIFEST_RESPONSE_BYTES,
                        "doorway: manifest GET body exceeded cap mid-stream — aborted"
                    );
                    return Vec::new();
                }
                buf.extend_from_slice(&bytes);
            }
            Err(e) => {
                debug!(url = %url, error = %e, "doorway: manifest GET body read failed");
                return Vec::new();
            }
        }
    }
    let body: serde_json::Value = match serde_json::from_slice(&buf) {
        Ok(v) => v,
        Err(e) => {
            debug!(url = %url, error = %e, "doorway: manifest GET body did not parse as JSON");
            return Vec::new();
        }
    };
    cap_entries(entries_of(body), &url)
}

/// Cap entries processed per read — bounds per-poll verify CPU a doorway can
/// spend on us even within the byte cap. Pure, so the bound is unit-testable
/// without an HTTP round-trip.
fn cap_entries(mut entries: Vec<serde_json::Value>, url: &str) -> Vec<serde_json::Value> {
    if entries.len() > MAX_BOOTSTRAP_ENTRIES {
        warn!(
            url = %url, got = entries.len(), cap = MAX_BOOTSTRAP_ENTRIES,
            "doorway: manifest GET returned more entries than the cap — truncated"
        );
        entries.truncate(MAX_BOOTSTRAP_ENTRIES);
    }
    entries
}

/// Pull the announcement list out of whichever envelope the doorway used.
fn entries_of(body: serde_json::Value) -> Vec<serde_json::Value> {
    match body {
        serde_json::Value::Array(items) => items,
        serde_json::Value::Object(mut map) => map
            .remove("manifests")
            .or_else(|| map.remove("items"))
            .and_then(|v| match v {
                serde_json::Value::Array(items) => Some(items),
                _ => None,
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// Outcome of one bulletin-board read.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct BootstrapOutcome {
    /// Entries the book actually took (new peer, or a fresher announcement).
    pub accepted: usize,
    /// Well-formed, verified announcements the book declined as stale.
    pub stale: usize,
    /// Entries that did not decode or did not verify — the doorway is NOT
    /// trusted, so this is an expected, non-fatal outcome.
    pub rejected: usize,
    /// Our own manifest, echoed back. Never a peer.
    pub own: usize,
}

/// Decode → **verify** → upsert. The one place doorway-sourced entries enter
/// the book, and the reason the book's signature invariant survives the
/// concession (see the module docs).
pub fn verify_and_upsert(
    book: &IrohPeerBook,
    self_node_id: Option<&str>,
    entries: Vec<serde_json::Value>,
) -> BootstrapOutcome {
    let mut out = BootstrapOutcome::default();
    for entry in entries {
        let ann: TransportManifestAnnouncement = match serde_json::from_value(entry) {
            Ok(a) => a,
            Err(e) => {
                out.rejected += 1;
                debug!(error = %e, "doorway bootstrap: entry did not decode as a transport manifest");
                continue;
            }
        };
        if let Err(e) = ann.verify() {
            out.rejected += 1;
            crate::metrics::inc_iroh_manifest_announcement("bad_signature");
            warn!(
                node = %ann.iroh_node_id, reason = %e,
                "doorway bootstrap: manifest failed verification — dropped (the doorway is not trusted)"
            );
            continue;
        }
        if self_node_id == Some(ann.iroh_node_id.as_str()) {
            out.own += 1;
            crate::metrics::inc_iroh_manifest_announcement("self");
            continue;
        }
        // `accept` re-derives the dial target and applies the book's monotone
        // upsert, so a replayed stale manifest cannot roll addresses backwards.
        if book.accept(&ann) {
            out.accepted += 1;
            crate::metrics::inc_iroh_manifest_announcement("accepted");
            info!(
                node = %ann.iroh_node_id, addrs = ?ann.iroh_direct_addrs,
                "iroh peer learned from the doorway bulletin board"
            );
        } else {
            out.stale += 1;
            crate::metrics::inc_iroh_manifest_announcement("stale");
        }
    }
    out
}

/// Static inputs for the bootstrap joiner.
pub struct DoorwayBootstrapInputs {
    pub base_url: String,
    pub book: IrohPeerBook,
    /// Our own iroh NodeId, so the board's echo of our own manifest is skipped.
    pub self_node_id: String,
}

/// Boot-seed retry cadence while the board reads empty. Sized with
/// [`BOOT_SEED_ATTEMPTS`] to finish inside `acquisition::FIRST_DRAIN_HOLD`
/// (10 s): 5 reads at 0/2/4/6/8 s, so the last one still releases the drain
/// on its 10 s tick. Bounded: at most `BOOT_SEED_ATTEMPTS` GETs per boot, and
/// only while the book is empty.
pub const BOOT_SEED_RETRY: Duration = Duration::from_secs(2);

/// See [`BOOT_SEED_RETRY`].
pub const BOOT_SEED_ATTEMPTS: u32 = 5;

/// Spawn the bootstrap joiner: one **boot seed** if the book is empty right
/// now, then — after `grace` — periodically read the doorway's board so peers
/// that register after this node booted join the live membership projection.
///
/// The boot seed is the pull leg's release. `acquisition::first_drain` holds
/// the first acquisition drain on this book for at most `FIRST_DRAIN_HOLD`
/// (10 s); the book otherwise fills from the transport-manifest gossip round,
/// which lands every 30 s (measured 2026-08-29, `pull-leg-drains-before-iroh-
/// book-warms`) — so without this read the hold always expired first and the
/// selector saw single-plane peers. One bounded GET at T0 turns the release
/// from a deadline into an event: the board holds every survivor's signed
/// manifest (24 h TTL), verified here before the book sees it.
///
/// Gossip remains the primary channel and can fill the book within seconds on
/// any node that can already reach the announcer. The recurring board read is
/// the liveness leg for an organic late joiner that has registered with the
/// doorway but has no gossip adjacency to the running fleet yet. Every entry
/// still passes client-side signature verification and the book's monotone
/// upsert, so making membership live adds no doorway authority.
///
/// Nothing here joins gossip topics: the receive loop already re-runs
/// `join_peers` on every book change (`gossip_receive::run_topic_receive`
/// selects on `book.subscribe()`), so seeding the book IS joining.
///
/// bounded-work: at most one GET per `retry` tick; each response is already
/// bounded by [`MAX_MANIFEST_RESPONSE_BYTES`] and [`MAX_BOOTSTRAP_ENTRIES`].
pub fn spawn_doorway_bootstrap(
    inputs: DoorwayBootstrapInputs,
    grace: Duration,
    retry: Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let Some(client) = doorway_client() else {
            warn!("doorway bootstrap: could not build an HTTP client — leg inert");
            return;
        };
        // T0 boot seed: the one read that runs BEFORE gossip's first refusal,
        // because the first acquisition drain is waiting on it (see the doc
        // above). A warm book (never at a real boot; tests) issues nothing.
        // Bounded retry: on a cold SIMULTANEOUS start the first peer up reads
        // an empty board (nobody has announced yet — measured 2026-08-29:
        // james `{boot,empty}`, then `expired` at 10 s while its neighbours
        // posted at +2 s). Re-reading every BOOT_SEED_RETRY for at most
        // BOOT_SEED_ATTEMPTS keeps the seed inside the drain's 10 s hold; a
        // warm restart seeds on the first read and never sleeps here.
        let mut attempts = 0;
        while inputs.book.is_empty() && attempts < BOOT_SEED_ATTEMPTS {
            read_board(&client, &inputs, BootstrapPhase::Boot).await;
            attempts += 1;
            if inputs.book.is_empty() && attempts < BOOT_SEED_ATTEMPTS {
                tokio::time::sleep(BOOT_SEED_RETRY).await;
            }
        }
        // The grace period gives gossip first refusal. After it elapses the
        // board becomes a recurring membership source: an already-warm book
        // must still learn peers that registered after this node booted.
        tokio::time::sleep(grace).await;
        info!(
            doorway = %inputs.base_url,
            "doorway bootstrap: watching the live membership board"
        );
        let mut ticker = tokio::time::interval(retry);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            read_board(&client, &inputs, BootstrapPhase::Watch).await;
        }
    })
}

/// Which leg of the joiner issued a board read — the `phase` label on
/// `elohim_iroh_doorway_bootstrap_reads_total`. A boot with
/// `{phase="boot",result="seeded"}` = 1 is the shape-3 path firing; a fleet
/// where `boot` only ever reads `empty`/`unreachable` has doorways with no
/// board, and the pull leg is back on the 10 s floor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootstrapPhase {
    Boot,
    Watch,
}

impl BootstrapPhase {
    pub const ALL: &'static [BootstrapPhase] = &[BootstrapPhase::Boot, BootstrapPhase::Watch];

    pub fn label(self) -> &'static str {
        match self {
            BootstrapPhase::Boot => "boot",
            BootstrapPhase::Watch => "watch",
        }
    }
}

/// One board read: fetch → verify → upsert, counted by phase and result.
/// `empty` covers an unreachable doorway too — `fetch_manifests` folds every
/// failure into an empty list, and the leg treats both as "nothing to seed".
async fn read_board(
    client: &reqwest::Client,
    inputs: &DoorwayBootstrapInputs,
    phase: BootstrapPhase,
) -> BootstrapOutcome {
    let entries = fetch_manifests(client, &inputs.base_url).await;
    if entries.is_empty() {
        debug!(doorway = %inputs.base_url, phase = phase.label(), "doorway bootstrap: board is empty");
        crate::metrics::inc_iroh_doorway_bootstrap_read(phase.label(), "empty");
        return BootstrapOutcome::default();
    }
    let outcome = verify_and_upsert(&inputs.book, Some(&inputs.self_node_id), entries);
    let result = if outcome.accepted > 0 {
        "seeded"
    } else {
        "none_accepted"
    };
    crate::metrics::inc_iroh_doorway_bootstrap_read(phase.label(), result);
    info!(
        doorway = %inputs.base_url, phase = phase.label(),
        accepted = outcome.accepted, stale = outcome.stale,
        rejected = outcome.rejected, own = outcome.own,
        "doorway bootstrap: board read"
    );
    crate::metrics::set_iroh_peers_known(inputs.book.len());
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use iroh::SecretKey;

    fn signed(secret: &[u8; 32], port: u16, at_ms: i64) -> TransportManifestAnnouncement {
        TransportManifestAnnouncement::sign(
            secret,
            vec![format!("127.0.0.1:{port}")],
            None,
            Some("agent-doorway".into()),
            None,
            vec!["sync".into()],
            at_ms,
        )
    }

    fn as_json(ann: &TransportManifestAnnouncement) -> serde_json::Value {
        serde_json::to_value(ann).expect("announcement serialises as JSON")
    }

    /// The happy path, and the shape the doorway is expected to hand back.
    #[test]
    fn a_verified_manifest_seeds_the_book() {
        let book = IrohPeerBook::new();
        let ann = signed(&[9u8; 32], 4001, 100);
        let out = verify_and_upsert(&book, None, vec![as_json(&ann)]);
        assert_eq!(out.accepted, 1);
        assert_eq!(out.rejected, 0);
        assert_eq!(book.len(), 1);
        let node_id: iroh::NodeId = ann.iroh_node_id.parse().unwrap();
        assert_eq!(
            book.get(&node_id).unwrap().addr.direct_addresses.len(),
            1,
            "the dial target must come from the announcement"
        );
    }

    /// The load-bearing one: the doorway may store opaquely, so a tampered
    /// entry MUST die at the client. If this ever passes a bad signature into
    /// the book, a compromised doorway can redirect every dial in the fleet.
    #[test]
    fn a_tampered_manifest_never_reaches_the_book() {
        let book = IrohPeerBook::new();
        let ann = signed(&[9u8; 32], 4002, 100);
        let mut tampered = as_json(&ann);
        tampered["irohDirectAddrs"] = serde_json::json!(["10.0.0.1:66"]);
        // Field naming is serde-derived (snake_case here); tamper the real one
        // too so the test cannot pass by editing a field that does not exist.
        tampered["iroh_direct_addrs"] = serde_json::json!(["10.0.0.1:66"]);

        let out = verify_and_upsert(&book, None, vec![tampered]);
        assert_eq!(out.accepted, 0);
        assert_eq!(out.rejected, 1, "a tampered manifest must be REJECTED");
        assert!(
            book.is_empty(),
            "the book must never take an unverified entry"
        );
    }

    /// Garbage on the board is expected (it is a public bulletin board) and is
    /// counted, not fatal — one bad row must not poison the rest of the read.
    #[test]
    fn undecodable_entries_are_dropped_without_poisoning_the_batch() {
        let book = IrohPeerBook::new();
        let good = signed(&[11u8; 32], 4003, 100);
        let out = verify_and_upsert(
            &book,
            None,
            vec![
                serde_json::json!({"not": "a manifest"}),
                serde_json::json!("nonsense"),
                as_json(&good),
            ],
        );
        assert_eq!(out.accepted, 1);
        assert_eq!(out.rejected, 2);
        assert_eq!(book.len(), 1);
    }

    /// A doorway replaying an OLD manifest must not roll a peer's addresses
    /// backwards — the book's monotone upsert is what guarantees it, and this
    /// pins that the doorway path goes through it.
    #[test]
    fn a_stale_announcement_never_replaces_a_fresher_one() {
        let book = IrohPeerBook::new();
        let secret = [13u8; 32];
        let fresh = signed(&secret, 4100, 500);
        let stale = signed(&secret, 4200, 100);

        assert_eq!(
            verify_and_upsert(&book, None, vec![as_json(&fresh)]).accepted,
            1
        );
        let out = verify_and_upsert(&book, None, vec![as_json(&stale)]);
        assert_eq!(out.accepted, 0);
        assert_eq!(out.stale, 1, "an older announcement is stale, not rejected");

        let node_id: iroh::NodeId = fresh.iroh_node_id.parse().unwrap();
        let held = book.get(&node_id).unwrap();
        assert!(
            held.addr
                .direct_addresses
                .contains(&([127, 0, 0, 1], 4100u16).into()),
            "the fresher announcement's address must survive the replay"
        );
        assert_eq!(held.announced_at_ms, 500);
    }

    /// Our own manifest, echoed back by the board, is not a peer.
    #[test]
    fn our_own_manifest_is_skipped() {
        let book = IrohPeerBook::new();
        let mut rng = rand::rngs::OsRng;
        let key = SecretKey::generate(&mut rng);
        let ann = signed(&key.to_bytes(), 4300, 100);
        let out = verify_and_upsert(&book, Some(&ann.iroh_node_id), vec![as_json(&ann)]);
        assert_eq!(out.own, 1);
        assert_eq!(out.accepted, 0);
        assert!(book.is_empty());
    }

    /// Both envelope shapes read, so a cosmetic difference on the doorway side
    /// cannot silently read as "no peers on the board".
    #[test]
    fn both_board_envelopes_are_read() {
        let ann = as_json(&signed(&[17u8; 32], 4400, 100));
        assert_eq!(entries_of(serde_json::json!([ann.clone()])).len(), 1);
        assert_eq!(
            entries_of(serde_json::json!({ "manifests": [ann.clone()] })).len(),
            1
        );
        assert_eq!(entries_of(serde_json::json!({ "items": [ann] })).len(), 1);
        assert!(entries_of(serde_json::json!({ "other": 1 })).is_empty());
        assert!(entries_of(serde_json::json!(7)).is_empty());
    }

    #[test]
    fn an_over_count_entry_list_is_truncated_to_the_cap() {
        // A malicious doorway serving more entries than the cap cannot make us
        // verify unbounded signatures per poll.
        let flood: Vec<serde_json::Value> = (0..MAX_BOOTSTRAP_ENTRIES * 4)
            .map(|_| serde_json::json!({}))
            .collect();
        assert_eq!(
            cap_entries(flood, "http://test").len(),
            MAX_BOOTSTRAP_ENTRIES
        );
        // A list within the cap is untouched.
        let ok: Vec<serde_json::Value> = (0..3).map(|_| serde_json::json!({})).collect();
        assert_eq!(cap_entries(ok, "http://test").len(), 3);
    }

    /// The origin reuse: storage already knows its doorway through
    /// `DOORWAY_CAPABILITY_URL`, which names a full endpoint path.
    #[test]
    fn capability_url_reduces_to_its_origin() {
        assert_eq!(
            origin_of("https://doorway-alpha.elohim.host/admin/capability").as_deref(),
            Some("https://doorway-alpha.elohim.host")
        );
        assert_eq!(
            origin_of("http://localhost:8080/admin/capability").as_deref(),
            Some("http://localhost:8080")
        );
        assert_eq!(origin_of("not a url"), None);
    }

    /// The emitter's labels are the pre-touched vocabulary — a rename here
    /// without one in `metrics.rs` would read as a series appearing from
    /// nowhere on the fleet.
    #[test]
    fn bootstrap_phase_labels_are_the_pretouched_vocabulary() {
        for phase in BootstrapPhase::ALL {
            assert!(
                crate::metrics::IROH_DOORWAY_BOOTSTRAP_PHASES.contains(&phase.label()),
                "{phase:?} is not pre-touched"
            );
        }
        for result in ["seeded", "none_accepted", "empty"] {
            assert!(crate::metrics::IROH_DOORWAY_BOOTSTRAP_READ_RESULTS.contains(&result));
        }
    }

    fn board_with(ann: &TransportManifestAnnouncement) -> serde_json::Value {
        serde_json::json!({ "manifests": [as_json(ann)] })
    }

    /// Shape 3 of `pull-leg-drains-before-iroh-book-warms`: an empty book at
    /// boot is seeded from the board IMMEDIATELY, not after the grace period.
    /// The grace here is 60 s and the test bounds the seed at 5 s, so a seed
    /// that waited for the grace would fail, and so would one that never ran.
    #[tokio::test]
    async fn the_boot_seed_fills_an_empty_book_before_the_grace_elapses() {
        let server = wiremock::MockServer::start().await;
        let ann = signed(&[21u8; 32], 4500, 100);
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path(MANIFESTS_PATH))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(board_with(&ann)))
            .mount(&server)
            .await;
        let book = IrohPeerBook::new();
        let handle = spawn_doorway_bootstrap(
            DoorwayBootstrapInputs {
                base_url: server.uri(),
                book: book.clone(),
                self_node_id: "not-the-announcer".into(),
            },
            Duration::from_secs(60),
            Duration::from_secs(60),
        );
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while book.is_empty() && tokio::time::Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        handle.abort();
        assert_eq!(
            book.len(),
            1,
            "the boot seed must fill the book well inside the 60 s grace"
        );
        let node_id: iroh::NodeId = ann.iroh_node_id.parse().unwrap();
        assert!(
            book.get(&node_id).is_some(),
            "the seeded peer is the board's announcer"
        );
        assert_eq!(
            server.received_requests().await.unwrap().len(),
            1,
            "exactly one GET at boot; the watch leg is still inside its grace"
        );
    }

    /// The cold-simultaneous-start shape: the board is empty on the first read
    /// (nobody has announced yet) and fills a moment later. The boot seed must
    /// re-read inside the hold rather than sleep through the grace.
    #[tokio::test]
    async fn an_empty_board_at_boot_is_re_read_inside_the_hold() {
        let server = wiremock::MockServer::start().await;
        // First read: empty board. Mounted first, expires after one match.
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path(MANIFESTS_PATH))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        let ann = signed(&[25u8; 32], 4700, 100);
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path(MANIFESTS_PATH))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(board_with(&ann)))
            .mount(&server)
            .await;
        let book = IrohPeerBook::new();
        let handle = spawn_doorway_bootstrap(
            DoorwayBootstrapInputs {
                base_url: server.uri(),
                book: book.clone(),
                self_node_id: "self".into(),
            },
            Duration::from_secs(60),
            Duration::from_secs(60),
        );
        let deadline = tokio::time::Instant::now() + BOOT_SEED_RETRY * 3;
        while book.is_empty() && tokio::time::Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        handle.abort();
        assert_eq!(book.len(), 1, "the second read must seed the book");
        assert_eq!(
            server.received_requests().await.unwrap().len(),
            2,
            "empty, then seeded — and no further reads once the book is warm"
        );
    }

    /// The boot seed is one GET at most, and none when the book is already
    /// warm — a node never pays a doorway round-trip it has no use for.
    #[tokio::test]
    async fn a_warm_book_issues_no_boot_read() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&server)
            .await;
        let book = IrohPeerBook::new();
        assert!(book.accept(&signed(&[23u8; 32], 4600, 100)));
        let handle = spawn_doorway_bootstrap(
            DoorwayBootstrapInputs {
                base_url: server.uri(),
                book: book.clone(),
                self_node_id: "self".into(),
            },
            Duration::from_secs(60),
            Duration::from_secs(60),
        );
        tokio::time::sleep(Duration::from_millis(400)).await;
        handle.abort();
        assert!(
            server.received_requests().await.unwrap().is_empty(),
            "a warm book must not issue a boot read before the watch grace elapses"
        );
    }

    /// A peer that registers after this node has already booted must be learned
    /// without clearing the warm book or restarting the fleet. This is the
    /// W2 late-joiner shape from `late-joiner-peer-discovery-boot-only-board`.
    #[tokio::test]
    async fn a_warm_book_learns_a_peer_that_joins_after_boot() {
        let server = wiremock::MockServer::start().await;
        let late_joiner = signed(&[29u8; 32], 4800, 200);
        let book = IrohPeerBook::new();
        assert!(book.accept(&signed(&[27u8; 32], 4700, 100)));
        let handle = spawn_doorway_bootstrap(
            DoorwayBootstrapInputs {
                base_url: server.uri(),
                book: book.clone(),
                self_node_id: "self".into(),
            },
            Duration::from_millis(25),
            Duration::from_millis(50),
        );

        // The node is already running with a warm book when the new peer
        // appears on the doorway board.
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path(MANIFESTS_PATH))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(board_with(&late_joiner)),
            )
            .mount(&server)
            .await;

        let late_node_id: iroh::NodeId = late_joiner.iroh_node_id.parse().unwrap();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        while book.get(&late_node_id).is_none() && tokio::time::Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        handle.abort();

        assert!(
            book.get(&late_node_id).is_some(),
            "the recurring board read must add a late joiner to an already-warm book"
        );
        assert!(
            !server.received_requests().await.unwrap().is_empty(),
            "the watch leg must read the board even while the book is warm"
        );
    }
}

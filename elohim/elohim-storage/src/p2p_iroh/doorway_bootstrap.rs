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

use tracing::{debug, info, warn};

use super::peer_book::IrohPeerBook;
use crate::p2p::gossip_dispatch::TransportManifestSink;
use crate::p2p::transport_manifest_gossip::TransportManifestAnnouncement;

/// Explicit doorway base URL (scheme://host[:port]). Highest precedence.
pub const DOORWAY_URL_ENV: &str = "ELOHIM_DOORWAY_URL";

/// The doorway's bounded manifest bulletin board.
pub const MANIFESTS_PATH: &str = "/p2p/manifests";

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
    let body: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            debug!(url = %url, error = %e, "doorway: manifest GET body did not parse as JSON");
            return Vec::new();
        }
    };
    entries_of(body)
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

/// Spawn the bootstrap joiner: after `grace`, whenever the book is EMPTY, read
/// the doorway's board and seed it.
///
/// Empty-only by design. Gossip is the primary channel and fills the book
/// within seconds on any node that can reach a peer; the doorway leg exists for
/// the node that cannot. Gating on emptiness means a dual-mode node pays one
/// cheap check per tick and never a request, while a pure-iroh node — or one
/// whose book a partition emptied — re-bootstraps without any mode flag to set.
///
/// Nothing here joins gossip topics: the receive loop already re-runs
/// `join_peers` on every book change (`gossip_receive::run_topic_receive`
/// selects on `book.subscribe()`), so seeding the book IS joining.
///
/// bounded-work: at most one GET per `retry` tick, and only when the book is
/// empty.
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
        // The grace period is what makes this a FALLBACK: gossip gets first
        // refusal on filling the book.
        tokio::time::sleep(grace).await;
        info!(
            doorway = %inputs.base_url,
            "doorway bootstrap: watching for an empty peer book"
        );
        let mut ticker = tokio::time::interval(retry);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            if !inputs.book.is_empty() {
                continue;
            }
            let entries = fetch_manifests(&client, &inputs.base_url).await;
            if entries.is_empty() {
                debug!(doorway = %inputs.base_url, "doorway bootstrap: board is empty");
                continue;
            }
            let outcome = verify_and_upsert(&inputs.book, Some(&inputs.self_node_id), entries);
            info!(
                doorway = %inputs.base_url,
                accepted = outcome.accepted, stale = outcome.stale,
                rejected = outcome.rejected, own = outcome.own,
                "doorway bootstrap: board read"
            );
            crate::metrics::set_iroh_peers_known(inputs.book.len());
        }
    })
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
}

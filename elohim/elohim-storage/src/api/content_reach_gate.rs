//! The reach gate every READ of a content atom passes through.
//!
//! One gate, applied at the `/db` dispatch before any content-scoped GET is
//! routed, so the body route and its siblings (`/head`, `/head-record`,
//! `/schedule`) and the atom's edges (`/db/relationships?contentId=…`,
//! `/db/relationships/graph/{id}`, `/db/relationships/{relId}`) answer the SAME
//! question the same way. Existence and timing are content: an intimate row's
//! head says a household keeps it and when they last touched it, so a route
//! that reveals the row at all is a read of it (backlog
//! `security-content-head-route-bypasses-reach-gate`).
//!
//! The decision itself is the body route's, moved here verbatim:
//!
//! - **Layer 1** — `commons` / `public` serve anonymously; every other tier
//!   needs a caller that RESOLVES to an identity through the explicit
//!   doorway-injected `X-Agent-Cid` header (never header presence, never the
//!   ambient local session).
//! - **Layer 1.5** — tiers ABOVE `community` (familiar / trusted / intimate /
//!   self) run the same steward-specific authorization the P2P transport path
//!   uses (`EprService::authorize_reach_for_human_with_own_trust`), deny by
//!   default when the requester cannot be resolved.
//!
//! A target whose row does not exist is not gated at dispatch: the route
//! answers for itself. That is NOT always a 404 — the body route falls back to
//! P2P resolution when it holds no row, and that fallback runs the SAME reach
//! check ([`reach_refusal`]) on the RESOLVED head's reach before it persists or
//! serves anything (a resolved head that states no reach is refused, never
//! defaulted open). The refusal bodies keep their `requiredReach` field —
//! callers (the seeder's idempotent lookup) parse it.
//!
//! ## Edges: both endpoints, every edge
//!
//! Gating the atom NAMED in a request is not enough for edges: a commons atom's
//! edge list, a single edge by id, or a graph walk can each NAME an intimate
//! neighbour. So every edge-returning route also runs [`ReachFilter`] over what
//! it returns: an edge is served only if the caller may read BOTH its source
//! and its target, and a graph walk prunes an unreadable node and never
//! traverses THROUGH it ([`crate::graph_engine::NodeAdmission`]). The list
//! route also pushes the open-tier filter into SQL for a caller who reads only
//! open tiers, so its page counts and offsets reveal no hidden edge.
//!
//! An endpoint with a local row is judged by that row. An endpoint with NO
//! local row (an atom held elsewhere) is judged by the edge ONLY when the edge
//! was projected from a verified signed head — then at the edge's reach, which
//! is its source atom's ([`crate::db::authored_edges::unknown_endpoint_reach`]).
//! A POSTed edge naming an id this peer does not hold vouches for nothing: that
//! endpoint is read by no one, so a fabricated commons edge can never name an
//! intimate atom held elsewhere.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use bytes::Bytes;
use diesel::SqliteConnection;
use http_body_util::Full;
use hyper::{header, Response, StatusCode};

use crate::db::{self, AppContext};
use crate::error::StorageError;

/// A content-scoped GET that reveals an atom (or its edges).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatedRead {
    /// `GET /db/content/{id}`
    Body,
    /// `GET /db/content/{id}/schedule`
    Schedule,
    /// `GET /db/content/{id}/head`
    Head,
    /// `GET /db/content/{id}/head-record`
    HeadRecord,
    /// `GET /db/relationships?contentId={id}`
    RelationshipsOf,
    /// `GET /db/relationships/graph/{id}`
    RelationshipGraph,
    /// `GET /db/relationships/{relId}` — gated here by the edge's SOURCE atom;
    /// the route then refuses (as absent) an edge whose TARGET the caller may
    /// not read ([`ReachFilter`]).
    Relationship,
}

/// Sub-routes of `content/{id}/…` that read the atom. The dispatch strips these
/// suffixes (in this order — `/head-record` before `/head`) before treating the
/// remainder as the id.
const CONTENT_READ_SUFFIXES: &[(&str, GatedRead)] = &[
    ("/schedule", GatedRead::Schedule),
    ("/head-record", GatedRead::HeadRecord),
    ("/head", GatedRead::Head),
];

/// Exact `content/…` resource paths that are collections, not an atom.
const CONTENT_COLLECTION_PATHS: &[&str] = &["content", "content/bulk", "content/search"];

/// Which gated read a `/db` GET targets, and the DECODED key that names its
/// subject: a content id for every arm except [`GatedRead::Relationship`],
/// whose key is the relationship id. Path segments decode exactly as the
/// routes' own leaf decode does (`percent_decode … decode_utf8_lossy`), so the
/// gate looks up the same row the route will.
///
/// `query` is the request's query string (only `relationships` reads it).
/// Returns `None` for every path that is not a content-scoped read.
pub fn gated_read_target(resource_path: &str, query: &str) -> Option<(GatedRead, String)> {
    if CONTENT_COLLECTION_PATHS.contains(&resource_path) {
        return None;
    }
    if let Some(rest) = resource_path.strip_prefix("content/") {
        for (suffix, route) in CONTENT_READ_SUFFIXES {
            if let Some(id) = rest.strip_suffix(suffix) {
                return Some((*route, decode(id)));
            }
        }
        return Some((GatedRead::Body, decode(rest)));
    }
    if resource_path == "relationships" {
        return url::form_urlencoded::parse(query.as_bytes())
            .find(|(k, _)| k == "contentId" || k == "content_id")
            .map(|(_, v)| (GatedRead::RelationshipsOf, v.into_owned()));
    }
    if let Some(id) = resource_path.strip_prefix("relationships/graph/") {
        return Some((GatedRead::RelationshipGraph, decode(id)));
    }
    if resource_path == "relationships/bulk" {
        return None;
    }
    if let Some(rel_id) = resource_path.strip_prefix("relationships/") {
        return Some((GatedRead::Relationship, decode(rel_id)));
    }
    None
}

fn decode(segment: &str) -> String {
    percent_encoding::percent_decode_str(segment)
        .decode_utf8_lossy()
        .into_owned()
}

/// The context a gated read resolves its atom under — the SAME context the
/// route itself reads with, so the gate and the route can never disagree about
/// which row they mean. The body route and the relationship routes read the
/// lamad default regardless of an app prefix; the sibling routes read `app_ctx`.
pub fn gate_context(route: GatedRead, app_ctx: &AppContext) -> AppContext {
    match route {
        GatedRead::Schedule | GatedRead::Head | GatedRead::HeadRecord => app_ctx.clone(),
        GatedRead::Body
        | GatedRead::RelationshipsOf
        | GatedRead::RelationshipGraph
        | GatedRead::Relationship => AppContext::default_lamad(),
    }
}

/// Resolve a gated read's subject to the content id whose reach governs it.
/// `None` when there is no such atom (the route answers its own 404).
fn subject_content_id(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    route: GatedRead,
    key: &str,
) -> Result<Option<String>, StorageError> {
    match route {
        GatedRead::Relationship => {
            Ok(db::relationships_diesel::get_relationship(conn, ctx, key)?.map(|r| r.source_id))
        }
        _ => Ok(Some(key.to_string())),
    }
}

/// The refusal for a gated read, or `None` to let the route answer.
pub fn gated_read_refusal(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    route: GatedRead,
    key: &str,
    agent_cid: Option<&str>,
    memo_store: Option<Arc<dyn crate::trust::VerificationMemoStore>>,
) -> Result<Option<Response<Full<Bytes>>>, StorageError> {
    let Some(content_id) = subject_content_id(conn, ctx, route, key)? else {
        return Ok(None);
    };
    let Some(reach) = db::content_diesel::reach_for(conn, ctx, &content_id)? else {
        return Ok(None);
    };
    Ok(reach_refusal(
        conn,
        ctx,
        &content_id,
        &reach,
        agent_cid,
        memo_store,
    ))
}

fn forbidden(body: String) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::FORBIDDEN)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

/// Layers 1 and 1.5 of the reach gate for one atom at `reach`.
pub fn reach_refusal(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    content_id: &str,
    reach: &str,
    agent_cid: Option<&str>,
    memo_store: Option<Arc<dyn crate::trust::VerificationMemoStore>>,
) -> Option<Response<Full<Bytes>>> {
    Caller::new(agent_cid, memo_store).refusal(conn, ctx, content_id, reach)
}

/// The requester of one request: who the explicit `X-Agent-Cid` names, and
/// the reach authorizer that judges them — each resolved at most ONCE and
/// only when a restricted tier first needs it, then reused for every atom the
/// request judges (a graph walk or an edge list asks per node).
struct Caller {
    agent_cid: Option<String>,
    memo_store: Option<Arc<dyn crate::trust::VerificationMemoStore>>,
    /// `humans.id`, then agent key — `None` inside when it resolves to no one.
    human: OnceLock<Option<crate::db::models::Human>>,
    authorizer: OnceLock<crate::epr_service::EprService>,
}

impl Caller {
    fn new(
        agent_cid: Option<&str>,
        memo_store: Option<Arc<dyn crate::trust::VerificationMemoStore>>,
    ) -> Self {
        Self {
            agent_cid: agent_cid.map(str::to_owned),
            memo_store,
            human: OnceLock::new(),
            authorizer: OnceLock::new(),
        }
    }

    fn human(&self, conn: &mut SqliteConnection) -> Option<&crate::db::models::Human> {
        self.human
            .get_or_init(|| {
                let idv = self.agent_cid.as_deref()?;
                crate::db::humans::get_human_by_id(conn, idv)
                    .ok()
                    .flatten()
                    .or_else(|| {
                        crate::db::humans::get_human_by_agent_key(conn, idv)
                            .ok()
                            .flatten()
                    })
            })
            .as_ref()
    }

    fn authorizer(&self) -> &crate::epr_service::EprService {
        self.authorizer.get_or_init(|| {
            crate::epr_service::EprService::new(
                None,
                None,
                None,
                crate::p2p::trust_cache::PeerTrustCache::new(),
            )
            // The memo store is the process-lifetime Arc wired at startup
            // (`HttpServer::with_memo_store`); the gradient selection lives in
            // the service's own dormancy gate (flag off ⇒ inert).
            .with_memo_store(self.memo_store.clone())
        })
    }

    /// Layers 1 and 1.5 for one atom at `reach`.
    fn refusal(
        &self,
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        content_id: &str,
        reach: &str,
    ) -> Option<Response<Full<Bytes>>> {
        // Layer 1: commons/public serve without auth; restricted content
        // requires a caller that RESOLVES via the explicit `X-Agent-Cid` header.
        let is_public = reach == "commons" || reach == "public";
        if !is_public && self.agent_cid.is_none() {
            return Some(forbidden(format!(
                r#"{{"error":"Authentication required","requiredReach":"{}"}}"#,
                reach
            )));
        }

        // Layer 1.5: steward-specific reach authorization for tiers ABOVE
        // community — the SAME gate the P2P transport path uses (single source
        // of truth), resolving the requester by humans.id, then agent key. Deny
        // by default when the requester cannot be resolved.
        if crate::epr_service::reach_level_index(reach)
            > crate::epr_service::reach_level_index("community")
        {
            let Some(human) = self.human(conn) else {
                return Some(forbidden(format!(
                    r#"{{"error":"Reach authorization required","requiredReach":"{}"}}"#,
                    reach
                )));
            };
            if let Err(reason) = self
                .authorizer()
                .authorize_reach_for_human_with_own_trust(conn, ctx, reach, human, content_id)
            {
                return Some(forbidden(format!(
                    r#"{{"error":"Reach denied","requiredReach":"{}","reason":"{}"}}"#,
                    reach, reason
                )));
            }
        }
        None
    }
}

/// Reaches Layer 1 serves to anyone, identified or not.
pub const OPEN_REACHES: &[&str] = &["commons", "public"];

/// The refusal for serving a head resolved over P2P (the body route's
/// fallback when this peer holds no row), judged by the RESOLVED head's own
/// reach. A head that states no reach is refused — the fallback used to
/// default it to `commons`, which widened whatever it was.
pub fn resolved_head_refusal(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    content_id: &str,
    head_reach: Option<&str>,
    agent_cid: Option<&str>,
    memo_store: Option<Arc<dyn crate::trust::VerificationMemoStore>>,
) -> Option<Response<Full<Bytes>>> {
    let reach = head_reach.map(str::trim).unwrap_or("");
    reach_refusal(conn, ctx, content_id, reach, agent_cid, memo_store)
}

/// The per-caller reach filter over everything an edge route returns — the
/// one reusable place edges are judged (see the module doc). The caller's
/// identity and authorizer are resolved once per filter (lazily, on the first
/// restricted atom); verdicts are memoized per (atom, reach) for the life of
/// one request.
pub struct ReachFilter {
    caller: Caller,
    verdicts: Mutex<HashMap<(String, String), bool>>,
}

impl std::fmt::Debug for ReachFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReachFilter")
            .field("identified", &self.caller.agent_cid.is_some())
            .finish_non_exhaustive()
    }
}

impl ReachFilter {
    /// A filter for the caller named by the explicit `X-Agent-Cid` header
    /// (`None` = anonymous).
    pub fn new(
        agent_cid: Option<&str>,
        memo_store: Option<Arc<dyn crate::trust::VerificationMemoStore>>,
    ) -> Self {
        Self {
            caller: Caller::new(agent_cid, memo_store),
            verdicts: Mutex::new(HashMap::new()),
        }
    }

    /// The reach set a list query may push into SQL: the open tiers for an
    /// anonymous caller (who reads nothing else), `None` for an identified one
    /// (whose rights above `community` are per-atom and judged in Rust).
    pub fn sql_readable_reaches(&self) -> Option<Vec<String>> {
        self.caller
            .agent_cid
            .is_none()
            .then(|| OPEN_REACHES.iter().map(|r| r.to_string()).collect())
    }

    /// May the caller read `content_id`? Its local row's reach governs; with no
    /// local row, `fallback_reach` does — for an edge endpoint that is
    /// [`crate::db::authored_edges::unknown_endpoint_reach`] (the edge's reach
    /// only when a verified head stated it, else `""`, which nobody reads). A
    /// lookup failure refuses.
    pub fn can_read_atom(
        &self,
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        content_id: &str,
        fallback_reach: &str,
    ) -> bool {
        let reach = match db::content_diesel::reach_for(conn, ctx, content_id) {
            Ok(Some(reach)) => reach,
            Ok(None) => fallback_reach.to_string(),
            Err(_) => return false,
        };
        let key = (content_id.to_string(), reach);
        if let Some(verdict) = self.verdicts.lock().ok().and_then(|m| m.get(&key).copied()) {
            return verdict;
        }
        let verdict = self.caller.refusal(conn, ctx, content_id, &key.1).is_none();
        if let Ok(mut m) = self.verdicts.lock() {
            m.insert(key, verdict);
        }
        verdict
    }

    /// May the caller see this edge? Only if it may read BOTH endpoints.
    pub fn can_read_edge(
        &self,
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        edge: &db::models::Relationship,
    ) -> bool {
        let unknown = db::authored_edges::unknown_endpoint_reach(edge);
        self.can_read_atom(conn, ctx, &edge.source_id, unknown)
            && self.can_read_atom(conn, ctx, &edge.target_id, unknown)
    }

    /// Keep only the edges the caller may see.
    pub fn retain_readable_edges(
        &self,
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        edges: Vec<db::models::Relationship>,
    ) -> Vec<db::models::Relationship> {
        edges
            .into_iter()
            .filter(|edge| self.can_read_edge(conn, ctx, edge))
            .collect()
    }
}

impl crate::graph_engine::NodeAdmission for ReachFilter {
    fn admits(
        &self,
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        node_id: &str,
        via: Option<&db::models::Relationship>,
    ) -> bool {
        let unknown = via.map_or("", db::authored_edges::unknown_endpoint_reach);
        self.can_read_atom(conn, ctx, node_id, unknown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_paths_name_their_route_and_raw_id() {
        assert_eq!(
            gated_read_target("content/love-map", ""),
            Some((GatedRead::Body, "love-map".into()))
        );
        assert_eq!(
            gated_read_target("content/love-map/head", ""),
            Some((GatedRead::Head, "love-map".into()))
        );
        assert_eq!(
            gated_read_target("content/love-map/head-record", ""),
            Some((GatedRead::HeadRecord, "love-map".into()))
        );
        assert_eq!(
            gated_read_target("content/love-map/schedule", ""),
            Some((GatedRead::Schedule, "love-map".into()))
        );
        // Encoded ids decode the way the routes' leaf decode does.
        assert_eq!(
            gated_read_target("content/a%20b/head", ""),
            Some((GatedRead::Head, "a b".into()))
        );
        for collection in ["content", "content/bulk", "content/search"] {
            assert_eq!(gated_read_target(collection, ""), None);
        }
    }

    /// The body route's P2P fallback judges the RESOLVED head's reach before
    /// persisting or serving: an intimate head is refused to an anonymous or
    /// unresolvable caller, a head stating no reach is refused (never
    /// defaulted open), and a commons head serves.
    #[test]
    fn a_p2p_resolved_head_is_judged_by_its_own_reach() {
        let pool = crate::test_util::test_pool();
        let mut conn = pool.get().unwrap();
        let ctx = AppContext::default_lamad();
        let status = |conn: &mut SqliteConnection, reach: Option<&str>, caller: Option<&str>| {
            resolved_head_refusal(conn, &ctx, "love-map-held-elsewhere", reach, caller, None)
                .map(|r| r.status().as_u16())
        };
        assert_eq!(status(&mut conn, Some("intimate"), None), Some(403));
        assert_eq!(
            status(&mut conn, Some("intimate"), Some("uhCAk-a-stranger")),
            Some(403)
        );
        assert_eq!(status(&mut conn, None, None), Some(403));
        assert_eq!(
            status(&mut conn, Some(" "), Some("uhCAk-a-stranger")),
            Some(403)
        );
        assert_eq!(status(&mut conn, Some("commons"), None), None);
    }

    /// An endpoint with no local row is judged by the edge's reach; a lookup
    /// of a held row uses the ROW's reach, whatever the edge says.
    #[test]
    fn the_filter_judges_missing_endpoints_by_edge_reach() {
        let pool = crate::test_util::test_pool();
        let mut conn = pool.get().unwrap();
        let ctx = AppContext::default_lamad();
        let anon = ReachFilter::new(None, None);
        assert!(anon.can_read_atom(&mut conn, &ctx, "held-elsewhere", "commons"));
        assert!(!anon.can_read_atom(&mut conn, &ctx, "held-elsewhere", "intimate"));
        assert!(!anon.can_read_atom(&mut conn, &ctx, "held-elsewhere", ""));
        assert_eq!(
            anon.sql_readable_reaches(),
            Some(vec!["commons".to_string(), "public".to_string()])
        );
        assert_eq!(
            ReachFilter::new(Some("uhCAk-x"), None).sql_readable_reaches(),
            None
        );
    }

    fn seed_atom(conn: &mut SqliteConnection, id: &str, reach: &str, created_by: Option<&str>) {
        db::content_diesel::create_content(
            conn,
            &AppContext::default_lamad(),
            db::content_diesel::CreateContentInput {
                id: id.into(),
                title: id.into(),
                description: None,
                content_type: "concept".into(),
                content_format: "markdown".into(),
                blob_hash: None,
                blob_cid: None,
                content_size_bytes: None,
                metadata_json: None,
                reach: reach.into(),
                created_by: created_by.map(str::to_owned),
                tags: Vec::new(),
                content_body: Some("body".into()),
                dht_anchor_hash: None,
            },
        )
        .unwrap();
    }

    fn edge(conn: &mut SqliteConnection, source: &str, target: &str) -> db::models::Relationship {
        use db::diesel_schema::relationships;
        use diesel::prelude::*;
        relationships::table
            .filter(relationships::source_id.eq(source))
            .filter(relationships::target_id.eq(target))
            .first(conn)
            .unwrap()
    }

    /// An endpoint this peer does not hold is vouched for only by an edge a
    /// verified head stated. A POSTed-shape commons edge from a commons atom to
    /// an unknown id is served to nobody — anonymous or identified; the same
    /// edge as the verified projection wrote it serves at its reach.
    #[test]
    fn an_unknown_endpoint_is_vouched_for_only_by_a_verified_edge() {
        let pool = crate::test_util::test_pool();
        let mut conn = pool.get().unwrap();
        let ctx = AppContext::default_lamad();
        seed_atom(&mut conn, "open-src", "commons", None);
        db::relationships_diesel::create_relationship(
            &mut conn,
            &ctx,
            db::relationships_diesel::CreateRelationshipInput {
                id: Some("rel-posted".into()),
                source_id: "open-src".into(),
                target_id: "fabricated-intimate-id".into(),
                relationship_type: "RELATES_TO".into(),
                confidence: 1.0,
                inference_source: "explicit".into(),
                is_bidirectional: false,
                provenance_chain_json: None,
                governance_layer: None,
                reach: Some("commons".into()),
                metadata_json: None,
            },
        )
        .unwrap();
        db::authored_edges::replace_authored_edges(
            &mut conn,
            &ctx,
            "open-src",
            &[
                db::authored_edges::AuthoredEdge {
                    relationship_type: "RELATES_TO".into(),
                    target_id: "fabricated-intimate-id".into(),
                    role: None,
                },
                db::authored_edges::AuthoredEdge {
                    relationship_type: "RELATES_TO".into(),
                    target_id: "commons-held-elsewhere".into(),
                    role: None,
                },
            ],
            "commons",
            Some("uhCkk-v1"),
        )
        .unwrap();
        // The projection took the POSTed slot over; POST a second unverified
        // edge to a fresh unknown id to keep one unmarked row.
        db::relationships_diesel::create_relationship(
            &mut conn,
            &ctx,
            db::relationships_diesel::CreateRelationshipInput {
                id: Some("rel-posted-2".into()),
                source_id: "open-src".into(),
                target_id: "another-fabricated-id".into(),
                relationship_type: "RELATES_TO".into(),
                confidence: 1.0,
                inference_source: "explicit".into(),
                is_bidirectional: false,
                provenance_chain_json: None,
                governance_layer: None,
                reach: Some("commons".into()),
                metadata_json: None,
            },
        )
        .unwrap();
        let verified = edge(&mut conn, "open-src", "commons-held-elsewhere");
        let posted = edge(&mut conn, "open-src", "another-fabricated-id");
        for caller in [None, Some("uhCAk-a-stranger")] {
            let filter = ReachFilter::new(caller, None);
            assert!(
                filter.can_read_edge(&mut conn, &ctx, &verified),
                "{caller:?}: a verified edge to a commons atom held elsewhere serves"
            );
            assert!(
                !filter.can_read_edge(&mut conn, &ctx, &posted),
                "{caller:?}: a POSTed edge vouches for no unknown id"
            );
            use crate::graph_engine::NodeAdmission;
            assert!(filter.admits(&mut conn, &ctx, "commons-held-elsewhere", Some(&verified)));
            assert!(!filter.admits(&mut conn, &ctx, "another-fabricated-id", Some(&posted)));
            assert!(!filter.admits(&mut conn, &ctx, "another-fabricated-id", None));
        }
    }

    /// The caller is resolved ONCE per filter and reused for every restricted
    /// atom: removing the human after the first verdict does not change the
    /// second (a fresh filter, resolving again, is refused).
    #[test]
    fn the_filter_resolves_its_caller_once() {
        use diesel::prelude::*;
        let pool = crate::test_util::test_pool();
        let mut conn = pool.get().unwrap();
        let ctx = AppContext::default_lamad();
        diesel::insert_into(db::diesel_schema::humans::table)
            .values(&db::models::NewHuman {
                id: "human-steward".into(),
                agent_pub_key: Some("uhCAk-steward".into()),
                display_name: "Steward".into(),
                bio: None,
                affinities: "[]".into(),
                profile_reach: "commons".into(),
                location: None,
                profile_photo_url: None,
                h_app_id: db::HUMANS_HAPP_ID.into(),
                household_id: None,
            })
            .execute(&mut conn)
            .unwrap();
        seed_atom(&mut conn, "journal-1", "self", Some("human-steward"));
        seed_atom(&mut conn, "journal-2", "self", Some("human-steward"));

        let filter = ReachFilter::new(Some("human-steward"), None);
        assert!(filter.can_read_atom(&mut conn, &ctx, "journal-1", ""));
        diesel::delete(db::diesel_schema::humans::table)
            .execute(&mut conn)
            .unwrap();
        assert!(
            filter.can_read_atom(&mut conn, &ctx, "journal-2", ""),
            "the second restricted atom reuses the identity resolved for the first"
        );
        assert!(
            !ReachFilter::new(Some("human-steward"), None).can_read_atom(
                &mut conn,
                &ctx,
                "journal-2",
                ""
            ),
            "a fresh filter resolves again and finds no one"
        );
    }

    #[test]
    fn relationship_paths_name_the_atom_they_reveal() {
        assert_eq!(
            gated_read_target("relationships", "contentId=love-map&direction=both"),
            Some((GatedRead::RelationshipsOf, "love-map".into()))
        );
        assert_eq!(gated_read_target("relationships", "limit=5"), None);
        assert_eq!(
            gated_read_target("relationships/graph/love-map", "depth=2"),
            Some((GatedRead::RelationshipGraph, "love-map".into()))
        );
        assert_eq!(
            gated_read_target("relationships/rel-1", ""),
            Some((GatedRead::Relationship, "rel-1".into()))
        );
        assert_eq!(gated_read_target("relationships/bulk", ""), None);
    }
}

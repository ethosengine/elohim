//! Source census for counted pending predicates and exact non-egress exemptions.

use std::path::{Path, PathBuf};

fn crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            out.push(path);
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct LegacySite {
    path: &'static str,
    needle: &'static str,
    maximum: usize,
    pending: Option<(&'static str, &'static str)>,
}

const LEGACY_SITES: &[LegacySite] = &[
    LegacySite {
        path: "api/epr.rs",
        needle: "reach_visible_to(",
        maximum: 8,
        pending: Some(("http_epr_api.rs", "P10")),
    },
    LegacySite {
        path: "epr_atom_service.rs",
        needle: "reach_gate_allows(",
        maximum: 2,
        pending: Some(("p2p_epr_atom.rs", "P8")),
    },
    LegacySite {
        path: "sync/projector.rs",
        needle: "reach_is_distribution_safe(",
        maximum: 3,
        pending: Some(("projection_sync.rs", "P5")),
    },
    LegacySite {
        path: "db/authored_edges.rs",
        needle: "reach_level_index(",
        maximum: 2,
        pending: None,
    },
    LegacySite {
        path: "db/content_diesel.rs",
        needle: "is_distribution_safe_reach(",
        maximum: 1,
        pending: None,
    },
    LegacySite {
        path: "epr_service.rs",
        needle: ".check_reach_authorization(",
        maximum: 1,
        pending: Some(("p2p_epr_resolve.rs", "P1")),
    },
    LegacySite {
        path: "epr_service.rs",
        needle: "reach_level_index(",
        maximum: 24,
        pending: None,
    },
    LegacySite {
        path: "http.rs",
        needle: "reach_level_index(",
        maximum: 1,
        pending: None,
    },
    LegacySite {
        path: "http.rs",
        needle: "is_distribution_safe_reach(",
        maximum: 1,
        pending: None,
    },
    LegacySite {
        path: "p2p/mod.rs",
        needle: "reach_gate_allows(",
        maximum: 7,
        pending: None,
    },
    LegacySite {
        path: "p2p/projection_reconcile.rs",
        needle: "is_distribution_safe_reach(",
        maximum: 1,
        pending: None,
    },
    LegacySite {
        path: "p2p/view_federation.rs",
        needle: "is_distribution_safe_reach(",
        maximum: 1,
        pending: None,
    },
    LegacySite {
        path: "services/relationship_service.rs",
        needle: "reach_level_index(",
        maximum: 3,
        pending: None,
    },
];

fn plane_is_pending(rule_file: &str, slice: &str) -> bool {
    let source = std::fs::read_to_string(
        crate_root()
            .join("src/reach_verifier/rules")
            .join(rule_file),
    )
    .unwrap();
    source.contains(&format!(
        r#"pub const STATUS: Status = Status::Pending("{slice}")"#
    ))
}

#[test]
fn legacy_reach_predicates_are_counted_and_shrink_with_plane_status() {
    let src = crate_root().join("src");
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    let needles = [
        ".check_reach_authorization(",
        "reach_level_index(",
        "is_distribution_safe_reach(",
        "reach_is_distribution_safe(",
        "reach_gate_allows(",
        "reach_visible_to(",
    ];
    let mut findings = Vec::new();
    for path in files {
        let relative = path.strip_prefix(&src).unwrap().to_string_lossy();
        if relative.starts_with("reach_verifier/") {
            continue;
        }
        let source = std::fs::read_to_string(&path).unwrap();
        for needle in needles {
            let count = source.matches(needle).count();
            if count == 0 {
                continue;
            }
            let Some(site) = LEGACY_SITES
                .iter()
                .find(|site| site.path == relative.as_ref() && site.needle == needle)
            else {
                findings.push(format!("{relative}: unclassified {needle} ({count})"));
                continue;
            };
            let maximum = match site.pending {
                Some((rule_file, slice)) if plane_is_pending(rule_file, slice) => site.maximum,
                Some(_) => 0,
                None => site.maximum,
            };
            if count > maximum {
                findings.push(format!(
                    "{relative}: {needle} count {count} exceeds status-aware maximum {maximum}"
                ));
            }
        }
    }
    assert!(
        findings.is_empty(),
        "new reach decision sites need an EgressPlane rule: {findings:?}"
    );
}

#[test]
fn every_rule_module_declares_its_owned_plane_and_status() {
    let rules = crate_root().join("src/reach_verifier/rules");
    let mut files = Vec::new();
    rust_files(&rules, &mut files);
    for path in files {
        if path.file_name().is_some_and(|name| name == "mod.rs") {
            continue;
        }
        let source = std::fs::read_to_string(&path).unwrap();
        assert!(source.contains("EgressPlane::"), "{}", path.display());
        assert!(
            source.contains("pub const STATUS"),
            "{} has no STATUS",
            path.display()
        );
    }
}

fn reach_plane_for_get(path: &str) -> Option<&'static str> {
    if path == "/" {
        return Some("HttpApp");
    }
    if path == "/db/content" || path == "/db/content/search" {
        return Some("HttpList");
    }
    if path.starts_with("/db/content/") {
        return Some("HttpRow");
    }
    if path.starts_with("/db/relationships") {
        return Some("HttpEdge");
    }
    if path.starts_with("/blob/") {
        return Some("HttpBlob");
    }
    if path.starts_with("/api/v1/blob/") {
        return Some("HttpBlob");
    }
    if path.starts_with("/apps/") {
        return Some("HttpApp");
    }
    if path.starts_with("/api/v1/epr") {
        return Some("HttpEprApi");
    }
    if path.starts_with("/epr-head/") {
        return Some("HttpEprHead");
    }
    if path.starts_with("/shard/") || path.starts_with("/ipfs/") || path.starts_with("/dag/") {
        return Some("HttpRaw");
    }
    if path.starts_with("/db/dht/")
        || path.starts_with("/db/source-chain/")
        || path.starts_with("/api/v1/compute/payloads/")
    {
        return Some("HttpRaw");
    }
    if path.starts_with("/lamad/") {
        return Some("HttpRow");
    }
    if path.starts_with("/sync/v1/") {
        return Some("HttpSync");
    }
    if path.starts_with("/account/export/") {
        return Some("HttpAccountExport");
    }
    None
}

fn not_egress_manifest_get(path: &str) -> bool {
    const EXACT: &[&str] = &[
        "/api/v1/account",
        "/api/v1/account/keys",
        "/api/v1/account/pending-recovery",
        "/api/v1/account/portal-hosts",
        "/api/v1/account/revocations",
        "/api/v1/agreements",
        "/api/v1/agreements/{id}",
        "/api/v1/attestations/unified",
        "/api/v1/attestations/unified/{id}",
        "/api/v1/cache/stream",
        "/api/v1/cluster",
        "/api/v1/collab/{cid}",
        "/api/v1/collective/{cid}",
        "/api/v1/commitments",
        "/api/v1/commitments/agent/{agent_id}",
        "/api/v1/commitments/capacity/ratios",
        "/api/v1/commitments/facing/rea",
        "/api/v1/commitments/{id}",
        "/api/v1/compute/dashboard",
        "/api/v1/contributors/me/dashboard",
        "/api/v1/contributors/{id}/dashboard",
        "/api/v1/contributors/{id}/impact",
        "/api/v1/contributors/{id}/recognition",
        "/api/v1/custodians/metrics",
        "/api/v1/custodians/metrics/alerts",
        "/api/v1/custodians/metrics/{id}",
        "/api/v1/custodians/metrics/{id}/recommendations",
        "/api/v1/custodians/protection/{id}",
        "/api/v1/economic-events",
        "/api/v1/economic-events/agent/{agent_id}",
        "/api/v1/economic-events/appreciations",
        "/api/v1/economic-events/content/{content_id}",
        "/api/v1/economic-events/{id}",
        "/api/v1/exchange/offers",
        "/api/v1/exchange/offers/{id}",
        "/api/v1/exchange/requests",
        "/api/v1/exchange/requests/{id}",
        "/api/v1/flow-planning",
        "/api/v1/governance-actions",
        "/api/v1/governance-actions/{id}",
        "/api/v1/governance-actions/{id}/tally",
        "/api/v1/governance/appeals/{challengeId}",
        "/api/v1/governance/challenges",
        "/api/v1/governance/challenges/{id}",
        "/api/v1/governance/discussions",
        "/api/v1/governance/discussions/{id}",
        "/api/v1/governance/precedents",
        "/api/v1/governance/precedents/{id}",
        "/api/v1/governance/proposals",
        "/api/v1/governance/proposals/{id}",
        "/api/v1/governance/proposals/{id}/options",
        "/api/v1/governance/proposals/{id}/ranked-votes",
        "/api/v1/governance/proposals/{id}/tally",
        "/api/v1/governance/proposals/{id}/votes",
        "/api/v1/governance/sensemaking/clusters",
        "/api/v1/governance/sensemaking/statements",
        "/api/v1/governance/sensemaking/votes",
        "/api/v1/governance/signals",
        "/api/v1/governance/signals/aggregate",
        "/api/v1/governance/state",
        "/api/v1/governance/states",
        "/api/v1/households/{id}/devices",
        "/api/v1/households/{id}/stewardship-allocations",
        "/api/v1/identity/me",
        "/api/v1/identity/{id}/profile",
        "/api/v1/lamad/content/{contentId}/engagement",
        "/api/v1/mastery",
        "/api/v1/mastery/path/{path_id}",
        "/api/v1/mastery/pool/{id}",
        "/api/v1/mastery/stats",
        "/api/v1/mastery/{id}",
        "/api/v1/network/posture",
        "/api/v1/observations/stream",
        "/api/v1/observations/{id}/report",
        "/api/v1/peer-statuses",
        "/api/v1/peer-topology",
        "/api/v1/peers/delivery",
        "/api/v1/pins",
        "/api/v1/pins/{epr_id}/pull",
        "/api/v1/placement-gaps",
        "/api/v1/places",
        "/api/v1/places/{id}",
        "/api/v1/presence",
        "/api/v1/presence/{id}",
        "/api/v1/reciprocity",
        "/api/v1/resilience/{content_id}",
        "/api/v1/resilience/{content_id}/household",
        "/api/v1/resources",
        "/api/v1/resources/constitutional-limits/{id}",
        "/api/v1/resources/dashboard/{id}",
        "/api/v1/resources/{id}",
        "/api/v1/schedules",
        "/api/v1/schedules/{id}",
        "/api/v1/spatial-contexts",
        "/api/v1/spatial-contexts/{id}",
        "/api/v1/status/arc-policy",
        "/api/v1/status/projector",
        "/api/v1/status/write-through",
        "/api/v1/steward-affinity",
        "/api/v1/steward-affinity/{id}",
        "/api/v1/steward/access",
        "/api/v1/steward/credentials",
        "/api/v1/steward/credentials/{id}",
        "/api/v1/steward/gates",
        "/api/v1/steward/gates/{id}",
        "/api/v1/steward/grants",
        "/api/v1/steward/grants/{id}",
        "/api/v1/steward/revenue/{id}",
        "/api/v1/stewardship/access/time",
        "/api/v1/stewardship/allocations",
        "/api/v1/stewardship/allocations/content/{content_id}",
        "/api/v1/stewardship/allocations/steward/{steward_id}",
        "/api/v1/stewardship/allocations/{id}",
        "/api/v1/stewardship/grants",
        "/api/v1/stewardship/grants/{id}",
        "/api/v1/stewardship/policies",
        "/api/v1/stewardship/policies/me/chain",
        "/api/v1/stewardship/policies/{id}",
        "/api/v1/stewardship/policies/{id}/chain",
        "/api/v1/stewardship/policies/{id}/parent",
        "/api/v1/stewardship/policy",
        "/api/v1/weave",
        "/auth/me",
        "/db/allocations",
        "/db/allocations/content/{content_id}",
        "/db/allocations/steward/{steward_id}",
        "/db/allocations/{id}",
        "/db/challenge-outcomes",
        "/db/challenge-outcomes/{cid}",
        "/db/collectives",
        "/db/collectives/{id}",
        "/db/collectives/{id}/participants",
        "/db/events",
        "/db/events/{id}",
        "/db/gate-decision-challenges",
        "/db/gate-decision-challenges/{cid}",
        "/db/gate-decisions",
        "/db/gate-decisions/{cid}",
        "/db/human-relationships",
        "/db/human-relationships/{id}",
        "/db/humans",
        "/db/identity/did/{did}",
        "/db/knowledge-maps",
        "/db/knowledge-maps/{id}",
        "/db/mastery",
        "/db/mastery/human/{human_id}",
        "/db/mastery/{id}",
        "/db/nodes",
        "/db/nodes/{id}",
        "/db/p2p/conductor-diagnostics",
        "/db/participations/{human_id}",
        "/db/presences",
        "/db/presences/{id}",
        "/db/rea_commitments",
        "/db/schema",
        "/db/stats",
        "/p2p/status",
        "/p2p/sync-mode",
        "/p2p/sync-mode/history",
    ];
    EXACT.contains(&path)
}

#[test]
fn every_reach_bearing_manifest_get_maps_to_a_plane() {
    let mut missing = Vec::new();
    for route in elohim_storage::http::build_manifest().routes {
        if route.method != doorway_client::HttpMethod::Get {
            continue;
        }
        if reach_plane_for_get(&route.path).is_none() && !not_egress_manifest_get(&route.path) {
            missing.push(route.path);
        }
    }
    assert!(
        missing.is_empty(),
        "manifest GET routes need an EgressPlane or named NotEgress exemption: {missing:?}"
    );
}

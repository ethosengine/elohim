//! G — who reads the manifest's needs block (G2, G3).
//!
//! placement-core cannot name `eprfs_core::TreeView` (R-P1), so the caller computes the closure's
//! key (`NeedKey::TreeView(*tree_view.cid()?.as_cid())`, §1.4).

use std::collections::{BTreeMap, BTreeSet};

use cid::Cid;
use elohim_epr::Reach;
use elohim_epr_rea::AgentRef;
use serde::Deserialize;

use crate::decide::LossDomain;
use crate::kind::GrantScope;
use crate::need::{Depth, NeedKey, PrivateNeed};
use crate::params::PlaceError;

/// Hand-written, not generated: schema codegen emits TypeScript only, and the Rust mirror is pinned
/// to `$defs.ManifestNeeds` by `tests/schema_contract.rs`. It holds only the fields placement reads,
/// and each one lands with its reader, as the schema's `properties` entry does. There is no
/// `deny_unknown_fields`: `channels`, `signing`, `passing` and the rest belong to other readers, and
/// the schema, not this struct, refuses fields nothing declares.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestNeeds {
    /// ViewPaths, slash-joined; `""` is the root.
    pub closure: Option<Vec<String>>,
    /// Absent with a declared closure means `Kept`.
    pub works_offline: Option<WorksOffline>,
    /// Content type → loss domain.
    pub loss: Option<BTreeMap<String, LossDomain>>,
    pub compute: Option<Vec<ComputeNeed>>,
}

/// `query`, which holds no bytes, cannot mean "works offline", so the schema refuses it (G7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorksOffline {
    Sparse,
    OnRead,
    Kept,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputeNeed {
    pub scope: GrantScope,
    pub bounds: ComputeBounds,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputeBounds {
    pub rate_per_hour: Option<u32>,
}

pub struct ManifestContext {
    /// The manifest's CID; every real manifest id is a slug, so the reader takes the CID, never the
    /// id field. Carried on every grant request as its source.
    pub manifest: Cid,
    /// The release root the release record binds (finding 8). A declared closure is accepted only
    /// with a [`ClosureKey`] computed over this root.
    pub release_root: Cid,
    /// The closure's key, as ark-core computed it, with the root and paths it was computed over.
    pub closure: Option<ClosureKey>,
    /// Who the app's needs are for: the requester of every grant request.
    pub receiver: AgentRef,
    /// The space the app runs in: every grant request's `in_scope_of`.
    pub scope: Cid,
    /// The reach the grant requests are expressed at; they cross no wider.
    pub reach: Reach,
}

/// `NeedKey::TreeView(*tree_view.cid()?.as_cid())`, computed by ark-core, which links both crates
/// and maps `Depth` 1:1, together with what it was computed over. placement-core cannot recompute
/// the tree view (R-P1), so it checks the provenance the caller states against the manifest and
/// the release: `root` must be `ManifestContext::release_root` and `paths` the manifest's declared
/// closure, or the key is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosureKey {
    pub key: NeedKey,
    pub root: Cid,
    pub paths: Vec<String>,
    /// The closure's size in bytes, read from the same tree view. `None` leaves the private need
    /// unsized, and `decide_private` refuses it by name rather than keeping it as if it cost nothing.
    pub bytes: Option<u64>,
}

pub struct ManifestNeedsOut {
    /// closure + worksOffline → a `PrivateNeed` at the declared depth.
    pub private: Vec<PrivateNeed>,
    /// Content type → loss; the custody need source applies `charter.resolve` to each content root
    /// it holds.
    pub loss_rules: BTreeMap<String, LossDomain>,
    /// compute → standing delegates-compute requests; a Task need comes only from a `TaskSpec`.
    pub grant_requests: Vec<GrantRequest>,
}

/// A standing delegates-compute request: what the app asks, for whom, where, at which reach, and
/// from which manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantRequest {
    pub scope: GrantScope,
    pub rate_per_hour: Option<u32>,
    pub requester: AgentRef,
    pub in_scope_of: Cid,
    pub reach: Reach,
    pub manifest: Cid,
}

fn path_set(paths: &[String]) -> BTreeSet<&str> {
    paths.iter().map(String::as_str).collect()
}

pub fn needs_from_manifest(
    n: &ManifestNeeds,
    ctx: &ManifestContext,
) -> Result<ManifestNeedsOut, PlaceError> {
    let mut private = Vec::new();
    if let Some(paths) = &n.closure {
        if paths.is_empty() {
            return Err(PlaceError("needs.closure names no path".into()));
        }
        let closure = ctx.closure.as_ref().ok_or_else(|| {
            PlaceError("needs.closure is declared but the caller computed no closure key".into())
        })?;
        if closure.root != ctx.release_root {
            return Err(PlaceError(format!(
                "the closure key was computed over {}, not the release root {}",
                closure.root, ctx.release_root
            )));
        }
        if path_set(&closure.paths) != path_set(paths) {
            return Err(PlaceError(
                "the closure key was computed over other paths than needs.closure declares".into(),
            ));
        }
        let depth = match n.works_offline {
            Some(WorksOffline::Sparse) => Depth::Sparse,
            Some(WorksOffline::OnRead) => Depth::OnRead,
            Some(WorksOffline::Kept) | None => Depth::Kept,
        };
        private.push(PrivateNeed {
            key: closure.key.clone(),
            depth,
            budget_bytes: closure.bytes,
        });
    }
    let grant_requests = n
        .compute
        .iter()
        .flatten()
        .map(|c| GrantRequest {
            scope: c.scope,
            rate_per_hour: c.bounds.rate_per_hour,
            requester: ctx.receiver.clone(),
            in_scope_of: ctx.scope,
            reach: ctx.reach,
            manifest: ctx.manifest,
        })
        .collect();
    Ok(ManifestNeedsOut {
        private,
        loss_rules: n.loss.clone().unwrap_or_default(),
        grant_requests,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cid(s: &str) -> Cid {
        elohim_epr::cid::compute_cid(s.as_bytes())
    }

    fn ctx(closure: Option<ClosureKey>) -> ManifestContext {
        ManifestContext {
            manifest: cid("manifest"),
            release_root: cid("release"),
            closure,
            receiver: AgentRef("uhCAk-r".into()),
            scope: cid("space"),
            reach: Reach::Intimate,
        }
    }

    fn closure(paths: &[&str]) -> ClosureKey {
        ClosureKey {
            key: NeedKey::TreeView(cid("view")),
            root: cid("release"),
            paths: paths.iter().map(|p| p.to_string()).collect(),
            bytes: Some(4_096),
        }
    }

    #[test]
    fn the_lamad_worked_example() {
        // `needs: { closure: [""], worksOffline: "kept" }` and `loss: { "concept": "household" }`.
        let n: ManifestNeeds = serde_json::from_str(
            r#"{"closure":[""],"worksOffline":"kept","loss":{"concept":"household"},"channels":["stable"]}"#,
        )
        .unwrap();
        let out = needs_from_manifest(&n, &ctx(Some(closure(&[""])))).unwrap();
        assert_eq!(
            out.private,
            vec![PrivateNeed {
                key: NeedKey::TreeView(cid("view")),
                depth: Depth::Kept,
                budget_bytes: Some(4_096)
            }]
        );
        assert_eq!(out.loss_rules.get("concept"), Some(&LossDomain::Household));
        assert!(out.grant_requests.is_empty());
    }

    #[test]
    fn absent_works_offline_with_a_closure_means_kept_and_query_is_refused() {
        let n: ManifestNeeds = serde_json::from_str(r#"{"closure":["a"]}"#).unwrap();
        assert_eq!(
            needs_from_manifest(&n, &ctx(Some(closure(&["a"]))))
                .unwrap()
                .private[0]
                .depth,
            Depth::Kept
        );
        assert!(serde_json::from_str::<ManifestNeeds>(r#"{"worksOffline":"query"}"#).is_err());
        assert!(
            needs_from_manifest(&n, &ctx(None)).is_err(),
            "a closure needs its key"
        );
    }

    #[test]
    fn a_closure_key_is_bound_to_the_release_root_and_the_declared_paths() {
        let n: ManifestNeeds = serde_json::from_str(r#"{"closure":["a","b"]}"#).unwrap();
        // Either order of the same paths is the same closure.
        assert!(needs_from_manifest(&n, &ctx(Some(closure(&["b", "a"])))).is_ok());
        assert!(
            needs_from_manifest(&n, &ctx(Some(closure(&["a"])))).is_err(),
            "a key over fewer paths than declared is refused"
        );
        let mut elsewhere = closure(&["a", "b"]);
        elsewhere.root = cid("some-other-release");
        assert!(
            needs_from_manifest(&n, &ctx(Some(elsewhere))).is_err(),
            "a key over another root than the release's is refused"
        );
        // The release root is read: moving it refuses the same key.
        let mut moved = ctx(Some(closure(&["a", "b"])));
        moved.release_root = cid("next-release");
        assert!(needs_from_manifest(&n, &moved).is_err());
    }

    #[test]
    fn compute_becomes_standing_grant_requests_naming_who_where_and_whence() {
        let n: ManifestNeeds = serde_json::from_str(
            r#"{"compute":[{"scope":"measure-stage","bounds":{"ratePerHour":4}}]}"#,
        )
        .unwrap();
        let out = needs_from_manifest(&n, &ctx(None)).unwrap();
        assert_eq!(
            out.grant_requests,
            vec![GrantRequest {
                scope: GrantScope::MeasureStage,
                rate_per_hour: Some(4),
                requester: AgentRef("uhCAk-r".into()),
                in_scope_of: cid("space"),
                reach: Reach::Intimate,
                manifest: cid("manifest"),
            }]
        );
        // Every context field reaches the output.
        let mut other = ctx(None);
        other.receiver = AgentRef("uhCAk-other".into());
        other.scope = cid("other-space");
        other.reach = Reach::Public;
        other.manifest = cid("other-manifest");
        let moved = &needs_from_manifest(&n, &other).unwrap().grant_requests[0];
        assert_eq!(moved.requester, other.receiver);
        assert_eq!(moved.in_scope_of, other.scope);
        assert_eq!(moved.reach, Reach::Public);
        assert_eq!(moved.manifest, other.manifest);
    }
}

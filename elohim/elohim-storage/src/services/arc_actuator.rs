//! Conductor authority-arc actuation — the storage-side executor core.
//!
//! Implements the actuatable half of §5 of
//! `genesis/docs/superpowers/specs/2026-06-13-conductor-authority-arc-auto-policy.md`
//! for the lever that actually exists on the deployed substrate.
//!
//! ## Why `{0,1}` only
//! The §2 spike VERIFIED that holochain_p2p 0.6.0 / kitsune2 0.3.2 expose **no
//! fractional arc lever** — `target_arc_factor` is a `{0,1}` participation
//! switch (1 = full authority anchor, 0 = leecher), and `> 1` is hard-clamped
//! with *"not yet allowed until sharding is implemented"*. So actuation here is
//! binary: a node is made a full anchor or an accountable leecher. The
//! fractional value from [`super::arc_policy::derive`] is a SIGNAL (surfaced by
//! the §6 gauge), not something we can set.
//!
//! ## A commitment, not an admin edit (§5)
//! An arc change is the fulfillment of a bounded, revocable authority grant —
//! never a silent config edit (a resilience↔memory trade must be accountable).
//! This module is the EXECUTOR: given a proposed factor and the bounds of an
//! authorizing grant, it (a) checks the proposal is within the grant, (b) checks
//! the **coverage invariant operationally** (a leecher must leave the mesh
//! covered) — REFUSING and ELEVATING rather than opening a keyspace gap, and
//! (c) renders the new conductor-config. The actual config-write + staggered
//! conductor restart, and the DNA-side grant (the integrity-validated
//! `sets-authority-arc` action — a DNA-hash-move, governance-gated) are the
//! integration/ceremony layers above this core.
//!
//! ## Purity
//! The decision functions are pure (clock passed in as `now_epoch_s`, coverage
//! passed in as a snapshot) and fully unit-tested. Only the thin `apply` shell
//! touches the filesystem / conductor.

/// The `{0,1}` factor a node may run at. Fractional is not actuatable (§2).
pub const FACTOR_LEECHER: u32 = 0;
pub const FACTOR_FULL: u32 = 1;

/// Bounds of the authorizing commitment grant — what factors it permits and
/// until when. Parsed by the orchestration layer from the grant (the deployed
/// no-DNA-hash-move path reads these from a `delegates-compute` commitment's
/// `bounds.additionalProperties`; the clean future is a `sets-authority-arc`
/// action — spec §5 fork). The executor only trusts these bounds, never a raw
/// config knob.
#[derive(Debug, Clone, Copy)]
pub struct ArcGrantBounds {
    pub min_factor: u32,
    pub max_factor: u32,
    /// Grant expiry (unix seconds); `None` = no expiry encoded.
    pub expires_at_epoch_s: Option<u64>,
}

/// A proposed actuation: set this node to `target_factor`, authorized by the
/// commitment at `commitment_cid` (CID = entry_hash, gospel).
#[derive(Debug, Clone)]
pub struct ArcActuationRequest {
    pub target_factor: u32,
    pub commitment_cid: String,
}

/// What the node can see of mesh coverage at decision time (the conductor DHT
/// plane, not the storage swarm). `observed_n` is the conductor peer count;
/// `r_floor` the hard per-key redundancy floor (spec §4).
#[derive(Debug, Clone, Copy)]
pub struct CoverageSnapshot {
    pub observed_n: u32,
    pub r_floor: u32,
}

/// A refusal carrying a finding to ELEVATE. This is the runtime form of "do not
/// shrink into a keyspace gap / this device is mismatched" — and it is the exact
/// payload the self-healing elevate/finding sink consumes (kept deliberately
/// small: a machine `code` + a human `elevate` message).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActuationRefusal {
    pub code: RefusalCode,
    pub elevate: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefusalCode {
    /// Proposed factor is outside the grant's `[min,max]`.
    OutOfGrantBounds,
    /// The grant has expired.
    GrantExpired,
    /// Proposed factor is not an actuatable value (only `{0,1}` exist, §2).
    NotActuatable,
    /// Going leecher would drop mesh coverage below `r_floor` — refuse, elevate.
    WouldBreakCoverage,
}

/// Errors from the impure render/apply layer (kept separate from refusals,
/// which are *policy* outcomes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActuationError {
    /// The conductor-config has no top-level `network:` block to set the factor in.
    NoNetworkBlock,
}

/// A validated, coverage-admitted plan — the input to the impure apply step.
#[derive(Debug, Clone)]
pub struct ArcActuationPlan {
    pub target_factor: u32,
    pub commitment_cid: String,
}

/// Check the proposal is within the grant and actuatable. Pure.
pub fn authorize(
    req: &ArcActuationRequest,
    bounds: &ArcGrantBounds,
    now_epoch_s: u64,
) -> Result<(), ActuationRefusal> {
    if req.target_factor != FACTOR_LEECHER && req.target_factor != FACTOR_FULL {
        return Err(ActuationRefusal {
            code: RefusalCode::NotActuatable,
            elevate: format!(
                "target_arc_factor={} is not actuatable: the deployed lever is {{0,1}} only \
                 (no fractional arc until kitsune2 sharding lands — spec §2)",
                req.target_factor
            ),
        });
    }
    if req.target_factor < bounds.min_factor || req.target_factor > bounds.max_factor {
        return Err(ActuationRefusal {
            code: RefusalCode::OutOfGrantBounds,
            elevate: format!(
                "target_arc_factor={} outside grant bounds [{}, {}]",
                req.target_factor, bounds.min_factor, bounds.max_factor
            ),
        });
    }
    if let Some(exp) = bounds.expires_at_epoch_s {
        if now_epoch_s >= exp {
            return Err(ActuationRefusal {
                code: RefusalCode::GrantExpired,
                elevate: format!("arc-set grant expired at {exp} (now {now_epoch_s})"),
            });
        }
    }
    Ok(())
}

/// The operational coverage gate. On the `{0,1}` lever every non-leecher node
/// covers the WHOLE keyspace, so making this node a leecher is admissible only
/// if the *remaining* mesh still meets the floor. Conservative (treats the other
/// `N-1` as full — the safe assumption when per-peer arcs aren't yet advertised,
/// spec §4 "mesh_coverage unknown@P0"): require `observed_n - 1 >= r_floor`.
/// Becoming/staying full always admits. Refuse-and-elevate otherwise — the cure
/// must never cause the partition. Pure.
pub fn coverage_admits(
    target_factor: u32,
    snap: &CoverageSnapshot,
) -> Result<(), ActuationRefusal> {
    if target_factor != FACTOR_LEECHER {
        return Ok(()); // full anchor only adds coverage
    }
    let remaining = snap.observed_n.saturating_sub(1);
    if remaining >= snap.r_floor {
        Ok(())
    } else {
        Err(ActuationRefusal {
            code: RefusalCode::WouldBreakCoverage,
            elevate: format!(
                "refusing leecher: only {} other node(s) would remain, below r_floor={} — \
                 add peers or keep this node a full anchor (do NOT open a keyspace gap)",
                remaining, snap.r_floor
            ),
        })
    }
}

/// Compose authorization + the coverage gate into a plan, or a refusal-to-elevate.
/// This is the whole policy decision; the caller executes the returned plan
/// (render config + staggered restart). Pure.
pub fn plan_actuation(
    req: &ArcActuationRequest,
    bounds: &ArcGrantBounds,
    snap: &CoverageSnapshot,
    now_epoch_s: u64,
) -> Result<ArcActuationPlan, ActuationRefusal> {
    authorize(req, bounds, now_epoch_s)?;
    coverage_admits(req.target_factor, snap)?;
    Ok(ArcActuationPlan {
        target_factor: req.target_factor,
        commitment_cid: req.commitment_cid.clone(),
    })
}

/// Render a conductor-config YAML with `network.target_arc_factor` set to
/// `factor`. Minimal line-based edit (preserves the rest of the file): inserts
/// the key as the first child of the top-level `network:` block, replacing any
/// existing `target_arc_factor:` child. Returns `NoNetworkBlock` if there is no
/// `network:` block to set it under. Pure.
pub fn render_conductor_arc_factor(
    config_yaml: &str,
    factor: u32,
) -> Result<String, ActuationError> {
    let mut out = String::with_capacity(config_yaml.len() + 32);
    let mut in_network = false;
    let mut saw_network = false;

    for line in config_yaml.lines() {
        let is_top_level_key = !line.is_empty() && !line.starts_with([' ', '\t', '#']);

        if is_top_level_key && line.trim_end() == "network:" {
            // Enter the block and write our key as its first child.
            saw_network = true;
            in_network = true;
            out.push_str(line);
            out.push('\n');
            out.push_str(&format!("  target_arc_factor: {factor}\n"));
            continue;
        }

        if in_network {
            if is_top_level_key {
                // A new top-level key ends the network block.
                in_network = false;
            } else if line.trim_start().starts_with("target_arc_factor:") {
                // Drop any pre-existing child — we already wrote the new value.
                continue;
            }
        }

        out.push_str(line);
        out.push('\n');
    }

    if !saw_network {
        return Err(ActuationError::NoNetworkBlock);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grant(min: u32, max: u32, exp: Option<u64>) -> ArcGrantBounds {
        ArcGrantBounds {
            min_factor: min,
            max_factor: max,
            expires_at_epoch_s: exp,
        }
    }
    fn req(factor: u32) -> ArcActuationRequest {
        ArcActuationRequest {
            target_factor: factor,
            commitment_cid: "uhCkk-test".to_string(),
        }
    }

    #[test]
    fn authorize_accepts_in_bounds_unexpired() {
        assert!(authorize(&req(0), &grant(0, 1, Some(2_000)), 1_000).is_ok());
        assert!(authorize(&req(1), &grant(0, 1, None), 1_000).is_ok());
    }

    #[test]
    fn authorize_rejects_out_of_bounds() {
        // grant permits leecher only; proposing full is out of bounds.
        let r = authorize(&req(1), &grant(0, 0, None), 1_000).unwrap_err();
        assert_eq!(r.code, RefusalCode::OutOfGrantBounds);
    }

    #[test]
    fn authorize_rejects_expired() {
        let r = authorize(&req(0), &grant(0, 1, Some(1_000)), 1_000).unwrap_err();
        assert_eq!(r.code, RefusalCode::GrantExpired);
    }

    #[test]
    fn authorize_rejects_non_binary_factor() {
        let r = authorize(&req(2), &grant(0, 5, None), 1_000).unwrap_err();
        assert_eq!(r.code, RefusalCode::NotActuatable);
    }

    #[test]
    fn coverage_full_always_admits() {
        // Even a tiny mesh: becoming/staying full only adds coverage.
        assert!(coverage_admits(
            FACTOR_FULL,
            &CoverageSnapshot {
                observed_n: 1,
                r_floor: 3
            }
        )
        .is_ok());
    }

    #[test]
    fn coverage_leecher_admits_when_mesh_covers() {
        // N=14, r_floor=3: 13 others remain ≥ 3 → leecher OK.
        assert!(coverage_admits(
            FACTOR_LEECHER,
            &CoverageSnapshot {
                observed_n: 14,
                r_floor: 3
            }
        )
        .is_ok());
    }

    #[test]
    fn coverage_leecher_refused_and_elevates_when_mesh_too_small() {
        // N=3, r_floor=3: only 2 others remain < 3 → refuse + elevate.
        let r = coverage_admits(
            FACTOR_LEECHER,
            &CoverageSnapshot {
                observed_n: 3,
                r_floor: 3,
            },
        )
        .unwrap_err();
        assert_eq!(r.code, RefusalCode::WouldBreakCoverage);
        assert!(r.elevate.contains("r_floor=3"));
    }

    #[test]
    fn plan_actuation_composes_both_gates() {
        // Leecher within grant but coverage too thin → refused.
        let r = plan_actuation(
            &req(0),
            &grant(0, 1, None),
            &CoverageSnapshot {
                observed_n: 2,
                r_floor: 3,
            },
            1_000,
        )
        .unwrap_err();
        assert_eq!(r.code, RefusalCode::WouldBreakCoverage);

        // Leecher within grant and coverage OK → plan.
        let p = plan_actuation(
            &req(0),
            &grant(0, 1, None),
            &CoverageSnapshot {
                observed_n: 14,
                r_floor: 3,
            },
            1_000,
        )
        .unwrap();
        assert_eq!(p.target_factor, 0);
    }

    const SAMPLE_CONFIG: &str = "\
---
data_root_path: \"/var/local/lib/holochain\"
network:
  bootstrap_url: \"https://doorway.elohim.host/bootstrap\"
  signal_url: \"wss://signal.doorway.elohim.host\"
  enable_mdns: false
  webrtc_config:
    ice_servers:
      - urls: [\"stun:stun.l.google.com:19302\"]
admin_interfaces:
  - driver:
      type: websocket
";

    #[test]
    fn render_inserts_target_arc_factor_into_network_block() {
        let out = render_conductor_arc_factor(SAMPLE_CONFIG, 0).unwrap();
        // Inserted as the first child of network:, 2-space indented.
        assert!(out.contains("network:\n  target_arc_factor: 0\n"));
        // Preserved siblings + later top-level keys.
        assert!(out.contains("bootstrap_url:"));
        assert!(out.contains("admin_interfaces:"));
        // Did not leak into the webrtc ice_servers nesting.
        assert!(out.contains("ice_servers:"));
    }

    #[test]
    fn render_replaces_existing_target_arc_factor() {
        let with_existing = "network:\n  target_arc_factor: 1\n  enable_mdns: false\n";
        let out = render_conductor_arc_factor(with_existing, 0).unwrap();
        assert!(out.contains("target_arc_factor: 0"));
        assert!(!out.contains("target_arc_factor: 1"));
        // Exactly one occurrence.
        assert_eq!(out.matches("target_arc_factor:").count(), 1);
        assert!(out.contains("enable_mdns: false"));
    }

    #[test]
    fn render_errors_without_network_block() {
        let no_net = "data_root_path: \"/x\"\nadmin_interfaces: []\n";
        assert_eq!(
            render_conductor_arc_factor(no_net, 1).unwrap_err(),
            ActuationError::NoNetworkBlock
        );
    }
}

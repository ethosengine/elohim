//! Conductor authority-arc Auto policy — the pure `derive()` function.
//!
//! Implements §3 of
//! `genesis/docs/superpowers/specs/2026-06-13-conductor-authority-arc-auto-policy.md`:
//!
//! ```text
//!   target_arc_factor = derive(mem_ceiling, archetype, observed_N, corpus, local_share)
//! ```
//!
//! Memory is a **ceiling** (pressure → shrink); coverage is a **floor**
//! (`a_cov = min(1, R/N)` — shrink past it and the keyspace gaps). So the shape
//! is a CLAMP, not a min-of-ceilings (contrast `min_cpu_budget`, the cpu-budget
//! precedent where every input pushes the same direction).
//!
//! ## Purity
//! `derive` reads NOTHING ambiently. Every live value — the cgroup memory
//! ceiling (M1: [`super::system_metrics::container_memory_limit_bytes`]), the
//! conductor peer count (`hc_client` `network.peer_count`), the corpus
//! working-set, the node's own authored share — is passed in as a point-in-time
//! sample taken at the call site (boot / re-derive event). This keeps it
//! deterministic and fully unit-testable without a cluster.
//!
//! ## Safety stance
//! When any input needed for a shrink is missing or degenerate, `derive` returns
//! the SAFE STATIC DEFAULT = **full arc (1.0)** — the current behavior — never a
//! guessed shrink. A memory-derived shrink is unsafe until M1 (the cgroup
//! reader) AND a fractional-arc lever (the §2 spike) both land; until then the
//! computed value is observable via the gauge but actuation can express only
//! `{0,1}` (spec §2 tiers).
//!
//! ## What `derive` does NOT decide
//! The actuation tier (T1 `{0,1}` switch / T2 fractional hint / T3 custom) and
//! whether a fractional value is actuatable at all are §5/§2 concerns, not this
//! function's. `derive` always computes the fractional aim; the actuation layer
//! maps it to the available lever.

/// Per-key redundancy parameters. Recommended POLICY values adopted from the
/// data/durability plane (spec §4), not a protocol mandate: `r_floor` mirrors
/// `default_min_replicas = 2` / `replica_health` (AtRisk at 1..=2, Healthy ≥3);
/// `r_target` mirrors RS(7,4)'s `replica_target = 7`, carrying a churn margin.
#[derive(Debug, Clone, Copy)]
pub struct CoverageParams {
    /// Hard safe-floor — coverage must never drop below this many distinct
    /// holders per key.
    pub r_floor: u32,
    /// Desired per-key redundancy (`r_floor` + churn margin δ). The arc aim is
    /// computed against this; transient peer loss still leaves `≥ r_floor`.
    pub r_target: u32,
}

impl Default for CoverageParams {
    fn default() -> Self {
        Self {
            r_floor: 3,
            r_target: 7,
        }
    }
}

/// Sampled inputs to [`derive`]. All live state is captured by the caller and
/// passed in — `derive` never reads it ambiently.
#[derive(Debug, Clone, Copy)]
pub struct ArcInputs {
    /// Container cgroup memory ceiling in bytes (M1 —
    /// [`super::system_metrics::container_memory_limit_bytes`]). `None` ⇒ unknown
    /// ⇒ safe default (full arc). NB the conductor shares one cgroup with the
    /// storage process, so this is the JOINT ceiling.
    pub mem_ceiling_bytes: Option<u64>,
    /// Bytes to reserve for the storage parent inside the shared cgroup —
    /// subtracted from the ceiling before sizing the conductor arc.
    pub storage_headroom_bytes: u64,
    /// Conductor / kitsune2 DHT peer count (`hc_client` `network.peer_count`).
    /// NOT the storage libp2p swarm count. `None` ⇒ safe default.
    pub observed_n: Option<u32>,
    /// Corpus DHT working-set size in bytes (sampled). `None` ⇒ safe default.
    pub corpus_bytes: Option<u64>,
    /// This node's own always-resident authored share in bytes (the `L` term):
    /// stays resident regardless of arc, so it bounds the win for heavy authors.
    pub local_authored_bytes: u64,
    /// The archetype's base aim in `[0,1]` (1.0 = a full-capable node aims full).
    /// Per-archetype tuning is P1; sane default 1.0.
    pub archetype_base_arc: f64,
    /// Per-key redundancy parameters (spec §4).
    pub coverage: CoverageParams,
}

/// The result of [`derive`] — the arc fraction plus the full derivation trace
/// (the gauge's "why", spec §6) and any finding to elevate.
#[derive(Debug, Clone, PartialEq)]
pub struct ArcDecision {
    /// The derived authority-arc fraction in `[0,1]`.
    pub arc_factor: f64,
    /// The coverage floor `a_cov = min(1, R/N)` that bounded the shrink.
    pub coverage_floor: f64,
    /// True in the "james" case: the memory budget cannot meet required
    /// coverage, so coverage won and a finding was elevated (over-replication
    /// bias — never shrink into a keyspace gap).
    pub coverage_over_memory: bool,
    /// Whether this is the safe static default (a shrink input was missing).
    pub safe_default: bool,
    /// Human-readable derivation trace — the gauge surfaces this so an operator
    /// / agent / UI sees the *reasoning*, not just the value (spec §6).
    pub reasons: Vec<String>,
    /// A finding to elevate when the device is fundamentally mismatched (cannot
    /// meet coverage within its memory budget at this N) — the runtime form of
    /// "add peers / this device is under-provisioned", NOT "shrink further".
    pub elevate: Option<String>,
}

/// The minimum *contributing* arc a node may derive. Spec §4: a non-zero
/// contributing floor (a "minimum shard", never a silent leecher) is only
/// EXPRESSIBLE once a fractional lever lands (the §2 spike); under the verified
/// `{0,1}` lever the only sub-full value is 0. So this stays 0.0 for now — and
/// it does not matter in practice because `a_cov = R/N > 0` for any finite N is
/// always the binding floor. `derive` therefore never returns 0; a true leecher
/// (arc = 0) is reachable only by an accountable operator override (spec §3c/§5),
/// never by Auto.
const SAFE_MIN_ARC: f64 = 0.0;

#[inline]
fn clip01(x: f64) -> f64 {
    x.clamp(0.0, 1.0)
}

/// How the archetype's base aim is divided as the mesh grows. ~1.0 until the
/// mesh is well past the redundancy target, then a gentle taper so a
/// well-provisioned node at very large N reclaims memory even without pressure.
/// The operative shrink driver remains the memory ceiling (the `[a_floor,
/// a_mem]` clamp); this only lowers the *aim*, never below `a_cov`. Placeholder
/// tuning (spec §3) — the knee constant is adjustable; the invariants are not.
fn arc_share(n: u32, r_target: u32) -> f64 {
    let knee = (r_target.max(1) as f64) * 4.0;
    (n as f64 / knee).max(1.0)
}

/// Derive the authority-arc fraction from sampled inputs. Pure, deterministic,
/// re-runnable. See module docs for the safety stance and the clamp shape.
pub fn derive(inp: ArcInputs) -> ArcDecision {
    let mut reasons = Vec::new();

    // ── safe static default ─────────────────────────────────────────────────
    // Any missing/degenerate shrink-input ⇒ full arc. Never guess a shrink.
    let (m, n, c) = match (inp.mem_ceiling_bytes, inp.observed_n, inp.corpus_bytes) {
        (Some(m), Some(n), Some(c)) if n > 0 && c > inp.local_authored_bytes && m > 0 => (m, n, c),
        _ => {
            reasons.push(
                "safe static default = full arc (1.0): a shrink input is unknown or degenerate \
                 (mem_ceiling / observed_N / corpus>local) — never guess a shrink"
                    .to_string(),
            );
            return ArcDecision {
                arc_factor: 1.0,
                coverage_floor: 0.0,
                coverage_over_memory: false,
                safe_default: true,
                reasons,
                elevate: None,
            };
        }
    };

    // ── coverage floor: a_cov = min(1, R/N) ─────────────────────────────────
    let r = inp.coverage.r_target.max(inp.coverage.r_floor) as f64;
    let a_cov = clip01(r / n as f64);
    reasons.push(format!(
        "coverage floor a_cov = min(1, R/N) = min(1, {}/{}) = {:.3}",
        r as u32, n, a_cov
    ));

    // ── memory-permitted ceiling: clip01((M - L) / (C - L)) ─────────────────
    let l = inp.local_authored_bytes as f64;
    let m_eff = m.saturating_sub(inp.storage_headroom_bytes) as f64;
    let a_mem = if m_eff <= l {
        0.0 // local authored share alone exceeds the budget — no foreign arc fits
    } else {
        clip01((m_eff - l) / (c as f64 - l))
    };
    reasons.push(format!(
        "memory-permitted a_mem = clip01((M_eff-L)/(C-L)) = {:.3}  (M_eff={:.0}B, L={:.0}B, C={:.0}B)",
        a_mem, m_eff, l, c as f64
    ));

    let a_floor = a_cov.max(SAFE_MIN_ARC);
    let target = clip01(inp.archetype_base_arc / arc_share(n, inp.coverage.r_target));

    if a_mem < a_floor {
        // The "james" case: coverage cannot be met within the memory budget.
        // COVERAGE WINS — bias to over-replication; never shrink into a gap.
        reasons.push(format!(
            "a_mem ({:.3}) < a_floor ({:.3}) — COVERAGE WINS (over-replication bias); elevate finding",
            a_mem, a_floor
        ));
        let elevate = format!(
            "cannot meet coverage (a_cov={:.3}) within the memory budget (a_mem={:.3}) at N={}: \
             needs more peers or a RAM allowance, not a smaller arc",
            a_cov, a_mem, n
        );
        ArcDecision {
            arc_factor: a_floor,
            coverage_floor: a_cov,
            coverage_over_memory: true,
            safe_default: false,
            reasons,
            elevate: Some(elevate),
        }
    } else {
        // Memory permits at least the floor: aim at `target`, clamped from below
        // by coverage and from above by memory. a_floor ≤ a_mem here, so the
        // clamp is well-defined (the degenerate low>high region is the branch
        // above).
        let a = target.clamp(a_floor, a_mem);
        reasons.push(format!(
            "a = clamp(target={:.3}, low=a_floor={:.3}, high=a_mem={:.3}) = {:.3}",
            target, a_floor, a_mem, a
        ));
        ArcDecision {
            arc_factor: a,
            coverage_floor: a_cov,
            coverage_over_memory: false,
            safe_default: false,
            reasons,
            elevate: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GI: u64 = 1024 * 1024 * 1024;

    fn base_inputs() -> ArcInputs {
        ArcInputs {
            mem_ceiling_bytes: Some(8 * GI),
            storage_headroom_bytes: 0,
            observed_n: Some(14),
            corpus_bytes: Some(GI),
            local_authored_bytes: 0,
            archetype_base_arc: 1.0,
            coverage: CoverageParams::default(),
        }
    }

    #[test]
    fn missing_mem_ceiling_yields_safe_full_arc() {
        let d = derive(ArcInputs {
            mem_ceiling_bytes: None,
            ..base_inputs()
        });
        assert_eq!(d.arc_factor, 1.0);
        assert!(d.safe_default);
        assert!(d.elevate.is_none());
    }

    #[test]
    fn missing_peer_count_yields_safe_full_arc() {
        let d = derive(ArcInputs {
            observed_n: None,
            ..base_inputs()
        });
        assert_eq!(d.arc_factor, 1.0);
        assert!(d.safe_default);
    }

    #[test]
    fn missing_corpus_yields_safe_full_arc() {
        let d = derive(ArcInputs {
            corpus_bytes: None,
            ..base_inputs()
        });
        assert_eq!(d.arc_factor, 1.0);
        assert!(d.safe_default);
    }

    #[test]
    fn slack_memory_small_n_permits_full_arc() {
        // 8Gi ceiling, 1Gi corpus, N=14: memory is slack (a_mem=1.0), aim is
        // full at the alpha (arc_share≈1.0), so a = 1.0 — full arc PERMITTED,
        // a_cov=0.5 sits harmlessly below.
        let d = derive(base_inputs());
        assert!(!d.safe_default);
        assert!((d.coverage_floor - 0.5).abs() < 1e-9, "a_cov(14,7)=0.5");
        assert!(
            (d.arc_factor - 1.0).abs() < 1e-9,
            "slack memory ⇒ full arc, got {}",
            d.arc_factor
        );
        assert!(!d.coverage_over_memory);
    }

    #[test]
    fn memory_pressure_shrinks_to_a_mem_floored_by_coverage() {
        // Tight ceiling vs a large corpus drives a_mem between a_cov and 1.0:
        // M_eff=4Gi, C=5Gi, L=0 ⇒ a_mem=0.8; N=14 ⇒ a_cov=0.5. a clamps to 0.8.
        let d = derive(ArcInputs {
            mem_ceiling_bytes: Some(4 * GI),
            corpus_bytes: Some(5 * GI),
            ..base_inputs()
        });
        assert!(!d.coverage_over_memory);
        assert!(
            (d.arc_factor - 0.8).abs() < 1e-6,
            "a should clamp to a_mem=0.8, got {}",
            d.arc_factor
        );
    }

    #[test]
    fn james_case_coverage_wins_and_elevates() {
        // Lean ceiling, large corpus, small N: a_mem < a_cov. Coverage WINS
        // (a = a_cov), and a finding is elevated. The degenerate clamp(low>high)
        // region — must NOT panic, must NOT shrink into a gap.
        let d = derive(ArcInputs {
            mem_ceiling_bytes: Some(768 * 1024 * 1024), // chromebook-edu floor
            corpus_bytes: Some(6 * GI),                 // large corpus
            local_authored_bytes: 256 * 1024 * 1024,
            observed_n: Some(14),
            ..base_inputs()
        });
        assert!(d.coverage_over_memory, "memory cannot meet coverage");
        assert!(
            (d.arc_factor - d.coverage_floor).abs() < 1e-9,
            "coverage wins: a = a_cov"
        );
        assert!((d.coverage_floor - 0.5).abs() < 1e-9, "a_cov(14,7)=0.5");
        assert!(
            d.elevate.is_some(),
            "must elevate a finding, not shrink into a gap"
        );
    }

    #[test]
    fn local_share_exceeds_budget_still_holds_coverage() {
        // L ≥ M_eff ⇒ a_mem = 0, far below a_cov ⇒ coverage wins (never 0).
        let d = derive(ArcInputs {
            mem_ceiling_bytes: Some(GI),
            local_authored_bytes: 2 * GI, // own corpus alone exceeds the ceiling
            corpus_bytes: Some(4 * GI),
            ..base_inputs()
        });
        assert!(d.coverage_over_memory);
        assert!(
            d.arc_factor > 0.0,
            "Auto never derives a leecher (0); coverage floor holds"
        );
        assert!(d.elevate.is_some());
    }

    #[test]
    fn large_n_lowers_the_coverage_floor() {
        // Growing the network is the only way below a_cov=0.5. N=70 ⇒ a_cov=0.1.
        let d = derive(ArcInputs {
            observed_n: Some(70),
            mem_ceiling_bytes: Some(2 * GI),
            corpus_bytes: Some(8 * GI), // pressure so a_mem binds, not the slack case
            ..base_inputs()
        });
        assert!(
            (d.coverage_floor - 0.1).abs() < 1e-9,
            "a_cov(70,7)=0.1, got {}",
            d.coverage_floor
        );
        assert!(
            d.arc_factor >= d.coverage_floor - 1e-9,
            "never below coverage floor"
        );
    }

    #[test]
    fn auto_never_returns_leecher_zero() {
        // Across a sweep of pressures, Auto must never derive arc=0 (only an
        // operator override may, per spec §3c/§5).
        for n in [2u32, 7, 14, 50, 200] {
            for corpus_gi in [1u64, 4, 16, 64] {
                let d = derive(ArcInputs {
                    observed_n: Some(n),
                    mem_ceiling_bytes: Some(512 * 1024 * 1024),
                    corpus_bytes: Some(corpus_gi * GI),
                    ..base_inputs()
                });
                assert!(
                    d.arc_factor > 0.0,
                    "Auto leecher at N={n} corpus={corpus_gi}Gi"
                );
            }
        }
    }
}

//! CELL PROBE — the one read-only question that lets a QUIET role prove it
//! recovered.
//!
//! ## The hole this fills
//!
//! `3ec3614dd` made recovery provable by exactly one thing: a zome call on the
//! role that RETURNS. That is the correct evidence, and it left a gap it stated
//! honestly rather than hiding: a role with no continuing organic traffic has
//! nothing to prove recovery WITH. On this conductor (holochain 0.7 fork) the app
//! interfaces accept calls for up to ~11 minutes before cell initialization
//! finishes and answer `CellDisabled` meanwhile, so EVERY node restart produces a
//! disabled-cell episode on every role that is called early. For the quiet roles
//! that episode then never ends:
//!
//! | Role | Organic traffic | Guaranteed to continue? |
//! |---|---|---|
//! | `infrastructure` | the 60s heartbeat | yes, while the heartbeat is enabled |
//! | `lamad` | projection reconcile, adoption trigger | work-dependent |
//! | `imagodei` | membership reconcile | NO — an empty household set makes no calls |
//! | `node_registry` | shard assignment during an upload | NO |
//! | `mishpat` | commitment operations | NO |
//!
//! A permanently-open episode keeps the aggregate `zomePath` verdict at
//! `app-disabled`, which keeps `GET /health/serving` answering 503 for the life
//! of the process, and keeps `elohim_conductor_app_enabled{role}` at 0.
//!
//! ## The probe, and every bound on it
//!
//! ONE read-only zome call on the role's OWN cell, per enable-ladder rung, while
//! and only while that role is in a not-running episode. Success is not a new
//! transition: the probe is an ordinary zome call, so a probe that returns
//! reaches `conductor_bridge_health::record_role_success` through the same single
//! path organic traffic uses, and a probe that FAILS lands in the responsive
//! class, which cannot end an episode however the conductor phrased its refusal
//! — including `ZomeNotFound`/`FunctionNotFound` from a probe pointed at a
//! function the DNA does not have. That ordering matters: the probe is only safe
//! BECAUSE the responsive class was first made unable to declare recovery.
//!
//! `HcClient::call_zome` has no timeout and no cancellation path — a caller-side
//! timeout abandons the call while the conductor keeps running it, still holding
//! the read permit. So this is bounded BEFORE the call, by construction, not
//! after it by a deadline:
//!
//! **bounded-work:** work budget = one extern with NO DHT access and NO link
//! traversal — `is_bootstrap_steward` is `dna_info()` + `agent_info()` (both
//! cell-local, no network, no DB scan), and the `infrastructure` probe is an
//! extern that returns an empty vector without reading anything. Concurrency
//! budget = ONE in flight per role, enforced by [`ProbeGate`] rather than by the
//! shape of the caller, so it holds no matter which supervisor task drives it.
//! Rate budget = one per role per [`crate::services::enable_app_backoff::
//! enable_backoff`] window (60s doubling to a 1h cap) — the EXISTING ladder, no
//! new timer and no new task. Never retried inside a rung. Never on a request
//! path. Never while the role is healthy — a probe is evidence-gathering about an
//! episode, and a role with no episode has nothing to gather.
//!
//! ## Why these externs, and what would make the choice wrong
//!
//! Each name below is an EXISTING extern in the shipped coordinator zome. No
//! zome was changed to add a probe: a coordinator change is a separate decision
//! with its own hot-swap consequences, and a probe is not a reason to spend one.
//!
//! `is_bootstrap_steward` is the natural choice — it exists in four of the five
//! coordinators and reads only DNA modifiers and this agent's own key. The
//! `infrastructure` zome has no `is_bootstrap_steward`, and its cheapest
//! write-free extern is `get_doorway_attestations`, which today is a
//! `TODO(Stage-F)` stub returning `Ok(Vec::new())` — zero work of any kind. If
//! Stage-F ever implements it as the cross-DNA bridge its TODO describes, this
//! choice stops being zero-cost and should move to `get_doorway_by_id` with an id
//! that cannot exist (one local `hash_entry` plus one `get_links` that misses).
//! That is the one maintenance obligation this module carries.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock, RwLock};

use tracing::{debug, info, warn};

use crate::hc_client::{HcClient, IMAGODEI_ROLE, MISHPAT_ROLE};

/// What a probe sends. Two shapes because the two chosen externs take two
/// argument types; both encode to a handful of bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbePayload {
    /// `(_: ())` — msgpack `nil`.
    Unit,
    /// A `String` argument the extern ignores (or that cannot match anything).
    IgnoredId,
}

/// The one read-only question asked of a role's cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellProbe {
    pub zome: &'static str,
    pub fn_name: &'static str,
    pub payload: ProbePayload,
}

/// A `String` argument for [`ProbePayload::IgnoredId`]. Namespaced so that if an
/// extern that currently ignores its argument ever starts reading it, the probe
/// asks about something that provably does not exist rather than something real.
pub const PROBE_ID: &str = "__elohim_cell_probe__";

/// The probe extern for `role`, or `None` for a role this node has no verified
/// write-free read on.
///
/// `None` is a real answer and NOT a failure: the role's episode simply stays
/// open until organic traffic proves recovery, exactly as before this module
/// existed. It is never a licence to guess a function name — a guess answers
/// `FunctionNotFound`, which is precisely the false-recovery route the
/// responsive/served split closed, and it would be a silent no-op besides.
pub fn probe_for(role: &str) -> Option<CellProbe> {
    let (zome, fn_name, payload) = match role {
        // content_store::is_bootstrap_steward — dna_info() + agent_info().
        "lamad" => ("content_store", "is_bootstrap_steward", ProbePayload::Unit),
        IMAGODEI_ROLE => ("imagodei", "is_bootstrap_steward", ProbePayload::Unit),
        MISHPAT_ROLE => ("mishpat", "is_bootstrap_steward", ProbePayload::Unit),
        "node_registry" => (
            "node_registry_coordinator",
            "is_bootstrap_steward",
            ProbePayload::Unit,
        ),
        // No `is_bootstrap_steward` in this zome; see the module doc.
        "infrastructure" => (
            "infrastructure",
            "get_doorway_attestations",
            ProbePayload::IgnoredId,
        ),
        _ => return None,
    };
    Some(CellProbe {
        zome,
        fn_name,
        payload,
    })
}

impl CellProbe {
    /// MessagePack bytes for this probe's argument, using the crate's
    /// `to_vec_named` convention.
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        match self.payload {
            ProbePayload::Unit => {
                rmp_serde::to_vec_named(&()).map_err(|e| format!("encode (): {e}"))
            }
            ProbePayload::IgnoredId => rmp_serde::to_vec_named(&PROBE_ID.to_string())
                .map_err(|e| format!("encode probe id: {e}")),
        }
    }
}

/// ONE probe in flight per role, whoever asks.
///
/// A per-role latch rather than "the supervisor task is sequential, so it
/// cannot overlap": that reasoning is true of the four roles with their own
/// task and FALSE of a cross-cell role tended by another role's task, and it
/// would go on being true right up until someone added a second driver. The
/// invariant is cheap enough to enforce instead of argue.
#[derive(Debug, Default)]
pub struct ProbeGate {
    roles: RwLock<BTreeMap<String, Arc<AtomicBool>>>,
}

/// Holds one role's probe slot. Releases on drop, so a panic or an early return
/// inside a probe cannot wedge the role shut forever.
pub struct ProbeSlot {
    flag: Arc<AtomicBool>,
}

impl Drop for ProbeSlot {
    fn drop(&mut self) {
        self.flag.store(false, Ordering::SeqCst);
    }
}

impl ProbeGate {
    pub fn new() -> Self {
        Self::default()
    }

    fn flag(&self, role: &str) -> Arc<AtomicBool> {
        if let Some(f) = self
            .roles
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(role)
        {
            return Arc::clone(f);
        }
        let mut w = self.roles.write().unwrap_or_else(|e| e.into_inner());
        Arc::clone(w.entry(role.to_string()).or_default())
    }

    /// Claim `role`'s single probe slot, or `None` when a probe is already in
    /// flight for it. `compare_exchange` so two drivers cannot both win.
    pub fn claim(&self, role: &str) -> Option<ProbeSlot> {
        let flag = self.flag(role);
        flag.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .ok()
            .map(|_| ProbeSlot { flag })
    }

    /// Is a probe in flight for `role`? Observability and tests only.
    pub fn in_flight(&self, role: &str) -> bool {
        self.flag(role).load(Ordering::SeqCst)
    }
}

/// The ONE process-wide gate. Tests build their own [`ProbeGate`], the same
/// parallel-test discipline `conductor_bridge_health::bridge_health` documents.
pub fn probe_gate() -> &'static ProbeGate {
    static GATE: OnceLock<ProbeGate> = OnceLock::new();
    GATE.get_or_init(ProbeGate::new)
}

/// Why a probe did not happen, or what happened when it did.
///
/// `Served` carries no payload on purpose: the recovery has ALREADY been
/// recorded by the time this returns, inside the `HcClient` call path, through
/// `record_role_success`. Returning a verdict the caller must act on would be a
/// second transition, which is the thing this design forbids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeVerdict {
    /// The call returned. `record_role_success(role)` has already fired.
    Served,
    /// The call failed. The episode is untouched; `msg` is the conductor's own
    /// words.
    Refused { msg: String },
    /// The role is not in a not-running episode — there is nothing to prove.
    NotInEpisode,
    /// No verified write-free read exists for this role.
    NoProbeAvailable,
    /// A probe is already in flight for this role.
    AlreadyInFlight,
    /// This client holds no `CellId` for this role, so it cannot ask.
    Unreachable,
    /// The argument would not encode. Cannot happen for the two shapes above;
    /// reported rather than unwrapped.
    EncodeFailed { msg: String },
}

/// May a probe of `role` happen at all, and if so which one?
///
/// EVERY bound that does not need a conductor lives here, so every bound is
/// unit-testable: the never-while-healthy gate, the have-we-a-verified-read
/// gate, and the one-in-flight-per-role gate. [`probe_cell`] is this decision
/// plus the call, and it has no gate of its own.
///
/// `Err(verdict)` is the reason nothing will be asked. The returned [`ProbeSlot`]
/// must be held for the whole of the call: dropping it releases the role.
pub fn probe_admission(
    role: &str,
    gate: &ProbeGate,
) -> Result<(CellProbe, ProbeSlot), ProbeVerdict> {
    probe_admission_with(role, probe_for(role), gate)
}

/// [`probe_admission`] with the table lookup handed in.
///
/// The seam exists so the gates can be driven against a role name that belongs
/// to one test alone. Keying the episode state by a REAL role in a test would
/// mean a parallel test reading the process-wide aggregate could observe this
/// one's synthetic outage — the flake class this crate avoids by construction.
pub fn probe_admission_with(
    role: &str,
    probe: Option<CellProbe>,
    gate: &ProbeGate,
) -> Result<(CellProbe, ProbeSlot), ProbeVerdict> {
    // NEVER while the role is healthy. Checked first so a healthy role costs a
    // single atomic read and no lock, no encode and no conductor round trip.
    // This is the bound that keeps the probe evidence-gathering about an
    // OPEN episode rather than a background poll of a working node.
    if !crate::conductor_bridge_health::role_is_not_running(role) {
        return Err(ProbeVerdict::NotInEpisode);
    }
    let Some(probe) = probe else {
        return Err(ProbeVerdict::NoProbeAvailable);
    };
    let Some(slot) = gate.claim(role) else {
        return Err(ProbeVerdict::AlreadyInFlight);
    };
    Ok((probe, slot))
}

/// Ask `role`'s cell one read-only question through `hc`, if and only if every
/// bound in [`probe_admission`] permits it.
///
/// `hc` must be a client that can reach `role`'s cell: its own configured role,
/// or one of the cross-cell roles it resolved at connect time.
///
/// Contains NO recovery transition of its own — deliberately. The `HcClient`
/// method below records the success, against the TARGET role, through
/// `conductor_bridge_health::record_role_success`, which is the one function
/// entitled to end an episode. A `record_role_success` call in THIS module
/// would be the second transition the design forbids.
pub async fn probe_cell(role: &str, hc: &HcClient, gate: &ProbeGate) -> ProbeVerdict {
    let (probe, _slot) = match probe_admission(role, gate) {
        Ok(admitted) => admitted,
        Err(verdict) => return verdict,
    };
    let payload = match probe.encode() {
        Ok(bytes) => bytes,
        Err(msg) => return ProbeVerdict::EncodeFailed { msg },
    };

    debug!(
        role,
        zome = probe.zome,
        fn_name = probe.fn_name,
        "probing a not-running role's cell with one read-only call — a call that RETURNS is the \
         only proof of recovery this node accepts"
    );

    // Routed so the observation is attributed to the TARGET cell's role: each
    // method records against the role that owns the cell it targets, which is
    // what makes a probe on mishpat end mishpat's episode and nobody else's.
    let result = if role == hc.role_key() {
        hc.call_zome(probe.zome, probe.fn_name, payload).await
    } else if role == MISHPAT_ROLE {
        hc.call_zome_mishpat(probe.zome, probe.fn_name, payload)
            .await
    } else if role == IMAGODEI_ROLE {
        hc.call_zome_imagodei(probe.zome, probe.fn_name, payload)
            .await
    } else {
        return ProbeVerdict::Unreachable;
    };

    match result {
        Ok(_) => {
            info!(
                role,
                zome = probe.zome,
                fn_name = probe.fn_name,
                "the read-only cell probe RETURNED — this role's cell is serving calls again"
            );
            ProbeVerdict::Served
        }
        Err(e) => {
            let msg = e.to_string();
            // A probe aimed at a function the DNA does not have can never read
            // as recovery (the responsive class cannot end an episode), so the
            // failure mode is a role that never clears — silent unless it is
            // said. Say it, once per rung, at WARN.
            if names_a_missing_function(&msg) {
                warn!(
                    role,
                    zome = probe.zome,
                    fn_name = probe.fn_name,
                    error = %msg,
                    "the read-only cell probe names a zome function this DNA does not have — the \
                     probe cannot prove recovery for this role and the episode will only clear on \
                     organic traffic. Fix the probe table in services::cell_probe."
                );
            } else {
                debug!(role, error = %msg, "the cell probe was refused — the episode stands");
            }
            ProbeVerdict::Refused { msg }
        }
    }
}

/// Does this error say the zome or function does not exist, as opposed to the
/// cell refusing to serve?
///
/// Used ONLY to decide whether to shout about a misconfigured probe. It is
/// deliberately not part of [`crate::conductor_bridge_health::
/// classify_zome_error`]: the health verdict must not depend on recognising this
/// string, because the whole responsive class is already barred from ending an
/// episode.
pub fn names_a_missing_function(msg: &str) -> bool {
    let m = msg.to_ascii_lowercase();
    [
        "zomenotfound",
        "functionnotfound",
        "zome function not found",
    ]
    .iter()
    .any(|marker| m.contains(marker))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_observed_role_has_a_probe_and_it_is_a_verified_read() {
        // The table's reason to exist: a role with no probe is a role whose
        // episode can only be ended by organic traffic it may never get.
        for role in crate::hc_client_registry::OBSERVED_ROLES {
            let probe = probe_for(role)
                .unwrap_or_else(|| panic!("observed role '{role}' has no read-only cell probe"));
            // A probe that the chain-write gate reads as a WRITE would take the
            // cell's write lock and queue behind real writers — unbounded wait
            // on a call nothing can cancel. It must be a classified read.
            assert!(
                crate::chain_write_gate::is_read_fn(probe.fn_name),
                "probe '{}' for role '{role}' is not in the chain-write read table — it would \
                 serialize behind writers on an uncancellable call",
                probe.fn_name
            );
            assert!(probe.encode().is_ok(), "probe for '{role}' must encode");
        }
    }

    #[test]
    fn an_unknown_role_has_no_probe_rather_than_a_guess() {
        assert_eq!(probe_for("qahal"), None);
        assert_eq!(probe_for(""), None);
    }

    #[test]
    fn the_unit_probe_encodes_as_msgpack_nil() {
        let probe = probe_for("lamad").expect("lamad probes");
        assert_eq!(probe.payload, ProbePayload::Unit);
        assert_eq!(probe.encode().expect("encodes"), vec![0xc0]);
    }

    #[test]
    fn at_most_one_probe_is_in_flight_per_role() {
        let gate = ProbeGate::new();
        let first = gate.claim("lamad").expect("the first claim wins");
        assert!(gate.in_flight("lamad"));
        assert!(
            gate.claim("lamad").is_none(),
            "a second probe on the same role must be refused, not queued — the call it would make \
             cannot be cancelled"
        );
        // A different role is independent: one stuck cell must not silence the
        // proof-gathering for another.
        let other = gate.claim("mishpat").expect("another role is unblocked");
        assert!(gate.in_flight("mishpat"));

        drop(first);
        assert!(!gate.in_flight("lamad"), "the slot releases on drop");
        assert!(gate.claim("lamad").is_some(), "and is reclaimable");
        drop(other);
        assert!(!gate.in_flight("mishpat"));
    }

    /// The same invariant through the ADMISSION path production uses, so a
    /// future `probe_cell` that grew its own gate could not pass this.
    #[test]
    fn at_most_one_probe_is_admitted_per_role() {
        // A role name that belongs to this test alone. The episode state is
        // process-wide and keyed by role, and this name is in no role roster, so
        // it cannot reach the aggregate or another test.
        const ROLE: &str = "test-one-probe-in-flight";
        let gate = ProbeGate::new();
        let probe = probe_for("lamad").expect("a real probe shape");

        // A role in an episode with NO verified read is refused for that reason
        // — never guessed at, because a guessed function name answers
        // FunctionNotFound and would be a silent no-op forever.
        crate::conductor_bridge_health::record_role_app_disabled(
            ROLE,
            "synthetic: cells not running",
        );
        assert_eq!(
            probe_admission_with(ROLE, None, &gate).err(),
            Some(ProbeVerdict::NoProbeAvailable)
        );
        assert!(
            !gate.in_flight(ROLE),
            "a refused admission must not hold the slot"
        );

        // With a probe available: admitted once, refused while in flight.
        let admitted =
            probe_admission_with(ROLE, Some(probe), &gate).expect("the first rung is admitted");
        assert_eq!(
            probe_admission_with(ROLE, Some(probe), &gate).err(),
            Some(ProbeVerdict::AlreadyInFlight),
            "a second probe while one is in flight is refused — never retried inside a rung, \
             because the call it would make cannot be cancelled"
        );
        drop(admitted);
        assert!(
            probe_admission_with(ROLE, Some(probe), &gate).is_ok(),
            "the NEXT rung may ask"
        );
    }

    #[test]
    fn the_probe_never_runs_while_the_role_is_healthy() {
        // A probe is evidence-gathering about an OPEN episode. A role that is
        // serving calls has nothing to prove, and a background poll of a
        // working conductor is exactly the cost this design refuses to add.
        const ROLE: &str = "test-probe-never-while-healthy";
        let gate = ProbeGate::new();

        // Never observed at all — Unknown, which is not an episode.
        assert_eq!(
            probe_admission(ROLE, &gate).err(),
            Some(ProbeVerdict::NotInEpisode)
        );

        // Observed serving — still not an episode.
        crate::conductor_bridge_health::record_role_success(ROLE);
        assert_eq!(
            probe_admission(ROLE, &gate).err(),
            Some(ProbeVerdict::NotInEpisode)
        );
        assert!(
            !gate.in_flight(ROLE),
            "a healthy role's slot is never even claimed"
        );
    }

    /// The probe is not a second recovery transition. Asserted on the source,
    /// because the claim is an ABSENCE and no runtime assertion can see one.
    #[test]
    fn the_probe_declares_no_recovery_of_its_own() {
        // Scan the PRODUCTION half only, with `//` tails dropped: the claim is
        // about executable code, and both the module doc and this test's own
        // failure message name the function they forbid. (Same crude-but-adequate
        // technique as `chain_write_gate`'s drift rail — over-trimming can only
        // ever hide a name, never invent one.)
        let source = include_str!("cell_probe.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("the module has a production half");
        let code: String = production
            .lines()
            .map(|line| match line.find("//") {
                Some(idx) => &line[..idx],
                None => line,
            })
            .collect::<Vec<_>>()
            .join("\n");

        for forbidden in [
            "record_role_success",
            "record_success",
            "record_responsive",
            "note_running",
        ] {
            assert!(
                !code.contains(forbidden),
                "cell_probe must not call `{forbidden}` — the HcClient path records the success, \
                 against the TARGET role, through the ONE transition. A fold here would be a \
                 second transition, which is what let a probe's own answer become its own proof."
            );
        }
        // ...and it must still be the case that the probe REACHES that path: the
        // only thing it does with a client is make a zome call.
        assert!(
            code.contains("hc.call_zome(")
                && code.contains("hc.call_zome_mishpat(")
                && code.contains("hc.call_zome_imagodei("),
            "the probe must route through the HcClient call paths, which are what attribute the \
             observation to the target role"
        );
    }

    #[test]
    fn a_missing_function_answer_is_recognised_so_a_bad_probe_is_loud() {
        for msg in [
            "Zome call failed: ZomeNotFound: mishpat",
            "Zome call failed: FunctionNotFound(\"is_bootstrap_steward\")",
        ] {
            assert!(names_a_missing_function(msg), "{msg}");
        }
        // ...and a cell that simply will not serve is NOT that.
        assert!(!names_a_missing_function(
            "Zome call failed: Conductor returned an error while using a ConductorApi: \
             CellDisabled(CellId(DnaHash(uhC0k), AgentPubKey(uhCAk)))"
        ));
    }
}

//! D.9 — the explanation, recomputed rather than stored (spec §7.4).
//!
//! Each commitment carries method, parameter-set CID and inputs digest. Asking why re-runs the pure
//! pass over the inputs the peer holds and reads the one need back out of it.

use std::collections::BTreeMap;

use cid::Cid;
use elohim_epr_rea::AgentRef;
use seam_contracts::{ReasonLabel, Unanswered};
use serde::Serialize;

use crate::decide::{not_carried, observed_digest, run_pass, Observed, Snapshot};
use crate::eligibility::Ineligible;
use crate::kind::{KindId, KindRegistry, MethodRegistry};
use crate::need::{NeedClass, NeedKey};
use crate::offer::CidString;
use crate::params::{ParamFallback, RatificationKind, RatificationRef};
use crate::rank::{MethodId, Scratch};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Explanation {
    pub need: CidString,
    pub key: NeedKey,
    pub kind: KindId,
    pub method: MethodId,
    pub parameters: RatificationRef,
    /// WHICH set was in force (`ParameterSet::cid`); `parameters` says what ratified it. One of the
    /// three fields a commitment carries, so it is written for every outcome. The view schema
    /// requires it (`parameterSet`); added in Codex review 1 with the schema validation.
    pub parameter_set: CidString,
    /// Which fallback applied, when one did (E10).
    pub fallback: Option<ParamFallback>,
    /// 64 hex characters on the wire (G13).
    pub inputs_digest: String,
    /// The asker's own devices, listed.
    pub own_refusals: Vec<Ineligible>,
    /// Other members' refusals, COUNTED by label, never listed.
    pub others_refused: BTreeMap<String, u32>,
    pub eligible: u32,
    /// Named only at the reach their posture declared.
    pub chosen: Vec<AgentRef>,
    pub outcome: Outcome,
    /// `None`: no commitment to compare.
    pub digest_matches_commitment: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    tag = "outcome"
)]
pub enum Outcome {
    /// Honored holders meet the demand in holders and domains.
    Carried,
    /// The chosen carriers meet the demand; honoring has not caught up yet.
    Held,
    NotCarried {
        why: Unanswered<NotCarriedReason>,
    },
    InputsGone {
        why: Unanswered<NotCarriedReason>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    tag = "reason"
)]
pub enum NotCarriedReason {
    NoEligible,
    DomainShort { have: u16, need: u16 },
    ServedFirst { class: NeedClass },
    NoPriorityChosen,
    MethodNotRun,
    InputsGone,
}

/// The `Copy` face of [`NotCarriedReason`], for `ReasonLabel` (§2.0's contract). Additive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotCarriedKind {
    NoEligible,
    DomainShort,
    ServedFirst,
    NoPriorityChosen,
    MethodNotRun,
    InputsGone,
}

impl ReasonLabel for NotCarriedKind {
    const ALL: &'static [Self] = &[
        NotCarriedKind::NoEligible,
        NotCarriedKind::DomainShort,
        NotCarriedKind::ServedFirst,
        NotCarriedKind::NoPriorityChosen,
        NotCarriedKind::MethodNotRun,
        NotCarriedKind::InputsGone,
    ];
    fn label(&self) -> &'static str {
        match self {
            NotCarriedKind::NoEligible => "no-eligible",
            NotCarriedKind::DomainShort => "domain-short",
            NotCarriedKind::ServedFirst => "served-first",
            NotCarriedKind::NoPriorityChosen => "no-priority-chosen",
            NotCarriedKind::MethodNotRun => "method-not-run",
            NotCarriedKind::InputsGone => "inputs-gone",
        }
    }
}

impl NotCarriedReason {
    pub fn kind(&self) -> NotCarriedKind {
        match self {
            NotCarriedReason::NoEligible => NotCarriedKind::NoEligible,
            NotCarriedReason::DomainShort { .. } => NotCarriedKind::DomainShort,
            NotCarriedReason::ServedFirst { .. } => NotCarriedKind::ServedFirst,
            NotCarriedReason::NoPriorityChosen => NotCarriedKind::NoPriorityChosen,
            NotCarriedReason::MethodNotRun => NotCarriedKind::MethodNotRun,
            NotCarriedReason::InputsGone => NotCarriedKind::InputsGone,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotCarried {
    pub need: CidString,
    pub key: NeedKey,
    pub receiver: AgentRef,
    pub why: Unanswered<NotCarriedReason>,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Recompute why one need is placed as it is. `None` when the snapshot holds no such need: the three
/// fields every commitment carries (method, parameter set, inputs digest) are then what remains, and
/// they live with the commitment, not in this snapshot.
pub fn explain(
    s: &Snapshot,
    kinds: &KindRegistry,
    methods: &MethodRegistry,
    need: &Cid,
    asker: &AgentRef,
    committed: Option<[u8; 32]>,
) -> Option<Explanation> {
    let mut scratch = Scratch::default();
    let pass = run_pass(s, kinds, methods, &mut scratch);
    let eval = pass.evals.into_iter().find(|e| e.id == *need)?;
    // The retained set's own provenance first (`LastVerified`), then the built-in.
    let fallback = eval.in_force.fallback().or_else(|| {
        (eval.builtin_fallback || eval.in_force.ratification().by == RatificationKind::BuiltIn)
            .then_some(ParamFallback::BuiltIn)
    });
    let mut out = Explanation {
        need: CidString::from(&eval.id),
        key: eval.need.key.clone(),
        kind: eval.need.kind,
        method: eval.in_force.set().method,
        parameters: eval.in_force.ratification().clone(),
        parameter_set: CidString::from(eval.in_force.cid()),
        fallback,
        // Observe-only (no charter, no kind, or the method not run here): the digest is over the
        // need and the set in force with no ranked order, so it is still 64 hex characters (G13,
        // `^[0-9a-f]{64}$`). Codex review 1, finding 3.
        inputs_digest: hex(&observed_digest(&eval.id, &eval.in_force)),
        own_refusals: Vec::new(),
        others_refused: BTreeMap::new(),
        eligible: 0,
        chosen: Vec::new(),
        outcome: Outcome::NotCarried {
            why: not_carried(NotCarriedReason::MethodNotRun),
        },
        digest_matches_commitment: None,
    };
    let ranked = match eval.outcome {
        Err(Observed::NoCharter | Observed::UnknownKind | Observed::MethodNotRun) => {
            return Some(out)
        }
        Ok(r) => r,
    };
    out.inputs_digest = hex(&ranked.digest);
    out.digest_matches_commitment = committed.map(|d| d == ranked.digest);
    out.eligible = ranked.order.len() as u32;
    for refusal in ranked.ineligible {
        if s.normalize(&refusal.agent) == asker {
            out.own_refusals.push(refusal);
        } else {
            *out.others_refused
                .entry(refusal.why.why.kind().label().to_string())
                .or_default() += 1;
        }
    }
    let need_openness = eval.need.reach.openness();
    out.chosen = ranked
        .chosen
        .iter()
        .filter(|a| {
            ranked
                .order
                .iter()
                .any(|(agent, _, _, reach)| agent == *a && reach.openness() >= need_openness)
        })
        .map(|a| (*a).clone())
        .collect();
    out.outcome = match ranked.short {
        Some(why) => Outcome::NotCarried {
            why: not_carried(why),
        },
        None if ranked.honored >= ranked.demand.holders as usize
            && ranked.honored_domains >= ranked.demand.domains as usize =>
        {
            Outcome::Carried
        }
        None => Outcome::Held,
    };
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_carried_labels_conform() {
        seam_contracts::assert_reason_labels_conformant::<NotCarriedKind>();
    }
}

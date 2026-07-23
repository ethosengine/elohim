//! Epistemic standing — peer review as value flow.
//!
//! Status is DERIVED by this fold, never stored (the founding law). Two peers
//! folding the same events must produce byte-identical standing: events are
//! sorted by CID before accumulation because f64 summation is order-sensitive.
//!
//! The fold's input is a borrowed projection of the fields it needs from
//! [`elohim_epr::witness::WitnessedInteraction`] — the wire parent. Only the review
//! verbs `Cite`/`Affirm`/`Dismiss` are review-relevant; every other [`ReaVerb`]
//! (`Use`/`Consume`/`Produce`) is ignored by the fold.
//!
//! This module is the **source of truth for [`EpistemicStatus`]** (per the axis card):
//! the fold decides communal standing, so the vocabulary lives beside the fold.
//! `elohim/epr` stays codec-pure and does not carry it. The wire values match
//! `enums/epistemic-status.schema.json` exactly.

use cid::Cid;
use serde::{Deserialize, Serialize};

use elohim_epr::verdict::{
    CheckOutcome, CheckWitness, Decision, ReferQuestion, ReferReason, Verdict, Witness,
};
use elohim_epr::witness::ReaVerb;

/// A borrowed review event — the three fields the fold reads from a witnessed
/// interaction.
///
/// NOT a persisted type: the wire parent is [`elohim_epr::witness::WitnessedInteraction`]
/// (`object_cid`, `action`, `magnitude`). This is only the fold's input shape, carrying
/// the event's own CID so the fold can order deterministically.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReviewEvent {
    /// The event's own CID — the canonical sort key (the determinism law).
    pub event_cid: Cid,
    /// What the witness did to the reviewed object.
    pub verb: ReaVerb,
    /// The magnitude the witness attached (substrate-denominated weight).
    pub magnitude: f64,
}

/// Derived standing of one subject: review tallies + magnitude weights, folded over
/// its witnessed review events. Never stored (the founding law) — a re-fold in
/// canonical CID order reconstructs it identically on any peer.
#[derive(Debug, Clone, PartialEq)]
pub struct EpistemicStanding {
    pub subject: Cid,
    pub affirmations: u64,
    pub dismissals: u64,
    pub citations: u64,
    pub affirm_weight: f64,
    pub dismiss_weight: f64,
}

impl EpistemicStanding {
    /// Reviews that carry a communal-standing signal (affirm/dismiss). Citations are
    /// usage, not votes — excluded here and from [`Self::standing_ratio`].
    pub fn review_count(&self) -> u64 {
        self.affirmations + self.dismissals
    }

    /// affirm_weight / (affirm_weight + dismiss_weight); `None` when both are zero
    /// (empty-never-projects — no signal is not a 0.0 signal). Mirrors the Option
    /// shape of `FulfillmentStatus::ratio`.
    pub fn standing_ratio(&self) -> Option<f64> {
        let total = self.affirm_weight + self.dismiss_weight;
        if total > 0.0 {
            Some(self.affirm_weight / total)
        } else {
            None
        }
    }

    /// dismiss_weight / (affirm_weight + dismiss_weight); `None` when both zero.
    /// The contest measure — how much of the weighted signal is dissent.
    fn dissent_ratio(&self) -> Option<f64> {
        let total = self.affirm_weight + self.dismiss_weight;
        if total > 0.0 {
            Some(self.dismiss_weight / total)
        } else {
            None
        }
    }
}

/// Thresholds governing classification. Slice-1 scaffolding — hardcoded defaults.
#[derive(Debug, Clone, PartialEq)]
pub struct EpistemicThresholds {
    /// Reviews (affirm + dismiss) needed to graduate `emergent → reviewed`.
    pub min_reviews: u64,
    /// Weighted dissent share at/above which a standing is `contested`.
    pub contest_ratio: f64,
}

impl Default for EpistemicThresholds {
    fn default() -> Self {
        // TODO(policy-ref): thresholds graduate to a manifest-declared policy binding;
        // hardcoded defaults are slice-1 scaffolding.
        Self {
            min_reviews: 3,
            contest_ratio: 0.25,
        }
    }
}

/// A reference to the governance act (Mishpat `Precedent` lineage) that conferred canon.
///
/// A threshold alone is structurally unable to mint canon: [`classify`] requires one of
/// these to ever return [`EpistemicStatus::Canon`]. The precedent is the act; this type
/// only points at it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonizationRef {
    pub precedent: Cid,
}

/// Communal standing of a claim — derived, never stored. Wire values match
/// `enums/epistemic-status.schema.json` exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EpistemicStatus {
    Emergent,
    Reviewed,
    Contested,
    Canon,
    Superseded,
}

/// Fold review events into a subject's [`EpistemicStanding`].
///
/// **Determinism law.** Events are sorted by their CID before accumulation. f64
/// summation is not associative under reordering, so two peers handed the same event
/// set in different orders would otherwise compute different weights — and a
/// derived-never-stored status only holds if every peer derives the *same* value.
/// Sorting by the immutable event CID gives one canonical order every peer agrees on.
pub fn fold_standing(subject: Cid, events: &[ReviewEvent]) -> EpistemicStanding {
    let mut ordered: Vec<&ReviewEvent> = events.iter().collect();
    ordered.sort_by_key(|e| e.event_cid);

    let mut standing = EpistemicStanding {
        subject,
        affirmations: 0,
        dismissals: 0,
        citations: 0,
        affirm_weight: 0.0,
        dismiss_weight: 0.0,
    };
    for ev in ordered {
        match ev.verb {
            ReaVerb::Affirm => {
                standing.affirmations += 1;
                standing.affirm_weight += ev.magnitude;
            }
            ReaVerb::Dismiss => {
                standing.dismissals += 1;
                standing.dismiss_weight += ev.magnitude;
            }
            ReaVerb::Cite => {
                standing.citations += 1;
            }
            // Use / Consume / Produce are consumption/production verbs, not review
            // signals — ignored by the epistemic fold.
            ReaVerb::Use | ReaVerb::Consume | ReaVerb::Produce => {}
        }
    }
    standing
}

/// Classify a standing into its communal [`EpistemicStatus`].
///
/// - **Contested** — any review present (`review_count >= 1`) AND weighted dissent share
///   at/above `contest_ratio`. Contested is sideways, not a rung: it dominates every
///   other status (a live dispute is never silently "reviewed" or "canon").
/// - **Canon** — ONLY when a [`CanonizationRef`] is present AND not contested
///   (the canon-requires-governance-act law). A threshold alone can never mint canon;
///   the maximum *mechanical* status is `Reviewed`.
/// - **Reviewed** — enough reviews (`review_count >= min_reviews`), not contested, no
///   canonization.
/// - **Emergent** — the floor (everything else).
///
/// [`EpistemicStatus::Superseded`] is deliberately NOT in this function's range: it is a
/// close-interval lifecycle state reached when a successor supersedes the claim, not a
/// fold outcome.
pub fn classify(
    standing: &EpistemicStanding,
    canonization: Option<&CanonizationRef>,
    thresholds: &EpistemicThresholds,
) -> EpistemicStatus {
    let contested = standing.review_count() >= 1
        && standing
            .dissent_ratio()
            .map(|r| r >= thresholds.contest_ratio)
            .unwrap_or(false);

    if contested {
        return EpistemicStatus::Contested;
    }
    // Canon can only be conferred by a referenced governance act — never by a threshold.
    if canonization.is_some() {
        return EpistemicStatus::Canon;
    }
    if standing.review_count() >= thresholds.min_reviews {
        return EpistemicStatus::Reviewed;
    }
    EpistemicStatus::Emergent
}

/// The cite gate — may this subject be cited, and how does the answer read on the spine?
///
/// Returns a full [`Verdict`] on axis `"epistemic"` whose [`Witness`] carries the standing
/// tallies as positive `observed` evidence:
/// - **Canon / Reviewed** — [`Decision::Permit`] (settled or peer-reviewed).
/// - **Emergent** — [`Decision::Permit`], served honestly AS emergent: emergent claims MAY
///   be cited (as emergent); the witness carries the status so the citer knows what it is.
/// - **Contested** — [`Decision::Refer`] to the `community` layer with
///   [`ReferReason::ContestedEvidence`]: an active dispute is not the gate's to resolve.
/// - **Superseded** — [`Decision::Refuse`]: cite the successor instead (noted in the witness).
pub fn cite_gate(standing: &EpistemicStanding, status: &EpistemicStatus) -> Verdict {
    let status_str = serde_json::to_value(status)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default();

    let tally_observed = serde_json::json!({
        "affirmations": standing.affirmations,
        "dismissals": standing.dismissals,
        "citations": standing.citations,
        "affirmWeight": standing.affirm_weight,
        "dismissWeight": standing.dismiss_weight,
        "standingRatio": standing.standing_ratio(),
    });

    let decision = match status {
        EpistemicStatus::Canon | EpistemicStatus::Reviewed | EpistemicStatus::Emergent => {
            Decision::Permit
        }
        EpistemicStatus::Contested => Decision::Refer(ReferQuestion {
            layer: "community".into(),
            reason: ReferReason::ContestedEvidence,
            note: Some(format!(
                "dissent share {:.3} meets the contest threshold — cite only after the dispute resolves",
                standing.dissent_ratio().unwrap_or(0.0)
            )),
        }),
        EpistemicStatus::Superseded => Decision::Refuse,
    };

    let status_outcome = match status {
        EpistemicStatus::Contested | EpistemicStatus::Superseded => CheckOutcome::Failed,
        _ => CheckOutcome::Passed,
    };

    let mut checks = vec![
        CheckWitness {
            check_id: "epistemic-status".into(),
            outcome: status_outcome,
            summary: format!("subject stands as {status_str}"),
            observed: Some(serde_json::Value::String(status_str.clone())),
        },
        CheckWitness {
            check_id: "review-tally".into(),
            outcome: CheckOutcome::Passed,
            summary: format!(
                "{} affirmations, {} dismissals, {} citations",
                standing.affirmations, standing.dismissals, standing.citations
            ),
            observed: Some(tally_observed),
        },
    ];
    if matches!(status, EpistemicStatus::Superseded) {
        checks.push(CheckWitness {
            check_id: "supersession".into(),
            outcome: CheckOutcome::Failed,
            summary: "a successor supersedes this claim — cite the successor".into(),
            observed: None,
        });
    }

    Verdict {
        axis: "epistemic".into(),
        subject: Some(standing.subject.to_string()),
        decision,
        witness: Witness { checks },
        policy_ref: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elohim_epr::cid::compute_cid;

    fn cid(seed: u8) -> Cid {
        compute_cid(&[seed])
    }

    fn subject() -> Cid {
        cid(0)
    }

    fn ev(seed: u8, verb: ReaVerb, magnitude: f64) -> ReviewEvent {
        ReviewEvent {
            event_cid: cid(seed),
            verb,
            magnitude,
        }
    }

    /// Positive-only evidence below the min-review count stays at the floor: affirmations
    /// alone cannot self-promote to `reviewed` without enough reviews.
    #[test]
    fn positive_only_below_min_stays_emergent() {
        let events = [ev(1, ReaVerb::Affirm, 1.0), ev(2, ReaVerb::Affirm, 1.0)];
        let s = fold_standing(subject(), &events);
        assert_eq!(
            classify(&s, None, &EpistemicThresholds::default()),
            EpistemicStatus::Emergent
        );
    }

    /// Enough uncontested reviews graduates `emergent → reviewed`.
    #[test]
    fn min_reviews_reaches_reviewed() {
        let events = [
            ev(1, ReaVerb::Affirm, 1.0),
            ev(2, ReaVerb::Affirm, 1.0),
            ev(3, ReaVerb::Affirm, 1.0),
        ];
        let s = fold_standing(subject(), &events);
        assert_eq!(
            classify(&s, None, &EpistemicThresholds::default()),
            EpistemicStatus::Reviewed
        );
    }

    /// Dismiss weight past the contest ratio flips the standing to `contested`, and the
    /// cite gate routes that to a `Refer(ContestedEvidence)` — the ceiling, not a silent Refuse.
    #[test]
    fn dismiss_weight_past_ratio_is_contested_and_gate_refers() {
        // affirm_weight = 2, dismiss_weight = 1 → dissent = 1/3 ≈ 0.333 ≥ 0.25.
        let events = [
            ev(1, ReaVerb::Affirm, 1.0),
            ev(2, ReaVerb::Affirm, 1.0),
            ev(3, ReaVerb::Dismiss, 1.0),
        ];
        let s = fold_standing(subject(), &events);
        let status = classify(&s, None, &EpistemicThresholds::default());
        assert_eq!(status, EpistemicStatus::Contested);

        let v = cite_gate(&s, &status);
        match v.decision {
            Decision::Refer(q) => assert_eq!(q.reason, ReferReason::ContestedEvidence),
            other => panic!("contested must route to Refer(ContestedEvidence), got {other:?}"),
        }
        assert_eq!(v.axis, "epistemic");
    }

    /// Canon is impossible without a `CanonizationRef`, no matter how strong the affirmations.
    /// The maximum a threshold alone can mint is `reviewed`.
    #[test]
    fn canon_impossible_without_canonization_ref() {
        let events: Vec<ReviewEvent> = (1..=10).map(|i| ev(i, ReaVerb::Affirm, 5.0)).collect();
        let s = fold_standing(subject(), &events);
        let status = classify(&s, None, &EpistemicThresholds::default());
        assert_eq!(status, EpistemicStatus::Reviewed);
        assert_ne!(status, EpistemicStatus::Canon);
    }

    /// A governance act CAN confer canon on an uncontested standing.
    #[test]
    fn canon_with_ref_not_contested_is_canon() {
        let events = [
            ev(1, ReaVerb::Affirm, 1.0),
            ev(2, ReaVerb::Affirm, 1.0),
            ev(3, ReaVerb::Affirm, 1.0),
        ];
        let s = fold_standing(subject(), &events);
        let cref = CanonizationRef { precedent: cid(99) };
        assert_eq!(
            classify(&s, Some(&cref), &EpistemicThresholds::default()),
            EpistemicStatus::Canon
        );
    }

    /// A governance act does NOT override an active contest: contested dominates canon.
    #[test]
    fn canon_with_ref_but_contested_stays_contested() {
        // affirm_weight = 1, dismiss_weight = 1 → dissent = 0.5 ≥ 0.25.
        let events = [ev(1, ReaVerb::Affirm, 1.0), ev(2, ReaVerb::Dismiss, 1.0)];
        let s = fold_standing(subject(), &events);
        let cref = CanonizationRef { precedent: cid(99) };
        assert_eq!(
            classify(&s, Some(&cref), &EpistemicThresholds::default()),
            EpistemicStatus::Contested
        );
    }

    /// The fold is order-invariant: any permutation of the same event set produces
    /// byte-identical standing (the determinism law — asserted down to the f64 bits).
    #[test]
    fn fold_is_order_invariant() {
        let a = [
            ev(3, ReaVerb::Affirm, 1.5),
            ev(1, ReaVerb::Dismiss, 2.5),
            ev(7, ReaVerb::Cite, 0.0),
            ev(2, ReaVerb::Affirm, 0.25),
            ev(5, ReaVerb::Dismiss, 0.125),
        ];
        let reference = fold_standing(subject(), &a);

        // Reversed order.
        let mut rev = a;
        rev.reverse();
        // A hand-scrambled order.
        let scrambled = [a[2], a[4], a[0], a[3], a[1]];

        for perm in [rev, scrambled] {
            let s = fold_standing(subject(), &perm);
            assert_eq!(s, reference);
            // Byte-identical weights, not merely PartialEq-close.
            assert_eq!(s.affirm_weight.to_bits(), reference.affirm_weight.to_bits());
            assert_eq!(
                s.dismiss_weight.to_bits(),
                reference.dismiss_weight.to_bits()
            );
        }
    }

    /// Positive-only feedback law. A standing with zero dismissals still classifies
    /// (the system never *requires* a negative to make progress), AND `Dismiss` is a
    /// representable signal that actually moves the fold — the law binds the system to
    /// allow negatives, so a negative must weigh.
    #[test]
    fn positive_only_feedback_law() {
        // Positive-only still classifies — no panic, a real status, zero dissent.
        let pos = fold_standing(subject(), &[ev(1, ReaVerb::Affirm, 1.0)]);
        let _ = classify(&pos, None, &EpistemicThresholds::default());
        assert_eq!(pos.dismiss_weight, 0.0);
        assert_eq!(pos.standing_ratio(), Some(1.0));

        // Dismiss is representable and moves the fold: standing ratio drops, dissent rises.
        let with_neg = fold_standing(
            subject(),
            &[ev(1, ReaVerb::Affirm, 1.0), ev(2, ReaVerb::Dismiss, 3.0)],
        );
        assert_eq!(with_neg.dismissals, 1);
        assert_eq!(with_neg.dismiss_weight, 3.0);
        assert!(
            with_neg.standing_ratio().unwrap() < pos.standing_ratio().unwrap(),
            "a Dismiss must lower the standing ratio (negatives weigh)"
        );
    }
}

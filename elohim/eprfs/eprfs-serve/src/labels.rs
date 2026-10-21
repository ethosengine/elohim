//! The `ReasonLabel` faces of eprfs-core's reasons (A10, I3).
//!
//! The orphan rule forbids implementing seam-contracts' trait on eprfs-core's types from here, so
//! each reason has a local fieldless face with a `From` conversion, and each face passes
//! `check_reason_labels`. The labels are the reasons' own wire tags.
//!
//! `ItemAbsence` and `DerivationUnanswered` are `#[non_exhaustive]`, so a crate outside eprfs-core
//! cannot match them exhaustively: their faces carry an `Unrecognized` label for a variant this
//! crate does not know yet, and the tests run every value of eprfs-core's `one_of_each()` through
//! the face, so a new variant fails the build's tests here until it has its own label.
//! `ReachReason` is exhaustive and its face has no such label.

use eprfs_core::{DerivationUnanswered, ItemAbsence, ReachReason};
use seam_contracts::ReasonLabel;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemAbsenceLabel {
    NotHeld,
    Refused,
    OverBudget,
    Unreachable,
    MembershipUnknown,
    Integrity,
    /// A variant newer than this crate; never produced by a known absence.
    Unrecognized,
}

impl ReasonLabel for ItemAbsenceLabel {
    const ALL: &'static [Self] = &[
        Self::NotHeld,
        Self::Refused,
        Self::OverBudget,
        Self::Unreachable,
        Self::MembershipUnknown,
        Self::Integrity,
        Self::Unrecognized,
    ];

    fn label(&self) -> &'static str {
        match self {
            Self::NotHeld => "not-held",
            Self::Refused => "refused",
            Self::OverBudget => "over-budget",
            Self::Unreachable => "unreachable",
            Self::MembershipUnknown => "membership-unknown",
            Self::Integrity => "integrity",
            Self::Unrecognized => "unrecognized",
        }
    }
}

impl From<&ItemAbsence> for ItemAbsenceLabel {
    fn from(absence: &ItemAbsence) -> Self {
        match absence {
            ItemAbsence::NotHeld { .. } => Self::NotHeld,
            ItemAbsence::Refused { .. } => Self::Refused,
            ItemAbsence::OverBudget { .. } => Self::OverBudget,
            ItemAbsence::Unreachable { .. } => Self::Unreachable,
            ItemAbsence::MembershipUnknown { .. } => Self::MembershipUnknown,
            ItemAbsence::Integrity { .. } => Self::Integrity,
            _ => Self::Unrecognized,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReachReasonLabel {
    OutsideStanding,
    NoRecord,
    NotUnderRoot,
    IdentityRequired,
    AuthorizerUnavailable,
}

impl ReasonLabel for ReachReasonLabel {
    const ALL: &'static [Self] = &[
        Self::OutsideStanding,
        Self::NoRecord,
        Self::NotUnderRoot,
        Self::IdentityRequired,
        Self::AuthorizerUnavailable,
    ];

    fn label(&self) -> &'static str {
        match self {
            Self::OutsideStanding => "outside-standing",
            Self::NoRecord => "no-record",
            Self::NotUnderRoot => "not-under-root",
            Self::IdentityRequired => "identity-required",
            Self::AuthorizerUnavailable => "authorizer-unavailable",
        }
    }
}

impl From<&ReachReason> for ReachReasonLabel {
    fn from(reason: &ReachReason) -> Self {
        match reason {
            ReachReason::OutsideStanding => Self::OutsideStanding,
            ReachReason::NoRecord => Self::NoRecord,
            ReachReason::NotUnderRoot => Self::NotUnderRoot,
            ReachReason::IdentityRequired => Self::IdentityRequired,
            ReachReason::AuthorizerUnavailable => Self::AuthorizerUnavailable,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DerivationUnansweredLabel {
    NotRecorded,
    IndexUnreachable,
    Refused,
    InputIncomplete,
    OutputNotFetchable,
    Mismatch,
    NotCarried,
    /// A variant newer than this crate.
    Unrecognized,
}

impl ReasonLabel for DerivationUnansweredLabel {
    const ALL: &'static [Self] = &[
        Self::NotRecorded,
        Self::IndexUnreachable,
        Self::Refused,
        Self::InputIncomplete,
        Self::OutputNotFetchable,
        Self::Mismatch,
        Self::NotCarried,
        Self::Unrecognized,
    ];

    fn label(&self) -> &'static str {
        match self {
            Self::NotRecorded => "not-recorded",
            Self::IndexUnreachable => "index-unreachable",
            Self::Refused => "refused",
            Self::InputIncomplete => "input-incomplete",
            Self::OutputNotFetchable => "output-not-fetchable",
            Self::Mismatch => "mismatch",
            Self::NotCarried => "not-carried",
            Self::Unrecognized => "unrecognized",
        }
    }
}

impl From<&DerivationUnanswered> for DerivationUnansweredLabel {
    fn from(reason: &DerivationUnanswered) -> Self {
        match reason {
            DerivationUnanswered::NotRecorded => Self::NotRecorded,
            DerivationUnanswered::IndexUnreachable => Self::IndexUnreachable,
            DerivationUnanswered::Refused { .. } => Self::Refused,
            DerivationUnanswered::InputIncomplete { .. } => Self::InputIncomplete,
            DerivationUnanswered::OutputNotFetchable => Self::OutputNotFetchable,
            DerivationUnanswered::Mismatch { .. } => Self::Mismatch,
            DerivationUnanswered::NotCarried => Self::NotCarried,
            _ => Self::Unrecognized,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use seam_contracts::{assert_reason_labels_conformant, check_reason_labels};

    #[test]
    fn every_face_is_reason_label_conformant() {
        assert!(check_reason_labels::<ItemAbsenceLabel>().is_ok());
        assert!(check_reason_labels::<ReachReasonLabel>().is_ok());
        assert!(check_reason_labels::<DerivationUnansweredLabel>().is_ok());
        assert_reason_labels_conformant::<ItemAbsenceLabel>();
    }

    #[test]
    fn every_known_item_absence_has_its_own_label_equal_to_its_wire_tag() {
        let all = ItemAbsence::one_of_each();
        let mut labels: Vec<&str> = Vec::new();
        for absence in &all {
            let face = ItemAbsenceLabel::from(absence);
            assert_ne!(
                face,
                ItemAbsenceLabel::Unrecognized,
                "{absence:?} has no label"
            );
            assert_eq!(face.label(), absence.kind());
            labels.push(face.label());
        }
        labels.sort();
        labels.dedup();
        assert_eq!(labels.len(), all.len(), "one label per variant");
        assert_eq!(all.len() + 1, ItemAbsenceLabel::ALL.len());
    }

    #[test]
    fn every_known_derivation_reason_has_its_own_label() {
        let all = DerivationUnanswered::one_of_each();
        for reason in &all {
            let face = DerivationUnansweredLabel::from(reason);
            assert_ne!(face, DerivationUnansweredLabel::Unrecognized);
            assert_eq!(face.label(), reason.kind());
        }
        assert_eq!(all.len() + 1, DerivationUnansweredLabel::ALL.len());
    }

    #[test]
    fn reach_reason_labels_are_the_wire_tags() {
        for (reason, label) in [
            (ReachReason::OutsideStanding, "outside-standing"),
            (ReachReason::NoRecord, "no-record"),
            (ReachReason::NotUnderRoot, "not-under-root"),
            (ReachReason::IdentityRequired, "identity-required"),
            (ReachReason::AuthorizerUnavailable, "authorizer-unavailable"),
        ] {
            assert_eq!(ReachReasonLabel::from(&reason).label(), label);
            assert_eq!(
                serde_json::to_value(reason).unwrap(),
                serde_json::json!(label)
            );
        }
    }
}

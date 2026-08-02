//! Property harnesses — the concern classes that have a *property* form rather
//! than a type form.
//!
//! Design: `genesis/docs/superpowers/plans/2026-08-02-seam-concern-contract-architecture-plan.md`
//! (Design surface 3).
//!
//! Behind the default-off `harness` feature so a runtime consumer never links
//! them; an implementor enables it in `[dev-dependencies]`. The feature stays in
//! *this* crate (one crate, one version) unless the P2 `Liveness` table needs
//! `proptest`/`arbitrary`, at which point — and only then — the plan authorizes
//! splitting a `-testkit` sibling. Splitting earlier would give implementors two
//! version numbers to keep in step for no gain.
//!
//! Every harness in this module is **self-bounded** (C6a): it declares a budget,
//! respects it, and reports whether its check was exhaustive or sampled. A
//! harness that silently degraded from exhaustive to sampled would be the
//! advertise/serve asymmetry (C7) it exists to catch, committed by the
//! instrument itself.
//!
//! | Module | Concern | Property |
//! |---|---|---|
//! | [`arbitrated`] | C2 monotonic authority | permutation-invariance + tiebreak determinism |
//! | [`quiescent`] | C6b idempotent effect | replay against settled state mints nothing |
//! | [`liveness`] | C3 liveness | every reachable non-terminal state has an automated move |
//!
//! [`liveness`] is a **table** harness rather than a property harness: the
//! caller enumerates the state space as plain data and the check is exhaustive
//! over exactly what was enumerated. That is deliberate — a randomized search
//! that misses the one dead corner reports green, and the corners of a decision
//! predicate are few and nameable. It needs no `proptest`/`arbitrary`, so the
//! plan's conditional `-testkit` split (Design surface 7) stays unexercised and
//! the crate stays a leaf.

pub mod arbitrated;
pub mod liveness;
pub mod quiescent;

pub use arbitrated::{
    assert_arbitration, check_arbitration, check_arbitration_with, ArbitrationBudget,
    ArbitrationReport, ArbitrationViolation, ArbitrationViolationClass,
};
pub use liveness::{
    assert_liveness, check_liveness, check_liveness_with, Agency, LivenessBudget, LivenessFailure,
    LivenessReport, LivenessViolation, LivenessViolationClass, ProgressSkip, StateClass,
    Transition,
};
pub use quiescent::{
    assert_quiescent, check_quiescence, check_quiescence_with, QuiescenceBudget, QuiescenceReport,
    QuiescenceViolation, QuiescenceViolationClass,
};

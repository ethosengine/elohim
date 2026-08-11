//! Pure folds over event history — resource STATE is derived, never stored
//! (P1: storage as reconciliation controller, applied to economics).

use std::collections::BTreeMap;

use cid::Cid;
use elohim_epr::algedonic::AlgedonicEvidence;
use elohim_epr::measure::{ClaimKind, Confidence, Interval, MeasureKind, Quantity};
use elohim_epr::witness::{Magnitude, ReaVerb};

use crate::model::{Bound, Commitment, FlowEvent};

/// Derived state of one resource: per-(verb, unit) totals over its event history.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ResourceState {
    pub event_count: u64,
    totals: BTreeMap<(String, String), f64>,
}

impl ResourceState {
    /// Total `Count` magnitude accumulated for `verb` in `unit` (0.0 if none).
    pub fn total(&self, verb: ReaVerb, unit: &str) -> f64 {
        self.totals
            .get(&(verb_key(&verb), unit.to_string()))
            .copied()
            .unwrap_or(0.0)
    }
}

fn verb_key(verb: &ReaVerb) -> String {
    format!("{verb:?}")
}

/// Fold the events touching `resource` into its derived state.
pub fn resource_state(resource: &Cid, events: &[FlowEvent]) -> ResourceState {
    let mut state = ResourceState::default();
    for event in events.iter().filter(|e| &e.resource == resource) {
        state.event_count += 1;
        if let Magnitude::Count { value, unit } = &event.quantity {
            *state
                .totals
                .entry((verb_key(&event.action), unit.clone()))
                .or_insert(0.0) += value;
        }
    }
    state
}

/// Plan-vs-actual for one commitment: which events discharge it, how far along — and whether
/// the stock flowing against it has crossed a bound the promise declared.
#[derive(Debug, Clone, PartialEq)]
pub struct FulfillmentStatus {
    pub commitment: Cid,
    pub event_count: u64,
    pub fulfilled_quantity: f64,
    pub expected_quantity: Option<f64>,
    /// Derived algedonic pain against [`Commitment::bound`]. `Some` only when the promise
    /// declared a ceiling AND the folded stock has reached one of its lines; `None` covers
    /// both honest silences — an unbounded promise, and a stock still inside the band.
    ///
    /// Like every other field here it is DERIVED, never stored: two peers folding the same
    /// events mint the same evidence.
    pub pain: Option<AlgedonicEvidence>,
}

impl FulfillmentStatus {
    /// fulfilled/expected in the commitment's declared unit; None when unbounded.
    pub fn ratio(&self) -> Option<f64> {
        match self.expected_quantity {
            Some(expected) if expected > 0.0 => Some(self.fulfilled_quantity / expected),
            _ => None,
        }
    }
}

/// The crate's SINGLE algedonic-evidence constructor: a bound plus the stock folded against it
/// yields exactly one evidence shape, or silence.
///
/// Because every `AlgedonicEvidence` minted here is chosen from the crossing rather than
/// assembled field-by-field, the extra-evidence-key edge — a `threshold_pct` riding on breach
/// evidence, which the wire coherence check in `elohim_epr::algedonic` refuses — stays
/// theoretical on this path. `bound_ref` is the bounding commitment's CID and nothing else.
fn bound_evidence(commitment_cid: &Cid, bound: &Bound, stock: f64) -> Option<AlgedonicEvidence> {
    let bound_ref = commitment_cid.to_string();
    if stock >= bound.limit {
        Some(AlgedonicEvidence::Breach {
            stock,
            limit: bound.limit,
            bound_ref,
        })
    } else if stock >= bound.band_edge() {
        Some(AlgedonicEvidence::Approach {
            stock,
            limit: bound.limit,
            bound_ref,
            threshold_pct: bound.threshold_pct,
        })
    } else {
        None
    }
}

/// Fold `events` against one commitment (identified by `commitment_cid`).
/// Only events whose `fulfills` names the commitment count; quantity sums `Count`
/// magnitudes matching the commitment's declared unit.
///
/// The same event filter carries the bound: `fulfills` is the edge the DHT spells
/// `bounded_by`, so the flows that discharge a promise ARE the flows that accumulate against
/// its ceiling — summed in the bound's own unit, which need not be the expected quantity's.
pub fn fulfillment(
    commitment_cid: &Cid,
    commitment: &Commitment,
    events: &[FlowEvent],
) -> FulfillmentStatus {
    let expected = commitment
        .resource_spec
        .quantity
        .as_ref()
        .and_then(|m| match m {
            Magnitude::Count { value, unit } => Some((unit.clone(), *value)),
            _ => None,
        });

    let mut event_count = 0;
    let mut fulfilled_quantity = 0.0;
    let mut bound_stock = 0.0;
    for event in events
        .iter()
        .filter(|e| e.fulfills.contains(commitment_cid))
    {
        event_count += 1;
        if let Some((unit, _)) = &expected {
            if let Some(value) = count_in_unit(&event.quantity, unit) {
                fulfilled_quantity += value;
            }
        }
        if let Some(bound) = &commitment.bound {
            if let Some(value) = count_in_unit(&event.quantity, &bound.unit) {
                bound_stock += value;
            }
        }
    }

    FulfillmentStatus {
        commitment: *commitment_cid,
        event_count,
        fulfilled_quantity,
        expected_quantity: expected.map(|(_, value)| value),
        pain: commitment
            .bound
            .as_ref()
            .and_then(|bound| bound_evidence(commitment_cid, bound, bound_stock)),
    }
}

/// Helper: the `Count` value of a magnitude when its unit matches, else None.
pub(crate) fn count_in_unit(magnitude: &Magnitude, unit: &str) -> Option<f64> {
    match magnitude {
        Magnitude::Count { value, unit: u } if u == unit => Some(*value),
        _ => None,
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum FoldError {
    #[error("cannot fold mixed measure kinds: {0:?} and {1:?}")]
    MixedKinds(MeasureKind, MeasureKind),
    #[error("cannot fold an empty set")]
    Empty,
}

/// Rank claim kinds weakest-last so a fold can take the minimum honestly.
fn claim_rank(c: ClaimKind) -> u8 {
    match c {
        ClaimKind::Witnessed => 0,
        ClaimKind::InstrumentMeasured => 1,
        ClaimKind::Estimated => 2,
        ClaimKind::Modelled => 3,
        ClaimKind::Imputed => 4,
    }
}

/// The closure law (L5): a fold over interval-carrying quantities RETURNS an
/// interval, and never claims to be better-known than its worst input.
///
/// Slice-1 interval arithmetic is deliberately naive — `[a,b]+[c,d]=[a+c,b+d]`
/// assumes perfect correlation and therefore OVER-widens for independent terms.
/// Over-widening never manufactures precision; a correlation-aware fold is
/// slice-2 work (spec open question Q1).
///
/// Determinism here is narrower than "commutative, therefore order-independent
/// by construction": `f64` addition is commutative but NOT associative, so a
/// multi-term sum is order-dependent in general. The reordering regression
/// test below passes because its fixture bounds are exactly representable in
/// binary floating point, not because summation order is provably irrelevant.
/// The weakest-claim reduction IS a genuine order-independent min — `claim_rank`
/// has no associativity concern. Cross-peer determinism at a band edge, where
/// two peers could fold the same terms in different orders and land on
/// different rounding, is explicitly out of scope for slice 1 and is named as
/// slice-2 work. Do not "fix" this with a canonicalizing sort here — sort order
/// is a slice-2 decision (event id? timestamp? CID?) this fold does not make.
pub fn with_uncertainty(items: &[Quantity]) -> Result<Quantity, FoldError> {
    let first = items.first().ok_or(FoldError::Empty)?;
    let kind = first.kind;
    for q in items {
        if q.kind != kind {
            return Err(FoldError::MixedKinds(kind, q.kind));
        }
    }
    let value = items.iter().map(|q| q.value).sum();
    let lo = items.iter().map(|q| q.confidence.interval.lo).sum();
    let hi = items.iter().map(|q| q.confidence.interval.hi).sum();
    let weakest = items
        .iter()
        .max_by_key(|q| claim_rank(q.confidence.claim))
        .map(|q| q.confidence.claim)
        .unwrap_or(ClaimKind::Imputed);
    Ok(Quantity {
        value,
        kind,
        confidence: Confidence {
            claim: weakest,
            interval: Interval::new(lo, hi),
            basis: format!("fold of {} terms", items.len()),
        },
    })
}

//! D.6 — adoption: self-adopted, idempotent, make-before-break (R3, R4).
//!
//! I9: only [`RealEvidence`], which only `Real`, present readings of the kind's fulfilment promise
//! inside the period construct, can settle. Its fields are private, so no struct literal outside
//! the crate compiles:
//!
//! ```compile_fail
//! use placement_core::RealEvidence;
//! let forged = RealEvidence { readings: Vec::new() };
//! ```

use cid::Cid;
use elohim_epr::Reach;
use elohim_epr_rea::AgentRef;
use seam_contracts::{EvidenceStage, ProbeAnswer, Reading};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::kind::{KindId, MishpatAction};
use crate::need::{Need, NeedKey};
use crate::offer::{CidString, EpochSecs};
use crate::params::PlaceError;
use crate::rank::MethodId;

/// R1: long-lived. Renewed by nothing periodic: it lapses only on an observed carrier loss or a
/// revokes-commitment (spec §7.2). `until` is set only when the need itself is bounded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lease {
    pub from: EpochSecs,
    pub until: Option<EpochSecs>,
}

/// One settlement per commitment per period.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Period {
    pub start: EpochSecs,
    pub len_secs: u64,
}

/// R2: below public, placement is household-local and not notarized. Derived, never chosen. The
/// plane names its owner: a carrier writes into the RECEIVER's household plane (D13).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    tag = "plane"
)]
pub enum RecordPlane {
    Notarized,
    HouseholdLocal { household: String },
}

impl RecordPlane {
    /// `openness < Public` → `HouseholdLocal`, naming the receiver's household, or the receiver's
    /// own `agent:<key>` domain when it belongs to none.
    pub fn for_need(need: &Need, receiver_household: Option<&str>) -> Self {
        if need.reach.openness() >= Reach::Public.openness() {
            RecordPlane::Notarized
        } else {
            RecordPlane::HouseholdLocal {
                household: receiver_household
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("agent:{}", need.receiver.0)),
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitmentDraft {
    pub id: String,
    pub need: CidString,
    pub kind: KindId,
    pub action: MishpatAction,
    /// Always the deciding peer: no peer assigns work to another.
    pub provider: AgentRef,
    pub receiver: AgentRef,
    pub key: NeedKey,
    pub in_scope_of: CidString,
    pub lease: Lease,
    pub plane: RecordPlane,
    pub method: MethodId,
    /// The set's CID.
    pub params: CidString,
    /// The set's `ranking_cid`.
    pub ranking: CidString,
    /// Spec §7.4: what remains to explain it.
    pub inputs_digest: [u8; 32],
    /// Make-before-break: the commitment this one succeeds.
    pub replaces: Option<String>,
}

fn hex16(digest: &[u8]) -> String {
    digest.iter().take(8).map(|b| format!("{b:02x}")).collect()
}

/// `CustodyBlob`: byte-identical to `rea_commitment_service::deterministic_custody_id(provider,
/// receiver, marker)` (`"custody-blob-"` + 16 hex). Others: `"<kind>-"` + hex16(sha256(kind |
/// provider | receiver | key)), the key in its string form.
pub fn commitment_id(
    kind: KindId,
    provider: &AgentRef,
    receiver: &AgentRef,
    key: &NeedKey,
) -> String {
    match (kind, key) {
        (KindId::CustodyBlob, NeedKey::Blob(marker)) => {
            let digest =
                Sha256::digest(format!("{}|{}|{marker}", provider.0, receiver.0).as_bytes());
            format!("custody-blob-{}", hex16(&digest))
        }
        _ => {
            let digest = Sha256::digest(
                format!(
                    "{}|{}|{}|{}",
                    kind.label(),
                    provider.0,
                    receiver.0,
                    key.as_id_string()
                )
                .as_bytes(),
            );
            format!("{}-{}", kind.label(), hex16(&digest))
        }
    }
}

/// An observed commitment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Carrying {
    pub commitment: String,
    /// `Need::id` (D10).
    pub need: Cid,
    pub key: NeedKey,
    pub kind: KindId,
    pub carrier: AgentRef,
    pub receiver: AgentRef,
    /// The scope it was adopted in (`CommitmentDraft.in_scope_of`): a withdrawal is decided only
    /// where that scope's needs were read (fix round 2).
    pub in_scope_of: Cid,
    pub lease: Lease,
    /// The `ranking_cid` it was adopted under.
    pub under_params: Cid,
    /// Claimed until a holding probe exists; `Unreachable` is not "not honored".
    pub honored: seam_contracts::Answer<bool>,
}

/// Only ever my own commitment (D18): another carrier's loss is a `Carrying` with honored `Absent`,
/// never a `Release`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub commitment: String,
    pub provider: AgentRef,
    pub reason: ReleaseReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReleaseReason {
    OverHeld,
    Superseded { by: String },
    NeedWithdrawn,
}

/// Evidence class is required (I9): only `RealEvidence` can settle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FulfilmentDraft {
    /// [`fulfilment_id`] of (commitment, period start): one settlement per commitment per period,
    /// so `Ports::settle` is idempotent by it and a re-delivered `PeriodClosed` settles nothing new.
    pub id: String,
    pub commitment: String,
    pub period: Period,
    pub provider: AgentRef,
    pub quantity: u64,
    pub unit: &'static str,
    pub evidence: RealEvidence,
}

/// The promise whose real, present reading is fulfilment evidence for a kind's commitment. Custody
/// settles on a holding reading; every other kind has no fulfilment promise named yet (the task
/// kind settles on its completion event, F11), so nothing settles it through a reading. Placeholder
/// vocabulary, open in spec §6 with the units (Codex review 2, finding 8).
pub fn fulfilment_promise(kind: KindId) -> Option<&'static str> {
    match kind {
        KindId::CustodyBlob | KindId::CustodyRoot => Some("holding-is-honored@1"),
        _ => None,
    }
}

/// Fulfilment evidence for one commitment in one period: the DAG-CBOR CIDs of `Real`, present
/// readings of the kind's fulfilment promise, about the commitment's own need key, observed inside
/// the period. `from_readings` is the only constructor and checks every reading at run time;
/// [`settle`] then refuses evidence for another commitment or period, and any quantity above what
/// the readings support. A real reading of another promise, of an absence, of an unreachable
/// carrier, or from another period settles nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RealEvidence {
    commitment: String,
    period: Period,
    promise: &'static str,
    subject: String,
    /// The least value any reading observed: the most a settlement may claim.
    supported: u64,
    readings: Vec<CidString>,
}

impl RealEvidence {
    /// Revises the spec's `from_readings(readings)` (third review; Codex review 2): the evidence is
    /// bound to `carrying`'s commitment and to `period`. Each reading must be `EvidenceStage::Real`,
    /// of [`fulfilment_promise`]`(carrying.kind)`, about `carrying.key.as_id_string()`, with a
    /// `Present` answer, and observed (`observed_at`, RFC 3339) at or after `period.start` and before
    /// its end. The present values bound the quantity a settlement may claim.
    pub fn from_readings<T: Serialize + Copy + Into<u64>>(
        carrying: &Carrying,
        period: Period,
        readings: &[Reading<T>],
    ) -> Result<Self, PlaceError> {
        let promise = fulfilment_promise(carrying.kind).ok_or_else(|| {
            PlaceError(format!(
                "no fulfilment promise is named for {}",
                carrying.kind.label()
            ))
        })?;
        if readings.is_empty() {
            return Err(PlaceError("settlement needs at least one reading".into()));
        }
        let subject = carrying.key.as_id_string();
        let end = period.start.saturating_add(period.len_secs);
        let mut supported = u64::MAX;
        let mut cids = Vec::with_capacity(readings.len());
        for r in readings {
            if r.evidence() != EvidenceStage::Real {
                return Err(PlaceError(format!(
                    "a {:?} reading of {} cannot settle",
                    r.evidence(),
                    r.promise()
                )));
            }
            if r.promise() != promise {
                return Err(PlaceError(format!(
                    "a reading of {:?} is not fulfilment evidence ({promise:?} is)",
                    r.promise()
                )));
            }
            if r.subject() != subject {
                return Err(PlaceError(format!(
                    "a reading of {:?} is not evidence about {subject:?}",
                    r.subject()
                )));
            }
            let ProbeAnswer::Present(value) = r.answer() else {
                return Err(PlaceError(format!(
                    "a {:?} answer proves no fulfilment",
                    r.answer().state()
                )));
            };
            let at = rfc3339_epoch(r.observed_at()).ok_or_else(|| {
                PlaceError(format!("observed_at {:?} is not RFC 3339", r.observed_at()))
            })?;
            if at < period.start || at >= end {
                return Err(PlaceError(format!(
                    "a reading observed at {at} lies outside the period [{}, {end})",
                    period.start
                )));
            }
            supported = supported.min((*value).into());
            let bytes = serde_ipld_dagcbor::to_vec(r)
                .map_err(|e| PlaceError(format!("reading encoding: {e}")))?;
            cids.push(CidString::from(&elohim_epr::cid::compute_cid(&bytes)));
        }
        Ok(Self {
            commitment: carrying.commitment.clone(),
            period,
            promise,
            subject,
            supported,
            readings: cids,
        })
    }
    pub fn readings(&self) -> &[CidString] {
        &self.readings
    }
    pub fn subject(&self) -> &str {
        &self.subject
    }
    pub fn commitment(&self) -> &str {
        &self.commitment
    }
    pub fn period(&self) -> Period {
        self.period
    }
    pub fn promise(&self) -> &'static str {
        self.promise
    }
    pub fn supported(&self) -> u64 {
        self.supported
    }
}

/// Seconds since the epoch of an RFC 3339 timestamp (`YYYY-MM-DDTHH:MM:SS[.frac](Z|±HH:MM)`), by
/// arithmetic alone: R-P1 admits no time crate and R-P2 no clock. `None` for anything else, or a
/// time before the epoch. Fractional seconds are truncated.
fn rfc3339_epoch(s: &str) -> Option<EpochSecs> {
    let b = s.as_bytes();
    let num = |from: usize, len: usize| -> Option<i64> {
        let part = b.get(from..from + len)?;
        part.iter().try_fold(0i64, |n, d| {
            d.is_ascii_digit().then(|| n * 10 + i64::from(d - b'0'))
        })
    };
    if b.len() < 20
        || b[4] != b'-'
        || b[7] != b'-'
        || !matches!(b[10], b'T' | b't')
        || b[13] != b':'
        || b[16] != b':'
    {
        return None;
    }
    let (y, mo, d) = (num(0, 4)?, num(5, 2)?, num(8, 2)?);
    let (h, mi, sec) = (num(11, 2)?, num(14, 2)?, num(17, 2)?);
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let days_in = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if !(1..=12).contains(&mo)
        || d < 1
        || d > days_in[(mo - 1) as usize]
        || h > 23
        || mi > 59
        || sec > 60
    {
        return None;
    }
    let mut rest = &b[19..];
    if rest.first() == Some(&b'.') {
        let digits = rest[1..].iter().take_while(|c| c.is_ascii_digit()).count();
        if digits == 0 {
            return None;
        }
        rest = &rest[1 + digits..];
    }
    let offset = match rest {
        [b'Z' | b'z'] => 0,
        [sign @ (b'+' | b'-'), h1, h2, b':', m1, m2] => {
            let two = |a: u8, c: u8| -> Option<i64> {
                (a.is_ascii_digit() && c.is_ascii_digit())
                    .then(|| i64::from(a - b'0') * 10 + i64::from(c - b'0'))
            };
            let (oh, om) = (two(*h1, *h2)?, two(*m1, *m2)?);
            if oh > 23 || om > 59 {
                return None;
            }
            let o = oh * 3600 + om * 60;
            if *sign == b'+' {
                o
            } else {
                -o
            }
        }
        _ => return None,
    };
    // Days from the civil date (Howard Hinnant's algorithm).
    let yy = if mo <= 2 { y - 1 } else { y };
    let era = yy.div_euclid(400);
    let yoe = yy - era * 400;
    let mp = (mo + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    let secs = days * 86_400 + h * 3600 + mi * 60 + sec - offset;
    u64::try_from(secs).ok()
}

/// One settlement per commitment per period: `"fulfilment-"` + hex16(sha256(commitment | period
/// start)). Deterministic, so every re-delivery of one `PeriodClosed` drafts the same id.
pub fn fulfilment_id(commitment: &str, period: Period) -> String {
    let digest = Sha256::digest(format!("{commitment}|{}", period.start).as_bytes());
    format!("fulfilment-{}", hex16(&digest))
}

/// The unit a settlement counts in, by kind. The spec leaves units open; these are placeholders.
fn unit_of(kind: KindId) -> &'static str {
    match kind {
        KindId::CustodyBlob | KindId::CustodyRoot => "byte-period",
        KindId::Task => "task",
        _ => "period",
    }
}

/// The CARRIER's record of what it carried, once per commitment per period (spec §8 gap 1, R1).
/// Refuses evidence built for another commitment or another period, and a quantity above what the
/// evidence's readings support. Returns a `Result`,
/// revising the spec's infallible `settle` (third review).
pub fn settle(
    c: &Carrying,
    period: Period,
    quantity: u64,
    evidence: RealEvidence,
) -> Result<FulfilmentDraft, PlaceError> {
    if evidence.commitment != c.commitment || evidence.subject != c.key.as_id_string() {
        return Err(PlaceError(format!(
            "evidence for {} about {:?} cannot settle {}",
            evidence.commitment, evidence.subject, c.commitment
        )));
    }
    if evidence.period != period {
        return Err(PlaceError(format!(
            "evidence for the period from {} cannot settle the period from {}",
            evidence.period.start, period.start
        )));
    }
    if quantity > evidence.supported {
        return Err(PlaceError(format!(
            "a quantity of {quantity} exceeds the {} the readings support",
            evidence.supported
        )));
    }
    Ok(FulfilmentDraft {
        id: fulfilment_id(&c.commitment, period),
        commitment: c.commitment.clone(),
        period,
        provider: c.carrier.clone(),
        quantity,
        unit: unit_of(c.kind),
        evidence,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use seam_contracts::{Disclosure, ProbeAnswer};

    #[test]
    fn custody_blob_id_is_byte_identical_to_storage() {
        // rea_commitment_service::deterministic_custody_id, restated as its oracle.
        let reference = |provider: &str, receiver: &str, marker: &str| {
            let digest = Sha256::digest(format!("{provider}|{receiver}|{marker}").as_bytes());
            let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
            format!("custody-blob-{}", &hex[..16])
        };
        let id = commitment_id(
            KindId::CustodyBlob,
            &AgentRef("uhCAk-p".into()),
            &AgentRef("uhCAk-r".into()),
            &NeedKey::Blob("sha256-ab".into()),
        );
        assert_eq!(id, reference("uhCAk-p", "uhCAk-r", "sha256-ab"));
        assert_eq!(id.len(), "custody-blob-".len() + 16);
    }

    #[test]
    fn other_kinds_prefix_their_label() {
        let id = commitment_id(
            KindId::CustodyRoot,
            &AgentRef("p".into()),
            &AgentRef("r".into()),
            &NeedKey::TreeView(elohim_epr::cid::compute_cid(b"t")),
        );
        assert!(
            id.starts_with("custody-root-") && id.len() == "custody-root-".len() + 16,
            "{id}"
        );
    }

    const PERIOD: Period = Period {
        start: 86_400,
        len_secs: 86_400,
    };
    /// 1970-01-02T12:00:00Z, inside `PERIOD`.
    const INSIDE: &str = "1970-01-02T12:00:00Z";

    fn read(
        promise: &str,
        subject: &str,
        answer: ProbeAnswer<u64>,
        evidence: EvidenceStage,
        at: &str,
    ) -> Reading<u64> {
        Reading::new(
            promise.into(),
            "@concern:holding".into(),
            subject.into(),
            answer,
            evidence,
            Disclosure::Holder,
            at.into(),
        )
    }

    fn holding(subject: &str, value: u64) -> Reading<u64> {
        read(
            "holding-is-honored@1",
            subject,
            ProbeAnswer::present(value),
            EvidenceStage::Real,
            INSIDE,
        )
    }

    fn carrying(marker: &str) -> Carrying {
        let (p, r) = (AgentRef("uhCAk-p".into()), AgentRef("uhCAk-r".into()));
        let key = NeedKey::Blob(marker.into());
        Carrying {
            commitment: commitment_id(KindId::CustodyBlob, &p, &r, &key),
            need: elohim_epr::cid::compute_cid(b"need"),
            key,
            kind: KindId::CustodyBlob,
            carrier: p,
            receiver: r,
            in_scope_of: elohim_epr::cid::compute_cid(b"space"),
            lease: Lease {
                from: 0,
                until: None,
            },
            under_params: elohim_epr::cid::compute_cid(b"params"),
            honored: seam_contracts::Answer::Present(true),
        }
    }

    #[test]
    fn only_real_present_readings_of_the_fulfilment_promise_settle() {
        let held = carrying("sha256-aa");
        let from = |r: &[Reading<u64>]| RealEvidence::from_readings(&held, PERIOD, r);
        assert!(from(&[]).is_err());
        let ok = from(&[holding("sha256-aa", 1)]).unwrap();
        assert_eq!(ok.readings().len(), 1);
        assert!(ok.readings()[0].as_str().starts_with("bafyrei"));
        let refused = [
            // Not real.
            read(
                "holding-is-honored@1",
                "sha256-aa",
                ProbeAnswer::present(1),
                EvidenceStage::Fixture,
                INSIDE,
            ),
            // Real, but an observed absence.
            read(
                "holding-is-honored@1",
                "sha256-aa",
                ProbeAnswer::absent(),
                EvidenceStage::Real,
                INSIDE,
            ),
            // Real, but nothing answered.
            read(
                "holding-is-honored@1",
                "sha256-aa",
                ProbeAnswer::unreachable(seam_contracts::AnswerReason::Timeout).unwrap(),
                EvidenceStage::Real,
                INSIDE,
            ),
            // Real and present, but another promise about the same subject.
            read(
                "bytes-layer-reads-only-what-is-read@1",
                "sha256-aa",
                ProbeAnswer::present(1),
                EvidenceStage::Real,
                INSIDE,
            ),
            // Another subject.
            holding("sha256-bb", 1),
            // Before the period, at its end, and long after.
            read(
                "holding-is-honored@1",
                "sha256-aa",
                ProbeAnswer::present(1),
                EvidenceStage::Real,
                "1970-01-01T23:59:59Z",
            ),
            read(
                "holding-is-honored@1",
                "sha256-aa",
                ProbeAnswer::present(1),
                EvidenceStage::Real,
                "1970-01-03T00:00:00Z",
            ),
            read(
                "holding-is-honored@1",
                "sha256-aa",
                ProbeAnswer::present(1),
                EvidenceStage::Real,
                "2026-10-20T00:00:00Z",
            ),
            // Not a timestamp.
            read(
                "holding-is-honored@1",
                "sha256-aa",
                ProbeAnswer::present(1),
                EvidenceStage::Real,
                "yesterday",
            ),
        ];
        for r in &refused {
            assert!(from(std::slice::from_ref(r)).is_err(), "{r:?}");
            // One bad reading spoils the set.
            assert!(
                from(&[holding("sha256-aa", 1), r.clone()]).is_err(),
                "{r:?}"
            );
        }
        // A kind with no fulfilment promise settles through no reading.
        let mut task = held.clone();
        task.kind = KindId::Task;
        assert!(RealEvidence::from_readings(&task, PERIOD, &[holding("sha256-aa", 1)]).is_err());
    }

    #[test]
    fn evidence_settles_only_its_own_commitment_period_and_quantity() {
        let held = carrying("sha256-aa");
        let other = carrying("sha256-bb");
        let of_aa = RealEvidence::from_readings(&held, PERIOD, &[holding("sha256-aa", 7)]).unwrap();
        let of_bb =
            RealEvidence::from_readings(&other, PERIOD, &[holding("sha256-bb", 7)]).unwrap();
        assert!(
            settle(&held, PERIOD, 1, of_bb).is_err(),
            "evidence for another commitment never settles this one"
        );
        let a = settle(&held, PERIOD, 7, of_aa.clone()).unwrap();
        let again = settle(&held, PERIOD, 1, of_aa.clone()).unwrap();
        assert_eq!(a.id, again.id, "one id per commitment per period");
        assert_eq!(a.id, fulfilment_id(&held.commitment, PERIOD));
        assert!(
            settle(&held, PERIOD, 8, of_aa.clone()).is_err(),
            "no quantity above what the readings support"
        );
        let next = Period {
            start: 2 * 86_400,
            ..PERIOD
        };
        assert!(
            settle(&held, next, 1, of_aa).is_err(),
            "one period's evidence never settles the next"
        );
        let in_next = RealEvidence::from_readings(
            &held,
            next,
            &[read(
                "holding-is-honored@1",
                "sha256-aa",
                ProbeAnswer::present(3),
                EvidenceStage::Real,
                "1970-01-03T00:00:00.5+00:00",
            )],
        )
        .unwrap();
        let n = settle(&held, next, 3, in_next).unwrap();
        assert_ne!(a.id, n.id);
        // The least reading bounds the quantity.
        let two = RealEvidence::from_readings(
            &held,
            PERIOD,
            &[holding("sha256-aa", 9), holding("sha256-aa", 4)],
        )
        .unwrap();
        assert_eq!(two.supported(), 4);
    }

    #[test]
    fn rfc3339_reads_by_arithmetic() {
        assert_eq!(rfc3339_epoch("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(rfc3339_epoch("2026-10-20T00:00:00Z"), Some(1_792_454_400));
        assert_eq!(
            rfc3339_epoch("2026-10-20T02:00:00+02:00"),
            Some(1_792_454_400)
        );
        assert_eq!(
            rfc3339_epoch("2026-10-19T19:30:00-04:30"),
            Some(1_792_454_400)
        );
        assert_eq!(
            rfc3339_epoch("2024-02-29T00:00:00.123z"),
            Some(1_709_164_800)
        );
        for bad in [
            "",
            "2026-10-20",
            "2026-10-20 00:00:00Z",
            "2026-13-01T00:00:00Z",
            "2025-02-29T00:00:00Z",
            "2026-10-20T24:00:00Z",
            "2026-10-20T00:00:00",
            "2026-10-20T00:00:00.Z",
            "1969-12-31T23:59:59Z",
            "2026-10-20T00:00:00+2:00",
        ] {
            assert_eq!(rfc3339_epoch(bad), None, "{bad:?}");
        }
    }
}

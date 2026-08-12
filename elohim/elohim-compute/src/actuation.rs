//! The canonical actuation contract — the one `trait Governor` the whole
//! control plane shares.
//!
//! Lifted (2026-06-14, operator-blessed) from the proven, pure, unit-tested
//! arc-actuation spine (`elohim-storage::services::arc_actuator`) into the
//! shared crate so EVERY actuation face — conductor arc, blob custody, served
//! head, self-limit, data capability, an AI covenant — is the SAME governed,
//! refuse-and-elevate decision, never a clone. Graduated design rungs:
//!   - `2026-06-14-escalated-architecture-design` (the one control-plane spine)
//!   - `2026-06-14-elohim-sdk-design` (the contract + `limit_owner`)
//!   - `2026-06-14-recursive-architecture-design` (`RefusalCode::ReservedPlace` — the unbuilt place)
//!   - `2026-06-14-design-oracle-design` (the elevate sink consumes `Refusal`)
//!
//! ## The capture-resistance invariant
//! Every [`Refusal`] names **whose line it honored** via [`LimitOwner`]. A
//! system actuating on a person's behalf can never be mistaken for an operator
//! overriding them — the single most important capture-resistance property in
//! the design. `Operator` and `SelfLimit` are different refusals even when the
//! `code` is identical, so a consumer can never collapse the person's sovereign
//! limit into an operator veto.

use serde::{Deserialize, Serialize};

/// Whose line a [`Refusal`] honored. The agency gradient carried in one field:
/// a refusal on the PERSON's own limit (`SelfLimit`) is categorically different
/// from one the operator imposed (`Operator`) — and `Faith` marks the
/// deliberately-unbuilt place no actuation may fill (see
/// [`RefusalCode::ReservedPlace`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LimitOwner {
    /// The person's own declared limit. Sovereign — never operator-overridable.
    #[serde(rename = "self")]
    SelfLimit,
    /// A bounded, revocable grant (a `Mishpat::Commitment`).
    Commitment,
    /// The operator / runtime default.
    Operator,
    /// The reserved, unbuilt place — left empty for the faith no architecture
    /// may crowd out. The only `LimitOwner` that is not a lever.
    Faith,
}

/// Why a [`Governor`] refused — a small machine `code` the elevate sink keys
/// on, paired (in [`Refusal`]) with a human `elevate` message and the owner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RefusalCode {
    /// The request is outside the authorizing grant's bounds.
    OutOfGrantBounds,
    /// The authorizing grant has expired.
    GrantExpired,
    /// The request is not an actuatable value at all.
    NotActuatable,
    /// An operational invariant (coverage, capacity, no-overwhelm, a self-limit)
    /// refused it — the short reason is the carried string; the full human
    /// message is the [`Refusal::elevate`] field.
    GateRefused(String),
    /// The request would fill the deliberately-unbuilt place. Never permitted;
    /// the invariant HOLDING is the success, not a gap to close.
    ReservedPlace,
}

/// A refusal carrying a finding to ELEVATE. The runtime form of "do not
/// actuate into harm" — and the exact payload the self-healing elevate sink
/// consumes (deliberately small). Always names [`LimitOwner`], so a refusal can
/// never be read as an operator override of a person's own line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Refusal {
    pub code: RefusalCode,
    pub limit_owner: LimitOwner,
    pub elevate: String,
}

impl Refusal {
    /// A refusal with an explicit code + owner.
    pub fn new(code: RefusalCode, limit_owner: LimitOwner, elevate: impl Into<String>) -> Self {
        Refusal {
            code,
            limit_owner,
            elevate: elevate.into(),
        }
    }

    /// An operational-gate refusal on a named owner's line.
    pub fn gate(
        limit_owner: LimitOwner,
        reason: impl Into<String>,
        elevate: impl Into<String>,
    ) -> Self {
        Refusal {
            code: RefusalCode::GateRefused(reason.into()),
            limit_owner,
            elevate: elevate.into(),
        }
    }

    /// A refusal to fill the unbuilt place. Always `Faith` — by construction
    /// there is no other owner of the reserved place.
    pub fn reserved_place(elevate: impl Into<String>) -> Self {
        Refusal {
            code: RefusalCode::ReservedPlace,
            limit_owner: LimitOwner::Faith,
            elevate: elevate.into(),
        }
    }

    /// True iff this refusal honors the PERSON's own sovereign limit — which a
    /// consumer must never override (the capture-resistance check).
    pub fn is_self_sovereign(&self) -> bool {
        self.limit_owner == LimitOwner::SelfLimit
    }
}

/// The one actuation contract. Every face (arc, custody, head, self-limit,
/// capability, covenant) implements it; all refuse with the SAME [`Refusal`],
/// so there is one refuse-and-elevate spine across the whole control plane.
///
/// The associated types let each face carry its own request/grant/context/effect
/// shapes while sharing the decision spine and the refusal vocabulary.
pub trait Governor {
    /// The proposed change.
    type Request;
    /// The bounds of the authorizing grant.
    type Grant;
    /// The live operational context the gate reads (coverage, capacity, …).
    type Context;
    /// The validated effect to apply.
    type Effect;

    /// Authorization stage: is the request within the grant and actuatable?
    fn authorize(
        &self,
        req: &Self::Request,
        grant: &Self::Grant,
        now_epoch_s: u64,
    ) -> Result<(), Refusal>;

    /// Operational gate: does the live invariant admit it? Refuse-and-elevate,
    /// never silently degrade or open a gap.
    fn gate(&self, req: &Self::Request, ctx: &Self::Context) -> Result<(), Refusal>;

    /// Render the validated effect. Pure (no I/O); the caller applies it.
    fn render(&self, req: &Self::Request, ctx: &Self::Context) -> Result<Self::Effect, Refusal>;

    /// The whole decision, composed: authorize → gate → render, short-circuiting
    /// to the first [`Refusal`]. This is the spine every face shares; override
    /// only with cause.
    fn decide(
        &self,
        req: &Self::Request,
        grant: &Self::Grant,
        ctx: &Self::Context,
        now_epoch_s: u64,
    ) -> Result<Self::Effect, Refusal> {
        self.authorize(req, grant, now_epoch_s)?;
        self.gate(req, ctx)?;
        self.render(req, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A toy face that proves the contract composes, short-circuits, and carries
    /// the owner — the shape `arc_actuator` (and every future face) will take.
    struct DemoGovernor {
        /// gate admits only when `ctx.headroom >= req.cost`
        owner: LimitOwner,
    }
    struct Req {
        value: u32,
        cost: u32,
    }
    struct Grant {
        max: u32,
        expires_at: Option<u64>,
    }
    struct Ctx {
        headroom: u32,
    }

    impl Governor for DemoGovernor {
        type Request = Req;
        type Grant = Grant;
        type Context = Ctx;
        type Effect = u32;

        fn authorize(&self, req: &Req, grant: &Grant, now: u64) -> Result<(), Refusal> {
            if let Some(exp) = grant.expires_at {
                if now >= exp {
                    return Err(Refusal::new(
                        RefusalCode::GrantExpired,
                        self.owner,
                        "grant expired",
                    ));
                }
            }
            if req.value > grant.max {
                return Err(Refusal::new(
                    RefusalCode::OutOfGrantBounds,
                    self.owner,
                    format!("value {} > max {}", req.value, grant.max),
                ));
            }
            Ok(())
        }

        fn gate(&self, req: &Req, ctx: &Ctx) -> Result<(), Refusal> {
            if ctx.headroom >= req.cost {
                Ok(())
            } else {
                Err(Refusal::gate(
                    self.owner,
                    "insufficient-headroom",
                    format!("cost {} exceeds headroom {}", req.cost, ctx.headroom),
                ))
            }
        }

        fn render(&self, req: &Req, _ctx: &Ctx) -> Result<u32, Refusal> {
            Ok(req.value)
        }
    }

    #[test]
    fn decide_composes_and_renders_when_admitted() {
        let g = DemoGovernor {
            owner: LimitOwner::Commitment,
        };
        let effect = g
            .decide(
                &Req { value: 5, cost: 2 },
                &Grant {
                    max: 10,
                    expires_at: Some(2_000),
                },
                &Ctx { headroom: 4 },
                1_000,
            )
            .unwrap();
        assert_eq!(effect, 5);
    }

    #[test]
    fn decide_short_circuits_at_authorize_and_names_owner() {
        let g = DemoGovernor {
            owner: LimitOwner::Operator,
        };
        // value over grant max → OutOfGrantBounds at the authorize stage; gate
        // (which would also fail on headroom) is never reached.
        let r = g
            .decide(
                &Req {
                    value: 99,
                    cost: 999,
                },
                &Grant {
                    max: 10,
                    expires_at: None,
                },
                &Ctx { headroom: 0 },
                1_000,
            )
            .unwrap_err();
        assert_eq!(r.code, RefusalCode::OutOfGrantBounds);
        assert_eq!(r.limit_owner, LimitOwner::Operator);
    }

    #[test]
    fn gate_refusal_carries_reason_and_owner() {
        let g = DemoGovernor {
            owner: LimitOwner::SelfLimit,
        };
        let r = g
            .gate(&Req { value: 1, cost: 9 }, &Ctx { headroom: 2 })
            .unwrap_err();
        assert!(matches!(r.code, RefusalCode::GateRefused(ref s) if s == "insufficient-headroom"));
        // The capture-resistance invariant: a refusal on the person's own line
        // is sovereign and must never be read as an operator override.
        assert!(r.is_self_sovereign());
    }

    #[test]
    fn the_same_code_under_different_owners_is_a_different_refusal() {
        let person = Refusal::gate(LimitOwner::SelfLimit, "limit", "you set this line");
        let operator = Refusal::gate(LimitOwner::Operator, "limit", "the runtime set this");
        assert_ne!(person, operator);
        assert!(person.is_self_sovereign());
        assert!(!operator.is_self_sovereign());
    }

    #[test]
    fn reserved_place_is_always_faith_and_never_a_lever() {
        let r = Refusal::reserved_place(
            "the center is left empty for the faith no architecture may crowd out",
        );
        assert_eq!(r.code, RefusalCode::ReservedPlace);
        assert_eq!(r.limit_owner, LimitOwner::Faith);
        // Faith is the only owner that is not a lever — the unbuilt place holds.
        assert!(!r.is_self_sovereign());
    }

    #[test]
    fn limit_owner_serializes_self_as_kebab_self() {
        // The wire form of the agency gradient: `self` (not `self-limit`), so
        // the gradient reads cleanly across the ts-rs boundary and in findings.
        let json = serde_json::to_string(&LimitOwner::SelfLimit).unwrap();
        assert_eq!(json, "\"self\"");
        assert_eq!(
            serde_json::to_string(&LimitOwner::Faith).unwrap(),
            "\"faith\""
        );
    }
}

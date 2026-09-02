//! Verdicts and the restart governor.

use elohim_compute::{Governor, LimitOwner, Refusal, RefusalCode};
use serde::{Deserialize, Serialize};

use crate::{
    manifest::{Backoff, ChildPolicy, Restart},
    tally::{same_cause_key, DeathRecord, DeathTally},
};

/// The pure result of applying a restart policy to one child death.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RestartVerdict {
    /// Restart after the computed delay.
    Restart { after_s: u64, attempt: u32 },
    /// Permanently stop retrying this child.
    GiveUp { reason: GiveUpReason },
    /// Stop cleanly without attempting a restart.
    Stop,
}

/// The machine-readable reason a restart policy gave up.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GiveUpReason {
    /// The configured number of consecutive identical causes was reached.
    SameCause { key: String, count: u32 },
    /// The configured sliding-window death intensity was exceeded.
    IntensityExceeded { deaths: u32, window_s: u64 },
    /// The manifest declares a non-restarting temporary child.
    PolicyTemporary,
}

/// Request to consider restarting one process after its observed death.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct RestartRequest {
    /// Process name within the runtime manifest.
    pub process: String,
    /// Death for which restart is being considered.
    pub death: DeathRecord,
}

/// Policy and authority bounding a restart decision.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct RestartGrant {
    /// Authority whose line bounds this grant.
    pub bounded_by: BoundedBy,
    /// Manifest child policy being granted.
    pub policy: ChildPolicy,
}

/// Authority bounding a restart grant.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BoundedBy {
    /// The local runtime manifest and operator defaults.
    ManifestPolicy,
    /// A bounded commitment, introduced in S1.
    Commitment { cid: String },
}

/// Live pure inputs consulted by the restart governor.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(default, rename_all = "snake_case")]
pub struct RestartContext {
    /// Wall-clock time at which the decision is made.
    pub now_epoch_s: u64,
    /// Persisted tally, which MUST already contain [`RestartRequest::death`].
    /// The supervisor records the death before asking the governor to decide.
    pub tally: DeathTally,
}

/// Pure restart policy governor.
#[derive(Clone, Copy, Debug, Default)]
pub struct RestartGovernor;

impl Governor for RestartGovernor {
    type Request = RestartRequest;
    type Grant = RestartGrant;
    type Context = RestartContext;
    type Effect = RestartVerdict;

    fn authorize(
        &self,
        req: &Self::Request,
        grant: &Self::Grant,
        _now_epoch_s: u64,
    ) -> Result<(), Refusal> {
        if grant.policy.restart == Restart::Temporary {
            return Err(Refusal::gate(
                limit_owner(&grant.bounded_by),
                "policy-temporary",
                format!(
                    "restart of {} refused by temporary child policy",
                    req.process
                ),
            ));
        }
        Ok(())
    }

    /// Evaluates the manifest-default policy under [`LimitOwner::Operator`]
    /// because the trait signature carries no grant. Call [`Governor::decide`]
    /// or [`RestartGovernor::verdict`] for a real decision.
    fn gate(&self, req: &Self::Request, ctx: &Self::Context) -> Result<(), Refusal> {
        gate_with_policy(req, &ChildPolicy::default(), LimitOwner::Operator, ctx)
    }

    /// Renders the manifest-default policy because the trait signature carries
    /// no grant. Call [`Governor::decide`] or [`RestartGovernor::verdict`] for a
    /// real decision.
    fn render(&self, req: &Self::Request, ctx: &Self::Context) -> Result<Self::Effect, Refusal> {
        Ok(render_with_policy(req, &ChildPolicy::default(), ctx))
    }

    fn decide(
        &self,
        req: &Self::Request,
        grant: &Self::Grant,
        ctx: &Self::Context,
        now_epoch_s: u64,
    ) -> Result<Self::Effect, Refusal> {
        self.authorize(req, grant, now_epoch_s)?;
        gate_with_policy(req, &grant.policy, limit_owner(&grant.bounded_by), ctx)?;
        Ok(render_with_policy(req, &grant.policy, ctx))
    }
}

impl RestartGovernor {
    /// The whole decision as a verdict; a refusal becomes a matching give-up.
    pub fn verdict(
        &self,
        req: &RestartRequest,
        grant: &RestartGrant,
        ctx: &RestartContext,
    ) -> (RestartVerdict, Option<Refusal>) {
        match self.decide(req, grant, ctx, ctx.now_epoch_s) {
            Ok(verdict) => (verdict, None),
            Err(refusal) => {
                let reason = give_up_reason(req, grant, ctx, &refusal);
                (RestartVerdict::GiveUp { reason }, Some(refusal))
            }
        }
    }
}

fn limit_owner(bounded_by: &BoundedBy) -> LimitOwner {
    match bounded_by {
        BoundedBy::ManifestPolicy => LimitOwner::Operator,
        BoundedBy::Commitment { .. } => LimitOwner::Commitment,
    }
}

fn gate_with_policy(
    req: &RestartRequest,
    policy: &ChildPolicy,
    owner: LimitOwner,
    ctx: &RestartContext,
) -> Result<(), Refusal> {
    let tally = &ctx.tally;
    let same_cause_count = tally.same_cause_run();
    if same_cause_count >= policy.same_cause_limit {
        let key = tally
            .deaths
            .last()
            .map(same_cause_key)
            .unwrap_or_else(|| same_cause_key(&req.death));
        return Err(Refusal::gate(
            owner,
            "same-cause",
            format!(
                "restart of {} refused: cause {key} repeated {same_cause_count} times",
                req.process
            ),
        ));
    }

    let deaths = tally.deaths_within(ctx.now_epoch_s, policy.intensity.window_s);
    if deaths > policy.intensity.max_deaths {
        return Err(Refusal::gate(
            owner,
            "intensity",
            format!(
                "restart of {} refused: {deaths} deaths in {} seconds exceeds {}",
                req.process, policy.intensity.window_s, policy.intensity.max_deaths
            ),
        ));
    }

    Ok(())
}

fn render_with_policy(
    req: &RestartRequest,
    policy: &ChildPolicy,
    ctx: &RestartContext,
) -> RestartVerdict {
    if policy.restart == Restart::Transient && req.death.class.is_clean() {
        return RestartVerdict::Stop;
    }

    let attempt = ctx
        .tally
        .deaths_within(ctx.now_epoch_s, policy.intensity.window_s)
        .saturating_sub(1);
    RestartVerdict::Restart {
        after_s: backoff_delay_s(&policy.backoff, attempt),
        attempt,
    }
}

fn backoff_delay_s(backoff: &Backoff, attempt: u32) -> u64 {
    let exponent = attempt.min(backoff.steps);
    let factor = 1_u64.checked_shl(exponent).unwrap_or(u64::MAX);
    backoff.min_s.saturating_mul(factor).min(backoff.max_s)
}

fn give_up_reason(
    req: &RestartRequest,
    grant: &RestartGrant,
    ctx: &RestartContext,
    refusal: &Refusal,
) -> GiveUpReason {
    let tally = &ctx.tally;
    match &refusal.code {
        RefusalCode::GateRefused(reason) if reason == "same-cause" => GiveUpReason::SameCause {
            key: tally
                .deaths
                .last()
                .map(same_cause_key)
                .unwrap_or_else(|| same_cause_key(&req.death)),
            count: tally.same_cause_run(),
        },
        RefusalCode::GateRefused(reason) if reason == "intensity" => {
            GiveUpReason::IntensityExceeded {
                deaths: tally.deaths_within(ctx.now_epoch_s, grant.policy.intensity.window_s),
                window_s: grant.policy.intensity.window_s,
            }
        }
        RefusalCode::GateRefused(reason)
            if reason == "policy-temporary" && grant.policy.restart == Restart::Temporary =>
        {
            GiveUpReason::PolicyTemporary
        }
        // S0 emits only the three refusal codes above. A new refusal code must
        // gain its own GiveUpReason variant instead of borrowing PolicyTemporary.
        _ => unreachable!("S0 restart refusal has no matching GiveUpReason: {refusal:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{manifest::Intensity, ExitClass};
    use elohim_compute::{LimitOwner, RefusalCode};

    fn death(at_epoch_s: u64, class: ExitClass) -> DeathRecord {
        DeathRecord {
            at_epoch_s,
            class,
            uptime_ms: 1_000,
            first_stderr_line: Some("fatal: repeated failure".to_string()),
        }
    }

    fn policy() -> ChildPolicy {
        ChildPolicy {
            intensity: Intensity {
                max_deaths: 5,
                window_s: 300,
            },
            backoff: Backoff {
                min_s: 1,
                max_s: 60,
                steps: 6,
            },
            same_cause_limit: 3,
            ..ChildPolicy::default()
        }
    }

    fn request(death: DeathRecord) -> RestartRequest {
        RestartRequest {
            process: "conductor".to_string(),
            death,
        }
    }

    fn grant(policy: ChildPolicy) -> RestartGrant {
        RestartGrant {
            bounded_by: BoundedBy::ManifestPolicy,
            policy,
        }
    }

    #[test]
    fn three_identical_fast_deaths_give_up_by_same_cause() {
        let class = ExitClass::Signaled {
            signal: 9,
            core_dumped: false,
        };
        let deaths = vec![death(98, class), death(99, class), death(100, class)];
        let req = request(deaths[2].clone());
        let key = same_cause_key(&req.death);
        let ctx = RestartContext {
            now_epoch_s: 100,
            tally: DeathTally { deaths },
        };

        let (verdict, refusal) = RestartGovernor.verdict(&req, &grant(policy()), &ctx);

        assert_eq!(
            verdict,
            RestartVerdict::GiveUp {
                reason: GiveUpReason::SameCause { key, count: 3 }
            }
        );
        let refusal = refusal.expect("same-cause give-up carries its refusal");
        assert_eq!(
            refusal.code,
            RefusalCode::GateRefused("same-cause".to_string())
        );
        assert_eq!(refusal.limit_owner, LimitOwner::Operator);
    }

    #[test]
    fn intensity_excess_gives_up_with_exact_refusal() {
        let mut limited = policy();
        limited.intensity.max_deaths = 2;
        limited.same_cause_limit = 10;
        let deaths = vec![
            death(98, ExitClass::Exited { code: 1 }),
            death(99, ExitClass::Exited { code: 2 }),
            death(100, ExitClass::Exited { code: 3 }),
        ];
        let req = request(deaths[2].clone());
        let ctx = RestartContext {
            now_epoch_s: 100,
            tally: DeathTally { deaths },
        };

        let (verdict, refusal) = RestartGovernor.verdict(&req, &grant(limited), &ctx);

        assert_eq!(
            verdict,
            RestartVerdict::GiveUp {
                reason: GiveUpReason::IntensityExceeded {
                    deaths: 3,
                    window_s: 300,
                }
            }
        );
        assert_eq!(
            refusal.expect("intensity give-up carries its refusal").code,
            RefusalCode::GateRefused("intensity".to_string())
        );
    }

    #[test]
    fn backoff_doubles_and_caps() {
        let backoff = Backoff {
            min_s: 1,
            max_s: 60,
            steps: 6,
        };

        let actual: Vec<u64> = (0..=8)
            .map(|attempt| backoff_delay_s(&backoff, attempt))
            .collect();

        assert_eq!(actual, vec![1, 2, 4, 8, 16, 32, 60, 60, 60]);

        let mut restart_policy = policy();
        restart_policy.same_cause_limit = 100;
        restart_policy.intensity.max_deaths = 100;
        let mut actual_from_governor = Vec::new();
        for prior_deaths in 0..=6 {
            let current = death(100, ExitClass::Exited { code: 100 });
            let mut tally = DeathTally::default();
            for offset in 0..prior_deaths {
                tally.record(death(
                    99 - u64::from(offset),
                    ExitClass::Exited {
                        code: i32::try_from(offset).unwrap(),
                    },
                ));
            }
            tally.record(current.clone());
            let (verdict, refusal) = RestartGovernor.verdict(
                &request(current),
                &grant(restart_policy.clone()),
                &RestartContext {
                    now_epoch_s: 100,
                    tally,
                },
            );
            assert_eq!(refusal, None);
            let RestartVerdict::Restart { after_s, attempt } = verdict else {
                panic!("expected restart verdict");
            };
            assert_eq!(attempt, prior_deaths);
            actual_from_governor.push(after_s);
        }

        assert_eq!(actual_from_governor, vec![1, 2, 4, 8, 16, 32, 60]);
    }

    #[test]
    fn transient_clean_exit_stops_without_restart() {
        let mut transient = policy();
        transient.restart = Restart::Transient;
        let death = death(100, ExitClass::Exited { code: 0 });
        let req = request(death.clone());
        let ctx = RestartContext {
            now_epoch_s: 100,
            tally: DeathTally {
                deaths: vec![death],
            },
        };

        assert_eq!(
            RestartGovernor.verdict(&req, &grant(transient), &ctx),
            (RestartVerdict::Stop, None)
        );
    }

    #[test]
    fn temporary_never_restarts() {
        let mut temporary = policy();
        temporary.restart = Restart::Temporary;
        let death = death(100, ExitClass::Exited { code: 1 });
        let req = request(death.clone());
        let ctx = RestartContext {
            now_epoch_s: 100,
            tally: DeathTally {
                deaths: vec![death],
            },
        };

        let (verdict, refusal) = RestartGovernor.verdict(&req, &grant(temporary), &ctx);

        assert_eq!(
            verdict,
            RestartVerdict::GiveUp {
                reason: GiveUpReason::PolicyTemporary
            }
        );
        assert_eq!(
            refusal.expect("temporary policy carries its refusal").code,
            RefusalCode::GateRefused("policy-temporary".to_string())
        );
    }

    #[test]
    fn commitment_bounded_refusal_names_commitment_owner() {
        let mut temporary = policy();
        temporary.restart = Restart::Temporary;
        let death = death(100, ExitClass::Exited { code: 1 });
        let req = request(death.clone());
        let ctx = RestartContext {
            now_epoch_s: 100,
            tally: DeathTally {
                deaths: vec![death],
            },
        };
        let grant = RestartGrant {
            bounded_by: BoundedBy::Commitment {
                cid: "bafy-commitment".to_string(),
            },
            policy: temporary,
        };

        let (_, refusal) = RestartGovernor.verdict(&req, &grant, &ctx);

        assert_eq!(
            refusal
                .expect("temporary policy carries its refusal")
                .limit_owner,
            LimitOwner::Commitment
        );
    }

    #[test]
    fn transient_clean_exit_still_obeys_commitment_same_cause_gate() {
        let mut transient = policy();
        transient.restart = Restart::Transient;
        let current = death(100, ExitClass::Exited { code: 0 });
        let req = request(current.clone());
        let ctx = RestartContext {
            now_epoch_s: 100,
            tally: DeathTally {
                deaths: vec![
                    death(98, ExitClass::Exited { code: 0 }),
                    death(99, ExitClass::Exited { code: 0 }),
                    current,
                ],
            },
        };
        let grant = RestartGrant {
            bounded_by: BoundedBy::Commitment {
                cid: "bafy-commitment".to_string(),
            },
            policy: transient,
        };

        let (verdict, refusal) = RestartGovernor.verdict(&req, &grant, &ctx);

        assert!(matches!(
            verdict,
            RestartVerdict::GiveUp {
                reason: GiveUpReason::SameCause { count: 3, .. }
            }
        ));
        let refusal = refusal.expect("same-cause gate carries its refusal");
        assert_eq!(
            refusal.code,
            RefusalCode::GateRefused("same-cause".to_string())
        );
        assert_eq!(refusal.limit_owner, LimitOwner::Commitment);
    }

    #[test]
    fn empty_tally_is_safe() {
        let current = death(100, ExitClass::Exited { code: 1 });
        let req = request(current.clone());
        let ctx = RestartContext {
            now_epoch_s: 100,
            tally: DeathTally {
                deaths: vec![current],
            },
        };

        assert_eq!(
            RestartGovernor.verdict(&req, &grant(policy()), &ctx),
            (
                RestartVerdict::Restart {
                    after_s: 1,
                    attempt: 0,
                },
                None,
            )
        );
    }
}

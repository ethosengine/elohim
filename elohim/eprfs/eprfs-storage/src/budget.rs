//! `SemaphoreGate`: a bounded forwarding budget over tokio's semaphore (interfaces spec §A.6).
//!
//! eprfs-core's `ForwardGate` is a synchronous try-acquire, so the core needs no runtime; this
//! adapter supplies one: a slot per request in flight, optionally per requester (a shared tier's
//! budget is per requester, A4). A refusal names its rule and gives both the relative and the
//! absolute come-back time, which only an adapter can give because only an adapter reads a clock
//! (rail R-E2).
//!
//! No byte ceiling per request: the chain cannot know an item's size before it arrives, so it
//! passes no `bytes_hint`, and a ceiling nothing feeds would be a bound that holds nothing. The
//! chain bounds the bytes it holds itself (`eprfs_core::ChainLimits::max_write_back_bytes`).
//!
//! A per-requester gate keeps at most `max_requesters` budgets. A requester's budget is dropped
//! once it is idle (every slot free and no permit outstanding), so the gate's own memory stays
//! bounded by the requesters in flight; a new requester past the cap, with every budget busy, is
//! refused by the rule `<rule>:requesters`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use eprfs_core::{ForwardGate, ForwardPermit, OverBudget, Requester, TierId};
use tokio::sync::Semaphore;

/// How the in-flight slots are shared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetScope {
    /// One budget for every requester: a person's own device.
    Shared,
    /// A budget per requester agent (anonymous requesters share one): a shared tier.
    PerRequester,
}

pub struct SemaphoreGate {
    max_in_flight: usize,
    scope: BudgetScope,
    max_requesters: usize,
    retry_after_ms: u64,
    rule: String,
    shared: Arc<Semaphore>,
    per_requester: Mutex<HashMap<Option<String>, Arc<Semaphore>>>,
}

impl SemaphoreGate {
    pub fn new(max_in_flight: usize, scope: BudgetScope, rule: impl Into<String>) -> Self {
        Self {
            max_in_flight,
            scope,
            max_requesters: 1024,
            retry_after_ms: 1000,
            rule: rule.into(),
            shared: Arc::new(Semaphore::new(max_in_flight)),
            per_requester: Mutex::new(HashMap::new()),
        }
    }

    /// The most requester budgets a per-requester gate keeps at once (default 1024; at least 1).
    pub fn with_max_requesters(mut self, n: usize) -> Self {
        self.max_requesters = n.max(1);
        self
    }

    /// The come-back interval a refusal names.
    pub fn with_retry_after_ms(mut self, ms: u64) -> Self {
        self.retry_after_ms = ms;
        self
    }

    fn refusal(&self, rule: String) -> OverBudget {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        OverBudget {
            retry_after_ms: self.retry_after_ms,
            retry_at_epoch_secs: now + self.retry_after_ms.div_ceil(1000),
            rule,
        }
    }

    fn semaphore(&self, on_behalf: &Requester) -> Result<Arc<Semaphore>, OverBudget> {
        match self.scope {
            BudgetScope::Shared => Ok(self.shared.clone()),
            BudgetScope::PerRequester => {
                let mut map = self
                    .per_requester
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                let key = on_behalf.agent().map(str::to_string);
                if let Some(found) = map.get(&key) {
                    return Ok(found.clone());
                }
                if map.len() >= self.max_requesters {
                    // Idle: no permit outstanding (only the map holds it) and every slot free.
                    let max = self.max_in_flight;
                    map.retain(|_, s| Arc::strong_count(s) > 1 || s.available_permits() < max);
                }
                if map.len() >= self.max_requesters {
                    return Err(self.refusal(format!("{}:requesters", self.rule)));
                }
                Ok(map
                    .entry(key)
                    .or_insert_with(|| Arc::new(Semaphore::new(self.max_in_flight)))
                    .clone())
            }
        }
    }

    /// How many requester budgets the gate holds now.
    pub fn requester_budgets(&self) -> usize {
        self.per_requester
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len()
    }
}

impl ForwardGate for SemaphoreGate {
    fn admit(
        &self,
        on_behalf: &Requester,
        _to: &TierId,
        _bytes_hint: Option<u64>,
    ) -> Result<ForwardPermit, OverBudget> {
        self.semaphore(on_behalf)?
            .try_acquire_owned()
            .map(ForwardPermit::new)
            .map_err(|_| self.refusal(self.rule.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eprfs_core::KeyCustody;

    #[test]
    fn slots_are_released_when_the_permit_drops_and_refusals_name_their_rule() {
        let gate =
            SemaphoreGate::new(1, BudgetScope::Shared, "phone-forward").with_retry_after_ms(2500);
        let me = Requester::authenticated("uhCAkme", KeyCustody::DeviceHeld);
        let hub = TierId::agent("uhCAkhub");
        let permit = gate.admit(&me, &hub, None).unwrap();
        let refused = gate.admit(&me, &hub, None).unwrap_err();
        assert_eq!(refused.rule, "phone-forward");
        assert_eq!(refused.retry_after_ms, 2500);
        assert!(refused.retry_at_epoch_secs > 0);
        drop(permit);
        assert!(gate.admit(&me, &hub, None).is_ok());
    }

    #[test]
    fn a_per_requester_budget_keeps_one_requester_from_spending_anothers() {
        let gate = SemaphoreGate::new(1, BudgetScope::PerRequester, "hub-forward");
        let hub = TierId::agent("uhCAkhub");
        let a = Requester::authenticated("uhCAka", KeyCustody::DeviceHeld);
        let b = Requester::authenticated("uhCAkb", KeyCustody::DeviceHeld);
        let _held = gate.admit(&a, &hub, None).unwrap();
        assert!(gate.admit(&a, &hub, None).is_err());
        assert!(gate.admit(&b, &hub, None).is_ok());
    }

    #[test]
    fn idle_requester_budgets_are_dropped_and_a_busy_full_gate_refuses_by_name() {
        let gate =
            SemaphoreGate::new(1, BudgetScope::PerRequester, "hub-forward").with_max_requesters(2);
        let hub = TierId::agent("uhCAkhub");
        let who = |n: usize| Requester::authenticated(format!("uhCAk{n}"), KeyCustody::DeviceHeld);
        // A thousand requesters one after another: each budget is idle once its permit drops.
        for n in 0..1000 {
            drop(gate.admit(&who(n), &hub, None).unwrap());
            assert!(gate.requester_budgets() <= 2);
        }
        // Two budgets busy: a third requester is refused by the cap's rule, not given a budget.
        let _a = gate.admit(&who(1), &hub, None).unwrap();
        let _b = gate.admit(&who(2), &hub, None).unwrap();
        assert_eq!(
            gate.admit(&who(3), &hub, None).unwrap_err().rule,
            "hub-forward:requesters"
        );
        assert_eq!(gate.requester_budgets(), 2);
    }
}

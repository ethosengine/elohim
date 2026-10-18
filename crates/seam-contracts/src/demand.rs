//! Bounded demand leases (C6a/C6b), shared by clients and resident runtimes.
//! This primitive aggregates urgency; it does not schedule work or grant resources.
//! Forcing case: 2026-10-14 portable-app refresh must promote existing background
//! work without adding a second queue or cancelling another reader's demand.
use crate::ReasonLabel;
use std::collections::BTreeMap;

/// Admission refusal; C6a/C8. Tested by `leases_are_bounded_and_independent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemandRefusal {
    /// The declared number of concurrent leases is already held.
    Capacity,
    /// Handle space is exhausted; never wrap and replace a live lease.
    HandleExhausted,
}
impl ReasonLabel for DemandRefusal {
    const ALL: &'static [Self] = &[Self::Capacity, Self::HandleExhausted];
    fn label(&self) -> &'static str {
        match self {
            Self::Capacity => "demand_capacity",
            Self::HandleExhausted => "demand_handle_exhausted",
        }
    }
}

/// Per-consumer leases over coalesced work keys. C6a/C6b: bounded insertion,
/// immutable targets, idempotent release. No clocks, transport, persistence or authority.
/// Forcing case: concurrent dashboard refreshes must not duplicate workers.
/// Contract test: `leases_are_bounded_and_independent`.
pub struct DemandLeases<K, T> {
    limit: usize,
    next_id: u64,
    leases: BTreeMap<u64, (K, bool, T)>,
}
impl<K: Ord, T> DemandLeases<K, T> {
    /// Declare a concurrent-consumer bound. Zero deliberately refuses every insertion.
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            next_id: 0,
            leases: BTreeMap::new(),
        }
    }
    /// Freeze a target under a work key. Foreground is a request, never admission.
    pub fn insert(&mut self, key: K, foreground: bool, target: T) -> Result<u64, DemandRefusal> {
        if self.leases.len() >= self.limit {
            return Err(DemandRefusal::Capacity);
        }
        let id = self
            .next_id
            .checked_add(1)
            .ok_or(DemandRefusal::HandleExhausted)?;
        self.next_id = id;
        self.leases.insert(id, (key, foreground, target));
        Ok(id)
    }
    /// Aggregate a single worker demand per key; any foreground lease promotes it.
    pub fn work(&self) -> BTreeMap<&K, bool> {
        let mut work = BTreeMap::new();
        for (key, foreground, _) in self.leases.values() {
            *work.entry(key).or_insert(false) |= foreground;
        }
        work
    }
    /// Inspect a fixed target without permitting in-place extension.
    pub fn target(&self, id: u64) -> Option<&T> {
        self.leases.get(&id).map(|(_, _, target)| target)
    }
    /// Release one consumer. Repeating release is harmless; durable work is not owned here.
    pub fn release(&mut self, id: u64) {
        self.leases.remove(&id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn leases_are_bounded_and_independent() {
        let mut leases = DemandLeases::new(3);
        let background = leases.insert("observations", false, 1).unwrap();
        let first = leases.insert("observations", true, 10).unwrap();
        let second = leases.insert("observations", true, 20).unwrap();
        assert_eq!(
            leases.insert("other", true, 0),
            Err(DemandRefusal::Capacity)
        );
        assert_eq!(leases.work().len(), 1);
        assert_eq!(leases.target(first), Some(&10));
        leases.release(first);
        leases.release(first);
        assert!(leases.work()[&"observations"]);
        leases.release(second);
        assert!(!leases.work()[&"observations"]);
        assert_eq!(leases.target(background), Some(&1));
        crate::assert_reason_labels_conformant::<DemandRefusal>();
    }
    #[test]
    fn exhausted_handles_never_replace_a_lease() {
        let mut leases = DemandLeases::new(2);
        let id = leases.insert("scope", false, 1).unwrap();
        leases.next_id = u64::MAX;
        assert_eq!(
            leases.insert("scope", true, 2),
            Err(DemandRefusal::HandleExhausted)
        );
        assert_eq!(leases.target(id), Some(&1));
    }
}

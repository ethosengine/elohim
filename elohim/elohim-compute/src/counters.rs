//! Thread-safe request throughput counters.

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestCounterSnapshot {
    pub total: u64,
    pub by_category: HashMap<String, u64>,
}

pub struct RequestCounters {
    total: AtomicU64,
    by_category: DashMap<String, AtomicU64>,
}

impl RequestCounters {
    pub fn new() -> Self {
        Self {
            total: AtomicU64::new(0),
            by_category: DashMap::new(),
        }
    }

    pub fn increment(&self, category: &str) {
        self.total.fetch_add(1, Ordering::Relaxed);
        self.by_category
            .entry(category.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Snapshot all counters for serialization.
    ///
    /// Note: `total` may transiently exceed `sum(by_category)` by the number of
    /// in-flight `increment()` calls (Relaxed ordering). Acceptable for monitoring;
    /// not suitable for billing.
    pub fn snapshot(&self) -> RequestCounterSnapshot {
        let by_category: HashMap<String, u64> = self
            .by_category
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().load(Ordering::Relaxed)))
            .collect();

        RequestCounterSnapshot {
            total: self.total.load(Ordering::Relaxed),
            by_category,
        }
    }
}

impl Default for RequestCounters {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_increment_and_snapshot() {
        let counters = RequestCounters::new();
        counters.increment("Content");
        counters.increment("Content");
        counters.increment("LearningPath");
        let snap = counters.snapshot();
        assert_eq!(snap.total, 3);
        assert_eq!(*snap.by_category.get("Content").unwrap(), 2);
        assert_eq!(*snap.by_category.get("LearningPath").unwrap(), 1);
    }

    #[test]
    fn test_empty_snapshot() {
        let counters = RequestCounters::new();
        let snap = counters.snapshot();
        assert_eq!(snap.total, 0);
        assert!(snap.by_category.is_empty());
    }

    #[test]
    fn test_snapshot_serializes_camel_case() {
        let counters = RequestCounters::new();
        counters.increment("Content");
        let snap = counters.snapshot();
        let json = serde_json::to_string(&snap).unwrap();
        assert!(json.contains("\"byCategory\""));
        assert!(json.contains("\"total\""));
    }

    #[test]
    fn test_concurrent_increments() {
        use std::sync::Arc;
        use std::thread;

        let counters = Arc::new(RequestCounters::new());
        let mut handles = vec![];

        for _ in 0..10 {
            let c = Arc::clone(&counters);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    c.increment("Content");
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        let snap = counters.snapshot();
        assert_eq!(snap.total, 1000);
        assert_eq!(*snap.by_category.get("Content").unwrap(), 1000);
    }
}

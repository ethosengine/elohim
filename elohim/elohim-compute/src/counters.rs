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

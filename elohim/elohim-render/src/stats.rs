//! `RenderTraceStats` — in-process aggregation of render-trace outcomes.
//!
//! The feed-forward seam: every render's terminal + latency is folded into a
//! thread-safe running summary that the render-capability layer (and a
//! compute-commitment scorer) reads to tune concurrency and peer diversity.
//! Lives in the core crate so both doorway and a capable storage peer aggregate
//! their own outcomes — the per-peer `stall_rate` and `avg_wall_ms` are exactly
//! the signal that tunes the diversity of peer compute commitments.
//!
//! Cumulative (lifetime) counters for the MVP; a bounded rolling window is the
//! natural refinement once the scorer wants recency-weighting.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::types::{RenderTerminal, RenderTrace};

/// Thread-safe running aggregate of render-trace outcomes.
#[derive(Default)]
pub struct RenderTraceStats {
    inner: Mutex<Counters>,
}

#[derive(Default)]
struct Counters {
    total: u64,
    rendered: u64,
    rendered_empty: u64,
    stalled: u64,
    timed_out: u64,
    errored: u64,
    wall_ms_sum: u64,
    wall_ms_max: u64,
}

/// A point-in-time, serializable view of the aggregate — surfaced on a peer's
/// capability/admin endpoint and consumed by the compute-commitment scorer.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderTraceSnapshot {
    pub total: u64,
    pub rendered: u64,
    pub rendered_empty: u64,
    pub stalled: u64,
    pub timed_out: u64,
    pub errored: u64,
    pub avg_wall_ms: u64,
    pub max_wall_ms: u64,
    /// Fraction of renders that ended degenerate (stalled or timed-out) — the
    /// headline health signal for compute-commitment tuning. 0.0 when no renders.
    pub degenerate_rate: f64,
}

impl RenderTraceStats {
    pub fn new() -> Self {
        Self::default()
    }

    /// Fold one render's trace into the aggregate.
    pub fn record(&self, trace: &RenderTrace) {
        let mut c = self.inner.lock().expect("render stats poisoned");
        c.total += 1;
        match trace.terminal {
            RenderTerminal::Rendered => c.rendered += 1,
            RenderTerminal::RenderedEmpty => c.rendered_empty += 1,
            RenderTerminal::Stalled => c.stalled += 1,
            RenderTerminal::TimedOut => c.timed_out += 1,
            RenderTerminal::Errored => c.errored += 1,
        }
        c.wall_ms_sum = c.wall_ms_sum.saturating_add(trace.wall_ms);
        c.wall_ms_max = c.wall_ms_max.max(trace.wall_ms);
    }

    /// A serializable snapshot of the aggregate.
    pub fn snapshot(&self) -> RenderTraceSnapshot {
        let c = self.inner.lock().expect("render stats poisoned");
        let degenerate = c.stalled + c.timed_out;
        let avg_wall_ms = c.wall_ms_sum.checked_div(c.total).unwrap_or(0);
        let degenerate_rate = if c.total == 0 {
            0.0
        } else {
            degenerate as f64 / c.total as f64
        };
        RenderTraceSnapshot {
            total: c.total,
            rendered: c.rendered,
            rendered_empty: c.rendered_empty,
            stalled: c.stalled,
            timed_out: c.timed_out,
            errored: c.errored,
            avg_wall_ms,
            max_wall_ms: c.wall_ms_max,
            degenerate_rate,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FetchEvent;

    fn trace(terminal: RenderTerminal, wall_ms: u64) -> RenderTrace {
        RenderTrace {
            trace_id: String::new(),
            fetches: Vec::<FetchEvent>::new(),
            terminal,
            wall_ms,
        }
    }

    #[test]
    fn empty_stats_snapshot_is_zeroed() {
        let snap = RenderTraceStats::new().snapshot();
        assert_eq!(snap, RenderTraceSnapshot::default());
        assert_eq!(snap.degenerate_rate, 0.0);
    }

    #[test]
    fn records_terminal_counts_and_total() {
        let stats = RenderTraceStats::new();
        stats.record(&trace(RenderTerminal::Rendered, 50));
        stats.record(&trace(RenderTerminal::RenderedEmpty, 30));
        stats.record(&trace(RenderTerminal::Stalled, 1200));

        let snap = stats.snapshot();
        assert_eq!(snap.total, 3);
        assert_eq!(snap.rendered, 1);
        assert_eq!(snap.rendered_empty, 1);
        assert_eq!(snap.stalled, 1);
    }

    #[test]
    fn degenerate_rate_counts_stalled_and_timed_out() {
        let stats = RenderTraceStats::new();
        stats.record(&trace(RenderTerminal::Rendered, 10));
        stats.record(&trace(RenderTerminal::Stalled, 1200));
        stats.record(&trace(RenderTerminal::TimedOut, 60_000));
        stats.record(&trace(RenderTerminal::Rendered, 10));

        // 2 degenerate (stalled + timed-out) of 4 renders.
        assert_eq!(stats.snapshot().degenerate_rate, 0.5);
    }

    #[test]
    fn tracks_average_and_max_latency() {
        let stats = RenderTraceStats::new();
        stats.record(&trace(RenderTerminal::Rendered, 100));
        stats.record(&trace(RenderTerminal::Rendered, 300));

        let snap = stats.snapshot();
        assert_eq!(snap.avg_wall_ms, 200);
        assert_eq!(snap.max_wall_ms, 300);
    }
}

//! `TracingFetcher` — a framework-agnostic `DataFetcher` decorator.
//!
//! Wraps any `DataFetcher` and records a [`FetchEvent`] per call into a shared
//! collector, below the V8/framework boundary. The render path wraps the real
//! fetcher in this before building the JS runtime, then drains the events to
//! assemble the [`RenderTrace`]. No `Renderer` impl is aware of it — that is
//! what keeps the trace framework-agnostic and server-side.
//!
//! This layer records *settled* fetches (arrived / errored) with timing and the
//! empty classification. Converting a never-settling fetch into a recorded
//! `Stalled` is the per-fetch soft-deadline added in the watchdog phase.

use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::data_fetcher::{DataFetcher, FetchRequest, FetchResponse, FetcherTrust};
use crate::types::{FetchEvent, FetchOutcome};
use crate::Result;

/// Shared, thread-safe collector of fetch breadcrumbs for one render.
pub type FetchLog = Arc<Mutex<Vec<FetchEvent>>>;

/// Default per-fetch soft budget. A fetch outstanding longer than this is
/// recorded as `Stalled` and the call returns an error so the render falls back
/// fast instead of riding to the hard wall-time. Sits below the 2s wall default.
pub const DEFAULT_SOFT_BUDGET_MS: u64 = 1_200;

/// Decorates a `DataFetcher`, recording each fetch's outcome and timing, and
/// converting a never-settling fetch into a recorded `Stalled` at the soft budget.
pub struct TracingFetcher {
    inner: Arc<dyn DataFetcher>,
    start: Mutex<Instant>,
    log: FetchLog,
    soft_budget_ms: u64,
}

impl TracingFetcher {
    /// Wrap `inner` with the default soft budget. The render start clock begins
    /// now; per-fetch `offset_ms` is measured from this instant.
    pub fn new(inner: Arc<dyn DataFetcher>) -> Self {
        Self::with_soft_budget(inner, DEFAULT_SOFT_BUDGET_MS)
    }

    /// Wrap `inner` with an explicit per-fetch soft budget (milliseconds). The
    /// consumer sets this below the render's wall-time limit — it is effectively
    /// the per-peer fetch SLA whose breach feeds compute-commitment tuning.
    pub fn with_soft_budget(inner: Arc<dyn DataFetcher>, soft_budget_ms: u64) -> Self {
        Self {
            inner,
            start: Mutex::new(Instant::now()),
            log: Arc::new(Mutex::new(Vec::new())),
            soft_budget_ms,
        }
    }

    /// Reset the fetch log and the offset clock to scope the trace to a fresh
    /// render. The render worker reuses one fetcher across sequential renders, so
    /// it calls this at each render's start.
    pub fn reset(&self) {
        *self.start.lock().expect("start clock poisoned") = Instant::now();
        self.log.lock().expect("fetch log poisoned").clear();
    }

    /// A handle to the shared fetch log, cloneable for post-render drain.
    pub fn log(&self) -> FetchLog {
        self.log.clone()
    }

    /// Drain the recorded events.
    pub fn take_events(&self) -> Vec<FetchEvent> {
        std::mem::take(&mut self.log.lock().expect("fetch log poisoned"))
    }
}

/// Whether a settled response carries no usable content — a truthful "nothing
/// here" (404/204, empty body, or an empty JSON array) that must not be
/// confused with a stall.
pub(crate) fn is_empty_payload(status: u16, body: &[u8]) -> bool {
    if status == 404 || status == 204 {
        return true;
    }
    let trimmed = body.trim_ascii();
    trimmed.is_empty() || trimmed == b"[]"
}

#[async_trait]
impl DataFetcher for TracingFetcher {
    /// Delegates to the wrapped fetcher — a decorator must never answer for
    /// itself here. `TracingFetcher` sits between the render worker and the
    /// real fetcher on EVERY render (see [`crate::angular::AngularRenderer`]),
    /// so returning [`FetcherTrust::Ambient`] from this impl would launder
    /// every credentialed fetcher in the system into an ambient one and render
    /// the isolate trust bookkeeping permanently blind.
    fn trust_scope(&self) -> FetcherTrust {
        self.inner.trust_scope()
    }

    async fn fetch(&self, request: FetchRequest) -> Result<FetchResponse> {
        let offset_ms = self
            .start
            .lock()
            .expect("start clock poisoned")
            .elapsed()
            .as_millis() as u64;
        let url = request.url.clone();
        let method = request.method.clone();

        let started = Instant::now();
        let soft = Duration::from_millis(self.soft_budget_ms);
        let timed = tokio::time::timeout(soft, self.inner.fetch(request)).await;
        let duration_ms = started.elapsed().as_millis() as u64;

        let (outcome, result) = match timed {
            Ok(Ok(resp)) => {
                let outcome = FetchOutcome::Arrived {
                    status: resp.status,
                    bytes: resp.body.len(),
                    duration_ms,
                    empty: is_empty_payload(resp.status, &resp.body),
                };
                (outcome, Ok(resp))
            }
            Ok(Err(e)) => {
                let message = e.to_string();
                (
                    FetchOutcome::Errored {
                        message,
                        duration_ms,
                    },
                    Err(e),
                )
            }
            Err(_elapsed) => {
                // The fetch never settled within the soft budget — the degenerate
                // case. Record it and return an error so the render falls back fast
                // instead of riding to the hard wall-time limit.
                let outcome = FetchOutcome::Stalled {
                    waited_ms: self.soft_budget_ms,
                };
                let err = crate::RenderError::DataFetch(format!(
                    "fetch stalled past soft budget {}ms: {method} {url}",
                    self.soft_budget_ms
                ));
                (outcome, Err(err))
            }
        };

        self.log
            .lock()
            .expect("fetch log poisoned")
            .push(FetchEvent {
                url,
                method,
                offset_ms,
                outcome,
            });

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// A controllable inner fetcher: returns a configured response or an error.
    struct StubFetcher {
        response: Option<FetchResponse>,
    }

    #[async_trait]
    impl DataFetcher for StubFetcher {
        fn trust_scope(&self) -> FetcherTrust {
            FetcherTrust::Ambient
        }

        async fn fetch(&self, _request: FetchRequest) -> Result<FetchResponse> {
            match &self.response {
                Some(r) => Ok(r.clone()),
                None => Err(crate::RenderError::DataFetch("stub error".into())),
            }
        }
    }

    fn req() -> FetchRequest {
        FetchRequest {
            method: "GET".into(),
            url: "/api/v1/thing".into(),
            headers: HashMap::new(),
            body: None,
        }
    }

    fn resp(status: u16, body: &str) -> FetchResponse {
        FetchResponse {
            status,
            headers: HashMap::new(),
            body: body.as_bytes().to_vec(),
            content_hash: None,
        }
    }

    #[tokio::test]
    async fn records_an_arrival_with_bytes_as_non_empty() {
        let inner = Arc::new(StubFetcher {
            response: Some(resp(200, "{\"title\":\"hi\"}")),
        });
        let traced = TracingFetcher::new(inner);
        traced.fetch(req()).await.expect("ok");

        let events = traced.take_events();
        assert_eq!(events.len(), 1, "one fetch should be recorded");
        let e = &events[0];
        assert_eq!(e.url, "/api/v1/thing");
        assert_eq!(e.method, "GET");
        match e.outcome {
            FetchOutcome::Arrived {
                status,
                bytes,
                empty,
                ..
            } => {
                assert_eq!(status, 200);
                assert_eq!(bytes, 14);
                assert!(!empty, "a body with content is not empty");
            }
            ref other => panic!("expected Arrived, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn records_a_404_as_empty() {
        let inner = Arc::new(StubFetcher {
            response: Some(resp(404, "")),
        });
        let traced = TracingFetcher::new(inner);
        traced.fetch(req()).await.expect("ok");

        let events = traced.take_events();
        assert_eq!(events.len(), 1);
        assert!(
            matches!(events[0].outcome, FetchOutcome::Arrived { empty: true, .. }),
            "a 404 is a truthful empty"
        );
    }

    #[tokio::test]
    async fn records_an_empty_json_array_as_empty() {
        // The EprRouter-empties degenerate returns `[]` with a 200.
        let inner = Arc::new(StubFetcher {
            response: Some(resp(200, "[]")),
        });
        let traced = TracingFetcher::new(inner);
        traced.fetch(req()).await.expect("ok");

        let events = traced.take_events();
        assert!(
            matches!(events[0].outcome, FetchOutcome::Arrived { empty: true, .. }),
            "an empty array body is empty"
        );
    }

    #[tokio::test]
    async fn records_an_error_outcome() {
        let inner = Arc::new(StubFetcher { response: None });
        let traced = TracingFetcher::new(inner);
        let result = traced.fetch(req()).await;
        assert!(result.is_err(), "error must propagate to the caller");

        let events = traced.take_events();
        assert_eq!(events.len(), 1, "an errored fetch is still recorded");
        assert!(matches!(events[0].outcome, FetchOutcome::Errored { .. }));
    }

    /// An inner fetcher that never settles within the soft budget.
    struct SlowFetcher {
        delay_ms: u64,
    }

    #[async_trait]
    impl DataFetcher for SlowFetcher {
        fn trust_scope(&self) -> FetcherTrust {
            FetcherTrust::Ambient
        }

        async fn fetch(&self, _request: FetchRequest) -> Result<FetchResponse> {
            tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;
            Ok(resp(200, "finally"))
        }
    }

    #[tokio::test]
    async fn records_a_stall_when_a_fetch_exceeds_the_soft_budget() {
        // Inner takes 10s; soft budget is 20ms. The fetch must give up at the
        // budget, record a Stalled breadcrumb, and return an error so the render
        // falls back fast (rather than hanging to the hard wall-time).
        let inner = Arc::new(SlowFetcher { delay_ms: 10_000 });
        let traced = TracingFetcher::with_soft_budget(inner, 20);

        let result = traced.fetch(req()).await;
        assert!(
            result.is_err(),
            "a stalled fetch returns an error so the render can fall back fast"
        );

        let events = traced.take_events();
        assert_eq!(events.len(), 1);
        match events[0].outcome {
            FetchOutcome::Stalled { waited_ms } => {
                assert!(waited_ms >= 20, "waited at least the soft budget");
            }
            ref other => panic!("expected Stalled, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn a_fast_fetch_under_the_soft_budget_still_arrives() {
        // Guard: the soft deadline must not turn a healthy fast fetch into a stall.
        let inner = Arc::new(StubFetcher {
            response: Some(resp(200, "{\"ok\":true}")),
        });
        let traced = TracingFetcher::with_soft_budget(inner, 1_000);
        traced.fetch(req()).await.expect("ok");

        let events = traced.take_events();
        assert!(matches!(
            events[0].outcome,
            FetchOutcome::Arrived { empty: false, .. }
        ));
    }

    #[tokio::test]
    async fn reset_clears_events_for_reuse_across_renders() {
        // The render worker reuses one fetcher across sequential renders; reset
        // scopes the log (and the offset clock) to a single render.
        let inner = Arc::new(StubFetcher {
            response: Some(resp(200, "x")),
        });
        let traced = TracingFetcher::new(inner);
        traced.fetch(req()).await.expect("ok");
        assert_eq!(traced.log().lock().unwrap().len(), 1);

        traced.reset();
        assert_eq!(
            traced.log().lock().unwrap().len(),
            0,
            "reset clears the log for the next render"
        );
    }
}

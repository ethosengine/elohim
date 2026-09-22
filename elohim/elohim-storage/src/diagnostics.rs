//! Bounded, default-off runtime diagnostics built on the existing tracing,
//! runtime-config and Prometheus surfaces.
//!
//! No query, bind, payload, database URL, peer/cell identity or error text is
//! ever rendered here. Correlation ids are process-local and never metric labels.

use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use diesel::connection::{Instrumentation, InstrumentationEvent};

const MAX_WINDOW_SECONDS: u64 = 15 * 60;
const EVENT_SCHEMA_VERSION: u8 = 1;
const MAX_LOG_EVENTS_PER_WINDOW: u64 = 10_000;

static NEXT_CORRELATION: AtomicU64 = AtomicU64::new(1);
static WINDOW: LazyLock<Mutex<WindowState>> = LazyLock::new(|| Mutex::new(WindowState::default()));
static APPLIED_WINDOW_GENERATION: AtomicU64 = AtomicU64::new(0);
static LOG_EVENTS_REMAINING: AtomicU64 = AtomicU64::new(0);
static LOG_EVENTS_DROPPED: AtomicU64 = AtomicU64::new(0);
static PRODUCER: LazyLock<Producer> = LazyLock::new(|| {
    let pid = std::process::id();
    let initialized_micros = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros();
    Producer {
        pid,
        id: format!("storage-{pid}-{initialized_micros}"),
    }
});

struct Producer {
    pid: u32,
    id: String,
}

fn producer() -> &'static Producer {
    &PRODUCER
}

#[derive(Default)]
struct WindowState {
    generation: u64,
    expires: Option<Instant>,
    closed_witness: Option<Arc<AtomicBool>>,
}

impl WindowState {
    fn capture(&mut self, requested: u64, generation: u64, now: Instant) -> Option<WindowCapture> {
        if generation != self.generation {
            self.generation = generation;
            self.expires = (requested != 0)
                .then(|| now + Duration::from_secs(requested.min(MAX_WINDOW_SECONDS)));
            LOG_EVENTS_REMAINING.store(MAX_LOG_EVENTS_PER_WINDOW, Ordering::Release);
            LOG_EVENTS_DROPPED.store(0, Ordering::Release);
            self.closed_witness = (requested != 0).then(|| Arc::new(AtomicBool::new(false)));
        }
        self.expires
            .filter(|&expires| now < expires)
            .map(|expires| WindowCapture {
                generation: self.generation,
                expires,
                closed_witness: self
                    .closed_witness
                    .as_ref()
                    .expect("active window has a witness gate")
                    .clone(),
            })
    }
}

#[derive(Clone)]
struct WindowCapture {
    generation: u64,
    expires: Instant,
    closed_witness: Arc<AtomicBool>,
}

/// True only during the one finite window armed by runtime config.
fn capture() -> Option<WindowCapture> {
    let (requested, generation) = crate::runtime_config::diagnostics_window_state();
    if requested == 0 && APPLIED_WINDOW_GENERATION.load(Ordering::Acquire) == generation {
        return None;
    }

    let mut window = WINDOW.lock().unwrap();
    // Re-read after taking the transition lock. The fast-path cache is written
    // only here, so an older caller cannot publish stale state over a new arm.
    let (requested, generation) = crate::runtime_config::diagnostics_window_state();
    let capture = window.capture(requested, generation, Instant::now());
    APPLIED_WINDOW_GENERATION.store(generation, Ordering::Release);
    capture
}

fn capture_is_current(capture: &WindowCapture, now: Instant) -> bool {
    let (requested, generation) = crate::runtime_config::diagnostics_window_state();
    capture_matches_state(capture, requested, generation, now)
}

fn capture_matches_state(
    capture: &WindowCapture,
    requested: u64,
    generation: u64,
    now: Instant,
) -> bool {
    requested != 0 && generation == capture.generation && now < capture.expires
}

fn note_suppressed_terminal(capture: &WindowCapture) {
    if !capture.closed_witness.swap(true, Ordering::AcqRel) {
        let producer = producer();
        tracing::warn!(
            target: "elohim_storage::diagnostics",
            diagnostic_schema = EVENT_SCHEMA_VERSION,
            diagnostic_kind = "window_closed",
            producer_pid = producer.pid,
            producer_id = producer.id.as_str(),
            suppressed_terminals = 1_u8,
            "diagnostic terminal fell outside its capture window; the unmatched start is incomplete"
        );
    }
}

fn reserve_log_events(count: u64) -> bool {
    if LOG_EVENTS_REMAINING
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |remaining| {
            remaining.checked_sub(count)
        })
        .is_ok()
    {
        return true;
    }
    let dropped = LOG_EVENTS_DROPPED.fetch_add(count, Ordering::AcqRel) + count;
    if dropped.is_power_of_two() {
        let producer = producer();
        tracing::warn!(
            target: "elohim_storage::diagnostics",
            diagnostic_schema = EVENT_SCHEMA_VERSION,
            diagnostic_kind = "events_dropped",
            producer_pid = producer.pid,
            producer_id = producer.id.as_str(),
            dropped_events = dropped,
            event_limit = MAX_LOG_EVENTS_PER_WINDOW,
            "bounded diagnostic log-event budget exhausted"
        );
    }
    false
}

/// Closed, source-controlled DB operation vocabulary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    CapacityReport,
    Unattributed,
}

/// Closed statement call-site vocabulary. This is intentionally supplied by
/// the caller instead of derived by formatting Diesel's query object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatementSite {
    CapacityResolveCustodian,
    CapacityMeasure,
    CapacityUpsert,
    Unattributed,
}

impl StatementSite {
    pub const fn label(self) -> &'static str {
        match self {
            Self::CapacityResolveCustodian => "capacity_resolve_custodian",
            Self::CapacityMeasure => "capacity_measure",
            Self::CapacityUpsert => "capacity_upsert",
            Self::Unattributed => "unattributed",
        }
    }
}

impl Operation {
    pub const fn label(self) -> &'static str {
        match self {
            Self::CapacityReport => "capacity_report",
            Self::Unattributed => "unattributed",
        }
    }
}

#[derive(Clone, Copy)]
struct ScopeFrame {
    operation: Operation,
    statement_site: StatementSite,
    correlation: u64,
    next_query: u64,
    capture_generation: u64,
}

thread_local! {
    static SCOPES: RefCell<Vec<ScopeFrame>> = const { RefCell::new(Vec::new()) };
}

/// Private synchronous scope guard. Its `Rc` marker makes it `!Send`; callers
/// can only reach it through [`with_operation`], whose closure cannot suspend.
struct OperationScope {
    armed: bool,
    _not_send: PhantomData<Rc<()>>,
}

impl OperationScope {
    pub fn enter(operation: Operation, statement_site: StatementSite) -> Self {
        let Some(capture) = capture() else {
            return Self {
                armed: false,
                _not_send: PhantomData,
            };
        };
        let correlation = NEXT_CORRELATION.fetch_add(1, Ordering::Relaxed);
        SCOPES.with(|scopes| {
            scopes.borrow_mut().push(ScopeFrame {
                operation,
                statement_site,
                correlation,
                next_query: 1,
                capture_generation: capture.generation,
            });
        });
        Self {
            armed: true,
            _not_send: PhantomData,
        }
    }
}

/// Execute synchronous Diesel work under explicit bounded attribution.
///
/// The private RAII token is `!Send`, and the closure cannot suspend, so scope
/// state cannot migrate between async executor threads.
pub fn with_operation<T>(
    operation: Operation,
    statement_site: StatementSite,
    work: impl FnOnce() -> T,
) -> T {
    let _scope = OperationScope::enter(operation, statement_site);
    work()
}

impl Drop for OperationScope {
    fn drop(&mut self) {
        if self.armed {
            SCOPES.with(|scopes| {
                scopes.borrow_mut().pop();
            });
        }
    }
}

struct StartedQuery {
    started: Instant,
    operation: Operation,
    statement_site: StatementSite,
    correlation: u64,
    ordinal: u64,
    log_events: bool,
    capture: WindowCapture,
}

/// Per-connection Diesel instrumentation. It deliberately ignores every event
/// field that could expose SQL, bind values, URLs or error contents.
#[derive(Default)]
pub struct DieselDiagnostics {
    started: Vec<StartedQuery>,
}

impl DieselDiagnostics {
    fn start_query(&mut self, capture: Option<WindowCapture>) {
        let Some(capture) = capture else {
            return;
        };
        let (operation, statement_site, correlation, ordinal) = SCOPES.with(|scopes| {
            let mut scopes = scopes.borrow_mut();
            match scopes.last_mut() {
                Some(scope) if scope.capture_generation == capture.generation => {
                    let ordinal = scope.next_query;
                    scope.next_query += 1;
                    (
                        scope.operation,
                        scope.statement_site,
                        scope.correlation,
                        ordinal,
                    )
                }
                Some(_) | None => (
                    Operation::Unattributed,
                    StatementSite::Unattributed,
                    NEXT_CORRELATION.fetch_add(1, Ordering::Relaxed),
                    1,
                ),
            }
        });
        let log_events = reserve_log_events(2) && capture_is_current(&capture, Instant::now());
        if log_events {
            let producer = producer();
            tracing::info!(
                target: "elohim_storage::diagnostics",
                diagnostic_schema = EVENT_SCHEMA_VERSION,
                diagnostic_kind = "db_query_start",
                producer_pid = producer.pid,
                producer_id = producer.id.as_str(),
                operation = operation.label(),
                statement_site = statement_site.label(),
                correlation,
                query_ordinal = ordinal,
                "bounded diagnostic event"
            );
        }
        self.started.push(StartedQuery {
            started: Instant::now(),
            operation,
            statement_site,
            correlation,
            ordinal,
            log_events,
            capture,
        });
    }

    fn finish_query(&mut self, failed: bool) {
        let Some(started) = self.started.pop() else {
            return;
        };
        let outcome = if failed { "error" } else { "success" };
        let elapsed_ms = started.started.elapsed().as_secs_f64() * 1_000.0;
        crate::metrics::observe_db_diagnostic_query(
            started.operation,
            started.statement_site,
            failed,
            elapsed_ms,
        );
        if started.log_events && capture_is_current(&started.capture, Instant::now()) {
            let producer = producer();
            tracing::info!(
            target: "elohim_storage::diagnostics",
            diagnostic_schema = EVENT_SCHEMA_VERSION,
            diagnostic_kind = "db_query_finish",
            producer_pid = producer.pid,
            producer_id = producer.id.as_str(),
            operation = started.operation.label(),
            statement_site = started.statement_site.label(),
            correlation = started.correlation,
            query_ordinal = started.ordinal,
            outcome,
            elapsed_ms,
            "bounded diagnostic event"
            );
        } else if started.log_events {
            note_suppressed_terminal(&started.capture);
        }
    }
}

impl Instrumentation for DieselDiagnostics {
    fn on_connection_event(&mut self, event: InstrumentationEvent<'_>) {
        match event {
            InstrumentationEvent::StartQuery { .. } => self.start_query(capture()),
            InstrumentationEvent::FinishQuery { error, .. } => self.finish_query(error.is_some()),
            _ => {}
        }
    }
}

/// One admitted conductor attempt. Dropping without `finish` is local caller
/// abandonment, not proof that the conductor cancelled the work.
pub struct ConductorAttempt<'a> {
    correlation: Option<u64>,
    started: Instant,
    completed: bool,
    zome: &'a str,
    function: &'a str,
    class: &'static str,
    log_events: bool,
    capture: Option<WindowCapture>,
}

impl<'a> ConductorAttempt<'a> {
    pub fn start(zome: &'a str, function: &'a str, class: &'static str) -> Self {
        let capture = capture();
        let correlation = capture
            .as_ref()
            .map(|_| NEXT_CORRELATION.fetch_add(1, Ordering::Relaxed));
        let log_events = correlation.is_some()
            && reserve_log_events(2)
            && capture
                .as_ref()
                .is_some_and(|capture| capture_is_current(capture, Instant::now()));
        if let Some(correlation) = correlation.filter(|_| log_events) {
            let producer = producer();
            tracing::info!(
                target: "elohim_storage::diagnostics",
                diagnostic_schema = EVENT_SCHEMA_VERSION,
                diagnostic_kind = "conductor_attempt_start",
                producer_pid = producer.pid,
                producer_id = producer.id.as_str(),
                operation = "conductor_call",
                zome,
                function,
                class,
                correlation,
                attempt = 1_u8,
                "bounded diagnostic event"
            );
        }
        Self {
            correlation,
            started: Instant::now(),
            completed: false,
            zome,
            function,
            class,
            log_events,
            capture,
        }
    }

    pub fn finish(mut self, success: bool) {
        if let Some(correlation) = self.correlation.filter(|_| {
            self.log_events
                && self
                    .capture
                    .as_ref()
                    .is_some_and(|capture| capture_is_current(capture, Instant::now()))
        }) {
            let producer = producer();
            tracing::info!(
                target: "elohim_storage::diagnostics",
                diagnostic_schema = EVENT_SCHEMA_VERSION,
                diagnostic_kind = "conductor_attempt_finish",
                producer_pid = producer.pid,
                producer_id = producer.id.as_str(),
                operation = "conductor_call",
                zome = self.zome,
                function = self.function,
                class = self.class,
                correlation,
                attempt = 1_u8,
                outcome = if success { "success" } else { "error" },
                elapsed_ms = self.started.elapsed().as_secs_f64() * 1_000.0,
                "bounded diagnostic event"
            );
        } else if self.log_events {
            note_suppressed_terminal(
                self.capture
                    .as_ref()
                    .expect("logged attempt has a capture window"),
            );
        }
        self.completed = true;
    }
}

impl Drop for ConductorAttempt<'_> {
    fn drop(&mut self) {
        if !self.completed {
            if let Some(correlation) = self.correlation.filter(|_| {
                self.log_events
                    && self
                        .capture
                        .as_ref()
                        .is_some_and(|capture| capture_is_current(capture, Instant::now()))
            }) {
                let producer = producer();
                tracing::info!(
                    target: "elohim_storage::diagnostics",
                    diagnostic_schema = EVENT_SCHEMA_VERSION,
                    diagnostic_kind = "conductor_attempt_finish",
                    producer_pid = producer.pid,
                    producer_id = producer.id.as_str(),
                    operation = "conductor_call",
                    zome = self.zome,
                    function = self.function,
                    class = self.class,
                    correlation,
                    attempt = 1_u8,
                    outcome = "caller_dropped",
                    elapsed_ms = self.started.elapsed().as_secs_f64() * 1_000.0,
                    "bounded diagnostic event"
                );
            } else if self.log_events {
                note_suppressed_terminal(
                    self.capture
                        .as_ref()
                        .expect("logged attempt has a capture window"),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_is_one_shot_clamped_and_requires_zero_to_rearm_same_value() {
        let origin = Instant::now();
        let mut window = WindowState::default();
        assert!(window.capture(0, 0, origin).is_none());
        assert!(window.capture(10_000, 1, origin).is_some());
        assert!(window
            .capture(10_000, 1, origin + Duration::from_secs(901))
            .is_none());
        // Generation 2 (zero) was never sampled. Generation 3 still re-arms
        // the same requested value because the registry preserves the edge.
        assert!(window
            .capture(10_000, 3, origin + Duration::from_secs(903))
            .is_some());
    }

    #[test]
    fn terminal_requires_same_generation_and_unexpired_window() {
        let origin = Instant::now();
        let capture = WindowCapture {
            generation: 4,
            expires: origin + Duration::from_secs(10),
            closed_witness: Arc::new(AtomicBool::new(false)),
        };
        assert!(capture_matches_state(
            &capture,
            30,
            4,
            origin + Duration::from_secs(9)
        ));
        assert!(!capture_matches_state(
            &capture,
            0,
            5,
            origin + Duration::from_secs(9)
        ));
        assert!(!capture_matches_state(
            &capture,
            30,
            4,
            origin + Duration::from_secs(10)
        ));
    }

    #[test]
    fn query_ordinals_are_scoped_and_missing_scope_is_unattributed() {
        let mut observer = DieselDiagnostics::default();
        SCOPES.with(|scopes| {
            scopes.borrow_mut().push(ScopeFrame {
                operation: Operation::CapacityReport,
                statement_site: StatementSite::CapacityMeasure,
                correlation: 42,
                next_query: 1,
                capture_generation: 1,
            });
        });
        let capture = WindowCapture {
            generation: 1,
            expires: Instant::now() + Duration::from_secs(1),
            closed_witness: Arc::new(AtomicBool::new(false)),
        };
        observer.start_query(Some(capture.clone()));
        observer.start_query(Some(capture.clone()));
        assert_eq!(observer.started[0].ordinal, 1);
        assert_eq!(observer.started[1].ordinal, 2);
        assert_eq!(
            observer.started[0].statement_site,
            StatementSite::CapacityMeasure
        );
        SCOPES.with(|scopes| {
            scopes.borrow_mut().clear();
        });

        let mut unscoped = DieselDiagnostics::default();
        unscoped.start_query(Some(capture));
        assert_eq!(unscoped.started[0].operation, Operation::Unattributed);
        assert_eq!(
            unscoped.started[0].statement_site,
            StatementSite::Unattributed
        );
        assert_ne!(unscoped.started[0].correlation, 0);
        assert_eq!(unscoped.started[0].ordinal, 1);
    }

    #[test]
    fn disabled_query_start_records_nothing() {
        let mut observer = DieselDiagnostics::default();
        observer.start_query(None);
        assert!(observer.started.is_empty());
    }

    #[test]
    fn finish_consumes_exactly_one_started_query_for_success_and_error() {
        let mut observer = DieselDiagnostics::default();
        let capture = WindowCapture {
            generation: 1,
            expires: Instant::now() + Duration::from_secs(1),
            closed_witness: Arc::new(AtomicBool::new(false)),
        };
        observer.start_query(Some(capture.clone()));
        observer.finish_query(false);
        assert!(observer.started.is_empty());

        observer.start_query(Some(capture));
        observer.finish_query(true);
        assert!(observer.started.is_empty());

        // A finish without a matching start is ignored rather than fabricated
        // as an unattributed zero-duration query.
        observer.finish_query(true);
        assert!(observer.started.is_empty());
    }
}

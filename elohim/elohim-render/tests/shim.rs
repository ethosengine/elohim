use elohim_render::runtime::JsRuntime;

/// Minimal hand-rolled `tracing::Subscriber` (no `tracing-subscriber` dep
/// needed) that captures event *message* field text into a shared buffer,
/// filtered to a single target. Lets a test assert on the exact string the
/// `console.*` JS shim hands to `tracing::error!`/`warn!`/`info!`, bypassing
/// stdout formatting entirely. Spans are unused by the console ops, so span
/// bookkeeping methods are no-ops with a dummy incrementing id.
mod capture {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use tracing::field::{Field, Visit};
    use tracing::span::{Attributes, Id, Record};
    use tracing::{Event, Metadata, Subscriber};

    pub struct CapturingSubscriber {
        target: &'static str,
        messages: Arc<Mutex<Vec<String>>>,
        next_id: AtomicU64,
    }

    impl CapturingSubscriber {
        pub fn new(target: &'static str) -> (Self, Arc<Mutex<Vec<String>>>) {
            let messages = Arc::new(Mutex::new(Vec::new()));
            (
                Self {
                    target,
                    messages: messages.clone(),
                    next_id: AtomicU64::new(0),
                },
                messages,
            )
        }
    }

    #[derive(Default)]
    struct MessageVisitor(Option<String>);

    impl Visit for MessageVisitor {
        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            if field.name() == "message" {
                self.0 = Some(format!("{value:?}"));
            }
        }
    }

    impl Subscriber for CapturingSubscriber {
        fn enabled(&self, metadata: &Metadata<'_>) -> bool {
            metadata.target() == self.target
        }

        fn new_span(&self, _span: &Attributes<'_>) -> Id {
            Id::from_u64(self.next_id.fetch_add(1, Ordering::Relaxed) + 1)
        }

        fn record(&self, _span: &Id, _values: &Record<'_>) {}

        fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

        fn event(&self, event: &Event<'_>) {
            if event.metadata().target() != self.target {
                return;
            }
            let mut visitor = MessageVisitor::default();
            event.record(&mut visitor);
            if let Some(msg) = visitor.0 {
                self.messages.lock().unwrap().push(msg);
            }
        }

        fn enter(&self, _span: &Id) {}

        fn exit(&self, _span: &Id) {}
    }
}

#[test]
fn shim_js_files_are_pure_ascii() {
    let shim_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/shim");
    let mut violations = Vec::new();
    for entry in std::fs::read_dir(&shim_dir).expect("read shim dir") {
        let entry = entry.expect("entry");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("js") {
            continue;
        }
        let content = std::fs::read(&path).expect("read .js file");
        if !content.is_ascii() {
            violations.push(path);
        }
    }
    assert!(
        violations.is_empty(),
        "non-ASCII bytes in shim .js files (deno_core ascii_str_include! will panic): {:?}",
        violations
    );
}

#[tokio::test]
async fn console_log_does_not_throw() {
    let mut rt = JsRuntime::with_shims();
    let v = rt.eval_string("console.log('hello'); 42").await.unwrap();
    assert_eq!(v, "42");
}

/// Regression test for the bare `ERROR {}` js_console log line: Angular's SSR
/// bootstrap catches an in-app error and calls `console.error(someError)`.
/// The old `fmt` in `src/shim/console.js` ran every non-string arg through
/// `JSON.stringify`, and `JSON.stringify(new Error(...))` is `"{}"` because
/// `message`/`stack` are non-enumerable own properties on `Error` -- so the
/// one thing an operator needs (the actual error text) was discarded before
/// it ever reached `tracing`. This asserts end-to-end through the real
/// `op_console_error` -> `tracing::error!(target: "elohim_render::js_console", ...)`
/// path (see `src/shim/console.rs`), capturing the formatted message via a
/// custom `tracing::Subscriber` set as the thread-local default for the
/// duration of the eval (see the `capture` module above).
///
/// Confirmed this fails on the old `fmt`: temporarily reverting the
/// Error-special-case in `console.js` back to the bare `JSON.stringify`
/// branch reproduces the captured message `"{}"` (see the accompanying PR/
/// report evidence) -- i.e. this test would fail both the `contains(...)`
/// and the `!= "{}"` assertions below without the fix.
#[tokio::test]
async fn console_error_serializes_error_message_not_empty_object() {
    let (subscriber, messages) = capture::CapturingSubscriber::new("elohim_render::js_console");
    let _guard = tracing::subscriber::set_default(subscriber);

    let mut rt = JsRuntime::with_shims();
    rt.eval_string("console.error(new Error('boom-diagnostic')); 0")
        .await
        .unwrap();

    drop(_guard);

    let captured = messages.lock().unwrap();
    assert_eq!(
        captured.len(),
        1,
        "expected exactly one js_console event, got {:?}",
        *captured
    );
    let msg = &captured[0];

    // The defect this guards against: JSON.stringify(new Error(...)) === "{}",
    // silently swallowing the error text.
    assert_ne!(
        msg, "{}",
        "console.error(Error) must not collapse to the empty-object literal \
         (the JSON.stringify(new Error()) trap this test guards against)"
    );
    assert!(
        !msg.is_empty(),
        "console.error(Error) produced an empty message"
    );
    assert!(
        msg.contains("boom-diagnostic"),
        "expected the Error's message text in the captured js_console output, got: {msg:?}"
    );
    assert!(
        msg.contains("Error"),
        "expected the Error's name/tag in the captured js_console output, got: {msg:?}"
    );
}

#[tokio::test]
async fn url_constructor_works() {
    let mut rt = JsRuntime::with_shims();
    let v = rt
        .eval_string("new URL('/foo', 'https://example.com').href")
        .await
        .unwrap();
    assert_eq!(v, "https://example.com/foo");
}

#[tokio::test]
async fn text_encoder_round_trips() {
    let mut rt = JsRuntime::with_shims();
    let v = rt
        .eval_string("new TextDecoder().decode(new TextEncoder().encode('hi'))")
        .await
        .unwrap();
    assert_eq!(v, "hi");
}

#[tokio::test]
async fn url_searchparams_basic_ops() {
    let mut rt = JsRuntime::with_shims();
    let v = rt
        .eval_string(
            r#"
            const u = new URL('https://example.com/?a=1&b=2');
            const p = u.searchParams;
            `${p.get('a')}|${p.get('b')}|${p.has('c')}`
            "#,
        )
        .await
        .unwrap();
    assert_eq!(v, "1|2|false");
}

/// Regression: Angular 22's HTTP transfer-cache interceptor builds its cache key
/// via `sortAndConcatParams` -> `new URLSearchParams(...).sort()`. The shim had no
/// `sort()`, so EVERY HttpClient request threw `TypeError: ...sort is not a function`
/// synchronously inside `HttpInterceptorHandler.handle()` -- after `PendingTasks.add()`
/// but before `.pipe(finalize(removeTask))` was attached. Under
/// `provideZonelessChangeDetection()` the leaked tasks made `ApplicationRef.whenStable()`
/// (awaited by `renderApplication`) never resolve: a silent, total SSR stall on any route
/// that issues HttpClient traffic.
/// See `genesis/data/timeline/backlog/elohim-render-v22-elohim-app-stall.md`.
#[tokio::test]
async fn url_searchparams_sort_is_stable_by_name() {
    let mut rt = JsRuntime::with_shims();
    let v = rt
        .eval_string(
            r#"
            const p = new URLSearchParams('c=3&a=1&b=2&a=0');
            p.sort();
            p.toString()
            "#,
        )
        .await
        .unwrap();
    // Sorted by name; equal names keep their relative order (1 before 0).
    assert_eq!(v, "a=1&a=0&b=2&c=3");
}

/// Regression: the same Angular code path does
/// `new URLSearchParams(x instanceof URLSearchParams ? x : x.toString())`, so the
/// shim's constructor must copy pairs from another instance rather than enumerating
/// its own fields (which minted a single bogus `_params=...` entry).
#[tokio::test]
async fn url_searchparams_copy_constructor_and_size() {
    let mut rt = JsRuntime::with_shims();
    let v = rt
        .eval_string(
            r#"
            const src = new URLSearchParams('a=1&b=2');
            const copy = new URLSearchParams(src);
            `${copy.toString()}|${copy.size}`
            "#,
        )
        .await
        .unwrap();
    assert_eq!(v, "a=1&b=2|2");
}

#[tokio::test]
async fn url_ipv6_hostname_port() {
    let mut rt = JsRuntime::with_shims();
    let v = rt
        .eval_string(
            r#"
            const u = new URL('http://[::1]:8080/path');
            `${u.hostname}|${u.port}`
            "#,
        )
        .await
        .unwrap();
    assert_eq!(v, "[::1]|8080");
}

#[tokio::test]
async fn url_default_port_elided() {
    let mut rt = JsRuntime::with_shims();
    let v = rt
        .eval_string(
            r#"
            const u = new URL('https://example.com:443/');
            `${u.host}|${u.origin}`
            "#,
        )
        .await
        .unwrap();
    assert_eq!(v, "example.com|https://example.com");
}

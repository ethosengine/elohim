//! Go-compatible pprof CPU profiling endpoint (`GET /debug/pprof/profile`).
//!
//! # Why this exists
//!
//! Grafana Alloy's `pyroscope.scrape` component discovers targets that speak
//! the **Go `net/http/pprof` wire contract** — a `GET /debug/pprof/profile?seconds=N`
//! that blocks for `N` seconds and answers with a gzip-framed `profile.proto`
//! body. This module makes elohim-storage answerable by that scraper without
//! adding a second HTTP surface: the route rides the existing storage-http
//! listener (`:8090`), the same port `/health` and `/metrics` already use.
//!
//! # Substrate-floor discipline
//!
//! This is an **operational (Path C) surface**: pure, reconstructable telemetry
//! about *this process*, never notarized, never projected, never joined to any
//! REA ledger. It carries no care-class signal and no compute-class breach
//! signal — a CPU sample is not a commitment. It exists to let an operator (or
//! an elohim-ceiling agent) see where wall-clock is going on a peer.
//!
//! # Default OFF — env-gated
//!
//! Profiling installs a `SIGPROF` timer and unwinds the stack of every thread
//! at 100 Hz. That is cheap but not free, and it is not something a household
//! peer should pay for by default. The route answers ONLY when
//! [`PPROF_ENABLE_ENV`] (`ELOHIM_PPROF_ENABLED`) is truthy — the same
//! `1|true|yes|on` idiom `ELOHIM_TRUST_MEMO_READS` and friends use in
//! `main.rs`. When disabled the route returns **404**, which is exactly what a
//! scraper sees from a peer that simply does not carry the endpoint: an
//! honest absence, not an error.
//!
//! # Process-global guard
//!
//! `pprof::ProfilerGuard` takes a process-wide lock — two concurrent profiles
//! cannot coexist, and the second `build()` fails with a `CreatingError`.
//! Rather than surface that as a 500, [`PPROF_BUSY`] turns an overlap into a
//! **429 + `Retry-After`**, which is the honest answer to "a profile is already
//! running" and the one a scraper knows how to back off from.
//!
//! # Runtime discipline
//!
//! `ProfilerGuard` is `!Send` (it holds a std `RwLockWriteGuard` over the
//! global profiler), so it can never be held across an `.await` in a `Send`
//! future. The whole profile — build guard, sleep, report, encode — therefore
//! runs inside one [`tokio::task::spawn_blocking`] closure on the blocking
//! pool, with a plain `std::thread::sleep`. No core tokio worker is parked for
//! the profile window (the never-block-a-core-worker rule that the doorway
//! `getaddrinfo` wedge taught).
//!
//! # Encoding
//!
//! pprof-rs supplies the SAMPLER only. Its own protobuf codec features carry a
//! build script that cannot link under this crate's
//! `--cfg getrandom_backend="custom"` regime, so `profile.proto` is encoded by
//! [`profile_proto`] with prost. See that module's doc and the `pprof` entry in
//! `Cargo.toml` for the full rationale.

pub mod profile_proto;

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{header, Response, StatusCode};

/// Env var that enables the pprof endpoint. Default OFF.
pub const PPROF_ENABLE_ENV: &str = "ELOHIM_PPROF_ENABLED";

/// Default profile duration when `?seconds=` is absent — matches Go's
/// `net/http/pprof` default of 30s halved, so a scrape interval of 15s+ can
/// run back-to-back without overlapping into the busy-guard.
pub const DEFAULT_SECONDS: u64 = 15;

/// Minimum accepted profile duration.
pub const MIN_SECONDS: u64 = 1;

/// Maximum accepted profile duration. A scraper asking for more than a minute
/// of continuous sampling is misconfigured; clamp rather than honour it.
pub const MAX_SECONDS: u64 = 60;

/// Sampling frequency in Hz. 100 Hz is the Go runtime's default CPU profile
/// rate and what Pyroscope's Go-target dashboards assume.
pub const SAMPLE_FREQUENCY_HZ: i32 = 100;

/// Process-global "a profile is in flight" flag. `pprof`'s guard is
/// process-wide; this converts an overlap into a 429 instead of a 500.
static PPROF_BUSY: AtomicBool = AtomicBool::new(false);

/// Parse a truthy env value using the storage-wide idiom (`1|true|yes|on`).
///
/// Takes the value rather than reading the environment so it is testable
/// without `set_var` — the env-var-on-the-hot-path flake rule.
pub fn enabled_from(value: Option<&str>) -> bool {
    matches!(
        value.map(|v| v.trim().to_ascii_lowercase()).as_deref(),
        Some("1" | "true" | "yes" | "on")
    )
}

/// Whether the pprof endpoint is enabled for this process.
///
/// Read once per request (the route match arm) rather than threaded through
/// `HttpServer` state, because an operator flipping the env var is a pod
/// restart either way — and the read is off any hot path (it gates one
/// debug route, not a serving path).
pub fn is_enabled() -> bool {
    enabled_from(std::env::var(PPROF_ENABLE_ENV).ok().as_deref())
}

/// Extract and clamp `?seconds=N` from a raw query string.
///
/// Unparseable or absent → [`DEFAULT_SECONDS`]. Out of range → clamped into
/// `MIN_SECONDS..=MAX_SECONDS`. Never errors: a scraper sending garbage gets a
/// usable profile, not a 400.
pub fn parse_seconds(query: &str) -> u64 {
    let raw = query.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        if k == "seconds" {
            Some(v)
        } else {
            None
        }
    });

    match raw.and_then(|v| v.parse::<u64>().ok()) {
        Some(n) => n.clamp(MIN_SECONDS, MAX_SECONDS),
        None => DEFAULT_SECONDS,
    }
}

/// Outcome of a profile attempt.
pub enum ProfileOutcome {
    /// gzip-framed `profile.proto` bytes, ready to serve as the body.
    Profile(Vec<u8>),
    /// A profile was already running; the caller should answer 429.
    Busy,
    /// The profiler or encoder failed; the caller should answer 500 with this.
    Failed(String),
}

/// RAII release for [`PPROF_BUSY`] so an early return or a panic inside the
/// blocking task cannot wedge the flag ON for the life of the process.
struct BusyGuard;

impl Drop for BusyGuard {
    fn drop(&mut self) {
        PPROF_BUSY.store(false, Ordering::SeqCst);
    }
}

/// Run a CPU profile for `seconds` and return a gzip-framed pprof body.
///
/// The entire profile runs on the blocking pool: `ProfilerGuard` is `!Send`
/// and must never cross an `.await`, and a 15–60s sleep must never park a core
/// tokio worker.
pub async fn capture_profile(seconds: u64) -> ProfileOutcome {
    // Claim the process-global profiler slot, or report Busy.
    if PPROF_BUSY
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return ProfileOutcome::Busy;
    }
    let _busy = BusyGuard;

    let joined = tokio::task::spawn_blocking(move || build_profile_blocking(seconds)).await;

    match joined {
        Ok(Ok(bytes)) => ProfileOutcome::Profile(bytes),
        Ok(Err(e)) => ProfileOutcome::Failed(e),
        Err(e) => ProfileOutcome::Failed(format!("pprof task join failed: {e}")),
    }
}

/// The blocking half: guard → sleep → report → profile.proto → gzip.
fn build_profile_blocking(seconds: u64) -> Result<Vec<u8>, String> {
    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(SAMPLE_FREQUENCY_HZ)
        // Frames from these objects are unwind noise, not elohim call sites;
        // pprof-rs documents blocklisting them to avoid deadlocking the
        // unwinder inside malloc/pthread internals.
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
        .map_err(|e| format!("pprof guard build failed: {e}"))?;

    std::thread::sleep(Duration::from_secs(seconds));

    let report = guard
        .report()
        .build()
        .map_err(|e| format!("pprof report build failed: {e}"))?;

    let raw = profile_proto::encode(&profile_proto::report_to_profile(&report));

    gzip(&raw).map_err(|e| format!("pprof gzip failed: {e}"))
}

/// gzip-frame the profile the way Go's `net/http/pprof` does.
///
/// `google/pprof` (and Pyroscope, which parses with it) sniffs the gzip magic
/// and accepts raw protobuf too — but emitting the gzip frame keeps this
/// endpoint byte-shaped like a Go target, which is the whole point.
fn gzip(raw: &[u8]) -> std::io::Result<Vec<u8>> {
    use std::io::Write as _;

    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(raw)?;
    encoder.finish()
}

/// Run a profile for `seconds` and shape the outcome into an HTTP response.
///
/// * success → `200` + `application/octet-stream`, gzip-framed `profile.proto`
/// * overlap → `429` + `Retry-After` (a profile is already in flight)
/// * failure → `500` + `text/plain` reason
pub async fn profile_response(seconds: u64) -> Response<Full<Bytes>> {
    match capture_profile(seconds).await {
        ProfileOutcome::Profile(body) => Response::builder()
            .status(StatusCode::OK)
            // Go's net/http/pprof answers application/octet-stream; the
            // Content-Disposition mirrors it so a manual `curl -O` lands a
            // file `go tool pprof` opens without renaming.
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .header(
                header::CONTENT_DISPOSITION,
                r#"attachment; filename="profile""#,
            )
            .body(Full::new(Bytes::from(body)))
            .unwrap(),
        ProfileOutcome::Busy => Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            // MAX_SECONDS, not the requested duration: the in-flight profile's
            // remaining time is unknown, and MAX_SECONDS is the ceiling that
            // guarantees it has finished by the time the scraper retries.
            .header(header::RETRY_AFTER, MAX_SECONDS.to_string())
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .body(Full::new(Bytes::from(
                "a CPU profile is already in flight (the pprof profiler is process-global)\n",
            )))
            .unwrap(),
        ProfileOutcome::Failed(reason) => {
            tracing::warn!(error = %reason, "pprof CPU profile failed");
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
                .body(Full::new(Bytes::from(format!("{reason}\n"))))
                .unwrap()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_by_default() {
        assert!(!enabled_from(None));
    }

    #[test]
    fn accepts_truthy_idiom() {
        for v in ["1", "true", "TRUE", "yes", "on", " on "] {
            assert!(enabled_from(Some(v)), "expected {v:?} to enable");
        }
    }

    #[test]
    fn rejects_falsy_and_garbage() {
        for v in ["0", "false", "no", "off", "", "maybe"] {
            assert!(!enabled_from(Some(v)), "expected {v:?} to stay disabled");
        }
    }

    #[test]
    fn seconds_defaults_when_absent() {
        assert_eq!(parse_seconds(""), DEFAULT_SECONDS);
        assert_eq!(parse_seconds("debug=1"), DEFAULT_SECONDS);
    }

    #[test]
    fn seconds_parsed_from_query() {
        assert_eq!(parse_seconds("seconds=30"), 30);
        assert_eq!(parse_seconds("debug=1&seconds=5"), 5);
        assert_eq!(parse_seconds("seconds=7&debug=1"), 7);
    }

    #[test]
    fn seconds_clamped_to_range() {
        assert_eq!(parse_seconds("seconds=0"), MIN_SECONDS);
        assert_eq!(parse_seconds("seconds=600"), MAX_SECONDS);
    }

    #[test]
    fn seconds_falls_back_on_garbage() {
        assert_eq!(parse_seconds("seconds=abc"), DEFAULT_SECONDS);
        assert_eq!(parse_seconds("seconds=-1"), DEFAULT_SECONDS);
        assert_eq!(parse_seconds("seconds="), DEFAULT_SECONDS);
    }

    #[test]
    fn gzip_frames_with_magic() {
        let out = gzip(b"hello pprof").expect("gzip");
        assert_eq!(&out[..2], &[0x1f, 0x8b], "expected gzip magic bytes");
    }

    /// A real 1-second profile: proves the guard builds, the report encodes,
    /// and the body is a gzip-framed protobuf the scraper can parse — and that
    /// [`PPROF_BUSY`] is released afterwards so a scraper's next interval is
    /// not permanently 429'd.
    ///
    /// Deliberately ONE test, not two: `PPROF_BUSY` is process-global, so two
    /// profiling tests running in parallel would race (one would see `Busy`,
    /// and a busy-flag assertion would observe the other's in-flight claim).
    /// The `pprof` guard itself is process-wide for the same reason.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn captures_a_gzip_framed_profile_and_releases_the_guard() {
        // Give the sampler something to see.
        std::thread::spawn(|| {
            let deadline = std::time::Instant::now() + Duration::from_millis(1200);
            let mut acc: u64 = 0;
            while std::time::Instant::now() < deadline {
                acc = acc.wrapping_mul(31).wrapping_add(7);
            }
            std::hint::black_box(acc);
        });

        match capture_profile(1).await {
            ProfileOutcome::Profile(body) => {
                assert!(!body.is_empty(), "profile body must not be empty");
                assert_eq!(&body[..2], &[0x1f, 0x8b], "expected gzip magic bytes");

                // Parse it the way the scraper's decoder does: gunzip, then
                // decode profile.proto. A body that gzips but doesn't decode
                // would look healthy to a naive length check and be useless.
                use std::io::Read as _;
                let mut raw = Vec::new();
                flate2::read::GzDecoder::new(body.as_slice())
                    .read_to_end(&mut raw)
                    .expect("gunzip");

                let profile = <profile_proto::Profile as prost::Message>::decode(raw.as_slice())
                    .expect("decode profile.proto");
                assert_eq!(profile.string_table[0], "", "index 0 must be empty");
                assert_eq!(profile.sample_type.len(), 2);
                assert!(profile.duration_nanos > 0, "duration must be recorded");
                assert!(
                    !profile.sample.is_empty(),
                    "a busy thread should have produced at least one sample"
                );
            }
            ProfileOutcome::Busy => panic!("no concurrent profile should be running"),
            ProfileOutcome::Failed(e) => panic!("profile failed: {e}"),
        }

        assert!(
            !PPROF_BUSY.load(Ordering::SeqCst),
            "busy flag must be released after capture"
        );
    }
}

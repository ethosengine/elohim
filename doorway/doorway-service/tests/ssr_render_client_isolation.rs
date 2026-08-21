//! The doorway's ~60s post-render stall, reproduced at unit scale.
//!
//! MEASURED (local mesh, 2026-08-21 20:22): doorway A served 169 requests in the
//! 10s before its first SSR render of the lamad SPA, then 23 → 15 → 3 → 2 → 1 in
//! the six 10s buckets after it. Every request THROUGH the doorway took
//! **exactly 12.00s** — `STORAGE_PROXY_REQUEST_TIMEOUT_SECS` to the millisecond —
//! while the same storage answered a DIRECT probe in 1.6ms. `/admin/self-healing`
//! was unreachable. Then `storage forward failed (connect/timeout) — recording
//! breaker failure` ×3 and the breaker opened, `/health` degraded, saga ch01 red.
//!
//! Exactly-the-timeout is the tell. A slow upstream produces a spread of
//! latencies; a client-side request timeout firing on a connection that never
//! produces a byte produces one number, repeatedly. Storage was never slow.
//!
//! THE MECHANISM. The SSR render does NOT run on the tokio runtime — it runs on
//! a dedicated `angular-renderer` OS thread owning the V8 isolate, handed work
//! through a non-blocking `try_send` and awaited with `tokio::time::timeout`
//! (elohim-render/src/angular.rs:316-366, 579-617). That part is sound. The
//! defect is what the render's V8 `fetch` shim reaches for: doorway handed it
//! `state.storage_proxy_client` — **the same pooled reqwest client the proxy path
//! uses** (server/http.rs, `ResolverFetcher::new`).
//!
//! hyper-util spawns each new connection's driver task onto whatever runtime is
//! current when the connection is established. During a render that is the render
//! thread's `current_thread` runtime. When the render finishes, that thread
//! returns to `for work in rx` — a **`std::sync::mpsc::Receiver`**, i.e. a
//! blocking recv, executed *inside* `local_rt.block_on(...)`
//! (elohim-render/src/angular.rs:25, 315, 329, 344). The runtime stops polling.
//! Every connection driver parked on it freezes — while the connections stay in
//! the SHARED pool, indistinguishable from healthy ones. The main runtime then
//! checks them out for proxy reads, health, admin, everything, and each request
//! hangs until the 12s client timeout.
//!
//! So the render poisons the proxy's connection pool. These tests pin the
//! mechanism and the containment.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// A keep-alive HTTP/1.1 server that answers every request instantly. Stands in
/// for storage answering in 1.6ms — the point of these tests is that the SERVER
/// is never the slow party. The returned counter tracks ACCEPTED connections,
/// which is how "does this client pool?" is answered behaviourally.
async fn spawn_instant_server_counting_accepts() -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let accepts = Arc::new(AtomicUsize::new(0));
    let server_accepts = Arc::clone(&accepts);
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                return;
            };
            server_accepts.fetch_add(1, Ordering::SeqCst);
            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                loop {
                    match sock.read(&mut buf).await {
                        Ok(0) | Err(_) => return,
                        Ok(_) => {}
                    }
                    if sock
                        .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok")
                        .await
                        .is_err()
                    {
                        return;
                    }
                }
            });
        }
    });
    (format!("http://{addr}"), accepts)
}

/// Most tests only need the URL.
async fn spawn_instant_server() -> String {
    spawn_instant_server_counting_accepts().await.0
}

/// Establish a pooled connection from a `current_thread` runtime on its own OS
/// thread, then STOP POLLING that runtime — the render thread's blocking
/// `recv()` inside `block_on`, exactly. Returns once the connection is
/// established and the thread is parked.
fn strand_a_connection_on_a_frozen_runtime(
    client: reqwest::Client,
    url: String,
) -> mpsc::Sender<()> {
    let (done_tx, done_rx) = mpsc::channel::<()>();
    // The channel the parked thread blocks on — never sent to, like a renderer
    // waiting for its next work item.
    let (park_tx, park_rx) = mpsc::channel::<()>();
    std::thread::Builder::new()
        .name("frozen-render-thread".into())
        .spawn(move || {
            let local_rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("current_thread runtime");
            local_rt.block_on(async move {
                // The render's own fetch: establishes a connection whose hyper
                // driver task is spawned onto THIS runtime.
                let resp = client.get(&url).send().await.expect("render fetch");
                assert!(resp.status().is_success());
                let _ = resp.text().await;
                let _ = done_tx.send(());
                // The render is over. `for work in rx` — a std::sync::mpsc
                // blocking recv, inside block_on. Nothing on this runtime is
                // ever polled again.
                let _ = park_rx.recv();
            });
        })
        .expect("spawn frozen render thread");
    done_rx
        .recv_timeout(Duration::from_secs(10))
        .expect("render fetch completed and the thread parked");
    park_tx
}

/// THE DEFECT. A pooled client shared between the render thread and the proxy
/// path: after one render, the proxy's next request hangs on the stranded
/// connection until its own timeout — the exactly-12.00s signature.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_shared_pooled_client_strands_a_connection_and_hangs_the_proxy_path() {
    let url = spawn_instant_server().await;
    // Default pooling, exactly as `init_storage_proxy_client` builds it.
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(3))
        .build()
        .expect("client");

    let _park = strand_a_connection_on_a_frozen_runtime(client.clone(), url.clone());

    // The proxy path, on the main runtime, using the same client. A 2s budget
    // stands in for the 12s production one — the assertion is that it does not
    // complete, not how long it waits.
    let proxied = tokio::time::timeout(Duration::from_secs(2), client.get(&url).send()).await;
    assert!(
        proxied.is_err(),
        "SHARED POOL: the proxy checked out the connection the frozen render \
         thread stranded and hung on it — this is the 12.00s stall, reproduced. \
         If this assertion starts failing, reqwest/hyper began detecting the \
         stranded connection and the containment below may no longer be needed."
    );
}

/// THE CONTAINMENT. A render client that never contributes an idle connection to
/// any pool cannot strand one: every render fetch opens a connection that dies
/// with the render, so a frozen render thread leaves nothing behind for the
/// proxy path to check out.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_pool_free_render_client_leaves_the_proxy_path_untouched() {
    let url = spawn_instant_server().await;
    let render_client = doorway::server::http::init_ssr_render_client();
    let proxy_client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(3))
        .build()
        .expect("proxy client");

    let _park = strand_a_connection_on_a_frozen_runtime((*render_client).clone(), url.clone());

    // The proxy path is a different client entirely, so it cannot inherit the
    // render's stranded connection...
    let proxied = tokio::time::timeout(Duration::from_secs(5), proxy_client.get(&url).send())
        .await
        .expect("the proxy path must not hang behind a frozen render thread")
        .expect("proxy request");
    assert!(proxied.status().is_success());

    // ...and the render client itself keeps no idle connection to strand, so
    // even a second render (on a live runtime) is unaffected by the frozen one.
    let second = tokio::time::timeout(Duration::from_secs(5), render_client.get(&url).send())
        .await
        .expect("a fresh render connection must not queue behind the frozen one")
        .expect("render request");
    assert!(second.status().is_success());
}

/// The render client's pool-free-ness, asserted BEHAVIOURALLY (reqwest's `Debug`
/// does not expose pool config, and a shape test over a Debug string would be a
/// rail against the wrong thing anyway).
///
/// Two sequential requests must open two connections. A pooled client reuses one
/// — and a reused connection is exactly the thing a parked render thread strands.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_render_client_keeps_no_idle_connection_to_strand() {
    let (url, accepts) = spawn_instant_server_counting_accepts().await;

    // Control: a pooled client reuses its connection across two requests.
    let pooled = reqwest::Client::builder().build().expect("pooled client");
    for _ in 0..2 {
        pooled.get(&url).send().await.expect("pooled request");
    }
    let pooled_accepts = accepts.swap(0, Ordering::SeqCst);
    assert_eq!(
        pooled_accepts, 1,
        "a POOLED client reuses one connection across two requests — this is the \
         connection a frozen render thread strands"
    );

    // The render client: a fresh connection each time, nothing left behind.
    let render = doorway::server::http::init_ssr_render_client();
    for _ in 0..2 {
        render.get(&url).send().await.expect("render request");
    }
    assert_eq!(
        accepts.load(Ordering::SeqCst),
        2,
        "the SSR render client must open a FRESH connection per fetch — an idle \
         one handed back to a pool is what parks the whole gateway for 12s a \
         request. If this reads 1, pool_max_idle_per_host(0) was lost."
    );
}

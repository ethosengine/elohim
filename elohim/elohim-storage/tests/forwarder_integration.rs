//! Integration test: when expose_conductor_externally is true, main.rs
//! should spin up TWO forwarders (app and admin), each bridging a
//! pod-network bind to the corresponding 127.0.0.1 upstream.
//!
//! We can't run main.rs directly — it depends on HTTP, Holochain, etc. —
//! so this test exercises the forwarder module at the same API
//! granularity main.rs uses it.

use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use elohim_storage::forwarder::spawn_forwarder;

/// Stand up two echo upstreams (one per port), spawn two forwarders
/// pointed at them, and prove each forwarder forwards traffic to its
/// own upstream (not the other's).
#[tokio::test]
async fn two_forwarders_isolate_admin_and_app_traffic() {
    // Upstream 1 — "app" echoes "A" + client bytes
    let up_app = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let up_app_addr = up_app.local_addr().unwrap();
    tokio::spawn(run_prefix_echo(up_app, b"A:"));

    // Upstream 2 — "admin" echoes "B" + client bytes
    let up_adm = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let up_adm_addr = up_adm.local_addr().unwrap();
    tokio::spawn(run_prefix_echo(up_adm, b"B:"));

    // Forwarder 1 — front the app upstream on an ephemeral port
    let app_front = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let app_front_addr = app_front.local_addr().unwrap();
    drop(app_front); // spawn_forwarder will rebind
    spawn_forwarder(&app_front_addr.to_string(), up_app_addr.port())
        .await
        .unwrap();

    // Forwarder 2 — front the admin upstream on a different ephemeral port
    let adm_front = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let adm_front_addr = adm_front.local_addr().unwrap();
    drop(adm_front);
    spawn_forwarder(&adm_front_addr.to_string(), up_adm_addr.port())
        .await
        .unwrap();

    // Give accept loops a tick to settle
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Hit the app front → should see "A:hello"
    let got_app = roundtrip(app_front_addr, b"hello").await;
    assert_eq!(&got_app, b"A:hello");

    // Hit the admin front → should see "B:hello"
    let got_adm = roundtrip(adm_front_addr, b"hello").await;
    assert_eq!(&got_adm, b"B:hello");
}

async fn run_prefix_echo(listener: TcpListener, prefix: &'static [u8]) {
    while let Ok((mut sock, _)) = listener.accept().await {
        let pfx = prefix;
        tokio::spawn(async move {
            let mut buf = [0u8; 64];
            if let Ok(n) = sock.read(&mut buf).await {
                let mut out = Vec::with_capacity(pfx.len() + n);
                out.extend_from_slice(pfx);
                out.extend_from_slice(&buf[..n]);
                let _ = sock.write_all(&out).await;
            }
        });
    }
}

async fn roundtrip(addr: SocketAddr, payload: &[u8]) -> Vec<u8> {
    let mut s = TcpStream::connect(addr).await.unwrap();
    s.write_all(payload).await.unwrap();
    let mut buf = vec![0u8; payload.len() + 8];
    let n = s.read(&mut buf).await.unwrap();
    buf.truncate(n);
    buf
}

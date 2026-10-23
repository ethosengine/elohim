//! The device's iroh endpoint: bound on the device's own key, offering the given protocols, and
//! naming every inbound peer by the node id iroh authenticated in the handshake.

use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

use eprfs_peer::{bind, relay_from_value, PeerKey};
use iroh::NodeAddr;

const ALPN: &[u8] = b"/elohim/eprfs-peer-test/0";
const WITHIN: Duration = Duration::from_secs(20);

fn key() -> (tempfile::TempDir, PeerKey) {
    let dir = tempfile::tempdir().expect("tempdir");
    let key = PeerKey::load_or_generate(dir.path()).expect("key");
    (dir, key)
}

#[tokio::test(flavor = "multi_thread")]
async fn bind_answers_on_the_devices_own_node_id() {
    let (_d, key) = key();
    let endpoint = bind(&key, None, vec![ALPN.to_vec()]).await.expect("bind");
    assert_eq!(endpoint.node_id(), key.node_id());
    endpoint.close().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn an_inbound_peer_is_named_by_its_authenticated_node_id() {
    let (_da, key_a) = key();
    let (_db, key_b) = key();
    let server = bind(&key_b, None, vec![ALPN.to_vec()])
        .await
        .expect("bind b");
    let client = bind(&key_a, None, vec![]).await.expect("bind a");
    let port = server
        .bound_sockets()
        .into_iter()
        .find(SocketAddr::is_ipv4)
        .expect("an ipv4 socket")
        .port();
    let addr = NodeAddr::new(server.node_id())
        .with_direct_addresses([SocketAddr::from((Ipv4Addr::LOCALHOST, port))]);

    let accept = tokio::spawn({
        let server = server.clone();
        async move {
            let incoming = server.accept().await.expect("an incoming connection");
            let conn = incoming.await.expect("handshake");
            let remote = conn.remote_node_id().expect("authenticated");
            (remote, conn.alpn())
        }
    });
    let conn = tokio::time::timeout(WITHIN, client.connect(addr, ALPN))
        .await
        .expect("connects in time")
        .expect("connects");
    let (remote, alpn) = tokio::time::timeout(WITHIN, accept)
        .await
        .expect("accepts in time")
        .expect("accept task");

    assert_eq!(
        remote,
        key_a.node_id(),
        "iroh names the dialer by its own key"
    );
    assert_eq!(alpn.as_deref(), Some(ALPN));
    drop(conn);
    client.close().await;
    server.close().await;
}

#[test]
fn relay_value_parses_or_refuses_never_falls_back() {
    assert_eq!(relay_from_value(None).expect("unset"), None);
    assert_eq!(relay_from_value(Some("  ")).expect("empty"), None);
    let url = relay_from_value(Some("https://relay.elohim.host"))
        .expect("valid")
        .expect("set");
    assert_eq!(url.as_str(), "https://relay.elohim.host/");
    assert!(
        relay_from_value(Some("not a url")).is_err(),
        "a bad relay is a hard error"
    );
}

#[test]
fn a_peer_argument_is_a_node_id_at_a_socket_address() {
    let dir = tempfile::tempdir().expect("tempdir");
    let key = eprfs_peer::PeerKey::load_or_generate(dir.path()).expect("key");
    let addr =
        eprfs_peer::peer_from_arg(&format!("{}@127.0.0.1:4919", key.node_id())).expect("parses");
    assert_eq!(addr.node_id, key.node_id());
    assert_eq!(
        addr.direct_addresses().copied().collect::<Vec<_>>(),
        vec!["127.0.0.1:4919"
            .parse::<std::net::SocketAddr>()
            .expect("addr")]
    );
    for bad in [
        "127.0.0.1:4919",
        "nonsense@127.0.0.1:4919",
        &format!("{}@nowhere", key.node_id()),
    ] {
        assert!(
            matches!(
                eprfs_peer::peer_from_arg(bad),
                Err(eprfs_peer::PeerError::InvalidPeer { .. })
            ),
            "{bad}"
        );
    }
}

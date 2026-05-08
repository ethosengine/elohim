//! End-to-end test for the Phase 3 custom-ALPN harness.
//!
//! Implements a minimal `Echo` protocol on a fresh ALPN, mounts it on
//! `IrohNode::start_with_protocols`, and verifies a request/response
//! round-trip through `codec::{read_frame, write_frame}` over real QUIC.
//!
//! This is the worked example Phases 5–9 follow when wiring sync, EPR,
//! shard, view-fed, and identity protocols. Keep the shape stable.
//!
//! Gated on `p2p-iroh`.

#![cfg(feature = "p2p-iroh")]

use std::io;

use anyhow::Result;
use elohim_storage::p2p_iroh::{
    codec::{read_frame_default, write_frame},
    parity_harness::TwoNodeFixture,
    AlpnRegistration,
};
use iroh::{
    endpoint::Connection,
    protocol::{AcceptError, ProtocolHandler},
};
use serde::{Deserialize, Serialize};
use tempfile::tempdir;

/// ALPN for our test echo protocol. Distinct from any production ALPN.
const ECHO_ALPN: &[u8] = b"/elohim-test/echo/1.0.0";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct EchoRequest {
    payload: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct EchoResponse {
    payload: String,
    server_marker: String,
}

#[derive(Debug, Clone)]
struct EchoHandler;

impl ProtocolHandler for EchoHandler {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let (mut send, mut recv) = connection.accept_bi().await?;
        let req: EchoRequest = read_frame_default(&mut recv).await.map_err(io_to_accept)?;
        let res = EchoResponse {
            payload: req.payload,
            server_marker: "echoed".to_string(),
        };
        write_frame(&mut send, &res).await.map_err(io_to_accept)?;
        send.finish().map_err(|e| {
            AcceptError::from_err(io::Error::new(io::ErrorKind::Other, e.to_string()))
        })?;
        connection.closed().await;
        Ok(())
    }
}

fn io_to_accept(e: io::Error) -> AcceptError {
    AcceptError::from_err(e)
}

fn echo_protocols() -> Vec<AlpnRegistration> {
    vec![(ECHO_ALPN.to_vec(), Box::new(EchoHandler))]
}

#[tokio::test]
async fn echo_protocol_round_trips_over_iroh_quic() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let fixture =
        TwoNodeFixture::new(provider_dir.path(), fetcher_dir.path(), echo_protocols).await?;

    // Open a fresh bidi stream on the echo ALPN, send a request, read a response.
    let conn = fixture
        .fetcher
        .endpoint()
        .connect(fixture.provider_addr.clone(), ECHO_ALPN)
        .await?;
    let (mut send, mut recv) = conn.open_bi().await?;

    let req = EchoRequest {
        payload: "hello phase 3".into(),
    };
    write_frame(&mut send, &req).await?;
    send.finish()?;

    let res: EchoResponse = read_frame_default(&mut recv).await?;

    assert_eq!(res.payload, "hello phase 3");
    assert_eq!(res.server_marker, "echoed");

    fixture.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn multiple_sequential_echo_calls() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let fixture =
        TwoNodeFixture::new(provider_dir.path(), fetcher_dir.path(), echo_protocols).await?;

    for i in 0..3 {
        let conn = fixture
            .fetcher
            .endpoint()
            .connect(fixture.provider_addr.clone(), ECHO_ALPN)
            .await?;
        let (mut send, mut recv) = conn.open_bi().await?;
        let req = EchoRequest {
            payload: format!("call-{i}"),
        };
        write_frame(&mut send, &req).await?;
        send.finish()?;
        let res: EchoResponse = read_frame_default(&mut recv).await?;
        assert_eq!(res.payload, format!("call-{i}"));
    }

    fixture.shutdown().await?;
    Ok(())
}

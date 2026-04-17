use std::net::SocketAddr;
use tokio::io::copy_bidirectional;
use tokio::net::{TcpListener, TcpStream};

/// Accept inbound TCP connections on `listener` and pipe each to `upstream`
/// (localhost conductor port). Bidirectional, concurrent connections.
pub async fn forwarder_accept_loop(listener: TcpListener, upstream: SocketAddr) {
    loop {
        let (mut inbound, peer) = match listener.accept().await {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("forwarder accept error: {e}");
                continue;
            }
        };
        tokio::spawn(async move {
            let mut outbound = match TcpStream::connect(upstream).await {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("forwarder upstream connect failed ({peer} -> {upstream}): {e}");
                    return;
                }
            };
            if let Err(e) = copy_bidirectional(&mut inbound, &mut outbound).await {
                tracing::debug!("forwarder copy ended: {e}");
            }
        });
    }
}

/// Spawn the forwarder as a background task. Parses `bind` (e.g., "0.0.0.0:4445"),
/// binds the listener, logs the forward, and starts the accept loop.
pub async fn spawn_forwarder(bind: &str, upstream_port: u16) -> anyhow::Result<()> {
    let listener = TcpListener::bind(bind).await?;
    let upstream: SocketAddr = format!("127.0.0.1:{upstream_port}").parse()?;
    tracing::info!("peer-status forwarder: {bind} -> {upstream}");
    tokio::spawn(forwarder_accept_loop(listener, upstream));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn forwarder_pipes_bytes_bidirectionally() {
        // Upstream echo server on random port.
        let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let upstream_addr = upstream.local_addr().unwrap();
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = upstream.accept().await {
                tokio::spawn(async move {
                    let mut buf = [0u8; 64];
                    if let Ok(n) = socket.read(&mut buf).await {
                        let _ = socket.write_all(&buf[..n]).await;
                    }
                });
            }
        });

        // Forwarder on another random port that pipes to upstream.
        let ext = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let ext_addr = ext.local_addr().unwrap();
        tokio::spawn(forwarder_accept_loop(ext, upstream_addr));

        // Give the forwarder loop a moment to be ready.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Client connects via forwarder and expects echo.
        let mut client = TcpStream::connect(ext_addr).await.unwrap();
        client.write_all(b"ping").await.unwrap();
        let mut resp = [0u8; 4];
        client.read_exact(&mut resp).await.unwrap();
        assert_eq!(&resp, b"ping");
    }
}

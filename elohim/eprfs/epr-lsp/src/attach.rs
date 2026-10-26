//! The attach shim's two modes (contract C4): a byte proxy between the editor's stdio and the
//! daemon's socket, and a null server for when no daemon can be reached.
//!
//! The proxy watches the client's frames only to learn whether the client sent `exit`: when the
//! daemon closes the connection after the client's `exit`, the shim's work is done; when it
//! closes without one, the shared server died, and the shim exits non-zero so the editor's
//! restart-on-crash brings up a fresh attach (and with it a fresh daemon).

use std::cell::Cell;

use serde_json::{json, Value};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::UnixStream;

use crate::frame::{read_message, write_message, Decoder};
use crate::mux::{notice_diagnostic, show_message};

/// How a proxied session ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyEnd {
    /// The client closed its input.
    ClientClosed,
    /// The daemon closed the connection after the client's `exit`.
    ClientExited,
    /// The daemon closed the connection on its own: the shared server is gone.
    ServerClosed,
}

/// Proxy bytes both ways until either side ends.
///
/// # Errors
///
/// An I/O error on either side.
pub async fn proxy<I, O>(
    mut stdin: I,
    mut stdout: O,
    stream: UnixStream,
) -> std::io::Result<ProxyEnd>
where
    I: AsyncRead + Unpin,
    O: AsyncWrite + Unpin,
{
    let (mut from_daemon, mut to_daemon) = stream.into_split();
    let saw_exit = Cell::new(false);
    let up = async {
        let mut buffer = vec![0_u8; 16 << 10];
        let mut decoder = Some(Decoder::default());
        loop {
            let read = stdin.read(&mut buffer).await?;
            if read == 0 {
                let _ = to_daemon.shutdown().await;
                return Ok::<(), std::io::Error>(());
            }
            // Decode before forwarding, so `exit` is known before the daemon can react to it.
            if let Some(active) = decoder.as_mut() {
                active.push(&buffer[..read]);
                loop {
                    match active.next_message() {
                        Ok(Some(message)) => {
                            if message.get("method").and_then(Value::as_str) == Some("exit") {
                                saw_exit.set(true);
                            }
                        }
                        Ok(None) => break,
                        Err(error) if error.is_recoverable() => {}
                        // Out of sync: stop watching; bytes still flow untouched.
                        Err(_) => {
                            decoder = None;
                            break;
                        }
                    }
                }
            }
            to_daemon.write_all(&buffer[..read]).await?;
        }
    };
    let down = async {
        tokio::io::copy(&mut from_daemon, &mut stdout).await?;
        stdout.flush().await
    };
    tokio::select! {
        result = up => result.map(|()| ProxyEnd::ClientClosed),
        result = down => result.map(|()| {
            if saw_exit.get() {
                ProxyEnd::ClientExited
            } else {
                ProxyEnd::ServerClosed
            }
        }),
    }
}

/// Answer as a server with no diagnostics until the client leaves.
///
/// `initialize` gets minimal capabilities (open/close sync only, so the client tells us which
/// file it opens). The reason reaches the client twice: a `window/showMessage` (type 2) right
/// after `initialize`, and an Information diagnostic on the first file it opens, or on
/// `<root>/Cargo.toml` if it edits before opening anything (C4 amendment).
///
/// # Errors
///
/// An I/O error writing to the client.
pub async fn null_server<I, O>(
    mut stdin: I,
    mut stdout: O,
    reason: &str,
    root_cargo_toml_uri: &str,
) -> std::io::Result<()>
where
    I: AsyncRead + Unpin,
    O: AsyncWrite + Unpin,
{
    let text = format!("no diagnostics: {reason}");
    let mut decoder = Decoder::default();
    let mut noticed = false;
    loop {
        let message = match read_message(&mut stdin, &mut decoder).await {
            Ok(Some(message)) => message,
            Ok(None) => return Ok(()),
            Err(error) if error.is_recoverable() => continue,
            Err(_) => return Ok(()),
        };
        let method = message
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let id = message.get("id").filter(|id| !id.is_null()).cloned();
        match (method, id) {
            ("initialize", Some(id)) => {
                let result = json!({
                    "capabilities": {"textDocumentSync": {"openClose": true, "change": 0}},
                    "serverInfo": {"name": "epr-lsp-null"}
                });
                write_message(
                    &mut stdout,
                    &json!({"jsonrpc": "2.0", "id": id, "result": result}),
                )
                .await?;
                write_message(&mut stdout, &show_message(2, &text)).await?;
            }
            ("exit", None) => return Ok(()),
            (_, Some(id)) => {
                write_message(
                    &mut stdout,
                    &json!({"jsonrpc": "2.0", "id": id, "result": null}),
                )
                .await?;
            }
            ("textDocument/didOpen" | "textDocument/didChange" | "textDocument/didSave", None)
                if !noticed =>
            {
                let uri = if method == "textDocument/didOpen" {
                    message
                        .pointer("/params/textDocument/uri")
                        .and_then(Value::as_str)
                        .unwrap_or(root_cargo_toml_uri)
                } else {
                    root_cargo_toml_uri
                };
                noticed = true;
                let publish = json!({
                    "jsonrpc": "2.0",
                    "method": "textDocument/publishDiagnostics",
                    "params": {"uri": uri, "diagnostics": [notice_diagnostic(&text)]}
                });
                write_message(&mut stdout, &publish).await?;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::encode;

    async fn run_null(input: Vec<Value>) -> Vec<Value> {
        let bytes: Vec<u8> = input.iter().flat_map(encode).collect();
        let mut output = Vec::new();
        null_server(
            &bytes[..],
            &mut output,
            "daemon unreachable",
            "file:///r/Cargo.toml",
        )
        .await
        .unwrap();
        let mut decoder = Decoder::default();
        decoder.push(&output);
        let mut messages = Vec::new();
        while let Some(message) = decoder.next_message().unwrap() {
            messages.push(message);
        }
        messages
    }

    #[tokio::test]
    async fn null_server_answers_initialize_and_publishes_the_reason_on_first_open() {
        let out = run_null(vec![
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
            json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}),
            json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {"textDocument": {"uri": "file:///r/a.rs", "text": ""}}}),
            json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {"textDocument": {"uri": "file:///r/b.rs", "text": ""}}}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "shutdown"}),
            json!({"jsonrpc": "2.0", "method": "exit"}),
        ])
        .await;
        assert_eq!(out[0]["id"], 1);
        assert_eq!(out[1]["method"], "window/showMessage");
        assert_eq!(out[1]["params"]["type"], 2);
        assert_eq!(
            out[1]["params"]["message"],
            "no diagnostics: daemon unreachable"
        );
        assert_eq!(out[2]["params"]["uri"], "file:///r/a.rs");
        assert_eq!(out[2]["params"]["diagnostics"][0]["severity"], 3);
        assert_eq!(out[2]["params"]["diagnostics"][0]["source"], "epr-lsp");
        assert_eq!(
            out[3]["id"], 2,
            "the reason is published once; shutdown is answered"
        );
        assert_eq!(out.len(), 4);
    }

    #[tokio::test]
    async fn null_server_publishes_on_the_root_manifest_when_an_edit_comes_first() {
        let out = run_null(vec![
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
            json!({"jsonrpc": "2.0", "method": "textDocument/didSave", "params": {"textDocument": {"uri": "file:///r/x.rs"}}}),
        ])
        .await;
        assert_eq!(out[2]["params"]["uri"], "file:///r/Cargo.toml");
    }

    #[tokio::test]
    async fn proxy_tells_a_client_exit_from_a_server_death() {
        for exit_first in [true, false] {
            let (ours, theirs) = UnixStream::pair().unwrap();
            let (client_in, mut client_feed) = tokio::io::duplex(1 << 16);
            let (client_out, _client_read) = tokio::io::duplex(1 << 16);
            let daemon = tokio::spawn(async move {
                let mut theirs = theirs;
                let mut decoder = Decoder::default();
                if exit_first {
                    let _ = read_message(&mut theirs, &mut decoder).await;
                }
                drop(theirs);
            });
            if exit_first {
                client_feed
                    .write_all(&encode(&json!({"jsonrpc": "2.0", "method": "exit"})))
                    .await
                    .unwrap();
            }
            let end = proxy(client_in, client_out, ours).await.unwrap();
            daemon.await.unwrap();
            let expected = if exit_first {
                ProxyEnd::ClientExited
            } else {
                ProxyEnd::ServerClosed
            };
            assert_eq!(end, expected);
            drop(client_feed);
        }
    }
}

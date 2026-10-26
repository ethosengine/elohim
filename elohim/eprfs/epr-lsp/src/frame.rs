//! LSP base-protocol framing: `Content-Length: N\r\n\r\n` followed by N bytes of JSON.
//!
//! [`Decoder`] is pure: it accepts bytes as they arrive and yields whole message bodies. The two
//! async helpers wrap it around a reader and a writer.

use serde_json::Value;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// The largest body this multiplexer accepts ([`crate::limits::MAX_FRAME`]). A larger declared
/// length is refused rather than buffered, so a corrupt header cannot make the daemon allocate
/// without bound.
pub const MAX_BODY: usize = crate::limits::MAX_FRAME;

/// The longest header block accepted before its terminating blank line.
const MAX_HEADER: usize = 8 << 10;

/// Why a frame could not be decoded.
#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    /// A header block ended without a `Content-Length` header.
    #[error("frame header has no Content-Length")]
    MissingLength,
    /// A header line or value could not be read; the stream cannot be resynchronised.
    #[error("malformed frame header: {0}")]
    BadHeader(String),
    /// The declared body is larger than [`MAX_BODY`].
    #[error("frame body of {0} bytes exceeds the {MAX_BODY}-byte limit")]
    TooLarge(usize),
    /// The body was framed correctly but is not JSON; the stream is still in sync.
    #[error("frame body is not JSON: {0}")]
    Json(#[from] serde_json::Error),
    /// The underlying reader failed.
    #[error("frame io: {0}")]
    Io(#[from] std::io::Error),
}

impl FrameError {
    /// Whether the stream can continue after this error. Only a JSON body error leaves the byte
    /// stream aligned on the next frame.
    #[must_use]
    pub fn is_recoverable(&self) -> bool {
        matches!(self, FrameError::Json(_))
    }
}

/// Serialize one message as a framed byte sequence.
#[must_use]
pub fn encode(message: &Value) -> Vec<u8> {
    let body = message.to_string();
    let mut bytes = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    bytes.extend_from_slice(body.as_bytes());
    bytes
}

/// An incremental frame decoder.
#[derive(Debug, Default)]
pub struct Decoder {
    buffer: Vec<u8>,
}

impl Decoder {
    /// Append bytes read from the stream.
    pub fn push(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }

    /// Whether bytes of an unfinished frame are buffered.
    #[must_use]
    pub fn has_partial(&self) -> bool {
        !self.buffer.is_empty()
    }

    /// The next whole body, if one is buffered.
    ///
    /// # Errors
    ///
    /// A header error: the stream is out of sync and must be abandoned.
    pub fn next_body(&mut self) -> Result<Option<Vec<u8>>, FrameError> {
        let Some(end) = find(&self.buffer, b"\r\n\r\n") else {
            if self.buffer.len() > MAX_HEADER {
                return Err(FrameError::BadHeader("header block too long".into()));
            }
            return Ok(None);
        };
        let header = std::str::from_utf8(&self.buffer[..end])
            .map_err(|e| FrameError::BadHeader(e.to_string()))?;
        let mut length = None;
        for line in header.split("\r\n") {
            let Some((name, value)) = line.split_once(':') else {
                return Err(FrameError::BadHeader(format!("`{line}`")));
            };
            if name.trim().eq_ignore_ascii_case("content-length") {
                let parsed: usize = value
                    .trim()
                    .parse()
                    .map_err(|_| FrameError::BadHeader(format!("Content-Length `{value}`")))?;
                length = Some(parsed);
            }
        }
        let length = length.ok_or(FrameError::MissingLength)?;
        if length > MAX_BODY {
            return Err(FrameError::TooLarge(length));
        }
        let start = end + 4;
        if self.buffer.len() < start + length {
            return Ok(None);
        }
        let body = self.buffer[start..start + length].to_vec();
        self.buffer.drain(..start + length);
        Ok(Some(body))
    }

    /// The next whole message, if one is buffered.
    ///
    /// # Errors
    ///
    /// A header error (fatal for the stream) or a JSON error (the body is consumed and the
    /// stream may continue; see [`FrameError::is_recoverable`]).
    pub fn next_message(&mut self) -> Result<Option<Value>, FrameError> {
        match self.next_body()? {
            Some(body) => Ok(Some(serde_json::from_slice(&body)?)),
            None => Ok(None),
        }
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Read the next message. `Ok(None)` is a clean end of stream between frames.
///
/// # Errors
///
/// An I/O error, a header error, a JSON body error, or an end of stream inside a frame.
pub async fn read_message<R: AsyncRead + Unpin>(
    reader: &mut R,
    decoder: &mut Decoder,
) -> Result<Option<Value>, FrameError> {
    let mut chunk = [0_u8; 16 << 10];
    loop {
        if let Some(message) = decoder.next_message()? {
            return Ok(Some(message));
        }
        let read = reader.read(&mut chunk).await?;
        if read == 0 {
            if decoder.has_partial() {
                return Err(FrameError::Io(std::io::ErrorKind::UnexpectedEof.into()));
            }
            return Ok(None);
        }
        decoder.push(&chunk[..read]);
    }
}

/// Write one framed message and flush it.
///
/// # Errors
///
/// The writer's I/O error.
pub async fn write_message<W: AsyncWrite + Unpin>(
    writer: &mut W,
    message: &Value,
) -> std::io::Result<()> {
    writer.write_all(&encode(message)).await?;
    writer.flush().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn round_trips_a_message_split_across_pushes() {
        let message = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize"});
        let bytes = encode(&message);
        let mut decoder = Decoder::default();
        for byte in &bytes {
            assert!(decoder.next_message().unwrap().is_none());
            decoder.push(std::slice::from_ref(byte));
        }
        assert_eq!(decoder.next_message().unwrap(), Some(message));
        assert!(!decoder.has_partial());
    }

    #[test]
    fn decodes_two_frames_in_one_push_and_ignores_other_headers() {
        let mut bytes = b"Content-Type: application/vscode-jsonrpc; charset=utf-8\r\n".to_vec();
        bytes.extend_from_slice(&encode(&json!({"a": 1})));
        bytes.extend_from_slice(&encode(&json!({"b": 2})));
        let mut decoder = Decoder::default();
        decoder.push(&bytes);
        assert_eq!(decoder.next_message().unwrap(), Some(json!({"a": 1})));
        assert_eq!(decoder.next_message().unwrap(), Some(json!({"b": 2})));
        assert_eq!(decoder.next_message().unwrap(), None);
    }

    #[test]
    fn header_without_length_is_fatal_and_bad_json_is_recoverable() {
        let mut decoder = Decoder::default();
        decoder.push(b"X: 1\r\n\r\n{}");
        assert!(matches!(
            decoder.next_message(),
            Err(FrameError::MissingLength)
        ));

        let mut decoder = Decoder::default();
        decoder.push(b"Content-Length: 3\r\n\r\nnot");
        decoder.push(&encode(&json!(1)));
        let error = decoder.next_message().unwrap_err();
        assert!(error.is_recoverable(), "{error}");
        assert_eq!(decoder.next_message().unwrap(), Some(json!(1)));
    }

    #[test]
    fn oversized_length_is_refused_before_buffering() {
        let mut decoder = Decoder::default();
        decoder.push(format!("Content-Length: {}\r\n\r\n", MAX_BODY + 1).as_bytes());
        assert!(matches!(decoder.next_body(), Err(FrameError::TooLarge(_))));
    }

    #[tokio::test]
    async fn async_read_reports_clean_and_torn_ends() {
        let bytes = encode(&json!({"x": true}));
        let mut reader = &bytes[..];
        let mut decoder = Decoder::default();
        assert_eq!(
            read_message(&mut reader, &mut decoder).await.unwrap(),
            Some(json!({"x": true}))
        );
        assert!(read_message(&mut reader, &mut decoder)
            .await
            .unwrap()
            .is_none());

        let torn = &bytes[..bytes.len() - 1];
        let mut reader = torn;
        let mut decoder = Decoder::default();
        assert!(read_message(&mut reader, &mut decoder).await.is_err());
    }
}

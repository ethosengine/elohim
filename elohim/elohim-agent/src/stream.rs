//! Streaming response support.
//!
//! Provides token streaming for real-time LLM responses.

use futures::Stream;
use pin_project_lite::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc;

use crate::backend::traits::{CompletionResponse, FinishReason, Usage};

/// A chunk of streamed response.
#[derive(Debug, Clone)]
pub struct StreamChunk {
    /// Token content
    pub content: String,
    /// Whether this is the final chunk
    pub is_final: bool,
    /// Finish reason (only on final chunk)
    pub finish_reason: Option<FinishReason>,
}

impl StreamChunk {
    /// Create a content chunk.
    pub fn content(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            is_final: false,
            finish_reason: None,
        }
    }

    /// Create a final chunk.
    pub fn final_chunk(content: impl Into<String>, reason: FinishReason) -> Self {
        Self {
            content: content.into(),
            is_final: true,
            finish_reason: Some(reason),
        }
    }
}

pin_project! {
    /// Stream of tokens from LLM completion.
    pub struct TokenStream {
        #[pin]
        receiver: mpsc::Receiver<StreamChunk>,
        // Accumulated content (for getting full response)
        accumulated: String,
        // Whether stream is complete
        complete: bool,
        // Final usage (available after stream completes)
        usage: Option<Usage>,
    }
}

impl TokenStream {
    /// Create a new token stream.
    pub fn new(receiver: mpsc::Receiver<StreamChunk>) -> Self {
        Self {
            receiver,
            accumulated: String::new(),
            complete: false,
            usage: None,
        }
    }

    /// Create a token stream from a complete response (for non-streaming backends).
    pub fn from_complete(response: CompletionResponse) -> Self {
        let (tx, rx) = mpsc::channel(1);

        // Spawn task to send the complete response as a single chunk
        tokio::spawn(async move {
            let _ = tx
                .send(StreamChunk::final_chunk(response.content, response.finish_reason))
                .await;
        });

        Self {
            receiver: rx,
            accumulated: String::new(),
            complete: false,
            usage: Some(response.usage),
        }
    }

    /// Create a sender/receiver pair for streaming.
    pub fn channel(buffer: usize) -> (TokenStreamSender, Self) {
        let (tx, rx) = mpsc::channel(buffer);
        let sender = TokenStreamSender { sender: tx };
        let stream = Self::new(rx);
        (sender, stream)
    }

    /// Get accumulated content so far.
    pub fn accumulated(&self) -> &str {
        &self.accumulated
    }

    /// Check if stream is complete.
    pub fn is_complete(&self) -> bool {
        self.complete
    }

    /// Get usage (available after stream completes).
    pub fn usage(&self) -> Option<&Usage> {
        self.usage.as_ref()
    }

    /// Collect all chunks into a complete response.
    pub async fn collect(mut self) -> CompletionResponse {
        use futures::StreamExt;

        let mut finish_reason = FinishReason::Stop;

        while let Some(chunk) = self.next().await {
            if let Some(reason) = chunk.finish_reason {
                finish_reason = reason;
            }
        }

        CompletionResponse {
            content: self.accumulated,
            finish_reason,
            usage: self.usage.unwrap_or_default(),
        }
    }
}

impl Stream for TokenStream {
    type Item = StreamChunk;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        match this.receiver.poll_recv(cx) {
            Poll::Ready(Some(chunk)) => {
                // Accumulate content
                this.accumulated.push_str(&chunk.content);

                if chunk.is_final {
                    *this.complete = true;
                }

                Poll::Ready(Some(chunk))
            }
            Poll::Ready(None) => {
                *this.complete = true;
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Sender for a token stream.
pub struct TokenStreamSender {
    sender: mpsc::Sender<StreamChunk>,
}

impl TokenStreamSender {
    /// Send a content chunk.
    pub async fn send(&self, content: impl Into<String>) -> Result<(), StreamError> {
        self.sender
            .send(StreamChunk::content(content))
            .await
            .map_err(|_| StreamError::Closed)
    }

    /// Send the final chunk.
    pub async fn finish(
        self,
        content: impl Into<String>,
        reason: FinishReason,
    ) -> Result<(), StreamError> {
        self.sender
            .send(StreamChunk::final_chunk(content, reason))
            .await
            .map_err(|_| StreamError::Closed)
    }

    /// Close without sending final content.
    pub async fn close(self) {
        drop(self.sender);
    }
}

/// Error during streaming.
#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    /// Stream was closed
    #[error("Stream closed")]
    Closed,
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_token_stream() {
        let (sender, mut stream) = TokenStream::channel(10);

        // Spawn sender
        tokio::spawn(async move {
            sender.send("Hello").await.unwrap();
            sender.send(", ").await.unwrap();
            sender.send("world").await.unwrap();
            sender.finish("!", FinishReason::Stop).await.unwrap();
        });

        // Collect chunks
        let mut chunks = Vec::new();
        while let Some(chunk) = stream.next().await {
            chunks.push(chunk);
        }

        assert_eq!(chunks.len(), 4);
        assert_eq!(stream.accumulated(), "Hello, world!");
        assert!(stream.is_complete());
    }

    #[tokio::test]
    async fn test_from_complete() {
        let response = CompletionResponse {
            content: "Complete response".to_string(),
            finish_reason: FinishReason::Stop,
            usage: Usage::default(),
        };

        let stream = TokenStream::from_complete(response);
        let collected = stream.collect().await;

        assert_eq!(collected.content, "Complete response");
    }
}

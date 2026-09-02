//! A bounded ring of the most recently observed output lines.

use std::collections::VecDeque;

/// Bounded FIFO of output lines, lifted from elohim-storage
/// process_manager.rs 264ce8ce4; storage delegates here in S1.
#[derive(Debug)]
pub struct RingBuffer {
    capacity: usize,
    lines: VecDeque<String>,
}

impl RingBuffer {
    /// Creates an empty ring retaining at most `capacity` lines.
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            lines: VecDeque::with_capacity(capacity),
        }
    }

    /// Appends a line, evicting the oldest line when the ring is full.
    pub fn push(&mut self, line: String) {
        if self.lines.len() >= self.capacity {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }

    /// Returns the last `n` lines oldest-first.
    pub fn last_n(&self, n: usize) -> Vec<String> {
        let skip = self.lines.len().saturating_sub(n);
        self.lines.iter().skip(skip).cloned().collect()
    }

    /// Returns the number of lines currently retained.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Returns whether the ring currently holds no lines.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_buffer_keeps_the_last_n_and_drops_the_oldest() {
        let mut ring = RingBuffer::new(3);
        assert_eq!(ring.last_n(10), Vec::<String>::new(), "empty ring");

        ring.push("a".to_string());
        ring.push("b".to_string());
        assert_eq!(ring.last_n(10), vec!["a".to_string(), "b".to_string()]);

        // Pushing past capacity evicts the oldest line first (FIFO), never a
        // middle or newest one.
        ring.push("c".to_string());
        ring.push("d".to_string());
        assert_eq!(
            ring.last_n(10),
            vec!["b".to_string(), "c".to_string(), "d".to_string()],
            "oldest ('a') dropped once capacity was exceeded"
        );

        // last_n caps the tail even when the ring holds more.
        assert_eq!(ring.last_n(2), vec!["c".to_string(), "d".to_string()]);
        assert_eq!(ring.last_n(0), Vec::<String>::new());
    }
}

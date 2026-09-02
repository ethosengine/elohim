//! Threaded readers for child-process output pipes.

use std::{
    fs::File,
    io::{BufRead, BufReader, Read, Write},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

use ark_core::RingBuffer;

/// The shared observations produced by one output-reader thread.
pub struct StreamTap {
    /// Bounded tail of lines read from the stream.
    pub ring: Arc<Mutex<RingBuffer>>,
    /// Readiness needles observed in stream order.
    pub matched: Arc<Mutex<Vec<String>>>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl StreamTap {
    /// Waits for the underlying reader to reach EOF.
    pub fn join(&self) -> thread::Result<()> {
        let Some(thread) = self.thread.lock().expect("stream thread lock").take() else {
            return Ok(());
        };
        thread.join()
    }
}

/// Starts one plain operating-system thread that fans a byte stream into a ring, optional
/// append log, and readiness-needle matches.
pub fn spawn_line_reader<R: Read + Send + 'static>(
    name: &'static str,
    reader: R,
    ring_lines: usize,
    mut log: Option<File>,
    needles: Vec<String>,
) -> StreamTap {
    let ring = Arc::new(Mutex::new(RingBuffer::new(ring_lines)));
    let matched = Arc::new(Mutex::new(Vec::new()));
    let thread_ring = Arc::clone(&ring);
    let thread_matched = Arc::clone(&matched);
    let thread = thread::Builder::new()
        .name(format!("ark-{name}-reader"))
        .spawn(move || {
            for line in BufReader::new(reader).lines() {
                let Ok(line) = line else {
                    break;
                };

                thread_ring
                    .lock()
                    .expect("stream ring lock")
                    .push(line.clone());

                if let Some(file) = log.as_mut() {
                    if writeln!(file, "{line}").is_err() {
                        log = None;
                    }
                }

                let seen: Vec<String> = needles
                    .iter()
                    .filter(|needle| line.contains(needle.as_str()))
                    .cloned()
                    .collect();
                if !seen.is_empty() {
                    thread_matched
                        .lock()
                        .expect("stream matcher lock")
                        .extend(seen);
                }
            }
        })
        .expect("spawn pipe reader thread");

    StreamTap {
        ring,
        matched,
        thread: Mutex::new(Some(thread)),
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Cursor};

    use super::*;

    #[test]
    fn line_reader_fills_ring_and_matches_needle() {
        let directory = tempfile::tempdir().unwrap();
        let log_path = directory.path().join("stdout.log");
        let log = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .unwrap();
        let tap = spawn_line_reader(
            "stdout",
            Cursor::new("a\nConductor ready.\nb\n"),
            10,
            Some(log),
            vec!["Conductor ready.".to_string()],
        );

        tap.join().unwrap();

        assert_eq!(
            tap.ring.lock().unwrap().last_n(10),
            vec![
                "a".to_string(),
                "Conductor ready.".to_string(),
                "b".to_string(),
            ]
        );
        assert_eq!(
            *tap.matched.lock().unwrap(),
            vec!["Conductor ready.".to_string()]
        );
        assert_eq!(
            fs::read_to_string(log_path).unwrap(),
            "a\nConductor ready.\nb\n"
        );
    }
}

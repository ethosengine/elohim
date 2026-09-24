//! Integration-test binary for the peer-side content search lane
//! (plan Lane S, `recall-reaches-authority` L2 rung).
//!
//! One binary, one link of the lib — the same aggregation shape as
//! `tests/api/main.rs`. Each module below is a slice of the search lane.

mod fts5_probe;

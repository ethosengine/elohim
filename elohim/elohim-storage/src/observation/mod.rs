//! Observation/Event Layer — peer-witnessed evidence on Track 2 substrate.
//!
//! See `genesis/docs/superpowers/specs/2026-05-11-observation-event-layer-design.md`.
//! Source of truth for an observation is the observer's iroh-blob log; the SQL
//! projection is rebuildable by log replay. This module owns the wire format.

pub mod wire;

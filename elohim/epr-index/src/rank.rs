//! RANK — cosine, the printed score, and each unit's best chunk. Pure.
//!
//! Lifted out of the recall executor's semantic route (post-station-4 sprint, ruling R-S1). A
//! *unit* is whatever a fold keys its chunks by: a repository-relative path in the executor, a
//! content id in the storage peer. The ordering is unchanged: within a unit the higher score
//! wins and the earlier chunk (lower id) breaks a tie; across units the printed (4-decimal)
//! score orders, then the unit id, so two runs over one store print one order.
use std::collections::BTreeMap;

/// The cosine of two vectors; 0 when either has no length or their widths differ.
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    let (mut dot, mut na, mut nb) = (0f32, 0f32, 0f32);
    for (x, y) in a.iter().zip(b) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na.sqrt() * nb.sqrt())
    }
}

/// A score at the 4 decimals every candidate prints (the lexical route prints its own at the same
/// precision).
pub fn rounded(score: f64) -> f64 {
    (score * 10_000.0).round() / 10_000.0
}

/// One unit's best chunk.
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub unit_id: String,
    pub id: i64,
    pub score: f64,
}

/// Each unit's best chunk, offered one chunk at a time. Within a unit the higher score (a cosine
/// on the semantic route, a negated `bm25()` on the lexical route) wins and the earlier chunk
/// (lower id) breaks a tie; across units [`BestPerUnit::ranked`] orders by the printed
/// (4-decimal) score, then by unit id, so two runs over one store print one order.
#[derive(Debug, Default)]
pub struct BestPerUnit {
    best: BTreeMap<String, (f64, i64)>,
}

impl BestPerUnit {
    pub fn offer(&mut self, id: i64, unit_id: &str, score: f64) {
        match self.best.get_mut(unit_id) {
            Some((kept, kept_id)) => {
                if score > *kept || (score == *kept && id < *kept_id) {
                    *kept = score;
                    *kept_id = id;
                }
            }
            None => {
                self.best.insert(unit_id.to_string(), (score, id));
            }
        }
    }

    pub fn ranked(self) -> Vec<Hit> {
        let mut hits: Vec<Hit> = self
            .best
            .into_iter()
            .map(|(unit_id, (score, id))| Hit { unit_id, id, score })
            .collect();
        hits.sort_by(|a, b| {
            rounded(b.score)
                .total_cmp(&rounded(a.score))
                .then_with(|| a.unit_id.cmp(&b.unit_id))
        });
        hits
    }
}

/// A vector as the store keeps it: little-endian `f32`s, one after another.
pub fn encode(vector: &[f32]) -> Vec<u8> {
    vector.iter().flat_map(|x| x.to_le_bytes()).collect()
}

/// A stored vector (little-endian `f32` × `dims`), or `None` when its width is not the measure's.
pub fn decode(blob: &[u8], dims: usize) -> Option<Vec<f32>> {
    if dims == 0 || blob.len() != dims * 4 {
        return None;
    }
    Some(
        blob.as_chunks::<4>()
            .0
            .iter()
            .map(|b| f32::from_le_bytes(*b))
            .collect(),
    )
}

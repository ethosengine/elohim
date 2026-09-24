//! EMBEDDER — the embedding boundary: the [`Embedder`] trait, the [`EmbedBudget`] a call runs
//! under, and the [`Fixture`] embedder for tests.
//!
//! A runtime that embeds (the recall executor's pinned Python fold procedure; a native runtime
//! later) implements [`Embedder`] on its own side of this boundary. The budget is resolved by the
//! caller from whatever it declares its envelopes in — this crate reads no contract.
//!
//! Lifted out of the recall executor (post-station-4 sprint, ruling R-S1): every body is
//! unchanged; only the error type moved from `FlowError` to [`IndexError`].
use sha2::{Digest, Sha256};

use crate::error::{IndexError, Result};

/// The fixture embedder's width — the pinned model's, so fixture folds interchange with live ones.
pub const FIXTURE_DIMS: usize = 384;

/// What a fixture embedding may claim, declared like the `fixture` provider's own line.
pub const FIXTURE_FITNESS: &str =
    "fixture embedder only; test interchange, no live provider fitness established";

/// One reply: a unit-length vector per text, in order, and what the vectors may claim.
#[derive(Debug, Clone, PartialEq)]
pub struct Embedding {
    pub dims: usize,
    pub vectors: Vec<Vec<f32>>,
    pub fitness: String,
    /// How many texts of this reply were longer than the model's token window and embedded from
    /// their head only; `None` when the procedure does not count (unknown — never a guessed 0).
    pub truncated: Option<usize>,
}

/// The envelope one `embed` call runs under — resolved by the caller from what it declares.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EmbedBudget {
    pub bytes: usize,
    pub seconds: f64,
    /// The most texts one call may carry.
    pub texts: usize,
}

/// Texts in, one vector per text out, under the budget the caller resolved.
pub trait Embedder {
    fn embed(&self, texts: &[String], budget: EmbedBudget) -> Result<Embedding>;

    /// `embed`, also saying whether an embedding PROCESS actually ran — the metering fact a
    /// caller charges on (a process that ran and then failed still cost its seconds; a call
    /// refused before any spawn cost nothing). An in-process embedder spawns nothing.
    fn embed_metered(&self, texts: &[String], budget: EmbedBudget) -> (Result<Embedding>, bool) {
        (self.embed(texts, budget), false)
    }
}

/// A batch larger than the budget allows is the caller's error, refused before anything runs.
pub fn within_batch(texts: &[String], budget: EmbedBudget) -> Result<()> {
    if texts.len() > budget.texts {
        return Err(IndexError::Refused(format!(
            "{} texts exceed the embedding budget's batch of {}",
            texts.len(),
            budget.texts
        )));
    }
    Ok(())
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Fixture — deterministic hashed bag-of-words; test interchange only, never live fitness
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// Each lower-cased alphanumeric word adds ±1 at a sha256-chosen coordinate of a 384-wide vector,
/// then the vector is L2-normalised. Texts sharing words land near each other, which is all a
/// test of the fold or the ranking needs; it knows nothing a model knows.
pub struct Fixture;

impl Fixture {
    pub fn vector(text: &str) -> Vec<f32> {
        fn add(vector: &mut [f32], token: &[u8]) {
            let digest = Sha256::digest(token);
            let mut index = [0u8; 8];
            index.copy_from_slice(&digest[..8]);
            let slot = (u64::from_le_bytes(index) % FIXTURE_DIMS as u64) as usize;
            vector[slot] += if digest[8] & 1 == 0 { 1.0 } else { -1.0 };
        }
        let mut vector = vec![0f32; FIXTURE_DIMS];
        let lower = text.to_lowercase();
        for word in lower.split(|c: char| !c.is_alphanumeric()) {
            if !word.is_empty() {
                add(&mut vector, word.as_bytes());
            }
        }
        // No words, or words that cancelled out: the whole text is one token, so every text
        // still has a unit vector.
        if vector.iter().all(|x| *x == 0.0) {
            add(&mut vector, text.as_bytes());
        }
        let norm = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        vector.iter_mut().for_each(|x| *x /= norm);
        vector
    }
}

impl Embedder for Fixture {
    fn embed(&self, texts: &[String], budget: EmbedBudget) -> Result<Embedding> {
        within_batch(texts, budget)?;
        Ok(Embedding {
            dims: FIXTURE_DIMS,
            vectors: texts.iter().map(|text| Fixture::vector(text)).collect(),
            fitness: FIXTURE_FITNESS.to_string(),
            // A bag of words has no token window: nothing is ever cut.
            truncated: Some(0),
        })
    }
}

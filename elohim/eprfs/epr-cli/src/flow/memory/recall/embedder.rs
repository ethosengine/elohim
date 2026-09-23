//! EMBEDDER — the embedding step of the semantic route: the [`Embedder`] trait, the
//! [`PinnedProcedure`] that runs the declared fold procedure, and the [`Fixture`] embedder for
//! tests (governed-discovery station 4, task 4.2). No Rust ONNX runtime is reachable from this
//! workspace, so embedding is a **declared fold procedure**: `embedder/embed.py`, pinned by its
//! own CID in the model manifest, run through [`bounded_process`] under a budget the pinned
//! contract declares. A native runtime later is a new procedure version behind the same trait.
//!
//! **The method is checked at call time, never inferred.** Before and while it runs, the
//! procedure's own bytes, the model directory, the interpreter, the Python modules and the model
//! bytes are each checked against what the manifest pins; any that fails is reported as
//! [`FlowError::Unavailable`] (`unavailable: <reason>`) — a route that cannot run here, not a
//! fault in the question. A caller's own misuse (a batch over its budget) is a refusal.
//!
//! **Two budgets, both declared.** [`EmbedBudget::query`] is the provider envelope for embedding
//! one question on the query path (`provider_bytes`/`provider_seconds`); [`EmbedBudget::fold`] is
//! the fold procedure's own envelope off the query path (`fold_procedure_bytes`,
//! `fold_procedure_seconds`, `fold_batch_texts`). A 32-text batch of 384-float vectors is ~130 KB
//! of JSON and cannot fit the query envelope, so the fold never borrows it.
use super::*;
use serde::Deserialize;

/// The governed model manifest the semantic measure's `ModelPin` names.
pub const MODEL_MANIFEST_REL: &str =
    ".epr-meta/elohim/algorithms/embedding-models/all-minilm-l6-v2.json";

/// The declared fold procedure, pinned by the manifest's `procedure` CID.
pub const PROCEDURE_REL: &str = "elohim/eprfs/epr-cli/embedder/embed.py";

/// An operator override naming the interpreter that runs the procedure (default `python3`).
pub const INTERPRETER_ENV: &str = "EPR_EMBED_PYTHON";

/// The fixture embedder's width — the pinned model's, so fixture folds interchange with live ones.
pub const FIXTURE_DIMS: usize = 384;

/// What a fixture embedding may claim, declared like the `fixture` provider's own line.
pub const FIXTURE_FITNESS: &str =
    "fixture embedder only; test interchange, no live provider fitness established";

/// `embed.py`'s exit codes (its module doc is the other half of this contract).
const EXIT_BAD_REQUEST: i32 = 2;
const EXIT_PIN_MISMATCH: i32 = 3;

/// One reply: a unit-length vector per text, in order, and what the vectors may claim.
#[derive(Debug, Clone, PartialEq)]
pub struct Embedding {
    pub dims: usize,
    pub vectors: Vec<Vec<f32>>,
    pub fitness: String,
}

/// The envelope one `embed` call runs under — resolved from the pinned contract by the caller.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EmbedBudget {
    pub bytes: usize,
    pub seconds: f64,
    /// The most texts one call may carry.
    pub texts: usize,
}

impl EmbedBudget {
    /// Embedding one question on the query path: the provider envelope.
    pub fn query(contract: &Contract) -> FlowResult<Self> {
        Ok(Self {
            bytes: declared_limit(contract, "provider_bytes", true)? as usize,
            seconds: declared_limit(contract, "provider_seconds", false)?,
            texts: 1,
        })
    }

    /// One fold batch, off the query path: the fold procedure's own declared envelope.
    pub fn fold(contract: &Contract) -> FlowResult<Self> {
        Ok(Self {
            bytes: declared_limit(contract, "fold_procedure_bytes", true)? as usize,
            seconds: declared_limit(contract, "fold_procedure_seconds", false)?,
            texts: declared_limit(contract, "fold_batch_texts", true)? as usize,
        })
    }
}

/// A budget the contract must declare, positive and finite — the same refusal `Contract::validate`
/// gives its required limits, raised here because only an embedding caller needs these.
fn declared_limit(contract: &Contract, name: &str, integer: bool) -> FlowResult<f64> {
    let raw = contract.value.pointer(&format!("/limits/{name}"));
    let value = raw
        .and_then(Value::as_f64)
        .filter(|v| v.is_finite() && *v > 0.0)
        .ok_or_else(|| refused(format!("invalid positive budget: {name}")))?;
    if integer && !raw.is_some_and(Value::is_u64) {
        return Err(refused(format!(
            "byte/count budgets must be integers: {name}"
        )));
    }
    Ok(value)
}

/// Texts in, one vector per text out, under the budget the caller resolved from the contract.
pub trait Embedder {
    fn embed(&self, texts: &[String], budget: EmbedBudget) -> FlowResult<Embedding>;
}

fn within_batch(texts: &[String], budget: EmbedBudget) -> FlowResult<()> {
    if texts.len() > budget.texts {
        return Err(refused(format!(
            "{} texts exceed the embedding budget's batch of {}",
            texts.len(),
            budget.texts
        )));
    }
    Ok(())
}

fn unavailable(reason: impl Into<String>) -> FlowError {
    FlowError::Unavailable(reason.into())
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// The model manifest
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// The fields of the governed model manifest the procedure is run against.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelManifest {
    pub artifact_type: String,
    pub model_bytes: String,
    pub tokenizer_bytes: String,
    pub dims: usize,
    /// Model directories in order: `$VAR` (skipped when unset), `~` (`$HOME`), or a path. The
    /// first one that exists wins.
    pub resolve: Vec<String>,
    /// The raw CID of the fold procedure's bytes.
    pub procedure: Option<String>,
    /// The fold that judges this model.
    pub fitness: String,
}

impl ModelManifest {
    pub fn load(path: &Path) -> FlowResult<Self> {
        let raw = std::fs::read(path).map_err(|source| FlowError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        let manifest: Self = serde_json::from_slice(&raw)?;
        if manifest.artifact_type != "model-manifest" {
            return Err(refused(format!(
                "{} is not a model-manifest",
                path.display()
            )));
        }
        Ok(manifest)
    }

    /// The first `resolve` entry that names an existing directory, reading this process's env.
    pub fn resolve_model_dir(&self) -> Option<PathBuf> {
        self.resolve_with(|name| std::env::var(name).ok())
    }

    fn resolve_with(&self, var: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
        self.resolve
            .iter()
            .filter_map(|entry| expand_entry(entry, &var))
            .find(|dir| dir.is_dir())
    }
}

/// One `resolve` entry as a path: `$VAR[/rest]` (None when VAR is unset or empty), `~[/rest]`
/// (None without `$HOME`), anything else verbatim.
fn expand_entry(entry: &str, var: &impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let (base, rest) = if let Some(named) = entry.strip_prefix('$') {
        let (name, rest) = named.split_once('/').unwrap_or((named, ""));
        (var(name)?, rest)
    } else if entry == "~" || entry.starts_with("~/") {
        (var("HOME")?, entry[1..].trim_start_matches('/'))
    } else {
        return Some(PathBuf::from(entry));
    };
    if base.is_empty() {
        return None;
    }
    let base = PathBuf::from(base);
    Some(if rest.is_empty() {
        base
    } else {
        base.join(rest)
    })
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// PinnedProcedure — the declared fold procedure behind the bounded envelope
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// Runs `procedure` with `interpreter` against the model the manifest pins.
pub struct PinnedProcedure {
    pub manifest: ModelManifest,
    pub procedure: PathBuf,
    pub interpreter: String,
}

impl PinnedProcedure {
    /// The declared procedure and manifest under `root`; the interpreter is `$EPR_EMBED_PYTHON`
    /// when set, else `python3` from PATH.
    pub fn declared(root: &Path) -> FlowResult<Self> {
        Ok(Self {
            manifest: ModelManifest::load(&root.join(MODEL_MANIFEST_REL))?,
            procedure: root.join(PROCEDURE_REL),
            interpreter: interpreter_from(std::env::var(INTERPRETER_ENV).ok()),
        })
    }

    /// The procedure is part of the method: bytes on disk that are not the pinned bytes never run.
    fn verify_procedure(&self) -> FlowResult<()> {
        let pin = self
            .manifest
            .procedure
            .as_deref()
            .ok_or_else(|| unavailable("the model manifest pins no procedure"))?;
        let bytes = std::fs::read(&self.procedure).map_err(|error| {
            unavailable(format!(
                "procedure {} unreadable: {error}",
                self.procedure.display()
            ))
        })?;
        if BlobCid::compute_raw(&bytes).to_string() != pin {
            return Err(unavailable("procedure bytes do not match the pin"));
        }
        Ok(())
    }

    fn reply(&self, stdout: &[u8], expected: usize) -> FlowResult<Embedding> {
        #[derive(Deserialize)]
        struct Reply {
            dims: usize,
            vectors: Vec<Vec<f32>>,
        }
        let reply: Reply = serde_json::from_slice(stdout).map_err(|error| {
            unavailable(format!(
                "embedding procedure replied malformed JSON: {error}"
            ))
        })?;
        let dims = self.manifest.dims;
        if reply.dims != dims {
            return Err(unavailable(format!(
                "embedding procedure replied {} dims; the manifest pins {dims}",
                reply.dims
            )));
        }
        if reply.vectors.len() != expected {
            return Err(unavailable(format!(
                "embedding procedure replied {} vectors for {expected} texts",
                reply.vectors.len()
            )));
        }
        if let Some(short) = reply.vectors.iter().find(|v| v.len() != dims) {
            return Err(unavailable(format!(
                "embedding procedure replied a vector of {} floats; the manifest pins {dims}",
                short.len()
            )));
        }
        Ok(Embedding {
            dims,
            vectors: reply.vectors,
            fitness: format!(
                "{} on model {}",
                self.manifest.fitness, self.manifest.model_bytes
            ),
        })
    }
}

/// `$EPR_EMBED_PYTHON` when it names something, else `python3` from PATH.
pub fn interpreter_from(configured: Option<String>) -> String {
    configured
        .filter(|program| !program.trim().is_empty())
        .unwrap_or_else(|| "python3".to_string())
}

impl Embedder for PinnedProcedure {
    fn embed(&self, texts: &[String], budget: EmbedBudget) -> FlowResult<Embedding> {
        within_batch(texts, budget)?;
        self.verify_procedure()?;
        let model_dir = self.manifest.resolve_model_dir().ok_or_else(|| {
            unavailable(format!(
                "no model directory resolves from the manifest's resolve list ({})",
                self.manifest.resolve.join(", ")
            ))
        })?;
        let request = serde_json::to_vec(&json!({
            "model_dir": model_dir.to_string_lossy(),
            "pin": {
                "model_bytes": self.manifest.model_bytes,
                "tokenizer_bytes": self.manifest.tokenizer_bytes,
            },
            "texts": texts,
        }))?;
        // `-B`: the procedure writes nothing, not even bytecode caches.
        let args = vec![
            "-B".to_string(),
            self.procedure.to_string_lossy().into_owned(),
        ];
        let outcome = bounded_process(
            &self.interpreter,
            &args,
            budget.bytes,
            budget.seconds,
            Some(&request),
        )
        .map_err(|error| {
            unavailable(format!(
                "interpreter `{}` cannot run: {error}",
                self.interpreter
            ))
        })?;
        if let Some(reason) = outcome.error {
            return Err(unavailable(format!("embedding procedure: {reason}")));
        }
        let said = String::from_utf8_lossy(&outcome.stderr)
            .lines()
            .next()
            .unwrap_or_default()
            .trim()
            .to_string();
        match outcome.status {
            Some(0) => self.reply(&outcome.stdout, texts.len()),
            Some(EXIT_PIN_MISMATCH) => Err(unavailable("model bytes do not match the pin")),
            Some(EXIT_BAD_REQUEST) => match said.strip_prefix("unavailable: ") {
                Some(reason) => Err(unavailable(reason)),
                None => Err(refused(format!(
                    "the embedding procedure refused the request: {said}"
                ))),
            },
            Some(code) => Err(unavailable(format!(
                "embedding procedure failed (exit {code}): {said}"
            ))),
            None => Err(unavailable(format!(
                "embedding procedure ended by a signal: {said}"
            ))),
        }
    }
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
    fn embed(&self, texts: &[String], budget: EmbedBudget) -> FlowResult<Embedding> {
        within_batch(texts, budget)?;
        Ok(Embedding {
            dims: FIXTURE_DIMS,
            vectors: texts.iter().map(|text| Fixture::vector(text)).collect(),
            fitness: FIXTURE_FITNESS.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(resolve: &[&str]) -> ModelManifest {
        ModelManifest {
            artifact_type: "model-manifest".into(),
            model_bytes: String::new(),
            tokenizer_bytes: String::new(),
            dims: FIXTURE_DIMS,
            resolve: resolve.iter().map(|s| s.to_string()).collect(),
            procedure: None,
            fitness: String::new(),
        }
    }

    #[test]
    fn resolve_is_explicit_first_skips_unset_vars_and_expands_home() {
        let explicit = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(home.path().join(".cache/model")).unwrap();
        let declared = manifest(&["$EPR_EMBED_MODEL_DIR", "~/.cache/model"]);
        let env = |explicit: Option<&Path>| {
            let explicit = explicit.map(|p| p.to_string_lossy().into_owned());
            let home = home.path().to_string_lossy().into_owned();
            move |name: &str| match name {
                "EPR_EMBED_MODEL_DIR" => explicit.clone(),
                "HOME" => Some(home.clone()),
                _ => None,
            }
        };
        assert_eq!(
            declared.resolve_with(env(Some(explicit.path()))),
            Some(explicit.path().to_path_buf()),
            "an operator's explicit directory wins"
        );
        assert_eq!(
            declared.resolve_with(env(None)),
            Some(home.path().join(".cache/model")),
            "an unset variable is skipped; `~` is $HOME"
        );
        let absent = explicit.path().join("absent");
        assert_eq!(
            declared.resolve_with(env(Some(&absent))),
            Some(home.path().join(".cache/model")),
            "a directory that does not exist is passed over"
        );
        assert_eq!(declared.resolve_with(|_| None), None);
        assert_eq!(
            expand_entry("$DIR/onnx", &|_| Some("/models".into())),
            Some(PathBuf::from("/models/onnx"))
        );
        assert_eq!(expand_entry("$DIR", &|_| Some(String::new())), None);
    }

    #[test]
    fn the_interpreter_is_python3_unless_the_operator_names_another() {
        assert_eq!(interpreter_from(None), "python3");
        assert_eq!(interpreter_from(Some("  ".into())), "python3");
        assert_eq!(
            interpreter_from(Some("/nonexistent".into())),
            "/nonexistent"
        );
    }

    #[test]
    fn fixture_vectors_share_direction_with_shared_words() {
        let a = Fixture::vector("rebuild the stale index");
        let b = Fixture::vector("Rebuild the STALE index!");
        let c = Fixture::vector("garden stewardship");
        assert_eq!(a, b, "case and punctuation do not change the words");
        let dot = |x: &[f32], y: &[f32]| x.iter().zip(y).map(|(p, q)| p * q).sum::<f32>();
        assert!(dot(&a, &c) < dot(&a, &b));
    }
}

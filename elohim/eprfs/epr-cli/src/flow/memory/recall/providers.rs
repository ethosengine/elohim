//! `Provider` — the shape any candidate-discovery source implements — and the two the pinned
//! recipe declares: `local` (deterministic bounded filesystem traversal, always present, wraps
//! [`discovery::discover_scored`]) and `mempalace` (an optional external process, bounded by
//! bytes and seconds, honest about what it cannot claim). `providers_for` reads the pinned
//! contract's own `ceremony.providers` and returns them in the recipe's declared order.
//!
//! Moved out of `mod.rs` (governed-discovery station zero, task 0.4): `ProcessOutcome`,
//! `bounded_process` and `process_result` are the same bytes as before, just relocated next to
//! the provider that is their only real consumer. The trait, `LocalLexical`, `MemPalace` and
//! `providers_for` are new — `retrieve()` (still in `mod.rs`, still the explicit-provider
//! dispatcher every currently-declared `--provider` test exercises) is untouched, so its rich,
//! per-kind JSON shape (`candidates`/`groups`/`selection`/`omissions`/…) stays byte-identical.
//!
//! **`providers_for` is a naming lookup, not a trial run — today.** The `search` operation's
//! *default* provider (no `--provider`, no persisted session choice) reads `providers_for(...)
//! .first().id()` to name the declared default (today: `"local"`) and then runs the SAME single
//! `retrieve()` traversal every other path does. It deliberately does NOT call
//! `Provider::candidates` to "check" a provider first — that would run `discover_scored` a
//! second time (once to check, once inside `retrieve()` to actually answer) and double-charge
//! `view["usage"]` for one `search` (2026-09-11 review finding on this task). `Provider::candidates`
//! and [`ProviderResult`] therefore have no production caller yet — only this file's own unit
//! tests — until a richer caller (task 0.5's `journey.rs`) needs to compare more than one
//! provider's actual answer, at which point calling it once, deliberately, to compare (not to
//! probe-then-redo) is the right shape.
use super::discovery::discover_scored;
use super::*;

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Provider
// ───────────────────────────────────────────────────────────────────────────────────────────────

pub(super) type ProviderId = String;

/// One provider's answer to one question: what it ranked (or merely returned), whether the
/// ranking is a known method or an honestly unknown one, the method pinned to it (if any), and
/// what asking it cost.
///
/// No production code constructs or reads this yet (see the module doc — `providers_for` names a
/// provider without asking it anything); it is exercised by this file's own unit tests, ahead of
/// a caller (task 0.5's `journey.rs`) that actually needs one provider's real answer.
#[allow(dead_code)]
pub(super) struct ProviderResult {
    pub ranked: Vec<Value>,
    pub ranking_known: bool,
    pub method: Option<String>,
    pub usage: Value,
}

/// A source of candidates the pinned recipe may declare under `ceremony.providers`. `id` names it
/// the way that map does; `candidates` produces its answer for one question, scoped under
/// `session_root`.
pub(super) trait Provider {
    fn id(&self) -> ProviderId;
    // See the module doc: exercised by this file's unit tests, not yet by production code.
    #[allow(dead_code)]
    fn candidates(
        &self,
        terms: &[String],
        scope: &Path,
        contract: &Contract,
        session_root: &Path,
    ) -> FlowResult<ProviderResult>;
}

/// Resolve a `scope` the caller may already have made absolute (a session root, or a directory
/// under it) back into the root-relative string `discover_scored` understands — `.` when it
/// names `session_root` itself, otherwise the path beneath it. A `scope` that is already relative
/// (the CLI's own `--search-scope`) passes through unchanged.
///
/// A `LocalLexical::candidates` helper — dead in production today for the same reason
/// `Provider::candidates` is (see the module doc).
#[allow(dead_code)]
fn scope_string(scope: &Path, session_root: &Path) -> String {
    let relative = if scope.is_absolute() {
        scope.strip_prefix(session_root).unwrap_or(scope)
    } else {
        scope
    };
    let text = relative.to_string_lossy().into_owned();
    if text.is_empty() {
        ".".to_string()
    } else {
        text
    }
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// LocalLexical — deterministic bounded traversal, the always-declared default
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// The same traversal `discover`/`discover_scored` have always run, wrapped so it can be asked in
/// the same shape as any other provider. Its ranking method IS the pinned contract — that is what
/// `method` names — because ranking over declared metadata is the algorithm the contract itself
/// describes, not a third-party claim about it.
pub(super) struct LocalLexical;

impl Provider for LocalLexical {
    fn id(&self) -> ProviderId {
        "local".to_string()
    }

    fn candidates(
        &self,
        terms: &[String],
        scope: &Path,
        contract: &Contract,
        session_root: &Path,
    ) -> FlowResult<ProviderResult> {
        let scope = scope_string(scope, session_root);
        let found = discover_scored(
            session_root,
            contract,
            &scope,
            "",
            terms,
            &[],
            "directory",
            "*.md",
        )?;
        Ok(ProviderResult {
            ranked: found["candidates"].as_array().cloned().unwrap_or_default(),
            ranking_known: true,
            method: Some(contract.method_cid()),
            usage: found["usage"].clone(),
        })
    }
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Bounded subprocess — the only shape in which a foreign provider or lens may speak
// ───────────────────────────────────────────────────────────────────────────────────────────────

pub struct ProcessOutcome {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub status: Option<i32>,
    pub error: Option<String>,
}

/// Bound bytes and seconds before buffering foreign output, including stderr; kill on excess.
pub fn bounded_process(
    program: &str,
    args: &[String],
    output_limit: usize,
    seconds: f64,
) -> std::io::Result<ProcessOutcome> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let cap = output_limit + 1;
    let mut out = child.stdout.take().expect("piped");
    let mut err = child.stderr.take().expect("piped");
    let out_handle = std::thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = out.by_ref().take(cap as u64).read_to_end(&mut buffer);
        buffer
    });
    let err_handle = std::thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = err.by_ref().take(cap as u64).read_to_end(&mut buffer);
        buffer
    });
    let began = Instant::now();
    let mut reason = None;
    let mut status = None;
    loop {
        match child.try_wait()? {
            Some(exit) => {
                status = exit.code();
                break;
            }
            None => {
                if began.elapsed().as_secs_f64() >= seconds {
                    reason = Some("provider timed out".to_string());
                    let _ = child.kill();
                    let _ = child.wait();
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
    let stdout = out_handle.join().unwrap_or_default();
    let stderr = err_handle.join().unwrap_or_default();
    if reason.is_none() && stdout.len() + stderr.len() > output_limit {
        reason = Some("provider output exceeds budget; results withheld".into());
    }
    Ok(ProcessOutcome {
        stdout,
        stderr,
        status,
        error: reason,
    })
}

pub(super) fn process_result(program: &str, args: &[String], contract: &Contract) -> Value {
    let outcome = match bounded_process(
        program,
        args,
        contract.limit_usize("provider_bytes"),
        contract.limit_secs("provider_seconds"),
    ) {
        Ok(outcome) => outcome,
        Err(error) => {
            return json!({
                "unresolved": [format!("provider unavailable: {error}")],
                "usage": {"search_queries": 1},
                "candidate_text": "",
            })
        }
    };
    let failure = outcome
        .error
        .clone()
        .or_else(|| (outcome.status != Some(0)).then(|| "provider failed".to_string()));
    json!({
        "usage": {"search_queries": 1, "provider_bytes": outcome.stdout.len() + outcome.stderr.len()},
        "unresolved": failure.clone().map(|f| vec![f]).unwrap_or_default(),
        "candidate_text": if failure.is_some() { String::new() }
                          else { String::from_utf8_lossy(&outcome.stdout).to_string() },
    })
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// MemPalace — the optional external provider, honest about its unknowns
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// A separately installed `mempalace` executable, searched over bounded stdout/stderr with a
/// timeout. Declared `optional` in the recipe; absent, slow or over-budget output all collapse to
/// an empty, honestly-unranked result rather than an error — a missing optional provider is not a
/// fault in the question, and `bounded_process`/`process_result` already say so.
///
/// `palace` is the recipe-declared location, relative to `session_root` unless it is already
/// absolute (the shape a test double supplies to point at a fixed, possibly nonexistent, path).
/// Read only inside `candidates` (see the module doc — not yet called by production code).
pub(super) struct MemPalace {
    #[allow(dead_code)]
    pub palace: PathBuf,
}

impl Provider for MemPalace {
    fn id(&self) -> ProviderId {
        "mempalace".to_string()
    }

    fn candidates(
        &self,
        terms: &[String],
        _scope: &Path,
        contract: &Contract,
        session_root: &Path,
    ) -> FlowResult<ProviderResult> {
        let palace = if self.palace.is_absolute() {
            self.palace.clone()
        } else {
            session_root.join(&self.palace)
        };
        let args = vec![
            "--palace".to_string(),
            palace.to_string_lossy().to_string(),
            "search".to_string(),
            terms.join(" "),
            "--results".to_string(),
            contract.limit_usize("search_results").to_string(),
        ];
        let result = process_result("mempalace", &args, contract);
        let candidate_text = result["candidate_text"].as_str().unwrap_or_default();
        let ranked = if candidate_text.is_empty() {
            Vec::new()
        } else {
            vec![json!({"candidate_text": candidate_text})]
        };
        Ok(ProviderResult {
            ranked,
            ranking_known: false,
            method: None,
            usage: result["usage"].clone(),
        })
    }
}

/// The providers this executor knows how to speak to, in the pinned recipe's own
/// `ceremony.providers` order (an object, so — absent `preserve_order` — alphabetical; `local` <
/// `mempalace` either way): `local` whenever declared with kind `local` (contract validation
/// requires at least one provider, and every pinned recipe has always named this one), `mempalace`
/// whenever declared with kind `mempalace`. A declared `fixture` provider (test interchange only,
/// never live-fit) is not a `Provider` and is not returned here; `retrieve()` still answers it
/// directly by name.
///
/// **A coupling to keep in step:** each returned `Provider`'s `id()` is a hard-coded literal
/// (`"local"`/`"mempalace"`) matched by `kind`, not by the declaration's own JSON key —
/// `retrieve()` then looks that `id()` up by KEY at `/ceremony/providers/{id}`. Every pinned
/// recipe today names its provider keys identically to their kinds, so the two agree; a recipe
/// that ever declared a `kind: "local"` provider under some other key would make that lookup
/// refuse ("provider is not declared by the pinned recipe") even though `providers_for` found it.
pub(super) fn providers_for(contract: &Contract) -> Vec<Box<dyn Provider>> {
    let mut providers: Vec<Box<dyn Provider>> = Vec::new();
    let Some(declared) = contract
        .value
        .pointer("/ceremony/providers")
        .and_then(Value::as_object)
    else {
        return providers;
    };
    for declaration in declared.values() {
        match declaration.get("kind").and_then(Value::as_str) {
            Some("local") => providers.push(Box::new(LocalLexical)),
            Some("mempalace") => providers.push(Box::new(MemPalace {
                palace: PathBuf::from(".mempalace/palace"),
            })),
            _ => {}
        }
    }
    providers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_lexical_declares_its_ranking_known_and_pins_the_contract_method() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("a.md"),
            "---\ntitle: alpha\ndescription: mempalace re-mine\n---\nbody\n",
        )
        .unwrap();
        let contract =
            Contract::from_value(crate::flow::memory::recall::tests_support::minimal_contract())
                .unwrap();
        let out = LocalLexical
            .candidates(&["mempalace".into()], dir.path(), &contract, dir.path())
            .unwrap();
        assert!(out.ranking_known);
        assert_eq!(out.method.as_deref(), Some(contract.method_cid().as_str()));
        assert_eq!(out.ranked.len(), 1);
    }

    #[test]
    fn mempalace_declares_its_ranking_unknown() {
        let p = MemPalace {
            palace: PathBuf::from("/nonexistent"),
        };
        assert_eq!(p.id(), "mempalace");
        // without a binary the provider returns an empty, honest result rather than an error
        let contract =
            Contract::from_value(crate::flow::memory::recall::tests_support::minimal_contract())
                .unwrap();
        let out = p
            .candidates(&["x".into()], Path::new("."), &contract, Path::new("."))
            .unwrap();
        assert!(!out.ranking_known);
        assert!(out.ranked.is_empty());
    }
}

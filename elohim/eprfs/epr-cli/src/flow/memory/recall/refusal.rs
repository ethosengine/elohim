//! The one refusal envelope: what went wrong, which session it concerns, and what to do.
use super::*;

/// The flags an operation actually accepts, so a refusal teaches the surface instead of ending it.
///
/// A reader guessing flag names pays a refusal per guess, and the guess it needed was never in the
/// message. Naming them is one line of prose against an unbounded number of wrong tries.
pub(super) fn accepted_flags(operation: &str) -> &'static str {
    match operation {
        "open" => "--need --intent --purpose bootstrap --scope --limit --offset",
        "select" => "--edge --need",
        "read" => "--path --lines START:END --need",
        "source" => "--path --tag --query --search-scope --name --need",
        "search" => "--provider --query --tag --search-scope --name --need",
        "remember" => {
            "--finding --question --next-action --evidence path:START:END --classification"
        }
        "finish" => "--outcome --question",
        "prepare" => "--kind --finding",
        "measure" => "--phase baseline|close --measure-scope",
        "resume" | "context" => "--need --section --context-pin --evidence --evidence-offset",
        "adopt" => "--from-session --need",
        "history" | "compare" => "--limit --offset",
        _ => "--session --need --json --root --contract (see `recall --help`)",
    }
}

/// The one refusal envelope: what went wrong, which session it concerns, and what to do about it.
///
/// Two renderings of ONE fact. `--json` keeps the envelope a machine reader parses; a human (or an
/// agent reading a terminal) gets exactly two lines, because the remedy is the only part of a
/// refusal that changes what the caller does next, and it was previously buried under ~900 bytes
/// of re-printed orientation the caller had already read.
pub(super) fn print_refusal(
    message: &str,
    session: &str,
    output_limit: Option<usize>,
    json_output: bool,
) -> FlowResult<ExitCode> {
    let remedy = remedy_for(message, session);
    if !json_output {
        print!("{}", refusal_lines(message, &remedy));
        return Ok(ExitCode::from(2));
    }
    let failure = json!({
        "unresolved": [truncate(message, 1000)],
        "session": session,
        "next": remedy,
        "accounting": "Session unavailable; refusal outside session accounting.",
    });
    let mut encoded = serde_json::to_string(&failure)?;
    if output_limit.is_some_and(|limit| encoded.len() > limit) {
        encoded = serde_json::to_string(
            &json!({"unresolved": ["request refused; context exceeds output budget"]}),
        )?;
    }
    println!("{encoded}");
    Ok(ExitCode::from(2))
}

/// The human refusal: the fault, then the one thing to do about it. Nothing else.
pub(super) fn refusal_lines(message: &str, remedy: &str) -> String {
    format!(
        "refused: {}\nnext: {}\n",
        one_line(&truncate(message, 1000)),
        one_line(remedy)
    )
}

/// Collapse authored whitespace so a multi-line constant still prints as one line.
pub(super) fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The `next` line for a refusal: the remedy for the fault that ACTUALLY fired.
///
/// One remedy for every refusal is worse than none. A caller who typed a path outside the declared
/// scope, and is told "method changes require explicit adopt into a new session", learns nothing
/// about their own mistake and learns something false about the executor's state. The classes here
/// are matched on the refusal's own words, which is exactly as durable as the words are — and they
/// are constants in this module, not prose that drifts.
pub(super) fn remedy_for(message: &str, session: &str) -> String {
    if message.contains("cannot read the habit register") {
        return "The register is a GENERATED projection, never hand-edited — re-project it, then \
                retry: python3 .claude/scripts/habits-project.py"
            .into();
    }
    if message.contains("algorithm bytes changed") || message.contains("executor bytes changed") {
        return format!(
            "The pinned method changed. Retain this receipt and continue explicitly: \
             epr flow memory recall adopt --from-session {session} --session <new-session>"
        );
    }
    if message.contains("private recall record") {
        return "Recall exposes what was read and what was concluded. A receipt or continuation is                 never an input to any verb; name the source it was taken from instead."
            .into();
    }
    if message.contains("unknown flag") {
        return "The message names the flags this operation accepts. Re-run with one of them, or                 `epr flow memory recall --help` for the whole surface."
            .into();
    }
    if message.contains("this session has 0 receipts") {
        return "Read the passage the answer rests on first, then finish:                 recall read --path <path> --lines START:END, then                 recall finish --outcome <what you found> --question <what is still open>."
            .into();
    }
    if message.contains("another checkout's working tree") {
        return "`.claude/` is in scope so the tooling layer is reachable, but a sibling checkout                 is not this repository's source. Name the path inside this tree."
            .into();
    }
    if message.contains("outside declared source scope") {
        return "Name a path inside the contract's declared source_roots;                 `epr flow memory recall recipe` lists them."
            .into();
    }
    if message.contains("session is executing another packet") {
        return "Another packet holds this session's lock. Wait for it to finish, or open a                 differently named --session."
            .into();
    }
    if message.contains("no ceremony continuation exists") {
        return "Begin the ceremony: epr flow memory recall open --intent <why> [--scope <path>]."
            .into();
    }
    if message.contains("choose a concern from open first") {
        return "Open the concern page and choose a row: epr flow memory recall open, then                 select --edge <n>."
            .into();
    }
    if message.contains("adopt needs a new session") {
        return "Adoption cannot overwrite an investigation. Name a --session that has no                 continuation yet."
            .into();
    }
    if message.contains("intent/scope changed") {
        return format!(
            "Intent and scope are fixed at open. Continue this one unchanged, or start a new \
             session retaining this receipt: epr flow memory recall adopt --from-session {session} \
             --session <new-session>"
        );
    }
    if message.contains("measurement scope changed") {
        return "The measurement cohort is fixed at the baseline. Re-run without --measure-scope,                 or begin a new run for a new cohort."
            .into();
    }
    if message.contains("continuation exceeds state budget") {
        return format!(
            "This continuation is full. The prior receipt is intact: \
             epr flow memory recall adopt --from-session {session} --session <new-session>"
        );
    }
    "Read the refusal above and correct the named input; the session and its prior receipt are      unchanged."
        .into()
}

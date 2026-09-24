//! TERMS — a question's terms, the light suffix stem, and the FTS5 match expression.
//!
//! Lifted out of the recall executor (post-station-4 sprint, ruling R-S1) and decoupled from its
//! contract: the one thing the executor read from the contract here, the declared vocabulary of
//! short terms (`discovery.short_terms`), is now a parameter. Every body is otherwise unchanged.
use std::collections::BTreeSet;

/// Lower-level text with `_` and `-` read as word breaks and runs of whitespace collapsed, so a
/// phrase and an identifier spelling of the same words compare equal.
pub fn identifier_words(text: &str) -> String {
    text.replace(['_', '-'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Suffixes light stemming strips, longest first (`discovery.stemming = "suffix-strip-v1"`) —
/// tried in this order so an `"ings"`-shaped tail strips as `"ing"`, not as the plain `"s"` that
/// would leave a spurious trailing `"g"`.
pub const STEM_SUFFIXES: [&str; 4] = ["ing", "es", "ed", "s"];

/// The stem of `term` when stripping the first matching [`STEM_SUFFIXES`] entry leaves at least
/// four characters, else `term` itself. `"habits"` -> `"habit"` (kept: 5 chars); `"stamps"` ->
/// `"stamp"` (kept: 5 chars); `"cid"` (no matching suffix) -> `"cid"` unchanged; `"mined"` ->
/// `"min"` is BELOW the four-character floor, so it is left unstemmed rather than reduced to a
/// fragment common enough to match nearly everything. A literal suffix strip, not a lexical
/// dictionary — it will miss irregular inflections (`"prove"`/`"proof"`) it was never asked to
/// know, and that is the declared, modest shape of it.
pub fn stem(term: &str) -> &str {
    for suffix in STEM_SUFFIXES {
        if let Some(stripped) = term.strip_suffix(suffix) {
            if stripped.chars().count() >= 4 {
                return stripped;
            }
        }
    }
    term
}

/// Words that carry no area, so they never select a habit or a source.
pub const STOPWORDS: [&str; 49] = [
    "about", "after", "again", "against", "because", "before", "being", "between", "could", "does",
    "doing", "down", "from", "have", "here", "how", "into", "just", "like", "make", "more", "most",
    "much", "must", "only", "other", "over", "same", "should", "some", "such", "than", "that",
    "their", "them", "then", "there", "they", "this", "were", "what", "when", "where", "which",
    "while", "will", "with", "would", "your",
];

/// The question's distinctive terms: what an area match is made of.
///
/// A token under four characters is kept only when `allowed_short` (a recipe's declared
/// vocabulary of short terms) names it — that is what lets `top`/`red` survive tokenizing "Which
/// habit is top red right now" without every three-letter word in ordinary prose becoming a term.
/// Two ADJACENT declared-short tokens in the source text (`top red`) also mint the two-word phrase
/// as an additional term, matched as a phrase by the ordinary substring matching every other term
/// already uses — no separate phrase-matching code path.
pub fn question_terms(allowed_short: &BTreeSet<String>, need: &str) -> Vec<String> {
    let is_kept_short = |term: &str| -> bool {
        !term.is_empty()
            && term.len() < 4
            && allowed_short.contains(term)
            && !STOPWORDS.contains(&term)
    };
    let raw_tokens: Vec<String> = need
        .split(|c: char| {
            !(c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '/' || c == '.')
        })
        .map(|raw| {
            raw.trim_matches(|c| c == '.' || c == '/')
                .to_ascii_lowercase()
        })
        .collect();

    let mut seen: Vec<String> = Vec::new();
    for (index, term) in raw_tokens.iter().enumerate() {
        if term.is_empty() || STOPWORDS.contains(&term.as_str()) || seen.contains(term) {
            continue;
        }
        let keep = term.len() >= 4 || allowed_short.contains(term.as_str());
        if keep {
            seen.push(term.clone());
            if seen.len() >= 12 {
                break;
            }
        }
        if is_kept_short(term) {
            if let Some(next) = raw_tokens.get(index + 1) {
                if is_kept_short(next) {
                    let phrase = format!("{term} {next}");
                    if !seen.contains(&phrase) {
                        seen.push(phrase);
                        if seen.len() >= 12 {
                            break;
                        }
                    }
                }
            }
        }
    }
    seen
}

/// The shortest last token a prefix match extends: `stem`'s own floor, so a short declared term
/// (`top`, `red`) never matches a longer word that merely begins with it.
pub const PREFIX_FLOOR: usize = 4;

/// The FTS5 match expression for `terms`: each term with at least one letter or digit, its last
/// token stemmed, quoted as an FTS5 string (an inner `"` doubled), OR-joined. `None` when no term is left.
/// Quoting is what makes question text inert: inside a string, FTS5 reads only the tokenizer's
/// tokens — never an operator, a column filter or a `NEAR` group. The string is a prefix (`*`,
/// which applies to its last token) only when that last token has at least [`PREFIX_FLOOR`]
/// characters: a declared short term and a phrase of them match exactly.
pub fn match_expression(terms: &[String]) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    for term in terms {
        if !term.chars().any(char::is_alphanumeric) {
            continue;
        }
        // FTS5's `*` extends the string's LAST token, and a phrase's other tokens must match
        // exactly — so only the last token is stemmed (`top red` stays `top red`; stemming the
        // whole phrase would cut `red` to `r`). The tokenizer splits on everything that is not a
        // letter or digit, so trailing punctuation is no token and is dropped.
        let body = term.trim_end_matches(|c: char| !c.is_alphanumeric());
        let split = body
            .char_indices()
            .rev()
            .find(|(_, c)| !c.is_alphanumeric())
            .map_or(0, |(at, c)| at + c.len_utf8());
        let last = stem(&body[split..]);
        let prefix = if last.chars().count() >= PREFIX_FLOOR {
            "*"
        } else {
            ""
        };
        let text = format!("{}{last}", &body[..split]);
        let quoted = format!("\"{}\"{prefix}", text.replace('"', "\"\""));
        if !parts.contains(&quoted) {
            parts.push(quoted);
        }
    }
    (!parts.is_empty()).then(|| parts.join(" OR "))
}

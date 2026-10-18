//! What to say when a person types a word a command does not know: the word
//! they most likely meant, said beside the refusal, before the usage.

/// The candidate `word` most likely meant: the only candidate it begins, else
/// the nearest within two single-letter edits (one for a word of three
/// letters or fewer, where two edits reach almost anything). A tie names
/// nothing: a guess between two commands is worse than none.
pub fn nearest<'a>(word: &str, candidates: &[&'a str]) -> Option<&'a str> {
    let word = word.trim();
    if word.is_empty() {
        return None;
    }
    let begun: Vec<&str> = candidates
        .iter()
        .copied()
        .filter(|c| c.starts_with(word) && *c != word)
        .collect();
    if let [only] = begun.as_slice() {
        return Some(only);
    }
    let reach = if word.chars().count() <= 3 { 1 } else { 2 };
    let mut near: Vec<(usize, &str)> = candidates
        .iter()
        .map(|c| (edits(word, c), *c))
        .filter(|(d, _)| *d <= reach)
        .collect();
    near.sort_by_key(|(d, _)| *d);
    match near.as_slice() {
        [(d, c), (next, _), ..] if d < next => Some(c),
        [(_, c)] => Some(c),
        _ => None,
    }
}

/// `; did you mean `<said>`?`, where `said` is the likely candidate after
/// `before` (the command typed so far, or nothing for a flag); empty when
/// nothing is likely.
pub fn did_you_mean(word: &str, before: &str, candidates: &[&str]) -> String {
    nearest(word, candidates)
        .map(|c| meant(before, c))
        .unwrap_or_default()
}

/// The hint itself, for a caller that already knows the likely word.
pub fn meant(before: &str, candidate: &str) -> String {
    let space = if before.is_empty() { "" } else { " " };
    format!("; did you mean `{before}{space}{candidate}`?")
}

/// How many single-letter edits turn `a` into `b`.
fn edits(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut last = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let next = (last + usize::from(ca != cb))
                .min(row[j] + 1)
                .min(row[j + 1] + 1);
            last = row[j + 1];
            row[j + 1] = next;
        }
    }
    row[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLOW: [&str; 5] = ["seal", "reseal", "status", "stocks", "note"];

    #[test]
    fn a_near_word_names_its_command_and_a_far_one_names_nothing() {
        assert_eq!(nearest("stauts", &FLOW), Some("status"));
        assert_eq!(nearest("notes", &FLOW), Some("note"));
        assert_eq!(nearest("stat", &FLOW), Some("status"));
        assert_eq!(nearest("feedback", &FLOW), None);
        assert_eq!(nearest("", &FLOW), None);
    }

    #[test]
    fn a_word_between_two_commands_is_not_guessed() {
        assert_eq!(nearest("st", &FLOW), None, "begins both status and stocks");
        assert_eq!(nearest("seak", &["seal", "seat"]), None);
        assert_eq!(
            nearest("xy", &["ab", "cd"]),
            None,
            "a short word reaches one edit only"
        );
    }

    #[test]
    fn the_hint_says_the_whole_command_or_the_bare_flag() {
        assert_eq!(
            did_you_mean("stauts", "epr flow", &FLOW),
            "; did you mean `epr flow status`?"
        );
        assert_eq!(
            did_you_mean("--lable", "", &["--label", "--node"]),
            "; did you mean `--label`?"
        );
        assert_eq!(did_you_mean("feedback", "epr flow", &FLOW), "");
    }
}

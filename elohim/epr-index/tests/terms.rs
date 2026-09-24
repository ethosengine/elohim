//! Question terms, the suffix stem and the FTS5 match expression, decoupled from any contract.
use std::collections::BTreeSet;

use elohim_epr_index::terms::{identifier_words, match_expression, question_terms, stem};

fn terms(words: &[&str]) -> Vec<String> {
    words.iter().map(|w| w.to_string()).collect()
}

#[test]
fn match_expression_prefixes_4plus_and_quotes_short_terms() {
    assert_eq!(
        match_expression(&terms(&["folding", "stamps", "habit"])).as_deref(),
        Some("\"fold\"* OR \"stamp\"* OR \"habit\"*")
    );
    assert_eq!(
        match_expression(&terms(&["habits", "habit"])).as_deref(),
        Some("\"habit\"*")
    );
    assert_eq!(match_expression(&[]), None);
    assert_eq!(
        match_expression(&terms(&["habit", "top", "top red", "red", "wasm"])).as_deref(),
        Some("\"habit\"* OR \"top\" OR \"top red\" OR \"red\" OR \"wasm\"*")
    );
    assert_eq!(
        match_expression(&terms(&["stamps folding"])).as_deref(),
        Some("\"stamps fold\"*")
    );
    assert_eq!(match_expression(&terms(&["-", "./"])), None);
    // FTS5 syntax in a term stays inside its quoted string.
    assert_eq!(
        match_expression(&terms(&["near(ab", "b\" OR cdef", "col:x^"])).as_deref(),
        Some("\"near(ab\" OR \"b\"\" OR cdef\"* OR \"col:x\"")
    );
}

#[test]
fn stem_keeps_a_four_character_floor_and_identifier_words_join_spellings() {
    assert_eq!(stem("habits"), "habit");
    assert_eq!(stem("folding"), "fold");
    assert_eq!(stem("mined"), "mined");
    assert_eq!(stem("cid"), "cid");
    assert_eq!(identifier_words("fold_lag-bound  x"), "fold lag bound x");
}

#[test]
fn question_terms_keep_declared_short_terms_and_their_phrase() {
    let short: BTreeSet<String> = ["top", "red"].iter().map(|s| s.to_string()).collect();
    assert_eq!(
        question_terms(&short, "Which habit is top red right now"),
        terms(&["habit", "top", "top red", "red", "right"])
    );
    assert_eq!(
        question_terms(&BTreeSet::new(), "Which habit is top red right now"),
        terms(&["habit", "right"])
    );
}

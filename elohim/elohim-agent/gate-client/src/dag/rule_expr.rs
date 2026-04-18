//! Rule condition DSL for `MechanicalRulesetExecutor`.
//!
//! This is **distinct** from the edge expression language in [`super::expr`].
//! The differences:
//!
//! - Edge expressions (`expr.rs`) are 1–2 segment paths, `==`/`!=` only. They
//!   live on DAG edges and the interpreter checks them to route control flow.
//!   They are deliberately minimal to keep DAG traversal inspection trivial.
//!
//! - Rule conditions (this module) live inside a `gate-rules-declaration`
//!   artifact and express match conditions on assembled GateContext values.
//!   They allow `AND` conjunction and arbitrary-depth dotted paths because rule
//!   bodies need to test nested computed fields (e.g.,
//!   `priorAttestations.count == 0`, `priorAttestations.mostRecentStatus == "passed"`).
//!
//! # Grammar (informal)
//!
//! ```text
//! condition   ::= comparison (AND comparison)*
//! comparison  ::= path op literal
//! path        ::= ident ('.' ident)*          -- any depth ≥ 1
//! op          ::= '==' | '!='
//! literal     ::= 'null' | 'true' | 'false' | number | quoted-string
//! ```
//!
//! Unsupported: `OR`, `NOT`, arithmetic, function calls. Hard parse failure on
//! any unsupported syntax.
//!
//! # Examples
//!
//! ```
//! use gate_client::dag::rule_expr::{parse_rule_condition, evaluate_condition};
//! use gate_client::dag::context::GateContext;
//! use serde_json::json;
//!
//! let mut ctx = GateContext::new();
//! ctx.insert("moment", json!({"status": "passed"}));
//! ctx.insert("priorAttestations", json!({"count": 0}));
//!
//! let cond = parse_rule_condition(
//!     r#"moment.status == "passed" AND priorAttestations.count == 0"#
//! ).unwrap();
//! assert!(evaluate_condition(&cond, &ctx));
//! ```

use crate::dag::context::GateContext;
use crate::error::GateError;

// ─── AST ─────────────────────────────────────────────────────────────────────

/// A dotted path that resolves against the GateContext.
///
/// Unlike the edge expression language, any depth ≥ 1 is supported here
/// because rule conditions frequently access computed nested fields such as
/// `priorAttestations.mostRecentStatus`.
#[derive(Debug, Clone, PartialEq)]
pub struct RulePath(pub Vec<String>);

/// A literal value on the right-hand side of a comparison.
#[derive(Debug, Clone, PartialEq)]
pub enum RuleLiteral {
    Null,
    Bool(bool),
    Number(f64),
    Str(String),
}

impl RuleLiteral {
    /// Convert to a `serde_json::Value` for comparison.
    pub fn to_json_value(&self) -> serde_json::Value {
        match self {
            RuleLiteral::Null => serde_json::Value::Null,
            RuleLiteral::Bool(b) => serde_json::Value::Bool(*b),
            RuleLiteral::Number(n) => serde_json::Value::Number(
                serde_json::Number::from_f64(*n).unwrap_or_else(|| serde_json::Number::from(0u64)),
            ),
            RuleLiteral::Str(s) => serde_json::Value::String(s.clone()),
        }
    }
}

/// A single comparison clause.
#[derive(Debug, Clone, PartialEq)]
pub enum Comparison {
    /// `path == literal`
    Eq(RulePath, RuleLiteral),
    /// `path != literal`
    Ne(RulePath, RuleLiteral),
}

/// A rule condition — one or more comparison clauses joined by `AND`.
///
/// Evaluation short-circuits on the first false clause.
#[derive(Debug, Clone, PartialEq)]
pub struct RuleCondition {
    pub clauses: Vec<Comparison>,
}

// ─── Tokenizer ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Ident(String),
    Dot,
    EqEq,
    BangEq,
    And, // uppercase AND keyword
    StringLit(String),
    NumberLit(f64),
}

struct Tokenizer<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> Tokenizer<'a> {
    fn new(s: &'a str) -> Self {
        Self {
            input: s.as_bytes(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        let b = self.input.get(self.pos).copied();
        if b.is_some() {
            self.pos += 1;
        }
        b
    }

    fn skip_whitespace(&mut self) {
        while let Some(b) = self.peek() {
            if b == b' ' || b == b'\t' || b == b'\r' || b == b'\n' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn read_ident(&mut self) -> String {
        let start = self.pos;
        while let Some(b) = self.peek() {
            if b.is_ascii_alphanumeric() || b == b'_' || b == b'-' {
                self.advance();
            } else {
                break;
            }
        }
        String::from_utf8_lossy(&self.input[start..self.pos]).into_owned()
    }

    fn read_number(&mut self) -> Result<f64, GateError> {
        let start = self.pos;
        while let Some(b) = self.peek() {
            if b.is_ascii_digit() || b == b'.' || b == b'e' || b == b'E' || b == b'+' || b == b'-' {
                self.advance();
            } else {
                break;
            }
        }
        let s = String::from_utf8_lossy(&self.input[start..self.pos]).into_owned();
        s.parse::<f64>()
            .map_err(|_| GateError::DagExecution(format!("invalid number literal: {s}")))
    }

    fn read_quoted_string(&mut self, quote: u8) -> Result<String, GateError> {
        let mut result = String::new();
        loop {
            match self.advance() {
                None => {
                    return Err(GateError::DagExecution(
                        "unterminated string literal in rule condition".to_string(),
                    ))
                }
                Some(b'\\') => match self.advance() {
                    Some(b'"') => result.push('"'),
                    Some(b'\'') => result.push('\''),
                    Some(b'\\') => result.push('\\'),
                    Some(b'n') => result.push('\n'),
                    Some(b't') => result.push('\t'),
                    Some(b) => {
                        return Err(GateError::DagExecution(format!(
                            "unsupported escape \\{} in rule condition",
                            b as char
                        )))
                    }
                    None => {
                        return Err(GateError::DagExecution(
                            "unexpected end after backslash in rule condition".to_string(),
                        ))
                    }
                },
                Some(b) if b == quote => break,
                Some(b) => result.push(b as char),
            }
        }
        Ok(result)
    }

    fn tokenize(mut self) -> Result<Vec<Token>, GateError> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace();
            match self.peek() {
                None => break,
                Some(b'.') => {
                    self.advance();
                    tokens.push(Token::Dot);
                }
                Some(b'=') => {
                    self.advance();
                    match self.advance() {
                        Some(b'=') => tokens.push(Token::EqEq),
                        _ => {
                            return Err(GateError::DagExecution(
                                "expected '==' but got single '=' in rule condition".to_string(),
                            ))
                        }
                    }
                }
                Some(b'!') => {
                    self.advance();
                    match self.advance() {
                        Some(b'=') => tokens.push(Token::BangEq),
                        _ => {
                            return Err(GateError::DagExecution(
                                "expected '!=' but got lone '!' in rule condition".to_string(),
                            ))
                        }
                    }
                }
                Some(b'"') => {
                    self.advance();
                    let s = self.read_quoted_string(b'"')?;
                    tokens.push(Token::StringLit(s));
                }
                Some(b'\'') => {
                    self.advance();
                    let s = self.read_quoted_string(b'\'')?;
                    tokens.push(Token::StringLit(s));
                }
                Some(b) if b.is_ascii_digit() => {
                    let n = self.read_number()?;
                    tokens.push(Token::NumberLit(n));
                }
                Some(b'-') => {
                    self.advance();
                    if self.peek().map(|b| b.is_ascii_digit()).unwrap_or(false) {
                        let n = self.read_number()?;
                        tokens.push(Token::NumberLit(-n));
                    } else {
                        return Err(GateError::DagExecution(
                            "unexpected '-' in rule condition (not followed by digit)".to_string(),
                        ));
                    }
                }
                Some(b) if b.is_ascii_alphabetic() || b == b'_' => {
                    let ident = self.read_ident();
                    // Classify uppercase AND keyword.
                    if ident == "AND" {
                        tokens.push(Token::And);
                    } else {
                        tokens.push(Token::Ident(ident));
                    }
                }
                Some(b) => {
                    return Err(GateError::DagExecution(format!(
                        "unexpected character '{}' in rule condition",
                        b as char
                    )))
                }
            }
        }
        Ok(tokens)
    }
}

// ─── Parser ──────────────────────────────────────────────────────────────────

struct Parser<'t> {
    tokens: &'t [Token],
    pos: usize,
}

impl<'t> Parser<'t> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<&Token> {
        let t = self.tokens.get(self.pos);
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn parse(&mut self) -> Result<RuleCondition, GateError> {
        if self.peek().is_none() {
            return Err(GateError::DagExecution(
                "empty rule condition string".to_string(),
            ));
        }

        let mut clauses = Vec::new();

        loop {
            let clause = self.parse_comparison()?;
            clauses.push(clause);

            match self.peek() {
                None => break,
                Some(Token::And) => {
                    self.advance(); // consume AND
                                    // loop to parse next clause
                }
                Some(other) => {
                    return Err(GateError::DagExecution(format!(
                        "expected AND or end of condition, got {other:?}; \
                         OR and NOT are not supported in rule conditions"
                    )));
                }
            }
        }

        Ok(RuleCondition { clauses })
    }

    fn parse_comparison(&mut self) -> Result<Comparison, GateError> {
        let path = self.parse_path()?;

        let op = self.advance().ok_or_else(|| {
            GateError::DagExecution("expected '==' or '!=' in rule condition".to_string())
        })?;
        let op = op.clone();

        let literal = self.parse_literal()?;

        match op {
            Token::EqEq => Ok(Comparison::Eq(path, literal)),
            Token::BangEq => Ok(Comparison::Ne(path, literal)),
            other => Err(GateError::DagExecution(format!(
                "unsupported operator {other:?} in rule condition; only '==' and '!=' are allowed"
            ))),
        }
    }

    fn parse_path(&mut self) -> Result<RulePath, GateError> {
        let mut segments = Vec::new();

        match self.advance() {
            Some(Token::Ident(s)) => segments.push(s.clone()),
            other => {
                return Err(GateError::DagExecution(format!(
                    "expected identifier at start of path in rule condition, got {other:?}"
                )))
            }
        }

        // Consume optional `.segment` chains — any depth allowed.
        loop {
            if self.peek() == Some(&Token::Dot) {
                self.advance();
                match self.advance() {
                    Some(Token::Ident(s)) => segments.push(s.clone()),
                    other => {
                        return Err(GateError::DagExecution(format!(
                            "expected identifier after '.' in rule path, got {other:?}"
                        )))
                    }
                }
            } else {
                break;
            }
        }

        Ok(RulePath(segments))
    }

    fn parse_literal(&mut self) -> Result<RuleLiteral, GateError> {
        match self.advance() {
            Some(Token::Ident(s)) if s == "null" => Ok(RuleLiteral::Null),
            Some(Token::Ident(s)) if s == "true" => Ok(RuleLiteral::Bool(true)),
            Some(Token::Ident(s)) if s == "false" => Ok(RuleLiteral::Bool(false)),
            Some(Token::StringLit(s)) => Ok(RuleLiteral::Str(s.clone())),
            Some(Token::NumberLit(n)) => Ok(RuleLiteral::Number(*n)),
            Some(Token::Ident(s)) => Err(GateError::DagExecution(format!(
                "expected literal (null/true/false/string/number) in rule condition, \
                 got identifier '{s}'"
            ))),
            other => Err(GateError::DagExecution(format!(
                "expected literal in rule condition, got {other:?}"
            ))),
        }
    }
}

// ─── Public parse entry point ─────────────────────────────────────────────────

/// Parse a rule `when` string into a [`RuleCondition`].
///
/// Returns `Err(GateError::DagExecution(_))` for any malformed or unsupported
/// condition. Hard failures prevent unknown logic from silently passing.
pub fn parse_rule_condition(s: &str) -> Result<RuleCondition, GateError> {
    let tokens = Tokenizer::new(s).tokenize()?;
    let mut parser = Parser {
        tokens: &tokens,
        pos: 0,
    };
    parser.parse()
}

// ─── Evaluator ───────────────────────────────────────────────────────────────

/// Resolve a [`RulePath`] of arbitrary depth against a [`GateContext`].
///
/// - 1-segment path: `ctx.get("key")`
/// - 2+ segment path: descend through nested JSON objects, returning
///   `Value::Null` at any missing or non-object intermediate.
fn resolve_rule_path<'c>(
    path: &RulePath,
    ctx: &'c GateContext,
) -> std::borrow::Cow<'c, serde_json::Value> {
    static NULL: serde_json::Value = serde_json::Value::Null;

    let segs = &path.0;
    if segs.is_empty() {
        return std::borrow::Cow::Borrowed(&NULL);
    }

    let first = ctx.get(&segs[0]).unwrap_or(&NULL);

    if segs.len() == 1 {
        return std::borrow::Cow::Borrowed(first);
    }

    // Descend through remaining segments.
    let mut current = first;
    for seg in &segs[1..] {
        match current {
            serde_json::Value::Object(map) => {
                current = map.get(seg.as_str()).unwrap_or(&NULL);
            }
            _ => return std::borrow::Cow::Borrowed(&NULL),
        }
    }
    std::borrow::Cow::Borrowed(current)
}

/// Compare a resolved JSON `Value` against a [`RuleLiteral`].
///
/// Numeric comparison is performed as f64 to avoid integer-vs-float
/// mismatches in JSON (e.g., `json!(0)` and `json!(0.0)` are distinct
/// `serde_json::Value` variants but represent the same number).
fn values_equal(context_val: &serde_json::Value, lit: &RuleLiteral) -> bool {
    match (context_val, lit) {
        // Both numeric — compare as f64.
        (serde_json::Value::Number(n), RuleLiteral::Number(lit_f)) => {
            n.as_f64().map(|v| v == *lit_f).unwrap_or(false)
        }
        // All other types — use exact value equality after conversion.
        _ => *context_val == lit.to_json_value(),
    }
}

/// Evaluate a single [`Comparison`] clause against a [`GateContext`].
fn evaluate_comparison(cmp: &Comparison, ctx: &GateContext) -> bool {
    match cmp {
        Comparison::Eq(path, lit) => {
            let val = resolve_rule_path(path, ctx);
            values_equal(&val, lit)
        }
        Comparison::Ne(path, lit) => {
            let val = resolve_rule_path(path, ctx);
            !values_equal(&val, lit)
        }
    }
}

/// Evaluate a [`RuleCondition`] (all clauses must hold — short-circuit AND).
pub fn evaluate_condition(cond: &RuleCondition, ctx: &GateContext) -> bool {
    cond.clauses.iter().all(|c| evaluate_comparison(c, ctx))
}

/// Parse and evaluate in one step.
pub fn eval_rule_condition(when: &str, ctx: &GateContext) -> Result<bool, GateError> {
    let cond = parse_rule_condition(when)?;
    Ok(evaluate_condition(&cond, ctx))
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ctx(pairs: &[(&str, serde_json::Value)]) -> GateContext {
        let mut c = GateContext::new();
        for (k, v) in pairs {
            c.insert(*k, v.clone());
        }
        c
    }

    // ─── Parser: basic comparisons ────────────────────────────────────────────

    #[test]
    fn parse_single_eq_null() {
        let cond = parse_rule_condition("x == null").unwrap();
        assert_eq!(cond.clauses.len(), 1);
        assert_eq!(
            cond.clauses[0],
            Comparison::Eq(RulePath(vec!["x".into()]), RuleLiteral::Null)
        );
    }

    #[test]
    fn parse_single_ne_string() {
        let cond = parse_rule_condition(r#"status != "failed""#).unwrap();
        assert_eq!(cond.clauses.len(), 1);
        assert_eq!(
            cond.clauses[0],
            Comparison::Ne(
                RulePath(vec!["status".into()]),
                RuleLiteral::Str("failed".into())
            )
        );
    }

    #[test]
    fn parse_two_segment_path() {
        let cond = parse_rule_condition(r#"moment.status == "passed""#).unwrap();
        assert_eq!(
            cond.clauses[0],
            Comparison::Eq(
                RulePath(vec!["moment".into(), "status".into()]),
                RuleLiteral::Str("passed".into())
            )
        );
    }

    #[test]
    fn parse_three_segment_path_is_allowed() {
        // Rule conditions permit arbitrary depth — unlike edge expressions.
        let cond = parse_rule_condition("a.b.c == null").unwrap();
        assert_eq!(cond.clauses.len(), 1);
        assert_eq!(
            cond.clauses[0],
            Comparison::Eq(
                RulePath(vec!["a".into(), "b".into(), "c".into()]),
                RuleLiteral::Null
            )
        );
    }

    #[test]
    fn parse_and_conjunction() {
        let cond =
            parse_rule_condition(r#"moment.status == "passed" AND priorAttestations.count == 0"#)
                .unwrap();
        assert_eq!(cond.clauses.len(), 2);
        assert_eq!(
            cond.clauses[0],
            Comparison::Eq(
                RulePath(vec!["moment".into(), "status".into()]),
                RuleLiteral::Str("passed".into())
            )
        );
        assert_eq!(
            cond.clauses[1],
            Comparison::Eq(
                RulePath(vec!["priorAttestations".into(), "count".into()]),
                RuleLiteral::Number(0.0)
            )
        );
    }

    #[test]
    fn parse_three_clause_and() {
        let cond = parse_rule_condition("a == null AND b == true AND c.d == 42").unwrap();
        assert_eq!(cond.clauses.len(), 3);
    }

    #[test]
    fn parse_bool_literals() {
        let t = parse_rule_condition("flag == true").unwrap();
        assert_eq!(
            t.clauses[0],
            Comparison::Eq(RulePath(vec!["flag".into()]), RuleLiteral::Bool(true))
        );
        let f = parse_rule_condition("flag == false").unwrap();
        assert_eq!(
            f.clauses[0],
            Comparison::Eq(RulePath(vec!["flag".into()]), RuleLiteral::Bool(false))
        );
    }

    #[test]
    fn parse_number_literal() {
        let cond = parse_rule_condition("count == 7").unwrap();
        assert_eq!(
            cond.clauses[0],
            Comparison::Eq(RulePath(vec!["count".into()]), RuleLiteral::Number(7.0))
        );
    }

    #[test]
    fn parse_negative_number() {
        let cond = parse_rule_condition("delta == -1").unwrap();
        assert_eq!(
            cond.clauses[0],
            Comparison::Eq(RulePath(vec!["delta".into()]), RuleLiteral::Number(-1.0))
        );
    }

    #[test]
    fn parse_single_quoted_string() {
        let cond = parse_rule_condition("x == 'hello'").unwrap();
        assert_eq!(
            cond.clauses[0],
            Comparison::Eq(RulePath(vec!["x".into()]), RuleLiteral::Str("hello".into()))
        );
    }

    // ─── Parser: rejection of unsupported syntax ─────────────────────────────

    #[test]
    fn parse_rejects_empty_string() {
        assert!(parse_rule_condition("").is_err());
    }

    #[test]
    fn parse_rejects_single_equals() {
        assert!(parse_rule_condition("x = null").is_err());
    }

    #[test]
    fn parse_rejects_lowercase_and() {
        // lowercase 'and' is an identifier, not a keyword — treated as bare identifier RHS
        let err = parse_rule_condition("x == null and y == null");
        assert!(err.is_err(), "lowercase 'and' must be rejected");
    }

    #[test]
    fn parse_rejects_or_keyword() {
        // OR is not supported
        assert!(
            parse_rule_condition("x == null OR y == null").is_err(),
            "OR must be rejected at parse time"
        );
    }

    #[test]
    fn parse_rejects_bare_identifier_rhs() {
        assert!(parse_rule_condition("x == someVar").is_err());
    }

    #[test]
    fn parse_rejects_missing_rhs() {
        assert!(parse_rule_condition("x ==").is_err());
    }

    #[test]
    fn parse_rejects_missing_operator() {
        assert!(parse_rule_condition("x null").is_err());
    }

    #[test]
    fn parse_rejects_unterminated_string() {
        assert!(parse_rule_condition("x == \"hello").is_err());
    }

    // ─── Evaluator: single clause ─────────────────────────────────────────────

    #[test]
    fn eval_eq_null_missing_key() {
        let c = GateContext::new();
        assert!(eval_rule_condition("missing == null", &c).unwrap());
    }

    #[test]
    fn eval_eq_string_match() {
        let c = ctx(&[("moment", json!({"status": "passed"}))]);
        assert!(eval_rule_condition(r#"moment.status == "passed""#, &c).unwrap());
    }

    #[test]
    fn eval_eq_string_no_match() {
        let c = ctx(&[("moment", json!({"status": "failed"}))]);
        assert!(!eval_rule_condition(r#"moment.status == "passed""#, &c).unwrap());
    }

    #[test]
    fn eval_ne_works() {
        let c = ctx(&[("moment", json!({"status": "passed"}))]);
        assert!(eval_rule_condition(r#"moment.status != "failed""#, &c).unwrap());
    }

    #[test]
    fn eval_deep_path_three_levels() {
        let c = ctx(&[("a", json!({"b": {"c": "found"}}))]);
        assert!(eval_rule_condition(r#"a.b.c == "found""#, &c).unwrap());
    }

    #[test]
    fn eval_deep_path_missing_intermediate() {
        let c = ctx(&[("a", json!({"other": "x"}))]);
        // a.b is missing → null → == null is true
        assert!(eval_rule_condition("a.b == null", &c).unwrap());
    }

    #[test]
    fn eval_number_match() {
        let c = ctx(&[("priorAttestations", json!({"count": 0}))]);
        assert!(eval_rule_condition("priorAttestations.count == 0", &c).unwrap());
    }

    // ─── Evaluator: AND conjunction ────────────────────────────────────────────

    #[test]
    fn eval_and_both_true() {
        let c = ctx(&[
            ("moment", json!({"status": "passed"})),
            ("priorAttestations", json!({"count": 0})),
        ]);
        assert!(eval_rule_condition(
            r#"moment.status == "passed" AND priorAttestations.count == 0"#,
            &c
        )
        .unwrap());
    }

    #[test]
    fn eval_and_first_false_short_circuits() {
        let c = ctx(&[
            ("moment", json!({"status": "failed"})),
            ("priorAttestations", json!({"count": 0})),
        ]);
        assert!(!eval_rule_condition(
            r#"moment.status == "passed" AND priorAttestations.count == 0"#,
            &c
        )
        .unwrap());
    }

    #[test]
    fn eval_and_second_false() {
        let c = ctx(&[
            ("moment", json!({"status": "passed"})),
            ("priorAttestations", json!({"count": 5})),
        ]);
        assert!(!eval_rule_condition(
            r#"moment.status == "passed" AND priorAttestations.count == 0"#,
            &c
        )
        .unwrap());
    }

    #[test]
    fn eval_and_three_clauses_all_true() {
        let c = ctx(&[
            ("a", json!(1.0)),
            ("b", json!(true)),
            ("c", json!({"d": "ok"})),
        ]);
        assert!(eval_rule_condition(r#"a == 1 AND b == true AND c.d == "ok""#, &c).unwrap());
    }

    // ─── Rule 1 shape: first-pass-green ──────────────────────────────────────
    // Mirrors the discernment rule 1 condition from spec §7.3.

    #[test]
    fn rule1_first_pass_green_matches() {
        let c = ctx(&[
            ("moment", json!({"status": "passed"})),
            ("priorAttestations", json!({"count": 0})),
        ]);
        let cond = r#"moment.status == "passed" AND priorAttestations.count == 0"#;
        assert!(eval_rule_condition(cond, &c).unwrap());
    }

    #[test]
    fn rule1_first_pass_green_does_not_match_when_prior_exists() {
        let c = ctx(&[
            ("moment", json!({"status": "passed"})),
            ("priorAttestations", json!({"count": 2})),
        ]);
        let cond = r#"moment.status == "passed" AND priorAttestations.count == 0"#;
        assert!(!eval_rule_condition(cond, &c).unwrap());
    }

    // ─── Rule 2 shape: regression ─────────────────────────────────────────────

    #[test]
    fn rule2_regression_matches() {
        let c = ctx(&[
            ("moment", json!({"status": "failed"})),
            ("priorAttestations", json!({"mostRecentStatus": "passed"})),
        ]);
        let cond =
            r#"moment.status == "failed" AND priorAttestations.mostRecentStatus == "passed""#;
        assert!(eval_rule_condition(cond, &c).unwrap());
    }

    // ─── Rule 4 shape: recovery ───────────────────────────────────────────────

    #[test]
    fn rule4_recovery_matches() {
        let c = ctx(&[
            ("moment", json!({"status": "passed"})),
            ("priorAttestations", json!({"mostRecentStatus": "failed"})),
        ]);
        let cond =
            r#"moment.status == "passed" AND priorAttestations.mostRecentStatus == "failed""#;
        assert!(eval_rule_condition(cond, &c).unwrap());
    }
}

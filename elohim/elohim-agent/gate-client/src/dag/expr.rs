//! Minimal conditional-edge expression language for DAG edges.
//!
//! Per spec §2.3, edges carry a `when` string like:
//!
//! ```text
//! "ruleDecision == null"
//! "decision.kind == 'allow'"
//! "moment.status != \"passed\""
//! ```
//!
//! This module parses such strings into a small AST and evaluates them against
//! a [`GateContext`]. The language is **deliberately non-Turing-complete**:
//!
//! - Only `==` and `!=` comparison operators.
//! - Left-hand side is a dotted path into the context (e.g., `decision.kind`).
//! - Right-hand side is a literal: `null`, `true`, `false`, a quoted string
//!   (single or double), or an unquoted number.
//! - No `and`/`or`/`not`, no arithmetic, no function calls.
//!
//! Any unknown token or unsupported structure is a hard parse error. An elohim
//! encountering a malformed DAG manifest must refuse to execute it.

use crate::dag::context::GateContext;
use crate::error::GateError;

// ─── AST ─────────────────────────────────────────────────────────────────────

/// A dotted path that resolves against the GateContext.
///
/// `"ruleDecision"` → `["ruleDecision"]`
/// `"decision.kind"` → `["decision", "kind"]`
#[derive(Debug, Clone, PartialEq)]
pub struct Path(pub Vec<String>);

/// A literal value on the right-hand side of a comparison.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Null,
    Bool(bool),
    Number(f64),
    Str(String),
}

impl Literal {
    /// Convert to a `serde_json::Value` for comparison.
    pub fn to_json_value(&self) -> serde_json::Value {
        match self {
            Literal::Null => serde_json::Value::Null,
            Literal::Bool(b) => serde_json::Value::Bool(*b),
            Literal::Number(n) => {
                serde_json::Value::Number(
                    serde_json::Number::from_f64(*n)
                        .unwrap_or_else(|| serde_json::Number::from(0u64)),
                )
            }
            Literal::Str(s) => serde_json::Value::String(s.clone()),
        }
    }
}

/// The expression AST — exactly two variants.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// `path == literal`
    Eq(Path, Literal),
    /// `path != literal`
    Ne(Path, Literal),
}

// ─── Tokenizer ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Ident(String),  // path segment or keyword (null, true, false)
    Dot,
    EqEq,
    BangEq,
    StringLit(String),
    NumberLit(f64),
}

struct Tokenizer<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> Tokenizer<'a> {
    fn new(s: &'a str) -> Self {
        Self { input: s.as_bytes(), pos: 0 }
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        let b = self.input.get(self.pos).copied();
        if b.is_some() { self.pos += 1; }
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
        // Optional leading minus already consumed by caller (if any).
        while let Some(b) = self.peek() {
            if b.is_ascii_digit() || b == b'.' || b == b'e' || b == b'E'
                || b == b'+' || b == b'-'
            {
                self.advance();
            } else {
                break;
            }
        }
        let s = String::from_utf8_lossy(&self.input[start..self.pos]).into_owned();
        s.parse::<f64>().map_err(|_| {
            GateError::DagExecution(format!("invalid number literal: {s}"))
        })
    }

    fn read_quoted_string(&mut self, quote: u8) -> Result<String, GateError> {
        // Opening quote already consumed.
        let mut result = String::new();
        loop {
            match self.advance() {
                None => {
                    return Err(GateError::DagExecution(
                        "unterminated string literal in edge expression".to_string(),
                    ))
                }
                Some(b) if b == b'\\' => {
                    // Escape sequence — only \" and \' and \\ are supported.
                    match self.advance() {
                        Some(b'"') => result.push('"'),
                        Some(b'\'') => result.push('\''),
                        Some(b'\\') => result.push('\\'),
                        Some(b'n') => result.push('\n'),
                        Some(b't') => result.push('\t'),
                        Some(b) => {
                            return Err(GateError::DagExecution(format!(
                                "unsupported escape \\{} in edge expression",
                                b as char
                            )))
                        }
                        None => {
                            return Err(GateError::DagExecution(
                                "unexpected end after backslash in edge expression".to_string(),
                            ))
                        }
                    }
                }
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
                                "expected '==' but got single '=' in edge expression".to_string(),
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
                                "expected '!=' but got lone '!' in edge expression".to_string(),
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
                    // Could be a negative number.
                    self.advance();
                    if self.peek().map(|b| b.is_ascii_digit()).unwrap_or(false) {
                        // Rewind conceptually: embed the sign into the read.
                        // We already consumed '-', so read rest and negate.
                        let n = self.read_number()?;
                        tokens.push(Token::NumberLit(-n));
                    } else {
                        return Err(GateError::DagExecution(
                            "unexpected '-' in edge expression (not followed by digit)".to_string(),
                        ));
                    }
                }
                Some(b) if b.is_ascii_alphabetic() || b == b'_' => {
                    let ident = self.read_ident();
                    tokens.push(Token::Ident(ident));
                }
                Some(b) => {
                    return Err(GateError::DagExecution(format!(
                        "unexpected character '{}' in edge expression",
                        b as char
                    )))
                }
            }
        }
        Ok(tokens)
    }
}

// ─── Parser ──────────────────────────────────────────────────────────────────

/// Parse an edge `when` string into an [`Expr`].
///
/// Returns `Err(GateError::DagExecution(_))` for any malformed or unsupported
/// expression. Unknown operators are hard errors — not silent allows.
pub fn parse_expr(s: &str) -> Result<Expr, GateError> {
    let tokens = Tokenizer::new(s).tokenize()?;
    let mut parser = Parser { tokens: &tokens, pos: 0 };
    parser.parse()
}

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
        if t.is_some() { self.pos += 1; }
        t
    }

    fn parse(&mut self) -> Result<Expr, GateError> {
        let lhs = self.parse_path()?;
        let op = self.advance().ok_or_else(|| {
            GateError::DagExecution("expected '==' or '!=' in edge expression".to_string())
        })?;
        let op = op.clone();
        let rhs = self.parse_literal()?;

        // Ensure nothing is left over.
        if self.peek().is_some() {
            return Err(GateError::DagExecution(
                "trailing tokens in edge expression — only 'path op literal' is supported"
                    .to_string(),
            ));
        }

        match op {
            Token::EqEq => Ok(Expr::Eq(lhs, rhs)),
            Token::BangEq => Ok(Expr::Ne(lhs, rhs)),
            other => Err(GateError::DagExecution(format!(
                "unsupported operator {other:?} in edge expression; only '==' and '!=' are allowed"
            ))),
        }
    }

    fn parse_path(&mut self) -> Result<Path, GateError> {
        let mut segments = Vec::new();

        // Expect first ident.
        match self.advance() {
            Some(Token::Ident(s)) => segments.push(s.clone()),
            other => {
                return Err(GateError::DagExecution(format!(
                    "expected identifier at start of path in edge expression, got {other:?}"
                )))
            }
        }

        // Consume optional `.segment` chains.
        loop {
            if self.peek() == Some(&Token::Dot) {
                self.advance(); // consume '.'
                match self.advance() {
                    Some(Token::Ident(s)) => segments.push(s.clone()),
                    other => {
                        return Err(GateError::DagExecution(format!(
                            "expected identifier after '.' in path, got {other:?}"
                        )))
                    }
                }
            } else {
                break;
            }
        }

        Ok(Path(segments))
    }

    fn parse_literal(&mut self) -> Result<Literal, GateError> {
        match self.advance() {
            Some(Token::Ident(s)) if s == "null" => Ok(Literal::Null),
            Some(Token::Ident(s)) if s == "true" => Ok(Literal::Bool(true)),
            Some(Token::Ident(s)) if s == "false" => Ok(Literal::Bool(false)),
            Some(Token::StringLit(s)) => Ok(Literal::Str(s.clone())),
            Some(Token::NumberLit(n)) => Ok(Literal::Number(*n)),
            Some(Token::Ident(s)) => Err(GateError::DagExecution(format!(
                "expected literal (null/true/false/string/number) in edge expression, got identifier '{s}'"
            ))),
            other => Err(GateError::DagExecution(format!(
                "expected literal in edge expression, got {other:?}"
            ))),
        }
    }
}

// ─── Evaluator ───────────────────────────────────────────────────────────────

/// Resolve a [`Path`] against a [`GateContext`] and return the `serde_json::Value`.
///
/// Only top-level and one-level-deep accesses are supported:
/// - `["key"]` → `ctx.get("key")`
/// - `["key", "field"]` → `ctx.get("key")[field]` (if the value is an object)
///
/// Paths deeper than two levels return `Value::Null` (treated as missing).
fn resolve_path<'c>(path: &Path, ctx: &'c GateContext) -> &'c serde_json::Value {
    static NULL: serde_json::Value = serde_json::Value::Null;

    let segments = &path.0;
    match segments.as_slice() {
        [] => &NULL,
        [key] => ctx.get(key).unwrap_or(&NULL),
        [key, field] => match ctx.get(key) {
            Some(serde_json::Value::Object(map)) => map.get(field).unwrap_or(&NULL),
            _ => &NULL,
        },
        _ => &NULL, // deeper paths not supported; treat as null
    }
}

/// Evaluate a parsed [`Expr`] against a [`GateContext`].
///
/// Returns `true` if the expression holds, `false` otherwise.
pub fn evaluate(expr: &Expr, ctx: &GateContext) -> bool {
    match expr {
        Expr::Eq(path, lit) => {
            let context_val = resolve_path(path, ctx);
            let literal_val = lit.to_json_value();
            *context_val == literal_val
        }
        Expr::Ne(path, lit) => {
            let context_val = resolve_path(path, ctx);
            let literal_val = lit.to_json_value();
            *context_val != literal_val
        }
    }
}

/// Parse and evaluate in one step. Convenience wrapper.
pub fn eval_edge(when: &str, ctx: &GateContext) -> Result<bool, GateError> {
    let expr = parse_expr(when)?;
    Ok(evaluate(&expr, ctx))
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ctx_with(key: &str, val: serde_json::Value) -> GateContext {
        let mut c = GateContext::new();
        c.insert(key, val);
        c
    }

    // ─── Parser: equality operator ────────────────────────────────────────────

    #[test]
    fn parse_eq_null() {
        let expr = parse_expr("ruleDecision == null").unwrap();
        assert_eq!(expr, Expr::Eq(Path(vec!["ruleDecision".into()]), Literal::Null));
    }

    #[test]
    fn parse_ne_null() {
        let expr = parse_expr("ruleDecision != null").unwrap();
        assert_eq!(expr, Expr::Ne(Path(vec!["ruleDecision".into()]), Literal::Null));
    }

    #[test]
    fn parse_eq_single_quoted_string() {
        let expr = parse_expr("decision.kind == 'allow'").unwrap();
        assert_eq!(
            expr,
            Expr::Eq(
                Path(vec!["decision".into(), "kind".into()]),
                Literal::Str("allow".into())
            )
        );
    }

    #[test]
    fn parse_eq_double_quoted_string() {
        let expr = parse_expr(r#"moment.status != "passed""#).unwrap();
        assert_eq!(
            expr,
            Expr::Ne(
                Path(vec!["moment".into(), "status".into()]),
                Literal::Str("passed".into())
            )
        );
    }

    #[test]
    fn parse_eq_bool_true() {
        let expr = parse_expr("isReady == true").unwrap();
        assert_eq!(expr, Expr::Eq(Path(vec!["isReady".into()]), Literal::Bool(true)));
    }

    #[test]
    fn parse_eq_bool_false() {
        let expr = parse_expr("isReady == false").unwrap();
        assert_eq!(expr, Expr::Eq(Path(vec!["isReady".into()]), Literal::Bool(false)));
    }

    #[test]
    fn parse_eq_integer_number() {
        let expr = parse_expr("count == 42").unwrap();
        assert_eq!(
            expr,
            Expr::Eq(Path(vec!["count".into()]), Literal::Number(42.0))
        );
    }

    #[test]
    fn parse_eq_float_number() {
        let expr = parse_expr("score == 0.75").unwrap();
        assert_eq!(
            expr,
            Expr::Eq(Path(vec!["score".into()]), Literal::Number(0.75))
        );
    }

    #[test]
    fn parse_eq_negative_number() {
        let expr = parse_expr("delta == -1").unwrap();
        assert_eq!(
            expr,
            Expr::Eq(Path(vec!["delta".into()]), Literal::Number(-1.0))
        );
    }

    #[test]
    fn parse_dotted_two_level_path() {
        let expr = parse_expr("a.b == null").unwrap();
        assert_eq!(
            expr,
            Expr::Eq(Path(vec!["a".into(), "b".into()]), Literal::Null)
        );
    }

    #[test]
    fn parse_escaped_double_quote_inside_string() {
        let expr = parse_expr(r#"x == "he\"llo""#).unwrap();
        assert_eq!(expr, Expr::Eq(Path(vec!["x".into()]), Literal::Str(r#"he"llo"#.into())));
    }

    // ─── Parser: rejection of unsupported syntax ─────────────────────────────

    #[test]
    fn parse_rejects_single_equals() {
        assert!(parse_expr("x = null").is_err(), "single = is not supported");
    }

    #[test]
    fn parse_rejects_unknown_operator() {
        assert!(parse_expr("x < 1").is_err(), "< is not supported");
    }

    #[test]
    fn parse_rejects_missing_rhs() {
        assert!(parse_expr("x ==").is_err(), "missing RHS");
    }

    #[test]
    fn parse_rejects_missing_operator() {
        assert!(parse_expr("x null").is_err(), "missing operator");
    }

    #[test]
    fn parse_rejects_trailing_tokens() {
        assert!(parse_expr("x == null extra").is_err(), "trailing token");
    }

    #[test]
    fn parse_rejects_bare_identifier_as_rhs() {
        assert!(parse_expr("x == someVar").is_err(), "bare identifier (non-keyword) as RHS");
    }

    #[test]
    fn parse_rejects_empty_string() {
        assert!(parse_expr("").is_err(), "empty expression");
    }

    #[test]
    fn parse_rejects_unterminated_string() {
        assert!(parse_expr("x == \"hello").is_err(), "unterminated string");
    }

    #[test]
    fn parse_rejects_lone_bang() {
        assert!(parse_expr("x ! null").is_err(), "lone bang");
    }

    // ─── Evaluator: == ────────────────────────────────────────────────────────

    #[test]
    fn eval_eq_null_when_key_missing() {
        let ctx = GateContext::new();
        let expr = parse_expr("missing == null").unwrap();
        assert!(evaluate(&expr, &ctx), "missing key should equal null");
    }

    #[test]
    fn eval_eq_null_when_key_is_null() {
        let ctx = ctx_with("x", json!(null));
        let expr = parse_expr("x == null").unwrap();
        assert!(evaluate(&expr, &ctx));
    }

    #[test]
    fn eval_eq_null_false_when_key_has_value() {
        let ctx = ctx_with("x", json!("something"));
        let expr = parse_expr("x == null").unwrap();
        assert!(!evaluate(&expr, &ctx));
    }

    #[test]
    fn eval_eq_string_match() {
        let ctx = ctx_with("decision", json!({"kind": "allow"}));
        // dotted path: decision.kind
        let expr = parse_expr("decision.kind == 'allow'").unwrap();
        assert!(evaluate(&expr, &ctx));
    }

    #[test]
    fn eval_eq_string_no_match() {
        let ctx = ctx_with("decision", json!({"kind": "decline"}));
        let expr = parse_expr("decision.kind == 'allow'").unwrap();
        assert!(!evaluate(&expr, &ctx));
    }

    #[test]
    fn eval_ne_null_when_key_has_value() {
        let ctx = ctx_with("result", json!(42));
        let expr = parse_expr("result != null").unwrap();
        assert!(evaluate(&expr, &ctx));
    }

    #[test]
    fn eval_ne_null_false_when_key_missing() {
        let ctx = GateContext::new();
        let expr = parse_expr("missing != null").unwrap();
        assert!(!evaluate(&expr, &ctx), "missing key is null; != null is false");
    }

    #[test]
    fn eval_bool_true() {
        let ctx = ctx_with("ready", json!(true));
        let expr = parse_expr("ready == true").unwrap();
        assert!(evaluate(&expr, &ctx));
    }

    #[test]
    fn eval_number_match() {
        let ctx = ctx_with("count", json!(7.0));
        let expr = parse_expr("count == 7").unwrap();
        assert!(evaluate(&expr, &ctx));
    }

    #[test]
    fn eval_edge_convenience_wrapper() {
        let ctx = ctx_with("k", json!("v"));
        assert!(eval_edge("k == 'v'", &ctx).unwrap());
        assert!(!eval_edge("k == 'other'", &ctx).unwrap());
    }

    #[test]
    fn eval_edge_returns_err_for_malformed_expr() {
        let ctx = GateContext::new();
        assert!(eval_edge("x = null", &ctx).is_err());
    }
}

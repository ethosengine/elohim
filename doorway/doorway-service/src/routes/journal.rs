//! Journal routing endpoints — intent analysis and suggestion generation.
//!
//! When the inference sidecar is available, requests are forwarded to it.
//! Otherwise, keyword-matching stubs run locally.

use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;

use crate::server::AppState;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeRequest {
    pub text: String,
    pub content_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntentAnalysis {
    pub summary: String,
    pub detected_types: Vec<String>,
    pub suggested_path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestRequest {
    pub text: String,
    pub content_id: String,
    pub intent: IntentAnalysis,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutingSuggestion {
    pub id: String,
    pub kind: String,
    pub destination_type: String,
    pub title: String,
    pub summary: String,
    pub suggested_path: String,
    pub reach: String,
    pub status: String,
}

// ---------------------------------------------------------------------------
// Keyword stub functions (pure, no async)
// ---------------------------------------------------------------------------

const EXCHANGE_KEYWORDS: &[&str] = &[
    "need", "want", "broken", "repair", "help", "fix", "hire", "find",
];
const GOVERNANCE_KEYWORDS: &[&str] = &[
    "should",
    "vote",
    "policy",
    "propose",
    "fund",
    "community",
    "rule",
];
const CONTENT_KEYWORDS: &[&str] = &[
    "learned",
    "discovered",
    "guide",
    "how-to",
    "tutorial",
    "explain",
];

pub fn analyze_intent_stub(text: &str) -> IntentAnalysis {
    let lower = text.to_lowercase();
    let mut detected_types = Vec::new();

    if EXCHANGE_KEYWORDS.iter().any(|kw| lower.contains(kw)) {
        detected_types.push("exchange-request".to_string());
    }
    if GOVERNANCE_KEYWORDS.iter().any(|kw| lower.contains(kw)) {
        detected_types.push("governance-proposal".to_string());
    }
    if CONTENT_KEYWORDS.iter().any(|kw| lower.contains(kw)) {
        detected_types.push("content".to_string());
    }

    let mut summary_parts = Vec::new();
    if detected_types.contains(&"exchange-request".to_string()) {
        summary_parts.push("a need or request");
    }
    if detected_types.contains(&"governance-proposal".to_string()) {
        summary_parts.push("a governance concern");
    }
    if detected_types.contains(&"content".to_string()) {
        summary_parts.push("shareable knowledge");
    }

    let summary = if summary_parts.is_empty() {
        "This entry can be filed in your journal.".to_string()
    } else {
        format!("This sounds like {}.", summary_parts.join(" and "))
    };

    let suggested_path = guess_filing_path(&lower);

    IntentAnalysis {
        summary,
        detected_types,
        suggested_path,
    }
}

fn guess_filing_path(lower_text: &str) -> String {
    let home = ["faucet", "roof", "plumbing", "kitchen", "garage", "repair"];
    let learning = ["learned", "studied", "course", "tutorial", "formula"];
    let work = ["meeting", "project", "deadline", "client", "office"];

    if home.iter().any(|kw| lower_text.contains(kw)) {
        return "home-maintenance".to_string();
    }
    if learning.iter().any(|kw| lower_text.contains(kw)) {
        return "learning".to_string();
    }
    if work.iter().any(|kw| lower_text.contains(kw)) {
        return "work".to_string();
    }
    "general".to_string()
}

pub fn generate_suggestions_stub(intent: &IntentAnalysis) -> Vec<RoutingSuggestion> {
    let mut suggestions = Vec::new();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    // Always: filing card
    suggestions.push(RoutingSuggestion {
        id: format!("filing-{now}"),
        kind: "filing".to_string(),
        destination_type: "journal-filing".to_string(),
        title: "File in journal".to_string(),
        summary: format!("File this entry under {}", intent.suggested_path),
        suggested_path: intent.suggested_path.clone(),
        reach: "private".to_string(),
        status: "suggested".to_string(),
    });

    // Derivative cards
    let templates: &[(&str, &str, &str, &str, &str)] = &[
        (
            "exchange-request",
            "Post to exchange",
            "Post a request or offer on the community exchange.",
            "/shefa/exchange/",
            "community",
        ),
        (
            "governance-proposal",
            "Draft governance proposal",
            "Draft a governance proposal for community review.",
            "/qahal/proposals/",
            "community",
        ),
        (
            "content",
            "Share as learning content",
            "Share this as learning content for others.",
            "/lamad/contributions/",
            "network",
        ),
    ];

    for (dtype, title, summary, path, reach) in templates {
        if intent.detected_types.contains(&dtype.to_string()) {
            suggestions.push(RoutingSuggestion {
                id: format!("derivative-{dtype}-{now}"),
                kind: "derivative".to_string(),
                destination_type: dtype.to_string(),
                title: title.to_string(),
                summary: summary.to_string(),
                suggested_path: path.to_string(),
                reach: reach.to_string(),
                status: "suggested".to_string(),
            });
        }
    }

    suggestions
}

// ---------------------------------------------------------------------------
// HTTP handler functions
// ---------------------------------------------------------------------------

pub async fn handle_journal_analyze(
    req: Request<Incoming>,
    state: Arc<AppState>,
) -> Response<Full<Bytes>> {
    // Parse body
    let body = match parse_json_body::<AnalyzeRequest>(req).await {
        Ok(b) => b,
        Err(resp) => return resp,
    };

    // If sidecar available, forward (future — for now always stub)
    if state.journal_inference_available {
        debug!("Journal analyze: forwarding to sidecar");
        // TODO: forward to sidecar when deployed
        // For now, fall through to stub
    }

    let analysis = analyze_intent_stub(&body.text);
    json_response(StatusCode::OK, &analysis)
}

pub async fn handle_journal_suggest(
    req: Request<Incoming>,
    state: Arc<AppState>,
) -> Response<Full<Bytes>> {
    let body = match parse_json_body::<SuggestRequest>(req).await {
        Ok(b) => b,
        Err(resp) => return resp,
    };

    if state.journal_inference_available {
        debug!("Journal suggest: forwarding to sidecar");
        // TODO: forward to sidecar when deployed
    }

    let suggestions = generate_suggestions_stub(&body.intent);
    json_response(StatusCode::OK, &suggestions)
}

// Helper to parse JSON body from hyper request
async fn parse_json_body<T: serde::de::DeserializeOwned>(
    req: Request<Incoming>,
) -> Result<T, Response<Full<Bytes>>> {
    use http_body_util::BodyExt;
    let body_bytes = req
        .collect()
        .await
        .map_err(|e| {
            json_response(
                StatusCode::BAD_REQUEST,
                &serde_json::json!({"error": format!("Failed to read body: {e}")}),
            )
        })?
        .to_bytes();
    serde_json::from_slice(&body_bytes).map_err(|e| {
        json_response(
            StatusCode::BAD_REQUEST,
            &serde_json::json!({"error": format!("Invalid JSON: {e}")}),
        )
    })
}

fn json_response<T: Serialize>(status: StatusCode, body: &T) -> Response<Full<Bytes>> {
    let json = serde_json::to_string(body).unwrap_or_else(|_| "{}".to_string());
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(json)))
        .unwrap()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_detects_exchange_request() {
        let result = analyze_intent_stub("I need someone to fix my leaking roof");
        assert!(result
            .detected_types
            .contains(&"exchange-request".to_string()));
    }

    #[test]
    fn test_analyze_detects_governance_proposal() {
        let result = analyze_intent_stub("We should vote on a community maintenance fund");
        assert!(result
            .detected_types
            .contains(&"governance-proposal".to_string()));
    }

    #[test]
    fn test_analyze_detects_content() {
        let result = analyze_intent_stub("I learned how to fix a faucet today");
        assert!(result.detected_types.contains(&"content".to_string()));
    }

    #[test]
    fn test_analyze_detects_multiple_types() {
        let result = analyze_intent_stub(
            "I need help and we should vote on funding for repairs I learned about",
        );
        assert!(result.detected_types.len() >= 2);
    }

    #[test]
    fn test_analyze_returns_empty_for_generic_text() {
        let result = analyze_intent_stub("Hello world, nice day today");
        assert!(result.detected_types.is_empty());
    }

    #[test]
    fn test_analyze_guesses_home_maintenance_path() {
        let result = analyze_intent_stub("My roof is leaking");
        assert_eq!(result.suggested_path, "home-maintenance");
    }

    #[test]
    fn test_analyze_defaults_to_general_path() {
        let result = analyze_intent_stub("Just some thoughts");
        assert_eq!(result.suggested_path, "general");
    }

    #[test]
    fn test_suggestions_always_include_filing_card() {
        let intent = IntentAnalysis {
            summary: "test".to_string(),
            detected_types: vec![],
            suggested_path: "general".to_string(),
        };
        let suggestions = generate_suggestions_stub(&intent);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].kind, "filing");
        assert_eq!(suggestions[0].destination_type, "journal-filing");
        assert_eq!(suggestions[0].reach, "private");
    }

    #[test]
    fn test_suggestions_include_derivative_for_each_type() {
        let intent = IntentAnalysis {
            summary: "test".to_string(),
            detected_types: vec![
                "exchange-request".to_string(),
                "governance-proposal".to_string(),
            ],
            suggested_path: "general".to_string(),
        };
        let suggestions = generate_suggestions_stub(&intent);
        assert_eq!(suggestions.len(), 3); // filing + 2 derivatives
        assert_eq!(suggestions[0].kind, "filing");
        assert_eq!(suggestions[1].kind, "derivative");
        assert_eq!(suggestions[2].kind, "derivative");
    }

    #[test]
    fn test_suggestions_filing_card_is_first() {
        let intent = IntentAnalysis {
            summary: "test".to_string(),
            detected_types: vec!["content".to_string()],
            suggested_path: "learning".to_string(),
        };
        let suggestions = generate_suggestions_stub(&intent);
        assert_eq!(suggestions[0].kind, "filing");
        assert_eq!(suggestions[0].suggested_path, "learning");
    }

    #[test]
    fn test_suggestion_ids_are_unique() {
        let intent = IntentAnalysis {
            summary: "test".to_string(),
            detected_types: vec!["exchange-request".to_string(), "content".to_string()],
            suggested_path: "general".to_string(),
        };
        let suggestions = generate_suggestions_stub(&intent);
        let ids: Vec<&str> = suggestions.iter().map(|s| s.id.as_str()).collect();
        let unique: std::collections::HashSet<&str> = ids.iter().copied().collect();
        assert_eq!(ids.len(), unique.len());
    }

    #[test]
    fn test_summary_describes_detected_types() {
        let result = analyze_intent_stub("I need a roofer");
        assert!(result.summary.contains("need") || result.summary.contains("request"));
    }

    #[test]
    fn test_generic_summary_for_no_types() {
        let result = analyze_intent_stub("Hello world");
        assert!(result.summary.contains("filed"));
    }
}

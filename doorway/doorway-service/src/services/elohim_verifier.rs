//! Elohim Knowledge Verification Service
//!
//! Provides AI-assisted identity verification by interrogating users about
//! their own imagodei profile data. This is used as part of disaster recovery
//! to verify that someone claiming to be a user actually knows their "stuff".
//!
//! Questions are dynamically generated based on:
//! - Content mastery (paths completed, quiz scores)
//! - Relationships (trusted contacts, intimacy levels)
//! - Preferences (learning style, affinities)
//! - Activity history (recent completions, milestones)
//!
//! This is NOT static security questions - questions are derived from actual
//! usage patterns, making them harder to social-engineer.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

/// Maximum confidence score from Elohim verification (0-60%)
pub const MAX_ELOHIM_CONFIDENCE: f64 = 60.0;

/// Minimum accuracy required to pass Elohim verification (70%)
pub const MIN_ACCURACY_THRESHOLD: f64 = 0.70;

/// Number of questions to ask
pub const QUESTION_COUNT: usize = 5;

// =============================================================================
// Profile Data Types (from DHT)
// =============================================================================

/// User's imagodei profile data fetched from DHT
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserProfileData {
    /// Human ID
    pub human_id: String,
    /// Display name
    pub display_name: String,
    /// User's affinities/interests
    pub affinities: Vec<String>,
    /// Completed content paths
    pub completed_paths: Vec<PathCompletion>,
    /// Quiz scores
    pub quiz_scores: Vec<QuizScore>,
    /// Trusted relationships (names only, for privacy)
    pub relationship_names: Vec<String>,
    /// Learning preferences
    pub learning_preferences: Option<LearningPreferences>,
    /// Milestones achieved
    pub milestones: Vec<String>,
    /// Account creation date
    pub created_at: String,
}

/// A completed learning path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathCompletion {
    pub path_id: String,
    pub path_title: String,
    pub completed_at: String,
}

/// A quiz score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuizScore {
    pub quiz_id: String,
    pub quiz_title: String,
    pub score: f64,
    pub max_score: f64,
    pub completed_at: String,
}

/// Learning preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningPreferences {
    pub preferred_style: Option<String>,
    pub daily_goal_minutes: Option<u32>,
    pub notification_enabled: bool,
}

// =============================================================================
// Verification Question Types
// =============================================================================

/// Category of verification question
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum QuestionCategory {
    /// Questions about completed content
    ContentMastery,
    /// Questions about relationships
    Relationships,
    /// Questions about preferences/settings
    Preferences,
    /// Questions about quiz performance
    QuizScores,
    /// Questions about account history
    AccountHistory,
}

/// A verification question with expected answer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationQuestion {
    /// Unique question ID
    pub id: String,
    /// The question text
    pub question: String,
    /// Category of question
    pub category: QuestionCategory,
    /// Expected answer(s) - could be multiple valid answers
    #[serde(skip_serializing)]
    pub expected_answers: Vec<String>,
    /// Whether this is a multiple choice question
    pub is_multiple_choice: bool,
    /// Multiple choice options (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    /// Weight for scoring (higher = more important)
    pub weight: f64,
}

/// User's answer to a verification question
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionAnswer {
    /// Question ID
    pub question_id: String,
    /// User's answer
    pub answer: String,
}

/// Result of scoring an answer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnswerScore {
    /// Question ID
    pub question_id: String,
    /// Whether the answer was correct
    pub correct: bool,
    /// Partial credit (0.0 - 1.0)
    pub score: f64,
    /// Feedback message
    pub feedback: String,
}

/// Overall verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Overall accuracy (0.0 - 1.0)
    pub accuracy: f64,
    /// Confidence score contribution (0.0 - MAX_ELOHIM_CONFIDENCE)
    pub confidence_score: f64,
    /// Whether verification passed
    pub passed: bool,
    /// Individual answer scores
    pub answer_scores: Vec<AnswerScore>,
    /// Summary message
    pub summary: String,
}

// =============================================================================
// Elohim Verifier Service
// =============================================================================

/// Service for generating and scoring verification questions
pub struct ElohimVerifier;

impl ElohimVerifier {
    /// Generate verification questions based on user profile data.
    ///
    /// Returns questions that only the real user should be able to answer.
    pub fn generate_questions(profile: &UserProfileData) -> Vec<VerificationQuestion> {
        let mut questions = Vec::new();
        let mut question_id = 0;

        // 1. Content mastery questions
        if !profile.completed_paths.is_empty() {
            let path = &profile.completed_paths[0];
            questions.push(VerificationQuestion {
                id: format!("q_{question_id}"),
                question: "What was the most recent learning path you completed?".to_string(),
                category: QuestionCategory::ContentMastery,
                expected_answers: vec![path.path_title.clone(), path.path_id.clone()],
                is_multiple_choice: false,
                options: None,
                weight: 1.0,
            });
            question_id += 1;

            if profile.completed_paths.len() > 1 {
                let titles: Vec<String> = profile
                    .completed_paths
                    .iter()
                    .take(3)
                    .map(|p| p.path_title.clone())
                    .collect();
                questions.push(VerificationQuestion {
                    id: format!("q_{question_id}"),
                    question: "Name one of the learning paths you have completed.".to_string(),
                    category: QuestionCategory::ContentMastery,
                    expected_answers: titles,
                    is_multiple_choice: false,
                    options: None,
                    weight: 0.8,
                });
                question_id += 1;
            }
        }

        // 2. Quiz score questions
        if !profile.quiz_scores.is_empty() {
            let quiz = &profile.quiz_scores[0];
            let percentage = (quiz.score / quiz.max_score * 100.0).round() as i32;
            questions.push(VerificationQuestion {
                id: format!("q_{question_id}"),
                question: format!(
                    "What was your approximate score on the '{}' quiz?",
                    quiz.quiz_title
                ),
                category: QuestionCategory::QuizScores,
                expected_answers: vec![
                    format!("{}%", percentage),
                    format!("{}", percentage),
                    format!("{}/{}", quiz.score as i32, quiz.max_score as i32),
                ],
                is_multiple_choice: true,
                options: Some(vec![
                    format!("{}%", (percentage - 20).max(0)),
                    format!("{}%", (percentage - 10).max(0)),
                    format!("{}%", percentage),
                    format!("{}%", (percentage + 10).min(100)),
                ]),
                weight: 1.2,
            });
            question_id += 1;
        }

        // 3. Relationship questions
        if profile.relationship_names.len() >= 2 {
            questions.push(VerificationQuestion {
                id: format!("q_{question_id}"),
                question: "Name one of your trusted contacts in the network.".to_string(),
                category: QuestionCategory::Relationships,
                expected_answers: profile.relationship_names.clone(),
                is_multiple_choice: false,
                options: None,
                weight: 1.5, // Higher weight - relationships are personal
            });
            question_id += 1;
        }

        // 4. Affinity questions
        if !profile.affinities.is_empty() {
            questions.push(VerificationQuestion {
                id: format!("q_{question_id}"),
                question: "What is one of your listed interests/affinities?".to_string(),
                category: QuestionCategory::Preferences,
                expected_answers: profile.affinities.clone(),
                is_multiple_choice: false,
                options: None,
                weight: 0.8,
            });
            question_id += 1;
        }

        // 5. Learning preferences questions
        if let Some(prefs) = &profile.learning_preferences {
            if let Some(style) = &prefs.preferred_style {
                questions.push(VerificationQuestion {
                    id: format!("q_{question_id}"),
                    question: "What is your preferred learning style?".to_string(),
                    category: QuestionCategory::Preferences,
                    expected_answers: vec![style.clone()],
                    is_multiple_choice: true,
                    options: Some(vec![
                        "Visual".to_string(),
                        "Auditory".to_string(),
                        "Reading/Writing".to_string(),
                        "Kinesthetic".to_string(),
                    ]),
                    weight: 0.6,
                });
                question_id += 1;
            }
        }

        // 6. Account history questions
        if !profile.created_at.is_empty() {
            // Extract year from created_at
            let year = profile.created_at.split('-').next().unwrap_or("2024");
            questions.push(VerificationQuestion {
                id: format!("q_{question_id}"),
                question: "What year did you create your account?".to_string(),
                category: QuestionCategory::AccountHistory,
                expected_answers: vec![year.to_string()],
                is_multiple_choice: true,
                options: Some(vec![
                    "2023".to_string(),
                    "2024".to_string(),
                    "2025".to_string(),
                    "2026".to_string(),
                ]),
                weight: 0.5,
            });
            question_id += 1;
        }

        // 7. Milestone questions
        if !profile.milestones.is_empty() {
            questions.push(VerificationQuestion {
                id: format!("q_{question_id}"),
                question: "Name one milestone you have achieved.".to_string(),
                category: QuestionCategory::AccountHistory,
                expected_answers: profile.milestones.clone(),
                is_multiple_choice: false,
                options: None,
                weight: 0.7,
            });
        }

        // Limit to QUESTION_COUNT questions, prioritizing by weight
        questions.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap());
        questions.truncate(QUESTION_COUNT);

        debug!(
            "Generated {} verification questions for {}",
            questions.len(),
            profile.human_id
        );

        questions
    }

    /// Score user's answers against expected answers.
    ///
    /// Uses fuzzy matching for text answers and exact matching for multiple choice.
    pub fn score_answers(
        questions: &[VerificationQuestion],
        answers: &[QuestionAnswer],
    ) -> VerificationResult {
        let mut answer_scores = Vec::new();
        let mut total_weighted_score = 0.0;
        let mut total_weight = 0.0;

        // Build answer lookup
        let answer_map: HashMap<&str, &str> = answers
            .iter()
            .map(|a| (a.question_id.as_str(), a.answer.as_str()))
            .collect();

        for question in questions {
            let user_answer = answer_map.get(question.id.as_str()).copied();
            let (correct, score, feedback) = Self::score_single_answer(question, user_answer);

            answer_scores.push(AnswerScore {
                question_id: question.id.clone(),
                correct,
                score,
                feedback,
            });

            total_weighted_score += score * question.weight;
            total_weight += question.weight;
        }

        let accuracy = if total_weight > 0.0 {
            total_weighted_score / total_weight
        } else {
            0.0
        };

        let confidence_score = accuracy * MAX_ELOHIM_CONFIDENCE;
        let passed = accuracy >= MIN_ACCURACY_THRESHOLD;

        let summary = if passed {
            format!(
                "Verification passed with {:.0}% accuracy. Identity confirmed.",
                accuracy * 100.0
            )
        } else {
            format!(
                "Verification failed with {:.0}% accuracy. Need {:.0}% to pass.",
                accuracy * 100.0,
                MIN_ACCURACY_THRESHOLD * 100.0
            )
        };

        info!(
            "Elohim verification complete: accuracy={:.2}, passed={}",
            accuracy, passed
        );

        VerificationResult {
            accuracy,
            confidence_score,
            passed,
            answer_scores,
            summary,
        }
    }

    /// Score a single answer against expected answers.
    fn score_single_answer(
        question: &VerificationQuestion,
        user_answer: Option<&str>,
    ) -> (bool, f64, String) {
        let answer = match user_answer {
            Some(a) if !a.trim().is_empty() => a.trim().to_lowercase(),
            _ => {
                return (false, 0.0, "No answer provided".to_string());
            }
        };

        // Check for exact or fuzzy match against expected answers
        for expected in &question.expected_answers {
            let expected_lower = expected.to_lowercase();

            // Exact match
            if answer == expected_lower {
                return (true, 1.0, "Correct!".to_string());
            }

            // Fuzzy match - answer contains expected or vice versa
            if answer.contains(&expected_lower) || expected_lower.contains(&answer) {
                return (true, 0.9, "Close enough - accepted.".to_string());
            }

            // For numeric answers, check if close
            if let (Ok(a), Ok(e)) = (answer.parse::<f64>(), expected_lower.parse::<f64>()) {
                let diff = (a - e).abs();
                if diff <= 10.0 {
                    return (true, 0.8, "Within acceptable range.".to_string());
                }
            }
        }

        // Partial credit for similar answers (basic similarity check)
        let best_similarity = question
            .expected_answers
            .iter()
            .map(|e| Self::simple_similarity(&answer, &e.to_lowercase()))
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);

        if best_similarity >= 0.7 {
            (false, 0.5, "Partially correct.".to_string())
        } else {
            (false, 0.0, "Incorrect.".to_string())
        }
    }

    /// Simple character-based similarity (0.0 - 1.0)
    fn simple_similarity(a: &str, b: &str) -> f64 {
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }

        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();
        let common = a_chars.iter().filter(|c| b_chars.contains(c)).count();

        (2.0 * common as f64) / (a_chars.len() + b_chars.len()) as f64
    }

    /// Create questions from profile data, stripped of answers for client.
    pub fn questions_for_client(questions: &[VerificationQuestion]) -> Vec<ClientQuestion> {
        questions
            .iter()
            .map(|q| ClientQuestion {
                id: q.id.clone(),
                question: q.question.clone(),
                category: q.category.clone(),
                is_multiple_choice: q.is_multiple_choice,
                options: q.options.clone(),
            })
            .collect()
    }
}

/// Question format safe to send to client (no expected answers)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientQuestion {
    pub id: String,
    pub question: String,
    pub category: QuestionCategory,
    pub is_multiple_choice: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_profile() -> UserProfileData {
        UserProfileData {
            human_id: "human-123".to_string(),
            display_name: "Test User".to_string(),
            affinities: vec!["Technology".to_string(), "Philosophy".to_string()],
            completed_paths: vec![
                PathCompletion {
                    path_id: "elohim-protocol".to_string(),
                    path_title: "Elohim Protocol Foundations".to_string(),
                    completed_at: "2024-12-01".to_string(),
                },
                PathCompletion {
                    path_id: "governance".to_string(),
                    path_title: "Governance Principles".to_string(),
                    completed_at: "2024-11-15".to_string(),
                },
            ],
            quiz_scores: vec![QuizScore {
                quiz_id: "quiz-manifesto".to_string(),
                quiz_title: "Manifesto Foundations".to_string(),
                score: 8.0,
                max_score: 10.0,
                completed_at: "2024-12-05".to_string(),
            }],
            relationship_names: vec!["Alice".to_string(), "Bob".to_string()],
            learning_preferences: Some(LearningPreferences {
                preferred_style: Some("Visual".to_string()),
                daily_goal_minutes: Some(30),
                notification_enabled: true,
            }),
            milestones: vec!["First Path Complete".to_string()],
            created_at: "2024-06-15".to_string(),
        }
    }

    #[test]
    fn test_generate_questions() {
        let profile = sample_profile();
        let questions = ElohimVerifier::generate_questions(&profile);

        assert!(!questions.is_empty());
        assert!(questions.len() <= QUESTION_COUNT);

        // Should have content mastery question
        assert!(questions
            .iter()
            .any(|q| q.category == QuestionCategory::ContentMastery));
    }

    #[test]
    fn test_score_correct_answers() {
        let profile = sample_profile();
        let questions = ElohimVerifier::generate_questions(&profile);

        // Create correct answers
        let answers: Vec<QuestionAnswer> = questions
            .iter()
            .map(|q| QuestionAnswer {
                question_id: q.id.clone(),
                answer: q.expected_answers.first().cloned().unwrap_or_default(),
            })
            .collect();

        let result = ElohimVerifier::score_answers(&questions, &answers);

        assert!(result.accuracy >= 0.9);
        assert!(result.passed);
    }

    #[test]
    fn test_score_wrong_answers() {
        let profile = sample_profile();
        let questions = ElohimVerifier::generate_questions(&profile);

        // Create wrong answers
        let answers: Vec<QuestionAnswer> = questions
            .iter()
            .map(|q| QuestionAnswer {
                question_id: q.id.clone(),
                answer: "completely wrong answer xyz".to_string(),
            })
            .collect();

        let result = ElohimVerifier::score_answers(&questions, &answers);

        assert!(result.accuracy < MIN_ACCURACY_THRESHOLD);
        assert!(!result.passed);
    }

    #[test]
    fn test_fuzzy_matching() {
        let question = VerificationQuestion {
            id: "test".to_string(),
            question: "Test?".to_string(),
            category: QuestionCategory::ContentMastery,
            expected_answers: vec!["Elohim Protocol Foundations".to_string()],
            is_multiple_choice: false,
            options: None,
            weight: 1.0,
        };

        // Partial match should score
        let (correct, score, _) =
            ElohimVerifier::score_single_answer(&question, Some("elohim protocol"));
        assert!(correct);
        assert!(score >= 0.8);
    }

    #[test]
    fn test_client_questions_no_answers() {
        let profile = sample_profile();
        let questions = ElohimVerifier::generate_questions(&profile);
        let client_questions = ElohimVerifier::questions_for_client(&questions);

        // Serialize and check no answers leak
        let json = serde_json::to_string(&client_questions).unwrap();
        assert!(!json.contains("expected_answers"));
        assert!(!json.contains("Elohim Protocol Foundations")); // Actual answer shouldn't be in question
    }
}

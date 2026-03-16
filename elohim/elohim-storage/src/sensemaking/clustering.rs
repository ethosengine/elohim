//! Agglomerative clustering of participants by vote similarity.
//!
//! Algorithm:
//! 1. Build vote matrix: rows = humans, cols = statements, values = agree(1)/disagree(-1)/pass(0)
//! 2. Cosine similarity between human vote vectors
//! 3. Agglomerative clustering: start with each human as own cluster, merge most-similar until threshold
//! 4. For each cluster: characteristic statements = >70% cluster agreement
//! 5. Bridging statements = >60% agreement in EVERY cluster

use std::collections::{HashMap, HashSet};

use crate::db::models::{Statement, StatementVote};
use crate::views::{OpinionClusterView, SensemakingResultView, StatementView};

/// Similarity threshold below which clusters stop merging
const MERGE_THRESHOLD: f64 = 0.3;

/// Minimum agreement ratio within a cluster for a statement to be characteristic
const CHARACTERISTIC_THRESHOLD: f64 = 0.7;

/// Minimum agreement ratio across ALL clusters for a statement to be bridging
const BRIDGING_THRESHOLD: f64 = 0.6;

/// Run opinion clustering on statements and votes for an entity.
pub fn cluster_opinions(
    entity_type: &str,
    entity_id: &str,
    statements: &[Statement],
    votes: &[StatementVote],
) -> SensemakingResultView {
    if statements.is_empty() || votes.is_empty() {
        return SensemakingResultView {
            entity_type: entity_type.to_string(),
            entity_id: entity_id.to_string(),
            clusters: Vec::new(),
            bridging_statements: Vec::new(),
            total_participants: 0,
            total_statements: statements.len(),
        };
    }

    // Collect unique humans and statement IDs
    let human_ids: Vec<String> = votes
        .iter()
        .map(|v| v.human_id.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let stmt_ids: Vec<String> = statements.iter().map(|s| s.id.clone()).collect();
    let stmt_index: HashMap<&str, usize> = stmt_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), i))
        .collect();

    if human_ids.len() < 2 {
        // Single participant — one cluster with all statements as characteristic
        let characteristic: Vec<StatementView> = statements
            .iter()
            .cloned()
            .map(StatementView::from)
            .collect();
        return SensemakingResultView {
            entity_type: entity_type.to_string(),
            entity_id: entity_id.to_string(),
            clusters: vec![OpinionClusterView {
                id: "cluster-0".to_string(),
                member_count: human_ids.len(),
                characteristic_statements: characteristic,
                internal_agreement: 1.0,
            }],
            bridging_statements: Vec::new(),
            total_participants: human_ids.len(),
            total_statements: statements.len(),
        };
    }

    // Build vote matrix: human_index -> vec of vote values per statement
    let human_index: HashMap<&str, usize> = human_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), i))
        .collect();
    let n_humans = human_ids.len();
    let n_stmts = stmt_ids.len();

    let mut matrix: Vec<Vec<f64>> = vec![vec![0.0; n_stmts]; n_humans];
    for v in votes {
        if let (Some(&hi), Some(&si)) = (
            human_index.get(v.human_id.as_str()),
            stmt_index.get(v.statement_id.as_str()),
        ) {
            matrix[hi][si] = match v.vote.as_str() {
                "agree" => 1.0,
                "disagree" => -1.0,
                _ => 0.0, // pass or unknown
            };
        }
    }

    // Compute similarity matrix (cosine similarity)
    let sim_matrix = compute_similarity_matrix(&matrix);

    // Agglomerative clustering
    let cluster_assignments = agglomerative_cluster(&sim_matrix, n_humans, MERGE_THRESHOLD);

    // Group humans by cluster
    let mut cluster_members: HashMap<usize, Vec<usize>> = HashMap::new();
    for (human_idx, &cluster_id) in cluster_assignments.iter().enumerate() {
        cluster_members
            .entry(cluster_id)
            .or_default()
            .push(human_idx);
    }

    // Build statement lookup
    let stmt_map: HashMap<&str, &Statement> =
        statements.iter().map(|s| (s.id.as_str(), s)).collect();

    // For each cluster, find characteristic statements
    let mut clusters: Vec<OpinionClusterView> = Vec::new();
    let mut cluster_ids_sorted: Vec<usize> = cluster_members.keys().cloned().collect();
    cluster_ids_sorted.sort();

    for (idx, &cid) in cluster_ids_sorted.iter().enumerate() {
        let members = &cluster_members[&cid];
        let member_count = members.len();

        // Compute characteristic statements (>70% agreement within cluster)
        let mut characteristic: Vec<StatementView> = Vec::new();
        for (si, stmt_id) in stmt_ids.iter().enumerate() {
            let agree_count = members.iter().filter(|&&hi| matrix[hi][si] > 0.5).count();
            let ratio = agree_count as f64 / member_count as f64;
            if ratio >= CHARACTERISTIC_THRESHOLD {
                if let Some(&stmt) = stmt_map.get(stmt_id.as_str()) {
                    characteristic.push(StatementView::from(stmt.clone()));
                }
            }
        }

        // Compute internal agreement as average pairwise similarity
        let internal_agreement = if members.len() < 2 {
            1.0
        } else {
            let mut sum = 0.0;
            let mut count = 0;
            for i in 0..members.len() {
                for j in (i + 1)..members.len() {
                    sum += sim_matrix[members[i]][members[j]];
                    count += 1;
                }
            }
            if count > 0 {
                sum / count as f64
            } else {
                1.0
            }
        };

        clusters.push(OpinionClusterView {
            id: format!("cluster-{}", idx),
            member_count,
            characteristic_statements: characteristic,
            internal_agreement,
        });
    }

    // Find bridging statements (>60% agreement in EVERY cluster)
    let mut bridging: Vec<StatementView> = Vec::new();
    if clusters.len() >= 2 {
        for (si, stmt_id) in stmt_ids.iter().enumerate() {
            let bridges_all = cluster_ids_sorted.iter().all(|&cid| {
                let members = &cluster_members[&cid];
                let agree_count = members.iter().filter(|&&hi| matrix[hi][si] > 0.5).count();
                let ratio = agree_count as f64 / members.len() as f64;
                ratio >= BRIDGING_THRESHOLD
            });
            if bridges_all {
                if let Some(&stmt) = stmt_map.get(stmt_id.as_str()) {
                    bridging.push(StatementView::from(stmt.clone()));
                }
            }
        }
    }

    SensemakingResultView {
        entity_type: entity_type.to_string(),
        entity_id: entity_id.to_string(),
        clusters,
        bridging_statements: bridging,
        total_participants: n_humans,
        total_statements: statements.len(),
    }
}

/// Compute pairwise cosine similarity matrix between human vote vectors.
fn compute_similarity_matrix(matrix: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = matrix.len();
    let mut sim = vec![vec![0.0f64; n]; n];

    for i in 0..n {
        sim[i][i] = 1.0;
        for j in (i + 1)..n {
            let s = cosine_similarity(&matrix[i], &matrix[j]);
            sim[i][j] = s;
            sim[j][i] = s;
        }
    }
    sim
}

/// Cosine similarity between two vectors. Returns 0.0 if either is zero-length.
fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let mag_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if mag_a < 1e-10 || mag_b < 1e-10 {
        0.0
    } else {
        dot / (mag_a * mag_b)
    }
}

/// Agglomerative clustering using average linkage.
/// Returns a vector of cluster IDs, one per human.
fn agglomerative_cluster(sim_matrix: &[Vec<f64>], n: usize, threshold: f64) -> Vec<usize> {
    // Each human starts in their own cluster
    let mut assignments: Vec<usize> = (0..n).collect();
    let mut next_cluster_id = n;

    // Track which cluster IDs are active and their members
    let mut active_clusters: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..n {
        active_clusters.insert(i, vec![i]);
    }

    loop {
        // Find the two most similar clusters
        let active_ids: Vec<usize> = active_clusters.keys().cloned().collect();
        if active_ids.len() < 2 {
            break;
        }

        let mut best_sim = f64::NEG_INFINITY;
        let mut best_pair = (0, 0);

        for i in 0..active_ids.len() {
            for j in (i + 1)..active_ids.len() {
                let ci = active_ids[i];
                let cj = active_ids[j];
                let sim = average_linkage_similarity(
                    &active_clusters[&ci],
                    &active_clusters[&cj],
                    sim_matrix,
                );
                if sim > best_sim {
                    best_sim = sim;
                    best_pair = (ci, cj);
                }
            }
        }

        // Stop if best similarity is below threshold
        if best_sim < threshold {
            break;
        }

        // Merge the two clusters
        let (c1, c2) = best_pair;
        let mut merged = active_clusters.remove(&c1).unwrap();
        merged.extend(active_clusters.remove(&c2).unwrap());

        // Update assignments
        for &hi in &merged {
            assignments[hi] = next_cluster_id;
        }
        active_clusters.insert(next_cluster_id, merged);
        next_cluster_id += 1;
    }

    assignments
}

/// Average linkage: mean similarity between all pairs across two clusters.
fn average_linkage_similarity(
    cluster_a: &[usize],
    cluster_b: &[usize],
    sim_matrix: &[Vec<f64>],
) -> f64 {
    let mut sum = 0.0;
    let count = cluster_a.len() * cluster_b.len();
    for &a in cluster_a {
        for &b in cluster_b {
            sum += sim_matrix[a][b];
        }
    }
    if count > 0 {
        sum / count as f64
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{Statement, StatementVote};

    fn make_statement(id: &str, entity_type: &str, entity_id: &str) -> Statement {
        Statement {
            id: id.to_string(),
            entity_type: entity_type.to_string(),
            entity_id: entity_id.to_string(),
            human_id: "author".to_string(),
            text: format!("Statement {}", id),
            agree_count: 0,
            disagree_count: 0,
            pass_count: 0,
            group_id: None,
            is_bridging: 0,
            created_at: "2026-03-16T00:00:00Z".to_string(),
        }
    }

    fn make_vote(statement_id: &str, human_id: &str, vote: &str) -> StatementVote {
        StatementVote {
            id: format!("sv-{}-{}", statement_id, human_id),
            statement_id: statement_id.to_string(),
            human_id: human_id.to_string(),
            vote: vote.to_string(),
            created_at: "2026-03-16T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn two_groups_with_opposite_votes_produce_two_clusters() {
        let stmts = vec![
            make_statement("s1", "proposal", "p1"),
            make_statement("s2", "proposal", "p1"),
        ];
        let votes = vec![
            // Group A: agrees with s1, disagrees with s2
            make_vote("s1", "alice", "agree"),
            make_vote("s2", "alice", "disagree"),
            make_vote("s1", "bob", "agree"),
            make_vote("s2", "bob", "disagree"),
            // Group B: disagrees with s1, agrees with s2
            make_vote("s1", "carol", "disagree"),
            make_vote("s2", "carol", "agree"),
            make_vote("s1", "dave", "disagree"),
            make_vote("s2", "dave", "agree"),
        ];

        let result = cluster_opinions("proposal", "p1", &stmts, &votes);
        assert_eq!(result.total_participants, 4);
        assert_eq!(result.total_statements, 2);
        assert_eq!(result.clusters.len(), 2, "Should produce 2 clusters");
        // Each cluster should have 2 members
        for c in &result.clusters {
            assert_eq!(c.member_count, 2);
        }
    }

    #[test]
    fn unanimous_agreement_produces_one_cluster_and_bridging() {
        let stmts = vec![
            make_statement("s1", "proposal", "p1"),
            make_statement("s2", "proposal", "p1"),
        ];
        let votes = vec![
            make_vote("s1", "alice", "agree"),
            make_vote("s2", "alice", "agree"),
            make_vote("s1", "bob", "agree"),
            make_vote("s2", "bob", "agree"),
            make_vote("s1", "carol", "agree"),
            make_vote("s2", "carol", "agree"),
        ];

        let result = cluster_opinions("proposal", "p1", &stmts, &votes);
        assert_eq!(result.total_participants, 3);
        assert_eq!(
            result.clusters.len(),
            1,
            "Unanimous should produce 1 cluster"
        );
        assert_eq!(result.clusters[0].member_count, 3);
        // With only one cluster, bridging is not computed (requires >=2 clusters)
        assert!(result.bridging_statements.is_empty());
    }

    #[test]
    fn no_votes_returns_empty_result() {
        let stmts = vec![make_statement("s1", "proposal", "p1")];
        let votes: Vec<StatementVote> = vec![];

        let result = cluster_opinions("proposal", "p1", &stmts, &votes);
        assert_eq!(result.total_participants, 0);
        assert_eq!(result.total_statements, 1);
        assert!(result.clusters.is_empty());
        assert!(result.bridging_statements.is_empty());
    }

    #[test]
    fn single_voter_produces_one_cluster() {
        let stmts = vec![
            make_statement("s1", "proposal", "p1"),
            make_statement("s2", "proposal", "p1"),
        ];
        let votes = vec![
            make_vote("s1", "alice", "agree"),
            make_vote("s2", "alice", "disagree"),
        ];

        let result = cluster_opinions("proposal", "p1", &stmts, &votes);
        assert_eq!(result.total_participants, 1);
        assert_eq!(result.clusters.len(), 1);
        assert_eq!(result.clusters[0].member_count, 1);
        assert_eq!(result.clusters[0].internal_agreement, 1.0);
    }

    #[test]
    fn cosine_similarity_orthogonal_vectors() {
        assert!((cosine_similarity(&[1.0, 0.0], &[0.0, 1.0])).abs() < 1e-10);
    }

    #[test]
    fn cosine_similarity_identical_vectors() {
        let s = cosine_similarity(&[1.0, 1.0], &[1.0, 1.0]);
        assert!((s - 1.0).abs() < 1e-10);
    }

    #[test]
    fn cosine_similarity_opposite_vectors() {
        let s = cosine_similarity(&[1.0, 1.0], &[-1.0, -1.0]);
        assert!((s - (-1.0)).abs() < 1e-10);
    }

    #[test]
    fn bridging_statement_found_when_two_clusters_agree_on_one() {
        let stmts = vec![
            make_statement("s1", "proposal", "p1"),
            make_statement("s2", "proposal", "p1"),
            make_statement("s3", "proposal", "p1"), // bridging: everyone agrees
        ];
        let votes = vec![
            // Group A: agrees s1, disagrees s2, agrees s3
            make_vote("s1", "alice", "agree"),
            make_vote("s2", "alice", "disagree"),
            make_vote("s3", "alice", "agree"),
            make_vote("s1", "bob", "agree"),
            make_vote("s2", "bob", "disagree"),
            make_vote("s3", "bob", "agree"),
            // Group B: disagrees s1, agrees s2, agrees s3
            make_vote("s1", "carol", "disagree"),
            make_vote("s2", "carol", "agree"),
            make_vote("s3", "carol", "agree"),
            make_vote("s1", "dave", "disagree"),
            make_vote("s2", "dave", "agree"),
            make_vote("s3", "dave", "agree"),
        ];

        let result = cluster_opinions("proposal", "p1", &stmts, &votes);
        assert_eq!(result.clusters.len(), 2);
        assert!(
            !result.bridging_statements.is_empty(),
            "s3 should be a bridging statement"
        );
        assert!(result.bridging_statements.iter().any(|s| s.id == "s3"));
    }
}

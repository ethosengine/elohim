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
use crate::views::{
    OpinionClusterView, ParticipantPositionView, SensemakingResultView, StatementMetricsView,
    StatementView,
};

use super::pca;

/// Similarity threshold below which clusters stop merging
const MERGE_THRESHOLD: f64 = 0.3;

/// Minimum agreement ratio within a cluster for a statement to be characteristic
const CHARACTERISTIC_THRESHOLD: f64 = 0.7;

/// Minimum agreement ratio across ALL clusters for a statement to be bridging
const BRIDGING_THRESHOLD: f64 = 0.6;

/// Minimum cross-cluster variance for a statement to be classified as divisive
const DIVISIVE_THRESHOLD: f64 = 0.15;

/// Run opinion clustering on statements and votes for an entity.
///
/// Returns clusters, bridging/divisive statements, per-statement metrics,
/// and PCA-projected 2D positions for visualization.
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
            divisive_statements: Vec::new(),
            statement_metrics: Vec::new(),
            participant_positions: Vec::new(),
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
        // Single participant — one cluster, no meaningful projection
        let characteristic: Vec<StatementView> = statements
            .iter()
            .cloned()
            .map(StatementView::from)
            .collect();
        let positions = vec![ParticipantPositionView {
            human_id: human_ids[0].clone(),
            x: 0.0,
            y: 0.0,
            cluster_id: "cluster-0".to_string(),
        }];
        let metrics = compute_statement_metrics_single(&stmt_ids, statements);
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
            divisive_statements: Vec::new(),
            statement_metrics: metrics,
            participant_positions: positions,
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

    // Build a map from human_index -> cluster view index (for positions)
    let mut human_to_cluster_view: Vec<String> = vec![String::new(); n_humans];

    for (idx, &cid) in cluster_ids_sorted.iter().enumerate() {
        let members = &cluster_members[&cid];
        let member_count = members.len();
        let cluster_view_id = format!("cluster-{}", idx);

        // Record cluster assignment for each member
        for &hi in members {
            human_to_cluster_view[hi] = cluster_view_id.clone();
        }

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
            id: cluster_view_id,
            member_count,
            characteristic_statements: characteristic,
            internal_agreement,
        });
    }

    // Compute per-statement metrics and classify
    let (metrics, bridging, divisive) = compute_statement_metrics(
        &stmt_ids,
        &matrix,
        &cluster_ids_sorted,
        &cluster_members,
        &stmt_map,
    );

    // PCA projection for 2D visualization
    let projections = pca::project_2d(&matrix);
    let participant_positions: Vec<ParticipantPositionView> = human_ids
        .iter()
        .enumerate()
        .map(|(i, hid)| ParticipantPositionView {
            human_id: hid.clone(),
            x: projections.get(i).map_or(0.0, |p| p.0),
            y: projections.get(i).map_or(0.0, |p| p.1),
            cluster_id: human_to_cluster_view[i].clone(),
        })
        .collect();

    SensemakingResultView {
        entity_type: entity_type.to_string(),
        entity_id: entity_id.to_string(),
        clusters,
        bridging_statements: bridging,
        divisive_statements: divisive,
        statement_metrics: metrics,
        participant_positions,
        total_participants: n_humans,
        total_statements: statements.len(),
    }
}

/// Compute per-statement metrics for the single-participant case.
fn compute_statement_metrics_single(
    stmt_ids: &[String],
    statements: &[Statement],
) -> Vec<StatementMetricsView> {
    let stmt_map: HashMap<&str, &Statement> =
        statements.iter().map(|s| (s.id.as_str(), s)).collect();
    stmt_ids
        .iter()
        .filter_map(|sid| {
            stmt_map.get(sid.as_str()).map(|_| StatementMetricsView {
                statement_id: sid.clone(),
                overall_agreement: 0.0,
                cross_cluster_variance: 0.0,
                classification: "neutral".to_string(),
            })
        })
        .collect()
}

/// Compute per-statement metrics, and return classified bridging + divisive statements.
///
/// For each statement:
/// - `overall_agreement`: mean vote value across all participants (-1 to 1)
/// - `cross_cluster_variance`: variance of per-cluster agreement ratios
///   - Low variance + high agreement across clusters → bridging
///   - High variance → divisive (clusters disagree about this statement)
/// - `classification`: "bridging", "divisive", "characteristic", or "neutral"
fn compute_statement_metrics(
    stmt_ids: &[String],
    matrix: &[Vec<f64>],
    cluster_ids_sorted: &[usize],
    cluster_members: &HashMap<usize, Vec<usize>>,
    stmt_map: &HashMap<&str, &Statement>,
) -> (
    Vec<StatementMetricsView>,
    Vec<StatementView>,
    Vec<StatementView>,
) {
    let n_clusters = cluster_ids_sorted.len();
    let mut metrics = Vec::with_capacity(stmt_ids.len());
    let mut bridging = Vec::new();
    let mut divisive = Vec::new();

    for (si, stmt_id) in stmt_ids.iter().enumerate() {
        // Overall agreement: mean vote value across all humans
        let all_votes: Vec<f64> = matrix.iter().map(|row| row[si]).collect();
        let overall_agreement = if all_votes.is_empty() {
            0.0
        } else {
            all_votes.iter().sum::<f64>() / all_votes.len() as f64
        };

        // Per-cluster agreement ratio (fraction who agree)
        let cluster_ratios: Vec<f64> = cluster_ids_sorted
            .iter()
            .map(|&cid| {
                let members = &cluster_members[&cid];
                if members.is_empty() {
                    return 0.0;
                }
                let agree_count = members.iter().filter(|&&hi| matrix[hi][si] > 0.5).count();
                agree_count as f64 / members.len() as f64
            })
            .collect();

        // Cross-cluster variance of agreement ratios
        let cross_cluster_variance = if n_clusters < 2 {
            0.0
        } else {
            let mean_ratio = cluster_ratios.iter().sum::<f64>() / n_clusters as f64;
            cluster_ratios
                .iter()
                .map(|r| (r - mean_ratio).powi(2))
                .sum::<f64>()
                / n_clusters as f64
        };

        // Classify the statement
        let is_bridging = n_clusters >= 2
            && cluster_ratios
                .iter()
                .all(|&ratio| ratio >= BRIDGING_THRESHOLD);
        let is_divisive = n_clusters >= 2 && cross_cluster_variance >= DIVISIVE_THRESHOLD;

        let classification = if is_bridging {
            "bridging"
        } else if is_divisive {
            "divisive"
        } else {
            "neutral"
        };

        if is_bridging {
            if let Some(&stmt) = stmt_map.get(stmt_id.as_str()) {
                bridging.push(StatementView::from(stmt.clone()));
            }
        }
        if is_divisive {
            if let Some(&stmt) = stmt_map.get(stmt_id.as_str()) {
                divisive.push(StatementView::from(stmt.clone()));
            }
        }

        metrics.push(StatementMetricsView {
            statement_id: stmt_id.clone(),
            overall_agreement,
            cross_cluster_variance,
            classification: classification.to_string(),
        });
    }

    (metrics, bridging, divisive)
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
            dht_anchor_hash: None,
        }
    }

    fn make_vote(statement_id: &str, human_id: &str, vote: &str) -> StatementVote {
        StatementVote {
            id: format!("sv-{}-{}", statement_id, human_id),
            statement_id: statement_id.to_string(),
            human_id: human_id.to_string(),
            vote: vote.to_string(),
            created_at: "2026-03-16T00:00:00Z".to_string(),
            dht_anchor_hash: None,
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

        // Participant positions should exist for all 4
        assert_eq!(result.participant_positions.len(), 4);
        // Members of same cluster should have same cluster_id
        let alice_pos = result
            .participant_positions
            .iter()
            .find(|p| p.human_id == "alice")
            .unwrap();
        let bob_pos = result
            .participant_positions
            .iter()
            .find(|p| p.human_id == "bob")
            .unwrap();
        assert_eq!(alice_pos.cluster_id, bob_pos.cluster_id);

        // Both statements should be divisive (each cluster disagrees with the other)
        assert!(
            !result.divisive_statements.is_empty(),
            "Polarized statements should be divisive"
        );

        // Statement metrics should classify correctly
        assert_eq!(result.statement_metrics.len(), 2);
        for m in &result.statement_metrics {
            assert_eq!(
                m.classification, "divisive",
                "Stmt {} should be divisive",
                m.statement_id
            );
            assert!(
                m.cross_cluster_variance > 0.1,
                "Divisive stmt should have high cross-cluster variance"
            );
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

        // Positions should exist
        assert_eq!(result.participant_positions.len(), 3);
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
        assert!(result.participant_positions.is_empty());
        assert!(result.statement_metrics.is_empty());
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

        // Single participant gets position at origin
        assert_eq!(result.participant_positions.len(), 1);
        assert_eq!(result.participant_positions[0].human_id, "alice");
        assert_eq!(result.participant_positions[0].cluster_id, "cluster-0");
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

        // s3 should be classified as bridging in metrics
        let s3_metrics = result
            .statement_metrics
            .iter()
            .find(|m| m.statement_id == "s3")
            .unwrap();
        assert_eq!(s3_metrics.classification, "bridging");
        assert!(
            s3_metrics.cross_cluster_variance < DIVISIVE_THRESHOLD,
            "Bridging statement should have low cross-cluster variance"
        );

        // s1 and s2 should be divisive (each cluster splits on them)
        let s1_metrics = result
            .statement_metrics
            .iter()
            .find(|m| m.statement_id == "s1")
            .unwrap();
        assert_eq!(s1_metrics.classification, "divisive");

        // Positions should separate the two groups
        assert_eq!(result.participant_positions.len(), 4);
    }

    #[test]
    fn participant_positions_from_pca_separate_groups() {
        let stmts = vec![
            make_statement("s1", "proposal", "p1"),
            make_statement("s2", "proposal", "p1"),
            make_statement("s3", "proposal", "p1"),
        ];
        let votes = vec![
            // Group A: all agree
            make_vote("s1", "alice", "agree"),
            make_vote("s2", "alice", "agree"),
            make_vote("s3", "alice", "agree"),
            make_vote("s1", "bob", "agree"),
            make_vote("s2", "bob", "agree"),
            make_vote("s3", "bob", "agree"),
            // Group B: all disagree
            make_vote("s1", "carol", "disagree"),
            make_vote("s2", "carol", "disagree"),
            make_vote("s3", "carol", "disagree"),
            make_vote("s1", "dave", "disagree"),
            make_vote("s2", "dave", "disagree"),
            make_vote("s3", "dave", "disagree"),
        ];

        let result = cluster_opinions("proposal", "p1", &stmts, &votes);

        // Participants in same group should be at the same position
        let alice = result
            .participant_positions
            .iter()
            .find(|p| p.human_id == "alice")
            .unwrap();
        let bob = result
            .participant_positions
            .iter()
            .find(|p| p.human_id == "bob")
            .unwrap();
        let carol = result
            .participant_positions
            .iter()
            .find(|p| p.human_id == "carol")
            .unwrap();

        assert!(
            (alice.x - bob.x).abs() < 1e-10 && (alice.y - bob.y).abs() < 1e-10,
            "Same-group members should be at same position"
        );

        // Different groups should be separated
        let dist = ((alice.x - carol.x).powi(2) + (alice.y - carol.y).powi(2)).sqrt();
        assert!(
            dist > 0.5,
            "Different groups should be separated in 2D, dist={}",
            dist
        );
    }
}

//! Minimal PCA (Principal Component Analysis) for 2D projection.
//!
//! Computes the top-2 principal components of a vote matrix so participants
//! can be projected into a 2D opinion landscape for visualization.
//!
//! Algorithm:
//! 1. Center the matrix (subtract column means)
//! 2. Compute covariance matrix
//! 3. Find top-2 eigenvectors via power iteration
//! 4. Project rows onto those eigenvectors → (x, y) coordinates

/// Project an N×M matrix down to N×2 using the top-2 principal components.
/// Returns a Vec of (x, y) pairs, one per row, normalized to [-1, 1].
pub fn project_2d(matrix: &[Vec<f64>]) -> Vec<(f64, f64)> {
    let n_rows = matrix.len();
    if n_rows == 0 {
        return Vec::new();
    }
    let n_cols = matrix[0].len();
    if n_cols == 0 {
        return vec![(0.0, 0.0); n_rows];
    }

    // 1. Center the matrix (subtract column means)
    let col_means: Vec<f64> = (0..n_cols)
        .map(|j| matrix.iter().map(|row| row[j]).sum::<f64>() / n_rows as f64)
        .collect();

    let centered: Vec<Vec<f64>> = matrix
        .iter()
        .map(|row| {
            row.iter()
                .zip(col_means.iter())
                .map(|(v, m)| v - m)
                .collect()
        })
        .collect();

    // 2. Compute covariance matrix (M×M)
    let cov = covariance_matrix(&centered, n_cols);

    // 3. Find top-2 eigenvectors via power iteration
    let pc1 = power_iteration(&cov, n_cols, 100, None);
    let pc2 = power_iteration(&cov, n_cols, 100, Some(&pc1));

    // 4. Project each row onto PC1 and PC2
    let mut projections: Vec<(f64, f64)> = centered
        .iter()
        .map(|row| {
            let x = dot(row, &pc1);
            let y = dot(row, &pc2);
            (x, y)
        })
        .collect();

    // 5. Normalize to [-1, 1] range
    normalize_projections(&mut projections);

    projections
}

/// Compute the M×M covariance matrix from a centered N×M matrix.
fn covariance_matrix(centered: &[Vec<f64>], n_cols: usize) -> Vec<Vec<f64>> {
    let n = centered.len() as f64;
    if n <= 1.0 {
        return vec![vec![0.0; n_cols]; n_cols];
    }

    let mut cov = vec![vec![0.0f64; n_cols]; n_cols];
    for row in centered {
        for i in 0..n_cols {
            for j in i..n_cols {
                let v = row[i] * row[j];
                cov[i][j] += v;
                if i != j {
                    cov[j][i] += v;
                }
            }
        }
    }

    let denom = n - 1.0;
    for row in cov.iter_mut() {
        for val in row.iter_mut() {
            *val /= denom;
        }
    }
    cov
}

/// Power iteration to find the dominant eigenvector of a symmetric matrix.
/// If `deflate_against` is provided, deflates to find the next eigenvector.
fn power_iteration(
    matrix: &[Vec<f64>],
    dim: usize,
    max_iter: usize,
    deflate_against: Option<&[f64]>,
) -> Vec<f64> {
    if dim == 0 {
        return Vec::new();
    }

    // Deflate the matrix if finding second eigenvector
    let mat = if let Some(v1) = deflate_against {
        deflate(matrix, v1, dim)
    } else {
        matrix.to_vec()
    };

    // Start with a non-zero vector (alternating 1, -1 to avoid unlucky alignment)
    let mut v: Vec<f64> = (0..dim)
        .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
        .collect();
    let mag = norm(&v);
    if mag > 1e-10 {
        for x in &mut v {
            *x /= mag;
        }
    }

    for _ in 0..max_iter {
        // Multiply: w = mat * v
        let w: Vec<f64> = (0..dim)
            .map(|i| mat[i].iter().zip(v.iter()).map(|(a, b)| a * b).sum())
            .collect();

        let w_norm = norm(&w);
        if w_norm < 1e-10 {
            // Matrix is zero in this direction — return current v
            break;
        }

        let new_v: Vec<f64> = w.iter().map(|x| x / w_norm).collect();

        // Check convergence (dot product of old and new ≈ 1)
        let convergence = dot(&v, &new_v).abs();
        v = new_v;
        if convergence > 1.0 - 1e-10 {
            break;
        }
    }

    v
}

/// Deflate a matrix by removing the component along eigenvector v1:
/// M' = M - λ * v1 * v1^T, where λ = v1^T * M * v1
fn deflate(matrix: &[Vec<f64>], v1: &[f64], dim: usize) -> Vec<Vec<f64>> {
    // Compute eigenvalue: λ = v1^T * M * v1
    let mv1: Vec<f64> = (0..dim)
        .map(|i| matrix[i].iter().zip(v1.iter()).map(|(a, b)| a * b).sum())
        .collect();
    let eigenvalue = dot(v1, &mv1);

    // Deflate: M' = M - λ * v1 * v1^T
    let mut deflated = matrix.to_vec();
    for i in 0..dim {
        for j in 0..dim {
            deflated[i][j] -= eigenvalue * v1[i] * v1[j];
        }
    }
    deflated
}

/// Dot product of two vectors.
fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Euclidean norm of a vector.
fn norm(v: &[f64]) -> f64 {
    v.iter().map(|x| x * x).sum::<f64>().sqrt()
}

/// Normalize (x, y) projections to [-1, 1] range.
fn normalize_projections(projections: &mut [(f64, f64)]) {
    if projections.is_empty() {
        return;
    }

    let max_abs = projections
        .iter()
        .flat_map(|(x, y)| [x.abs(), y.abs()])
        .fold(0.0f64, f64::max);

    if max_abs > 1e-10 {
        for p in projections.iter_mut() {
            p.0 /= max_abs;
            p.1 /= max_abs;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_matrix_returns_empty() {
        let result = project_2d(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn single_row_returns_origin() {
        let result = project_2d(&[vec![1.0, -1.0, 0.0]]);
        assert_eq!(result.len(), 1);
        // Single row after centering is all zeros
        assert!(result[0].0.abs() < 1e-10);
        assert!(result[0].1.abs() < 1e-10);
    }

    #[test]
    fn two_opposite_voters_project_to_opposite_ends() {
        let matrix = vec![vec![1.0, 1.0, 1.0], vec![-1.0, -1.0, -1.0]];
        let result = project_2d(&matrix);
        assert_eq!(result.len(), 2);
        // Should be at opposite ends of the x-axis (or y-axis)
        let (x1, _) = result[0];
        let (x2, _) = result[1];
        // They should be on opposite sides, normalized to ~±1
        assert!(
            (x1 - (-x2)).abs() < 0.1,
            "Expected opposite positions, got ({}, {})",
            x1,
            x2
        );
    }

    #[test]
    fn identical_voters_cluster_at_same_point() {
        let matrix = vec![
            vec![1.0, -1.0, 0.0],
            vec![1.0, -1.0, 0.0],
            vec![1.0, -1.0, 0.0],
        ];
        let result = project_2d(&matrix);
        assert_eq!(result.len(), 3);
        // All should be at approximately the same point
        for i in 1..3 {
            assert!((result[i].0 - result[0].0).abs() < 1e-10);
            assert!((result[i].1 - result[0].1).abs() < 1e-10);
        }
    }

    #[test]
    fn three_distinct_groups_spread_in_2d() {
        let matrix = vec![
            // Group A: agree with s1-s3, disagree with s4-s6
            vec![1.0, 1.0, 1.0, -1.0, -1.0, -1.0],
            vec![1.0, 1.0, 1.0, -1.0, -1.0, -1.0],
            // Group B: disagree with s1-s3, agree with s4-s6
            vec![-1.0, -1.0, -1.0, 1.0, 1.0, 1.0],
            vec![-1.0, -1.0, -1.0, 1.0, 1.0, 1.0],
            // Group C: mixed pattern (agree with odd, disagree with even)
            vec![1.0, -1.0, 1.0, -1.0, 1.0, -1.0],
            vec![1.0, -1.0, 1.0, -1.0, 1.0, -1.0],
        ];
        let result = project_2d(&matrix);
        assert_eq!(result.len(), 6);

        // Members of same group should be at same position
        assert!((result[0].0 - result[1].0).abs() < 1e-10);
        assert!((result[2].0 - result[3].0).abs() < 1e-10);
        assert!((result[4].0 - result[5].0).abs() < 1e-10);

        // Different groups should be at different positions
        let dist_ab =
            ((result[0].0 - result[2].0).powi(2) + (result[0].1 - result[2].1).powi(2)).sqrt();
        let dist_ac =
            ((result[0].0 - result[4].0).powi(2) + (result[0].1 - result[4].1).powi(2)).sqrt();
        assert!(
            dist_ab > 0.3,
            "Groups A and B should be separated, dist={}",
            dist_ab
        );
        assert!(
            dist_ac > 0.3,
            "Groups A and C should be separated, dist={}",
            dist_ac
        );
    }

    #[test]
    fn projections_normalized_to_unit_range() {
        let matrix = vec![
            vec![1.0, 1.0, 1.0, 1.0, 1.0],
            vec![-1.0, -1.0, -1.0, -1.0, -1.0],
            vec![0.0, 0.0, 0.0, 0.0, 0.0],
        ];
        let result = project_2d(&matrix);
        for (x, y) in &result {
            assert!(x.abs() <= 1.0 + 1e-10, "x={} exceeds [-1,1]", x);
            assert!(y.abs() <= 1.0 + 1e-10, "y={} exceeds [-1,1]", y);
        }
    }
}

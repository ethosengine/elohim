//! Concentration measures for the per-substrate limitarian governor.
//!
//! Pure, no-I/O math (spec §2, per-substrate-limitarian-governor-design):
//! scale-invariant (z(λD)=z(D)), tail-sensitive, decomposable. Gini ships as a
//! human-readable diagnostic and convergence-test target — it is NOT a friction
//! driver. v1 placement: storage-local (arc decision #2); graduates to a shared
//! crate when a second consumer (WASM measure math) appears.

/// Generalized Entropy index at sensitivity parameter α.
///
/// α=1 → Theil-T (log-weighted, standard economics measure).
/// α=2 → Herfindahl-like (squared, amplifies large shares).
/// Returns 0.0 on empty input or zero-mean distributions.
/// Inputs must be non-negative (x_i >= 0): a negative value with non-integer α
/// produces silent NaN via powf. α must be non-zero (α=0 divides by zero → ±Inf);
/// the governed path clamps α to [1,2] before reaching here.
pub fn ge_alpha(xs: &[f32], alpha: f32) -> f32 {
    debug_assert!(
        xs.iter().all(|&x| x >= 0.0),
        "ge_alpha requires non-negative inputs"
    );
    let n = xs.len();
    if n == 0 {
        return 0.0;
    }
    let mu: f64 = xs.iter().map(|&x| x as f64).sum::<f64>() / n as f64;
    if mu <= 0.0 {
        return 0.0;
    }
    let a = alpha as f64;
    if (a - 1.0).abs() < 1e-6 {
        // Theil-T: (1/N) Σ (x/μ)·ln(x/μ); zero terms contribute 0 (lim x·ln x = 0).
        let s: f64 = xs
            .iter()
            .map(|&x| {
                let r = x as f64 / mu;
                if r > 0.0 {
                    r * r.ln()
                } else {
                    0.0
                }
            })
            .sum();
        (s / n as f64) as f32
    } else {
        let s: f64 = xs.iter().map(|&x| ((x as f64 / mu).powf(a)) - 1.0).sum();
        (s / (n as f64 * a * (a - 1.0))) as f32
    }
}

/// squash(g) = g/(1+g) — the spec's mandated normalizer: fixed, monotone,
/// scale-invariance-preserving (a function of already-scale-invariant GE,
/// introducing NO N- or μ-dependence). Do NOT replace with GE/GE_max(N).
pub fn squash(g: f32) -> f32 {
    if g <= 0.0 {
        0.0
    } else {
        g / (1.0 + g)
    }
}

/// Share of the total held by the top ⌈q·N⌉ holders (q=0.01 default).
/// With small N, the top-1 holder is the tail (⌈q·N⌉ ≥ 1 always).
pub fn top_quantile_share(xs: &[f32], q: f32) -> f32 {
    if xs.is_empty() {
        return 0.0;
    }
    let total: f64 = xs.iter().map(|&x| x as f64).sum();
    if total <= 0.0 {
        return 0.0;
    }
    let mut sorted: Vec<f64> = xs.iter().map(|&x| x as f64).collect();
    sorted.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    let k = ((q as f64 * xs.len() as f64).ceil() as usize)
        .max(1)
        .min(xs.len());
    let top: f64 = sorted[..k].iter().sum();
    (top / total) as f32
}

/// Gini coefficient — DIAGNOSTIC ONLY (human-readable; the convergence test's
/// secondary series). Never a friction driver (spec §2).
pub fn gini(xs: &[f32]) -> f32 {
    let n = xs.len();
    if n == 0 {
        return 0.0;
    }
    let mut sorted: Vec<f64> = xs.iter().map(|&x| x as f64).collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let total: f64 = sorted.iter().sum();
    if total <= 0.0 {
        return 0.0;
    }
    // G = (2·Σ i·x_(i) / (N·Σx)) − (N+1)/N  with 1-based ranks over ascending sort.
    let weighted: f64 = sorted
        .iter()
        .enumerate()
        .map(|(i, &x)| (i as f64 + 1.0) * x)
        .sum();
    ((2.0 * weighted) / (n as f64 * total) - (n as f64 + 1.0) / n as f64) as f32
}

/// The composite concentration C(D) = w_e·squash(GE(α)) + w_s·S_q (spec §2).
pub fn composite_concentration(xs: &[f32], alpha: f32, q: f32, w_e: f32, w_s: f32) -> f32 {
    w_e * squash(ge_alpha(xs, alpha)) + w_s * top_quantile_share(xs, q)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_invariance_z_lambda_d_equals_z_d() {
        let d = vec![100.0_f32, 250.0, 50.0, 900.0, 75.0, 320.0];
        let scaled: Vec<f32> = d.iter().map(|x| x * 37.5).collect();
        for alpha in [1.0_f32, 1.5, 2.0] {
            let a = ge_alpha(&d, alpha);
            let b = ge_alpha(&scaled, alpha);
            assert!(
                (a - b).abs() < 1e-4,
                "GE(α={alpha}) not scale-invariant: {a} vs {b}"
            );
        }
        let c1 = composite_concentration(&d, 2.0, 0.01, 0.6, 0.4);
        let c2 = composite_concentration(&scaled, 2.0, 0.01, 0.6, 0.4);
        assert!((c1 - c2).abs() < 1e-4, "composite not scale-invariant");
        assert!(
            (gini(&d) - gini(&scaled)).abs() < 1e-4,
            "gini not scale-invariant"
        );
    }

    #[test]
    fn equality_zero_on_the_equality_manifold() {
        let equal = vec![500.0_f32; 12];
        for alpha in [1.0_f32, 2.0] {
            assert!(ge_alpha(&equal, alpha).abs() < 1e-6, "GE(equal) must be 0");
        }
        assert!(gini(&equal).abs() < 1e-5, "gini(equal) must be 0");
        // top-share of an equal 12-agent distribution at q=0.01 is exactly 1/12.
        assert!((top_quantile_share(&equal, 0.01) - 1.0 / 12.0).abs() < 1e-5);
    }

    #[test]
    fn tail_sensitivity_one_giant_moves_the_measure() {
        let flat = vec![100.0_f32; 100];
        let mut spiked = flat.clone();
        spiked[0] = 100_000.0;
        assert!(
            ge_alpha(&spiked, 2.0) > ge_alpha(&flat, 2.0) + 1.0,
            "GE(2) must move strongly on a mega-concentrator"
        );
        assert!(
            top_quantile_share(&spiked, 0.01) > 0.9,
            "top-1% share must capture the giant"
        );
    }

    #[test]
    fn squash_is_bounded_monotone_and_fixed() {
        assert_eq!(squash(0.0), 0.0);
        assert!(squash(1.0) - 0.5 < 1e-6);
        assert!(squash(1e7) < 1.0); // 1e7 is safely below the f32 +1-precision wall (ULP=2 from 2^24 ≈ 1.68e7).
        assert!(squash(2.0) > squash(1.0));
    }

    #[test]
    fn degenerate_inputs_do_not_panic() {
        assert_eq!(ge_alpha(&[], 2.0), 0.0);
        assert_eq!(top_quantile_share(&[], 0.01), 0.0);
        assert_eq!(gini(&[]), 0.0);
        assert_eq!(ge_alpha(&[0.0, 0.0], 2.0), 0.0);
    }
}

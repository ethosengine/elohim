//! Cosine, the printed 4-decimal score, and each unit's best chunk.
use elohim_epr_index::rank::{cosine, decode, encode, rounded, BestPerUnit};

#[test]
fn cosine_and_rounded_match_pinned_values() {
    assert!((cosine(&[1.0, 0.0], &[2.0, 0.0]) - 1.0).abs() < 1e-6);
    assert!(cosine(&[1.0, 0.0], &[0.0, 3.0]).abs() < 1e-6);
    assert!((cosine(&[1.0, 1.0], &[-1.0, -1.0]) + 1.0).abs() < 1e-6);
    assert_eq!(cosine(&[0.0, 0.0], &[1.0, 0.0]), 0.0);
    assert_eq!(cosine(&[1.0], &[1.0, 0.0]), 0.0, "widths differ");
    assert_eq!(rounded(std::f64::consts::FRAC_1_SQRT_2), 7071.0 / 10_000.0);
    assert_eq!(rounded(-1.234_56), -1.2346);
    assert_eq!(rounded(0.000_04), 0.0);
    // The pinned f32 cosine of (1,0)·(1,1), widened, prints at 4 decimals.
    assert_eq!(
        rounded(f64::from(cosine(&[1.0, 0.0], &[1.0, 1.0]))),
        7071.0 / 10_000.0
    );
}

#[test]
fn each_unit_keeps_its_best_chunk_and_ties_break_by_unit() {
    let question = [1.0f32, 0.0];
    let mut best = BestPerUnit::default();
    for (id, unit, vector) in [
        (1, "b.md", [0.0f32, 1.0]),
        (2, "b.md", [1.0, 0.0]),
        (3, "a.md", [1.0, 0.0]),
        (4, "c.md", [1.0, 1.0]),
        (5, "a.md", [1.0, 0.0]),
    ] {
        best.offer(id, unit, f64::from(cosine(&question, &vector)));
    }
    let ranked = best.ranked();
    let order: Vec<(&str, i64)> = ranked.iter().map(|h| (h.unit_id.as_str(), h.id)).collect();
    assert_eq!(order, vec![("a.md", 3), ("b.md", 2), ("c.md", 4)]);
}

#[test]
fn a_vector_is_little_endian_f32_of_the_measures_width() {
    let blob = encode(&[1.5f32, -2.0]);
    assert_eq!(
        blob,
        [1.5f32.to_le_bytes(), (-2.0f32).to_le_bytes()].concat()
    );
    assert_eq!(decode(&blob, 2), Some(vec![1.5, -2.0]));
    assert_eq!(decode(&blob, 3), None);
    assert_eq!(decode(&blob, 0), None);
}

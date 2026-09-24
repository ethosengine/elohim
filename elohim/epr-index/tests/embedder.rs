//! The embedder boundary: the fixture embedder and the batch budget, with no contract in sight.
use elohim_epr_index::embedder::{EmbedBudget, Embedder, Fixture, FIXTURE_DIMS, FIXTURE_FITNESS};
use elohim_epr_index::IndexError;

fn budget(texts: usize) -> EmbedBudget {
    EmbedBudget {
        bytes: 1 << 20,
        seconds: 5.0,
        texts,
    }
}

#[test]
fn the_fixture_embeds_unit_vectors_under_its_declared_fitness() {
    let texts = vec!["rebuild the stale index".to_string(), "garden".to_string()];
    let reply = Fixture.embed(&texts, budget(2)).expect("fixture embeds");
    assert_eq!(reply.dims, FIXTURE_DIMS);
    assert_eq!(reply.fitness, FIXTURE_FITNESS);
    assert_eq!(reply.truncated, Some(0));
    for vector in &reply.vectors {
        let norm = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
    }
    assert_eq!(
        Fixture::vector("Rebuild the STALE index!"),
        reply.vectors[0],
        "case and punctuation do not change the words"
    );
}

#[test]
fn a_batch_over_the_budget_is_refused_before_anything_runs() {
    let texts = vec!["one".to_string(), "two".to_string()];
    let (result, spawned) = Fixture.embed_metered(&texts, budget(1));
    assert!(!spawned, "an in-process embedder spawns nothing");
    match result {
        Err(IndexError::Refused(why)) => {
            assert_eq!(why, "2 texts exceed the embedding budget's batch of 1")
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
}

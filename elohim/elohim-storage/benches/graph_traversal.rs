//! Graph traversal benchmarks — neighborhood depth-2 at 1K and 10K atom scale.
//!
//! Run with: `cargo bench --features graph-native --bench graph_traversal -- --quick`
//!
//! Baselines (2026-05-16, sled backend, debug profile):
//!   n=1000  m=5  : see commit message for measurements
//!   n=1000  m=20 : see commit message for measurements
//!   n=10000 m=5  : see commit message for measurements
//!   n=10000 m=20 : see commit message for measurements
//!
//! These benchmarks are only compiled when the `graph-native` feature is enabled.
#![cfg(feature = "graph-native")]

use cozo::DataValue;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use elohim_storage::epr_codec::{
    EprHead, EprLamadContext, EprQahalContext, EprRelationship, EprShefaContext,
};
use elohim_storage::graph::{
    engine::GraphEngine, primitives::scripts::NEIGHBORHOOD, projector::GraphProjector,
    schema::apply_core_schema,
};

/// Seed `n` atoms into a fresh engine, each with `m` outbound TEACHES edges.
/// Returns the engine (with `tempdir` forgotten to avoid Drop during bench).
fn seed_n_atoms_m_fanout(n: usize, m: usize) -> GraphEngine {
    let tmp = tempfile::tempdir().unwrap();
    let engine = GraphEngine::open(&tmp.path().join("bench.db")).unwrap();
    // Forget the TempDir so the path survives the bench — the OS will clean it up.
    std::mem::forget(tmp);
    apply_core_schema(&engine).unwrap();
    let projector = GraphProjector::new(&engine);

    for i in 0..n {
        let cid = format!("bafy{i:09}");
        let rels: Vec<EprRelationship> = (1..=m)
            .map(|j| EprRelationship {
                rel_type: "TEACHES".into(),
                target: format!("t{j}"),
                target_cid: Some(format!("bafy{:09}", (i + j) % n)),
            })
            .collect();
        let head = EprHead {
            version: 1,
            id: format!("slug-{i}"),
            content: format!("bafyc{i}"),
            lamad: EprLamadContext {
                title: format!("Atom {i}"),
                content_type: "concept".into(),
                description: None,
                content_format: None,
                tags: vec![],
            },
            shefa: EprShefaContext {
                stewards: vec![],
                allocations: vec![],
            },
            qahal: EprQahalContext {
                reach: Some("commons".into()),
                layer: None,
                attestation_requirements: vec![],
            },
            relationships: rels,
            author: None,
            updated: None,
        };
        projector.project_head(&cid, &head).unwrap();
    }
    engine
}

fn bench_neighborhood(c: &mut Criterion) {
    let mut group = c.benchmark_group("neighborhood_depth_2");

    for n in [1_000_usize, 10_000] {
        for m in [5_usize, 20] {
            let engine = seed_n_atoms_m_fanout(n, m);
            group.bench_with_input(
                BenchmarkId::new(format!("n={n}_m={m}"), 0),
                &(n, m),
                |b, _| {
                    let script =
                        format!("{NEIGHBORHOOD}\n?[to, hops] := neighborhood[to, hops], hops <= 2");
                    b.iter(|| {
                        engine
                            .run_script(
                                &script,
                                &[
                                    ("start", DataValue::from("bafy000000000")),
                                    ("max_hops", DataValue::from(2_i64)),
                                ],
                            )
                            .unwrap();
                    });
                },
            );
        }
    }

    group.finish();
}

criterion_group!(benches, bench_neighborhood);
criterion_main!(benches);

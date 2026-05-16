use crate::graph::engine::{GraphEngine, GraphError};

pub fn apply_core_schema(engine: &GraphEngine) -> Result<(), GraphError> {
    // epr_node — primary EprHead projection with embedding slot for deferred HNSW pipeline.
    // Validity type encodes bitemporal "as-of" semantics natively in CozoDB.
    // NOTE: :create returns an error if the relation already exists; we swallow it (idempotent startup).
    let _ = engine.run_script(
        r#"
        :create epr_node {
            cid: String =>
            slug: String,
            content_cid: String,
            version: Int default 1,
            author_did: String? default null,
            updated_at: Validity default [9223372036854775807, true],
            embedding: <F32; 768>? default null,
        }
        "#,
        &[],
    );

    // epr_edge — directed relation between EPR nodes; tolerates forward-references
    // (target need not exist in epr_node yet — no FK enforcement in CozoDB).
    let _ = engine.run_script(
        r#"
        :create epr_edge {
            from_cid: String,
            to_cid: String,
            rel_type: String =>
            asserted_at: Validity default [9223372036854775807, true],
        }
        "#,
        &[],
    );

    // epr_lamad — learning-domain properties for an EPR node
    let _ = engine.run_script(
        r#"
        :create epr_lamad {
            cid: String =>
            title: String,
            content_type: String,
            description: String? default null,
            content_format: String? default null,
            tags: [String] default [],
        }
        "#,
        &[],
    );

    // epr_shefa — economic/stewardship properties for an EPR node
    let _ = engine.run_script(
        r#"
        :create epr_shefa {
            cid: String =>
            stewards: [String] default [],
            allocations: [Float] default [],
        }
        "#,
        &[],
    );

    // epr_qahal — community/reach properties for an EPR node
    let _ = engine.run_script(
        r#"
        :create epr_qahal {
            cid: String =>
            reach: String? default null,
            layer: String? default null,
            attestation_requirements: [String] default [],
        }
        "#,
        &[],
    );

    // Core composite indexes for traversal performance
    let _ = engine.run_script("::index create epr_edge:by_rel_type { rel_type, from_cid }", &[]);
    let _ = engine.run_script("::index create epr_edge:by_target { to_cid, rel_type }", &[]);
    let _ = engine.run_script("::index create epr_qahal:by_reach { reach }", &[]);
    let _ = engine.run_script("::index create epr_node:by_author { author_did }", &[]);
    let _ = engine.run_script("::index create epr_node:by_updated { updated_at }", &[]);
    // HNSW vector index for embedding column — deferred until embedding pipeline lands.
    // CozoDB HNSW requires non-null vector data at index-creation time; leaving as comment.

    Ok(())
}

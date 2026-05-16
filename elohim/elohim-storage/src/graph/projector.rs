use crate::epr_codec::EprHead;
use crate::graph::engine::{GraphEngine, GraphError};
use cozo::DataValue;

pub struct GraphProjector<'a> {
    engine: &'a GraphEngine,
}

impl<'a> GraphProjector<'a> {
    pub fn new(engine: &'a GraphEngine) -> Self {
        Self { engine }
    }

    /// Project an EprHead into the graph: epr_node + three pillar relations + relationship edges.
    pub fn project_head(&self, cid: &str, head: &EprHead) -> Result<(), GraphError> {
        self.upsert_node(cid, head)?;
        self.upsert_lamad(cid, head)?;
        self.upsert_shefa(cid, head)?;
        self.upsert_qahal(cid, head)?;
        for rel in &head.relationships {
            let to_cid = rel.target_cid.as_deref().unwrap_or(&rel.target);
            self.upsert_edge(cid, &rel.rel_type, to_cid)?;
        }
        Ok(())
    }

    /// Write a SUPERSEDES edge from predecessor_cid to successor_cid.
    pub fn project_supersedence(
        &self,
        predecessor_cid: &str,
        successor_cid: &str,
    ) -> Result<(), GraphError> {
        self.upsert_edge(predecessor_cid, "SUPERSEDES", successor_cid)
    }

    // ── private helpers ────────────────────────────────────────────────────

    fn upsert_node(&self, cid: &str, head: &EprHead) -> Result<(), GraphError> {
        // Validity columns require [timestamp_micros, true] tuple — plain ints rejected by CozoDB.
        let updated_at_secs = head
            .updated
            .as_ref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.timestamp())
            .unwrap_or_else(|| chrono::Utc::now().timestamp());
        // CozoDB Validity is [Int, Bool] — represent as a list literal via inline script.
        let author = head.author.clone().unwrap_or_default();
        self.engine.run_script(
            r#"?[cid, slug, content_cid, version, author_did, updated_at, embedding] <-
                [[$cid, $slug, $content_cid, $version, $author, [$updated_secs, true], null]]
               :put epr_node { cid => slug, content_cid, version, author_did, updated_at, embedding }"#,
            &[
                ("cid", DataValue::from(cid)),
                ("slug", DataValue::from(head.id.as_str())),
                ("content_cid", DataValue::from(head.content.as_str())),
                ("version", DataValue::from(head.version as i64)),
                ("author", if author.is_empty() { DataValue::Null } else { DataValue::from(author.as_str()) }),
                ("updated_secs", DataValue::from(updated_at_secs)),
            ],
        )?;
        Ok(())
    }

    fn upsert_lamad(&self, cid: &str, head: &EprHead) -> Result<(), GraphError> {
        let tags: Vec<DataValue> = head
            .lamad
            .tags
            .iter()
            .map(|t| DataValue::from(t.as_str()))
            .collect();
        self.engine.run_script(
            r#"?[cid, title, content_type, description, content_format, tags] <-
                [[$cid, $title, $content_type, $description, $content_format, $tags]]
               :put epr_lamad { cid => title, content_type, description, content_format, tags }"#,
            &[
                ("cid", DataValue::from(cid)),
                ("title", DataValue::from(head.lamad.title.as_str())),
                ("content_type", DataValue::from(head.lamad.content_type.as_str())),
                (
                    "description",
                    head.lamad
                        .description
                        .as_deref()
                        .map(DataValue::from)
                        .unwrap_or(DataValue::Null),
                ),
                (
                    "content_format",
                    head.lamad
                        .content_format
                        .as_deref()
                        .map(DataValue::from)
                        .unwrap_or(DataValue::Null),
                ),
                ("tags", DataValue::List(tags)),
            ],
        )?;
        Ok(())
    }

    fn upsert_shefa(&self, cid: &str, head: &EprHead) -> Result<(), GraphError> {
        let stewards: Vec<DataValue> = head
            .shefa
            .stewards
            .iter()
            .map(|s| DataValue::from(s.as_str()))
            .collect();
        let allocations: Vec<DataValue> = head
            .shefa
            .allocations
            .iter()
            .map(|a| DataValue::from(*a))
            .collect();
        self.engine.run_script(
            r#"?[cid, stewards, allocations] <- [[$cid, $stewards, $allocations]]
               :put epr_shefa { cid => stewards, allocations }"#,
            &[
                ("cid", DataValue::from(cid)),
                ("stewards", DataValue::List(stewards)),
                ("allocations", DataValue::List(allocations)),
            ],
        )?;
        Ok(())
    }

    fn upsert_qahal(&self, cid: &str, head: &EprHead) -> Result<(), GraphError> {
        let reqs: Vec<DataValue> = head
            .qahal
            .attestation_requirements
            .iter()
            .map(|r| DataValue::from(r.as_str()))
            .collect();
        self.engine.run_script(
            r#"?[cid, reach, layer, attestation_requirements] <-
                [[$cid, $reach, $layer, $reqs]]
               :put epr_qahal { cid => reach, layer, attestation_requirements }"#,
            &[
                ("cid", DataValue::from(cid)),
                (
                    "reach",
                    head.qahal
                        .reach
                        .as_deref()
                        .map(DataValue::from)
                        .unwrap_or(DataValue::Null),
                ),
                (
                    "layer",
                    head.qahal
                        .layer
                        .as_deref()
                        .map(DataValue::from)
                        .unwrap_or(DataValue::Null),
                ),
                ("reqs", DataValue::List(reqs)),
            ],
        )?;
        Ok(())
    }

    /// Upsert a directed edge. `asserted_at` uses Validity [timestamp_secs, true] format.
    fn upsert_edge(&self, from_cid: &str, rel_type: &str, to_cid: &str) -> Result<(), GraphError> {
        let now = chrono::Utc::now().timestamp();
        self.engine.run_script(
            r#"?[from_cid, to_cid, rel_type, asserted_at] <- [[$from, $to, $rel, [$now, true]]]
               :put epr_edge { from_cid, to_cid, rel_type => asserted_at }"#,
            &[
                ("from", DataValue::from(from_cid)),
                ("to", DataValue::from(to_cid)),
                ("rel", DataValue::from(rel_type)),
                ("now", DataValue::from(now)),
            ],
        )?;
        Ok(())
    }
}

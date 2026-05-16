use cozo::DbInstance;
use std::path::Path;

pub struct GraphEngine {
    db: DbInstance,
}

pub struct QueryResult {
    pub rows: Vec<Vec<cozo::DataValue>>,
}

impl GraphEngine {
    pub fn open(path: &Path) -> Result<Self, GraphError> {
        // sled backend (not sqlite) — libsqlite3-sys links conflict with rusqlite via Holochain. See sprint result memory.
        // cozo storage-sqlite declares `links = "sqlite3"` which collides with rusqlite (transitive from Holochain);
        // Cargo rejects two crates with the same links key. sled is also embedded/ACID/file-based — same operational shape.
        let db = DbInstance::new("sled", path.to_str().unwrap(), Default::default())
            .map_err(|e| GraphError::Open(e.to_string()))?;
        Ok(Self { db })
    }

    pub fn run_script(
        &self,
        script: &str,
        params: &[(&str, cozo::DataValue)],
    ) -> Result<QueryResult, GraphError> {
        let mut map = std::collections::BTreeMap::new();
        for (k, v) in params {
            map.insert(k.to_string(), v.clone());
        }
        let res = self
            .db
            .run_script(script, map, cozo::ScriptMutability::Mutable)
            .map_err(|e| GraphError::Query(e.to_string()))?;
        Ok(QueryResult { rows: res.rows })
    }
}

#[derive(thiserror::Error, Debug)]
pub enum GraphError {
    #[error("open: {0}")]
    Open(String),
    #[error("query: {0}")]
    Query(String),
    #[error("schema: {0}")]
    Schema(String),
}

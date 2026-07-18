//! The recipe registry (`.claude/epr-meta/recipes.yaml`): parse the YAML and mint one
//! `ProcessSpec` atom per recipe from `stages` + `edges` ONLY — `paths:` are
//! binding/placement and are EXCLUDED from the hashed atom (spec §3, §4).

use std::path::Path;

use elohim_epr_rea::{EdgeSpec, ProcessSpec, StageSpec, ValidatorRef};
use serde::Deserialize;

use super::{FlowError, FlowResult};

#[derive(Debug, Deserialize)]
pub struct Registry {
    #[allow(dead_code)]
    pub version: u32,
    pub recipes: Vec<Recipe>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recipe {
    pub id: String,
    pub version: u32,
    #[serde(default)]
    #[allow(dead_code)]
    pub description: String,
    pub stages: Vec<RecipeStage>,
    #[serde(default)]
    pub edges: Vec<RecipeEdge>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeStage {
    pub name: String,
    pub artifact_kind: String,
    /// Repo-relative globs — BINDING/PLACEMENT, excluded from the hashed ProcessSpec.
    #[serde(default)]
    pub paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeEdge {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub meaningful: bool,
    #[serde(default)]
    pub validators: Vec<String>,
}

impl Registry {
    pub fn load(path: &Path) -> FlowResult<Self> {
        let text = std::fs::read_to_string(path).map_err(|source| FlowError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        serde_yaml::from_str(&text).map_err(|source| FlowError::Yaml {
            path: path.to_path_buf(),
            source,
        })
    }

    pub fn recipe(&self, id: &str) -> Option<&Recipe> {
        self.recipes.iter().find(|r| r.id == id)
    }
}

impl Recipe {
    /// Mint the hashed `ProcessSpec` atom: stages (name + artifactKind) and edges only.
    /// `paths:` are deliberately absent so placement drift never moves the recipe CID.
    pub fn to_process_spec(&self) -> ProcessSpec {
        ProcessSpec {
            id: self.id.clone(),
            version: self.version,
            stages: self
                .stages
                .iter()
                .map(|s| StageSpec {
                    name: s.name.clone(),
                    artifact_kind: s.artifact_kind.clone(),
                })
                .collect(),
            edges: self
                .edges
                .iter()
                .map(|e| EdgeSpec {
                    from: e.from.clone(),
                    to: e.to.clone(),
                    validators: e
                        .validators
                        .iter()
                        .map(|id| ValidatorRef { id: id.clone() })
                        .collect(),
                    meaningful: e.meaningful,
                })
                .collect(),
        }
    }

    /// The stage whose `name` matches, if any.
    pub fn stage(&self, name: &str) -> Option<&RecipeStage> {
        self.stages.iter().find(|s| s.name == name)
    }
}

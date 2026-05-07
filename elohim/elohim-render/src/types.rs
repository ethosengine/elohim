use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::data_fetcher::DataFetcher;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RenderSpec {
    Echo,
    AngularSsr,
}

impl RenderSpec {
    pub fn parse(s: &str) -> crate::Result<Self> {
        match s {
            "echo" => Ok(Self::Echo),
            "angular-ssr" => Ok(Self::AngularSsr),
            other => Err(crate::RenderError::UnsupportedSpec(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RenderLimits {
    pub wall_time_ms: u64,
    pub memory_mb: u64,
    pub max_fetches: u32,
    pub max_output_bytes: usize,
}

impl Default for RenderLimits {
    fn default() -> Self {
        Self {
            wall_time_ms: 2_000,
            memory_mb: 128,
            max_fetches: 32,
            max_output_bytes: 2 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentRef {
    pub id: String,
    pub content_hash: String,
}

pub struct RenderContext {
    pub spec: RenderSpec,
    pub url: String,
    pub data_fetcher: Arc<dyn DataFetcher>,
    pub limits: RenderLimits,
}

#[derive(Debug, Clone)]
pub struct RenderOutput {
    pub html: String,
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub fetched_inputs: Vec<ContentRef>,
}

impl Default for RenderOutput {
    fn default() -> Self {
        Self {
            html: String::new(),
            status: 200,
            headers: Vec::new(),
            fetched_inputs: Vec::new(),
        }
    }
}

use async_trait::async_trait;

use crate::{RenderContext, RenderOutput, Result};

#[async_trait]
pub trait Renderer: Send + Sync {
    async fn render(&self, ctx: RenderContext) -> Result<RenderOutput>;
}

#[derive(Default)]
pub struct EchoRenderer;

#[async_trait]
impl Renderer for EchoRenderer {
    async fn render(&self, ctx: RenderContext) -> Result<RenderOutput> {
        let html = format!(
            "<!doctype html><html><body><pre>echo: {}</pre></body></html>",
            ctx.url
        );
        Ok(RenderOutput {
            html,
            status: 200,
            headers: vec![("content-type".into(), "text/html; charset=utf-8".into())],
            fetched_inputs: vec![],
        })
    }
}

use async_trait::async_trait;
use elohim_render::{
    DataFetcher, EchoRenderer, FetchRequest, FetchResponse, RenderContext, RenderLimits,
    RenderSpec, Renderer, Result,
};
use std::sync::Arc;

struct NoopFetcher;

#[async_trait]
impl DataFetcher for NoopFetcher {
    async fn fetch(&self, _request: FetchRequest) -> Result<FetchResponse> {
        unreachable!("EchoRenderer should not call fetch");
    }
}

#[tokio::test]
async fn echo_renderer_returns_url_in_html() {
    let renderer = EchoRenderer::default();
    let ctx = RenderContext {
        spec: RenderSpec::Echo,
        url: "/test/path".to_string(),
        data_fetcher: Arc::new(NoopFetcher),
        limits: RenderLimits::default(),
    };

    let out = renderer.render(ctx).await.expect("render ok");
    assert_eq!(out.status, 200);
    assert!(out.html.contains("/test/path"), "html: {}", out.html);
}

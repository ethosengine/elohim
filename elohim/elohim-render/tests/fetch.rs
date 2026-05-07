use async_trait::async_trait;
use elohim_render::runtime::JsRuntime;
use elohim_render::{DataFetcher, FetchRequest, FetchResponse, Result};
use std::collections::HashMap;
use std::sync::Arc;

struct FixedFetcher;

#[async_trait]
impl DataFetcher for FixedFetcher {
    async fn fetch(&self, request: FetchRequest) -> Result<FetchResponse> {
        Ok(FetchResponse {
            status: 200,
            headers: HashMap::from([("content-type".into(), "application/json".into())]),
            body: format!(r#"{{"url":"{}"}}"#, request.url).into_bytes(),
            content_hash: Some("test-hash".into()),
        })
    }
}

#[tokio::test]
async fn fetch_dispatches_to_data_fetcher() {
    let mut rt = JsRuntime::with_full_shims(Arc::new(FixedFetcher));
    let v = rt
        .eval_string("(async () => { const r = await fetch('/foo'); return await r.text(); })()")
        .await
        .unwrap();
    assert!(v.contains("/foo"), "got: {}", v);
}

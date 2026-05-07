use deno_core::{op2, OpState};
use deno_error::JsErrorBox;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use crate::{DataFetcher, FetchRequest, FetchResponse};

/// Wraps an `Arc<dyn DataFetcher>` for storage in deno_core's `OpState`.
pub struct FetcherHandle(pub Arc<dyn DataFetcher>);

#[op2(async)]
#[serde]
async fn op_fetch(
    state: Rc<RefCell<OpState>>,
    #[serde] request: FetchRequest,
) -> Result<FetchResponse, JsErrorBox> {
    let fetcher = {
        let state = state.borrow();
        state.borrow::<FetcherHandle>().0.clone()
    };
    fetcher
        .fetch(request)
        .await
        .map_err(|e| JsErrorBox::generic(e.to_string()))
}

deno_core::extension!(
    fetch_ext,
    ops = [op_fetch],
    js = [dir "src/shim", "fetch.js"],
    options = { fetcher: FetcherHandle },
    state = |state, options| {
        state.put(options.fetcher);
    },
);

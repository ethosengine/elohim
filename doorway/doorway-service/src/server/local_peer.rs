//! Opt-in local developer relay. Projection semantics stay in the peer's hub service.
use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Request, Response, StatusCode};

pub async fn relay(req: Request<Incoming>) -> Response<Full<Bytes>> {
    let refused = || {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header("Cache-Control", "private, no-store")
            .body(Full::new(Bytes::new()))
            .unwrap()
    };
    if req.uri().path() != "/api/local-views" && !req.uri().path().starts_with("/api/local-views/")
    {
        return refused();
    }
    let Ok(target) = std::env::var("LOCAL_VIEW_READER_URL") else {
        return refused();
    };
    let Ok(url) = reqwest::Url::parse(&target) else {
        return refused();
    };
    if url.scheme() != "http"
        || !matches!(url.host_str(), Some("127.0.0.1" | "[::1]"))
        || url.path() != "/"
    {
        return refused();
    }
    // No cookies, credentials or origins are manufactured. The access adapter supplies
    // the reader capability; the reader checks it even when the relay is reached directly.
    let authorization = req.headers().get("authorization").cloned();
    if authorization.is_none()
        || req.uri().query().is_some()
        || !matches!(req.method().as_str(), "GET" | "POST")
    {
        return refused();
    }
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    let client = CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap()
    });
    let result = client
        .request(
            req.method().clone(),
            format!("{}{}", target.trim_end_matches('/'), req.uri().path()),
        )
        .header("Authorization", authorization.unwrap())
        .send()
        .await;
    match result {
        Ok(upstream) => {
            let status = upstream.status();
            match upstream.bytes().await {
                Ok(bytes) => Response::builder()
                    .status(status)
                    .header("Content-Type", "application/json")
                    .header("Cache-Control", "private, no-store")
                    .body(Full::new(bytes))
                    .unwrap(),
                Err(_) => refused(),
            }
        }
        Err(_) => refused(),
    }
}

/// Local delivery adapter mode: no Mongo, conductors, swarms or hosted agent services.
pub async fn serve(listen: std::net::SocketAddr) -> anyhow::Result<()> {
    anyhow::ensure!(listen.ip().is_loopback(), "local relay requires loopback");
    let listener = tokio::net::TcpListener::bind(listen).await?;
    loop {
        let (stream, _) = listener.accept().await?;
        tokio::spawn(async move {
            let service = hyper::service::service_fn(|request| async move {
                Ok::<_, std::convert::Infallible>(relay(request).await)
            });
            let _ = hyper::server::conn::http1::Builder::new()
                .serve_connection(hyper_util::rt::TokioIo::new(stream), service)
                .await;
        });
    }
}

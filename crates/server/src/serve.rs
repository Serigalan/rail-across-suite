// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The HTTP side: axum takes the connection, reads the body up to its limit
//! and hands the request to `App::handle` on a blocking thread.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::body::Body as AxBody;
use axum::extract::{ConnectInfo, Request, State};
use axum::http::{HeaderName, HeaderValue, Response, StatusCode};
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::ReaderStream;

use crate::app::{App, BODY_LIMIT, LogLevel, upload_limit};
use crate::http::{Payload, Resp, build_req};

async fn handle(State(app): State<Arc<App>>, ConnectInfo(peer): ConnectInfo<SocketAddr>, request: Request) -> Response<AxBody> {
    let (parts, body) = request.into_parts();
    let url = parts.uri.path_and_query().map(|p| p.as_str().to_string()).unwrap_or_else(|| "/".into());
    let path = parts.uri.path().to_string();
    let method = parts.method.as_str().to_string();
    let mut headers = std::collections::HashMap::new();
    for (k, v) in &parts.headers {
        headers
            .entry(k.as_str().to_ascii_lowercase())
            .or_insert_with(|| String::from_utf8_lossy(v.as_bytes()).into_owned());
    }
    let mut remote = match peer.ip() {
        std::net::IpAddr::V6(v6) => v6.to_ipv4_mapped().map(|v4| v4.to_string()).unwrap_or_else(|| v6.to_string()),
        ip => ip.to_string(),
    };
    if app.settings.test_mode {
        if let Some(r) = headers.get("x-ra-test-remote") {
            remote = r.clone();
        }
    }
    let limit = upload_limit(&method, &path).unwrap_or(BODY_LIMIT);
    let declared = headers.get("content-length").and_then(|l| l.parse::<usize>().ok());
    let (raw, too_large) = if declared.is_some_and(|l| l > limit) {
        (Vec::new(), true)
    } else {
        match axum::body::to_bytes(body, limit).await {
            Ok(b) => (b.to_vec(), false),
            Err(_) => (Vec::new(), true),
        }
    };
    let req = build_req(&method, &url, headers, &remote, &app.settings.trust_proxy);
    let head_only = req.method == "HEAD";
    let (log, ip, started) = (app.settings.log, req.ip.clone(), std::time::Instant::now());
    let resp = tokio::task::spawn_blocking(move || app.handle(req, raw, too_large))
        .await
        .unwrap_or_else(|_| crate::http::ApiError::new(500, "internal").into_resp());
    // The path without its query: a share link's token stays out of the log.
    if log >= LogLevel::Info || (log >= LogLevel::Error && resp.status >= 500) {
        let ms = started.elapsed().as_secs_f64() * 1000.0;
        println!("{} {ip} {method} {path} {} {ms:.1} ms", crate::util::iso(crate::util::now_ms()), resp.status);
    }
    into_response(resp, head_only).await
}

async fn into_response(resp: Resp, head_only: bool) -> Response<AxBody> {
    let mut b = Response::builder().status(StatusCode::from_u16(resp.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR));
    let mut has_type = false;
    for (k, v) in &resp.headers {
        has_type |= k == "content-type";
        if let (Ok(k), Ok(v)) = (HeaderName::try_from(k.as_str()), HeaderValue::from_str(v)) {
            b = b.header(k, v);
        }
    }
    let body = match resp.payload {
        Payload::Empty => AxBody::empty(),
        Payload::Json(v) => {
            if !has_type {
                b = b.header("content-type", "application/json; charset=utf-8");
            }
            AxBody::from(rail_across_js::stringify(&v))
        }
        Payload::Bytes(bytes) => AxBody::from(bytes),
        Payload::File(path, start, len) => {
            b = b.header("content-length", len.to_string());
            match tokio::fs::File::open(&path).await {
                Ok(mut f) => {
                    let _ = f.seek(std::io::SeekFrom::Start(start)).await;
                    AxBody::from_stream(ReaderStream::new(f.take(len)))
                }
                Err(_) => AxBody::empty(),
            }
        }
    };
    let body = if head_only { AxBody::empty() } else { body };
    b.body(body).unwrap_or_else(|_| Response::new(AxBody::empty()))
}

/// Serve `app` on `addr` until ctrl-c or SIGTERM; `ready(port)` once bound.
pub async fn serve(app: Arc<App>, addr: &str, ready: impl FnOnce(u16)) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    ready(listener.local_addr()?.port());
    let router = Router::new().fallback(handle).with_state(app);
    axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown())
        .await
}

async fn shutdown() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let term = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {},
        _ = term => {},
    }
}

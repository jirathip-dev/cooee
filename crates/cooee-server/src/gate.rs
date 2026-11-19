use std::{
    io::{self, Write},
    net::IpAddr,
    sync::Arc,
};

use cooee_core::Devices;
use flate2::write::GzEncoder;
use futures_util::StreamExt;
use topcoat::{
    context::Cx,
    router::{
        Body, HeaderValue, Layer, LayerFuture, Next, Path,
        request::{method, uri},
        response::{IntoResponse, Response},
        try_endpoint,
    },
};

use crate::{
    Hub,
    http::{COOKIE, Caller, Paired, header, now, plain},
};

pub(crate) const OPEN: [&str; 7] = [
    "/connectors/callback",
    "/pair",
    "/pair/request",
    "/pair/claim",
    "/pair/cancel",
    "/passkey/challenge",
    "/passkey/signin",
];

pub(crate) struct Gate {
    pub(crate) names: Vec<String>,
    pub(crate) devices: Devices,
    pub(crate) hub: Arc<Hub>,
}

impl Gate {
    fn ours(&self, host: &str) -> bool {
        let name = if host.starts_with('[') {
            host.split_inclusive(']').next().unwrap_or(host)
        } else {
            host.split(':').next().unwrap_or(host)
        };
        self.names.iter().any(|n| n.eq_ignore_ascii_case(name))
    }
}

impl Layer for Gate {
    fn path(&self) -> Option<&Path> {
        None
    }

    fn handle<'a>(&'a self, cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            let host = header(cx, "host").or_else(|| uri(cx).host());
            let origin = header(cx, "origin").map(|o| {
                o.strip_prefix("https://")
                    .or_else(|| o.strip_prefix("http://"))
                    .is_some_and(|o| self.ours(o))
            });
            if !host.is_some_and(|h| self.ours(h)) || origin == Some(false) {
                return plain(403, "forbidden: foreign Host or Origin");
            }
            let path = uri(cx).path();
            if OPEN.contains(&path) || path.starts_with("/look/") {
                return next.run(cx, body).await;
            }
            if path.starts_with("/mcp/agent/") {
                return next.run(cx, body).await;
            }
            if path.starts_with("/plugin/") {
                let token = header(cx, "authorization").and_then(|a| a.strip_prefix("Bearer "));
                let token = token.map(|t| cooee_core::sha256(t.as_bytes()));
                let found = self.hub.plugins().into_iter().find(|p| {
                    token.as_deref() == Some(cooee_core::sha256(p.token.as_bytes()).as_str())
                });
                return match found {
                    Some(p) => next.run(&cx.with(Caller(p)), body).await,
                    None => plain(401, "a plugin's token is needed"),
                };
            }
            let token = header(cx, "cookie").and_then(|c| {
                c.split(';')
                    .find_map(|c| c.trim().strip_prefix(COOKIE)?.strip_prefix('='))
            });
            let found = match token.map(|t| self.devices.find(t, now())).transpose() {
                Ok(found) => found.flatten(),
                Err(e) => {
                    cooee_core::note!("cooee: devices: {e}");
                    return plain(503, "cooee can't read its paired devices: try again soon.");
                }
            };
            match found {
                Some(d) => next.run(&cx.with(Paired(d)), body).await,
                None => plain(
                    401,
                    "pair this device first: run `cooee pair` on the machine",
                ),
            }
        })
    }
}

pub(crate) struct Timing;

impl Layer for Timing {
    fn path(&self) -> Option<&Path> {
        None
    }

    fn handle<'a>(&'a self, cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            let start = cooee_core::clock::instant();
            let response = next.run(cx, body).await.into_response(cx)?;
            let route = try_endpoint(cx).map_or("(none)", |e| match e.path().as_str() {
                "" => "/",
                p => p,
            });
            let status = response.status().as_u16();
            cooee_core::perf::request(method(cx).as_str(), route, status, start);
            Ok(response)
        })
    }
}

pub(crate) struct Gzip;

impl Layer for Gzip {
    fn path(&self) -> Option<&Path> {
        None
    }

    fn handle<'a>(&'a self, cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            let accepts = header(cx, "accept-encoding").is_some_and(|a| {
                a.split(',').any(|e| {
                    let mut parts = e.split(';').map(str::trim);
                    let refused = |q: &str| q.parse::<f32>().is_ok_and(|q| q <= 0.0);
                    parts.next() == Some("gzip")
                        && !parts.any(|p| p.strip_prefix("q=").is_some_and(refused))
                })
            });
            let response = next.run(cx, body).await.into_response(cx)?;
            let kind = response.headers().get("content-type");
            let kind = kind.and_then(|k| k.to_str().ok()).unwrap_or("");
            let text = kind.starts_with("text/") && !kind.starts_with("text/event-stream");
            let squeezable = text
                || ["application/json", "image/svg+xml"]
                    .iter()
                    .any(|k| kind.starts_with(k));
            if !accepts || !squeezable || response.headers().contains_key("content-encoding") {
                return Ok(response);
            }
            let (mut parts, body) = response.into_parts();
            let mut whole = Vec::new();
            let mut chunks = body.into_data_stream();
            while let Some(chunk) = chunks.next().await {
                whole.extend_from_slice(&chunk.map_err(io::Error::other)?);
            }
            if whole.len() <= 1024 {
                return Ok(Response::from_parts(parts, Body::from(whole)));
            }
            let mut zip = GzEncoder::new(Vec::new(), flate2::Compression::default());
            zip.write_all(&whole)?;
            let headers = &mut parts.headers;
            headers.remove("content-length");
            headers.insert("content-encoding", HeaderValue::from_static("gzip"));
            headers.append("vary", HeaderValue::from_static("accept-encoding"));
            if let Some(tag) = headers.get("etag").and_then(|t| t.to_str().ok())
                && !tag.starts_with("W/")
            {
                let weak = HeaderValue::try_from(format!("W/{tag}"))?;
                headers.insert("etag", weak);
            }
            Ok(Response::from_parts(parts, Body::from(zip.finish()?)))
        })
    }
}

pub(crate) fn names(bind: IpAddr, allowed_hosts: &[String]) -> Vec<String> {
    let mut names: Vec<String> = ["localhost", "127.0.0.1", "[::1]"]
        .map(str::to_owned)
        .into();
    if !bind.is_unspecified() {
        names.push(match bind {
            IpAddr::V4(ip) => ip.to_string(),
            IpAddr::V6(ip) => format!("[{ip}]"),
        });
    }
    names.extend(allowed_hosts.iter().cloned());
    names
}

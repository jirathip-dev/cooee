//! cooee's web UI in Topcoat (docs/decisions/0001-ui-framework.md): server-rendered pages and the PWA manifest.
//! M0 has only the placeholder page.

#![forbid(unsafe_code)]

use topcoat::{
    Result,
    context::Cx,
    router::{
        Body, Router, page,
        response::{IntoResponse, Response},
        route,
    },
    view::{View, view},
};

/// Every page and route of the web UI.
#[must_use]
pub fn router() -> Router {
    Router::builder()
        .page(home)
        .route(manifest)
        .route(icon)
        .build()
}

/// A fixed file served with its content type.
struct Static(&'static str, &'static str);

impl IntoResponse for Static {
    fn into_response(self, _cx: &Cx) -> Result<Response> {
        Ok(Response::builder()
            .header("Content-Type", self.0)
            .body(Body::from(self.1))?)
    }
}

#[page("/")]
async fn home() -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8">
                <meta
                    name="viewport"
                    content="width=device-width,initial-scale=1,viewport-fit=cover"
                >
                <meta name="theme-color" content="#1d3b2a">
                <title>"cooee"</title>
                <link rel="manifest" href="/manifest.webmanifest">
                <link rel="icon" href="/icon.svg" type="image/svg+xml">
                <style>
                    "body{margin:0;font:16px/1.5 system-ui,sans-serif;background:#f6f3ea;color:#1d3b2a}main{max-width:40rem;margin:0 auto;padding:2rem 1rem}"
                </style>
            </head>
            <body>
                <main>
                    <h1>"cooee"</h1>
                    <p>
                        "Chat with any coding agent from your phone. Nothing here yet."
                    </p>
                </main>
            </body>
        </html>
    })
}

#[route(GET "/manifest.webmanifest")]
async fn manifest() -> Result<Static> {
    Ok(Static(
        "application/manifest+json",
        include_str!("manifest.webmanifest"),
    ))
}

#[route(GET "/icon.svg")]
async fn icon() -> Result<Static> {
    Ok(Static("image/svg+xml", include_str!("icon.svg")))
}

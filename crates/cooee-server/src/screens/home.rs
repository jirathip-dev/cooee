//! `GET /screens/home` (#311): the app's bundled home, its blocks filled with the threads and personas they draw.

use cooee_core::personas;
use serde_json::{Value, json};
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::route,
};

use crate::http::{Answer, Data, blocking, refused};

const BUNDLED: &str = include_str!("../../../../apple/Sources/CooeeUI/Default/screens/home.json");

/// `{type: screen, source: core, title, blocks, threads, personas}`: `threads` as `GET /threads`, `personas` as
/// `GET /personas`.
#[route(GET "/screens/home")]
pub async fn home(cx: &Cx) -> Result<Answer<Value>> {
    let d = app_context::<Data>(cx);
    let (data, config) = (d.0.clone(), d.1.clone());
    let screen = blocking(cx, move |hub| {
        let mut screen: Value = serde_json::from_str(BUNDLED).map_err(refused)?;
        screen["threads"] = json!(hub.list());
        screen["personas"] = json!(personas::list(config.as_deref(), &data));
        Ok(super::core(screen))
    });
    Ok(Answer(screen.await))
}

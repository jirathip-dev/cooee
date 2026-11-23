use std::sync::Arc;

use blocks::{Bye, Sync};
use futures_util::{StreamExt, stream};
use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, app_context, try_request_context},
    router::{
        RouterBuilder,
        content::{
            Json,
            sse::{Event, KeepAlive, Sse},
        },
        error::not_found,
        parse_query_params, path_param,
        response::{IntoResponse, Response},
        route,
    },
};

use crate::{
    http::{Asking, Data, Paired, header, plain},
    live::{Frames, Live, Sent, Who},
};

path_param!(sid: String);

#[derive(Deserialize)]
struct Version {
    v: Option<u8>,
}

fn live(cx: &Cx) -> Arc<Live> {
    Arc::clone(app_context::<Arc<Live>>(cx))
}

fn who(cx: &Cx) -> Option<Who> {
    if let Some(p) = try_request_context::<Paired>(cx) {
        return Some(Who::Device(p.0.clone()));
    }
    let claim = try_request_context::<Asking>(cx)?.0.clone();
    let data = app_context::<Data>(cx).0.clone();
    Some(Who::Asking { claim, data })
}

#[route(GET "/stream")]
async fn open(cx: &Cx) -> Result<Response> {
    let asked: Option<Version> = parse_query_params(cx).ok();
    let accept = match asked.and_then(|q| q.v) {
        Some(v) => format!("application/vnd.blocks+json;v={v}"),
        None => header(cx, "accept").unwrap_or_default().to_owned(),
    };
    let Some(who) = who(cx) else {
        return plain(401, "pair this device first");
    };
    let (hello, frames) = match live(cx).attach(&who, &accept) {
        Ok(attached) => attached,
        Err(e) => return plain(406, &e.to_string()),
    };
    let hello = Event::new().event("hello").json_data(&hello)?;
    let rest = stream::unfold(Some(frames), |frames: Option<Frames>| async move {
        let mut frames = frames?;
        let event = match frames.next().await? {
            Sent::Frame(f) => Ok(Event::new().event("message").data(&*f)),
            Sent::Bye(why) => {
                return Some((Event::new().event("bye").json_data(&Bye { why }), None));
            }
        };
        Some((event, Some(frames)))
    });
    let all = stream::iter([Ok(hello)]).chain(rest);
    Sse::new(all).keep_alive(KeepAlive::new()).into_response(cx)
}

#[route(POST "/stream/{sid}")]
async fn sync(cx: &Cx, Json(s): Json<Sync>) -> Result<Json<blocks::Synced>> {
    let sid = match path_param::<Sid>(cx) {
        Ok(sid) => sid.clone(),
        Err(never) => match *never {},
    };
    let Some(who) = who(cx) else {
        return Err(not_found().into());
    };
    live(cx)
        .sync(&sid, &who, s)
        .await
        .map(Json)
        .ok_or_else(|| not_found().into())
}

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(open).route(sync)
}

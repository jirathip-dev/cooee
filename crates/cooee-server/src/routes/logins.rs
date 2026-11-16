use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    router::{RouterBuilder, content::Json, route},
};

use crate::{
    LoginView,
    http::{Answer, Code, logins, on_logins, thread_id},
};

#[derive(Deserialize, Default)]
struct Start {
    #[serde(default)]
    account: Option<String>,
    #[serde(default)]
    new: bool,
    #[serde(default)]
    machine: Option<String>,
}

#[route(POST "/logins/{id}")]
async fn login_start(cx: &Cx, Json(s): Json<Option<Start>>) -> Result<Answer<()>> {
    let name = thread_id(cx);
    let s = s.unwrap_or_default();
    Ok(Answer(
        on_logins(cx, move |l| {
            let machine = s.machine.filter(|m| !m.is_empty());
            l.start(&name, (s.account.as_deref(), s.new), machine)
        })
        .await,
    ))
}

#[derive(Deserialize)]
struct Pick {
    account: String,
}

#[route(POST "/logins/{id}/signout")]
async fn login_signout(cx: &Cx, Json(p): Json<Pick>) -> Result<Answer<()>> {
    let name = thread_id(cx);
    Ok(Answer(
        on_logins(cx, move |l| l.sign_out(&name, &p.account)).await,
    ))
}

#[route(POST "/logins")]
async fn logins_check(cx: &Cx) -> Result<Answer<()>> {
    logins(cx).ask();
    Ok(Answer(Ok(())))
}

#[route(GET "/logins/{id}")]
async fn login_view(cx: &Cx) -> Result<Answer<LoginView>> {
    let name = thread_id(cx);
    Ok(Answer(on_logins(cx, move |l| l.view(&name)).await))
}

#[route(POST "/logins/{id}/send")]
async fn login_send(cx: &Cx, Json(c): Json<Code>) -> Result<Answer<()>> {
    let name = thread_id(cx);
    Ok(Answer(on_logins(cx, move |l| l.send(&name, &c.text)).await))
}

#[route(POST "/logins/{id}/cancel")]
async fn login_cancel(cx: &Cx) -> Result<Answer<()>> {
    let name = thread_id(cx);
    Ok(Answer(on_logins(cx, move |l| l.cancel(&name)).await))
}

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(login_start)
        .route(logins_check)
        .route(login_view)
        .route(login_send)
        .route(login_cancel)
        .route(login_signout)
}

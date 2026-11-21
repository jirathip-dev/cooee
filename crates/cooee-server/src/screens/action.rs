use topcoat::{
    Result,
    context::Cx,
    router::{
        content::Json,
        response::{IntoResponse, Response},
        route,
    },
};

use crate::http::{Answer, refused};

/// `POST /screens/action`: blocks' `Action`, answered by the screen module whose `act` takes its `screen`.
#[route(POST "/screens/action")]
pub async fn action(cx: &Cx, Json(a): Json<blocks::Action>) -> Result<Response> {
    match super::act(cx, &a).await {
        Some(done) => done,
        None => Answer::<()>(Err(refused(format!("no screen {}", a.screen)))).into_response(cx),
    }
}

pub(super) fn routes(b: topcoat::router::RouterBuilder) -> topcoat::router::RouterBuilder {
    b.route(action)
}

mod action;
mod attach;
mod chat;
mod connectors;
mod home;
mod locked;
mod menu;
mod persona;
mod routines;
mod sessions;
mod settings;

use topcoat::router::RouterBuilder;

pub(crate) fn routes(b: RouterBuilder) -> RouterBuilder {
    let b = action::routes(b);
    let b = attach::routes(b);
    let b = chat::routes(b);
    let b = connectors::routes(b);
    let b = home::routes(b);
    let b = locked::routes(b);
    let b = menu::routes(b);
    let b = persona::routes(b);
    let b = routines::routes(b);
    let b = sessions::routes(b);
    settings::routes(b)
}

pub(crate) fn source(path: &str, who: &crate::live::Who) -> Option<crate::live::Source> {
    chat::source(path, who).or_else(|| sessions::source(path, who))
}

pub(crate) async fn act(
    cx: &topcoat::context::Cx,
    a: &blocks::Action,
) -> Option<topcoat::Result<topcoat::router::response::Response>> {
    if let Some(done) = chat::act(cx, a).await {
        return Some(done);
    }
    if let Some(done) = persona::act(cx, a).await {
        return Some(done);
    }
    if let Some(done) = routines::act(cx, a).await {
        return Some(done);
    }
    if let Some(done) = sessions::act(cx, a).await {
        return Some(done);
    }
    None
}

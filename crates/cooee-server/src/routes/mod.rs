mod chief;
mod connectors;
mod devices;
mod folders;
mod logins;
mod mcp;
mod pairing;
mod personas;
mod plugins;
mod push;
mod relaunch;
mod report;
mod rooms;
mod routines;
mod rules;
mod screen;
mod skills;
mod stream;
mod threads;

use topcoat::router::RouterBuilder;

pub(crate) fn routes(b: RouterBuilder) -> RouterBuilder {
    let b = chief::routes(b);
    let b = connectors::routes(b);
    let b = devices::routes(b);
    let b = folders::routes(b);
    let b = logins::routes(b);
    let b = mcp::routes(b);
    let b = pairing::routes(b);
    let b = personas::routes(b);
    let b = plugins::routes(b);
    let b = push::routes(b);
    let b = relaunch::routes(b);
    let b = report::routes(b);
    let b = rooms::routes(b);
    let b = routines::routes(b);
    let b = rules::routes(b);
    let b = screen::routes(b);
    let b = skills::routes(b);
    let b = stream::routes(b);
    threads::routes(b)
}

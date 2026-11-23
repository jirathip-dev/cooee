mod ask;
mod desk;
mod handoff;
mod mail;
mod memory;
mod persona;
mod propose;
mod proxy;
mod secret;

use topcoat::router::RouterBuilder;

pub(crate) fn routes(b: RouterBuilder) -> RouterBuilder {
    let b = ask::routes(b);
    let b = desk::routes(b);
    let b = handoff::routes(b);
    let b = mail::routes(b);
    let b = memory::routes(b);
    let b = persona::routes(b);
    let b = propose::routes(b);
    let b = proxy::routes(b);
    secret::routes(b)
}

mod attach;
mod connectors;
mod home;
mod locked;
mod menu;
mod persona;
mod routines;
mod settings;

use topcoat::router::RouterBuilder;

pub(crate) fn routes(b: RouterBuilder) -> RouterBuilder {
    let b = attach::routes(b);
    let b = connectors::routes(b);
    let b = home::routes(b);
    let b = locked::routes(b);
    let b = menu::routes(b);
    let b = persona::routes(b);
    let b = routines::routes(b);
    settings::routes(b)
}

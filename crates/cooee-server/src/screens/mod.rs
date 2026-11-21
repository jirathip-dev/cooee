mod attach;
mod connectors;
mod home;
mod locked;
mod menu;
mod pair;
mod persona;
mod routines;
mod sessions;
mod settings;

use topcoat::router::RouterBuilder;

pub(crate) fn routes(b: RouterBuilder) -> RouterBuilder {
    let b = attach::routes(b);
    let b = connectors::routes(b);
    let b = home::routes(b);
    let b = locked::routes(b);
    let b = menu::routes(b);
    let b = pair::routes(b);
    let b = persona::routes(b);
    let b = routines::routes(b);
    let b = sessions::routes(b);
    settings::routes(b)
}

pub(crate) fn source(path: &str, who: &crate::live::Who) -> Option<crate::live::Source> {
    pair::source(path, who).or_else(|| sessions::source(path, who))
}

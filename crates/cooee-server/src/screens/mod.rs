pub mod attach;
pub mod home;
pub mod locked;
pub mod menu;
pub mod persona;
pub mod routines;
pub mod settings;

use blocks::{Block, Screen};
use topcoat::router::RouterBuilder;

pub(crate) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(home::home)
        .route(menu::menu)
        .route(settings::devices)
        .route(settings::signins)
        .route(settings::machines)
        .route(settings::plugins)
        .route(attach::attach)
        .route(persona::persona)
        .route(persona::action)
        .route(routines::guards)
        .route(locked::pairing)
        .route(locked::reset_layout)
}

fn core(screen: Screen) -> Block {
    Block::Screen(Box::new(screen.source("core")))
}

fn bundled(text: &str) -> Result<Screen, crate::HubError> {
    match serde_json::from_str(text).map_err(crate::http::refused)? {
        Block::Screen(screen) => Ok(*screen),
        _ => Err(crate::http::refused("a bundled screen is no screen")),
    }
}

fn fields(value: impl serde::Serialize) -> blocks::Fields {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::Object(fields)) => fields,
        _ => blocks::Fields::new(),
    }
}

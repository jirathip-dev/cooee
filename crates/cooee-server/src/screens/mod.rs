pub mod attach;
pub mod home;
pub mod locked;
pub mod menu;
pub mod persona;
pub mod settings;

use serde_json::Value;
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
        .route(locked::pairing)
        .route(locked::reset_layout)
}

fn core(mut screen: Value) -> Value {
    if let Some(o) = screen.as_object_mut() {
        o.insert("source".to_owned(), Value::from("core"));
    }
    screen
}

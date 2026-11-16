pub mod attach;
pub mod home;
pub mod menu;
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
}

fn core(mut screen: Value) -> Value {
    screen["source"] = Value::from("core");
    screen
}

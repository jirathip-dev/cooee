//! cooee core's own screens as blocks (#300), answered from the data its JSON routes read. Each goes out with
//! `"source": "core"`, set here and never taken from a body.

pub mod attach;
pub mod home;
pub mod menu;
pub mod settings;

use serde_json::Value;
use topcoat::router::RouterBuilder;

/// Core's screen routes.
pub(crate) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(home::home)
        .route(menu::menu)
        .route(settings::devices)
        .route(settings::signins)
        .route(settings::machines)
        .route(settings::plugins)
        .route(attach::attach)
}

/// `screen`, marked as core's.
fn core(mut screen: Value) -> Value {
    screen["source"] = Value::from("core");
    screen
}

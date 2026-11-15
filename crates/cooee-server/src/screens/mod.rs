//! cooee core's own screens as blocks (#300), answered from the data its JSON routes read. Each goes out with
//! `"source": "core"`, set here and never taken from a body.

pub mod home;

use serde_json::Value;

/// `screen`, marked as core's.
fn core(mut screen: Value) -> Value {
    screen["source"] = Value::from("core");
    screen
}

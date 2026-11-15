pub mod home;

use serde_json::Value;

fn core(mut screen: Value) -> Value {
    screen["source"] = Value::from("core");
    screen
}

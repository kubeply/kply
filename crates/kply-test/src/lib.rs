use assert_cmd::Command;
use serde_json::Value;

pub fn kply_cmd() -> Command {
    Command::cargo_bin("kply").expect("kply binary should be built for tests")
}

#[must_use]
pub fn normalized_json(output: &str) -> Value {
    let mut value: Value = serde_json::from_str(output).expect("output should be JSON");
    normalize_dynamic_fields(&mut value);
    value
}

fn normalize_dynamic_fields(value: &mut Value) {
    match value {
        Value::Object(map) => {
            if map.contains_key("id") {
                map.insert("id".to_owned(), Value::String("$SESSION_ID".to_owned()));
            }
            if map.contains_key("created_at") {
                map.insert(
                    "created_at".to_owned(),
                    Value::String("$TIMESTAMP".to_owned()),
                );
            }
            for nested in map.values_mut() {
                normalize_dynamic_fields(nested);
            }
        }
        Value::Array(values) => {
            for nested in values {
                normalize_dynamic_fields(nested);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

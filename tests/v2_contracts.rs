use serde_json::{json, Value};
use webdeck::domain::{
    command::Capability,
    error::ErrorCode,
    transport::{CommandRequest, ConfigRequest, DeviceRequest},
};
#[test]
fn schema_enums_match_runtime_and_typed_requests_reject_unknown_fields() {
    let schema: Value = serde_json::from_str(include_str!("../contracts/v2.schema.json")).unwrap();
    for field in ["Capability", "ErrorCode"] {
        for value in schema["$defs"][field]["enum"].as_array().unwrap() {
            let encoded = if field == "Capability" {
                serde_json::to_value(serde_json::from_value::<Capability>(value.clone()).unwrap())
                    .unwrap()
            } else {
                serde_json::to_value(serde_json::from_value::<ErrorCode>(value.clone()).unwrap())
                    .unwrap()
            };
            assert_eq!(&encoded, value);
        }
    }
    assert!(serde_json::from_value::<CommandRequest>(json!({"message":"/debug-send {}"})).is_ok());
    assert!(serde_json::from_value::<CommandRequest>(
        json!({"message":"/debug-send {}","other":true})
    )
    .is_err());
    assert!(serde_json::from_value::<ConfigRequest>(json!({"config":{},"revision":"1"})).is_err());
    assert!(serde_json::from_value::<DeviceRequest>(
        json!({"name":"device","capabilities":["unknown"],"ttl_seconds":60})
    )
    .is_err());
}

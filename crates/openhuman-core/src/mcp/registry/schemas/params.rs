//! Shared param-deserialisation and outcome-conversion helpers used by
//! every `mcp_clients_*` and `mcp_setup_*` handler.

use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::rpc::RpcOutcome;

// ── Param helpers ─────────────────────────────────────────────────────────────

pub(super) fn read_required<T: DeserializeOwned>(
    params: &Map<String, Value>,
    key: &str,
) -> Result<T, String> {
    let value = params
        .get(key)
        .cloned()
        .ok_or_else(|| format!("missing required param '{key}'"))?;
    serde_json::from_value(value).map_err(|e| format!("invalid '{key}': {e}"))
}

pub(super) fn read_optional<T: DeserializeOwned>(
    params: &Map<String, Value>,
    key: &str,
) -> Result<Option<T>, String> {
    match params.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => serde_json::from_value(v.clone())
            .map(Some)
            .map_err(|e| format!("invalid '{key}': {e}")),
    }
}

pub(super) fn read_optional_string(
    params: &Map<String, Value>,
    key: &str,
) -> Result<Option<String>, String> {
    read_optional::<String>(params, key)
}

pub(super) fn read_optional_u32(
    params: &Map<String, Value>,
    key: &str,
) -> Result<Option<u32>, String> {
    match params.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => n
            .as_u64()
            .and_then(|v| u32::try_from(v).ok())
            .map(Some)
            .ok_or_else(|| format!("invalid '{key}': expected u32")),
        Some(other) => Err(format!(
            "invalid '{key}': expected number, got {}",
            type_name(other)
        )),
    }
}

pub(super) fn read_optional_json(
    params: &Map<String, Value>,
    key: &str,
) -> Result<Option<Value>, String> {
    match params.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => Ok(Some(v.clone())),
    }
}

pub(super) fn to_json<T: serde::Serialize>(outcome: RpcOutcome<T>) -> Result<Value, String> {
    serde_json::to_value(outcome.value).map_err(|e| e.to_string())
}

pub(super) fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

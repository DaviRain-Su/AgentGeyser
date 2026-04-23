//! AgentGeyser proxy — HTTP JSON-RPC entry point for the Spike.
//!
//! Methods: `ag_listSkills`, `ag_getIdl`, `ag_invokeSkill`.
//! `ag_invokeSkill` returns an unsigned placeholder transaction (non-custodial
//! invariant: the proxy never signs anything).

use std::sync::Arc;

use axum::{extract::State, routing::post, Json, Router};
use idl_registry::{Idl, IdlInstruction, IdlInstructionArg, IdlRegistry};
use serde::Deserialize;
use serde_json::{json, Value};

pub const UNSIGNED_TX_PLACEHOLDER: &str = "SPIKE_UNSIGNED_TX";

#[derive(Clone)]
pub struct AppState {
    pub registry: Arc<IdlRegistry>,
}

#[derive(Debug, Deserialize)]
pub struct JsonRpcReq {
    #[serde(default)]
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

/// Build the axum `Router` backed by an `AppState`.
pub fn router(state: AppState) -> Router {
    Router::new().route("/", post(rpc_handler)).with_state(state)
}

async fn rpc_handler(State(st): State<AppState>, Json(req): Json<JsonRpcReq>) -> Json<Value> {
    let id = req.id.clone();
    match req.method.as_str() {
        "ag_listSkills" => Json(ok(id, serde_json::to_value(st.registry.list_skills()).unwrap_or(Value::Null))),
        "ag_getIdl" => {
            let program_id = req
                .params
                .get("program_id")
                .and_then(Value::as_str)
                .unwrap_or_default();
            match st.registry.get_idl(program_id) {
                Some(idl) => Json(ok(id, serde_json::to_value(idl).unwrap_or(Value::Null))),
                None => Json(err(id, -32004, "idl not found")),
            }
        }
        "ag_invokeSkill" => {
            let skill_id = req
                .params
                .get("skill_id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            if skill_id.is_empty() || !st.registry.has_skill(&skill_id) {
                return Json(err(id, -32004, "skill not found"));
            }
            Json(ok(
                id,
                json!({
                    "skill_id": skill_id,
                    "transaction_base64": UNSIGNED_TX_PLACEHOLDER
                }),
            ))
        }
        _ => Json(err(id, -32601, "method not found")),
    }
}

fn ok(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn err(id: Value, code: i32, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// A small demo IDL used by the default binary so a fresh boot has something to list.
pub fn sample_hello_idl() -> Idl {
    Idl {
        version: "0.1.0".into(),
        name: "hello_world".into(),
        instructions: vec![
            IdlInstruction {
                name: "initialize".into(),
                args: vec![IdlInstructionArg { name: "authority".into(), kind: "publicKey".into(), ..Default::default() }],
                ..Default::default()
            },
            IdlInstruction {
                name: "greet".into(),
                args: vec![IdlInstructionArg { name: "name".into(), kind: "string".into(), ..Default::default() }],
                ..Default::default()
            },
            IdlInstruction {
                name: "set_counter".into(),
                args: vec![IdlInstructionArg { name: "value".into(), kind: "u64".into(), ..Default::default() }],
                ..Default::default()
            },
        ],
        ..Default::default()
    }
}

pub const DEMO_PROGRAM_ID: &str = "HELLO111111111111111111111111111111111111111";
